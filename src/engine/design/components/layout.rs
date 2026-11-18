use super::primitives::{icon, link};
use super::super::config::Section;
use crate::engine::security_engine::escape_html as esc;

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
    format!(
        "<nav class=\"breadcrumb-line\" aria-label=\"İçerik yolu\"><a href=\"/\">{}</a><span>/</span><a href=\"/kategori/{}\">{}</a></nav>",
        esc(home_label),
        esc(category_slug),
        esc(category_label)
    )
}

pub fn heading(s: &Section) -> String {
    let cta = if !s.link.is_empty() {
        format!(
            "<a class=\"text-link\" href=\"{}\">{} {}</a>",
            link(&s.link),
            esc(&s.link_label),
            icon("arrow")
        )
    } else {
        String::new()
    };
    format!(
        "<div class=\"section-heading\"><div><div class=\"eyebrow\">{}</div><h2>{}</h2>{}</div>{}</div>",
        esc(&s.eyebrow),
        esc(&s.title),
        if s.description.is_empty() {
            String::new()
        } else {
            format!("<p>{}</p>", esc(&s.description))
        },
        cta
    )
}

