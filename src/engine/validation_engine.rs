//! Centralized Input Validation & Sanitization Engine
//!
//! All user input validation, sanitization, and normalization MUST go through this module.
//! No controller or service should perform ad-hoc validation — call these functions instead.
//! This ensures consistent rules, error messages, and security posture across the platform.

use crate::engine::security_engine::CentralSecurityFilter;

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum file upload size in bytes (25 MB)
pub const MAX_UPLOAD_SIZE: usize = 25 * 1024 * 1024;

/// Minimum password length
pub const MIN_PASSWORD_LEN: usize = 8;

/// Maximum slug length (characters)
pub const MAX_SLUG_LEN: usize = 64;

/// Allowed media/PDF upload extensions
pub const ALLOWED_UPLOAD_EXTENSIONS: &[&str] = &["webp", "png", "jpg", "jpeg", "svg", "pdf", "mp4"];

/// Generic filename stem prefixes that indicate non-descriptive uploads
pub const GENERIC_FILENAME_PREFIXES: &[&str] = &[
    "whatsapp", "img-", "dsc", "screenshot", "ekran-goruntusu",
];

/// Exact generic filename stems (no prefix match needed)
pub const GENERIC_FILENAME_STEMS: &[&str] = &[
    "image", "photo", "asset", "untitled",
];

// ── Validation Results ───────────────────────────────────────────────────────

/// Outcome of an email validation check.
#[derive(Debug, Clone)]
pub enum EmailValidation {
    Valid(String),
    Empty,
    MissingAtSign,
}

/// Outcome of a password validation check.
#[derive(Debug, Clone)]
pub enum PasswordValidation {
    Valid(String),
    TooShort,
    Empty,
}

/// Outcome of a required text field validation.
#[derive(Debug, Clone)]
pub enum TextFieldValidation {
    Valid(String),
    Empty,
}

// ── Core Validators ──────────────────────────────────────────────────────────

/// Validates and normalizes an email address.
/// Returns the trimmed, lowercased email if valid.
pub fn validate_email(raw: &str) -> EmailValidation {
    let normalized = raw.trim().to_lowercase();
    if normalized.is_empty() {
        return EmailValidation::Empty;
    }
    if !normalized.contains('@') {
        return EmailValidation::MissingAtSign;
    }
    EmailValidation::Valid(normalized)
}

/// Validates a password meets minimum requirements.
/// Returns the trimmed password if valid.
pub fn validate_password(raw: Option<&str>) -> PasswordValidation {
    match raw.map(|s| s.trim()) {
        None | Some("") => PasswordValidation::Empty,
        Some(pw) if pw.len() < MIN_PASSWORD_LEN => PasswordValidation::TooShort,
        Some(pw) => PasswordValidation::Valid(pw.to_string()),
    }
}

/// Validates a required text field is non-empty after trimming.
pub fn validate_required_text(raw: &str) -> TextFieldValidation {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        TextFieldValidation::Empty
    } else {
        TextFieldValidation::Valid(trimmed.to_string())
    }
}

/// Validates and sanitizes a URL slug. If empty, returns None so caller can auto-generate.
pub fn validate_slug(raw: &str) -> Option<String> {
    let sanitized = CentralSecurityFilter::sanitize_slug(raw);
    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized.chars().take(MAX_SLUG_LEN).collect::<String>().trim_matches('-').to_string())
    }
}

/// Sanitizes rich editorial HTML using the centralized ammonia allowlist policy.
pub fn sanitize_html(raw: &str) -> String {
    CentralSecurityFilter::sanitize_rich_html(raw)
}

/// Escapes untrusted strings into XSS-safe HTML entities.
pub fn escape_html(input: &str) -> String {
    CentralSecurityFilter::escape_html(input)
}

// ── File Upload Validation ───────────────────────────────────────────────────

/// Result of file upload validation.
#[derive(Debug, Clone)]
pub enum UploadValidation {
    Valid { extension: String },
    InvalidExtension(String),
    EmptyFile,
    ExceedsSizeLimit,
}

/// Validates a file upload by extension and size.
pub fn validate_upload(filename: &str, byte_len: usize) -> UploadValidation {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    if !ALLOWED_UPLOAD_EXTENSIONS.contains(&ext.as_str()) {
        return UploadValidation::InvalidExtension(ext);
    }
    if byte_len == 0 {
        return UploadValidation::EmptyFile;
    }
    if byte_len > MAX_UPLOAD_SIZE {
        return UploadValidation::ExceedsSizeLimit;
    }
    UploadValidation::Valid { extension: ext }
}

/// Determines whether a filename stem is generic/non-descriptive.
pub fn is_generic_filename(stem: &str) -> bool {
    GENERIC_FILENAME_PREFIXES.iter().any(|p| stem.starts_with(p))
        || GENERIC_FILENAME_STEMS.contains(&stem)
}

/// Produces a clean, SEO-friendly file stem from an original filename.
pub fn clean_upload_stem(original_name: &str) -> String {
    let raw_stem = std::path::Path::new(original_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| CentralSecurityFilter::sanitize_slug(s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "savunma-gorsel".to_string());

    if is_generic_filename(&raw_stem) {
        "savunma-sanayii-medya".to_string()
    } else {
        raw_stem
            .chars()
            .take(MAX_SLUG_LEN)
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }
}

// ── Optional Field Helpers ───────────────────────────────────────────────────

/// Trims an optional string, returning None if empty.
pub fn trim_optional(raw: Option<&str>) -> Option<String> {
    raw.and_then(|s| {
        let t = s.trim();
        if t.is_empty() { None } else { Some(t.to_string()) }
    })
}

/// Trims an optional owned String, returning None if empty.
pub fn trim_optional_owned(raw: Option<String>) -> Option<String> {
    raw.and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    })
}

/// Returns the value if non-empty after trim, otherwise the default.
pub fn with_default(raw: Option<&str>, default: &str) -> String {
    trim_optional(raw).unwrap_or_else(|| default.to_string())
}

// ── Email Header Sanitization (RFC 5322) ─────────────────────────────────────

/// Strips CR, LF, and NUL from email header addresses to prevent header injection.
pub fn sanitize_email_header(addr: &str) -> String {
    addr.chars()
        .filter(|c| !matches!(c, '\r' | '\n' | '\0'))
        .collect()
}

/// Strips CR and LF from arbitrary text destined for email headers (subjects, etc.).
pub fn sanitize_header_text(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_validation_cases() {
        assert!(matches!(validate_email("  USER@Example.COM  "), EmailValidation::Valid(e) if e == "user@example.com"));
        assert!(matches!(validate_email(""), EmailValidation::Empty));
        assert!(matches!(validate_email("   "), EmailValidation::Empty));
        assert!(matches!(validate_email("no-at-sign"), EmailValidation::MissingAtSign));
    }

    #[test]
    fn password_validation_cases() {
        assert!(matches!(validate_password(Some("12345678")), PasswordValidation::Valid(_)));
        assert!(matches!(validate_password(Some("short")), PasswordValidation::TooShort));
        assert!(matches!(validate_password(None), PasswordValidation::Empty));
        assert!(matches!(validate_password(Some("  ")), PasswordValidation::Empty));
    }

    #[test]
    fn slug_validation_cases() {
        assert_eq!(validate_slug("Hello World!"), Some("hello-world".to_string()));
        assert_eq!(validate_slug(""), None);
        assert_eq!(validate_slug("!!!"), None);
        assert!(validate_slug("Türkçe Başlık").is_some());
    }

    #[test]
    fn upload_validation_cases() {
        assert!(matches!(validate_upload("photo.webp", 1024), UploadValidation::Valid { .. }));
        assert!(matches!(validate_upload("photo.exe", 1024), UploadValidation::InvalidExtension(_)));
        assert!(matches!(validate_upload("photo.png", 0), UploadValidation::EmptyFile));
        assert!(matches!(validate_upload("photo.png", MAX_UPLOAD_SIZE + 1), UploadValidation::ExceedsSizeLimit));
    }

    #[test]
    fn generic_filename_detection() {
        assert!(is_generic_filename("whatsapp-image"));
        assert!(is_generic_filename("screenshot-2024"));
        assert!(is_generic_filename("image"));
        assert!(!is_generic_filename("kaan-interview"));
    }

    #[test]
    fn html_sanitization_preserves_safe_markup() {
        let dirty = r#"<p class="excerpt"><strong>Bold</strong><script>alert(1)</script></p>"#;
        let clean = sanitize_html(dirty);
        assert!(clean.contains("<strong>Bold</strong>"));
        assert!(!clean.contains("<script"));
    }

    #[test]
    fn email_header_injection_prevention() {
        let injected = "evil@example.com\r\nBcc: spam@bad.com";
        let clean = sanitize_email_header(injected);
        assert!(!clean.contains('\r'));
        assert!(!clean.contains('\n'));
        assert!(clean.contains("evil@example.com"));
    }
}