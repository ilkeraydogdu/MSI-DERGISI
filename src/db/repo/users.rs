#[allow(unused_imports)]
use super::*;

impl Database {
    pub(super) fn load_users_db(&self) -> Vec<UserAccount> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT id, email, full_name, role, organization_category, organization_name,
                    job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                    reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                    email_consent, ssb_interests_json, created_at,
                    COALESCE(email_verified, 0),
                    COALESCE(marketing_emails_opted_in, 1),
                    CASE WHEN totp_secret IS NOT NULL AND totp_secret != '' THEN 1 ELSE 0 END,
                    COALESCE(tos_accepted_at, ''),
                    COALESCE(tos_version, '2026-01')
             FROM users ORDER BY id ASC",
            [],
            |r| {
                let ssb_raw: String = r.get(15)?;
                let ssb_interests: Vec<String> = serde_json::from_str(&ssb_raw).unwrap_or_default();
                Ok(UserAccount {
                    id: r.get(0)?,
                    email: r.get(1)?,
                    full_name: r.get(2)?,
                    role: r.get(3)?,
                    organization_category: r.get(4)?,
                    organization_name: r.get(5)?,
                    job_title: r.get(6)?,
                    is_gov_verified: r.get::<_, i32>(7)? != 0,
                    subscription_tier: r.get(8)?,
                    subscription_expires_at: r.get(9)?,
                    reminders_halted: r.get::<_, i32>(10)? != 0,
                    kvkk_consent_at: r.get(11)?,
                    kvkk_consent_ip: r.get(12)?,
                    kvkk_consent_hash: r.get(13)?,
                    email_consent: r.get::<_, i32>(14)? != 0,
                    ssb_interests,
                    created_at: r.get(16)?,
                    email_verified: r.get::<_, i32>(17)? != 0,
                    marketing_emails_opted_in: r.get::<_, i32>(18)? != 0,
                    totp_enabled: r.get::<_, i32>(19)? != 0,
                    tos_accepted_at: r.get(20)?,
                    tos_version: r.get(21)?,
                })
            },
        )
    }

    pub fn get_all_users(&self) -> Vec<UserAccount> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.users.clone();
            }
        }
        self.load_users_db()
    }

    // =========================================================================
    // Phase 3.2: User Analytics Aggregation Queries

    pub fn get_user_by_email(&self, email: &str) -> Option<(UserAccount, String)> {
        let norm = email.trim().to_lowercase();
        // Try L1 cache first for the account data
        let cached_user = if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                snap.users.iter().find(|u| u.email.to_lowercase() == norm).cloned()
            } else {
                None
            }
        } else {
            None
        };

        // Password hash always requires a targeted SQL query (not in cache)
        let pw_hash: Option<String> = crate::db::query_helpers::query_one(
            self,
            "SELECT password_hash FROM users WHERE LOWER(email) = ?1",
            params![norm.as_str()],
            |r| r.get(0),
        );

        match (cached_user, pw_hash) {
            (Some(user), Some(hash)) => Some((user, hash)),
            (None, Some(hash)) => {
                // Cache miss but user exists — load from DB
                let user_opt: Option<UserAccount> = crate::db::query_helpers::query_one(
                    self,
                    "SELECT id, email, full_name, role, organization_category, organization_name,
                            job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                            reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                            email_consent, ssb_interests_json, created_at,
                            COALESCE(email_verified, 0),
                            COALESCE(marketing_emails_opted_in, 1),
                            CASE WHEN totp_secret IS NOT NULL AND totp_secret != '' THEN 1 ELSE 0 END,
                            COALESCE(tos_accepted_at, ''),
                            COALESCE(tos_version, '2026-01')
                     FROM users WHERE LOWER(email) = ?1",
                    params![norm.as_str()],
                    |r| {
                        let ssb_raw: String = r.get(15)?;
                        let ssb_interests: Vec<String> = serde_json::from_str(&ssb_raw).unwrap_or_default();
                        Ok(UserAccount {
                            id: r.get(0)?,
                            email: r.get(1)?,
                            full_name: r.get(2)?,
                            role: r.get(3)?,
                            organization_category: r.get(4)?,
                            organization_name: r.get(5)?,
                            job_title: r.get(6)?,
                            is_gov_verified: r.get::<_, i32>(7)? != 0,
                            subscription_tier: r.get(8)?,
                            subscription_expires_at: r.get(9)?,
                            reminders_halted: r.get::<_, i32>(10)? != 0,
                            kvkk_consent_at: r.get(11)?,
                            kvkk_consent_ip: r.get(12)?,
                            kvkk_consent_hash: r.get(13)?,
                            email_consent: r.get::<_, i32>(14)? != 0,
                            ssb_interests,
                            created_at: r.get(16)?,
                            email_verified: r.get::<_, i32>(17)? != 0,
                            marketing_emails_opted_in: r.get::<_, i32>(18)? != 0,
                            totp_enabled: r.get::<_, i32>(19)? != 0,
                            tos_accepted_at: r.get(20)?,
                            tos_version: r.get(21)?,
                        })
                    },
                );
                user_opt.map(|u| (u, hash))
            }
            _ => None,
        }
    }

    pub fn get_user_by_id(&self, id: i64) -> Option<UserAccount> {
        self.get_all_users().into_iter().find(|u| u.id == id)
    }

    /// GDPR Data Portability: Collect all personal data for a user into a structured JSON object.
    /// Excludes sensitive credentials (`password_hash`, `totp_secret`, bearer tokens).
    pub fn export_user_data(&self, user_id: i64) -> Option<serde_json::Value> {
        let user = self.get_user_by_id(user_id)?;

        let orders: Vec<serde_json::Value> = match self.open_conn() {
            Ok(conn) => match conn.prepare(
                "SELECT invoice_no, item_type, item_code, item_title, amount_try, payment_status, created_at
                 FROM order_invoices WHERE LOWER(user_email) = LOWER(?1) ORDER BY created_at DESC",
            ) {
                Ok(mut stmt) => stmt
                    .query_map(params![user.email], |r| {
                        Ok(serde_json::json!({
                            "invoice_no": r.get::<_, String>(0)?,
                            "item_type": r.get::<_, String>(1)?,
                            "item_code": r.get::<_, String>(2)?,
                            "item_title": r.get::<_, String>(3)?,
                            "amount_try": r.get::<_, f64>(4)?,
                            "payment_status": r.get::<_, String>(5)?,
                            "created_at": r.get::<_, String>(6)?
                        }))
                    })
                    .map(|iter| iter.filter_map(Result::ok).collect())
                    .unwrap_or_default(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        };

        let purchased_codes = self.get_user_purchased_codes(&user.email);

        let analytics_summary: serde_json::Value = match self.open_conn() {
            Ok(conn) => {
                let total_events: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM analytics_events WHERE user_id = ?1",
                        params![user_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                let true_reads: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM analytics_events WHERE user_id = ?1 AND is_true_read = 1",
                        params![user_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                let avg_duration: f64 = conn
                    .query_row(
                        "SELECT COALESCE(AVG(duration_sec), 0) FROM analytics_events WHERE user_id = ?1",
                        params![user_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0.0);
                let unique_articles: i64 = conn
                    .query_row(
                        "SELECT COUNT(DISTINCT content_slug) FROM analytics_events WHERE user_id = ?1 AND content_slug IS NOT NULL",
                        params![user_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                serde_json::json!({
                    "total_events": total_events,
                    "true_reads": true_reads,
                    "avg_duration_sec": avg_duration,
                    "unique_articles_read": unique_articles
                })
            }
            Err(_) => serde_json::json!({}),
        };

        let corporate_requests: Vec<serde_json::Value> = match self.open_conn() {
            Ok(conn) => match conn.prepare(
                "SELECT organization_category, organization_name, request_message, status, created_at
                 FROM corporate_verification_requests WHERE user_id = ?1 ORDER BY created_at DESC",
            ) {
                Ok(mut stmt) => stmt
                    .query_map(params![user_id], |r| {
                        Ok(serde_json::json!({
                            "organization_category": r.get::<_, String>(0)?,
                            "organization_name": r.get::<_, String>(1)?,
                            "request_message": r.get::<_, String>(2)?,
                            "status": r.get::<_, String>(3)?,
                            "created_at": r.get::<_, String>(4)?
                        }))
                    })
                    .map(|iter| iter.filter_map(Result::ok).collect())
                    .unwrap_or_default(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        };

        let renewal_reminders: Vec<serde_json::Value> = match self.open_conn() {
            Ok(conn) => match conn.prepare(
                "SELECT stage_label, days_remaining, status, sent_at
                 FROM renewal_reminders WHERE user_id = ?1 ORDER BY sent_at DESC",
            ) {
                Ok(mut stmt) => stmt
                    .query_map(params![user_id], |r| {
                        Ok(serde_json::json!({
                            "stage_label": r.get::<_, String>(0)?,
                            "days_remaining": r.get::<_, i32>(1)?,
                            "status": r.get::<_, String>(2)?,
                            "sent_at": r.get::<_, String>(3)?
                        }))
                    })
                    .map(|iter| iter.filter_map(Result::ok).collect())
                    .unwrap_or_default(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        };

        let kvkk_logs = self.list_kvkk_consent_logs_for_user(user_id, &user.email);

        Some(serde_json::json!({
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "data_controller": "UMSA Prodüksiyon - MSI Dergisi",
            "gdpr_article": "KVKK Madde 11 / GDPR Article 20 - Veri Taşınabilirliği",
            "profile": {
                "email": user.email,
                "full_name": user.full_name,
                "role": user.role,
                "organization_category": user.organization_category,
                "organization_name": user.organization_name,
                "job_title": user.job_title,
                "is_gov_verified": user.is_gov_verified,
                "subscription_tier": user.subscription_tier,
                "subscription_expires_at": user.subscription_expires_at,
                "reminders_halted": user.reminders_halted,
                "ssb_interests": user.ssb_interests,
                "email_verified": user.email_verified,
                "totp_enabled": user.totp_enabled,
                "tos_accepted_at": user.tos_accepted_at,
                "tos_version": user.tos_version,
                "created_at": user.created_at
            },
            "consent_records": {
                "kvkk_consent_at": user.kvkk_consent_at,
                "kvkk_consent_ip": user.kvkk_consent_ip,
                "kvkk_consent_hash": user.kvkk_consent_hash,
                "tos_accepted_at": user.tos_accepted_at,
                "tos_version": user.tos_version,
                "email_consent": user.email_consent,
                "marketing_emails_opted_in": user.marketing_emails_opted_in,
                "kvkk_consent_logs": kvkk_logs
            },
            "orders": orders,
            "purchased_item_codes": purchased_codes,
            "analytics_summary": analytics_summary,
            "corporate_verification_requests": corporate_requests,
            "renewal_reminders": renewal_reminders
        }))
    }

    pub fn insert_kvkk_consent_log(&self, log: &crate::models::KvkkConsentLog) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let norm_email = log.email.trim().to_lowercase();
        let version = if log.consent_version.trim().is_empty() {
            "2026-01"
        } else {
            log.consent_version.trim()
        };
        let created_at = if log.created_at.trim().is_empty() {
            chrono::Utc::now().to_rfc3339()
        } else {
            log.created_at.trim().to_string()
        };
        let accepted_int: i32 = if log.accepted { 1 } else { 0 };
        conn.execute(
            "INSERT INTO kvkk_consent_logs (
                user_id, email, consent_type, consent_version, accepted, ip_address, consent_hash, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                log.user_id,
                norm_email,
                log.consent_type.trim(),
                version,
                accepted_int,
                log.ip_address.trim(),
                log.consent_hash.trim(),
                created_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_kvkk_consent_logs_for_user(
        &self,
        user_id: i64,
        email: &str,
    ) -> Vec<crate::models::KvkkConsentLog> {
        let norm_email = email.trim().to_lowercase();
        crate::db::query_helpers::query_all(
            self,
            "SELECT id, user_id, email, consent_type, consent_version, accepted, ip_address, consent_hash, created_at
             FROM kvkk_consent_logs
             WHERE user_id = ?1 OR (email != '' AND LOWER(email) = ?2)
             ORDER BY created_at DESC, id DESC
             LIMIT 100",
            params![user_id, norm_email],
            |r| {
                Ok(crate::models::KvkkConsentLog {
                    id: r.get(0)?,
                    user_id: r.get(1)?,
                    email: r.get(2)?,
                    consent_type: r.get(3)?,
                    consent_version: r.get(4)?,
                    accepted: r.get::<_, i32>(5)? != 0,
                    ip_address: r.get(6)?,
                    consent_hash: r.get(7)?,
                    created_at: r.get(8)?,
                })
            },
        )
    }

    pub fn create_or_update_user(
        &self,
        user: &UserAccount,
        password_hash: &str,
    ) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let ssb_json =
            serde_json::to_string(&user.ssb_interests).unwrap_or_else(|_| "[]".to_string());
        let norm_email = user.email.trim().to_lowercase();
        let tos_ver = if user.tos_version.trim().is_empty() {
            "2026-01"
        } else {
            user.tos_version.trim()
        };
        conn.execute(
            "INSERT INTO users (
                email, password_hash, full_name, role, organization_category, organization_name,
                job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                email_consent, ssb_interests_json, created_at, email_verified, marketing_emails_opted_in,
                tos_accepted_at, tos_version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
            ON CONFLICT(email) DO UPDATE SET
                password_hash = excluded.password_hash,
                full_name = excluded.full_name,
                role = excluded.role,
                organization_category = excluded.organization_category,
                organization_name = excluded.organization_name,
                job_title = excluded.job_title,
                is_gov_verified = excluded.is_gov_verified,
                subscription_tier = excluded.subscription_tier,
                subscription_expires_at = excluded.subscription_expires_at,
                reminders_halted = excluded.reminders_halted,
                email_consent = excluded.email_consent,
                ssb_interests_json = excluded.ssb_interests_json,
                email_verified = excluded.email_verified,
                marketing_emails_opted_in = excluded.marketing_emails_opted_in,
                tos_accepted_at = CASE WHEN excluded.tos_accepted_at != '' THEN excluded.tos_accepted_at ELSE users.tos_accepted_at END,
                tos_version = CASE WHEN excluded.tos_version != '' THEN excluded.tos_version ELSE users.tos_version END",
            params![
                norm_email,
                password_hash,
                user.full_name,
                user.role,
                user.organization_category,
                user.organization_name,
                user.job_title,
                if user.is_gov_verified { 1 } else { 0 },
                user.subscription_tier,
                user.subscription_expires_at,
                if user.reminders_halted { 1 } else { 0 },
                user.kvkk_consent_at,
                user.kvkk_consent_ip,
                user.kvkk_consent_hash,
                if user.email_consent { 1 } else { 0 },
                ssb_json,
                user.created_at,
                if user.email_verified { 1 } else { 0 },
                if user.marketing_emails_opted_in { 1 } else { 0 },
                user.tos_accepted_at,
                tos_ver
            ],
        )?;
        let uid: i64 = conn.query_row(
            "SELECT id FROM users WHERE LOWER(email) = ?1",
            params![norm_email],
            |r| r.get(0),
        )?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(uid)
    }

    pub fn update_user_profile(
        &self,
        user_id: i64,
        full_name: &str,
        organization_category: &str,
        organization_name: &str,
        job_title: &str,
        ssb_interests: &[String],
        email_consent: bool,
    ) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let ssb_json = serde_json::to_string(ssb_interests).unwrap_or_else(|_| "[]".to_string());
        let consent_int = if email_consent { 1 } else { 0 };
        conn.execute(
            "UPDATE users SET
                full_name = ?1,
                organization_category = ?2,
                organization_name = ?3,
                job_title = ?4,
                ssb_interests_json = ?5,
                email_consent = ?6,
                marketing_emails_opted_in = ?6
             WHERE id = ?7",
            params![
                full_name.trim(),
                organization_category.trim(),
                organization_name.trim(),
                job_title.trim(),
                ssb_json,
                consent_int,
                user_id
            ],
        )?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    pub fn update_user_password_hash(&self, email: &str, new_hash: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE users SET password_hash = ?1 WHERE LOWER(email) = LOWER(?2)",
            params![new_hash, email.trim()],
        )?;
        Ok(())
    }

    // =========================================================================
    // TOTP Two-Factor Authentication & Backup Codes
    // =========================================================================

    /// Returns the TOTP secret (base32-encoded) for a user, or None if 2FA is not enabled.
    pub fn get_user_totp_secret(&self, email: &str) -> Option<String> {
        let conn = self.open_conn().ok()?;
        conn.query_row(
            "SELECT totp_secret FROM users WHERE LOWER(email) = LOWER(?1) AND totp_secret IS NOT NULL AND totp_secret != ''",
            params![email.trim()],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|s| !s.trim().is_empty())
    }

    /// Saves or clears the TOTP secret for a user and refreshes L1 `users` cache.
    pub fn set_user_totp_secret(&self, email: &str, secret: Option<&str>) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let norm_secret = secret.map(|s| s.trim()).filter(|s| !s.is_empty());
        if let Some(sec) = norm_secret {
            conn.execute(
                "UPDATE users SET totp_secret = ?1 WHERE LOWER(email) = LOWER(?2)",
                params![sec, email.trim()],
            )?;
        } else {
            conn.execute(
                "UPDATE users SET totp_secret = NULL, totp_backup_codes_json = '[]' WHERE LOWER(email) = LOWER(?1)",
                params![email.trim()],
            )?;
        }
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    /// Retrieves the stored TOTP backup recovery codes (hashed or raw strings) for a user.
    pub fn get_user_totp_backup_codes(&self, email: &str) -> Vec<String> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let raw: String = conn
            .query_row(
                "SELECT COALESCE(totp_backup_codes_json, '[]') FROM users WHERE LOWER(email) = LOWER(?1)",
                params![email.trim()],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
    }

    /// Updates the stored TOTP backup recovery codes JSON array for a user.
    pub fn set_user_totp_backup_codes(&self, email: &str, codes: &[String]) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let json_str = serde_json::to_string(codes).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "UPDATE users SET totp_backup_codes_json = ?1 WHERE LOWER(email) = LOWER(?2)",
            params![json_str, email.trim()],
        )?;
        Ok(())
    }

    /// KVKK/GDPR Right to Erasure: soft-delete user account and purge personal data.
    /// Captures `original_email` BEFORE anonymizing `users.email`, anonymizes `order_invoices`
    /// for tax compliance, purges `user_purchased_items`, `corporate_verification_requests`,
    /// `password_reset_tokens`, `email_verification_tokens`, `unsubscribe_tokens`, `renewal_reminders`,
    /// and `analytics_events`, then refreshes L1 `users` cache.
    pub fn delete_user_account(&self, user_id: i64, client_ip: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;

        // 1. Capture original_email BEFORE overwriting users.email
        let original_email: String = tx.query_row(
            "SELECT email FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get(0),
        )?;

        let anonymized_email = format!("deleted_{}@erased.local", user_id);
        let now = chrono::Utc::now().to_rfc3339();

        // 2. Anonymize user record and wipe secrets
        tx.execute(
            "UPDATE users SET
                email = ?1,
                full_name = 'Silinmiş Kullanıcı',
                password_hash = '',
                organization_name = '',
                job_title = '',
                is_gov_verified = 0,
                kvkk_consent_ip = '',
                kvkk_consent_hash = '',
                ssb_interests_json = '[]',
                email_consent = 0,
                marketing_emails_opted_in = 0,
                email_verified = 0,
                totp_secret = NULL,
                totp_backup_codes_json = '[]',
                subscription_tier = '',
                subscription_expires_at = NULL,
                tos_accepted_at = '',
                role = 'deleted'
             WHERE id = ?2",
            params![anonymized_email, user_id],
        )?;

        // 2b. Anonymize PII in kvkk_consent_logs and append immutable erasure entry
        let _ = tx.execute(
            "UPDATE kvkk_consent_logs SET email = ?1, ip_address = '' WHERE user_id = ?2 OR LOWER(email) = LOWER(?3)",
            params![anonymized_email, user_id, original_email],
        );
        let _ = tx.execute(
            "INSERT INTO kvkk_consent_logs (user_id, email, consent_type, consent_version, accepted, ip_address, consent_hash, created_at)
             VALUES (?1, ?2, 'erasure', '2026-01', 1, '', ?3, ?4)",
            params![user_id, anonymized_email, format!("ERASURE-{}", user_id), now],
        );

        // 3. Anonymize order_invoices (retained for tax compliance) using original_email
        let _ = tx.execute(
            "UPDATE order_invoices SET
                user_email = ?1,
                full_name = 'Silinmiş Kullanıcı',
                organization_name = ''
             WHERE LOWER(user_email) = LOWER(?2) OR user_email = ?1",
            params![anonymized_email, original_email],
        );

        // 4. Purge analytics_events, reminders, purchased items, corporate requests, and all tokens
        let _ = tx.execute(
            "DELETE FROM analytics_events WHERE user_id = ?1",
            params![user_id],
        );
        let _ = tx.execute(
            "DELETE FROM renewal_reminders WHERE user_id = ?1 OR LOWER(user_email) = LOWER(?2)",
            params![user_id, original_email],
        );
        let _ = tx.execute(
            "DELETE FROM user_purchased_items WHERE LOWER(user_email) = LOWER(?1) OR LOWER(user_email) = LOWER(?2)",
            params![original_email, anonymized_email],
        );
        let _ = tx.execute(
            "DELETE FROM corporate_verification_requests WHERE user_id = ?1 OR LOWER(email) = LOWER(?2)",
            params![user_id, original_email],
        );
        let _ = tx.execute(
            "DELETE FROM password_reset_tokens WHERE user_id = ?1",
            params![user_id],
        );
        let _ = tx.execute(
            "DELETE FROM email_verification_tokens WHERE user_id = ?1",
            params![user_id],
        );
        let _ = tx.execute(
            "DELETE FROM unsubscribe_tokens WHERE user_id = ?1",
            params![user_id],
        );
        let _ = tx.execute(
            "DELETE FROM system_notifications WHERE target_email IS NOT NULL AND LOWER(target_email) = LOWER(?1)",
            params![original_email],
        );

        // 5. Audit trail entry
        let _ = tx.execute(
            "INSERT INTO security_audit_logs (client_ip, request_path, event_type, threat_level, details, blocked, created_at)
             VALUES (?1, '/api/v1/account/delete', 'ACCOUNT_DELETED', 'info', ?2, 0, ?3)",
            params![
                client_ip,
                format!("User ID {} account erased per KVKK/GDPR right to erasure", user_id),
                now
            ],
        );

        tx.commit()?;
        drop(conn);

        self.refresh_l1_sections(&["users", "notifications"]);
        Ok(())
    }

    pub fn admin_create_user(
        &self,
        email: &str,
        password_hash: &str,
        full_name: &str,
        role: &str,
        organization_category: &str,
        organization_name: &str,
        job_title: &str,
        is_gov_verified: bool,
        subscription_tier: &str,
    ) -> Result<i64, String> {
        let norm_email = email.trim().to_lowercase();
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let existing: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE LOWER(email) = ?1",
                params![norm_email],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if existing > 0 {
            return Err("Bu e-posta adresi ile kayıtlı bir kullanıcı zaten mevcut.".to_string());
        }
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO users (
                email, password_hash, full_name, role, organization_category, organization_name,
                job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                email_consent, ssb_interests_json, created_at, email_verified, marketing_emails_opted_in
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, 1, ?10, 'admin', 'ADMIN-CREATED', 1, '[]', ?10, 1, 1)",
            params![
                norm_email,
                password_hash,
                full_name.trim(),
                role.trim(),
                organization_category.trim(),
                organization_name.trim(),
                job_title.trim(),
                if is_gov_verified { 1 } else { 0 },
                subscription_tier.trim(),
                now
            ],
        ).map_err(|e| e.to_string())?;
        let uid = conn.last_insert_rowid();
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(uid)
    }

    pub fn admin_update_user(
        &self,
        user_id: i64,
        full_name: &str,
        role: Option<&str>,
        organization_category: &str,
        organization_name: &str,
        job_title: &str,
        is_gov_verified: bool,
        subscription_tier: &str,
        new_password_hash: Option<&str>,
    ) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let affected = conn
            .execute(
                "UPDATE users SET
                    full_name = ?1,
                    role = COALESCE(?2, role),
                    organization_category = ?3,
                    organization_name = ?4,
                    job_title = ?5,
                    is_gov_verified = ?6,
                    subscription_tier = ?7,
                    password_hash = COALESCE(?8, password_hash)
                 WHERE id = ?9",
                params![
                    full_name.trim(),
                    role.map(str::trim),
                    organization_category.trim(),
                    organization_name.trim(),
                    job_title.trim(),
                    if is_gov_verified { 1 } else { 0 },
                    subscription_tier.trim(),
                    new_password_hash,
                    user_id
                ],
            )
            .map_err(|e| e.to_string())?;
        drop(conn);
        if affected > 0 {
            self.refresh_l1_sections(&["users"]);
        }
        Ok(affected > 0)
    }

    pub fn admin_delete_user(&self, user_id: i64, actor_id: i64) -> Result<bool, String> {
        if user_id == actor_id {
            return Err("Kendi oturum açmış kullanıcı hesabınızı silemezsiniz.".to_string());
        }
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let target_role: String = conn
            .query_row("SELECT role FROM users WHERE id = ?1", params![user_id], |r| r.get(0))
            .map_err(|_| "Kullanıcı bulunamadı.".to_string())?;
        if matches!(target_role.as_str(), "site_operator" | "admin" | "super_admin") {
            let super_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM users WHERE role IN ('site_operator', 'admin', 'super_admin') AND id != ?1",
                    params![user_id],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if super_count == 0 {
                return Err("Sistemdeki son tam yetkili yönetici hesabı silinemez.".to_string());
            }
        }
        drop(conn);
        self.delete_user_account(user_id, "admin")
            .map(|_| true)
            .map_err(|e| e.to_string())
    }

    /// Returns complete subscriber entitlement inspection payload for Support / Admin views.
    pub fn get_user_entitlement_details(&self, user_id: i64) -> Option<serde_json::Value> {
        let user = self.get_user_by_id(user_id)?;
        let purchased_items = self.get_user_purchased_codes(&user.email);
        let invoices: Vec<serde_json::Value> = self
            .get_order_invoices()
            .into_iter()
            .filter(|inv| inv.user_email.eq_ignore_ascii_case(&user.email))
            .map(|inv| serde_json::json!(inv))
            .collect();
        let corporate_requests = self
            .get_corporate_requests_detailed()
            .into_iter()
            .filter(|r| r.get("user_id").and_then(|v| v.as_i64()) == Some(user_id))
            .collect::<Vec<_>>();
        Some(serde_json::json!({
            "user": user,
            "purchased_items": purchased_items,
            "invoices": invoices,
            "corporate_requests": corporate_requests,
        }))
    }

    /// Allows support/admin operators to manually toggle `email_verified` state for a user.
    pub fn admin_set_email_verified(&self, user_id: i64, verified: bool) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let affected = conn
            .execute(
                "UPDATE users SET email_verified = ?1 WHERE id = ?2",
                params![if verified { 1 } else { 0 }, user_id],
            )
            .map_err(|e| e.to_string())?;
        drop(conn);
        if affected > 0 {
            self.refresh_l1_sections(&["users"]);
        }
        Ok(affected > 0)
    }

    /// Allows support/admin operators to adjust subscription tier and expiry date cleanly.
    pub fn admin_adjust_subscription(
        &self,
        user_id: i64,
        tier: &str,
        expires_at: Option<&str>,
    ) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let affected = conn
            .execute(
                "UPDATE users SET subscription_tier = ?1, subscription_expires_at = ?2 WHERE id = ?3",
                params![tier.trim(), expires_at.map(str::trim).filter(|s| !s.is_empty()), user_id],
            )
            .map_err(|e| e.to_string())?;
        drop(conn);
        if affected > 0 {
            self.refresh_l1_sections(&["users"]);
        }
        Ok(affected > 0)
    }
}
