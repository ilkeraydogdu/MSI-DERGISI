#[allow(unused_imports)]
use super::*;

impl Database {
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
}