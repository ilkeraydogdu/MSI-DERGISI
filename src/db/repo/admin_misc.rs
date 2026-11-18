#[allow(unused_imports)]
use super::*;

impl Database {
    // =========================================================================

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
        conn.execute(
            "INSERT INTO content_versions (
                content_id, version, title_tr, title_en, body_tr, body_en, summary_tr, summary_en,
                changed_by, change_note, created_at, en_status, previous_en_status, source_action, review_note
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                content_id, max_ver + 1, title_tr, title_en, body_tr, body_en, summary_tr, summary_en,
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

        let restored_tr_hash = Self::compute_tr_source_hash(&row.0, &row.2, &row.4);
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
            rusqlite::params![row.0, row.1, row.2, row.3, row.4, row.5, row.6, restored_tr_hash, content_id],
        ).map_err(|e| e.to_string())?;
        drop(conn);
        if affected > 0 {
            let _ = self.insert_content_version_full(
                content_id, &row.0, &row.1, &row.4, &row.5, &row.2, &row.3,
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

    pub fn search_media_library_paginated(&self, query: &str, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let q = query.trim();
        if q.is_empty() {
            return self.get_media_library_paginated(limit, offset);
        }
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let pattern = format!("%{}%", q);
        let total: u64 = conn.query_row(
            "SELECT COUNT(*) FROM media_library WHERE original_name LIKE ?1 OR path LIKE ?1 OR alt_text_tr LIKE ?1 OR tags LIKE ?1",
            rusqlite::params![pattern], |r| r.get(0),
        ).unwrap_or(0);
        let mut stmt = match conn.prepare(
            "SELECT id, filename, original_name, mime_type, file_size, path, alt_text_tr, alt_text_en, tags, uploaded_by, created_at
             FROM media_library
             WHERE original_name LIKE ?1 OR path LIKE ?1 OR alt_text_tr LIKE ?1 OR tags LIKE ?1
             ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
        ) { Ok(s) => s, Err(_) => return (vec![], total) };
        let rows = match stmt.query_map(rusqlite::params![pattern, limit, offset], |r| {
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

    pub fn update_media_item(&self, id: i64, alt_text_tr: &str, alt_text_en: &str, tags: &str) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let affected = conn.execute(
            "UPDATE media_library SET alt_text_tr = ?1, alt_text_en = ?2, tags = ?3 WHERE id = ?4",
            rusqlite::params![alt_text_tr.trim(), alt_text_en.trim(), tags.trim(), id],
        ).map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    pub fn delete_media_item(&self, id: i64) -> Result<Option<String>, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let path_opt: Option<String> = conn
            .query_row("SELECT path FROM media_library WHERE id = ?1", rusqlite::params![id], |r| r.get(0))
            .ok();
        let Some(media_path) = path_opt else {
            return Ok(None);
        };
        conn.execute("DELETE FROM media_library WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
        Ok(Some(media_path))
    }
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
        let filter_all = applies_to.trim().is_empty() || applies_to.eq_ignore_ascii_case("all");
        let total: u64 = if filter_all {
            conn.query_row("SELECT COUNT(*) FROM custom_fields", [], |r| r.get(0)).unwrap_or(0)
        } else {
            conn.query_row("SELECT COUNT(*) FROM custom_fields WHERE applies_to = ?1", rusqlite::params![applies_to], |r| r.get(0)).unwrap_or(0)
        };
        let sql = if filter_all {
            "SELECT id, field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to FROM custom_fields ORDER BY sort_order LIMIT ?1 OFFSET ?2"
        } else {
            "SELECT id, field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to FROM custom_fields WHERE applies_to = ?1 ORDER BY sort_order LIMIT ?2 OFFSET ?3"
        };
        let mut stmt = match conn.prepare(sql) { Ok(s) => s, Err(_) => return (vec![], total) };
        let map_row = |r: &rusqlite::Row<'_>| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "field_key": r.get::<_, String>(1)?,
                "label_tr": r.get::<_, String>(2)?, "label_en": r.get::<_, String>(3)?,
                "field_type": r.get::<_, String>(4)?, "options_json": r.get::<_, String>(5)?,
                "is_required": r.get::<_, bool>(6)?, "sort_order": r.get::<_, i64>(7)?,
                "applies_to": r.get::<_, String>(8)?
            }))
        };
        let rows: Vec<serde_json::Value> = if filter_all {
            stmt.query_map(rusqlite::params![limit, offset], map_row).ok().into_iter().flatten().filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map(rusqlite::params![applies_to, limit, offset], map_row).ok().into_iter().flatten().filter_map(|r| r.ok()).collect()
        };
        (rows, total)
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
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(e) => return serde_json::json!({"status": "degraded", "db_ok": false, "message": e.to_string()}),
        };
        let integrity_ok = conn
            .query_row("PRAGMA quick_check(1)", [], |r| r.get::<_, String>(0))
            .map(|s| s.eq_ignore_ascii_case("ok"))
            .unwrap_or(false);
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap_or_else(|_| "unknown".to_string());
        let content_count: i64 = conn.query_row("SELECT COUNT(*) FROM contents", [], |r| r.get(0)).unwrap_or(0);
        let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap_or(0);
        let event_count: i64 = conn.query_row("SELECT COUNT(*) FROM analytics_events", [], |r| r.get(0)).unwrap_or(0);
        let ad_count: i64 = conn.query_row("SELECT COUNT(*) FROM ad_campaigns", [], |r| r.get(0)).unwrap_or(0);
        let category_count: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0)).unwrap_or(0);
        let issue_count: i64 = conn.query_row("SELECT COUNT(*) FROM magazine_issues", [], |r| r.get(0)).unwrap_or(0);
        let order_count: i64 = conn.query_row("SELECT COUNT(*) FROM order_invoices", [], |r| r.get(0)).unwrap_or(0);
        let db_size: i64 = conn.query_row("SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()", [], |r| r.get(0)).unwrap_or(0);
        let l1_cached = self.l1_cache.read().map(|g| g.is_some()).unwrap_or(false);
        let mem = crate::engine::monitoring_engine::read_memory_stats();
        let disk = crate::engine::monitoring_engine::read_disk_stats();
        let mem_used_mb = mem.total_kb.saturating_sub(mem.available_kb) / 1024;
        let mem_total_mb = mem.total_kb / 1024;
        let disk_used_gb = ((disk.used_bytes as f64) / 1_073_741_824.0 * 100.0).round() / 100.0;
        let disk_total_gb = ((disk.total_bytes as f64) / 1_073_741_824.0 * 100.0).round() / 100.0;
        let status = if !integrity_ok || disk.used_pct >= 98.0 {
            "degraded"
        } else {
            "healthy"
        };
        serde_json::json!({
            "status": status,
            "db_ok": integrity_ok,
            "journal_mode": journal_mode,
            "l1_cache_warm": l1_cached,
            "db_size_bytes": db_size,
            "memory": {
                "used_mb": mem_used_mb,
                "total_mb": mem_total_mb,
                "used_pct": mem.used_pct
            },
            "disk": {
                "used_gb": disk_used_gb,
                "total_gb": disk_total_gb,
                "used_pct": disk.used_pct
            },
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
}
