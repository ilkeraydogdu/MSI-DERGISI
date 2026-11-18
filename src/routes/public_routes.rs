use axum::{routing::get, Router};

use crate::controllers::{api_controller, web_controller, AppState};

pub fn build(state: &AppState) -> Router<AppState> {
    let admin_root = format!("/{}", state.config.admin_path_secret);

    Router::new()
        // ── Public SSR Pages ────────────────────────────────────────────
        .route("/", get(web_controller::render_home))
        .route("/index.html", get(web_controller::render_home))
        .route("/en", get(web_controller::render_en_home))
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
        // ── Hidden Enterprise Command Gateway (/{ADMIN_PATH_SECRET}) ───
        .route(&admin_root, get(web_controller::render_admin))
        .route(&format!("{}/tasarim", admin_root), get(crate::engine::design::editor::view))
        .route(
            &format!("{}/tasarim/onizleme", admin_root),
            axum::routing::post(crate::engine::design::editor::preview),
        )
}