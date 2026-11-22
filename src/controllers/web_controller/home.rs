use super::*;
use crate::engine::design::{
    labels_helper::localize_section_copy,
    shell::{self, ShellOptions},
};

pub async fn render_home(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_home_for_lang(&state, &params, &headers, &csp_nonce.0, None).await
}

pub async fn render_en_home(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_home_for_lang(&state, &params, &headers, &csp_nonce.0, Some("en")).await
}

async fn render_home_for_lang(
    state: &AppState,
    params: &HashMap<String, String>,
    headers: &HeaderMap,
    csp_nonce: &str,
    forced_lang: Option<&str>,
) -> Response {
    let t0 = Instant::now();
    let lang = forced_lang
        .map(str::to_string)
        .unwrap_or_else(|| detect_language(params, headers));
    let domain_mode = detect_domain_mode(params, headers);
    let canonical_path = if forced_lang == Some("en") { "/en" } else { "/" };

    let snap = CentralQueryEngine::snapshot(&state.db);
    let settings_map = state.db.get_setting_map(&lang);

    let design = crate::engine::design::config::load(&settings_map);
    let page = design.page(&domain_mode).expect("validated home page");
    let brand_title = if lang == "en" {
        settings_map
            .get("brand_title")
            .map(|s| localize_section_copy(&lang, s))
            .unwrap_or_else(|| localize_section_copy(&lang, &page.title))
    } else {
        &page.title
    };
    let brand_desc = localize_section_copy(&lang, &page.description);
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
        canonical_path,
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

    let mut body_html = design_engine::render_dual_homepage(
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

    let user_opt = authenticated_reader(state, headers);
    if let Some(ref user) = user_opt {
        let allowed_feed_contents: Vec<ContentItem> = snap
            .data
            .contents
            .iter()
            .filter(|c| {
                content_service::user_matches_target_institutions(c, Some(user), &state.db)
            })
            .cloned()
            .collect();
        let feed_html = crate::engine::design::components::sections::render_personalized_feed(
            &allowed_feed_contents,
            &user.ssb_interests,
            &snap.data.ssb_tags,
            &lang,
            Some(&state.translations),
            6,
        );
        body_html.push_str(&feed_html);
    }
    body_html.push_str(
        &crate::engine::design::components::sections::render_newsletter_trigger_band(
            &lang,
            Some(&state.translations),
        ),
    );
    body_html.push_str(
        &crate::engine::design::components::sections::render_related_strip(
            &snap.data.contents,
            &crate::engine::design::components::sections::RelatedStripOptions {
                lang: &lang,
                is_current_sponsored: false,
                exclude_ids: &[],
                limit: 4,
                tr: Some(&state.translations),
                source_context: "home",
            },
        ),
    );
    let floating_popup =
        helpers::select_floating_popup_ad(&snap.data.ads, None, None, user_opt.as_ref());
    let can_read_pdf = helpers::user_can_read_edergi_pdf(state, user_opt.as_ref());
    let institutions = crate::engine::design::labels_helper::institution_choices_from_db(&state.db);

    let exec_us = t0.elapsed().as_micros();
    let full_html = shell::render_full(
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
        Some(csp_nonce),
        &ShellOptions {
            tr: Some(&state.translations),
            institutions: &institutions,
            floating_popup: floating_popup.as_ref(),
            can_read_pdf,
        },
    );

    Html(full_html).into_response()
}

pub async fn render_admin(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = detect_language(&params, &headers);
    let user_opt = crate::services::auth_service::session_user(&state, &headers);
    let admin_headers = [
        (
            header::HeaderName::from_static("x-robots-tag"),
            "noindex, nofollow, noarchive",
        ),
        (header::CACHE_CONTROL, "no-store, private"),
    ];

    let admin_seg = crate::config::normalize_admin_path_secret(&state.config.admin_path_secret);
    if let Some(user) = user_opt {
        if crate::extractors::operator_user::is_staff_role(&state.db, &user.role) {
            let html = admin_view::render_admin_dashboard_full(
                Some(&csp_nonce.0),
                &admin_seg,
                &user.email,
                &user.role,
                &state.translations,
                &lang,
            );
            return (StatusCode::OK, admin_headers, Html(html)).into_response();
        }
    }

    let login_html = admin_view::render_admin_login(
        Some(&csp_nonce.0),
        &admin_seg,
        &state.translations,
        &lang,
    );
    (StatusCode::OK, admin_headers, Html(login_html)).into_response()
}

/// Alternative admin gateway at `/podmin` — delegates cleanly to [`render_admin`]
/// so both login and authenticated staff dashboard support dynamic `?lang=` with zero duplication.
pub async fn render_podmin_gateway(
    state: State<AppState>,
    params: Query<HashMap<String, String>>,
    headers: HeaderMap,
    csp_nonce: Extension<CspNonce>,
) -> Response {
    render_admin(state, params, headers, csp_nonce).await
}

#[allow(dead_code)]
pub(crate) fn render_shell_with_seo(
    state: &AppState,
    lang: &str,
    mode: &str,
    seo: &SeoMetaBundle,
    main_body_html: &str,
    nonce: Option<&str>,
) -> String {
    render_shell_with_context(state, lang, mode, seo, main_body_html, nonce, None, None, None)
}

pub(crate) fn is_utility_or_auth_route(canonical_url: &str, admin_secret: &str) -> bool {
    let raw_path = if let Some(after_scheme) = canonical_url.split("://").nth(1) {
        after_scheme
            .find('/')
            .map(|idx| &after_scheme[idx..])
            .unwrap_or("/")
    } else {
        canonical_url
    };
    let without_query = raw_path.split(&['?', '#'][..]).next().unwrap_or("/");
    let trimmed = without_query.trim_end_matches('/');
    let path_lower = if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_ascii_lowercase()
    };

    if matches!(
        path_lower.as_str(),
        "/hesabim"
            | "/sifre-sifirla"
            | "/sifre-yenile"
            | "/hesap-dogrula"
            | "/abonelik-iptal"
            | "/coming-soon"
            | "/yakinda"
            | "/podmin"
    ) || path_lower.starts_with("/podmin/")
    {
        return true;
    }

    let admin_seg = crate::config::normalize_admin_path_secret(admin_secret).to_ascii_lowercase();
    let admin_prefix = format!("/{}", admin_seg);
    path_lower == admin_prefix || path_lower.starts_with(&format!("{}/", admin_prefix))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_shell_with_context(
    state: &AppState,
    lang: &str,
    mode: &str,
    seo: &SeoMetaBundle,
    main_body_html: &str,
    nonce: Option<&str>,
    headers: Option<&HeaderMap>,
    article_opt: Option<&ContentItem>,
    page_category: Option<&str>,
) -> String {
    let t0 = Instant::now();
    let snap = CentralQueryEngine::snapshot(&state.db);
    let settings_map = state.db.get_setting_map(lang);
    let user_opt = headers.and_then(|h| authenticated_reader(state, h));
    let floating_popup = if seo.canonical_url.is_empty()
        || is_utility_or_auth_route(&seo.canonical_url, &state.config.admin_path_secret)
    {
        None
    } else {
        helpers::select_floating_popup_ad(
            &snap.data.ads,
            page_category,
            article_opt,
            user_opt.as_ref(),
        )
    };
    let can_read_pdf = helpers::user_can_read_edergi_pdf(state, user_opt.as_ref());
    let institutions = crate::engine::design::labels_helper::institution_choices_from_db(&state.db);
    let exec_us = t0.elapsed().as_micros();
    shell::render_full(
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
        &ShellOptions {
            tr: Some(&state.translations),
            institutions: &institutions,
            floating_popup: floating_popup.as_ref(),
            can_read_pdf,
        },
    )
}

pub fn render_shared_shell(
    state: &AppState,
    lang: &str,
    page_title: &str,
    main_body_html: &str,
    canonical_path: Option<&str>,
    article_opt: Option<&ContentItem>,
    noindex: bool,
    nonce: Option<&str>,
) -> String {
    render_shared_shell_with_request(
        state,
        lang,
        page_title,
        main_body_html,
        canonical_path,
        article_opt,
        noindex,
        nonce,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn render_shared_shell_with_request(
    state: &AppState,
    lang: &str,
    page_title: &str,
    main_body_html: &str,
    canonical_path: Option<&str>,
    article_opt: Option<&ContentItem>,
    noindex: bool,
    nonce: Option<&str>,
    headers: Option<&HeaderMap>,
    page_category: Option<&str>,
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

    let default_desc = if lang.eq_ignore_ascii_case("en") {
        "MSI Review & MSI TDR — Defence, Aerospace and Security Technologies Reference Platform (ISSN 1306-0309)"
    } else {
        "MSI Dergisi & MSI TDR — Savunma, Havacılık ve Güvenlik Teknolojileri Referans Platformu (ISSN 1306-0309)"
    };
    let desc = article_opt
        .map(|c| {
            if lang == "en" && !c.summary_en.is_empty() {
                c.summary_en.as_str()
            } else {
                c.summary_tr.as_str()
            }
        })
        .unwrap_or(default_desc);
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
        seo.robots = "noindex, nofollow".to_string();
        if canonical_path.is_none() {
            seo.canonical_url.clear();
            seo.hreflang_tr.clear();
            seo.hreflang_en.clear();
            seo.json_ld.clear();
        }
    }

    render_shell_with_context(
        state,
        lang,
        "magazine",
        &seo,
        main_body_html,
        nonce,
        headers,
        article_opt,
        page_category,
    )
}

#[cfg(test)]
mod tests {
    use super::is_utility_or_auth_route;

    #[test]
    fn utility_and_auth_routes_suppress_floating_popup_while_public_content_preserves_it() {
        for url in [
            "https://msidergisi.com/hesabim",
            "https://msidergisi.com/hesabim/",
            "https://msidergisi.com/sifre-sifirla?token=preview",
            "https://msidergisi.com/sifre-yenile?token=abc",
            "https://msidergisi.com/hesap-dogrula?token=xyz",
            "https://msidergisi.com/abonelik-iptal?token=123",
            "https://msidergisi.com/coming-soon",
            "https://msidergisi.com/yakinda",
            "https://msidergisi.com/podmin",
            "https://msidergisi.com/podmin/tasarim",
            "/hesabim",
        ] {
            assert!(
                is_utility_or_auth_route(url, "podmin"),
                "Expected floating popup suppression on {}",
                url
            );
        }

        assert!(is_utility_or_auth_route("https://msidergisi.com/custom-adm/panel", "custom-adm"));

        for url in [
            "https://msidergisi.com/",
            "https://msidergisi.com/en",
            "https://msidergisi.com/icerik/ornek-haber",
            "https://msidergisi.com/e-dergi",
            "https://msidergisi.com/kurumsal/abonelik",
        ] {
            assert!(
                !is_utility_or_auth_route(url, "podmin"),
                "Expected floating popup allowed on {}",
                url
            );
        }
    }
}
