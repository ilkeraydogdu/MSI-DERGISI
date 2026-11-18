use super::*;
use crate::engine::design::labels_helper::localize_section_copy;

pub async fn render_edergi_kiosk(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    use crate::engine::design::components::{render_page, PageData};
    let lang = detect_language(&params, &headers);
    let settings = state.db.get_setting_map(&lang);
    let config = crate::engine::design::config::load(&settings);
    let Some(page) = config.page("e-dergi") else {
        return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
    };
    let snap = CentralQueryEngine::snapshot(&state.db).data;
    let selected_issue = if let Some(raw_iss) =
        params.get("issue").map(String::as_str).filter(|s| !s.trim().is_empty())
    {
        let Ok(num) = raw_iss.trim().parse::<i32>() else {
            return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
        };
        if !snap.issues.iter().any(|i| i.issue_number == num) {
            return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
        }
        Some(num)
    } else {
        settings
            .get("current_featured_issue")
            .and_then(|s| s.parse().ok())
    };
    let data = PageData {
        lang: &lang,
        contents: &snap.contents,
        issues: &snap.issues,
        tags: &snap.ssb_tags,
        ads: &snap.ads,
        plans: &snap.subscription_plans,
        products: &snap.commerce_products,
        featured_issue: selected_issue,
    };
    let body = render_page(page, &data);
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings);
    let seo_title = localize_section_copy(&lang, &page.title);
    let seo_desc = localize_section_copy(&lang, &page.description);
    let seo = CentralSeoEngine::build_edergi_seo_meta(
        &state.config.public_url,
        seo_title,
        seo_desc,
        &snap.issues,
        &lang,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );
    Html(home::render_shell_with_context(
        &state,
        &lang,
        &page.mode,
        &seo,
        &body,
        Some(&csp_nonce.0),
        Some(&headers),
        None,
        Some("e-dergi"),
    ))
    .into_response()
}

pub async fn render_subscriptions_and_store(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = detect_language(&params, &headers);
    let settings = state.db.get_setting_map(&lang);
    let config = crate::engine::design::config::load(&settings);
    match config.page("abonelik-ve-satis") {
        Some(page) => pages::render_configured_page_with_headers(
            &state,
            &lang,
            page,
            &settings,
            Some(&csp_nonce.0),
            Some(&headers),
        ),
        None => render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await,
    }
}

pub async fn render_corporate_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = detect_language(&params, &headers);
    let settings = state.db.get_setting_map(&lang);
    let config = crate::engine::design::config::load(&settings);
    let normalized = match slug.as_str() {
        "imprint" => "kunye",
        "about" | "about-us" => "hakkimizda",
        "our-journey" => "yolculugumuz",
        "privacy-policy" | "kvkk-ve-gizlilik" | "aydinlatma-metni" => "kvkk",
        "biz-kimiz" => "hakkimizda",
        "contact" => "iletisim",
        "gizlilik" => "gizlilik-politikasi",
        "cerezler" | "cookie-policy" => "cerez-politikasi",
        "kullanim-sartlari" => "kullanim-kosullari",
        "mesafeli-satis" | "on-bilgilendirme" => "mesafeli-satis-sozlesmesi",
        "iade" | "iptal-iade" => "iptal-ve-iade-kosullari",
        "iys" | "ticari-ileti" => "ticari-ileti-aydinlatma",
        "tekzip" | "duzeltme" => "duzeltme-ve-cevap-hakki",
        "reklam" | "medya-kiti" => "reklam-ve-sponsorluk",
        other => other,
    };
    match config.page(&format!("kurumsal-{}", normalized)) {
        Some(page) => pages::render_configured_page_with_headers(
            &state,
            &lang,
            page,
            &settings,
            Some(&csp_nonce.0),
            Some(&headers),
        ),
        None => render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await,
    }
}
