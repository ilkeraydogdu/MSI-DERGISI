#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_renewal_reminders(&self) -> Vec<RenewalReminderLog> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, user_id, user_email, full_name, organization_name,
                    stage_label, days_remaining, status, sent_at
             FROM renewal_reminders ORDER BY days_remaining ASC, id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(RenewalReminderLog {
                id: r.get(0)?,
                user_id: r.get(1)?,
                user_email: r.get(2)?,
                full_name: r.get(3)?,
                organization_name: r.get(4)?,
                stage_label: r.get(5)?,
                days_remaining: r.get(6)?,
                status: r.get(7)?,
                sent_at: r.get(8)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_renewal_reminders_paginated(&self, limit: u32, offset: u32) -> (Vec<RenewalReminderLog>, u64) {
        crate::db::query_helpers::query_paginated(
            self,
            "SELECT COUNT(*) FROM renewal_reminders",
            [],
            "SELECT id, user_id, user_email, full_name, organization_name,
                    stage_label, days_remaining, status, sent_at
             FROM renewal_reminders ORDER BY days_remaining ASC, id DESC LIMIT ?1 OFFSET ?2",
            rusqlite::params![limit, offset],
            |r| Ok(RenewalReminderLog {
                id: r.get(0)?,
                user_id: r.get(1)?,
                user_email: r.get(2)?,
                full_name: r.get(3)?,
                organization_name: r.get(4)?,
                stage_label: r.get(5)?,
                days_remaining: r.get(6)?,
                status: r.get(7)?,
                sent_at: r.get(8)?,
            }),
        )
    }

    pub fn mark_renewal_reminder_sent(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "UPDATE renewal_reminders SET status = 'sent', sent_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn halt_renewed_reminders(&self) -> rusqlite::Result<usize> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let cutoff = (chrono::Utc::now() + chrono::Duration::days(65))
            .format("%Y-%m-%d")
            .to_string();
        let _ = conn.execute(
            "UPDATE users SET reminders_halted = 1 WHERE subscription_expires_at IS NOT NULL AND subscription_expires_at > ?1",
            params![cutoff],
        );
        let updated = conn.execute(
            "UPDATE renewal_reminders SET status = 'halted_by_renewal'
             WHERE status = 'scheduled' AND user_id IN (SELECT id FROM users WHERE reminders_halted = 1)",
            [],
        )?;
        Ok(updated)
    }

    pub fn sync_evaluated_reminders(&self, logs: &[RenewalReminderLog]) -> rusqlite::Result<usize> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        // Ensure unique constraint exists for UPSERT conflict target
        let _ = tx.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_renewal_reminders_user_stage ON renewal_reminders(user_id, stage_label)"
        );
        let mut stmt = tx.prepare_cached(
            "INSERT INTO renewal_reminders (
                user_id, user_email, full_name, organization_name,
                stage_label, days_remaining, status, sent_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(user_id, stage_label) DO UPDATE SET
                days_remaining = excluded.days_remaining,
                status = excluded.status,
                sent_at = excluded.sent_at"
        )?;
        let mut affected = 0usize;
        for log in logs {
            affected += stmt.execute(params![
                log.user_id,
                log.user_email,
                log.full_name,
                log.organization_name,
                log.stage_label,
                log.days_remaining,
                log.status,
                log.sent_at
            ])?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(affected)
    }

}
