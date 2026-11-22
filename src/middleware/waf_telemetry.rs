use axum::{
    extract::{ConnectInfo, Request, State},
    http::{
        header::{HeaderName, COOKIE},
        HeaderValue, Method, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::time::Instant;

use crate::controllers::AppState;
use crate::engine::security_engine::{self, CentralSecurityFilter, CspNonce, ResolvedClientIp};
use crate::middleware::auth_gate::{check_admin_access, AdminAccess};
use crate::middleware::csrf::validate_csrf;

/// Resolves the real client IP. Proxy headers are only trusted when the TCP peer is loopback
/// (i.e. the request came through the local Plesk/nginx reverse proxy) in non-test environments
/// (or when `ConnectInfo` is absent in unit tests); direct non-loopback peers never trust proxy
/// headers, and direct loopback requests in test environments ignore `X-Forwarded-For` while
/// flagging `was_forwarded = true` so spoofed `X-Forwarded-For` cannot rotate IPs or bypass limits.
fn resolve_client_ip(req: &Request, app_env: &str) -> ResolvedClientIp {
    let peer: Option<IpAddr> = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip());
    let has_proxy_headers = req.headers().contains_key("x-real-ip")
        || req.headers().contains_key("x-forwarded-for");

    let trust_proxy = match peer {
        Some(ip) => ip.is_loopback() && !app_env.eq_ignore_ascii_case("test"),
        None => true,
    };

    if trust_proxy {
        if let Some(real) = req
            .headers()
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| s.parse::<IpAddr>().is_ok())
        {
            return ResolvedClientIp {
                ip: real.to_string(),
                was_forwarded: true,
            };
        }
        // Right-most entry is the one appended by our own reverse proxy (client-supplied entries are to the left).
        if let Some(xff) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.rsplit(',').map(str::trim).find(|p| p.parse::<IpAddr>().is_ok()))
        {
            return ResolvedClientIp {
                ip: xff.to_string(),
                was_forwarded: true,
            };
        }
    }

    ResolvedClientIp {
        ip: peer
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string()),
        was_forwarded: has_proxy_headers && peer.map(|ip| ip.is_loopback()).unwrap_or(true),
    }
}

fn cookie_value<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    let prefix = format!("{}=", name);
    req.headers()
        .get_all(COOKIE)
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

fn is_private_path(path: &str, admin_path: &str) -> bool {
    path == admin_path
        || path.starts_with(&format!("{}/", admin_path))
        || path == "/podmin"
        || path.starts_with("/podmin/")
        || path.starts_with("/hesabim")
        || path.starts_with("/hesap-dogrula")
        || path.starts_with("/abonelik-iptal")
        || path.starts_with("/sifre-sifirla")
        || path.starts_with("/sifre-yenile")
        || path.starts_with("/api/")
}

pub async fn central_waf_and_telemetry_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let t0 = Instant::now();
    let resolved_ip = resolve_client_ip(&req, &state.config.app_env);
    let client_ip = resolved_ip.ip.clone();
    req.extensions_mut().insert(resolved_ip);

    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let query_opt = req.uri().query().map(|q| q.to_string());
    let ua_opt = req
        .headers()
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Block access to sensitive files and directories before any further processing.
    let is_sensitive_path = path == "/.env"
        || path.starts_with("/.env.")
        || path.starts_with("/.git/")
        || path == "/.git"
        || path == "/Cargo.toml"
        || path == "/Cargo.lock"
        || path.starts_with("/target/")
        || path.starts_with("/storage/db/")
        || path.starts_with("/.claude/");
    if is_sensitive_path {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "code": "SENSITIVE_PATH_BLOCKED",
                "message": "Access to this resource is forbidden."
            })),
        )
            .into_response();
    }

    // Deep WAF inspection (Rate Limiter + SQLi + XSS + Path Traversal)
    if let Err(log_entry) = CentralSecurityFilter::inspect_request_security(
        &path,
        query_opt.as_deref(),
        ua_opt.as_deref(),
        &client_ip,
        state.config.rate_limit_max_requests_per_minute,
        state.config.rate_limit_window_secs,
    ) {
        let _ = state.db.insert_security_audit_log(&log_entry);
        let _ = crate::services::security_anomaly_service::evaluate_and_emit_anomalies(&state.db);
        let status = if log_entry.event_type == "RATE_LIMIT_EXCEEDED" {
            StatusCode::TOO_MANY_REQUESTS
        } else {
            StatusCode::FORBIDDEN
        };
        return (
            status,
            Json(json!({
                "status": "error",
                "code": "BLOCKED_BY_WAF",
                "message": if status == StatusCode::TOO_MANY_REQUESTS {
                    "Çok fazla istek gönderildi. Lütfen kısa bir süre sonra tekrar deneyin."
                } else {
                    "İstek güvenlik politikası nedeniyle engellendi."
                }
            })),
        )
            .into_response();
    }

    let admin_seg = crate::config::normalize_admin_path_secret(&state.config.admin_path_secret);
    let admin_path = format!("/{}", admin_seg);
    let is_podmin_root = path == "/podmin";
    let is_podmin_subpage = path.starts_with("/podmin/");
    let is_admin_root = path == admin_path || is_podmin_root;
    let is_admin_subpage = path.starts_with(&format!("{}/", admin_path)) || is_podmin_subpage;
    let is_admin_api = path.starts_with("/api/v1/admin/");

    // Operator gate. The admin root (`/{ADMIN_PATH_SECRET}`) is intentionally NOT gated here:
    // `web_controller::render_admin` serves the dedicated command login screen to anonymous users.
    let state_changing = matches!(
        method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    if is_admin_subpage || is_admin_api {
        let redirect_target = if is_podmin_subpage { "/podmin" } else { admin_path.as_str() };
        match check_admin_access(&state, &req, redirect_target, is_admin_api) {
            AdminAccess::Allowed => {}
            AdminAccess::Denied(response) => return response,
        }

        // CSRF: double-submit cookie + Origin/Referer verification on state-changing admin requests.
        if (is_admin_subpage || is_admin_api) && state_changing {
            if let Some(csrf_error) = validate_csrf(&req, &state.config) {
                return csrf_error;
            }
        }
    } else if state_changing
        && (path.starts_with("/api/v1/account/")
            || path.starts_with("/api/v1/auth/totp/")
            || path == "/api/v1/auth/logout"
            || path == "/api/v1/subscriptions/checkout"
            || path == "/api/v1/commerce/cart-checkout"
            || path == "/api/v1/email-preferences"
            || path.starts_with("/api/v1/notifications/"))
    {
        if let Some(csrf_error) = validate_csrf(&req, &state.config) {
            return csrf_error;
        }
    }

    let needs_csrf_cookie = cookie_value(&req, "csrf_token")
        .map(|t| !security_engine::validate_csrf_token(t, &state.config.session_secret))
        .unwrap_or(true);

    let forwarded_https = req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false);
    let is_loopback_http = !forwarded_https
        && req
            .headers()
            .get(axum::http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .map(|h| h.starts_with("127.0.0.1") || h.starts_with("localhost"))
            .unwrap_or(false);
    let cookie_secure = if forwarded_https || (state.config.public_url.starts_with("https://") && !is_loopback_http) {
        "; Secure"
    } else {
        ""
    };

    let csp_nonce = security_engine::generate_csp_nonce();
    req.extensions_mut().insert(CspNonce(csp_nonce.clone()));

    let mut response = next.run(req).await;
    let exec_us = t0.elapsed().as_micros();

    for (k, v) in security_engine::security_headers_list_with_nonce(exec_us, Some(&csp_nonce)) {
        if let (Ok(h_name), Ok(h_val)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_str(&v),
        ) {
            response.headers_mut().insert(h_name, h_val);
        }
    }
    let dur_ms = (exec_us as f64) / 1000.0;
    if let Ok(st_val) =
        HeaderValue::from_str(&format!("msi_ssr;dur={:.2};desc=\"MSI Rust SSR\"", dur_ms))
    {
        response
            .headers_mut()
            .insert(HeaderName::from_static("server-timing"), st_val);
    }

    let is_static = path.starts_with("/images/")
        || path.starts_with("/css/")
        || path.starts_with("/js/")
        || path.starts_with("/fonts/")
        || path.starts_with("/uploads/");

    if is_private_path(&path, &admin_path) {
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, private"),
        );
    }
    let is_noindex_path = is_admin_root
        || is_admin_subpage
        || path.starts_with("/hesabim")
        || path.starts_with("/sifre-")
        || path == "/hesap-dogrula"
        || path == "/abonelik-iptal"
        || path.starts_with("/api/");
    if is_noindex_path {
        response.headers_mut().insert(
            HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_static("noindex, nofollow, noarchive"),
        );
    } else if response.status() == StatusCode::NOT_FOUND {
        response.headers_mut().insert(
            HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_static("noindex, nofollow"),
        );
    }
    if needs_csrf_cookie && !is_static {
        let token = security_engine::generate_csrf_token(&state.config.session_secret);
        if let Ok(v) = HeaderValue::from_str(&format!(
            "csrf_token={}; Path=/; SameSite=Lax; Max-Age=3600{}",
            token, cookie_secure
        )) {
            response
                .headers_mut()
                .append(axum::http::header::SET_COOKIE, v);
        }
    }
    if response.status().is_success()
        && !path.starts_with("/api/")
        && !is_static
        && !is_admin_root
        && !is_admin_subpage
    {
        let canonical_url = format!(
            "{}{}",
            state.config.public_url.trim_end_matches('/'),
            path
        );
        let link_header = format!(
            "<{}>; rel=\"canonical\", </llms.txt>; rel=\"describedby\"; type=\"text/plain\", </feed.json>; rel=\"alternate\"; type=\"application/feed+json\"",
            canonical_url
        );
        if let Ok(v) = HeaderValue::from_str(&link_header) {
            response
                .headers_mut()
                .insert(HeaderName::from_static("link"), v);
        }
    }

    response
}