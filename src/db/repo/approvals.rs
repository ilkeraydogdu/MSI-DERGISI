#[allow(unused_imports)]
use super::*;

impl Database {
    // =========================================================================

    /// Creates a new approval request for a content item and transitions its status to `pending_review`.
    pub fn create_approval_request(&self, content_id: i64, requested_by: i64) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO approval_requests (content_id, requested_by, requested_at, status) VALUES (?1, ?2, ?3, 'pending')",
            rusqlite::params![content_id, requested_by, now],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "UPDATE contents SET status = 'pending_review' WHERE id = ?1",
            rusqlite::params![content_id],
        )?;
        tx.commit()?;
        drop(conn);
        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);
        Ok(id)
    }

    /// Lists all pending approval requests joined with content metadata and requester info.
    pub fn list_pending_approvals(&self) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut stmt = match conn.prepare(
            "SELECT ar.id, ar.content_id, ar.requested_by, ar.requested_at, ar.status,
                    c.title_tr, c.author_name, c.content_type, c.slug,
                    COALESCE(u.email, ''), COALESCE(u.full_name, '')
             FROM approval_requests ar
             JOIN contents c ON c.id = ar.content_id
             LEFT JOIN users u ON u.id = ar.requested_by
             WHERE ar.status = 'pending'
             ORDER BY ar.requested_at DESC",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "content_id": r.get::<_, i64>(1)?,
                "requested_by": r.get::<_, i64>(2)?,
                "requested_at": r.get::<_, String>(3)?,
                "status": r.get::<_, String>(4)?,
                "title_tr": r.get::<_, String>(5)?,
                "author_name": r.get::<_, String>(6)?,
                "content_type": r.get::<_, String>(7)?,
                "slug": r.get::<_, String>(8)?,
                "requested_by_email": r.get::<_, String>(9)?,
                "requested_by_name": r.get::<_, String>(10)?
            }))
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    /// Updates an approval request (`approved` / `rejected`) and transitions the linked content item status.
    /// Enforces state-machine integrity: only `pending` requests can be decided.
    pub fn update_approval_status(
        &self,
        id: i64,
        status: &str,
        reviewed_by: i64,
        review_note: Option<&str>,
    ) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        let (content_id, current_status): (i64, String) = tx.query_row(
            "SELECT content_id, status FROM approval_requests WHERE id = ?1",
            rusqlite::params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if current_status != "pending" {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Bu onay talebi zaten '{}' durumunda karara bağlanmış.",
                current_status
            )));
        }
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE approval_requests SET status = ?1, reviewed_by = ?2, reviewed_at = ?3, review_note = ?4 WHERE id = ?5 AND status = 'pending'",
            rusqlite::params![status, reviewed_by, now, review_note.unwrap_or(""), id],
        )?;
        let content_status = match status {
            "approved" => "published",
            "rejected" => "rejected",
            _ => "pending_review",
        };
        tx.execute(
            "UPDATE contents SET status = ?1 WHERE id = ?2",
            rusqlite::params![content_status, content_id],
        )?;
        tx.commit()?;
        drop(conn);
        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);
        Ok(())
    }

    /// Updates the editorial status of a content item directly and invalidates L1/L2 content caches.
    pub fn update_content_status(&self, content_id: i64, status: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE contents SET status = ?1 WHERE id = ?2",
            rusqlite::params![status.trim(), content_id],
        )?;
        drop(conn);
        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);
        Ok(())
    }

    // =========================================================================
    // MONITORING SYSTEM DB METHODS (007_monitoring_schema.sql)
}
