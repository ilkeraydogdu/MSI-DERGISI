//! TOTP Two-Factor Authentication Service (`src/services/totp_service.rs`)
//!
//! Provides RFC 6238 compliant TOTP secret generation, `otpauth://` URI creation,
//! SVG QR code rendering, code verification (`Algorithm::SHA1`, 6 digits, 30s step, skew = 1),
//! and one-time recovery backup codes (generation + SHA-256 hashing).
//! Compatible out of the box with Google Authenticator, Microsoft Authenticator,
//! Apple Passwords, 1Password, and Authy.

use sha2::{Digest, Sha256};
use totp_rs::{Algorithm, Secret, TOTP};

use crate::utils::crypto::constant_time_eq;

/// Application issuer name shown in authenticator apps
const APP_NAME: &str = "MSI Dergisi";

/// Standard RFC 6238 6-digit OTP for universal authenticator compatibility
const DIGITS: usize = 6;

/// Standard RFC 6238 time step in seconds (30s)
const TIME_STEP: u64 = 30;

/// Allowed time skew tolerance (1 period before/after current = ±30s)
const SKEW: u8 = 1;

/// Number of one-time recovery backup codes generated per enrollment
const BACKUP_CODE_COUNT: usize = 8;

/// Length of each alphanumeric recovery backup code
const BACKUP_CODE_LEN: usize = 8;

/// Unambiguous 32-character uppercase alphanumeric alphabet (power-of-2 -> zero modulo bias)
const BACKUP_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Internal helper that validates Base32 secret bytes and constructs an RFC 6238 `TOTP` instance.
/// Never falls back to an empty `Vec` and never panics.
fn build_totp(secret_base32: &str, account_name: &str) -> Result<TOTP, String> {
    let normalized_secret: String = secret_base32
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    if normalized_secret.is_empty() {
        return Err("TOTP Base32 anahtarı boş olamaz.".to_string());
    }

    let secret_bytes = Secret::Encoded(normalized_secret)
        .to_bytes()
        .map_err(|e| format!("Geçersiz Base32 TOTP anahtarı: {}", e))?;

    if secret_bytes.is_empty() {
        return Err("Çözümlenen TOTP anahtarı boş.".to_string());
    }

    // totp-rs forbids ':' inside account_name / issuer in otpauth URLs
    let cleaned_account: String = account_name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if c == ':' { '_' } else { c })
        .collect();

    let safe_account = if cleaned_account.is_empty() {
        "account@msidergisi.com".to_string()
    } else {
        cleaned_account
    };

    TOTP::new(
        Algorithm::SHA1,
        DIGITS,
        SKEW,
        TIME_STEP,
        secret_bytes,
        Some(APP_NAME.to_string()),
        safe_account,
    )
    .map_err(|e| format!("TOTP yapılandırma hatası: {}", e))
}

/// Generates a new cryptographically random TOTP secret (160-bit Base32-encoded).
pub fn generate_totp_secret() -> String {
    let secret = Secret::generate_secret();
    secret.to_encoded().to_string()
}

/// Creates an `otpauth://` URI suitable for QR code generation.
/// Format: `otpauth://totp/{issuer}:{email}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits=6&period=30`
///
/// `totp-rs` omits RFC 6238 defaults from its URL output; we append them explicitly so every
/// authenticator app (including those that don't apply the spec defaults) reads 6 digits / 30s / SHA1.
pub fn generate_totp_uri(secret_base32: &str, email: &str) -> Result<String, String> {
    let totp = build_totp(secret_base32, email)?;
    let mut uri = totp.get_url();

    let mut missing: Vec<&str> = Vec::new();
    if !uri.contains("algorithm=") {
        missing.push("algorithm=SHA1");
    }
    if !uri.contains("digits=") {
        missing.push("digits=6");
    }
    if !uri.contains("period=") {
        missing.push("period=30");
    }
    if !missing.is_empty() {
        // get_url() always emits at least `secret=`, so a query string separator exists.
        uri.push('&');
        uri.push_str(&missing.join("&"));
    }

    Ok(uri)
}

/// Generates an SVG string QR code for the given `otpauth://` URI.
/// Returns `None` if `uri` is empty or QR encoding fails.
pub fn generate_qr_code_svg(uri: &str) -> Option<String> {
    use qrcode::render::svg;
    use qrcode::QrCode;

    if uri.trim().is_empty() {
        return None;
    }

    let code = QrCode::new(uri.as_bytes()).ok()?;
    let svg_string = code
        .render::<svg::Color>()
        .min_dimensions(256, 256)
        .max_dimensions(512, 512)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();

    Some(svg_string)
}

/// Result-returning wrapper around `generate_qr_code_svg` for controller ergonomics.
pub fn generate_qr_svg(uri: &str) -> Result<String, String> {
    generate_qr_code_svg(uri).ok_or_else(|| "TOTP QR SVG oluşturulamadı.".to_string())
}

/// Verifies a 6-digit TOTP code against the stored Base32 secret.
/// Returns `true` only if `secret_base32` is valid Base32 and `code` matches within `±SKEW` steps.
pub fn verify_totp(secret_base32: &str, code: &str) -> bool {
    verify_totp_step(secret_base32, code).is_some()
}

/// Verifies a 6-digit TOTP code against the stored Base32 secret and returns the matched
/// 30-second time step so callers can enforce anti-replay protection within the same step.
pub fn verify_totp_step(secret_base32: &str, code: &str) -> Option<u64> {
    let trimmed = code.trim();
    if trimmed.len() != DIGITS || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let totp = build_totp(secret_base32, "verify@msidergisi.com").ok()?;
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let base_step = now_secs / TIME_STEP;
    let mut matched: Option<u64> = None;
    for step in base_step.saturating_sub(SKEW as u64)..=(base_step + SKEW as u64) {
        let expected = totp.generate(step * TIME_STEP);
        if constant_time_eq(trimmed.as_bytes(), expected.as_bytes()) {
            matched = Some(step);
        }
    }
    matched
}

/// Generates 8 random 8-character uppercase alphanumeric recovery backup codes using UUIDv4 CSPRNG bytes.
pub fn generate_backup_codes() -> Vec<String> {
    let mut codes = Vec::with_capacity(BACKUP_CODE_COUNT);
    while codes.len() < BACKUP_CODE_COUNT {
        let uuid_bytes = *uuid::Uuid::new_v4().as_bytes();
        let mut code = String::with_capacity(BACKUP_CODE_LEN);
        for i in 0..BACKUP_CODE_LEN {
            // Mix upper and lower halves of the 128-bit UUIDv4 entropy
            let mixed = uuid_bytes[i] ^ uuid_bytes[i + 8];
            let idx = (mixed as usize) & 0x1F;
            code.push(BACKUP_ALPHABET[idx] as char);
        }
        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    codes
}

/// Computes the lowercase SHA-256 hex digest of an uppercase-normalized recovery backup code
/// (stripping optional whitespace and hyphens so user formatting never causes false negatives).
pub fn hash_backup_code(code: &str) -> String {
    let normalized: String = code
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

/// Checks a candidate recovery code against stored SHA-256 hex hashes in constant time.
/// Returns `Some(index)` of the matching hash so the caller can remove/consume it.
pub fn verify_backup_code(code: &str, stored_hashes: &[String]) -> Option<usize> {
    let normalized_len = code
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .count();
    if normalized_len == 0 {
        return None;
    }
    let candidate_hash = hash_backup_code(code);
    let mut matched_idx: Option<usize> = None;
    for (idx, existing) in stored_hashes.iter().enumerate() {
        let existing_norm = existing.trim().to_ascii_lowercase();
        if constant_time_eq(candidate_hash.as_bytes(), existing_norm.as_bytes()) {
            matched_idx = Some(idx);
        }
    }
    matched_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_secret_is_valid_base32() {
        let secret = generate_totp_secret();
        assert!(secret.len() >= 26);
        assert!(Secret::Encoded(secret).to_bytes().is_ok());
    }

    #[test]
    fn totp_uri_uses_universal_sha1_6_digits() {
        let secret = generate_totp_secret();
        let uri = generate_totp_uri(&secret, "admin@msidergisi.com").expect("valid URI");
        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("MSI%20Dergisi"));
        assert!(uri.contains("admin%40msidergisi.com"));
        assert!(uri.contains("digits=6"));
        assert!(uri.contains("period=30"));
        assert!(!uri.contains("SHA512"));
    }

    #[test]
    fn invalid_base32_returns_err_instead_of_panicking() {
        let bad = "!!!NOT_VALID_BASE32$$$";
        assert!(generate_totp_uri(bad, "admin@test.com").is_err());
        assert!(!verify_totp(bad, "123456"));
        assert!(generate_totp_uri("", "admin@test.com").is_err());
    }

    #[test]
    fn qr_code_generation_produces_svg() {
        let secret = generate_totp_secret();
        let uri = generate_totp_uri(&secret, "test@example.com").unwrap();
        let svg = generate_qr_code_svg(&uri).expect("SVG generated");
        assert!(svg.contains("<svg"));
        assert!(svg.len() > 100);
    }

    #[test]
    fn verify_accepts_valid_6_digit_code_and_rejects_invalid() {
        let secret_b32 = generate_totp_secret();
        let totp = build_totp(&secret_b32, "test@msidergisi.com").unwrap();
        let current_code = totp.generate_current().unwrap();
        assert_eq!(current_code.len(), 6);
        assert!(verify_totp(&secret_b32, &current_code));
        assert!(!verify_totp(&secret_b32, "00000000")); // wrong length
    }

    #[test]
    fn backup_codes_generation_and_sha256_hashing() {
        let codes = generate_backup_codes();
        assert_eq!(codes.len(), 8);
        for c in &codes {
            assert_eq!(c.len(), 8);
            assert!(c.chars().all(|ch| ch.is_ascii_alphanumeric() && !ch.is_ascii_lowercase()));
        }

        let sample = &codes[0];
        let h_upper = hash_backup_code(sample);
        let h_lower = hash_backup_code(&sample.to_ascii_lowercase());
        assert_eq!(h_upper.len(), 64);
        assert_eq!(h_upper, h_lower);

        let hashed: Vec<String> = codes.iter().map(|c| hash_backup_code(c)).collect();
        assert_eq!(verify_backup_code(&codes[3].to_ascii_lowercase(), &hashed), Some(3));
        assert_eq!(verify_backup_code("ZZZZZZZZ", &hashed), None);
    }
}
