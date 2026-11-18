#[allow(unused_imports)]
use super::*;

impl Database {
    // =========================================================================

    /// Creates a new approval request for a content item and transitions its status to `pending_review`.
    /// Prevents duplicate pending requests for the same content item and captures `previous_en_status`.
    pub fn create_approval_request(&self, content_id: i64, requested_by: i64) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        let now = chrono::Utc::now().to_rfc3339();
        let prev_en_status: String = tx
            .query_row(
                "SELECT COALESCE(en_status, 'none') FROM contents WHERE id = ?1",
                rusqlite::params![content_id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "none".to_string());

        let existing_pending: Option<i64> = tx
            .query_row(
                "SELECT id FROM approval_requests WHERE content_id = ?1 AND status = 'pending' ORDER BY id DESC LIMIT 1",
                rusqlite::params![content_id],
                |r| r.get(0),
            )
            .ok();

        let id = if let Some(existing_id) = existing_pending {
            tx.execute(
                "UPDATE approval_requests SET requested_by = ?1, requested_at = ?2, previous_en_status = ?3 WHERE id = ?4",
                rusqlite::params![requested_by, now, prev_en_status, existing_id],
            )?;
            existing_id
        } else {
            tx.execute(
                "INSERT INTO approval_requests (
                    content_id, requested_by, requested_at, status, previous_en_status, source_action, target_en_status
                 ) VALUES (?1, ?2, ?3, 'pending', ?4, 'submit_review', '')",
                rusqlite::params![content_id, requested_by, now, prev_en_status],
            )?;
            tx.last_insert_rowid()
        };

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
                    COALESCE(u.email, ''), COALESCE(u.full_name, ''),
                    COALESCE(c.title_en, ''), COALESCE(c.summary_tr, ''), COALESCE(c.summary_en, ''),
                    COALESCE(c.en_status, 'none'), COALESCE(ar.previous_en_status, 'none'),
                    COALESCE(ar.source_action, 'submit_review')
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
                "requested_by_name": r.get::<_, String>(10)?,
                "title_en": r.get::<_, String>(11)?,
                "summary_tr": r.get::<_, String>(12)?,
                "summary_en": r.get::<_, String>(13)?,
                "en_status": r.get::<_, String>(14)?,
                "previous_en_status": r.get::<_, String>(15)?,
                "source_action": r.get::<_, String>(16)?
            }))
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    /// Updates an approval request (`approved` / `rejected`) and transitions the linked content item status.
    /// Enforces state-machine integrity: only `pending` requests can be decided, stamps `published_at`,
    /// and writes a symmetrical bilingual version snapshot on approval.
    pub fn update_approval_status(
        &self,
        id: i64,
        status: &str,
        reviewed_by: i64,
        review_note: Option<&str>,
    ) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        let (content_id, current_status, previous_en_status): (i64, String, String) = tx.query_row(
            "SELECT content_id, status, COALESCE(previous_en_status, 'none') FROM approval_requests WHERE id = ?1",
            rusqlite::params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if current_status != "pending" {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Bu onay talebi zaten '{}' durumunda karara bağlanmış.",
                current_status
            )));
        }
        let now = chrono::Utc::now().to_rfc3339();
        let pub_now = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
        let note_str = review_note.unwrap_or("");
        tx.execute(
            "UPDATE approval_requests SET status = ?1, reviewed_by = ?2, reviewed_at = ?3, review_note = ?4 WHERE id = ?5 AND status = 'pending'",
            rusqlite::params![status, reviewed_by, now, note_str, id],
        )?;
        let content_status = match status {
            "approved" => "published",
            "rejected" => "rejected",
            _ => "pending_review",
        };
        if status == "approved" {
            tx.execute(
                "UPDATE contents SET
                    status = ?1,
                    published_at = CASE WHEN TRIM(COALESCE(published_at, '')) = '' THEN ?2 ELSE published_at END
                 WHERE id = ?3",
                rusqlite::params![content_status, pub_now, content_id],
            )?;
            let reviewer_email: String = tx
                .query_row(
                    "SELECT email FROM users WHERE id = ?1",
                    rusqlite::params![reviewed_by],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| "reviewer".to_string());
            if let Ok((t_tr, t_en, b_tr, b_en, s_tr, s_en, cur_en_st)) = tx.query_row(
                "SELECT title_tr, title_en, body_tr, body_en, summary_tr, summary_en, COALESCE(en_status, 'none')
                 FROM contents WHERE id = ?1",
                rusqlite::params![content_id],
                |r| Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                )),
            ) {
                let max_ver: i64 = tx
                    .query_row(
                        "SELECT COALESCE(MAX(version), 0) FROM content_versions WHERE content_id = ?1",
                        rusqlite::params![content_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                let _ = tx.execute(
                    "INSERT INTO content_versions (
                        content_id, version, title_tr, title_en, body_tr, body_en, summary_tr, summary_en,
                        changed_by, change_note, created_at, en_status, previous_en_status, source_action, review_note
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Onay kuyruğundan yayımlandı', ?10, ?11, ?12, 'approval_approved', ?13)",
                    rusqlite::params![
                        content_id, max_ver + 1, t_tr, t_en, b_tr, b_en, s_tr, s_en,
                        reviewer_email, now, cur_en_st, previous_en_status, note_str
                    ],
                );
            }
        } else {
            tx.execute(
                "UPDATE contents SET status = ?1 WHERE id = ?2",
                rusqlite::params![content_status, content_id],
            )?;
        }
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
