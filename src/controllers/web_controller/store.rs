use super::*;
use crate::engine::design::{
    components::sections::{
        render_newsletter_trigger_band, render_related_strip, RelatedStripOptions,
    },
    labels_helper::{localize_section_copy, tr_text},
};
use crate::engine::security_engine::escape_html as esc;

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
    let pub_type_filter = match params
        .get("publication_type")
        .map(String::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"))
    {
        Some(raw_pt) => {
            let Some(normalized) =
                crate::controllers::api_controller::admin_issues::normalize_publication_type(raw_pt)
            else {
                return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
            };
            Some(normalized)
        }
        None => None,
    };
    let filtered_issues: Vec<crate::models::MagazineIssue> = if let Some(pt) = pub_type_filter {
        snap.issues
            .iter()
            .filter(|i| i.publication_type == pt)
            .cloned()
            .collect()
    } else {
        snap.issues.clone()
    };
    let selected_issue = if let Some(raw_iss) =
        params.get("issue").map(String::as_str).filter(|s| !s.trim().is_empty())
    {
        let Ok(num) = raw_iss.trim().parse::<i32>() else {
            return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
        };
        if !filtered_issues.iter().any(|i| i.issue_number == num) {
            return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
        }
        Some(num)
    } else if pub_type_filter.is_some() {
        filtered_issues
            .iter()
            .max_by_key(|i| i.issue_number)
            .map(|i| i.issue_number)
    } else {
        settings
            .get("current_featured_issue")
            .and_then(|s| s.parse().ok())
    };
    let data = PageData {
        lang: &lang,
        contents: &snap.contents,
        issues: &filtered_issues,
        tags: &snap.ssb_tags,
        ads: &snap.ads,
        plans: &snap.subscription_plans,
        products: &snap.commerce_products,
        featured_issue: selected_issue,
    };
    let mut body = render_page(page, &data);
    body.push_str(&render_related_strip(
        &snap.contents,
        &RelatedStripOptions {
            lang: &lang,
            is_current_sponsored: false,
            exclude_ids: &[],
            limit: 4,
            tr: Some(&state.translations),
            source_context: "e-dergi",
        },
    ));
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings);
    let seo_title = localize_section_copy(&lang, &page.title);
    let seo_desc = localize_section_copy(&lang, &page.description);
    let seo = CentralSeoEngine::build_edergi_seo_meta(
        &state.config.public_url,
        seo_title,
        seo_desc,
        &filtered_issues,
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
    use crate::engine::design::components::{localized, render_page, PageData};
    let lang = detect_language(&params, &headers);
    let settings = state.db.get_setting_map(&lang);
    let config = crate::engine::design::config::load(&settings);
    let Some(page) = config.page("abonelik-ve-satis") else {
        return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
    };
    let snap = CentralQueryEngine::snapshot(&state.db).data;
    let data = PageData {
        lang: &lang,
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

    let tr = Some(&state.translations);
    let cart_eyebrow = tr_text(
        tr,
        &lang,
        "commerce.cart_catalog_eyebrow",
        "DİJİTAL VE BASILI YAYIN SEPETİ",
        "DIGITAL & PRINT PUBLICATIONS CART",
    );
    let cart_heading = tr_text(
        tr,
        &lang,
        "commerce.cart_catalog_title",
        "Çoklu Sipariş & Anında Dijital İndirme Kataloğu",
        "Multi-Item Order & Instant Digital Download Catalogue",
    );
    let open_cart_lbl = tr_text(
        tr,
        &lang,
        "commerce.open_cart",
        "Sepeti Aç",
        "Open Cart",
    );
    let add_to_cart_lbl = tr_text(
        tr,
        &lang,
        "commerce.add_to_cart",
        "Sepete Ekle",
        "Add to Cart",
    );

    let product_cards: String = snap
        .commerce_products
        .iter()
        .filter(|p| p.is_available)
        .map(|p| {
            let title = localized(&lang, &p.title_tr, &p.title_en);
            let desc = localized(&lang, &p.description_tr, &p.description_en);
            format!(
                r#"<article class="plan-card"><div class="eyebrow">{ptype}</div><h3>{title}</h3><p>{desc}</p><div class="plan-price">₺{price:.0}</div><button type="button" class="button primary" data-add-to-cart data-item-type="product" data-item-code="{sku}" data-item-title="{title}" data-item-price="{price:.2}">{btn}</button></article>"#,
                ptype = esc(&p.product_type),
                title = esc(title),
                desc = esc(desc),
                price = p.price_try,
                sku = esc(&p.sku),
                btn = esc(&add_to_cart_lbl),
            )
        })
        .collect();

    if !product_cards.is_empty() {
        body.push_str(&format!(
            r#"<section class="container section-space" id="commerce-cart-catalog"><div class="section-heading"><div><div class="eyebrow">{eyebrow}</div><h2>{heading}</h2></div><button type="button" class="button secondary" data-cart-toggle>{open_cart}</button></div><div class="plans-grid">{cards}</div></section>"#,
            eyebrow = esc(&cart_eyebrow),
            heading = esc(&cart_heading),
            open_cart = esc(&open_cart_lbl),
            cards = product_cards,
        ));
    }

    body.push_str(&render_newsletter_trigger_band(&lang, tr));
    body.push_str(&render_related_strip(
        &snap.contents,
        &RelatedStripOptions {
            lang: &lang,
            is_current_sponsored: false,
            exclude_ids: &[],
            limit: 4,
            tr,
            source_context: "abonelik-ve-satis",
        },
    ));

    let path = crate::engine::url_builder::designed_page_url(&page.slug);
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings);
    let seo_title = localize_section_copy(&lang, &page.title);
    let seo_desc = localize_section_copy(&lang, &page.description);
    let seo = seo_router_engine::build_seo_meta(
        &state.config.public_url,
        &path,
        seo_title,
        seo_desc,
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
        &page.mode,
        &seo,
        &body,
        Some(&csp_nonce.0),
        Some(&headers),
        None,
        Some(&page.slug),
    ))
    .into_response()
}

pub async fn render_corporate_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_corporate_page_for_lang(state, slug, params, headers, &csp_nonce.0, None).await
}

pub async fn render_en_corporate_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(mut params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    params.insert("lang".to_string(), "en".to_string());
    render_corporate_page_for_lang(state, slug, params, headers, &csp_nonce.0, Some("en")).await
}

async fn render_corporate_page_for_lang(
    state: AppState,
    slug: String,
    params: HashMap<String, String>,
    headers: HeaderMap,
    csp_nonce: &str,
    forced_lang: Option<&str>,
) -> Response {
    let lang = forced_lang
        .map(str::to_string)
        .unwrap_or_else(|| detect_language(&params, &headers));
    let settings = state.db.get_setting_map(&lang);
    let config = crate::engine::design::config::load(&settings);
    let normalized = match slug.as_str() {
        "imprint" => "kunye",
        "about" | "about-us" => "hakkimizda",
        "who-we-are" => "biz-kimiz",
        "our-journey" => "yolculugumuz",
        "solution-partners" => "cozum-ortaklarimiz",
        "privacy-policy" | "kvkk-ve-gizlilik" | "aydinlatma-metni" => "kvkk",
        "data-request-form" | "kvkk-application-form" => "kvkk-basvuru-formu",
        "contact" => "iletisim",
        "gizlilik" | "privacy" => "gizlilik-politikasi",
        "cerezler" | "cookie-policy" => "cerez-politikasi",
        "kullanim-sartlari" | "terms" | "terms-of-use" => "kullanim-kosullari",
        "mesafeli-satis" | "on-bilgilendirme" | "distance-sales" | "distance-sales-agreement" => {
            "mesafeli-satis-sozlesmesi"
        }
        "iade" | "iptal-iade" | "cancellation-refund" | "refund-policy" => {
            "iptal-ve-iade-kosullari"
        }
        "iys" | "ticari-ileti" | "commercial-messaging" => "ticari-ileti-aydinlatma",
        "copyright" | "copyright-ethics" => "telif-ve-etik",
        "tekzip" | "duzeltme" | "correction-reply" => "duzeltme-ve-cevap-hakki",
        "reklam" | "medya-kiti" | "advertising" | "advertising-policy" => "reklam-ve-sponsorluk",
        other => other,
    };
    match config.page(&format!("kurumsal-{}", normalized)) {
        Some(page) => pages::render_configured_page_with_headers(
            &state,
            &lang,
            page,
            &settings,
            Some(csp_nonce),
            Some(&headers),
        ),
        None => render_not_found_with_nonce(&state, &headers, Some(csp_nonce)).await,
    }
}
