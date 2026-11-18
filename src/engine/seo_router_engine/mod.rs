pub mod brand;
pub mod feeds;
pub mod jsonld;
pub mod llms;
pub mod meta;
pub mod robots;
pub mod sitemaps;
pub mod util;
pub mod webapp;

pub use brand::SeoBrandConfig;
use crate::models::{CategoryNode, ContentItem, MagazineIssue, SeoMetaBundle};

pub struct CentralSeoEngine;

#[allow(clippy::too_many_arguments)]
pub fn build_seo_meta(
    base_url: &str,
    path: &str,
    title: &str,
    description: &str,
    og_image: &str,
    og_type: &str,
    lang: &str,
    article: Option<&ContentItem>,
    brand: &SeoBrandConfig,
    primary_domain: &str,
    news_domain: &str,
) -> SeoMetaBundle {
    CentralSeoEngine::build_seo_meta(
        base_url,
        path,
        title,
        description,
        og_image,
        og_type,
        lang,
        article,
        brand,
        primary_domain,
        news_domain,
    )
}

pub fn render_seo_head_tags(meta: &SeoMetaBundle, article: Option<&ContentItem>, brand: &SeoBrandConfig) -> String {
    CentralSeoEngine::render_seo_head_tags(meta, article, brand)
}

pub fn generate_sitemap_xml(
    base_url: &str,
    categories: &[CategoryNode],
    contents: &[ContentItem],
    issues: &[MagazineIssue],
    brand: &SeoBrandConfig,
) -> String {
    CentralSeoEngine::generate_sitemap_xml(base_url, categories, contents, issues, brand)
}

pub fn generate_robots_txt(base_url: &str) -> String {
    CentralSeoEngine::generate_robots_txt(base_url)
}

pub fn generate_rss_xml(base_url: &str, contents: &[ContentItem], lang: &str, brand: &SeoBrandConfig) -> String {
    CentralSeoEngine::generate_rss_xml(base_url, contents, lang, brand)
}

#[cfg(test)]
mod tests;
