use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
    Router,
};

use crate::controllers::{api_controller, AppState};

pub fn build() -> Router<AppState> {
    Router::new()
        // ── Admin OMS/CMS REST API (/api/v1/admin/*) ───────────────────
        .route("/api/v1/admin/me", get(api_controller::admin_me))
        .route("/api/v1/admin/overview", get(api_controller::admin_overview))
        .route(
            "/api/v1/admin/design",
            get(crate::engine::design::editor::get).post(crate::engine::design::editor::save),
        )
        .route("/api/v1/admin/seo/indexnow-ping", post(api_controller::admin_ping_indexnow))
        .route(
            "/api/v1/admin/upload",
            post(api_controller::admin_upload_media).layer(DefaultBodyLimit::max(25 * 1024 * 1024)),
        )
        .route(
            "/api/v1/admin/notifications",
            get(api_controller::admin_list_notifications_api)
                .post(api_controller::admin_create_notification_api),
        )
        .route(
            "/api/v1/admin/settings",
            get(api_controller::admin_list_settings_api)
                .post(api_controller::admin_upsert_setting_api),
        )
        .route("/api/v1/admin/security-logs", get(api_controller::admin_list_security_logs_api))
        .route(
            "/api/v1/admin/contents",
            get(api_controller::list_admin_contents).post(api_controller::admin_upsert_content),
        )
        .route("/api/v1/admin/contents/bulk", post(api_controller::admin_bulk_content_operation))
        .route("/api/v1/admin/contents/:id", delete(api_controller::admin_delete_content))
        .route(
            "/api/v1/admin/contents/:id/versions",
            get(api_controller::admin_list_content_versions)
                .post(api_controller::admin_create_content_version),
        )
        .route(
            "/api/v1/admin/contents/:id/versions/:version/restore",
            post(api_controller::admin_restore_content_version),
        )
        .route(
            "/api/v1/admin/contents/:id/submit-review",
            post(api_controller::submit_content_for_review),
        )
        .route("/api/v1/admin/approvals/pending", get(api_controller::list_pending_approvals))
        .route("/api/v1/admin/approvals/:id/approve", post(api_controller::approve_content))
        .route("/api/v1/admin/approvals/:id/reject", post(api_controller::reject_content))
        .route(
            "/api/v1/admin/ads",
            get(api_controller::list_admin_ads).post(api_controller::admin_upsert_ad),
        )
        .route("/api/v1/admin/ads/check-conflict", post(api_controller::admin_check_ad_conflict))
        .route("/api/v1/admin/ads/:id", delete(api_controller::admin_delete_ad))
        .route(
            "/api/v1/admin/recommendation-rules",
            get(api_controller::admin_list_recommendation_rules)
                .post(api_controller::admin_create_recommendation_rule),
        )
        .route(
            "/api/v1/admin/recommendation-rules/:id",
            put(api_controller::admin_update_recommendation_rule)
                .delete(api_controller::admin_delete_recommendation_rule),
        )
        .route("/api/v1/admin/analytics", get(api_controller::admin_analytics_bundle))
        .route("/api/v1/admin/analytics/bundle", get(api_controller::admin_analytics_bundle))
        .route(
            "/api/v1/admin/analytics/sponsor-report",
            get(api_controller::admin_analytics_bundle),
        )
        .route("/api/v1/admin/analytics/contents", get(api_controller::admin_analytics_contents))
        .route("/api/v1/admin/analytics/ads", get(api_controller::admin_analytics_ads))
        .route("/api/v1/admin/analytics/readers", get(api_controller::admin_analytics_readers))
        .route("/api/v1/admin/analytics/daily", get(api_controller::admin_daily_analytics))
        .route(
            "/api/v1/admin/analytics/author-performance",
            get(api_controller::admin_author_performance),
        )
        .route(
            "/api/v1/admin/analytics/category-performance",
            get(api_controller::admin_category_performance),
        )
        .route(
            "/api/v1/admin/analytics/trending-content",
            get(api_controller::admin_trending_content),
        )
        .route(
            "/api/v1/admin/analytics/revenue-summary",
            get(api_controller::admin_revenue_summary),
        )
        .route(
            "/api/v1/admin/analytics/keyword-performance",
            get(api_controller::admin_keyword_performance),
        )
        .route(
            "/api/v1/admin/analytics/advertiser-report",
            get(api_controller::admin_advertiser_report),
        )
        .route("/api/v1/admin/analytics/user-cohorts", get(api_controller::admin_user_cohorts))
        .route("/api/v1/admin/analytics/churn-rate", get(api_controller::admin_churn_rate))
        .route("/api/v1/admin/analytics/user-ltv", get(api_controller::admin_user_ltv))
        .route("/api/v1/admin/analytics/lifetime-value", get(api_controller::admin_lifetime_value))
        .route(
            "/api/v1/admin/analytics/conversion-funnel",
            get(api_controller::admin_conversion_funnel),
        )
        .route(
            "/api/v1/admin/reports/subscribers",
            get(api_controller::admin_subscribers_report),
        )
        .route(
            "/api/v1/admin/corporate-requests",
            get(api_controller::admin_list_corporate_requests)
                .post(api_controller::admin_decide_corporate_request_body),
        )
        .route(
            "/api/v1/admin/corporate-requests/decide",
            post(api_controller::admin_decide_corporate_request_body),
        )
        .route(
            "/api/v1/admin/corporate-requests/:id/decide",
            post(api_controller::admin_decide_corporate_request_by_path),
        )
        .route(
            "/api/v1/admin/corporate-requests/:id/decision",
            post(api_controller::admin_decide_corporate_request_by_path),
        )
        .route(
            "/api/v1/admin/categories",
            get(api_controller::admin_list_categories).post(api_controller::admin_insert_category),
        )
        .route(
            "/api/v1/admin/categories/:id",
            put(api_controller::admin_update_category).delete(api_controller::admin_delete_category),
        )
        .route(
            "/api/v1/admin/ssb-tags",
            get(api_controller::admin_list_ssb_tags).post(api_controller::admin_insert_ssb_tag),
        )
        .route(
            "/api/v1/admin/ssb-tags/:code",
            put(api_controller::admin_update_ssb_tag).delete(api_controller::admin_delete_ssb_tag),
        )
        .route(
            "/api/v1/admin/newsletters",
            get(api_controller::admin_list_newsletters).post(api_controller::admin_insert_newsletter),
        )
        .route(
            "/api/v1/admin/newsletters/:id",
            delete(api_controller::admin_delete_newsletter),
        )
        .route("/api/v1/admin/reminders", get(api_controller::admin_list_reminders))
        .route(
            "/api/v1/admin/reminders/halt-renewed",
            post(api_controller::admin_halt_renewed_reminders),
        )
        .route(
            "/api/v1/admin/subscriptions/trigger-reminders",
            post(api_controller::admin_halt_renewed_reminders),
        )
        .route(
            "/api/v1/admin/redirects",
            get(api_controller::admin_list_redirects).post(api_controller::admin_insert_redirect),
        )
        .route(
            "/api/v1/admin/redirects/:id",
            delete(api_controller::admin_delete_redirect),
        )
        .route("/api/v1/admin/export/open-data", get(api_controller::admin_export_open_data))
        .route(
            "/api/v1/admin/translations",
            get(api_controller::admin_list_translations_api)
                .post(api_controller::admin_upsert_translation_api),
        )
        .route(
            "/api/v1/admin/translations/auto-translate",
            post(api_controller::admin_auto_translate),
        )
        .route(
            "/api/v1/admin/translations/pending",
            get(api_controller::admin_list_pending_translations),
        )
        .route(
            "/api/v1/admin/translations/:key/approve",
            post(api_controller::admin_approve_translation),
        )
        .route(
            "/api/v1/admin/translations/:key/reject",
            post(api_controller::admin_reject_translation),
        )
        .route(
            "/api/v1/admin/translations/:key",
            delete(api_controller::admin_delete_translation_api),
        )
        .route("/api/v1/admin/media-library", get(api_controller::admin_list_media_library))
        .route(
            "/api/v1/admin/media-library/:id",
            put(api_controller::admin_update_media_item)
                .delete(api_controller::admin_delete_media_item),
        )
        .route(
            "/api/v1/admin/issues",
            get(api_controller::admin_list_issues).post(api_controller::admin_upsert_issue),
        )
        .route(
            "/api/v1/admin/issues/:issue_number",
            delete(api_controller::admin_delete_issue),
        )
        .route(
            "/api/v1/admin/issues/:publication_type/:issue_number",
            get(api_controller::admin_get_issue_by_key)
                .put(api_controller::admin_update_issue_by_key)
                .delete(api_controller::admin_delete_issue_by_key),
        )
        .route(
            "/api/v1/admin/issues/:publication_type/:issue_number/convert-pdf",
            post(api_controller::admin_convert_issue_pdf),
        )
        .route(
            "/api/v1/admin/edergi-files/:name",
            get(api_controller::admin_preview_edergi_pdf),
        )
        .route(
            "/api/v1/admin/subscription-plans",
            get(api_controller::admin_list_subscription_plans)
                .post(api_controller::admin_upsert_subscription_plan),
        )
        .route(
            "/api/v1/admin/subscription-plans/:code",
            delete(api_controller::admin_delete_subscription_plan),
        )
        .route(
            "/api/v1/admin/commerce-products",
            get(api_controller::admin_list_commerce_products)
                .post(api_controller::admin_upsert_commerce_product),
        )
        .route(
            "/api/v1/admin/commerce-products/:id",
            delete(api_controller::admin_delete_commerce_product),
        )
        .route("/api/v1/admin/invoices", get(api_controller::admin_list_invoices))
        .route(
            "/api/v1/admin/invoices/:id/mark-paid",
            post(api_controller::admin_mark_invoice_paid),
        )
        .route(
            "/api/v1/admin/audit-logs",
            get(api_controller::admin_list_audit_logs).post(api_controller::admin_insert_audit_log),
        )
        .route(
            "/api/v1/admin/users",
            get(api_controller::admin_list_users).post(api_controller::admin_create_user),
        )
        .route(
            "/api/v1/admin/users/:id",
            put(api_controller::admin_update_user).delete(api_controller::admin_delete_user),
        )
        .route(
            "/api/v1/admin/users/:id/assign-role",
            post(api_controller::admin_assign_role_to_user),
        )
        .route(
            "/api/v1/admin/user-roles",
            get(api_controller::admin_list_user_roles).post(api_controller::admin_upsert_user_role),
        )
        .route(
            "/api/v1/admin/user-roles/:role_code",
            put(api_controller::admin_update_user_role)
                .post(api_controller::admin_update_user_role)
                .delete(api_controller::admin_delete_user_role),
        )
        .route(
            "/api/v1/admin/custom-fields",
            get(api_controller::admin_list_custom_fields)
                .post(api_controller::admin_upsert_custom_field),
        )
        .route(
            "/api/v1/admin/custom-fields/:field_key",
            delete(api_controller::admin_delete_custom_field),
        )
        .route(
            "/api/v1/admin/custom-field-values",
            get(api_controller::admin_get_custom_field_values)
                .put(api_controller::admin_put_custom_field_values),
        )
        .route(
            "/api/v1/admin/cache",
            get(api_controller::admin_list_cache_operations).post(api_controller::admin_flush_cache),
        )
        .route("/api/v1/admin/system-health", get(api_controller::admin_system_health))
        .route("/api/v1/admin/monitoring/bundle", get(api_controller::admin_monitoring_bundle))
        .route("/api/v1/admin/monitoring/system", get(api_controller::admin_monitoring_system))
        .route("/api/v1/admin/monitoring/domains", get(api_controller::admin_monitoring_domains))
        .route("/api/v1/admin/monitoring/traffic", get(api_controller::admin_monitoring_traffic))
        .route("/api/v1/admin/monitoring/attacks", get(api_controller::admin_monitoring_attacks))
        .route("/api/v1/admin/monitoring/services", get(api_controller::admin_monitoring_services))
        .route("/api/v1/admin/monitoring/seo", get(api_controller::admin_monitoring_seo))
        .route("/api/v1/admin/monitoring/refresh", post(api_controller::admin_monitoring_refresh))
        // ── Glossary CRUD (glossary:manage) ────────────────────────────────
        .route(
            "/api/v1/admin/glossary",
            get(api_controller::admin_list_glossary).post(api_controller::admin_upsert_glossary),
        )
        .route(
            "/api/v1/admin/glossary/:id",
            delete(api_controller::admin_delete_glossary),
        )
        // ── Locales CRUD (locales:manage) ──────────────────────────────────
        .route(
            "/api/v1/admin/locales",
            get(api_controller::admin_list_locales).post(api_controller::admin_upsert_locale),
        )
        .route(
            "/api/v1/admin/locales/:code",
            put(api_controller::admin_update_locale).delete(api_controller::admin_delete_locale),
        )
        // ── Email Groups CRUD (email_groups:manage) ────────────────────────
        .route(
            "/api/v1/admin/email-groups",
            get(api_controller::admin_list_email_groups)
                .post(api_controller::admin_upsert_email_group),
        )
        .route(
            "/api/v1/admin/email-groups/:code",
            delete(api_controller::admin_delete_email_group),
        )
        // ── Backup (backup:manage) ─────────────────────────────────────────
        .route("/api/v1/admin/backup", post(api_controller::admin_trigger_backup))
        // ── Editorial Studio API (/api/v1/admin/editorial/*) ───────────────
        .route(
            "/api/v1/admin/editorial/session",
            get(api_controller::editorial_session),
        )
        .route(
            "/api/v1/admin/editorial/articles",
            get(api_controller::editorial_list_articles)
                .post(api_controller::editorial_save_article),
        )
        .route(
            "/api/v1/admin/editorial/articles/:id",
            get(api_controller::editorial_get_article)
                .delete(api_controller::editorial_delete_article),
        )
        .route(
            "/api/v1/admin/editorial/articles/:id/refresh-stats",
            post(api_controller::editorial_refresh_stats),
        )
        .route(
            "/api/v1/admin/editorial/articles/:id/revisions",
            get(api_controller::editorial_list_revisions),
        )
        .route(
            "/api/v1/admin/editorial/articles/:id/revisions/:rev_id/restore",
            post(api_controller::editorial_restore_revision),
        )
        .route(
            "/api/v1/admin/editorial/locks",
            get(api_controller::editorial_list_locks),
        )
        .route(
            "/api/v1/admin/editorial/articles/:id/lock",
            post(api_controller::editorial_acquire_lock)
                .delete(api_controller::editorial_release_lock),
        )
        .route(
            "/api/v1/admin/editorial/categories",
            get(api_controller::editorial_list_categories)
                .post(api_controller::editorial_create_category),
        )
        .route(
            "/api/v1/admin/editorial/categories/:id",
            put(api_controller::editorial_update_category)
                .delete(api_controller::editorial_delete_category),
        )
        .route(
            "/api/v1/admin/editorial/meta",
            get(api_controller::editorial_meta),
        )
        .route(
            "/api/v1/admin/editorial/issues",
            post(api_controller::editorial_upsert_issue),
        )
        .route(
            "/api/v1/admin/editorial/issues/:id",
            delete(api_controller::editorial_delete_issue),
        )
        .route(
            "/api/v1/admin/editorial/glossary",
            post(api_controller::editorial_upsert_glossary),
        )
        .route(
            "/api/v1/admin/editorial/glossary/:id",
            delete(api_controller::editorial_delete_glossary),
        )
        .route(
            "/api/v1/admin/editorial/tags",
            get(api_controller::editorial_list_tags),
        )
        .route(
            "/api/v1/admin/editorial/redirects",
            get(api_controller::editorial_list_redirects),
        )
        .route(
            "/api/v1/admin/editorial/redirects/:id",
            delete(api_controller::editorial_delete_redirect),
        )
        .route(
            "/api/v1/admin/editorial/media",
            get(api_controller::editorial_list_media),
        )
        .route(
            "/api/v1/admin/editorial/media/:id",
            delete(api_controller::editorial_delete_media),
        )
        .route(
            "/api/v1/admin/editorial/upload",
            post(api_controller::editorial_upload_image)
                .layer(DefaultBodyLimit::max(12 * 1024 * 1024)),
        )
        .route(
            "/api/v1/admin/editorial/fetch-remote-image",
            post(api_controller::editorial_fetch_remote_image),
        )
        .route(
            "/api/v1/admin/editorial/media/watermark-url",
            post(api_controller::editorial_watermark_url),
        )
        .route(
            "/api/v1/admin/editorial/settings",
            get(api_controller::editorial_get_settings)
                .post(api_controller::editorial_save_settings),
        )
        .route(
            "/api/v1/admin/editorial/social/share",
            post(api_controller::editorial_social_share),
        )
        .route(
            "/api/v1/admin/editorial/social/test-twitter",
            post(api_controller::editorial_test_twitter),
        )
        .route(
            "/api/v1/admin/editorial/social/test-telegram",
            post(api_controller::editorial_test_telegram),
        )
        .route(
            "/api/v1/admin/editorial/research/sources",
            get(api_controller::editorial_list_research_sources)
                .post(api_controller::editorial_create_research_source),
        )
        .route(
            "/api/v1/admin/editorial/research/sources/:id",
            put(api_controller::editorial_update_research_source)
                .delete(api_controller::editorial_delete_research_source),
        )
        .route(
            "/api/v1/admin/editorial/research/search",
            get(api_controller::editorial_research_search),
        )
        .route(
            "/api/v1/admin/editorial/research/read",
            post(api_controller::editorial_research_read),
        )
}