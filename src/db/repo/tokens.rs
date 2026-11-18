#[allow(unused_imports)]
use super::*;

impl Database {
    // =========================================================================

    pub fn create_password_reset_token(
        &self,
        user_id: i64,
        token: &str,
        expires_at: &str,
    ) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        let token_hash = crate::utils::crypto::sha256_hex(token.trim());
        conn.execute(
            "INSERT INTO password_reset_tokens (user_id, token, expires_at) VALUES (?1, ?2, ?3)",
            params![user_id, token_hash, expires_at],
        )
        .map_err(|e| format!("Failed to create reset token: {}", e))?;
        Ok(())
    }

    pub fn find_password_reset_token(&self, token: &str) -> Option<(i64, i64, String)> {
        let raw = token.trim();
        if raw.is_empty() {
            return None;
        }
        let token_hash = crate::utils::crypto::sha256_hex(raw);
        crate::db::query_helpers::query_one(
            self,
            "SELECT id, user_id, expires_at FROM password_reset_tokens
             WHERE (token = ?1 OR token = ?2) AND used = 0
             ORDER BY id DESC LIMIT 1",
            params![token_hash, raw],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
    }

    pub fn mark_reset_token_used(&self, token_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE password_reset_tokens SET used = 1 WHERE id = ?1",
            params![token_id],
        )
        .map_err(|e| format!("Failed to mark reset token used: {}", e))?;
        Ok(())
    }

    pub fn invalidate_user_reset_tokens(&self, user_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE password_reset_tokens SET used = 1 WHERE user_id = ?1 AND used = 0",
            params![user_id],
        )
        .map_err(|e| format!("Failed to invalidate user reset tokens: {}", e))?;
        Ok(())
    }

    // =========================================================================
    // KVKK/GDPR Unsubscribe Token Methods (010_unsubscribe_tokens.sql — SHA-256 + fallback)
    // =========================================================================

    pub fn create_unsubscribe_token(&self, user_id: i64, token: &str) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        let token_hash = crate::utils::crypto::sha256_hex(token.trim());
        conn.execute(
            "INSERT INTO unsubscribe_tokens (user_id, token) VALUES (?1, ?2)",
            params![user_id, token_hash],
        )
        .map_err(|e| format!("Failed to create unsubscribe token: {}", e))?;
        Ok(())
    }

    pub fn find_unsubscribe_token(&self, token: &str) -> Option<(i64, i64)> {
        let raw = token.trim();
        if raw.is_empty() {
            return None;
        }
        let token_hash = crate::utils::crypto::sha256_hex(raw);
        crate::db::query_helpers::query_one(
            self,
            "SELECT id, user_id FROM unsubscribe_tokens
             WHERE (token = ?1 OR token = ?2) AND used = 0
             ORDER BY id DESC LIMIT 1",
            params![token_hash, raw],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
    }

    pub fn mark_unsubscribe_token_used(&self, token_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE unsubscribe_tokens SET used = 1 WHERE id = ?1",
            params![token_id],
        )
        .map_err(|e| format!("Failed to mark unsubscribe token used: {}", e))?;
        Ok(())
    }

    pub fn opt_out_marketing_emails(&self, user_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE users SET marketing_emails_opted_in = 0, email_consent = 0 WHERE id = ?1",
            params![user_id],
        )
        .map_err(|e| format!("Failed to opt out marketing emails: {}", e))?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    pub fn opt_in_marketing_emails(&self, user_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE users SET marketing_emails_opted_in = 1, email_consent = 1 WHERE id = ?1",
            params![user_id],
        )
        .map_err(|e| format!("Failed to opt in marketing emails: {}", e))?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    pub fn get_user_email_preference(&self, user_id: i64) -> bool {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return true,
        };
        conn.query_row(
            "SELECT CASE WHEN COALESCE(marketing_emails_opted_in, 1) != 0 AND COALESCE(email_consent, 1) != 0 THEN 1 ELSE 0 END FROM users WHERE id = ?1",
            params![user_id],
            |r| r.get::<_, i32>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(true)
    }

    // =========================================================================
    // Email Verification Token Methods (009_email_verification.sql — SHA-256 + fallback)
    // =========================================================================

    pub fn create_email_verification_token(
        &self,
        user_id: i64,
        token: &str,
        expires_at: &str,
    ) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        let token_hash = crate::utils::crypto::sha256_hex(token.trim());
        conn.execute(
            "INSERT INTO email_verification_tokens (user_id, token, expires_at) VALUES (?1, ?2, ?3)",
            params![user_id, token_hash, expires_at],
        )
        .map_err(|e| format!("Failed to create verification token: {}", e))?;
        Ok(())
    }

    pub fn find_email_verification_token(&self, token: &str) -> Option<(i64, i64, String)> {
        let raw = token.trim();
        if raw.is_empty() {
            return None;
        }
        let token_hash = crate::utils::crypto::sha256_hex(raw);
        crate::db::query_helpers::query_one(
            self,
            "SELECT id, user_id, expires_at FROM email_verification_tokens
             WHERE (token = ?1 OR token = ?2) AND verified = 0
             ORDER BY id DESC LIMIT 1",
            params![token_hash, raw],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
    }

    pub fn mark_email_verified(&self, user_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE users SET email_verified = 1 WHERE id = ?1",
            params![user_id],
        )
        .map_err(|e| format!("Failed to mark email verified: {}", e))?;
        drop(conn);
        self.refresh_l1_sections(&["users"]);
        Ok(())
    }

    pub fn mark_verification_token_used(&self, token_id: i64) -> Result<(), String> {
        let conn = self
            .open_conn()
            .map_err(|e| format!("DB connection error: {}", e))?;
        conn.execute(
            "UPDATE email_verification_tokens SET verified = 1 WHERE id = ?1",
            params![token_id],
        )
        .map_err(|e| format!("Failed to mark verification token used: {}", e))?;
        Ok(())
    }

}
