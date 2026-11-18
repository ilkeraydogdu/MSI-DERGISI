//! Pure-Rust live content synchronization CLI for `msidergisi.com`.
//!
//! Replaces the legacy `tools/sync-msidergisi.py` script so operators can run
//! `cargo run -- --sync-live [--limit=N]` using the compiled Rust binary.

use crate::config::AppConfig;
use crate::db::Database;
use chrono::Utc;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use reqwest::Client;
use rusqlite::{params, types::ValueRef, Connection, Row};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

const DEFAULT_BASE_URL: &str = "https://msidergisi.com";
const FALLBACK_COVER_IMAGE: &str = "images/msi/dergi-286.webp";
const USER_AGENT_VALUE: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

const CAPTCHA_MARKERS: &[&str] = &[
    "cf-turnstile",
    "g-recaptcha",
    "hcaptcha",
    "verify you are human",
    "/cdn-cgi/challenge-platform",
];

static RE_ENTITY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|[a-zA-Z][a-zA-Z0-9]+);").unwrap());
static RE_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").unwrap());
static RE_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
static RE_NON_ALNUM: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^\w\s]").unwrap());
static RE_LOC: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<loc[^>]*>(.*?)</loc>").unwrap());
static RE_INERTIA_SCRIPT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)<script[^>]*data-page=["']app["'][^>]*>(.*?)</script>"#).unwrap());
static RE_INERTIA_ATTR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)data-page="([^"]+)""#).unwrap());
static RE_DT_FULL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}:\d{2})").unwrap());
static RE_DT_DATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\d{4}-\d{2}-\d{2})").unwrap());
static RE_DT_DMY: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\d{2})\.(\d{2})\.(\d{4})").unwrap());

/// Parsed command-line options for `--sync-live`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncLiveOptions {
    pub limit: Option<usize>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SitemapDiscovery {
    pub tr_slugs: BTreeSet<String>,
    pub en_slugs: BTreeSet<String>,
    pub tr_category_urls: BTreeSet<String>,
    pub en_category_urls: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct CategoryMeta {
    name_tr: Option<String>,
    name_en: Option<String>,
    desc_tr: Option<String>,
}

#[derive(Debug, Clone)]
struct ExistingContentRow {
    id: i64,
    slug: String,
    title_tr: String,
    body_tr: String,
    body_en: String,
    source_group: String,
}

#[derive(Debug)]
struct CategoryCrawlResult {
    slug: String,
    is_en: bool,
    cat_name: Option<String>,
    post_slugs: Vec<String>,
    error: Option<String>,
}

#[derive(Debug)]
struct PostFetchResult {
    slug: String,
    post: Option<Value>,
    error: Option<String>,
}

/// Parses CLI arguments for `--sync-live` and optional `--limit=N` / `--limit N`.
pub fn parse_sync_live_args<I, S>(args: I) -> Option<SyncLiveOptions>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let collected: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if !collected.iter().any(|a| a == "--sync-live") {
        return None;
    }

    let mut limit: Option<usize> = None;
    let mut iter = collected.iter().peekable();
    while let Some(arg) = iter.next() {
        if let Some(val) = arg.strip_prefix("--limit=") {
            if let Ok(parsed) = val.trim().parse::<usize>() {
                limit = Some(parsed);
            }
        } else if arg == "--limit" {
            if let Some(next_val) = iter.peek() {
                if let Ok(parsed) = next_val.trim().parse::<usize>() {
                    limit = Some(parsed);
                    iter.next();
                }
            }
        }
    }

    Some(SyncLiveOptions { limit })
}

/// Decodes standard HTML/XML named, decimal, and hexadecimal entities in one pass.
pub fn decode_html_entities(input: &str) -> String {
    RE_ENTITY
        .replace_all(input, |caps: &Captures<'_>| {
            let token = &caps[1];
            if let Some(hex_str) = token
                .strip_prefix("#x")
                .or_else(|| token.strip_prefix("#X"))
            {
                return u32::from_str_radix(hex_str, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| caps[0].to_string());
            }
            if let Some(dec_str) = token.strip_prefix('#') {
                return dec_str
                    .parse::<u32>()
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| caps[0].to_string());
            }
            match token {
                "amp" => "&",
                "lt" => "<",
                "gt" => ">",
                "quot" => "\"",
                "apos" => "'",
                "nbsp" => " ",
                "ndash" => "–",
                "mdash" => "—",
                "lsquo" => "‘",
                "rsquo" => "’",
                "ldquo" => "“",
                "rdquo" => "”",
                "hellip" => "…",
                "copy" => "©",
                "reg" => "®",
                "trade" => "™",
                "deg" => "°",
                "euro" => "€",
                "pound" => "£",
                "bull" => "•",
                "middot" => "·",
                "laquo" => "«",
                "raquo" => "»",
                "ccedil" => "ç",
                "Ccedil" => "Ç",
                "ouml" => "ö",
                "Ouml" => "Ö",
                "uuml" => "ü",
                "Uuml" => "Ü",
                "acirc" => "â",
                "Acirc" => "Â",
                "icirc" => "î",
                "Icirc" => "Î",
                "ucirc" => "û",
                "Ucirc" => "Û",
                "scedil" => "ş",
                "Scedil" => "Ş",
                "gbreve" => "ğ",
                "Gbreve" => "Ğ",
                "Idot" => "İ",
                "inodot" => "ı",
                _ => return caps[0].to_string(),
            }
            .to_string()
        })
        .into_owned()
}

/// Strips HTML tags, decodes entities, and normalizes whitespace.
pub fn strip_html_tags(input: &str) -> String {
    let without_tags = RE_TAG.replace_all(input, " ");
    let decoded = decode_html_entities(&without_tags);
    RE_WS.replace_all(decoded.trim(), " ").into_owned()
}

/// Extracts the trailing non-empty URL path segment as a slug, stripping query/fragment parts.
pub fn extract_slug_from_url(url: &str) -> Option<String> {
    let no_frag = url.split('#').next().unwrap_or("");
    let no_query = no_frag.split('?').next().unwrap_or("").trim();
    let (has_scheme, path_part) = match no_query.split_once("://") {
        Some((_, rest)) => (true, rest.trim_end_matches('/')),
        None => (false, no_query.trim_end_matches('/')),
    };
    if has_scheme && !path_part.contains('/') {
        return None;
    }
    let seg = path_part.rsplit('/').next().unwrap_or("").trim();
    if seg.is_empty() {
        return None;
    }
    Some(seg.to_string())
}

/// Extracts all `<loc>...</loc>` URLs from a sitemap XML payload.
pub fn extract_sitemap_urls(xml: &str) -> Vec<String> {
    RE_LOC
        .captures_iter(xml)
        .filter_map(|cap| {
            let val = decode_html_entities(cap[1].trim());
            let trimmed = val.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

/// Classifies sitemap URLs into TR/EN article slugs and category URLs.
pub fn classify_sitemap_urls(urls: &[String]) -> SitemapDiscovery {
    let mut discovery = SitemapDiscovery::default();
    for raw_url in urls {
        let clean = raw_url
            .split('#')
            .next()
            .unwrap_or("")
            .split('?')
            .next()
            .unwrap_or("")
            .trim()
            .trim_end_matches('/');
        if clean.contains("/en/haberler/") {
            if let Some(slug) = extract_slug_from_url(clean) {
                discovery.en_slugs.insert(slug);
            }
        } else if clean.contains("/haberler/") {
            if let Some(slug) = extract_slug_from_url(clean) {
                discovery.tr_slugs.insert(slug);
            }
        } else if clean.contains("/en/kategoriler/") {
            discovery.en_category_urls.insert(clean.to_string());
        } else if clean.contains("/kategoriler/") {
            discovery.tr_category_urls.insert(clean.to_string());
        }
    }
    discovery
}

/// Extracts the Inertia `props` object from an HTML response.
pub fn extract_inertia_props(html: &str) -> Result<Value, String> {
    if let Some(cap) = RE_INERTIA_SCRIPT.captures(html) {
        let parsed: Value = serde_json::from_str(cap[1].trim())
            .map_err(|e| format!("Invalid Inertia <script> JSON: {}", e))?;
        return Ok(parsed.get("props").cloned().unwrap_or(Value::Object(Map::new())));
    }

    for cap in RE_INERTIA_ATTR.captures_iter(html) {
        let unescaped = decode_html_entities(&cap[1]);
        if unescaped.trim_start().starts_with('{') {
            let parsed: Value = serde_json::from_str(&unescaped)
                .map_err(|e| format!("Invalid Inertia data-page JSON: {}", e))?;
            return Ok(parsed.get("props").cloned().unwrap_or(Value::Object(Map::new())));
        }
    }

    Err("No Inertia payload found".to_string())
}

/// Normalizes a Turkish/Latin title for slug deduplication matching.
pub fn norm_title(text: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    let folded: String = text
        .chars()
        .map(|ch| match ch {
            'Ç' | 'ç' => 'c',
            'Ğ' | 'ğ' => 'g',
            'İ' | 'I' | 'ı' | 'î' | 'Î' => 'i',
            'Ö' | 'ö' => 'o',
            'Ş' | 'ş' => 's',
            'Ü' | 'ü' | 'û' | 'Û' => 'u',
            'â' | 'Â' => 'a',
            other => other,
        })
        .collect::<String>()
        .to_lowercase();
    let clean = RE_NON_ALNUM.replace_all(&folded, " ");
    RE_WS.replace_all(clean.trim(), " ").into_owned()
}

/// Parses ISO or DD.MM.YYYY date strings into `YYYY-MM-DD HH:MM:SS`.
pub fn parse_published_at(raw_dt: Option<&str>, fallback_dmy: Option<&str>) -> String {
    if let Some(raw) = raw_dt.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(cap) = RE_DT_FULL.captures(raw) {
            return format!("{} {}", &cap[1], &cap[2]);
        }
        if let Some(cap) = RE_DT_DATE.captures(raw) {
            return format!("{} 12:00:00", &cap[1]);
        }
    }
    if let Some(dmy) = fallback_dmy.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(cap) = RE_DT_DMY.captures(dmy) {
            return format!("{}-{}-{} 12:00:00", &cap[3], &cap[2], &cap[1]);
        }
    }
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Estimates reading time in minutes at 180 words/minute (minimum 1 minute).
pub fn estimate_read_time(html_body: &str) -> i64 {
    let text = strip_html_tags(html_body);
    let words = text.split_whitespace().count();
    ((words as f64 / 180.0).round() as i64).max(1)
}

fn pick_image_url(img_obj: Option<&Value>, base_url: &str) -> Option<String> {
    let obj = img_obj?.as_object()?;
    if let Some(url) = obj.get("url").and_then(Value::as_str) {
        if url.starts_with("http") {
            return Some(url.to_string());
        }
    }
    let file_rel = obj.get("file").and_then(Value::as_str)?;
    let uuid_str = obj.get("uuid").and_then(Value::as_str)?;
    Some(format!(
        "{}/media/{}/{}",
        base_url.trim_end_matches('/'),
        uuid_str,
        file_rel
    ))
}

fn sanitize_asset_slug(slug: &str) -> String {
    let mapped: String = slug
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .take(110)
        .collect();
    let trimmed = mapped.trim_matches('-');
    if trimmed.is_empty() {
        "msi-asset".to_string()
    } else {
        trimmed.to_string()
    }
}

fn pick_image_extension(url: &str) -> &'static str {
    let path = url.split('#').next().unwrap_or(url).split('?').next().unwrap_or("");
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "webp" => ".webp",
        "jpg" => ".jpg",
        "jpeg" => ".jpeg",
        "png" => ".png",
        "gif" => ".gif",
        _ => ".webp",
    }
}

/// Maps category slug + placement tags to canonical `(content_type, source_group)`.
pub fn classify_content_type_and_source(
    cat_slug: &str,
    placements: &HashSet<String>,
    existing_ct: Option<&str>,
) -> (String, String) {
    const ORDERED: &[&str] = &[
        "kapak-dosyasi",
        "basyazilar",
        "soylesiler",
        "ozel-haberler",
        "tanitici-makaleler",
        "makaleler-analizler",
        "mesajlar",
        "advertorials",
        "yeni-sayi-kapak",
        "haberler",
    ];

    let c_slug = cat_slug.trim().to_ascii_lowercase();
    let mut chosen_ct = ORDERED
        .iter()
        .find(|&&cand| cand == c_slug)
        .map(|s| (*s).to_string());

    if chosen_ct.is_none() {
        chosen_ct = ORDERED
            .iter()
            .find(|&&cand| placements.contains(cand))
            .map(|s| (*s).to_string());
    }

    if chosen_ct.is_none() {
        if let Some(ex) = existing_ct {
            let mapped = match ex {
                "advertorial" => "advertorials",
                "basyazi" => "basyazilar",
                "kapak-dosyalari" => "kapak-dosyasi",
                other => other,
            };
            chosen_ct = Some(mapped.to_string());
        }
    }

    let ct = chosen_ct.unwrap_or_else(|| "haberler".to_string());
    let sg = if matches!(
        ct.as_str(),
        "kapak-dosyasi"
            | "basyazilar"
            | "soylesiler"
            | "ozel-haberler"
            | "mesajlar"
            | "tanitici-makaleler"
            | "makaleler-analizler"
            | "advertorials"
    ) || placements.contains("ozel-dosya")
        || placements.contains("dergi-icerikleri")
    {
        "dergi-icerigi"
    } else if ct == "yeni-sayi-kapak" {
        "sadece-online"
    } else {
        "gunluk-haber"
    };

    (ct, sg.to_string())
}

fn detect_captcha_marker(content_type: &str, body: &[u8]) -> Option<&'static str> {
    let ctype_low = content_type.to_ascii_lowercase();
    if !(ctype_low.contains("text/") || ctype_low.contains("json") || ctype_low.contains("xml")) {
        return None;
    }
    let slice_len = body.len().min(8192);
    let prefix = String::from_utf8_lossy(&body[..slice_len]).to_ascii_lowercase();
    CAPTCHA_MARKERS
        .iter()
        .copied()
        .find(|marker| prefix.contains(marker))
}

fn build_http_client() -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        ),
    );
    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("tr-TR,tr;q=0.9,en-US;q=0.8,en;q=0.7"),
    );

    Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

async fn http_get_bytes(client: &Client, url: &str, retries: usize) -> Result<Vec<u8>, String> {
    let mut last_err = String::new();
    for attempt in 0..=retries {
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                let ctype = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                if !status.is_success() {
                    last_err = format!("HTTP {}", status);
                } else {
                    match resp.bytes().await {
                        Ok(bytes) => {
                            let vec = bytes.to_vec();
                            if let Some(marker) = detect_captcha_marker(&ctype, &vec) {
                                return Err(format!(
                                    "CAPTCHA/anti-bot gate encountered at {}: {}",
                                    url, marker
                                ));
                            }
                            return Ok(vec);
                        }
                        Err(err) => {
                            last_err = err.to_string();
                        }
                    }
                }
            }
            Err(err) => {
                last_err = err.to_string();
            }
        }
        if attempt < retries {
            tokio::time::sleep(Duration::from_millis(600 * (attempt as u64 + 1))).await;
        }
    }
    Err(format!("Failed GET {}: {}", url, last_err))
}

async fn fetch_inertia_props(client: &Client, url: &str) -> Result<Value, String> {
    let bytes = http_get_bytes(client, url, 2).await?;
    let raw = String::from_utf8_lossy(&bytes);
    extract_inertia_props(&raw)
}

async fn download_image_for_slug(
    client: &Client,
    public_dir: &Path,
    slug: &str,
    url: &str,
) -> Option<String> {
    let img_dir = public_dir.join("images").join("msi");
    if fs::create_dir_all(&img_dir).is_err() {
        return None;
    }

    let safe_slug = sanitize_asset_slug(slug);
    let ext = pick_image_extension(url);
    let rel_path = format!("images/msi/{}{}", safe_slug, ext);
    let dest = public_dir.join(&rel_path);

    if let Ok(meta) = fs::metadata(&dest) {
        if meta.len() > 200 {
            return Some(rel_path);
        }
    }

    let data = http_get_bytes(client, url, 1).await.ok()?;
    if data.len() < 200 {
        return None;
    }
    let head = String::from_utf8_lossy(&data[..data.len().min(16)]).to_ascii_lowercase();
    if head.starts_with("<!doctype") || head.starts_with("<html") {
        return None;
    }
    fs::write(&dest, &data).ok()?;
    Some(rel_path)
}

async fn crawl_category_all_pages(
    client: &Client,
    base_cat_url: &str,
    cat_slug: &str,
) -> Result<(Option<String>, Vec<String>), String> {
    let first = fetch_inertia_props(client, &format!("{}?page=1", base_cat_url)).await?;
    let posts_obj = first.get("posts").and_then(Value::as_object);
    let last_page = posts_obj
        .and_then(|p| p.get("meta"))
        .and_then(|m| m.get("last_page"))
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .max(1);
    let cat_name = first
        .get("category")
        .and_then(|c| c.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);

    let mut slugs = Vec::new();
    if let Some(arr) = posts_obj.and_then(|p| p.get("data")).and_then(Value::as_array) {
        for item in arr {
            if let Some(sl) = item.get("slug").and_then(Value::as_str) {
                slugs.push(sl.to_string());
            }
        }
    }

    if last_page > 1 {
        let sem = Arc::new(Semaphore::new(6));
        let mut join_set = JoinSet::new();
        for page in 2..=last_page {
            let client = client.clone();
            let sem = Arc::clone(&sem);
            let url = format!("{}?page={}", base_cat_url, page);
            let cat_slug_owned = cat_slug.to_string();
            join_set.spawn(async move {
                let _permit = sem.acquire_owned().await.ok()?;
                match fetch_inertia_props(&client, &url).await {
                    Ok(props) => Some(Ok(props)),
                    Err(err) => Some(Err((cat_slug_owned, err))),
                }
            });
        }

        while let Some(res) = join_set.join_next().await {
            if let Ok(Some(item)) = res {
                match item {
                    Ok(props) => {
                        if let Some(arr) = props
                            .get("posts")
                            .and_then(|p| p.get("data"))
                            .and_then(Value::as_array)
                        {
                            for post in arr {
                                if let Some(sl) = post.get("slug").and_then(Value::as_str) {
                                    slugs.push(sl.to_string());
                                }
                            }
                        }
                    }
                    Err((cat, err)) => {
                        println!("  WARN page error on {}: {}", cat, err);
                    }
                }
            }
        }
    }

    Ok((cat_name, slugs))
}

async fn crawl_categories_batch(
    client: &Client,
    urls: &BTreeSet<String>,
    is_en: bool,
) -> Vec<CategoryCrawlResult> {
    let sem = Arc::new(Semaphore::new(4));
    let mut join_set = JoinSet::new();

    for url in urls {
        let Some(cat_slug) = extract_slug_from_url(url) else {
            continue;
        };
        let client = client.clone();
        let sem = Arc::clone(&sem);
        let url = url.clone();
        join_set.spawn(async move {
            let _permit = sem.acquire_owned().await;
            match crawl_category_all_pages(&client, &url, &cat_slug).await {
                Ok((cat_name, post_slugs)) => CategoryCrawlResult {
                    slug: cat_slug,
                    is_en,
                    cat_name,
                    post_slugs,
                    error: None,
                },
                Err(err) => CategoryCrawlResult {
                    slug: cat_slug,
                    is_en,
                    cat_name: None,
                    post_slugs: Vec::new(),
                    error: Some(err),
                },
            }
        });
    }

    let mut out = Vec::new();
    while let Some(res) = join_set.join_next().await {
        if let Ok(item) = res {
            out.push(item);
        }
    }
    out
}

async fn fetch_posts_batch(
    client: &Client,
    base_url: &str,
    slugs: &[String],
    is_en: bool,
) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    if slugs.is_empty() {
        return out;
    }

    let sem = Arc::new(Semaphore::new(8));
    let mut join_set = JoinSet::new();
    let prefix = if is_en { "/en/haberler/" } else { "/haberler/" };
    let clean_base = base_url.trim_end_matches('/');

    for slug in slugs {
        let client = client.clone();
        let sem = Arc::clone(&sem);
        let slug_owned = slug.clone();
        let url = format!("{}{}{}", clean_base, prefix, urlencoding::encode(&slug_owned));
        join_set.spawn(async move {
            let _permit = sem.acquire_owned().await;
            match fetch_inertia_props(&client, &url).await {
                Ok(props) => PostFetchResult {
                    slug: slug_owned,
                    post: props.get("post").cloned(),
                    error: None,
                },
                Err(err) => PostFetchResult {
                    slug: slug_owned,
                    post: None,
                    error: Some(err),
                },
            }
        });
    }

    while let Some(res) = join_set.join_next().await {
        if let Ok(item) = res {
            if let Some(err) = item.error {
                let lang_lbl = if is_en { "EN" } else { "TR" };
                println!("  WARN {} post {}: {}", lang_lbl, item.slug, err);
            } else if let Some(post) = item.post {
                if !post.is_null() {
                    out.insert(item.slug, post);
                }
            }
        }
    }
    out
}

fn collect_home_section_data(
    props: &Value,
    base_url: &str,
    is_en: bool,
    cat_meta: &mut BTreeMap<String, CategoryMeta>,
    discovery: &mut SitemapDiscovery,
    placements: &mut HashMap<String, HashSet<String>>,
) {
    let clean_base = base_url.trim_end_matches('/');
    let cat_prefix = if is_en { "/en/kategoriler/" } else { "/kategoriler/" };

    for list_key in ["categories", "webNavigationMenus"] {
        if let Some(items) = props.get(list_key).and_then(Value::as_array) {
            for c in items {
                let Some(slug) = c
                    .get("link")
                    .and_then(Value::as_str)
                    .and_then(extract_slug_from_url)
                else {
                    continue;
                };
                let entry = cat_meta.entry(slug.clone()).or_default();
                let name = c.get("name").and_then(Value::as_str).map(ToString::to_string);
                if is_en {
                    if let Some(n) = name {
                        entry.name_en = Some(n);
                    }
                    discovery
                        .en_category_urls
                        .insert(format!("{}{}{}", clean_base, cat_prefix, slug));
                } else {
                    if let Some(n) = name {
                        entry.name_tr = Some(n);
                    }
                    entry.desc_tr = c
                        .get("description")
                        .and_then(Value::as_str)
                        .map(ToString::to_string);
                    discovery
                        .tr_category_urls
                        .insert(format!("{}{}{}", clean_base, cat_prefix, slug));
                }
            }
        }
    }

    if let Some(sections) = props.get("content").and_then(Value::as_array) {
        for sec in sections {
            let key = sec
                .get("key")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if !key.is_empty() {
                let entry = cat_meta.entry(key.to_string()).or_default();
                let title = sec.get("title").and_then(Value::as_str).map(ToString::to_string);
                if is_en {
                    entry.name_en.get_or_insert_with(|| title.unwrap_or_else(|| key.to_string()));
                    discovery
                        .en_category_urls
                        .insert(format!("{}{}{}", clean_base, cat_prefix, key));
                } else {
                    entry.name_tr.get_or_insert_with(|| title.unwrap_or_else(|| key.to_string()));
                    discovery
                        .tr_category_urls
                        .insert(format!("{}{}{}", clean_base, cat_prefix, key));
                }
            }

            if let Some(items) = sec.get("items").and_then(Value::as_array) {
                for it in items {
                    if let Some(sl) = it.get("slug").and_then(Value::as_str) {
                        if is_en {
                            discovery.en_slugs.insert(sl.to_string());
                        } else {
                            discovery.tr_slugs.insert(sl.to_string());
                        }
                        if !key.is_empty() {
                            placements
                                .entry(sl.to_string())
                                .or_default()
                                .insert(key.to_string());
                        }
                    }
                }
            }
        }
    }
}

fn table_has_column(conn: &Connection, table: &str, col_name: &str) -> bool {
    let sql = format!("PRAGMA table_info({})", table);
    let Ok(mut stmt) = conn.prepare(&sql) else {
        return false;
    };
    let Ok(mut rows) = stmt.query([]) else {
        return false;
    };
    while let Ok(Some(row)) = rows.next() {
        if let Ok(name) = row.get::<_, String>(1) {
            if name == col_name {
                return true;
            }
        }
    }
    false
}

fn row_to_json_map(row: &Row<'_>, col_names: &[String]) -> rusqlite::Result<Map<String, Value>> {
    let mut map = Map::with_capacity(col_names.len());
    for (idx, name) in col_names.iter().enumerate() {
        let val = match row.get_ref(idx)? {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(i) => Value::from(i),
            ValueRef::Real(f) => Value::from(f),
            ValueRef::Text(bytes) => Value::String(String::from_utf8_lossy(bytes).into_owned()),
            ValueRef::Blob(bytes) => Value::String(format!("blob:{}B", bytes.len())),
        };
        map.insert(name.clone(), val);
    }
    Ok(map)
}

fn query_rows_as_json(conn: &Connection, sql: &str) -> Result<Vec<Value>, String> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Archive SQL prepare failed: {}", e))?;
    let col_names: Vec<String> = stmt.column_names().iter().map(|s| (*s).to_string()).collect();
    let rows = stmt
        .query_map([], |row| row_to_json_map(row, &col_names).map(Value::Object))
        .map_err(|e| format!("Archive SQL query failed: {}", e))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("Archive row decode failed: {}", e))?);
    }
    Ok(out)
}

fn extract_author_name(post: &Value, fallback: &str) -> String {
    let Some(author) = post.get("author").and_then(Value::as_object) else {
        return fallback.to_string();
    };
    let first = author.get("name").and_then(Value::as_str).unwrap_or("").trim();
    let last = author.get("last_name").and_then(Value::as_str).unwrap_or("").trim();
    let full = format!("{} {}", first, last).trim().to_string();
    if full.is_empty() {
        fallback.to_string()
    } else {
        full
    }
}

/// Runs live synchronization against `msidergisi.com` into SQLite and `live_msi_archive.json`.
pub async fn run_sync_live(
    cfg: &AppConfig,
    database: &Database,
    opts: SyncLiveOptions,
) -> Result<(), String> {
    let base_url = std::env::var("MSI_SYNC_BASE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let clean_base = base_url.trim_end_matches('/').to_string();
    let public_dir = PathBuf::from(&cfg.static_dir);
    let archive_path = std::env::var("MSI_ARCHIVE_JSON_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "storage/seed/live_msi_archive.json".to_string());

    let client = build_http_client()?;

    println!("[1/6] Fetching sitemap & homepages from {} ...", clean_base);
    let sm_bytes = http_get_bytes(&client, &format!("{}/sitemap.xml", clean_base), 2).await?;
    let sm_xml = String::from_utf8_lossy(&sm_bytes);
    let sm_urls = extract_sitemap_urls(&sm_xml);
    let mut discovery = classify_sitemap_urls(&sm_urls);

    let home_tr = fetch_inertia_props(&client, &format!("{}/", clean_base)).await?;
    let home_en = fetch_inertia_props(&client, &format!("{}/en", clean_base)).await?;

    let mut cat_meta: BTreeMap<String, CategoryMeta> = BTreeMap::new();
    let mut tr_placements: HashMap<String, HashSet<String>> = HashMap::new();
    let mut en_placements: HashMap<String, HashSet<String>> = HashMap::new();

    collect_home_section_data(
        &home_tr,
        &clean_base,
        false,
        &mut cat_meta,
        &mut discovery,
        &mut tr_placements,
    );
    collect_home_section_data(
        &home_en,
        &clean_base,
        true,
        &mut cat_meta,
        &mut discovery,
        &mut en_placements,
    );

    println!(
        "  Discovered TR sitemap slugs={}, EN sitemap slugs={}, TR cats={}, EN cats={}",
        discovery.tr_slugs.len(),
        discovery.en_slugs.len(),
        discovery.tr_category_urls.len(),
        discovery.en_category_urls.len()
    );

    println!("[2/6] Crawling all paginated TR & EN category archives ...");
    let tr_crawl = crawl_categories_batch(&client, &discovery.tr_category_urls, false).await;
    let en_crawl = crawl_categories_batch(&client, &discovery.en_category_urls, true).await;

    for res in tr_crawl.into_iter().chain(en_crawl.into_iter()) {
        if let Some(err) = res.error {
            let lang_lbl = if res.is_en { "EN" } else { "TR" };
            println!("  WARN {} cat {}: {}", lang_lbl, res.slug, err);
            continue;
        }
        let entry = cat_meta.entry(res.slug.clone()).or_default();
        if let Some(name) = res.cat_name {
            if res.is_en {
                entry.name_en.get_or_insert(name);
            } else {
                entry.name_tr.get_or_insert(name);
            }
        }
        let count = res.post_slugs.len();
        for p_slug in res.post_slugs {
            if res.is_en {
                discovery.en_slugs.insert(p_slug.clone());
                en_placements.entry(p_slug).or_default().insert(res.slug.clone());
            } else {
                discovery.tr_slugs.insert(p_slug.clone());
                tr_placements.entry(p_slug).or_default().insert(res.slug.clone());
            }
        }
        let lang_lbl = if res.is_en { "EN" } else { "TR" };
        println!("  {} /{}: {} items", lang_lbl, res.slug, count);
    }

    let mut conn = database
        .open_conn()
        .map_err(|e| format!("Failed to open SQLite connection: {}", e))?;
    let has_en_status = table_has_column(&conn, "contents", "en_status");

    println!("[3/6] Upserting live categories ...");
    {
        let mut existing_cat_slugs = HashSet::new();
        {
            let mut stmt = conn
                .prepare("SELECT slug FROM categories")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for r in rows.flatten() {
                existing_cat_slugs.insert(r);
            }
        }

        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let mut next_sort = 50i64;
        for (cslug, info) in &cat_meta {
            let ntr = info.name_tr.as_deref().unwrap_or(cslug);
            let nen = info.name_en.as_deref().unwrap_or(ntr);
            let dtr = info.desc_tr.as_deref().unwrap_or("");
            let gtype = if [
                "idef", "igef", "saha", "sedec", "amac", "nato", "milmast", "ni",
            ]
            .iter()
            .any(|k| cslug.contains(k))
            {
                "etkinlik-dosyasi"
            } else {
                "icerikler"
            };

            if !existing_cat_slugs.contains(cslug) {
                tx.execute(
                    "INSERT OR IGNORE INTO categories
                     (slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic)
                     VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, 'fa-newspaper-o', ?7, 1)",
                    params![cslug, ntr, nen, gtype, dtr, nen, next_sort],
                )
                .map_err(|e| e.to_string())?;
                next_sort += 1;
            } else if matches!(
                cslug.as_str(),
                "highlights"
                    | "feature"
                    | "ozel-dosya"
                    | "advertorials"
                    | "basyazilar"
                    | "kapak-dosyasi"
                    | "yeni-sayi-kapak"
            ) {
                tx.execute(
                    "UPDATE categories SET name_tr = COALESCE(NULLIF(name_tr, ''), ?1), name_en = ?2 WHERE slug = ?3",
                    params![ntr, nen, cslug],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    let mut db_by_slug: HashMap<String, ExistingContentRow> = HashMap::new();
    let mut db_by_norm_title: HashMap<String, ExistingContentRow> = HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, slug, title_tr, body_tr, body_en, source_group FROM contents")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ExistingContentRow {
                    id: r.get(0)?,
                    slug: r.get(1)?,
                    title_tr: r.get(2)?,
                    body_tr: r.get(3)?,
                    body_en: r.get(4)?,
                    source_group: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;

        for row in rows.flatten() {
            let nt = norm_title(&row.title_tr);
            if !nt.is_empty() {
                db_by_norm_title.entry(nt).or_insert_with(|| row.clone());
            }
            db_by_slug.insert(row.slug.clone(), row);
        }
    }

    let mut tr_to_fetch: Vec<String> = discovery
        .tr_slugs
        .iter()
        .filter(|sl| match db_by_slug.get(sl.as_str()) {
            None => true,
            Some(row) => row.body_tr.trim().len() < 80,
        })
        .cloned()
        .collect();

    if let Some(lim) = opts.limit {
        tr_to_fetch.truncate(lim);
    }

    println!(
        "[4/6] Fetching {} missing/short TR articles (limit={:?}) ...",
        tr_to_fetch.len(),
        opts.limit
    );
    let tr_posts_data = fetch_posts_batch(&client, &clean_base, &tr_to_fetch, false).await;

    let mut downloaded_covers: HashMap<String, String> = HashMap::new();
    for (sl, post) in &tr_posts_data {
        if let Some(url) = pick_image_url(post.get("img"), &clean_base) {
            if let Some(rel) = download_image_for_slug(&client, &public_dir, sl, &url).await {
                downloaded_covers.insert(sl.clone(), rel);
            }
        }
    }

    let mut created_tr = 0usize;
    let mut reconciled_hash = 0usize;
    {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for (sl, post) in &tr_posts_data {
            let title = post.get("title").and_then(Value::as_str).unwrap_or("").trim();
            let summary = post.get("subTitle").and_then(Value::as_str).unwrap_or("").trim();
            let body = post.get("content").and_then(Value::as_str).unwrap_or("").trim();
            let cat_slug = post
                .get("category")
                .and_then(|c| c.get("slug"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("haberler");
            tr_placements
                .entry(sl.clone())
                .or_default()
                .insert(cat_slug.to_string());
            let pls = tr_placements.get(sl).cloned().unwrap_or_default();
            let (ct, sg) = classify_content_type_and_source(cat_slug, &pls, None);
            let is_sponsored: i64 =
                if ct.contains("advertorial") || pls.contains("advertorials") {
                    1
                } else {
                    0
                };
            let pub_at = parse_published_at(
                post.get("publishedAt").and_then(Value::as_str),
                post.get("date").and_then(Value::as_str),
            );
            let author_name = extract_author_name(post, "MSI Editöryal Masası");
            let cover = downloaded_covers
                .get(sl)
                .map(String::as_str)
                .unwrap_or(FALLBACK_COVER_IMAGE);
            let read_min = estimate_read_time(body);
            let nt = norm_title(title);

            if let Some(existing) = db_by_slug.get_mut(sl) {
                tx.execute(
                    "UPDATE contents
                     SET title_tr = ?1, summary_tr = ?2, body_tr = ?3, content_type = ?4, source_group = ?5,
                         cover_image = CASE WHEN cover_image = '' THEN ?6 ELSE cover_image END,
                         published_at = ?7, is_sponsored = MAX(is_sponsored, ?8)
                     WHERE slug = ?9",
                    params![title, summary, body, ct, sg, cover, pub_at, is_sponsored, sl],
                )
                .map_err(|e| e.to_string())?;
                existing.title_tr = title.to_string();
                existing.body_tr = body.to_string();
            } else if !nt.is_empty() && db_by_norm_title.contains_key(&nt) {
                let old_row = db_by_norm_title.get(&nt).cloned().unwrap();
                let old_slug = old_row.slug.clone();
                tx.execute(
                    "UPDATE contents
                     SET slug = ?1, title_tr = ?2, summary_tr = COALESCE(NULLIF(?3, ''), summary_tr),
                         body_tr = CASE WHEN length(?4) > length(body_tr) THEN ?4 ELSE body_tr END,
                         published_at = ?5, is_sponsored = MAX(is_sponsored, ?6)
                     WHERE id = ?7",
                    params![sl, title, summary, body, pub_at, is_sponsored, old_row.id],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT OR IGNORE INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?1, ?2, 301, 0)",
                    params![format!("/icerik/{}", old_slug), format!("/icerik/{}", sl)],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT OR IGNORE INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?1, ?2, 301, 0)",
                    params![format!("/haberler/{}", old_slug), format!("/icerik/{}", sl)],
                )
                .map_err(|e| e.to_string())?;

                db_by_slug.remove(&old_slug);
                let mut updated_row = old_row;
                updated_row.slug = sl.clone();
                updated_row.title_tr = title.to_string();
                if body.len() > updated_row.body_tr.len() {
                    updated_row.body_tr = body.to_string();
                }
                db_by_slug.insert(sl.clone(), updated_row.clone());
                db_by_norm_title.insert(nt, updated_row);
                reconciled_hash += 1;
            } else {
                let sponsor_lbl = if is_sponsored == 1 {
                    Some("ADVERTORIAL")
                } else {
                    None
                };
                tx.execute(
                    "INSERT INTO contents (
                        slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                        content_type, source_group, cover_image, author_name, author_title,
                        issue_number, published_at, read_time_min, visibility_level, single_price_try,
                        is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                        view_count, true_read_count, status
                    ) VALUES (?1, ?2, '', ?3, '', ?4, '', ?5, ?6, ?7, ?8, 'Savunma Sanayii Analisti',
                              NULL, ?9, ?10, 0, 0.0, ?11, NULL, ?12, 0, NULL, 1, 1, 'published')",
                    params![
                        sl,
                        title,
                        summary,
                        body,
                        ct,
                        sg,
                        cover,
                        author_name,
                        pub_at,
                        read_min,
                        is_sponsored,
                        sponsor_lbl
                    ],
                )
                .map_err(|e| e.to_string())?;
                let new_id = tx.last_insert_rowid();
                let new_row = ExistingContentRow {
                    id: new_id,
                    slug: sl.clone(),
                    title_tr: title.to_string(),
                    body_tr: body.to_string(),
                    body_en: String::new(),
                    source_group: sg,
                };
                if !nt.is_empty() {
                    db_by_norm_title.insert(nt, new_row.clone());
                }
                db_by_slug.insert(sl.clone(), new_row);
                created_tr += 1;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
    }
    println!(
        "  TR inserted={}, hashed-slug reconciled={}",
        created_tr, reconciled_hash
    );

    let mut en_to_fetch: Vec<String> = discovery
        .en_slugs
        .iter()
        .filter(|sl| match db_by_slug.get(sl.as_str()) {
            None => true,
            Some(row) => {
                (row.source_group != "msi-tdr" && !discovery.tr_slugs.contains(sl.as_str()))
                    || row.body_en.trim().len() < 80
            }
        })
        .cloned()
        .collect();

    if let Some(lim) = opts.limit {
        let remaining = lim.saturating_sub(tr_to_fetch.len());
        en_to_fetch.truncate(remaining);
    }

    println!(
        "  Fetching {} EN articles for creation/reconciliation ...",
        en_to_fetch.len()
    );
    let en_posts_data = fetch_posts_batch(&client, &clean_base, &en_to_fetch, true).await;

    let mut en_downloaded_covers: HashMap<String, String> = HashMap::new();
    for (sl, post) in &en_posts_data {
        if !db_by_slug.contains_key(sl) {
            if let Some(url) = pick_image_url(post.get("img"), &clean_base) {
                if let Some(rel) = download_image_for_slug(&client, &public_dir, sl, &url).await {
                    en_downloaded_covers.insert(sl.clone(), rel);
                }
            }
        }
    }

    let mut created_en = 0usize;
    let mut updated_en = 0usize;
    {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for (sl, post) in &en_posts_data {
            let en_title = post.get("title").and_then(Value::as_str).unwrap_or("").trim();
            let en_summary = post.get("subTitle").and_then(Value::as_str).unwrap_or("").trim();
            let en_body = post.get("content").and_then(Value::as_str).unwrap_or("").trim();
            let cat_slug = post
                .get("category")
                .and_then(|c| c.get("slug"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("haberler");
            en_placements
                .entry(sl.clone())
                .or_default()
                .insert(cat_slug.to_string());
            let pls = en_placements.get(sl).cloned().unwrap_or_default();
            let (ct, _) = classify_content_type_and_source(cat_slug, &pls, None);
            let is_sponsored: i64 =
                if ct.contains("advertorial") || pls.contains("advertorials") {
                    1
                } else {
                    0
                };
            let pub_at = parse_published_at(
                post.get("publishedAt").and_then(Value::as_str),
                post.get("date").and_then(Value::as_str),
            );
            let author_name = extract_author_name(post, "MSI TDR Editorial Desk");
            let read_min = estimate_read_time(en_body);

            if db_by_slug.contains_key(sl) {
                let is_bilingual_shared = discovery.tr_slugs.contains(sl);
                if is_bilingual_shared {
                    tx.execute(
                        "UPDATE contents SET title_en = ?1, summary_en = ?2, body_en = ?3 WHERE slug = ?4",
                        params![en_title, en_summary, en_body, sl],
                    )
                    .map_err(|e| e.to_string())?;
                    if has_en_status && !en_title.is_empty() && !en_body.is_empty() {
                        tx.execute(
                            "UPDATE contents SET en_status = 'approved', en_translated_at = ?1 WHERE slug = ?2 AND title_tr != ?3",
                            params![Utc::now().to_rfc3339(), sl, en_title],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                } else {
                    tx.execute(
                        "UPDATE contents
                         SET title_tr = COALESCE(NULLIF(title_tr, ''), ?1),
                             title_en = ?1,
                             summary_tr = COALESCE(NULLIF(summary_tr, ''), ?2),
                             summary_en = ?2,
                             body_tr = COALESCE(NULLIF(body_tr, ''), ?3),
                             body_en = ?3,
                             source_group = 'msi-tdr',
                             content_type = ?4,
                             published_at = COALESCE(NULLIF(published_at, ''), ?5)
                         WHERE slug = ?6",
                        params![en_title, en_summary, en_body, ct, pub_at, sl],
                    )
                    .map_err(|e| e.to_string())?;
                    if has_en_status && !en_title.is_empty() && !en_body.is_empty() {
                        tx.execute(
                            "UPDATE contents SET en_status = 'approved', en_translated_at = ?1 WHERE slug = ?2",
                            params![Utc::now().to_rfc3339(), sl],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
                updated_en += 1;
            } else {
                let cover = en_downloaded_covers
                    .get(sl)
                    .map(String::as_str)
                    .unwrap_or(FALLBACK_COVER_IMAGE);
                let sponsor_lbl = if is_sponsored == 1 {
                    Some("ADVERTORIAL")
                } else {
                    None
                };
                tx.execute(
                    "INSERT INTO contents (
                        slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                        content_type, source_group, cover_image, author_name, author_title,
                        issue_number, published_at, read_time_min, visibility_level, single_price_try,
                        is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                        view_count, true_read_count, status
                    ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, 'msi-tdr', ?6, ?7, 'Defence Industry Analyst',
                              NULL, ?8, ?9, 0, 0.0, ?10, NULL, ?11, 0, NULL, 1, 1, 'published')",
                    params![
                        sl,
                        en_title,
                        en_summary,
                        en_body,
                        ct,
                        cover,
                        author_name,
                        pub_at,
                        read_min,
                        is_sponsored,
                        sponsor_lbl
                    ],
                )
                .map_err(|e| e.to_string())?;
                if has_en_status && !en_title.is_empty() && !en_body.is_empty() {
                    tx.execute(
                        "UPDATE contents SET en_status = 'approved', en_translated_at = ?1 WHERE slug = ?2",
                        params![Utc::now().to_rfc3339(), sl],
                    )
                    .map_err(|e| e.to_string())?;
                }
                created_en += 1;
            }
        }

        for sl in &discovery.en_slugs {
            if !discovery.tr_slugs.contains(sl) {
                tx.execute(
                    "UPDATE contents SET source_group = 'msi-tdr' WHERE slug = ?1 AND source_group != 'msi-tdr'",
                    params![sl],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        // Sync placements & advertorial flag
        let mut slug_to_id: HashMap<String, i64> = HashMap::new();
        {
            let mut stmt = tx
                .prepare("SELECT id, slug FROM contents")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                .map_err(|e| e.to_string())?;
            for (id, slug) in rows.flatten() {
                slug_to_id.insert(slug, id);
            }
        }

        let mut pl_count = 0usize;
        for (sl, pset) in tr_placements.iter().chain(en_placements.iter()) {
            let Some(&cid) = slug_to_id.get(sl) else {
                continue;
            };
            for pslug in pset {
                if pslug.trim().is_empty() {
                    continue;
                }
                let changed = tx
                    .execute(
                        "INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)",
                        params![cid, pslug],
                    )
                    .map_err(|e| e.to_string())?;
                pl_count += changed;
            }
        }

        tx.execute(
            "UPDATE contents SET is_sponsored = 1 WHERE content_type IN ('advertorial', 'advertorials')",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        println!(
            "  EN created={}, updated={}; added {} new content_placements",
            created_en, updated_en, pl_count
        );
    }

    println!("[5/6] Auditing all cover images on disk under {} ...", public_dir.display());
    let mut broken_rows: Vec<(i64, String)> = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, slug, cover_image FROM contents")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for (cid, sl, cover) in rows.flatten() {
            let trimmed = cover.trim_start_matches('/');
            let valid = !trimmed.is_empty()
                && fs::metadata(public_dir.join(trimmed))
                    .map(|m| m.len() >= 100)
                    .unwrap_or(false);
            if !valid {
                broken_rows.push((cid, sl));
            }
        }
    }

    if let Some(lim) = opts.limit {
        broken_rows.truncate(lim);
    }

    let mut repaired_imgs = 0usize;
    if !broken_rows.is_empty() {
        let sem = Arc::new(Semaphore::new(6));
        let mut join_set = JoinSet::new();
        for (cid, sl) in broken_rows {
            let client = client.clone();
            let sem = Arc::clone(&sem);
            let public_dir = public_dir.clone();
            let clean_base = clean_base.clone();
            let is_en = discovery.en_slugs.contains(&sl) && !discovery.tr_slugs.contains(&sl);
            join_set.spawn(async move {
                let _permit = sem.acquire_owned().await;
                let url = format!(
                    "{}{}/haberler/{}",
                    clean_base,
                    if is_en { "/en" } else { "" },
                    urlencoding::encode(&sl)
                );
                if let Ok(props) = fetch_inertia_props(&client, &url).await {
                    if let Some(img_url) =
                        pick_image_url(props.get("post").and_then(|p| p.get("img")), &clean_base)
                    {
                        if let Some(rel) =
                            download_image_for_slug(&client, &public_dir, &sl, &img_url).await
                        {
                            return (cid, rel);
                        }
                    }
                }
                (cid, FALLBACK_COVER_IMAGE.to_string())
            });
        }

        let tx = conn.transaction().map_err(|e| e.to_string())?;
        while let Some(res) = join_set.join_next().await {
            if let Ok((cid, new_rel)) = res {
                tx.execute(
                    "UPDATE contents SET cover_image = ?1 WHERE id = ?2",
                    params![new_rel, cid],
                )
                .map_err(|e| e.to_string())?;
                repaired_imgs += 1;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    {
        let mut broken_issue_slugs = Vec::new();
        {
            let mut stmt = conn
                .prepare("SELECT slug, cover_image FROM magazine_issues")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .map_err(|e| e.to_string())?;
            for (slug, cover) in rows.flatten() {
                let trimmed = cover.trim_start_matches('/');
                let valid = !trimmed.is_empty()
                    && fs::metadata(public_dir.join(trimmed))
                        .map(|m| m.len() >= 100)
                        .unwrap_or(false);
                if !valid {
                    broken_issue_slugs.push(slug);
                }
            }
        }
        if !broken_issue_slugs.is_empty() {
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            for slug in broken_issue_slugs {
                tx.execute(
                    "UPDATE magazine_issues SET cover_image = ?1 WHERE slug = ?2",
                    params![FALLBACK_COVER_IMAGE, slug],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())?;
        }
    }
    println!("  Repaired missing cover images={}", repaired_imgs);

    println!("[6/6] Regenerating {} & checkpointing WAL ...", archive_path);
    let cats_out = query_rows_as_json(
        &conn,
        "SELECT slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic FROM categories ORDER BY sort_order, id",
    )?;
    let issues_out = query_rows_as_json(
        &conn,
        "SELECT issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en, year, cover_image, is_special_issue, event_name, page_count, dossier_count, summary_tr, summary_en, pdf_path, spreads_json FROM magazine_issues ORDER BY issue_number",
    )?;

    let mut pls_map: HashMap<i64, Vec<Value>> = HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT content_id, category_slug FROM content_placements")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for (cid, cat) in rows.flatten() {
            pls_map.entry(cid).or_default().push(Value::String(cat));
        }
    }

    let mut ssb_map: HashMap<i64, Vec<Value>> = HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT content_id, ssb_code FROM content_ssb_tags")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for (cid, code) in rows.flatten() {
            ssb_map.entry(cid).or_default().push(Value::String(code));
        }
    }

    let mut contents_out = query_rows_as_json(&conn, "SELECT * FROM contents ORDER BY id")?;
    for item in &mut contents_out {
        if let Some(obj) = item.as_object_mut() {
            let cid = obj.get("id").and_then(Value::as_i64).unwrap_or(0);
            obj.insert(
                "placements".to_string(),
                Value::Array(pls_map.remove(&cid).unwrap_or_default()),
            );
            obj.insert(
                "ssb_tags".to_string(),
                Value::Array(ssb_map.remove(&cid).unwrap_or_default()),
            );
        }
    }

    let redirects_out = query_rows_as_json(
        &conn,
        "SELECT old_path, new_path, status_code FROM seo_redirects ORDER BY id",
    )?;

    let archive_doc = serde_json::json!({
        "exported_at": Utc::now().to_rfc3339(),
        "source": clean_base,
        "categories": cats_out,
        "issues": issues_out,
        "contents": contents_out,
        "seo_redirects": redirects_out,
    });

    let archive_file = PathBuf::from(&archive_path);
    if let Some(parent) = archive_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed creating archive dir: {}", e))?;
    }
    let serialized = serde_json::to_string(&archive_doc)
        .map_err(|e| format!("Failed serializing archive JSON: {}", e))?;
    fs::write(&archive_file, serialized)
        .map_err(|e| format!("Failed writing archive JSON: {}", e))?;

    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| format!("WAL checkpoint failed: {}", e))?;

    let total_tr: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contents WHERE source_group != 'msi-tdr'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let total_en: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contents WHERE source_group = 'msi-tdr'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let total_all: i64 = conn
        .query_row("SELECT COUNT(*) FROM contents", [], |r| r.get(0))
        .unwrap_or(0);

    drop(conn);
    database.refresh_l1_cache();

    println!(
        "SYNC COMPLETE: total contents={} (TR/bilingual={}, EN MSI TDR={}), archive={}",
        total_all, total_tr, total_en, archive_path
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_html_entities_and_strip_tags() {
        let raw = "<p>T&uuml;rkiye&#39;nin &quot;KAAN&quot; Projesi &amp; &Scedil;ahin &#x2014; 100&#37; Yerli</p>";
        assert_eq!(
            decode_html_entities(raw),
            "<p>Türkiye'nin \"KAAN\" Projesi & Şahin — 100% Yerli</p>"
        );
        assert_eq!(
            strip_html_tags(raw),
            "Türkiye'nin \"KAAN\" Projesi & Şahin — 100% Yerli"
        );
        // Must not double-decode &amp;lt;
        assert_eq!(decode_html_entities("&amp;lt;script&amp;gt;"), "&lt;script&gt;");
    }

    #[test]
    fn test_extract_slug_from_url_and_sitemap_classification() {
        assert_eq!(
            extract_slug_from_url("https://msidergisi.com/haberler/kaan-ilk-ucus/"),
            Some("kaan-ilk-ucus".to_string())
        );
        assert_eq!(
            extract_slug_from_url("https://msidergisi.com/en/haberler/kaan-first-flight?ref=1#top"),
            Some("kaan-first-flight".to_string())
        );
        assert_eq!(extract_slug_from_url("https://msidergisi.com/"), None);

        let xml = r#"
            <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
              <url><loc>https://msidergisi.com/haberler/aselsan-aesa-radar/</loc></url>
              <url><loc>https://msidergisi.com/en/haberler/aselsan-aesa-radar-en</loc></url>
              <url><loc>https://msidergisi.com/kategoriler/kapak-dosyasi/</loc></url>
              <url><loc>https://msidergisi.com/en/kategoriler/cover-stories?utm=1</loc></url>
            </urlset>
        "#;
        let urls = extract_sitemap_urls(xml);
        assert_eq!(urls.len(), 4);
        let disc = classify_sitemap_urls(&urls);
        assert!(disc.tr_slugs.contains("aselsan-aesa-radar"));
        assert!(disc.en_slugs.contains("aselsan-aesa-radar-en"));
        assert!(disc
            .tr_category_urls
            .contains("https://msidergisi.com/kategoriler/kapak-dosyasi"));
        assert!(disc
            .en_category_urls
            .contains("https://msidergisi.com/en/kategoriler/cover-stories"));
    }

    #[test]
    fn test_extract_inertia_props_script_and_attr() {
        let html_script = r#"<html><script type="application/json" data-page="app">{"component":"Post","props":{"post":{"slug":"test-slug","title":"Test"}}}</script></html>"#;
        let props1 = extract_inertia_props(html_script).unwrap();
        assert_eq!(props1["post"]["slug"], "test-slug");

        let html_attr = r#"<div id="app" data-page="{&quot;props&quot;:{&quot;post&quot;:{&quot;slug&quot;:&quot;attr-slug&quot;}}}"></div>"#;
        let props2 = extract_inertia_props(html_attr).unwrap();
        assert_eq!(props2["post"]["slug"], "attr-slug");
    }

    #[test]
    fn test_parse_sync_live_args_and_helpers() {
        assert_eq!(parse_sync_live_args(["msi-server"]), None);
        assert_eq!(
            parse_sync_live_args(["msi-server", "--sync-live"]),
            Some(SyncLiveOptions { limit: None })
        );
        assert_eq!(
            parse_sync_live_args(["msi-server", "--sync-live", "--limit=50"]),
            Some(SyncLiveOptions { limit: Some(50) })
        );
        assert_eq!(
            parse_sync_live_args(["msi-server", "--sync-live", "--limit", "25"]),
            Some(SyncLiveOptions { limit: Some(25) })
        );

        assert_eq!(
            norm_title("TÜRK Savunma Sanayii: İHA &amp; SİHA!"),
            "turk savunma sanayii iha amp siha"
        );
        assert_eq!(
            parse_published_at(Some("2026-11-18T14:30:05.000Z"), None),
            "2026-11-18 14:30:05"
        );
        assert_eq!(
            parse_published_at(None, Some("18.11.2026")),
            "2026-11-18 12:00:00"
        );
        assert_eq!(estimate_read_time("<p>Tek kelime</p>"), 1);

        let mut pls = HashSet::new();
        pls.insert("ozel-dosya".to_string());
        assert_eq!(
            classify_content_type_and_source("haberler", &pls, None),
            ("haberler".to_string(), "dergi-icerigi".to_string())
        );

        assert!(detect_captcha_marker("text/html", b"<html>cf-turnstile</html>").is_some());
        assert!(detect_captcha_marker("image/webp", b"cf-turnstile").is_none());
    }
}
