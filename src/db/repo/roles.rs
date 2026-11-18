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
}
