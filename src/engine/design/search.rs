use super::components::article_card;
use super::labels_helper::tr_text;
use crate::{
    engine::{security_engine::escape_html as esc, translation_engine::TranslationCache},
    models::SearchFacetResponse,
};

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
    tr: &TranslationCache,
) -> String {
    let url = |cat: &str, ssb: &str, iss: Option<i32>| {
        crate::engine::url_builder::search_facet_url_lang(lang, q, cat, ssb, iss, source, sort)
    };
    let search_path = if lang.eq_ignore_ascii_case("en") {
        "/en/search"
    } else {
        "/arama"
    };
    let tr_opt = Some(tr);

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
    let issue_prefix = tr_text(tr_opt, lang, "search.issue_prefix", "Sayı", "Issue");
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
                esc(&issue_prefix),
                esc(&f.code),
                f.count
            )
        })
        .collect();
    let results = if facets.items.is_empty() {
        format!(
            "<div class=\"empty-state\"><h2>{}</h2><p>{}</p><a class=\"button secondary\" href=\"{}\">{}</a></div>",
            esc(&tr_text(
                tr_opt,
                lang,
                "search.no_results_title",
                "Sonuç bulunamadı.",
                "No results found."
            )),
            esc(&tr_text(
                tr_opt,
                lang,
                "search.no_results_desc",
                "Farklı bir kelime deneyin veya seçtiğiniz filtreleri temizleyin.",
                "Try a different keyword or clear your filters."
            )),
            search_path,
            esc(&tr_text(
                tr_opt,
                lang,
                "search.clear_filters",
                "Filtreleri temizle",
                "Clear filters"
            )),
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
        (
            "latest",
            tr_text(tr_opt, lang, "search.sort.latest", "En yeni", "Latest"),
        ),
        (
            "popular",
            tr_text(tr_opt, lang, "search.sort.popular", "En çok okunan", "Most read"),
        ),
        (
            "true_read",
            tr_text(
                tr_opt,
                lang,
                "search.sort.true_read",
                "Okuma ilgisine göre",
                "By reading engagement",
            ),
        ),
    ]
    .iter()
    .map(|(value, label)| {
        format!(
            "<option value=\"{}\" {}>{}</option>",
            value,
            if sort == *value { "selected" } else { "" },
            esc(label)
        )
    })
    .collect();
    let source_options: String = [
        (
            "",
            tr_text(tr_opt, lang, "search.source.all", "Tüm yayınlar", "All sources"),
        ),
        (
            "dergi-icerigi",
            tr_text(
                tr_opt,
                lang,
                "search.source.dergi",
                "Dergi içerikleri",
                "Magazine content",
            ),
        ),
        (
            "gunluk-haber",
            tr_text(
                tr_opt,
                lang,
                "search.source.gunluk",
                "Günlük haberler",
                "Daily news",
            ),
        ),
        (
            "sadece-online",
            tr_text(
                tr_opt,
                lang,
                "search.source.online",
                "Online içerikler",
                "Online content",
            ),
        ),
    ]
    .iter()
    .map(|(value, label)| {
        format!(
            "<option value=\"{}\" {}>{}</option>",
            value,
            if source == *value { "selected" } else { "" },
            esc(label)
        )
    })
    .collect();
    let results_summary = tr_text(
        tr_opt,
        lang,
        "search.results_summary",
        "{count} sonuç · {showing} içerik gösteriliyor",
        "{count} results · showing {showing} items",
    )
    .replace("{count}", &format!("<strong>{}</strong>", facets.total_matches))
    .replace("{showing}", &facets.items.len().to_string());

    format!(
        r#"<section class="container section-space"><div class="page-heading"><div class="eyebrow">{eyebrow}</div><h1>{heading}</h1><p>{description}</p></div><form class="search-form" action="{search_path}" method="get"><label class="search-query">{query_label}<input name="q" type="search" value="{q}" placeholder="{placeholder}"></label><input type="hidden" name="category" value="{category}"><input type="hidden" name="ssb_tag" value="{tag}"><input type="hidden" name="issue" value="{issue}"><label>{source_lbl}<select name="source_group">{source_options}</select></label><label>{sort_lbl}<select name="sort">{sort_options}</select></label><button class="button primary" type="submit">{submit}</button></form><div class="search-summary"><span>{results_summary}</span><a class="text-link" href="{search_path}">{clear}</a></div><div class="search-layout"><aside class="search-filters"><details open><summary>{filter_cats}</summary><a class="facet-link" href="{all_cat_url}">{all_cats}</a>{categories}</details><details open><summary>{filter_tags_lbl}</summary><a class="facet-link" href="{all_tag_url}">{all_topics}</a>{tags}</details><details><summary>{filter_issues_lbl}</summary><a class="facet-link" href="{all_issue_url}">{all_issues}</a>{issues}</details></aside><div>{results}</div></div></section>"#,
        eyebrow = esc(&tr_text(
            tr_opt,
            lang,
            "search.eyebrow",
            "MSI ARŞİVİNİ KEŞFEDİN",
            "EXPLORE THE MSI ARCHIVE"
        )),
        heading = esc(&tr_text(
            tr_opt,
            lang,
            "search.heading",
            "Bir konunun peşinden gidin.",
            "Follow a topic through."
        )),
        description = esc(&tr_text(
            tr_opt,
            lang,
            "search.description",
            "Haberler, analizler, söyleşiler ve dergi arşivi. Hepsi aynı yerde.",
            "News, analyses, interviews and magazine archive. All in one place."
        )),
        search_path = search_path,
        query_label = esc(&tr_text(
            tr_opt,
            lang,
            "search.query_label",
            "Arama kelimesi",
            "Search term"
        )),
        q = esc(q),
        placeholder = esc(&tr_text(
            tr_opt,
            lang,
            "search.placeholder",
            "Konu, kurum veya teknoloji…",
            "Topic, institution or technology…"
        )),
        category = esc(category),
        tag = esc(tag),
        issue = issue.map(|i| i.to_string()).unwrap_or_default(),
        source_lbl = esc(&tr_text(tr_opt, lang, "search.source_label", "Yayın", "Source")),
        sort_lbl = esc(&tr_text(tr_opt, lang, "search.sort_label", "Sıralama", "Sort")),
        submit = esc(&tr_text(tr_opt, lang, "search.submit", "Ara →", "Search →")),
        results_summary = results_summary,
        clear = esc(&tr_text(
            tr_opt,
            lang,
            "search.clear_filters",
            "Filtreleri temizle",
            "Clear filters"
        )),
        filter_cats = esc(&tr_text(
            tr_opt,
            lang,
            "search.filter_categories",
            "Kategoriler",
            "Categories"
        )),
        all_cat_url = url("", tag, issue),
        all_cats = esc(&tr_text(
            tr_opt,
            lang,
            "search.all_categories",
            "Tüm kategoriler",
            "All categories"
        )),
        filter_tags_lbl = esc(&tr_text(
            tr_opt,
            lang,
            "search.filter_tags",
            "İlgi alanları",
            "Interest areas"
        )),
        all_tag_url = url(category, "", issue),
        all_topics = esc(&tr_text(
            tr_opt,
            lang,
            "search.all_topics",
            "Tüm konular",
            "All topics"
        )),
        filter_issues_lbl = esc(&tr_text(
            tr_opt,
            lang,
            "search.filter_issues",
            "Dergi sayıları",
            "Magazine issues"
        )),
        all_issue_url = url(category, tag, None),
        all_issues = esc(&tr_text(
            tr_opt,
            lang,
            "search.all_issues",
            "Tüm sayılar",
            "All issues"
        )),
        results = results,
    )
}
