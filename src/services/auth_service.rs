use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::models::UserAccount;

type HmacSha256 = Hmac<Sha256>;

/// Legacy seed secret retained strictly for transparent migration of pre-Argon2id hashes on login.
/// SECURITY: Now centralized in AppConfig.legacy_seed_secret; this function is kept as a
/// compatibility shim but should be replaced with direct config access where possible.
fn legacy_seed_secret() -> String {
    std::env::var("LEGACY_SEED_SECRET")
        .or_else(|_| std::env::var("SESSION_SECRET"))
        .unwrap_or_else(|_| "CHANGE-ME-LEGACY-SEED-SECRET".to_string())
}

/// Returns the legacy seed secret from the centralized AppConfig when available,
/// falling back to the env-based shim for contexts without config access.
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
/// Format: `{user_id}.{role}.{issued_ts}.{hmac_hex}`
pub fn generate_session_token(user: &UserAccount, secret: &str) -> String {
    let issued_ts = Utc::now().timestamp();
    let payload = format!("{}.{}.{}", user.id, user.role, issued_ts);
    let sig = sign_session_payload(&payload, secret);
    format!("{}.{}", payload, sig)
}

/// Verifies a signed session token and returns `Some(user_id)` if authentic and untampered.
pub fn verify_session_token(token: &str, secret: &str) -> Option<i64> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let user_id: i64 = parts[0].parse().ok()?;
    let role = parts[1];
    let issued_ts: i64 = parts[2].parse().ok()?;
    let age = Utc::now().timestamp().checked_sub(issued_ts)?;
    if !(-60..=86400).contains(&age) {
        return None;
    }
    let provided_sig = parts[3];

    let payload = format!("{}.{}.{}", user_id, role, issued_ts);
    let expected_sig = sign_session_payload(&payload, secret);

    if constant_time_eq(provided_sig.as_bytes(), expected_sig.as_bytes()) {
        Some(user_id)
    } else {
        None
    }
}

/// Identity is resolved from a signed, HttpOnly session, never a query parameter.
pub fn session_user(
    state: &crate::controllers::AppState,
    headers: &axum::http::HeaderMap,
) -> Option<UserAccount> {
    let token = headers
        .get(axum::http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|p| p.trim().strip_prefix("msi_session="))?;
    state
        .db
        .get_user_by_id(verify_session_token(token, &state.config.session_secret)?)
}

/// Determines the initial `(role, subscription_tier, is_gov_verified)` tuple for a newly registering user.
/// - Public sector domains (`.gov.tr`, `.tsk.tr`, `.mil.tr`, etc.) automatically receive
///   `("subscriber_public", "Kamu E-Dergi Abonesi (Ücretsiz)", true)`.
/// - Academic `.edu.tr` domains receive `("registered", "Akademik Okur", false)`.
/// - Standard or corporate users start as `("registered", "Standart Kayıtlı Okur", false)` until subscription purchase or operator approval.
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

// SECURITY FIX (H12): Constant-time comparison without length side channel.
// Always compares max(len_a, len_b) bytes to avoid leaking hash length via
// early return on mismatched lengths.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max_len = std::cmp::max(a.len(), b.len());
    let mut diff: u8 = if a.len() != b.len() { 1 } else { 0 };
    for i in 0..max_len {
        let va = if i < a.len() { a[i] } else { 0 };
        let vb = if i < b.len() { b[i] } else { 0 };
        diff |= va ^ vb;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2id_password_round_trip_and_legacy_migration() {
        let secret = "unit-test-secret";
        let hash = hash_password("Savunma2026!", secret);
        assert!(is_argon2_hash(&hash));
        assert!(verify_password("Savunma2026!", secret, &hash));
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
}
