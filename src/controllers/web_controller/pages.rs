use super::*;
use crate::engine::design::{
    components::{
        render_page,
        sections::{
            dossier_toc_from_contents, render_dossier_toc, render_events_hub,
            render_newsletter_trigger_band, render_related_strip, EventsHubData, RelatedStripOptions,
        },
        PageData,
    },
    labels_helper::{is_en_content_ready, lang_href, localize_section_copy, tr_text},
};
use crate::engine::security_engine::escape_html as esc;

pub async fn render_design_tokens_css(State(state): State<AppState>) -> Response {
    let settings = state.db.get_setting_map("tr");
    let css = crate::engine::design::config::load(&settings).css();
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=60"),
        ],
        css,
    )
        .into_response()
}

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
    let mut body = render_page(page, &data);
    body.push_str(&render_related_strip(
        &snap.contents,
        &RelatedStripOptions {
            lang,
            is_current_sponsored: false,
            exclude_ids: &[],
            limit: 4,
            tr: Some(&state.translations),
            source_context: &page.slug,
        },
    ));
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
      <img src="/images/logo.svg" alt="{brand}" width="240" height="89" decoding="async">
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
</section>"##,
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

pub async fn render_events_hub_page(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_events_hub_for_lang(&state, &params, &headers, &csp_nonce.0, None).await
}

pub async fn render_en_events_hub_page(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_events_hub_for_lang(&state, &params, &headers, &csp_nonce.0, Some("en")).await
}

async fn render_events_hub_for_lang(
    state: &AppState,
    params: &HashMap<String, String>,
    headers: &HeaderMap,
    csp_nonce: &str,
    forced_lang: Option<&str>,
) -> Response {
    let lang = forced_lang
        .map(str::to_string)
        .unwrap_or_else(|| detect_language(params, headers));
    let is_en = lang.eq_ignore_ascii_case("en");
    let snap = CentralQueryEngine::snapshot(&state.db).data;

    let mut body = render_events_hub(&EventsHubData {
        lang: &lang,
        categories: &snap.categories,
        issues: &snap.issues,
        contents: &snap.contents,
        tr: Some(&state.translations),
    });

    if !body.contains("id=\"dossier-toc\"") {
        let fallback_items: Vec<ContentItem> = snap
            .contents
            .iter()
            .filter(|c| {
                content_service::is_publicly_visible(c)
                    && !c.is_sponsored
                    && (!is_en || is_en_content_ready(c))
                    && (is_en || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
            })
            .take(5)
            .cloned()
            .collect();
        let toc_entries = dossier_toc_from_contents(&fallback_items, &lang);
        if !toc_entries.is_empty() {
            let toc_html = render_dossier_toc(
                "Etkinlik ve Dosya Dizini",
                "Event & Dossier Table of Contents",
                &toc_entries,
                &lang,
                Some(&state.translations),
            );
            body.push_str(&format!("<div class=\"container section-space\">{}</div>", toc_html));
        }
    }

    body.push_str(&render_newsletter_trigger_band(
        &lang,
        Some(&state.translations),
    ));
    body.push_str(&render_related_strip(
        &snap.contents,
        &RelatedStripOptions {
            lang: &lang,
            is_current_sponsored: false,
            exclude_ids: &[],
            limit: 4,
            tr: Some(&state.translations),
            source_context: "etkinlikler",
        },
    ));

    let page_title = tr_text(
        Some(&state.translations),
        &lang,
        "events.page_title",
        "Etkinlikler ve Fuar Özel Dosyaları",
        "Defence Events & Post-Event Dossiers",
    );
    let canonical = if forced_lang == Some("en") {
        "/en/events"
    } else {
        "/etkinlikler"
    };

    Html(home::render_shared_shell_with_request(
        state,
        &lang,
        &page_title,
        &body,
        Some(canonical),
        None,
        false,
        Some(csp_nonce),
        Some(headers),
        Some("etkinlikler"),
    ))
    .into_response()
}
