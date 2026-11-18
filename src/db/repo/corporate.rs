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

    pub fn get_corporate_requests_detailed(&self) -> Vec<serde_json::Value> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, user_id, email, full_name, organization_category, organization_name,
                    request_message, status, reviewed_by, COALESCE(decision_note, ''), created_at
             FROM corporate_verification_requests
             ORDER BY id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "user_id": r.get::<_, i64>(1)?,
                "email": r.get::<_, String>(2)?,
                "full_name": r.get::<_, String>(3)?,
                "organization_category": r.get::<_, String>(4)?,
                "organization_name": r.get::<_, String>(5)?,
                "request_message": r.get::<_, String>(6)?,
                "status": r.get::<_, String>(7)?,
                "reviewed_by": r.get::<_, Option<String>>(8)?,
                "decision_note": r.get::<_, String>(9)?,
                "created_at": r.get::<_, String>(10)?,
            }))
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn decide_corporate_request(
        &self,
        id: i64,
        status: &str,
        reviewed_by: &str,
    ) -> rusqlite::Result<()> {
        self.decide_corporate_request_with_note(id, status, reviewed_by, "")
    }

    pub fn decide_corporate_request_with_note(
        &self,
        id: i64,
        status: &str,
        reviewed_by: &str,
        decision_note: &str,
    ) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE corporate_verification_requests
             SET status = ?1, reviewed_by = ?2, decision_note = ?3
             WHERE id = ?4",
            params![status, reviewed_by, decision_note.trim(), id],
        )?;
        if status == "approved" {
            let dynamic_exp = (chrono::Utc::now() + chrono::Duration::days(365))
                .format("%Y-%m-%d")
                .to_string();
            if let Ok((user_id, email, org_cat)) = conn.query_row(
                "SELECT user_id, email, organization_category FROM corporate_verification_requests WHERE id = ?1",
                params![id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
            ) {
                let is_public_sector = self
                    .resolve_institution_category(&org_cat)
                    .map(|c| c.is_public_sector)
                    .unwrap_or_else(|| {
                        matches!(
                            org_cat.trim(),
                            "MSB" | "SSB" | "K.K.K.lığı" | "Dz.K.K.lığı" | "Hv.K.K.lığı"
                            | "J.G.K.lığı" | "EGM" | "S.G.K.lığı" | "MİT" | "Diğer Kamu Kurumları"
                        )
                    });
                let (assigned_role, gov_flag, tier_label) = if is_public_sector {
                    ("subscriber_public", 1i32, "Kamu Onaylı Ücretsiz Abone")
                } else {
                    ("subscriber_corporate", 0i32, "Kurumsal Doğrulanmış Üye")
                };
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
                            ELSE ?1
                        END,
                        is_gov_verified = CASE WHEN ?2 = 1 THEN 1 ELSE is_gov_verified END,
                        subscription_tier = ?3,
                        subscription_expires_at = ?4
                     WHERE id = ?5 OR LOWER(email) = LOWER(?6)",
                    params![assigned_role, gov_flag, tier_label, dynamic_exp, user_id, email],
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
