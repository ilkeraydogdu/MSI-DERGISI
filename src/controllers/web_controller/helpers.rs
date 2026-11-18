use super::*;

pub fn detect_language(params: &HashMap<String, String>, headers: &HeaderMap) -> String {
    if let Some(l) = params.get("lang") {
        if l.eq_ignore_ascii_case("en") {
            return "en".to_string();
        }
        if l.eq_ignore_ascii_case("tr") {
            return "tr".to_string();
        }
    }
    if let Some(cookie) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for part in cookie.split(';') {
            let trimmed = part.trim();
            if trimmed.eq_ignore_ascii_case("msi_lang=en") {
                return "en".to_string();
            }
            if trimmed.eq_ignore_ascii_case("msi_lang=tr") {
                return "tr".to_string();
            }
        }
    }
    "tr".to_string()
}

/// Constructs a typed response with the given content-type and body, eliminating
/// the repeated `(StatusCode::OK, [(header::CONTENT_TYPE, ct)], body).into_response()`
/// boilerplate used by every feed / XML / manifest endpoint.
pub(crate) fn typed_response(content_type: &'static str, body: impl Into<axum::body::Body>) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        body.into(),
    )
        .into_response()
}

pub(crate) fn detect_domain_mode(params: &HashMap<String, String>, headers: &HeaderMap) -> String {
    if let Some(m) = params.get("mode") {
        if m == "magazine" || m == "dergi" {
            return "magazine".into();
        }
        if m == "news" || m == "haber" {
            return "news".to_string();
        }
    }
    if let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) {
        if host.to_lowercase().contains("savunmahaber") {
            return "news".to_string();
        }
    }
    "magazine".to_string()
}

