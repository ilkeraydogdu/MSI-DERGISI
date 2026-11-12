use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Duration;

use crate::config::AppConfig;

/// Derives a deterministic 32-character hexadecimal IndexNow verification key
/// (or uses `INDEXNOW_KEY` from `.env` if explicitly configured).
pub fn resolve_indexnow_key(cfg: &AppConfig) -> String {
    if let Ok(explicit) = std::env::var("INDEXNOW_KEY") {
        let trimmed = explicit.trim();
        if trimmed.len() >= 8
            && trimmed.len() <= 128
            && trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return trimmed.to_string();
        }
    }
    let mut hasher = Sha256::new();
    hasher.update(b"msi-indexnow-v1:");
    hasher.update(cfg.primary_domain.as_bytes());
    hasher.update(b":");
    hasher.update(cfg.session_secret.as_bytes());
    let digest = hex::encode(hasher.finalize());
    digest[..32].to_string()
}

/// Builds the canonical IndexNow JSON payload for instant push indexing
/// across Yandex, Bing, Seznam, Naver, and AI search indexers.
pub fn build_indexnow_payload(cfg: &AppConfig, paths: &[String]) -> serde_json::Value {
    let base = cfg.public_url.trim_end_matches('/');
    let host = base
        .strip_prefix("https://")
        .or_else(|| base.strip_prefix("http://"))
        .unwrap_or(&cfg.primary_domain)
        .split('/')
        .next()
        .unwrap_or(&cfg.primary_domain);
    let key = resolve_indexnow_key(cfg);
    let key_location = format!("{}/{}.txt", base, key);
    let url_list: Vec<String> = paths
        .iter()
        .map(|p| {
            if p.starts_with("http://") || p.starts_with("https://") {
                p.clone()
            } else {
                format!("{}/{}", base, p.trim_start_matches('/'))
            }
        })
        .collect();

    json!({
        "host": host,
        "key": key,
        "keyLocation": key_location,
        "urlList": url_list,
    })
}

/// Asynchronously pushes updated URLs to the global IndexNow hub without blocking SSR/API responses.
pub fn spawn_instant_seo_push(cfg: AppConfig, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    let base = cfg.public_url.trim_end_matches('/');
    if base.contains("127.0.0.1") || base.contains("localhost") {
        return;
    }
    tokio::spawn(async move {
        let _ = push_urls_now(&cfg, &paths).await;
    });
}

pub async fn push_urls_now(cfg: &AppConfig, paths: &[String]) -> Result<u16, String> {
    let payload = build_indexnow_payload(cfg, paths);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("MSI-Dergisi-Rust-SEO-Engine/2.6 (+https://msidergisi.com)")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post("https://api.indexnow.org/indexnow")
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp.status().as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_32_hex_indexnow_key_and_payload() {
        let cfg = AppConfig::from_env();
        let key = resolve_indexnow_key(&cfg);
        assert_eq!(key.len(), 32);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));

        let payload = build_indexnow_payload(&cfg, &["/icerik/test-analiz".to_string()]);
        assert_eq!(payload["key"].as_str(), Some(key.as_str()));
        let urls = payload["urlList"].as_array().unwrap();
        assert_eq!(urls.len(), 1);
        assert!(urls[0].as_str().unwrap().ends_with("/icerik/test-analiz"));
    }
}

