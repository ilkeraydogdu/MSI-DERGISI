use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::config::AppConfig;

/// Validates CSRF protection for state-changing admin API requests.
///
/// Performs two checks:
/// 1. Origin header verification against allowed domains.
/// 2. Double-submit cookie token matching the `x-csrf-token` header.
///
/// Returns `None` if validation passes, or `Some(Response)` with an error
/// if validation fails.
fn extract_url_host(raw: &str) -> &str {
    let without_scheme = raw
        .trim()
        .strip_prefix("https://")
        .or_else(|| raw.trim().strip_prefix("http://"))
        .unwrap_or(raw.trim());
    let host_port = without_scheme.split('/').next().unwrap_or("");
    host_port.split(':').next().unwrap_or("")
}

fn host_matches_domain(origin_host: &str, domain: &str) -> bool {
    let oh = origin_host.trim().to_ascii_lowercase();
    let dom = extract_url_host(domain).to_ascii_lowercase();
    if oh.is_empty() || dom.is_empty() {
        return false;
    }
    oh == dom || oh.ends_with(&format!(".{dom}"))
}

fn origin_matches_allowed(raw_origin: &str, req: &Request, config: &AppConfig) -> bool {
    let req_host = req
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let origin_host = extract_url_host(raw_origin);
    !origin_host.is_empty()
        && (host_matches_domain(origin_host, req_host)
            || host_matches_domain(origin_host, &config.public_url)
            || host_matches_domain(origin_host, &config.primary_domain)
            || host_matches_domain(origin_host, &config.news_domain))
}

/// Validates same-origin `Origin` or `Referer` header when present on state-changing
/// cookie-authenticated reader endpoints (e.g. `/api/v1/account/*`).
pub fn validate_same_origin(req: &Request, config: &AppConfig) -> Option<Response> {
    if let Some(origin) = req.headers().get("origin").and_then(|v| v.to_str().ok()) {
        if !origin_matches_allowed(origin, req, config) {
            return Some(
                (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "error",
                        "code": "CSRF_ORIGIN",
                        "message": "Geçersiz istek kaynağı."
                    })),
                )
                    .into_response(),
            );
        }
    } else if let Some(referer) = req.headers().get("referer").and_then(|v| v.to_str().ok()) {
        if !origin_matches_allowed(referer, req, config) {
            return Some(
                (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "error",
                        "code": "CSRF_ORIGIN",
                        "message": "Geçersiz istek kaynağı."
                    })),
                )
                    .into_response(),
            );
        }
    }
    None
}

pub fn validate_csrf(req: &Request, config: &AppConfig) -> Option<Response> {
    // Origin / Referer verification
    if let Some(origin) = req.headers().get("origin").and_then(|v| v.to_str().ok()) {
        if !origin_matches_allowed(origin, req, config) {
            return Some(
                (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "error",
                        "code": "CSRF_ORIGIN",
                        "message": "Geçersiz istek kaynağı."
                    })),
                )
                    .into_response(),
            );
        }
    } else if let Some(referer) = req.headers().get("referer").and_then(|v| v.to_str().ok()) {
        if !origin_matches_allowed(referer, req, config) {
            return Some(
                (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "error",
                        "code": "CSRF_ORIGIN",
                        "message": "Geçersiz istek kaynağı."
                    })),
                )
                    .into_response(),
            );
        }
    }

    // Double-submit cookie token verification: ALWAYS enforce when msi_session cookie is present
    let uses_cookie_auth = cookie_value(req, "msi_session").is_some();

    match cookie_value(req, "csrf_token") {
        Some(cookie_token) => {
            let valid_sig = crate::engine::security_engine::validate_csrf_token(
                cookie_token,
                &config.session_secret,
            );
            let header_token = req
                .headers()
                .get("x-csrf-token")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if !valid_sig
                || header_token.is_empty()
                || !crate::utils::crypto::constant_time_eq(
                    header_token.as_bytes(),
                    cookie_token.as_bytes(),
                )
            {
                return Some(csrf_error_response());
            }
        }
        None if uses_cookie_auth => {
            return Some(csrf_error_response());
        }
        None => {}
    }

    None
}

fn csrf_error_response() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "status": "error",
            "code": "CSRF_TOKEN",
            "message": "Güvenlik doğrulaması başarısız. Sayfayı yenileyip tekrar deneyin."
        })),
    )
        .into_response()
}

fn cookie_value<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    let prefix = format!("{}=", name);
    req.headers()
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|cookies| cookies.split(';'))
        .find_map(|p| {
            p.trim()
                .strip_prefix(prefix.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[test]
    fn csrf_rejects_missing_cookie_when_using_session_cookie() {
        let cfg = AppConfig::from_env();
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/cache")
            .header("cookie", "msi_session=dummy")
            .body(Body::empty())
            .unwrap();
        let res = validate_csrf(&req, &cfg).expect("missing csrf_token cookie must be rejected");
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn csrf_rejects_mismatched_or_forged_tokens() {
        let cfg = AppConfig::from_env();
        let valid = crate::engine::security_engine::generate_csrf_token(&cfg.session_secret);
        let req_mismatch = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/cache")
            .header("cookie", format!("msi_session=dummy; csrf_token={}", valid))
            .header("x-csrf-token", "wrong-token")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            validate_csrf(&req_mismatch, &cfg).unwrap().status(),
            StatusCode::FORBIDDEN
        );

        let forged = "1700000000.forgedsignature";
        let req_forged = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/cache")
            .header("cookie", format!("msi_session=dummy; csrf_token={}", forged))
            .header("x-csrf-token", forged)
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            validate_csrf(&req_forged, &cfg).unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn csrf_accepts_valid_signed_double_submit_token() {
        let cfg = AppConfig::from_env();
        let valid = crate::engine::security_engine::generate_csrf_token(&cfg.session_secret);
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/cache")
            .header("cookie", format!("msi_session=dummy; csrf_token={}", valid))
            .header("x-csrf-token", &valid)
            .body(Body::empty())
            .unwrap();
        assert!(validate_csrf(&req, &cfg).is_none());
    }
}