use super::{CentralSeoEngine};

impl CentralSeoEngine {
    pub fn normalize_base_url(base_url: &str) -> String {
        let trimmed = base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            // Fallback only when no base_url is provided at all; callers should
            // always pass state.config.public_url. Uses PRIMARY_DOMAIN env var
            // as single source of truth instead of hardcoded domain.
            let domain = std::env::var("PRIMARY_DOMAIN")
                .unwrap_or_else(|_| "msidergisi.com".to_string());
            format!("https://{}", domain)
        } else {
            trimmed.to_string()
        }
    }

    pub fn resolve_asset_url(base_url: &str, asset_path: &str, default_image: &str) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let trimmed = asset_path.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else if trimmed.is_empty() {
            format!("{}{}", clean_base, default_image)
        } else if trimmed.starts_with('/') {
            format!("{}{}", clean_base, trimmed)
        } else {
            format!("{}/{}", clean_base, trimmed)
        }
    }

    /// Converts `"YYYY-MM-DD HH:MM:SS"` or `"YYYY-MM-DD"` into timezone-aware ISO 8601 (`+03:00`)
    /// required by Google News, Yandex News, and Schema.org `NewsArticle`.
    pub fn to_iso8601_tr(raw: &str) -> String {
        let t = raw.trim();
        if t.contains('T') && (t.contains('+') || t.ends_with('Z')) {
            return t.to_string();
        }
        if t.len() >= 19 && t.as_bytes().get(10) == Some(&b' ') {
            return format!("{}T{}+03:00", &t[..10], &t[11..19]);
        }
        if t.len() >= 16 && t.as_bytes().get(10) == Some(&b' ') {
            return format!("{}T{}:00+03:00", &t[..10], &t[11..16]);
        }
        if t.len() >= 10 {
            return format!("{}T09:00:00+03:00", &t[..10]);
        }
        // SEO FIX (M1): Removed silent hardcoded fallback that produced wrong metadata for
        // unparseable dates. Log a warning so invalid dates are caught during development.
        tracing::warn!(
            raw_date = %raw,
            "to_iso8601_tr: could not parse date, returning empty string"
        );
        String::new()
    }

    /// Converts `"YYYY-MM-DD HH:MM:SS"` into RFC-2822 format for RSS 2.0 `<pubDate>`.
    pub(super) fn to_rfc2822_tr(raw: &str) -> String {
        let iso = Self::to_iso8601_tr(raw);
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&iso) {
            return dt.to_rfc2822();
        }
        iso
    }

    pub(super) fn xml_escape(input: &str) -> String {
        let mut out = String::with_capacity(input.len() + 16);
        for ch in input.chars() {
            match ch {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&apos;"),
                _ => out.push(ch),
            }
        }
        out
    }

    pub(super) fn strip_html_plain(html: &str) -> String {
        let mut out = String::with_capacity(html.len());
        let mut in_tag = false;
        for ch in html.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => {
                    in_tag = false;
                    out.push(' ');
                }
                _ if !in_tag => out.push(ch),
                _ => {}
            }
        }
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}
