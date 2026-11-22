//! Live News Research & Clean Reader View Engine (`Haber Araştırma & Okuyucu Görünümü`):
//! - SSRF-hardened HTTP fetcher with DNS pinning, per-hop redirect validation (`MAX_REDIRECTS = 5`),
//!   9s timeout, and 3.5 MB response cap.
//! - Live RSS search across Bing News (`TR` / `Global` + time filters) and multi-source breaking
//!   news aggregation (`TRT Haber`, `BBC Türkçe`, `BBC World`, `Bing News`).
//! - Pure-Rust, UTF-8 safe News Readability & Metadata (`OpenGraph`, `JSON-LD`, semantic `<article>`,
//!   noise stripping, `key_facts` & `media_embeds` extraction, graceful `blocked_by_source` fallback).

use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use crate::editorial::common::{
    sanitize_inline_rich_html, strip_all_html_tags, ContentBlock, ExtractedReaderArticle,
    ResearchSearchItem, ResearchSource,
};
use reqwest::{redirect::Policy, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::JoinSet;

use crate::services::editorial_media_service::{
    is_disallowed_hostname, is_disallowed_ip, EditorialResult as AppResult,
    EditorialServiceError as AppError,
};

/// Turkish-aware lowercase helper (`İ -> i`, `I -> ı`) for accurate keyword filtering.
pub fn turkish_lowercase(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'İ' => out.push('i'),
            'I' => out.push('ı'),
            _ => {
                for lc in ch.to_lowercase() {
                    out.push(lc);
                }
            }
        }
    }
    out
}

const MAX_REDIRECTS: usize = 5;
const HTTP_TIMEOUT_SECS: u64 = 9;
const CONNECT_TIMEOUT_SECS: u64 = 5;
const MAX_HTML_BYTES: usize = 3_670_016; // 3.5 MB (3.5 * 1024 * 1024)
const MAX_SEARCH_RESULTS: usize = 40;
const MAX_CONCURRENT_CUSTOM_SOURCES: usize = 8;
const CHROME_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedSourceSpec {
    pub name: String,
    pub url: String,
    pub kind: String,
    pub category: String,
    pub scope: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadArticleRequest {
    #[serde(default)]
    pub url: String,
    #[serde(default, alias = "fallback_title")]
    pub fallback_title: Option<String>,
    #[serde(default, alias = "fallback_snippet")]
    pub fallback_snippet: Option<String>,
    #[serde(default, alias = "fallback_source")]
    pub fallback_source: Option<String>,
    #[serde(default, alias = "fallback_image")]
    pub fallback_image: Option<String>,
}

enum RemoteFetchResult {
    Success { final_url: Url, body: String },
    BlockedBySource,
}

/// Synchronous URL syntax, scheme, hostname, and literal-IP SSRF guard.
pub fn validate_url_syntax_and_host(raw_url: &str) -> AppResult<(Url, String, u16, Option<IpAddr>)> {
    let trimmed = raw_url.trim();
    let parsed = Url::parse(trimmed)
        .map_err(|_| AppError::BadRequest("Geçerli bir haber bağlantısı (URL) girin.".to_string()))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::BadRequest(
            "Yalnızca http:// veya https:// bağlantıları desteklenir.".to_string(),
        ));
    }

    let host_str = parsed
        .host_str()
        .ok_or_else(|| AppError::BadRequest("Geçerli bir alan adı gerekli.".to_string()))?
        .to_string();

    if is_disallowed_hostname(&host_str) {
        return Err(AppError::BadRequest(
            "Güvenlik nedeniyle yerel/dahili adreslere erişim engellendi.".to_string(),
        ));
    }

    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| AppError::BadRequest("Geçerli bir bağlantı portu gerekli.".to_string()))?;

    let clean_ip_candidate = host_str.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip_lit) = clean_ip_candidate.parse::<IpAddr>() {
        if is_disallowed_ip(ip_lit) {
            return Err(AppError::BadRequest(
                "Güvenlik nedeniyle özel ağ ve metadata adreslerine erişim engellendi.".to_string(),
            ));
        }
        return Ok((parsed, host_str, port, Some(ip_lit)));
    }

    Ok((parsed, host_str, port, None))
}

/// Validates URL syntax + resolves DNS and ensures every resolved IP is public (anti-SSRF + anti-rebinding).
pub async fn validate_and_resolve_public_url(
    raw_url: &str,
) -> AppResult<(Url, String, Vec<SocketAddr>)> {
    let (parsed, host_str, port, ip_opt) = validate_url_syntax_and_host(raw_url)?;

    if let Some(ip_lit) = ip_opt {
        return Ok((parsed, host_str, vec![SocketAddr::new(ip_lit, port)]));
    }

    let resolved: Vec<SocketAddr> = tokio::net::lookup_host((host_str.as_str(), port))
        .await
        .map_err(|_| AppError::Internal("Kaynak haber sunucusu çözümlenemedi.".to_string()))?
        .collect();

    if resolved.is_empty() {
        return Err(AppError::Internal(
            "Kaynak haber sunucusu çözümlenemedi.".to_string(),
        ));
    }

    for sock in &resolved {
        if is_disallowed_ip(sock.ip()) {
            return Err(AppError::BadRequest(
                "Güvenlik nedeniyle özel ağ adreslerine erişim engellendi.".to_string(),
            ));
        }
    }

    Ok((parsed, host_str, resolved))
}

/// Fetches a remote HTML/XML text document with per-hop SSRF verification, 9s timeout, and 3.5 MB cap.
async fn fetch_remote_text_ssrf_safe(
    raw_url: &str,
    accept_header: &str,
    accept_lang: &str,
) -> AppResult<RemoteFetchResult> {
    let mut current_url = raw_url.trim().to_string();

    for _hop in 0..=MAX_REDIRECTS {
        let (parsed_url, host, addrs) = validate_and_resolve_public_url(&current_url).await?;

        let mut builder = reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS));

        if parsed_url.host_str().is_some() && !host.starts_with('[') {
            builder = builder.resolve_to_addrs(&host, &addrs);
        }

        let client = builder
            .build()
            .map_err(|e| AppError::Internal(format!("HTTP istemcisi başlatılamadı: {e}")))?;

        let send_res = client
            .get(parsed_url.clone())
            .header("User-Agent", CHROME_USER_AGENT)
            .header("Accept", accept_header)
            .header("Accept-Language", accept_lang)
            .header("Cache-Control", "no-cache")
            .send()
            .await;

        let mut resp = match send_res {
            Ok(r) => r,
            Err(e) => {
                if e.is_timeout() || e.is_connect() {
                    return Ok(RemoteFetchResult::BlockedBySource);
                }
                return Err(AppError::Internal(
                    "Kaynak haber sayfasına ulaşılamadı.".to_string(),
                ));
            }
        };

        let status = resp.status();
        if status.is_redirection() {
            if let Some(loc) = resp.headers().get(reqwest::header::LOCATION) {
                if let Ok(loc_str) = loc.to_str() {
                    let next_url = parsed_url.join(loc_str.trim()).map_err(|_| {
                        AppError::BadRequest("Geçersiz yönlendirme adresi.".to_string())
                    })?;
                    current_url = next_url.to_string();
                    continue;
                }
            }
            return Err(AppError::Internal(
                "Kaynak yönlendirmesi tamamlanamadı.".to_string(),
            ));
        }

        if matches!(
            status,
            StatusCode::UNAUTHORIZED
                | StatusCode::FORBIDDEN
                | StatusCode::TOO_MANY_REQUESTS
                | StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS
                | StatusCode::SERVICE_UNAVAILABLE
        ) || (520..=526).contains(&status.as_u16())
        {
            return Ok(RemoteFetchResult::BlockedBySource);
        }

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(
                "Kaynak haber sayfası bulunamadı (404).".to_string(),
            ));
        }

        if !status.is_success() {
            return Err(AppError::Internal(format!(
                "Kaynak sunucu yanıt hatası (HTTP {}).",
                status.as_u16()
            )));
        }

        if let Some(len) = resp.content_length() {
            if len as usize > MAX_HTML_BYTES {
                return Err(AppError::BadRequest(
                    "Kaynak sayfa boyutu çok büyük (maksimum 3.5 MB).".to_string(),
                ));
            }
        }

        let mut buf = Vec::with_capacity(64 * 1024);
        loop {
            match resp.chunk().await {
                Ok(Some(chunk)) => {
                    if buf.len().saturating_add(chunk.len()) > MAX_HTML_BYTES {
                        return Err(AppError::BadRequest(
                            "Kaynak sayfa boyutu çok büyük (maksimum 3.5 MB).".to_string(),
                        ));
                    }
                    buf.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(e) => {
                    if e.is_timeout() && buf.is_empty() {
                        return Ok(RemoteFetchResult::BlockedBySource);
                    }
                    if !buf.is_empty() {
                        break;
                    }
                    return Err(AppError::Internal(
                        "Kaynak sayfa verisi okunamadı.".to_string(),
                    ));
                }
            }
        }

        let body = decode_response_bytes(&buf, resp.headers());
        return Ok(RemoteFetchResult::Success {
            final_url: parsed_url,
            body,
        });
    }

    Err(AppError::BadRequest(
        "Çok fazla yönlendirme nedeniyle bağlantı reddedildi.".to_string(),
    ))
}

/// Decodes response bytes cleanly (UTF-8 fast path, with ISO-8859-9 / Windows-1254 Turkish fallback if legacy site).
fn decode_response_bytes(bytes: &[u8], headers: &reqwest::header::HeaderMap) -> String {
    if let Ok(utf8) = std::str::from_utf8(bytes) {
        return utf8.to_string();
    }
    let ct_lower = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ct_lower.contains("1254") || ct_lower.contains("8859-9") {
        return decode_windows_1254(bytes);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn decode_windows_1254(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 32);
    for &b in bytes {
        let ch = match b {
            0xD0 => 'Ğ',
            0xDD => 'İ',
            0xDE => 'Ş',
            0xF0 => 'ğ',
            0xFD => 'ı',
            0xFE => 'ş',
            0xC7 => 'Ç',
            0xE7 => 'ç',
            0xD6 => 'Ö',
            0xF6 => 'ö',
            0xDC => 'Ü',
            0xFC => 'ü',
            0x91 | 0x92 => '\'',
            0x93 | 0x94 => '"',
            0x96 | 0x97 => '-',
            0x80..=0x9F => ' ',
            _ => b as char,
        };
        out.push(ch);
    }
    out
}

/// Decodes CDATA wrappers (`<![CDATA[...]]>`) and HTML/XML named + numeric (`&#252;`, `&#x15F;`) entities.
pub fn decode_html_entities(input: &str) -> String {
    let unwrapped = unwrap_cdata_sections(input);
    if !unwrapped.contains('&') {
        return unwrapped;
    }

    let mut out = String::with_capacity(unwrapped.len());
    let mut rest = unwrapped.as_str();

    while let Some(amp_idx) = rest.find('&') {
        out.push_str(&rest[..amp_idx]);
        let after_amp = &rest[amp_idx + 1..];

        let max_scan = after_amp.len().min(14);
        // Safe slice: ';' is ASCII so its byte index in after_amp is always a valid char boundary
        let semi_opt = after_amp
            .as_bytes()
            .iter()
            .take(max_scan)
            .position(|&b| b == b';');

        if let Some(semi_pos) = semi_opt {
            let entity = &after_amp[..semi_pos];
            if let Some(decoded) = resolve_single_entity(entity) {
                out.push_str(&decoded);
                rest = &after_amp[semi_pos + 1..];
                continue;
            }
        }

        out.push('&');
        rest = after_amp;
    }

    out.push_str(rest);
    out
}

fn unwrap_cdata_sections(input: &str) -> String {
    if !input.contains("<![CDATA[") {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("<![CDATA[") {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 9..];
        if let Some(end) = after_open.find("]]>") {
            out.push_str(&after_open[..end]);
            rest = &after_open[end + 3..];
        } else {
            out.push_str(after_open);
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn resolve_single_entity(entity: &str) -> Option<String> {
    if let Some(num_str) = entity.strip_prefix('#') {
        let code = if let Some(hex_str) = num_str
            .strip_prefix('x')
            .or_else(|| num_str.strip_prefix('X'))
        {
            u32::from_str_radix(hex_str, 16).ok()?
        } else {
            num_str.parse::<u32>().ok()?
        };
        if code == 0 {
            return Some(String::new());
        }
        let mapped = match code {
            160 => ' ',
            145 => '‘',
            146 => '’',
            147 => '“',
            148 => '”',
            150 => '–',
            151 => '—',
            _ => char::from_u32(code)?,
        };
        return Some(mapped.to_string());
    }

    let named = match entity {
        "amp" | "AMP" => "&",
        "quot" | "QUOT" => "\"",
        "apos" | "APOS" => "'",
        "lt" | "LT" => "<",
        "gt" | "GT" => ">",
        "nbsp" | "NBSP" => " ",
        "rsquo" | "lsquo" => "'",
        "rdquo" | "ldquo" | "laquo" | "raquo" => "\"",
        "ndash" => "–",
        "mdash" => "—",
        "hellip" => "…",
        "bull" | "middot" => "•",
        "uuml" => "ü",
        "Uuml" => "Ü",
        "ouml" => "ö",
        "Ouml" => "Ö",
        "ccedil" => "ç",
        "Ccedil" => "Ç",
        "euro" => "€",
        "pound" => "£",
        "copy" => "©",
        "reg" => "®",
        "trade" => "™",
        _ => return None,
    };
    Some(named.to_string())
}

/// Unwraps Bing News redirect wrapper URLs (`apiclick.aspx?...&url=https%3a%2f%2f...`) into clean publisher URLs.
pub fn unwrap_bing_news_url(raw_link: &str) -> String {
    let decoded_xml = decode_html_entities(raw_link).trim().to_string();
    let lower = decoded_xml.to_ascii_lowercase();

    if lower.contains("bing.com/news/apiclick.aspx") {
        if let Ok(parsed) = Url::parse(&decoded_xml) {
            for (k, v) in parsed.query_pairs() {
                if k.eq_ignore_ascii_case("url") {
                    let candidate = v.trim();
                    if candidate.starts_with("https://") || candidate.starts_with("http://") {
                        return candidate.to_string();
                    }
                    if let Ok(redecoded) = urlencoding::decode(candidate) {
                        let c2 = redecoded.trim();
                        if c2.starts_with("https://") || c2.starts_with("http://") {
                            return c2.to_string();
                        }
                    }
                }
            }
        }
        // Fallback manual query parameter extraction
        if let Some(q_idx) = decoded_xml.find('?') {
            for pair in decoded_xml[q_idx + 1..].split('&') {
                if let Some(val) = pair.strip_prefix("url=").or_else(|| pair.strip_prefix("URL=")) {
                    if let Ok(dec) = urlencoding::decode(val) {
                        let clean = dec.trim();
                        if clean.starts_with("https://") || clean.starts_with("http://") {
                            return clean.to_string();
                        }
                    }
                }
            }
        }
    }

    decoded_xml
}

/// Upgrades Bing thumbnail URLs to HTTPS 480x270 and BBC thumbnails to crisp 640px.
pub fn normalize_rss_image_url(raw_img: &str) -> Option<String> {
    let clean = decode_html_entities(raw_img).trim().to_string();
    if clean.is_empty() || clean.starts_with("data:") {
        return None;
    }

    let lower = clean.to_ascii_lowercase();
    if lower.starts_with("http://www.bing.com/th?") || lower.starts_with("https://www.bing.com/th?")
    {
        let https_base = if let Some(rest) = clean.strip_prefix("http://") {
            format!("https://{rest}")
        } else {
            clean
        };
        if !https_base.to_ascii_lowercase().contains("&w=") {
            return Some(format!("{https_base}&w=480&h=270&c=7"));
        }
        return Some(https_base);
    }

    if clean.contains("ichef.bbci.co.uk/ace/ws/240/") {
        return Some(clean.replace("/ace/ws/240/", "/ace/ws/640/"));
    }

    if clean.starts_with("https://") || clean.starts_with("http://") {
        Some(clean)
    } else if clean.starts_with("//") {
        Some(format!("https:{clean}"))
    } else {
        None
    }
}

/// Parses RSS 2.0 `<item>` and Atom 1.0 `<entry>` nodes in pure UTF-8 safe Rust.
pub fn parse_rss_items(xml: &str, fallback_source: &str) -> Vec<ResearchSearchItem> {
    let mut out = Vec::new();
    let lower = xml.to_ascii_lowercase();

    // 1. RSS 2.0 / RDF `<item>...</item>`
    let mut cursor = 0usize;
    while let Some(rel_open) = lower[cursor..].find("<item") {
        let item_start = cursor + rel_open;
        let after_item_tag = &lower[item_start + 5..];
        if let Some(first_b) = after_item_tag.as_bytes().first() {
            if !matches!(first_b, b'>' | b' ' | b'\t' | b'\r' | b'\n') {
                cursor = item_start + 5;
                continue;
            }
        }
        let Some(gt_rel) = lower[item_start..].find('>') else {
            break;
        };
        let body_start = item_start + gt_rel + 1;
        let Some(end_rel) = lower[body_start..].find("</item>") else {
            break;
        };
        let item_xml = &xml[body_start..body_start + end_rel];
        cursor = body_start + end_rel + 7;

        if let Some(parsed) = parse_single_rss_item(item_xml, fallback_source) {
            out.push(parsed);
        }
    }

    // 2. Atom 1.0 `<entry>...</entry>`
    let mut ecursor = 0usize;
    while let Some(rel_open) = lower[ecursor..].find("<entry") {
        let entry_start = ecursor + rel_open;
        let after_entry_tag = &lower[entry_start + 6..];
        if let Some(first_b) = after_entry_tag.as_bytes().first() {
            if !matches!(first_b, b'>' | b' ' | b'\t' | b'\r' | b'\n') {
                ecursor = entry_start + 6;
                continue;
            }
        }
        let Some(gt_rel) = lower[entry_start..].find('>') else {
            break;
        };
        let body_start = entry_start + gt_rel + 1;
        let Some(end_rel) = lower[body_start..].find("</entry>") else {
            break;
        };
        let entry_xml = &xml[body_start..body_start + end_rel];
        ecursor = body_start + end_rel + 8;

        if let Some(parsed) = parse_single_atom_entry(entry_xml, fallback_source) {
            out.push(parsed);
        }
    }

    out
}

fn parse_single_rss_item(item_xml: &str, fallback_source: &str) -> Option<ResearchSearchItem> {
    let raw_title = extract_xml_tag_text(item_xml, "title")?;
    let title = collapse_ws(&strip_tags_raw(&decode_html_entities(&raw_title)));
    if title.is_empty() {
        return None;
    }

    let raw_link = extract_xml_tag_text(item_xml, "link")
        .or_else(|| extract_xml_tag_text(item_xml, "guid"))?;
    let url = unwrap_bing_news_url(&raw_link);
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return None;
    }
    if validate_url_syntax_and_host(&url).is_err() {
        return None;
    }

    let raw_desc = extract_xml_tag_text(item_xml, "description").unwrap_or_default();
    let decoded_desc = decode_html_entities(&raw_desc);
    let desc_img_fallback = extract_first_img_src_raw(&decoded_desc);
    let snippet_clean = collapse_ws(&strip_tags_raw(&decoded_desc));
    let snippet = if snippet_clean.is_empty() || snippet_clean == title {
        None
    } else {
        Some(truncate_chars(&snippet_clean, 280))
    };

    let source_name = extract_xml_tag_text(item_xml, "news:source")
        .or_else(|| extract_xml_tag_text(item_xml, "source"))
        .map(|s| collapse_ws(&strip_tags_raw(&decode_html_entities(&s))))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if !fallback_source.trim().is_empty() {
                fallback_source.to_string()
            } else {
                domain_from_url_str(&url)
            }
        });

    let raw_img = extract_xml_tag_text(item_xml, "news:image")
        .or_else(|| extract_xml_tag_attr(item_xml, "media:content", "url"))
        .or_else(|| extract_xml_tag_attr(item_xml, "media:thumbnail", "url"))
        .or_else(|| extract_xml_tag_attr(item_xml, "enclosure", "url"))
        .or_else(|| extract_xml_tag_text(item_xml, "image"))
        .or_else(|| desc_img_fallback);
    let image_url = raw_img.and_then(|u| normalize_rss_image_url(&u));

    let published_at = extract_xml_tag_text(item_xml, "pubdate")
        .or_else(|| extract_xml_tag_text(item_xml, "dc:date"))
        .or_else(|| extract_xml_tag_text(item_xml, "published"))
        .map(|d| collapse_ws(&decode_html_entities(&d)))
        .filter(|d| !d.is_empty());

    Some(ResearchSearchItem {
        title,
        url,
        source_name,
        snippet,
        image_url,
        published_at,
    })
}

fn parse_single_atom_entry(entry_xml: &str, fallback_source: &str) -> Option<ResearchSearchItem> {
    let raw_title = extract_xml_tag_text(entry_xml, "title")?;
    let title = collapse_ws(&strip_tags_raw(&decode_html_entities(&raw_title)));
    if title.is_empty() {
        return None;
    }

    let (atom_href_opt, enclosure_img_opt) = extract_atom_entry_links(entry_xml);
    let raw_link = atom_href_opt
        .or_else(|| extract_xml_tag_text(entry_xml, "link"))
        .or_else(|| extract_xml_tag_text(entry_xml, "id"))?;
    let url = unwrap_bing_news_url(&raw_link);
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return None;
    }
    if validate_url_syntax_and_host(&url).is_err() {
        return None;
    }

    let raw_summary = extract_xml_tag_text(entry_xml, "summary")
        .or_else(|| extract_xml_tag_text(entry_xml, "content"))
        .or_else(|| extract_xml_tag_text(entry_xml, "media:description"))
        .unwrap_or_default();
    let decoded_summary = decode_html_entities(&raw_summary);
    let content_img_fallback = extract_first_img_src_raw(&decoded_summary);
    let snippet_clean = collapse_ws(&strip_tags_raw(&decoded_summary));
    let snippet = if snippet_clean.is_empty() || snippet_clean == title {
        None
    } else {
        Some(truncate_chars(&snippet_clean, 280))
    };

    let entry_source_title = extract_xml_tag_text(entry_xml, "source")
        .and_then(|src_inner| {
            extract_xml_tag_text(&src_inner, "title").or(Some(src_inner))
        })
        .map(|s| collapse_ws(&strip_tags_raw(&decode_html_entities(&s))))
        .filter(|s| !s.is_empty());

    let source_name = entry_source_title.unwrap_or_else(|| {
        if !fallback_source.trim().is_empty() {
            fallback_source.to_string()
        } else {
            domain_from_url_str(&url)
        }
    });

    let raw_img = extract_xml_tag_attr(entry_xml, "media:content", "url")
        .or_else(|| extract_xml_tag_attr(entry_xml, "media:thumbnail", "url"))
        .or(enclosure_img_opt)
        .or(content_img_fallback);
    let image_url = raw_img.and_then(|u| normalize_rss_image_url(&u));

    let published_at = extract_xml_tag_text(entry_xml, "published")
        .or_else(|| extract_xml_tag_text(entry_xml, "updated"))
        .or_else(|| extract_xml_tag_text(entry_xml, "dc:date"))
        .map(|d| collapse_ws(&decode_html_entities(&d)))
        .filter(|d| !d.is_empty());

    Some(ResearchSearchItem {
        title,
        url,
        source_name,
        snippet,
        image_url,
        published_at,
    })
}

fn extract_atom_entry_links(entry_xml: &str) -> (Option<String>, Option<String>) {
    let lower = entry_xml.to_ascii_lowercase();
    let mut pos = 0usize;
    let mut preferred_alt: Option<String> = None;
    let mut fallback_href: Option<String> = None;
    let mut enclosure_img: Option<String> = None;

    while let Some(rel) = lower[pos..].find("<link") {
        let start = pos + rel;
        let after = start + 5;
        let next_b = lower.as_bytes().get(after).copied();
        if !matches!(next_b, Some(b'>' | b' ' | b'\t' | b'\r' | b'\n' | b'/')) {
            pos = after;
            continue;
        }
        let Some(gt_rel) = lower[after..].find('>') else {
            break;
        };
        let attrs = &entry_xml[after..after + gt_rel];
        pos = after + gt_rel + 1;

        let Some(href) = get_html_attr(attrs, "href") else {
            continue;
        };
        let clean_href = href.trim();
        if clean_href.is_empty() {
            continue;
        }

        let rel_attr = get_html_attr(attrs, "rel")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let type_attr = get_html_attr(attrs, "type")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();

        if rel_attr == "enclosure"
            && (type_attr.starts_with("image/")
                || clean_href.ends_with(".jpg")
                || clean_href.ends_with(".jpeg")
                || clean_href.ends_with(".png")
                || clean_href.ends_with(".webp"))
        {
            if enclosure_img.is_none() {
                enclosure_img = Some(clean_href.to_string());
            }
            continue;
        }

        if rel_attr.is_empty() || rel_attr == "alternate" {
            if preferred_alt.is_none() {
                preferred_alt = Some(clean_href.to_string());
            }
        } else if rel_attr != "self" && rel_attr != "replies" && fallback_href.is_none() {
            fallback_href = Some(clean_href.to_string());
        }
    }

    (preferred_alt.or(fallback_href), enclosure_img)
}

fn extract_xml_tag_text(xml: &str, tag_lower_name: &str) -> Option<String> {
    let lower = xml.to_ascii_lowercase();
    let open_pat = format!("<{tag_lower_name}");
    let close_pat = format!("</{tag_lower_name}>");
    let mut pos = 0usize;

    while let Some(rel) = lower[pos..].find(&open_pat) {
        let start = pos + rel;
        let after_name = start + open_pat.len();
        let next_b = lower.as_bytes().get(after_name).copied();
        if !matches!(next_b, Some(b'>' | b' ' | b'\t' | b'\r' | b'\n')) {
            pos = after_name;
            continue;
        }
        let gt_rel = lower[after_name..].find('>')?;
        let gt_abs = after_name + gt_rel;
        if gt_abs > start && lower.as_bytes().get(gt_abs - 1) == Some(&b'/') {
            pos = gt_abs + 1;
            continue;
        }
        let content_start = gt_abs + 1;
        let end_rel = lower[content_start..].find(&close_pat)?;
        return Some(xml[content_start..content_start + end_rel].trim().to_string());
    }
    None
}

fn extract_xml_tag_attr(xml: &str, tag_lower_name: &str, attr_lower: &str) -> Option<String> {
    let lower = xml.to_ascii_lowercase();
    let open_pat = format!("<{tag_lower_name}");
    let mut pos = 0usize;

    while let Some(rel) = lower[pos..].find(&open_pat) {
        let start = pos + rel;
        let after_name = start + open_pat.len();
        let next_b = lower.as_bytes().get(after_name).copied();
        if !matches!(next_b, Some(b'>' | b' ' | b'\t' | b'\r' | b'\n' | b'/')) {
            pos = after_name;
            continue;
        }
        let gt_rel = lower[after_name..].find('>')?;
        let raw_tag = &xml[after_name..after_name + gt_rel];
        if let Some(val) = get_html_attr(raw_tag, attr_lower) {
            if !val.trim().is_empty() {
                return Some(val);
            }
        }
        pos = after_name + gt_rel + 1;
    }
    None
}

/// Smart RSS/Atom & Publisher Site Auto-Discovery (`POST`/`PUT /api/research/sources`).
pub async fn discover_and_normalize_source(
    raw_name: &str,
    raw_url: &str,
    raw_category: &str,
    raw_scope: &str,
    enabled: bool,
) -> AppResult<NormalizedSourceSpec> {
    let trimmed_url = raw_url.trim();
    if trimmed_url.is_empty() {
        return Err(AppError::BadRequest(
            "Lütfen bir RSS adresi veya haber sitesi bağlantısı girin.".to_string(),
        ));
    }

    let url_with_scheme = if trimmed_url.to_ascii_lowercase().starts_with("http://")
        || trimmed_url.to_ascii_lowercase().starts_with("https://")
    {
        trimmed_url.to_string()
    } else {
        format!("https://{trimmed_url}")
    };

    let (parsed_initial, _, _, _) = validate_url_syntax_and_host(&url_with_scheme)?;
    let normalized_initial_url = parsed_initial.to_string();
    let initial_domain = domain_from_url_str(&normalized_initial_url);

    let scope = if raw_scope.trim().eq_ignore_ascii_case("global") {
        "global".to_string()
    } else {
        "tr".to_string()
    };

    let category_clean = collapse_ws(raw_category);
    let category = if category_clean.is_empty() {
        "Gündem".to_string()
    } else {
        truncate_plain_chars(&category_clean, 80)
    };

    let explicit_name = collapse_ws(raw_name);
    let probe_accept = "application/rss+xml, application/atom+xml, application/xml, text/xml, text/html;q=0.9, */*;q=0.8";
    let probe_lang = if scope == "global" {
        "en-US,en;q=0.9"
    } else {
        "tr-TR,tr;q=0.9,en-US;q=0.8"
    };

    match fetch_remote_text_ssrf_safe(&normalized_initial_url, probe_accept, probe_lang).await {
        Ok(RemoteFetchResult::Success { final_url, body }) => {
            if body_looks_like_feed_xml(&body) {
                let final_str = final_url.to_string();
                let resolved_name = if !explicit_name.is_empty() {
                    explicit_name
                } else {
                    extract_feed_channel_title(&body)
                        .unwrap_or_else(|| domain_from_url_str(&final_str))
                };
                return Ok(NormalizedSourceSpec {
                    name: truncate_plain_chars(&resolved_name, 160),
                    url: final_str,
                    kind: "rss".to_string(),
                    category,
                    scope,
                    enabled,
                });
            }

            // HTML response: scan `<link rel="alternate" type="application/rss+xml|atom+xml" href="...">`
            let meta = extract_head_metadata(&body);
            let inferred_site_name = if !explicit_name.is_empty() {
                explicit_name
            } else {
                first_nonempty(&[meta.og_site_name.as_deref(), meta.doc_title.as_deref()])
                    .map(|t| strip_site_suffix(&t, ""))
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| domain_from_url_str(final_url.as_str()))
            };

            if let Some(feed_url) = discover_rss_link_in_html(Some(&final_url), &body) {
                return Ok(NormalizedSourceSpec {
                    name: truncate_plain_chars(&inferred_site_name, 160),
                    url: feed_url,
                    kind: "rss".to_string(),
                    category,
                    scope,
                    enabled,
                });
            }

            Ok(NormalizedSourceSpec {
                name: truncate_plain_chars(&inferred_site_name, 160),
                url: final_url.to_string(),
                kind: "site".to_string(),
                category,
                scope,
                enabled,
            })
        }
        Ok(RemoteFetchResult::BlockedBySource) => {
            let kind = if url_looks_like_feed(&normalized_initial_url) {
                "rss"
            } else {
                "site"
            };
            let resolved_name = if !explicit_name.is_empty() {
                explicit_name
            } else {
                initial_domain
            };
            Ok(NormalizedSourceSpec {
                name: truncate_plain_chars(&resolved_name, 160),
                url: normalized_initial_url,
                kind: kind.to_string(),
                category,
                scope,
                enabled,
            })
        }
        Err(err) => {
            if let AppError::BadRequest(ref msg) = err {
                if msg.contains("Güvenlik") || msg.contains("Yalnızca") {
                    return Err(err);
                }
            }
            // Do not fail source creation on transient probe timeout / bot-protection / 404 on homepage
            let kind = if url_looks_like_feed(&normalized_initial_url) {
                "rss"
            } else {
                "site"
            };
            let resolved_name = if !explicit_name.is_empty() {
                explicit_name
            } else {
                initial_domain
            };
            Ok(NormalizedSourceSpec {
                name: truncate_plain_chars(&resolved_name, 160),
                url: normalized_initial_url,
                kind: kind.to_string(),
                category,
                scope,
                enabled,
            })
        }
    }
}

/// Checks whether fetched document text starts like an RSS 2.0, RDF, or Atom XML feed.
pub fn body_looks_like_feed_xml(body: &str) -> bool {
    let prefix_end = floor_char_boundary(body, 4096);
    let head = body[..prefix_end]
        .trim_start_matches('\u{feff}')
        .trim_start()
        .to_ascii_lowercase();
    head.contains("<rss")
        || head.contains("<feed")
        || head.contains("<rdf:rdf")
        || head.contains("<channel>")
        || head.contains("<channel ")
}

/// Heuristic feed URL detector used when remote server blocks automated probes.
pub fn url_looks_like_feed(url: &str) -> bool {
    let l = url.trim().to_ascii_lowercase();
    l.ends_with(".rss")
        || l.ends_with(".xml")
        || l.ends_with(".atom")
        || l.contains("/rss")
        || l.contains("/feed")
        || l.contains("format=rss")
        || l.contains("format=atom")
        || l.contains("://feeds.")
        || l.contains("://rss.")
}

/// Extracts the feed/channel `<title>` appearing before the first `<item>` or `<entry>`.
pub fn extract_feed_channel_title(xml: &str) -> Option<String> {
    let lower = xml.to_ascii_lowercase();
    let cutoff = lower
        .find("<item")
        .into_iter()
        .chain(lower.find("<entry"))
        .min()
        .unwrap_or(xml.len());
    let header_slice = &xml[..cutoff];
    let raw = extract_xml_tag_text(header_slice, "title")?;
    let clean = collapse_ws(&strip_tags_raw(&decode_html_entities(&raw)));
    (!clean.is_empty()).then_some(clean)
}

/// Scans first ~64 KB of HTML for `<link rel="alternate" type="application/rss+xml|atom+xml" href="...">`.
pub fn discover_rss_link_in_html(base_url: Option<&Url>, html: &str) -> Option<String> {
    let scan_end = floor_char_boundary(html, 65_536);
    let slice = &html[..scan_end];
    let lower = slice.to_ascii_lowercase();

    let mut pos = 0usize;
    let mut fallback_comment_feed: Option<String> = None;

    while let Some(rel) = lower[pos..].find("<link") {
        let start = pos + rel;
        let after = start + 5;
        if !matches!(
            lower.as_bytes().get(after),
            Some(b' ' | b'\t' | b'\r' | b'\n')
        ) {
            pos = after;
            continue;
        }
        let Some(gt_rel) = lower[after..].find('>') else {
            break;
        };
        let attrs = &slice[after..after + gt_rel];
        pos = after + gt_rel + 1;

        let is_alternate = get_html_attr(attrs, "rel")
            .map(|r| {
                r.split_whitespace()
                    .any(|tok| tok.eq_ignore_ascii_case("alternate"))
            })
            .unwrap_or(false);
        if !is_alternate {
            continue;
        }

        let is_feed_mime = get_html_attr(attrs, "type")
            .map(|t| {
                let tl = t.to_ascii_lowercase();
                tl.contains("rss+xml") || tl.contains("atom+xml") || tl.contains("rdf+xml")
            })
            .unwrap_or(false);
        if !is_feed_mime {
            continue;
        }

        let Some(href) = get_html_attr(attrs, "href") else {
            continue;
        };
        let Some(resolved) = resolve_relative_url(base_url, &href) else {
            continue;
        };
        if validate_url_syntax_and_host(&resolved).is_err() {
            continue;
        }

        let lower_res = resolved.to_ascii_lowercase();
        if lower_res.contains("/comments/feed") || lower_res.contains("comment") {
            if fallback_comment_feed.is_none() {
                fallback_comment_feed = Some(resolved);
            }
            continue;
        }

        return Some(resolved);
    }

    fallback_comment_feed
}

/// Live News Search (`GET /api/research/search`) supporting custom RSS/Site sources & round-robin aggregation.
pub async fn search_news(
    query: &str,
    scope: &str,
    time_range: &str,
    source_filter: Option<&ResearchSource>,
    enabled_sources: &[ResearchSource],
) -> AppResult<Vec<ResearchSearchItem>> {
    let q = query.trim();

    // Direct URL paste mode (when no specific source filter is active)
    if source_filter.is_none() && (q.starts_with("https://") || q.starts_with("http://")) {
        let (parsed, _, _, _) = validate_url_syntax_and_host(q)?;
        let domain = parsed
            .host_str()
            .unwrap_or("Kaynak")
            .trim_start_matches("www.")
            .to_string();
        let slug_hint = parsed
            .path_segments()
            .and_then(|segs| segs.rev().find(|s| !s.is_empty()))
            .map(humanize_url_segment)
            .filter(|s| s.chars().count() >= 5)
            .unwrap_or_else(|| parsed.to_string());

        return Ok(vec![ResearchSearchItem {
            title: slug_hint,
            url: parsed.to_string(),
            source_name: domain,
            snippet: Some(
                "Doğrudan haber bağlantısı — Okuyucu Görünümü ile temiz metni açmak için tıklayın."
                    .to_string(),
            ),
            image_url: None,
            published_at: None,
        }]);
    }

    let effective_scope = source_filter
        .map(|s| s.scope.as_str())
        .unwrap_or(scope);
    let is_global = effective_scope.trim().eq_ignore_ascii_case("global");
    let (lang, cc, accept_lang) = if is_global {
        ("en-US", "US", "en-US,en;q=0.9")
    } else {
        ("tr-TR", "TR", "tr-TR,tr;q=0.9,en-US;q=0.7")
    };

    let qft = match time_range.trim().to_ascii_lowercase().as_str() {
        "1h" => "&qft=interval%3d%224%22+sortbydate%3d%221%22",
        "24h" => "&qft=interval%3d%227%22+sortbydate%3d%221%22",
        "7d" => "&qft=interval%3d%228%22+sortbydate%3d%221%22",
        _ => "&qft=sortbydate%3d%221%22",
    };

    let rss_accept = "application/rss+xml, application/atom+xml, application/xml, text/xml, */*;q=0.8";

    // 1. Source-filtered mode (`sourceId` selected in UI)
    if let Some(src) = source_filter {
        let site_domain = clean_host_for_site_search(&src.url);
        if src.kind.eq_ignore_ascii_case("rss") {
            if q.is_empty() {
                let rss_items = fetch_and_parse_rss(&src.url, &src.name, rss_accept, accept_lang)
                    .await
                    .unwrap_or_default();
                if !rss_items.is_empty() {
                    return Ok(dedup_and_limit_items(vec![rss_items], MAX_SEARCH_RESULTS));
                }
                if !site_domain.is_empty() {
                    let bing_fallback = build_bing_rss_url(&format!("site:{site_domain}"), lang, cc, qft);
                    let items = fetch_and_parse_rss(&bing_fallback, &src.name, rss_accept, accept_lang)
                        .await
                        .unwrap_or_default();
                    return Ok(dedup_and_limit_items(vec![items], MAX_SEARCH_RESULTS));
                }
                return Ok(Vec::new());
            }

            // Non-empty query on an RSS source: filter live RSS feed + run `site:{domain} {q}` concurrently
            let bing_site_q = if site_domain.is_empty() {
                format!("{} {q}", src.name)
            } else {
                format!("site:{site_domain} {q}")
            };
            let bing_url = build_bing_rss_url(&bing_site_q, lang, cc, qft);

            let (rss_res, bing_res) = tokio::join!(
                fetch_and_parse_rss(&src.url, &src.name, rss_accept, accept_lang),
                fetch_and_parse_rss(&bing_url, &src.name, rss_accept, accept_lang),
            );

            let mut feeds = Vec::new();
            if let Ok(rss_list) = rss_res {
                let matched: Vec<ResearchSearchItem> = rss_list
                    .into_iter()
                    .filter(|item| item_matches_query(item, q))
                    .collect();
                if !matched.is_empty() {
                    feeds.push(matched);
                }
            }
            if let Ok(bing_list) = bing_res {
                if !bing_list.is_empty() {
                    feeds.push(bing_list);
                }
            }
            return Ok(dedup_and_limit_items(feeds, MAX_SEARCH_RESULTS));
        }

        // `src.kind == "site"`
        let site_q = if site_domain.is_empty() {
            if q.is_empty() {
                src.name.clone()
            } else {
                format!("{} {q}", src.name)
            }
        } else if q.is_empty() {
            format!("site:{site_domain}")
        } else {
            format!("site:{site_domain} {q}")
        };
        let bing_url = build_bing_rss_url(&site_q, lang, cc, qft);
        let items = fetch_and_parse_rss(&bing_url, &src.name, rss_accept, accept_lang).await?;
        return Ok(dedup_and_limit_items(vec![items], MAX_SEARCH_RESULTS));
    }

    // 2. Global keyword search across all news (`source_filter == None` & non-empty query)
    if !q.is_empty() {
        let bing_url = build_bing_rss_url(q, lang, cc, qft);
        let default_src = if is_global { "Global News" } else { "Haber Kaynağı" };
        let items = fetch_and_parse_rss(&bing_url, default_src, rss_accept, accept_lang).await?;
        return Ok(dedup_and_limit_items(vec![items], MAX_SEARCH_RESULTS));
    }

    // 3. Empty query (`Canlı Gündem`) -> concurrent fetch of enabled custom sources + Bing breaking feed
    let target_scope = if is_global { "global" } else { "tr" };
    let scoped_sources: Vec<&ResearchSource> = enabled_sources
        .iter()
        .filter(|s| s.enabled && s.scope.trim().eq_ignore_ascii_case(target_scope))
        .take(MAX_CONCURRENT_CUSTOM_SOURCES)
        .collect();

    let mut join_set: JoinSet<(usize, Vec<ResearchSearchItem>)> = JoinSet::new();
    let mut slot_idx = 0usize;

    if scoped_sources.is_empty() {
        // Fallback default official feeds when caller passes empty `enabled_sources` (e.g. standalone tests)
        if is_global {
            let url = "https://feeds.bbci.co.uk/news/world/rss.xml".to_string();
            let idx = slot_idx;
            slot_idx += 1;
            join_set.spawn(async move {
                let v = fetch_and_parse_rss(&url, "BBC World News", rss_accept, accept_lang)
                    .await
                    .unwrap_or_default();
                (idx, v)
            });
        } else {
            for (name, url) in [
                ("TRT Haber", "https://www.trthaber.com/sondakika_articles.rss"),
                ("BBC Türkçe", "https://feeds.bbci.co.uk/turkce/rss.xml"),
                ("Anadolu Ajansı", "https://www.aa.com.tr/tr/rss/default?cat=guncel"),
            ] {
                let idx = slot_idx;
                slot_idx += 1;
                let u = url.to_string();
                let n = name.to_string();
                join_set.spawn(async move {
                    let v = fetch_and_parse_rss(&u, &n, rss_accept, accept_lang)
                        .await
                        .unwrap_or_default();
                    (idx, v)
                });
            }
        }
    } else {
        for src in scoped_sources {
            let idx = slot_idx;
            slot_idx += 1;
            let src_name = src.name.clone();
            let fetch_url = if src.kind.eq_ignore_ascii_case("rss") {
                src.url.clone()
            } else {
                let dom = clean_host_for_site_search(&src.url);
                let sq = if dom.is_empty() {
                    src.name.clone()
                } else {
                    format!("site:{dom}")
                };
                build_bing_rss_url(&sq, lang, cc, "&qft=interval%3d%227%22+sortbydate%3d%221%22")
            };
            join_set.spawn(async move {
                let v = fetch_and_parse_rss(&fetch_url, &src_name, rss_accept, accept_lang)
                    .await
                    .unwrap_or_default();
                (idx, v)
            });
        }
    }

    // Always include live Bing breaking news feed for broad coverage (sorted by date)
    let bing_breaking_url = if is_global {
        "https://www.bing.com/news/search?q=world+breaking+news&format=rss&setlang=en-US&cc=US&qft=interval%3d%227%22+sortbydate%3d%221%22".to_string()
    } else {
        "https://www.bing.com/news/search?q=g%C3%BCndem+son+dakika&format=rss&setlang=tr-TR&cc=TR&qft=interval%3d%227%22+sortbydate%3d%221%22".to_string()
    };
    let bing_default_src = if is_global { "Global News" } else { "Gündem" };
    let bing_slot = slot_idx;
    join_set.spawn(async move {
        let v = fetch_and_parse_rss(&bing_breaking_url, bing_default_src, rss_accept, accept_lang)
            .await
            .unwrap_or_default();
        (bing_slot, v)
    });

    let mut indexed_results: Vec<(usize, Vec<ResearchSearchItem>)> = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        if let Ok((idx, items)) = joined {
            if !items.is_empty() {
                indexed_results.push((idx, items));
            }
        }
    }
    indexed_results.sort_by_key(|(idx, _)| *idx);

    let feeds: Vec<Vec<ResearchSearchItem>> =
        indexed_results.into_iter().map(|(_, list)| list).collect();

    Ok(dedup_and_limit_items(feeds, MAX_SEARCH_RESULTS))
}

fn build_bing_rss_url(query: &str, lang: &str, cc: &str, qft: &str) -> String {
    format!(
        "https://www.bing.com/news/search?q={}&format=rss&setlang={lang}&cc={cc}{qft}",
        urlencoding::encode(query)
    )
}

fn clean_host_for_site_search(raw_url: &str) -> String {
    let Ok(parsed) = Url::parse(raw_url.trim()) else {
        return String::new();
    };
    let Some(host) = parsed.host_str() else {
        return String::new();
    };
    let h = host.trim().to_ascii_lowercase();
    if h == "feeds.bbci.co.uk" {
        return "bbc.com".to_string();
    }
    let no_www = h.strip_prefix("www.").unwrap_or(&h);
    for feed_pref in ["feeds.", "rss."] {
        if let Some(rest) = no_www.strip_prefix(feed_pref) {
            if rest.contains('.') {
                return rest.to_string();
            }
        }
    }
    no_www.to_string()
}

fn item_matches_query(item: &ResearchSearchItem, query: &str) -> bool {
    let hay = turkish_lowercase(&format!(
        "{} {}",
        item.title,
        item.snippet.as_deref().unwrap_or("")
    ));
    query
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .all(|word| hay.contains(&turkish_lowercase(word)))
}

async fn fetch_and_parse_rss(
    rss_url: &str,
    default_source: &str,
    accept_header: &str,
    accept_lang: &str,
) -> AppResult<Vec<ResearchSearchItem>> {
    match fetch_remote_text_ssrf_safe(rss_url, accept_header, accept_lang).await? {
        RemoteFetchResult::Success { body, .. } => Ok(parse_rss_items(&body, default_source)),
        RemoteFetchResult::BlockedBySource => Ok(Vec::new()),
    }
}

/// Round-robin interleaves items across feeds (`idx 0` of each feed, then `idx 1`, ...),
/// deduplicates, and sorts chronologically so the freshest news is always at the top.
pub fn dedup_and_limit_items(
    feed_groups: Vec<Vec<ResearchSearchItem>>,
    limit: usize,
) -> Vec<ResearchSearchItem> {
    let max_group_len = feed_groups.iter().map(Vec::len).max().unwrap_or(0);
    let mut seen_urls = HashSet::new();
    let mut seen_titles = HashSet::new();
    let pool_capacity = (limit.saturating_mul(2)).max(64);
    let mut candidates = Vec::with_capacity(pool_capacity);

    for idx in 0..max_group_len {
        for group in &feed_groups {
            let Some(item) = group.get(idx) else {
                continue;
            };
            let norm_url = normalize_url_for_dedup(&item.url);
            let norm_title = normalize_title_for_dedup(&item.title);
            if norm_url.is_empty() || norm_title.is_empty() {
                continue;
            }
            if !seen_urls.insert(norm_url) || !seen_titles.insert(norm_title) {
                continue;
            }
            candidates.push(item.clone());
            if candidates.len() >= pool_capacity {
                break;
            }
        }
        if candidates.len() >= pool_capacity {
            break;
        }
    }

    // Sort chronologically (freshest timestamps first, preserving stable order for equal/undated)
    candidates.sort_by(|a, b| {
        let ts_b = parse_date_epoch_secs(b.published_at.as_deref());
        let ts_a = parse_date_epoch_secs(a.published_at.as_deref());
        ts_b.cmp(&ts_a)
    });

    if candidates.len() > limit {
        candidates.truncate(limit);
    }
    candidates
}

fn parse_date_epoch_secs(raw: Option<&str>) -> i64 {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return 0;
    };
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(s) {
        return dt.timestamp();
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.timestamp();
    }
    0
}

fn floor_char_boundary(s: &str, max_bytes: usize) -> usize {
    let mut idx = max_bytes.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn truncate_plain_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect::<String>().trim().to_string()
}

fn normalize_url_for_dedup(raw: &str) -> String {
    if let Ok(mut u) = Url::parse(raw.trim()) {
        u.set_fragment(None);
        let host = u
            .host_str()
            .unwrap_or("")
            .trim_start_matches("www.")
            .to_ascii_lowercase();
        let path = u.path().trim_end_matches('/').to_string();
        return format!("{host}{path}");
    }
    raw.trim().trim_end_matches('/').to_ascii_lowercase()
}

fn normalize_title_for_dedup(title: &str) -> String {
    title
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn humanize_url_segment(seg: &str) -> String {
    let cleaned = seg
        .split('.')
        .next()
        .unwrap_or(seg)
        .replace(['-', '_'], " ");
    let collapsed = collapse_ws(&cleaned);
    let mut chars = collapsed.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => collapsed,
    }
}

fn domain_from_url_str(raw: &str) -> String {
    Url::parse(raw)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Kaynak".to_string())
}

/// Clean Reader View Extractor (`POST /api/research/read`).
pub async fn extract_reader_article(req: ReadArticleRequest) -> AppResult<ExtractedReaderArticle> {
    let (parsed_req_url, _, _, _) = validate_url_syntax_and_host(&req.url)?;
    let normalized_req_url = parsed_req_url.to_string();

    let has_fallback = req
        .fallback_title
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
        || req
            .fallback_snippet
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

    let accept_html = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
    let accept_lang = "tr-TR,tr;q=0.9,en-US;q=0.8,en;q=0.7";

    let fetch_res = fetch_remote_text_ssrf_safe(&normalized_req_url, accept_html, accept_lang).await;

    match fetch_res {
        Ok(RemoteFetchResult::Success { final_url, body }) => {
            let mut extracted = parse_article_html_to_reader(final_url.as_str(), &body);

            // Fill missing fields from RSS card metadata when available
            if extracted.title.trim().is_empty() {
                if let Some(ft) = req.fallback_title.as_deref().map(str::trim).filter(|s| !s.is_empty())
                {
                    extracted.title = ft.to_string();
                }
            }
            if extracted.subtitle.is_none() {
                if let Some(fs) = req
                    .fallback_snippet
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    extracted.subtitle = Some(fs.to_string());
                }
            }
            if extracted.lead_image.is_none() {
                if let Some(fi) = req
                    .fallback_image
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    extracted.lead_image = Some(fi.to_string());
                }
            }
            if extracted.site_name.trim().is_empty() || extracted.site_name == extracted.domain {
                if let Some(fsrc) = req
                    .fallback_source
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    extracted.site_name = fsrc.to_string();
                }
            }

            // If remote returned a 200 OK bot-challenge shell with 0 article blocks and we have fallback metadata
            if extracted.blocks.is_empty() && has_fallback && looks_like_bot_gate_html(&body) {
                return Ok(build_blocked_fallback_article(&normalized_req_url, &req));
            }

            if extracted.blocks.is_empty() {
                if let Some(sub) = extracted.subtitle.clone().filter(|s| !s.trim().is_empty()) {
                    extracted.blocks.push(ContentBlock {
                        id: "rdr-1".to_string(),
                        block_type: "paragraph".to_string(),
                        content: sanitize_inline_rich_html(&sub),
                        meta: None,
                    });
                    extracted.word_count = compute_blocks_word_count(&extracted.blocks);
                }
            }

            Ok(extracted)
        }
        Ok(RemoteFetchResult::BlockedBySource) => {
            if has_fallback {
                Ok(build_blocked_fallback_article(&normalized_req_url, &req))
            } else {
                Err(AppError::BadRequest(
                    "Kaynak site otomatik okuyucu erişimini kısıtladı. Lütfen haberi orijinal sitede açın."
                        .to_string(),
                ))
            }
        }
        Err(err) => {
            // Preserve all SSRF / BadRequest errors strictly; only use fallback on transient network failures
            if matches!(err, AppError::BadRequest(_)) || !has_fallback {
                Err(err)
            } else {
                Ok(build_blocked_fallback_article(&normalized_req_url, &req))
            }
        }
    }
}

fn build_blocked_fallback_article(url: &str, req: &ReadArticleRequest) -> ExtractedReaderArticle {
    let domain = domain_from_url_str(url);
    let site_name = req
        .fallback_source
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| domain.clone());
    let title = req
        .fallback_title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| site_name.clone());
    let subtitle = req
        .fallback_snippet
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let lead_image = req
        .fallback_image
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let mut blocks = Vec::new();
    if let Some(ref snip) = subtitle {
        blocks.push(ContentBlock {
            id: "rdr-1".to_string(),
            block_type: "paragraph".to_string(),
            content: sanitize_inline_rich_html(snip),
            meta: None,
        });
    }
    let word_count = compute_blocks_word_count(&blocks);
    let key_facts = extract_key_facts_from_blocks(&blocks);

    ExtractedReaderArticle {
        url: url.to_string(),
        canonical_url: url.to_string(),
        domain,
        site_name,
        title,
        subtitle,
        author: None,
        published_at: None,
        lead_image,
        blocks,
        key_facts,
        media_embeds: Vec::new(),
        word_count,
        blocked_by_source: true,
    }
}

fn looks_like_bot_gate_html(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("cf-browser-verification")
        || lower.contains("enable javascript and cookies to continue")
        || lower.contains("attention required! | cloudflare")
        || lower.contains("just a moment...")
}

#[derive(Default)]
struct ExtractedMeta {
    og_title: Option<String>,
    twitter_title: Option<String>,
    doc_title: Option<String>,
    og_desc: Option<String>,
    meta_desc: Option<String>,
    twitter_desc: Option<String>,
    og_image: Option<String>,
    twitter_image: Option<String>,
    og_site_name: Option<String>,
    published_time: Option<String>,
    author: Option<String>,
    canonical: Option<String>,
}

#[derive(Default)]
struct LdJsonExtracted {
    headline: Option<String>,
    description: Option<String>,
    article_body: Option<String>,
    date_published: Option<String>,
    author_name: Option<String>,
    publisher_name: Option<String>,
    image_url: Option<String>,
    canonical_url: Option<String>,
}

/// Pure-Rust, UTF-8 safe News Readability & Metadata extractor.
pub fn parse_article_html_to_reader(url: &str, html: &str) -> ExtractedReaderArticle {
    let base_url = Url::parse(url).ok();
    let domain = base_url
        .as_ref()
        .and_then(|u| u.host_str())
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_string();

    // 1. OpenGraph / Meta / Canonical / <title>
    let meta = extract_head_metadata(html);

    // 2. <script type="application/ld+json">
    let ld = extract_ld_json_metadata(html);

    let canonical_url = meta
        .canonical
        .as_deref()
        .and_then(|c| resolve_relative_url(base_url.as_ref(), c))
        .or_else(|| {
            ld.canonical_url
                .as_deref()
                .and_then(|c| resolve_relative_url(base_url.as_ref(), c))
        })
        .unwrap_or_else(|| url.to_string());

    let site_name = first_nonempty(&[
        meta.og_site_name.as_deref(),
        ld.publisher_name.as_deref(),
    ])
    .unwrap_or_else(|| {
        if domain.is_empty() {
            "Haber Kaynağı".to_string()
        } else {
            domain.clone()
        }
    });

    let h1_fallback = extract_first_h1_text(html);
    let cleaned_doc_title = meta
        .doc_title
        .as_deref()
        .map(|t| strip_site_suffix(t, &site_name));

    let title = first_nonempty(&[
        meta.og_title.as_deref(),
        ld.headline.as_deref(),
        meta.twitter_title.as_deref(),
        h1_fallback.as_deref(),
        cleaned_doc_title.as_deref(),
    ])
    .unwrap_or_default();

    let subtitle = first_nonempty(&[
        meta.og_desc.as_deref(),
        ld.description.as_deref(),
        meta.meta_desc.as_deref(),
        meta.twitter_desc.as_deref(),
    ])
    .filter(|s| !titles_roughly_equal(s, &title));

    let author = first_nonempty(&[ld.author_name.as_deref(), meta.author.as_deref()])
        .filter(|a| !a.starts_with("http://") && !a.starts_with("https://"));

    let published_at = first_nonempty(&[
        meta.published_time.as_deref(),
        ld.date_published.as_deref(),
    ])
    .or_else(|| extract_first_time_datetime(html));

    let mut lead_image = first_nonempty(&[
        meta.og_image.as_deref(),
        ld.image_url.as_deref(),
        meta.twitter_image.as_deref(),
    ])
    .and_then(|img| resolve_relative_url(base_url.as_ref(), &img));

    // 3. Clean HTML body extraction
    let mut media_embeds: Vec<String> = Vec::new();
    let mut seen_embeds: HashSet<String> = HashSet::new();

    let stripped_doc = strip_inert_tags_and_comments(html);
    let candidate_region = select_best_article_region(&stripped_doc);
    let clean_region = strip_noise_containers(candidate_region);

    collect_links_and_iframe_embeds(
        &clean_region,
        base_url.as_ref(),
        &mut media_embeds,
        &mut seen_embeds,
    );

    let mut blocks = extract_blocks_from_region(
        &clean_region,
        base_url.as_ref(),
        &title,
        lead_image.as_deref(),
        &mut media_embeds,
        &mut seen_embeds,
    );

    // If article region was overly narrow (< 2 paragraphs) and we hadn't tried full <body>, try cleaned <body>
    if count_paragraph_blocks(&blocks) < 2 {
        if let Some(body_slice) = extract_tag_inner(&stripped_doc, "body") {
            if body_slice.len() != candidate_region.len() {
                let cleaned_body = strip_noise_containers(body_slice);
                let body_blocks = extract_blocks_from_region(
                    &cleaned_body,
                    base_url.as_ref(),
                    &title,
                    lead_image.as_deref(),
                    &mut media_embeds,
                    &mut seen_embeds,
                );
                if count_paragraph_blocks(&body_blocks) > count_paragraph_blocks(&blocks) {
                    collect_links_and_iframe_embeds(
                        &cleaned_body,
                        base_url.as_ref(),
                        &mut media_embeds,
                        &mut seen_embeds,
                    );
                    blocks = body_blocks;
                }
            }
        }
    }

    // Fallback to JSON-LD `articleBody` when HTML yielded < 2 paragraphs
    if count_paragraph_blocks(&blocks) < 2 {
        if let Some(ref raw_article_body) = ld.article_body {
            let ld_paras = split_ld_article_body(raw_article_body);
            if !ld_paras.is_empty() {
                // Keep non-paragraph rich blocks (images/embeds) and replace sparse paragraphs with full ld+json paragraphs
                let preserved_media: Vec<ContentBlock> = blocks
                    .into_iter()
                    .filter(|b| b.block_type == "image" || b.block_type == "embed")
                    .collect();
                let mut rebuilt = Vec::with_capacity(ld_paras.len() + preserved_media.len());
                let mut seq = 1usize;
                for p in ld_paras {
                    rebuilt.push(ContentBlock {
                        id: format!("rdr-{seq}"),
                        block_type: "paragraph".to_string(),
                        content: sanitize_inline_rich_html(&p),
                        meta: None,
                    });
                    seq += 1;
                }
                for mut m in preserved_media {
                    m.id = format!("rdr-{seq}");
                    seq += 1;
                    rebuilt.push(m);
                }
                blocks = rebuilt;
            }
        }
    }

    // Secondary fallback for legacy news templates using `<br><br>` inside article container without `<p>`
    if count_paragraph_blocks(&blocks) < 2 {
        let br_paras = extract_br_separated_paragraphs(&clean_region, base_url.as_ref());
        if br_paras.len() > count_paragraph_blocks(&blocks) {
            let preserved_non_para: Vec<ContentBlock> = blocks
                .into_iter()
                .filter(|b| b.block_type != "paragraph")
                .collect();
            let mut rebuilt = Vec::with_capacity(br_paras.len() + preserved_non_para.len());
            let mut seq = 1usize;
            for p in br_paras {
                rebuilt.push(ContentBlock {
                    id: format!("rdr-{seq}"),
                    block_type: "paragraph".to_string(),
                    content: p,
                    meta: None,
                });
                seq += 1;
            }
            for mut other in preserved_non_para {
                other.id = format!("rdr-{seq}");
                seq += 1;
                rebuilt.push(other);
            }
            blocks = rebuilt;
        }
    }

    if lead_image.is_none() {
        lead_image = blocks
            .iter()
            .find(|b| b.block_type == "image")
            .map(|b| b.content.clone());
    }

    media_embeds.truncate(5);
    let key_facts = extract_key_facts_from_blocks(&blocks);
    let word_count = compute_blocks_word_count(&blocks);

    ExtractedReaderArticle {
        url: url.to_string(),
        canonical_url,
        domain,
        site_name,
        title,
        subtitle,
        author,
        published_at,
        lead_image,
        blocks,
        key_facts,
        media_embeds,
        word_count,
        blocked_by_source: false,
    }
}

fn first_nonempty(candidates: &[Option<&str>]) -> Option<String> {
    for c in candidates.iter().flatten() {
        let clean = collapse_ws(c);
        if !clean.is_empty() {
            return Some(clean);
        }
    }
    None
}

fn strip_site_suffix(raw_title: &str, site_name: &str) -> String {
    let clean = collapse_ws(raw_title);
    for sep in [" | ", " - ", " – ", " — "] {
        if let Some((left, right)) = clean.rsplit_once(sep) {
            if left.chars().count() >= 18
                && (right.chars().count() <= 32
                    || (!site_name.is_empty()
                        && right.to_lowercase().contains(&site_name.to_lowercase())))
            {
                return left.trim().to_string();
            }
        }
    }
    clean
}

fn titles_roughly_equal(a: &str, b: &str) -> bool {
    let na = normalize_title_for_dedup(a);
    let nb = normalize_title_for_dedup(b);
    !na.is_empty() && na == nb
}

fn resolve_relative_url(base: Option<&Url>, raw: &str) -> Option<String> {
    let decoded = decode_html_entities(raw);
    let trimmed = decoded.trim();
    if trimmed.is_empty() || trimmed.starts_with("data:") || trimmed.starts_with("javascript:") {
        return None;
    }
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return Some(trimmed.to_string());
    }
    if trimmed.starts_with("//") {
        return Some(format!("https:{trimmed}"));
    }
    base.and_then(|b| b.join(trimmed).ok())
        .map(|u| u.to_string())
}

fn extract_head_metadata(html: &str) -> ExtractedMeta {
    let mut meta = ExtractedMeta::default();
    let lower = html.to_ascii_lowercase();

    // Limit metadata scan to first 256 KB (or up to </head> + small buffer) for speed
    let head_end = lower
        .find("</head>")
        .map(|i| (i + 7).min(html.len()))
        .unwrap_or_else(|| html.len().min(262_144));
    let head_html = &html[..head_end];
    let head_lower = &lower[..head_end];

    if let Some(t) = extract_tag_inner(head_html, "title") {
        let clean = collapse_ws(&strip_tags_raw(&decode_html_entities(t)));
        if !clean.is_empty() {
            meta.doc_title = Some(clean);
        }
    }

    // Scan <meta ...> tags
    let mut pos = 0usize;
    while let Some(rel) = head_lower[pos..].find("<meta") {
        let start = pos + rel;
        let after = start + 5;
        if !matches!(
            head_lower.as_bytes().get(after),
            Some(b' ' | b'\t' | b'\r' | b'\n' | b'/')
        ) {
            pos = after;
            continue;
        }
        let Some(gt_rel) = head_lower[after..].find('>') else {
            break;
        };
        let raw_attrs = &head_html[after..after + gt_rel];
        pos = after + gt_rel + 1;

        let Some(raw_content) = get_html_attr(raw_attrs, "content") else {
            continue;
        };
        let content = collapse_ws(&decode_html_entities(&raw_content));
        if content.is_empty() {
            continue;
        }

        let key = get_html_attr(raw_attrs, "property")
            .or_else(|| get_html_attr(raw_attrs, "name"))
            .or_else(|| get_html_attr(raw_attrs, "itemprop"))
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();

        match key.as_str() {
            "og:title" if meta.og_title.is_none() => meta.og_title = Some(content),
            "twitter:title" if meta.twitter_title.is_none() => meta.twitter_title = Some(content),
            "og:description" if meta.og_desc.is_none() => meta.og_desc = Some(content),
            "description" if meta.meta_desc.is_none() => meta.meta_desc = Some(content),
            "twitter:description" if meta.twitter_desc.is_none() => {
                meta.twitter_desc = Some(content);
            }
            "og:image:secure_url" => meta.og_image = Some(content),
            "og:image" | "og:image:url" if meta.og_image.is_none() => {
                meta.og_image = Some(content);
            }
            "twitter:image" | "twitter:image:src" if meta.twitter_image.is_none() => {
                meta.twitter_image = Some(content);
            }
            "og:site_name" | "application-name" if meta.og_site_name.is_none() => {
                meta.og_site_name = Some(content);
            }
            "article:published_time"
            | "og:published_time"
            | "pubdate"
            | "publishdate"
            | "datepublished"
                if meta.published_time.is_none() =>
            {
                meta.published_time = Some(content);
            }
            "author" | "article:author" | "byl" if meta.author.is_none() => {
                meta.author = Some(content);
            }
            _ => {}
        }
    }

    // Scan <link rel="canonical" href="...">
    let mut lpos = 0usize;
    while let Some(rel) = head_lower[lpos..].find("<link") {
        let start = lpos + rel;
        let after = start + 5;
        if !matches!(
            head_lower.as_bytes().get(after),
            Some(b' ' | b'\t' | b'\r' | b'\n')
        ) {
            lpos = after;
            continue;
        }
        let Some(gt_rel) = head_lower[after..].find('>') else {
            break;
        };
        let raw_attrs = &head_html[after..after + gt_rel];
        lpos = after + gt_rel + 1;

        if let Some(rel_val) = get_html_attr(raw_attrs, "rel") {
            if rel_val
                .split_whitespace()
                .any(|tok| tok.eq_ignore_ascii_case("canonical"))
            {
                if let Some(href) = get_html_attr(raw_attrs, "href") {
                    let clean_href = href.trim().to_string();
                    if !clean_href.is_empty() {
                        meta.canonical = Some(clean_href);
                        break;
                    }
                }
            }
        }
    }

    meta
}

fn extract_ld_json_metadata(html: &str) -> LdJsonExtracted {
    let mut out = LdJsonExtracted::default();
    let lower = html.to_ascii_lowercase();
    let mut pos = 0usize;

    while let Some(rel) = lower[pos..].find("<script") {
        let start = pos + rel;
        let after = start + 7;
        let Some(gt_rel) = lower[after..].find('>') else {
            break;
        };
        let attrs = &html[after..after + gt_rel];
        let content_start = after + gt_rel + 1;
        let Some(end_rel) = lower[content_start..].find("</script>") else {
            break;
        };
        let script_body = &html[content_start..content_start + end_rel];
        pos = content_start + end_rel + 9;

        let is_ld_json = get_html_attr(attrs, "type")
            .map(|t| t.to_ascii_lowercase().contains("ld+json"))
            .unwrap_or(false);
        if !is_ld_json {
            continue;
        }

        let cleaned_json = unwrap_cdata_sections(script_body)
            .trim()
            .trim_start_matches("<!--")
            .trim_end_matches("-->")
            .trim()
            .to_string();

        let parsed_val = serde_json::from_str::<Value>(&cleaned_json).or_else(|_| {
            // Tolerate raw unescaped newlines inside articleBody strings on quirky CMSes
            let sanitized_ctrl: String = cleaned_json
                .chars()
                .map(|c| match c {
                    '\r' => ' ',
                    '\t' => ' ',
                    _ => c,
                })
                .collect();
            serde_json::from_str::<Value>(&sanitized_ctrl)
        });

        if let Ok(val) = parsed_val {
            let mut article_objs = Vec::new();
            collect_ld_article_objects(&val, &mut article_objs);
            for obj in article_objs {
                populate_from_ld_object(obj, &mut out);
            }
        }
    }

    out
}

fn collect_ld_article_objects<'a>(
    val: &'a Value,
    out: &mut Vec<&'a serde_json::Map<String, Value>>,
) {
    match val {
        Value::Array(arr) => {
            for item in arr {
                collect_ld_article_objects(item, out);
            }
        }
        Value::Object(map) => {
            if let Some(graph) = map.get("@graph") {
                collect_ld_article_objects(graph, out);
            }
            if is_ld_article_type(map.get("@type")) {
                out.push(map);
            }
        }
        _ => {}
    }
}

fn is_ld_article_type(type_val: Option<&Value>) -> bool {
    let matches_str = |s: &str| {
        let l = s.to_ascii_lowercase();
        l.contains("newsarticle")
            || l == "article"
            || l.ends_with(":article")
            || l.contains("reportagenewsarticle")
            || l.contains("analysisnewsarticle")
            || l.contains("blogposting")
            || l.contains("liveblogposting")
    };
    match type_val {
        Some(Value::String(s)) => matches_str(s),
        Some(Value::Array(arr)) => arr.iter().filter_map(Value::as_str).any(matches_str),
        _ => false,
    }
}

fn populate_from_ld_object(map: &serde_json::Map<String, Value>, out: &mut LdJsonExtracted) {
    if out.headline.is_none() {
        out.headline = map
            .get("headline")
            .or_else(|| map.get("name"))
            .and_then(Value::as_str)
            .map(|s| collapse_ws(&decode_html_entities(s)))
            .filter(|s| !s.is_empty());
    }
    if out.description.is_none() {
        out.description = map
            .get("description")
            .and_then(Value::as_str)
            .map(|s| collapse_ws(&decode_html_entities(s)))
            .filter(|s| !s.is_empty());
    }
    if out.article_body.is_none() {
        out.article_body = map
            .get("articleBody")
            .and_then(Value::as_str)
            .map(decode_html_entities)
            .filter(|s| !s.trim().is_empty());
    }
    if out.date_published.is_none() {
        out.date_published = map
            .get("datePublished")
            .or_else(|| map.get("dateCreated"))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    if out.author_name.is_none() {
        out.author_name = extract_ld_person_or_org_name(map.get("author"));
    }
    if out.publisher_name.is_none() {
        out.publisher_name = extract_ld_person_or_org_name(map.get("publisher"));
    }
    if out.image_url.is_none() {
        out.image_url = extract_ld_image_url(map.get("image"));
    }
    if out.canonical_url.is_none() {
        out.canonical_url = match map.get("mainEntityOfPage") {
            Some(Value::String(s)) if s.starts_with("http") => Some(s.clone()),
            Some(Value::Object(o)) => o
                .get("@id")
                .or_else(|| o.get("url"))
                .and_then(Value::as_str)
                .filter(|s| s.starts_with("http"))
                .map(str::to_string),
            _ => None,
        };
    }
}

fn extract_ld_person_or_org_name(val: Option<&Value>) -> Option<String> {
    match val? {
        Value::String(s) => {
            let clean = collapse_ws(&decode_html_entities(s));
            (!clean.is_empty()).then_some(clean)
        }
        Value::Object(o) => o
            .get("name")
            .and_then(Value::as_str)
            .map(|s| collapse_ws(&decode_html_entities(s)))
            .filter(|s| !s.is_empty()),
        Value::Array(arr) => {
            let names: Vec<String> = arr
                .iter()
                .filter_map(|v| extract_ld_person_or_org_name(Some(v)))
                .collect();
            (!names.is_empty()).then(|| names.join(", "))
        }
        _ => None,
    }
}

fn extract_ld_image_url(val: Option<&Value>) -> Option<String> {
    match val? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Object(o) => o
            .get("url")
            .or_else(|| o.get("contentUrl"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        Value::Array(arr) => arr.iter().find_map(|v| extract_ld_image_url(Some(v))),
        _ => None,
    }
}

/// Removes HTML comments and `<script>`, `<style>`, `<noscript>`, `<svg>`, `<template>` blocks safely.
fn strip_inert_tags_and_comments(html: &str) -> String {
    let mut without_comments = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(c_start) = rest.find("<!--") {
        without_comments.push_str(&rest[..c_start]);
        let after = &rest[c_start + 4..];
        if let Some(c_end) = after.find("-->") {
            rest = &after[c_end + 3..];
        } else {
            rest = "";
            break;
        }
    }
    without_comments.push_str(rest);

    let mut cur = without_comments;
    for tag in ["script", "style", "noscript", "svg", "template"] {
        cur = remove_all_tag_blocks(&cur, tag);
    }
    cur
}

fn remove_all_tag_blocks(html: &str, tag_lower: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let open_pat = format!("<{tag_lower}");
    let close_pat = format!("</{tag_lower}>");
    if !lower.contains(&open_pat) {
        return html.to_string();
    }

    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;

    while let Some(rel) = lower[cursor..].find(&open_pat) {
        let start = cursor + rel;
        let after_name = start + open_pat.len();
        let next_b = lower.as_bytes().get(after_name).copied();
        if !matches!(next_b, Some(b'>' | b' ' | b'\t' | b'\r' | b'\n' | b'/')) {
            out.push_str(&html[cursor..after_name]);
            cursor = after_name;
            continue;
        }
        out.push_str(&html[cursor..start]);
        let Some(gt_rel) = lower[after_name..].find('>') else {
            return out;
        };
        let gt_abs = after_name + gt_rel;
        if lower.as_bytes().get(gt_abs.saturating_sub(1)) == Some(&b'/') {
            cursor = gt_abs + 1;
            continue;
        }
        if let Some(end_rel) = lower[gt_abs + 1..].find(&close_pat) {
            cursor = gt_abs + 1 + end_rel + close_pat.len();
        } else {
            return out;
        }
    }
    out.push_str(&html[cursor..]);
    out
}

/// Selects the best `<article>` or semantic news content container, falling back to `<body>`.
fn select_best_article_region(html: &str) -> &str {
    let lower = html.to_ascii_lowercase();

    // 1. Check all `<article>...</article>` candidates and pick the richest one
    let mut best_article: Option<(&str, usize)> = None;
    let mut pos = 0usize;
    while let Some(rel) = lower[pos..].find("<article") {
        let start = pos + rel;
        let after = start + 8;
        if !matches!(
            lower.as_bytes().get(after),
            Some(b'>' | b' ' | b'\t' | b'\r' | b'\n')
        ) {
            pos = after;
            continue;
        }
        if let Some((inner, next_pos)) = find_balanced_element_inner(html, &lower, start, "article")
        {
            let score = plain_text_len_estimate(inner);
            if best_article.map(|(_, bs)| score > bs).unwrap_or(true) {
                best_article = Some((inner, score));
            }
            pos = next_pos;
        } else {
            pos = after;
        }
    }
    if let Some((art_html, score)) = best_article {
        if score >= 140 {
            return art_html;
        }
    }

    // 2. Semantic news detail containers (`itemprop="articleBody"`, `article-body`, `haber-detay`, etc.)
    let mut best_div: Option<(&str, usize)> = None;
    for tag in ["div", "section", "main"] {
        let pat = format!("<{tag}");
        let mut dpos = 0usize;
        while let Some(rel) = lower[dpos..].find(&pat) {
            let start = dpos + rel;
            let after = start + pat.len();
            if !matches!(
                lower.as_bytes().get(after),
                Some(b'>' | b' ' | b'\t' | b'\r' | b'\n')
            ) {
                dpos = after;
                continue;
            }
            let Some(gt_rel) = lower[after..].find('>') else {
                break;
            };
            let attrs_lower = &lower[after..after + gt_rel];
            let is_candidate = tag == "main"
                || attrs_lower.contains("articlebody")
                || attrs_lower.contains("article-body")
                || attrs_lower.contains("article_body")
                || attrs_lower.contains("article-content")
                || attrs_lower.contains("news-detail")
                || attrs_lower.contains("haber-detay")
                || attrs_lower.contains("haber_metni")
                || attrs_lower.contains("story-body")
                || attrs_lower.contains("entry-content")
                || attrs_lower.contains("post-content");

            if is_candidate && !has_noise_attribute(attrs_lower) {
                if let Some((inner, _)) = find_balanced_element_inner(html, &lower, start, tag) {
                    let bonus = if attrs_lower.contains("articlebody")
                        || attrs_lower.contains("haber-detay")
                        || attrs_lower.contains("article-body")
                    {
                        500
                    } else {
                        0
                    };
                    let score = plain_text_len_estimate(inner) + bonus;
                    if plain_text_len_estimate(inner) >= 140
                        && best_div.map(|(_, bs)| score > bs).unwrap_or(true)
                    {
                        best_div = Some((inner, score));
                    }
                }
            }
            dpos = after + gt_rel + 1;
        }
    }
    if let Some((div_html, _)) = best_div {
        return div_html;
    }

    extract_tag_inner(html, "body").unwrap_or(html)
}

fn plain_text_len_estimate(html: &str) -> usize {
    let mut count = 0usize;
    let mut in_tag = false;
    for b in html.bytes() {
        match b {
            b'<' => in_tag = true,
            b'>' => in_tag = false,
            b' ' | b'\n' | b'\r' | b'\t' => {}
            _ if !in_tag => count += 1,
            _ => {}
        }
    }
    count
}

/// Finds the balanced inner slice `(&html[inner_start..inner_end], close_end)` of `<tag ...>...</tag>`.
fn find_balanced_element_inner<'a>(
    html: &'a str,
    lower: &str,
    open_start: usize,
    tag_lower: &str,
) -> Option<(&'a str, usize)> {
    let gt_rel = lower[open_start..].find('>')?;
    let gt_abs = open_start + gt_rel;
    if lower.as_bytes().get(gt_abs.saturating_sub(1)) == Some(&b'/') {
        return Some(("", gt_abs + 1));
    }
    let inner_start = gt_abs + 1;
    let open_pat = format!("<{tag_lower}");
    let close_pat = format!("</{tag_lower}");

    let mut depth = 1usize;
    let mut scan = inner_start;

    while depth > 0 && scan < lower.len() {
        let next_open = lower[scan..].find(&open_pat).map(|i| scan + i);
        let next_close = lower[scan..].find(&close_pat).map(|i| scan + i)?;

        if let Some(op) = next_open {
            if op < next_close {
                let after_op = op + open_pat.len();
                let valid_open = matches!(
                    lower.as_bytes().get(after_op),
                    Some(b'>' | b' ' | b'\t' | b'\r' | b'\n' | b'/')
                );
                if valid_open {
                    if let Some(op_gt_rel) = lower[after_op..].find('>') {
                        let op_gt = after_op + op_gt_rel;
                        if lower.as_bytes().get(op_gt.saturating_sub(1)) != Some(&b'/') {
                            depth += 1;
                        }
                        scan = op_gt + 1;
                        continue;
                    }
                }
                scan = after_op;
                continue;
            }
        }

        let after_cl = next_close + close_pat.len();
        let valid_close = matches!(
            lower.as_bytes().get(after_cl),
            Some(b'>' | b' ' | b'\t' | b'\r' | b'\n')
        );
        if valid_close {
            let cl_gt_rel = lower[after_cl..].find('>')?;
            let cl_end = after_cl + cl_gt_rel + 1;
            depth -= 1;
            if depth == 0 {
                return Some((&html[inner_start..next_close], cl_end));
            }
            scan = cl_end;
        } else {
            scan = after_cl;
        }
    }

    None
}

/// Strips semantic noise tags (`nav`, `header`, `footer`, `aside`, `form`, `button`) and elements whose
/// `class` or `id` indicates ads, related news, cookie banners, comments, share bars, or sidebars.
fn strip_noise_containers(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;

    while let Some(lt_rel) = lower[cursor..].find('<') {
        let lt = cursor + lt_rel;
        out.push_str(&html[cursor..lt]);

        let rest_lower = &lower[lt + 1..];
        if rest_lower.starts_with('/') || rest_lower.starts_with('!') || rest_lower.starts_with('?')
        {
            out.push('<');
            cursor = lt + 1;
            continue;
        }

        let name_len = rest_lower
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'-')
            .count();
        if name_len == 0 {
            out.push('<');
            cursor = lt + 1;
            continue;
        }
        let tag_name = &rest_lower[..name_len];
        let Some(gt_rel) = rest_lower[name_len..].find('>') else {
            out.push_str(&html[lt..]);
            return out;
        };
        let attrs_lower = &rest_lower[name_len..name_len + gt_rel];
        let tag_end = lt + 1 + name_len + gt_rel + 1;
        let is_self_closing = attrs_lower.trim_end().ends_with('/')
            || matches!(tag_name, "img" | "br" | "hr" | "meta" | "link" | "input" | "source");

        let is_noise_tag = matches!(
            tag_name,
            "nav" | "header" | "footer" | "aside" | "form" | "button" | "select" | "dialog"
        );
        let is_noise_box = !matches!(tag_name, "article" | "main" | "body")
            && has_noise_attribute(attrs_lower);

        if is_noise_tag || is_noise_box {
            if is_self_closing {
                cursor = tag_end;
                continue;
            }
            if let Some((_, close_end)) = find_balanced_element_inner(html, &lower, lt, tag_name) {
                cursor = close_end;
                continue;
            }
        }

        out.push_str(&html[lt..tag_end]);
        cursor = tag_end;
    }

    out.push_str(&html[cursor..]);
    out
}

fn has_noise_attribute(attrs_lower: &str) -> bool {
    let mut combined = String::new();
    for attr in ["class", "id", "role", "data-component", "data-testid"] {
        if let Some(val) = get_html_attr(attrs_lower, attr) {
            combined.push(' ');
            combined.push_str(&val);
        }
    }
    if combined.is_empty() {
        return false;
    }

    const SUBSTRING_NOISE: &[&str] = &[
        "cookie",
        "consent",
        "kvkk",
        "advert",
        "adsense",
        "reklam",
        "taboola",
        "outbrain",
        "sidebar",
        "related",
        "ilgili-haber",
        "ilgili_haber",
        "benzer-haber",
        "benzer_haber",
        "cok-okunan",
        "most-read",
        "comments",
        "comment-list",
        "yorumlar",
        "newsletter",
        "breadcrumb",
        "social-share",
        "share-bar",
        "share-buttons",
        "paylas",
        "footer",
        "popup",
        "modal",
        "widget",
        "etiketler",
        "tag-list",
        "tags-list",
    ];

    if SUBSTRING_NOISE.iter().any(|pat| combined.contains(pat)) {
        return true;
    }

    // Token-exact noise checks so words like "lead" or "header-image" or "thread" are NOT falsely matched
    for token in combined.split(|c: char| !c.is_ascii_alphanumeric()) {
        if matches!(
            token,
            "ad" | "ads" | "dfp" | "menu" | "nav" | "navbar" | "sponsor" | "sponsored" | "promo"
        ) {
            return true;
        }
    }

    false
}

fn collect_links_and_iframe_embeds(
    html: &str,
    base_url: Option<&Url>,
    media_embeds: &mut Vec<String>,
    seen_embeds: &mut HashSet<String>,
) {
    let lower = html.to_ascii_lowercase();

    // 1. Scan <iframe ...>
    let mut ipos = 0usize;
    while let Some(rel) = lower[ipos..].find("<iframe") {
        let start = ipos + rel;
        let after = start + 7;
        let Some(gt_rel) = lower[after..].find('>') else {
            break;
        };
        let attrs = &html[after..after + gt_rel];
        ipos = after + gt_rel + 1;

        if let Some(src) = get_html_attr(attrs, "data-src").or_else(|| get_html_attr(attrs, "src"))
        {
            if let Some(resolved) = resolve_relative_url(base_url, &src) {
                if let Some(canon) = normalize_youtube_or_twitter_url(&resolved) {
                    if seen_embeds.insert(canon.clone()) && media_embeds.len() < 5 {
                        media_embeds.push(canon);
                    }
                }
            }
        }
    }

    // 2. Scan <a href="..."> for YouTube watch/embed or X/Twitter /status/ links
    let mut apos = 0usize;
    while let Some(rel) = lower[apos..].find("<a") {
        let start = apos + rel;
        let after = start + 2;
        if !matches!(
            lower.as_bytes().get(after),
            Some(b' ' | b'\t' | b'\r' | b'\n')
        ) {
            apos = after;
            continue;
        }
        let Some(gt_rel) = lower[after..].find('>') else {
            break;
        };
        let attrs = &html[after..after + gt_rel];
        apos = after + gt_rel + 1;

        if let Some(href) = get_html_attr(attrs, "href") {
            if let Some(resolved) = resolve_relative_url(base_url, &href) {
                if let Some(canon) = normalize_youtube_or_twitter_url(&resolved) {
                    if seen_embeds.insert(canon.clone()) && media_embeds.len() < 5 {
                        media_embeds.push(canon);
                    }
                }
            }
        }
    }
}

fn normalize_youtube_or_twitter_url(raw_url: &str) -> Option<String> {
    let parsed = Url::parse(raw_url.trim()).ok()?;
    let host = parsed
        .host_str()?
        .trim_start_matches("www.")
        .trim_start_matches("mobile.")
        .to_ascii_lowercase();
    let path = parsed.path();

    // YouTube watch?v= or embed/ or youtu.be/
    if host == "youtube.com" || host == "m.youtube.com" || host == "youtube-nocookie.com" {
        if path == "/watch" {
            for (k, v) in parsed.query_pairs() {
                if k == "v" && !v.trim().is_empty() {
                    return Some(format!("https://www.youtube.com/watch?v={}", v.trim()));
                }
            }
        }
        if let Some(vid) = path
            .strip_prefix("/embed/")
            .or_else(|| path.strip_prefix("/shorts/"))
        {
            let clean_id = vid.split(['/', '?']).next().unwrap_or("").trim();
            if !clean_id.is_empty() && clean_id != "videoseries" {
                return Some(format!("https://www.youtube.com/embed/{clean_id}"));
            }
        }
    }
    if host == "youtu.be" {
        let clean_id = path.trim_matches('/').split('/').next().unwrap_or("").trim();
        if !clean_id.is_empty() {
            return Some(format!("https://www.youtube.com/watch?v={clean_id}"));
        }
    }

    // X / Twitter /status/<id>
    if host == "twitter.com" || host == "x.com" {
        let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segs.len() >= 3 && segs[1].eq_ignore_ascii_case("status") {
            let status_id = segs[2];
            if !status_id.is_empty() && status_id.chars().all(|c| c.is_ascii_digit()) {
                return Some(format!("https://x.com/{}/status/{status_id}", segs[0]));
            }
        }
        // platform.twitter.com/embed/Tweet.html?id=...
    }
    if host == "platform.twitter.com" && path.to_ascii_lowercase().contains("tweet") {
        for (k, v) in parsed.query_pairs() {
            if k == "id" && !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()) {
                return Some(format!("https://x.com/i/web/status/{v}"));
            }
        }
    }

    None
}

fn is_supported_iframe_embed(url: &str) -> Option<String> {
    if let Some(yt_tw) = normalize_youtube_or_twitter_url(url) {
        return Some(yt_tw);
    }
    let parsed = Url::parse(url.trim()).ok()?;
    let host = parsed
        .host_str()?
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    if host.contains("vimeo.com")
        || host.contains("dailymotion.com")
        || host.contains("instagram.com")
    {
        return Some(parsed.to_string());
    }
    None
}

fn extract_blocks_from_region(
    html: &str,
    base_url: Option<&Url>,
    article_title: &str,
    lead_image: Option<&str>,
    media_embeds: &mut Vec<String>,
    seen_embeds: &mut HashSet<String>,
) -> Vec<ContentBlock> {
    let lower = html.to_ascii_lowercase();
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut seen_images: HashSet<String> = HashSet::new();
    if let Some(lead) = lead_image {
        seen_images.insert(normalize_url_for_dedup(lead));
    }

    let mut seq = 1usize;
    let mut next_id = || {
        let id = format!("rdr-{seq}");
        seq += 1;
        id
    };

    let mut cursor = 0usize;
    while let Some(lt_rel) = lower[cursor..].find('<') {
        let lt = cursor + lt_rel;
        let rest_lower = &lower[lt + 1..];
        if rest_lower.starts_with('/') || rest_lower.starts_with('!') || rest_lower.starts_with('?')
        {
            cursor = lt + 1;
            continue;
        }

        let name_len = rest_lower
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'-')
            .count();
        if name_len == 0 {
            cursor = lt + 1;
            continue;
        }

        let tag_name = &rest_lower[..name_len];
        let Some(gt_rel) = rest_lower[name_len..].find('>') else {
            break;
        };
        let raw_attrs = &html[lt + 1 + name_len..lt + 1 + name_len + gt_rel];
        let tag_end = lt + 1 + name_len + gt_rel + 1;

        match tag_name {
            "h2" | "h3" | "h4" => {
                if let Some((inner, close_end)) =
                    find_balanced_element_inner(html, &lower, lt, tag_name)
                {
                    let heading_text = collapse_ws(&strip_tags_raw(&decode_html_entities(inner)));
                    if heading_text.chars().count() >= 3
                        && !titles_roughly_equal(&heading_text, article_title)
                        && !is_boilerplate_heading(&heading_text)
                    {
                        let block_type = if tag_name == "h2" {
                            "heading"
                        } else {
                            "heading3"
                        };
                        blocks.push(ContentBlock {
                            id: next_id(),
                            block_type: block_type.to_string(),
                            content: heading_text,
                            meta: None,
                        });
                    }
                    cursor = close_end;
                    continue;
                }
            }
            "blockquote" => {
                if let Some((inner, close_end)) =
                    find_balanced_element_inner(html, &lower, lt, "blockquote")
                {
                    let class_lower = get_html_attr(raw_attrs, "class")
                        .unwrap_or_default()
                        .to_ascii_lowercase();

                    if class_lower.contains("twitter-tweet")
                        || class_lower.contains("instagram-media")
                    {
                        if let Some(embed_url) =
                            extract_social_blockquote_url(inner, base_url)
                        {
                            if let Some(canon) = normalize_youtube_or_twitter_url(&embed_url) {
                                if seen_embeds.insert(canon.clone()) && media_embeds.len() < 5 {
                                    media_embeds.push(canon.clone());
                                }
                                blocks.push(ContentBlock {
                                    id: next_id(),
                                    block_type: "embed".to_string(),
                                    content: canon,
                                    meta: None,
                                });
                            } else {
                                blocks.push(ContentBlock {
                                    id: next_id(),
                                    block_type: "embed".to_string(),
                                    content: embed_url,
                                    meta: None,
                                });
                            }
                            cursor = close_end;
                            continue;
                        }
                    }

                    let cite = extract_tag_inner(inner, "cite")
                        .map(|c| collapse_ws(&strip_tags_raw(&decode_html_entities(c))))
                        .filter(|c| !c.is_empty());
                    let inner_without_cite = remove_all_tag_blocks(inner, "cite");
                    let clean_quote =
                        clean_inline_fragment(&inner_without_cite, base_url);
                    let plain_quote = collapse_ws(&strip_all_html_tags(&clean_quote));
                    if plain_quote.chars().count() >= 8 {
                        blocks.push(ContentBlock {
                            id: next_id(),
                            block_type: "quote".to_string(),
                            content: clean_quote,
                            meta: cite,
                        });
                    }
                    cursor = close_end;
                    continue;
                }
            }
            "ul" | "ol" => {
                if let Some((inner, close_end)) =
                    find_balanced_element_inner(html, &lower, lt, tag_name)
                {
                    let items = extract_clean_list_items(inner, base_url);
                    if !items.is_empty() {
                        let block_type = if tag_name == "ol" {
                            "numbered_list"
                        } else {
                            "bullet_list"
                        };
                        blocks.push(ContentBlock {
                            id: next_id(),
                            block_type: block_type.to_string(),
                            content: items.join("\n"),
                            meta: None,
                        });
                    }
                    cursor = close_end;
                    continue;
                }
            }
            "figure" => {
                if let Some((inner, close_end)) =
                    find_balanced_element_inner(html, &lower, lt, "figure")
                {
                    // Check if figure wraps an iframe embed first
                    if let Some(iframe_url) = extract_first_iframe_embed(inner, base_url) {
                        blocks.push(ContentBlock {
                            id: next_id(),
                            block_type: "embed".to_string(),
                            content: iframe_url,
                            meta: None,
                        });
                        cursor = close_end;
                        continue;
                    }

                    let figcap = extract_tag_inner(inner, "figcaption")
                        .map(|c| collapse_ws(&strip_tags_raw(&decode_html_entities(c))))
                        .filter(|c| !c.is_empty() && c.chars().count() <= 300);

                    if let Some((img_url, alt_opt)) = extract_first_valid_img(inner, base_url) {
                        let norm = normalize_url_for_dedup(&img_url);
                        if seen_images.insert(norm) {
                            blocks.push(ContentBlock {
                                id: next_id(),
                                block_type: "image".to_string(),
                                content: img_url,
                                meta: figcap.or(alt_opt),
                            });
                        }
                    }
                    cursor = close_end;
                    continue;
                }
            }
            "img" => {
                if let Some((img_url, alt_opt)) =
                    parse_img_attrs_if_valid(raw_attrs, base_url)
                {
                    let norm = normalize_url_for_dedup(&img_url);
                    if seen_images.insert(norm) {
                        blocks.push(ContentBlock {
                            id: next_id(),
                            block_type: "image".to_string(),
                            content: img_url,
                            meta: alt_opt,
                        });
                    }
                }
                cursor = tag_end;
                continue;
            }
            "iframe" => {
                if let Some(raw_src) =
                    get_html_attr(raw_attrs, "data-src").or_else(|| get_html_attr(raw_attrs, "src"))
                {
                    if let Some(resolved) = resolve_relative_url(base_url, &raw_src) {
                        if let Some(embed_url) = is_supported_iframe_embed(&resolved) {
                            if let Some(canon) = normalize_youtube_or_twitter_url(&embed_url) {
                                if seen_embeds.insert(canon.clone()) && media_embeds.len() < 5 {
                                    media_embeds.push(canon);
                                }
                            }
                            blocks.push(ContentBlock {
                                id: next_id(),
                                block_type: "embed".to_string(),
                                content: embed_url,
                                meta: None,
                            });
                        }
                    }
                }
                if let Some((_, close_end)) =
                    find_balanced_element_inner(html, &lower, lt, "iframe")
                {
                    cursor = close_end;
                } else {
                    cursor = tag_end;
                }
                continue;
            }
            "p" => {
                if let Some((inner, close_end)) =
                    find_balanced_element_inner(html, &lower, lt, "p")
                {
                    // Capture any inline image inside <p>
                    if let Some((img_url, alt_opt)) = extract_first_valid_img(inner, base_url) {
                        let norm = normalize_url_for_dedup(&img_url);
                        if seen_images.insert(norm) {
                            blocks.push(ContentBlock {
                                id: next_id(),
                                block_type: "image".to_string(),
                                content: img_url,
                                meta: alt_opt,
                            });
                        }
                    }

                    let clean_html = clean_inline_fragment(inner, base_url);
                    let plain = collapse_ws(&strip_all_html_tags(&clean_html));
                    if plain.chars().count() >= 25 && !is_boilerplate_paragraph(&plain) {
                        let is_dup = blocks
                            .last()
                            .map(|prev| {
                                prev.block_type == "paragraph"
                                    && collapse_ws(&strip_all_html_tags(&prev.content)) == plain
                            })
                            .unwrap_or(false);
                        if !is_dup {
                            blocks.push(ContentBlock {
                                id: next_id(),
                                block_type: "paragraph".to_string(),
                                content: clean_html,
                                meta: None,
                            });
                        }
                    }
                    cursor = close_end;
                    continue;
                }
            }
            _ => {}
        }

        cursor = tag_end;
    }

    blocks
}

fn clean_inline_fragment(fragment_html: &str, base_url: Option<&Url>) -> String {
    let resolved_links = rewrite_relative_anchor_hrefs(fragment_html, base_url);
    let decoded = decode_html_entities(&resolved_links);
    collapse_inline_ws(&sanitize_inline_rich_html(&decoded))
}

fn rewrite_relative_anchor_hrefs(html: &str, base_url: Option<&Url>) -> String {
    let Some(base) = base_url else {
        return html.to_string();
    };
    let lower = html.to_ascii_lowercase();
    if !lower.contains("<a") {
        return html.to_string();
    }
    let mut out = String::with_capacity(html.len() + 32);
    let mut cursor = 0usize;
    while let Some(rel) = lower[cursor..].find("<a") {
        let start = cursor + rel;
        let after = start + 2;
        if !matches!(
            lower.as_bytes().get(after),
            Some(b' ' | b'\t' | b'\r' | b'\n')
        ) {
            out.push_str(&html[cursor..after]);
            cursor = after;
            continue;
        }
        let Some(gt_rel) = lower[after..].find('>') else {
            break;
        };
        out.push_str(&html[cursor..start]);
        let attrs = &html[after..after + gt_rel];
        if let Some(href) = get_html_attr(attrs, "href") {
            if let Some(resolved) = resolve_relative_url(Some(base), &href) {
                out.push_str(&format!("<a href=\"{}\">", escape_attr(&resolved)));
            } else {
                out.push_str(&html[start..after + gt_rel + 1]);
            }
        } else {
            out.push_str(&html[start..after + gt_rel + 1]);
        }
        cursor = after + gt_rel + 1;
    }
    out.push_str(&html[cursor..]);
    out
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}

fn extract_social_blockquote_url(inner: &str, base_url: Option<&Url>) -> Option<String> {
    let lower = inner.to_ascii_lowercase();
    let mut pos = 0usize;
    let mut fallback_social = None;
    while let Some(rel) = lower[pos..].find("<a") {
        let start = pos + rel;
        let after = start + 2;
        let Some(gt_rel) = lower[after..].find('>') else {
            break;
        };
        let attrs = &inner[after..after + gt_rel];
        pos = after + gt_rel + 1;
        if let Some(href) = get_html_attr(attrs, "href") {
            if let Some(resolved) = resolve_relative_url(base_url, &href) {
                if resolved.contains("/status/")
                    || resolved.contains("instagram.com/p/")
                    || resolved.contains("instagram.com/reel/")
                {
                    return Some(resolved);
                }
                if resolved.contains("twitter.com/")
                    || resolved.contains("x.com/")
                    || resolved.contains("instagram.com/")
                {
                    fallback_social = Some(resolved);
                }
            }
        }
    }
    fallback_social
}

fn extract_first_iframe_embed(inner: &str, base_url: Option<&Url>) -> Option<String> {
    let lower = inner.to_ascii_lowercase();
    let rel = lower.find("<iframe")?;
    let after = rel + 7;
    let gt_rel = lower[after..].find('>')?;
    let attrs = &inner[after..after + gt_rel];
    let src = get_html_attr(attrs, "data-src").or_else(|| get_html_attr(attrs, "src"))?;
    let resolved = resolve_relative_url(base_url, &src)?;
    is_supported_iframe_embed(&resolved)
}

fn extract_first_valid_img(inner: &str, base_url: Option<&Url>) -> Option<(String, Option<String>)> {
    let lower = inner.to_ascii_lowercase();
    let mut pos = 0usize;
    while let Some(rel) = lower[pos..].find("<img") {
        let start = pos + rel;
        let after = start + 4;
        let Some(gt_rel) = lower[after..].find('>') else {
            break;
        };
        let attrs = &inner[after..after + gt_rel];
        pos = after + gt_rel + 1;
        if let Some(valid) = parse_img_attrs_if_valid(attrs, base_url) {
            return Some(valid);
        }
    }
    None
}

fn parse_img_attrs_if_valid(
    raw_attrs: &str,
    base_url: Option<&Url>,
) -> Option<(String, Option<String>)> {
    // Reject tiny explicit pixel/icon dimensions
    for dim_attr in ["width", "height"] {
        if let Some(v) = get_html_attr(raw_attrs, dim_attr) {
            let digits: String = v.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(px) = digits.parse::<u32>() {
                if px > 0 && px < 80 {
                    return None;
                }
            }
        }
    }

    let class_id = format!(
        "{} {}",
        get_html_attr(raw_attrs, "class").unwrap_or_default(),
        get_html_attr(raw_attrs, "id").unwrap_or_default()
    )
    .to_ascii_lowercase();
    if class_id.contains("avatar")
        || class_id.contains("logo")
        || class_id.contains("icon")
        || class_id.contains("emoji")
    {
        return None;
    }

    // Prefer lazy-loaded high-res attributes over placeholder `src`
    let candidates = [
        get_html_attr(raw_attrs, "data-src"),
        get_html_attr(raw_attrs, "data-original"),
        get_html_attr(raw_attrs, "data-lazy-src"),
        get_html_attr(raw_attrs, "data-url"),
        get_html_attr(raw_attrs, "data-srcset").and_then(|s| pick_best_srcset_url(&s)),
        get_html_attr(raw_attrs, "srcset").and_then(|s| pick_best_srcset_url(&s)),
        get_html_attr(raw_attrs, "src"),
    ];

    let mut chosen_url = None;
    for cand in candidates.into_iter().flatten() {
        let trimmed = cand.trim();
        if trimmed.is_empty() || trimmed.starts_with("data:") {
            continue;
        }
        if let Some(resolved) = resolve_relative_url(base_url, trimmed) {
            if !is_junk_image_url(&resolved) {
                chosen_url = Some(resolved);
                break;
            }
        }
    }

    let final_url = chosen_url?;
    let alt = get_html_attr(raw_attrs, "alt")
        .map(|a| collapse_ws(&decode_html_entities(&a)))
        .filter(|a| a.chars().count() >= 4 && a.chars().count() <= 240);

    Some((final_url, alt))
}

fn pick_best_srcset_url(srcset: &str) -> Option<String> {
    let mut best: Option<(u32, String)> = None;
    for part in srcset.split(',') {
        let mut iter = part.split_whitespace();
        let Some(url_part) = iter.next() else {
            continue;
        };
        if url_part.starts_with("data:") {
            continue;
        }
        let weight = iter
            .next()
            .and_then(|w| w.trim_end_matches(['w', 'x']).parse::<u32>().ok())
            .unwrap_or(1);
        if best.as_ref().map(|(bw, _)| weight >= *bw).unwrap_or(true) {
            best = Some((weight, url_part.to_string()));
        }
    }
    best.map(|(_, u)| u)
}

fn is_junk_image_url(url: &str) -> bool {
    let l = url.to_ascii_lowercase();
    let path_part = l.split('?').next().unwrap_or(&l);
    path_part.ends_with(".svg")
        || l.contains("1x1")
        || l.contains("pixel")
        || l.contains("spacer")
        || l.contains("blank.")
        || l.contains("placeholder")
        || l.contains("/logo")
        || l.contains("-logo.")
        || l.contains("_logo.")
        || l.contains("/icons/")
        || l.contains("/avatar")
        || l.contains("google-news")
        || l.contains("abone-ol")
}

fn extract_clean_list_items(list_inner: &str, base_url: Option<&Url>) -> Vec<String> {
    let lower = list_inner.to_ascii_lowercase();
    let mut items = Vec::new();
    let mut pos = 0usize;

    while let Some(rel) = lower[pos..].find("<li") {
        let start = pos + rel;
        let after = start + 3;
        if !matches!(
            lower.as_bytes().get(after),
            Some(b'>' | b' ' | b'\t' | b'\r' | b'\n')
        ) {
            pos = after;
            continue;
        }
        if let Some((li_inner, close_end)) =
            find_balanced_element_inner(list_inner, &lower, start, "li")
        {
            let cleaned = clean_inline_fragment(li_inner, base_url);
            let plain = collapse_ws(&strip_all_html_tags(&cleaned));
            if plain.chars().count() >= 6 && !is_boilerplate_paragraph(&plain) {
                items.push(cleaned);
            }
            pos = close_end;
        } else {
            break;
        }
    }

    items
}

fn is_boilerplate_heading(text: &str) -> bool {
    let lower = text.to_lowercase();
    matches!(
        lower.as_str(),
        "ilgili haberler"
            | "benzer haberler"
            | "yorumlar"
            | "etiketler"
            | "paylaş"
            | "bizi takip edin"
            | "en çok okunanlar"
            | "son dakika"
    )
}

fn is_boilerplate_paragraph(plain: &str) -> bool {
    let lower = plain.to_lowercase();
    const PHRASES: &[&str] = &[
        "tüm hakları saklıdır",
        "her hakkı saklıdır",
        "çerez politikası",
        "gizlilik politikası",
        "kvkk aydınlatma",
        "google news'e abone",
        "google haberler'e abone",
        "abone olmak için tıklayın",
        "whatsapp ihbar hattı",
        "yasal uyarı:",
        "izinsiz kopyalanamaz",
        "kaynak gösterilmeden kullanılamaz",
        "all rights reserved",
        "cookie policy",
    ];
    PHRASES.iter().any(|p| lower.contains(p))
}

fn count_paragraph_blocks(blocks: &[ContentBlock]) -> usize {
    blocks
        .iter()
        .filter(|b| b.block_type == "paragraph")
        .count()
}

fn extract_br_separated_paragraphs(clean_region: &str, base_url: Option<&Url>) -> Vec<String> {
    let normalized = clean_region
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n")
        .replace("<BR>", "\n")
        .replace("<BR/>", "\n")
        .replace("</div>", "\n\n");
    normalized
        .split("\n\n")
        .map(|chunk| clean_inline_fragment(chunk, base_url))
        .filter(|clean_html| {
            let plain = collapse_ws(&strip_all_html_tags(clean_html));
            plain.chars().count() >= 30 && !is_boilerplate_paragraph(&plain)
        })
        .collect()
}

fn split_ld_article_body(raw: &str) -> Vec<String> {
    let cleaned = strip_tags_raw(raw);
    let mut paras: Vec<String> = cleaned
        .split('\n')
        .map(collapse_ws)
        .filter(|p| p.chars().count() >= 25 && !is_boilerplate_paragraph(p))
        .collect();

    // If JSON-LD articleBody is one long unsplit wall of text (> 420 chars), group ~3 sentences per paragraph
    if paras.len() == 1 && paras[0].chars().count() > 420 {
        let sentences = split_news_sentences(&paras[0]);
        if sentences.len() >= 3 {
            paras.clear();
            let mut chunk = Vec::new();
            let mut chunk_len = 0usize;
            for s in sentences {
                chunk_len += s.chars().count();
                chunk.push(s);
                if chunk.len() >= 3 || chunk_len >= 280 {
                    paras.push(chunk.join(" "));
                    chunk.clear();
                    chunk_len = 0;
                }
            }
            if !chunk.is_empty() {
                paras.push(chunk.join(" "));
            }
        }
    }

    paras
}

/// Extracts up to 5 informative sentences (`40..=220` chars) containing figures, currencies, percentages, or dates.
pub fn extract_key_facts_from_blocks(blocks: &[ContentBlock]) -> Vec<String> {
    let mut facts = Vec::new();
    let mut seen = HashSet::new();

    for block in blocks {
        if block.block_type != "paragraph" && block.block_type != "bullet_list" {
            continue;
        }
        let plain = strip_all_html_tags(&block.content);
        for line in plain.lines() {
            for sentence in split_news_sentences(line) {
                let clean = collapse_ws(&sentence);
                let char_len = clean.chars().count();
                if !(40..=220).contains(&char_len) {
                    continue;
                }
                if is_boilerplate_paragraph(&clean) {
                    continue;
                }
                if sentence_contains_fact_signal(&clean) {
                    let dedup_key = normalize_title_for_dedup(&clean);
                    if seen.insert(dedup_key) {
                        facts.push(clean);
                        if facts.len() >= 5 {
                            return facts;
                        }
                    }
                }
            }
        }
    }

    facts
}

fn sentence_contains_fact_signal(sentence: &str) -> bool {
    if sentence
        .chars()
        .any(|c| c.is_ascii_digit() || matches!(c, '%' | '₺' | '$' | '€' | '£'))
    {
        return true;
    }
    let lower = sentence.to_lowercase();
    const FACT_WORDS: &[&str] = &[
        "milyon", "milyar", "trilyon", "yüzde", "dolar", "euro", "lira", "puan", "tarih", "million",
        "billion", "percent",
    ];
    FACT_WORDS.iter().any(|w| lower.contains(w))
}

/// Splits Turkish & English prose into sentences without breaking decimals (`%42.5`), ordinals (`3. çeyrek`), or titles (`Prof. Dr.`).
fn split_news_sentences(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let chars: Vec<char> = trimmed.chars().collect();
    let mut sentences = Vec::new();
    let mut buf = String::new();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        buf.push(ch);

        if matches!(ch, '.' | '!' | '?') {
            // Must be followed by whitespace and then an uppercase letter or opening quote
            let mut j = i + 1;
            while j < chars.len() && matches!(chars[j], '"' | '\'' | '”' | '’' | ')') {
                buf.push(chars[j]);
                j += 1;
            }
            if j < chars.len() && chars[j].is_whitespace() {
                let mut k = j;
                while k < chars.len() && chars[k].is_whitespace() {
                    k += 1;
                }
                let next_starts_sentence = k < chars.len()
                    && (chars[k].is_uppercase() || matches!(chars[k], '"' | '“' | '‘'));

                if next_starts_sentence && !ends_with_abbreviation(&buf) {
                    let s = collapse_ws(&buf);
                    if !s.is_empty() {
                        sentences.push(s);
                    }
                    buf.clear();
                    i = k;
                    continue;
                }
            }
        }
        i += 1;
    }

    let tail = collapse_ws(&buf);
    if !tail.is_empty() {
        sentences.push(tail);
    }
    sentences
}

fn ends_with_abbreviation(buf: &str) -> bool {
    let clean = buf.trim_end_matches(['.', '"', '\'', '”', '’', ')']);
    let last_word = clean
        .rsplit(|c: char| c.is_whitespace())
        .next()
        .unwrap_or("")
        .to_lowercase();
    matches!(
        last_word.as_str(),
        "prof"
            | "dr"
            | "doç"
            | "av"
            | "alb"
            | "org"
            | "yd"
            | "yrd"
            | "sn"
            | "no"
            | "yy"
            | "vb"
            | "vs"
            | "mr"
            | "mrs"
            | "ms"
            | "st"
    )
}

fn compute_blocks_word_count(blocks: &[ContentBlock]) -> usize {
    blocks
        .iter()
        .filter(|b| b.block_type != "image" && b.block_type != "embed" && b.block_type != "divider")
        .map(|b| strip_all_html_tags(&b.content).split_whitespace().count())
        .sum()
}

fn extract_first_h1_text(html: &str) -> Option<String> {
    let inner = extract_tag_inner(html, "h1")?;
    let clean = collapse_ws(&strip_tags_raw(&decode_html_entities(inner)));
    (!clean.is_empty()).then_some(clean)
}

fn extract_first_time_datetime(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let rel = lower.find("<time")?;
    let after = rel + 5;
    let gt_rel = lower[after..].find('>')?;
    let attrs = &html[after..after + gt_rel];
    get_html_attr(attrs, "datetime")
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
}

fn extract_tag_inner<'a>(html: &'a str, tag_lower: &str) -> Option<&'a str> {
    let lower = html.to_ascii_lowercase();
    let pat = format!("<{tag_lower}");
    let mut pos = 0usize;
    while let Some(rel) = lower[pos..].find(&pat) {
        let start = pos + rel;
        let after = start + pat.len();
        if !matches!(
            lower.as_bytes().get(after),
            Some(b'>' | b' ' | b'\t' | b'\r' | b'\n')
        ) {
            pos = after;
            continue;
        }
        if let Some((inner, _)) = find_balanced_element_inner(html, &lower, start, tag_lower) {
            return Some(inner);
        }
        pos = after;
    }
    None
}

fn extract_first_img_src_raw(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let rel = lower.find("<img")?;
    let after = rel + 4;
    let gt_rel = lower[after..].find('>')?;
    let attrs = &html[after..after + gt_rel];
    get_html_attr(attrs, "src")
}

/// UTF-8 safe HTML/XML attribute reader supporting double-quoted, single-quoted, and unquoted attributes.
fn get_html_attr(raw_attrs: &str, target_attr_lower: &str) -> Option<String> {
    let bytes = raw_attrs.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let name_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'>'
            && bytes[i] != b'/'
        {
            i += 1;
        }
        let attr_name = &raw_attrs[name_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            continue;
        }
        i += 1; // skip '='
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let val_slice = if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            let v_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            let s = &raw_attrs[v_start..i];
            if i < bytes.len() {
                i += 1;
            }
            s
        } else {
            let v_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                i += 1;
            }
            &raw_attrs[v_start..i]
        };

        if attr_name.eq_ignore_ascii_case(target_attr_lower) {
            return Some(val_slice.to_string());
        }
    }

    None
}

fn strip_tags_raw(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collapse_inline_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.trim().chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_chars).collect();
    format!("{}…", truncated.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_turkish_numeric_hex_named_and_cdata_entities_cleanly() {
        let raw = "<![CDATA[T&#252;rkiye &#304;statistik Kurumu: &#286;&#287;&#350;&#351;&#231;&#246;&#305; &amp; &#x20BA;100 &quot;rekor&quot; &#39;art&#305;&#351;&#39;]]>";
        let decoded = decode_html_entities(raw);
        assert_eq!(
            decoded,
            "Türkiye İstatistik Kurumu: ĞğŞşçöı & ₺100 \"rekor\" 'artış'"
        );
    }

    #[test]
    fn parses_bing_rss_items_unwraps_apiclick_and_upgrades_thumbnails() {
        let sample_rss = r#"<?xml version="1.0" encoding="utf-8" ?>
        <rss version="2.0" xmlns:News="https://www.bing.com/news">
          <channel>
            <title>ekonomi - BingHaberler</title>
            <item>
              <title>Merkez Bankas&#305; rezervleri t&#252;m zamanlar&#305;n zirvesine &#231;&#305;kt&#305;</title>
              <link>http://www.bing.com/news/apiclick.aspx?ref=FexRss&amp;aid=&amp;tid=1234&amp;url=https%3a%2f%2fwww.ekonomigazetesi.com%2fhaber%2fmerkez-bankasi-rekor-98765&amp;c=999&amp;mkt=tr-tr</link>
              <description>T&#252;rkiye Cumhuriyet Merkez Bankas&#305; toplam rezervleri ge&#231;en hafta 159,4 milyar dolara y&#252;kselerek rekor k&#305;rd&#305;.</description>
              <pubDate>Tue, 17 Nov 2026 09:15:00 GMT</pubDate>
              <News:Source>Ekonomi Gazetesi</News:Source>
              <News:Image>http://www.bing.com/th?id=ONUT.SampleThumb123&amp;pid=News</News:Image>
            </item>
            <item>
              <title><![CDATA[Savunma sanayiinde yeni ihracat başarısı]]></title>
              <link>https://www.trthaber.com/haber/ekonomi/savunma-ihracat-12345.html</link>
              <description><![CDATA[<img src="https://cdn.trthaber.com/img/savunma.jpg"/>Savunma ve havacılık ihracatı yüzde 28 artış kaydetti.]]></description>
              <pubDate>Tue, 17 Nov 2026 10:00:00 +0300</pubDate>
              <media:content url="https://cdn.trthaber.com/img/savunma.jpg" type="image/jpeg" />
            </item>
          </channel>
        </rss>"#;

        let items = parse_rss_items(sample_rss, "TRT Haber");
        assert_eq!(items.len(), 2);

        assert_eq!(
            items[0].title,
            "Merkez Bankası rezervleri tüm zamanların zirvesine çıktı"
        );
        assert_eq!(
            items[0].url,
            "https://www.ekonomigazetesi.com/haber/merkez-bankasi-rekor-98765"
        );
        assert_eq!(items[0].source_name, "Ekonomi Gazetesi");
        assert_eq!(
            items[0].image_url.as_deref(),
            Some("https://www.bing.com/th?id=ONUT.SampleThumb123&pid=News&w=480&h=270&c=7")
        );
        assert!(items[0]
            .snippet
            .as_deref()
            .unwrap_or("")
            .contains("159,4 milyar dolara yükselerek"));

        assert_eq!(items[1].title, "Savunma sanayiinde yeni ihracat başarısı");
        assert_eq!(items[1].source_name, "TRT Haber");
        assert_eq!(
            items[1].image_url.as_deref(),
            Some("https://cdn.trthaber.com/img/savunma.jpg")
        );
        assert_eq!(
            items[1].snippet.as_deref(),
            Some("Savunma ve havacılık ihracatı yüzde 28 artış kaydetti.")
        );
    }

    #[test]
    fn parses_realistic_turkish_news_html_with_og_ldjson_lazy_images_noise_facts_and_embeds() {
        let html = r#"<!DOCTYPE html>
        <html lang="tr">
        <head>
          <meta charset="UTF-8">
          <title>İstanbul Havalimanı yolcu rekoru kırdı - Haber Portalı</title>
          <link rel="canonical" href="/ekonomi/istanbul-havalimani-yolcu-rekoru-kirdi-2026" />
          <meta property="og:site_name" content="Anadolu Haber" />
          <meta property="og:title" content="İstanbul Havalimanı 2026 yılında yolcu rekoru kırdı" />
          <meta property="og:description" content="Avrupa'nın zirvesindeki İstanbul Havalimanı, yılın ilk 10 ayında yolcu sayısını yüzde 14 artırdı." />
          <meta property="og:image" content="/uploads/2026/11/iga-kapak-1200.jpg" />
          <meta property="article:published_time" content="2026-11-17T08:45:00+03:00" />
          <script type="application/ld+json">
          {
            "@context": "https://schema.org",
            "@graph": [
              {
                "@type": "NewsArticle",
                "headline": "İstanbul Havalimanı 2026 yılında yolcu rekoru kırdı",
                "author": { "@type": "Person", "name": "Selin Yılmaz" },
                "datePublished": "2026-11-17T08:45:00+03:00",
                "image": ["https://www.anadoluhaber.com.tr/uploads/2026/11/iga-kapak-1200.jpg"]
              }
            ]
          }
          </script>
        </head>
        <body>
          <header class="site-header"><nav>Gündem | Ekonomi | Spor</nav></header>
          <article class="haber-detay">
            <header>
              <h1>İstanbul Havalimanı 2026 yılında yolcu rekoru kırdı</h1>
              <div class="social-share">Paylaş: X | WhatsApp</div>
            </header>

            <p>Avrupa hava trafiği verilerine göre <strong>İstanbul Havalimanı</strong>, 2026 yılının Ocak-Ekim döneminde toplam 71,4 milyon yolcuya hizmet vererek tüm zamanların rekorunu yeniledi.</p>

            <h2>Günlük Uçuş Sayısında Avrupa Liderliği</h2>

            <p>Eurocontrol raporuna göre havalimanında günlük ortalama 1.540 uçuş gerçekleşti ve kargo hacmi geçen yıla kıyasla %18,5 oranında büyüdü.</p>

            <figure class="article-photo">
              <img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=" data-src="/uploads/2026/11/pist-grafik.webp" alt="Uçuş trafiği grafiği" />
              <figcaption>2026 Ekim ayı Avrupa havalimanları yolcu karşılaştırması</figcaption>
            </figure>

            <blockquote>
              <p>Yeni paralel pist yatırımıyla yıllık kapasitemizi 120 milyon yolcunun üzerine taşıyoruz.</p>
              <cite>Ulaştırma Bakanlığı Açıklaması</cite>
            </blockquote>

            <div class="video-wrapper">
              <iframe src="https://www.youtube.com/embed/dQw4w9WgXcQ" width="560" height="315"></iframe>
            </div>

            <blockquote class="twitter-tweet">
              <p>Resmi veriler açıklandı.</p>
              <a href="https://x.com/igairport/status/1858112233445566778">17 Kasım 2026</a>
            </blockquote>

            <aside class="related-news">
              <h3>İlgili Haberler</h3>
              <p>Bu ilgisiz yan kutu metni okuyucu görünümüne kesinlikle girmemelidir çünkü benzer haberler bloğudur.</p>
            </aside>

            <div class="reklam-alani">
              <p>Sponsorlu reklam içeriği burada yer almaktadır ve filtrelenmelidir.</p>
            </div>

            <p>Tüm hakları saklıdır. Google News'e abone olmak için tıklayın.</p>
            <p>Kısa satır</p>
          </article>
        </body>
        </html>"#;

        let res = parse_article_html_to_reader(
            "https://www.anadoluhaber.com.tr/ekonomi/iga-rekor",
            html,
        );

        assert!(!res.blocked_by_source);
        assert_eq!(res.domain, "anadoluhaber.com.tr");
        assert_eq!(res.site_name, "Anadolu Haber");
        assert_eq!(
            res.canonical_url,
            "https://www.anadoluhaber.com.tr/ekonomi/istanbul-havalimani-yolcu-rekoru-kirdi-2026"
        );
        assert_eq!(
            res.title,
            "İstanbul Havalimanı 2026 yılında yolcu rekoru kırdı"
        );
        assert_eq!(res.author.as_deref(), Some("Selin Yılmaz"));
        assert_eq!(
            res.published_at.as_deref(),
            Some("2026-11-17T08:45:00+03:00")
        );
        assert_eq!(
            res.lead_image.as_deref(),
            Some("https://www.anadoluhaber.com.tr/uploads/2026/11/iga-kapak-1200.jpg")
        );

        // Verify clean blocks: noise `<aside class="related-news">`, `.reklam-alani`, boilerplate & tiny lines stripped!
        let all_text: String = res
            .blocks
            .iter()
            .map(|b| b.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!all_text.contains("ilgisiz yan kutu"));
        assert!(!all_text.contains("Sponsorlu reklam"));
        assert!(!all_text.contains("Tüm hakları saklıdır"));
        assert!(!all_text.contains("Kısa satır"));

        assert!(res
            .blocks
            .iter()
            .any(|b| b.block_type == "heading" && b.content == "Günlük Uçuş Sayısında Avrupa Liderliği"));
        assert!(res.blocks.iter().any(|b| {
            b.block_type == "image"
                && b.content == "https://www.anadoluhaber.com.tr/uploads/2026/11/pist-grafik.webp"
                && b.meta.as_deref() == Some("2026 Ekim ayı Avrupa havalimanları yolcu karşılaştırması")
        }));
        assert!(res.blocks.iter().any(|b| b.block_type == "quote"));

        // Verify key_facts extracted numerical sentences
        assert!(
            res.key_facts.len() >= 2,
            "expected >=2 key facts, got {:?}",
            res.key_facts
        );
        assert!(res.key_facts.iter().any(|f| f.contains("71,4 milyon")));
        assert!(res.key_facts.iter().any(|f| f.contains("%18,5")));

        // Verify YouTube & X/Twitter media_embeds detection
        assert!(res
            .media_embeds
            .iter()
            .any(|u| u.contains("youtube.com/embed/dQw4w9WgXcQ")));
        assert!(res
            .media_embeds
            .iter()
            .any(|u| u == "https://x.com/igairport/status/1858112233445566778"));
        assert!(res.word_count >= 35);
    }

    #[test]
    fn falls_back_to_ld_json_article_body_when_html_has_fewer_than_two_paragraphs() {
        let html = r#"<!DOCTYPE html><html><head>
          <meta property="og:title" content="Borsa İstanbul günü yükselişle tamamladı" />
          <script type="application/ld+json">
          {
            "@type": "ReportageNewsArticle",
            "headline": "Borsa İstanbul günü yükselişle tamamladı",
            "articleBody": "BIST 100 endeksi günü yüzde 2,34 değer kazancıyla 11.450 puandan tamamladı.\nBankacılık endeksi yüzde 3,8 yükselirken toplam işlem hacmi 118 milyar lira olarak gerçekleşti."
          }
          </script>
        </head><body><div id="app"></div></body></html>"#;

        let res = parse_article_html_to_reader("https://www.borsahaber.com/bist-kapanis", html);
        assert_eq!(count_paragraph_blocks(&res.blocks), 2);
        assert!(res.key_facts.iter().any(|f| f.contains("118 milyar lira")));
    }

    #[tokio::test]
    async fn ssrf_guard_blocks_loopback_link_local_metadata_and_private_urls() {
        for bad_url in [
            "http://127.0.0.1:8080/admin",
            "http://169.254.169.254/latest/meta-data/",
            "http://localhost:3000/api/health",
            "http://10.0.0.5/internal",
            "http://192.168.1.1/status",
            "http://[::1]/secret",
            "file:///etc/passwd",
        ] {
            let sync_res = validate_url_syntax_and_host(bad_url);
            assert!(
                matches!(sync_res, Err(AppError::BadRequest(_))),
                "Expected SSRF BadRequest for {bad_url}, got {sync_res:?}"
            );

            // Even if fallback_title is provided, SSRF URLs must NEVER return fallback Ok(...)!
            let reader_res = extract_reader_article(ReadArticleRequest {
                url: bad_url.to_string(),
                fallback_title: Some("Sahte Başlık".to_string()),
                fallback_snippet: Some("Sahte özet metni".to_string()),
                fallback_source: None,
                fallback_image: None,
            })
            .await;
            assert!(
                matches!(reader_res, Err(AppError::BadRequest(_))),
                "extract_reader_article must reject SSRF URL {bad_url}"
            );
        }
    }

    #[tokio::test]
    async fn direct_url_search_and_utf8_dotted_i_stress_test() {
        let direct = search_news(
            "https://www.trthaber.com/haber/ekonomi/merkez-bankasi-faiz-karari-9988.html",
            "tr",
            "24h",
            None,
            &[],
        )
        .await
        .expect("direct URL search");
        assert_eq!(direct.len(), 1);
        assert_eq!(direct[0].source_name, "trthaber.com");
        assert_eq!(
            direct[0].url,
            "https://www.trthaber.com/haber/ekonomi/merkez-bankasi-faiz-karari-9988.html"
        );

        // Stress test UTF-8 Turkish uppercase İ/Ş/Ğ + legacy <br><br> body without <p>
        let tricky_html = r#"<html><head><title>İSTANBUL BÜYÜKŞEHİR İSTATİSTİK RAPORU - İBB</title></head>
        <body><div class="haber-detay">
          <div class="reklam">İLGİSİZ İNTERAKTİF İLAN</div>
          İSTANBUL genelinde toplu taşıma kullanım oranı 2026 yılında yüzde 22 artışla günlük 9,8 milyon yolculuğa ulaştı.<br/><br/>
          Şehir içi raylı sistem ağının toplam uzunluğu yeni açılan etaplarla birlikte 380 kilometreye yükseldi.
        </div></body></html>"#;
        let parsed = parse_article_html_to_reader("https://ibb.istanbul/haber/1", tricky_html);
        assert_eq!(count_paragraph_blocks(&parsed.blocks), 2);
        assert!(parsed.key_facts.iter().any(|f| f.contains("9,8 milyon")));
    }

    #[tokio::test]
    #[ignore]
    async fn live_network_rss_search_and_reader_view_verification() {
        let agenda = search_news("", "tr", "24h", None, &[])
            .await
            .expect("live TR agenda RSS");
        assert!(!agenda.is_empty(), "TR live agenda must return real items");
        println!("[LIVE TR AGENDA] count={}", agenda.len());
        for item in agenda.iter().take(3) {
            println!("  -> [{}] {} ({})", item.source_name, item.title, item.url);
            assert!(
                !item.url.contains("apiclick.aspx"),
                "Bing URL must be unwrapped"
            );
        }

        let kw = search_news("ekonomi", "tr", "24h", None, &[])
            .await
            .expect("live Bing keyword search");
        assert!(!kw.is_empty(), "Bing keyword search must return real items");
        println!("[LIVE KEYWORD 'ekonomi'] count={}", kw.len());
        for item in kw.iter().take(2) {
            println!("  -> [{}] {} ({})", item.source_name, item.title, item.url);
            assert!(!item.url.contains("apiclick.aspx"));
        }

        // Test real Reader View extraction on a live TRT Haber or BBC Türkçe article from agenda
        let target = agenda
            .iter()
            .find(|i| i.url.contains("trthaber.com") || i.url.contains("bbc.com"))
            .unwrap_or(&agenda[0]);
        let article = extract_reader_article(ReadArticleRequest {
            url: target.url.clone(),
            fallback_title: Some(target.title.clone()),
            fallback_snippet: target.snippet.clone(),
            fallback_source: Some(target.source_name.clone()),
            fallback_image: target.image_url.clone(),
        })
        .await
        .expect("live reader article extraction");

        println!(
            "[LIVE READER] title='{}' site='{}' blocks={} words={} facts={} blocked={}",
            article.title,
            article.site_name,
            article.blocks.len(),
            article.word_count,
            article.key_facts.len(),
            article.blocked_by_source
        );
        assert!(!article.title.is_empty());
        assert!(!article.blocks.is_empty());
    }
}
