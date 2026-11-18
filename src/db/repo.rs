use crate::db::{Database, L1CacheSnapshot};
use crate::models::{
    AdCampaign, AnalyticsEvent, CategoryNode, CommerceProduct, ContentItem,
    CorporateVerificationRequest, MagazineIssue, MagazinePageSpread, NewsletterCampaign,
    OrderInvoice, RecommendationRule, RenewalReminderLog, SecurityAuditLog, SeoRedirectRule,
    SiteSetting, SsbInterestTag, SubscriptionPlan, SystemNotification, UserAccount,
};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::time::Instant;

impl Database {
    pub fn refresh_l1_cache(&self) {
        let start = Instant::now();
        let categories = self.load_categories_db();
        let ssb_tags = self.load_ssb_tags_db();
        let contents = self.load_contents_db();
        let mut issues = self.load_issues_db();
        Self::enrich_issue_spreads_from_contents(&mut issues, &contents);
        let ads = self.load_ads_db();
        let site_settings = self.load_site_settings_db();
        let notifications = self.load_notifications_db();
        let seo_redirects = self.load_seo_redirects_db();
        let subscription_plans = self.load_subscription_plans_db();
        let commerce_products = self.load_commerce_products_db();
        let users = self.load_users_db();
        let elapsed_us = start.elapsed().as_micros().max(1);

        if let Ok(mut guard) = self.l1_cache.write() {
            *guard = Some(L1CacheSnapshot {
                categories,
                ssb_tags,
                contents,
                issues,
                ads,
                site_settings,
                notifications,
                seo_redirects,
                subscription_plans,
                commerce_products,
                users,
                last_refreshed_us: elapsed_us,
            });
        }
    }

    /// Selectively reload only the named L1 cache sections.
    /// Supported section names: "categories", "ssb_tags", "contents", "issues",
    /// "ads", "site_settings", "notifications", "seo_redirects",
    /// "subscription_plans", "commerce_products", "users".
    pub fn refresh_l1_sections(&self, sections: &[&str]) {
        // Clone current snapshot; if none exists, do a full refresh instead.
        let mut snap = match self.l1_cache.read() {
            Ok(guard) => match guard.as_ref() {
                Some(s) => s.clone(),
                None => {
                    drop(guard);
                    self.refresh_l1_cache();
                    return;
                }
            },
            Err(_) => {
                self.refresh_l1_cache();
                return;
            }
        };

        for section in sections {
            match *section {
                "categories" => snap.categories = self.load_categories_db(),
                "ssb_tags" => snap.ssb_tags = self.load_ssb_tags_db(),
                "contents" => {
                    snap.contents = self.load_contents_db();
                    // Issues depend on contents for spread enrichment
                    let mut issues = self.load_issues_db();
                    Self::enrich_issue_spreads_from_contents(&mut issues, &snap.contents);
                    snap.issues = issues;
                }
                "issues" => {
                    let mut issues = self.load_issues_db();
                    Self::enrich_issue_spreads_from_contents(&mut issues, &snap.contents);
                    snap.issues = issues;
                }
                "ads" => snap.ads = self.load_ads_db(),
                "site_settings" => snap.site_settings = self.load_site_settings_db(),
                "notifications" => snap.notifications = self.load_notifications_db(),
                "seo_redirects" => snap.seo_redirects = self.load_seo_redirects_db(),
                "subscription_plans" => snap.subscription_plans = self.load_subscription_plans_db(),
                "commerce_products" => snap.commerce_products = self.load_commerce_products_db(),
                "users" => snap.users = self.load_users_db(),
                _ => {}
            }
        }

        if let Ok(mut guard) = self.l1_cache.write() {
            *guard = Some(snap);
        }

        // Invalidate L2 content query cache when contents change
        if sections.iter().any(|s| *s == "contents") {
            crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        }
    }

    fn enrich_issue_spreads_from_contents(
        issues: &mut [MagazineIssue],
        contents: &[ContentItem],
    ) {
        for issue in issues.iter_mut() {
            if issue.spreads.is_empty() {
                issue.spreads = contents
                    .iter()
                    .filter(|c| c.issue_number == Some(issue.issue_number))
                    .take(crate::db::constants::PDF_SPREADS_LIMIT)
                    .enumerate()
                    .map(|(idx, c)| {
                        let start_p = crate::db::constants::PDF_SPREAD_START_PAGE + idx * crate::db::constants::PDF_SPREAD_PAGE_INCREMENT;
                        let end_p = start_p + 6;
                        MagazinePageSpread {
                            page_no: format!("S. {:02}-{:02}", start_p, end_p),
                            title_tr: c.title_tr.clone(),
                            title_en: if c.title_en.is_empty() {
                                c.title_tr.clone()
                            } else {
                                c.title_en.clone()
                            },
                            category_badge: c.content_type.replace('-', " ").to_uppercase(),
                            image_url: c.cover_image.clone(),
                            excerpt_tr: c.summary_tr.clone(),
                            excerpt_en: if c.summary_en.is_empty() {
                                c.summary_tr.clone()
                            } else {
                                c.summary_en.clone()
                            },
                            linked_content_slug: Some(c.slug.clone()),
                        }
                    })
                    .collect();
            }
        }
    }

    pub fn get_all_categories(&self) -> Vec<CategoryNode> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.categories.clone();
            }
        }
        self.load_categories_db()
    }

    fn load_categories_db(&self) -> Vec<CategoryNode> {
        super::query_helpers::query_all(
            self,
            "SELECT id, slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic
             FROM categories ORDER BY sort_order ASC, id ASC",
            [],
            |r| Ok(CategoryNode {
                id: r.get(0)?,
                slug: r.get(1)?,
                parent_slug: r.get(2)?,
                name_tr: r.get(3)?,
                name_en: r.get(4)?,
                group_type: r.get(5)?,
                description_tr: r.get(6)?,
                description_en: r.get(7)?,
                icon_class: r.get(8)?,
                sort_order: r.get(9)?,
                is_dynamic: r.get::<_, i32>(10)? != 0,
            }),
        )
    }

    pub fn get_category_by_slug(&self, slug: &str) -> Option<CategoryNode> {
        self.get_all_categories()
            .into_iter()
            .find(|c| c.slug.eq_ignore_ascii_case(slug))
    }

    pub fn insert_category(&self, cat: &CategoryNode) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO categories (slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                cat.slug,
                cat.parent_slug,
                cat.name_tr,
                cat.name_en,
                cat.group_type,
                cat.description_tr,
                cat.description_en,
                cat.icon_class,
                cat.sort_order,
                if cat.is_dynamic { 1 } else { 0 }
            ],
        )?;
        let rowid = conn.last_insert_rowid();
        drop(conn);
        self.refresh_l1_sections(&["categories"]);
        Ok(rowid)
    }

    pub fn get_all_ssb_tags(&self) -> Vec<SsbInterestTag> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.ssb_tags.clone();
            }
        }
        self.load_ssb_tags_db()
    }

    fn load_ssb_tags_db(&self) -> Vec<SsbInterestTag> {
        super::query_helpers::query_all(
            self,
            "SELECT id, code, name_tr, name_en, category_group, sort_order
             FROM ssb_tags ORDER BY sort_order ASC, id ASC",
            [],
            |r| Ok(SsbInterestTag {
                id: r.get(0)?,
                code: r.get(1)?,
                name_tr: r.get(2)?,
                name_en: r.get(3)?,
                category_group: r.get(4)?,
                sort_order: r.get(5)?,
            }),
        )
    }

    pub fn insert_ssb_tag(&self, tag: &SsbInterestTag) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO ssb_tags (code, name_tr, name_en, category_group, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                tag.code,
                tag.name_tr,
                tag.name_en,
                tag.category_group,
                tag.sort_order
            ],
        )?;
        let rowid = conn.last_insert_rowid();
        drop(conn);
        self.refresh_l1_sections(&["ssb_tags"]);
        Ok(rowid)
    }

    fn load_all_placements_map(conn: &Connection) -> HashMap<i64, Vec<String>> {
        let mut map: HashMap<i64, Vec<String>> = HashMap::new();
        let Ok(mut stmt) = conn.prepare(
            "SELECT content_id, category_slug FROM content_placements ORDER BY content_id ASC, category_slug ASC",
        ) else {
            return map;
        };
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))) {
            for (cid, slug) in rows.flatten() {
                map.entry(cid).or_default().push(slug);
            }
        }
        map
    }

    fn load_all_ssb_tags_map(conn: &Connection) -> HashMap<i64, Vec<String>> {
        let mut map: HashMap<i64, Vec<String>> = HashMap::new();
        let Ok(mut stmt) = conn.prepare(
            "SELECT content_id, ssb_code FROM content_ssb_tags ORDER BY content_id ASC, ssb_code ASC",
        ) else {
            return map;
        };
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))) {
            for (cid, code) in rows.flatten() {
                map.entry(cid).or_default().push(code);
            }
        }
        map
    }

    pub fn get_all_contents(&self) -> Vec<ContentItem> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.contents.clone();
            }
        }
        self.load_contents_db()
    }

    fn load_contents_db(&self) -> Vec<ContentItem> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let mut placements_map = Self::load_all_placements_map(&conn);
        let mut tags_map = Self::load_all_ssb_tags_map(&conn);

        let Ok(mut stmt) = conn.prepare(
            "SELECT id, slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                    content_type, source_group, cover_image, author_name, author_title,
                    issue_number, published_at, read_time_min, visibility_level, single_price_try,
                    is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                    view_count, true_read_count, COALESCE(status, 'published')
             FROM contents
             ORDER BY published_at DESC, id DESC",
        ) else {
            return Vec::new();
        };

        let Ok(rows) = stmt.query_map([], |r| {
            Ok(ContentItem {
                id: r.get(0)?,
                slug: r.get(1)?,
                title_tr: r.get(2)?,
                title_en: r.get(3)?,
                summary_tr: r.get(4)?,
                summary_en: r.get(5)?,
                body_tr: r.get(6)?,
                body_en: r.get(7)?,
                content_type: r.get(8)?,
                source_group: r.get(9)?,
                cover_image: r.get(10)?,
                author_name: r.get(11)?,
                author_title: r.get(12)?,
                issue_number: r.get(13)?,
                published_at: r.get(14)?,
                read_time_min: r.get(15)?,
                visibility_level: r.get(16)?,
                single_price_try: r.get(17)?,
                is_sponsored: r.get::<_, i32>(18)? != 0,
                sponsor_brand: r.get(19)?,
                sponsor_legal_label: r.get(20)?,
                is_featured: r.get::<_, i32>(21)? != 0,
                video_url: r.get(22)?,
                view_count: r.get(23)?,
                true_read_count: r.get(24)?,
                placements: Vec::new(),
                ssb_tags: Vec::new(),
                status: r.get::<_, String>(25).unwrap_or_else(|_| "published".to_string()),
            })
        }) else {
            return Vec::new();
        };

        let mut list: Vec<ContentItem> = rows.filter_map(Result::ok).collect();
        for item in &mut list {
            item.placements = placements_map.remove(&item.id).unwrap_or_default();
            item.ssb_tags = tags_map.remove(&item.id).unwrap_or_default();
        }
        list
    }

    pub fn get_content_by_slug(&self, slug: &str) -> Option<ContentItem> {
        self.get_all_contents()
            .into_iter()
            .find(|c| c.slug.eq_ignore_ascii_case(slug))
    }

    pub fn get_contents_by_placement_or_type(&self, slug: &str) -> Vec<ContentItem> {
        let norm = slug.trim().to_lowercase();
        self.get_all_contents()
            .into_iter()
            .filter(|c| {
                c.content_type.to_lowercase() == norm
                    || c.source_group.to_lowercase() == norm
                    || c.placements.iter().any(|p| p.to_lowercase() == norm)
            })
            .collect()
    }

    pub fn get_contents_by_ssb_tag(&self, ssb_code: &str) -> Vec<ContentItem> {
        let norm = ssb_code.trim().to_lowercase();
        self.get_all_contents()
            .into_iter()
            .filter(|c| c.ssb_tags.iter().any(|t| t.to_lowercase() == norm))
            .collect()
    }

    /// Sanitize a user search query into a safe FTS5 MATCH expression.
    /// Splits on whitespace, strips unsafe characters (keeping alphanumeric,
    /// underscore, and Turkish letters), wraps each token in quotes with '*'
    /// prefix-match suffix, and joins with OR. Returns empty string if no
    /// valid tokens remain.
    #[cfg(test)]
    fn sanitize_fts_query(query: &str) -> String {
        let tokens: Vec<String> = query
            .split_whitespace()
            .map(|tok| {
                tok.chars()
                    .filter(|c| {
                        c.is_ascii_alphanumeric()
                            || *c == '_'
                            || matches!(*c,
                                'ç' | 'Ç' | 'ğ' | 'Ğ' | 'ı' | 'İ' |
                                'ö' | 'Ö' | 'ş' | 'Ş' | 'ü' | 'Ü' |
                                'â' | 'Â' | 'ê' | 'Ê' | 'î' | 'Î' |
                                'ô' | 'Ô' | 'û' | 'Û')
                    })
                    .collect::<String>()
            })
            .filter(|t| !t.is_empty())
            .map(|t| format!("\"{}\"*", t))
            .collect();
        if tokens.is_empty() {
            String::new()
        } else {
            tokens.join(" OR ")
        }
    }

    pub fn search_contents(&self, query: &str) -> Vec<ContentItem> {
        let q = query.trim();
        if q.is_empty() {
            return self.get_all_contents();
        }

        // Try AND first for precision, fall back to OR
        let fts_and = Self::sanitize_fts_query_with_join(q, " AND ");
        let fts_or = Self::sanitize_fts_query_with_join(q, " OR ");

        let mut fts_ids: Vec<i64> = Vec::new();

        // Attempt AND query first
        if !fts_and.is_empty() {
            let sql = format!(
                "SELECT c.id FROM contents_fts f JOIN contents c ON c.id = f.content_id \
                 WHERE contents_fts MATCH ?1 ORDER BY rank LIMIT {}",
                crate::db::constants::SEARCH_MAX_RESULTS
            );
            let ids: Vec<i64> = super::query_helpers::query_all(
                self, &sql, params![fts_and.as_str()], |r| r.get(0),
            );
            if !ids.is_empty() {
                fts_ids = ids;
            }
        }

        // Fall back to OR if AND yielded nothing
        if fts_ids.is_empty() && !fts_or.is_empty() {
            let sql = format!(
                "SELECT c.id FROM contents_fts f JOIN contents c ON c.id = f.content_id \
                 WHERE contents_fts MATCH ?1 ORDER BY rank LIMIT {}",
                crate::db::constants::SEARCH_MAX_RESULTS
            );
            fts_ids = super::query_helpers::query_all(
                self, &sql, params![fts_or.as_str()], |r| r.get(0),
            );
        }

        // Hydrate from L1 cache (preserves placements/ssb_tags)
        let all_contents = self.get_all_contents();
        let q_lower = q.to_lowercase();

        // Collect FTS-matched items first (in rank order)
        let mut result: Vec<ContentItem> = Vec::new();
        let mut seen_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

        for id in &fts_ids {
            if let Some(item) = all_contents.iter().find(|c| c.id == *id) {
                result.push(item.clone());
                seen_ids.insert(*id);
            }
        }

        // Union with legacy in-memory matches (author, ssb_tags, placements, slug)
        for item in &all_contents {
            if seen_ids.contains(&item.id) {
                continue;
            }
            if item.author_name.to_lowercase().contains(&q_lower)
                || item.ssb_tags.iter().any(|t| t.to_lowercase().contains(&q_lower))
                || item.placements.iter().any(|p| p.to_lowercase().contains(&q_lower))
                || item.slug.to_lowercase().contains(&q_lower)
            {
                result.push(item.clone());
                seen_ids.insert(item.id);
            }
        }

        // If FTS returned nothing and legacy also nothing, fall back to full list
        if result.is_empty() {
            return all_contents;
        }

        result
    }

    /// Build an FTS query with a specific join operator (AND or OR).
    fn sanitize_fts_query_with_join(query: &str, joiner: &str) -> String {
        let tokens: Vec<String> = query
            .split_whitespace()
            .map(|tok| {
                tok.chars()
                    .filter(|c| {
                        c.is_ascii_alphanumeric()
                            || *c == '_'
                            || matches!(*c,
                                'ç' | 'Ç' | 'ğ' | 'Ğ' | 'ı' | 'İ' |
                                'ö' | 'Ö' | 'ş' | 'Ş' | 'ü' | 'Ü' |
                                'â' | 'Â' | 'ê' | 'Ê' | 'î' | 'Î' |
                                'ô' | 'Ô' | 'û' | 'Û')
                    })
                    .collect::<String>()
            })
            .filter(|t| !t.is_empty())
            .map(|t| format!("\"{}\"*", t))
            .collect();
        if tokens.is_empty() {
            String::new()
        } else {
            tokens.join(joiner)
        }
    }

    pub fn upsert_content(&self, item: &ContentItem) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let status = if item.status.trim().is_empty() {
            "published"
        } else {
            item.status.trim()
        };
        if item.id > 0 {
            conn.execute(
                "UPDATE contents SET
                    slug = ?1, title_tr = ?2, title_en = ?3, summary_tr = ?4, summary_en = ?5,
                    body_tr = ?6, body_en = ?7, content_type = ?8, source_group = ?9,
                    cover_image = ?10, author_name = ?11, author_title = ?12, issue_number = ?13,
                    published_at = ?14, read_time_min = ?15, visibility_level = ?16,
                    single_price_try = ?17, is_sponsored = ?18, sponsor_brand = ?19,
                    sponsor_legal_label = ?20, is_featured = ?21, video_url = ?22,
                    status = COALESCE(NULLIF(?23, ''), status, 'published')
                 WHERE id = ?24",
                params![
                    item.slug,
                    item.title_tr,
                    item.title_en,
                    item.summary_tr,
                    item.summary_en,
                    item.body_tr,
                    item.body_en,
                    item.content_type,
                    item.source_group,
                    item.cover_image,
                    item.author_name,
                    item.author_title,
                    item.issue_number,
                    item.published_at,
                    item.read_time_min,
                    item.visibility_level,
                    item.single_price_try,
                    if item.is_sponsored { 1 } else { 0 },
                    item.sponsor_brand,
                    item.sponsor_legal_label,
                    if item.is_featured { 1 } else { 0 },
                    item.video_url,
                    status,
                    item.id
                ],
            )?;
        } else {
            conn.execute(
                "INSERT INTO contents (
                    slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                    content_type, source_group, cover_image, author_name, author_title,
                    issue_number, published_at, read_time_min, visibility_level, single_price_try,
                    is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                    view_count, true_read_count, status
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                    ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17,
                    ?18, ?19, ?20, ?21, ?22,
                    ?23, ?24, ?25
                )
                ON CONFLICT(slug) DO UPDATE SET
                    title_tr = excluded.title_tr,
                    title_en = excluded.title_en,
                    summary_tr = excluded.summary_tr,
                    summary_en = excluded.summary_en,
                    body_tr = excluded.body_tr,
                    body_en = excluded.body_en,
                    content_type = excluded.content_type,
                    source_group = excluded.source_group,
                    cover_image = excluded.cover_image,
                    author_name = excluded.author_name,
                    author_title = excluded.author_title,
                    issue_number = excluded.issue_number,
                    published_at = excluded.published_at,
                    read_time_min = excluded.read_time_min,
                    visibility_level = excluded.visibility_level,
                    single_price_try = excluded.single_price_try,
                    is_sponsored = excluded.is_sponsored,
                    sponsor_brand = excluded.sponsor_brand,
                    sponsor_legal_label = excluded.sponsor_legal_label,
                    is_featured = excluded.is_featured,
                    video_url = excluded.video_url,
                    status = COALESCE(NULLIF(excluded.status, ''), contents.status, 'published')",
                params![
                    item.slug,
                    item.title_tr,
                    item.title_en,
                    item.summary_tr,
                    item.summary_en,
                    item.body_tr,
                    item.body_en,
                    item.content_type,
                    item.source_group,
                    item.cover_image,
                    item.author_name,
                    item.author_title,
                    item.issue_number,
                    item.published_at,
                    item.read_time_min,
                    item.visibility_level,
                    item.single_price_try,
                    if item.is_sponsored { 1 } else { 0 },
                    item.sponsor_brand,
                    item.sponsor_legal_label,
                    if item.is_featured { 1 } else { 0 },
                    item.video_url,
                    item.view_count.max(1),
                    item.true_read_count.max(1),
                    status
                ],
            )?;
        }

        let content_id: i64 = if item.id > 0 {
            item.id
        } else {
            conn.query_row(
                "SELECT id FROM contents WHERE slug = ?1",
                params![item.slug],
                |r| r.get(0),
            )?
        };

        // Batch junction-table writes: one prepared statement per table, reused across rows,
        // wrapped in a single transaction so partial failures cannot leave orphaned links.
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM content_placements WHERE content_id = ?1",
            params![content_id],
        )?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)",
            )?;
            // Always include primary content_type in placements as well
            for p in item.placements.iter().chain(std::iter::once(&item.content_type)) {
                stmt.execute(params![content_id, p])?;
            }
        }

        tx.execute(
            "DELETE FROM content_ssb_tags WHERE content_id = ?1",
            params![content_id],
        )?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)",
            )?;
            for tag in &item.ssb_tags {
                stmt.execute(params![content_id, tag])?;
            }
        }
        tx.commit()?;

        drop(conn);
        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);
        Ok(content_id)
    }

    pub fn delete_content(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        let _ = tx.execute("DELETE FROM approval_requests WHERE content_id = ?1", params![id]);
        tx.execute("DELETE FROM contents WHERE id = ?1", params![id])?;
        tx.commit()?;
        drop(conn);
        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);
        Ok(())
    }

    pub fn increment_content_view(&self, slug: &str, is_true_read: bool) {
        if let Ok(conn) = self.open_conn() {
            if is_true_read {
                let _ = conn.execute(
                    "UPDATE contents SET view_count = view_count + 1, true_read_count = true_read_count + 1 WHERE LOWER(slug) = LOWER(?1)",
                    params![slug],
                );
            } else {
                let _ = conn.execute(
                    "UPDATE contents SET view_count = view_count + 1 WHERE LOWER(slug) = LOWER(?1)",
                    params![slug],
                );
            }
        }
        if let Ok(mut guard) = self.l1_cache.write() {
            if let Some(ref mut snap) = *guard {
                if let Some(item) = snap
                    .contents
                    .iter_mut()
                    .find(|c| c.slug.eq_ignore_ascii_case(slug))
                {
                    item.view_count += 1;
                    if is_true_read {
                        item.true_read_count += 1;
                    }
                    return;
                }
            }
        }
        self.refresh_l1_cache();
    }

    pub fn get_all_issues(&self) -> Vec<MagazineIssue> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.issues.clone();
            }
        }
        self.load_issues_db()
    }

    fn load_issues_db(&self) -> Vec<MagazineIssue> {
        super::query_helpers::query_all(
            self,
            "SELECT issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
                    year, cover_image, is_special_issue, event_name, page_count, dossier_count,
                    summary_tr, summary_en, pdf_path, spreads_json
             FROM magazine_issues
             ORDER BY issue_number DESC",
            [],
            |r| {
                let spreads_raw: String = r.get(15)?;
                let spreads: Vec<MagazinePageSpread> =
                    serde_json::from_str(&spreads_raw).unwrap_or_default();
                Ok(MagazineIssue {
                    issue_number: r.get(0)?,
                    slug: r.get(1)?,
                    title_tr: r.get(2)?,
                    title_en: r.get(3)?,
                    publish_month_tr: r.get(4)?,
                    publish_month_en: r.get(5)?,
                    year: r.get(6)?,
                    cover_image: r.get(7)?,
                    is_special_issue: r.get::<_, i32>(8)? != 0,
                    event_name: r.get(9)?,
                    page_count: r.get(10)?,
                    dossier_count: r.get(11)?,
                    summary_tr: r.get(12)?,
                    summary_en: r.get(13)?,
                    pdf_path: r.get(14)?,
                    spreads,
                })
            },
        )
    }

    pub fn get_issue_by_number(&self, issue_number: i32) -> Option<MagazineIssue> {
        self.get_all_issues()
            .into_iter()
            .find(|i| i.issue_number == issue_number)
            .or_else(|| self.get_all_issues().into_iter().next())
    }

    fn load_users_db(&self) -> Vec<UserAccount> {
        super::query_helpers::query_all(
            self,
            "SELECT id, email, full_name, role, organization_category, organization_name,
                    job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                    reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                    email_consent, ssb_interests_json, created_at,
                    COALESCE(email_verified, 0),
                    COALESCE(marketing_emails_opted_in, 1),
                    CASE WHEN totp_secret IS NOT NULL AND totp_secret != '' THEN 1 ELSE 0 END
             FROM users ORDER BY id ASC",
            [],
            |r| {
                let ssb_raw: String = r.get(15)?;
                let ssb_interests: Vec<String> = serde_json::from_str(&ssb_raw).unwrap_or_default();
                Ok(UserAccount {
                    id: r.get(0)?,
                    email: r.get(1)?,
                    full_name: r.get(2)?,
                    role: r.get(3)?,
                    organization_category: r.get(4)?,
                    organization_name: r.get(5)?,
                    job_title: r.get(6)?,
                    is_gov_verified: r.get::<_, i32>(7)? != 0,
                    subscription_tier: r.get(8)?,
                    subscription_expires_at: r.get(9)?,
                    reminders_halted: r.get::<_, i32>(10)? != 0,
                    kvkk_consent_at: r.get(11)?,
                    kvkk_consent_ip: r.get(12)?,
                    kvkk_consent_hash: r.get(13)?,
                    email_consent: r.get::<_, i32>(14)? != 0,
                    ssb_interests,
                    created_at: r.get(16)?,
                    email_verified: r.get::<_, i32>(17)? != 0,
                    marketing_emails_opted_in: r.get::<_, i32>(18)? != 0,
                    totp_enabled: r.get::<_, i32>(19)? != 0,
                })
            },
        )
    }

    pub fn get_all_users(&self) -> Vec<UserAccount> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.users.clone();
            }
        }
        self.load_users_db()
    }

    // =========================================================================
    // Phase 3.2: User Analytics Aggregation Queries
    // =========================================================================

    /// Returns cohort analysis data: users grouped by registration month with
    /// activity tracking across subsequent months (m1..m6).
    pub fn get_user_cohort_analysis(&self) -> Vec<serde_json::Value> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };

        let cohort_sql = "SELECT SUBSTR(created_at, 1, 7) AS cohort_month, COUNT(*) AS registered
            FROM users
            WHERE role != 'deleted' AND SUBSTR(created_at, 1, 10) >= date('now', '-12 months')
            GROUP BY cohort_month ORDER BY cohort_month";
        let Ok(mut cohort_stmt) = conn.prepare(cohort_sql) else {
            return Vec::new();
        };

        let cohorts: Vec<(String, i64)> = cohort_stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map(|iter| iter.filter_map(Result::ok).collect())
            .unwrap_or_default();

        let mut results = Vec::new();
        for (cohort_month, registered) in &cohorts {
            let mut row = serde_json::json!({
                "cohort_month": cohort_month,
                "registered": registered
            });

            for m in 1..=6 {
                let activity_sql = format!(
                    "SELECT COUNT(DISTINCT ae.user_id)
                     FROM analytics_events ae
                     JOIN users u ON ae.user_id = u.id
                     WHERE SUBSTR(u.created_at, 1, 7) = ?1
                       AND SUBSTR(ae.created_at, 1, 7) = SUBSTR(date(?1 || '-01', '+{} months'), 1, 7)",
                    m
                );
                let active: i64 = conn
                    .query_row(&activity_sql, rusqlite::params![cohort_month], |r| r.get(0))
                    .unwrap_or(0);
                row[format!("active_m{}", m)] = serde_json::json!(active);
            }
            results.push(row);
        }
        results
    }

    /// Returns monthly churn rate & subscriber expiry metrics, excluding free/standard/academic tiers.
    pub fn get_churn_rate(&self) -> serde_json::Value {
        let Ok(conn) = self.open_conn() else {
            return serde_json::json!({
                "monthly_churn_pct": 0.0,
                "total_subscribers": 0,
                "expired": 0,
                "churned_users": 0,
                "total_cohort": 0,
                "avg_lifetime_days": 0
            });
        };

        let total_subscribers: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users
                 WHERE role != 'deleted'
                   AND subscription_tier IS NOT NULL
                   AND TRIM(subscription_tier) != ''
                   AND LOWER(TRIM(subscription_tier)) NOT IN (
                       'free',
                       'standart kayıtlı okur',
                       'standart kayitli okur',
                       'akademik / öğrenci okur',
                       'akademik / ogrenci okur',
                       'akademik okur'
                   )",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let expired: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users
                 WHERE role != 'deleted'
                   AND subscription_tier IS NOT NULL
                   AND TRIM(subscription_tier) != ''
                   AND LOWER(TRIM(subscription_tier)) NOT IN (
                       'free',
                       'standart kayıtlı okur',
                       'standart kayitli okur',
                       'akademik / öğrenci okur',
                       'akademik / ogrenci okur',
                       'akademik okur'
                   )
                   AND subscription_expires_at IS NOT NULL
                   AND SUBSTR(subscription_expires_at, 1, 10) < date('now')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let churn_sql = "SELECT
            COUNT(*) AS total_cohort,
            COALESCE(SUM(CASE WHEN NOT EXISTS (
                SELECT 1 FROM analytics_events ae
                WHERE ae.user_id = u.id
                  AND SUBSTR(ae.created_at, 1, 10) >= date(SUBSTR(u.created_at, 1, 10), '+1 month')
                  AND SUBSTR(ae.created_at, 1, 10) < date(SUBSTR(u.created_at, 1, 10), '+2 months')
            ) THEN 1 ELSE 0 END), 0) AS churned
            FROM users u
            WHERE u.role != 'deleted'
              AND SUBSTR(u.created_at, 1, 10) < date('now', '-2 months')";

        let (total_cohort, churned): (i64, i64) = conn
            .query_row(churn_sql, [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap_or((0, 0));

        let churn_pct = if total_subscribers > 0 {
            (expired as f64 / total_subscribers as f64) * 100.0
        } else if total_cohort > 0 {
            (churned as f64 / total_cohort as f64) * 100.0
        } else {
            0.0
        };

        let lifetime_sql = "SELECT COALESCE(AVG(julianday(SUBSTR(last_activity, 1, 10)) - julianday(SUBSTR(created_at, 1, 10))), 0)
            FROM (
                SELECT u.created_at, MAX(ae.created_at) AS last_activity
                FROM users u
                JOIN analytics_events ae ON ae.user_id = u.id
                WHERE u.role != 'deleted'
                GROUP BY u.id
            )";
        let avg_lifetime: f64 = conn
            .query_row(lifetime_sql, [], |r| r.get::<_, f64>(0))
            .unwrap_or(0.0_f64)
            .max(0.0_f64);

        serde_json::json!({
            "monthly_churn_pct": (churn_pct * 100.0).round() / 100.0,
            "total_subscribers": total_subscribers,
            "expired": expired,
            "churned_users": churned,
            "total_cohort": total_cohort,
            "avg_lifetime_days": avg_lifetime.round() as i64
        })
    }

    /// Returns user Lifetime Value (LTV) based on paid invoices.
    pub fn get_user_ltv(&self) -> serde_json::Value {
        let Ok(conn) = self.open_conn() else {
            return serde_json::json!({"avg_ltv_try": 0.0, "by_item_type": {}, "paying_users": 0});
        };

        let avg_ltv: f64 = conn
            .query_row(
                "SELECT COALESCE(AVG(user_total), 0) FROM (
                    SELECT LOWER(user_email) AS email, SUM(amount_try) AS user_total
                    FROM order_invoices WHERE payment_status = 'paid'
                    GROUP BY LOWER(user_email)
                )",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        let paying_users: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT LOWER(user_email)) FROM order_invoices WHERE payment_status = 'paid'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let type_sql = "SELECT item_type, AVG(user_total) FROM (
                SELECT LOWER(user_email) AS email, item_type, SUM(amount_try) AS user_total
                FROM order_invoices WHERE payment_status = 'paid'
                GROUP BY LOWER(user_email), item_type
            ) GROUP BY item_type";
        let by_type: serde_json::Map<String, serde_json::Value> = match conn.prepare(type_sql) {
            Ok(mut stmt) => stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))
                .map(|iter| {
                    iter.filter_map(Result::ok)
                        .map(|(k, v)| (k, serde_json::json!((v * 100.0).round() / 100.0)))
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => serde_json::Map::new(),
        };

        serde_json::json!({
            "avg_ltv_try": (avg_ltv * 100.0).round() / 100.0,
            "by_item_type": by_type,
            "paying_users": paying_users
        })
    }

    /// Returns conversion funnel: visitors -> registered -> subscribed -> paid,
    /// excluding `'free'`, `'Standart Kayıtlı Okur'`, and `'Akademik / Öğrenci Okur'` from subscriber counts.
    pub fn get_conversion_funnel(&self) -> serde_json::Value {
        let Ok(conn) = self.open_conn() else {
            return serde_json::json!({"visitors": 0, "registered": 0, "subscribed": 0, "paid": 0});
        };

        let visitors: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT session_id) FROM analytics_events",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let registered: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE role != 'deleted'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let paid: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT LOWER(user_email)) FROM order_invoices WHERE payment_status = 'paid'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let subscribed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users
                 WHERE role != 'deleted'
                   AND (
                       (
                           subscription_tier IS NOT NULL
                           AND TRIM(subscription_tier) != ''
                           AND LOWER(TRIM(subscription_tier)) NOT IN (
                               'free',
                               'standart kayıtlı okur',
                               'standart kayitli okur',
                               'akademik / öğrenci okur',
                               'akademik / ogrenci okur',
                               'akademik okur',
                               'kurumsal tam yetki'
                           )
                       )
                       OR LOWER(email) IN (
                           SELECT LOWER(user_email) FROM order_invoices
                           WHERE payment_status = 'paid' AND item_type IN ('subscription', 'subscription_plan')
                       )
                   )",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        serde_json::json!({
            "visitors": visitors,
            "registered": registered,
            "subscribed": subscribed,
            "paid": paid,
            "conversion_rates": {
                "visitor_to_registered": if visitors > 0 { ((registered as f64 / visitors as f64) * 10000.0).round() / 100.0 } else { 0.0 },
                "registered_to_subscribed": if registered > 0 { ((subscribed as f64 / registered as f64) * 10000.0).round() / 100.0 } else { 0.0 },
                "subscribed_to_paid": if subscribed > 0 { ((paid as f64 / subscribed as f64) * 10000.0).round() / 100.0 } else { 0.0 }
            }
        })
    }

    pub fn get_user_by_email(&self, email: &str) -> Option<(UserAccount, String)> {
        let norm = email.trim().to_lowercase();
        // Try L1 cache first for the account data
        let cached_user = if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                snap.users.iter().find(|u| u.email.to_lowercase() == norm).cloned()
            } else {
                None
            }
        } else {
            None
        };

        // Password hash always requires a targeted SQL query (not in cache)
        let pw_hash: Option<String> = super::query_helpers::query_one(
            self,
            "SELECT password_hash FROM users WHERE LOWER(email) = ?1",
            params![norm.as_str()],
            |r| r.get(0),
        );

        match (cached_user, pw_hash) {
            (Some(user), Some(hash)) => Some((user, hash)),
            (None, Some(hash)) => {
                // Cache miss but user exists — load from DB
                let user_opt: Option<UserAccount> = super::query_helpers::query_one(
                    self,
                    "SELECT id, email, full_name, role, organization_category, organization_name,
                            job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                            reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                            email_consent, ssb_interests_json, created_at,
                            COALESCE(email_verified, 0),
                            COALESCE(marketing_emails_opted_in, 1),
                            CASE WHEN totp_secret IS NOT NULL AND totp_secret != '' THEN 1 ELSE 0 END
                     FROM users WHERE LOWER(email) = ?1",
                    params![norm.as_str()],
                    |r| {
                        let ssb_raw: String = r.get(15)?;
                        let ssb_interests: Vec<String> = serde_json::from_str(&ssb_raw).unwrap_or_default();
                        Ok(UserAccount {
                            id: r.get(0)?,
                            email: r.get(1)?,
                            full_name: r.get(2)?,
                            role: r.get(3)?,
                            organization_category: r.get(4)?,
                            organization_name: r.get(5)?,
                            job_title: r.get(6)?,
                            is_gov_verified: r.get::<_, i32>(7)? != 0,
                            subscription_tier: r.get(8)?,
                            subscription_expires_at: r.get(9)?,
                            reminders_halted: r.get::<_, i32>(10)? != 0,
                            kvkk_consent_at: r.get(11)?,
                            kvkk_consent_ip: r.get(12)?,
                            kvkk_consent_hash: r.get(13)?,
                            email_consent: r.get::<_, i32>(14)? != 0,
                            ssb_interests,
                            created_at: r.get(16)?,
                            email_verified: r.get::<_, i32>(17)? != 0,
                            marketing_emails_opted_in: r.get::<_, i32>(18)? != 0,
                            totp_enabled: r.get::<_, i32>(19)? != 0,
                        })
                    },
                );
                user_opt.map(|u| (u, hash))
            }
            _ => None,
        }
    }

    pub fn get_user_by_id(&self, id: i64) -> Option<UserAccount> {
        self.get_all_users().into_iter().find(|u| u.id == id)
    }

    /// GDPR Data Portability: Collect all personal data for a user into a structured JSON object.
    /// Excludes sensitive credentials (`password_hash`, `totp_secret`, bearer tokens).
    pub fn export_user_data(&self, user_id: i64) -> Option<serde_json::Value> {
        let user = self.get_user_by_id(user_id)?;

        let orders: Vec<serde_json::Value> = match self.open_conn() {
            Ok(conn) => match conn.prepare(
                "SELECT invoice_no, item_type, item_code, item_title, amount_try, payment_status, created_at
                 FROM order_invoices WHERE LOWER(user_email) = LOWER(?1) ORDER BY created_at DESC",
            ) {
                Ok(mut stmt) => stmt
                    .query_map(params![user.email], |r| {
                        Ok(serde_json::json!({
                            "invoice_no": r.get::<_, String>(0)?,
                            "item_type": r.get::<_, String>(1)?,
                            "item_code": r.get::<_, String>(2)?,
                            "item_title": r.get::<_, String>(3)?,
                            "amount_try": r.get::<_, f64>(4)?,
                            "payment_status": r.get::<_, String>(5)?,
                            "created_at": r.get::<_, String>(6)?
                        }))
                    })
                    .map(|iter| iter.filter_map(Result::ok).collect())
                    .unwrap_or_default(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        };

        let purchased_codes = self.get_user_purchased_codes(&user.email);

        let analytics_summary: serde_json::Value = match self.open_conn() {
            Ok(conn) => {
                let total_events: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM analytics_events WHERE user_id = ?1",
                        params![user_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                let true_reads: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM analytics_events WHERE user_id = ?1 AND is_true_read = 1",
                        params![user_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                let avg_duration: f64 = conn
                    .query_row(
                        "SELECT COALESCE(AVG(duration_sec), 0) FROM analytics_events WHERE user_id = ?1",
                        params![user_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0.0);
                let unique_articles: i64 = conn
                    .query_row(
                        "SELECT COUNT(DISTINCT content_slug) FROM analytics_events WHERE user_id = ?1 AND content_slug IS NOT NULL",
                        params![user_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                serde_json::json!({
                    "total_events": total_events,
                    "true_reads": true_reads,
                    "avg_duration_sec": avg_duration,
                    "unique_articles_read": unique_articles
                })
            }
            Err(_) => serde_json::json!({}),
        };

        let corporate_requests: Vec<serde_json::Value> = match self.open_conn() {
            Ok(conn) => match conn.prepare(
                "SELECT organization_category, organization_name, request_message, status, created_at
                 FROM corporate_verification_requests WHERE user_id = ?1 ORDER BY created_at DESC",
            ) {
                Ok(mut stmt) => stmt
                    .query_map(params![user_id], |r| {
                        Ok(serde_json::json!({
                            "organization_category": r.get::<_, String>(0)?,
                            "organization_name": r.get::<_, String>(1)?,
                            "request_message": r.get::<_, String>(2)?,
                            "status": r.get::<_, String>(3)?,
                            "created_at": r.get::<_, String>(4)?
                        }))
                    })
                    .map(|iter| iter.filter_map(Result::ok).collect())
                    .unwrap_or_default(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        };

        let renewal_reminders: Vec<serde_json::Value> = match self.open_conn() {
            Ok(conn) => match conn.prepare(
                "SELECT stage_label, days_remaining, status, sent_at
                 FROM renewal_reminders WHERE user_id = ?1 ORDER BY sent_at DESC",
            ) {
                Ok(mut stmt) => stmt
                    .query_map(params![user_id], |r| {
                        Ok(serde_json::json!({
                            "stage_label": r.get::<_, String>(0)?,
                            "days_remaining": r.get::<_, i32>(1)?,
                            "status": r.get::<_, String>(2)?,
                            "sent_at": r.get::<_, String>(3)?
                        }))
                    })
                    .map(|iter| iter.filter_map(Result::ok).collect())
                    .unwrap_or_default(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        };

        Some(serde_json::json!({
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "data_controller": "UMSA Prodüksiyon - MSI Dergisi",
            "gdpr_article": "KVKK Madde 11 / GDPR Article 20 - Veri Taşınabilirliği",
            "profile": {
                "email": user.email,
                "full_name": user.full_name,
                "role": user.role,
                "organization_category": user.organization_category,
                "organization_name": user.organization_name,
                "job_title": user.job_title,
                "is_gov_verified": user.is_gov_verified,
                "subscription_tier": user.subscription_tier,
                "subscription_expires_at": user.subscription_expires_at,
                "reminders_halted": user.reminders_halted,
                "ssb_interests": user.ssb_interests,
                "email_verified": user.email_verified,
                "totp_enabled": user.totp_enabled,
                "created_at": user.created_at
            },
            "consent_records": {
                "kvkk_consent_at": user.kvkk_consent_at,
                "kvkk_consent_ip": user.kvkk_consent_ip,
                "kvkk_consent_hash": user.kvkk_consent_hash,
                "email_consent": user.email_consent,
                "marketing_emails_opted_in": user.marketing_emails_opted_in
            },
            "orders": orders,
            "purchased_item_codes": purchased_codes,
            "analytics_summary": analytics_summary,
            "corporate_verification_requests": corporate_requests,
            "renewal_reminders": renewal_reminders
        }))
    }

    pub fn create_or_update_user(
        &self,
        user: &UserAccount,
        password_hash: &str,
    ) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let ssb_json =
            serde_json::to_string(&user.ssb_interests).unwrap_or_else(|_| "[]".to_string());
        let norm_email = user.email.trim().to_lowercase();
        conn.execute(
            "INSERT INTO users (
                email, password_hash, full_name, role, organization_category, organization_name,
                job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                email_consent, ssb_interests_json, created_at, email_verified, marketing_emails_opted_in
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
            ON CONFLICT(email) DO UPDATE SET
                password_hash = excluded.password_hash,
                full_name = excluded.full_name,
                role = excluded.role,
                organization_category = excluded.organization_category,
                organization_name = excluded.organization_name,
                job_title = excluded.job_title,
                is_gov_verified = excluded.is_gov_verified,
                subscription_tier = excluded.subscription_tier,
                subscription_expires_at = excluded.subscription_expires_at,
                reminders_halted = excluded.reminders_halted,
                email_consent = excluded.email_consent,
                ssb_interests_json = excluded.ssb_interests_json",
            params![
                norm_email,
                password_hash,
                user.full_name,
                user.role,
                user.organization_category,
                user.organization_name,
                user.job_title,
                if user.is_gov_verified { 1 } else { 0 },
                user.subscription_tier,
                user.subscription_expires_at,
                if user.reminders_halted { 1 } else { 0 },
                user.kvkk_consent_at,
                user.kvkk_consent_ip,
                user.kvkk_consent_hash,
                if user.email_consent { 1 } else { 0 },
                ssb_json,
                user.created_at,
                if user.email_verified { 1 } else { 0 },
                if user.marketing_emails_opted_in { 1 } else { 0 }
            ],
        )?;
        let uid: i64 = conn.query_row(
            "SELECT id FROM users WHERE LOWER(email) = ?1",
            params![norm_email],
            |r| r.get(0),
        )?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(uid)
    }

    pub fn update_user_profile(
        &self,
        user_id: i64,
        full_name: &str,
        organization_category: &str,
        organization_name: &str,
        job_title: &str,
        ssb_interests: &[String],
        email_consent: bool,
    ) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let ssb_json = serde_json::to_string(ssb_interests).unwrap_or_else(|_| "[]".to_string());
        let consent_int = if email_consent { 1 } else { 0 };
        conn.execute(
            "UPDATE users SET
                full_name = ?1,
                organization_category = ?2,
                organization_name = ?3,
                job_title = ?4,
                ssb_interests_json = ?5,
                email_consent = ?6,
                marketing_emails_opted_in = ?6
             WHERE id = ?7",
            params![
                full_name.trim(),
                organization_category.trim(),
                organization_name.trim(),
                job_title.trim(),
                ssb_json,
                consent_int,
                user_id
            ],
        )?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    pub fn update_user_password_hash(&self, email: &str, new_hash: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE users SET password_hash = ?1 WHERE LOWER(email) = LOWER(?2)",
            params![new_hash, email.trim()],
        )?;
        Ok(())
    }

    // =========================================================================
    // TOTP Two-Factor Authentication & Backup Codes
    // =========================================================================

    /// Returns the TOTP secret (base32-encoded) for a user, or None if 2FA is not enabled.
    pub fn get_user_totp_secret(&self, email: &str) -> Option<String> {
        let conn = self.open_conn().ok()?;
        conn.query_row(
            "SELECT totp_secret FROM users WHERE LOWER(email) = LOWER(?1) AND totp_secret IS NOT NULL AND totp_secret != ''",
            params![email.trim()],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|s| !s.trim().is_empty())
    }

    /// Saves or clears the TOTP secret for a user and refreshes L1 `users` cache.
    pub fn set_user_totp_secret(&self, email: &str, secret: Option<&str>) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let norm_secret = secret.map(|s| s.trim()).filter(|s| !s.is_empty());
        if let Some(sec) = norm_secret {
            conn.execute(
                "UPDATE users SET totp_secret = ?1 WHERE LOWER(email) = LOWER(?2)",
                params![sec, email.trim()],
            )?;
        } else {
            conn.execute(
                "UPDATE users SET totp_secret = NULL, totp_backup_codes_json = '[]' WHERE LOWER(email) = LOWER(?1)",
                params![email.trim()],
            )?;
        }
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    /// Retrieves the stored TOTP backup recovery codes (hashed or raw strings) for a user.
    pub fn get_user_totp_backup_codes(&self, email: &str) -> Vec<String> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let raw: String = conn
            .query_row(
                "SELECT COALESCE(totp_backup_codes_json, '[]') FROM users WHERE LOWER(email) = LOWER(?1)",
                params![email.trim()],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
    }

    /// Updates the stored TOTP backup recovery codes JSON array for a user.
    pub fn set_user_totp_backup_codes(&self, email: &str, codes: &[String]) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let json_str = serde_json::to_string(codes).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "UPDATE users SET totp_backup_codes_json = ?1 WHERE LOWER(email) = LOWER(?2)",
            params![json_str, email.trim()],
        )?;
        Ok(())
    }

    /// KVKK/GDPR Right to Erasure: soft-delete user account and purge personal data.
    /// Captures `original_email` BEFORE anonymizing `users.email`, anonymizes `order_invoices`
    /// for tax compliance, purges `user_purchased_items`, `corporate_verification_requests`,
    /// `password_reset_tokens`, `email_verification_tokens`, `unsubscribe_tokens`, `renewal_reminders`,
    /// and `analytics_events`, then refreshes L1 `users` cache.
    pub fn delete_user_account(&self, user_id: i64, client_ip: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;

        // 1. Capture original_email BEFORE overwriting users.email
        let original_email: String = tx.query_row(
            "SELECT email FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get(0),
        )?;

        let anonymized_email = format!("deleted_{}@erased.local", user_id);
        let now = chrono::Utc::now().to_rfc3339();

        // 2. Anonymize user record and wipe secrets
        tx.execute(
            "UPDATE users SET
                email = ?1,
                full_name = 'Silinmiş Kullanıcı',
                password_hash = '',
                organization_name = '',
                job_title = '',
                is_gov_verified = 0,
                kvkk_consent_ip = '',
                kvkk_consent_hash = '',
                ssb_interests_json = '[]',
                email_consent = 0,
                marketing_emails_opted_in = 0,
                email_verified = 0,
                totp_secret = NULL,
                totp_backup_codes_json = '[]',
                subscription_tier = '',
                subscription_expires_at = NULL,
                role = 'deleted'
             WHERE id = ?2",
            params![anonymized_email, user_id],
        )?;

        // 3. Anonymize order_invoices (retained for tax compliance) using original_email
        let _ = tx.execute(
            "UPDATE order_invoices SET
                user_email = ?1,
                full_name = 'Silinmiş Kullanıcı',
                organization_name = ''
             WHERE LOWER(user_email) = LOWER(?2) OR user_email = ?1",
            params![anonymized_email, original_email],
        );

        // 4. Purge analytics_events, reminders, purchased items, corporate requests, and all tokens
        let _ = tx.execute(
            "DELETE FROM analytics_events WHERE user_id = ?1",
            params![user_id],
        );
        let _ = tx.execute(
            "DELETE FROM renewal_reminders WHERE user_id = ?1 OR LOWER(user_email) = LOWER(?2)",
            params![user_id, original_email],
        );
        let _ = tx.execute(
            "DELETE FROM user_purchased_items WHERE LOWER(user_email) = LOWER(?1) OR LOWER(user_email) = LOWER(?2)",
            params![original_email, anonymized_email],
        );
        let _ = tx.execute(
            "DELETE FROM corporate_verification_requests WHERE user_id = ?1 OR LOWER(email) = LOWER(?2)",
            params![user_id, original_email],
        );
        let _ = tx.execute(
            "DELETE FROM password_reset_tokens WHERE user_id = ?1",
            params![user_id],
        );
        let _ = tx.execute(
            "DELETE FROM email_verification_tokens WHERE user_id = ?1",
            params![user_id],
        );
        let _ = tx.execute(
            "DELETE FROM unsubscribe_tokens WHERE user_id = ?1",
            params![user_id],
        );
        let _ = tx.execute(
            "DELETE FROM system_notifications WHERE target_email IS NOT NULL AND LOWER(target_email) = LOWER(?1)",
            params![original_email],
        );

        // 5. Audit trail entry
        let _ = tx.execute(
            "INSERT INTO security_audit_logs (client_ip, request_path, event_type, threat_level, details, blocked, created_at)
             VALUES (?1, '/api/v1/account/delete', 'ACCOUNT_DELETED', 'info', ?2, 0, ?3)",
            params![
                client_ip,
                format!("User ID {} account erased per KVKK/GDPR right to erasure", user_id),
                now
            ],
        );

        tx.commit()?;
        drop(conn);

        self.refresh_l1_sections(&["users", "notifications"]);
        Ok(())
    }

    pub fn get_corporate_requests(&self) -> Vec<CorporateVerificationRequest> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, user_id, email, full_name, organization_category, organization_name,
                    request_message, status, reviewed_by, created_at
             FROM corporate_verification_requests
             ORDER BY id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(CorporateVerificationRequest {
                id: r.get(0)?,
                user_id: r.get(1)?,
                email: r.get(2)?,
                full_name: r.get(3)?,
                organization_category: r.get(4)?,
                organization_name: r.get(5)?,
                request_message: r.get(6)?,
                status: r.get(7)?,
                reviewed_by: r.get(8)?,
                created_at: r.get(9)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn insert_corporate_request(
        &self,
        req: &CorporateVerificationRequest,
    ) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO corporate_verification_requests (
                user_id, email, full_name, organization_category, organization_name,
                request_message, status, reviewed_by, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                req.user_id,
                req.email,
                req.full_name,
                req.organization_category,
                req.organization_name,
                req.request_message,
                req.status,
                req.reviewed_by,
                req.created_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn decide_corporate_request(
        &self,
        id: i64,
        status: &str,
        reviewed_by: &str,
    ) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE corporate_verification_requests SET status = ?1, reviewed_by = ?2 WHERE id = ?3",
            params![status, reviewed_by, id],
        )?;
        if status == "approved" {
            let dynamic_exp = (chrono::Utc::now() + chrono::Duration::days(365))
                .format("%Y-%m-%d")
                .to_string();
            if let Ok((user_id, email)) = conn.query_row(
                "SELECT user_id, email FROM corporate_verification_requests WHERE id = ?1",
                params![id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            ) {
                let _ = conn.execute(
                    "UPDATE users SET role = 'subscriber_public', is_gov_verified = 1, subscription_tier = 'Kurumsal / Kamu Onaylı Ücretsiz Abone', subscription_expires_at = ?3 WHERE id = ?1 OR LOWER(email) = LOWER(?2)",
                    params![user_id, email, dynamic_exp],
                );
            }
        }
        drop(conn);
        // If approved, users table was modified; refresh users section
        if status == "approved" {
            self.refresh_l1_sections(&["users"]);
        }
        Ok(())
    }

    pub fn get_all_ads(&self) -> Vec<AdCampaign> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.ads.clone();
            }
        }
        self.load_ads_db()
    }

    fn load_ads_db(&self) -> Vec<AdCampaign> {
        super::query_helpers::query_all(
            self,
            "SELECT id, slot_code, badge_label, dimensions, sponsor_name, sponsor_type, pill_style,
                    title_tr, title_en, desc_tr, desc_en, target_url, image_url,
                    budget_score, is_default_fallback, is_floating_popup, is_newsletter_ad,
                    target_categories_json, target_ssb_tags_json, target_institutions_json,
                    impressions_count, clicks_count, is_active
             FROM ad_campaigns ORDER BY id ASC",
            [],
            |r| {
                let cat_raw: String = r.get(17)?;
                let ssb_raw: String = r.get(18)?;
                let inst_raw: String = r.get(19)?;
                Ok(AdCampaign {
                    id: r.get(0)?,
                    slot_code: r.get(1)?,
                    badge_label: r.get(2)?,
                    dimensions: r.get(3)?,
                    sponsor_name: r.get(4)?,
                    sponsor_type: r.get(5)?,
                    pill_style: r.get(6)?,
                    title_tr: r.get(7)?,
                    title_en: r.get(8)?,
                    desc_tr: r.get(9)?,
                    desc_en: r.get(10)?,
                    target_url: r.get(11)?,
                    image_url: r.get(12)?,
                    budget_score: r.get(13)?,
                    is_default_fallback: r.get::<_, i32>(14)? != 0,
                    is_floating_popup: r.get::<_, i32>(15)? != 0,
                    is_newsletter_ad: r.get::<_, i32>(16)? != 0,
                    target_categories: serde_json::from_str(&cat_raw).unwrap_or_default(),
                    target_ssb_tags: serde_json::from_str(&ssb_raw).unwrap_or_default(),
                    target_institutions: serde_json::from_str(&inst_raw).unwrap_or_default(),
                    impressions_count: r.get(20)?,
                    clicks_count: r.get(21)?,
                    is_active: r.get::<_, i32>(22)? != 0,
                })
            },
        )
    }

    pub fn upsert_ad(&self, ad: &AdCampaign) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let cat_json =
            serde_json::to_string(&ad.target_categories).unwrap_or_else(|_| "[]".to_string());
        let ssb_json =
            serde_json::to_string(&ad.target_ssb_tags).unwrap_or_else(|_| "[]".to_string());
        let inst_json =
            serde_json::to_string(&ad.target_institutions).unwrap_or_else(|_| "[]".to_string());

        let result_id = if ad.id > 0 {
            conn.execute(
                "UPDATE ad_campaigns SET
                    slot_code = ?1, badge_label = ?2, dimensions = ?3, sponsor_name = ?4,
                    sponsor_type = ?5, pill_style = ?6, title_tr = ?7, title_en = ?8,
                    desc_tr = ?9, desc_en = ?10, target_url = ?11, image_url = ?12,
                    budget_score = ?13, is_default_fallback = ?14, is_floating_popup = ?15,
                    is_newsletter_ad = ?16, target_categories_json = ?17, target_ssb_tags_json = ?18,
                    target_institutions_json = ?19, is_active = ?20
                 WHERE id = ?21",
                params![
                    ad.slot_code,
                    ad.badge_label,
                    ad.dimensions,
                    ad.sponsor_name,
                    ad.sponsor_type,
                    ad.pill_style,
                    ad.title_tr,
                    ad.title_en,
                    ad.desc_tr,
                    ad.desc_en,
                    ad.target_url,
                    ad.image_url,
                    ad.budget_score,
                    if ad.is_default_fallback { 1 } else { 0 },
                    if ad.is_floating_popup { 1 } else { 0 },
                    if ad.is_newsletter_ad { 1 } else { 0 },
                    cat_json,
                    ssb_json,
                    inst_json,
                    if ad.is_active { 1 } else { 0 },
                    ad.id
                ],
            )?;
            ad.id
        } else {
            conn.execute(
                "INSERT INTO ad_campaigns (
                    slot_code, badge_label, dimensions, sponsor_name, sponsor_type, pill_style,
                    title_tr, title_en, desc_tr, desc_en, target_url, image_url,
                    budget_score, is_default_fallback, is_floating_popup, is_newsletter_ad,
                    target_categories_json, target_ssb_tags_json, target_institutions_json,
                    impressions_count, clicks_count, is_active
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16,
                    ?17, ?18, ?19,
                    ?20, ?21, ?22
                )",
                params![
                    ad.slot_code,
                    ad.badge_label,
                    ad.dimensions,
                    ad.sponsor_name,
                    ad.sponsor_type,
                    ad.pill_style,
                    ad.title_tr,
                    ad.title_en,
                    ad.desc_tr,
                    ad.desc_en,
                    ad.target_url,
                    ad.image_url,
                    ad.budget_score,
                    if ad.is_default_fallback { 1 } else { 0 },
                    if ad.is_floating_popup { 1 } else { 0 },
                    if ad.is_newsletter_ad { 1 } else { 0 },
                    cat_json,
                    ssb_json,
                    inst_json,
                    ad.impressions_count.max(1),
                    ad.clicks_count,
                    if ad.is_active { 1 } else { 0 }
                ],
            )?;
            conn.last_insert_rowid()
        };
        drop(conn);
        self.refresh_l1_sections(&["ads"]);
        Ok(result_id)
    }

    pub fn delete_ad_campaign(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute("DELETE FROM ad_campaigns WHERE id = ?1", params![id])?;
        drop(conn);
        self.refresh_l1_sections(&["ads"]);
        Ok(())
    }

    pub fn increment_ad_metric(&self, ad_id: i64, is_click: bool) {
        if let Ok(conn) = self.open_conn() {
            if is_click {
                let _ = conn.execute(
                    "UPDATE ad_campaigns SET clicks_count = clicks_count + 1 WHERE id = ?1",
                    params![ad_id],
                );
            } else {
                let _ = conn.execute(
                    "UPDATE ad_campaigns SET impressions_count = impressions_count + 1 WHERE id = ?1",
                    params![ad_id],
                );
            }
        }
        if let Ok(mut guard) = self.l1_cache.write() {
            if let Some(ref mut snap) = *guard {
                if let Some(ad) = snap.ads.iter_mut().find(|a| a.id == ad_id) {
                    if is_click {
                        ad.clicks_count += 1;
                    } else {
                        ad.impressions_count += 1;
                    }
                    return;
                }
            }
        }
        self.refresh_l1_cache();
    }

    pub fn get_recommendation_rules(&self) -> Vec<RecommendationRule> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, rule_name, source_tag_or_cat, target_institution, recommended_content_slug, priority, is_active
             FROM recommendation_rules ORDER BY priority DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(RecommendationRule {
                id: r.get(0)?,
                rule_name: r.get(1)?,
                source_tag_or_cat: r.get(2)?,
                target_institution: r.get(3)?,
                recommended_content_slug: r.get(4)?,
                priority: r.get(5)?,
                is_active: r.get::<_, i32>(6)? != 0,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Returns analytics events with pagination and optional inclusive date-range filtering (`YYYY-MM-DD`).
    /// Uses `SUBSTR(created_at, 1, 10)` so both space-delimited (`YYYY-MM-DD HH:MM:SS`) and ISO-8601 (`YYYY-MM-DDTHH:MM:SSZ`) timestamps match accurately.
    pub fn get_analytics_events_paginated(
        &self,
        limit: u32,
        offset: u32,
        date_from: Option<&str>,
        date_to: Option<&str>,
    ) -> Vec<AnalyticsEvent> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let effective_limit = std::cmp::min(limit, 10_000);

        let mut conditions: Vec<String> = Vec::new();
        let mut param_idx = 1u32;
        let mut from_param: Option<String> = None;
        let mut to_param: Option<String> = None;

        if let Some(df) = date_from.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            conditions.push(format!("SUBSTR(created_at, 1, 10) >= SUBSTR(?{}, 1, 10)", param_idx));
            from_param = Some(df.to_string());
            param_idx += 1;
        }
        if let Some(dt) = date_to.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            conditions.push(format!("SUBSTR(created_at, 1, 10) <= SUBSTR(?{}, 1, 10)", param_idx));
            to_param = Some(dt.to_string());
            param_idx += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, session_id, user_id, organization_category, organization_name,
                    event_type, content_slug, ad_id, duration_sec, scroll_depth_pct,
                    video_completed, is_true_read, ssb_tag, created_at
             FROM analytics_events {} ORDER BY id DESC LIMIT ?{} OFFSET ?{}",
            where_clause,
            param_idx,
            param_idx + 1
        );

        let Ok(mut stmt) = conn.prepare(&sql) else {
            return Vec::new();
        };

        let mut dyn_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(ref f) = from_param {
            dyn_params.push(Box::new(f.clone()));
        }
        if let Some(ref t) = to_param {
            dyn_params.push(Box::new(t.clone()));
        }
        dyn_params.push(Box::new(effective_limit));
        dyn_params.push(Box::new(offset));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            dyn_params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |r| {
            Ok(AnalyticsEvent {
                id: r.get(0)?,
                session_id: r.get(1)?,
                user_id: r.get(2)?,
                organization_category: r.get(3)?,
                organization_name: r.get(4)?,
                event_type: r.get(5)?,
                content_slug: r.get(6)?,
                ad_id: r.get(7)?,
                duration_sec: r.get(8)?,
                scroll_depth_pct: r.get(9)?,
                video_completed: r.get::<_, i32>(10)? != 0,
                is_true_read: r.get::<_, i32>(11)? != 0,
                ssb_tag: r.get(12)?,
                created_at: r.get(13)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_all_analytics_events(&self) -> Vec<AnalyticsEvent> {
        self.get_analytics_events_paginated(2000, 0, None, None)
    }

    /// Returns total count of analytics events, optionally filtered by inclusive `YYYY-MM-DD` date range.
    pub fn count_analytics_events(&self, date_from: Option<&str>, date_to: Option<&str>) -> u64 {
        let Ok(conn) = self.open_conn() else {
            return 0;
        };

        let mut conditions: Vec<String> = Vec::new();
        let mut str_params: Vec<String> = Vec::new();

        if let Some(df) = date_from.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            conditions.push(format!(
                "SUBSTR(created_at, 1, 10) >= SUBSTR(?{}, 1, 10)",
                str_params.len() + 1
            ));
            str_params.push(df.to_string());
        }
        if let Some(dt) = date_to.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            conditions.push(format!(
                "SUBSTR(created_at, 1, 10) <= SUBSTR(?{}, 1, 10)",
                str_params.len() + 1
            ));
            str_params.push(dt.to_string());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT COUNT(*) FROM analytics_events {}", where_clause);
        let boxed: Vec<Box<dyn rusqlite::types::ToSql>> = str_params
            .iter()
            .map(|s| Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = boxed.iter().map(|p| p.as_ref()).collect();

        conn.query_row(&sql, rusqlite::params_from_iter(refs), |r| r.get::<_, i64>(0))
            .unwrap_or(0)
            .max(0) as u64
    }

    /// Aggregates raw `analytics_events` for a specific `YYYY-MM-DD` date into `analytics_daily_agg`.
    /// Uses `SUBSTR(created_at, 1, 10)` and `COALESCE` on composite PK dimensions (`content_slug`, `ad_id`, `organization_category`).
    pub fn aggregate_analytics_for_date(&self, date: &str) -> Result<usize, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let norm_date: String = date.trim().chars().take(10).collect();

        let sql = r#"
            INSERT OR REPLACE INTO analytics_daily_agg
                (date, content_slug, ad_id, organization_category, event_type,
                 event_count, unique_sessions, avg_duration_sec, avg_scroll_pct, true_read_count)
            SELECT
                ?1 AS date,
                COALESCE(content_slug, '') AS content_slug,
                COALESCE(ad_id, 0) AS ad_id,
                COALESCE(organization_category, '') AS organization_category,
                event_type,
                COUNT(*) AS event_count,
                COUNT(DISTINCT session_id) AS unique_sessions,
                COALESCE(AVG(duration_sec), 0) AS avg_duration_sec,
                COALESCE(AVG(scroll_depth_pct), 0) AS avg_scroll_pct,
                COALESCE(SUM(CASE WHEN is_true_read = 1 THEN 1 ELSE 0 END), 0) AS true_read_count
            FROM analytics_events
            WHERE SUBSTR(created_at, 1, 10) = ?1
            GROUP BY COALESCE(content_slug, ''), COALESCE(ad_id, 0), COALESCE(organization_category, ''), event_type
        "#;

        conn.execute(sql, rusqlite::params![norm_date])
            .map_err(|e| format!("Aggregation failed for {}: {}", date, e))
    }

    /// Returns aggregated daily analytics for an inclusive date range.
    pub fn get_daily_aggregated_analytics(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Vec<serde_json::Value> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let sql = r#"
            SELECT date, content_slug, ad_id, organization_category, event_type,
                   event_count, unique_sessions, avg_duration_sec, avg_scroll_pct, true_read_count
            FROM analytics_daily_agg
            WHERE date >= SUBSTR(?1, 1, 10) AND date <= SUBSTR(?2, 1, 10)
            ORDER BY date DESC, event_count DESC
        "#;
        let Ok(mut stmt) = conn.prepare(sql) else {
            return Vec::new();
        };
        let rows = stmt.query_map(rusqlite::params![date_from.trim(), date_to.trim()], |r| {
            let slug_raw: String = r.get(1)?;
            let ad_raw: i64 = r.get(2)?;
            let org_raw: String = r.get(3)?;
            Ok(serde_json::json!({
                "date": r.get::<_, String>(0)?,
                "content_slug": if slug_raw.is_empty() { None } else { Some(slug_raw) },
                "ad_id": if ad_raw == 0 { None } else { Some(ad_raw) },
                "organization_category": org_raw,
                "event_type": r.get::<_, String>(4)?,
                "event_count": r.get::<_, i64>(5)?,
                "unique_sessions": r.get::<_, i64>(6)?,
                "avg_duration_sec": r.get::<_, f64>(7)?,
                "avg_scroll_pct": r.get::<_, f64>(8)?,
                "true_read_count": r.get::<_, i64>(9)?,
            }))
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn insert_analytics_event(&self, ev: &AnalyticsEvent) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO analytics_events (
                session_id, user_id, organization_category, organization_name,
                event_type, content_slug, ad_id, duration_sec, scroll_depth_pct,
                video_completed, is_true_read, ssb_tag, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                ev.session_id,
                ev.user_id,
                ev.organization_category,
                ev.organization_name,
                ev.event_type,
                ev.content_slug,
                ev.ad_id,
                ev.duration_sec,
                ev.scroll_depth_pct,
                if ev.video_completed { 1 } else { 0 },
                if ev.is_true_read { 1 } else { 0 },
                ev.ssb_tag,
                ev.created_at
            ],
        )?;

        if let Some(ref slug) = ev.content_slug {
            if matches!(
                ev.event_type.as_str(),
                "reading_depth" | "article_view" | "article_read"
            ) {
                self.increment_content_view(slug, ev.is_true_read);
            }
        }
        if let Some(ad_id) = ev.ad_id {
            if ev.event_type == "ad_click" {
                self.increment_ad_metric(ad_id, true);
            } else if ev.event_type == "ad_impression" {
                self.increment_ad_metric(ad_id, false);
            }
        }

        Ok(conn.last_insert_rowid())
    }

    pub fn get_subscription_plans(&self) -> Vec<SubscriptionPlan> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.subscription_plans.clone();
            }
        }
        self.load_subscription_plans_db()
    }

    fn load_subscription_plans_db(&self) -> Vec<SubscriptionPlan> {
        super::query_helpers::query_all(
            self,
            "SELECT code, name_tr, name_en, price_try, period_label_tr, period_label_en,
                    features_tr_json, features_en_json, is_featured
             FROM subscription_plans ORDER BY sort_order ASC",
            [],
            |r| {
                let f_tr: String = r.get(6)?;
                let f_en: String = r.get(7)?;
                Ok(SubscriptionPlan {
                    code: r.get(0)?,
                    name_tr: r.get(1)?,
                    name_en: r.get(2)?,
                    price_try: r.get(3)?,
                    period_label_tr: r.get(4)?,
                    period_label_en: r.get(5)?,
                    features_tr: serde_json::from_str(&f_tr).unwrap_or_default(),
                    features_en: serde_json::from_str(&f_en).unwrap_or_default(),
                    is_featured: r.get::<_, i32>(8)? != 0,
                })
            },
        )
    }

    pub fn get_commerce_products(&self) -> Vec<CommerceProduct> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.commerce_products.clone();
            }
        }
        self.load_commerce_products_db()
    }

    fn load_commerce_products_db(&self) -> Vec<CommerceProduct> {
        super::query_helpers::query_all(
            self,
            "SELECT id, sku, product_type, title_tr, title_en, author_or_source,
                    description_tr, description_en, price_try, cover_image,
                    file_resolution_or_pages, is_available
             FROM commerce_products ORDER BY id ASC",
            [],
            |r| Ok(CommerceProduct {
                id: r.get(0)?,
                sku: r.get(1)?,
                product_type: r.get(2)?,
                title_tr: r.get(3)?,
                title_en: r.get(4)?,
                author_or_source: r.get(5)?,
                description_tr: r.get(6)?,
                description_en: r.get(7)?,
                price_try: r.get(8)?,
                cover_image: r.get(9)?,
                file_resolution_or_pages: r.get(10)?,
                is_available: r.get::<_, i32>(11)? != 0,
            }),
        )
    }

    /// Returns revenue summary grouped by date for an inclusive `YYYY-MM-DD` range.
    /// Uses `SUBSTR(created_at, 1, 10)` so both space-delimited and ISO-8601 timestamps match accurately.
    pub fn get_revenue_summary(&self, date_from: &str, date_to: &str) -> serde_json::Value {
        let Ok(conn) = self.open_conn() else {
            return serde_json::json!({
                "daily": [],
                "total_amount": 0.0,
                "weekly_total": 0.0,
                "monthly_total": 0.0,
                "invoice_count": 0,
                "by_item_type": {},
                "date_range": { "from": date_from, "to": date_to }
            });
        };

        let df = date_from.trim();
        let dt = date_to.trim();

        let daily_sql = "SELECT SUBSTR(created_at, 1, 10) AS date, COALESCE(SUM(amount_try), 0) AS total
                         FROM order_invoices
                         WHERE payment_status = 'paid'
                           AND SUBSTR(created_at, 1, 10) >= SUBSTR(?1, 1, 10)
                           AND SUBSTR(created_at, 1, 10) <= SUBSTR(?2, 1, 10)
                         GROUP BY date ORDER BY date ASC";
        let daily: Vec<serde_json::Value> = match conn.prepare(daily_sql) {
            Ok(mut stmt) => stmt
                .query_map(rusqlite::params![df, dt], |r| {
                    Ok(serde_json::json!({
                        "date": r.get::<_, String>(0)?,
                        "amount": r.get::<_, f64>(1)?
                    }))
                })
                .map(|iter| iter.filter_map(Result::ok).collect())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        let total: f64 = daily.iter().filter_map(|d| d["amount"].as_f64()).sum();
        let weekly_total: f64 = daily
            .iter()
            .rev()
            .take(7)
            .filter_map(|d| d["amount"].as_f64())
            .sum();

        let type_sql = "SELECT item_type, COALESCE(SUM(amount_try), 0) AS total
                        FROM order_invoices
                        WHERE payment_status = 'paid'
                          AND SUBSTR(created_at, 1, 10) >= SUBSTR(?1, 1, 10)
                          AND SUBSTR(created_at, 1, 10) <= SUBSTR(?2, 1, 10)
                        GROUP BY item_type";
        let by_type: serde_json::Map<String, serde_json::Value> = match conn.prepare(type_sql) {
            Ok(mut stmt) => stmt
                .query_map(rusqlite::params![df, dt], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
                })
                .map(|iter| {
                    iter.filter_map(Result::ok)
                        .map(|(k, v)| (k, serde_json::json!(v)))
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => serde_json::Map::new(),
        };

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM order_invoices
                 WHERE payment_status = 'paid'
                   AND SUBSTR(created_at, 1, 10) >= SUBSTR(?1, 1, 10)
                   AND SUBSTR(created_at, 1, 10) <= SUBSTR(?2, 1, 10)",
                rusqlite::params![df, dt],
                |r| r.get(0),
            )
            .unwrap_or(0);

        serde_json::json!({
            "daily": daily,
            "total_amount": (total * 100.0).round() / 100.0,
            "weekly_total": (weekly_total * 100.0).round() / 100.0,
            "monthly_total": (total * 100.0).round() / 100.0,
            "invoice_count": count,
            "by_item_type": by_type,
            "date_range": { "from": df, "to": dt }
        })
    }

    pub fn get_order_invoices(&self) -> Vec<OrderInvoice> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, invoice_no, user_email, full_name, organization_name,
                    item_type, item_code, item_title, amount_try, payment_status,
                    created_at, pdf_watermark_code
             FROM order_invoices ORDER BY id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(OrderInvoice {
                id: r.get(0)?,
                invoice_no: r.get(1)?,
                user_email: r.get(2)?,
                full_name: r.get(3)?,
                organization_name: r.get(4)?,
                item_type: r.get(5)?,
                item_code: r.get(6)?,
                item_title: r.get(7)?,
                amount_try: r.get(8)?,
                payment_status: r.get(9)?,
                created_at: r.get(10)?,
                pdf_watermark_code: r.get(11)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn insert_order_invoice(&self, inv: &OrderInvoice) -> rusqlite::Result<i64> {
        // Retry loop for invoice_no UNIQUE constraint collisions.
        // On collision, regenerate invoice_no with a fresh timestamp-derived suffix.
        let mut current_inv = inv.clone();
        let max_attempts = 3;

        for attempt in 0..max_attempts {
            let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            let result = conn.execute(
                "INSERT INTO order_invoices (
                    invoice_no, user_email, full_name, organization_name,
                    item_type, item_code, item_title, amount_try, payment_status,
                    created_at, pdf_watermark_code
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    current_inv.invoice_no,
                    current_inv.user_email,
                    current_inv.full_name,
                    current_inv.organization_name,
                    current_inv.item_type,
                    current_inv.item_code,
                    current_inv.item_title,
                    current_inv.amount_try,
                    current_inv.payment_status,
                    current_inv.created_at,
                    current_inv.pdf_watermark_code
                ],
            );

            match result {
                Ok(_) => {
                    let rowid = conn.last_insert_rowid();
                    drop(conn);

                    if current_inv.payment_status == "paid" {
                        self.activate_invoice_entitlements(&current_inv);
                    }
                    return Ok(rowid);
                }
                Err(e) => {
                    if matches!(e.sqlite_error_code(), Some(rusqlite::ErrorCode::ConstraintViolation))
                        && attempt < max_attempts - 1
                    {
                        // Regenerate invoice_no with fresh nanosecond timestamp to avoid collision
                        let now = chrono::Utc::now();
                        let year_str = now.format("%Y").to_string();
                        let nanos = now.timestamp_nanos_opt().unwrap_or(0);
                        let numeric_suffix: u32 = (nanos.abs() as u64 % 900_000 + 100_000) as u32;
                        current_inv.invoice_no = format!("MSI-{}-{:06}", year_str, numeric_suffix);
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(rusqlite::Error::InvalidParameterName(
            "Invoice number collision after max retries".to_string(),
        ))
    }

    fn activate_invoice_entitlements(&self, inv: &OrderInvoice) {
        let _ = self.record_user_purchase(&inv.user_email, &inv.item_code);
        if inv.item_type == "subscription" {
            let dynamic_exp = (chrono::Utc::now() + chrono::Duration::days(365))
                .format("%Y-%m-%d")
                .to_string();
            let assigned_role = match inv.item_code.as_str() {
                "kurumsal" => "subscriber_corporate",
                "basili-dijital" => "subscriber_print_digital",
                _ => "subscriber_digital",
            };
            if let Ok(conn) = self.open_conn() {
                if let Ok(tx) = conn.unchecked_transaction() {
                    let _ = tx.execute(
                        "UPDATE renewal_reminders SET status = 'halted_by_renewal' WHERE LOWER(user_email) = LOWER(?1) AND status = 'scheduled'",
                        params![inv.user_email],
                    );
                    let _ = tx.execute(
                        "UPDATE users SET
                            reminders_halted = 1,
                            role = CASE
                                WHEN role IN ('site_operator', 'admin', 'super_admin')
                                  OR EXISTS (SELECT 1 FROM user_roles ur WHERE ur.role_code = users.role)
                                THEN role
                                ELSE ?1
                            END,
                            subscription_tier = ?2,
                            subscription_expires_at = ?3
                         WHERE LOWER(email) = LOWER(?4)",
                        params![assigned_role, inv.item_title, dynamic_exp, inv.user_email],
                    );
                    let _ = tx.commit();
                }
            }
            self.refresh_l1_sections(&["users"]);
        }
    }

    /// Atomically finalizes a paid invoice from PayTR callback (`merchant_oid`).
    /// PayTR strips hyphens and prefixes `MSI` (`MSIMSI2026481920` vs `MSI-2026-481920`),
    /// so we match canonical, hyphen-stripped, and `MSI`-prefixed forms inside `conn.unchecked_transaction()`.
    pub fn finalize_paid_invoice(&self, merchant_oid: &str) -> Option<OrderInvoice> {
        let oid = merchant_oid.trim();
        if oid.is_empty() {
            return None;
        }

        let conn = self.open_conn().ok()?;
        let tx = conn.unchecked_transaction().ok()?;

        let mut target: OrderInvoice = {
            let mut stmt = tx
                .prepare(
                    "SELECT id, invoice_no, user_email, full_name, organization_name,
                            item_type, item_code, item_title, amount_try, payment_status,
                            created_at, pdf_watermark_code
                     FROM order_invoices
                     WHERE UPPER(invoice_no) = UPPER(?1)
                        OR UPPER(REPLACE(invoice_no, '-', '')) = UPPER(?1)
                        OR UPPER('MSI' || REPLACE(invoice_no, '-', '')) = UPPER(?1)
                     LIMIT 1",
                )
                .ok()?;
            stmt.query_row(params![oid], |r| {
                Ok(OrderInvoice {
                    id: r.get(0)?,
                    invoice_no: r.get(1)?,
                    user_email: r.get(2)?,
                    full_name: r.get(3)?,
                    organization_name: r.get(4)?,
                    item_type: r.get(5)?,
                    item_code: r.get(6)?,
                    item_title: r.get(7)?,
                    amount_try: r.get(8)?,
                    payment_status: r.get(9)?,
                    created_at: r.get(10)?,
                    pdf_watermark_code: r.get(11)?,
                })
            })
            .ok()?
        };

        let affected = tx
            .execute(
                "UPDATE order_invoices SET payment_status = 'paid'
                 WHERE id = ?1 AND payment_status != 'paid'",
                params![target.id],
            )
            .ok()?;

        if affected == 0 {
            drop(tx);
            return Some(target);
        }

        target.payment_status = "paid".to_string();
        let now_rfc = chrono::Utc::now().to_rfc3339();

        // Record purchase; subscription activation is handled by the caller
        // (activate_subscription_after_payment) to avoid duplicate role/tier updates.
        let _ = tx.execute(
            "INSERT OR IGNORE INTO user_purchased_items (user_email, item_code, created_at) VALUES (?1, ?2, ?3)",
            params![target.user_email.trim().to_lowercase(), target.item_code.trim(), now_rfc],
        );

        if tx.commit().is_err() {
            return None;
        }
        drop(conn);

        Some(target)
    }

    pub fn mark_invoice_failed(&self, merchant_oid: &str) -> bool {
        let oid = merchant_oid.trim();
        if oid.is_empty() {
            return false;
        }
        let Ok(conn) = self.open_conn() else {
            return false;
        };
        let Ok(tx) = conn.unchecked_transaction() else {
            return false;
        };
        let target_opt: Option<(i64, String)> = tx
            .query_row(
                "SELECT id, payment_status FROM order_invoices
                 WHERE UPPER(invoice_no) = UPPER(?1)
                    OR UPPER(REPLACE(invoice_no, '-', '')) = UPPER(?1)
                    OR UPPER('MSI' || REPLACE(invoice_no, '-', '')) = UPPER(?1)
                 LIMIT 1",
                params![oid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();

        let Some((id, status)) = target_opt else {
            return false;
        };
        if status != "paid" {
            let affected = tx
                .execute(
                    "UPDATE order_invoices SET payment_status = 'failed' WHERE id = ?1 AND payment_status != 'paid'",
                    params![id],
                )
                .unwrap_or(0);
            if tx.commit().is_ok() && affected > 0 {
                return true;
            }
        }
        false
    }

    pub fn get_user_purchased_codes(&self, email: &str) -> Vec<String> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT item_code FROM user_purchased_items WHERE LOWER(user_email) = LOWER(?1)",
        ) else {
            return Vec::new();
        };
        let Ok(rows) = stmt.query_map(params![email.trim()], |r| r.get::<_, String>(0)) else {
            return Vec::new();
        };
        rows.filter_map(Result::ok).collect()
    }

    pub fn record_user_purchase(&self, email: &str, item_code: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO user_purchased_items (user_email, item_code, created_at) VALUES (?1, ?2, ?3)",
            params![email.trim().to_lowercase(), item_code.trim(), now],
        )?;
        Ok(())
    }

    pub fn get_renewal_reminders(&self) -> Vec<RenewalReminderLog> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, user_id, user_email, full_name, organization_name,
                    stage_label, days_remaining, status, sent_at
             FROM renewal_reminders ORDER BY days_remaining ASC, id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(RenewalReminderLog {
                id: r.get(0)?,
                user_id: r.get(1)?,
                user_email: r.get(2)?,
                full_name: r.get(3)?,
                organization_name: r.get(4)?,
                stage_label: r.get(5)?,
                days_remaining: r.get(6)?,
                status: r.get(7)?,
                sent_at: r.get(8)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_renewal_reminders_paginated(&self, limit: u32, offset: u32) -> (Vec<RenewalReminderLog>, u64) {
        let Ok(conn) = self.open_conn() else {
            return (Vec::new(), 0);
        };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM renewal_reminders", [], |r| r.get(0)).unwrap_or(0);
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, user_id, user_email, full_name, organization_name,
                    stage_label, days_remaining, status, sent_at
             FROM renewal_reminders ORDER BY days_remaining ASC, id DESC LIMIT ?1 OFFSET ?2",
        ) else {
            return (Vec::new(), total);
        };
        let rows = stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(RenewalReminderLog {
                id: r.get(0)?,
                user_id: r.get(1)?,
                user_email: r.get(2)?,
                full_name: r.get(3)?,
                organization_name: r.get(4)?,
                stage_label: r.get(5)?,
                days_remaining: r.get(6)?,
                status: r.get(7)?,
                sent_at: r.get(8)?,
            })
        });
        match rows {
            Ok(iter) => (iter.filter_map(Result::ok).collect(), total),
            Err(_) => (Vec::new(), total),
        }
    }

    pub fn mark_renewal_reminder_sent(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "UPDATE renewal_reminders SET status = 'sent', sent_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn halt_renewed_reminders(&self) -> rusqlite::Result<usize> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let cutoff = (chrono::Utc::now() + chrono::Duration::days(65))
            .format("%Y-%m-%d")
            .to_string();
        let _ = conn.execute(
            "UPDATE users SET reminders_halted = 1 WHERE subscription_expires_at IS NOT NULL AND subscription_expires_at > ?1",
            params![cutoff],
        );
        let updated = conn.execute(
            "UPDATE renewal_reminders SET status = 'halted_by_renewal'
             WHERE status = 'scheduled' AND user_id IN (SELECT id FROM users WHERE reminders_halted = 1)",
            [],
        )?;
        Ok(updated)
    }

    pub fn sync_evaluated_reminders(&self, logs: &[RenewalReminderLog]) -> rusqlite::Result<usize> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        // Ensure unique constraint exists for UPSERT conflict target
        let _ = tx.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_renewal_reminders_user_stage ON renewal_reminders(user_id, stage_label)"
        );
        let mut stmt = tx.prepare_cached(
            "INSERT INTO renewal_reminders (
                user_id, user_email, full_name, organization_name,
                stage_label, days_remaining, status, sent_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(user_id, stage_label) DO UPDATE SET
                days_remaining = excluded.days_remaining,
                status = excluded.status,
                sent_at = excluded.sent_at"
        )?;
        let mut affected = 0usize;
        for log in logs {
            affected += stmt.execute(params![
                log.user_id,
                log.user_email,
                log.full_name,
                log.organization_name,
                log.stage_label,
                log.days_remaining,
                log.status,
                log.sent_at
            ])?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(affected)
    }

    pub fn get_newsletter_campaigns(&self) -> Vec<NewsletterCampaign> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, subject_tr, subject_en, target_group_filter, target_institution,
                    target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at
             FROM newsletter_campaigns ORDER BY id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(NewsletterCampaign {
                id: r.get(0)?,
                subject_tr: r.get(1)?,
                subject_en: r.get(2)?,
                target_group_filter: r.get(3)?,
                target_institution: r.get(4)?,
                target_ssb_tag: r.get(5)?,
                sponsor_ad_id: r.get(6)?,
                sponsor_name: r.get(7)?,
                recipient_count: r.get(8)?,
                status: r.get(9)?,
                sent_at: r.get(10)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_newsletter_campaigns_paginated(&self, limit: u32, offset: u32) -> (Vec<NewsletterCampaign>, u64) {
        let Ok(conn) = self.open_conn() else {
            return (Vec::new(), 0);
        };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM newsletter_campaigns", [], |r| r.get(0)).unwrap_or(0);
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, subject_tr, subject_en, target_group_filter, target_institution,
                    target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at
             FROM newsletter_campaigns ORDER BY id DESC LIMIT ?1 OFFSET ?2",
        ) else {
            return (Vec::new(), total);
        };
        let rows = stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(NewsletterCampaign {
                id: r.get(0)?,
                subject_tr: r.get(1)?,
                subject_en: r.get(2)?,
                target_group_filter: r.get(3)?,
                target_institution: r.get(4)?,
                target_ssb_tag: r.get(5)?,
                sponsor_ad_id: r.get(6)?,
                sponsor_name: r.get(7)?,
                recipient_count: r.get(8)?,
                status: r.get(9)?,
                sent_at: r.get(10)?,
            })
        });
        match rows {
            Ok(iter) => (iter.filter_map(Result::ok).collect(), total),
            Err(_) => (Vec::new(), total),
        }
    }

    pub fn insert_newsletter_campaign(&self, nl: &NewsletterCampaign) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let mut conditions: Vec<String> = vec![
            "role != 'deleted'".to_string(),
            "email_consent = 1".to_string(),
            "COALESCE(marketing_emails_opted_in, 1) = 1".to_string(),
        ];
        let mut str_params: Vec<String> = Vec::new();

        let grp = nl.target_group_filter.trim().to_lowercase();
        match grp.as_str() {
            "gov_verified" | "public_sector" | "kamu" => {
                conditions.push("is_gov_verified = 1".to_string());
            }
            "active_subscribers" | "subscribers" | "paid_subscribers" => {
                conditions.push(
                    "(is_gov_verified = 1 OR (
                        subscription_tier IS NOT NULL
                        AND TRIM(subscription_tier) != ''
                        AND LOWER(TRIM(subscription_tier)) NOT IN (
                            'free',
                            'standart kayıtlı okur',
                            'standart kayitli okur',
                            'akademik / öğrenci okur',
                            'akademik / ogrenci okur',
                            'akademik okur'
                        )
                        AND (subscription_expires_at IS NULL OR SUBSTR(subscription_expires_at, 1, 10) >= date('now'))
                    ))"
                    .to_string(),
                );
            }
            _ => {}
        }

        let inst = nl.target_institution.trim();
        if !inst.is_empty() && inst != "*" && !inst.eq_ignore_ascii_case("all") {
            let idx = str_params.len() + 1;
            conditions.push(format!(
                "(LOWER(organization_category) = LOWER(?{}) OR LOWER(organization_name) = LOWER(?{}))",
                idx, idx
            ));
            str_params.push(inst.to_string());
        }

        let tag = nl.target_ssb_tag.trim();
        if !tag.is_empty() && tag != "*" && !tag.eq_ignore_ascii_case("all") {
            let idx = str_params.len() + 1;
            conditions.push(format!("LOWER(ssb_interests_json) LIKE LOWER(?{})", idx));
            str_params.push(format!("%{}%", tag));
        }

        let count_sql = format!(
            "SELECT COUNT(*) FROM users WHERE {}",
            conditions.join(" AND ")
        );
        let boxed: Vec<Box<dyn rusqlite::types::ToSql>> = str_params
            .iter()
            .map(|s| Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = boxed.iter().map(|p| p.as_ref()).collect();

        let matched_count: i64 = conn
            .query_row(&count_sql, rusqlite::params_from_iter(refs), |r| r.get(0))
            .unwrap_or(0);

        let sent_at = if nl.sent_at.trim().is_empty() {
            chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string()
        } else {
            nl.sent_at.clone()
        };
        let status = if nl.status.trim().is_empty() {
            "sent".to_string()
        } else {
            nl.status.clone()
        };
        conn.execute(
            "INSERT INTO newsletter_campaigns (
                subject_tr, subject_en, target_group_filter, target_institution,
                target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                nl.subject_tr,
                nl.subject_en,
                nl.target_group_filter,
                nl.target_institution,
                nl.target_ssb_tag,
                nl.sponsor_ad_id,
                nl.sponsor_name,
                matched_count,
                status,
                sent_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_seo_redirects(&self) -> Vec<SeoRedirectRule> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.seo_redirects.clone();
            }
        }
        self.load_seo_redirects_db()
    }

    pub fn get_seo_redirects_paginated(&self, limit: u32, offset: u32) -> (Vec<SeoRedirectRule>, u64) {
        let Ok(conn) = self.open_conn() else {
            return (Vec::new(), 0);
        };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM seo_redirects", [], |r| r.get(0)).unwrap_or(0);
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, old_path, new_path, status_code, hit_count FROM seo_redirects ORDER BY id DESC LIMIT ?1 OFFSET ?2",
        ) else {
            return (Vec::new(), total);
        };
        let rows = stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(SeoRedirectRule {
                id: r.get(0)?,
                old_path: r.get(1)?,
                new_path: r.get(2)?,
                status_code: r.get::<_, u16>(3)?,
                hit_count: r.get(4)?,
            })
        });
        match rows {
            Ok(iter) => (iter.filter_map(Result::ok).collect(), total),
            Err(_) => (Vec::new(), total),
        }
    }

    fn load_seo_redirects_db(&self) -> Vec<SeoRedirectRule> {
        super::query_helpers::query_all(
            self,
            "SELECT id, old_path, new_path, status_code, hit_count
             FROM seo_redirects ORDER BY id DESC",
            [],
            |r| {
                Ok(SeoRedirectRule {
                    id: r.get(0)?,
                    old_path: r.get(1)?,
                    new_path: r.get(2)?,
                    status_code: r.get::<_, u16>(3)?,
                    hit_count: r.get(4)?,
                })
            },
        )
    }

    pub fn find_seo_redirect(&self, old_path: &str) -> Option<SeoRedirectRule> {
        let rule = self
            .get_seo_redirects()
            .into_iter()
            .find(|r| r.old_path.eq_ignore_ascii_case(old_path))?;
        if let Ok(conn) = self.open_conn() {
            let _ = conn.execute(
                "UPDATE seo_redirects SET hit_count = hit_count + 1 WHERE id = ?1",
                params![rule.id],
            );
        }
        Some(rule)
    }

    pub fn insert_seo_redirect(&self, rd: &SeoRedirectRule) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO seo_redirects (old_path, new_path, status_code, hit_count)
             VALUES (?1, ?2, ?3, ?4)",
            params![rd.old_path, rd.new_path, rd.status_code, rd.hit_count],
        )?;
        let rowid = conn.last_insert_rowid();
        drop(conn);
        self.refresh_l1_sections(&["seo_redirects"]);
        Ok(rowid)
    }

    // =========================================================================
    // CENTRAL SITE SETTINGS & THEME CONFIGURATION ENGINE
    // =========================================================================
    pub fn get_site_settings(&self) -> Vec<SiteSetting> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.site_settings.clone();
            }
        }
        self.load_site_settings_db()
    }

    fn load_site_settings_db(&self) -> Vec<SiteSetting> {
        super::query_helpers::query_all(
            self,
            "SELECT key, value_tr, value_en, setting_group, updated_at
             FROM site_settings ORDER BY setting_group ASC, key ASC",
            [],
            |r| Ok(SiteSetting {
                key: r.get(0)?,
                value_tr: r.get(1)?,
                value_en: r.get(2)?,
                setting_group: r.get(3)?,
                updated_at: r.get(4)?,
            }),
        )
    }

    pub fn get_setting_map(&self, lang: &str) -> HashMap<String, String> {
        let is_en = lang.eq_ignore_ascii_case("en");
        let mut map = HashMap::new();
        for s in self.get_site_settings() {
            let val = if is_en && !s.value_en.is_empty() {
                s.value_en
            } else {
                s.value_tr
            };
            map.insert(s.key, val);
        }
        map
    }

    pub fn upsert_site_setting(&self, setting: &SiteSetting) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now_str = if setting.updated_at.is_empty() {
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            setting.updated_at.clone()
        };
        conn.execute(
            "INSERT OR REPLACE INTO site_settings (key, value_tr, value_en, setting_group, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                setting.key.trim(),
                setting.value_tr,
                if setting.value_en.is_empty() {
                    &setting.value_tr
                } else {
                    &setting.value_en
                },
                if setting.setting_group.is_empty() {
                    "general"
                } else {
                    &setting.setting_group
                },
                now_str
            ],
        )?;
        drop(conn);
        self.refresh_l1_sections(&["site_settings"]);
        Ok(())
    }

    // =========================================================================
    // CENTRAL NOTIFICATION ENGINE PERSISTENCE
    // =========================================================================
    pub fn get_system_notifications(&self, target_email: Option<&str>) -> Vec<SystemNotification> {
        let all = if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                snap.notifications.clone()
            } else {
                self.load_notifications_db()
            }
        } else {
            self.load_notifications_db()
        };

        if let Some(email) = target_email.filter(|e| !e.trim().is_empty()) {
            all.into_iter()
                .filter(|n| {
                    n.target_audience == "all"
                        || n.target_audience == "subscribers"
                        || n.target_email
                            .as_deref()
                            .map(|te| te.eq_ignore_ascii_case(email))
                            .unwrap_or(false)
                })
                .collect()
        } else {
            all
        }
    }

    fn load_notifications_db(&self) -> Vec<SystemNotification> {
        super::query_helpers::query_all(
            self,
            "SELECT id, target_audience, target_email, notification_type, severity,
                    title_tr, title_en, message_tr, message_en, action_url,
                    is_read, is_active_banner, created_at
             FROM system_notifications ORDER BY id DESC LIMIT 100",
            [],
            |r| Ok(SystemNotification {
                id: r.get(0)?,
                target_audience: r.get(1)?,
                target_email: r.get(2)?,
                notification_type: r.get(3)?,
                severity: r.get(4)?,
                title_tr: r.get(5)?,
                title_en: r.get(6)?,
                message_tr: r.get(7)?,
                message_en: r.get(8)?,
                action_url: r.get(9)?,
                is_read: r.get::<_, i32>(10)? != 0,
                is_active_banner: r.get::<_, i32>(11)? != 0,
                created_at: r.get(12)?,
            }),
        )
    }

    pub fn insert_notification(&self, notif: &SystemNotification) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now_str = if notif.created_at.is_empty() {
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            notif.created_at.clone()
        };
        conn.execute(
            "INSERT INTO system_notifications (
                target_audience, target_email, notification_type, severity,
                title_tr, title_en, message_tr, message_en, action_url,
                is_read, is_active_banner, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                notif.target_audience,
                notif.target_email,
                notif.notification_type,
                notif.severity,
                notif.title_tr,
                if notif.title_en.is_empty() {
                    &notif.title_tr
                } else {
                    &notif.title_en
                },
                notif.message_tr,
                if notif.message_en.is_empty() {
                    &notif.message_tr
                } else {
                    &notif.message_en
                },
                notif.action_url,
                if notif.is_read { 1 } else { 0 },
                if notif.is_active_banner { 1 } else { 0 },
                now_str
            ],
        )?;
        let rowid = conn.last_insert_rowid();
        drop(conn);
        self.refresh_l1_sections(&["notifications"]);
        Ok(rowid)
    }

    pub fn get_notification_by_id(&self, id: i64) -> Option<SystemNotification> {
        super::query_helpers::query_one(
            self,
            "SELECT id, target_audience, target_email, notification_type, severity,
                    title_tr, title_en, message_tr, message_en, action_url,
                    is_read, is_active_banner, created_at
             FROM system_notifications WHERE id = ?1",
            params![id],
            |r| Ok(SystemNotification {
                id: r.get(0)?,
                target_audience: r.get(1)?,
                target_email: r.get(2)?,
                notification_type: r.get(3)?,
                severity: r.get(4)?,
                title_tr: r.get(5)?,
                title_en: r.get(6)?,
                message_tr: r.get(7)?,
                message_en: r.get(8)?,
                action_url: r.get(9)?,
                is_read: r.get::<_, i32>(10)? != 0,
                is_active_banner: r.get::<_, i32>(11)? != 0,
                created_at: r.get(12)?,
            }),
        )
    }

    pub fn mark_notification_read(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE system_notifications SET is_read = 1 WHERE id = ?1",
            params![id],
        )?;
        drop(conn);
        self.refresh_l1_sections(&["notifications"]);
        Ok(())
    }

    // =========================================================================
    // CENTRAL WAF & SECURITY AUDIT LOGS
    // =========================================================================
    pub fn get_security_audit_logs(&self) -> Vec<SecurityAuditLog> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, client_ip, request_path, event_type, threat_level, details, blocked, created_at
             FROM security_audit_logs ORDER BY id DESC LIMIT 200",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(SecurityAuditLog {
                id: r.get(0)?,
                client_ip: r.get(1)?,
                request_path: r.get(2)?,
                event_type: r.get(3)?,
                threat_level: r.get(4)?,
                details: r.get(5)?,
                blocked: r.get::<_, i32>(6)? != 0,
                created_at: r.get(7)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn insert_security_audit_log(&self, log: &SecurityAuditLog) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO security_audit_logs (
                client_ip, request_path, event_type, threat_level, details, blocked, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                log.client_ip,
                log.request_path,
                log.event_type,
                log.threat_level,
                log.details,
                if log.blocked { 1 } else { 0 },
                log.created_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    // ── Translation Engine DB Methods ──────────────────────────────────

    /// Load all translations from the `translations` table.
    pub fn load_all_translations(&self) -> Vec<crate::engine::translation_engine::TranslationEntry> {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare(
            "SELECT key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp FROM translations ORDER BY key ASC"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map([], |row| {
            Ok(crate::engine::translation_engine::TranslationEntry {
                key: row.get(0)?,
                value_tr: row.get(1)?,
                value_en: row.get(2)?,
                value_ar: row.get(3)?,
                value_de: row.get(4)?,
                value_ru: row.get(5)?,
                value_fr: row.get(6)?,
                group: row.get(7)?,
            })
        }) {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        };
        rows
    }

    /// Upsert a single translation entry.
    pub fn upsert_translation(&self, entry: &crate::engine::translation_engine::TranslationEntry) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.key,
                entry.value_tr,
                entry.value_en,
                entry.value_ar,
                entry.value_de,
                entry.value_ru,
                entry.value_fr,
                entry.group,
            ],
        )?;
        drop(conn);
        self.refresh_l1_cache();
        Ok(())
    }

    /// Delete a translation by key.
    pub fn delete_translation(&self, key: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute("DELETE FROM translations WHERE key = ?1", rusqlite::params![key])?;
        drop(conn);
        self.refresh_l1_cache();
        Ok(())
    }

    // =========================================================================
    // ADVANCED ADMIN: Content Version History
    // =========================================================================

    pub fn insert_content_version(&self, content_id: i64, title_tr: &str, body_tr: &str, summary_tr: &str, changed_by: &str, change_note: &str) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let max_ver: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM content_versions WHERE content_id = ?1",
            rusqlite::params![content_id], |r| r.get(0))?;
        conn.execute(
            "INSERT INTO content_versions (content_id, version, title_tr, body_tr, summary_tr, changed_by, change_note, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![content_id, max_ver + 1, title_tr, body_tr, summary_tr, changed_by, change_note, now])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_content_versions(&self, content_id: i64) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return vec![] };
        let mut stmt = match conn.prepare("SELECT id, content_id, version, title_tr, change_note, changed_by, created_at FROM content_versions WHERE content_id = ?1 ORDER BY version DESC") { Ok(s) => s, Err(_) => return vec![] };
        let rows = match stmt.query_map(rusqlite::params![content_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "content_id": r.get::<_, i64>(1)?,
                "version": r.get::<_, i64>(2)?, "title_tr": r.get::<_, String>(3)?,
                "change_note": r.get::<_, String>(4)?, "changed_by": r.get::<_, String>(5)?,
                "created_at": r.get::<_, String>(6)?
            }))
        }) { Ok(r) => r, Err(_) => return vec![] };
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_content_versions_paginated(&self, content_id: i64, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM content_versions WHERE content_id = ?1", rusqlite::params![content_id], |r| r.get(0)).unwrap_or(0);
        let mut stmt = match conn.prepare("SELECT id, content_id, version, title_tr, change_note, changed_by, created_at FROM content_versions WHERE content_id = ?1 ORDER BY version DESC LIMIT ?2 OFFSET ?3") { Ok(s) => s, Err(_) => return (vec![], total) };
        let rows = match stmt.query_map(rusqlite::params![content_id, limit, offset], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "content_id": r.get::<_, i64>(1)?,
                "version": r.get::<_, i64>(2)?, "title_tr": r.get::<_, String>(3)?,
                "change_note": r.get::<_, String>(4)?, "changed_by": r.get::<_, String>(5)?,
                "created_at": r.get::<_, String>(6)?
            }))
        }) { Ok(r) => r, Err(_) => return (vec![], total) };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }

    pub fn load_all_translations_paginated(&self, limit: u32, offset: u32) -> (Vec<crate::engine::translation_engine::TranslationEntry>, u64) {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM translations", [], |r| r.get(0)).unwrap_or(0);
        let mut stmt = match conn.prepare("SELECT key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp FROM translations ORDER BY key ASC LIMIT ?1 OFFSET ?2") { Ok(s) => s, Err(_) => return (vec![], total) };
        let rows = match stmt.query_map(rusqlite::params![limit, offset], |row| {
            Ok(crate::engine::translation_engine::TranslationEntry {
                key: row.get(0)?,
                value_tr: row.get(1)?,
                value_en: row.get(2)?,
                value_ar: row.get(3)?,
                value_de: row.get(4)?,
                value_ru: row.get(5)?,
                value_fr: row.get(6)?,
                group: row.get(7)?,
            })
        }) { Ok(r) => r, Err(_) => return (vec![], total) };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }

    // =========================================================================
    // ADVANCED ADMIN: Media Library
    // =========================================================================

    pub fn insert_media_item(&self, filename: &str, original_name: &str, mime_type: &str, file_size: i64, path: &str, uploaded_by: &str) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO media_library (filename, original_name, mime_type, file_size, path, uploaded_by, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![filename, original_name, mime_type, file_size, path, uploaded_by, now])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_media_library(&self) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return vec![] };
        let mut stmt = match conn.prepare("SELECT id, filename, original_name, mime_type, file_size, path, alt_text_tr, alt_text_en, tags, uploaded_by, created_at FROM media_library ORDER BY created_at DESC") { Ok(s) => s, Err(_) => return vec![] };
        let rows = match stmt.query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "filename": r.get::<_, String>(1)?,
                "original_name": r.get::<_, String>(2)?, "mime_type": r.get::<_, String>(3)?,
                "file_size": r.get::<_, i64>(4)?, "path": r.get::<_, String>(5)?,
                "alt_text_tr": r.get::<_, String>(6)?, "alt_text_en": r.get::<_, String>(7)?,
                "tags": r.get::<_, String>(8)?, "uploaded_by": r.get::<_, String>(9)?,
                "created_at": r.get::<_, String>(10)?
            }))
        }) { Ok(r) => r, Err(_) => return vec![] };
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_media_library_paginated(&self, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM media_library", [], |r| r.get(0)).unwrap_or(0);
        let mut stmt = match conn.prepare("SELECT id, filename, original_name, mime_type, file_size, path, alt_text_tr, alt_text_en, tags, uploaded_by, created_at FROM media_library ORDER BY created_at DESC LIMIT ?1 OFFSET ?2") { Ok(s) => s, Err(_) => return (vec![], total) };
        let rows = match stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "filename": r.get::<_, String>(1)?,
                "original_name": r.get::<_, String>(2)?, "mime_type": r.get::<_, String>(3)?,
                "file_size": r.get::<_, i64>(4)?, "path": r.get::<_, String>(5)?,
                "alt_text_tr": r.get::<_, String>(6)?, "alt_text_en": r.get::<_, String>(7)?,
                "tags": r.get::<_, String>(8)?, "uploaded_by": r.get::<_, String>(9)?,
                "created_at": r.get::<_, String>(10)?
            }))
        }) { Ok(r) => r, Err(_) => return (vec![], total) };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }

    // =========================================================================
    // ADVANCED ADMIN: Audit Log
    // =========================================================================

    pub fn insert_audit_log(&self, actor_email: &str, action: &str, entity_type: &str, entity_id: Option<i64>, details_json: &str, ip: &str) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO admin_audit_log (actor_email, action, entity_type, entity_id, details_json, ip_address, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![actor_email, action, entity_type, entity_id, details_json, ip, now])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_audit_logs(&self, limit: i64) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return vec![] };
        let mut stmt = match conn.prepare("SELECT id, actor_email, action, entity_type, entity_id, details_json, ip_address, created_at FROM admin_audit_log ORDER BY created_at DESC LIMIT ?1") { Ok(s) => s, Err(_) => return vec![] };
        let rows = match stmt.query_map(rusqlite::params![limit], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "actor_email": r.get::<_, String>(1)?,
                "action": r.get::<_, String>(2)?, "entity_type": r.get::<_, String>(3)?,
                "entity_id": r.get::<_, Option<i64>>(4)?, "details_json": r.get::<_, String>(5)?,
                "ip_address": r.get::<_, String>(6)?, "created_at": r.get::<_, String>(7)?
            }))
        }) { Ok(r) => r, Err(_) => return vec![] };
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_audit_logs_paginated(&self, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM admin_audit_log", [], |r| r.get(0)).unwrap_or(0);
        let mut stmt = match conn.prepare("SELECT id, actor_email, action, entity_type, entity_id, details_json, ip_address, created_at FROM admin_audit_log ORDER BY created_at DESC LIMIT ?1 OFFSET ?2") { Ok(s) => s, Err(_) => return (vec![], total) };
        let rows = match stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "actor_email": r.get::<_, String>(1)?,
                "action": r.get::<_, String>(2)?, "entity_type": r.get::<_, String>(3)?,
                "entity_id": r.get::<_, Option<i64>>(4)?, "details_json": r.get::<_, String>(5)?,
                "ip_address": r.get::<_, String>(6)?, "created_at": r.get::<_, String>(7)?
            }))
        }) { Ok(r) => r, Err(_) => return (vec![], total) };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }

    // =========================================================================
    // ADVANCED ADMIN: User Roles & Permissions
    // =========================================================================

    pub fn get_user_roles(&self) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return vec![] };
        let mut stmt = match conn.prepare("SELECT id, role_code, name_tr, name_en, permissions_json, is_system FROM user_roles ORDER BY id") { Ok(s) => s, Err(_) => return vec![] };
        let rows = match stmt.query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "role_code": r.get::<_, String>(1)?,
                "name_tr": r.get::<_, String>(2)?, "name_en": r.get::<_, String>(3)?,
                "permissions_json": r.get::<_, String>(4)?, "is_system": r.get::<_, bool>(5)?
            }))
        }) { Ok(r) => r, Err(_) => return vec![] };
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_user_roles_paginated(&self, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM user_roles", [], |r| r.get(0)).unwrap_or(0);
        let mut stmt = match conn.prepare("SELECT id, role_code, name_tr, name_en, permissions_json, is_system FROM user_roles ORDER BY id LIMIT ?1 OFFSET ?2") { Ok(s) => s, Err(_) => return (vec![], total) };
        let rows = match stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "role_code": r.get::<_, String>(1)?,
                "name_tr": r.get::<_, String>(2)?, "name_en": r.get::<_, String>(3)?,
                "permissions_json": r.get::<_, String>(4)?, "is_system": r.get::<_, bool>(5)?
            }))
        }) { Ok(r) => r, Err(_) => return (vec![], total) };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }

    /// Upserts a role into `user_roles`. Empty `name_tr`/`name_en` values will NOT blank out
    /// existing labels on conflict, and `is_system` is preserved for built-in roles.
    pub fn upsert_user_role(&self, role_code: &str, name_tr: &str, name_en: &str, permissions_json: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let code = role_code.trim();
        let insert_tr = if name_tr.trim().is_empty() {
            code
        } else {
            name_tr.trim()
        };
        let insert_en = if name_en.trim().is_empty() {
            insert_tr
        } else {
            name_en.trim()
        };
        let insert_perms = if permissions_json.trim().is_empty() {
            "[]"
        } else {
            permissions_json.trim()
        };

        conn.execute(
            "INSERT INTO user_roles (role_code, name_tr, name_en, permissions_json, is_system)
             VALUES (?1, ?2, ?3, ?4, 0)
             ON CONFLICT(role_code) DO UPDATE SET
                 name_tr = CASE WHEN TRIM(?5) != '' THEN TRIM(?5) ELSE user_roles.name_tr END,
                 name_en = CASE WHEN TRIM(?6) != '' THEN TRIM(?6) ELSE user_roles.name_en END,
                 permissions_json = CASE WHEN TRIM(?7) != '' THEN TRIM(?7) ELSE user_roles.permissions_json END",
            rusqlite::params![code, insert_tr, insert_en, insert_perms, name_tr, name_en, permissions_json],
        )?;
        Ok(())
    }

    /// Assigns a role (`role_code`) to a user by ID and refreshes the L1 `users` cache.
    pub fn update_user_role(&self, user_id: i64, role_code: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE users SET role = ?1 WHERE id = ?2",
            rusqlite::params![role_code.trim(), user_id],
        )?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    /// Deletes a custom user role (`is_system = 0`). System roles (`is_system = 1`) are protected
    /// and return `Ok(false)` without deleting.
    pub fn delete_user_role(&self, role_code: &str) -> rusqlite::Result<bool> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let affected = conn.execute(
            "DELETE FROM user_roles WHERE role_code = ?1 AND is_system = 0",
            rusqlite::params![role_code.trim()],
        )?;
        Ok(affected > 0)
    }

    // =========================================================================
    // ADVANCED ADMIN: Custom Fields
    // =========================================================================

    pub fn get_custom_fields(&self, applies_to: &str) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return vec![] };
        let mut stmt = match conn.prepare("SELECT id, field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to FROM custom_fields WHERE applies_to = ?1 ORDER BY sort_order") { Ok(s) => s, Err(_) => return vec![] };
        let rows = match stmt.query_map(rusqlite::params![applies_to], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "field_key": r.get::<_, String>(1)?,
                "label_tr": r.get::<_, String>(2)?, "label_en": r.get::<_, String>(3)?,
                "field_type": r.get::<_, String>(4)?, "options_json": r.get::<_, String>(5)?,
                "is_required": r.get::<_, bool>(6)?, "sort_order": r.get::<_, i64>(7)?,
                "applies_to": r.get::<_, String>(8)?
            }))
        }) { Ok(r) => r, Err(_) => return vec![] };
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_custom_fields_paginated(&self, applies_to: &str, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM custom_fields WHERE applies_to = ?1", rusqlite::params![applies_to], |r| r.get(0)).unwrap_or(0);
        let mut stmt = match conn.prepare("SELECT id, field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to FROM custom_fields WHERE applies_to = ?1 ORDER BY sort_order LIMIT ?2 OFFSET ?3") { Ok(s) => s, Err(_) => return (vec![], total) };
        let rows = match stmt.query_map(rusqlite::params![applies_to, limit, offset], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "field_key": r.get::<_, String>(1)?,
                "label_tr": r.get::<_, String>(2)?, "label_en": r.get::<_, String>(3)?,
                "field_type": r.get::<_, String>(4)?, "options_json": r.get::<_, String>(5)?,
                "is_required": r.get::<_, bool>(6)?, "sort_order": r.get::<_, i64>(7)?,
                "applies_to": r.get::<_, String>(8)?
            }))
        }) { Ok(r) => r, Err(_) => return (vec![], total) };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }

    pub fn upsert_custom_field(&self, field_key: &str, label_tr: &str, label_en: &str, field_type: &str, options_json: &str, is_required: bool, sort_order: i64, applies_to: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO custom_fields (field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(field_key) DO UPDATE SET label_tr=?2, label_en=?3, field_type=?4, options_json=?5, is_required=?6, sort_order=?7, applies_to=?8",
            rusqlite::params![field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to])?;
        Ok(())
    }

    pub fn delete_custom_field(&self, field_key: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute("DELETE FROM custom_fields WHERE field_key = ?1", rusqlite::params![field_key])?;
        Ok(())
    }

    // =========================================================================
    // ADVANCED ADMIN: Cache Operations
    // =========================================================================

    pub fn insert_cache_operation(&self, cache_key: &str, operation: &str, actor_email: &str, details: &str) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO cache_operations (cache_key, operation, actor_email, details, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![cache_key, operation, actor_email, details, now])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_cache_operations(&self) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return vec![] };
        let mut stmt = match conn.prepare("SELECT id, cache_key, operation, actor_email, details, created_at FROM cache_operations ORDER BY created_at DESC LIMIT 50") { Ok(s) => s, Err(_) => return vec![] };
        let rows = match stmt.query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "cache_key": r.get::<_, String>(1)?,
                "operation": r.get::<_, String>(2)?, "actor_email": r.get::<_, String>(3)?,
                "details": r.get::<_, String>(4)?, "created_at": r.get::<_, String>(5)?
            }))
        }) { Ok(r) => r, Err(_) => return vec![] };
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_cache_operations_paginated(&self, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM cache_operations", [], |r| r.get(0)).unwrap_or(0);
        let mut stmt = match conn.prepare("SELECT id, cache_key, operation, actor_email, details, created_at FROM cache_operations ORDER BY created_at DESC LIMIT ?1 OFFSET ?2") { Ok(s) => s, Err(_) => return (vec![], total) };
        let rows = match stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "cache_key": r.get::<_, String>(1)?,
                "operation": r.get::<_, String>(2)?, "actor_email": r.get::<_, String>(3)?,
                "details": r.get::<_, String>(4)?, "created_at": r.get::<_, String>(5)?
            }))
        }) { Ok(r) => r, Err(_) => return (vec![], total) };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }

    // =========================================================================
    // ADVANCED ADMIN: System Health
    // =========================================================================

    pub fn get_system_health(&self) -> serde_json::Value {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return serde_json::json!({"status": "error", "message": "DB connection failed"}) };
        let content_count: i64 = conn.query_row("SELECT COUNT(*) FROM contents", [], |r| r.get(0)).unwrap_or(0);
        let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap_or(0);
        let event_count: i64 = conn.query_row("SELECT COUNT(*) FROM analytics_events", [], |r| r.get(0)).unwrap_or(0);
        let ad_count: i64 = conn.query_row("SELECT COUNT(*) FROM ad_campaigns", [], |r| r.get(0)).unwrap_or(0);
        let category_count: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0)).unwrap_or(0);
        let issue_count: i64 = conn.query_row("SELECT COUNT(*) FROM magazine_issues", [], |r| r.get(0)).unwrap_or(0);
        let order_count: i64 = conn.query_row("SELECT COUNT(*) FROM order_invoices", [], |r| r.get(0)).unwrap_or(0);
        let db_size: i64 = conn.query_row("SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()", [], |r| r.get(0)).unwrap_or(0);
        serde_json::json!({
            "status": "healthy",
            "db_size_bytes": db_size,
            "tables": {
                "contents": content_count,
                "users": user_count,
                "analytics_events": event_count,
                "ads": ad_count,
                "categories": category_count,
                "issues": issue_count,
                "orders": order_count
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }

    // =========================================================================
    // FAZ 3: DYNAMIC SOURCE GROUPS (replaces hardcoded slugs in search_engine)
    // =========================================================================

    /// Returns distinct source_group values with their display labels from site_settings.
    /// Each entry: (code, label_tr, label_en). Falls back to code-as-label if no setting exists.
    pub fn get_distinct_source_groups(&self) -> Vec<(String, String, String)> {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return vec![] };
        let mut stmt = match conn.prepare(
            "SELECT DISTINCT source_group FROM contents WHERE source_group IS NOT NULL AND source_group != '' ORDER BY source_group"
        ) { Ok(s) => s, Err(_) => return vec![] };
        let codes: Vec<String> = match stmt.query_map([], |r| r.get::<_, String>(0)) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => return vec![],
        };
        // Load label overrides from site_settings (key pattern: source_label.{code}.tr / .en)
        let settings = self.load_site_settings_db();
        codes.into_iter().map(|code| {
            let tr_key = format!("source_label.{}.tr", code);
            let en_key = format!("source_label.{}.en", code);
            let label_tr = settings.iter().find(|s| s.key == tr_key).map(|s| s.value_tr.clone()).unwrap_or_else(|| code.clone());
            let label_en = settings.iter().find(|s| s.key == en_key).map(|s| s.value_en.clone()).unwrap_or_else(|| code.clone());
            (code, label_tr, label_en)
        }).collect()
    }

    // =========================================================================
    // FAZ 2: %100 SQL MERKEZİLEŞTİRME — Translation Seed Methods
    // =========================================================================

    /// Returns the total number of translation records in the database.
    pub fn count_translations(&self) -> Result<i64, rusqlite::Error> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.query_row("SELECT COUNT(*) FROM translations", [], |r| r.get(0))
    }

    /// Seeds default UI translations into the database if the table is empty.
    /// Centralized replacement for translation_engine.rs raw SQL.
    pub fn seed_default_translations(&self) -> Result<(), rusqlite::Error> {
        let count = self.count_translations()?;
        if count > 0 {
            return Ok(());
        }

        let defaults: Vec<(&str, &str, &str, &str)> = vec![
            ("nav.home", "Ana Sayfa", "Home", "navigation"),
            ("nav.search", "Arama", "Search", "navigation"),
            ("nav.account", "Hesabım", "My Account", "navigation"),
            ("nav.login", "Giriş yap", "Log in", "navigation"),
            ("nav.register", "Kayıt ol", "Register", "navigation"),
            ("nav.logout", "Çıkış", "Log out", "navigation"),
            ("nav.subscriptions", "Abonelik ve Satış", "Subscriptions & Store", "navigation"),
            ("nav.edergi", "E-Dergi", "E-Magazine", "navigation"),
            ("nav.corporate", "Kurumsal", "Corporate", "navigation"),
            ("search.eyebrow", "MSI ARŞİVİNİ KEŞFEDİN", "EXPLORE THE MSI ARCHIVE", "search"),
            ("search.heading", "Bir konunun peşinden gidin.", "Follow a topic through.", "search"),
            ("search.description", "Haberler, analizler, söyleşiler ve dergi arşivi. Hepsi aynı yerde.", "News, analyses, interviews and magazine archive. All in one place.", "search"),
            ("search.query_label", "Arama kelimesi", "Search term", "search"),
            ("search.placeholder", "Konu, kurum veya teknoloji…", "Topic, institution or technology…", "search"),
            ("search.source_label", "Yayın", "Source", "search"),
            ("search.sort_label", "Sıralama", "Sort", "search"),
            ("search.submit", "Ara →", "Search →", "search"),
            ("search.results_summary", "{count} sonuç · {showing} içerik gösteriliyor", "{count} results · showing {showing} items", "search"),
            ("search.clear_filters", "Filtreleri temizle", "Clear filters", "search"),
            ("search.filter_categories", "Kategoriler", "Categories", "search"),
            ("search.filter_tags", "İlgi alanları", "Interest areas", "search"),
            ("search.filter_issues", "Dergi sayıları", "Magazine issues", "search"),
            ("search.all_categories", "Tüm kategoriler", "All categories", "search"),
            ("search.all_topics", "Tüm konular", "All topics", "search"),
            ("search.all_issues", "Tüm sayılar", "All issues", "search"),
            ("search.no_results_title", "Sonuç bulunamadı.", "No results found.", "search"),
            ("search.no_results_desc", "Farklı bir kelime deneyin veya seçtiğiniz filtreleri temizleyin.", "Try a different keyword or clear your filters.", "search"),
            ("search.sort.latest", "En yeni", "Latest", "search"),
            ("search.sort.popular", "En çok okunan", "Most read", "search"),
            ("search.sort.true_read", "Okuma ilgisine göre", "By reading engagement", "search"),
            ("search.source.all", "Tüm yayınlar", "All sources", "search"),
            ("search.source.dergi", "Dergi içerikleri", "Magazine content", "search"),
            ("search.source.gunluk", "Günlük haberler", "Daily news", "search"),
            ("search.source.online", "Online içerikler", "Online content", "search"),
            ("search.issue_prefix", "Sayı", "Issue", "search"),
            ("category.empty", "Bu kategoride henüz içerik bulunmuyor.", "No content in this category yet.", "category"),
            ("category.explore_all", "Tüm içerikleri keşfedin →", "Explore all content →", "category"),
            ("category.prev_page", "← Önceki", "← Previous", "category"),
            ("category.next_page", "Sonraki →", "Next →", "category"),
            ("category.pagination_label", "Kategori sayfaları", "Category pages", "category"),
            ("category.topic_filter_label", "İlgi alanı", "Interest area", "category"),
            ("category.all_topics", "Tüm konular", "All topics", "category"),
            ("category.filter_button", "Filtrele", "Filter", "category"),
            ("category.reset_filter", "Sıfırla", "Reset", "category"),
            ("category.content_count", "{count} içerik", "{count} items", "category"),
            ("category.articles_suffix", "içerik", "articles", "category"),
            ("category.sub_hub_label", "FUAR, KONFERANS VE ETKİNLİK ÖZEL DOSYALARI", "EXHIBITION & CONFERENCE SPECIAL DOSSIERS", "category"),
            ("error.404_badge", "SAYFA BULUNAMADI", "PAGE NOT FOUND", "errors"),
            ("error.404_title", "Aradığınız Sayfa Bulunamadı", "Page Not Found", "errors"),
            ("error.404_desc", "İstediğiniz içerik taşınmış olabilir veya adres hatalı girilmiş olabilir.", "The content you requested may have been moved or the address may be incorrect.", "errors"),
            ("error.404_home", "Ana Sayfaya Dön", "Return to Home", "errors"),
            ("error.404_search", "İçerikleri ara", "Search content", "errors"),
            ("error.404_page_title", "404 Sayfa Bulunamadı", "404 Page Not Found", "errors"),
            ("account.eyebrow", "OKUYUCU MERKEZİ", "READER CENTER", "account"),
            ("account.heading_line1", "Okuma yolculuğunuz", "Your reading journey", "account"),
            ("account.heading_line2", "burada devam ediyor.", "continues here.", "account"),
            ("account.description", "Aboneliğinizi, yayınlarınızı ve bildirimlerinizi tek yerden takip edin.", "Manage your subscription, publications and notifications in one place.", "account"),
            ("account.login_cta", "Giriş yap / Hesap oluştur →", "Log in / Create account →", "account"),
            ("account.subscription_link", "Abonelik seçenekleri", "Subscription options", "account"),
            ("account.page_title_guest", "Hesabım", "My Account", "account"),
            ("account.page_title_logged_in", "Okuyucu & Abone Kontrol Merkezi", "Reader & Subscriber Control Center", "account"),
            ("paywall.subscriber_only_en", "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.", "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.", "paywall"),
            ("paywall.subscriber_only_tr", "Abonelere Özel İçerik: Bu analizin tamamı sadece MSI Dergisi abonelerine açıktır. Okumaya devam etmek için abone olabilir veya bu makaleyi tekil olarak ({price} TL) satın alabilirsiniz. (.gov.tr ve .tsk.tr kurumsal e-postalarına ücretsizdir.)", "Abonelere Özel İçerik: Bu analizin tamamı sadece MSI Dergisi abonelerine açıktır. Okumaya devam etmek için abone olabilir veya bu makaleyi tekil olarak ({price} TL) satın alabilirsiniz. (.gov.tr ve .tsk.tr kurumsal e-postalarına ücretsizdir.)", "paywall"),
            ("auth.admin_required", "Bu işlem için yönetici hesabınızla giriş yapın.", "Please log in with your administrator account for this operation.", "auth"),
            ("seo.default_description", "MSI Dergisi & MSI TDR — Savunma, Havacılık ve Güvenlik Teknolojileri Referans Platformu (ISSN 1306-0309)", "MSI Magazine & MSI TDR — Defense, Aerospace and Security Technologies Reference Platform (ISSN 1306-0309)", "seo"),
            ("seo.search_page_title", "Arama ve keşif", "Search and discovery", "seo"),
            ("plugin.description_human", "Türkiye'nin savunma ve havacılık sanayii dergisi MSI Dergisi'nin güncel haberleri, analizleri ve dergi içerikleri.", "Latest news, analyses and magazine content from MSI Dergisi, Turkey's defense and aerospace industry magazine.", "plugin"),
            ("drm.public_license", "KAMU / ABONE E-DERGI LISANSI", "PUBLIC / SUBSCRIBER E-MAGAZINE LICENSE", "drm"),
            ("drm.dossier_header", "BU SAYIDA YER ALAN DOSYALAR", "DOSSIERS IN THIS ISSUE", "drm"),
            ("drm.page_prefix", "Sayfa", "Page", "drm"),
            ("drm.crypto_signature", "Kriptografik Imza", "Cryptographic Signature", "drm"),
            ("drm.selected_pages", "Secili Sayfalar", "Selected Pages", "drm"),
            ("drm.full_archive", "Tam Sayi Arsiv Paketi", "Full Issue Archive Package", "drm"),
            ("drm.pages_word", "Sayfa", "Pages", "drm"),
            ("ads.conflict_warning", "UYARI: '{slot}' slotunda veya [{tags}] hedef etiketlerinde maksimum kampanya kotası ({current}/{limit}) aşıldı{suffix}. Çakışan aktif kampanyalar: {conflicts}. Yine de mevcut kampanyaların yanına eklemek istiyor musunuz?", "WARNING: Maximum campaign quota ({current}/{limit}) exceeded in slot '{slot}' or target tags [{tags}]{suffix}. Conflicting active campaigns: {conflicts}. Do you still want to add alongside existing campaigns?", "ads"),
            ("ads.no_conflict", "Çakışma tespit edilmedi{suffix}. Reklam alanı ve SSB anahtar kelime kotası uygun.", "No conflict detected{suffix}. Ad slot and SSB keyword quota available.", "ads"),
            ("ads.date_range", " [Tarih Aralığı: {start} → {end}, Mod: {mode}]", " [Date Range: {start} → {end}, Mode: {mode}]", "ads"),
            ("ads.fallback_dimensions", "970 × 90 STANDART VARSAYILAN VİTRİN", "970 × 90 STANDARD DEFAULT SHOWCASE", "ads"),
            ("ads.fallback_title_tr", "Hava-Hava, Kara ve Deniz Platformlarında Telifli Havacılık & Savunma Fotoğrafçılığı", "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms", "ads"),
            ("ads.fallback_title_en", "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms", "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms", "ads"),
            ("ads.fallback_desc_tr", "Onur Kurç & Tayfun Yaşar yönetiminde Remove Before Photos yüksek çözünürlüklü telifli savunma görsel arşivi ve MSI Dergisi kurumsal abonelik çözümleri.", "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.", "ads"),
            ("ads.fallback_desc_en", "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.", "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.", "ads"),
            ("tier.public_subscriber", "Kamu E-Dergi Abonesi (Ücretsiz)", "Public E-Magazine Subscriber (Free)", "subscriptions"),
            ("tier.academic", "Akademik / Öğrenci Okur", "Academic / Student Reader", "subscriptions"),
            ("tier.standard", "Standart Kayıtlı Okur", "Standard Registered Reader", "subscriptions"),
            ("analytics.campaign_prefix", "Kampanya #", "Campaign #", "analytics"),
        ];

        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        for (key, tr, en, group) in &defaults {
            conn.execute(
                "INSERT OR IGNORE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp) VALUES (?1, ?2, ?3, '', '', '', '', ?4)",
                rusqlite::params![key, tr, en, group],
            )?;
        }
        Ok(())
    }

    /// Seeds API-specific translation keys into the database.
    /// Centralized replacement for api_response.rs raw SQL.
    pub fn seed_api_translations(&self) -> Result<(), rusqlite::Error> {
        let entries: Vec<(&str, &str, &str)> = vec![
            ("api.error.content_not_found", "İçerik bulunamadı", "Content not found"),
            ("api.error.invalid_email", "Geçerli bir kurumsal e-posta adresi giriniz.", "Please enter a valid corporate email address."),
            ("api.error.email_already_exists", "Bu e-posta ile bir hesap zaten var. Lütfen giriş yapın.", "An account with this email already exists. Please log in."),
            ("api.error.registration_fields_required", "Ad soyad, en az 8 karakterli şifre ve KVKK onayı gerekli.", "Full name, minimum 8-character password and KVKK consent are required."),
            ("api.error.kvkk_consent_required", "Ad soyad, en az 8 karakterli şifre ve KVKK aydınlatma beyanı gerekli.", "Full name, minimum 8-character password and KVKK disclosure statement are required."),
            ("api.error.login_failed", "E-posta veya şifre hatalı.", "Invalid email or password."),
            ("api.error.auth_required_pdf", "E-Dergi PDF indirmek için abone girişi yapınız.", "Subscriber login required to download E-Magazine PDF."),
            ("api.error.subscriber_access_required", "PDF indirme yetkisi aktif MSI Dergisi abonelerine ve onaylı kamu personeline özeldir.", "PDF download access is exclusive to active MSI Magazine subscribers and verified government personnel."),
            ("api.error.issue_not_found", "İstenen dergi sayısı bulunamadı.", "Requested magazine issue not found."),
            ("api.error.checkout_fields_required", "Geçerli ad soyad ve e-posta adresi zorunludur.", "Valid full name and email address are required."),
            ("api.error.distance_sales_consent_required", "6502 sayılı Kanun gereğince Ön Bilgilendirme Formu ve Mesafeli Satış Sözleşmesi onayı zorunludur.", "Preliminary Information Form and Distance Sales Agreement consent required per Law No. 6502."),
            ("api.error.plan_not_found", "Seçilen abonelik planı veya yayın katalogda bulunamadı.", "Selected subscription plan or publication not found in catalog."),
            ("api.error.invalid_file_type", "Yalnızca .webp, .png, .jpg, .svg, .pdf ve .mp4 dosyaları yüklenebilir.", "Only .webp, .png, .jpg, .svg, .pdf and .mp4 files can be uploaded."),
            ("api.error.file_too_large", "Dosya boş veya 25 MB sınırını aşıyor.", "File is empty or exceeds the 25 MB limit."),
            ("api.error.upload_no_field", "Yüklenecek dosya alanı bulunamadı.", "No file field found for upload."),
            ("api.error.server_error_generic", "Bir sunucu hatası oluştu. Lütfen tekrar deneyin.", "A server error occurred. Please try again."),
            ("api.error.server_error_save", "Kaydedilirken bir sunucu hatası oluştu.", "A server error occurred while saving."),
            ("api.error.server_error_upload", "Dosya yükleme sırasında bir sunucu hatası oluştu.", "A server error occurred during file upload."),
            ("api.error.server_error_file_write", "Dosya kaydedilirken bir sunucu hatası oluştu.", "A server error occurred while saving the file."),
            ("api.error.server_error_order", "Sipariş kaydı oluşturulurken bir sunucu hatası oluştu. Lütfen tekrar deneyin.", "A server error occurred while creating the order record. Please try again."),
            ("api.error.server_error_ad_save", "Reklam kaydedilirken bir sunucu hatası oluştu.", "A server error occurred while saving the advertisement."),
            ("api.error.bad_request", "Geçersiz istek.", "Bad request."),
            ("api.error.validation_error", "Doğrulama hatası. Lütfen girdilerinizi kontrol edin.", "Validation error. Please check your input."),
            ("api.error.auth_required", "Kimlik doğrulama gerekli.", "Authentication required."),
            ("api.error.payment_error", "Ödeme işlemi sırasında bir hata oluştu. Lütfen tekrar deneyin.", "Payment processing error. Please try again."),
        ];

        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        for (key, tr, en) in &entries {
            conn.execute(
                "INSERT OR IGNORE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp) VALUES (?1, ?2, ?3, '', '', '', '', 'api')",
                rusqlite::params![key, tr, en],
            )?;
        }
        Ok(())
    }

    // =========================================================================
    // PHASE 4: EDITORIAL APPROVAL WORKFLOW SYSTEM
    // =========================================================================

    /// Creates a new approval request for a content item and transitions its status to `pending_review`.
    pub fn create_approval_request(&self, content_id: i64, requested_by: i64) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO approval_requests (content_id, requested_by, requested_at, status) VALUES (?1, ?2, ?3, 'pending')",
            rusqlite::params![content_id, requested_by, now],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "UPDATE contents SET status = 'pending_review' WHERE id = ?1",
            rusqlite::params![content_id],
        )?;
        tx.commit()?;
        drop(conn);
        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);
        Ok(id)
    }

    /// Lists all pending approval requests joined with content metadata and requester info.
    pub fn list_pending_approvals(&self) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut stmt = match conn.prepare(
            "SELECT ar.id, ar.content_id, ar.requested_by, ar.requested_at, ar.status,
                    c.title_tr, c.author_name, c.content_type, c.slug,
                    COALESCE(u.email, ''), COALESCE(u.full_name, '')
             FROM approval_requests ar
             JOIN contents c ON c.id = ar.content_id
             LEFT JOIN users u ON u.id = ar.requested_by
             WHERE ar.status = 'pending'
             ORDER BY ar.requested_at DESC",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "content_id": r.get::<_, i64>(1)?,
                "requested_by": r.get::<_, i64>(2)?,
                "requested_at": r.get::<_, String>(3)?,
                "status": r.get::<_, String>(4)?,
                "title_tr": r.get::<_, String>(5)?,
                "author_name": r.get::<_, String>(6)?,
                "content_type": r.get::<_, String>(7)?,
                "slug": r.get::<_, String>(8)?,
                "requested_by_email": r.get::<_, String>(9)?,
                "requested_by_name": r.get::<_, String>(10)?
            }))
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    /// Updates an approval request (`approved` / `rejected`) and transitions the linked content item status.
    pub fn update_approval_status(
        &self,
        id: i64,
        status: &str,
        reviewed_by: i64,
        review_note: Option<&str>,
    ) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE approval_requests SET status = ?1, reviewed_by = ?2, reviewed_at = ?3, review_note = ?4 WHERE id = ?5",
            rusqlite::params![status, reviewed_by, now, review_note.unwrap_or(""), id],
        )?;
        let content_id: i64 = tx.query_row(
            "SELECT content_id FROM approval_requests WHERE id = ?1",
            rusqlite::params![id],
            |r| r.get(0),
        )?;
        let content_status = match status {
            "approved" => "published",
            "rejected" => "rejected",
            _ => "pending_review",
        };
        tx.execute(
            "UPDATE contents SET status = ?1 WHERE id = ?2",
            rusqlite::params![content_status, content_id],
        )?;
        tx.commit()?;
        drop(conn);
        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);
        Ok(())
    }

    /// Updates the editorial status of a content item directly and invalidates L1/L2 content caches.
    pub fn update_content_status(&self, content_id: i64, status: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE contents SET status = ?1 WHERE id = ?2",
            rusqlite::params![status.trim(), content_id],
        )?;
        drop(conn);
        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);
        Ok(())
    }

    // =========================================================================
    // MONITORING SYSTEM DB METHODS (007_monitoring_schema.sql)
    // =========================================================================

    /// Insert a monitoring snapshot (`system`, `domain`, `traffic`, etc.) as a JSON payload.
    pub fn insert_monitoring_snapshot(&self, snapshot_type: &str, payload_json: &str) -> Result<(), String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO monitoring_snapshots (snapshot_type, payload_json, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![snapshot_type, payload_json, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get the latest monitoring snapshot JSON payload for a given `snapshot_type`.
    pub fn get_latest_monitoring_snapshot(&self, snapshot_type: &str) -> Option<String> {
        let conn = self.pool.get().ok()?;
        let mut stmt = conn
            .prepare(
                "SELECT payload_json FROM monitoring_snapshots WHERE snapshot_type = ?1 ORDER BY created_at DESC LIMIT 1",
            )
            .ok()?;
        stmt.query_row(rusqlite::params![snapshot_type], |row| row.get(0))
            .ok()
    }

    /// Insert an attack event into `monitoring_attack_events`.
    pub fn insert_attack_event(
        &self,
        source_ip: &str,
        attack_type: &str,
        target_service: &str,
        details: &str,
        severity: &str,
    ) -> Result<(), String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO monitoring_attack_events (source_ip, attack_type, target_service, details, severity, detected_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![source_ip, attack_type, target_service, details, severity, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get recent attack events ordered by `detected_at DESC`.
    pub fn get_recent_attack_events(&self, limit: i64) -> Vec<crate::engine::monitoring_engine::AttackEvent> {
        let conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare(
            "SELECT id, source_ip, attack_type, target_service, details, severity, detected_at
             FROM monitoring_attack_events ORDER BY detected_at DESC LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map(rusqlite::params![limit], |row| {
            Ok(crate::engine::monitoring_engine::AttackEvent {
                id: Some(row.get(0)?),
                source_ip: row.get(1)?,
                attack_type: row.get(2)?,
                target_service: row.get(3)?,
                details: row.get(4)?,
                severity: row.get(5)?,
                detected_at: row.get(6)?,
            })
        }) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    /// Upsert a discovered vhost domain into `monitored_domains`.
    pub fn upsert_monitored_domain(
        &self,
        domain: &str,
        vhost_path: &str,
        access_log: &str,
        error_log: &str,
    ) -> Result<(), String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO monitored_domains (domain, vhost_path, access_log_path, error_log_path)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(domain) DO UPDATE SET
                 vhost_path = excluded.vhost_path,
                 access_log_path = excluded.access_log_path,
                 error_log_path = excluded.error_log_path",
            rusqlite::params![domain, vhost_path, access_log, error_log],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Prune monitoring snapshots and attack events older than `retention_days`.
    pub fn prune_monitoring_data(&self, retention_days: u32) -> Result<usize, String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_str = cutoff.to_rfc3339();
        let deleted_snapshots = conn
            .execute(
                "DELETE FROM monitoring_snapshots WHERE created_at < ?1",
                rusqlite::params![cutoff_str],
            )
            .map_err(|e| e.to_string())?;
        let deleted_attacks = conn
            .execute(
                "DELETE FROM monitoring_attack_events WHERE detected_at < ?1",
                rusqlite::params![cutoff_str],
            )
            .map_err(|e| e.to_string())?;
        Ok(deleted_snapshots + deleted_attacks)
    }

    // =========================================================================
    // PASSWORD RESET TOKEN METHODS (008_password_reset.sql — SHA-256 + fallback)
    // =========================================================================

    pub fn create_password_reset_token(
        &self,
        user_id: i64,
        token: &str,
        expires_at: &str,
    ) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        let token_hash = crate::utils::crypto::sha256_hex(token.trim());
        conn.execute(
            "INSERT INTO password_reset_tokens (user_id, token, expires_at) VALUES (?1, ?2, ?3)",
            params![user_id, token_hash, expires_at],
        )
        .map_err(|e| format!("Failed to create reset token: {}", e))?;
        Ok(())
    }

    pub fn find_password_reset_token(&self, token: &str) -> Option<(i64, i64, String)> {
        let raw = token.trim();
        if raw.is_empty() {
            return None;
        }
        let token_hash = crate::utils::crypto::sha256_hex(raw);
        super::query_helpers::query_one(
            self,
            "SELECT id, user_id, expires_at FROM password_reset_tokens
             WHERE (token = ?1 OR token = ?2) AND used = 0
             ORDER BY id DESC LIMIT 1",
            params![token_hash, raw],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
    }

    pub fn mark_reset_token_used(&self, token_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE password_reset_tokens SET used = 1 WHERE id = ?1",
            params![token_id],
        )
        .map_err(|e| format!("Failed to mark reset token used: {}", e))?;
        Ok(())
    }

    pub fn invalidate_user_reset_tokens(&self, user_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE password_reset_tokens SET used = 1 WHERE user_id = ?1 AND used = 0",
            params![user_id],
        )
        .map_err(|e| format!("Failed to invalidate user reset tokens: {}", e))?;
        Ok(())
    }

    // =========================================================================
    // KVKK/GDPR Unsubscribe Token Methods (010_unsubscribe_tokens.sql — SHA-256 + fallback)
    // =========================================================================

    pub fn create_unsubscribe_token(&self, user_id: i64, token: &str) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        let token_hash = crate::utils::crypto::sha256_hex(token.trim());
        conn.execute(
            "INSERT INTO unsubscribe_tokens (user_id, token) VALUES (?1, ?2)",
            params![user_id, token_hash],
        )
        .map_err(|e| format!("Failed to create unsubscribe token: {}", e))?;
        Ok(())
    }

    pub fn find_unsubscribe_token(&self, token: &str) -> Option<(i64, i64)> {
        let raw = token.trim();
        if raw.is_empty() {
            return None;
        }
        let token_hash = crate::utils::crypto::sha256_hex(raw);
        super::query_helpers::query_one(
            self,
            "SELECT id, user_id FROM unsubscribe_tokens
             WHERE (token = ?1 OR token = ?2) AND used = 0
             ORDER BY id DESC LIMIT 1",
            params![token_hash, raw],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
    }

    pub fn mark_unsubscribe_token_used(&self, token_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE unsubscribe_tokens SET used = 1 WHERE id = ?1",
            params![token_id],
        )
        .map_err(|e| format!("Failed to mark unsubscribe token used: {}", e))?;
        Ok(())
    }

    pub fn opt_out_marketing_emails(&self, user_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE users SET marketing_emails_opted_in = 0, email_consent = 0 WHERE id = ?1",
            params![user_id],
        )
        .map_err(|e| format!("Failed to opt out marketing emails: {}", e))?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    pub fn opt_in_marketing_emails(&self, user_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE users SET marketing_emails_opted_in = 1, email_consent = 1 WHERE id = ?1",
            params![user_id],
        )
        .map_err(|e| format!("Failed to opt in marketing emails: {}", e))?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    pub fn get_user_email_preference(&self, user_id: i64) -> bool {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return true,
        };
        conn.query_row(
            "SELECT CASE WHEN COALESCE(marketing_emails_opted_in, 1) != 0 AND COALESCE(email_consent, 1) != 0 THEN 1 ELSE 0 END FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get::<_, i32>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(true)
    }

    // =========================================================================
    // Email Verification Token Methods (009_email_verification.sql — SHA-256 + fallback)
    // =========================================================================

    pub fn create_email_verification_token(
        &self,
        user_id: i64,
        token: &str,
        expires_at: &str,
    ) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        let token_hash = crate::utils::crypto::sha256_hex(token.trim());
        conn.execute(
            "INSERT INTO email_verification_tokens (user_id, token, expires_at) VALUES (?1, ?2, ?3)",
            params![user_id, token_hash, expires_at],
        )
        .map_err(|e| format!("Failed to create verification token: {}", e))?;
        Ok(())
    }

    pub fn find_email_verification_token(&self, token: &str) -> Option<(i64, i64, String)> {
        let raw = token.trim();
        if raw.is_empty() {
            return None;
        }
        let token_hash = crate::utils::crypto::sha256_hex(raw);
        super::query_helpers::query_one(
            self,
            "SELECT id, user_id, expires_at FROM email_verification_tokens
             WHERE (token = ?1 OR token = ?2) AND verified = 0
             ORDER BY id DESC LIMIT 1",
            params![token_hash, raw],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
    }

    pub fn mark_email_verified(&self, user_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE users SET email_verified = 1 WHERE id = ?1",
            params![user_id],
        )
        .map_err(|e| format!("Failed to mark email verified: {}", e))?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    pub fn mark_verification_token_used(&self, token_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE email_verification_tokens SET verified = 1 WHERE id = ?1",
            params![token_id],
        )
        .map_err(|e| format!("Failed to mark verification token used: {}", e))?;
        Ok(())
    }

    /// Returns the list of permission strings for a given `role_code` from the `user_roles` table.
    pub fn get_role_permissions(&self, role_code: &str) -> Option<Vec<String>> {
        let conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return None,
        };
        let json_str: String = match conn.query_row(
            "SELECT permissions_json FROM user_roles WHERE LOWER(role_code) = LOWER(?1)",
            params![role_code.trim()],
            |r| r.get(0),
        ) {
            Ok(s) => s,
            Err(_) => return None,
        };
        serde_json::from_str::<Vec<String>>(&json_str).ok()
    }
}

#[cfg(test)]
mod fts_tests {
    use super::*;

    #[test]
    fn test_sanitize_fts_query_basic() {
        let result = Database::sanitize_fts_query("hello world");
        assert_eq!(result, "\"hello\"* OR \"world\"*");
    }

    #[test]
    fn test_sanitize_fts_query_turkish_chars() {
        let result = Database::sanitize_fts_query("uçak gemisi");
        assert_eq!(result, "\"uçak\"* OR \"gemisi\"*");
    }

    #[test]
    fn test_sanitize_fts_query_strips_punctuation() {
        let result = Database::sanitize_fts_query("hello! @world# test...");
        assert_eq!(result, "\"hello\"* OR \"world\"* OR \"test\"*");
    }

    #[test]
    fn test_sanitize_fts_query_quotes_removed() {
        let result = Database::sanitize_fts_query("\"quoted\" 'single'");
        assert_eq!(result, "\"quoted\"* OR \"single\"*");
    }

    #[test]
    fn test_sanitize_fts_query_empty_input() {
        assert_eq!(Database::sanitize_fts_query(""), "");
        assert_eq!(Database::sanitize_fts_query("   "), "");
    }

    #[test]
    fn test_sanitize_fts_query_only_special_chars() {
        assert_eq!(Database::sanitize_fts_query("!@#$%^&*()"), "");
    }

    #[test]
    fn test_sanitize_fts_query_with_join_and() {
        let result = Database::sanitize_fts_query_with_join("foo bar baz", " AND ");
        assert_eq!(result, "\"foo\"* AND \"bar\"* AND \"baz\"*");
    }

    #[test]
    fn test_sanitize_fts_query_underscore_preserved() {
        let result = Database::sanitize_fts_query("my_test query");
        assert_eq!(result, "\"my_test\"* OR \"query\"*");
    }
}

