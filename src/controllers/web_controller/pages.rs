use super::*;

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
    render_configured_page(&state, &lang, page, &settings, Some(&csp_nonce.0))
}

pub(crate) fn render_configured_page(
    state: &AppState,
    lang: &str,
    page: &crate::engine::design::config::Page,
    settings: &HashMap<String, String>,
    nonce: Option<&str>,
) -> Response {
    use crate::engine::design::components::{render_page, PageData};
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
    let seo = seo_router_engine::build_seo_meta(
        &state.config.public_url,
        &path,
        &page.title,
        &page.description,
        media_service::DEFAULT_OG_IMAGE,
        "website",
        lang,
        None,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );
    Html(design_engine::render_master_shell_with_nonce(
        lang,
        &page.mode,
        settings,
        &seo,
        &snap.categories,
        &snap.ssb_tags,
        &snap.issues,
        &snap.contents,
        &snap.notifications,
        &body,
        0,
        nonce,
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

    let (title, message, cta_label, contact_label, feedback_label) = if lang == "en" {
        (
            "Coming Soon",
            "This page will be active soon. We are working on something special for our readers.",
            "Back to Homepage",
            "Contact Us",
            "Send Feedback",
        )
    } else {
        (
            "Yakında",
            "Bu sayfa yakında aktif olacak. Okuyucularımız için özel bir deneyim üzerinde çalışıyoruz.",
            "Ana Sayfaya Dön",
            "İletişime Geçin",
            "Geri Bildirim Gönder",
        )
    };

    let brand_name = settings_map
        .get("brand_title")
        .map(|s| s.as_str())
        .unwrap_or("MSI Dergisi");

    let contact_email = settings_map
        .get("contact_email")
        .map(|s| s.as_str())
        .unwrap_or("info@savunmahaber.com");

    let body_html = format!(
        r##"<section class="coming-soon-page">
  <div class="coming-soon-container">
    <div class="coming-soon-logo">
      <img src="/images/logo.svg" alt="{brand}" onerror="this.style.display='none';this.nextElementSibling.style.display='block'">
      <span style="display:none;font-size:2rem;font-weight:800;letter-spacing:-0.03em;color:#b91c1c;">{brand}</span>
    </div>
    <h1 class="coming-soon-title">{title}</h1>
    <p class="coming-soon-message">{message}</p>
    <div class="coming-soon-actions">
      <a href="/" class="button primary">{cta_label}</a>
      <a href="/kurumsal/iletisim" class="button secondary">{contact_label}</a>
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
  color: var(--text-main, #1a1a1a);
}}
.coming-soon-message {{
  font-size: 1.125rem;
  line-height: 1.6;
  color: var(--text-secondary, #555);
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
  color: var(--text-muted, #999);
  border-top: 1px solid var(--border-subtle, #e5e5e5);
  padding-top: 1.5rem;
}}
</style>"##,
        brand = brand_name,
        title = title,
        message = message,
        cta_label = cta_label,
        contact_label = contact_label,
        email = contact_email,
        feedback_subject = urlencoding::encode(if lang == "en" { "Feedback" } else { "Geri Bildirim" }),
        feedback_label = feedback_label,
        year = chrono::Utc::now().year(),
    );

    let settings_map = state.db.get_setting_map(&lang);
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings_map);
    let seo = seo_router_engine::build_seo_meta(
        &state.config.public_url,
        "/coming-soon",
        &format!("{} - {}", title, brand_name),
        message,
        media_service::DEFAULT_OG_IMAGE,
        "website",
        &lang,
        None,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );

    Html(render_shell_with_seo(&state, &lang, "magazine", &seo, &body_html, Some(&csp_nonce.0))).into_response()
}

