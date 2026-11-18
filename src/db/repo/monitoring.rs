#[allow(unused_imports)]
use super::*;

impl Database {
    // =========================================================================

    /// Insert a monitoring snapshot (`system`, `domain`, `traffic`, etc.) as a JSON payload.
    pub fn insert_monitoring_snapshot(&self, snapshot_type: &str, payload_json: &str) -> Result<(), String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO monitoring_snapshots (snapshot_type, payload_json, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![snapshot_type, payload_json, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get the latest monitoring snapshot JSON payload for a given `snapshot_type`.
    pub fn get_latest_monitoring_snapshot(&self, snapshot_type: &str) -> Option<String> {
        let conn = self.pool.get().ok()?;
        let mut stmt = conn
            .prepare(
                "SELECT payload_json FROM monitoring_snapshots WHERE snapshot_type = ?1 ORDER BY created_at DESC LIMIT 1",
            )
            .ok()?;
        stmt.query_row(rusqlite::params![snapshot_type], |row| row.get(0))
            .ok()
    }

    /// Insert an attack event into `monitoring_attack_events`.
    pub fn insert_attack_event(
        &self,
        source_ip: &str,
        attack_type: &str,
        target_service: &str,
        details: &str,
        severity: &str,
    ) -> Result<(), String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO monitoring_attack_events (source_ip, attack_type, target_service, details, severity, detected_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![source_ip, attack_type, target_service, details, severity, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get recent attack events ordered by `detected_at DESC`.
    pub fn get_recent_attack_events(&self, limit: i64) -> Vec<crate::engine::monitoring_engine::AttackEvent> {
        let conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare(
            "SELECT id, source_ip, attack_type, target_service, details, severity, detected_at
             FROM monitoring_attack_events ORDER BY detected_at DESC LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map(rusqlite::params![limit], |row| {
            Ok(crate::engine::monitoring_engine::AttackEvent {
                id: Some(row.get(0)?),
                source_ip: row.get(1)?,
                attack_type: row.get(2)?,
                target_service: row.get(3)?,
                details: row.get(4)?,
                severity: row.get(5)?,
                detected_at: row.get(6)?,
            })
        }) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    /// Upsert a discovered vhost domain into `monitored_domains`.
    pub fn upsert_monitored_domain(
        &self,
        domain: &str,
        vhost_path: &str,
        access_log: &str,
        error_log: &str,
    ) -> Result<(), String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO monitored_domains (domain, vhost_path, access_log_path, error_log_path)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(domain) DO UPDATE SET
                 vhost_path = excluded.vhost_path,
                 access_log_path = excluded.access_log_path,
                 error_log_path = excluded.error_log_path",
            rusqlite::params![domain, vhost_path, access_log, error_log],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Prune monitoring snapshots and attack events older than `retention_days`.
    pub fn prune_monitoring_data(&self, retention_days: u32) -> Result<usize, String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_str = cutoff.to_rfc3339();
        let deleted_snapshots = conn
            .execute(
                "DELETE FROM monitoring_snapshots WHERE created_at < ?1",
                rusqlite::params![cutoff_str],
            )
            .map_err(|e| e.to_string())?;
        let deleted_attacks = conn
            .execute(
                "DELETE FROM monitoring_attack_events WHERE detected_at < ?1",
                rusqlite::params![cutoff_str],
            )
            .map_err(|e| e.to_string())?;
        Ok(deleted_snapshots + deleted_attacks)
    }

    // =========================================================================
    // PASSWORD RESET TOKEN METHODS (008_password_reset.sql — SHA-256 + fallback)
}
