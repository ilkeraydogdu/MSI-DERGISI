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
        </section>"#,
        heading = esc(&heading_label),
        name = display_name,
        title = display_title,
        count = esc(&count_label),
        cards = cards_html,
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
