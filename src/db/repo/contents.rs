#[allow(unused_imports)]
use super::*;

impl Database {
    pub(super) fn load_all_placements_map(conn: &Connection) -> HashMap<i64, Vec<String>> {
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

    pub(super) fn load_all_ssb_tags_map(conn: &Connection) -> HashMap<i64, Vec<String>> {
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

    pub(super) fn load_contents_db(&self) -> Vec<ContentItem> {
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

}
