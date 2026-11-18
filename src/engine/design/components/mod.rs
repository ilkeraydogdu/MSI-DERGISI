pub mod cards;
pub mod filters;
pub mod labels;
pub mod layout;
pub mod media;
pub mod primitives;
pub mod sections;
pub mod select;

use super::config::{Page, SectionKind};
use crate::engine::security_engine::escape_html as esc;
use crate::models::*;

pub struct PageData<'a> {
    pub lang: &'a str,
    pub contents: &'a [ContentItem],
    pub issues: &'a [MagazineIssue],
    pub tags: &'a [SsbInterestTag],
    pub ads: &'a [AdCampaign],
    pub plans: &'a [SubscriptionPlan],
    pub products: &'a [CommerceProduct],
    pub featured_issue: Option<i32>,
}

pub use super::labels_helper::is_en_content_ready;
pub use cards::{
    advertisement, article_card, editorial_sidebar, floating_popup_ad, issue_card,
    notification_item,
};
pub use filters::enrich_issue_spreads;
pub use labels::{CANONICAL_SSB_TAG_CODES, category_display_label, localized, ssb_tag_display_label};
pub use layout::{breadcrumb, breadcrumb_localized, empty_state, page_heading};
pub use media::image_dimensions;
pub use primitives::{asset, icon, link};

pub fn render_page(page: &Page, data: &PageData<'_>) -> String {
    let mut seen_ids = std::collections::HashSet::new();
    let sections: String = page
        .sections
        .iter()
        .filter(|s| s.enabled)
        .enumerate()
        .map(|(i, s)| sections::render_section(s, data, i, &mut seen_ids))
        .collect();
    if page
        .sections
        .iter()
        .filter(|s| s.enabled)
        .any(|s| matches!(s.kind, SectionKind::MagazineHero | SectionKind::NewsHero))
    {
        sections
    } else {
        let brand_lbl = if data.lang == "en" { "MSI REVIEW" } else { "MSI DERGİSİ" };
        format!(
            "<header class=\"container page-heading section-space\"><div class=\"eyebrow\">{}</div><h1>{}</h1><p>{}</p></header>{}",
            brand_lbl,
            esc(super::labels_helper::localize_section_copy(data.lang, &page.title)),
            esc(super::labels_helper::localize_section_copy(data.lang, &page.description)),
            sections
        )
    }
}

#[cfg(test)]
mod tests;
