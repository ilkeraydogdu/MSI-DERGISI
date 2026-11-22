#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn insert_content_version(&self, content_id: i64, title_tr: &str, body_tr: &str, summary_tr: &str, changed_by: &str, change_note: &str) -> rusqlite::Result<i64> {
        let (title_en, body_en, summary_en, en_status) = self
            .open_conn()
            .ok()
            .and_then(|conn| {
                conn.query_row(
                    "SELECT title_en, body_en, summary_en, COALESCE(en_status, 'none') FROM contents WHERE id = ?1",
                    rusqlite::params![content_id],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)),
                )
                .ok()
            })
            .unwrap_or_else(|| (String::new(), String::new(), String::new(), "none".to_string()));
        self.insert_content_version_full(
            content_id,
            title_tr,
            &title_en,
            body_tr,
            &body_en,
            summary_tr,
            &summary_en,
            &en_status,
            "",
            "manual_snapshot",
            "",
            changed_by,
            change_note,
        )
    }

    pub fn insert_content_version_bilingual(
        &self,
        content_id: i64,
        title_tr: &str,
        title_en: &str,
        body_tr: &str,
        body_en: &str,
        summary_tr: &str,
        summary_en: &str,
        changed_by: &str,
        change_note: &str,
    ) -> rusqlite::Result<i64> {
        let en_status = self
            .open_conn()
            .ok()
            .and_then(|conn| {
                conn.query_row(
                    "SELECT COALESCE(en_status, 'none') FROM contents WHERE id = ?1",
                    rusqlite::params![content_id],
                    |r| r.get::<_, String>(0),
                )
                .ok()
            })
            .unwrap_or_else(|| "none".to_string());
        self.insert_content_version_full(
            content_id,
            title_tr,
            title_en,
            body_tr,
            body_en,
            summary_tr,
            summary_en,
            &en_status,
            "",
            "edit",
            "",
            changed_by,
            change_note,
        )
    }

    pub fn insert_content_version_full(
        &self,
        content_id: i64,
        title_tr: &str,
        title_en: &str,
        body_tr: &str,
        body_en: &str,
        summary_tr: &str,
        summary_en: &str,
        en_status: &str,
        previous_en_status: &str,
        source_action: &str,
        review_note: &str,
        changed_by: &str,
        change_note: &str,
    ) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let max_ver: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM content_versions WHERE content_id = ?1",
            rusqlite::params![content_id], |r| r.get(0))?;
        let norm_en = match en_status.trim() {
            "approved" => "approved",
            "auto_draft" => "auto_draft",
            _ => "none",
        };
        let clean_title_tr = crate::engine::validation_engine::sanitize_plain_text(title_tr);
        let clean_title_en = crate::engine::validation_engine::sanitize_plain_text(title_en);
        let clean_body_tr = crate::engine::validation_engine::sanitize_html(body_tr);
        let clean_body_en = crate::engine::validation_engine::sanitize_html(body_en);
        let clean_summary_tr = crate::engine::validation_engine::sanitize_plain_text(summary_tr);
        let clean_summary_en = crate::engine::validation_engine::sanitize_plain_text(summary_en);
        conn.execute(
            "INSERT INTO content_versions (
                content_id, version, title_tr, title_en, body_tr, body_en, summary_tr, summary_en,
                changed_by, change_note, created_at, en_status, previous_en_status, source_action, review_note
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                content_id, max_ver + 1, clean_title_tr, clean_title_en, clean_body_tr, clean_body_en, clean_summary_tr, clean_summary_en,
                changed_by, change_note, now, norm_en, previous_en_status, source_action, review_note
            ])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn restore_content_version(&self, content_id: i64, version: i64, actor_email: &str) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let row: (String, String, String, String, String, String, String) = conn.query_row(
            "SELECT title_tr, COALESCE(title_en,''), summary_tr, COALESCE(summary_en,''),
                    body_tr, COALESCE(body_en,''), COALESCE(en_status, 'none')
             FROM content_versions WHERE content_id = ?1 AND version = ?2",
            rusqlite::params![content_id, version],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        ).map_err(|_| format!("İçerik #{} için v{} sürümü bulunamadı.", content_id, version))?;

        let prev_en_status: String = conn
            .query_row(
                "SELECT COALESCE(en_status, 'none') FROM contents WHERE id = ?1",
                rusqlite::params![content_id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "none".to_string());

        let clean_title_tr = crate::engine::validation_engine::sanitize_plain_text(&row.0);
        let clean_title_en = crate::engine::validation_engine::sanitize_plain_text(&row.1);
        let clean_summary_tr = crate::engine::validation_engine::sanitize_plain_text(&row.2);
        let clean_summary_en = crate::engine::validation_engine::sanitize_plain_text(&row.3);
        let clean_body_tr = crate::engine::validation_engine::sanitize_html(&row.4);
        let clean_body_en = crate::engine::validation_engine::sanitize_html(&row.5);

        let restored_tr_hash = Self::compute_tr_source_hash(&clean_title_tr, &clean_summary_tr, &clean_body_tr);
        // Symmetrical TR + EN + en_status rollback (never keeps newer EN over an older TR snapshot)
        let affected = conn.execute(
            "UPDATE contents SET
                title_tr = ?1,
                title_en = ?2,
                summary_tr = ?3,
                summary_en = ?4,
                body_tr = ?5,
                body_en = ?6,
                en_status = ?7,
                tr_source_hash = ?8
             WHERE id = ?9",
            rusqlite::params![
                clean_title_tr,
                clean_title_en,
                clean_summary_tr,
                clean_summary_en,
                clean_body_tr,
                clean_body_en,
                row.6,
                restored_tr_hash,
                content_id
            ],
        ).map_err(|e| e.to_string())?;
        drop(conn);
        if affected > 0 {
            let _ = self.insert_content_version_full(
                content_id, &clean_title_tr, &clean_title_en, &clean_body_tr, &clean_body_en, &clean_summary_tr, &clean_summary_en,
                &row.6, &prev_en_status, "restore", "",
                actor_email, &format!("v{} sürümünden geri yüklendi", version),
            );
            crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
            self.refresh_l1_sections(&["contents"]);
        }
        Ok(affected > 0)
    }

    pub fn get_content_versions(&self, content_id: i64) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return vec![] };
        let mut stmt = match conn.prepare(
            "SELECT id, content_id, version, title_tr, COALESCE(title_en,''), summary_tr, COALESCE(summary_en,''),
                    body_tr, COALESCE(body_en,''), change_note, changed_by, created_at,
                    COALESCE(en_status,'none'), COALESCE(previous_en_status,''), COALESCE(source_action,'edit'), COALESCE(review_note,'')
             FROM content_versions WHERE content_id = ?1 ORDER BY version DESC"
        ) { Ok(s) => s, Err(_) => return vec![] };
        let rows = match stmt.query_map(rusqlite::params![content_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "content_id": r.get::<_, i64>(1)?,
                "version": r.get::<_, i64>(2)?, "title_tr": r.get::<_, String>(3)?,
                "title_en": r.get::<_, String>(4)?, "summary_tr": r.get::<_, String>(5)?,
                "summary_en": r.get::<_, String>(6)?,
                "body_tr": r.get::<_, String>(7)?, "body_en": r.get::<_, String>(8)?,
                "change_note": r.get::<_, String>(9)?, "changed_by": r.get::<_, String>(10)?,
                "created_at": r.get::<_, String>(11)?,
                "en_status": r.get::<_, String>(12)?,
                "previous_en_status": r.get::<_, String>(13)?,
                "source_action": r.get::<_, String>(14)?,
                "review_note": r.get::<_, String>(15)?
            }))
        }) { Ok(r) => r, Err(_) => return vec![] };
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_content_versions_paginated(&self, content_id: i64, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM content_versions WHERE content_id = ?1", rusqlite::params![content_id], |r| r.get(0)).unwrap_or(0);
        let mut stmt = match conn.prepare(
            "SELECT id, content_id, version, title_tr, COALESCE(title_en,''), summary_tr, COALESCE(summary_en,''),
                    body_tr, COALESCE(body_en,''), change_note, changed_by, created_at,
                    COALESCE(en_status,'none'), COALESCE(previous_en_status,''), COALESCE(source_action,'edit'), COALESCE(review_note,'')
             FROM content_versions WHERE content_id = ?1 ORDER BY version DESC LIMIT ?2 OFFSET ?3"
        ) { Ok(s) => s, Err(_) => return (vec![], total) };
        let rows = match stmt.query_map(rusqlite::params![content_id, limit, offset], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "content_id": r.get::<_, i64>(1)?,
                "version": r.get::<_, i64>(2)?, "title_tr": r.get::<_, String>(3)?,
                "title_en": r.get::<_, String>(4)?, "summary_tr": r.get::<_, String>(5)?,
                "summary_en": r.get::<_, String>(6)?,
                "body_tr": r.get::<_, String>(7)?, "body_en": r.get::<_, String>(8)?,
                "change_note": r.get::<_, String>(9)?, "changed_by": r.get::<_, String>(10)?,
                "created_at": r.get::<_, String>(11)?,
                "en_status": r.get::<_, String>(12)?,
                "previous_en_status": r.get::<_, String>(13)?,
                "source_action": r.get::<_, String>(14)?,
                "review_note": r.get::<_, String>(15)?
            }))
        }) { Ok(r) => r, Err(_) => return (vec![], total) };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }
}