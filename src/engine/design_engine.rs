pub use crate::engine::design::account::render as render_reader_account_portal;
pub use crate::engine::design::shell::render as render_master_shell;
pub use crate::engine::design::shell::render_with_nonce as render_master_shell_with_nonce;

/// Wrapper that passes the translation cache into the search renderer.
#[allow(clippy::too_many_arguments)]
pub fn render_search_and_filter_page(
    lang: &str,
    facets: &crate::models::SearchFacetResponse,
    q: &str,
    category: &str,
    tag: &str,
    issue: Option<i32>,
    source: &str,
    sort: &str,
    tr: &crate::engine::translation_engine::TranslationCache,
) -> String {
    crate::engine::design::search::render(lang, facets, q, category, tag, issue, source, sort, tr)
}
use crate::models::{
    AdCampaign, CategoryNode, CommerceProduct, ContentItem, MagazineIssue, SsbInterestTag,
    SubscriptionPlan,
};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn render_dual_homepage(
    lang: &str,
    settings: &HashMap<String, String>,
    contents: &[ContentItem],
    _categories: &[CategoryNode],
    ssb_tags: &[SsbInterestTag],
    issues: &[MagazineIssue],
    ads: &[AdCampaign],
    plans: &[SubscriptionPlan],
    products: &[CommerceProduct],
) -> String {
    use crate::engine::design::{
        components::{render_page, PageData},
        config,
    };
    let config = config::load(settings);
    let data = PageData {
        lang,
        contents,
        issues,
        tags: ssb_tags,
        ads,
        plans,
        products,
        featured_issue: settings
            .get("current_featured_issue")
            .and_then(|n| n.parse().ok()),
    };
    format!(
        "<div id=\"view-magazine\">{}</div><div id=\"view-news\">{}</div>",
        render_page(config.page("magazine").unwrap(), &data),
        render_page(config.page("news").unwrap(), &data)
    )
}
