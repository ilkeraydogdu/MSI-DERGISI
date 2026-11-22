#[allow(unused_imports)]
use super::*;

impl Database {
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
}