use axum::{routing::{get, post}, Router};

use crate::controllers::{api_controller, AppState};

pub fn build() -> Router<AppState> {
    Router::new()
        // ── Public REST API (/api/v1/*) ─────────────────────────────────
        .route("/api/v1/health", get(api_controller::health_check))
        .route("/api/v1/search", get(api_controller::faceted_search_api))
        .route("/api/v1/notifications", get(api_controller::list_notifications_api))
        .route(
            "/api/v1/notifications/:id/read",
            post(api_controller::mark_notification_read_api),
        )
        .route("/api/v1/settings", get(api_controller::list_site_settings_api))
        .route("/api/v1/contents", get(api_controller::list_contents))
        .route("/api/v1/contents/:slug", get(api_controller::get_content_detail))
        .route("/api/v1/public/personalized", get(api_controller::public_personalized_feed))
        .route("/api/v1/public/related", get(api_controller::public_related_articles))
        .route(
            "/api/v1/public/newsletter/subscribe",
            post(api_controller::public_newsletter_subscribe),
        )
        .route("/api/v1/categories", get(api_controller::list_categories))
        .route("/api/v1/ssb-tags", get(api_controller::list_ssb_tags))
        .route("/api/v1/issues", get(api_controller::list_issues))
        .route("/api/v1/ads", get(api_controller::list_ads))
        .route(
            "/api/v1/edergi/:issue_no/download-pdf",
            get(api_controller::download_edergi_pdf),
        )
        .route(
            "/api/v1/edergi/:issue_no/pdf",
            get(api_controller::download_edergi_pdf),
        )
        .route(
            "/api/v1/edergi/:issue_no/pages/:page_num",
            get(api_controller::stream_edergi_page),
        )
        .route("/api/v1/auth/register", post(api_controller::register_user))
        .route("/api/v1/auth/login", post(api_controller::login_user))
        .route("/api/v1/auth/logout", post(api_controller::logout_user))
        .route(
            "/api/v1/auth/totp/setup",
            post(api_controller::totp_setup),
        )
        .route("/api/v1/auth/totp/verify-setup", post(api_controller::totp_verify_setup))
        .route("/api/v1/auth/totp/verify", post(api_controller::totp_verify_setup))
        .route("/api/v1/auth/totp/login", post(api_controller::totp_login))
        .route("/api/v1/auth/totp/disable", post(api_controller::totp_disable))
        .route("/api/v1/auth/forgot-password", post(api_controller::forgot_password))
        .route("/api/v1/auth/reset-password", post(api_controller::reset_password))
        .route(
            "/api/v1/auth/resend-verification",
            post(api_controller::resend_verification),
        )
        .route("/api/v1/email-preferences", post(api_controller::manage_email_preferences))
        .route("/api/v1/account/profile", post(api_controller::update_account_profile))
        .route("/api/v1/account/export", get(api_controller::export_account_data))
        .route("/api/v1/account/delete", post(api_controller::delete_account))
        .route("/api/v1/analytics/track", post(api_controller::track_analytics_event))
        .route("/api/v1/commerce/catalog", get(api_controller::get_commerce_catalog))
        .route("/api/v1/commerce/cart-checkout", post(api_controller::cart_checkout))
        .route("/api/v1/commerce/download/:sku", get(api_controller::download_purchased_asset))
        .route("/api/v1/subscriptions/checkout", post(api_controller::checkout_order))
        .route("/api/v1/payments/paytr-callback", post(api_controller::paytr_callback))
        .route("/api/v1/payments/paytr/callback", post(api_controller::paytr_callback))
}