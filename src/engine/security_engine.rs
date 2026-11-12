use crate::models::SecurityAuditLog;
use base64::Engine as _;
use chrono::Utc;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::Mutex;

/// Generates a cryptographically random nonce for Content-Security-Policy headers.
/// Uses 16 random bytes from `/dev/urandom`, base64-encoded to produce a 24-character string.
pub fn generate_csp_nonce() -> String {
    let mut buf = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut buf);
    } else {
        // Fallback: use uuid v4 randomness if /dev/urandom is unavailable
        let id = uuid::Uuid::new_v4();
        buf.copy_from_slice(id.as_bytes());
    }
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(buf)
}

static RATE_LIMIT_STORE: OnceCell<Mutex<HashMap<String, (u32, i64)>>> = OnceCell::new();

/// Default rate limit values for backward compatibility with tests.
const DEFAULT_MAX_REQUESTS_PER_MINUTE: u32 = 120;
const DEFAULT_RATE_LIMIT_WINDOW_SECS: i64 = 60;

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
    ///
    /// # Arguments
    /// * `path` - The request path
    /// * `query` - Optional query string
    /// * `user_agent` - Optional User-Agent header
    /// * `client_ip` - Client IP address for rate limiting
    /// * `max_rpm` - Maximum requests per minute (rate limit threshold)
    /// * `window_secs` - Rate limit window duration in seconds
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
            || path == "/favicon.ico";

        if !is_static_asset {
            if let Ok(mut map) = Self::bucket_store().lock() {
                if map.len() > 10_000 {
                    map.retain(|_, (_, ts)| now_ts - *ts < window_secs * 2);
                }
                let entry = map
                    .entry(client_ip.to_string())
                    .or_insert((0, now_ts));
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
                    details: format!("Dizin gezinme (Path Traversal) imzası tespit edildi: '{}'", pat),
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
                    details: format!("SQL Injection vektörü tespit edildi ve engellendi: '{}'", pat),
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
        use std::collections::{HashMap, HashSet};

        let mut builder = ammonia::Builder::default();
        let extra_tags = ["figure", "figcaption", "video", "source", "section", "aside", "time", "mark"];
        for tag in extra_tags {
            builder.add_tags([tag]);
        }

        let generic_attrs: HashSet<&str> = ["class", "lang", "dir", "title"].into_iter().collect();
        builder.add_generic_attributes(generic_attrs);

        let mut tag_attrs: HashMap<&str, HashSet<&str>> = HashMap::new();
        tag_attrs.insert(
            "img",
            ["src", "alt", "width", "height", "loading", "decoding", "fetchpriority"]
                .into_iter()
                .collect(),
        );
        tag_attrs.insert(
            "video",
            ["src", "controls", "poster", "preload", "width", "height", "playsinline", "muted", "loop"]
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
    /// When `nonce` is provided, replaces `'unsafe-inline'` in `script-src` with `'nonce-{value}'`.
    /// The `style-src` directive retains `'unsafe-inline'` temporarily for CSS compatibility.
    pub fn security_headers_list(exec_us: u128, nonce: Option<&str>) -> Vec<(&'static str, String)> {
        let csp = if let Some(n) = nonce {
            format!(
                "default-src 'self' https: data: blob: 'unsafe-inline'; script-src 'self' 'nonce-{}'; style-src 'self' https: 'unsafe-inline'; img-src 'self' https: data: blob:; font-src 'self' https: data:; frame-src 'self' https://www.paytr.com; frame-ancestors 'self';",
                n
            )
        } else {
            "default-src 'self' https: data: blob: 'unsafe-inline'; script-src 'self' 'unsafe-inline'; style-src 'self' https: 'unsafe-inline'; img-src 'self' https: data: blob:; font-src 'self' https: data:; frame-src 'self' https://www.paytr.com; frame-ancestors 'self';".to_string()
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
            // SECURITY FIX: Removed deprecated X-XSS-Protection header (ignored by modern browsers,
            // provides false sense of security). CSP is the correct XSS mitigation mechanism.
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
/// Retained for existing test compatibility.
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
    CentralSecurityFilter::inspect_request_security(path, query, user_agent, client_ip, max_rpm, window_secs)
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
pub fn security_headers_list_with_nonce(exec_us: u128, nonce: Option<&str>) -> Vec<(&'static str, String)> {
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
}
