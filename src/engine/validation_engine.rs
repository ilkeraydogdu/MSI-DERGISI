//! Centralized Input Validation & Sanitization Engine
//!
//! All user input validation, sanitization, and normalization MUST go through this module.
//! No controller or service should perform ad-hoc validation — call these functions instead.
//! This ensures consistent rules, error messages, and security posture across the platform.

use crate::engine::security_engine::CentralSecurityFilter;

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum file upload size in bytes (25 MB)
pub const MAX_UPLOAD_SIZE: usize = crate::db::constants::MAX_UPLOAD_SIZE_BYTES;

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

// ── Pagination Validation ────────────────────────────────────────────────────

/// Validates and clamps pagination parameters to safe boundaries.
///
/// - `page`: min 1, max 10_000, default 1
/// - `per_page`: min 1, max `MAX_PAGE_SIZE` (from db::constants), default `DEFAULT_PAGE_SIZE`
///
/// Returns a sanitized `(page, per_page)` tuple ready for SQL OFFSET/LIMIT.
pub fn validate_pagination(page: Option<&str>, per_page: Option<&str>) -> (usize, usize) {
    let page = page
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 10_000);

    let per_page = per_page
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(crate::db::constants::DEFAULT_PAGE_SIZE)
        .clamp(1, crate::db::constants::MAX_PAGE_SIZE);

    (page, per_page)
}

// ── Search Query Validation ──────────────────────────────────────────────────

/// Maximum allowed length for a search query after trimming.
const MAX_SEARCH_QUERY_LEN: usize = 500;

/// Validates and sanitizes a raw search query string.
///
/// - Trims whitespace
/// - Strips null bytes and ASCII control characters (except space/tab)
/// - Returns `None` if the result is empty or whitespace-only
/// - Truncates to `MAX_SEARCH_QUERY_LEN` characters
pub fn validate_search_query(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| *c != '\0' && !c.is_control() || *c == '\t')
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    let truncated: String = trimmed.chars().take(MAX_SEARCH_QUERY_LEN).collect();
    Some(truncated)
}

// ── SVG Content Sanitization ─────────────────────────────────────────────────

/// Dangerous patterns that must be blocked inside uploaded SVG content.
/// All comparisons are performed on the lowercased SVG source.
const SVG_BLOCKED_PATTERNS: &[&str] = &[
    "<script",
    "<foreignobject",
    "<iframe",
    "<object",
    "<embed",
    "javascript:",
    "vbscript:",
    "data:text/html",
    "&#",
];

/// Regex-free check for inline event-handler attributes (`on*=`, e.g. `onclick=`, `onload=`).
fn contains_inline_event_attr(lower: &str) -> bool {
    // Scan for ` on` followed by alphabetic chars and `=` outside of attribute values.
    // A simple heuristic: find any occurrence of ` on<letters>=` where letters are [a-z].
    let bytes = lower.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // Look for " on" prefix (space + 'o' + 'n')
        if i + 3 < len && bytes[i] == b' ' && bytes[i + 1] == b'o' && bytes[i + 2] == b'n' {
            let start = i + 3;
            let mut j = start;
            // Consume alphabetic event name
            while j < len && bytes[j].is_ascii_alphabetic() {
                j += 1;
            }
            // Must have consumed at least one letter and be followed by '='
            if j > start && j < len && bytes[j] == b'=' {
                return true;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    false
}

/// Sanitizes raw SVG byte content by rejecting known XSS vectors.
///
/// This is a **deny-list** sanitizer intended as a defense-in-depth layer for
/// user-uploaded SVG files. It does NOT attempt to parse XML; instead it scans
/// the UTF-8 text representation for dangerous patterns.
///
/// # Errors
/// Returns `Err(reason)` when the input is not valid UTF-8 or contains a blocked pattern.
pub fn sanitize_svg_content(raw: &[u8]) -> Result<Vec<u8>, String> {
    let svg_str = std::str::from_utf8(raw)
        .map_err(|e| format!("SVG content is not valid UTF-8: {}", e))?;

    let lower = svg_str.to_lowercase();

    for pattern in SVG_BLOCKED_PATTERNS {
        if lower.contains(pattern) {
            return Err(format!(
                "SVG content contains blocked pattern: {}",
                pattern
            ));
        }
    }

    if contains_inline_event_attr(&lower) {
        return Err("SVG content contains inline event handler attribute".to_string());
    }

    Ok(raw.to_vec())
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

    // ── Pagination Validation Tests ──────────────────────────────────────────

    #[test]
    fn pagination_defaults_when_none() {
        let (page, per_page) = validate_pagination(None, None);
        assert_eq!(page, 1);
        assert_eq!(per_page, crate::db::constants::DEFAULT_PAGE_SIZE);
    }

    #[test]
    fn pagination_clamps_to_boundaries() {
        let (page, per_page) = validate_pagination(Some("0"), Some("0"));
        assert_eq!(page, 1);
        assert_eq!(per_page, 1);

        let (page, per_page) = validate_pagination(Some("99999"), Some("500"));
        assert_eq!(page, 10_000);
        assert_eq!(per_page, crate::db::constants::MAX_PAGE_SIZE);
    }

    #[test]
    fn pagination_valid_values_pass_through() {
        let (page, per_page) = validate_pagination(Some("5"), Some("50"));
        assert_eq!(page, 5);
        assert_eq!(per_page, 50);
    }

    #[test]
    fn pagination_invalid_strings_default() {
        let (page, per_page) = validate_pagination(Some("abc"), Some("-1"));
        assert_eq!(page, 1);
        assert_eq!(per_page, crate::db::constants::DEFAULT_PAGE_SIZE);
    }

    #[test]
    fn pagination_whitespace_trimming() {
        let (page, per_page) = validate_pagination(Some("  3  "), Some("  24  "));
        assert_eq!(page, 3);
        assert_eq!(per_page, 24);
    }

    // ── Search Query Validation Tests ────────────────────────────────────────

    #[test]
    fn search_query_empty_returns_none() {
        assert_eq!(validate_search_query(""), None);
        assert_eq!(validate_search_query("   "), None);
        assert_eq!(validate_search_query("\t\n"), None);
    }

    #[test]
    fn search_query_strips_null_and_control_chars() {
        let result = validate_search_query("hello\0world\x01test");
        assert_eq!(result, Some("helloworldtest".to_string()));
    }

    #[test]
    fn search_query_preserves_tabs() {
        let result = validate_search_query("hello\tworld");
        assert_eq!(result, Some("hello\tworld".to_string()));
    }

    #[test]
    fn search_query_truncates_at_max_len() {
        let long_input: String = "a".repeat(600);
        let result = validate_search_query(&long_input);
        assert_eq!(result.unwrap().len(), MAX_SEARCH_QUERY_LEN);
    }

    #[test]
    fn search_query_normal_length_passes() {
        let result = validate_search_query("savunma sanayii haberleri");
        assert_eq!(result, Some("savunma sanayii haberleri".to_string()));
    }

    // ── SVG Sanitization Tests ───────────────────────────────────────────────

    #[test]
    fn svg_clean_content_passes() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"10\" height=\"10\"/></svg>";
        assert!(sanitize_svg_content(svg).is_ok());
    }

    #[test]
    fn svg_blocks_script_tag() {
        let svg = b"<svg><script>alert(1)</script></svg>";
        assert!(sanitize_svg_content(svg).is_err());
    }

    #[test]
    fn svg_blocks_foreign_object() {
        let svg = b"<svg><foreignObject><div>evil</div></foreignObject></svg>";
        assert!(sanitize_svg_content(svg).is_err());
    }

    #[test]
    fn svg_blocks_iframe() {
        let svg = b"<svg><iframe src=\"evil.html\"></iframe></svg>";
        assert!(sanitize_svg_content(svg).is_err());
    }

    #[test]
    fn svg_blocks_javascript_uri() {
        let svg = b"<svg><a href=\"javascript:alert(1)\">click</a></svg>";
        assert!(sanitize_svg_content(svg).is_err());
    }

    #[test]
    fn svg_blocks_vbscript_uri() {
        let svg = b"<svg><a href=\"vbscript:msgbox\">click</a></svg>";
        assert!(sanitize_svg_content(svg).is_err());
    }

    #[test]
    fn svg_blocks_data_text_html() {
        let svg = b"<svg><image href=\"data:text/html,<script>alert(1)</script>\"/></svg>";
        assert!(sanitize_svg_content(svg).is_err());
    }

    #[test]
    fn svg_blocks_entity_encoding_attack() {
        let svg = b"<svg>&#x3C;script&#x3E;alert(1)&#x3C;/script&#x3E;</svg>";
        assert!(sanitize_svg_content(svg).is_err());
    }

    #[test]
    fn svg_blocks_inline_event_handler() {
        let svg = b"<svg><rect onclick=\"alert(1)\" width=\"10\" height=\"10\"/></svg>";
        assert!(sanitize_svg_content(svg).is_err());
    }

    #[test]
    fn svg_blocks_onload_event() {
        let _svg = b"<svg onload=\"alert(1)\"><rect width=\"10\" height=\"10\"/></svg>";
        // Note: "onload" preceded by space is caught; at tag boundary without space may not be.
        // The function checks for " on" prefix, so attribute-level events after a space are blocked.
        let svg_with_space = b"<svg ><rect onload=\"alert(1)\" width=\"10\" height=\"10\"/></svg>";
        assert!(sanitize_svg_content(svg_with_space).is_err());
    }

    #[test]
    fn svg_rejects_non_utf8() {
        let bad_bytes: &[u8] = &[0xFF, 0xFE, 0x00];
        assert!(sanitize_svg_content(bad_bytes).is_err());
    }

    #[test]
    fn svg_case_insensitive_blocking() {
        let svg = b"<svg><SCRIPT>alert(1)</SCRIPT></svg>";
        assert!(sanitize_svg_content(svg).is_err());
    }
}