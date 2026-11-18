use super::*;

/// In-memory pending TOTP setup secrets keyed by lowercase user email -> `(secret_base32, issued_ts)`.
/// Prevents half-finished 2FA setups from locking out a user's database record before verification.
pub(super) static PENDING_TOTP_SETUPS: Lazy<RwLock<HashMap<String, (String, i64)>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub(super) const PENDING_TOTP_SETUP_TTL_SECS: i64 = 900;

/// Centralized JSON error response helper.
pub(super) fn json_error(state: &AppState, status: StatusCode, msg_key: &'static str) -> Response {
    let error_code = match status {
        StatusCode::NOT_FOUND => error_codes::NOT_FOUND,
        StatusCode::UNAUTHORIZED => error_codes::UNAUTHORIZED,
        StatusCode::FORBIDDEN => error_codes::FORBIDDEN,
        StatusCode::CONFLICT => error_codes::CONFLICT,
        StatusCode::TOO_MANY_REQUESTS => "TOO_MANY_REQUESTS",
        StatusCode::SERVICE_UNAVAILABLE => error_codes::SERVICE_UNAVAILABLE,
        StatusCode::INTERNAL_SERVER_ERROR => error_codes::INTERNAL_ERROR,
        _ => error_codes::BAD_REQUEST,
    };
    ApiResponse::error(&state.translations, "tr", status, error_code, msg_key).into_response()
}

pub(super) fn extract_client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("127.0.0.1")
        .trim()
        .to_string()
}

pub(super) fn extract_raw_session_cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .find_map(|p| p.trim().strip_prefix("msi_session="))
        })
        .filter(|t| !t.trim().is_empty())
}

pub(super) fn cookie_secure_flag(headers: &HeaderMap, public_url: &str) -> &'static str {
    let forwarded_https = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false);
    let is_loopback_http = !forwarded_https
        && headers
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .map(|h| h.starts_with("127.0.0.1") || h.starts_with("localhost"))
            .unwrap_or(false);
    if forwarded_https || (public_url.starts_with("https://") && !is_loopback_http) {
        "; Secure"
    } else {
        ""
    }
}

#[allow(clippy::result_large_err)]
pub(super) fn require_perm(operator: &OperatorUser, perm: &str) -> Result<(), Response> {
    check_permission(operator, perm).map_err(IntoResponse::into_response)
}

#[allow(clippy::result_large_err)]
pub(super) fn require_any_perm(operator: &OperatorUser, perms: &[&str]) -> Result<(), Response> {
    if perms.iter().any(|p| operator.has_permission(p)) {
        Ok(())
    } else {
        let primary = perms.first().copied().unwrap_or("admin:access");
        check_permission(operator, primary).map_err(IntoResponse::into_response)
    }
}

pub(super) fn is_token_expired(expires_at: &str) -> bool {
    let s = expires_at.trim();
    if s.is_empty() {
        return true;
    }
    let now = chrono::Utc::now();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return now > dt.with_timezone(&chrono::Utc);
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
    {
        return now > naive.and_utc();
    }
    true
}

pub(super) fn render_shelled_transactional(
    state: &AppState,
    headers: &HeaderMap,
    nonce: Option<&str>,
    status: StatusCode,
    page_title: &str,
    canonical_path: &str,
    card_class: &str,
    inner_html: &str,
) -> Response {
    let lang = web_controller::detect_language(&HashMap::new(), headers);
    let body_html = format!(
        r#"<section class="container section-space"><div class="transactional-card {card_class}">{inner_html}</div></section>"#
    );
    let full_html = web_controller::render_shared_shell(
        state,
        &lang,
        page_title,
        &body_html,
        Some(canonical_path),
        None,
        true,
        nonce,
    );
    (status, Html(full_html)).into_response()
}

pub(super) fn default_date_bounds(params: &HashMap<String, String>) -> (String, String) {
    let now = chrono::Utc::now();
    let default_to = now.format("%Y-%m-%d").to_string();
    let default_from = (now - chrono::Duration::days(365))
        .format("%Y-%m-%d")
        .to_string();
    let from = params
        .get("from")
        .or_else(|| params.get("date_from"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(default_from);
    let to = params
        .get("to")
        .or_else(|| params.get("date_to"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(default_to);
    (from, to)
}

pub(super) fn not_found_error() -> Response {
    ApiResponse::raw(
        StatusCode::NOT_FOUND,
        json!({
            "status": "error",
            "error_code": error_codes::NOT_FOUND,
            "message": "Kayıt bulunamadı.",
            "data": null
        }),
    )
    .into_response()
}

// ============================================================================
// PUBLIC REST API (/api/v1/*)
