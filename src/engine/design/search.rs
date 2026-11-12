use super::components::article_card;
use crate::{engine::security_engine::escape_html as esc, models::SearchFacetResponse};

#[allow(clippy::too_many_arguments)]
pub fn render(
    lang: &str,
    facets: &SearchFacetResponse,
    q: &str,
    category: &str,
    tag: &str,
    issue: Option<i32>,
    source: &str,
    sort: &str,
    tr: &crate::engine::translation_engine::TranslationCache,
) -> String {
    let url = |cat: &str, ssb: &str, iss: Option<i32>| {
        crate::engine::url_builder::search_facet_url(q, cat, ssb, iss, source, sort)
    };
    let categories: String = facets
        .category_facets
        .iter()
        .map(|f| {
            format!(
                "<a class=\"facet-link {}\" href=\"{}\"><span>{}</span><span>{}</span></a>",
                if f.code == category { "selected" } else { "" },
                url(&f.code, tag, issue),
                esc(if lang == "en" {
                    &f.label_en
                } else {
                    &f.label_tr
                }),
                f.count
            )
        })
        .collect();
    let tags: String = facets
        .ssb_facets
        .iter()
        .map(|f| {
            format!(
                "<a class=\"facet-link {}\" href=\"{}\"><span>{}</span><span>{}</span></a>",
                if f.code == tag { "selected" } else { "" },
                url(category, &f.code, issue),
                esc(if lang == "en" {
                    &f.label_en
                } else {
                    &f.label_tr
                }),
                f.count
            )
        })
        .collect();
    let issues: String = facets
        .issue_facets
        .iter()
        .map(|f| {
            format!(
                "<a class=\"facet-link {}\" href=\"{}\"><span>{} {}</span><span>{}</span></a>",
                if Some(f.code.as_str()) == issue.map(|i| i.to_string()).as_deref() {
                    "selected"
                } else {
                    ""
                },
                url(category, tag, f.code.parse().ok()),
                tr.t(lang, "search.issue_prefix"),
                esc(&f.code),
                f.count
            )
        })
        .collect();
    let results = if facets.items.is_empty() {
        format!(
            "<div class=\"empty-state\"><h2>{}</h2><p>{}</p><a class=\"button secondary\" href=\"/arama\">{}</a></div>",
            tr.t(lang, "search.no_results_title"),
            tr.t(lang, "search.no_results_desc"),
            tr.t(lang, "search.clear_filters")
        )
    } else {
        format!(
            "<div class=\"stories search-stories\">{}</div>",
            facets
                .items
                .iter()
                .map(|c| article_card(c, lang, false))
                .collect::<String>()
        )
    };
    let sort_options: String = [
        ("latest", tr.t(lang, "search.sort.latest")),
        ("popular", tr.t(lang, "search.sort.popular")),
        ("true_read", tr.t(lang, "search.sort.true_read")),
    ]
    .iter()
    .map(|(value, label)| {
        format!(
            "<option value=\"{}\" {}>{}</option>",
            value,
            if sort == *value { "selected" } else { "" },
            label
        )
    })
    .collect();
    let source_options: String = [
        ("", tr.t(lang, "search.source.all")),
        ("dergi-icerigi", tr.t(lang, "search.source.dergi")),
        ("gunluk-haber", tr.t(lang, "search.source.gunluk")),
        ("sadece-online", tr.t(lang, "search.source.online")),
    ]
    .iter()
    .map(|(value, label)| {
        format!(
            "<option value=\"{}\" {}>{}</option>",
            value,
            if source == *value { "selected" } else { "" },
            label
        )
    })
    .collect();
    format!(
        r#"<section class="container section-space"><div class="page-heading"><div class="eyebrow">{eyebrow}</div><h1>{heading}</h1><p>{description}</p></div><form class="search-form" action="/arama" method="get"><label class="search-query">{query_label}<input name="q" type="search" value="{q}" placeholder="{placeholder}"></label><input type="hidden" name="category" value="{category}"><input type="hidden" name="ssb_tag" value="{tag}"><input type="hidden" name="issue" value="{issue}"><label>{source_lbl}<select name="source_group">{source_options}</select></label><label>{sort_lbl}<select name="sort">{sort_options}</select></label><button class="button primary" type="submit">{submit}</button></form><div class="search-summary"><span><strong>{total}</strong> {results_text}</span><a class="text-link" href="/arama">{clear}</a></div><div class="search-layout"><aside class="search-filters"><details open><summary>{filter_cats}</summary><a class="facet-link" href="{all_cat_url}">{all_cats}</a>{categories}</details><details open><summary>{filter_tags_lbl}</summary><a class="facet-link" href="{all_tag_url}">{all_topics}</a>{tags}</details><details><summary>{filter_issues_lbl}</summary><a class="facet-link" href="{all_issue_url}">{all_issues}</a>{issues}</details></aside><div>{results}</div></div></section>"#,
        eyebrow = tr.t(lang, "search.eyebrow"),
        heading = tr.t(lang, "search.heading"),
        description = tr.t(lang, "search.description"),
        query_label = tr.t(lang, "search.query_label"),
        q = esc(q),
        placeholder = tr.t(lang, "search.placeholder"),
        category = esc(category),
        tag = esc(tag),
        issue = issue.map(|i| i.to_string()).unwrap_or_default(),
        source_lbl = tr.t(lang, "search.source_label"),
        sort_lbl = tr.t(lang, "search.sort_label"),
        submit = tr.t(lang, "search.submit"),
        total = facets.total_matches,
        results_text = format!("sonuç · {} içerik gösteriliyor", facets.items.len()),
        clear = tr.t(lang, "search.clear_filters"),
        filter_cats = tr.t(lang, "search.filter_categories"),
        all_cat_url = url("", tag, issue),
        all_cats = tr.t(lang, "search.all_categories"),
        filter_tags_lbl = tr.t(lang, "search.filter_tags"),
        all_tag_url = url(category, "", issue),
        all_topics = tr.t(lang, "search.all_topics"),
        filter_issues_lbl = tr.t(lang, "search.filter_issues"),
        all_issue_url = url(category, tag, None),
        all_issues = tr.t(lang, "search.all_issues"),
        results = results,
    )
}