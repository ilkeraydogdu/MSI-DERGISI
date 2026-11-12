use axum::{
    extract::{Extension, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use chrono::Datelike;
use std::collections::HashMap;
use std::time::Instant;

use crate::controllers::{admin_view, AppState};
use crate::engine::{
    design_engine,
    query_engine::CentralQueryEngine,
    search_engine,
    security_engine::CspNonce,
    seo_router_engine::{self, CentralSeoEngine},
};
use crate::models::{ContentItem, SearchFilterQuery, SeoMetaBundle};
use crate::services::{ad_service, content_service, media_service};

fn detect_language(params: &HashMap<String, String>, headers: &HeaderMap) -> String {
    if let Some(l) = params.get("lang") {
        if l.eq_ignore_ascii_case("en") {
            return "en".to_string();
        }
        if l.eq_ignore_ascii_case("tr") {
            return "tr".to_string();
        }
    }
    if let Some(cookie) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        if cookie.contains("msi_lang=en") {
            return "en".to_string();
        }
        if cookie.contains("msi_lang=tr") {
            return "tr".to_string();
        }
    }
    "tr".to_string()
}

/// Constructs a typed response with the given content-type and body, eliminating
/// the repeated `(StatusCode::OK, [(header::CONTENT_TYPE, ct)], body).into_response()`
/// boilerplate used by every feed / XML / manifest endpoint.
fn typed_response(content_type: &'static str, body: impl Into<axum::body::Body>) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        body.into(),
    )
        .into_response()
}

fn detect_domain_mode(params: &HashMap<String, String>, headers: &HeaderMap) -> String {
    if let Some(m) = params.get("mode") {
        if m == "magazine" || m == "dergi" {
            return "magazine".into();
        }
        if m == "news" || m == "haber" {
            return "news".to_string();
        }
    }
    if let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) {
        if host.to_lowercase().contains("savunmahaber") {
            return "news".to_string();
        }
    }
    "magazine".to_string()
}

pub async fn render_home(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let t0 = Instant::now();
    let lang = detect_language(&params, &headers);
    let domain_mode = detect_domain_mode(&params, &headers);

    let snap = CentralQueryEngine::snapshot(&state.db);
    let settings_map = state.db.get_setting_map(&lang);

    let design = crate::engine::design::config::load(&settings_map);
    let page = design.page(&domain_mode).expect("validated home page");
    let brand_title = if lang == "en" {
        settings_map
            .get("brand_title")
            .map(String::as_str)
            .unwrap_or(&page.title)
    } else {
        &page.title
    };
    let brand_desc = &page.description;
    let featured = settings_map
        .get("current_featured_issue")
        .and_then(|v| v.parse::<i32>().ok());
    let cover = snap
        .data
        .issues
        .iter()
        .find(|i| Some(i.issue_number) == featured)
        .or_else(|| snap.data.issues.iter().max_by_key(|i| i.issue_number))
        .map(|i| i.cover_image.as_str())
        .map(|s| media_service::resolve_asset_url(s))
        .unwrap_or_else(|| media_service::resolve_asset_url(media_service::DEFAULT_OG_IMAGE));

    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings_map);
    let seo = seo_router_engine::build_seo_meta(
        &state.config.public_url,
        "/",
        brand_title,
        brand_desc,
        &cover,
        "website",
        &lang,
        None,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );

    let body_html = design_engine::render_dual_homepage(
        &lang,
        &settings_map,
        &snap.data.contents,
        &snap.data.categories,
        &snap.data.ssb_tags,
        &snap.data.issues,
        &snap.data.ads,
        &snap.data.subscription_plans,
        &snap.data.commerce_products,
    );

    let exec_us = t0.elapsed().as_micros();
    let full_html = design_engine::render_master_shell_with_nonce(
        &lang,
        &domain_mode,
        &settings_map,
        &seo,
        &snap.data.categories,
        &snap.data.ssb_tags,
        &snap.data.issues,
        &snap.data.contents,
        &snap.data.notifications,
        &body_html,
        exec_us,
        Some(&csp_nonce.0),
    );

    Html(full_html).into_response()
}

pub async fn render_admin(State(state): State<AppState>) -> Html<String> {
    let design = crate::engine::design::config::load(&state.db.get_setting_map("tr"));
    let palette = |p: &crate::engine::design::config::Palette| {
        format!("--bg-deep:{};--bg-panel:{};--bg-card:{};--bg-input:{};--border-subtle:{};--border-bright:{};--msi-crimson:{};--msi-crimson-glow:transparent;--text-main:{};--text-secondary:{};--text-muted:{};--radius:{}px;",p.background,p.surface,p.surface,p.subtle,p.border,p.accent,p.accent,p.text,p.muted,p.muted,design.theme.radius)
    };
    let tokens = format!(
        "<style>:root{{{}}}body.admin-light-mode{{{}}}</style>",
        palette(&design.theme.dark),
        palette(&design.theme.light)
    );
    Html(admin_view::render_admin_dashboard().replace("</head>", &format!("{}</head>", tokens)))
}

fn render_shell_with_seo(
    state: &AppState,
    lang: &str,
    mode: &str,
    seo: &SeoMetaBundle,
    main_body_html: &str,
    nonce: Option<&str>,
) -> String {
    let t0 = Instant::now();
    let snap = CentralQueryEngine::snapshot(&state.db);
    let settings_map = state.db.get_setting_map(lang);
    let exec_us = t0.elapsed().as_micros();
    design_engine::render_master_shell_with_nonce(
        lang,
        mode,
        &settings_map,
        seo,
        &snap.data.categories,
        &snap.data.ssb_tags,
        &snap.data.issues,
        &snap.data.contents,
        &snap.data.notifications,
        main_body_html,
        exec_us,
        nonce,
    )
}

fn render_shared_shell(
    state: &AppState,
    lang: &str,
    page_title: &str,
    main_body_html: &str,
    canonical_path: Option<&str>,
    article_opt: Option<&ContentItem>,
    noindex: bool,
    nonce: Option<&str>,
) -> String {
    let path = canonical_path
        .map(|s| {
            if s.starts_with('/') {
                s.to_string()
            } else {
                format!("/{}", s)
            }
        })
        .unwrap_or_else(|| "/".to_string());

    let desc = article_opt
        .map(|c| {
            if lang == "en" && !c.summary_en.is_empty() {
                c.summary_en.as_str()
            } else {
                c.summary_tr.as_str()
            }
        })
        .unwrap_or(
            "MSI Dergisi & MSI TDR — Savunma, Havacılık ve Güvenlik Teknolojileri Referans Platformu (ISSN 1306-0309)",
        );
    let og_img = article_opt
        .map(|c| media_service::resolve_asset_url(&c.cover_image))
        .unwrap_or_else(|| media_service::resolve_asset_url(media_service::DEFAULT_OG_IMAGE));
    let og_type = if article_opt.is_some() {
        "article"
    } else {
        "website"
    };

    let settings_map = state.db.get_setting_map(lang);
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings_map);
    let mut seo = seo_router_engine::build_seo_meta(
        &state.config.public_url,
        &path,
        page_title,
        desc,
        &og_img,
        og_type,
        lang,
        article_opt,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );
    if noindex {
        seo.robots = "noindex, follow".to_string();
    }

    render_shell_with_seo(state, lang, "magazine", &seo, main_body_html, nonce)
}

pub async fn render_article_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = detect_language(&params, &headers);
    let Some(item) = state.db.get_content_by_slug(&slug) else {
        for cand in [
            format!("/icerik/{}", slug),
            format!("/haberler/{}", slug),
            format!("/en/haberler/{}", slug),
        ] {
            if let Some(rule) = state.db.find_seo_redirect(&cand) {
                return Redirect::permanent(&rule.new_path).into_response();
            }
        }
        return render_not_found(State(state), headers).await;
    };
    let user = authenticated_reader(&state, &headers);
    let purchased = user
        .as_ref()
        .map(|u| state.db.get_user_purchased_codes(&u.email))
        .unwrap_or_default();
    let (body, locked, reason) =
        content_service::apply_paywall_view(&item, user.as_ref(), &purchased, &lang);
    state.db.increment_content_view(&slug, !locked);
    let contents = state.db.get_all_contents();
    let related = content_service::select_related_articles(
        &item,
        &contents,
        &state.db.get_recommendation_rules(),
        user.as_ref(),
        5,
    );
    let ads = ad_service::select_ads_for_context(
        &state.db.get_all_ads(),
        Some(&item.content_type),
        Some(&item),
        user.as_ref(),
    );
    let html = crate::engine::design::reader::article(
        &item, &body, locked, &reason, &related, &ads, &lang,
    );
    let title = crate::engine::design::components::localized(&lang, &item.title_tr, &item.title_en);
    let canonical_path = crate::engine::url_builder::article_url(&item.slug);
    Html(render_shared_shell(
        &state,
        &lang,
        title,
        &html,
        Some(&canonical_path),
        Some(&item),
        false,
        Some(&csp_nonce.0),
    ))
    .into_response()
}

pub async fn render_category_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    use crate::engine::{
        content_query_engine::CentralContentQueryEngine,
        design::components::{article_card, editorial_sidebar, localized},
        search_engine::CentralSearchEngine,
        security_engine::escape_html as esc,
    };
    let lang = detect_language(&params, &headers);
    let Some(category) = state.db.get_category_by_slug(&slug) else {
        return render_not_found(State(state), headers).await;
    };
    let ssb_filter = params.get("ssb").filter(|v| !v.is_empty()).cloned();
    let all_categories = state.db.get_all_categories();
    let child_parent = if slug == "ozel-dosyalar" {
        "ozel-dosya"
    } else {
        slug.as_str()
    };
    let child_categories: Vec<_> = all_categories
        .iter()
        .filter(|c| c.parent_slug.as_deref() == Some(child_parent) && c.group_type != "kurumsal")
        .collect();

    // Unified query via CentralContentQueryEngine — replaces scattered filter/pagination logic.
    let mut criteria = CentralContentQueryEngine::from_params(&params);
    criteria.category = Some(slug.clone());
    criteria.per_page = Some(24);
    let query_result = CentralContentQueryEngine::execute(&state.db, &criteria);
    let total_items = query_result.total_count;
    let total_pages = query_result.total_pages;
    let current_page = query_result.current_page;
    let page_items: Vec<_> = query_result.items.iter().collect();

    // Get full snapshot for subcategory counts (paginated result is insufficient)
    let snap_for_counts = crate::engine::query_engine::CentralQueryEngine::snapshot(&state.db);
    let title = localized(&lang, &category.name_tr, &category.name_en);
    let sub_hub_html = if child_categories.is_empty() {
        String::new()
    } else {
        let sub_cards: String = child_categories
            .iter()
            .map(|sc| {
                let sc_count = snap_for_counts.data.contents
                    .iter()
                    .filter(|it| CentralSearchEngine::item_matches_category(it, &sc.slug))
                    .count();
                let sc_title = esc(localized(&lang, &sc.name_tr, &sc.name_en));
                let sc_desc = esc(localized(&lang, &sc.description_tr, &sc.description_en));
                format!(
                    r#"<a class="subdossier-card" href="/kategori/{slug}"><div class="subdossier-badge">{title}</div><div class="subdossier-copy"><h3>{title}</h3><p>{desc}</p><span>{cnt} {cnt_lbl} →</span></div></a>"#,
                    slug = esc(&sc.slug),
                    title = sc_title,
                    desc = sc_desc,
                    cnt = sc_count,
                    cnt_lbl = if lang == "en" { "articles" } else { "içerik" }
                )
            })
            .collect();
        let hub_lbl = if lang == "en" {
            "EXHIBITION & CONFERENCE SPECIAL DOSSIERS"
        } else {
            "FUAR, KONFERANS VE ETKİNLİK ÖZEL DOSYALARI"
        };
        format!(
            r#"<div class="subcategories-hub"><div class="eyebrow">{hub_lbl}</div><div class="subcategories-grid">{sub_cards}</div></div>"#
        )
    };

    let cards = if page_items.is_empty() {
        "<div class=\"empty-state\">Bu kategoride henüz içerik bulunmuyor. <a href=\"/arama\">Tüm içerikleri keşfedin →</a></div>".into()
    } else {
        format!(
            "<div class=\"stories category-stories layout-grid\">{}</div>",
            page_items
                .iter()
                .enumerate()
                .map(|(idx, c)| article_card(c, &lang, idx < 3))
                .collect::<String>()
        )
    };

    let pagination_html = if total_pages > 1 {
        let ssb_q = ssb_filter
            .as_ref()
            .map(|s| format!("&ssb={}", urlencoding::encode(s)))
            .unwrap_or_default();
        let mut links = String::new();
        if current_page > 1 {
            links.push_str(&format!(
                "<a class=\"button secondary\" rel=\"prev\" href=\"/kategori/{}?page={}{}\">← Önceki</a>",
                esc(&slug),
                current_page - 1,
                ssb_q
            ));
        }
        let win_start = current_page.saturating_sub(2).max(1);
        let win_end = (win_start + 4).min(total_pages);
        for p_no in win_start..=win_end {
            let active = if p_no == current_page {
                " aria-current=\"page\" class=\"button primary\""
            } else {
                " class=\"button secondary\""
            };
            links.push_str(&format!(
                "<a{} href=\"/kategori/{}?page={}{}\">{}</a>",
                active,
                esc(&slug),
                p_no,
                ssb_q,
                p_no
            ));
        }
        if current_page < total_pages {
            links.push_str(&format!(
                "<a class=\"button secondary\" rel=\"next\" href=\"/kategori/{}?page={}{}\">Sonraki →</a>",
                esc(&slug),
                current_page + 1,
                ssb_q
            ));
        }
        format!(
            "<nav class=\"pagination-nav\" aria-label=\"Kategori sayfaları\">{}</nav>",
            links
        )
    } else {
        String::new()
    };

    let tags = state.db.get_all_ssb_tags();
    let filters: String = tags
        .iter()
        .map(|t| {
            format!(
                "<option value=\"{}\" {}>{}</option>",
                esc(&t.code),
                if ssb_filter.as_ref() == Some(&t.code) {
                    "selected"
                } else {
                    ""
                },
                esc(localized(&lang, &t.name_tr, &t.name_en))
            )
        })
        .collect();
    let mut sidebar_related: Vec<_> = snap_for_counts.data.contents
        .iter()
        .filter(|c| c.content_type != "yeni-sayi-kapak" && !page_items.iter().take(6).any(|x| x.id == c.id))
        .cloned()
        .collect();
    sidebar_related.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));
    sidebar_related.truncate(5);
    let all_ads = state.db.get_all_ads();
    let sidebar = editorial_sidebar(&slug, &sidebar_related, &all_ads, &lang);

    let body = format!(
        r#"<section class="container section-space category-page"><div class="category-master-grid"><div class="category-main-col"><div class="msi-section-ribbon">{ribbon}</div><div class="page-heading"><h1>{title}</h1><p>{desc}</p></div>{sub_hub}<form class="filter-toolbar" method="get"><label for="category-topic">İlgi alanı</label><select id="category-topic" name="ssb"><option value="">Tüm konular</option>{filters}</select><button class="button secondary">Filtrele</button><a href="/kategori/{slug}" class="text-link">Sıfırla</a><span>{total} içerik</span></form>{cards}{pagination}</div>{sidebar}</div></section>"#,
        ribbon = esc(title),
        title = esc(title),
        desc = esc(localized(
            &lang,
            &category.description_tr,
            &category.description_en
        )),
        sub_hub = sub_hub_html,
        filters = filters,
        slug = esc(&slug),
        total = total_items,
        cards = cards,
        pagination = pagination_html,
        sidebar = sidebar
    );

    let settings_map = state.db.get_setting_map(&lang);
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings_map);
    let page_items_owned: Vec<ContentItem> = page_items.iter().map(|i| (*i).clone()).collect();
    let mut seo = CentralSeoEngine::build_category_seo_meta(
        &state.config.public_url,
        &category,
        &page_items_owned,
        current_page,
        total_pages,
        &lang,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );
    if ssb_filter.is_some() {
        seo.robots = "noindex, follow".to_string();
    }

    Html(render_shell_with_seo(&state, &lang, "magazine", &seo, &body, Some(&csp_nonce.0))).into_response()
}

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
        return render_not_found(State(state), headers).await;
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
    let body = render_page(page, &data);
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings);
    let seo = CentralSeoEngine::build_edergi_seo_meta(
        &state.config.public_url,
        &page.title,
        &page.description,
        &snap.issues,
        &lang,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );
    Html(design_engine::render_master_shell_with_nonce(
        &lang,
        &page.mode,
        &settings,
        &seo,
        &snap.categories,
        &snap.ssb_tags,
        &snap.issues,
        &snap.contents,
        &snap.notifications,
        &body,
        0,
        Some(&csp_nonce.0),
    ))
    .into_response()
}

pub async fn render_subscriptions_and_store(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let lang = detect_language(&params, &headers);
    let settings = state.db.get_setting_map(&lang);
    let config = crate::engine::design::config::load(&settings);
    match config.page("abonelik-ve-satis") {
        Some(page) => render_configured_page(&state, &lang, page, &settings, None),
        None => render_not_found(State(state), headers).await,
    }
}

pub async fn render_corporate_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let lang = detect_language(&params, &headers);
    let settings = state.db.get_setting_map(&lang);
    let config = crate::engine::design::config::load(&settings);
    let normalized = match slug.as_str() {
        "kvkk-ve-gizlilik" | "aydinlatma-metni" => "kvkk",
        "biz-kimiz" => "hakkimizda",
        "gizlilik" => "gizlilik-politikasi",
        "cerezler" | "cookie-policy" => "cerez-politikasi",
        "kullanim-sartlari" => "kullanim-kosullari",
        "mesafeli-satis" | "on-bilgilendirme" => "mesafeli-satis-sozlesmesi",
        "iade" | "iptal-iade" => "iptal-ve-iade-kosullari",
        "iys" | "ticari-ileti" => "ticari-ileti-aydinlatma",
        "tekzip" | "duzeltme" => "duzeltme-ve-cevap-hakki",
        "medya-kiti" => "reklam-ve-sponsorluk",
        other => other,
    };
    match config.page(&format!("kurumsal-{}", normalized)) {
        Some(page) => render_configured_page(&state, &lang, page, &settings, None),
        None => render_not_found(State(state), headers).await,
    }
}

pub async fn render_search_page(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    use crate::engine::content_query_engine::CentralContentQueryEngine;

    let lang = detect_language(&params, &headers);

    // Unified query via CentralContentQueryEngine — replaces all manual param parsing and faceted search calls.
    let mut criteria = CentralContentQueryEngine::from_params(&params);
    criteria.sort_by = criteria.sort_by.or_else(|| Some("latest".to_string()));
    criteria.lang = Some(lang.clone());
    criteria.limit = Some(60);

    let _query_result = CentralContentQueryEngine::execute(&state.db, &criteria);

    let q = criteria.q.clone().unwrap_or_default();
    let cat = criteria.category.clone().unwrap_or_default();
    let ssb = criteria.ssb_tag.clone().unwrap_or_default();
    let iss = criteria.issue_number;
    let src = criteria.source_group.clone().unwrap_or_default();
    let sort = criteria.sort_by.clone().unwrap_or_else(|| "latest".to_string());

    let has_active_filter = !q.trim().is_empty()
        || !cat.trim().is_empty()
        || !ssb.trim().is_empty()
        || iss.is_some()
        || !src.trim().is_empty();

    // Build a SearchFacetResponse from the unified result for the existing design renderer.
    let snap = CentralQueryEngine::snapshot(&state.db);
    let facet_criteria = SearchFilterQuery {
        q: criteria.q.clone(),
        category: criteria.category.clone(),
        ssb_tag: criteria.ssb_tag.clone(),
        content_type: criteria.content_type.clone(),
        source_group: criteria.source_group.clone(),
        issue_number: criteria.issue_number,
        visibility_level: criteria.visibility_level,
        is_sponsored: criteria.is_sponsored,
        sort_by: criteria.sort_by.clone(),
        lang: criteria.lang.clone(),
        limit: Some(60),
        page: None,
        per_page: None,
    };
    let facet_res = search_engine::execute_faceted_search(
        &facet_criteria,
        &snap.data.contents,
        &snap.data.categories,
        &snap.data.ssb_tags,
        &snap.data.issues,
    );

    let body_html = design_engine::render_search_and_filter_page(
        &lang, &facet_res, &q, &cat, &ssb, iss, &src, &sort, &state.translations,
    );

    Html(render_shared_shell(
        &state,
        &lang,
        &state.translations.t(&lang, "seo.search_page_title"),
        &body_html,
        Some("/arama"),
        None,
        has_active_filter,
        Some(&csp_nonce.0),
    ))
    .into_response()
}

pub async fn render_account_page(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = detect_language(&params, &headers);
    let tr = &state.translations;
    let Some(user) = authenticated_reader(&state, &headers) else {
        let body = format!(r##"<section class="container section-space"><div class="account-welcome"><div class="eyebrow">{eyebrow}</div><h1>{line1}<br>{line2}</h1><p>{desc}</p><a href="#modal-login" class="button primary">{login_cta}</a><a href="/abonelik-ve-satis" class="text-link">{sub_link}</a></div></section>"##,
            eyebrow = tr.t(&lang, "account.eyebrow"),
            line1 = tr.t(&lang, "account.heading_line1"),
            line2 = tr.t(&lang, "account.heading_line2"),
            desc = tr.t(&lang, "account.description"),
            login_cta = tr.t(&lang, "account.login_cta"),
            sub_link = tr.t(&lang, "account.subscription_link"),
        );
        return Html(render_shared_shell(
            &state,
            &lang,
            &tr.t(&lang, "account.page_title_guest"),
            &body,
            Some("/hesabim"),
            None,
            true,
            Some(&csp_nonce.0),
        ))
        .into_response();
    };
    let reminders: Vec<_> = state
        .db
        .get_renewal_reminders()
        .into_iter()
        .filter(|r| r.user_id == user.id)
        .collect();
    let invoices: Vec<_> = state
        .db
        .get_order_invoices()
        .into_iter()
        .filter(|i| i.user_email == user.email)
        .collect();
    let notifications = crate::engine::notification_engine::filter_user_notifications(
        &state.db.get_system_notifications(Some(&user.email)),
        Some(&user.email),
        Some(&user.organization_category),
    );
    let ssb_tags = state.db.get_all_ssb_tags();

    let body_html = design_engine::render_reader_account_portal(
        &lang,
        &user,
        &reminders,
        &invoices,
        &notifications,
        &ssb_tags,
    );

    Html(render_shared_shell(
        &state,
        &lang,
        &tr.t(&lang, "account.page_title_logged_in"),
        &body_html,
        Some("/hesabim"),
        None,
        true,
        Some(&csp_nonce.0),
    ))
    .into_response()
}

pub async fn render_sitemap_xml(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let mut xml = seo_router_engine::generate_sitemap_xml(
        &state.config.public_url,
        &snap.data.categories,
        &snap.data.contents,
        &snap.data.issues,
        &brand,
    );
    let design = crate::engine::design::config::load(&state.db.get_setting_map("tr"));
    let clean_base = CentralSeoEngine::normalize_base_url(&state.config.public_url);
    let custom: String = design
        .pages
        .iter()
        .filter(|p| {
            !["magazine", "news", "e-dergi", "abonelik-ve-satis"].contains(&p.slug.as_str())
        })
        .map(|p| {
            let path = crate::engine::url_builder::designed_page_url(&p.slug);
            format!(
                "  <url><loc>{}{}</loc><changefreq>weekly</changefreq><priority>0.75</priority></url>\n",
                crate::engine::security_engine::escape_html(&clean_base),
                path
            )
        })
        .collect();
    xml = xml.replace("</urlset>", &format!("{}</urlset>", custom));
    typed_response("application/xml; charset=utf-8", xml)
}

pub async fn render_news_sitemap_xml(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let xml =
        CentralSeoEngine::generate_news_sitemap_xml(&state.config.public_url, &snap.data.contents, &brand);
    typed_response("application/xml; charset=utf-8", xml)
}

pub async fn render_image_sitemap_xml(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let xml = CentralSeoEngine::generate_image_sitemap_xml(
        &state.config.public_url,
        &snap.data.contents,
        &snap.data.issues,
        &brand,
    );
    typed_response("application/xml; charset=utf-8", xml)
}

pub async fn render_robots_txt(State(state): State<AppState>) -> Response {
    let txt = seo_router_engine::generate_robots_txt(&state.config.public_url);
    typed_response("text/plain; charset=utf-8", txt)
}

pub async fn render_rss_xml(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let lang = detect_language(&params, &headers);
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map(&lang));
    let xml =
        seo_router_engine::generate_rss_xml(&state.config.public_url, &snap.data.contents, &lang, &brand);
    typed_response("application/rss+xml; charset=utf-8", xml)
}

pub async fn render_json_feed(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let lang = detect_language(&params, &headers);
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map(&lang));
    let body = CentralSeoEngine::generate_json_feed(
        &state.config.public_url,
        &snap.data.contents,
        &lang,
        &brand,
    );
    typed_response("application/feed+json; charset=utf-8", body)
}

pub async fn render_llms_txt(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let txt = CentralSeoEngine::generate_llms_txt(
        &state.config.public_url,
        &snap.data.categories,
        &snap.data.ssb_tags,
        &snap.data.issues,
        &snap.data.contents,
        false,
        &brand,
    );
    typed_response("text/plain; charset=utf-8", txt)
}

pub async fn render_llms_full_txt(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let txt = CentralSeoEngine::generate_llms_txt(
        &state.config.public_url,
        &snap.data.categories,
        &snap.data.ssb_tags,
        &snap.data.issues,
        &snap.data.contents,
        true,
        &brand,
    );
    typed_response("text/plain; charset=utf-8", txt)
}

pub async fn render_ai_plugin_json(State(state): State<AppState>) -> Response {
    let base = state.config.public_url.trim_end_matches('/');
    let plugin = serde_json::json!({
        "schema_version": "v1",
        "name_for_human": "MSI Dergisi",
        "name_for_model": "msi_dergisi",
        "description_for_human": "Türkiye'nin savunma ve havacılık sanayii dergisi MSI Dergisi'nin güncel haberleri, analizleri ve dergi içerikleri.",
        "description_for_model": "MSI Dergisi is Turkey's leading defense and aerospace industry magazine. Use this plugin to search for latest defense news, military technology analyses, interviews with industry leaders, and magazine content about Turkish and global defense sector developments.",
        "auth": { "type": "none" },
        "api": {
            "type": "openapi",
            "url": format!("{}/api/v1/openapi.json", base),
            "is_user_authenticated": false
        },
        "logo_url": format!("{}/images/icon-512.png", base),
        "contact_email": "info@savunmahaber.com",
        "legal_info_url": format!("{}/kurumsal/kullanim-kosullari", base)
    });
    typed_response(
        "application/json; charset=utf-8",
        serde_json::to_string_pretty(&plugin).unwrap_or_default(),
    )
}

pub async fn render_opensearch_xml(State(state): State<AppState>) -> Response {
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let xml = CentralSeoEngine::generate_opensearch_xml(&state.config.public_url, &brand_config);
    typed_response("application/opensearchdescription+xml; charset=utf-8", xml)
}

pub async fn render_web_manifest(State(state): State<AppState>) -> Response {
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let json_body = CentralSeoEngine::generate_web_manifest(&state.config.public_url, &brand_config);
    typed_response("application/manifest+json; charset=utf-8", json_body)
}

pub async fn render_favicon_ico(State(state): State<AppState>) -> Response {
    let p = format!("{}/favicon.ico", state.config.static_dir.trim_end_matches('/'));
    if let Ok(bytes) = tokio::fs::read(&p).await {
        return typed_response("image/x-icon", bytes);
    }
    Redirect::permanent("/images/favicon.png").into_response()
}

pub async fn render_favicon_svg(State(state): State<AppState>) -> Response {
    let p = format!(
        "{}/images/favicon.svg",
        state.config.static_dir.trim_end_matches('/')
    );
    if let Ok(bytes) = tokio::fs::read(&p).await {
        return typed_response("image/svg+xml; charset=utf-8", bytes);
    }
    Redirect::permanent("/images/logo.svg").into_response()
}

pub async fn render_apple_touch_icon(State(state): State<AppState>) -> Response {
    let p = format!(
        "{}/images/apple-touch-icon.png",
        state.config.static_dir.trim_end_matches('/')
    );
    if let Ok(bytes) = tokio::fs::read(&p).await {
        return typed_response("image/png", bytes);
    }
    Redirect::permanent("/images/icon-192.png").into_response()
}

pub async fn render_indexnow_key(State(state): State<AppState>) -> Response {
    let key = crate::services::indexnow_service::resolve_indexnow_key(&state.config);
    typed_response("text/plain; charset=utf-8", key)
}

pub async fn redirect_en_home() -> Response {
    Redirect::permanent("/?lang=en").into_response()
}

pub async fn redirect_legacy_haber_tr(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Response {
    for old in [format!("/haberler/{}", slug), format!("/haber/{}", slug)] {
        if let Some(rule) = state.db.find_seo_redirect(&old) {
            return Redirect::permanent(&rule.new_path).into_response();
        }
    }
    Redirect::permanent(&crate::engine::url_builder::article_url(&slug)).into_response()
}

pub async fn redirect_legacy_haber_en(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Response {
    let old = crate::engine::url_builder::legacy_haber_en_url(&slug);
    if let Some(rule) = state.db.find_seo_redirect(&old) {
        return Redirect::permanent(&rule.new_path).into_response();
    }
    Redirect::permanent(&crate::engine::url_builder::article_url_lang(&slug, "en")).into_response()
}

pub async fn redirect_legacy_kategori_tr(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Response {
    for old in [
        format!("/kategoriler/{}", slug),
        crate::engine::url_builder::category_url(&slug),
    ] {
        if let Some(rule) = state.db.find_seo_redirect(&old) {
            return Redirect::permanent(&rule.new_path).into_response();
        }
    }
    Redirect::permanent(&crate::engine::url_builder::category_url(&slug)).into_response()
}

pub async fn redirect_legacy_kategori_en(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Response {
    for old in [
        format!("/en/kategoriler/{}", slug),
        format!("/en/kategori/{}", slug),
    ] {
        if let Some(rule) = state.db.find_seo_redirect(&old) {
            return Redirect::permanent(&rule.new_path).into_response();
        }
    }
    Redirect::permanent(&crate::engine::url_builder::category_url_lang(&slug, "en")).into_response()
}

pub async fn render_legacy_html_page(
    State(state): State<AppState>,
    Path(filename): Path<String>,
    Query(_params): Query<HashMap<String, String>>,
    _headers: HeaderMap,
) -> Response {
    // Check seo_redirects DB table first for any registered redirect
    let old_path = format!("/{}", filename);
    if let Some(rule) = state.db.find_seo_redirect(&old_path) {
        return Redirect::permanent(&rule.new_path).into_response();
    }

    // Fallback hardcoded 301 redirects for known legacy .html URLs
    match filename.as_str() {
        "detail-1.html" => {
            Redirect::permanent("/icerik/kaan-programinda-mevcut-durum-ve-yol-haritasi").into_response()
        }
        "detail-2.html" => {
            Redirect::permanent("/icerik/akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu").into_response()
        }
        "detail-3.html" => {
            Redirect::permanent("/icerik/ctech-suru-ida-hibrit-haberlesme-konsepti").into_response()
        }
        "index.html" => {
            Redirect::permanent("/").into_response()
        }
        "about.html" => {
            Redirect::permanent("/kurumsal/hakkimizda").into_response()
        }
        "contact.html" => {
            Redirect::permanent("/kurumsal/iletisim").into_response()
        }
        "gallery1.html" | "gallery.html" => {
            Redirect::permanent("/e-dergi").into_response()
        }
        "technology.html" | "lifestyle.html" | "travel.html" => {
            Redirect::permanent("/arama").into_response()
        }
        _ => render_not_found(State(state), _headers).await,
    }
}

pub async fn handle_fallback_or_redirect(
    State(state): State<AppState>,
    uri: axum::http::Uri,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let path = uri.path();

    // 1. Check 301 SEO Redirect database table first
    if let Some(rule) = state.db.find_seo_redirect(path) {
        return Redirect::permanent(&rule.new_path).into_response();
    }

    // 2. Check if path matches Dynamic IndexNow verification file (`/{key}.txt`) or a legacy `.html` URL
    let trimmed = path.trim_start_matches('/').trim_end_matches('/');
    let indexnow_key = crate::services::indexnow_service::resolve_indexnow_key(&state.config);
    if trimmed == format!("{}.txt", indexnow_key) {
        return typed_response("text/plain; charset=utf-8", indexnow_key);
    }
    if trimmed.ends_with(".html") {
        return render_legacy_html_page(
            State(state),
            Path(trimmed.to_string()),
            Query(params),
            headers,
        )
        .await;
    }

    // 2b. Direct corporate & legal page slugs
    if matches!(
        trimmed,
        "kunye"
            | "hakkimizda"
            | "yolculugumuz"
            | "kvkk"
            | "kvkk-basvuru-formu"
            | "gizlilik-politikasi"
            | "cerez-politikasi"
            | "kullanim-kosullari"
            | "mesafeli-satis-sozlesmesi"
            | "iptal-ve-iade-kosullari"
            | "ticari-ileti-aydinlatma"
            | "telif-ve-etik"
            | "duzeltme-ve-cevap-hakki"
            | "reklam-ve-sponsorluk"
            | "iletisim"
            | "cozum-ortaklarimiz"
    ) {
        return render_corporate_page(
            State(state),
            Path(trimmed.to_string()),
            Query(params),
            headers,
        )
        .await;
    }

    // 2c. English corporate shortcuts (`/en/hakkimizda`, `/en/kunye`, `/en/kvkk`, etc.)
    if let Some(en_sub) = trimmed.strip_prefix("en/") {
        let mut en_params = params.clone();
        en_params.insert("lang".to_string(), "en".to_string());
        if matches!(
            en_sub,
            "kunye" | "hakkimizda" | "yolculugumuz" | "kvkk" | "iletisim" | "cozum-ortaklarimiz"
        ) {
            return render_corporate_page(
                State(state),
                Path(en_sub.to_string()),
                Query(en_params),
                headers,
            )
            .await;
        }
    }

    // 2d. Legacy msidergisi.com magazine viewer routes (`/magazines`, `/magazines/*`, `/en/magazines/*`)
    if trimmed == "magazines" || trimmed.starts_with("magazines/") {
        return Redirect::permanent("/e-dergi").into_response();
    }
    if trimmed == "en/magazines" || trimmed.starts_with("en/magazines/") {
        return Redirect::permanent("/e-dergi?lang=en").into_response();
    }

    // 3. Check if path matches a category slug directly (e.g. /haberler, /ozel-dosyalar, /soylesiler)
    if state.db.get_category_by_slug(trimmed).is_some() {
        let csp_nonce = CspNonce(String::new());
        return render_category_page(
            State(state),
            Path(trimmed.to_string()),
            Query(params),
            headers,
            Extension(csp_nonce),
        )
        .await;
    }

    // 4. Check if path matches an article slug directly
    if state.db.get_content_by_slug(trimmed).is_some() {
        let csp_nonce = CspNonce(String::new());
        return render_article_detail(
            State(state),
            Path(trimmed.to_string()),
            Query(params),
            headers,
            Extension(csp_nonce),
        )
        .await;
    }

    render_not_found(State(state), headers).await
}

pub async fn render_not_found(State(state): State<AppState>, headers: HeaderMap) -> Response {
    // 404 handler cannot extract Extension<CspNonce> because it is called from other handlers
    // that already consumed the extension. Use None nonce — CSP will fall back to unsafe-inline
    // for this error page only.
    let lang = detect_language(&HashMap::new(), &headers);
    let tr = &state.translations;
    let body_html = format!(r##"<section class="py-5 bg-light text-center">
        <div class="container py-5">
            <span class="badge bg-danger mb-2">{badge}</span>
            <h1 class="fw-bold mb-3">{title}</h1>
            <p class="text-muted mb-4">{desc}</p>
            <div class="d-flex justify-content-center gap-2">
                <a href="/" class="btn btn-danger px-4">{home}</a>
                <a href="/arama" class="btn btn-outline-dark px-4">{search}</a>
            </div>
        </div>
    </section>"##,
        badge = tr.t(&lang, "error.404_badge"),
        title = tr.t(&lang, "error.404_title"),
        desc = tr.t(&lang, "error.404_desc"),
        home = tr.t(&lang, "error.404_home"),
        search = tr.t(&lang, "error.404_search"),
    );
    let html = render_shared_shell(
        &state,
        &lang,
        &tr.t(&lang, "error.404_page_title"),
        &body_html,
        None,
        None,
        true,
        None,
    );
    (StatusCode::NOT_FOUND, Html(html)).into_response()
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
        return render_not_found(State(state), headers).await;
    };
    render_configured_page(&state, &lang, page, &settings, Some(&csp_nonce.0))
}

fn render_configured_page(
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
    let all_contents = state.db.get_all_contents();

    // Find all contents whose author_name matches this slug
    let mut matched_articles: Vec<&ContentItem> = all_contents
        .iter()
        .filter(|c| author_slug_from_name(&c.author_name) == slug)
        .collect();

    if matched_articles.is_empty() {
        return render_not_found(State(state), headers).await;
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

    let count_label = if lang == "en" {
        format!("{} articles", total_articles)
    } else {
        format!("{} içerik", total_articles)
    };

    let heading_label = if lang == "en" {
        "AUTHOR PROFILE"
    } else {
        "YAZAR PROFİLİ"
    };

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
        heading = esc(heading_label),
        name = display_name,
        title = display_title,
        count = esc(&count_label),
        cards = cards_html,
    );

    let page_title = if lang == "en" {
        format!("{} — Author Profile", author_name)
    } else {
        format!("{} — Yazar Profili", author_name)
    };

    Html(render_shared_shell(
        &state,
        &lang,
        &page_title,
        &body,
        Some(&format!("/yazar/{}", slug)),
        None,
        false,
        Some(&csp_nonce.0),
    ))
    .into_response()
}

fn authenticated_reader(
    state: &AppState,
    headers: &HeaderMap,
) -> Option<crate::models::UserAccount> {
    crate::services::auth_service::session_user(state, headers)
}
