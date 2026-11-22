#[allow(unused_imports)]
use super::*;
use sha2::{Digest, Sha256};

impl Database {
    /// Deterministic SHA-256 hash of Turkish source fields used to detect EN staleness after TR edits.
    pub fn compute_tr_source_hash(title_tr: &str, summary_tr: &str, body_tr: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(title_tr.trim().as_bytes());
        hasher.update(b"\n---\n");
        hasher.update(summary_tr.trim().as_bytes());
        hasher.update(b"\n---\n");
        hasher.update(body_tr.trim().as_bytes());
        hex::encode(hasher.finalize())
    }

    fn normalize_en_status(raw: &str) -> &'static str {
        match raw.trim().to_ascii_lowercase().as_str() {
            "approved" => "approved",
            "auto_draft" => "auto_draft",
            _ => "none",
        }
    }
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
                    view_count, true_read_count, COALESCE(status, 'published'),
                    COALESCE(en_status, 'none'), en_translated_at, en_approved_by, en_approved_at, tr_source_hash,
                    COALESCE(target_institutions_json, '[]')
             FROM contents
             ORDER BY published_at DESC, id DESC",
        ) else {
            return Vec::new();
        };

        let Ok(rows) = stmt.query_map([], |r| {
            let inst_raw: String = r.get(31).unwrap_or_else(|_| "[]".to_string());
            let target_institutions: Vec<String> = serde_json::from_str(&inst_raw).unwrap_or_default();
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
                en_status: r.get::<_, String>(26).unwrap_or_else(|_| "none".to_string()),
                en_translated_at: r.get(27)?,
                en_approved_by: r.get(28)?,
                en_approved_at: r.get(29)?,
                tr_source_hash: r.get(30)?,
                target_institutions,
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
        let contents = self.get_all_contents();
        if let Some(found) = contents.iter().find(|c| c.slug.eq_ignore_ascii_case(slug)) {
            return Some(found.clone());
        }
        if slug.contains('_') && !slug.contains('/') && !slug.contains("..") {
            if let Some(norm) = crate::engine::validation_engine::validate_slug(slug) {
                return contents.into_iter().find(|c| c.slug.eq_ignore_ascii_case(&norm));
            }
        }
        None
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
        let tx = conn.unchecked_transaction()?;
        let status = if item.status.trim().is_empty() {
            "published"
        } else {
            item.status.trim()
        };
        let now = chrono::Utc::now().to_rfc3339();
        let new_tr_hash = Self::compute_tr_source_hash(&item.title_tr, &item.summary_tr, &item.body_tr);
        let target_inst_json =
            serde_json::to_string(&item.target_institutions).unwrap_or_else(|_| "[]".to_string());

        let content_id: i64 = if item.id > 0 {
            let prev: (String, String, String, String, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>) = tx.query_row(
                "SELECT title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                        COALESCE(en_status, 'none'), en_translated_at, en_approved_by, en_approved_at, tr_source_hash
                 FROM contents WHERE id = ?1",
                params![item.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?)),
            )?;

            // V1: Auto-snapshot previous DB state into content_versions before first edit
            let existing_versions: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM content_versions WHERE content_id = ?1",
                    params![item.id],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if existing_versions == 0 {
                tx.execute(
                    "INSERT INTO content_versions (
                        content_id, version, title_tr, title_en, body_tr, body_en, summary_tr, summary_en,
                        changed_by, change_note, created_at, en_status, previous_en_status, source_action, review_note
                     ) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7, 'system-autosnapshot', 'Düzenleme öncesi orijinal sürüm yedeği', ?8, ?9, ?9, 'auto_snapshot', '')",
                    params![item.id, prev.0, prev.1, prev.4, prev.5, prev.2, prev.3, now, prev.6],
                )?;
            }

            let prev_tr_hash = Self::compute_tr_source_hash(&prev.0, &prev.2, &prev.4);
            let tr_changed = prev_tr_hash != new_tr_hash;
            let en_unchanged = item.title_en == prev.1 && item.summary_en == prev.3 && item.body_en == prev.5;
            let requested_en = if item.en_status.trim().is_empty() {
                prev.6.as_str()
            } else {
                Self::normalize_en_status(&item.en_status)
            };
            let approved_timestamp_changed = item.en_approved_at.is_some() && item.en_approved_at != prev.9;

            let (next_en_status, next_en_translated_at, next_en_approved_by, next_en_approved_at, next_tr_hash) =
                if prev.6 == "approved" && tr_changed && en_unchanged && !approved_timestamp_changed {
                    // TR changed after EN approval without updating EN -> mark EN draft out of sync
                    (
                        "auto_draft".to_string(),
                        prev.7.clone(),
                        prev.8.clone(),
                        prev.9.clone(),
                        prev.10.clone().or(Some(prev_tr_hash)),
                    )
                } else {
                    let st = requested_en.to_string();
                    let tr_at = if !en_unchanged || (st == "auto_draft" && prev.7.is_none()) {
                        item.en_translated_at.clone().or_else(|| Some(now.clone()))
                    } else {
                        item.en_translated_at.clone().or(prev.7)
                    };
                    let app_by = item.en_approved_by.clone().or(prev.8);
                    let app_at = if st == "approved" {
                        item.en_approved_at.clone().or(prev.9).or_else(|| Some(now.clone()))
                    } else {
                        item.en_approved_at.clone().or(prev.9)
                    };
                    let src_hash = if !en_unchanged || (st == "approved" && prev.6 != "approved") {
                        Some(new_tr_hash)
                    } else {
                        item.tr_source_hash.clone().or(prev.10)
                    };
                    (st, tr_at, app_by, app_at, src_hash)
                };

            tx.execute(
                "UPDATE contents SET
                    slug = ?1, title_tr = ?2, title_en = ?3, summary_tr = ?4, summary_en = ?5,
                    body_tr = ?6, body_en = ?7, content_type = ?8, source_group = ?9,
                    cover_image = ?10, author_name = ?11, author_title = ?12, issue_number = ?13,
                    published_at = ?14, read_time_min = ?15, visibility_level = ?16,
                    single_price_try = ?17, is_sponsored = ?18, sponsor_brand = ?19,
                    sponsor_legal_label = ?20, is_featured = ?21, video_url = ?22,
                    status = COALESCE(NULLIF(?23, ''), status, 'published'),
                    en_status = ?24,
                    en_translated_at = ?25,
                    en_approved_by = ?26,
                    en_approved_at = ?27,
                    tr_source_hash = ?28,
                    target_institutions_json = ?29
                 WHERE id = ?30",
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
                    next_en_status,
                    next_en_translated_at,
                    next_en_approved_by,
                    next_en_approved_at,
                    next_tr_hash,
                    target_inst_json,
                    item.id
                ],
            )?;
            item.id
        } else {
            // N6: Creating a new item (id == 0) NEVER uses ON CONFLICT(slug) DO UPDATE;
            // auto-dedupe slug (-2, -3, ...) so a new article never overwrites a live article.
            let base_slug = if item.slug.trim().is_empty() {
                format!("icerik-{}", chrono::Utc::now().timestamp_millis())
            } else {
                item.slug.trim().to_string()
            };
            let mut final_slug = base_slug.clone();
            let mut suffix = 2usize;
            loop {
                let exists: i64 = tx
                    .query_row(
                        "SELECT COUNT(*) FROM contents WHERE LOWER(slug) = LOWER(?1)",
                        params![final_slug],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                if exists == 0 {
                    break;
                }
                final_slug = format!("{}-{}", base_slug, suffix);
                suffix += 1;
            }

            let en_st = if item.en_status.trim().is_empty() {
                if item.source_group == "msi-tdr" {
                    "approved"
                } else {
                    "none"
                }
            } else {
                Self::normalize_en_status(&item.en_status)
            };
            let tr_hash = if en_st != "none" {
                item.tr_source_hash.clone().or(Some(new_tr_hash))
            } else {
                item.tr_source_hash.clone()
            };

            tx.execute(
                "INSERT INTO contents (
                    slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                    content_type, source_group, cover_image, author_name, author_title,
                    issue_number, published_at, read_time_min, visibility_level, single_price_try,
                    is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                    view_count, true_read_count, status,
                    en_status, en_translated_at, en_approved_by, en_approved_at, tr_source_hash,
                    target_institutions_json
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                    ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17,
                    ?18, ?19, ?20, ?21, ?22,
                    ?23, ?24, ?25,
                    ?26, ?27, ?28, ?29, ?30,
                    ?31
                )",
                params![
                    final_slug,
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
                    status,
                    en_st,
                    item.en_translated_at,
                    item.en_approved_by,
                    item.en_approved_at,
                    tr_hash,
                    target_inst_json
                ],
            )?;
            tx.last_insert_rowid()
        };

        // Batch junction-table writes: one prepared statement per table, reused across rows,
        // wrapped in the same transaction so partial failures cannot leave orphaned links.
        tx.execute(
            "DELETE FROM content_placements WHERE content_id = ?1",
            params![content_id],
        )?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO content_placements (content_id, category_slug) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
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
                "INSERT INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
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

    /// Saves EN translation text (`auto_draft` or `approved`) and updates `tr_source_hash`.
    pub fn update_content_en_translation(
        &self,
        content_id: i64,
        title_en: &str,
        summary_en: &str,
        body_en: &str,
        en_status: &str,
        actor_email: &str,
    ) -> Result<bool, String> {
        let norm_status = Self::normalize_en_status(en_status);
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let (tr_t, tr_s, tr_b): (String, String, String) = tx
            .query_row(
                "SELECT title_tr, summary_tr, body_tr FROM contents WHERE id = ?1",
                params![content_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|_| "İçerik bulunamadı.".to_string())?;
        let tr_hash = Self::compute_tr_source_hash(&tr_t, &tr_s, &tr_b);
        let now = chrono::Utc::now().to_rfc3339();
        let (approved_by, approved_at): (Option<&str>, Option<&str>) = if norm_status == "approved" {
            (Some(actor_email.trim()), Some(now.as_str()))
        } else {
            (None, None)
        };
        let affected = tx
            .execute(
                "UPDATE contents SET
                    title_en = ?1,
                    summary_en = ?2,
                    body_en = ?3,
                    en_status = ?4,
                    en_translated_at = ?5,
                    en_approved_by = COALESCE(?6, en_approved_by),
                    en_approved_at = COALESCE(?7, en_approved_at),
                    tr_source_hash = ?8
                 WHERE id = ?9",
                params![
                    title_en,
                    summary_en,
                    body_en,
                    norm_status,
                    now,
                    approved_by,
                    approved_at,
                    tr_hash,
                    content_id
                ],
            )
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        drop(conn);
        if affected > 0 {
            crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
            self.refresh_l1_sections(&["contents"]);
        }
        Ok(affected > 0)
    }

    /// Explicitly marks an article's English translation as `approved` if non-empty.
    pub fn approve_content_en(&self, content_id: i64, actor_email: &str) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let (t_tr, s_tr, b_tr, t_en, b_en): (String, String, String, String, String) = conn
            .query_row(
                "SELECT title_tr, summary_tr, body_tr, title_en, body_en FROM contents WHERE id = ?1",
                params![content_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|_| "İçerik bulunamadı.".to_string())?;
        if t_en.trim().is_empty() || b_en.trim().is_empty() {
            return Err("İngilizce başlık ve gövde metni boşken onay verilemez.".to_string());
        }
        let tr_hash = Self::compute_tr_source_hash(&t_tr, &s_tr, &b_tr);
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE contents SET
                    en_status = 'approved',
                    en_approved_by = ?1,
                    en_approved_at = ?2,
                    en_translated_at = COALESCE(en_translated_at, ?2),
                    tr_source_hash = ?3
                 WHERE id = ?4",
                params![actor_email.trim(), now, tr_hash, content_id],
            )
            .map_err(|e| e.to_string())?;
        drop(conn);
        if affected > 0 {
            crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
            self.refresh_l1_sections(&["contents"]);
        }
        Ok(affected > 0)
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
