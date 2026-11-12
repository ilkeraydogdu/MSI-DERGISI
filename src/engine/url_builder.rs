/// Central URL Builder for MSI Dergisi
///
/// All canonical path construction MUST go through this module.
/// No controller, engine, or service should ever use `format!("/icerik/{}", slug)` directly.
/// This ensures a single source of truth for URL structure across the entire application.

// ── Public Content URLs ──────────────────────────────────────────────

/// Canonical URL for an article detail page: `/icerik/{slug}`
pub fn article_url(slug: &str) -> String {
    format!("/icerik/{}", slug)
}

/// Canonical URL for an article detail page with language query: `/icerik/{slug}?lang={lang}`
pub fn article_url_lang(slug: &str, lang: &str) -> String {
    format!("/icerik/{}?lang={}", slug, lang)
}

/// Canonical URL for a category listing page: `/kategori/{slug}`
pub fn category_url(slug: &str) -> String {
    format!("/kategori/{}", slug)
}

/// Canonical URL for a paginated category listing: `/kategori/{slug}?page={page}`
pub fn category_url_page(slug: &str, page: usize) -> String {
    if page <= 1 {
        category_url(slug)
    } else {
        format!("/kategori/{}?page={}", slug, page)
    }
}

/// Canonical URL for a category listing with language query: `/kategori/{slug}?lang={lang}`
pub fn category_url_lang(slug: &str, lang: &str) -> String {
    format!("/kategori/{}?lang={}", slug, lang)
}

/// Canonical URL for a corporate page: `/kurumsal/{slug}`
/// The slug stored in DB may be prefixed with "kurumsal-"; this function strips that prefix.
pub fn corporate_url(db_slug: &str) -> String {
    let clean = db_slug.trim_start_matches("kurumsal-");
    format!("/kurumsal/{}", clean)
}

/// Canonical URL for a designed/generic page: `/sayfa/{slug}`
pub fn page_url(slug: &str) -> String {
    format!("/sayfa/{}", slug)
}

/// Resolve the canonical URL for any designed page based on its slug convention.
/// - Slugs starting with "kurumsal-" → `/kurumsal/{stripped}`
/// - Special slugs ("e-dergi", "abonelik-ve-satis") → `/{slug}`
/// - Everything else → `/sayfa/{slug}`
pub fn designed_page_url(slug: &str) -> String {
    if slug.starts_with("kurumsal-") {
        corporate_url(slug)
    } else if ["e-dergi", "abonelik-ve-satis"].contains(&slug) {
        format!("/{}", slug)
    } else {
        page_url(slug)
    }
}

// ── Search URL ───────────────────────────────────────────────────────

/// Build a search facet URL with all filter parameters.
/// Values are URL-encoded; output uses `&amp;` for safe HTML embedding.
pub fn search_facet_url(
    q: &str,
    category: &str,
    ssb_tag: &str,
    issue: Option<i32>,
    source_group: &str,
    sort: &str,
) -> String {
    let esc = |s: &str| -> String { urlencoding::encode(s).to_string() };
    format!(
        "/arama?q={}&amp;category={}&amp;ssb_tag={}&amp;issue={}&amp;source_group={}&amp;sort={}",
        esc(q),
        esc(category),
        esc(ssb_tag),
        issue.map(|i| i.to_string()).unwrap_or_default(),
        esc(source_group),
        esc(sort)
    )
}

// ── Legacy Redirect URLs ─────────────────────────────────────────────

/// Legacy Turkish haber redirect source: `/haberler/{slug}`
pub fn legacy_haber_tr_url(slug: &str) -> String {
    format!("/haberler/{}", slug)
}

/// Legacy English haber redirect source: `/en/haberler/{slug}`
pub fn legacy_haber_en_url(slug: &str) -> String {
    format!("/en/haberler/{}", slug)
}

/// Legacy Turkish kategori redirect source: `/kategoriler/{slug}`
pub fn legacy_kategori_tr_url(slug: &str) -> String {
    format!("/kategoriler/{}", slug)
}

/// Legacy English kategori redirect source: `/en/kategoriler/{slug}`
pub fn legacy_kategori_en_url(slug: &str) -> String {
    format!("/en/kategoriler/{}", slug)
}

// ── Admin / Operator URLs ────────────────────────────────────────────

/// Admin panel section URL: `/admin/{section}` (empty section → `/admin`)
pub fn admin_url(section: &str) -> String {
    if section.is_empty() {
        "/admin".to_string()
    } else {
        format!("/admin/{}", section)
    }
}

/// Admin API endpoint URL: `/api/v1/admin/{endpoint}`
pub fn admin_api_url(endpoint: &str) -> String {
    format!("/api/v1/admin/{}", endpoint)
}

// ── Static / Well-Known URLs ─────────────────────────────────────────

/// Account page URL with optional next-redirect parameter
pub fn account_url(next: Option<&str>) -> String {
    match next {
        Some(path) => format!("/hesabim?next={}", urlencoding::encode(path)),
        None => "/hesabim".to_string(),
    }
}

// ── Route Path Constants ─────────────────────────────────────────────
// These constants define the canonical route patterns used in main.rs
// route registrations. They serve as documentation and can be referenced
// by middleware for path matching.

pub const HOME: &str = "/";
pub const HOME_INDEX: &str = "/index.html";
pub const EN_HOME: &str = "/en";
pub const SEARCH: &str = "/arama";
pub const ACCOUNT: &str = "/hesabim";
pub const SITEMAP: &str = "/sitemap.xml";
pub const SITEMAP_NEWS: &str = "/sitemap-news.xml";
pub const SITEMAP_IMAGES: &str = "/sitemap-images.xml";
pub const ROBOTS: &str = "/robots.txt";
pub const RSS: &str = "/rss.xml";
pub const JSON_FEED: &str = "/feed.json";
pub const LLMS: &str = "/llms.txt";
pub const LLMS_FULL: &str = "/llms-full.txt";
pub const AI_PLUGIN: &str = "/.well-known/ai-plugin.json";
pub const OPENSEARCH: &str = "/opensearch.xml";
pub const MANIFEST: &str = "/manifest.webmanifest";
pub const FAVICON: &str = "/favicon.ico";
pub const ADMIN_PANEL: &str = "/admin";
pub const ADMIN_DESIGN_EDITOR: &str = "/admin/tasarim";
pub const ADMIN_DESIGN_PREVIEW: &str = "/admin/tasarim/onizleme";
pub const ADMIN_LEGACY: &str = "/yonetim";
pub const ARTICLE_PATTERN: &str = "/icerik/:slug";
pub const CATEGORY_PATTERN: &str = "/kategori/:slug";
pub const DESIGNED_PAGE_PATTERN: &str = "/sayfa/:slug";
pub const CORPORATE_PATTERN: &str = "/kurumsal/:slug";
pub const EDERGI: &str = "/e-dergi";
pub const SUBSCRIPTIONS: &str = "/abonelik-ve-satis";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_url() {
        assert_eq!(article_url("test-slug"), "/icerik/test-slug");
    }

    #[test]
    fn test_article_url_lang() {
        assert_eq!(article_url_lang("test-slug", "en"), "/icerik/test-slug?lang=en");
    }

    #[test]
    fn test_category_url() {
        assert_eq!(category_url("savunma"), "/kategori/savunma");
    }

    #[test]
    fn test_category_url_page_first() {
        assert_eq!(category_url_page("savunma", 1_usize), "/kategori/savunma");
    }

    #[test]
    fn test_category_url_page_paginated() {
        assert_eq!(category_url_page("savunma", 3_usize), "/kategori/savunma?page=3");
    }

    #[test]
    fn test_corporate_url_strips_prefix() {
        assert_eq!(corporate_url("kurumsal-kunye"), "/kurumsal/kunye");
    }

    #[test]
    fn test_corporate_url_no_prefix() {
        assert_eq!(corporate_url("kunye"), "/kurumsal/kunye");
    }

    #[test]
    fn test_designed_page_url_kurumsal() {
        assert_eq!(designed_page_url("kurumsal-hakkimizda"), "/kurumsal/hakkimizda");
    }

    #[test]
    fn test_designed_page_url_special() {
        assert_eq!(designed_page_url("e-dergi"), "/e-dergi");
        assert_eq!(designed_page_url("abonelik-ve-satis"), "/abonelik-ve-satis");
    }

    #[test]
    fn test_designed_page_url_generic() {
        assert_eq!(designed_page_url("iletisim"), "/sayfa/iletisim");
    }

    #[test]
    fn test_admin_url_empty() {
        assert_eq!(admin_url(""), "/admin");
    }

    #[test]
    fn test_admin_url_section() {
        assert_eq!(admin_url("tasarim"), "/admin/tasarim");
    }

    #[test]
    fn test_account_url_with_next() {
        assert_eq!(account_url(Some("/admin")), "/hesabim?next=%2Fadmin");
    }

    #[test]
    fn test_account_url_without_next() {
        assert_eq!(account_url(None), "/hesabim");
    }
}