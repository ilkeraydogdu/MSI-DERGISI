//! MSI Dergisi Editorial Studio Repository (`contents`, `media_library`, `editorial_research_sources`, taxonomy, revisions, redirects).

use super::Database;
use crate::engine::security_engine::sanitize_rich_html;
use crate::models::{
    category::TranslationGlossaryTerm, CategoryNode, MagazineIssue, SeoRedirectRule,
};
use crate::editorial::common::{
    blocks_to_wp_html, category_ancestor_chain, estimate_read_time_minutes, parse_content_to_blocks,
    sanitize_inline_rich_html, slugify_turkish, Article, ArticleRevision, Category, ContentBlock,
    GlossaryRule, MagazineIssueItem, MediaAsset, ResearchSource, SeoData, SsbTag, UrlRedirect,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct EditorialArticleQuery {
    pub status: Option<String>,
    pub article_type: Option<String>,
    pub category_id: Option<i32>,
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorialStats {
    pub total: i64,
    pub published: i64,
    pub draft: i64,
    #[serde(rename = "pendingReview")]
    pub pending_review: i64,
    pub future: i64,
    pub archived: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorialArticleListResponse {
    pub articles: Vec<Article>,
    pub total: i64,
    pub stats: EditorialStats,
}

#[derive(Debug, Clone)]
pub struct EditorialUpsertOutcome {
    pub article: Article,
    pub redirect_created: Option<UrlRedirect>,
    pub was_insert: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EditorialMediaInsert {
    pub filename: String,
    pub original_name: String,
    pub url: String,
    pub thumb_url: Option<String>,
    pub mime_type: String,
    pub width: i32,
    pub height: i32,
    pub size_bytes: i64,
    pub watermarked: bool,
    pub alt_text: Option<String>,
    pub uploaded_by: String,
}

#[derive(Debug, Clone)]
pub struct DueScheduledArticle {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub spot: String,
    pub author_name: String,
    pub category_slug: String,
    pub body_html: String,
    pub social_shared: bool,
}

/// Parses either a numeric string (`"42"`) or `"msi-42"` into a positive SQLite `contents.id`.
pub fn parse_editorial_content_id(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let clean = trimmed.strip_prefix("msi-").unwrap_or(trimmed);
    clean.parse::<i64>().ok().filter(|id| *id > 0)
}

fn normalize_status_for_db(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "publish" | "published" => "published",
        "future" | "scheduled" => "scheduled",
        "pending_review" => "pending_review",
        "archived" | "archive" => "archived",
        "rejected" => "rejected",
        _ => "draft",
    }
}

fn map_db_status_to_editorial(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "published" => "publish".to_string(),
        "scheduled" => "future".to_string(),
        other => other.to_string(),
    }
}

fn normalize_en_status_for_db(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "approved" => "approved",
        "auto_draft" | "draft" | "review" | "pending_review" => "auto_draft",
        _ => "none",
    }
}

fn normalize_channel_for_db(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "magazine" | "print" => "magazine",
        "both" | "print_web" => "both",
        _ => "web",
    }
}

fn channel_to_source_group(channel: &str) -> &'static str {
    match channel {
        "magazine" | "both" => "dergi-icerigi",
        _ => "gunluk-haber",
    }
}

fn strip_legacy_icerik_prefix(path: &str) -> String {
    let clean = path.trim();
    clean
        .strip_prefix("/icerik/")
        .or_else(|| clean.strip_prefix("/haberler/"))
        .unwrap_or(clean)
        .trim_start_matches('/')
        .to_string()
}

fn derive_category_color(slug: &str) -> String {
    let palette = [
        "#3b66c4", "#0284c7", "#0d9488", "#059669", "#d97706", "#dc2626", "#7c3aed", "#4f46e5",
    ];
    let hash: usize = slug
        .bytes()
        .fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
    palette[hash % palette.len()].to_string()
}

fn resolve_unique_content_slug(
    conn: &Connection,
    desired_slug: &str,
    existing_id: Option<i64>,
) -> String {
    let base = slugify_turkish(desired_slug);
    let base = if base.is_empty() {
        "savunma-icerik".to_string()
    } else {
        base
    };

    for idx in 0..100 {
        let candidate = if idx == 0 {
            base.clone()
        } else {
            format!("{}-{}", base, idx + 1)
        };
        let conflict: Option<i64> = conn
            .query_row(
                "SELECT id FROM contents WHERE LOWER(slug) = LOWER(?1) LIMIT 1",
                params![candidate],
                |r| r.get(0),
            )
            .optional()
            .unwrap_or(None);

        match (conflict, existing_id) {
            (None, _) => return candidate,
            (Some(found_id), Some(curr_id)) if found_id == curr_id => return candidate,
            _ => continue,
        }
    }
    format!("{}-{}", base, chrono::Utc::now().timestamp())
}

fn sanitize_editorial_blocks(blocks: &[ContentBlock]) -> Vec<ContentBlock> {
    blocks
        .iter()
        .map(|b| {
            let clean_content = match b.block_type.as_str() {
                "gallery" | "related_article" | "image" | "video" | "embed" | "divider" => {
                    b.content.trim().to_string()
                }
                _ => sanitize_inline_rich_html(&b.content),
            };
            ContentBlock {
                id: b.id.clone(),
                block_type: b.block_type.clone(),
                content: clean_content,
                meta: b.meta.clone(),
            }
        })
        .collect()
}

struct ContentRowMaps {
    placements_by_content: HashMap<i64, Vec<i32>>,
    ssb_by_content: HashMap<i64, Vec<String>>,
    slug_to_cat_id: HashMap<String, i32>,
}

fn load_content_row_maps(conn: &Connection) -> ContentRowMaps {
    let mut slug_to_cat_id = HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT id, slug FROM categories") {
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))) {
            for r in rows.flatten() {
                slug_to_cat_id.insert(r.1.to_ascii_lowercase(), r.0 as i32);
            }
        }
    }

    let mut placements_by_content: HashMap<i64, Vec<i32>> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT content_id, category_slug FROM content_placements") {
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))) {
            for r in rows.flatten() {
                if let Some(&cid) = slug_to_cat_id.get(&r.1.to_ascii_lowercase()) {
                    let entry = placements_by_content.entry(r.0).or_default();
                    if !entry.contains(&cid) {
                        entry.push(cid);
                    }
                }
            }
        }
    }

    let mut ssb_by_content: HashMap<i64, Vec<String>> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT content_id, ssb_code FROM content_ssb_tags") {
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))) {
            for r in rows.flatten() {
                ssb_by_content.entry(r.0).or_default().push(r.1);
            }
        }
    }

    ContentRowMaps {
        placements_by_content,
        ssb_by_content,
        slug_to_cat_id,
    }
}

fn map_content_row(
    r: &rusqlite::Row<'_>,
    maps: &ContentRowMaps,
) -> rusqlite::Result<Article> {
    let id: i64 = r.get(0)?;
    let slug: String = r.get(1)?;
    let title_tr: String = r.get(2)?;
    let title_en: String = r.get(3)?;
    let summary_tr: String = r.get(4)?;
    let summary_en: String = r.get(5)?;
    let body_tr: String = r.get(6)?;
    let body_en: String = r.get(7)?;
    let content_type: String = r.get(8)?;
    let cover_image: String = r.get(9)?;
    let author_name: String = r.get(10)?;
    let author_title: String = r.get(11)?;
    let issue_number: Option<i32> = r.get(12)?;
    let published_at: String = r.get(13)?;
    let read_time_min: i32 = r.get(14)?;
    let visibility_level: i32 = r.get(15)?;
    let single_price_try: f64 = r.get(16)?;
    let is_sponsored_int: i32 = r.get(17)?;
    let sponsor_brand: Option<String> = r.get(18)?;
    let sponsor_legal_label: Option<String> = r.get(19)?;
    let is_featured_int: i32 = r.get(20)?;
    let video_url: Option<String> = r.get(21)?;
    let view_count: i32 = r.get(22)?;
    let status_raw: String = r.get(23)?;
    let en_status_raw: String = r.get(24)?;
    let category_id_col: Option<i32> = r.get(25)?;
    let primary_category: String = r.get(26)?;
    let category_ids_json: String = r.get(27)?;
    let blocks_tr_json: String = r.get(28)?;
    let blocks_en_json: String = r.get(29)?;
    let seo_json: String = r.get(30)?;
    let tags_json: String = r.get(31)?;
    let scheduled_at: Option<String> = r.get(32)?;
    let issue_page_range: Option<String> = r.get(33)?;
    let publication_channel: String = r.get(34)?;
    let cover_width: Option<i32> = r.get(35)?;
    let cover_height: Option<i32> = r.get(36)?;
    let original_url: Option<String> = r.get(37)?;
    let social_shared_int: i32 = r.get(38)?;
    let created_by: Option<i32> = r.get(39)?;
    let updated_at: Option<String> = r.get(40)?;

    let mut cat_ids: Vec<i32> = serde_json::from_str(&category_ids_json).unwrap_or_default();
    if cat_ids.is_empty() {
        if let Some(cid) = category_id_col {
            cat_ids.push(cid);
        }
        if let Some(placed) = maps.placements_by_content.get(&id) {
            for &pid in placed {
                if !cat_ids.contains(&pid) {
                    cat_ids.push(pid);
                }
            }
        }
        if cat_ids.is_empty() && !primary_category.is_empty() {
            if let Some(&cid) = maps.slug_to_cat_id.get(&primary_category.to_ascii_lowercase()) {
                cat_ids.push(cid);
            }
        }
    }

    let mut blocks_tr: Vec<ContentBlock> = serde_json::from_str(&blocks_tr_json).unwrap_or_default();
    if blocks_tr.is_empty() && !body_tr.trim().is_empty() {
        blocks_tr = parse_content_to_blocks(&body_tr);
    }

    let mut blocks_en: Vec<ContentBlock> = serde_json::from_str(&blocks_en_json).unwrap_or_default();
    if blocks_en.is_empty() && !body_en.trim().is_empty() {
        blocks_en = parse_content_to_blocks(&body_en);
    }

    let seo: Option<SeoData> = serde_json::from_str(&seo_json)
        .ok()
        .filter(|s: &SeoData| {
            !s.title.is_empty() || !s.description.is_empty() || !s.keywords.is_empty()
        });
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let ssb_tags: Vec<String> = maps.ssb_by_content.get(&id).cloned().unwrap_or_default();

    let status_wire = map_db_status_to_editorial(&status_raw);
    let cover_trimmed = cover_image.trim();
    let image_opt = if cover_trimmed.is_empty() {
        None
    } else if cover_trimmed.starts_with('/')
        || cover_trimmed.starts_with("http://")
        || cover_trimmed.starts_with("https://")
    {
        Some(cover_trimmed.to_string())
    } else {
        Some(format!("/{cover_trimmed}"))
    };
    let pub_date_opt = if published_at.trim().is_empty() {
        None
    } else {
        Some(published_at.clone())
    };
    let public_path = format!("/icerik/{slug}");

    Ok(Article {
        id: id.to_string(),
        article_type: Some(status_raw),
        title: title_tr,
        content: body_tr,
        blocks: Some(blocks_tr),
        status: status_wire,
        source_url: original_url.clone(),
        source_url_camel: original_url.clone(),
        image_url: image_opt.clone(),
        image_url_camel: image_opt,
        source_time: None,
        source_time_camel: None,
        original_url: original_url.clone(),
        original_url_camel: original_url,
        publish_date: pub_date_opt.clone(),
        publish_date_camel: pub_date_opt.clone(),
        created_at: updated_at.or(pub_date_opt),
        created_at_camel: None,
        wp_post_id: Some(id as i32),
        wp_post_id_camel: Some(id as i32),
        wp_post_url: Some(public_path.clone()),
        wp_post_url_camel: Some(public_path),
        wp_site_id: None,
        wp_site_id_camel: None,
        assigned_to: created_by,
        publisher_name: Some(author_name.clone()).filter(|s| !s.is_empty()),
        scheduled_at: scheduled_at.clone(),
        scheduled_at_camel: scheduled_at,
        target_sites: None,
        categories: Some(cat_ids),
        tags: Some(tags),
        spot: Some(summary_tr).filter(|s| !s.trim().is_empty()),
        slug: Some(slug),
        is_live_blog: false,
        comments_locked: false,
        ads_disabled: false,
        cover_width,
        cover_height,
        thumb_url: None,
        seo,
        auto_share: Some(social_shared_int != 0),
        views: view_count,
        comments: 0,
        title_en: Some(title_en).filter(|s| !s.trim().is_empty()),
        spot_en: Some(summary_en).filter(|s| !s.trim().is_empty()),
        content_en: Some(body_en.clone()).filter(|s| !s.trim().is_empty()),
        blocks_en: if blocks_en.is_empty() {
            None
        } else {
            Some(blocks_en)
        },
        en_status: en_status_raw,
        publication_channel: if publication_channel.trim().is_empty() {
            "web".to_string()
        } else {
            publication_channel
        },
        content_type: if content_type.trim().is_empty() {
            "haberler".to_string()
        } else {
            content_type
        },
        issue_number,
        issue_page_range,
        is_featured: is_featured_int != 0,
        author_name: Some(author_name).filter(|s| !s.trim().is_empty()),
        author_title: Some(author_title).filter(|s| !s.trim().is_empty()),
        video_url: video_url.filter(|s| !s.trim().is_empty()),
        read_time_min: Some(read_time_min),
        visibility_level,
        single_price_try,
        is_sponsored: is_sponsored_int != 0,
        sponsor_brand: sponsor_brand.filter(|s| !s.trim().is_empty()),
        sponsor_legal_label: sponsor_legal_label.filter(|s| !s.trim().is_empty()),
        ssb_tags: Some(ssb_tags),
    })
}

const EDITORIAL_SELECT_COLS: &str = "
    SELECT
        id, slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
        content_type, cover_image, author_name, author_title, issue_number,
        published_at, read_time_min, visibility_level, single_price_try,
        is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
        view_count, status, en_status, category_id, primary_category,
        category_ids_json, blocks_tr_json, blocks_en_json, seo_json, tags_json,
        scheduled_at, issue_page_range, publication_channel, cover_width,
        cover_height, original_url, social_shared, created_by, updated_at
    FROM contents
";

impl Database {
    /// Parses a positive content ID (`"42"` or `"msi-42"`).
    pub fn parse_editorial_content_id(raw: &str) -> Option<i64> {
        parse_editorial_content_id(raw)
    }

    /// Lists MSI articles formatted cleanly for the Editorial Studio WASM client, with status stats.
    pub fn editorial_list_articles(
        &self,
        q: &EditorialArticleQuery,
    ) -> Result<EditorialArticleListResponse, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let maps = load_content_row_maps(&conn);

        let stats = conn
            .query_row(
                "SELECT
                    COUNT(*),
                    SUM(CASE WHEN status = 'published' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'draft' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'pending_review' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'scheduled' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'archived' THEN 1 ELSE 0 END)
                 FROM contents",
                [],
                |r| {
                    let pub_c: i64 = r.get::<_, Option<i64>>(1)?.unwrap_or(0);
                    let draft_c: i64 = r.get::<_, Option<i64>>(2)?.unwrap_or(0);
                    let pend_c: i64 = r.get::<_, Option<i64>>(3)?.unwrap_or(0);
                    let fut_c: i64 = r.get::<_, Option<i64>>(4)?.unwrap_or(0);
                    let arch_c: i64 = r.get::<_, Option<i64>>(5)?.unwrap_or(0);
                    Ok(EditorialStats {
                        total: pub_c + draft_c + pend_c + fut_c,
                        published: pub_c,
                        draft: draft_c,
                        pending_review: pend_c,
                        future: fut_c,
                        archived: arch_c,
                    })
                },
            )
            .unwrap_or_default();

        let sql = format!(
            "{EDITORIAL_SELECT_COLS} ORDER BY COALESCE(NULLIF(TRIM(published_at), ''), updated_at, '0000-00-00') DESC, id DESC"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let all_rows: Vec<Article> = stmt
            .query_map([], |r| map_content_row(r, &maps))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();

        let is_non_published_bucket = q.status.is_none()
            && q.article_type
                .as_deref()
                .map(|t| t.trim().eq_ignore_ascii_case("draft"))
                .unwrap_or(false);

        let status_filter = if is_non_published_bucket {
            None
        } else {
            q.status
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty() && *s != "all")
                .map(normalize_status_for_db)
                .or_else(|| {
                    match q
                        .article_type
                        .as_deref()
                        .map(str::trim)
                        .unwrap_or("")
                        .to_ascii_lowercase()
                        .as_str()
                    {
                        "published" | "publish" => Some("published"),
                        "pending_review" => Some("pending_review"),
                        "future" | "scheduled" => Some("scheduled"),
                        "archived" => Some("archived"),
                        _ => None,
                    }
                })
        };

        let search_lower = q
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);

        let filtered: Vec<Article> = all_rows
            .into_iter()
            .filter(|art| {
                let art_db_status = art.article_type.as_deref().unwrap_or("draft");
                if is_non_published_bucket {
                    if art_db_status == "published" {
                        return false;
                    }
                } else if let Some(target_st) = status_filter {
                    if art_db_status != target_st {
                        return false;
                    }
                } else if art_db_status == "archived" {
                    return false;
                }

                if let Some(cid) = q.category_id {
                    let has_cat = art
                        .categories
                        .as_ref()
                        .map(|c| c.contains(&cid))
                        .unwrap_or(false);
                    if !has_cat {
                        return false;
                    }
                }

                if let Some(ref needle) = search_lower {
                    let hay = format!(
                        "{} {} {} {} {}",
                        art.title.to_lowercase(),
                        art.spot.as_deref().unwrap_or("").to_lowercase(),
                        art.slug.as_deref().unwrap_or("").to_lowercase(),
                        art.title_en.as_deref().unwrap_or("").to_lowercase(),
                        art.author_name.as_deref().unwrap_or("").to_lowercase(),
                    );
                    if !hay.contains(needle) {
                        return false;
                    }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = q.offset.max(0) as usize;
        let limit = q.limit.clamp(1, 200) as usize;
        let paged = filtered.into_iter().skip(offset).take(limit).collect();

        Ok(EditorialArticleListResponse {
            articles: paged,
            total,
            stats,
        })
    }

    /// Loads a single MSI content item in Editorial Studio `Article` format.
    pub fn editorial_get_article_by_id(&self, content_id: i64) -> Result<Option<Article>, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let maps = load_content_row_maps(&conn);
        let sql = format!("{EDITORIAL_SELECT_COLS} WHERE id = ?1 LIMIT 1");
        conn.query_row(&sql, params![content_id], |r| map_content_row(r, &maps))
            .optional()
            .map_err(|e| e.to_string())
    }

    /// Creates or updates an MSI article from Editorial Studio, syncing blocks, SEO, placements, SSB tags,
    /// revision history, approval requests, 301 redirects, and public query caches.
    pub fn editorial_upsert_article(
        &self,
        input: &Article,
        actor_user_id: i64,
        actor_email: &str,
        actor_name: &str,
        revision_note: Option<&str>,
    ) -> Result<EditorialUpsertOutcome, String> {
        let title_tr = input.title.trim().to_string();
        if title_tr.is_empty() {
            return Err("Haber/içerik başlığı zorunludur.".to_string());
        }

        let clean_blocks_tr = sanitize_editorial_blocks(input.blocks.as_deref().unwrap_or(&[]));
        let raw_body_tr = if !clean_blocks_tr.is_empty() {
            blocks_to_wp_html(&clean_blocks_tr)
        } else {
            input.content.trim().to_string()
        };
        let body_tr = sanitize_rich_html(&raw_body_tr);
        let blocks_tr_final = if !clean_blocks_tr.is_empty() {
            clean_blocks_tr
        } else if !body_tr.trim().is_empty() {
            parse_content_to_blocks(&body_tr)
        } else {
            Vec::new()
        };

        let clean_blocks_en = sanitize_editorial_blocks(input.blocks_en.as_deref().unwrap_or(&[]));
        let raw_body_en = if !clean_blocks_en.is_empty() {
            blocks_to_wp_html(&clean_blocks_en)
        } else {
            input.content_en.as_deref().unwrap_or("").trim().to_string()
        };
        let body_en = sanitize_rich_html(&raw_body_en);
        let blocks_en_final = if !clean_blocks_en.is_empty() {
            clean_blocks_en
        } else if !body_en.trim().is_empty() {
            parse_content_to_blocks(&body_en)
        } else {
            Vec::new()
        };

        let title_en = input.title_en.as_deref().unwrap_or("").trim().to_string();
        let summary_tr = input.spot.as_deref().unwrap_or("").trim().to_string();
        let summary_en = input.spot_en.as_deref().unwrap_or("").trim().to_string();
        let db_status = normalize_status_for_db(&input.status);
        let db_en_status = normalize_en_status_for_db(&input.en_status);
        let db_channel = normalize_channel_for_db(&input.publication_channel);
        let source_group = channel_to_source_group(db_channel);
        let content_type = if input.content_type.trim().is_empty() {
            "haberler".to_string()
        } else {
            input.content_type.trim().to_string()
        };

        let cover_image = input
            .effective_image_url()
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        let author_name = input
            .author_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(if actor_name.trim().is_empty() {
                actor_email
            } else {
                actor_name
            })
            .to_string();
        let author_title = input
            .author_title
            .as_deref()
            .map(str::trim)
            .unwrap_or("Editör")
            .to_string();
        let read_time_min = input
            .read_time_min
            .filter(|m| *m > 0)
            .unwrap_or_else(|| estimate_read_time_minutes(&title_tr, &summary_tr, &blocks_tr_final));

        let scheduled_at = input
            .effective_scheduled_at()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let now_iso = chrono::Utc::now().to_rfc3339();
        let pub_now = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
        let published_at = input
            .publish_date
            .as_deref()
            .or(input.publish_date_camel.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| {
                if db_status == "scheduled" {
                    scheduled_at.clone()
                } else {
                    None
                }
            })
            .unwrap_or(pub_now);

        let blocks_tr_json =
            serde_json::to_string(&blocks_tr_final).unwrap_or_else(|_| "[]".to_string());
        let blocks_en_json =
            serde_json::to_string(&blocks_en_final).unwrap_or_else(|_| "[]".to_string());
        let seo_json = input
            .seo
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok())
            .unwrap_or_else(|| "{}".to_string());
        let tags_json = input
            .tags
            .as_ref()
            .and_then(|t| serde_json::to_string(t).ok())
            .unwrap_or_else(|| "[]".to_string());

        let cat_ids: Vec<i32> = input
            .categories
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|id| *id > 0)
            .collect();
        let category_ids_json =
            serde_json::to_string(&cat_ids).unwrap_or_else(|_| "[]".to_string());

        let tr_hash = Self::compute_tr_source_hash(&title_tr, &summary_tr, &body_tr);
        let ssb_tags: Vec<String> = input
            .ssb_tags
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let existing_cats = self.editorial_list_categories();
        let cat_by_id: HashMap<i32, &Category> =
            existing_cats.iter().map(|c| (c.id, c)).collect();
        let first_cat_id = cat_ids.first().copied();
        let primary_cat_slug = first_cat_id
            .and_then(|id| cat_by_id.get(&id))
            .map(|c| c.slug.clone())
            .unwrap_or_else(|| content_type.clone());

        let desired_slug = input
            .slug
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&title_tr);

        let requested_id = parse_editorial_content_id(&input.id);

        let mut conn = self.open_conn().map_err(|e| e.to_string())?;
        let existing_row: Option<(i64, String, String)> = requested_id.and_then(|cid| {
            conn.query_row(
                "SELECT id, slug, status FROM contents WHERE id = ?1 LIMIT 1",
                params![cid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .unwrap_or(None)
        });

        let existing_id = existing_row.as_ref().map(|r| r.0);
        let unique_slug = resolve_unique_content_slug(&conn, desired_slug, existing_id);
        let original_url = input.effective_original_url().map(str::to_string);
        let issue_number = input.issue_number.filter(|n| {
            *n > 0
                && conn
                    .query_row(
                        "SELECT 1 FROM magazine_issues WHERE issue_number = ?1 LIMIT 1",
                        params![n],
                        |_| Ok(()),
                    )
                    .is_ok()
        });

        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let (content_id, was_insert) = if let Some(cid) = existing_id {
            tx.execute(
                "UPDATE contents SET
                    slug = ?1, title_tr = ?2, title_en = ?3, summary_tr = ?4, summary_en = ?5,
                    body_tr = ?6, body_en = ?7, content_type = ?8, source_group = ?9,
                    cover_image = ?10, author_name = ?11, author_title = ?12,
                    issue_number = ?13, published_at = ?14, read_time_min = ?15,
                    visibility_level = ?16, single_price_try = ?17, is_sponsored = ?18,
                    sponsor_brand = ?19, sponsor_legal_label = ?20, is_featured = ?21,
                    video_url = ?22, status = ?23, en_status = ?24, tr_source_hash = ?25,
                    category_id = ?26, primary_category = ?27, category_ids_json = ?28,
                    blocks_tr_json = ?29, blocks_en_json = ?30, seo_json = ?31, tags_json = ?32,
                    scheduled_at = ?33, issue_page_range = ?34, publication_channel = ?35,
                    cover_width = ?36, cover_height = ?37, original_url = ?38,
                    social_shared = ?39, updated_at = ?40
                 WHERE id = ?41",
                params![
                    unique_slug,
                    title_tr,
                    title_en,
                    summary_tr,
                    summary_en,
                    body_tr,
                    body_en,
                    content_type,
                    source_group,
                    cover_image,
                    author_name,
                    author_title,
                    issue_number,
                    published_at,
                    read_time_min,
                    input.visibility_level.clamp(0, 2),
                    input.single_price_try.max(0.0),
                    if input.is_sponsored { 1 } else { 0 },
                    input.sponsor_brand,
                    input.sponsor_legal_label,
                    if input.is_featured { 1 } else { 0 },
                    input.video_url,
                    db_status,
                    db_en_status,
                    tr_hash,
                    first_cat_id,
                    primary_cat_slug,
                    category_ids_json,
                    blocks_tr_json,
                    blocks_en_json,
                    seo_json,
                    tags_json,
                    scheduled_at,
                    input.issue_page_range,
                    db_channel,
                    input.cover_width,
                    input.cover_height,
                    original_url,
                    if input.auto_share.unwrap_or(false) { 1 } else { 0 },
                    now_iso,
                    cid,
                ],
            )
            .map_err(|e| e.to_string())?;
            (cid, false)
        } else {
            tx.execute(
                "INSERT INTO contents (
                    slug, title_tr, title_en, summary_tr, summary_en,
                    body_tr, body_en, content_type, source_group, cover_image,
                    author_name, author_title, issue_number, published_at, read_time_min,
                    visibility_level, single_price_try, is_sponsored, sponsor_brand,
                    sponsor_legal_label, is_featured, video_url, view_count, true_read_count,
                    status, en_status, tr_source_hash,
                    category_id, primary_category, category_ids_json,
                    blocks_tr_json, blocks_en_json, seo_json, tags_json,
                    scheduled_at, issue_page_range, publication_channel,
                    cover_width, cover_height, original_url, social_shared, created_by, updated_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    ?6, ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13, ?14, ?15,
                    ?16, ?17, ?18, ?19,
                    ?20, ?21, ?22, 0, 0,
                    ?23, ?24, ?25,
                    ?26, ?27, ?28,
                    ?29, ?30, ?31, ?32,
                    ?33, ?34, ?35,
                    ?36, ?37, ?38, ?39, ?40, ?41
                )",
                params![
                    unique_slug,
                    title_tr,
                    title_en,
                    summary_tr,
                    summary_en,
                    body_tr,
                    body_en,
                    content_type,
                    source_group,
                    cover_image,
                    author_name,
                    author_title,
                    issue_number,
                    published_at,
                    read_time_min,
                    input.visibility_level.clamp(0, 2),
                    input.single_price_try.max(0.0),
                    if input.is_sponsored { 1 } else { 0 },
                    input.sponsor_brand,
                    input.sponsor_legal_label,
                    if input.is_featured { 1 } else { 0 },
                    input.video_url,
                    db_status,
                    db_en_status,
                    tr_hash,
                    first_cat_id,
                    primary_cat_slug,
                    category_ids_json,
                    blocks_tr_json,
                    blocks_en_json,
                    seo_json,
                    tags_json,
                    scheduled_at,
                    input.issue_page_range,
                    db_channel,
                    input.cover_width,
                    input.cover_height,
                    original_url,
                    if input.auto_share.unwrap_or(false) { 1 } else { 0 },
                    if actor_user_id > 0 { Some(actor_user_id) } else { None },
                    now_iso,
                ],
            )
            .map_err(|e| e.to_string())?;
            (tx.last_insert_rowid(), true)
        };

        // Sync category placements (including ancestor categories)
        tx.execute(
            "DELETE FROM content_placements WHERE content_id = ?1",
            params![content_id],
        )
        .map_err(|e| e.to_string())?;

        let mut placement_slugs: HashSet<String> = HashSet::new();
        for &cid in &cat_ids {
            for anc_id in category_ancestor_chain(&existing_cats, cid) {
                if let Some(c) = cat_by_id.get(&anc_id) {
                    placement_slugs.insert(c.slug.clone());
                }
            }
        }
        if placement_slugs.is_empty() && !content_type.is_empty() {
            placement_slugs.insert(content_type.clone());
        }
        for slug_val in placement_slugs {
            tx.execute(
                "INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)",
                params![content_id, slug_val],
            )
            .map_err(|e| e.to_string())?;
        }

        // Sync SSB tags
        tx.execute(
            "DELETE FROM content_ssb_tags WHERE content_id = ?1",
            params![content_id],
        )
        .map_err(|e| e.to_string())?;
        for code in ssb_tags {
            tx.execute(
                "INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)",
                params![content_id, code],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())?;
        drop(conn);

        // 301 Redirect automatically on published slug change
        let mut redirect_created = None;
        if let Some((_, old_slug, old_status)) = existing_row {
            if !old_slug.is_empty()
                && !old_slug.eq_ignore_ascii_case(&unique_slug)
                && old_status == "published"
            {
                let old_path = format!("/icerik/{old_slug}");
                let new_path = format!("/icerik/{unique_slug}");
                if let Ok(rd_id) = self.insert_seo_redirect(&SeoRedirectRule {
                    id: 0,
                    old_path,
                    new_path,
                    status_code: 301,
                    hit_count: 0,
                }) {
                    redirect_created = Some(UrlRedirect {
                        id: rd_id as i32,
                        old_slug,
                        new_slug: unique_slug.clone(),
                        article_id: content_id.to_string(),
                        status_code: 301,
                        hits: 0,
                        created_at: Some(now_iso.clone()),
                    });
                }
            }
        }

        // Snapshot bilingual revision
        let note = revision_note.unwrap_or(if was_insert {
            "İlk stüdyo kaydı"
        } else if db_status == "published" {
            "Stüdyo üzerinden yayımlandı"
        } else {
            "Stüdyo güncellemesi"
        });
        let _ = self.insert_content_version_bilingual(
            content_id,
            &title_tr,
            &title_en,
            &body_tr,
            &body_en,
            &summary_tr,
            &summary_en,
            actor_email,
            note,
        );

        // Sync approval queue when transitioning to/from pending_review
        if db_status == "pending_review" && actor_user_id > 0 {
            if let Ok(conn_chk) = self.open_conn() {
                let already_pending: bool = conn_chk
                    .query_row(
                        "SELECT 1 FROM approval_requests WHERE content_id = ?1 AND status = 'pending' LIMIT 1",
                        params![content_id],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);
                if !already_pending {
                    let _ = self.create_approval_request(content_id, actor_user_id);
                }
            }
        } else if db_status == "published" && actor_user_id > 0 {
            if let Ok(conn_upd) = self.open_conn() {
                let _ = conn_upd.execute(
                    "UPDATE approval_requests SET status = 'approved', reviewed_by = ?2, reviewed_at = ?3
                     WHERE content_id = ?1 AND status = 'pending'",
                    params![content_id, actor_user_id, now_iso],
                );
            }
        }

        let _ = self.insert_audit_log(
            actor_email,
            if was_insert {
                "editorial_create"
            } else {
                "editorial_update"
            },
            "content",
            Some(content_id),
            &format!("{{\"slug\":\"{}\",\"status\":\"{}\"}}", unique_slug, db_status),
            "",
        );

        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);

        let saved = self
            .editorial_get_article_by_id(content_id)?
            .ok_or_else(|| "Kaydedilen içerik okunamadı.".to_string())?;

        Ok(EditorialUpsertOutcome {
            article: saved,
            redirect_created,
            was_insert,
        })
    }

    /// Deletes an MSI article safely along with pending approval rows.
    pub fn editorial_delete_article(
        &self,
        content_id: i64,
        actor_email: &str,
    ) -> Result<bool, String> {
        let existing = self.editorial_get_article_by_id(content_id)?;
        if existing.is_none() {
            return Ok(false);
        }
        self.delete_content(content_id).map_err(|e| e.to_string())?;
        let _ = self.insert_audit_log(
            actor_email,
            "editorial_delete",
            "content",
            Some(content_id),
            "{}",
            "",
        );
        Ok(true)
    }

    /// Lists content revisions formatted as `ArticleRevision` items for the Editorial Studio UI.
    pub fn editorial_list_revisions(&self, content_id: i64) -> Vec<ArticleRevision> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, content_id, version, title_tr, body_tr, summary_tr, changed_by,
                    change_note, created_at, title_en, summary_en, body_en, en_status
             FROM content_versions
             WHERE content_id = ?1
             ORDER BY version DESC, id DESC
             LIMIT 40",
        ) else {
            return Vec::new();
        };
        stmt.query_map(params![content_id], |r| {
            Ok(ArticleRevision {
                id: r.get::<_, i64>(0)? as i32,
                article_id: r.get::<_, i64>(1)?.to_string(),
                editor_id: None,
                editor_name: r.get(6)?,
                title: r.get(3)?,
                spot: Some(r.get::<_, String>(5)?).filter(|s| !s.is_empty()),
                slug: None,
                content: r.get(4)?,
                image_url: None,
                status: "revision".to_string(),
                categories: None,
                tags: None,
                is_live_blog: false,
                comments_locked: false,
                ads_disabled: false,
                title_en: Some(r.get::<_, String>(9)?).filter(|s| !s.is_empty()),
                spot_en: Some(r.get::<_, String>(10)?).filter(|s| !s.is_empty()),
                content_en: Some(r.get::<_, String>(11)?).filter(|s| !s.is_empty()),
                en_status: r.get(12)?,
                publication_channel: "web".to_string(),
                content_type: "haberler".to_string(),
                issue_number: None,
                note: Some(r.get::<_, String>(7)?).filter(|s| !s.is_empty()),
                created_at: Some(r.get(8)?),
            })
        })
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
    }

    /// Restores a revision by `content_versions.id` or `version` number, returning updated `Article`.
    pub fn editorial_restore_revision(
        &self,
        content_id: i64,
        rev_or_ver_id: i64,
        actor_email: &str,
    ) -> Result<Article, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let ver: Option<i64> = conn
            .query_row(
                "SELECT version FROM content_versions
                 WHERE content_id = ?1 AND (id = ?2 OR version = ?2)
                 LIMIT 1",
                params![content_id, rev_or_ver_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        drop(conn);

        let version = ver.ok_or_else(|| "Revizyon bulunamadı.".to_string())?;
        self.restore_content_version(content_id, version, actor_email)?;

        // Re-sync structured blocks JSON after restore
        if let Some(updated) = self.editorial_get_article_by_id(content_id)? {
            if let Ok(conn2) = self.open_conn() {
                let b_tr = parse_content_to_blocks(&updated.content);
                let b_en = updated
                    .content_en
                    .as_deref()
                    .map(parse_content_to_blocks)
                    .unwrap_or_default();
                let _ = conn2.execute(
                    "UPDATE contents SET blocks_tr_json = ?1, blocks_en_json = ?2 WHERE id = ?3",
                    params![
                        serde_json::to_string(&b_tr).unwrap_or_else(|_| "[]".to_string()),
                        serde_json::to_string(&b_en).unwrap_or_else(|_| "[]".to_string()),
                        content_id
                    ],
                );
            }
        }

        self.editorial_get_article_by_id(content_id)?
            .ok_or_else(|| "İçerik bulunamadı.".to_string())
    }

    /// Returns flat categories with resolved `parent_id` for `crate::editorial::common::build_category_tree`.
    pub fn editorial_list_categories(&self) -> Vec<Category> {
        let nodes = self.get_all_categories();
        let slug_to_id: HashMap<String, i32> = nodes
            .iter()
            .map(|c| (c.slug.to_ascii_lowercase(), c.id as i32))
            .collect();

        nodes
            .into_iter()
            .map(|c| {
                let parent_id = c
                    .parent_slug
                    .as_deref()
                    .map(|s| s.to_ascii_lowercase())
                    .and_then(|s| slug_to_id.get(&s).copied());
                Category {
                    id: c.id as i32,
                    name: c.name_tr,
                    slug: c.slug.clone(),
                    color: Some(derive_category_color(&c.slug)),
                    parent_id,
                    created_at: None,
                }
            })
            .collect()
    }

    /// Creates or updates an MSI category from Editorial Studio payloads.
    pub fn editorial_upsert_category(
        &self,
        id: Option<i64>,
        name: &str,
        slug_opt: Option<&str>,
        parent_id: Option<i32>,
    ) -> Result<Category, String> {
        let clean_name = name.trim();
        if clean_name.is_empty() {
            return Err("Kategori adı zorunludur.".to_string());
        }
        let existing_nodes = self.get_all_categories();
        let id_to_slug: HashMap<i32, String> = existing_nodes
            .iter()
            .map(|c| (c.id as i32, c.slug.clone()))
            .collect();
        let parent_slug = parent_id.and_then(|pid| id_to_slug.get(&pid).cloned());
        let base_slug = slug_opt
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(slugify_turkish)
            .unwrap_or_else(|| slugify_turkish(clean_name));

        let node = CategoryNode {
            id: id.unwrap_or(0),
            slug: if base_slug.is_empty() {
                "kategori".to_string()
            } else {
                base_slug
            },
            parent_slug,
            name_tr: clean_name.to_string(),
            name_en: clean_name.to_string(),
            group_type: "icerikler".to_string(),
            description_tr: String::new(),
            description_en: String::new(),
            icon_class: "fa-folder".to_string(),
            sort_order: 100,
            is_dynamic: true,
        };

        let saved_id = if let Some(cid) = id {
            let ok = self.update_category(cid, &node)?;
            if !ok {
                return Err("Kategori bulunamadı.".to_string());
            }
            cid
        } else {
            self.insert_category(&node).map_err(|e| e.to_string())?
        };

        Ok(self
            .editorial_list_categories()
            .into_iter()
            .find(|c| c.id == saved_id as i32)
            .unwrap_or(Category {
                id: saved_id as i32,
                name: node.name_tr,
                slug: node.slug.clone(),
                color: Some(derive_category_color(&node.slug)),
                parent_id,
                created_at: None,
            }))
    }

    pub fn editorial_delete_category(&self, id: i64) -> Result<bool, String> {
        self.delete_category(id)
    }

    /// Collects sorted tag suggestions from existing `contents.tags_json`, `ssb_tags`, and categories.
    pub fn editorial_list_tags(&self, query: Option<&str>) -> Vec<String> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        if let Ok(conn) = self.open_conn() {
            if let Ok(mut stmt) = conn.prepare("SELECT tags_json FROM contents WHERE tags_json != '[]'") {
                if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                    for json_str in rows.flatten() {
                        if let Ok(list) = serde_json::from_str::<Vec<String>>(&json_str) {
                            for tag in list {
                                let t = tag.trim().to_string();
                                if !t.is_empty() {
                                    *counts.entry(t).or_insert(0) += 3;
                                }
                            }
                        }
                    }
                }
            }
        }
        for ssb in self.get_all_ssb_tags() {
            counts.entry(ssb.name_tr).or_insert(1);
        }
        let needle = query
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);
        let mut pairs: Vec<(String, usize)> = counts
            .into_iter()
            .filter(|(k, _)| {
                if let Some(ref q) = needle {
                    k.to_lowercase().contains(q)
                } else {
                    true
                }
            })
            .collect();
        pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        pairs.into_iter().take(40).map(|(k, _)| k).collect()
    }

    /// Lists uploaded media items in Editorial Studio `MediaAsset` format.
    pub fn editorial_list_media(
        &self,
        query: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> (Vec<MediaAsset>, i64) {
        let Ok(conn) = self.open_conn() else {
            return (Vec::new(), 0);
        };
        let q_trim = query.map(str::trim).filter(|s| !s.is_empty());
        let like_pat = q_trim.map(|q| format!("%{}%", q.to_lowercase()));

        let total: i64 = if let Some(ref pat) = like_pat {
            conn.query_row(
                "SELECT COUNT(*) FROM media_library
                 WHERE LOWER(original_name) LIKE ?1 OR LOWER(alt_text_tr) LIKE ?1 OR LOWER(filename) LIKE ?1",
                params![pat],
                |r| r.get(0),
            )
            .unwrap_or(0)
        } else {
            conn.query_row("SELECT COUNT(*) FROM media_library", [], |r| r.get(0))
                .unwrap_or(0)
        };

        let sql = if like_pat.is_some() {
            "SELECT id, filename, original_name, path, thumb_url, mime_type, width, height,
                    file_size, watermarked, alt_text_tr, uploaded_by, created_at
             FROM media_library
             WHERE LOWER(original_name) LIKE ?1 OR LOWER(alt_text_tr) LIKE ?1 OR LOWER(filename) LIKE ?1
             ORDER BY id DESC LIMIT ?2 OFFSET ?3"
        } else {
            "SELECT id, filename, original_name, path, thumb_url, mime_type, width, height,
                    file_size, watermarked, alt_text_tr, uploaded_by, created_at
             FROM media_library
             ORDER BY id DESC LIMIT ?1 OFFSET ?2"
        };

        let Ok(mut stmt) = conn.prepare(sql) else {
            return (Vec::new(), 0);
        };
        let lim = limit.clamp(1, 100);
        let off = offset.max(0);
        let row_mapper = |r: &rusqlite::Row<'_>| -> rusqlite::Result<MediaAsset> {
            let wm_int: i32 = r.get::<_, Option<i32>>(9)?.unwrap_or(0);
            let alt_str: String = r.get::<_, Option<String>>(10)?.unwrap_or_default();
            Ok(MediaAsset {
                id: r.get::<_, i64>(0)? as i32,
                filename: r.get(1)?,
                original_name: r.get(2)?,
                url: r.get(3)?,
                thumb_url: r.get(4)?,
                mime_type: r.get(5)?,
                width: r.get::<_, Option<i32>>(6)?.unwrap_or(0),
                height: r.get::<_, Option<i32>>(7)?.unwrap_or(0),
                size_bytes: r.get(8)?,
                watermarked: wm_int != 0,
                alt_text: if alt_str.trim().is_empty() {
                    None
                } else {
                    Some(alt_str)
                },
                uploaded_by: Some(r.get(11)?),
                created_at: Some(r.get(12)?),
            })
        };

        let items = if let Some(ref pat) = like_pat {
            stmt.query_map(params![pat, lim, off], row_mapper)
                .map(|rows| rows.filter_map(Result::ok).collect())
                .unwrap_or_default()
        } else {
            stmt.query_map(params![lim, off], row_mapper)
                .map(|rows| rows.filter_map(Result::ok).collect())
                .unwrap_or_default()
        };

        (items, total)
    }

    /// Inserts a processed editorial image into `media_library`.
    pub fn editorial_insert_media(&self, item: &EditorialMediaInsert) -> Result<MediaAsset, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let alt_tr = item.alt_text.as_deref().unwrap_or("").trim().to_string();
        conn.execute(
            "INSERT INTO media_library (
                filename, original_name, mime_type, file_size, path,
                alt_text_tr, alt_text_en, tags, uploaded_by, created_at,
                thumb_url, width, height, watermarked
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', '[\"editorial\"]', ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                item.filename,
                item.original_name,
                item.mime_type,
                item.size_bytes,
                item.url,
                alt_tr,
                item.uploaded_by,
                now,
                item.thumb_url,
                item.width,
                item.height,
                if item.watermarked { 1 } else { 0 },
            ],
        )
        .map_err(|e| e.to_string())?;
        let id = conn.last_insert_rowid() as i32;
        Ok(MediaAsset {
            id,
            filename: item.filename.clone(),
            original_name: item.original_name.clone(),
            url: item.url.clone(),
            thumb_url: item.thumb_url.clone(),
            mime_type: item.mime_type.clone(),
            width: item.width,
            height: item.height,
            size_bytes: item.size_bytes,
            watermarked: item.watermarked,
            alt_text: if alt_tr.is_empty() {
                None
            } else {
                Some(alt_tr)
            },
            uploaded_by: Some(item.uploaded_by.clone()),
            created_at: Some(now),
        })
    }

    pub fn editorial_find_media_by_url(&self, raw_url: &str) -> Option<MediaAsset> {
        let clean = raw_url.split('?').next().unwrap_or(raw_url).trim();
        let conn = self.open_conn().ok()?;
        conn.query_row(
            "SELECT id, filename, original_name, path, thumb_url, mime_type, width, height,
                    file_size, watermarked, alt_text_tr, uploaded_by, created_at
             FROM media_library WHERE path = ?1 LIMIT 1",
            params![clean],
            |r| {
                let wm_int: i32 = r.get::<_, Option<i32>>(9)?.unwrap_or(0);
                let alt_str: String = r.get::<_, Option<String>>(10)?.unwrap_or_default();
                Ok(MediaAsset {
                    id: r.get::<_, i64>(0)? as i32,
                    filename: r.get(1)?,
                    original_name: r.get(2)?,
                    url: r.get(3)?,
                    thumb_url: r.get(4)?,
                    mime_type: r.get(5)?,
                    width: r.get::<_, Option<i32>>(6)?.unwrap_or(0),
                    height: r.get::<_, Option<i32>>(7)?.unwrap_or(0),
                    size_bytes: r.get(8)?,
                    watermarked: wm_int != 0,
                    alt_text: if alt_str.is_empty() { None } else { Some(alt_str) },
                    uploaded_by: Some(r.get(11)?),
                    created_at: Some(r.get(12)?),
                })
            },
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn editorial_delete_media(&self, id: i64) -> Result<Option<MediaAsset>, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let existing: Option<MediaAsset> = conn
            .query_row(
                "SELECT id, filename, original_name, path, thumb_url, mime_type, width, height,
                        file_size, watermarked, alt_text_tr, uploaded_by, created_at
                 FROM media_library WHERE id = ?1 LIMIT 1",
                params![id],
                |r| {
                    let wm_int: i32 = r.get::<_, Option<i32>>(9)?.unwrap_or(0);
                    let alt_str: String = r.get::<_, Option<String>>(10)?.unwrap_or_default();
                    Ok(MediaAsset {
                        id: r.get::<_, i64>(0)? as i32,
                        filename: r.get(1)?,
                        original_name: r.get(2)?,
                        url: r.get(3)?,
                        thumb_url: r.get(4)?,
                        mime_type: r.get(5)?,
                        width: r.get::<_, Option<i32>>(6)?.unwrap_or(0),
                        height: r.get::<_, Option<i32>>(7)?.unwrap_or(0),
                        size_bytes: r.get(8)?,
                        watermarked: wm_int != 0,
                        alt_text: if alt_str.is_empty() { None } else { Some(alt_str) },
                        uploaded_by: Some(r.get(11)?),
                        created_at: Some(r.get(12)?),
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if existing.is_some() {
            conn.execute("DELETE FROM media_library WHERE id = ?1", params![id])
                .map_err(|e| e.to_string())?;
        }
        Ok(existing)
    }

    pub fn editorial_list_ssb_tags(&self) -> Vec<SsbTag> {
        self.get_all_ssb_tags()
            .into_iter()
            .map(|t| SsbTag {
                id: t.id as i32,
                code: t.code,
                category_group: t.category_group,
                name_tr: t.name_tr,
                name_en: t.name_en,
                sort_order: t.sort_order,
            })
            .collect()
    }

    pub fn editorial_list_issues(&self) -> Vec<MagazineIssueItem> {
        self.get_all_issues()
            .into_iter()
            .map(|iss| {
                let is_en = iss.publication_type.eq_ignore_ascii_case("msi_tdr");
                MagazineIssueItem {
                    id: iss.issue_number,
                    issue_number: iss.issue_number,
                    edition_lang: if is_en { "en".to_string() } else { "tr".to_string() },
                    title_tr: iss.title_tr,
                    title_en: iss.title_en,
                    publish_month: if is_en {
                        iss.publish_month_en
                    } else {
                        iss.publish_month_tr
                    },
                    year: iss.year,
                    is_special_issue: iss.is_special_issue,
                    event_name: iss.event_name,
                }
            })
            .collect()
    }

    pub fn editorial_upsert_issue(
        &self,
        item: &MagazineIssueItem,
    ) -> Result<MagazineIssueItem, String> {
        if item.issue_number <= 0 {
            return Err("Geçerli bir sayı numarası girin.".to_string());
        }
        let pub_type = if item.edition_lang.eq_ignore_ascii_case("en") {
            "msi_tdr".to_string()
        } else {
            "msi".to_string()
        };
        let existing = self.get_issue(&pub_type, item.issue_number);
        let month_clean = item.publish_month.trim().to_string();
        let default_slug = if pub_type == "msi_tdr" {
            format!("msi-tdr-issue-{}", item.issue_number)
        } else {
            format!("sayi-{}", item.issue_number)
        };
        let iss = MagazineIssue {
            issue_number: item.issue_number,
            slug: existing
                .as_ref()
                .map(|e| e.slug.clone())
                .unwrap_or(default_slug),
            title_tr: if item.title_tr.trim().is_empty() {
                format!("MSI Dergisi {}. Sayı", item.issue_number)
            } else {
                item.title_tr.trim().to_string()
            },
            title_en: if item.title_en.trim().is_empty() {
                format!("MSI TDR Issue {}", item.issue_number)
            } else {
                item.title_en.trim().to_string()
            },
            publish_month_tr: if !month_clean.is_empty() {
                month_clean.clone()
            } else {
                existing
                    .as_ref()
                    .map(|e| e.publish_month_tr.clone())
                    .unwrap_or_else(|| "Kasım".to_string())
            },
            publish_month_en: if !month_clean.is_empty() {
                month_clean
            } else {
                existing
                    .as_ref()
                    .map(|e| e.publish_month_en.clone())
                    .unwrap_or_else(|| "November".to_string())
            },
            year: if item.year >= 2000 { item.year } else { 2026 },
            cover_image: existing
                .as_ref()
                .map(|e| e.cover_image.clone())
                .unwrap_or_else(|| "/images/logo.png".to_string()),
            is_special_issue: item.is_special_issue,
            event_name: item.event_name.clone(),
            page_count: existing.as_ref().map(|e| e.page_count).unwrap_or(84),
            dossier_count: existing.as_ref().map(|e| e.dossier_count).unwrap_or(1),
            summary_tr: existing
                .as_ref()
                .map(|e| e.summary_tr.clone())
                .unwrap_or_default(),
            summary_en: existing
                .as_ref()
                .map(|e| e.summary_en.clone())
                .unwrap_or_default(),
            pdf_path: existing
                .as_ref()
                .map(|e| e.pdf_path.clone())
                .unwrap_or_default(),
            publication_type: pub_type.clone(),
            uploaded_pdf_file: existing.as_ref().and_then(|e| e.uploaded_pdf_file.clone()),
            spreads: existing.as_ref().map(|e| e.spreads.clone()).unwrap_or_default(),
            en_status: existing
                .as_ref()
                .map(|e| e.en_status.clone())
                .unwrap_or_else(|| {
                    if pub_type == "msi_tdr" {
                        "approved".to_string()
                    } else {
                        "none".to_string()
                    }
                }),
            en_translated_at: existing.as_ref().and_then(|e| e.en_translated_at.clone()),
            en_approved_by: existing.as_ref().and_then(|e| e.en_approved_by.clone()),
            en_approved_at: existing.as_ref().and_then(|e| e.en_approved_at.clone()),
            tr_source_hash: existing.and_then(|e| e.tr_source_hash),
        };
        self.upsert_issue(&iss).map_err(|e| e.to_string())?;
        let target_lang = if pub_type == "msi_tdr" { "en" } else { "tr" };
        Ok(self
            .editorial_list_issues()
            .into_iter()
            .find(|i| i.issue_number == item.issue_number && i.edition_lang == target_lang)
            .unwrap_or_else(|| item.clone()))
    }

    pub fn editorial_delete_issue(
        &self,
        issue_num: i32,
        publication_type: Option<&str>,
    ) -> Result<bool, String> {
        if let Some(pt) = publication_type.map(str::trim).filter(|s| !s.is_empty()) {
            let norm = if pt.eq_ignore_ascii_case("en") || pt.eq_ignore_ascii_case("msi_tdr") {
                "msi_tdr"
            } else if pt.eq_ignore_ascii_case("tr") || pt.eq_ignore_ascii_case("msi") {
                "msi"
            } else {
                return Err("Geçersiz yayın türü (msi veya msi_tdr olmalıdır).".to_string());
            };
            return self
                .delete_issue_by_type(norm, issue_num)
                .map_err(|e| e.to_string());
        }
        let matches = self.get_issues_by_number(issue_num);
        if matches.len() > 1 {
            return Err(format!(
                "Sayı #{} hem msi hem de msi_tdr yayınında mevcut; silmek için publication_type belirtin.",
                issue_num
            ));
        }
        if let Some(only) = matches.first() {
            self.delete_issue_by_type(&only.publication_type, issue_num)
                .map_err(|e| e.to_string())
        } else {
            Ok(false)
        }
    }

    pub fn editorial_list_glossary(&self) -> Vec<GlossaryRule> {
        self.list_glossary_terms(None, true)
            .into_iter()
            .map(|t| GlossaryRule {
                id: t.id as i32,
                term_tr: t.term_tr,
                term_en: t.term_en,
                abbr_tr: if t.abbreviation_tr.trim().is_empty() {
                    None
                } else {
                    Some(t.abbreviation_tr)
                },
                abbr_en: if t.abbreviation_en.trim().is_empty() {
                    None
                } else {
                    Some(t.abbreviation_en)
                },
                category: if t.domain.trim().is_empty() {
                    "technical".to_string()
                } else {
                    t.domain
                },
                rule_type: t.rule_type,
                preferred_en: t.preferred_en,
                forbidden_variants: t.forbidden_variants,
                case_sensitive: t.case_sensitive,
                note: if t.notes.trim().is_empty() {
                    None
                } else {
                    Some(t.notes)
                },
            })
            .collect()
    }

    pub fn editorial_upsert_glossary(
        &self,
        rule: &GlossaryRule,
        actor_email: &str,
    ) -> Result<GlossaryRule, String> {
        let term = TranslationGlossaryTerm {
            id: if rule.id > 0 { rule.id as i64 } else { 0 },
            term_tr: rule.term_tr.trim().to_string(),
            term_en: rule.term_en.trim().to_string(),
            abbreviation_tr: rule.abbr_tr.as_deref().unwrap_or("").trim().to_string(),
            abbreviation_en: rule.abbr_en.as_deref().unwrap_or("").trim().to_string(),
            domain: if rule.category.trim().is_empty() {
                "technical".to_string()
            } else {
                rule.category.trim().to_string()
            },
            rule_type: if rule.rule_type.eq_ignore_ascii_case("forbidden") {
                "forbidden".to_string()
            } else if rule.rule_type.eq_ignore_ascii_case("preserve") {
                "preserve".to_string()
            } else {
                "fixed".to_string()
            },
            preferred_en: rule.preferred_en.trim().to_string(),
            forbidden_variants: rule.forbidden_variants.clone(),
            case_sensitive: rule.case_sensitive,
            priority: 10,
            source_ref: "Editorial Studio".to_string(),
            notes: rule.note.as_deref().unwrap_or("").trim().to_string(),
            is_active: true,
            created_by: actor_email.to_string(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        let saved_id = self.upsert_glossary_term(&term)?;
        Ok(self
            .editorial_list_glossary()
            .into_iter()
            .find(|g| g.id == saved_id as i32)
            .unwrap_or_else(|| {
                let mut r = rule.clone();
                r.id = saved_id as i32;
                r
            }))
    }

    pub fn editorial_delete_glossary(&self, id: i64) -> Result<bool, String> {
        self.delete_glossary_term(id)
    }

    pub fn editorial_list_redirects(&self) -> Vec<UrlRedirect> {
        let rules = self.get_seo_redirects();
        let slug_lookup: HashMap<String, String> = self
            .open_conn()
            .ok()
            .and_then(|conn| {
                let mut stmt = conn.prepare("SELECT id, slug FROM contents").ok()?;
                let mut map = HashMap::new();
                let rows = stmt
                    .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                    .ok()?;
                for (id, slug) in rows.flatten() {
                    map.insert(slug.to_ascii_lowercase(), id.to_string());
                }
                Some(map)
            })
            .unwrap_or_default();

        rules
            .into_iter()
            .map(|r| {
                let new_slug = strip_legacy_icerik_prefix(&r.new_path);
                let article_id = slug_lookup
                    .get(&new_slug.to_ascii_lowercase())
                    .cloned()
                    .unwrap_or_default();
                UrlRedirect {
                    id: r.id as i32,
                    old_slug: strip_legacy_icerik_prefix(&r.old_path),
                    new_slug,
                    article_id,
                    status_code: r.status_code as i32,
                    hits: r.hit_count as i32,
                    created_at: None,
                }
            })
            .collect()
    }

    pub fn editorial_delete_redirect(&self, id: i64) -> Result<bool, String> {
        self.delete_seo_redirect(id).map_err(|e| e.to_string())
    }

    // ── Research Sources ──────────────────────────────────────────────────────

    pub fn editorial_list_research_sources(&self) -> Vec<ResearchSource> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, name, url, kind, category, scope, enabled, created_at
             FROM editorial_research_sources
             ORDER BY enabled DESC, id DESC",
        ) else {
            return Vec::new();
        };
        stmt.query_map([], |r| {
            Ok(ResearchSource {
                id: r.get::<_, i64>(0)? as i32,
                name: r.get(1)?,
                url: r.get(2)?,
                kind: r.get(3)?,
                category: r.get(4)?,
                scope: r.get(5)?,
                enabled: r.get::<_, i32>(6)? != 0,
                created_at: Some(r.get(7)?),
            })
        })
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
    }

    pub fn editorial_create_research_source(
        &self,
        name: &str,
        url: &str,
        kind: &str,
        category: &str,
        scope: &str,
    ) -> Result<ResearchSource, String> {
        let clean_name = name.trim();
        let clean_url = url.trim();
        if clean_name.is_empty() || clean_url.is_empty() {
            return Err("Kaynak adı ve URL zorunludur.".to_string());
        }
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO editorial_research_sources (name, url, kind, category, scope, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)
             ON CONFLICT(url) DO UPDATE SET name = excluded.name, kind = excluded.kind,
                category = excluded.category, scope = excluded.scope, enabled = 1",
            params![clean_name, clean_url, kind, category, scope, now],
        )
        .map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT id, name, url, kind, category, scope, enabled, created_at
             FROM editorial_research_sources WHERE url = ?1 LIMIT 1",
            params![clean_url],
            |r| {
                Ok(ResearchSource {
                    id: r.get::<_, i64>(0)? as i32,
                    name: r.get(1)?,
                    url: r.get(2)?,
                    kind: r.get(3)?,
                    category: r.get(4)?,
                    scope: r.get(5)?,
                    enabled: r.get::<_, i32>(6)? != 0,
                    created_at: Some(r.get(7)?),
                })
            },
        )
        .map_err(|e| e.to_string())
    }

    pub fn editorial_update_research_source(
        &self,
        id: i32,
        name: Option<&str>,
        url: Option<&str>,
        kind: Option<&str>,
        category: Option<&str>,
        scope: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<Option<ResearchSource>, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let existing: Option<ResearchSource> = conn
            .query_row(
                "SELECT id, name, url, kind, category, scope, enabled, created_at
                 FROM editorial_research_sources WHERE id = ?1 LIMIT 1",
                params![id],
                |r| {
                    Ok(ResearchSource {
                        id: r.get::<_, i64>(0)? as i32,
                        name: r.get(1)?,
                        url: r.get(2)?,
                        kind: r.get(3)?,
                        category: r.get(4)?,
                        scope: r.get(5)?,
                        enabled: r.get::<_, i32>(6)? != 0,
                        created_at: Some(r.get(7)?),
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some(curr) = existing else {
            return Ok(None);
        };
        let next_name = name.map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&curr.name);
        let next_url = url.map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&curr.url);
        let next_kind = kind.map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&curr.kind);
        let next_cat = category
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&curr.category);
        let next_scope = scope.map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&curr.scope);
        let next_enabled = enabled.unwrap_or(curr.enabled);

        conn.execute(
            "UPDATE editorial_research_sources
             SET name = ?1, url = ?2, kind = ?3, category = ?4, scope = ?5, enabled = ?6
             WHERE id = ?7",
            params![
                next_name,
                next_url,
                next_kind,
                next_cat,
                next_scope,
                if next_enabled { 1 } else { 0 },
                id
            ],
        )
        .map_err(|e| e.to_string())?;

        Ok(Some(ResearchSource {
            id: curr.id,
            name: next_name.to_string(),
            url: next_url.to_string(),
            kind: next_kind.to_string(),
            category: next_cat.to_string(),
            scope: next_scope.to_string(),
            enabled: next_enabled,
            created_at: curr.created_at,
        }))
    }

    pub fn editorial_delete_research_source(&self, id: i32) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let affected = conn
            .execute(
                "DELETE FROM editorial_research_sources WHERE id = ?1",
                params![id],
            )
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    /// Promotes any `scheduled` articles whose `scheduled_at <= now` to `published`.
    pub fn editorial_publish_due_scheduled(&self) -> Result<Vec<DueScheduledArticle>, String> {
        let mut conn = self.open_conn().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, slug, title_tr, summary_tr, author_name, primary_category, body_tr, social_shared
                 FROM contents
                 WHERE status = 'scheduled'
                   AND scheduled_at IS NOT NULL
                   AND TRIM(scheduled_at) != ''
                   AND datetime(scheduled_at) <= datetime('now')",
            )
            .map_err(|e| e.to_string())?;

        let due: Vec<DueScheduledArticle> = stmt
            .query_map([], |r| {
                Ok(DueScheduledArticle {
                    id: r.get(0)?,
                    slug: r.get(1)?,
                    title: r.get(2)?,
                    spot: r.get(3)?,
                    author_name: r.get(4)?,
                    category_slug: r.get(5)?,
                    body_html: r.get(6)?,
                    social_shared: r.get::<_, i32>(7)? != 0,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        drop(stmt);

        if due.is_empty() {
            return Ok(Vec::new());
        }

        let pub_now = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
        let iso_now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for item in &due {
            tx.execute(
                "UPDATE contents SET status = 'published', published_at = ?1, updated_at = ?2 WHERE id = ?3",
                params![pub_now, iso_now, item.id],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        drop(conn);

        crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        self.refresh_l1_sections(&["contents"]);
        Ok(due)
    }
}
