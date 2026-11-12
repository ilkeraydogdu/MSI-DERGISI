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

    fn enrich_issue_spreads_from_contents(
        issues: &mut [MagazineIssue],
        contents: &[ContentItem],
    ) {
        for issue in issues.iter_mut() {
            if issue.spreads.is_empty() {
                issue.spreads = contents
                    .iter()
                    .filter(|c| c.issue_number == Some(issue.issue_number))
                    .take(8)
                    .enumerate()
                    .map(|(idx, c)| {
                        let start_p = 8 + idx * 10;
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
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic
             FROM categories ORDER BY sort_order ASC, id ASC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(CategoryNode {
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
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
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
        self.refresh_l1_cache();
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
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, code, name_tr, name_en, category_group, sort_order
             FROM ssb_tags ORDER BY sort_order ASC, id ASC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(SsbInterestTag {
                id: r.get(0)?,
                code: r.get(1)?,
                name_tr: r.get(2)?,
                name_en: r.get(3)?,
                category_group: r.get(4)?,
                sort_order: r.get(5)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
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
        self.refresh_l1_cache();
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
                    view_count, true_read_count
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

    pub fn search_contents(&self, query: &str) -> Vec<ContentItem> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.get_all_contents();
        }
        self.get_all_contents()
            .into_iter()
            .filter(|c| {
                c.title_tr.to_lowercase().contains(&q)
                    || c.title_en.to_lowercase().contains(&q)
                    || c.summary_tr.to_lowercase().contains(&q)
                    || c.summary_en.to_lowercase().contains(&q)
                    || c.body_tr.to_lowercase().contains(&q)
                    || c.author_name.to_lowercase().contains(&q)
                    || c.ssb_tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || c.placements.iter().any(|p| p.to_lowercase().contains(&q))
            })
            .collect()
    }

    pub fn upsert_content(&self, item: &ContentItem) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        if item.id > 0 {
            conn.execute(
                "UPDATE contents SET
                    slug = ?1, title_tr = ?2, title_en = ?3, summary_tr = ?4, summary_en = ?5,
                    body_tr = ?6, body_en = ?7, content_type = ?8, source_group = ?9,
                    cover_image = ?10, author_name = ?11, author_title = ?12, issue_number = ?13,
                    published_at = ?14, read_time_min = ?15, visibility_level = ?16,
                    single_price_try = ?17, is_sponsored = ?18, sponsor_brand = ?19,
                    sponsor_legal_label = ?20, is_featured = ?21, video_url = ?22
                 WHERE id = ?23",
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
                    item.id
                ],
            )?;
        } else {
            conn.execute(
                "INSERT OR REPLACE INTO contents (
                    slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                    content_type, source_group, cover_image, author_name, author_title,
                    issue_number, published_at, read_time_min, visibility_level, single_price_try,
                    is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                    view_count, true_read_count
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                    ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17,
                    ?18, ?19, ?20, ?21, ?22,
                    ?23, ?24
                )",
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
                    item.true_read_count.max(1)
                ],
            )?;
        }

        let content_id: i64 = conn.query_row(
            "SELECT id FROM contents WHERE slug = ?1",
            params![item.slug],
            |r| r.get(0),
        )?;

        conn.execute(
            "DELETE FROM content_placements WHERE content_id = ?1",
            params![content_id],
        )?;
        for p in &item.placements {
            conn.execute(
                "INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)",
                params![content_id, p],
            )?;
        }
        // Always include primary content_type in placements as well
        conn.execute(
            "INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)",
            params![content_id, item.content_type],
        )?;

        conn.execute(
            "DELETE FROM content_ssb_tags WHERE content_id = ?1",
            params![content_id],
        )?;
        for tag in &item.ssb_tags {
            conn.execute(
                "INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)",
                params![content_id, tag],
            )?;
        }

        drop(conn);
        self.refresh_l1_cache();
        Ok(content_id)
    }

    pub fn delete_content(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute("DELETE FROM contents WHERE id = ?1", params![id])?;
        drop(conn);
        self.refresh_l1_cache();
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
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
                    year, cover_image, is_special_issue, event_name, page_count, dossier_count,
                    summary_tr, summary_en, pdf_path, spreads_json
             FROM magazine_issues
             ORDER BY issue_number DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
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
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_issue_by_number(&self, issue_number: i32) -> Option<MagazineIssue> {
        self.get_all_issues()
            .into_iter()
            .find(|i| i.issue_number == issue_number)
            .or_else(|| self.get_all_issues().into_iter().next())
    }

    fn load_users_db(&self) -> Vec<UserAccount> {
        self.get_all_users()
    }

    pub fn get_all_users(&self) -> Vec<UserAccount> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, email, full_name, role, organization_category, organization_name,
                    job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                    reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                    email_consent, ssb_interests_json, created_at
             FROM users ORDER BY id ASC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
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
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_user_by_email(&self, email: &str) -> Option<(UserAccount, String)> {
        let norm = email.trim().to_lowercase();
        let conn = self.open_conn().ok()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, email, password_hash, full_name, role, organization_category,
                        organization_name, job_title, is_gov_verified, subscription_tier,
                        subscription_expires_at, reminders_halted, kvkk_consent_at,
                        kvkk_consent_ip, kvkk_consent_hash, email_consent, ssb_interests_json, created_at
                 FROM users WHERE LOWER(email) = ?1",
            )
            .ok()?;
        stmt.query_row(params![norm], |r| {
            let ssb_raw: String = r.get(16)?;
            let ssb_interests: Vec<String> = serde_json::from_str(&ssb_raw).unwrap_or_default();
            let user = UserAccount {
                id: r.get(0)?,
                email: r.get(1)?,
                full_name: r.get(3)?,
                role: r.get(4)?,
                organization_category: r.get(5)?,
                organization_name: r.get(6)?,
                job_title: r.get(7)?,
                is_gov_verified: r.get::<_, i32>(8)? != 0,
                subscription_tier: r.get(9)?,
                subscription_expires_at: r.get(10)?,
                reminders_halted: r.get::<_, i32>(11)? != 0,
                kvkk_consent_at: r.get(12)?,
                kvkk_consent_ip: r.get(13)?,
                kvkk_consent_hash: r.get(14)?,
                email_consent: r.get::<_, i32>(15)? != 0,
                ssb_interests,
                created_at: r.get(17)?,
            };
            let pw_hash: String = r.get(2)?;
            Ok((user, pw_hash))
        })
        .ok()
    }

    pub fn get_user_by_id(&self, id: i64) -> Option<UserAccount> {
        self.get_all_users().into_iter().find(|u| u.id == id)
    }

    pub fn create_or_update_user(
        &self,
        user: &UserAccount,
        password_hash: &str,
    ) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let ssb_json =
            serde_json::to_string(&user.ssb_interests).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT OR REPLACE INTO users (
                email, password_hash, full_name, role, organization_category, organization_name,
                job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                email_consent, ssb_interests_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                user.email.trim().to_lowercase(),
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
                user.created_at
            ],
        )?;
        let uid: i64 = conn.query_row(
            "SELECT id FROM users WHERE LOWER(email) = ?1",
            params![user.email.trim().to_lowercase()],
            |r| r.get(0),
        )?;
        Ok(uid)
    }

    pub fn update_user_password_hash(&self, email: &str, new_hash: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE users SET password_hash = ?1 WHERE LOWER(email) = LOWER(?2)",
            params![new_hash, email.trim()],
        )?;
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
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, slot_code, badge_label, dimensions, sponsor_name, sponsor_type, pill_style,
                    title_tr, title_en, desc_tr, desc_en, target_url, image_url,
                    budget_score, is_default_fallback, is_floating_popup, is_newsletter_ad,
                    target_categories_json, target_ssb_tags_json, target_institutions_json,
                    impressions_count, clicks_count, is_active
             FROM ad_campaigns ORDER BY id ASC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
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
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
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
        self.refresh_l1_cache();
        Ok(result_id)
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

    pub fn get_all_analytics_events(&self) -> Vec<AnalyticsEvent> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, session_id, user_id, organization_category, organization_name,
                    event_type, content_slug, ad_id, duration_sec, scroll_depth_pct,
                    video_completed, is_true_read, ssb_tag, created_at
             FROM analytics_events ORDER BY id DESC LIMIT 2000",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
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
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT code, name_tr, name_en, price_try, period_label_tr, period_label_en,
                    features_tr_json, features_en_json, is_featured
             FROM subscription_plans ORDER BY sort_order ASC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
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
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
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
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, sku, product_type, title_tr, title_en, author_or_source,
                    description_tr, description_en, price_try, cover_image,
                    file_resolution_or_pages, is_available
             FROM commerce_products ORDER BY id ASC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(CommerceProduct {
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
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
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
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO order_invoices (
                invoice_no, user_email, full_name, organization_name,
                item_type, item_code, item_title, amount_try, payment_status,
                created_at, pdf_watermark_code
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                inv.invoice_no,
                inv.user_email,
                inv.full_name,
                inv.organization_name,
                inv.item_type,
                inv.item_code,
                inv.item_title,
                inv.amount_try,
                inv.payment_status,
                inv.created_at,
                inv.pdf_watermark_code
            ],
        )?;
        let rowid = conn.last_insert_rowid();
        drop(conn);

        if inv.payment_status == "paid" {
            self.activate_invoice_entitlements(inv);
        }
        Ok(rowid)
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
                let _ = conn.execute(
                    "UPDATE renewal_reminders SET status = 'halted_by_renewal' WHERE LOWER(user_email) = LOWER(?1) AND status = 'scheduled'",
                    params![inv.user_email],
                );
                let _ = conn.execute(
                    "UPDATE users SET reminders_halted = 1, role = CASE WHEN role IN ('site_operator', 'admin') THEN role ELSE ?1 END, subscription_tier = ?2, subscription_expires_at = ?3 WHERE LOWER(email) = LOWER(?4)",
                    params![assigned_role, inv.item_title, dynamic_exp, inv.user_email],
                );
            }
        }
    }

    pub fn finalize_paid_invoice(&self, merchant_oid: &str) -> Option<OrderInvoice> {
        let mut target = self
            .get_order_invoices()
            .into_iter()
            .find(|inv| crate::services::payment_service::matches_merchant_oid(&inv.invoice_no, merchant_oid))?;

        if target.payment_status != "paid" {
            if let Ok(conn) = self.open_conn() {
                let _ = conn.execute(
                    "UPDATE order_invoices SET payment_status = 'paid' WHERE id = ?1",
                    params![target.id],
                );
            }
            target.payment_status = "paid".to_string();
            self.activate_invoice_entitlements(&target);
        }
        Some(target)
    }

    pub fn mark_invoice_failed(&self, merchant_oid: &str) -> bool {
        let Some(target) = self
            .get_order_invoices()
            .into_iter()
            .find(|inv| crate::services::payment_service::matches_merchant_oid(&inv.invoice_no, merchant_oid))
        else {
            return false;
        };
        if target.payment_status != "paid" {
            if let Ok(conn) = self.open_conn() {
                let _ = conn.execute(
                    "UPDATE order_invoices SET payment_status = 'failed' WHERE id = ?1",
                    params![target.id],
                );
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
        let mut upserted = 0usize;
        for log in logs {
            let existing_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM renewal_reminders WHERE user_id = ?1 AND stage_label = ?2",
                    params![log.user_id, log.stage_label],
                    |r| r.get(0),
                )
                .ok();

            if let Some(eid) = existing_id {
                conn.execute(
                    "UPDATE renewal_reminders SET days_remaining = ?1, status = ?2, sent_at = ?3 WHERE id = ?4",
                    params![log.days_remaining, log.status, log.sent_at, eid],
                )?;
            } else {
                conn.execute(
                    "INSERT INTO renewal_reminders (
                        user_id, user_email, full_name, organization_name,
                        stage_label, days_remaining, status, sent_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        log.user_id,
                        log.user_email,
                        log.full_name,
                        log.organization_name,
                        log.stage_label,
                        log.days_remaining,
                        log.status,
                        log.sent_at
                    ],
                )?;
                upserted += 1;
            }
        }
        Ok(upserted)
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

    pub fn insert_newsletter_campaign(&self, nl: &NewsletterCampaign) -> rusqlite::Result<i64> {
        let all_users = self.get_all_users();
        let matched_count = crate::services::email_service::filter_newsletter_recipients(
            &all_users,
            &nl.target_institution,
            &nl.target_ssb_tag,
        )
        .len() as i64;
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
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

    fn load_seo_redirects_db(&self) -> Vec<SeoRedirectRule> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, old_path, new_path, status_code, hit_count
             FROM seo_redirects ORDER BY id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(SeoRedirectRule {
                id: r.get(0)?,
                old_path: r.get(1)?,
                new_path: r.get(2)?,
                status_code: r.get::<_, u16>(3)?,
                hit_count: r.get(4)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
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
        self.refresh_l1_cache();
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
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT key, value_tr, value_en, setting_group, updated_at
             FROM site_settings ORDER BY setting_group ASC, key ASC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(SiteSetting {
                key: r.get(0)?,
                value_tr: r.get(1)?,
                value_en: r.get(2)?,
                setting_group: r.get(3)?,
                updated_at: r.get(4)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
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
        self.refresh_l1_cache();
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
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, target_audience, target_email, notification_type, severity,
                    title_tr, title_en, message_tr, message_en, action_url,
                    is_read, is_active_banner, created_at
             FROM system_notifications ORDER BY id DESC LIMIT 100",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(SystemNotification {
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
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
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
        self.refresh_l1_cache();
        Ok(rowid)
    }

    pub fn mark_notification_read(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE system_notifications SET is_read = 1 WHERE id = ?1",
            params![id],
        )?;
        drop(conn);
        self.refresh_l1_cache();
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
            "SELECT key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp FROM translations"
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

    pub fn upsert_user_role(&self, role_code: &str, name_tr: &str, name_en: &str, permissions_json: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO user_roles (role_code, name_tr, name_en, permissions_json, is_system) VALUES (?1, ?2, ?3, ?4, 0) ON CONFLICT(role_code) DO UPDATE SET name_tr=?2, name_en=?3, permissions_json=?4",
            rusqlite::params![role_code, name_tr, name_en, permissions_json])?;
        Ok(())
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

    // =========================================================================
    // ADVANCED ADMIN: System Health
    // =========================================================================

    pub fn get_system_health(&self) -> serde_json::Value {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return serde_json::json!({"status": "error", "message": "DB connection failed"}) };
        let content_count: i64 = conn.query_row("SELECT COUNT(*) FROM contents", [], |r| r.get(0)).unwrap_or(0);
        let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap_or(0);
        let event_count: i64 = conn.query_row("SELECT COUNT(*) FROM analytics_events", [], |r| r.get(0)).unwrap_or(0);
        let db_size: i64 = conn.query_row("SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()", [], |r| r.get(0)).unwrap_or(0);
        serde_json::json!({
            "status": "healthy",
            "db_size_bytes": db_size,
            "tables": { "contents": content_count, "users": user_count, "analytics_events": event_count },
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
}

