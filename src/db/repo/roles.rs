#[allow(unused_imports)]
use super::*;

impl Database {
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

    pub fn get_user_roles(&self) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut stmt = match conn.prepare(
            "SELECT id, role_code, name_tr, name_en, permissions_json, is_system FROM user_roles ORDER BY id",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "role_code": r.get::<_, String>(1)?,
                "name_tr": r.get::<_, String>(2)?,
                "name_en": r.get::<_, String>(3)?,
                "permissions_json": r.get::<_, String>(4)?,
                "is_system": r.get::<_, bool>(5)?
            }))
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_user_roles_paginated(&self, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return (vec![], 0),
        };
        let total: u64 = conn
            .query_row("SELECT COUNT(*) FROM user_roles", [], |r| r.get(0))
            .unwrap_or(0);
        let mut stmt = match conn.prepare(
            "SELECT id, role_code, name_tr, name_en, permissions_json, is_system FROM user_roles ORDER BY id LIMIT ?1 OFFSET ?2",
        ) {
            Ok(s) => s,
            Err(_) => return (vec![], total),
        };
        let rows = match stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "role_code": r.get::<_, String>(1)?,
                "name_tr": r.get::<_, String>(2)?,
                "name_en": r.get::<_, String>(3)?,
                "permissions_json": r.get::<_, String>(4)?,
                "is_system": r.get::<_, bool>(5)?
            }))
        }) {
            Ok(r) => r,
            Err(_) => return (vec![], total),
        };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }

    pub fn upsert_user_role(
        &self,
        role_code: &str,
        name_tr: &str,
        name_en: &str,
        permissions_json: &str,
    ) -> rusqlite::Result<()> {
        let conn = self
            .open_conn()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
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

    pub fn update_user_role(&self, user_id: i64, role_code: &str) -> rusqlite::Result<()> {
        let conn = self
            .open_conn()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE users SET role = ?1 WHERE id = ?2",
            rusqlite::params![role_code.trim(), user_id],
        )?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    pub fn delete_user_role(&self, role_code: &str) -> rusqlite::Result<bool> {
        let conn = self
            .open_conn()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let affected = conn.execute(
            "DELETE FROM user_roles WHERE role_code = ?1 AND is_system = 0",
            rusqlite::params![role_code.trim()],
        )?;
        Ok(affected > 0)
    }
}
