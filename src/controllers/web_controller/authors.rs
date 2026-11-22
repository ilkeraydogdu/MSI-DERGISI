use super::*;
use crate::engine::design::labels_helper::{is_en_content_ready, tr_text};

/// Derive a URL-safe slug from an author name for matching against the `/yazar/:slug` path segment.
/// Maps common Turkish characters to ASCII equivalents, lowercases, and strips non-alphanumeric chars.
pub fn author_slug_from_name(name: &str) -> String {
    let mapped: String = name
        .chars()
        .map(|c| match c {
            'ı' | 'İ' => 'i',
            'ş' | 'Ş' => 's',
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ü' | 'Ü' => 'u',
            'ö' | 'Ö' => 'o',
            'â' | 'Â' | 'á' | 'Á' | 'à' | 'À' => 'a',
            'é' | 'É' | 'è' | 'È' | 'ê' | 'Ê' => 'e',
            'î' | 'Î' | 'í' | 'Í' | 'ì' | 'Ì' => 'i',
            'ô' | 'Ô' | 'ó' | 'Ó' | 'ò' | 'Ò' => 'o',
            'û' | 'Û' | 'ú' | 'Ú' | 'ù' | 'Ù' => 'u',
            other => other,
        })
        .collect();
    mapped
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub async fn render_author_profile(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    use crate::engine::{
        design::components::article_card,
        security_engine::escape_html as esc,
    };

    let lang = detect_language(&params, &headers);
    let is_en = lang.eq_ignore_ascii_case("en");
    let tr = Some(&state.translations);
    let all_contents = state.db.get_all_contents();
    let is_staff = authenticated_reader(&state, &headers)
        .as_ref()
        .map(|u| crate::extractors::operator_user::is_staff_role(&state.db, &u.role))
        .unwrap_or(false);

    // Find all publicly visible contents whose author_name matches this slug and locale readiness
    let mut matched_articles: Vec<&ContentItem> = all_contents
        .iter()
        .filter(|c| {
            (is_staff || content_service::is_publicly_visible(c))
                && author_slug_from_name(&c.author_name) == slug
                && (is_staff
                    || if is_en {
                        is_en_content_ready(c)
                    } else {
                        !c.source_group.eq_ignore_ascii_case("msi-tdr")
                    })
        })
        .collect();

    if matched_articles.is_empty() {
        return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
    }

    // Sort by published_at descending
    matched_articles.sort_by(|a, b| b.published_at.cmp(&a.published_at));

    // Extract author info from first match
    let author_name = &matched_articles[0].author_name;
    let author_title = &matched_articles[0].author_title;
    let total_articles = matched_articles.len();

    let display_name = esc(author_name);
    let display_title = esc(author_title);

    let cards_html: String = matched_articles
        .iter()
        .enumerate()
        .map(|(idx, c)| article_card(c, &lang, idx < 3))
        .collect();

    let suffix = tr_text(tr, &lang, "author.articles_suffix", "içerik", "articles");
    let count_label = format!("{} {}", total_articles, suffix);
    let heading_label = tr_text(
        tr,
        &lang,
        "author.profile_eyebrow",
        "YAZAR PROFİLİ",
        "AUTHOR PROFILE",
    );

    let body = format!(
        r#"<section class="container section-space author-profile-page">
            <div class="category-master-grid">
                <div class="category-main-col">
                    <div class="msi-section-ribbon">{heading}</div>
                    <div class="page-heading author-header">
                        <h1>{name}</h1>
                        <p class="author-title">{title}</p>
                        <span class="author-count">{count}</span>
                    </div>
                    <div class="stories category-stories layout-grid">{cards}</div>
                </div>
            </div>
        </section>{related_strip}"#,
        heading = esc(&heading_label),
        name = display_name,
        title = display_title,
        count = esc(&count_label),
        cards = cards_html,
        related_strip = crate::engine::design::components::sections::render_related_strip(
            &all_contents,
            &crate::engine::design::components::sections::RelatedStripOptions {
                lang: &lang,
                is_current_sponsored: false,
                exclude_ids: &[],
                limit: 4,
                tr,
                source_context: "author",
            },
        ),
    );

    let title_suffix = tr_text(
        tr,
        &lang,
        "author.page_title_suffix",
        "Yazar Profili",
        "Author Profile",
    );
    let page_title = format!("{} — {}", author_name, title_suffix);

    Html(home::render_shared_shell_with_request(
        &state,
        &lang,
        &page_title,
        &body,
        Some(&format!("/yazar/{}", slug)),
        None,
        false,
        Some(&csp_nonce.0),
        Some(&headers),
        None,
    ))
    .into_response()
}

pub(crate) fn authenticated_reader(
    state: &AppState,
    headers: &HeaderMap,
) -> Option<crate::models::UserAccount> {
    crate::services::auth_service::session_user(state, headers)
}

pub async fn render_authors_directory(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_authors_directory_for_lang(&state, &params, &headers, &csp_nonce.0, None).await
}

pub async fn render_en_authors_directory(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_authors_directory_for_lang(&state, &params, &headers, &csp_nonce.0, Some("en")).await
}

async fn render_authors_directory_for_lang(
    state: &AppState,
    params: &HashMap<String, String>,
    headers: &HeaderMap,
    csp_nonce: &str,
    forced_lang: Option<&str>,
) -> Response {
    use crate::engine::{
        design::{
            components::sections::{render_related_strip, RelatedStripOptions},
            labels_helper::lang_href,
        },
        security_engine::escape_html as esc,
    };
    use std::collections::BTreeMap;

    let lang = forced_lang
        .map(str::to_string)
        .unwrap_or_else(|| detect_language(params, headers));
    let is_en = lang.eq_ignore_ascii_case("en");
    let tr = Some(&state.translations);
    let snap = CentralQueryEngine::snapshot(&state.db).data;

    let mut by_author: BTreeMap<String, (String, String, usize)> = BTreeMap::new();
    for c in snap.contents.iter().filter(|c| {
        content_service::is_publicly_visible(c)
            && !c.author_name.trim().is_empty()
            && (!is_en || is_en_content_ready(c))
            && (is_en || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
    }) {
        let name = c.author_name.trim().to_string();
        let slug = author_slug_from_name(&name);
        if slug.is_empty() {
            continue;
        }
        let entry = by_author
            .entry(slug)
            .or_insert_with(|| (name, c.author_title.trim().to_string(), 0));
        entry.2 += 1;
    }

    let mut authors: Vec<(String, String, String, usize)> = by_author
        .into_iter()
        .map(|(slug, (name, title, cnt))| (slug, name, title, cnt))
        .collect();
    authors.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| a.1.cmp(&b.1)));

    let eyebrow = tr_text(
        tr,
        &lang,
        "authors.directory_eyebrow",
        "EDİTÖRYAL KADRO VE KÖŞE YAZARLARI",
        "EDITORIAL BOARD & COLUMNISTS",
    );
    let title = tr_text(
        tr,
        &lang,
        "authors.directory_title",
        "Yazarlar ve Savunma Analistleri Dizini",
        "Authors & Defence Analysts Directory",
    );
    let desc = tr_text(
        tr,
        &lang,
        "authors.directory_desc",
        "MSI Dergisi yayın kurulu üyeleri, akademisyenler, emekli komutanlar ve savunma ekonomisi analistlerinin tüm dosyaları.",
        "Editorial board members, defence academics, retired commanders and industry analysts contributing to MSI Review.",
    );
    let suffix = tr_text(tr, &lang, "author.articles_suffix", "içerik", "articles");

    let cards: String = authors
        .into_iter()
        .map(|(slug, name, role_title, cnt)| {
            let href = lang_href(&format!("/yazar/{}", slug), &lang);
            let sub = if role_title.is_empty() {
                tr_text(tr, &lang, "authors.default_title", "Savunma Analisti", "Defence Analyst")
            } else {
                role_title
            };
            format!(
                r#"<a class="subdossier-card" href="{href}"><div class="subdossier-badge"><span class="subdossier-kicker">{count} {suffix}</span></div><div class="subdossier-copy"><h3>{name}</h3><p>{sub}</p><span>{cta} →</span></div></a>"#,
                href = esc(&href),
                count = cnt,
                suffix = esc(&suffix),
                name = esc(&name),
                sub = esc(&sub),
                cta = esc(&tr_text(tr, &lang, "authors.view_articles", "Yazıları incele", "View articles")),
            )
        })
        .collect();

    let related_html = render_related_strip(
        &snap.contents,
        &RelatedStripOptions {
            lang: &lang,
            is_current_sponsored: false,
            exclude_ids: &[],
            limit: 4,
            tr,
            source_context: "yazarlar",
        },
    );

    let body = format!(
        r#"<section class="container section-space" id="authors-directory"><div class="section-heading"><div><div class="eyebrow">{eyebrow}</div><h1>{title}</h1><p>{desc}</p></div></div><div class="subcategories-grid">{cards}</div></section>{related}"#,
        eyebrow = esc(&eyebrow),
        title = esc(&title),
        desc = esc(&desc),
        cards = cards,
        related = related_html,
    );

    let canonical = if forced_lang == Some("en") {
        "/en/authors"
    } else {
        "/yazarlar"
    };
    Html(home::render_shared_shell_with_request(
        state,
        &lang,
        &title,
        &body,
        Some(canonical),
        None,
        false,
        Some(csp_nonce),
        Some(headers),
        Some("yazarlar"),
    ))
    .into_response()
}

pub async fn render_history_hub(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_gelec_hub_for_lang(
        &state,
        &params,
        &headers,
        &csp_nonce.0,
        None,
        "savunma-sanayii-tarihi",
        "/savunma-sanayii-tarihi",
    )
    .await
}

pub async fn render_en_history_hub(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_gelec_hub_for_lang(
        &state,
        &params,
        &headers,
        &csp_nonce.0,
        Some("en"),
        "savunma-sanayii-tarihi",
        "/en/defence-industry-history",
    )
    .await
}

pub async fn render_student_clubs_hub(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_gelec_hub_for_lang(
        &state,
        &params,
        &headers,
        &csp_nonce.0,
        None,
        "ogrenci-kulupleri",
        "/ogrenci-kulupleri",
    )
    .await
}

pub async fn render_en_student_clubs_hub(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_gelec_hub_for_lang(
        &state,
        &params,
        &headers,
        &csp_nonce.0,
        Some("en"),
        "ogrenci-kulupleri",
        "/en/student-clubs",
    )
    .await
}

async fn render_gelec_hub_for_lang(
    state: &AppState,
    params: &HashMap<String, String>,
    headers: &HeaderMap,
    csp_nonce: &str,
    forced_lang: Option<&str>,
    cat_slug: &str,
    canonical: &str,
) -> Response {
    use crate::engine::{
        design::components::{
            article_card, localized,
            sections::{
                dossier_toc_from_contents, render_dossier_toc, render_related_strip,
                RelatedStripOptions,
            },
        },
        security_engine::escape_html as esc,
    };

    let lang = forced_lang
        .map(str::to_string)
        .unwrap_or_else(|| detect_language(params, headers));
    let is_en = lang.eq_ignore_ascii_case("en");
    let tr = Some(&state.translations);
    let snap = CentralQueryEngine::snapshot(&state.db).data;
    let cat_opt = state.db.get_category_by_slug(cat_slug);

    let (title, desc, eyebrow) = match cat_slug {
        "savunma-sanayii-tarihi" => (
            cat_opt
                .as_ref()
                .map(|c| localized(&lang, &c.name_tr, &c.name_en).to_string())
                .unwrap_or_else(|| {
                    tr_text(
                        tr,
                        &lang,
                        "history.title",
                        "Savunma Sanayii Tarihi",
                        "Defence Industry History",
                    )
                }),
            cat_opt
                .as_ref()
                .map(|c| localized(&lang, &c.description_tr, &c.description_en).to_string())
                .unwrap_or_else(|| {
                    tr_text(
                        tr,
                        &lang,
                        "history.desc",
                        "Türk ve dünya savunma sanayii kronolojisi ve dokümantasyon arşivi.",
                        "Chronology and archival documentation of Turkish and global defence history.",
                    )
                }),
            tr_text(
                tr,
                &lang,
                "history.eyebrow",
                "KRONOLOJİK SAVUNMA ARŞİVİ",
                "CHRONOLOGICAL DEFENCE ARCHIVE",
            ),
        ),
        _ => (
            cat_opt
                .as_ref()
                .map(|c| localized(&lang, &c.name_tr, &c.name_en).to_string())
                .unwrap_or_else(|| {
                    tr_text(
                        tr,
                        &lang,
                        "clubs.title",
                        "Öğrenci Kulüpleri",
                        "University Defence Clubs",
                    )
                }),
            cat_opt
                .as_ref()
                .map(|c| localized(&lang, &c.description_tr, &c.description_en).to_string())
                .unwrap_or_else(|| {
                    tr_text(
                        tr,
                        &lang,
                        "clubs.desc",
                        "Üniversite savunma teknolojileri ve havacılık toplulukları ekosistemi.",
                        "University defence technologies and aerospace societies ecosystem.",
                    )
                }),
            tr_text(
                tr,
                &lang,
                "clubs.eyebrow",
                "AKADEMİK YETENEK VE KULÜP AĞI",
                "ACADEMIC TALENT & CLUBS NETWORK",
            ),
        ),
    };

    let mut matched: Vec<ContentItem> = snap
        .contents
        .iter()
        .filter(|c| {
            content_service::is_publicly_visible(c)
                && !c.is_sponsored
                && (!is_en || is_en_content_ready(c))
                && (is_en || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
                && (c.content_type.eq_ignore_ascii_case(cat_slug)
                    || c.placements.iter().any(|p| p.eq_ignore_ascii_case(cat_slug)))
        })
        .cloned()
        .collect();
    matched.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));

    let toc_entries = dossier_toc_from_contents(&matched, &lang);
    let toc_html = if toc_entries.is_empty() {
        String::new()
    } else {
        render_dossier_toc(
            "Arşiv İçindekiler",
            "Archive Table of Contents",
            &toc_entries,
            &lang,
            tr,
        )
    };

    let cards_html = if matched.is_empty() {
        let empty_msg = tr_text(
            tr,
            &lang,
            "hub.empty_archive",
            "Bu bölümdeki arşiv kayıtları editöryal yayına hazırlanıyor.",
            "Archival records for this section are being curated by the editorial board.",
        );
        format!("<div class=\"empty-state\">{}</div>", esc(&empty_msg))
    } else {
        format!(
            "<div class=\"stories layout-grid\">{}</div>",
            matched
                .iter()
                .enumerate()
                .map(|(i, c)| article_card(c, &lang, i < 3))
                .collect::<String>()
        )
    };

    let related_html = render_related_strip(
        &snap.contents,
        &RelatedStripOptions {
            lang: &lang,
            is_current_sponsored: false,
            exclude_ids: &[],
            limit: 4,
            tr,
            source_context: cat_slug,
        },
    );

    let body = format!(
        r#"<section class="container section-space" id="hub-{slug}"><div class="section-heading"><div><div class="eyebrow">{eyebrow}</div><h1>{title}</h1><p>{desc}</p></div></div>{toc}{cards}</section>{related}"#,
        slug = esc(cat_slug),
        eyebrow = esc(&eyebrow),
        title = esc(&title),
        desc = esc(&desc),
        toc = toc_html,
        cards = cards_html,
        related = related_html,
    );

    Html(home::render_shared_shell_with_request(
        state,
        &lang,
        &title,
        &body,
        Some(canonical),
        None,
        false,
        Some(csp_nonce),
        Some(headers),
        Some(cat_slug),
    ))
    .into_response()
}
