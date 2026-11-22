use axum::{routing::get, Router};

use crate::controllers::{api_controller, web_controller, AppState};

pub fn build(state: &AppState) -> Router<AppState> {
    let seg = crate::config::normalize_admin_path_secret(&state.config.admin_path_secret);
    let admin_root = format!("/{}", seg);

    let router = Router::new()
        // ── Public SSR Pages ────────────────────────────────────────────
        .route("/", get(web_controller::render_home))
        .route("/index.html", get(web_controller::render_home))
        .route("/en", get(web_controller::render_en_home))
        .route("/css/design-tokens.css", get(web_controller::render_design_tokens_css))
        .route("/switch/:locale", get(web_controller::switch_locale))
        .route("/arama", get(web_controller::render_search_page))
        .route("/search", get(web_controller::render_search_page))
        .route("/en/search", get(web_controller::render_en_search_page))
        .route("/hesabim", get(web_controller::render_account_page))
        .route("/auth/sign-in", get(web_controller::redirect_account_tr))
        .route("/en/auth/sign-in", get(web_controller::redirect_account_en))
        .route("/profile/settings", get(web_controller::redirect_account_tr))
        .route("/en/profile/settings", get(web_controller::redirect_account_en))
        .route("/newsletters/unsubscribe/:token", get(web_controller::redirect_newsletter_unsub_tr))
        .route("/en/newsletters/unsubscribe/:token", get(web_controller::redirect_newsletter_unsub_en))
        .route("/kunye", get(web_controller::redirect_short_corporate_tr))
        .route("/hakkimizda", get(web_controller::redirect_short_corporate_tr))
        .route("/yolculugumuz", get(web_controller::redirect_short_corporate_tr))
        .route("/kvkk", get(web_controller::redirect_short_corporate_tr))
        .route("/iletisim", get(web_controller::redirect_short_corporate_tr))
        .route("/reklam", get(web_controller::redirect_short_corporate_tr))
        .route("/en/kunye", get(web_controller::redirect_short_corporate_en))
        .route("/en/imprint", get(web_controller::redirect_short_corporate_en))
        .route("/en/hakkimizda", get(web_controller::redirect_short_corporate_en))
        .route("/en/about", get(web_controller::redirect_short_corporate_en))
        .route("/en/about-us", get(web_controller::redirect_short_corporate_en))
        .route("/en/yolculugumuz", get(web_controller::redirect_short_corporate_en))
        .route("/en/our-journey", get(web_controller::redirect_short_corporate_en))
        .route("/en/kvkk", get(web_controller::redirect_short_corporate_en))
        .route("/en/privacy-policy", get(web_controller::redirect_short_corporate_en))
        .route("/en/iletisim", get(web_controller::redirect_short_corporate_en))
        .route("/en/contact", get(web_controller::redirect_short_corporate_en))
        .route("/sifre-sifirla", get(web_controller::render_reset_password_page))
        .route("/sifre-yenile", get(web_controller::render_reset_password_page))
        .route(
            "/hesap-dogrula",
            get(api_controller::verify_email).post(api_controller::verify_email_confirm),
        )
        .route(
            "/abonelik-iptal",
            get(api_controller::unsubscribe_page).post(api_controller::unsubscribe_confirm),
        )
        .route("/sitemap.xml", get(web_controller::render_sitemap_xml))
        .route("/sitemap-news.xml", get(web_controller::render_news_sitemap_xml))
        .route("/sitemap-images.xml", get(web_controller::render_image_sitemap_xml))
        .route("/robots.txt", get(web_controller::render_robots_txt))
        .route("/rss.xml", get(web_controller::render_rss_xml))
        .route("/feed.json", get(web_controller::render_json_feed))
        .route("/llms.txt", get(web_controller::render_llms_txt))
        .route("/llms-full.txt", get(web_controller::render_llms_full_txt))
        .route("/.well-known/ai-plugin.json", get(web_controller::render_ai_plugin_json))
        .route("/api/v1/openapi.json", get(web_controller::render_openapi_json))
        .route("/opensearch.xml", get(web_controller::render_opensearch_xml))
        .route("/manifest.webmanifest", get(web_controller::render_web_manifest))
        .route("/favicon.ico", get(web_controller::render_favicon_ico))
        .route("/favicon.svg", get(web_controller::render_favicon_svg))
        .route("/apple-touch-icon.png", get(web_controller::render_apple_touch_icon))
        .route("/indexnow.txt", get(web_controller::render_indexnow_key))
        .route("/sayfa/:slug", get(web_controller::render_designed_page))
        .route("/icerik/:slug", get(web_controller::render_article_detail))
        .route("/yazar/:slug", get(web_controller::render_author_profile))
        .route("/yazarlar", get(web_controller::render_authors_directory))
        .route("/en/authors", get(web_controller::render_en_authors_directory))
        .route("/savunma-sanayii-tarihi", get(web_controller::render_history_hub))
        .route(
            "/en/defence-industry-history",
            get(web_controller::render_en_history_hub),
        )
        .route(
            "/en/defense-industry-history",
            get(web_controller::render_en_history_hub),
        )
        .route("/ogrenci-kulupleri", get(web_controller::render_student_clubs_hub))
        .route("/en/student-clubs", get(web_controller::render_en_student_clubs_hub))
        .route("/etkinlikler", get(web_controller::render_events_hub_page))
        .route("/en/events", get(web_controller::render_en_events_hub_page))
        .route("/haberler/:slug", get(web_controller::redirect_legacy_haber_tr))
        .route("/en/haberler/:slug", get(web_controller::redirect_legacy_haber_en))
        .route("/kategori/:slug", get(web_controller::render_category_page))
        .route("/kategoriler/:slug", get(web_controller::redirect_legacy_kategori_tr))
        .route("/en/kategoriler/:slug", get(web_controller::redirect_legacy_kategori_en))
        .route("/coming-soon", get(web_controller::render_coming_soon))
        .route("/yakinda", get(web_controller::render_coming_soon))
        .route("/e-dergi", get(web_controller::render_edergi_kiosk))
        .route("/abonelik-ve-satis", get(web_controller::render_subscriptions_and_store))
        .route("/kurumsal/:slug", get(web_controller::render_corporate_page))
        .route("/en/kurumsal/:slug", get(web_controller::render_en_corporate_page))
        // ── Enterprise Command Gateway (/{ADMIN_PATH_SECRET}, e.g. /podmin) ───
        .route(&admin_root, get(web_controller::render_admin))
        .route(
            &format!("{}/", admin_root),
            get({
                let target = admin_root.clone();
                move || async move { axum::response::Redirect::permanent(&target) }
            }),
        )
        .route(&format!("{}/tasarim", admin_root), get(crate::engine::design::editor::view))
        .route(
            &format!("{}/tasarim/", admin_root),
            get({
                let target = format!("{}/tasarim", admin_root);
                move || async move { axum::response::Redirect::permanent(&target) }
            }),
        )
        .route(
            &format!("{}/tasarim/onizleme", admin_root),
            axum::routing::post(crate::engine::design::editor::preview),
        );

    let router = mount_editorial_studio_routes(router, &admin_root);

    if !seg.eq_ignore_ascii_case("podmin") {
        let router = router
            .route("/podmin", get(web_controller::render_podmin_gateway))
            .route(
                "/podmin/",
                get(|| async { axum::response::Redirect::permanent("/podmin") }),
            )
            .route("/podmin/tasarim", get(crate::engine::design::editor::view))
            .route(
                "/podmin/tasarim/",
                get(|| async { axum::response::Redirect::permanent("/podmin/tasarim") }),
            )
            .route(
                "/podmin/tasarim/onizleme",
                axum::routing::post(crate::engine::design::editor::preview),
            );
        mount_editorial_studio_routes(router, "/podmin")
    } else {
        router
    }
}

fn mount_editorial_studio_routes(
    router: axum::Router<crate::app_state::AppState>,
    prefix: &str,
) -> axum::Router<crate::app_state::AppState> {
    let editor_path = format!("{prefix}/editor");
    let editor_slash = format!("{prefix}/editor/");
    let editor_target = editor_path.clone();
    router
        .route(
            &editor_path,
            get(crate::controllers::api_controller::render_editorial_studio),
        )
        .route(
            &editor_slash,
            get(move || async move { axum::response::Redirect::permanent(&editor_target) }),
        )
        .route(
            &format!("{prefix}/editor/studio.js"),
            get(crate::controllers::api_controller::editorial_studio_js),
        )
        .route(
            &format!("{prefix}/editor/studio.wasm"),
            get(crate::controllers::api_controller::editorial_studio_wasm),
        )
}