use crate::models::SecurityAuditLog;
use crate::utils::crypto::constant_time_eq;
use base64::Engine as _;
use chrono::Utc;
use hmac::{Hmac, Mac};
use once_cell::sync::OnceCell;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Mutex;

type HmacSha256 = Hmac<Sha256>;

/// Generates a cryptographically random nonce for Content-Security-Policy headers.
/// Uses 16 random bytes from `/dev/urandom`, base64-encoded to produce a 24-character string.
pub fn generate_csp_nonce() -> String {
    let mut buf = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut buf);
    } else {
        let id = uuid::Uuid::new_v4();
        buf.copy_from_slice(id.as_bytes());
    }
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(buf)
}

static RATE_LIMIT_STORE: OnceCell<Mutex<HashMap<String, (u32, i64)>>> = OnceCell::new();

/// Default rate limit values for backward compatibility with tests.
const DEFAULT_MAX_REQUESTS_PER_MINUTE: u32 =
    crate::db::constants::RATE_LIMIT_MAX_REQUESTS_PER_MINUTE;
const DEFAULT_RATE_LIMIT_WINDOW_SECS: i64 = crate::db::constants::RATE_LIMIT_WINDOW_SECONDS;

/// Per-request CSP nonce stored in Axum request extensions.
/// Controllers extract this to inject `nonce="..."` on inline `<script>` tags.
#[derive(Clone)]
pub struct CspNonce(pub String);

pub struct CentralSecurityFilter;

impl CentralSecurityFilter {
    fn bucket_store() -> &'static Mutex<HashMap<String, (u32, i64)>> {
        RATE_LIMIT_STORE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// Inspects incoming HTTP request parameters for rate limiting, SQL injection, XSS, and Path Traversal.
    pub fn inspect_request_security(
        path: &str,
        query: Option<&str>,
        user_agent: Option<&str>,
        client_ip: &str,
        max_rpm: u32,
        window_secs: i64,
    ) -> Result<(), SecurityAuditLog> {
        let now = Utc::now();
        let now_ts = now.timestamp();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        // 1. Token-Bucket / Fixed-Window Rate Limiter per IP (applied to dynamic routes & APIs;
        // static asset sub-requests still undergo full Path Traversal / SQLi / XSS inspection below).
        let is_static_asset = path.starts_with("/images/")
            || path.starts_with("/css/")
            || path.starts_with("/js/")
            || path.starts_with("/fonts/")
            || path.starts_with("/uploads/")
            || path == "/favicon.ico";

        if !is_static_asset {
            if let Ok(mut map) = Self::bucket_store().lock() {
                if map.len() > crate::db::constants::RATE_LIMIT_STORE_MAX_ENTRIES {
                    map.retain(|_, (_, ts)| now_ts - *ts < window_secs * 2);
                }
                let entry = map.entry(client_ip.to_string()).or_insert((0, now_ts));
                if now_ts - entry.1 >= window_secs {
                    entry.0 = 1;
                    entry.1 = now_ts;
                } else {
                    entry.0 += 1;
                }

                if entry.0 > max_rpm {
                    return Err(SecurityAuditLog {
                        id: 0,
                        client_ip: client_ip.to_string(),
                        request_path: path.to_string(),
                        event_type: "RATE_LIMIT_EXCEEDED".to_string(),
                        threat_level: "HIGH".to_string(),
                        details: format!(
                            "Rate limit aşıldı: {} istek/dk (Limit: {})",
                            entry.0, max_rpm
                        ),
                        blocked: true,
                        created_at: now_str,
                    });
                }
            }
        }

        // Prepare inspection payload (both raw and URL-decoded, lowercased)
        let raw_query = query.unwrap_or("");
        let raw_ua = user_agent.unwrap_or("");
        let decoded_path = urlencoding::decode(path)
            .map(|c| c.into_owned())
            .unwrap_or_else(|_| path.to_string());
        let decoded_query = urlencoding::decode(raw_query)
            .map(|c| c.into_owned())
            .unwrap_or_else(|_| raw_query.to_string());

        let combined = format!(
            "{} {} {} {} {}",
            path, decoded_path, raw_query, decoded_query, raw_ua
        )
        .to_lowercase();

        // 2. Path Traversal Detection
        let traversal_patterns = [
            "../",
            "..\\",
            "/etc/passwd",
            "/etc/shadow",
            "/proc/self",
            "%2e%2e%2f",
            "%2e%2e/",
            "..%2f",
        ];
        for pat in traversal_patterns {
            if combined.contains(pat) {
                return Err(SecurityAuditLog {
                    id: 0,
                    client_ip: client_ip.to_string(),
                    request_path: path.to_string(),
                    event_type: "PATH_TRAVERSAL_ATTEMPT".to_string(),
                    threat_level: "CRITICAL".to_string(),
                    details: format!(
                        "Dizin gezinme (Path Traversal) imzası tespit edildi: '{}'",
                        pat
                    ),
                    blocked: true,
                    created_at: now_str,
                });
            }
        }

        // 3. SQL Injection Pattern Detection
        let sqli_patterns = [
            "union select",
            "union all select",
            "' or '1'='1",
            "\" or \"1\"=\"1",
            "' or 1=1",
            "drop table",
            "truncate table",
            ";--",
            "benchmark(",
            "sleep(",
            "waitfor delay",
            "information_schema",
            "sqlite_master",
        ];
        for pat in sqli_patterns {
            if combined.contains(pat) {
                return Err(SecurityAuditLog {
                    id: 0,
                    client_ip: client_ip.to_string(),
                    request_path: path.to_string(),
                    event_type: "SQL_INJECTION_ATTEMPT".to_string(),
                    threat_level: "CRITICAL".to_string(),
                    details: format!(
                        "SQL Injection vektörü tespit edildi ve engellendi: '{}'",
                        pat
                    ),
                    blocked: true,
                    created_at: now_str,
                });
            }
        }

        // 4. Cross-Site Scripting (XSS) Vector Detection
        let xss_patterns = [
            "<script",
            "</script",
            "javascript:",
            "vbscript:",
            "onerror=",
            "onload=",
            "onmouseover=",
            "onfocus=",
            "document.cookie",
            "<iframe",
            "<svg/onload",
        ];
        for pat in xss_patterns {
            if combined.contains(pat) {
                return Err(SecurityAuditLog {
                    id: 0,
                    client_ip: client_ip.to_string(),
                    request_path: path.to_string(),
                    event_type: "XSS_VECTOR_DETECTED".to_string(),
                    threat_level: "HIGH".to_string(),
                    details: format!("XSS (Cross-Site Scripting) vektörü engellendi: '{}'", pat),
                    blocked: true,
                    created_at: now_str,
                });
            }
        }

        Ok(())
    }

    /// Escapes untrusted strings into 100% XSS-safe HTML entities.
    pub fn escape_html(input: &str) -> String {
        let mut out = String::with_capacity(input.len() + 16);
        for ch in input.chars() {
            match ch {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#x27;"),
                _ => out.push(ch),
            }
        }
        out
    }

    /// Sanitizes an arbitrary string or URL slug into a clean, canonical kebab-case identifier.
    pub fn sanitize_slug(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut prev_dash = false;

        for ch in input.chars() {
            let mapped = match ch {
                'ş' | 'Ş' => Some('s'),
                'ı' | 'I' | 'İ' => Some('i'),
                'ğ' | 'Ğ' => Some('g'),
                'ü' | 'Ü' => Some('u'),
                'ö' | 'Ö' => Some('o'),
                'ç' | 'Ç' => Some('c'),
                'a'..='z' | '0'..='9' => Some(ch),
                'A'..='Z' => Some(ch.to_ascii_lowercase()),
                '-' | '_' | ' ' | '/' | '.' => None,
                _ => None,
            };

            if let Some(c) = mapped {
                out.push(c);
                prev_dash = false;
            } else if !prev_dash && !out.is_empty() {
                out.push('-');
                prev_dash = true;
            }
        }

        out.trim_matches('-').to_string()
    }

    /// Sanitizes rich editorial HTML using an allowlist (`ammonia`) to block stored XSS vectors
    /// (`<script>`, inline `on*` handlers, `javascript:` URIs, `<iframe` injections) while keeping
    /// legitimate article formatting, figures, tables, links, and media intact.
    pub fn sanitize_rich_html(raw: &str) -> String {
        use std::collections::HashSet;

        let mut builder = ammonia::Builder::default();
        let extra_tags = [
            "figure",
            "figcaption",
            "video",
            "source",
            "section",
            "aside",
            "time",
            "mark",
        ];
        for tag in extra_tags {
            builder.add_tags([tag]);
        }

        let generic_attrs: HashSet<&str> = [
            "class",
            "lang",
            "dir",
            "title",
            "data-gallery-title",
            "data-caption",
            "data-article-id",
            "data-slug",
            "data-img",
            "data-callout-kind",
            "data-live-time",
            "data-author",
        ]
        .into_iter()
        .collect();
        builder.add_generic_attributes(generic_attrs);

        let mut tag_attrs: HashMap<&str, HashSet<&str>> = HashMap::new();
        tag_attrs.insert(
            "img",
            [
                "src",
                "alt",
                "width",
                "height",
                "loading",
                "decoding",
                "fetchpriority",
            ]
            .into_iter()
            .collect(),
        );
        tag_attrs.insert(
            "video",
            [
                "src",
                "controls",
                "poster",
                "preload",
                "width",
                "height",
                "playsinline",
                "muted",
                "loop",
            ]
            .into_iter()
            .collect(),
        );
        tag_attrs.insert("source", ["src", "type"].into_iter().collect());
        tag_attrs.insert("time", ["datetime"].into_iter().collect());
        for (tag, attrs) in tag_attrs {
            builder.add_tag_attributes(tag, attrs);
        }

        builder.url_relative(ammonia::UrlRelative::PassThrough);
        builder.clean(raw).to_string()
    }

    /// Returns strict HTTP security headers and engine execution latency telemetry.
    /// Hardened CSP: removes `'unsafe-inline'` from `default-src` and enforces
    /// `object-src 'none'; base-uri 'self'; form-action 'self';`.
    pub fn security_headers_list(exec_us: u128, nonce: Option<&str>) -> Vec<(&'static str, String)> {
        let csp = if let Some(n) = nonce {
            format!(
                "default-src 'self' https: data: blob:; script-src 'self' 'nonce-{}' 'wasm-unsafe-eval'; style-src 'self' https: 'unsafe-inline'; img-src 'self' https: data: blob:; font-src 'self' https: data:; frame-src 'self' https://www.paytr.com; frame-ancestors 'self'; object-src 'none'; base-uri 'self'; form-action 'self';",
                n
            )
        } else {
            "default-src 'self' https: data: blob:; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' https: 'unsafe-inline'; img-src 'self' https: data: blob:; font-src 'self' https: data:; frame-src 'self' https://www.paytr.com; frame-ancestors 'self'; object-src 'none'; base-uri 'self'; form-action 'self';".to_string()
        };
        vec![
            ("X-Content-Type-Options", "nosniff".to_string()),
            ("X-Frame-Options", "SAMEORIGIN".to_string()),
            (
                "Strict-Transport-Security",
                "max-age=63072000; includeSubDomains; preload".to_string(),
            ),
            (
                "Referrer-Policy",
                "strict-origin-when-cross-origin".to_string(),
            ),
            (
                "Permissions-Policy",
                "geolocation=(), microphone=(), camera=()".to_string(),
            ),
            ("Content-Security-Policy", csp),
            ("X-Engine-Latency-Us", exec_us.to_string()),
        ]
    }
}

/// Backward-compatible wrapper using default rate limit values (120 RPM, 60s window).
#[allow(dead_code)]
pub fn inspect_request_security(
    path: &str,
    query: Option<&str>,
    user_agent: Option<&str>,
    client_ip: &str,
) -> Result<(), SecurityAuditLog> {
    CentralSecurityFilter::inspect_request_security(
        path,
        query,
        user_agent,
        client_ip,
        DEFAULT_MAX_REQUESTS_PER_MINUTE,
        DEFAULT_RATE_LIMIT_WINDOW_SECS,
    )
}

/// Parameterized inspection with configurable rate limits. Use this in production middleware.
pub fn inspect_request_security_with_limits(
    path: &str,
    query: Option<&str>,
    user_agent: Option<&str>,
    client_ip: &str,
    max_rpm: u32,
    window_secs: i64,
) -> Result<(), SecurityAuditLog> {
    CentralSecurityFilter::inspect_request_security(
        path,
        query,
        user_agent,
        client_ip,
        max_rpm,
        window_secs,
    )
}

// =========================================================================
// CSRF Double-Submit Cookie Protection
// =========================================================================

/// Generates a signed CSRF token: `{timestamp_secs}.{hmac_hex}`.
pub fn generate_csrf_token(secret: &str) -> String {
    let timestamp = Utc::now().timestamp();
    let payload = timestamp.to_string();
    let sig = sign_csrf_payload(&payload, secret);
    format!("{}.{}", payload, sig)
}

/// Validates a CSRF token against the server secret.
/// Returns `true` if the signature matches in constant time and the token is within `[-60s, +3600s]`.
pub fn validate_csrf_token(token: &str, secret: &str) -> bool {
    let parts: Vec<&str> = token.trim().splitn(2, '.').collect();
    if parts.len() != 2 {
        return false;
    }
    let Ok(timestamp) = parts[0].parse::<i64>() else {
        return false;
    };
    let age = Utc::now()
        .timestamp()
        .checked_sub(timestamp)
        .unwrap_or(i64::MAX);
    if !(-60..=3600).contains(&age) {
        return false;
    }
    let expected_sig = sign_csrf_payload(parts[0], secret);
    constant_time_eq(parts[1].as_bytes(), expected_sig.as_bytes())
}

fn sign_csrf_payload(payload: &str, secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts arbitrary key lengths");
    mac.update(b"MSI-CSRF-V1:");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// =========================================================================
// Login Brute-Force Protection (Case-Normalized)
// =========================================================================

// =========================================================================
// FORM SPAM PROTECTION (Honeypot + Timestamp + Per-Form Rate Limiting)
// =========================================================================

static FORM_RATE_LIMIT_STORE: OnceCell<Mutex<HashMap<String, (u32, i64)>>> = OnceCell::new();

/// Per-form-type rate limits (max requests per window).
pub const FORM_RATE_REGISTER_MAX: u32 = 3;
pub const FORM_RATE_REGISTER_WINDOW: i64 = 60; // 3 per minute
pub const FORM_RATE_FORGOT_PW_MAX: u32 = 5;
pub const FORM_RATE_FORGOT_PW_WINDOW: i64 = 3600; // 5 per hour
pub const FORM_RATE_CONTACT_MAX: u32 = 2;
pub const FORM_RATE_CONTACT_WINDOW: i64 = 60; // 2 per minute
pub const FORM_RATE_ANALYTICS_MAX: u32 = 30;
pub const FORM_RATE_ANALYTICS_WINDOW: i64 = 60; // 30 per minute

/// Minimum time (seconds) between form render and submit to detect bots.
pub const FORM_MIN_SUBMIT_DELAY_SECS: i64 = 2;
/// Maximum time (seconds) before a form submission is considered expired.
pub const FORM_MAX_SUBMIT_AGE_SECS: i64 = 1800; // 30 minutes

/// Result of form spam check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormSpamResult {
    /// Submission passed all checks.
    Passed,
    /// Honeypot field was filled — silent bot detection.
    HoneypotTriggered,
    /// Submission was too fast (< min delay) or too old (> max age).
    TimingViolation(String),
    /// Per-form rate limit exceeded.
    RateLimited { remaining_secs: i64 },
}

/// Checks the honeypot field value. Returns true if it appears to be a bot.
/// The honeypot field should be hidden from real users via CSS (`display:none`).
pub fn check_honeypot(value: Option<&str>) -> bool {
    value.map(|v| !v.trim().is_empty()).unwrap_or(false)
}

/// Deserializes an optional Unix timestamp (`_form_ts`) accepting integer, numeric string,
/// empty string (`None`), or `null` (`None`) so both JSON API clients and `FormData` payloads work.
pub fn deserialize_opt_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match opt {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Number(n)) => n.as_i64().filter(|v| *v > 0),
        Some(serde_json::Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<i64>().ok().filter(|v| *v > 0)
            }
        }
        _ => None,
    })
}

/// Validates the form timing anti-spam token.
/// The token is a Unix timestamp (seconds) set when the form was rendered.
/// When omitted (`None`) by programmatic JSON API clients, passes cleanly while still
/// enforcing honeypot and per-IP rate limits.
pub fn check_form_timing(rendered_ts: Option<i64>) -> FormSpamResult {
    let Some(ts) = rendered_ts.filter(|v| *v > 0) else {
        return FormSpamResult::Passed;
    };
    let now = Utc::now().timestamp();
    if ts > now + 5 {
        return FormSpamResult::TimingViolation("Form çok hızlı gönderildi.".to_string());
    }
    let elapsed = now.saturating_sub(ts);
    if elapsed < FORM_MIN_SUBMIT_DELAY_SECS {
        return FormSpamResult::TimingViolation(
            "Form çok hızlı gönderildi.".to_string(),
        );
    }
    if elapsed > FORM_MAX_SUBMIT_AGE_SECS {
        return FormSpamResult::TimingViolation(
            "Form süresi dolmuş. Lütfen sayfayı yenileyin.".to_string(),
        );
    }
    FormSpamResult::Passed
}

/// Client IP resolved by `waf_telemetry` with peer awareness.
#[derive(Clone, Debug)]
pub struct ResolvedClientIp {
    pub ip: String,
    pub was_forwarded: bool,
}

/// Checks per-form-type rate limiting for a given client IP.
pub fn check_form_rate_limit(form_type: &str, client_ip: &str) -> FormSpamResult {
    check_form_rate_limit_with_forwarded(form_type, client_ip, false)
}

/// Checks per-form-type rate limiting for a given client IP, never exempting forwarded loopback IPs.
pub fn check_form_rate_limit_with_forwarded(
    form_type: &str,
    client_ip: &str,
    was_forwarded: bool,
) -> FormSpamResult {
    if client_ip.is_empty()
        || client_ip == "unknown"
        || (!was_forwarded && (client_ip == "127.0.0.1" || client_ip == "::1"))
    {
        return FormSpamResult::Passed;
    }
    let (max_requests, window_secs) = match form_type {
        "login" => (20, 60),
        "register" => (FORM_RATE_REGISTER_MAX, FORM_RATE_REGISTER_WINDOW),
        "forgot_password" => (FORM_RATE_FORGOT_PW_MAX, FORM_RATE_FORGOT_PW_WINDOW),
        "contact" => (FORM_RATE_CONTACT_MAX, FORM_RATE_CONTACT_WINDOW),
        "analytics" => (FORM_RATE_ANALYTICS_MAX, FORM_RATE_ANALYTICS_WINDOW),
        _ => (FORM_RATE_CONTACT_MAX, FORM_RATE_CONTACT_WINDOW),
    };

    let store = FORM_RATE_LIMIT_STORE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = store.lock() else {
        return FormSpamResult::Passed;
    };

    let now = Utc::now().timestamp();
    let key = format!("{}:{}", form_type, client_ip);

    // Prune stale entries when store grows too large
    if map.len() > 5_000 {
        map.retain(|_, (_, ts)| now - *ts < window_secs * 2);
    }

    let entry = map.entry(key).or_insert((0, now));
    if now - entry.1 >= window_secs {
        entry.0 = 1;
        entry.1 = now;
    } else {
        entry.0 += 1;
    }

    if entry.0 > max_requests {
        let remaining = window_secs - (now - entry.1);
        FormSpamResult::RateLimited {
            remaining_secs: remaining.max(1),
        }
    } else {
        FormSpamResult::Passed
    }
}

/// Generates a form timing token (current Unix timestamp as string).
/// Embed this as a hidden field value when rendering forms.
pub fn generate_form_timing_token() -> String {
    Utc::now().timestamp().to_string()
}

/// Comprehensive form spam check combining honeypot, timing, and rate limiting.
/// Returns `Passed` only if all three checks succeed.
pub fn validate_form_submission(
    form_type: &str,
    client_ip: &str,
    honeypot_value: Option<&str>,
    rendered_timestamp: Option<i64>,
) -> FormSpamResult {
    validate_form_submission_with_forwarded(
        form_type,
        client_ip,
        false,
        honeypot_value,
        rendered_timestamp,
    )
}

/// Comprehensive form spam check with forwarded-IP awareness so spoofed `X-Forwarded-For: 127.0.0.1`
/// cannot bypass per-form rate limits.
pub fn validate_form_submission_with_forwarded(
    form_type: &str,
    client_ip: &str,
    was_forwarded: bool,
    honeypot_value: Option<&str>,
    rendered_timestamp: Option<i64>,
) -> FormSpamResult {
    // 1. Honeypot check (silent — don't reveal to attacker)
    if check_honeypot(honeypot_value) {
        return FormSpamResult::HoneypotTriggered;
    }
    // 2. Timing check
    let timing = check_form_timing(rendered_timestamp);
    if timing != FormSpamResult::Passed {
        return timing;
    }
    // 3. Rate limit check
    check_form_rate_limit_with_forwarded(form_type, client_ip, was_forwarded)
}

static LOGIN_ATTEMPT_STORE: OnceCell<Mutex<HashMap<String, (u32, i64)>>> = OnceCell::new();

/// Maximum failed login attempts before temporary lockout (15 minutes).
const LOGIN_LOCKOUT_THRESHOLD: u32 = 5;
const LOGIN_LOCKOUT_DURATION_SECS: i64 = 900;
/// Extended lockout after 10+ failures (1 hour).
const LOGIN_EXTENDED_LOCKOUT_THRESHOLD: u32 = 10;
const LOGIN_EXTENDED_LOCKOUT_DURATION_SECS: i64 = 3600;

/// Result of a login attempt check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginCheckResult {
    /// Login is allowed to proceed.
    Allowed,
    /// Account is temporarily locked. Contains remaining lockout seconds and failed attempt count.
    Locked { remaining_secs: i64, attempts: u32 },
}

fn normalize_login_key(key: &str) -> String {
    key.trim().to_lowercase()
}

/// Checks whether a login attempt should be allowed based on recent failure history.
/// Always normalizes `key.trim().to_lowercase()` so mixed-case emails cannot bypass lockout.
pub fn check_login_allowed(key: &str) -> LoginCheckResult {
    let normalized = normalize_login_key(key);
    let store = LOGIN_ATTEMPT_STORE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(map) = store.lock() else {
        return LoginCheckResult::Allowed;
    };

    if let Some((attempts, last_attempt_ts)) = map.get(&normalized) {
        let now = Utc::now().timestamp();
        let elapsed = now - last_attempt_ts;

        let lockout_duration = if *attempts >= LOGIN_EXTENDED_LOCKOUT_THRESHOLD {
            LOGIN_EXTENDED_LOCKOUT_DURATION_SECS
        } else if *attempts >= LOGIN_LOCKOUT_THRESHOLD {
            LOGIN_LOCKOUT_DURATION_SECS
        } else {
            return LoginCheckResult::Allowed;
        };

        if elapsed < lockout_duration {
            return LoginCheckResult::Locked {
                remaining_secs: lockout_duration - elapsed,
                attempts: *attempts,
            };
        }
    }

    LoginCheckResult::Allowed
}

/// Records a failed login attempt under `key.trim().to_lowercase()`.
pub fn record_failed_login(key: &str) {
    let normalized = normalize_login_key(key);
    if normalized.is_empty() {
        return;
    }
    let store = LOGIN_ATTEMPT_STORE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut map) = store.lock() {
        let now = Utc::now().timestamp();
        if map.len() > 10_000 {
            map.retain(|_, (_, ts)| now - *ts < LOGIN_EXTENDED_LOCKOUT_DURATION_SECS * 2);
        }
        let entry = map.entry(normalized).or_insert((0, 0));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = now;
    }
}

/// Resets the failed login counter for `key.trim().to_lowercase()` after successful authentication.
pub fn reset_login_attempts(key: &str) {
    let normalized = normalize_login_key(key);
    let store = LOGIN_ATTEMPT_STORE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut map) = store.lock() {
        map.remove(&normalized);
    }
}

pub fn escape_html(input: &str) -> String {
    CentralSecurityFilter::escape_html(input)
}

pub fn sanitize_rich_html(raw: &str) -> String {
    CentralSecurityFilter::sanitize_rich_html(raw)
}

#[allow(dead_code)]
pub fn sanitize_slug(input: &str) -> String {
    CentralSecurityFilter::sanitize_slug(input)
}

#[allow(dead_code)]
pub fn security_headers_list(exec_us: u128) -> Vec<(&'static str, String)> {
    CentralSecurityFilter::security_headers_list(exec_us, None)
}

/// Nonce-aware wrapper for production use.
pub fn security_headers_list_with_nonce(
    exec_us: u128,
    nonce: Option<&str>,
) -> Vec<(&'static str, String)> {
    CentralSecurityFilter::security_headers_list(exec_us, nonce)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_xss_while_preserving_editorial_markup() {
        let dirty = r#"<p class="paywall-excerpt"><strong>KAAN</strong> <img src="images/msi/kaan.webp" onerror="alert(1)" loading="lazy"><script>alert(2)</script><a href="javascript:alert(3)">Link</a></p>"#;
        let clean = sanitize_rich_html(dirty);
        assert!(clean.contains("<p class=\"paywall-excerpt\">"));
        assert!(clean.contains("<strong>KAAN</strong>"));
        assert!(clean.contains("src=\"images/msi/kaan.webp\""));
        assert!(!clean.contains("onerror"));
        assert!(!clean.contains("<script"));
        assert!(!clean.contains("javascript:"));
    }

    #[test]
    fn sanitize_rich_html_preserves_editorial_blocks_and_blocks_xss() {
        let dirty = r#"<section class="hf-gallery" data-gallery-title="Savunma"><figure class="hf-figure" data-caption="KAAN"><img src="/uploads/editorial/a.webp" alt="KAAN" onerror="alert(1)"></figure></section><aside class="hf-related-card" data-article-id="12" data-slug="kaan"><a href="javascript:alert(9)" onclick="steal()">Link</a><script>evil()</script></aside>"#;
        let clean = sanitize_rich_html(dirty);
        assert!(clean.contains("class=\"hf-gallery\""));
        assert!(clean.contains("data-gallery-title=\"Savunma\""));
        assert!(clean.contains("data-caption=\"KAAN\""));
        assert!(clean.contains("data-article-id=\"12\""));
        assert!(clean.contains("data-slug=\"kaan\""));
        assert!(!clean.contains("onerror"));
        assert!(!clean.contains("onclick"));
        assert!(!clean.contains("<script"));
        assert!(!clean.contains("javascript:"));
    }

    #[test]
    fn hardened_csp_contains_object_src_base_uri_and_form_action() {
        let headers = security_headers_list_with_nonce(120, Some("testNonce123"));
        let csp = headers
            .iter()
            .find(|(k, _)| *k == "Content-Security-Policy")
            .map(|(_, v)| v.as_str())
            .expect("CSP header present");
        assert!(csp.contains("object-src 'none'"));
        assert!(csp.contains("base-uri 'self'"));
        assert!(csp.contains("form-action 'self'"));
        assert!(csp.contains("'nonce-testNonce123'"));
        assert!(!csp.starts_with("default-src 'self' https: data: blob: 'unsafe-inline'"));
    }

    #[test]
    fn csrf_token_round_trip_and_tamper_rejection() {
        let secret = "csrf-unit-test-secret";
        let tok = generate_csrf_token(secret);
        assert!(validate_csrf_token(&tok, secret));
        assert!(!validate_csrf_token(&tok, "wrong-secret"));
        assert!(!validate_csrf_token("1700000000.deadbeef", secret));
    }

    #[test]
    fn login_lockout_normalizes_case_and_whitespace() {
        let key_mixed = "  Operator.LockoutTest@MSIDergisi.com ";
        let key_upper = "OPERATOR.LOCKOUTTEST@MSIDERGISI.COM";
        reset_login_attempts(key_mixed);
        assert_eq!(check_login_allowed(key_upper), LoginCheckResult::Allowed);

        for _ in 0..5 {
            record_failed_login(key_mixed);
        }

        match check_login_allowed(key_upper) {
            LoginCheckResult::Locked { remaining_secs, attempts } => {
                assert!(remaining_secs > 0);
                assert_eq!(attempts, 5);
            }
            LoginCheckResult::Allowed => panic!("expected account to be locked regardless of email case"),
        }

        reset_login_attempts("operator.lockouttest@msidergisi.com");
        assert_eq!(check_login_allowed(key_mixed), LoginCheckResult::Allowed);
    }
}
