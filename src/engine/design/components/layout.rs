use super::super::config::Section;
use super::super::labels_helper::{lang_href, localize_section_copy, tr_text};
use super::primitives::{icon, link};
use crate::engine::security_engine::escape_html as esc;
use crate::engine::translation_engine::TranslationCache;

/// Centralized empty-state placeholder.
pub fn empty_state(message: &str) -> String {
    format!("<div class=\"empty-state\">{}</div>", message)
}

/// Centralized page heading with eyebrow + title + optional description.
pub fn page_heading(eyebrow: &str, title: &str, description: &str) -> String {
    let desc = if description.is_empty() {
        String::new()
    } else {
        format!("<p>{}</p>", esc(description))
    };
    format!(
        "<div class=\"page-heading\"><div class=\"eyebrow\">{}</div><h1>{}</h1>{}</div>",
        esc(eyebrow),
        esc(title),
        desc
    )
}

/// Centralized breadcrumb navigation for article/reader pages.
pub fn breadcrumb(home_label: &str, category_slug: &str, category_label: &str) -> String {
    breadcrumb_localized("tr", home_label, category_slug, category_label, None)
}

/// Locale-aware breadcrumb navigation preserving `?lang=en` / `/en` paths and `aria-label`.
pub fn breadcrumb_localized(
    lang: &str,
    home_label: &str,
    category_slug: &str,
    category_label: &str,
    tr: Option<&TranslationCache>,
) -> String {
    let aria_lbl = tr_text(tr, lang, "reader.breadcrumb_aria", "İçerik yolu", "Breadcrumb");
    let home_url = lang_href("/", lang);
    let cat_url = lang_href(&format!("/kategori/{}", category_slug), lang);
    format!(
        "<nav class=\"breadcrumb-line\" aria-label=\"{}\"><a href=\"{}\">{}</a><span>/</span><a href=\"{}\">{}</a></nav>",
        esc(&aria_lbl),
        esc(&home_url),
        esc(home_label),
        esc(&cat_url),
        esc(category_label)
    )
}

pub fn heading(s: &Section) -> String {
    heading_localized(s, "tr")
}

pub fn heading_localized(s: &Section, lang: &str) -> String {
    let eyebrow = localize_section_copy(lang, &s.eyebrow);
    let title = localize_section_copy(lang, &s.title);
    let description = localize_section_copy(lang, &s.description);
    let link_label = localize_section_copy(lang, &s.link_label);
    let cta = if !s.link.is_empty() {
        let resolved = lang_href(&s.link, lang);
        format!(
            "<a class=\"text-link\" href=\"{}\">{} {}</a>",
            link(&resolved),
            esc(link_label),
            icon("arrow")
        )
    } else {
        String::new()
    };
    format!(
        "<div class=\"section-heading\"><div><div class=\"eyebrow\">{}</div><h2>{}</h2>{}</div>{}</div>",
        esc(eyebrow),
        esc(title),
        if description.is_empty() {
            String::new()
        } else {
            format!("<p>{}</p>", esc(description))
        },
        cta
    )
}

