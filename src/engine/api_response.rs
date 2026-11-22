//! Centralized API Response Builder
//!
//! All API endpoints MUST use this module to construct responses.
//! Format: { "status": "ok"|"error", "data": ..., "message": ..., "meta": ... }
//!
//! - User-facing messages come from the translation engine (multi-language)
//! - Server errors are logged with tracing; generic messages returned to client
//! - No sensitive data (emails, tokens, internal paths) leaks in responses

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

use crate::engine::translation_engine::TranslationCache;

// ---------------------------------------------------------------------------
// Error code constants – single source of truth for all API error codes
// ---------------------------------------------------------------------------

pub mod error_codes {
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const BAD_REQUEST: &str = "BAD_REQUEST";
    pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
    pub const FORBIDDEN: &str = "FORBIDDEN";
    pub const CONFLICT: &str = "CONFLICT";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const PAYMENT_GATEWAY_ERROR: &str = "PAYMENT_GATEWAY_ERROR";
    pub const SERVICE_UNAVAILABLE: &str = "SERVICE_UNAVAILABLE";
    pub const RATE_LIMITED: &str = "RATE_LIMITED";
}

// ---------------------------------------------------------------------------
// Translation keys for all user-facing API messages
// ---------------------------------------------------------------------------

pub mod msg_keys {
    // Content
    pub const CONTENT_NOT_FOUND: &str = "api.error.content_not_found";
    // Auth / Registration
    pub const INVALID_EMAIL: &str = "api.error.invalid_email";
    pub const EMAIL_ALREADY_EXISTS: &str = "api.error.email_already_exists";
    pub const REGISTRATION_FIELDS_REQUIRED: &str = "api.error.registration_fields_required";
    pub const KVKK_CONSENT_REQUIRED: &str = "api.error.kvkk_consent_required";
    pub const LOGIN_FAILED: &str = "api.error.login_failed";
    pub const AUTH_REQUIRED: &str = "api.error.auth_required";
    pub const AUTH_REQUIRED_PDF: &str = "api.error.auth_required_pdf";
    pub const SUBSCRIBER_ACCESS_REQUIRED: &str = "api.error.subscriber_access_required";
    pub const ISSUE_NOT_FOUND: &str = "api.error.issue_not_found";
    // Checkout
    pub const CHECKOUT_FIELDS_REQUIRED: &str = "api.error.checkout_fields_required";
    pub const DISTANCE_SALES_CONSENT_REQUIRED: &str = "api.error.distance_sales_consent_required";
    pub const PLAN_NOT_FOUND: &str = "api.error.plan_not_found";
    // Upload
    pub const INVALID_FILE_TYPE: &str = "api.error.invalid_file_type";
    pub const FILE_TOO_LARGE: &str = "api.error.file_too_large";
    pub const UPLOAD_NO_FIELD: &str = "api.error.upload_no_field";
    // Generic server errors
    pub const SERVER_ERROR_GENERIC: &str = "api.error.server_error_generic";
    pub const SERVER_ERROR_SAVE: &str = "api.error.server_error_save";
    pub const SERVER_ERROR_UPLOAD: &str = "api.error.server_error_upload";
    pub const SERVER_ERROR_FILE_WRITE: &str = "api.error.server_error_file_write";
    pub const SERVER_ERROR_ORDER: &str = "api.error.server_error_order";
    pub const SERVER_ERROR_AD_SAVE: &str = "api.error.server_error_ad_save";
    // Generic validation
    pub const BAD_REQUEST: &str = "api.error.bad_request";
    pub const VALIDATION_ERROR: &str = "api.error.validation_error";
    pub const PAYMENT_ERROR: &str = "api.error.payment_error";
    pub const TOO_MANY_REQUESTS: &str = "api.error.too_many_requests";
    pub const RATE_LIMIT_EXCEEDED: &str = "api.error.too_many_requests";
}

/// Seed API-specific translation keys via centralized repo method.
/// Raw SQL removed — now delegates to Database::seed_api_translations().
pub fn seed_api_translations(db: &crate::db::Database) -> Result<(), String> {
    db.seed_api_translations().map_err(|e| format!("API translation seed hatası: {}", e))
}

// ---------------------------------------------------------------------------
// ApiResponse builder
// ---------------------------------------------------------------------------

/// Unified API response envelope.
///
/// ```text
/// {
///   "status": "ok" | "error",
///   "data": <T>,           // present on success
///   "message": "...",      // human-readable (translated)
///   "meta": { ... }        // optional extra metadata
/// }
/// ```
pub struct ApiResponse {
    status_code: StatusCode,
    body: Value,
}

impl ApiResponse {
    // -- Success builders ---------------------------------------------------

    /// Success response with arbitrary data payload.
    pub fn ok(data: impl Into<Value>) -> Self {
        Self {
            status_code: StatusCode::OK,
            body: json!({
                "status": "ok",
                "data": data.into(),
                "message": null,
                "meta": null
            }),
        }
    }

    /// Success response with data + custom message key (translated).
    pub fn ok_with_message(
        cache: &TranslationCache,
        lang: &str,
        msg_key: &str,
        data: impl Into<Value>,
    ) -> Self {
        Self {
            status_code: StatusCode::OK,
            body: json!({
                "status": "ok",
                "data": data.into(),
                "message": cache.t(lang, msg_key),
                "meta": null
            }),
        }
    }

    /// Success response at a custom status code (e.g. 202 Accepted).
    pub fn ok_with_status(code: StatusCode, data: impl Into<Value>) -> Self {
        Self {
            status_code: code,
            body: json!({
                "status": "ok",
                "data": data.into(),
                "message": null,
                "meta": null
            }),
        }
    }

    /// Success response with meta block.
    pub fn ok_with_meta(data: impl Into<Value>, meta: impl Into<Value>) -> Self {
        Self {
            status_code: StatusCode::OK,
            body: json!({
                "status": "ok",
                "data": data.into(),
                "message": null,
                "meta": meta.into()
            }),
        }
    }

    // -- Error builders -----------------------------------------------------

    /// Client-visible error with translated message. Internal details are NOT included.
    pub fn error(
        cache: &TranslationCache,
        lang: &str,
        code: StatusCode,
        error_code: &str,
        msg_key: &str,
    ) -> Self {
        Self {
            status_code: code,
            body: json!({
                "status": "error",
                "error_code": error_code,
                "message": cache.t(lang, msg_key),
                "data": null,
                "meta": null
            }),
        }
    }

    /// Server error: logs the real error via tracing, returns generic translated message.
    pub fn server_error(
        cache: &TranslationCache,
        lang: &str,
        context: &str,
        internal_err: &dyn std::fmt::Display,
    ) -> Self {
        tracing::error!(context = %context, error = %internal_err, "API server error");
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            body: json!({
                "status": "error",
                "error_code": error_codes::INTERNAL_ERROR,
                "message": cache.t(lang, msg_keys::SERVER_ERROR_GENERIC),
                "data": null,
                "meta": null
            }),
        }
    }

    /// Server error variant that uses a specific message key instead of the generic one.
    pub fn server_error_with_key(
        cache: &TranslationCache,
        lang: &str,
        context: &str,
        internal_err: &dyn std::fmt::Display,
        msg_key: &str,
    ) -> Self {
        tracing::error!(context = %context, error = %internal_err, "API server error");
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            body: json!({
                "status": "error",
                "error_code": error_codes::INTERNAL_ERROR,
                "message": cache.t(lang, msg_key),
                "data": null,
                "meta": null
            }),
        }
    }

    /// Payment gateway error (502 Bad Gateway).
    pub fn payment_gateway_error(
        cache: &TranslationCache,
        lang: &str,
        _gateway_detail: &str,
    ) -> Self {
        // NOTE: _gateway_detail is intentionally NOT exposed to the client.
        Self {
            status_code: StatusCode::BAD_GATEWAY,
            body: json!({
                "status": "error",
                "error_code": error_codes::PAYMENT_GATEWAY_ERROR,
                "message": cache.t(lang, msg_keys::SERVER_ERROR_GENERIC),
                "data": null,
                "meta": null
            }),
        }
    }

    // -- Raw / pass-through (for non-standard endpoints like open-data export) --

    /// Build a raw JSON response at a given status code without the envelope.
    /// Use ONLY for endpoints that have their own format (e.g. file downloads, open-data exports).
    pub fn raw(code: StatusCode, value: impl Into<Value>) -> Self {
        Self {
            status_code: code,
            body: value.into(),
        }
    }

    /// Attach additional headers (returned alongside the response).
    pub fn into_response(self) -> Response {
        (self.status_code, Json(self.body)).into_response()
    }
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        (self.status_code, Json(self.body)).into_response()
    }
}

// ---------------------------------------------------------------------------
// Convenience helpers to reduce boilerplate in controllers
// ---------------------------------------------------------------------------

/// Shorthand: extract `lang` from query params or default to "tr".
pub fn resolve_lang(params: &std::collections::HashMap<String, String>) -> &str {
    params
        .get("lang")
        .map(|s| s.as_str())
        .unwrap_or("tr")
}

/// Parses an RFC 9110 `Accept-Language` header and returns the highest-weight
/// supported platform language (`"en"` or `"tr"`), skipping `q=0` and malformed
/// `q` values and breaking ties by earliest occurrence.
pub fn parse_accept_language(header_val: &str) -> Option<&'static str> {
    let mut best: Option<(f32, usize, &'static str)> = None;

    for (idx, raw_item) in header_val.split(',').enumerate() {
        let mut parts = raw_item.split(';');
        let raw_tag = parts.next().unwrap_or("").trim();
        if raw_tag.is_empty() {
            continue;
        }

        let mut q_val: f32 = 1.0;
        let mut malformed = false;
        for param in parts {
            let trimmed = param.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                if k.trim().eq_ignore_ascii_case("q") {
                    match v.trim().parse::<f32>() {
                        Ok(parsed) if parsed.is_finite() && (0.0..=1.0).contains(&parsed) => {
                            q_val = parsed;
                        }
                        _ => {
                            malformed = true;
                            break;
                        }
                    }
                }
            }
        }
        if malformed || q_val <= 0.0 {
            continue;
        }

        let primary = raw_tag.split(['-', '_']).next().unwrap_or("").trim();
        let supported = if primary.eq_ignore_ascii_case("en") {
            Some("en")
        } else if primary.eq_ignore_ascii_case("tr") {
            Some("tr")
        } else {
            None
        };

        let Some(lang) = supported else {
            continue;
        };

        let should_replace = match best {
            None => true,
            Some((best_q, best_idx, _)) => q_val > best_q || ((q_val - best_q).abs() < f32::EPSILON && idx < best_idx),
        };
        if should_replace {
            best = Some((q_val, idx, lang));
        }
    }

    best.map(|(_, _, lang)| lang)
}

/// Resolve language with precedence:
/// `?lang=` query > `msi_lang=` cookie > `Accept-Language` header > `"tr"`.
pub fn resolve_lang_from_headers(
    params: &std::collections::HashMap<String, String>,
    headers: &axum::http::HeaderMap,
) -> String {
    if let Some(l) = params.get("lang") {
        let trimmed = l.trim();
        if trimmed.eq_ignore_ascii_case("en") {
            return "en".to_string();
        }
        if trimmed.eq_ignore_ascii_case("tr") {
            return "tr".to_string();
        }
    }
    if let Some(cookie) = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
    {
        for part in cookie.split(';') {
            if let Some((k, v)) = part.trim().split_once('=') {
                if k.trim().eq_ignore_ascii_case("msi_lang") {
                    let val = v.trim().trim_matches('"');
                    if val.eq_ignore_ascii_case("en") {
                        return "en".to_string();
                    }
                    if val.eq_ignore_ascii_case("tr") {
                        return "tr".to_string();
                    }
                }
            }
        }
    }
    if let Some(accept_lang) = headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(detected) = parse_accept_language(accept_lang) {
            return detected.to_string();
        }
    }
    "tr".to_string()
}

// ---------------------------------------------------------------------------
// Centralized AppError — single error type for the entire application
// ---------------------------------------------------------------------------

/// Unified application error enum.
///
/// Every error that can occur in request handling is represented here.
/// The `IntoResponse` impl converts each variant into a consistent JSON
/// envelope via `ApiResponse`, logging internal details server-side and
/// returning only translated, generic messages to the client.
pub enum AppError {
    /// 404 — resource not found
    NotFound { msg_key: &'static str },
    /// 400 — validation / bad input
    ValidationError { msg_key: &'static str },
    /// 401 — authentication required
    Unauthorized { msg_key: &'static str },
    /// 403 — authenticated but not authorized
    Forbidden { msg_key: &'static str },
    /// 409 — conflict (e.g. duplicate email)
    Conflict { msg_key: &'static str },
    /// 422 — unprocessable entity (design validation, etc.)
    UnprocessableEntity { message: String },
    /// 500 — internal server error with context for logging
    Internal {
        context: &'static str,
        source: Box<dyn std::fmt::Display + Send + Sync>,
        msg_key: &'static str,
    },
    /// 502 — payment gateway error
    PaymentGateway { detail: String },
}

impl std::fmt::Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { msg_key } => f.debug_struct("NotFound").field("msg_key", msg_key).finish(),
            Self::ValidationError { msg_key } => f.debug_struct("ValidationError").field("msg_key", msg_key).finish(),
            Self::Unauthorized { msg_key } => f.debug_struct("Unauthorized").field("msg_key", msg_key).finish(),
            Self::Forbidden { msg_key } => f.debug_struct("Forbidden").field("msg_key", msg_key).finish(),
            Self::Conflict { msg_key } => f.debug_struct("Conflict").field("msg_key", msg_key).finish(),
            Self::UnprocessableEntity { message } => f.debug_struct("UnprocessableEntity").field("message", message).finish(),
            Self::Internal { context, source, msg_key } => f.debug_struct("Internal")
                .field("context", context)
                .field("source", &source.to_string())
                .field("msg_key", msg_key)
                .finish(),
            Self::PaymentGateway { detail } => f.debug_struct("PaymentGateway").field("detail", detail).finish(),
        }
    }
}

impl AppError {
    /// Shorthand for `Internal` from any displayable error.
    pub fn internal(
        context: &'static str,
        err: impl std::fmt::Display + Send + Sync + 'static,
        msg_key: &'static str,
    ) -> Self {
        Self::Internal {
            context,
            source: Box::new(err),
            msg_key,
        }
    }

    /// Convert this error into an `ApiResponse` using the translation cache.
    pub fn into_api_response(self, cache: &TranslationCache, lang: &str) -> ApiResponse {
        match self {
            Self::NotFound { msg_key } => ApiResponse::error(
                cache,
                lang,
                StatusCode::NOT_FOUND,
                error_codes::NOT_FOUND,
                msg_key,
            ),
            Self::ValidationError { msg_key } => ApiResponse::error(
                cache,
                lang,
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_key,
            ),
            Self::Unauthorized { msg_key } => ApiResponse::error(
                cache,
                lang,
                StatusCode::UNAUTHORIZED,
                error_codes::UNAUTHORIZED,
                msg_key,
            ),
            Self::Forbidden { msg_key } => ApiResponse::error(
                cache,
                lang,
                StatusCode::FORBIDDEN,
                error_codes::FORBIDDEN,
                msg_key,
            ),
            Self::Conflict { msg_key } => ApiResponse::error(
                cache,
                lang,
                StatusCode::CONFLICT,
                error_codes::CONFLICT,
                msg_key,
            ),
            Self::UnprocessableEntity { message } => {
                // Validation errors are client-visible (not sensitive).
                tracing::warn!(validation_error = %message, "Validation failed");
                ApiResponse {
                    status_code: StatusCode::UNPROCESSABLE_ENTITY,
                    body: serde_json::json!({
                        "status": "error",
                        "error_code": error_codes::BAD_REQUEST,
                        "message": message,
                        "data": null,
                        "meta": null
                    }),
                }
            }
            Self::Internal {
                context,
                source,
                msg_key,
            } => ApiResponse::server_error_with_key(cache, lang, context, &*source, msg_key),
            Self::PaymentGateway { detail } => {
                tracing::error!(gateway_detail = %detail, "Payment gateway error");
                ApiResponse::payment_gateway_error(cache, lang, &detail)
            }
        }
    }
}

impl IntoResponse for AppError {
    /// Fallback conversion without translation cache — uses Turkish defaults.
    /// Prefer `into_api_response(cache, lang)` when the cache is available.
    fn into_response(self) -> Response {
        // When used as a bare IntoResponse (no cache in scope), we still
        // produce a valid JSON envelope. The message will be the raw key.
        let (code, error_code, message) = match &self {
            Self::NotFound { msg_key } => (StatusCode::NOT_FOUND, error_codes::NOT_FOUND, *msg_key),
            Self::ValidationError { msg_key } => (StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, *msg_key),
            Self::Unauthorized { msg_key } => (StatusCode::UNAUTHORIZED, error_codes::UNAUTHORIZED, *msg_key),
            Self::Forbidden { msg_key } => (StatusCode::FORBIDDEN, error_codes::FORBIDDEN, *msg_key),
            Self::Conflict { msg_key } => (StatusCode::CONFLICT, error_codes::CONFLICT, *msg_key),
            Self::UnprocessableEntity { message } => {
                tracing::warn!(validation_error = %message, "Validation failed");
                (StatusCode::UNPROCESSABLE_ENTITY, error_codes::BAD_REQUEST, message.as_str())
            }
            Self::Internal { context, source, msg_key } => {
                tracing::error!(context = %context, error = %source, "API server error");
                (StatusCode::INTERNAL_SERVER_ERROR, error_codes::INTERNAL_ERROR, *msg_key)
            }
            Self::PaymentGateway { detail } => {
                tracing::error!(gateway_detail = %detail, "Payment gateway error");
                (StatusCode::BAD_GATEWAY, error_codes::PAYMENT_GATEWAY_ERROR, "Payment gateway error")
            }
        };
        (
            code,
            Json(serde_json::json!({
                "status": "error",
                "error_code": error_code,
                "message": message,
                "data": null,
                "meta": null
            })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header, HeaderMap};
    use std::collections::HashMap;

    #[test]
    fn accept_language_parser_honors_q_values_ties_and_invalid_entries() {
        assert_eq!(parse_accept_language("en-US,en;q=0.9,tr;q=0.8"), Some("en"));
        assert_eq!(parse_accept_language("tr-TR,tr;q=0.9,en-US;q=0.8"), Some("tr"));
        assert_eq!(parse_accept_language("en;q=0,tr;q=0.5"), Some("tr"));
        assert_eq!(parse_accept_language("en;q=0"), None);
        assert_eq!(parse_accept_language("fr-FR,de;q=0.9"), None);
        assert_eq!(parse_accept_language("en;q=invalid,tr;q=0.4"), Some("tr"));
        assert_eq!(parse_accept_language("en;q=1.5,tr;q=0.4"), Some("tr"));
        assert_eq!(parse_accept_language("en;q=-0.2,tr;q=0.4"), Some("tr"));
        assert_eq!(parse_accept_language("en;q=0.8,tr;q=0.8"), Some("en"));
        assert_eq!(parse_accept_language("tr;q=0.8,en;q=0.8"), Some("tr"));
        assert_eq!(parse_accept_language("fr-CH,de;q=0.9,en-GB;q=0.7"), Some("en"));
    }

    #[test]
    fn resolve_lang_from_headers_follows_precedence() {
        let mut params = HashMap::new();
        let mut headers = HeaderMap::new();
        assert_eq!(resolve_lang_from_headers(&params, &headers), "tr");

        headers.insert(header::ACCEPT_LANGUAGE, "en-GB,en;q=0.8".parse().unwrap());
        assert_eq!(resolve_lang_from_headers(&params, &headers), "en");

        headers.insert(header::COOKIE, "msi_lang=tr".parse().unwrap());
        assert_eq!(resolve_lang_from_headers(&params, &headers), "tr");

        params.insert("lang".to_string(), "en".to_string());
        assert_eq!(resolve_lang_from_headers(&params, &headers), "en");
    }
}