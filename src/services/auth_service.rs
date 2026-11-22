use std::collections::HashMap;
use std::sync::RwLock;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::Engine as _;
use chrono::Utc;
use hmac::{Hmac, Mac};
use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};

use crate::models::UserAccount;
use crate::utils::crypto::constant_time_eq;

type HmacSha256 = Hmac<Sha256>;

/// Time-to-live (in seconds) for dedicated TOTP 2FA pending challenge tokens (5 minutes).
const TOTP_PENDING_TTL_SECS: i64 = 300;

/// In-memory L1 cache of SHA-256 revoked token hashes -> expiration unix timestamp.
/// Overflow evicts ONLY already-expired entries (never clears active revocations),
/// and SQLite `revoked_tokens` serves as the durable backing store across restarts.
static REVOKED_TOKEN_HASHES: Lazy<RwLock<HashMap<String, i64>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Legacy seed secret retained strictly for transparent migration of pre-Argon2id hashes on login.
fn legacy_seed_secret() -> String {
    std::env::var("LEGACY_SEED_SECRET")
        .or_else(|_| std::env::var("SESSION_SECRET"))
        .unwrap_or_else(|_| "CHANGE-ME-LEGACY-SEED-SECRET".to_string())
}

/// Returns the legacy seed secret from the centralized `AppConfig` when available.
pub fn legacy_seed_secret_from_config(config: &crate::config::AppConfig) -> String {
    config.legacy_seed_secret.clone()
}

/// Public sector email suffixes eligible for automatic free E-Dergi subscription (`subscriber_public`).
const PUBLIC_SECTOR_DOMAINS: &[&str] = &[
    ".gov.tr",
    ".tsk.tr",
    ".mil.tr",
    ".pol.tr",
    ".jandarma.gov.tr",
    ".msb.gov.tr",
    ".ssb.gov.tr",
    ".sahilguvenlik.gov.tr",
    ".mit.gov.tr",
    ".egm.gov.tr",
];

/// Personal/free email providers that require manual corporate verification if a user selects a corporate/public institution.
const PERSONAL_EMAIL_DOMAINS: &[&str] = &[
    "gmail.com",
    "hotmail.com",
    "outlook.com",
    "yahoo.com",
    "icloud.com",
    "yandex.com",
    "yandex.com.tr",
    "windowslive.com",
    "live.com",
    "protonmail.com",
    "proton.me",
];

/// Detects `.gov.tr`, `.tsk.tr`, `.mil.tr`, `.pol.tr`, `.jandarma.gov.tr`, `.msb.gov.tr`, `.ssb.gov.tr`
/// domains for automatic free public-sector E-Dergi subscription (`subscriber_public`).
pub fn is_public_sector_email(email: &str) -> bool {
    let normalized = email.trim().to_lowercase();
    let domain = match normalized.split('@').nth(1) {
        Some(d) => d.trim(),
        None => return false,
    };

    PUBLIC_SECTOR_DOMAINS.iter().any(|suffix| {
        let bare = suffix.trim_start_matches('.');
        domain == bare || domain.ends_with(suffix)
    })
}

/// Detects `.edu.tr` academic domains for academic/student verification.
pub fn is_academic_email(email: &str) -> bool {
    let normalized = email.trim().to_lowercase();
    match normalized.split('@').nth(1) {
        Some(domain) => domain == "edu.tr" || domain.ends_with(".edu.tr"),
        None => false,
    }
}

/// Determines whether a registration requires operator verification (`CorporateVerificationRequest`)
/// when a user claims an official institution or defense OEM (`Sektör Kuruluşları`) using a non-gov personal email.
pub fn requires_corporate_verification(email: &str, org_category: &str) -> bool {
    if is_public_sector_email(email) {
        return false;
    }
    let normalized = email.trim().to_lowercase();
    let domain = normalized.split('@').nth(1).unwrap_or("");
    let is_personal = PERSONAL_EMAIL_DOMAINS.contains(&domain);

    let claimed_public_or_sector = matches!(
        org_category,
        "MSB"
            | "SSB"
            | "K.K.K.lığı"
            | "Dz.K.K.lığı"
            | "Hv.K.K.lığı"
            | "J.G.K.lığı"
            | "EGM"
            | "S.G.K.lığı"
            | "MİT"
            | "Diğer Kamu Kurumları"
            | "Sektör Kuruluşları"
    );

    is_personal && claimed_public_or_sector
}

/// Hashes a password using Argon2id with a unique 128-bit cryptographic salt.
pub fn hash_password(password: &str, secret: &str) -> String {
    let random_bytes = uuid::Uuid::new_v4();
    if let Ok(salt) = SaltString::encode_b64(random_bytes.as_bytes()) {
        if let Ok(phc) = Argon2::default().hash_password(password.as_bytes(), &salt) {
            return phc.to_string();
        }
    }
    legacy_sha256_password_hash(password, secret)
}

pub fn legacy_sha256_password_hash(password: &str, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"MSI-UMSA-AUTH-V2:");
    hasher.update(password.as_bytes());
    hasher.update(b":");
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

/// Returns true when the stored password hash already uses Argon2id PHC encoding.
pub fn is_argon2_hash(stored_hash: &str) -> bool {
    stored_hash.starts_with("$argon2")
}

/// Verifies a password against an Argon2id PHC string (or legacy SHA-256 hex digest during migration).
/// Supports both `(password, secret, expected_hash)` and `(password, expected_hash, secret)` parameter orders safely.
pub fn verify_password(password: &str, secret_or_hash: &str, hash_or_secret: &str) -> bool {
    if is_argon2_hash(hash_or_secret) {
        return PasswordHash::new(hash_or_secret)
            .ok()
            .map(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
            .unwrap_or(false);
    }
    if is_argon2_hash(secret_or_hash) {
        return PasswordHash::new(secret_or_hash)
            .ok()
            .map(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
            .unwrap_or(false);
    }

    let candidate_a = legacy_sha256_password_hash(password, secret_or_hash);
    if constant_time_eq(candidate_a.as_bytes(), hash_or_secret.as_bytes()) {
        return true;
    }
    let candidate_b = legacy_sha256_password_hash(password, hash_or_secret);
    if constant_time_eq(candidate_b.as_bytes(), secret_or_hash.as_bytes()) {
        return true;
    }

    // Transparent verification for initial seed hashes before automatic Argon2id upgrade on login
    let seed_candidate = legacy_sha256_password_hash(password, &legacy_seed_secret());
    if hash_or_secret.len() == 64
        && constant_time_eq(seed_candidate.as_bytes(), hash_or_secret.as_bytes())
    {
        return true;
    }
    if secret_or_hash.len() == 64
        && constant_time_eq(seed_candidate.as_bytes(), secret_or_hash.as_bytes())
    {
        return true;
    }

    false
}

/// Produces a tamper-evident KVKK timestamped + IP cryptographic consent hash.
pub fn generate_kvkk_consent_hash(email: &str, ip: &str, timestamp: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts arbitrary key lengths");
    mac.update(b"KVKK-CONSENT-PROOF:");
    mac.update(email.trim().to_lowercase().as_bytes());
    mac.update(b"|IP:");
    mac.update(ip.trim().as_bytes());
    mac.update(b"|TS:");
    mac.update(timestamp.trim().as_bytes());
    let digest = hex::encode(mac.finalize().into_bytes()).to_uppercase();
    format!("KVKK-2026-{}", &digest[..24])
}

/// Generates a cryptographically signed session token for the given `UserAccount`.
/// Format: `{user_id}.{role}.{issued_ts}.{jti}.{hmac_hex}` (5 segments).
pub fn generate_session_token(user: &UserAccount, secret: &str) -> String {
    let issued_ts = Utc::now().timestamp();
    let jti = uuid::Uuid::new_v4().simple().to_string();
    let payload = format!("{}.{}.{}.{}", user.id, user.role, issued_ts, &jti[..16]);
    let sig = sign_session_payload(&payload, secret);
    format!("{}.{}", payload, sig)
}

fn extract_token_expiry_ts(trimmed: &str) -> i64 {
    let now = Utc::now().timestamp();
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() == 5 {
        if let Ok(issued_ts) = parts[2].parse::<i64>() {
            return issued_ts.saturating_add(86_400);
        }
    } else if parts.len() == 4 {
        if let Ok(issued_ts) = parts[1].parse::<i64>() {
            return issued_ts.saturating_add(TOTP_PENDING_TTL_SECS);
        }
    }
    now.saturating_add(86_400)
}

fn cache_revoked_token_hash(token_hash: String, expires_ts: i64) {
    if let Ok(mut map) = REVOKED_TOKEN_HASHES.write() {
        if map.len() >= 10_000 {
            let now = Utc::now().timestamp();
            map.retain(|_, exp| *exp > now);
        }
        map.insert(token_hash, expires_ts);
    }
}

/// Clears only the in-memory revocation cache (useful for testing DB persistence across restarts).
pub fn clear_in_memory_revocation_cache() {
    if let Ok(mut map) = REVOKED_TOKEN_HASHES.write() {
        map.clear();
    }
}

/// Revokes a session (or pending 2FA) token in memory so it can no longer be used for authentication.
pub fn revoke_session_token(token: &str) {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return;
    }
    let token_hash = crate::utils::crypto::sha256_hex(trimmed);
    let expires_ts = extract_token_expiry_ts(trimmed);
    cache_revoked_token_hash(token_hash, expires_ts);
}

/// Revokes a session (or pending 2FA) token persistently in SQLite (`revoked_tokens`)
/// and updates the in-memory hash cache, opportunistically pruning expired rows.
pub fn revoke_session_token_db(db: &crate::db::Database, token: &str) {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return;
    }
    let token_hash = crate::utils::crypto::sha256_hex(trimmed);
    let expires_ts = extract_token_expiry_ts(trimmed);
    cache_revoked_token_hash(token_hash.clone(), expires_ts);

    let expires_at = chrono::DateTime::<Utc>::from_timestamp(expires_ts, 0)
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let _ = db.revoke_token_hash(&token_hash, &expires_at);
    let _ = db.cleanup_expired_revoked_tokens();
}

/// Returns `true` if the given token has been explicitly revoked in the in-memory cache.
pub fn is_token_revoked(token: &str) -> bool {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return false;
    }
    let token_hash = crate::utils::crypto::sha256_hex(trimmed);
    REVOKED_TOKEN_HASHES
        .read()
        .map(|map| map.contains_key(&token_hash))
        .unwrap_or(false)
}

/// Returns `true` if the given token has been revoked in memory or in SQLite `revoked_tokens`.
pub fn is_token_revoked_db(db: &crate::db::Database, token: &str) -> bool {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return false;
    }
    if is_token_revoked(trimmed) {
        return true;
    }
    let token_hash = crate::utils::crypto::sha256_hex(trimmed);
    if db.is_token_hash_revoked(&token_hash) {
        cache_revoked_token_hash(token_hash, extract_token_expiry_ts(trimmed));
        return true;
    }
    false
}

/// Returns `true` when the account has been soft-deleted / KVKK-erased and must
/// never be allowed to authenticate (including via pre-existing sibling cookies).
pub fn is_deleted_or_erased_user(user: &UserAccount) -> bool {
    let role = user.role.trim();
    let email = user.email.trim();
    role.eq_ignore_ascii_case("deleted")
        || email.is_empty()
        || email.to_ascii_lowercase().ends_with("@erased.local")
}

/// Verifies a signed session token against both in-memory cache and SQLite `revoked_tokens`.
pub fn verify_session_token_db(db: &crate::db::Database, token: &str, secret: &str) -> Option<i64> {
    if is_token_revoked_db(db, token) {
        return None;
    }
    let uid = verify_session_token(token, secret)?;
    if let Some(user) = db.get_user_by_id(uid) {
        if is_deleted_or_erased_user(&user) {
            return None;
        }
    }
    Some(uid)
}

/// Verifies a signed TOTP pending token against both in-memory cache and SQLite `revoked_tokens`.
pub fn verify_totp_pending_token_db(
    db: &crate::db::Database,
    token: &str,
    secret: &str,
) -> Option<String> {
    if is_token_revoked_db(db, token) {
        return None;
    }
    verify_totp_pending_token(token, secret)
}

/// Verifies a signed session token and returns `Some(user_id)` if authentic, untampered,
/// unexpired (`[-60s, +86400s]`), and not revoked.
pub fn verify_session_token(token: &str, secret: &str) -> Option<i64> {
    let trimmed = token.trim();
    if trimmed.is_empty() || is_token_revoked(trimmed) {
        return None;
    }
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() != 5 {
        return None;
    }
    let user_id: i64 = parts[0].parse().ok()?;
    let role = parts[1];
    let issued_ts: i64 = parts[2].parse().ok()?;
    let jti = parts[3];
    if jti.is_empty() {
        return None;
    }
    let age = Utc::now().timestamp().checked_sub(issued_ts)?;
    if !(-60..=86400).contains(&age) {
        return None;
    }
    let provided_sig = parts[4];

    let payload = format!("{}.{}.{}.{}", user_id, role, issued_ts, jti);
    let expected_sig = sign_session_payload(&payload, secret);

    if constant_time_eq(provided_sig.as_bytes(), expected_sig.as_bytes()) {
        Some(user_id)
    } else {
        None
    }
}

/// Generates a dedicated, short-lived (`300s` TTL) TOTP 2FA challenge token bound to `b"MSI-TOTP-PENDING-V1:"`.
/// Format: `{email_b64url}.{issued_ts}.{jti}.{hmac_hex}` (4 segments — structurally & cryptographically
/// incompatible with 5-segment `msi_session` cookies so `temp_token` can NEVER bypass 2FA).
pub fn generate_totp_pending_token(user_email: &str, secret: &str) -> String {
    let normalized_email = user_email.trim().to_lowercase();
    let email_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(normalized_email.as_bytes());
    let issued_ts = Utc::now().timestamp();
    let jti = uuid::Uuid::new_v4().simple().to_string();
    let payload = format!("{}.{}.{}", email_b64, issued_ts, &jti[..16]);
    let sig = sign_totp_pending_payload(&payload, secret);
    format!("{}.{}", payload, sig)
}

/// Verifies a dedicated TOTP pending token (`b"MSI-TOTP-PENDING-V1:"`, `300s` TTL) and returns
/// `Some(normalized_email)` if valid and unrevoked.
pub fn verify_totp_pending_token(token: &str, secret: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() || is_token_revoked(trimmed) {
        return None;
    }
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let email_b64 = parts[0];
    if email_b64.is_empty() {
        return None;
    }
    let issued_ts: i64 = parts[1].parse().ok()?;
    let jti = parts[2];
    if jti.is_empty() {
        return None;
    }
    let age = Utc::now().timestamp().checked_sub(issued_ts)?;
    if !(-30..=TOTP_PENDING_TTL_SECS).contains(&age) {
        return None;
    }
    let provided_sig = parts[3];

    let payload = format!("{}.{}.{}", email_b64, issued_ts, jti);
    let expected_sig = sign_totp_pending_payload(&payload, secret);
    if !constant_time_eq(provided_sig.as_bytes(), expected_sig.as_bytes()) {
        return None;
    }

    let email_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(email_b64.as_bytes())
        .ok()?;
    let email = String::from_utf8(email_bytes).ok()?;
    if !email.contains('@') {
        return None;
    }
    Some(email)
}

/// Identity is resolved from a signed, HttpOnly session cookie, never a query parameter.
pub fn session_user(
    state: &crate::controllers::AppState,
    headers: &axum::http::HeaderMap,
) -> Option<UserAccount> {
    let token = headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|cookies| cookies.split(';'))
        .find_map(|p| {
            p.trim()
                .strip_prefix("msi_session=")
                .map(str::trim)
                .filter(|t| !t.is_empty())
        })?;
    let user_id = verify_session_token_db(&state.db, token, &state.config.session_secret)?;
    let user = state.db.get_user_by_id(user_id)?;
    if is_deleted_or_erased_user(&user) {
        return None;
    }
    Some(user)
}

/// Determines the initial `(role, subscription_tier, is_gov_verified)` tuple for a newly registering user.
pub fn determine_initial_role_and_tier(email: &str, org_category: &str) -> (String, String, bool) {
    if is_public_sector_email(email) {
        return (
            "subscriber_public".to_string(),
            "Kamu E-Dergi Abonesi (Ücretsiz)".to_string(),
            true,
        );
    }

    if is_academic_email(email) || org_category == "Üniversite / Öğrenci" {
        return (
            "registered".to_string(),
            "Akademik / Öğrenci Okur".to_string(),
            false,
        );
    }

    (
        "registered".to_string(),
        "Standart Kayıtlı Okur".to_string(),
        false,
    )
}

fn sign_session_payload(payload: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts arbitrary key lengths");
    mac.update(b"MSI-SESSION-SIG-V3:");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn sign_totp_pending_payload(payload: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts arbitrary key lengths");
    mac.update(b"MSI-TOTP-PENDING-V1:");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Generates a cryptographically secure password reset token and stores it in the database.
/// Token format: base64url(SHA-256(random_uuid + timestamp + secret))
/// TTL: 1 hour from creation.
pub fn generate_password_reset_token(
    db: &crate::db::Database,
    user_id: i64,
    secret: &str,
) -> Result<String, String> {
    let random_bytes = uuid::Uuid::new_v4();
    let timestamp = Utc::now().timestamp();

    let mut hasher = Sha256::new();
    hasher.update(b"MSI-PASSWORD-RESET-V1:");
    hasher.update(random_bytes.as_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(secret.as_bytes());
    let digest = hasher.finalize();

    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    let expires_at = (Utc::now() + chrono::Duration::hours(1))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let created_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let conn = db
        .open_conn()
        .map_err(|e| format!("DB bağlantı hatası: {}", e))?;
    conn.execute(
        "INSERT INTO password_reset_tokens (user_id, token, expires_at, used, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
        rusqlite::params![user_id, &token, &expires_at, &created_at],
    )
    .map_err(|e| format!("Token kaydetme hatası: {}", e))?;

    Ok(token)
}

/// Verifies a password reset token: must exist, not used, not expired.
/// Returns Some(user_id) if valid, None otherwise.
pub fn verify_password_reset_token(db: &crate::db::Database, token: &str) -> Option<i64> {
    let conn = db.open_conn().ok()?;
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    conn.query_row(
        "SELECT user_id FROM password_reset_tokens WHERE token = ?1 AND used = 0 AND expires_at > ?2",
        rusqlite::params![token.trim(), &now],
        |row| row.get::<_, i64>(0),
    )
    .ok()
}

/// Marks a password reset token as used after successful password change.
pub fn consume_password_reset_token(db: &crate::db::Database, token: &str) -> Result<(), String> {
    let conn = db
        .open_conn()
        .map_err(|e| format!("DB bağlantı hatası: {}", e))?;
    let affected = conn
        .execute(
            "UPDATE password_reset_tokens SET used = 1 WHERE token = ?1 AND used = 0",
            rusqlite::params![token.trim()],
        )
        .map_err(|e| format!("Token tüketme hatası: {}", e))?;

    if affected == 0 {
        return Err("Geçersiz veya zaten kullanılmış token.".to_string());
    }
    Ok(())
}

/// Generates an email verification token and stores it in the database.
/// TTL: 24 hours from creation.
pub fn generate_email_verification_token(
    db: &crate::db::Database,
    user_id: i64,
    email: &str,
    secret: &str,
) -> Result<String, String> {
    let random_bytes = uuid::Uuid::new_v4();
    let timestamp = Utc::now().timestamp();

    let mut hasher = Sha256::new();
    hasher.update(b"MSI-EMAIL-VERIFY-V1:");
    hasher.update(random_bytes.as_bytes());
    hasher.update(b"|");
    hasher.update(email.trim().to_lowercase().as_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(secret.as_bytes());
    let digest = hasher.finalize();

    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    let expires_at = (Utc::now() + chrono::Duration::hours(24))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let created_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let conn = db
        .open_conn()
        .map_err(|e| format!("DB bağlantı hatası: {}", e))?;
    conn.execute(
        "INSERT INTO email_verification_tokens (user_id, token, expires_at, verified, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
        rusqlite::params![user_id, &token, &expires_at, &created_at],
    )
    .map_err(|e| format!("Verification token kaydetme hatası: {}", e))?;

    Ok(token)
}

/// Verifies an email verification token and marks the user's email as verified.
/// Returns Some(user_id) if valid and successfully applied, None otherwise.
pub fn verify_email_token(db: &crate::db::Database, token: &str) -> Option<i64> {
    let conn = db.open_conn().ok()?;
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let user_id: i64 = conn
        .query_row(
            "SELECT user_id FROM email_verification_tokens WHERE token = ?1 AND verified = 0 AND expires_at > ?2",
            rusqlite::params![token.trim(), &now],
            |row| row.get(0),
        )
        .ok()?;

    let _ = conn.execute(
        "UPDATE email_verification_tokens SET verified = 1 WHERE token = ?1",
        rusqlite::params![token.trim()],
    );

    let _ = conn.execute(
        "UPDATE users SET email_verified = 1 WHERE id = ?1",
        rusqlite::params![user_id],
    );

    drop(conn);
    db.refresh_l1_sections(&["users"]);

    Some(user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user() -> UserAccount {
        UserAccount {
            id: 42,
            email: "operator@msidergisi.com".to_string(),
            full_name: "Test Operator".to_string(),
            role: "site_operator".to_string(),
            organization_category: "SSB".to_string(),
            organization_name: "SSB".to_string(),
            job_title: "Uzman".to_string(),
            is_gov_verified: true,
            subscription_tier: "Kurumsal Tam Yetki".to_string(),
            subscription_expires_at: Some("2030-12-31".to_string()),
            reminders_halted: false,
            kvkk_consent_at: "2026-01-01".to_string(),
            kvkk_consent_ip: "127.0.0.1".to_string(),
            kvkk_consent_hash: "KVKK-TEST".to_string(),
            email_consent: true,
            ssb_interests: vec![],
            created_at: "2026-01-01".to_string(),
            email_verified: true,
            marketing_emails_opted_in: true,
            totp_enabled: true,
            tos_accepted_at: "2026-01-01T00:00:00Z".to_string(),
            tos_version: "2026-01".to_string(),
        }
    }

    #[test]
    fn argon2id_password_round_trip_and_legacy_migration() {
        let secret = "unit-test-secret";
        let hash = hash_password("[REDACTED_SEED_PW]", secret);
        assert!(is_argon2_hash(&hash));
        assert!(verify_password("[REDACTED_SEED_PW]", secret, &hash));
        assert!(!verify_password("WrongPass!", secret, &hash));

        let legacy = legacy_sha256_password_hash("LegacyPass1!", secret);
        assert!(verify_password("LegacyPass1!", secret, &legacy));
        assert!(!verify_password("OtherPass!", secret, &legacy));
    }

    #[test]
    fn detects_public_sector_and_corporate_verification() {
        assert!(is_public_sector_email("subay@msb.gov.tr"));
        assert!(is_public_sector_email("pilot@hvkk.tsk.tr"));
        assert!(!is_public_sector_email("muhendis@gmail.com"));
        assert!(requires_corporate_verification("muhendis@gmail.com", "SSB"));
        assert!(!requires_corporate_verification("uzman@ssb.gov.tr", "SSB"));
    }

    #[test]
    fn session_revocation_blocks_subsequent_verification() {
        let secret = "session-revocation-test-secret";
        let user = sample_user();
        let tok = generate_session_token(&user, secret);
        assert_eq!(verify_session_token(&tok, secret), Some(42));
        revoke_session_token(&tok);
        assert!(is_token_revoked(&tok));
        assert_eq!(verify_session_token(&tok, secret), None);

        // Immediate re-login within the same second must yield a distinct, valid token.
        let tok2 = generate_session_token(&user, secret);
        assert_ne!(tok, tok2);
        assert_eq!(verify_session_token(&tok2, secret), Some(42));
    }

    #[test]
    fn totp_pending_token_cannot_bypass_2fa_as_session_cookie() {
        let secret = "totp-domain-separation-secret";
        let user = sample_user();

        let pending_tok = generate_totp_pending_token("Operator@MSIDergisi.com", secret);
        assert_eq!(
            verify_totp_pending_token(&pending_tok, secret),
            Some("operator@msidergisi.com".to_string())
        );

        // Critical security invariant: pending 2FA token MUST fail session verification
        assert_eq!(verify_session_token(&pending_tok, secret), None);

        // Critical security invariant: session token MUST fail pending 2FA verification
        let session_tok = generate_session_token(&user, secret);
        assert_eq!(verify_totp_pending_token(&session_tok, secret), None);
    }

    #[test]
    fn persistent_token_revocation_survives_cache_clear_and_cleans_expired_rows() {
        let db_path = format!(
            "/tmp/msi_test_revoked_{}_{}.db",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(1)
        );
        let db = crate::db::Database::new(&db_path).expect("test db");
        let secret = "persistent-revocation-secret";
        let user = sample_user();
        let tok = generate_session_token(&user, secret);
        let tok_hash = crate::utils::crypto::sha256_hex(&tok);

        assert_eq!(verify_session_token_db(&db, &tok, secret), Some(42));
        revoke_session_token_db(&db, &tok);
        assert_eq!(verify_session_token_db(&db, &tok, secret), None);

        // Simulate process restart: wipe in-memory cache and reopen DB handle
        clear_in_memory_revocation_cache();
        drop(db);

        let db2 = crate::db::Database::new(&db_path).expect("reopened db");
        assert_eq!(verify_session_token_db(&db2, &tok, secret), None);

        // Verify SHA-256 hash stored (never raw token)
        let conn = db2.open_conn().unwrap();
        let stored_by_hash: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM revoked_tokens WHERE token_hash = ?1",
                rusqlite::params![tok_hash],
                |r| r.get(0),
            )
            .unwrap();
        let stored_raw: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM revoked_tokens WHERE token_hash = ?1",
                rusqlite::params![tok],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored_by_hash, 1);
        assert_eq!(stored_raw, 0);
        drop(conn);

        // Expired row cleanup removes only expired tokens
        db2.revoke_token_hash("expired_hash_test", "2020-01-01T00:00:00Z")
            .unwrap();
        let cleaned = db2.cleanup_expired_revoked_tokens();
        assert_eq!(cleaned, 1);
        assert!(!db2.is_token_hash_revoked("expired_hash_test"));
        assert!(db2.is_token_hash_revoked(&tok_hash));

        drop(db2);
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }

    #[test]
    fn deleted_account_invalidates_sibling_sessions_immediately_and_across_reopen() {
        let scratch_dir = std::env::temp_dir().join(format!(
            "msi_test_sibling_erasure_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(1)
        ));
        let _ = std::fs::create_dir_all(&scratch_dir);
        let db_path = scratch_dir.join("test.db").to_string_lossy().to_string();
        let db = crate::db::Database::new(&db_path).expect("test db");
        let secret = "sibling-erasure-test-secret";

        let mut new_user = sample_user();
        new_user.id = 0;
        new_user.email = "sibling-kvkk@example.com".to_string();
        new_user.role = "registered".to_string();
        let pw_hash = hash_password("StrongPass!2026", secret);
        let uid = db.create_or_update_user(&new_user, &pw_hash).expect("create user");
        let db_user = db.get_user_by_id(uid).expect("user exists");
        assert!(!is_deleted_or_erased_user(&db_user));

        let tok_a = generate_session_token(&db_user, secret);
        let tok_b = generate_session_token(&db_user, secret);
        assert_ne!(tok_a, tok_b);
        assert_eq!(verify_session_token_db(&db, &tok_a, secret), Some(uid));
        assert_eq!(verify_session_token_db(&db, &tok_b, secret), Some(uid));

        // Browser B deletes account and revokes tok_b; sibling tok_a was never explicitly revoked
        db.delete_user_account(uid, "127.0.0.1")
            .expect("delete_user_account");
        revoke_session_token_db(&db, &tok_b);

        // Database::get_user_by_id must keep returning anonymized row (option_b_api_test:544-546)
        let erased = db.get_user_by_id(uid).expect("anonymized row preserved");
        assert_eq!(erased.role, "deleted");
        assert!(erased.email.ends_with("@erased.local"));
        assert!(is_deleted_or_erased_user(&erased));

        // Both tok_a (sibling) and tok_b (revoked) are rejected immediately
        assert_eq!(verify_session_token_db(&db, &tok_a, secret), None);
        assert_eq!(verify_session_token_db(&db, &tok_b, secret), None);

        // Across cache clear & SQLite reopen from disk, both remain rejected
        clear_in_memory_revocation_cache();
        drop(db);
        let db2 = crate::db::Database::new(&db_path).expect("reopened db");
        assert_eq!(verify_session_token_db(&db2, &tok_a, secret), None);
        assert_eq!(verify_session_token_db(&db2, &tok_b, secret), None);

        drop(db2);
        let _ = std::fs::remove_dir_all(&scratch_dir);
    }
}
