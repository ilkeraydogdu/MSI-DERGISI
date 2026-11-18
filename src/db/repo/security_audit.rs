#[allow(unused_imports)]
use super::*;

impl Database {
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

}
