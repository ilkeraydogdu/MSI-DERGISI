//! Centralized media/image management service.
//!
//! Single source of truth for:
//! - File upload validation (type, size)
//! - SEO-friendly path generation
//! - DB metadata registration in `media_library`
//! - Fallback/default image constants
//! - Asset URL resolution

use std::path::Path as FsPath;

// ---------------------------------------------------------------------------
// Default / fallback image constants – single source of truth
// ---------------------------------------------------------------------------

/// Default OG / social sharing image when no cover is available.
pub const DEFAULT_OG_IMAGE: &str = "images/msi/msi-dergisi-sosyal-paylasim-kapagi.webp";

/// Default magazine issue cover fallback.
pub const DEFAULT_ISSUE_COVER: &str = "images/msi/msi-dergisi-sayi-286-kapak.webp";

/// Default right-sidebar ad placeholder.
pub const DEFAULT_AD_SIDEBAR: &str = "images/msi/msi-dergisi-reklam-home-right-1-78.webp";

// ---------------------------------------------------------------------------
// Allowed extensions & limits
// ---------------------------------------------------------------------------

/// Extensions accepted by the upload pipeline.
pub const ALLOWED_EXTENSIONS: &[&str] = &["webp", "png", "jpg", "jpeg", "svg", "pdf", "mp4"];

/// Maximum upload size in bytes (25 MB).
pub const MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;

/// MIME type mapping for allowed extensions.
pub fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "webp" => "image/webp",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Returns `true` when the extension is in the allow-list.
pub fn is_allowed_extension(ext: &str) -> bool {
    ALLOWED_EXTENSIONS.contains(&ext)
}

/// Returns `true` when the byte length is within the upload limit and non-zero.
pub fn is_valid_size(len: usize) -> bool {
    len != 0 && len <= MAX_UPLOAD_BYTES
}

// ---------------------------------------------------------------------------
// Path generation
// ---------------------------------------------------------------------------

/// Build an SEO-friendly filename and relative storage path for an uploaded file.
///
/// Returns `(relative_url, filename)` where `relative_url` is suitable for DB
/// storage and front-end `<img src>` attributes (no leading slash).
pub fn generate_upload_path(
    original_name: &str,
    ext: &str,
    sanitize_slug: fn(&str) -> String,
) -> (String, String) {
    let raw_stem = FsPath::new(original_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| sanitize_slug(s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "savunma-gorsel".to_string());

    let is_generic = raw_stem.starts_with("whatsapp")
        || raw_stem.starts_with("img-")
        || raw_stem.starts_with("dsc")
        || raw_stem.starts_with("screenshot")
        || raw_stem.starts_with("ekran-goruntusu")
        || matches!(raw_stem.as_str(), "image" | "photo" | "asset" | "untitled");

    let clean_stem = if is_generic {
        "savunma-sanayii-medya".to_string()
    } else {
        raw_stem
            .chars()
            .take(64)
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    };

    let seo_prefix = if clean_stem.starts_with("msi-dergisi") || clean_stem.starts_with("msi-tdr") {
        clean_stem
    } else {
        format!("msi-dergisi-{}", clean_stem)
    };

    let subdir = if ext == "pdf" {
        "images/uploads/issues"
    } else {
        "images/uploads"
    };

    let filename = format!(
        "{}-{}.{}",
        seo_prefix,
        chrono::Utc::now().timestamp(),
        ext
    );
    let relative_url = format!("{}/{}", subdir, filename);

    (relative_url, filename)
}

// ---------------------------------------------------------------------------
// Asset URL resolution
// ---------------------------------------------------------------------------

/// Resolve a relative asset path to a web-safe URL.
/// External URLs pass through unchanged; local paths get a leading `/`.
pub fn resolve_asset_url(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        trimmed.to_string()
    } else {
        format!("/{}", trimmed.trim_start_matches('/'))
    }
}

/// Return the effective OG image: falls back to [`DEFAULT_OG_IMAGE`] when the
/// supplied value is empty or an SVG (which cannot be used as og:image).
pub fn effective_og_image(og_image: &str) -> &str {
    let t = og_image.trim();
    if t.is_empty() || t.ends_with(".svg") {
        DEFAULT_OG_IMAGE
    } else {
        t
    }
}

/// Return the effective cover image with a fallback.
pub fn effective_cover<'a>(cover: Option<&'a str>, fallback: &'a str) -> &'a str {
    match cover.map(str::trim) {
        Some(c) if !c.is_empty() => c,
        _ => fallback,
    }
}

// ---------------------------------------------------------------------------
// DB registration helper
// ---------------------------------------------------------------------------

/// Register an uploaded file in the `media_library` table.
///
/// This is intentionally a thin wrapper so callers do not need to know the
/// column layout. Returns the new row id on success.
pub fn register_media(
    repo: &crate::db::Database,
    filename: &str,
    original_name: &str,
    ext: &str,
    file_size: usize,
    relative_path: &str,
    uploaded_by: &str,
) -> Result<i64, String> {
    repo.insert_media_item(
        filename,
        original_name,
        mime_for_ext(ext),
        file_size as i64,
        relative_path,
        uploaded_by,
    )
    .map_err(|e| format!("Media DB insert failed: {}", e))
}