//! MSI Dergisi Editorial Studio shared models, block/HTML parsers, SEO engine, and RBAC capabilities.

use serde::{Deserialize, Serialize};

/// Granular editorial capability flags derived from MSI Dergisi RBAC permissions for the WASM UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorialCapabilities {
    #[serde(default)]
    pub can_write: bool,
    #[serde(default)]
    pub can_publish: bool,
    #[serde(default)]
    pub can_delete: bool,
    #[serde(default)]
    pub can_upload_media: bool,
    #[serde(default)]
    pub can_delete_media: bool,
    #[serde(default)]
    pub can_manage_categories: bool,
    #[serde(default)]
    pub can_delete_categories: bool,
    #[serde(default)]
    pub can_manage_issues: bool,
    #[serde(default)]
    pub can_delete_issues: bool,
    #[serde(default)]
    pub can_manage_glossary: bool,
    #[serde(default)]
    pub can_manage_redirects: bool,
    #[serde(default)]
    pub can_view_analytics: bool,
    #[serde(default)]
    pub can_access_admin: bool,
}

fn canonicalize_editorial_resource(res: &str) -> &str {
    match res {
        "content" => "contents",
        "category" => "categories",
        "issue" => "issues",
        "redirect" => "redirects",
        "approval" => "approvals",
        "setting" => "settings",
        "translation" => "translations",
        other => other,
    }
}

/// Matches a granted MSI RBAC permission (including `*`, `<res>:*`, `<res>:manage`,
/// singular/plural aliases, and `publish -> write -> read` action hierarchy).
pub fn editorial_perm_matches(granted_raw: &str, required_raw: &str) -> bool {
    let granted = granted_raw.trim().to_ascii_lowercase().replace('.', ":");
    let required = required_raw.trim().to_ascii_lowercase().replace('.', ":");
    if granted.is_empty() || required.is_empty() {
        return false;
    }
    if granted == "*" || granted == required {
        return true;
    }
    let (g_res_raw, g_act) = match granted.split_once(':') {
        Some((r, a)) => (r.trim(), a.trim()),
        None => (granted.as_str(), "*"),
    };
    let (r_res_raw, r_act) = match required.split_once(':') {
        Some((r, a)) => (r.trim(), a.trim()),
        None => (required.as_str(), "*"),
    };
    if canonicalize_editorial_resource(g_res_raw) != canonicalize_editorial_resource(r_res_raw) {
        return false;
    }
    if g_act == "*" || g_act == "manage" || g_act == r_act {
        return true;
    }
    match r_act {
        "read" | "view" => matches!(g_act, "read" | "view" | "write" | "publish" | "upload"),
        "write" => matches!(g_act, "write" | "publish"),
        _ => false,
    }
}

impl EditorialCapabilities {
    /// Builds editorial capabilities from an operator's effective MSI permission list.
    pub fn from_permissions(perms: &[String]) -> Self {
        let has = |req: &str| perms.iter().any(|item| editorial_perm_matches(item, req));
        Self {
            can_write: has("contents:write"),
            can_publish: has("contents:publish"),
            can_delete: has("contents:delete"),
            can_upload_media: has("media:upload"),
            can_delete_media: has("media:delete"),
            can_manage_categories: has("categories:write"),
            can_delete_categories: has("categories:delete"),
            can_manage_issues: has("issues:write"),
            can_delete_issues: has("issues:delete"),
            can_manage_glossary: has("glossary:write") || has("glossary:manage"),
            can_manage_redirects: has("redirects:write"),
            can_view_analytics: has("analytics:read"),
            can_access_admin: has("admin:access") || has("contents:read"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct User {
    pub id: i32,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    pub role: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub capabilities: EditorialCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryTreeItem {
    pub category: Category,
    pub depth: usize,
    pub path_label: String,
}

/// Orders flat categories into a cycle-safe hierarchical tree (`Ana Kategori`, `Ana › Alt`, `Ana › Alt › Detay`).
pub fn build_category_tree(categories: &[Category]) -> Vec<CategoryTreeItem> {
    let mut sorted = categories.to_vec();
    sorted.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });

    let valid_ids: std::collections::HashSet<i32> = sorted.iter().map(|c| c.id).collect();
    let mut visited = std::collections::HashSet::with_capacity(sorted.len());
    let mut out = Vec::with_capacity(sorted.len());

    fn visit_children(
        parent: Option<i32>,
        depth: usize,
        prefix: &str,
        all: &[Category],
        valid_ids: &std::collections::HashSet<i32>,
        visited: &mut std::collections::HashSet<i32>,
        out: &mut Vec<CategoryTreeItem>,
    ) {
        for cat in all {
            let effective_parent = cat
                .parent_id
                .filter(|pid| *pid != cat.id && valid_ids.contains(pid));
            if effective_parent == parent && visited.insert(cat.id) {
                let path_label = if prefix.is_empty() {
                    cat.name.clone()
                } else {
                    format!("{prefix} › {}", cat.name)
                };
                out.push(CategoryTreeItem {
                    category: cat.clone(),
                    depth,
                    path_label: path_label.clone(),
                });
                visit_children(
                    Some(cat.id),
                    depth + 1,
                    &path_label,
                    all,
                    valid_ids,
                    visited,
                    out,
                );
            }
        }
    }

    visit_children(None, 0, "", &sorted, &valid_ids, &mut visited, &mut out);

    // Fallback safety: if malformed data had a cycle, still include unvisited nodes at root
    for cat in &sorted {
        if visited.insert(cat.id) {
            out.push(CategoryTreeItem {
                category: cat.clone(),
                depth: 0,
                path_label: cat.name.clone(),
            });
        }
    }

    out
}

/// Collects all descendant category IDs beneath `root_id` (cycle-safe).
pub fn collect_descendant_category_ids(categories: &[Category], root_id: i32) -> Vec<i32> {
    let mut descendants = Vec::new();
    let mut queue = vec![root_id];
    let mut seen = std::collections::HashSet::new();
    seen.insert(root_id);

    while let Some(cur) = queue.pop() {
        for c in categories {
            if c.parent_id == Some(cur) && seen.insert(c.id) {
                descendants.push(c.id);
                queue.push(c.id);
            }
        }
    }
    descendants
}

/// Returns `[top_root_id, ..., target_id]` ancestor chain from root down to `target_id`.
pub fn category_ancestor_chain(categories: &[Category], target_id: i32) -> Vec<i32> {
    let mut chain = Vec::new();
    let mut cur_opt = Some(target_id);
    let mut seen = std::collections::HashSet::new();

    while let Some(cid) = cur_opt {
        if !seen.insert(cid) {
            break;
        }
        let Some(cat) = categories.iter().find(|c| c.id == cid) else {
            break;
        };
        chain.push(cat.id);
        cur_opt = cat.parent_id.filter(|pid| *pid != cat.id);
    }

    chain.reverse();
    chain
}

/// Converts any Turkish or Unicode category/article title into a clean URL-safe slug.
pub fn slugify_turkish(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = false;

    for ch in input.trim().chars() {
        let mapped: Option<&str> = match ch {
            'ğ' | 'Ğ' => Some("g"),
            'ü' | 'Ü' => Some("u"),
            'ş' | 'Ş' => Some("s"),
            'ı' | 'I' | 'İ' => Some("i"),
            'ö' | 'Ö' => Some("o"),
            'ç' | 'Ç' => Some("c"),
            'â' | 'Â' => Some("a"),
            'î' | 'Î' => Some("i"),
            'û' | 'Û' => Some("u"),
            c if c.is_ascii_alphanumeric() => {
                out.push(c.to_ascii_lowercase());
                prev_dash = false;
                continue;
            }
            _ => None,
        };

        if let Some(rep) = mapped {
            out.push_str(rep);
            prev_dash = false;
        } else if !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    pub url: String,
    #[serde(default)]
    pub keywords: Option<String>,
    #[serde(default)]
    pub source_name: Option<String>,
    #[serde(default = "default_source_type")]
    pub source_type: String, // "tr" | "foreign"
    #[serde(default = "default_active_i8")]
    pub is_active: i8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

fn default_source_type() -> String {
    "tr".to_string()
}

fn default_active_i8() -> i8 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SuggestedSource {
    pub url: String,
    pub source_name: String,
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ContentBlock {
    pub id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SeoData {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub keywords: String,
    #[serde(default)]
    pub og_title: String,
    #[serde(default)]
    pub og_description: String,
    #[serde(default)]
    pub twitter_title: String,
    #[serde(default)]
    pub twitter_description: String,
    #[serde(default)]
    pub image_url: String,
}

/// Exact `Article` wire format supporting both snake_case DB columns and camelCase frontend fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub article_type: Option<String>, // "auto" | "xnews" | "published" | "draft"
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<ContentBlock>>,
    #[serde(default = "default_draft_status")]
    pub status: String, // "draft" | "pending_review" | "publish" | "future" | "archived"
    // Both snake_case and camelCase compatibility just like App.tsx & server.ts
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(rename = "sourceUrl", default, skip_serializing_if = "Option::is_none")]
    pub source_url_camel: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(rename = "imageUrl", default, skip_serializing_if = "Option::is_none")]
    pub image_url_camel: Option<String>,
    #[serde(default)]
    pub source_time: Option<String>,
    #[serde(rename = "sourceTime", default, skip_serializing_if = "Option::is_none")]
    pub source_time_camel: Option<String>,
    #[serde(default)]
    pub original_url: Option<String>,
    #[serde(rename = "originalUrl", default, skip_serializing_if = "Option::is_none")]
    pub original_url_camel: Option<String>,
    #[serde(default)]
    pub publish_date: Option<String>,
    #[serde(rename = "publishDate", default, skip_serializing_if = "Option::is_none")]
    pub publish_date_camel: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(rename = "createdAt", default, skip_serializing_if = "Option::is_none")]
    pub created_at_camel: Option<String>,
    #[serde(default)]
    pub wp_post_id: Option<i32>,
    #[serde(rename = "wpPostId", default, skip_serializing_if = "Option::is_none")]
    pub wp_post_id_camel: Option<i32>,
    #[serde(default)]
    pub wp_post_url: Option<String>,
    #[serde(rename = "wpPostUrl", default, skip_serializing_if = "Option::is_none")]
    pub wp_post_url_camel: Option<String>,
    #[serde(default)]
    pub wp_site_id: Option<i32>,
    #[serde(rename = "wpSiteId", default, skip_serializing_if = "Option::is_none")]
    pub wp_site_id_camel: Option<i32>,
    #[serde(default)]
    pub assigned_to: Option<i32>,
    #[serde(default)]
    pub publisher_name: Option<String>,
    #[serde(default)]
    pub scheduled_at: Option<String>,
    #[serde(rename = "scheduledAt", default, skip_serializing_if = "Option::is_none")]
    pub scheduled_at_camel: Option<String>,
    #[serde(rename = "targetSites", default, skip_serializing_if = "Option::is_none")]
    pub target_sites: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(rename = "isLiveBlog", alias = "is_live_blog", default)]
    pub is_live_blog: bool,
    #[serde(rename = "commentsLocked", alias = "comments_locked", default)]
    pub comments_locked: bool,
    #[serde(rename = "adsDisabled", alias = "ads_disabled", default)]
    pub ads_disabled: bool,
    #[serde(
        rename = "coverWidth",
        alias = "cover_width",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cover_width: Option<i32>,
    #[serde(
        rename = "coverHeight",
        alias = "cover_height",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cover_height: Option<i32>,
    #[serde(
        rename = "thumbUrl",
        alias = "thumb_url",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub thumb_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo: Option<SeoData>,
    #[serde(rename = "autoShare", default, skip_serializing_if = "Option::is_none")]
    pub auto_share: Option<bool>,
    #[serde(default)]
    pub views: i32,
    #[serde(default)]
    pub comments: i32,

    // Bilingual (`TR` / `EN` — MSI Dergisi & MSI TDR) fields
    #[serde(
        rename = "titleEn",
        alias = "title_en",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub title_en: Option<String>,
    #[serde(
        rename = "spotEn",
        alias = "spot_en",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub spot_en: Option<String>,
    #[serde(
        rename = "contentEn",
        alias = "content_en",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub content_en: Option<String>,
    #[serde(
        rename = "blocksEn",
        alias = "blocks_en",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub blocks_en: Option<Vec<ContentBlock>>,
    #[serde(rename = "enStatus", alias = "en_status", default = "default_en_status")]
    pub en_status: String,

    // Publication Channel, Format & Magazine Issue Binding
    #[serde(
        rename = "publicationChannel",
        alias = "publication_channel",
        default = "default_pub_channel"
    )]
    pub publication_channel: String,
    #[serde(
        rename = "contentType",
        alias = "content_type",
        default = "default_content_type"
    )]
    pub content_type: String,
    #[serde(
        rename = "issueNumber",
        alias = "issue_number",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub issue_number: Option<i32>,
    #[serde(
        rename = "issuePageRange",
        alias = "issue_page_range",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub issue_page_range: Option<String>,
    #[serde(rename = "isFeatured", alias = "is_featured", default)]
    pub is_featured: bool,

    // Author / Attribution & Media
    #[serde(
        rename = "authorName",
        alias = "author_name",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub author_name: Option<String>,
    #[serde(
        rename = "authorTitle",
        alias = "author_title",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub author_title: Option<String>,
    #[serde(
        rename = "videoUrl",
        alias = "video_url",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub video_url: Option<String>,
    #[serde(
        rename = "readTimeMin",
        alias = "read_time_min",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub read_time_min: Option<i32>,

    // Access Control (Paywall) & Sponsorship / Advertorial
    #[serde(rename = "visibilityLevel", alias = "visibility_level", default)]
    pub visibility_level: i32,
    #[serde(rename = "singlePriceTry", alias = "single_price_try", default)]
    pub single_price_try: f64,
    #[serde(rename = "isSponsored", alias = "is_sponsored", default)]
    pub is_sponsored: bool,
    #[serde(
        rename = "sponsorBrand",
        alias = "sponsor_brand",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sponsor_brand: Option<String>,
    #[serde(
        rename = "sponsorLegalLabel",
        alias = "sponsor_legal_label",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sponsor_legal_label: Option<String>,

    // SSB Defence Technology Taxonomy
    #[serde(
        rename = "ssbTags",
        alias = "ssb_tags",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ssb_tags: Option<Vec<String>>,
}

impl Default for Article {
    fn default() -> Self {
        Self {
            id: String::new(),
            article_type: None,
            title: String::new(),
            content: String::new(),
            blocks: None,
            status: String::new(),
            source_url: None,
            source_url_camel: None,
            image_url: None,
            image_url_camel: None,
            source_time: None,
            source_time_camel: None,
            original_url: None,
            original_url_camel: None,
            publish_date: None,
            publish_date_camel: None,
            created_at: None,
            created_at_camel: None,
            wp_post_id: None,
            wp_post_id_camel: None,
            wp_post_url: None,
            wp_post_url_camel: None,
            wp_site_id: None,
            wp_site_id_camel: None,
            assigned_to: None,
            publisher_name: None,
            scheduled_at: None,
            scheduled_at_camel: None,
            target_sites: None,
            categories: None,
            tags: None,
            spot: None,
            slug: None,
            is_live_blog: false,
            comments_locked: false,
            ads_disabled: false,
            cover_width: None,
            cover_height: None,
            thumb_url: None,
            seo: None,
            auto_share: None,
            views: 0,
            comments: 0,
            title_en: None,
            spot_en: None,
            content_en: None,
            blocks_en: None,
            en_status: default_en_status(),
            publication_channel: default_pub_channel(),
            content_type: default_content_type(),
            issue_number: None,
            issue_page_range: None,
            is_featured: false,
            author_name: None,
            author_title: None,
            video_url: None,
            read_time_min: None,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: false,
            sponsor_brand: None,
            sponsor_legal_label: None,
            ssb_tags: None,
        }
    }
}

pub fn default_en_status() -> String {
    "none".to_string()
}

pub fn default_pub_channel() -> String {
    "news".to_string()
}

pub fn default_content_type() -> String {
    "haberler".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SsbTag {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub code: String,
    #[serde(default, alias = "category_group")]
    pub category_group: String,
    #[serde(default, alias = "name_tr")]
    pub name_tr: String,
    #[serde(default, alias = "name_en")]
    pub name_en: String,
    #[serde(default, alias = "sort_order")]
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MagazineIssueItem {
    #[serde(default)]
    pub id: i32,
    #[serde(default, alias = "issue_number")]
    pub issue_number: i32,
    #[serde(default, alias = "edition_lang")]
    pub edition_lang: String, // "tr" | "en"
    #[serde(default, alias = "title_tr")]
    pub title_tr: String,
    #[serde(default, alias = "title_en")]
    pub title_en: String,
    #[serde(default, alias = "publish_month")]
    pub publish_month: String,
    #[serde(default)]
    pub year: i32,
    #[serde(default, alias = "is_special_issue")]
    pub is_special_issue: bool,
    #[serde(default, alias = "event_name", skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryRule {
    #[serde(default)]
    pub id: i32,
    #[serde(default, alias = "term_tr")]
    pub term_tr: String,
    #[serde(default, alias = "term_en")]
    pub term_en: String,
    #[serde(default, alias = "abbr_tr", skip_serializing_if = "Option::is_none")]
    pub abbr_tr: Option<String>,
    #[serde(default, alias = "abbr_en", skip_serializing_if = "Option::is_none")]
    pub abbr_en: Option<String>,
    #[serde(default)]
    pub category: String, // "style" | "institutional" | "technical"
    #[serde(default, alias = "rule_type")]
    pub rule_type: String, // "fixed" | "forbidden"
    #[serde(default, alias = "preferred_en")]
    pub preferred_en: String,
    #[serde(default, alias = "forbidden_variants")]
    pub forbidden_variants: Vec<String>,
    #[serde(default, alias = "case_sensitive")]
    pub case_sensitive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryViolation {
    pub rule_id: i32,
    pub matched_text: String,
    pub preferred_replacement: String,
    pub note: String,
    pub occurrences: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub id: i32,
    pub filename: String,
    pub original_name: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb_url: Option<String>,
    pub mime_type: String,
    pub width: i32,
    pub height: i32,
    pub size_bytes: i64,
    #[serde(default)]
    pub watermarked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uploaded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GalleryItem {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb_url: Option<String>,
    #[serde(default)]
    pub caption: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelatedArticleRef {
    pub article_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArticleLock {
    pub article_id: String,
    pub user_id: i32,
    pub username: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleRevision {
    pub id: i32,
    pub article_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_id: Option<i32>,
    pub editor_name: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub is_live_blog: bool,
    #[serde(default)]
    pub comments_locked: bool,
    #[serde(default)]
    pub ads_disabled: bool,
    #[serde(default, alias = "title_en", skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    #[serde(default, alias = "spot_en", skip_serializing_if = "Option::is_none")]
    pub spot_en: Option<String>,
    #[serde(default, alias = "content_en", skip_serializing_if = "Option::is_none")]
    pub content_en: Option<String>,
    #[serde(default = "default_en_status", alias = "en_status")]
    pub en_status: String,
    #[serde(default = "default_pub_channel", alias = "publication_channel")]
    pub publication_channel: String,
    #[serde(default = "default_content_type", alias = "content_type")]
    pub content_type: String,
    #[serde(default, alias = "issue_number", skip_serializing_if = "Option::is_none")]
    pub issue_number: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

impl Default for ArticleRevision {
    fn default() -> Self {
        Self {
            id: 0,
            article_id: String::new(),
            editor_id: None,
            editor_name: String::new(),
            title: String::new(),
            spot: None,
            slug: None,
            content: String::new(),
            image_url: None,
            status: String::new(),
            categories: None,
            tags: None,
            is_live_blog: false,
            comments_locked: false,
            ads_disabled: false,
            title_en: None,
            spot_en: None,
            content_en: None,
            en_status: default_en_status(),
            publication_channel: default_pub_channel(),
            content_type: default_content_type(),
            issue_number: None,
            note: None,
            created_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UrlRedirect {
    pub id: i32,
    pub old_slug: String,
    pub new_slug: String,
    pub article_id: String,
    pub status_code: i32,
    #[serde(default)]
    pub hits: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResearchSearchItem {
    pub title: String,
    pub url: String,
    pub source_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedReaderArticle {
    pub url: String,
    pub canonical_url: String,
    pub domain: String,
    pub site_name: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead_image: Option<String>,
    #[serde(default)]
    pub blocks: Vec<ContentBlock>,
    #[serde(default)]
    pub key_facts: Vec<String>,
    #[serde(default)]
    pub media_embeds: Vec<String>,
    #[serde(default)]
    pub word_count: usize,
    #[serde(default)]
    pub blocked_by_source: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResearchSource {
    pub id: i32,
    pub name: String,
    pub url: String,
    /// `"rss"` when `url` is an RSS/Atom feed, or `"site"` when `url` is a publisher website/domain.
    #[serde(default)]
    pub kind: String,
    /// Editorial category label (`"Gündem"`, `"Ajans"`, `"Ekonomi"`, `"Spor"`, `"Teknoloji"`, `"Dünya"`, `"Yerel"`).
    #[serde(default)]
    pub category: String,
    /// `"tr"` or `"global"`.
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeoDiscoverCheckItem {
    pub label: String,
    pub detail: String,
    pub level: CheckLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeoDiscoverReport {
    pub score: u8,
    pub checks: Vec<SeoDiscoverCheckItem>,
}

/// Evaluates real-time Google Discover, Google News & SEO readiness (`0-100` score + actionable checks).
pub fn evaluate_seo_discover(art: &Article) -> SeoDiscoverReport {
    let mut score: i32 = 0;
    let mut checks = Vec::with_capacity(6);

    // 1. Cover image & >= 1200px Google Discover requirement (25 pts)
    let has_img = art
        .effective_image_url()
        .map(|u| !u.trim().is_empty())
        .unwrap_or(false);
    match (has_img, art.cover_width) {
        (false, _) => checks.push(SeoDiscoverCheckItem {
            label: "Google Discover Kapak Görseli (Min. 1200px)".to_string(),
            detail: "Kapak görseli seçilmedi. Google Discover için en az 1200px genişlik şarttır."
                .to_string(),
            level: CheckLevel::Fail,
        }),
        (true, Some(w)) if w >= 1200 => {
            score += 25;
            checks.push(SeoDiscoverCheckItem {
                label: "Google Discover Kapak Görseli (Min. 1200px)".to_string(),
                detail: format!("Uygun ({w}px genişlik — Discover yüksek çözünürlük şartını karşılıyor)."),
                level: CheckLevel::Pass,
            });
        }
        (true, Some(w)) => {
            score += 10;
            checks.push(SeoDiscoverCheckItem {
                label: "Google Discover Kapak Görseli (Min. 1200px)".to_string(),
                detail: format!(
                    "Görsel genişliği {w}px (< 1200px). 2. adımda kırpıp kaydederek 1200x675'e yükseltin."
                ),
                level: CheckLevel::Warn,
            });
        }
        (true, None) => {
            score += 18;
            checks.push(SeoDiscoverCheckItem {
                label: "Google Discover Kapak Görseli (Min. 1200px)".to_string(),
                detail: "Kapak görseli mevcut (2. adımda 1200x675 optimizasyonu önerilir).".to_string(),
                level: CheckLevel::Pass,
            });
        }
    }

    // 2. Headline length & clickbait punctuation guard (20 pts)
    let effective_title = art
        .seo
        .as_ref()
        .map(|s| s.title.trim())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| art.title.trim());
    let title_chars = effective_title.chars().count();
    let clickbait_marks = effective_title.contains("!!!")
        || effective_title.contains("???")
        || effective_title.to_uppercase().contains("ŞOK ŞOK")
        || effective_title.to_uppercase().contains("İNANAMAYACAKSINIZ");

    if title_chars == 0 {
        checks.push(SeoDiscoverCheckItem {
            label: "Başlık Uzunluğu & Tıklama Tuzağı (Clickbait) Kontrolü".to_string(),
            detail: "Haber başlığı henüz girilmedi.".to_string(),
            level: CheckLevel::Fail,
        });
    } else if clickbait_marks {
        score += 5;
        checks.push(SeoDiscoverCheckItem {
            label: "Başlık Uzunluğu & Tıklama Tuzağı (Clickbait) Kontrolü".to_string(),
            detail: "Aşırı ünlem/clickbait ifadeleri Google Discover görünürlüğünü düşürür."
                .to_string(),
            level: CheckLevel::Warn,
        });
    } else if (35..=70).contains(&title_chars) {
        score += 20;
        checks.push(SeoDiscoverCheckItem {
            label: "Başlık Uzunluğu & Tıklama Tuzağı (Clickbait) Kontrolü".to_string(),
            detail: format!("İdeal uzunlukta ({title_chars} karakter) ve temiz haber dili."),
            level: CheckLevel::Pass,
        });
    } else {
        score += 12;
        checks.push(SeoDiscoverCheckItem {
            label: "Başlık Uzunluğu & Tıklama Tuzağı (Clickbait) Kontrolü".to_string(),
            detail: format!(
                "Mevcut: {title_chars} karakter (Google News/SERP için ideal aralık 35–70 karakter)."
            ),
            level: CheckLevel::Warn,
        });
    }

    // 3. Spot / Meta Description check (20 pts)
    let spot_len = art.spot.as_deref().map(|s| s.trim().chars().count()).unwrap_or(0);
    let desc_len = art
        .seo
        .as_ref()
        .map(|s| s.description.trim().chars().count())
        .unwrap_or(0);
    let best_desc = desc_len.max(spot_len);
    if (80..=165).contains(&best_desc) {
        score += 20;
        checks.push(SeoDiscoverCheckItem {
            label: "Spot & Meta Açıklama Uyumu".to_string(),
            detail: format!("Spot/meta açıklama ideal aralıkta ({best_desc} karakter)."),
            level: CheckLevel::Pass,
        });
    } else if best_desc > 0 {
        score += 10;
        checks.push(SeoDiscoverCheckItem {
            label: "Spot & Meta Açıklama Uyumu".to_string(),
            detail: format!("Mevcut: {best_desc} karakter (önerilen: 80–160 karakter)."),
            level: CheckLevel::Warn,
        });
    } else {
        checks.push(SeoDiscoverCheckItem {
            label: "Spot & Meta Açıklama Uyumu".to_string(),
            detail: "Spot veya SEO açıklaması boş. Okuyucu tıklama oranı için ekleyin.".to_string(),
            level: CheckLevel::Fail,
        });
    }

    // 4. Subheading (`H2` / `H3`) & Content Depth check (15 pts)
    let parsed_blocks = art
        .blocks
        .clone()
        .unwrap_or_else(|| parse_content_to_blocks(&art.content));
    let has_subheading = parsed_blocks
        .iter()
        .any(|b| b.block_type == "heading" || b.block_type == "heading3");
    let word_count: usize = parsed_blocks
        .iter()
        .map(|b| strip_all_html_tags(&b.content).split_whitespace().count())
        .sum();
    if has_subheading && word_count >= 80 {
        score += 15;
        checks.push(SeoDiscoverCheckItem {
            label: "Alt Başlık (H2/H3) & İçerik Derinliği".to_string(),
            detail: format!("Alt başlık mevcut ve içerik {word_count} kelime."),
            level: CheckLevel::Pass,
        });
    } else if word_count >= 40 {
        score += 8;
        checks.push(SeoDiscoverCheckItem {
            label: "Alt Başlık (H2/H3) & İçerik Derinliği".to_string(),
            detail: format!(
                "{word_count} kelime — Google News taraması için en az bir H2 alt başlık ekleyin."
            ),
            level: CheckLevel::Warn,
        });
    } else {
        checks.push(SeoDiscoverCheckItem {
            label: "Alt Başlık (H2/H3) & İçerik Derinliği".to_string(),
            detail: format!("İçerik çok kısa ({word_count} kelime) ve alt başlık yok."),
            level: CheckLevel::Fail,
        });
    }

    // 5. URL Slug cleanliness (10 pts)
    let effective_slug = art
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| slugify_turkish(&art.title));
    if !effective_slug.is_empty() && effective_slug.len() <= 90 {
        score += 10;
        checks.push(SeoDiscoverCheckItem {
            label: "Kalıcı URL (Slug) & 301 Koruması".to_string(),
            detail: format!("/icerik/{effective_slug} — temiz URL yapısı aktif."),
            level: CheckLevel::Pass,
        });
    } else {
        checks.push(SeoDiscoverCheckItem {
            label: "Kalıcı URL (Slug) & 301 Koruması".to_string(),
            detail: "Geçerli ve kısa bir URL kısa adı (slug) belirleyin.".to_string(),
            level: CheckLevel::Warn,
        });
    }

    // 6. Tags & Keywords / Internal Link engagement (10 pts)
    let tag_cnt = art.tags.as_ref().map(|t| t.len()).unwrap_or(0);
    let kw_nonempty = art
        .seo
        .as_ref()
        .map(|s| !s.keywords.trim().is_empty())
        .unwrap_or(false);
    let has_related = parsed_blocks.iter().any(|b| b.block_type == "related_article");
    if tag_cnt >= 2 || (tag_cnt >= 1 && (kw_nonempty || has_related)) {
        score += 10;
        checks.push(SeoDiscoverCheckItem {
            label: "Konu Etiketleri (#Tag) & İç Bağlantı".to_string(),
            detail: format!(
                "{tag_cnt} etiket tanımlı{} — konu otoritesi güçlü.",
                if has_related { " + ilgili haber kartı eklendi" } else { "" }
            ),
            level: CheckLevel::Pass,
        });
    } else if tag_cnt == 1 || kw_nonempty {
        score += 5;
        checks.push(SeoDiscoverCheckItem {
            label: "Konu Etiketleri (#Tag) & İç Bağlantı".to_string(),
            detail: "En az 2–3 konu etiketi (#Tag) ve yazı içi 'İlgili Haber' kartı önerilir."
                .to_string(),
            level: CheckLevel::Warn,
        });
    } else {
        checks.push(SeoDiscoverCheckItem {
            label: "Konu Etiketleri (#Tag) & İç Bağlantı".to_string(),
            detail: "Henüz konu etiketi (#Tag) veya SEO anahtar kelimesi girilmedi.".to_string(),
            level: CheckLevel::Fail,
        });
    }

    SeoDiscoverReport {
        score: score.clamp(0, 100) as u8,
        checks,
    }
}

fn default_draft_status() -> String {
    "draft".to_string()
}

impl Article {
    pub fn effective_image_url(&self) -> Option<&str> {
        self.image_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.image_url_camel.as_deref().filter(|s| !s.is_empty()))
    }

    pub fn effective_source_url(&self) -> Option<&str> {
        self.source_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.source_url_camel.as_deref().filter(|s| !s.is_empty()))
    }

    pub fn effective_original_url(&self) -> Option<&str> {
        self.original_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.original_url_camel.as_deref().filter(|s| !s.is_empty()))
    }

    pub fn effective_source_time(&self) -> Option<&str> {
        self.source_time
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.source_time_camel.as_deref().filter(|s| !s.is_empty()))
    }

    pub fn effective_publish_date(&self) -> Option<&str> {
        self.publish_date
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.publish_date_camel.as_deref().filter(|s| !s.is_empty()))
    }

    pub fn effective_scheduled_at(&self) -> Option<&str> {
        self.scheduled_at
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.scheduled_at_camel.as_deref().filter(|s| !s.is_empty()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WpSite {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_password: Option<String>,
    #[serde(default = "default_active_i8")]
    pub is_active: i8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TwitterCredentials {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_secret: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub access_secret: String,
    // Legacy server.ts field aliases inside wp_config.twitter
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub twitter_api_key: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub twitter_api_secret: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub twitter_access_token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub twitter_access_token_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FacebookCredentials {
    #[serde(default)]
    pub page_token: String,
    #[serde(default)]
    pub page_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstagramCredentials {
    #[serde(default)]
    pub page_token: String,
    #[serde(default)]
    pub ig_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LinkedinCredentials {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub org_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TelegramCredentials {
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub chat_id: String,
}

/// Exact `WordPressConfig` stored as JSON in `settings['wp_config']` (`App.tsx` lines 50-65).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WordPressConfig {
    #[serde(default)]
    pub site_url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub app_password: String,
    #[serde(default = "default_wp_api_path")]
    pub api_path: String,
    #[serde(default)]
    pub gemini_key: String,
    #[serde(default)]
    pub opusmax_key: String,
    #[serde(default = "default_ai_provider")]
    pub ai_provider: String, // "gemini" | "opusmax"
    #[serde(default)]
    pub eco_mode: Option<String>, // "true" | "false"
    #[serde(default)]
    pub twitter: TwitterCredentials,
    #[serde(default)]
    pub facebook: FacebookCredentials,
    #[serde(default)]
    pub instagram: InstagramCredentials,
    #[serde(default)]
    pub linkedin: LinkedinCredentials,
    #[serde(default)]
    pub telegram: TelegramCredentials,
}

fn default_wp_api_path() -> String {
    "/wp-json".to_string()
}

fn default_ai_provider() -> String {
    "gemini".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WpCategory {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub parent: i32,
    #[serde(default)]
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanLogEntry {
    pub time: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LastScanResult {
    pub success: bool,
    #[serde(default)]
    pub new_count: usize,
    #[serde(default)]
    pub message: Option<String>,
    pub end_time: u64,
}

/// Exact `/api/scan-status` and `/ws/scan` payload structures (`server.ts` lines 126-168 & 1765-1773).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatusResponse {
    pub is_scanning: bool,
    pub start_time: Option<u64>,
    pub logs: Vec<ScanLogEntry>,
    pub found_articles: Vec<Article>,
    pub last_scan_result: Option<LastScanResult>,
}

/// Sanitizes inline rich HTML, keeping only safe inline formatting tags:
/// `<strong>`, `<b>`, `<em>`, `<i>`, `<u>`, `<s>`, `<strike>`, `<mark>`, `<br>`, and safe `<a href="...">`.
pub fn sanitize_inline_rich_html(input: &str) -> String {
    let normalized = input
        .replace("&nbsp;", " ")
        .replace("<div><br></div>", "\n")
        .replace("<div>", "\n")
        .replace("</div>", "");
    let mut out = String::with_capacity(normalized.len() + 16);
    let mut chars = normalized.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        if ch == '<' {
            let rest = &normalized[idx..];
            if let Some(end_rel) = rest.find('>') {
                let raw_tag = &rest[1..end_rel];
                let advance_until = idx + end_rel + 1;
                while let Some(&(next_i, _)) = chars.peek() {
                    if next_i < advance_until {
                        chars.next();
                    } else {
                        break;
                    }
                }
                let trimmed = raw_tag.trim();
                let lower = trimmed.to_ascii_lowercase();
                match lower.as_str() {
                    "b" | "strong" => out.push_str("<strong>"),
                    "/b" | "/strong" => out.push_str("</strong>"),
                    "i" | "em" => out.push_str("<em>"),
                    "/i" | "/em" => out.push_str("</em>"),
                    "u" => out.push_str("<u>"),
                    "/u" => out.push_str("</u>"),
                    "s" | "strike" | "del" => out.push_str("<s>"),
                    "/s" | "/strike" | "/del" => out.push_str("</s>"),
                    "mark" => out.push_str("<mark>"),
                    "/mark" => out.push_str("</mark>"),
                    "code" => out.push_str("<code>"),
                    "/code" => out.push_str("</code>"),
                    "br" | "br/" | "br /" => out.push_str("<br/>"),
                    "/a" => out.push_str("</a>"),
                    _ if lower.starts_with("a ") || lower.starts_with("a\t") => {
                        if let Some(href) = extract_safe_href(trimmed) {
                            out.push_str(&format!(
                                "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
                                escape_html_basic(&href)
                            ));
                        }
                    }
                    _ => {}
                }
                continue;
            }
        }
        match ch {
            '&' => {
                let rest = &normalized[idx..];
                if rest.starts_with("&amp;")
                    || rest.starts_with("&lt;")
                    || rest.starts_with("&gt;")
                    || rest.starts_with("&quot;")
                    || rest.starts_with("&#39;")
                {
                    out.push('&');
                } else {
                    out.push_str("&amp;");
                }
            }
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }

    let with_bold = replace_double_asterisk_bold(&out);
    replace_single_asterisk_italic(&with_bold)
}

fn extract_safe_href(tag_body: &str) -> Option<String> {
    let lower = tag_body.to_ascii_lowercase();
    let pos = lower.find("href=")?;
    let after = tag_body[pos + 5..].trim_start();
    let val = if let Some(rest) = after.strip_prefix('"') {
        rest.split('"').next()?
    } else if let Some(rest) = after.strip_prefix('\'') {
        rest.split('\'').next()?
    } else {
        after.split_whitespace().next()?
    };
    let clean = val.trim();
    let cl = clean.to_ascii_lowercase();
    if cl.starts_with("https://")
        || cl.starts_with("http://")
        || cl.starts_with("mailto:")
        || clean.starts_with('/')
    {
        Some(clean.to_string())
    } else {
        None
    }
}

pub fn strip_all_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn extract_attr_val(tag: &str, attr_name: &str) -> Option<String> {
    let pat = format!("{attr_name}=\"");
    let pos = tag.find(&pat)?;
    let rest = &tag[pos + pat.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_inner_between<'a>(s: &'a str, open_pat: &str, close_pat: &str) -> Option<&'a str> {
    let start = s.find(open_pat)?;
    let after = &s[start + open_pat.len()..];
    let end = after.find(close_pat)?;
    Some(&after[..end])
}

fn parse_li_items(list_html: &str) -> String {
    let mut items = Vec::new();
    let mut rest = list_html;
    while let Some(pos) = rest.find("<li") {
        let after_li = &rest[pos + 3..];
        let Some(gt) = after_li.find('>') else { break };
        let body_start = &after_li[gt + 1..];
        let Some(end_li) = body_start.find("</li>") else {
            break;
        };
        let item = sanitize_inline_rich_html(body_start[..end_li].trim());
        if !item.is_empty() {
            items.push(item);
        }
        rest = &body_start[end_li + 5..];
    }
    items.join("\n")
}

/// Parses HTML or Markdown content losslessly into rich `ContentBlock` items.
pub fn parse_content_to_blocks(content: &str) -> Vec<ContentBlock> {
    if content.trim().is_empty() {
        return Vec::new();
    }
    let mut blocks = Vec::new();
    let mut seq = 1usize;
    let mut next_id = || {
        let s = format!("blk-{seq}");
        seq += 1;
        s
    };

    let looks_html = content.contains("<p>")
        || content.contains("<h1")
        || content.contains("<h2")
        || content.contains("<h3")
        || content.contains("<blockquote")
        || content.contains("<ul")
        || content.contains("<ol")
        || content.contains("<figure")
        || content.contains("<hr")
        || content.contains("<pre")
        || content.contains("hf-callout")
        || content.contains("hf-summary")
        || content.contains("hf-gallery")
        || content.contains("hf-related-card")
        || content.contains("hf-live-entry");

    if looks_html {
        for raw_chunk in content.split("\n\n") {
            let t = raw_chunk.trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with("<hr") {
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "divider".to_string(),
                    content: String::new(),
                    meta: None,
                });
                continue;
            }
            if t.contains("hf-gallery") {
                let title = extract_attr_val(t, "data-gallery-title")
                    .map(|s| strip_all_html_tags(&s).trim().to_string())
                    .filter(|s| !s.is_empty());
                let mut items: Vec<GalleryItem> = Vec::new();
                let mut rest = t;
                while let Some(fpos) = rest.find("<figure") {
                    let after_f = &rest[fpos..];
                    let Some(fend) = after_f.find("</figure>") else {
                        break;
                    };
                    let fig_chunk = &after_f[..fend + 9];
                    if let Some(src) = extract_attr_val(fig_chunk, "src") {
                        let thumb = extract_attr_val(fig_chunk, "data-thumb")
                            .filter(|u| !u.trim().is_empty());
                        let cap = extract_inner_between(fig_chunk, "<figcaption>", "</figcaption>")
                            .map(|c| strip_all_html_tags(c).trim().to_string())
                            .unwrap_or_default();
                        items.push(GalleryItem {
                            url: src,
                            thumb_url: thumb,
                            caption: cap,
                        });
                    }
                    rest = &after_f[fend + 9..];
                }
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "gallery".to_string(),
                    content: serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string()),
                    meta: title,
                });
                continue;
            }
            if t.contains("hf-related-card") {
                let aid = extract_attr_val(t, "data-article-id").unwrap_or_default();
                let slug = extract_attr_val(t, "data-slug").filter(|s| !s.trim().is_empty());
                let img = extract_attr_val(t, "data-img").filter(|s| !s.trim().is_empty());
                let kicker = extract_inner_between(t, "hf-related-kicker\">", "</span>")
                    .map(|k| strip_all_html_tags(k).trim().to_string())
                    .filter(|k| !k.is_empty());
                let title = extract_inner_between(t, "hf-related-title\">", "</span>")
                    .or_else(|| extract_inner_between(t, "<a", "</a>").and_then(|x| x.find('>').map(|i| &x[i + 1..])))
                    .map(|s| strip_all_html_tags(s).trim().to_string())
                    .unwrap_or_default();
                let payload = RelatedArticleRef {
                    article_id: aid,
                    title,
                    slug,
                    image_url: img,
                };
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "related_article".to_string(),
                    content: serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
                    meta: kicker,
                });
                continue;
            }
            if t.contains("hf-live-entry") {
                let time_str = extract_attr_val(t, "data-time").unwrap_or_default();
                let title_str = extract_inner_between(t, "hf-live-title\">", "</strong>")
                    .map(|h| strip_all_html_tags(h).trim().to_string())
                    .unwrap_or_default();
                let body = extract_inner_between(t, "hf-live-body\">", "</div>")
                    .map(|b| sanitize_inline_rich_html(b.trim()))
                    .unwrap_or_default();
                let meta_combined = if title_str.is_empty() {
                    time_str
                } else {
                    format!("{time_str}|{title_str}")
                };
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "live_entry".to_string(),
                    content: body,
                    meta: Some(meta_combined),
                });
                continue;
            }
            if t.contains("wp-block-embed") {
                let url = extract_inner_between(t, "wp-block-embed__wrapper\">", "</div>")
                    .map(|u| strip_all_html_tags(u).trim().to_string())
                    .unwrap_or_default();
                let cap = extract_inner_between(t, "<figcaption>", "</figcaption>")
                    .map(|c| strip_all_html_tags(c).trim().to_string())
                    .filter(|c| !c.is_empty());
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "embed".to_string(),
                    content: url,
                    meta: cap,
                });
                continue;
            }
            if t.contains("<img") {
                if let Some(src) = extract_attr_val(t, "src") {
                    let cap = extract_inner_between(t, "<figcaption>", "</figcaption>")
                        .map(|c| strip_all_html_tags(c).trim().to_string())
                        .filter(|c| !c.is_empty());
                    blocks.push(ContentBlock {
                        id: next_id(),
                        block_type: "image".to_string(),
                        content: src,
                        meta: cap,
                    });
                    continue;
                }
            }
            if t.contains("hf-summary") {
                let title = extract_inner_between(t, "<h4>", "</h4>")
                    .map(|h| strip_all_html_tags(h).trim().to_string())
                    .filter(|h| !h.is_empty());
                let items = parse_li_items(t);
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "summary".to_string(),
                    content: items,
                    meta: title,
                });
                continue;
            }
            if t.contains("hf-callout") {
                let variant = extract_attr_val(t, "data-variant").unwrap_or_else(|| "info".into());
                let body = extract_inner_between(t, "hf-callout-body\">", "</div>")
                    .or_else(|| extract_inner_between(t, "<p>", "</p>"))
                    .map(|b| sanitize_inline_rich_html(b.trim()))
                    .unwrap_or_else(|| sanitize_inline_rich_html(&strip_all_html_tags(t)));
                let title = extract_inner_between(t, "hf-callout-title\">", "</div>")
                    .map(|h| strip_all_html_tags(h).trim().to_string())
                    .unwrap_or_default();
                let meta_combined = if title.is_empty() {
                    variant
                } else {
                    format!("{variant}|{title}")
                };
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "callout".to_string(),
                    content: body,
                    meta: Some(meta_combined),
                });
                continue;
            }
            if t.starts_with("<blockquote") {
                let cite = extract_inner_between(t, "<cite>", "</cite>")
                    .map(|c| strip_all_html_tags(c).trim().to_string())
                    .filter(|c| !c.is_empty());
                let quote_body = extract_inner_between(t, "<p>", "</p>")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        let without_cite = if let Some(ci) = t.find("<cite>") {
                            &t[..ci]
                        } else {
                            t
                        };
                        strip_all_html_tags(without_cite)
                    });
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "quote".to_string(),
                    content: sanitize_inline_rich_html(quote_body.trim()),
                    meta: cite,
                });
                continue;
            }
            if t.starts_with("<ul") {
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "bullet_list".to_string(),
                    content: parse_li_items(t),
                    meta: None,
                });
                continue;
            }
            if t.starts_with("<ol") {
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "numbered_list".to_string(),
                    content: parse_li_items(t),
                    meta: None,
                });
                continue;
            }
            if t.starts_with("<pre") {
                let code = extract_inner_between(t, "<code>", "</code>")
                    .map(strip_all_html_tags)
                    .unwrap_or_else(|| strip_all_html_tags(t));
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "code".to_string(),
                    content: code,
                    meta: None,
                });
                continue;
            }
            if t.starts_with("<h1") {
                let inner = extract_inner_between(t, ">", "</h1>").unwrap_or(t);
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "heading1".to_string(),
                    content: strip_all_html_tags(inner).trim().to_string(),
                    meta: None,
                });
                continue;
            }
            if t.starts_with("<h2") {
                let inner = extract_inner_between(t, ">", "</h2>").unwrap_or(t);
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "heading".to_string(),
                    content: strip_all_html_tags(inner).trim().to_string(),
                    meta: None,
                });
                continue;
            }
            if t.starts_with("<h3") {
                let inner = extract_inner_between(t, ">", "</h3>").unwrap_or(t);
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "heading3".to_string(),
                    content: strip_all_html_tags(inner).trim().to_string(),
                    meta: None,
                });
                continue;
            }
            let inner_p = if t.starts_with("<p") {
                extract_inner_between(t, ">", "</p>").unwrap_or(t)
            } else {
                t
            };
            let cleaned = sanitize_inline_rich_html(inner_p.trim());
            if !strip_all_html_tags(&cleaned).trim().is_empty() {
                blocks.push(ContentBlock {
                    id: next_id(),
                    block_type: "paragraph".to_string(),
                    content: cleaned,
                    meta: None,
                });
            }
        }
        if !blocks.is_empty() {
            return blocks;
        }
    }

    let mut current_para: Vec<String> = Vec::new();
    let flush = |para: &mut Vec<String>, out: &mut Vec<ContentBlock>, id_fn: &mut dyn FnMut() -> String| {
        if !para.is_empty() {
            out.push(ContentBlock {
                id: id_fn(),
                block_type: "paragraph".to_string(),
                content: sanitize_inline_rich_html(para.join(" ").trim()),
                meta: None,
            });
            para.clear();
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut current_para, &mut blocks, &mut next_id);
            continue;
        }
        if trimmed == "---" || trimmed == "***" {
            flush(&mut current_para, &mut blocks, &mut next_id);
            blocks.push(ContentBlock {
                id: next_id(),
                block_type: "divider".to_string(),
                content: String::new(),
                meta: None,
            });
            continue;
        }
        if let Some(q) = trimmed.strip_prefix("> ") {
            flush(&mut current_para, &mut blocks, &mut next_id);
            blocks.push(ContentBlock {
                id: next_id(),
                block_type: "quote".to_string(),
                content: sanitize_inline_rich_html(q.trim()),
                meta: None,
            });
            continue;
        }
        if let Some(h3) = trimmed.strip_prefix("### ") {
            flush(&mut current_para, &mut blocks, &mut next_id);
            blocks.push(ContentBlock {
                id: next_id(),
                block_type: "heading3".to_string(),
                content: h3.trim().replace("**", ""),
                meta: None,
            });
            continue;
        }
        if let Some(h2) = trimmed.strip_prefix("## ") {
            flush(&mut current_para, &mut blocks, &mut next_id);
            blocks.push(ContentBlock {
                id: next_id(),
                block_type: "heading".to_string(),
                content: h2.trim().replace("**", ""),
                meta: None,
            });
            continue;
        }
        if let Some(h1) = trimmed.strip_prefix("# ") {
            flush(&mut current_para, &mut blocks, &mut next_id);
            blocks.push(ContentBlock {
                id: next_id(),
                block_type: "heading1".to_string(),
                content: h1.trim().replace("**", ""),
                meta: None,
            });
            continue;
        }
        let md_img = if trimmed.starts_with("![") && trimmed.ends_with(')') {
            trimmed.find("](").map(|i| (&trimmed[2..i], &trimmed[i + 2..trimmed.len() - 1]))
        } else {
            None
        };
        if let Some((alt, url)) = md_img {
            flush(&mut current_para, &mut blocks, &mut next_id);
            blocks.push(ContentBlock {
                id: next_id(),
                block_type: "image".to_string(),
                content: url.trim().to_string(),
                meta: if alt.trim().is_empty() {
                    None
                } else {
                    Some(alt.trim().to_string())
                },
            });
            continue;
        }
        if let Some(item) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            flush(&mut current_para, &mut blocks, &mut next_id);
            blocks.push(ContentBlock {
                id: next_id(),
                block_type: "bullet_list".to_string(),
                content: sanitize_inline_rich_html(item.trim()),
                meta: None,
            });
            continue;
        }
        current_para.push(trimmed.to_string());
    }
    flush(&mut current_para, &mut blocks, &mut next_id);
    blocks
}

/// Serializes all 13 `ContentBlock` types into clean semantic WordPress & Reader HTML.
pub fn blocks_to_wp_html(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block.block_type.as_str() {
            "heading1" => {
                let text = escape_html_basic(strip_all_html_tags(&block.content).trim());
                (!text.is_empty()).then(|| format!("<h1>{text}</h1>"))
            }
            "heading" => {
                let text = escape_html_basic(strip_all_html_tags(&block.content).trim());
                (!text.is_empty()).then(|| format!("<h2>{text}</h2>"))
            }
            "heading3" => {
                let text = escape_html_basic(strip_all_html_tags(&block.content).trim());
                (!text.is_empty()).then(|| format!("<h3>{text}</h3>"))
            }
            "image" => {
                let src = escape_html_basic(block.content.trim());
                if src.is_empty() {
                    return None;
                }
                let cap = block
                    .meta
                    .as_deref()
                    .map(|c| escape_html_basic(c.trim()))
                    .filter(|c| !c.is_empty());
                if let Some(c) = cap {
                    Some(format!(
                        "<figure><img src=\"{src}\" alt=\"{c}\" /><figcaption>{c}</figcaption></figure>"
                    ))
                } else {
                    Some(format!("<figure><img src=\"{src}\" alt=\"\" /></figure>"))
                }
            }
            "quote" => {
                let body = sanitize_inline_rich_html(block.content.trim()).replace('\n', "<br/>");
                if body.is_empty() {
                    return None;
                }
                let cite = block
                    .meta
                    .as_deref()
                    .map(|c| escape_html_basic(c.trim()))
                    .filter(|c| !c.is_empty());
                if let Some(c) = cite {
                    Some(format!("<blockquote><p>{body}</p><cite>{c}</cite></blockquote>"))
                } else {
                    Some(format!("<blockquote><p>{body}</p></blockquote>"))
                }
            }
            "bullet_list" => {
                let lis: Vec<String> = block
                    .content
                    .lines()
                    .map(|l| sanitize_inline_rich_html(l.trim().trim_start_matches(['•', '-', '*']).trim()))
                    .filter(|l| !strip_all_html_tags(l).trim().is_empty())
                    .map(|l| format!("<li>{l}</li>"))
                    .collect();
                (!lis.is_empty()).then(|| format!("<ul>{}</ul>", lis.join("")))
            }
            "numbered_list" => {
                let lis: Vec<String> = block
                    .content
                    .lines()
                    .map(|l| sanitize_inline_rich_html(l.trim()))
                    .filter(|l| !strip_all_html_tags(l).trim().is_empty())
                    .map(|l| format!("<li>{l}</li>"))
                    .collect();
                (!lis.is_empty()).then(|| format!("<ol>{}</ol>", lis.join("")))
            }
            "callout" => {
                let body = sanitize_inline_rich_html(block.content.trim()).replace('\n', "<br/>");
                if body.is_empty() {
                    return None;
                }
                let raw_meta = block.meta.as_deref().unwrap_or("info");
                let (variant, custom_title) = raw_meta.split_once('|').unwrap_or((raw_meta, ""));
                let default_title = match variant {
                    "breaking" => "SON DAKİKA",
                    "warning" => "ÖNEMLİ NOT",
                    _ => "BİLGİ KUTUSU",
                };
                let title = if custom_title.trim().is_empty() {
                    default_title.to_string()
                } else {
                    escape_html_basic(custom_title.trim())
                };
                let var_esc = escape_html_basic(variant);
                Some(format!(
                    "<div class=\"hf-callout\" data-variant=\"{var_esc}\"><div class=\"hf-callout-title\">{title}</div><div class=\"hf-callout-body\">{body}</div></div>"
                ))
            }
            "summary" => {
                let lis: Vec<String> = block
                    .content
                    .lines()
                    .map(|l| sanitize_inline_rich_html(l.trim().trim_start_matches(['•', '-', '*']).trim()))
                    .filter(|l| !strip_all_html_tags(l).trim().is_empty())
                    .map(|l| format!("<li>{l}</li>"))
                    .collect();
                if lis.is_empty() {
                    return None;
                }
                let heading = block
                    .meta
                    .as_deref()
                    .map(|m| escape_html_basic(m.trim()))
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| "Öne Çıkanlar".to_string());
                Some(format!(
                    "<div class=\"hf-summary\"><h4>{heading}</h4><ul>{}</ul></div>",
                    lis.join("")
                ))
            }
            "embed" => {
                let url = escape_html_basic(block.content.trim());
                if url.is_empty() {
                    return None;
                }
                let cap = block
                    .meta
                    .as_deref()
                    .map(|c| escape_html_basic(c.trim()))
                    .filter(|c| !c.is_empty());
                if let Some(c) = cap {
                    Some(format!(
                        "<figure class=\"wp-block-embed\"><div class=\"wp-block-embed__wrapper\">\n{url}\n</div><figcaption>{c}</figcaption></figure>"
                    ))
                } else {
                    Some(format!(
                        "<figure class=\"wp-block-embed\"><div class=\"wp-block-embed__wrapper\">\n{url}\n</div></figure>"
                    ))
                }
            }
            "divider" => Some("<hr />".to_string()),
            "code" => {
                let code = escape_html_basic(&block.content);
                (!code.trim().is_empty()).then(|| format!("<pre><code>{code}</code></pre>"))
            }
            "gallery" => {
                let items: Vec<GalleryItem> =
                    serde_json::from_str(block.content.trim()).unwrap_or_default();
                if items.is_empty() {
                    return None;
                }
                let gtitle = block
                    .meta
                    .as_deref()
                    .map(|m| escape_html_basic(m.trim()))
                    .unwrap_or_default();
                let heading_html = if gtitle.is_empty() {
                    String::new()
                } else {
                    format!("<h4 class=\"hf-gallery-heading\">{gtitle}</h4>")
                };
                let figs: Vec<String> = items
                    .iter()
                    .enumerate()
                    .filter(|(_, it)| !it.url.trim().is_empty())
                    .map(|(idx, it)| {
                        let url = escape_html_basic(it.url.trim());
                        let thumb_attr = it
                            .thumb_url
                            .as_deref()
                            .filter(|u| !u.trim().is_empty())
                            .map(|u| format!(" data-thumb=\"{}\"", escape_html_basic(u.trim())))
                            .unwrap_or_default();
                        let cap = escape_html_basic(it.caption.trim());
                        let cap_html = if cap.is_empty() {
                            String::new()
                        } else {
                            format!("<figcaption>{cap}</figcaption>")
                        };
                        format!(
                            "<figure class=\"hf-gallery-item\" data-index=\"{num}\"><img src=\"{url}\"{thumb_attr} alt=\"{cap}\" />{cap_html}</figure>",
                            num = idx + 1
                        )
                    })
                    .collect();
                if figs.is_empty() {
                    None
                } else {
                    Some(format!(
                        "<section class=\"hf-gallery\" data-gallery-title=\"{gtitle}\">{heading_html}<div class=\"hf-gallery-grid\">{}</div></section>",
                        figs.join("")
                    ))
                }
            }
            "related_article" => {
                let parsed: RelatedArticleRef =
                    serde_json::from_str(block.content.trim()).unwrap_or_default();
                if parsed.title.trim().is_empty() {
                    return None;
                }
                let aid = escape_html_basic(parsed.article_id.trim());
                let title = escape_html_basic(parsed.title.trim());
                let slug = parsed.slug.as_deref().unwrap_or("").trim();
                let slug_esc = escape_html_basic(slug);
                let href = if !slug.is_empty() {
                    format!("/icerik/{slug_esc}")
                } else {
                    "#".to_string()
                };
                let img_attr = parsed
                    .image_url
                    .as_deref()
                    .filter(|u| !u.trim().is_empty())
                    .map(|u| format!(" data-img=\"{}\"", escape_html_basic(u.trim())))
                    .unwrap_or_default();
                let thumb_html = parsed
                    .image_url
                    .as_deref()
                    .filter(|u| !u.trim().is_empty())
                    .map(|u| {
                        format!(
                            "<img src=\"{}\" alt=\"{title}\" class=\"hf-related-thumb\" />",
                            escape_html_basic(u.trim())
                        )
                    })
                    .unwrap_or_default();
                let kicker = block
                    .meta
                    .as_deref()
                    .map(|m| escape_html_basic(m.trim()))
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| "BUNU DA OKUYUN".to_string());
                Some(format!(
                    "<aside class=\"hf-related-card\" data-article-id=\"{aid}\" data-slug=\"{slug_esc}\"{img_attr}>{thumb_html}<div class=\"hf-related-meta\"><span class=\"hf-related-kicker\">{kicker}</span><a href=\"{href}\" class=\"hf-related-link\"><span class=\"hf-related-title\">{title}</span></a></div></aside>"
                ))
            }
            "live_entry" => {
                let body = sanitize_inline_rich_html(block.content.trim()).replace('\n', "<br/>");
                if body.is_empty() {
                    return None;
                }
                let raw_meta = block.meta.as_deref().unwrap_or("");
                let (time_raw, title_raw) = raw_meta.split_once('|').unwrap_or((raw_meta, ""));
                let time_esc = escape_html_basic(time_raw.trim());
                let title_esc = escape_html_basic(title_raw.trim());
                let title_html = if title_esc.is_empty() {
                    String::new()
                } else {
                    format!("<strong class=\"hf-live-title\">{title_esc}</strong>")
                };
                Some(format!(
                    "<div class=\"hf-live-entry\" data-time=\"{time_esc}\"><div class=\"hf-live-head\"><span class=\"hf-live-time\">{time_esc}</span>{title_html}</div><div class=\"hf-live-body\">{body}</div></div>"
                ))
            }
            _ => {
                let rich = sanitize_inline_rich_html(block.content.trim()).replace('\n', "<br/>");
                (!strip_all_html_tags(&rich).trim().is_empty()).then(|| format!("<p>{rich}</p>"))
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn escape_html_basic(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn replace_double_asterisk_bold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("**") {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find("**") {
            out.push_str("<strong>");
            out.push_str(&after_open[..end]);
            out.push_str("</strong>");
            rest = &after_open[end + 2..];
        } else {
            out.push_str("**");
            rest = after_open;
        }
    }
    out.push_str(rest);
    out
}

fn replace_single_asterisk_italic(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('*') {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 1..];
        if let Some(end) = after_open.find('*') {
            if end > 0 && !after_open[..end].contains('\n') {
                out.push_str("<em>");
                out.push_str(&after_open[..end]);
                out.push_str("</em>");
                rest = &after_open[end + 1..];
                continue;
            }
        }
        out.push('*');
        rest = after_open;
    }
    out.push_str(rest);
    out
}

fn is_glossary_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn char_matches_en(a: char, b: char, case_sensitive: bool) -> bool {
    if case_sensitive {
        a == b
    } else {
        a.to_lowercase().eq(b.to_lowercase())
    }
}

fn fold_tr_char(c: char) -> char {
    match c {
        'İ' | 'i' | 'î' | 'Î' => 'i',
        'I' | 'ı' => 'ı',
        'â' | 'Â' => 'a',
        'û' | 'Û' => 'u',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

fn char_matches_tr(a: char, b: char, case_sensitive: bool) -> bool {
    if case_sensitive {
        a == b
    } else {
        fold_tr_char(a) == fold_tr_char(b)
    }
}

fn split_html_segments(input: &str) -> Vec<(bool, &str)> {
    let mut segments = Vec::new();
    let mut cursor = 0usize;
    let bytes = input.as_bytes();
    let len = bytes.len();

    while cursor < len {
        if let Some(rel_lt) = input[cursor..].find('<') {
            let tag_start = cursor + rel_lt;
            if tag_start > cursor {
                segments.push((false, &input[cursor..tag_start]));
            }
            if let Some(rel_gt) = input[tag_start..].find('>') {
                let tag_end = tag_start + rel_gt + 1;
                segments.push((true, &input[tag_start..tag_end]));
                cursor = tag_end;
            } else {
                segments.push((false, &input[tag_start..]));
                break;
            }
        } else {
            segments.push((false, &input[cursor..]));
            break;
        }
    }

    segments
}

fn html_to_searchable_plain_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for (is_tag, seg) in split_html_segments(input) {
        if is_tag {
            out.push(' ');
        } else {
            out.push_str(
                &seg.replace("&nbsp;", " ")
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"")
                    .replace("&#39;", "'")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">"),
            );
        }
    }
    out
}

fn indexed_chars(s: &str) -> Vec<(usize, usize, char)> {
    let mut out = Vec::with_capacity(s.chars().count());
    for (idx, ch) in s.char_indices() {
        out.push((idx, idx + ch.len_utf8(), ch));
    }
    out
}

fn matches_at_boundary<F>(
    haystack: &[(usize, usize, char)],
    pos: usize,
    pat: &[char],
    eq_fn: F,
) -> bool
where
    F: Fn(char, char) -> bool,
{
    let plen = pat.len();
    if plen == 0 || pos + plen > haystack.len() {
        return false;
    }

    if is_glossary_word_char(pat[0]) && pos > 0 && is_glossary_word_char(haystack[pos - 1].2) {
        return false;
    }
    let end = pos + plen;
    if is_glossary_word_char(pat[plen - 1])
        && end < haystack.len()
        && is_glossary_word_char(haystack[end].2)
    {
        return false;
    }

    for i in 0..plen {
        if !eq_fn(haystack[pos + i].2, pat[i]) {
            return false;
        }
    }
    true
}

/// Scans English text (stripping HTML tags first) for whole-word matches of `forbidden_variants`
/// across `rules`, respecting `case_sensitive` and protecting already-compliant `preferred_en` phrases.
pub fn evaluate_glossary_compliance(
    text: &str,
    rules: &[GlossaryRule],
) -> Vec<GlossaryViolation> {
    if text.trim().is_empty() || rules.is_empty() {
        return Vec::new();
    }

    let plain = html_to_searchable_plain_text(text);
    let chars = indexed_chars(&plain);
    if chars.is_empty() {
        return Vec::new();
    }

    let mut violations = Vec::new();
    let mut globally_claimed = vec![false; chars.len()];

    for rule in rules {
        if rule.forbidden_variants.is_empty() {
            continue;
        }

        let mut protected = vec![false; chars.len()];
        let pref_trimmed = rule.preferred_en.trim();
        if !pref_trimmed.is_empty() {
            let pref_chars: Vec<char> = pref_trimmed.chars().collect();
            let plen = pref_chars.len();
            let mut idx = 0usize;
            while idx + plen <= chars.len() {
                if matches_at_boundary(&chars, idx, &pref_chars, |a, b| {
                    char_matches_en(a, b, rule.case_sensitive)
                }) {
                    for slot in &mut protected[idx..idx + plen] {
                        *slot = true;
                    }
                    idx += plen;
                } else {
                    idx += 1;
                }
            }
        }

        let mut variants: Vec<(&str, Vec<char>)> = rule
            .forbidden_variants
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| (v, v.chars().collect::<Vec<char>>()))
            .collect();
        variants.sort_by_key(|a| std::cmp::Reverse(a.1.len()));

        let mut rule_occurrences = 0usize;
        let mut first_matched: Option<String> = None;

        for (_, pat_chars) in &variants {
            let plen = pat_chars.len();
            if plen == 0 || plen > chars.len() {
                continue;
            }
            let mut idx = 0usize;
            while idx + plen <= chars.len() {
                let already_taken = (idx..idx + plen)
                    .any(|k| protected[k] || globally_claimed[k]);
                if !already_taken
                    && matches_at_boundary(&chars, idx, pat_chars, |a, b| {
                        char_matches_en(a, b, rule.case_sensitive)
                    })
                {
                    let byte_start = chars[idx].0;
                    let byte_end = chars[idx + plen - 1].1;
                    let slice = &plain[byte_start..byte_end];
                    if slice != pref_trimmed {
                        if first_matched.is_none() {
                            first_matched = Some(slice.to_string());
                        }
                        rule_occurrences += 1;
                        for k in idx..idx + plen {
                            protected[k] = true;
                            globally_claimed[k] = true;
                        }
                        idx += plen;
                        continue;
                    }
                }
                idx += 1;
            }
        }

        if let Some(matched_text) = first_matched {
            if rule_occurrences > 0 {
                violations.push(GlossaryViolation {
                    rule_id: rule.id,
                    matched_text,
                    preferred_replacement: pref_trimmed.to_string(),
                    note: rule.note.clone().unwrap_or_default(),
                    occurrences: rule_occurrences,
                });
            }
        }
    }

    violations
}

/// Safely replaces forbidden variants on word boundaries outside `<...>` HTML tags,
/// returning `(fixed_text, total_replacements)`.
pub fn apply_glossary_fixes_to_text(text: &str, rules: &[GlossaryRule]) -> (String, usize) {
    if text.is_empty() || rules.is_empty() {
        return (text.to_string(), 0);
    }

    struct FixCandidate<'a> {
        pat: Vec<char>,
        replacement: &'a str,
        case_sensitive: bool,
        is_keep_guard: bool,
    }

    let mut candidates: Vec<FixCandidate<'_>> = Vec::new();
    for rule in rules {
        let pref = rule.preferred_en.trim();
        if pref.is_empty() {
            continue;
        }
        let pref_chars: Vec<char> = pref.chars().collect();
        if !pref_chars.is_empty() {
            candidates.push(FixCandidate {
                pat: pref_chars,
                replacement: pref,
                case_sensitive: rule.case_sensitive,
                is_keep_guard: true,
            });
        }
        for raw_var in &rule.forbidden_variants {
            let v = raw_var.trim();
            if v.is_empty() || v == pref {
                continue;
            }
            candidates.push(FixCandidate {
                pat: v.chars().collect(),
                replacement: pref,
                case_sensitive: rule.case_sensitive,
                is_keep_guard: false,
            });
        }
    }

    if candidates.iter().all(|c| c.is_keep_guard) {
        return (text.to_string(), 0);
    }

    // Longest match wins; on equal length, keep-guards run before replacements so compliant text is untouched.
    candidates.sort_by(|a, b| {
        b.pat
            .len()
            .cmp(&a.pat.len())
            .then_with(|| b.is_keep_guard.cmp(&a.is_keep_guard))
    });

    let mut out = String::with_capacity(text.len() + 32);
    let mut total_replacements = 0usize;

    for (is_tag, seg) in split_html_segments(text) {
        if is_tag || seg.is_empty() {
            out.push_str(seg);
            continue;
        }
        let chars = indexed_chars(seg);
        let mut idx = 0usize;
        while idx < chars.len() {
            let mut matched_opt: Option<&FixCandidate<'_>> = None;
            for cand in &candidates {
                if matches_at_boundary(&chars, idx, &cand.pat, |a, b| {
                    char_matches_en(a, b, cand.case_sensitive)
                }) {
                    matched_opt = Some(cand);
                    break;
                }
            }

            if let Some(cand) = matched_opt {
                let plen = cand.pat.len();
                let byte_start = chars[idx].0;
                let byte_end = chars[idx + plen - 1].1;
                let orig_slice = &seg[byte_start..byte_end];
                if cand.is_keep_guard || orig_slice == cand.replacement {
                    out.push_str(orig_slice);
                } else {
                    out.push_str(cand.replacement);
                    total_replacements += 1;
                }
                idx += plen;
            } else {
                out.push(chars[idx].2);
                idx += 1;
            }
        }
    }

    (out, total_replacements)
}

/// Replaces known Turkish institutional names/abbreviations (`term_tr` / `abbr_tr`, longest-first,
/// outside HTML tags) with their official `preferred_en` equivalents for instant NATO/MSI TDR scaffolding.
pub fn apply_tr_to_en_glossary_assist(tr_text: &str, rules: &[GlossaryRule]) -> String {
    if tr_text.is_empty() || rules.is_empty() {
        return tr_text.to_string();
    }

    struct TrAssistCandidate<'a> {
        pat: Vec<char>,
        replacement: &'a str,
        case_sensitive: bool,
        is_keep_guard: bool,
    }

    let mut candidates: Vec<TrAssistCandidate<'_>> = Vec::new();

    for rule in rules {
        if rule.rule_type.eq_ignore_ascii_case("forbidden") {
            continue;
        }
        let pref = rule.preferred_en.trim();
        if pref.is_empty() {
            continue;
        }

        // Guard already-translated `preferred_en` spans so repeated runs are 100% idempotent
        candidates.push(TrAssistCandidate {
            pat: pref.chars().collect(),
            replacement: pref,
            case_sensitive: false,
            is_keep_guard: true,
        });

        let term_tr = rule.term_tr.trim();
        let abbr_tr = rule
            .abbr_tr
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        if !term_tr.is_empty() && !term_tr.contains('(') {
            if let Some(abbr) = abbr_tr {
                let combined = format!("{term_tr} ({abbr})");
                candidates.push(TrAssistCandidate {
                    pat: combined.chars().collect(),
                    replacement: pref,
                    case_sensitive: false,
                    is_keep_guard: false,
                });
            }
            if term_tr != pref {
                candidates.push(TrAssistCandidate {
                    pat: term_tr.chars().collect(),
                    replacement: pref,
                    case_sensitive: false,
                    is_keep_guard: false,
                });
            }
        }

        if let Some(abbr) = abbr_tr {
            if abbr.chars().count() >= 2 && abbr != "TR" && abbr != pref {
                let is_short_acronym = abbr.chars().count() <= 5
                    && abbr.chars().all(|c| !c.is_lowercase());
                candidates.push(TrAssistCandidate {
                    pat: abbr.chars().collect(),
                    replacement: pref,
                    case_sensitive: is_short_acronym,
                    is_keep_guard: false,
                });
            }
        }
    }

    candidates.sort_by(|a, b| {
        b.pat
            .len()
            .cmp(&a.pat.len())
            .then_with(|| b.is_keep_guard.cmp(&a.is_keep_guard))
    });

    let mut out = String::with_capacity(tr_text.len() + 64);
    for (is_tag, seg) in split_html_segments(tr_text) {
        if is_tag || seg.is_empty() {
            out.push_str(seg);
            continue;
        }
        let chars = indexed_chars(seg);
        let mut idx = 0usize;
        while idx < chars.len() {
            let mut matched_opt: Option<&TrAssistCandidate<'_>> = None;
            for cand in &candidates {
                if matches_at_boundary(&chars, idx, &cand.pat, |a, b| {
                    char_matches_tr(a, b, cand.case_sensitive)
                }) {
                    matched_opt = Some(cand);
                    break;
                }
            }

            if let Some(cand) = matched_opt {
                let plen = cand.pat.len();
                let byte_start = chars[idx].0;
                let byte_end = chars[idx + plen - 1].1;
                let orig_slice = &seg[byte_start..byte_end];
                if cand.is_keep_guard {
                    out.push_str(orig_slice);
                } else {
                    out.push_str(cand.replacement);
                }
                idx += plen;
            } else {
                out.push(chars[idx].2);
                idx += 1;
            }
        }
    }

    out
}

/// Computes realistic reading time in minutes (`max(1, ceil(words / 200.0))`).
pub fn estimate_read_time_minutes(title: &str, spot: &str, blocks: &[ContentBlock]) -> i32 {
    let mut words = strip_all_html_tags(title).split_whitespace().count()
        + strip_all_html_tags(spot).split_whitespace().count();

    for b in blocks {
        match b.block_type.as_str() {
            "divider" | "related_article" => {}
            "image" | "embed" => {
                if let Some(cap) = b.meta.as_deref() {
                    words += strip_all_html_tags(cap).split_whitespace().count();
                }
            }
            "gallery" => {
                if let Some(gtitle) = b.meta.as_deref() {
                    words += strip_all_html_tags(gtitle).split_whitespace().count();
                }
                if let Ok(items) = serde_json::from_str::<Vec<GalleryItem>>(b.content.trim()) {
                    for it in items {
                        words += strip_all_html_tags(&it.caption).split_whitespace().count();
                    }
                }
            }
            _ => {
                words += strip_all_html_tags(&b.content).split_whitespace().count();
            }
        }
    }

    let mins = ((words as f64) / 200.0).ceil() as i32;
    mins.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markdown_and_html_blocks_accurately() {
        let md = "## Alt Başlık\n\nBirinci *italik* ve **kalın** metin.\n\n![Kapak](https://example.com/a.jpg)";
        let blocks = parse_content_to_blocks(md);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].block_type, "heading");
        assert_eq!(blocks[1].block_type, "paragraph");
        assert!(blocks[1].content.contains("<em>italik</em>"));
        assert!(blocks[1].content.contains("<strong>kalın</strong>"));
        assert_eq!(blocks[2].block_type, "image");
        assert_eq!(blocks[2].meta.as_deref(), Some("Kapak"));
        let html = blocks_to_wp_html(&blocks);
        assert!(html.contains("<h2>Alt Başlık</h2>"));
        assert!(html.contains("<figcaption>Kapak</figcaption>"));
        let roundtrip = parse_content_to_blocks(&html);
        assert_eq!(roundtrip.len(), 3);
        assert_eq!(roundtrip[2].meta.as_deref(), Some("Kapak"));
    }

    #[test]
    fn roundtrips_extended_rich_blocks_and_strips_xss() {
        let blocks = vec![
            ContentBlock {
                id: "1".into(),
                block_type: "heading1".into(),
                content: "Ana Başlık".into(),
                meta: None,
            },
            ContentBlock {
                id: "2".into(),
                block_type: "quote".into(),
                content: "Önemli bir <em>açıklama</em> yapıldı.<script>alert(1)</script>".into(),
                meta: Some("Dr. Ahmet Yılmaz".into()),
            },
            ContentBlock {
                id: "3".into(),
                block_type: "callout".into(),
                content: "Acil gelişme detayları.".into(),
                meta: Some("breaking|SON DAKİKA".into()),
            },
            ContentBlock {
                id: "4".into(),
                block_type: "summary".into(),
                content: "İlk madde\nİkinci madde".into(),
                meta: Some("Kısaca Ne Oldu?".into()),
            },
            ContentBlock {
                id: "5".into(),
                block_type: "divider".into(),
                content: String::new(),
                meta: None,
            },
        ];
        let html = blocks_to_wp_html(&blocks);
        assert!(!html.contains("<script>"));
        assert!(html.contains("<em>açıklama</em>"));
        let parsed = parse_content_to_blocks(&html);
        assert_eq!(parsed.len(), 5);
        assert_eq!(parsed[0].block_type, "heading1");
        assert_eq!(parsed[1].block_type, "quote");
        assert_eq!(parsed[1].meta.as_deref(), Some("Dr. Ahmet Yılmaz"));
        assert_eq!(parsed[2].block_type, "callout");
        assert_eq!(parsed[3].block_type, "summary");
        assert_eq!(parsed[4].block_type, "divider");
    }

    #[test]
    fn builds_hierarchical_category_tree_and_ancestor_chains() {
        let cats = vec![
            Category {
                id: 1,
                name: "Spor".into(),
                slug: "spor".into(),
                color: Some("#10b981".into()),
                parent_id: None,
                created_at: None,
            },
            Category {
                id: 2,
                name: "Futbol".into(),
                slug: "futbol".into(),
                color: Some("#10b981".into()),
                parent_id: Some(1),
                created_at: None,
            },
            Category {
                id: 3,
                name: "Süper Lig".into(),
                slug: "super-lig".into(),
                color: Some("#10b981".into()),
                parent_id: Some(2),
                created_at: None,
            },
            Category {
                id: 4,
                name: "Ekonomi".into(),
                slug: "ekonomi".into(),
                color: Some("#f59e0b".into()),
                parent_id: None,
                created_at: None,
            },
        ];

        let tree = build_category_tree(&cats);
        assert_eq!(tree.len(), 4);
        assert_eq!(tree[0].path_label, "Ekonomi");
        assert_eq!(tree[0].depth, 0);
        assert_eq!(tree[1].path_label, "Spor");
        assert_eq!(tree[1].depth, 0);
        assert_eq!(tree[2].path_label, "Spor › Futbol");
        assert_eq!(tree[2].depth, 1);
        assert_eq!(tree[3].path_label, "Spor › Futbol › Süper Lig");
        assert_eq!(tree[3].depth, 2);

        let desc = collect_descendant_category_ids(&cats, 1);
        assert!(desc.contains(&2) && desc.contains(&3) && !desc.contains(&4));

        assert_eq!(category_ancestor_chain(&cats, 3), vec![1, 2, 3]);
    }

    #[test]
    fn roundtrips_gallery_related_and_live_blocks_and_scores_discover() {
        let g_items = vec![
            GalleryItem {
                url: "/uploads/1.webp".into(),
                thumb_url: Some("/uploads/1-thumb.webp".into()),
                caption: "İlk kare".into(),
            },
            GalleryItem {
                url: "/uploads/2.webp".into(),
                thumb_url: None,
                caption: "İkinci kare".into(),
            },
        ];
        let rel = RelatedArticleRef {
            article_id: "pub-99".into(),
            title: "Merkez Bankası Faiz Kararını Açıkladı".into(),
            slug: Some("merkez-bankasi-faiz-kararini-acikladi".into()),
            image_url: Some("/uploads/mb.webp".into()),
        };
        let blocks = vec![
            ContentBlock {
                id: "1".into(),
                block_type: "heading".into(),
                content: "Piyasalarda İlk Tepki".into(),
                meta: None,
            },
            ContentBlock {
                id: "2".into(),
                block_type: "live_entry".into(),
                content: "Dolar kuru kararın ardından geriledi.".into(),
                meta: Some("14:05|Piyasa Açılışı".into()),
            },
            ContentBlock {
                id: "3".into(),
                block_type: "related_article".into(),
                content: serde_json::to_string(&rel).unwrap(),
                meta: Some("BUNU DA OKUYUN".into()),
            },
            ContentBlock {
                id: "4".into(),
                block_type: "gallery".into(),
                content: serde_json::to_string(&g_items).unwrap(),
                meta: Some("Zirveden Fotoğraflar".into()),
            },
            ContentBlock {
                id: "5".into(),
                block_type: "paragraph".into(),
                content: "Türkiye ekonomisinde enflasyonla mücadele kapsamında atılan adımlar piyasalar tarafından yakından takip edilmeye devam ediyor ve uzmanlar yıl sonu beklentilerini güncelledi. Bankacılık endeksi gün boyu pozitif seyretti ve yabancı yatırımcı girişleri hız kazandı. İhracatçılar yeni kredi paketinin detaylarını beklerken sanayi üretim verileri de beklentilerin üzerinde gerçekleşti. Ekonomistler önümüzdeki çeyrekte dengelenme sürecinin güçleneceğini ifade ediyor. Kurumların yayınladığı raporlarda büyüme tahminleri yukarı yönlü revize edildi ve iç talep göstergeleri incelendi.".into(),
                meta: None,
            },
        ];

        let html = blocks_to_wp_html(&blocks);
        assert!(html.contains("hf-live-entry"));
        assert!(html.contains("hf-related-card"));
        assert!(html.contains("hf-gallery"));

        let parsed = parse_content_to_blocks(&html);
        assert_eq!(parsed.len(), 5);
        assert_eq!(parsed[1].block_type, "live_entry");
        assert_eq!(parsed[2].block_type, "related_article");
        assert_eq!(parsed[3].block_type, "gallery");

        let art = Article {
            title: "Merkez Bankası'ndan Kritik Faiz Adımı ve Enflasyon Mesajı".into(),
            spot: Some("Merkez Bankası Para Politikası Kurulu yılın son toplantısında politika faizini sabit tutarken sıkı duruş mesajını yineledi.".into()),
            slug: Some("merkez-bankasindan-kritik-faiz-adimi".into()),
            image_url: Some("/uploads/kapak.webp".into()),
            cover_width: Some(1200),
            cover_height: Some(675),
            tags: Some(vec!["Ekonomi".into(), "Merkez Bankası".into()]),
            blocks: Some(blocks),
            ..Default::default()
        };
        let report = evaluate_seo_discover(&art);
        assert_eq!(report.score, 100);
    }

    fn sample_msi_glossary_rules() -> Vec<GlossaryRule> {
        vec![
            GlossaryRule {
                id: 1,
                term_tr: "Türkiye".into(),
                term_en: "Türkiye".into(),
                abbr_tr: Some("TR".into()),
                abbr_en: Some("TR".into()),
                category: "style".into(),
                rule_type: "fixed".into(),
                preferred_en: "Türkiye".into(),
                forbidden_variants: vec!["Turkey".into(), "Republic of Turkey".into()],
                case_sensitive: true,
                note: Some("Use official UN-registered country name Türkiye.".into()),
            },
            GlossaryRule {
                id: 2,
                term_tr: "Savunma (İngiliz İngilizcesi)".into(),
                term_en: "Defence".into(),
                abbr_tr: None,
                abbr_en: None,
                category: "style".into(),
                rule_type: "fixed".into(),
                preferred_en: "Defence".into(),
                forbidden_variants: vec!["Defense".into()],
                case_sensitive: false,
                note: Some("MSI TDR strictly follows British English spelling: Defence.".into()),
            },
            GlossaryRule {
                id: 3,
                term_tr: "Millî Savunma Bakanlığı".into(),
                term_en: "Ministry of National Defence (MoND)".into(),
                abbr_tr: Some("MSB".into()),
                abbr_en: Some("MoND".into()),
                category: "institutional".into(),
                rule_type: "fixed".into(),
                preferred_en: "Ministry of National Defence (MoND)".into(),
                forbidden_variants: vec![
                    "Ministry of Defense".into(),
                    "MoD".into(),
                    "Ministry of National Defense".into(),
                ],
                case_sensitive: false,
                note: Some("Always Ministry of National Defence (MoND); never MoD.".into()),
            },
            GlossaryRule {
                id: 4,
                term_tr: "Savunma Sanayii Başkanlığı".into(),
                term_en: "Presidency of Defence Industries (SSB)".into(),
                abbr_tr: Some("SSB".into()),
                abbr_en: Some("SSB".into()),
                category: "institutional".into(),
                rule_type: "fixed".into(),
                preferred_en: "Presidency of Defence Industries (SSB)".into(),
                forbidden_variants: vec![
                    "Undersecretariat for Defence Industries".into(),
                    "SSM".into(),
                ],
                case_sensitive: false,
                note: Some("Use Presidency of Defence Industries (SSB).".into()),
            },
            GlossaryRule {
                id: 5,
                term_tr: "Türk Silahlı Kuvvetleri".into(),
                term_en: "Turkish Armed Forces (TAF)".into(),
                abbr_tr: Some("TSK".into()),
                abbr_en: Some("TAF".into()),
                category: "institutional".into(),
                rule_type: "fixed".into(),
                preferred_en: "Turkish Armed Forces (TAF)".into(),
                forbidden_variants: vec!["Turkish Army".into(), "TSK".into()],
                case_sensitive: false,
                note: Some("Use Turkish Armed Forces (TAF).".into()),
            },
            GlossaryRule {
                id: 6,
                term_tr: "Hava Kuvvetleri Komutanlığı".into(),
                term_en: "Turkish Air Force (TurAF)".into(),
                abbr_tr: Some("Hv.K.K.lığı".into()),
                abbr_en: Some("TurAF".into()),
                category: "institutional".into(),
                rule_type: "fixed".into(),
                preferred_en: "Turkish Air Force (TurAF)".into(),
                forbidden_variants: vec!["TAF Air Force".into()],
                case_sensitive: false,
                note: Some("Use Turkish Air Force (TurAF).".into()),
            },
            GlossaryRule {
                id: 7,
                term_tr: "Sahil Güvenlik Komutanlığı".into(),
                term_en: "Turkish Coast Guard Command".into(),
                abbr_tr: Some("S.G.K.lığı".into()),
                abbr_en: Some("TCGC".into()),
                category: "institutional".into(),
                rule_type: "fixed".into(),
                preferred_en: "Turkish Coast Guard Command".into(),
                forbidden_variants: vec!["Coast Guard".into()],
                case_sensitive: false,
                note: Some("Use Turkish Coast Guard Command.".into()),
            },
            GlossaryRule {
                id: 8,
                term_tr: "KAAN".into(),
                term_en: "KAAN".into(),
                abbr_tr: Some("MMU".into()),
                abbr_en: Some("KAAN".into()),
                category: "technical".into(),
                rule_type: "fixed".into(),
                preferred_en: "KAAN".into(),
                forbidden_variants: vec!["TF-X".into(), "Kaan".into()],
                case_sensitive: true,
                note: Some("Always uppercase KAAN.".into()),
            },
        ]
    }

    #[test]
    fn evaluates_and_fixes_glossary_compliance_with_strict_word_boundaries() {
        let rules = sample_msi_glossary_rules();

        // Compliant text with tricky substrings ("Türkiye", "modern", "mode", "KAAN", "Turkish Coast Guard Command")
        let clean_html = "<p><a href=\"https://defense.gov/turkey\">Türkiye</a> operates modern multi-mode radars with KAAN and the Turkish Coast Guard Command.</p>";
        let clean_violations = evaluate_glossary_compliance(clean_html, &rules);
        assert!(
            clean_violations.is_empty(),
            "Expected 0 violations on compliant HTML, got: {clean_violations:?}"
        );
        let (unchanged, count0) = apply_glossary_fixes_to_text(clean_html, &rules);
        assert_eq!(count0, 0);
        assert_eq!(unchanged, clean_html);

        // Non-compliant HTML: should catch forbidden variants outside tags while preserving href/class inside tags
        let dirty_html = "<p class=\"defense-card\"><a href=\"https://example.com/Turkey/MoD\">Turkey</a> and the MoD announced a Defense contract for Kaan and TF-X alongside the Coast Guard in modern mode.</p>";
        let violations = evaluate_glossary_compliance(dirty_html, &rules);
        assert_eq!(violations.len(), 5);
        assert!(violations.iter().any(|v| v.rule_id == 1 && v.occurrences == 1));
        assert!(violations.iter().any(|v| v.rule_id == 2 && v.occurrences == 1));
        assert!(violations.iter().any(|v| v.rule_id == 3 && v.occurrences == 1));
        assert!(violations.iter().any(|v| v.rule_id == 7 && v.occurrences == 1));
        assert!(violations.iter().any(|v| v.rule_id == 8 && v.occurrences == 2));

        let (fixed_html, total_fixes) = apply_glossary_fixes_to_text(dirty_html, &rules);
        assert_eq!(total_fixes, 6);
        assert!(fixed_html.contains("href=\"https://example.com/Turkey/MoD\""));
        assert!(fixed_html.contains("class=\"defense-card\""));
        assert!(fixed_html.contains(">Türkiye</a> and the Ministry of National Defence (MoND) announced a Defence contract for KAAN and KAAN alongside the Turkish Coast Guard Command in modern mode."));
        assert!(evaluate_glossary_compliance(&fixed_html, &rules).is_empty());
    }

    #[test]
    fn applies_tr_to_en_glossary_assist_longest_first_and_idempotently() {
        let rules = sample_msi_glossary_rules();
        let tr_html = "<p data-org=\"SSB\">Savunma Sanayii Başkanlığı (SSB) ve Milli Savunma Bakanlığı koordine etti. Türk Silahlı Kuvvetleri bünyesindeki Hava Kuvvetleri Komutanlığı için MMU teslimatı planlandı.</p>";
        let assisted = apply_tr_to_en_glossary_assist(tr_html, &rules);
        assert!(assisted.contains("data-org=\"SSB\""));
        assert!(assisted.contains("Presidency of Defence Industries (SSB) ve Ministry of National Defence (MoND)"));
        assert!(!assisted.contains("(Presidency of Defence Industries (SSB))"));
        assert!(assisted.contains("Turkish Armed Forces (TAF) bünyesindeki Turkish Air Force (TurAF) için KAAN teslimatı"));

        // Running assist a second time must be idempotent
        let assisted_twice = apply_tr_to_en_glossary_assist(&assisted, &rules);
        assert_eq!(assisted_twice, assisted);
    }

    #[test]
    fn estimates_read_time_and_roundtrips_msi_article_fields_in_camel_and_snake_case() {
        let blocks = vec![ContentBlock {
            id: "1".into(),
            block_type: "paragraph".into(),
            content: (0..220).map(|_| "savunma").collect::<Vec<_>>().join(" "),
            meta: None,
        }];
        assert_eq!(estimate_read_time_minutes("Başlık", "Spot metni", &blocks), 2);
        assert_eq!(estimate_read_time_minutes("", "", &[]), 1);

        let art = Article {
            id: "msi-286-1".into(),
            title: "KAAN İlk Uçuş Analizi".into(),
            title_en: Some("KAAN Maiden Flight Analysis".into()),
            spot_en: Some("Comprehensive technical dossier.".into()),
            content_en: Some("<p>English body</p>".into()),
            blocks_en: Some(vec![ContentBlock {
                id: "en-1".into(),
                block_type: "paragraph".into(),
                content: "English body".into(),
                meta: None,
            }]),
            en_status: "approved".into(),
            publication_channel: "both".into(),
            content_type: "kapak-dosyasi".into(),
            issue_number: Some(286),
            issue_page_range: Some("22-30".into()),
            is_featured: true,
            author_name: Some("Dr. Selim Kaya".into()),
            author_title: Some("Havacılık Editörü".into()),
            video_url: Some("https://www.youtube.com/watch?v=kaan286".into()),
            read_time_min: Some(6),
            visibility_level: 2,
            single_price_try: 49.90,
            is_sponsored: true,
            sponsor_brand: Some("Turkish Aerospace (TUSAŞ)".into()),
            sponsor_legal_label: Some("TANITICI MAKALE".into()),
            ssb_tags: Some(vec!["hava-araclari".into(), "radarlar-ve-sensorler".into()]),
            ..Default::default()
        };

        let json_camel = serde_json::to_value(&art).unwrap();
        assert_eq!(json_camel["titleEn"], "KAAN Maiden Flight Analysis");
        assert_eq!(json_camel["enStatus"], "approved");
        assert_eq!(json_camel["publicationChannel"], "both");
        assert_eq!(json_camel["contentType"], "kapak-dosyasi");
        assert_eq!(json_camel["issueNumber"], 286);
        assert_eq!(json_camel["visibilityLevel"], 2);
        assert_eq!(json_camel["ssbTags"][0], "hava-araclari");

        let from_camel: Article = serde_json::from_value(json_camel).unwrap();
        assert_eq!(from_camel.title_en.as_deref(), Some("KAAN Maiden Flight Analysis"));
        assert_eq!(from_camel.issue_number, Some(286));
        assert_eq!(from_camel.single_price_try, 49.90);
        assert!(from_camel.is_featured);
        assert!(from_camel.is_sponsored);

        let snake_payload = serde_json::json!({
            "id": "msi-snake",
            "title": "MİLGEM İstif Sınıfı",
            "title_en": "MİLGEM İSTİF-Class Frigate",
            "en_status": "review",
            "publication_channel": "magazine",
            "content_type": "makaleler-analizler",
            "issue_number": 153,
            "issue_page_range": "14-21",
            "is_featured": true,
            "author_name": "Deniz Masası",
            "visibility_level": 1,
            "single_price_try": 25.5,
            "is_sponsored": false,
            "ssb_tags": ["deniz-araclari"]
        });
        let from_snake: Article = serde_json::from_value(snake_payload).unwrap();
        assert_eq!(from_snake.en_status, "review");
        assert_eq!(from_snake.publication_channel, "magazine");
        assert_eq!(from_snake.content_type, "makaleler-analizler");
        assert_eq!(from_snake.issue_number, Some(153));
        assert_eq!(from_snake.issue_page_range.as_deref(), Some("14-21"));
        assert_eq!(from_snake.visibility_level, 1);
        assert_eq!(
            from_snake.ssb_tags,
            Some(vec!["deniz-araclari".to_string()])
        );
    }

    #[test]
    fn derives_editorial_capabilities_from_msi_permissions() {
        let editor_caps = EditorialCapabilities::from_permissions(&[
            "contents:read".into(),
            "contents:write".into(),
            "media:upload".into(),
        ]);
        assert!(editor_caps.can_write);
        assert!(editor_caps.can_upload_media);
        assert!(!editor_caps.can_publish);
        assert!(!editor_caps.can_delete);
        assert!(!editor_caps.can_manage_redirects);

        let manage_caps = EditorialCapabilities::from_permissions(&[
            "contents:manage".into(),
            "categories:manage".into(),
            "media:manage".into(),
            "issues:manage".into(),
            "redirects:manage".into(),
            "glossary:manage".into(),
            "analytics:read".into(),
        ]);
        assert!(manage_caps.can_write);
        assert!(manage_caps.can_publish);
        assert!(manage_caps.can_delete);
        assert!(manage_caps.can_upload_media);
        assert!(manage_caps.can_delete_media);
        assert!(manage_caps.can_manage_categories);
        assert!(manage_caps.can_delete_categories);
        assert!(manage_caps.can_manage_issues);
        assert!(manage_caps.can_delete_issues);
        assert!(manage_caps.can_manage_glossary);
        assert!(manage_caps.can_manage_redirects);
        assert!(manage_caps.can_view_analytics);
        assert!(manage_caps.can_access_admin);

        let admin_caps = EditorialCapabilities::from_permissions(&["*".into()]);
        assert!(admin_caps.can_publish);
        assert!(admin_caps.can_delete);
        assert!(admin_caps.can_manage_issues);
        assert!(admin_caps.can_manage_glossary);
        assert!(admin_caps.can_manage_redirects);
    }
}
