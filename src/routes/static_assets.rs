use axum::{
    extract::Request,
    http::{header::HeaderName, HeaderValue},
    middleware::{self, Next},
    response::Response,
    Router,
};
use tower_http::services::ServeDir;

use crate::controllers::AppState;

fn decode_path_lower(raw_path: &str) -> String {
    let decoded = urlencoding::decode_binary(raw_path.as_bytes());
    String::from_utf8_lossy(&decoded).to_ascii_lowercase()
}

fn normalize_path_segments(path: &str) -> Option<String> {
    let mut segments: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            segments.pop()?;
            continue;
        }
        segments.push(seg);
    }
    if segments.is_empty() {
        Some("/".to_string())
    } else {
        Some(format!("/{}", segments.join("/")))
    }
}

pub(crate) fn apply_uploaded_svg_headers(raw_path: &str, resp: &mut Response) {
    let raw_lc = raw_path.to_ascii_lowercase();
    let decoded_lc = decode_path_lower(raw_path);
    let norm_raw = normalize_path_segments(&raw_lc);
    let norm_decoded = normalize_path_segments(&decoded_lc);

    let is_upload_path = |p: &str| {
        p.starts_with("/uploads/editorial/") || p.starts_with("/images/uploads/")
    };
    let in_upload_dir = norm_decoded.as_deref().map(is_upload_path).unwrap_or(false)
        || norm_raw.as_deref().map(is_upload_path).unwrap_or(false);
    if !in_upload_dir {
        return;
    }
    let has_svg_ext = norm_decoded
        .as_deref()
        .map(|p| p.ends_with(".svg"))
        .unwrap_or(false)
        || norm_raw
            .as_deref()
            .map(|p| p.ends_with(".svg"))
            .unwrap_or(false);
    let has_svg_ct = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.trim().to_ascii_lowercase().starts_with("image/svg+xml"))
        .unwrap_or(false);
    if has_svg_ext || has_svg_ct {
        resp.headers_mut().insert(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        );
        resp.headers_mut().insert(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("sandbox; default-src 'none'"),
        );
    }
}

async fn harden_uploaded_svg_headers(req: Request, next: Next) -> Response {
    let raw_path = req.uri().path().to_string();
    let mut resp = next.run(req).await;
    apply_uploaded_svg_headers(&raw_path, &mut resp);
    resp
}

pub fn build(base_dir: &str) -> Router<AppState> {
    Router::new()
        .nest_service("/css", ServeDir::new(format!("{}/css", base_dir)))
        .nest_service("/js", ServeDir::new(format!("{}/js", base_dir)))
        .nest_service("/images", ServeDir::new(format!("{}/images", base_dir)))
        .nest_service("/fonts", ServeDir::new(format!("{}/fonts", base_dir)))
        .layer(middleware::from_fn(harden_uploaded_svg_headers))
}

pub fn build_with_uploads(base_dir: &str, editorial_upload_dir: &str) -> Router<AppState> {
    build(base_dir)
        .nest_service("/uploads/editorial", ServeDir::new(editorial_upload_dir))
        .layer(middleware::from_fn(harden_uploaded_svg_headers))
}