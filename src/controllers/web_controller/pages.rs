use super::*;
use crate::engine::design::{
    components::{render_page, PageData},
    labels_helper::{lang_href, localize_section_copy, tr_text},
};
use crate::engine::security_engine::escape_html as esc;

pub async fn render_designed_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = detect_language(&params, &headers);
    let settings = state.db.get_setting_map(&lang);
    let config = crate::engine::design::config::load(&settings);
    let Some(page) = config.page(&slug) else {
        return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
    };
    render_configured_page_with_headers(
        &state,
        &lang,
        page,
        &settings,
        Some(&csp_nonce.0),
        Some(&headers),
    )
}

#[allow(dead_code)]
pub(crate) fn render_configured_page(
    state: &AppState,
    lang: &str,
    page: &crate::engine::design::config::Page,
    settings: &HashMap<String, String>,
    nonce: Option<&str>,
) -> Response {
    render_configured_page_with_headers(state, lang, page, settings, nonce, None)
}

pub(crate) fn render_configured_page_with_headers(
    state: &AppState,
    lang: &str,
    page: &crate::engine::design::config::Page,
    settings: &HashMap<String, String>,
    nonce: Option<&str>,
    headers: Option<&HeaderMap>,
) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db).data;
    let data = PageData {
        lang,
        contents: &snap.contents,
        issues: &snap.issues,
        tags: &snap.ssb_tags,
        ads: &snap.ads,
        plans: &snap.subscription_plans,
        products: &snap.commerce_products,
        featured_issue: settings
            .get("current_featured_issue")
            .and_then(|s| s.parse().ok()),
    };
    let body = render_page(page, &data);
    let path = crate::engine::url_builder::designed_page_url(&page.slug);
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(settings);
    let seo_title = localize_section_copy(lang, &page.title);
    let seo_desc = localize_section_copy(lang, &page.description);
    let seo = seo_router_engine::build_seo_meta(
        &state.config.public_url,
        &path,
        seo_title,
        seo_desc,
        media_service::DEFAULT_OG_IMAGE,
        "website",
        lang,
        None,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );
    Html(home::render_shell_with_context(
        state,
        lang,
        &page.mode,
        &seo,
        &body,
        nonce,
        headers,
        None,
        Some(&page.slug),
    ))
    .into_response()
}

pub async fn render_coming_soon(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = detect_language(&params, &headers);
    let settings_map = state.db.get_setting_map(&lang);
    let tr = Some(&state.translations);

    let title = tr_text(tr, &lang, "coming_soon.title", "Yakında", "Coming Soon");
    let message = tr_text(
        tr,
        &lang,
        "coming_soon.message",
        "Bu sayfa yakında aktif olacak. Okuyucularımız için özel bir deneyim üzerinde çalışıyoruz.",
        "This page will be active soon. We are working on something special for our readers.",
    );
    let cta_label = tr_text(
        tr,
        &lang,
        "coming_soon.home",
        "Ana Sayfaya Dön",
        "Back to Homepage",
    );
    let contact_label = tr_text(
        tr,
        &lang,
        "coming_soon.contact",
        "İletişime Geçin",
        "Contact Us",
    );
    let feedback_label = tr_text(
        tr,
        &lang,
        "coming_soon.feedback",
        "Geri Bildirim Gönder",
        "Send Feedback",
    );

    let brand_name = settings_map
        .get("brand_title")
        .map(|s| s.as_str())
        .unwrap_or("MSI Dergisi");

    let contact_email = settings_map
        .get("contact_email")
        .map(|s| s.as_str())
        .unwrap_or("info@savunmahaber.com");

    let home_href = lang_href("/", &lang);
    let contact_href = lang_href("/kurumsal/iletisim", &lang);

    let body_html = format!(
        r##"<section class="coming-soon-page">
  <div class="coming-soon-container">
    <div class="coming-soon-logo">
      <img src="/images/logo.svg" alt="{brand}">
    </div>
    <h1 class="coming-soon-title">{title}</h1>
    <p class="coming-soon-message">{message}</p>
    <div class="coming-soon-actions">
      <a href="{home_href}" class="button primary">{cta_label}</a>
      <a href="{contact_href}" class="button secondary">{contact_label}</a>
      <a href="mailto:{email}?subject={feedback_subject}" class="text-link">{feedback_label} &rarr;</a>
    </div>
    <div class="coming-soon-footer">
      <p>&copy; {year} {brand}. ISSN 1306-0309</p>
    </div>
  </div>
</section>
<style>
.coming-soon-page {{
  min-height: 80vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem 1rem;
}}
.coming-soon-container {{
  text-align: center;
  max-width: 640px;
  margin: 0 auto;
}}
.coming-soon-logo img {{
  height: 72px;
  width: auto;
  margin-bottom: 2rem;
}}
.coming-soon-title {{
  font-size: clamp(2rem, 5vw, 3.5rem);
  font-weight: 800;
  letter-spacing: -0.03em;
  line-height: 1.1;
  margin-bottom: 1.25rem;
  color: var(--msi-text-primary, #1a1a1a);
}}
.coming-soon-message {{
  font-size: 1.125rem;
  line-height: 1.6;
  color: var(--msi-text-muted, #555);
  margin-bottom: 2.5rem;
}}
.coming-soon-actions {{
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  justify-content: center;
  align-items: center;
  margin-bottom: 3rem;
}}
.coming-soon-actions .text-link {{
  font-size: 0.95rem;
}}
.coming-soon-footer {{
  font-size: 0.8rem;
  color: var(--msi-text-muted, #999);
  border-top: 1px solid var(--msi-border, #e5e5e5);
  padding-top: 1.5rem;
}}
</style>"##,
        brand = esc(brand_name),
        title = esc(&title),
        message = esc(&message),
        home_href = esc(&home_href),
        cta_label = esc(&cta_label),
        contact_href = esc(&contact_href),
        contact_label = esc(&contact_label),
        email = esc(contact_email),
        feedback_subject = urlencoding::encode(if lang == "en" { "Feedback" } else { "Geri Bildirim" }),
        feedback_label = esc(&feedback_label),
        year = chrono::Utc::now().year(),
    );

    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings_map);
    let seo = seo_router_engine::build_seo_meta(
        &state.config.public_url,
        "/coming-soon",
        &format!("{} - {}", title, brand_name),
        &message,
        media_service::DEFAULT_OG_IMAGE,
        "website",
        &lang,
        None,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );

    Html(home::render_shell_with_context(
        &state,
        &lang,
        "magazine",
        &seo,
        &body_html,
        Some(&csp_nonce.0),
        Some(&headers),
        None,
        None,
    ))
    .into_response()
}
