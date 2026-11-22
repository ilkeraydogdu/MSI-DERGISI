//! Editorial Studio Media Archive processing service:
//! - Magic-byte validation + SSRF-guarded remote fetch
//! - Automatic WebP conversion, max-1920px proportional Lanczos3 resize (keeping `1200x675` untouched)
//! - Sleek bottom-right dark badge + MSI Dergisi `logo.png` watermark compositing (with fallback brand bar)
//! - Automatic `400x225` cover-cropped WebP thumbnail generation

use std::io::Cursor;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use crate::editorial::common::MediaAsset;
use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use rand_core::{OsRng, RngCore};
use reqwest::{redirect::Policy, Url};
use serde_json::json;

/// Service error type carrying clear localized Turkish validation/network feedback for Editorial Studio.
#[derive(Debug, Clone)]
pub enum EditorialServiceError {
    BadRequest(String),
    NotFound(String),
    Forbidden(String),
    Internal(String),
}

impl EditorialServiceError {
    pub fn into_json_response(self) -> Response {
        let (code, msg) = match self {
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            Self::NotFound(m) => (StatusCode::NOT_FOUND, m),
            Self::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            Self::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (code, Json(json!({ "success": false, "error": msg }))).into_response()
    }
}

pub type EditorialResult<T> = Result<T, EditorialServiceError>;
type AppError = EditorialServiceError;
type AppResult<T> = EditorialResult<T>;

/// Resolves the filesystem directory for editorial uploads (`<storage>/uploads/editorial`)
/// so systemd `ReadWritePaths=.../storage` is respected in production while temp DB tests stay isolated.
pub fn resolve_editorial_upload_dir(config: &crate::config::AppConfig) -> String {
    let db_raw = config.sqlite_db_path.trim();
    if db_raw.is_empty() || db_raw == ":memory:" {
        let root = Path::new(&config.static_dir)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        return root
            .join("storage")
            .join("uploads")
            .join("editorial")
            .to_string_lossy()
            .to_string();
    }

    let db_path = Path::new(db_raw);
    let base: PathBuf = if let Some(parent) = db_path.parent() {
        if parent.file_name().and_then(|s| s.to_str()) == Some("db") {
            parent.parent().unwrap_or(parent).to_path_buf()
        } else {
            parent.to_path_buf()
        }
    } else {
        Path::new(&config.static_dir)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("storage")
    };
    base.join("uploads")
        .join("editorial")
        .to_string_lossy()
        .to_string()
}

const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024; // 10 MB
const MAX_REDIRECTS: usize = 5;
const MAX_IMAGE_WIDTH: u32 = 1920;
const THUMB_WIDTH: u32 = 400;
const THUMB_HEIGHT: u32 = 225;

#[derive(Debug, Clone)]
pub struct ProcessedMedia {
    pub filename: String,
    pub original_name: String,
    pub url: String,
    pub thumb_url: Option<String>,
    pub mime_type: String,
    pub width: i32,
    pub height: i32,
    pub size_bytes: i64,
    pub watermarked: bool,
}

#[derive(Debug, Clone)]
pub struct MultipartUploadParsed {
    pub bytes: Vec<u8>,
    pub orig_name: String,
    pub watermark: bool,
    pub alt_text: Option<String>,
}

/// Inspects file magic bytes and returns a canonical safe image extension (`jpg`, `png`, `gif`, `webp`, `avif`, `bmp`, `svg`).
pub fn detect_image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 3 && bytes[0..3] == [0xFF, 0xD8, 0xFF] {
        return Some("jpg");
    }
    if bytes.len() >= 8 && bytes[0..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some("png");
    }
    if bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        return Some("gif");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("webp");
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" || brand == b"mif1" || brand == b"heic" {
            return Some("avif");
        }
    }
    if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
        return Some("bmp");
    }
    // SECURITY FIX: PDF magic byte validation (%PDF-)
    if bytes.len() >= 5 && &bytes[0..5] == b"%PDF-" {
        return Some("pdf");
    }
    // SECURITY FIX: MP4/ISO base media file format magic byte validation (ftyp box at offset 4)
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" || brand == b"mif1" || brand == b"heic" {
            return Some("avif");
        }
        // Accept common MP4/MOV brands
        if brand == b"isom" || brand == b"iso2" || brand == b"mp41" || brand == b"mp42"
            || brand == b"M4V " || brand == b"qt  " || brand == b"3gp4" || brand == b"3gp5"
            || brand == b"MSNV" || brand == b"F4V " || brand == b"F4P "
        {
            return Some("mp4");
        }
    }
    // Safe SVG check across 100% of bytes (rejects embedded scripts/event handlers/obfuscated entities)
    if crate::engine::validation_engine::validate_svg_safe(bytes).is_ok() {
        return Some("svg");
    }
    None
}

/// Checks whether an IPv4 address belongs to loopback, private, link-local, CGNAT, multicast, or broadcast space.
pub fn is_disallowed_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_documentation()
        || octets[0] == 0
        // Carrier-grade NAT 100.64.0.0/10
        || (octets[0] == 100 && (octets[1] & 0b1100_0000) == 64)
        // Benchmark 198.18.0.0/15
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
        // Reserved 240.0.0.0/4
        || octets[0] >= 240
}

/// Checks whether an IPv6 address belongs to loopback, unspecified, unique-local, link-local, multicast, or disallowed IPv4-mapped space.
pub fn is_disallowed_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_disallowed_ipv4(v4);
    }
    let segs = ip.segments();
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        // Unique local fc00::/7
        || (segs[0] & 0xfe00) == 0xfc00
        // Link-local fe80::/10
        || (segs[0] & 0xffc0) == 0xfe80
        // Documentation 2001:db8::/32
        || (segs[0] == 0x2001 && segs[1] == 0x0db8)
}

pub fn is_disallowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_disallowed_ipv4(v4),
        IpAddr::V6(v6) => is_disallowed_ipv6(v6),
    }
}

pub fn is_disallowed_hostname(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    h.is_empty()
        || h == "localhost"
        || h == "ip6-localhost"
        || h == "ip6-loopback"
        || h == "metadata"
        || h == "metadata.google.internal"
        || h.ends_with(".localhost")
        || h.ends_with(".local")
        || h.ends_with(".internal")
}

/// Validates URL syntax, scheme (`http`/`https`), hostname, and resolves DNS to ensure every IP is public
/// (returns verified socket addresses to pin DNS against rebinding).
pub async fn validate_public_url(raw_url: &str) -> AppResult<(Url, String, Vec<SocketAddr>)> {
    let parsed = Url::parse(raw_url.trim())
        .map_err(|_| AppError::BadRequest("Geçerli bir URL gerekli.".to_string()))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::BadRequest("Geçerli bir URL gerekli.".to_string()));
    }

    let host_str = parsed
        .host_str()
        .ok_or_else(|| AppError::BadRequest("Geçerli bir URL gerekli.".to_string()))?
        .to_string();

    if is_disallowed_hostname(&host_str) {
        return Err(AppError::BadRequest(
            "Güvenlik nedeniyle dahili/yerel adreslere erişim engellendi.".to_string(),
        ));
    }

    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| AppError::BadRequest("Geçerli bir URL portu gerekli.".to_string()))?;

    let clean_ip_candidate = host_str.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip_lit) = clean_ip_candidate.parse::<IpAddr>() {
        if is_disallowed_ip(ip_lit) {
            return Err(AppError::BadRequest(
                "Güvenlik nedeniyle özel ağ adreslerine erişim engellendi.".to_string(),
            ));
        }
        return Ok((parsed, host_str, vec![SocketAddr::new(ip_lit, port)]));
    }

    let resolved: Vec<SocketAddr> = tokio::net::lookup_host((host_str.as_str(), port))
        .await
        .map_err(|_| AppError::Internal("Görsel sunucusu çözümlenemedi.".to_string()))?
        .collect();

    if resolved.is_empty() {
        return Err(AppError::Internal(
            "Görsel sunucusu çözümlenemedi.".to_string(),
        ));
    }

    for sock in &resolved {
        if is_disallowed_ip(sock.ip()) {
            return Err(AppError::BadRequest(
                "Güvenlik nedeniyle özel ağ adreslerine erişim engellendi.".to_string(),
            ));
        }
    }

    Ok((parsed, host_str, resolved))
}

fn sanitize_original_name(raw: &str, fallback_ext: &str) -> String {
    let base = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim();
    if base.is_empty() {
        return format!("gorsel.{fallback_ext}");
    }
    let clean: String = base
        .chars()
        .filter(|c| !c.is_control())
        .take(240)
        .collect();
    if clean.is_empty() {
        format!("gorsel.{fallback_ext}")
    } else {
        clean
    }
}

fn generate_unique_stem() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let rand_num = OsRng.next_u64() % 1_000_000_000;
    format!("{ts}-{rand_num}")
}

fn blend_rgba(dst: &mut Rgba<u8>, src: [u8; 4]) {
    let sa = src[3] as f32 / 255.0;
    if sa <= 0.0 {
        return;
    }
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return;
    }
    for c in 0..3 {
        let sc = src[c] as f32 / 255.0;
        let dc = dst[c] as f32 / 255.0;
        let out_c = (sc * sa + dc * da * (1.0 - sa)) / out_a;
        dst[c] = (out_c * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn rounded_rect_sdf(px: f32, py: f32, x0: f32, y0: f32, w: f32, h: f32, r: f32) -> f32 {
    let cx = x0 + w * 0.5;
    let cy = y0 + h * 0.5;
    let hx = (w * 0.5 - r).max(0.0);
    let hy = (h * 0.5 - r).max(0.0);
    let dx = (px - cx).abs() - hx;
    let dy = (py - cy).abs() - hy;
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    let inside = dx.max(dy).min(0.0);
    outside + inside - r
}

fn draw_rounded_badge(
    canvas: &mut RgbaImage,
    x0: u32,
    y0: u32,
    w: u32,
    h: u32,
    radius: u32,
    fill: [u8; 4],
    border: Option<[u8; 4]>,
) {
    let (cw, ch) = canvas.dimensions();
    let x_end = x0.saturating_add(w).min(cw);
    let y_end = y0.saturating_add(h).min(ch);
    let r = (radius as f32).min(w as f32 * 0.5).min(h as f32 * 0.5);

    for py in y0..y_end {
        for px in x0..x_end {
            let dist = rounded_rect_sdf(
                px as f32 + 0.5,
                py as f32 + 0.5,
                x0 as f32,
                y0 as f32,
                w as f32,
                h as f32,
                r,
            );
            if dist <= 0.5 {
                let edge_alpha = (0.5 - dist).clamp(0.0, 1.0);
                let mut f = fill;
                f[3] = ((fill[3] as f32) * edge_alpha).round() as u8;
                blend_rgba(canvas.get_pixel_mut(px, py), f);

                if let Some(bcol) = border {
                    if dist >= -1.25 {
                        let border_alpha = (1.0 - (dist + 0.5).abs()).clamp(0.0, 1.0);
                        let mut bc = bcol;
                        bc[3] = ((bcol[3] as f32) * border_alpha).round() as u8;
                        blend_rgba(canvas.get_pixel_mut(px, py), bc);
                    }
                }
            }
        }
    }
}

fn apply_brand_watermark(canvas: &mut RgbaImage, static_dir: &str) {
    let (cw, ch) = canvas.dimensions();
    if cw < 32 || ch < 24 {
        return;
    }

    let logo_path = Path::new(static_dir).join("images").join("logo.png");
    if let Ok(logo_img) = image::open(&logo_path) {
        let (lw, lh) = logo_img.dimensions();
        if lw > 0 && lh > 0 {
            let max_allowed_w = cw.saturating_sub(16).max(1);
            let target_w = ((cw as f32 * 0.18).round() as u32)
                .clamp(90, 220)
                .min(max_allowed_w);
            let target_h = ((lh as f64 * target_w as f64 / lw as f64).round() as u32)
                .max(1)
                .min(ch.saturating_sub(16).max(1));

            let resized_logo =
                image::imageops::resize(&logo_img.to_rgba8(), target_w, target_h, FilterType::Lanczos3);

            let pad_x = (target_w / 8).clamp(8, 16);
            let pad_y = (target_h / 4).clamp(6, 12);
            let badge_w = target_w.saturating_add(pad_x * 2).min(cw);
            let badge_h = target_h.saturating_add(pad_y * 2).min(ch);
            let margin = (cw / 55).clamp(10, 24);
            let bx = cw.saturating_sub(badge_w.saturating_add(margin));
            let by = ch.saturating_sub(badge_h.saturating_add(margin));
            let radius = (badge_h / 4).clamp(6, 12);

            draw_rounded_badge(
                canvas,
                bx,
                by,
                badge_w,
                badge_h,
                radius,
                [10, 15, 28, 188],
                Some([59, 130, 246, 115]),
            );

            let logo_x = bx + badge_w.saturating_sub(target_w) / 2;
            let logo_y = by + badge_h.saturating_sub(target_h) / 2;
            image::imageops::overlay(canvas, &resized_logo, i64::from(logo_x), i64::from(logo_y));
            return;
        }
    }

    // Fallback crisp semi-transparent brand-blue & dark watermark bar in bottom-right
    let bar_w = ((cw as f32 * 0.20).round() as u32)
        .clamp(96, 220)
        .min(cw.saturating_sub(12).max(1));
    let bar_h = (bar_w / 4).clamp(24, 44).min(ch.saturating_sub(12).max(1));
    let margin = (cw / 55).clamp(8, 24);
    let bx = cw.saturating_sub(bar_w.saturating_add(margin));
    let by = ch.saturating_sub(bar_h.saturating_add(margin));
    let radius = (bar_h / 3).clamp(5, 10);

    draw_rounded_badge(
        canvas,
        bx,
        by,
        bar_w,
        bar_h,
        radius,
        [10, 15, 28, 200],
        Some([37, 99, 235, 165]),
    );

    // Brand-blue accent pill inside left side of fallback badge
    let accent_pad = (bar_h / 5).clamp(4, 8);
    let accent_w = (bar_h.saturating_sub(accent_pad * 2)).max(6);
    let accent_h = accent_w;
    draw_rounded_badge(
        canvas,
        bx + accent_pad,
        by + accent_pad,
        accent_w,
        accent_h,
        (accent_w / 3).max(2),
        [37, 99, 235, 238],
        None,
    );

    // Subtle crisp white double-line brand bar inside right side
    let text_x = bx + accent_pad * 2 + accent_w;
    let avail_w = (bx + bar_w).saturating_sub(text_x + accent_pad);
    if avail_w > 12 {
        let top_h = (bar_h / 6).clamp(3, 6);
        let top_y = by + bar_h / 3 - top_h / 2;
        draw_rounded_badge(
            canvas,
            text_x,
            top_y,
            avail_w,
            top_h,
            2,
            [248, 250, 252, 230],
            None,
        );
        let sub_w = (avail_w * 2 / 3).max(8);
        let sub_h = (bar_h / 8).clamp(2, 4);
        let sub_y = by + (bar_h * 2 / 3);
        draw_rounded_badge(
            canvas,
            text_x,
            sub_y,
            sub_w,
            sub_h,
            2,
            [147, 197, 253, 205],
            None,
        );
    }
}

fn encode_webp_bytes(img: &DynamicImage) -> AppResult<Vec<u8>> {
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::WebP)
        .map_err(|e| AppError::Internal(format!("WebP kodlama hatası: {e}")))?;
    Ok(buf)
}

/// Validates magic bytes, resizes >1920px wide images proportionally (leaving `1200x675` crops untouched),
/// optionally composites the MSI Dergisi watermark badge, writes `.webp` + `400x225` `-thumb.webp`,
/// and returns exact dimensions & metadata.
pub fn process_and_store_image(
    upload_dir: &str,
    static_dir: &str,
    bytes: &[u8],
    orig_name: &str,
    apply_watermark: bool,
) -> AppResult<ProcessedMedia> {
    if bytes.is_empty() {
        return Err(AppError::BadRequest("Görsel verisi boş.".to_string()));
    }
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(AppError::BadRequest(
            "Dosya boyutu çok büyük (maksimum 10MB).".to_string(),
        ));
    }

    std::fs::create_dir_all(upload_dir)
        .map_err(|e| AppError::Internal(format!("Yükleme dizini hazırlanamadı: {e}")))?;

    let magic_ext = detect_image_extension(bytes).ok_or_else(|| {
        AppError::BadRequest("Yalnızca geçerli görsel dosyaları yüklenebilir.".to_string())
    })?;
    if magic_ext == "svg" {
        crate::engine::validation_engine::validate_svg_safe(bytes).map_err(|_| {
            AppError::BadRequest("Güvenlik nedeniyle geçersiz SVG içeriği reddedildi.".to_string())
        })?;
    }

    let safe_orig = sanitize_original_name(orig_name, magic_ext);
    let stem = generate_unique_stem();

    // Non-raster formats (`svg`, `avif`) are stored safely as-is when `image::load_from_memory` cannot decode them
    if magic_ext == "svg" || magic_ext == "avif" {
        let filename = format!("{stem}.{magic_ext}");
        let full_path = Path::new(upload_dir).join(&filename);
        std::fs::write(&full_path, bytes)
            .map_err(|e| AppError::Internal(format!("Dosya kaydedilemedi: {e}")))?;
        let mime_type = if magic_ext == "svg" {
            "image/svg+xml".to_string()
        } else {
            "image/avif".to_string()
        };
        return Ok(ProcessedMedia {
            filename: filename.clone(),
            original_name: safe_orig,
            url: format!("/uploads/editorial/{filename}"),
            thumb_url: None,
            mime_type,
            width: 0,
            height: 0,
            size_bytes: bytes.len() as i64,
            watermarked: false,
        });
    }

    // Raster formats (`jpg`, `png`, `webp`, `gif`, `bmp`)
    let decoded = image::load_from_memory(bytes).map_err(|_| {
        AppError::BadRequest("Görsel dosyası bozuk veya çözümlenemedi.".to_string())
    })?;

    let (orig_w, orig_h) = decoded.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return Err(AppError::BadRequest("Geçersiz görsel boyutları.".to_string()));
    }

    let working_img = if orig_w > MAX_IMAGE_WIDTH {
        let scaled_h = ((orig_h as f64 * MAX_IMAGE_WIDTH as f64 / orig_w as f64).round() as u32)
            .max(1);
        let resized_rgba = image::imageops::resize(
            &decoded.to_rgba8(),
            MAX_IMAGE_WIDTH,
            scaled_h,
            FilterType::Lanczos3,
        );
        DynamicImage::ImageRgba8(resized_rgba)
    } else {
        decoded
    };

    let final_img = if apply_watermark {
        let mut rgba = working_img.to_rgba8();
        apply_brand_watermark(&mut rgba, static_dir);
        DynamicImage::ImageRgba8(rgba)
    } else {
        working_img
    };

    let (final_w, final_h) = final_img.dimensions();
    let main_webp = encode_webp_bytes(&final_img)?;

    let filename = format!("{stem}.webp");
    let main_path = Path::new(upload_dir).join(&filename);
    std::fs::write(&main_path, &main_webp)
        .map_err(|e| AppError::Internal(format!("WebP dosyası kaydedilemedi: {e}")))?;

    // Generate 400x225 cover-cropped thumbnail
    let thumb_img = final_img.resize_to_fill(THUMB_WIDTH, THUMB_HEIGHT, FilterType::Lanczos3);
    let thumb_webp = encode_webp_bytes(&thumb_img)?;
    let thumb_filename = format!("{stem}-thumb.webp");
    let thumb_path = Path::new(upload_dir).join(&thumb_filename);
    std::fs::write(&thumb_path, &thumb_webp)
        .map_err(|e| AppError::Internal(format!("Küçük görsel (thumbnail) kaydedilemedi: {e}")))?;

    Ok(ProcessedMedia {
        filename: filename.clone(),
        original_name: safe_orig,
        url: format!("/uploads/editorial/{filename}"),
        thumb_url: Some(format!("/uploads/editorial/{thumb_filename}")),
        mime_type: "image/webp".to_string(),
        width: final_w as i32,
        height: final_h as i32,
        size_bytes: main_webp.len() as i64,
        watermarked: apply_watermark,
    })
}

fn is_truthy_flag(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Parses multipart upload fields (`image`/`file`, optional `watermark`, optional `alt`/`altText`).
pub async fn parse_multipart_upload(
    mut multipart: Multipart,
    query_watermark: bool,
) -> AppResult<MultipartUploadParsed> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut orig_name: Option<String> = None;
    let mut field_watermark = false;
    let mut alt_text: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("Dosya yüklenemedi.".to_string()))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        let has_filename = field.file_name().is_some();

        if field_name == "watermark" {
            if let Ok(txt) = field.text().await {
                if is_truthy_flag(&txt) {
                    field_watermark = true;
                }
            }
            continue;
        }

        if field_name == "alt" || field_name == "altText" || field_name == "alt_text" {
            if let Ok(txt) = field.text().await {
                let clean = txt.trim().to_string();
                if !clean.is_empty() {
                    alt_text = Some(clean);
                }
            }
            continue;
        }

        if field_name == "image" || field_name == "file" || (has_filename && file_bytes.is_none()) {
            let fname = field.file_name().map(|s| s.to_string());
            let data = field.bytes().await.map_err(|_| {
                AppError::BadRequest("Dosya okunamadı veya boyut sınırı aşıldı.".to_string())
            })?;
            if data.is_empty() {
                continue;
            }
            if data.len() > MAX_UPLOAD_BYTES {
                return Err(AppError::BadRequest(
                    "Dosya boyutu çok büyük (maksimum 10MB).".to_string(),
                ));
            }
            if field_name == "image" || file_bytes.is_none() {
                orig_name = fname;
                file_bytes = Some(data.to_vec());
            }
        }
    }

    let bytes =
        file_bytes.ok_or_else(|| AppError::BadRequest("Dosya yüklenemedi.".to_string()))?;

    Ok(MultipartUploadParsed {
        bytes,
        orig_name: orig_name.unwrap_or_else(|| "upload.webp".to_string()),
        watermark: query_watermark || field_watermark,
        alt_text,
    })
}

/// Fetches a remote image byte buffer with SSRF & redirect protection.
pub async fn download_public_image_bytes(raw_url: &str) -> AppResult<(Vec<u8>, String)> {
    if !raw_url.trim().starts_with("http://") && !raw_url.trim().starts_with("https://") {
        return Err(AppError::BadRequest("Geçerli bir URL gerekli.".to_string()));
    }

    let mut current_url = raw_url.trim().to_string();

    for _hop in 0..=MAX_REDIRECTS {
        let (parsed_url, host, addrs) = validate_public_url(&current_url).await?;

        let mut builder = reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(15));
        if parsed_url.host_str().is_some() && !host.starts_with('[') {
            builder = builder.resolve_to_addrs(&host, &addrs);
        }
        let client = builder
            .build()
            .map_err(|e| AppError::Internal(format!("HTTP istemcisi başlatılamadı: {e}")))?;

        let origin = format!(
            "{}://{}",
            parsed_url.scheme(),
            parsed_url.host_str().unwrap_or("")
        );

        let resp = client
            .get(parsed_url.clone())
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
            )
            .header(
                "Accept",
                "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Referer", &origin)
            .header("Cache-Control", "no-cache")
            .header("Pragma", "no-cache")
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AppError::Internal("Görsel çekme zaman aşımına uğradı.".to_string())
                } else {
                    AppError::Internal("Görsel çekilemedi.".to_string())
                }
            })?;

        let status = resp.status();
        if status.is_redirection() {
            if let Some(loc) = resp.headers().get(reqwest::header::LOCATION) {
                if let Ok(loc_str) = loc.to_str() {
                    let next_url = parsed_url
                        .join(loc_str)
                        .map_err(|_| AppError::BadRequest("Geçersiz yönlendirme URL'si.".to_string()))?;
                    current_url = next_url.to_string();
                    continue;
                }
            }
            return Err(AppError::Internal("Görsel çekilemedi.".to_string()));
        }

        if status.as_u16() == 403 {
            return Err(AppError::Internal(
                "Görsel kaynağı erişimi reddetti (403). Lütfen farklı bir görsel deneyin."
                    .to_string(),
            ));
        }
        if status.as_u16() == 404 {
            return Err(AppError::Internal("Görsel bulunamadı (404).".to_string()));
        }
        if !status.is_success() {
            return Err(AppError::Internal(format!(
                "Görsel çekme hatası ({}).",
                status.as_u16()
            )));
        }

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if let Some(len) = resp.content_length() {
            if len as usize > MAX_UPLOAD_BYTES {
                return Err(AppError::BadRequest(
                    "Görsel boyutu çok büyük (maksimum 10MB).".to_string(),
                ));
            }
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|_| AppError::Internal("Görsel verisi okunamadı.".to_string()))?;

        if bytes.len() > MAX_UPLOAD_BYTES {
            return Err(AppError::BadRequest(
                "Görsel boyutu çok büyük (maksimum 10MB).".to_string(),
            ));
        }

        return Ok((bytes.to_vec(), content_type));
    }

    Err(AppError::Internal(
        "Çok fazla yönlendirme nedeniyle görsel çekilemedi.".to_string(),
    ))
}

/// Fetches a remote image, processes it into WebP (+ thumbnail + optional watermark), and returns `ProcessedMedia`.
pub async fn fetch_and_process_remote_image(
    upload_dir: &str,
    static_dir: &str,
    raw_url: &str,
    apply_watermark: bool,
) -> AppResult<ProcessedMedia> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty()
        || (!trimmed.starts_with("http://") && !trimmed.starts_with("https://"))
    {
        return Err(AppError::BadRequest("Geçerli bir URL gerekli.".to_string()));
    }

    let (bytes, content_type) = download_public_image_bytes(trimmed).await?;

    if !content_type.is_empty() && !content_type.to_ascii_lowercase().starts_with("image/") {
        if detect_image_extension(&bytes).is_none() {
            return Err(AppError::BadRequest(
                "URL bir görsel dosyasına işaret etmiyor.".to_string(),
            ));
        }
    }

    let orig_name = Url::parse(trimmed)
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|segs| segs.last())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "uzak-gorsel.webp".to_string());

    let up_dir = upload_dir.to_string();
    let st_dir = static_dir.to_string();
    tokio::task::spawn_blocking(move || {
        process_and_store_image(&up_dir, &st_dir, &bytes, &orig_name, apply_watermark)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Görsel işleme görevi hatası: {e}")))?
}

/// Validates a local `/uploads/...` URL, reads the file from `upload_dir`, and creates a watermarked WebP copy.
pub async fn watermark_local_upload_url(
    upload_dir: &str,
    static_dir: &str,
    raw_url: &str,
    orig_name_hint: Option<&str>,
) -> AppResult<ProcessedMedia> {
    let clean_url = raw_url.split('?').next().unwrap_or(raw_url).trim();
    let Some(rel_name) = clean_url
        .strip_prefix("/uploads/editorial/")
        .or_else(|| clean_url.strip_prefix("/uploads/"))
    else {
        return Err(AppError::BadRequest(
            "Yalnızca sunucudaki (/uploads/editorial/...) görsellere doğrudan filigran uygulanabilir."
                .to_string(),
        ));
    };

    let rel_trimmed = rel_name.trim();
    if rel_trimmed.is_empty()
        || rel_trimmed.contains('/')
        || rel_trimmed.contains('\\')
        || rel_trimmed.contains("..")
    {
        return Err(AppError::BadRequest(
            "Geçersiz yerel görsel yolu.".to_string(),
        ));
    }

    let src_path = Path::new(upload_dir).join(rel_trimmed);
    let bytes = tokio::fs::read(&src_path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound("Yerel görsel dosyası bulunamadı.".to_string())
        } else {
            AppError::Internal(format!("Görsel dosyası okunamadı: {e}"))
        }
    })?;

    let orig_name = orig_name_hint
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(rel_trimmed)
        .to_string();

    let up_dir = upload_dir.to_string();
    let st_dir = static_dir.to_string();
    tokio::task::spawn_blocking(move || {
        process_and_store_image(&up_dir, &st_dir, &bytes, &orig_name, true)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Filigran işleme hatası: {e}")))?
}

/// Safely removes the main file and optional thumbnail file of a `MediaAsset` from `upload_dir`.
pub async fn remove_media_files_from_disk(upload_dir: &str, asset: &MediaAsset) {
    let mut names_to_remove = Vec::with_capacity(2);
    if !asset.filename.is_empty()
        && !asset.filename.contains('/')
        && !asset.filename.contains('\\')
        && !asset.filename.contains("..")
    {
        names_to_remove.push(asset.filename.clone());
    }

    if let Some(ref thumb) = asset.thumb_url {
        let clean = thumb.split('?').next().unwrap_or(thumb).trim();
        if let Some(rel) = clean
            .strip_prefix("/uploads/editorial/")
            .or_else(|| clean.strip_prefix("/uploads/"))
        {
            if !rel.is_empty()
                && !rel.contains('/')
                && !rel.contains('\\')
                && !rel.contains("..")
            {
                names_to_remove.push(rel.to_string());
            }
        }
    }

    for fname in names_to_remove {
        let full = Path::new(upload_dir).join(fname);
        let _ = tokio::fs::remove_file(full).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn make_test_png_bytes(width: u32, height: u32) -> Vec<u8> {
        let img = ImageBuffer::from_fn(width, height, |x, y| {
            let r = ((x * 255) / width.max(1)) as u8;
            let g = ((y * 255) / height.max(1)) as u8;
            Rgb([r, g, 140])
        });
        let dyn_img = DynamicImage::ImageRgb8(img);
        let mut out = Vec::new();
        dyn_img
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .expect("encode test png");
        out
    }

    #[tokio::test]
    async fn ssrf_guard_blocks_loopback_private_metadata_and_ipv6_mapped() {
        assert!(is_disallowed_ip("127.0.0.1".parse().unwrap()));
        assert!(is_disallowed_ip("10.0.0.5".parse().unwrap()));
        assert!(is_disallowed_ip("172.16.12.1".parse().unwrap()));
        assert!(is_disallowed_ip("192.168.1.1".parse().unwrap()));
        assert!(is_disallowed_ip("169.254.169.254".parse().unwrap()));
        assert!(is_disallowed_ip("100.64.0.1".parse().unwrap()));
        assert!(is_disallowed_ip("0.0.0.0".parse().unwrap()));
        assert!(is_disallowed_ip("::1".parse().unwrap()));
        assert!(is_disallowed_ip("fe80::1".parse().unwrap()));
        assert!(is_disallowed_ip("fc00::1".parse().unwrap()));
        assert!(is_disallowed_ip("::ffff:127.0.0.1".parse().unwrap()));
        assert!(is_disallowed_ip("::ffff:169.254.169.254".parse().unwrap()));

        assert!(!is_disallowed_ip("93.184.216.34".parse().unwrap()));
        assert!(!is_disallowed_ip("1.1.1.1".parse().unwrap()));

        assert!(validate_public_url("http://127.0.0.1:3001/secret").await.is_err());
        assert!(validate_public_url("http://localhost/admin").await.is_err());
        assert!(validate_public_url("http://169.254.169.254/latest/meta-data/").await.is_err());
        assert!(validate_public_url("file:///etc/passwd").await.is_err());
        assert!(validate_public_url("http://[::ffff:10.0.0.1]/img.png").await.is_err());
    }

    #[test]
    fn detects_valid_image_magic_bytes_and_rejects_scripts() {
        assert_eq!(detect_image_extension(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(
            detect_image_extension(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            Some("png")
        );
        assert_eq!(detect_image_extension(b"GIF89a1234"), Some("gif"));
        assert_eq!(detect_image_extension(b"RIFF\x00\x00\x00\x00WEBP"), Some("webp"));
        assert_eq!(
            detect_image_extension(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>"),
            Some("svg")
        );
        assert_eq!(detect_image_extension(b"<svg><script>alert(1)</script></svg>"), None);
        assert_eq!(detect_image_extension(b"<html><body>not an image</body></html>"), None);
    }

    #[test]
    fn converts_to_webp_keeps_1200x675_resizes_over_1920_generates_400x225_thumb_and_watermarks() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "hf_media_test_{}",
            OsRng.next_u64()
        ));
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let up_str = tmp_dir.to_string_lossy().to_string();
        let static_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/public");

        // 1. 1200x675 Google Discover crop stays exact 1200x675 WebP + 400x225 thumb
        let png_1200 = make_test_png_bytes(1200, 675);
        let proc_1200 =
            process_and_store_image(&up_str, static_dir, &png_1200, "kapak.png", false)
                .expect("process 1200x675");
        assert_eq!(proc_1200.width, 1200);
        assert_eq!(proc_1200.height, 675);
        assert_eq!(proc_1200.mime_type, "image/webp");
        assert!(proc_1200.filename.ends_with(".webp"));
        assert!(!proc_1200.watermarked);
        assert!(proc_1200.size_bytes > 0);

        let thumb_rel = proc_1200
            .thumb_url
            .as_deref()
            .unwrap()
            .strip_prefix("/uploads/editorial/")
            .unwrap();
        let (tw, th) = image::image_dimensions(tmp_dir.join(thumb_rel)).unwrap();
        assert_eq!((tw, th), (400, 225));

        // 2. 2400x1200 oversized image scales down proportionally to 1920x960 + watermark with real logo
        let png_large = make_test_png_bytes(2400, 1200);
        let proc_wm =
            process_and_store_image(&up_str, static_dir, &png_large, "genis.png", true)
                .expect("process 2400x1200 watermarked");
        assert_eq!(proc_wm.width, 1920);
        assert_eq!(proc_wm.height, 960);
        assert!(proc_wm.watermarked);

        // 3. Fallback watermark when static_dir has no logo file
        let proc_fallback = process_and_store_image(
            &up_str,
            "/nonexistent/static/path",
            &png_1200,
            "fallback.png",
            true,
        )
        .expect("process fallback watermark");
        assert_eq!(proc_fallback.width, 1200);
        assert_eq!(proc_fallback.height, 675);
        assert!(proc_fallback.watermarked);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
