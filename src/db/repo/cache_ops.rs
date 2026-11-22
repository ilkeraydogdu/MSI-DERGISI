#[allow(unused_imports)]
use super::*;

impl Database {
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
}