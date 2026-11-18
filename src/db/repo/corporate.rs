#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_corporate_requests(&self) -> Vec<CorporateVerificationRequest> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, user_id, email, full_name, organization_category, organization_name,
                    request_message, status, reviewed_by, created_at
             FROM corporate_verification_requests
             ORDER BY id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(CorporateVerificationRequest {
                id: r.get(0)?,
                user_id: r.get(1)?,
                email: r.get(2)?,
                full_name: r.get(3)?,
                organization_category: r.get(4)?,
                organization_name: r.get(5)?,
                request_message: r.get(6)?,
                status: r.get(7)?,
                reviewed_by: r.get(8)?,
                created_at: r.get(9)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn insert_corporate_request(
        &self,
        req: &CorporateVerificationRequest,
    ) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO corporate_verification_requests (
                user_id, email, full_name, organization_category, organization_name,
                request_message, status, reviewed_by, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                req.user_id,
                req.email,
                req.full_name,
                req.organization_category,
                req.organization_name,
                req.request_message,
                req.status,
                req.reviewed_by,
                req.created_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn decide_corporate_request(
        &self,
        id: i64,
        status: &str,
        reviewed_by: &str,
    ) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE corporate_verification_requests SET status = ?1, reviewed_by = ?2 WHERE id = ?3",
            params![status, reviewed_by, id],
        )?;
        if status == "approved" {
            let dynamic_exp = (chrono::Utc::now() + chrono::Duration::days(365))
                .format("%Y-%m-%d")
                .to_string();
            if let Ok((user_id, email)) = conn.query_row(
                "SELECT user_id, email FROM corporate_verification_requests WHERE id = ?1",
                params![id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            ) {
                let _ = conn.execute(
                    "UPDATE users SET
                        role = CASE
                            WHEN role IN ('site_operator', 'admin', 'super_admin')
                              OR EXISTS (
                                  SELECT 1 FROM user_roles ur
                                  WHERE ur.role_code = users.role
                                    AND ur.role_code NOT IN ('registered', 'subscriber_public', 'subscriber_corporate', 'subscriber_print_digital', 'subscriber_digital')
                              )
                            THEN role
                            ELSE 'subscriber_public'
                        END,
                        is_gov_verified = 1,
                        subscription_tier = 'Kurumsal / Kamu Onaylı Ücretsiz Abone',
                        subscription_expires_at = ?3
                     WHERE id = ?1 OR LOWER(email) = LOWER(?2)",
                    params![user_id, email, dynamic_exp],
                );
            }
        }
        drop(conn);
        // If approved, users table was modified; refresh users section
        if status == "approved" {
            self.refresh_l1_sections(&["users"]);
        }
        Ok(())
    }

}
