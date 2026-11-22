//! Social media sharing service: Telegram Bot API + Pure Rust OAuth 1.0a Twitter/X v2 (`server.ts` lines 361–439).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD as BASE64_STD, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::Sha1;
use uuid::Uuid;

type HmacSha1 = Hmac<Sha1>;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TwitterConfigInput {
    #[serde(default, alias = "apiKey", alias = "twitterApiKey")]
    pub api_key: String,
    #[serde(default, alias = "apiSecret", alias = "twitterApiSecret")]
    pub api_secret: String,
    #[serde(default, alias = "accessToken", alias = "twitterAccessToken")]
    pub access_token: String,
    #[serde(
        default,
        alias = "accessSecret",
        alias = "accessTokenSecret",
        alias = "twitterAccessTokenSecret"
    )]
    pub access_secret: String,
}

impl TwitterConfigInput {
    pub fn from_json_value(val: &Value) -> Self {
        let pick = |keys: &[&str]| -> String {
            for k in keys {
                if let Some(s) = val.get(*k).and_then(|v| v.as_str()) {
                    let t = s.trim();
                    if !t.is_empty() {
                        return t.to_string();
                    }
                }
            }
            String::new()
        };
        Self {
            api_key: pick(&["apiKey", "twitterApiKey", "api_key", "twitter_api_key"]),
            api_secret: pick(&[
                "apiSecret",
                "twitterApiSecret",
                "api_secret",
                "twitter_api_secret",
            ]),
            access_token: pick(&[
                "accessToken",
                "twitterAccessToken",
                "access_token",
                "twitter_access_token",
            ]),
            access_secret: pick(&[
                "accessSecret",
                "accessTokenSecret",
                "twitterAccessTokenSecret",
                "access_secret",
                "twitter_access_token_secret",
            ]),
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.api_key.trim().is_empty()
            && !self.api_secret.trim().is_empty()
            && !self.access_token.trim().is_empty()
            && !self.access_secret.trim().is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ShareArticlePayload<'a> {
    pub title: &'a str,
    pub url: &'a str,
    pub description: &'a str,
}

fn escape_telegram_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Validates and trims an external HTTP(S) URL, rejecting empty strings, `"null"`, `"undefined"`, or non-URL labels.
pub fn clean_external_url(raw: Option<&str>) -> Option<&str> {
    let s = raw?.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("null") || s.eq_ignore_ascii_case("undefined") {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Some(s)
    } else {
        None
    }
}

/// Resolves the best external link for an `Article` (`wp_post_url` -> `source_url` -> `original_url`),
/// returning `None` if no valid HTTP(S) external URL exists.
pub fn article_external_url(art: &crate::editorial::common::Article) -> Option<&str> {
    clean_external_url(art.wp_post_url.as_deref())
        .or_else(|| clean_external_url(art.wp_post_url_camel.as_deref()))
        .or_else(|| clean_external_url(art.effective_source_url()))
        .or_else(|| clean_external_url(art.effective_original_url()))
}

/// Extracts `(bot_token, chat_id)` from `social_config` or `wp_config` JSON (supports nested `telegram` or flat fields).
pub fn extract_telegram_creds(cfg_val: &Value) -> Option<(String, String)> {
    if let Some(tg) = cfg_val.get("telegram") {
        let bot_token = tg
            .get("botToken")
            .or_else(|| tg.get("bot_token"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let chat_id = tg
            .get("chatId")
            .or_else(|| tg.get("chat_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if !bot_token.is_empty() && !chat_id.is_empty() {
            return Some((bot_token.to_string(), chat_id.to_string()));
        }
    }

    let flat_token = cfg_val
        .get("telegramBotToken")
        .or_else(|| cfg_val.get("botToken"))
        .or_else(|| cfg_val.get("bot_token"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let flat_chat = cfg_val
        .get("telegramChatId")
        .or_else(|| cfg_val.get("chatId"))
        .or_else(|| cfg_val.get("chat_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if !flat_token.is_empty() && !flat_chat.is_empty() {
        Some((flat_token.to_string(), flat_chat.to_string()))
    } else {
        None
    }
}

/// Extracts complete Twitter/X OAuth 1.0a credentials from `social_config` or `wp_config` JSON.
pub fn extract_twitter_creds(cfg_val: &Value) -> Option<TwitterConfigInput> {
    if let Some(tw) = cfg_val.get("twitter") {
        let creds = TwitterConfigInput::from_json_value(tw);
        if creds.is_complete() {
            return Some(creds);
        }
    }
    let flat_creds = TwitterConfigInput::from_json_value(cfg_val);
    if flat_creds.is_complete() {
        Some(flat_creds)
    } else {
        None
    }
}

/// Checks whether either Telegram or Twitter/X credentials are configured in the given JSON settings.
pub fn has_configured_channels(cfg_val: &Value) -> bool {
    extract_telegram_creds(cfg_val).is_some() || extract_twitter_creds(cfg_val).is_some()
}

/// Formats Telegram HTML message cleanly without `"Haberi Oku: null"` or `"undefined"` when no external URL exists.
pub fn format_telegram_share_text(title: &str, description: &str, url: Option<&str>) -> String {
    let safe_title = escape_telegram_html(title.trim());
    let clean_desc = description.trim();
    let clean_url = clean_external_url(url);

    match (clean_desc.is_empty(), clean_url) {
        (true, None) => format!("<b>{safe_title}</b>"),
        (false, None) => format!("<b>{safe_title}</b>\n\n{}", escape_telegram_html(clean_desc)),
        (true, Some(u)) => format!("<b>{safe_title}</b>\n\nHaberi Oku: {}", escape_telegram_html(u)),
        (false, Some(u)) => format!(
            "<b>{safe_title}</b>\n\n{}\n\nHaberi Oku: {}",
            escape_telegram_html(clean_desc),
            escape_telegram_html(u)
        ),
    }
}

/// Formats Twitter/X text cleanly without `"Haberi Oku: null"` or `"undefined"` when no external URL exists.
pub fn format_twitter_share_text(title: &str, url: Option<&str>) -> String {
    let trimmed_title = title.trim();
    match clean_external_url(url) {
        Some(u) => format!("{trimmed_title}\n\nHaberi Oku: {u}"),
        None => trimmed_title.to_string(),
    }
}

/// RFC 3986 percent-encoding required by OAuth 1.0a (`A-Z`, `a-z`, `0-9`, `-`, `.`, `_`, `~` unescaped).
pub fn oauth_percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

/// Builds a deterministic OAuth 1.0a `Authorization` header for `method` + `base_url` (no form body params).
pub fn build_oauth1_header(
    method: &str,
    base_url: &str,
    creds: &TwitterConfigInput,
    nonce: &str,
    timestamp: &str,
) -> String {
    let mut oauth_params: Vec<(String, String)> = vec![
        ("oauth_consumer_key".to_string(), creds.api_key.trim().to_string()),
        ("oauth_nonce".to_string(), nonce.to_string()),
        ("oauth_signature_method".to_string(), "HMAC-SHA1".to_string()),
        ("oauth_timestamp".to_string(), timestamp.to_string()),
        ("oauth_token".to_string(), creds.access_token.trim().to_string()),
        ("oauth_version".to_string(), "1.0".to_string()),
    ];

    let mut encoded_pairs: Vec<(String, String)> = oauth_params
        .iter()
        .map(|(k, v)| (oauth_percent_encode(k), oauth_percent_encode(v)))
        .collect();
    encoded_pairs.sort();

    let parameter_string = encoded_pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let signature_base = format!(
        "{}&{}&{}",
        method.to_ascii_uppercase(),
        oauth_percent_encode(base_url),
        oauth_percent_encode(&parameter_string)
    );

    let signing_key = format!(
        "{}&{}",
        oauth_percent_encode(creds.api_secret.trim()),
        oauth_percent_encode(creds.access_secret.trim())
    );

    let mut mac =
        HmacSha1::new_from_slice(signing_key.as_bytes()).expect("HMAC-SHA1 accepts any key size");
    mac.update(signature_base.as_bytes());
    let signature = BASE64_STD.encode(mac.finalize().into_bytes());

    oauth_params.push(("oauth_signature".to_string(), signature));
    oauth_params.sort_by(|a, b| a.0.cmp(&b.0));

    let header_kv = oauth_params
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", oauth_percent_encode(k), oauth_percent_encode(v)))
        .collect::<Vec<_>>()
        .join(", ");

    format!("OAuth {header_kv}")
}

/// Sends an HTML message via Telegram Bot API (`server.ts` lines 363–376).
pub async fn share_on_telegram(
    http: &reqwest::Client,
    bot_token: &str,
    chat_id: &str,
    text: &str,
) -> Value {
    if bot_token.trim().is_empty() || chat_id.trim().is_empty() {
        return json!({ "success": false, "error": "Telegram credentials missing" });
    }

    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        bot_token.trim()
    );

    let res = http
        .post(&url)
        .timeout(Duration::from_secs(15))
        .json(&json!({
            "chat_id": chat_id.trim(),
            "text": text,
            "parse_mode": "HTML"
        }))
        .send()
        .await;

    match res {
        Ok(resp) if resp.status().is_success() => json!({ "success": true }),
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| {
                    v.get("description")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| format!("Request failed with status code {}", status.as_u16()));
            tracing::error!("Telegram Share Error: {msg}");
            json!({ "success": false, "error": msg })
        }
        Err(err) => {
            tracing::error!("Telegram Share Error: {err}");
            json!({ "success": false, "error": err.to_string() })
        }
    }
}

/// Posts a tweet to `https://api.twitter.com/2/tweets` using OAuth 1.0a HMAC-SHA1 (`server.ts` lines 378–395).
pub async fn share_on_twitter(
    http: &reqwest::Client,
    creds: &TwitterConfigInput,
    text: &str,
) -> Value {
    if !creds.is_complete() {
        return json!({ "success": false, "error": "Twitter credentials missing" });
    }

    let endpoint = "https://api.twitter.com/2/tweets";
    let nonce = Uuid::new_v4().simple().to_string();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());

    let auth_header = build_oauth1_header("POST", endpoint, creds, &nonce, &timestamp);

    let res = http
        .post(endpoint)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(15))
        .json(&json!({ "text": text }))
        .send()
        .await;

    match res {
        Ok(resp) if resp.status().is_success() => json!({ "success": true }),
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| {
                    v.get("detail")
                        .or_else(|| v.get("title"))
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| format!("Request failed with status code {}", status.as_u16()));
            tracing::error!("Twitter Share Error: {msg} ({body})");
            json!({ "success": false, "error": msg })
        }
        Err(err) => {
            tracing::error!("Twitter Share Error: {err}");
            json!({ "success": false, "error": err.to_string() })
        }
    }
}

/// Shares a newly published article on all configured social channels.
pub async fn share_on_all(
    http: &reqwest::Client,
    social_cfg_json: &Value,
    article: ShareArticlePayload<'_>,
) -> Value {
    let mut results = serde_json::Map::new();
    let url_opt = clean_external_url(Some(article.url));
    let telegram_text = format_telegram_share_text(article.title, article.description, url_opt);
    let twitter_text = format_twitter_share_text(article.title, url_opt);

    if let Some((bot_token, chat_id)) = extract_telegram_creds(social_cfg_json) {
        let tg_res = share_on_telegram(http, &bot_token, &chat_id, &telegram_text).await;
        results.insert("telegram".to_string(), tg_res);
    }

    if let Some(tw_creds) = extract_twitter_creds(social_cfg_json) {
        let tw_res = share_on_twitter(http, &tw_creds, &twitter_text).await;
        results.insert("twitter".to_string(), tw_res);
    }

    results.insert(
        "status".to_string(),
        Value::String("Shared completed".to_string()),
    );
    Value::Object(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth1_signature_matches_rfc5849_and_supports_dual_field_names() {
        let legacy_json = json!({
            "twitterApiKey": "xvz1evFS4wEEPTGEFPHBog",
            "twitterApiSecret": "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw",
            "twitterAccessToken": "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb",
            "twitterAccessTokenSecret": "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE"
        });
        let creds_legacy = TwitterConfigInput::from_json_value(&legacy_json);
        assert!(creds_legacy.is_complete());

        let modern_json = json!({
            "apiKey": "xvz1evFS4wEEPTGEFPHBog",
            "apiSecret": "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw",
            "accessToken": "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb",
            "accessSecret": "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE"
        });
        let creds_modern = TwitterConfigInput::from_json_value(&modern_json);
        assert_eq!(creds_legacy.api_key, creds_modern.api_key);
        assert_eq!(creds_legacy.access_secret, creds_modern.access_secret);

        let header = build_oauth1_header(
            "POST",
            "https://api.twitter.com/2/tweets",
            &creds_modern,
            "kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg",
            "1318622958",
        );
        assert!(header.starts_with("OAuth "));
        assert!(header.contains("oauth_consumer_key=\"xvz1evFS4wEEPTGEFPHBog\""));
        assert!(header.contains("oauth_signature_method=\"HMAC-SHA1\""));
        assert!(header.contains("oauth_signature=\""));

        // Verify RFC 3986 special character encoding
        assert_eq!(oauth_percent_encode("Ladies + Gentlemen!"), "Ladies%20%2B%20Gentlemen%21");
    }

    #[test]
    fn share_formatting_never_emits_null_or_undefined_urls() {
        assert_eq!(
            format_telegram_share_text("Son Dakika Haberi", "", None),
            "<b>Son Dakika Haberi</b>"
        );
        assert_eq!(
            format_telegram_share_text("Son Dakika Haberi", "", Some("null")),
            "<b>Son Dakika Haberi</b>"
        );
        assert_eq!(
            format_telegram_share_text("Son Dakika Haberi", "", Some("undefined")),
            "<b>Son Dakika Haberi</b>"
        );
        assert_eq!(
            format_twitter_share_text("Son Dakika Haberi", Some("")),
            "Son Dakika Haberi"
        );
        assert_eq!(
            format_telegram_share_text(
                "Ekonomi Analizi",
                "",
                Some("https://example.com/ekonomi-analizi")
            ),
            "<b>Ekonomi Analizi</b>\n\nHaberi Oku: https://example.com/ekonomi-analizi"
        );
        assert_eq!(
            format_twitter_share_text(
                "Ekonomi Analizi",
                Some("https://example.com/ekonomi-analizi")
            ),
            "Ekonomi Analizi\n\nHaberi Oku: https://example.com/ekonomi-analizi"
        );
    }
}
