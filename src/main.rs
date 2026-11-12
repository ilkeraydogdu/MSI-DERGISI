#[allow(unused_imports)]
use msi_dergisi_platform::{config, controllers, db, engine, models, services};

use axum::{
    extract::{DefaultBodyLimit, Request, State},
    http::{
        header::{HeaderName, AUTHORIZATION, CONTENT_TYPE, COOKIE},
        HeaderValue, Method, StatusCode,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    services::ServeDir,
};
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::config::AppConfig;
use crate::controllers::{api_controller, web_controller, AppState};
use crate::db::Database;
use crate::engine::security_engine::CentralSecurityFilter;

async fn central_waf_and_telemetry_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let t0 = Instant::now();

    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .or_else(|| req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("127.0.0.1")
        .trim()
        .to_string();

    let path = req.uri().path().to_string();
    let query_opt = req.uri().query().map(|q| q.to_string());
    let ua_opt = req
        .headers()
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Deep WAF inspection (Rate Limiter + SQLi + XSS + Path Traversal)
    if let Err(log_entry) = CentralSecurityFilter::inspect_request_security(
        &path,
        query_opt.as_deref(),
        ua_opt.as_deref(),
        &client_ip,
    ) {
        let _ = state.db.insert_security_audit_log(&log_entry);
        let status = if log_entry.event_type == "RATE_LIMIT_EXCEEDED" {
            StatusCode::TOO_MANY_REQUESTS
        } else {
            StatusCode::FORBIDDEN
        };
        return (
            status,
            Json(json!({
                "status": "blocked_by_waf",
                "engine": "MSI-Rust-WAF-v3.0",
                "event_type": log_entry.event_type,
                "threat_level": log_entry.threat_level,
                "details": log_entry.details
            })),
        )
            .into_response();
    }

    // Operator tools and sensitive analytics/automation routes require a signed operator session.
    let requires_operator = path == "/admin"
        || path == "/yonetim"
        || path.starts_with("/admin/")
        || path.starts_with("/api/v1/admin/")
        || path == "/api/v1/analytics/sponsor-report"
        || path == "/api/v1/subscriptions/trigger-reminders"
        || path == "/api/v1/ads/check-conflict";

    if requires_operator {
        let user = crate::services::auth_service::session_user(&state, req.headers());
        if !user
            .as_ref()
            .map(|u| ["site_operator", "admin"].contains(&u.role.as_str()))
            .unwrap_or(false)
        {
            if path.starts_with("/api/") || req.method() != axum::http::Method::GET {
                return crate::engine::api_response::AppError::Unauthorized {
                    msg_key: crate::engine::api_response::msg_keys::AUTH_REQUIRED,
                }
                .into_response();
            }
            return axum::response::Redirect::to(&format!(
                "/hesabim?next={}",
                urlencoding::encode(&path)
            ))
            .into_response();
        }
    }
    let mut response = next.run(req).await;
    let exec_us = t0.elapsed().as_micros();

    for (k, v) in CentralSecurityFilter::security_headers_list(exec_us) {
        if let (Ok(h_name), Ok(h_val)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_str(&v),
        ) {
            response.headers_mut().insert(h_name, h_val);
        }
    }
    let dur_ms = (exec_us as f64) / 1000.0;
    if let Ok(st_val) = HeaderValue::from_str(&format!("msi_ssr;dur={:.2};desc=\"MSI Rust SSR\"", dur_ms)) {
        response.headers_mut().insert(HeaderName::from_static("server-timing"), st_val);
    }
    if !path.starts_with("/api/") && !path.starts_with("/images/") && !path.starts_with("/css/") && !path.starts_with("/js/") {
        if let Ok(link_val) = HeaderValue::from_str("</llms.txt>; rel=\"describedby\"; type=\"text/plain\", </feed.json>; rel=\"alternate\"; type=\"application/feed+json\"") {
            response.headers_mut().insert(HeaderName::from_static("link"), link_val);
        }
    }

    response
}

fn spawn_renewal_reminder_worker(state: AppState, cancel: CancellationToken) {
    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            _ = cancel.cancelled() => {
                info!("Renewal reminder worker cancelled during startup delay");
                return;
            }
        }
        let mut interval = tokio::time::interval(Duration::from_secs(6 * 3600));
        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = cancel.cancelled() => {
                    info!("Renewal reminder worker shutting down gracefully");
                    return;
                }
            }
            let users = state.db.get_all_users();
            let existing = state.db.get_renewal_reminders();
            let evaluated =
                crate::services::subscription_service::evaluate_5_stage_reminders(&users, &existing);
            let _ = state.db.sync_evaluated_reminders(&evaluated);

            if state.config.mail_delivery_enabled {
                for reminder in evaluated.iter().filter(|r| r.status == "scheduled") {
                    let _ = crate::services::email_service::dispatch_renewal_reminder_email(
                        &state.config,
                        reminder,
                    )
                    .await;
                }
            }
        }
    });
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
        }
        _ = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown...");
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = AppConfig::from_env();
    cfg.validate_production_secrets();
    info!(
        "Starting {} on {}:{} (Domains: {} / {}, StaticDir: {})",
        cfg.app_name, cfg.host, cfg.port, cfg.primary_domain, cfg.news_domain, cfg.static_dir
    );

    let database = Database::new(&cfg.sqlite_db_path)
        .expect("Failed to initialize MSI Dergisi SQLite WAL database");

    // Ensure configured operator account password hash is Argon2id-synced with ADMIN_DEFAULT_PASS.
    if let Some((_, existing_hash)) = database.get_user_by_email(&cfg.admin_default_user) {
        if !crate::services::auth_service::verify_password(
            &cfg.admin_default_pass,
            &cfg.session_secret,
            &existing_hash,
        ) || !crate::services::auth_service::is_argon2_hash(&existing_hash)
        {
            let argon_hash = crate::services::auth_service::hash_password(
                &cfg.admin_default_pass,
                &cfg.session_secret,
            );
            let _ = database.update_user_password_hash(&cfg.admin_default_user, &argon_hash);
        }
    }

    // Initialize translation engine: seed defaults if empty, then load into memory cache
    {
        let conn = database.open_conn().expect("Failed to open DB for translation seeding");
        let _ = crate::engine::translation_engine::seed_default_translations(&conn);
    }
    let translations = crate::engine::translation_engine::TranslationCache::new();
    translations.load(database.load_all_translations());

    let state = AppState {
        config: Arc::new(cfg.clone()),
        db: database,
        translations,
    };

    let shutdown_token = CancellationToken::new();
    spawn_renewal_reminder_worker(state.clone(), shutdown_token.clone());

    let base_dir = cfg.static_dir.trim_end_matches('/');

    // SECURITY FIX (H4): Only include http://host:port dev origin in non-production environments.
    // In production with allow_credentials(true), exposing a dev origin enables CSRF attacks.
    let is_production = cfg.app_env == "production" || cfg.app_env == "prod";
    let mut origin_strings: Vec<String> = vec![
        cfg.public_url.trim_end_matches('/').to_string(),
        format!("https://{}", cfg.primary_domain),
        format!("https://www.{}", cfg.primary_domain),
        format!("https://{}", cfg.news_domain),
        format!("https://www.{}", cfg.news_domain),
    ];
    if !is_production {
        origin_strings.push(format!("http://{}:{}", cfg.host, cfg.port));
    }
    let allowed_origins: Vec<HeaderValue> = origin_strings
        .into_iter()
        .filter_map(|o| o.parse::<HeaderValue>().ok())
        .collect();

    let app = Router::new()
        // Public SSR Pages (100% Pure Rust SSR via Central Design & SEO Engines)
        .route("/", get(web_controller::render_home))
        .route("/index.html", get(web_controller::render_home))
        .route("/en", get(web_controller::redirect_en_home))
        .route("/arama", get(web_controller::render_search_page))
        .route("/hesabim", get(web_controller::render_account_page))
        .route("/sitemap.xml", get(web_controller::render_sitemap_xml))
        .route(
            "/sitemap-news.xml",
            get(web_controller::render_news_sitemap_xml),
        )
        .route(
            "/sitemap-images.xml",
            get(web_controller::render_image_sitemap_xml),
        )
        .route("/robots.txt", get(web_controller::render_robots_txt))
        .route("/rss.xml", get(web_controller::render_rss_xml))
        .route("/feed.json", get(web_controller::render_json_feed))
        .route("/llms.txt", get(web_controller::render_llms_txt))
        .route("/llms-full.txt", get(web_controller::render_llms_full_txt))
        .route(
            "/.well-known/ai-plugin.json",
            get(web_controller::render_ai_plugin_json),
        )
        .route("/opensearch.xml", get(web_controller::render_opensearch_xml))
        .route(
            "/manifest.webmanifest",
            get(web_controller::render_web_manifest),
        )
        .route("/favicon.ico", get(web_controller::render_favicon_ico))
        .route("/favicon.svg", get(web_controller::render_favicon_svg))
        .route(
            "/apple-touch-icon.png",
            get(web_controller::render_apple_touch_icon),
        )
        .route("/indexnow.txt", get(web_controller::render_indexnow_key))
        .route(
            "/api/v1/admin/seo/indexnow-ping",
            post(api_controller::admin_ping_indexnow),
        )
        .route("/admin", get(web_controller::render_admin))
        .route("/admin/tasarim", get(crate::engine::design::editor::view))
        .route(
            "/admin/tasarim/onizleme",
            post(crate::engine::design::editor::preview),
        )
        .route(
            "/api/v1/admin/design",
            get(crate::engine::design::editor::get).post(crate::engine::design::editor::save),
        )
        .route("/sayfa/:slug", get(web_controller::render_designed_page))
        .route("/yonetim", get(web_controller::render_admin))
        .route("/icerik/:slug", get(web_controller::render_article_detail))
        .route("/yazar/:slug", get(web_controller::render_author_profile))
        .route(
            "/haberler/:slug",
            get(web_controller::redirect_legacy_haber_tr),
        )
        .route(
            "/en/haberler/:slug",
            get(web_controller::redirect_legacy_haber_en),
        )
        .route(
            "/kategori/:slug",
            get(web_controller::render_category_page),
        )
        .route(
            "/kategoriler/:slug",
            get(web_controller::redirect_legacy_kategori_tr),
        )
        .route(
            "/en/kategoriler/:slug",
            get(web_controller::redirect_legacy_kategori_en),
        )
        .route("/coming-soon", get(web_controller::render_coming_soon))
        .route("/yakinda", get(web_controller::render_coming_soon))
        .route("/e-dergi", get(web_controller::render_edergi_kiosk))
        .route(
            "/abonelik-ve-satis",
            get(web_controller::render_subscriptions_and_store),
        )
        .route(
            "/kurumsal/:slug",
            get(web_controller::render_corporate_page),
        )
        // Public REST API (/api/v1/*)
        .route("/api/v1/health", get(api_controller::health_check))
        .route("/api/v1/search", get(api_controller::faceted_search_api))
        .route(
            "/api/v1/notifications",
            get(api_controller::list_notifications_api),
        )
        .route(
            "/api/v1/notifications/:id/read",
            post(api_controller::mark_notification_read_api),
        )
        .route(
            "/api/v1/settings",
            get(api_controller::list_site_settings_api),
        )
        .route("/api/v1/contents", get(api_controller::list_contents))
        .route(
            "/api/v1/contents/:slug",
            get(api_controller::get_content_detail),
        )
        .route("/api/v1/categories", get(api_controller::list_categories))
        .route("/api/v1/ssb-tags", get(api_controller::list_ssb_tags))
        .route("/api/v1/issues", get(api_controller::list_issues))
        .route("/api/v1/ads", get(api_controller::list_ads))
        .route(
            "/api/v1/ads/check-conflict",
            post(api_controller::admin_check_ad_conflict),
        )
        .route(
            "/api/v1/edergi/:issue_no/download-pdf",
            get(api_controller::download_edergi_pdf),
        )
        .route("/api/v1/auth/register", post(api_controller::register_user))
        .route("/api/v1/auth/login", post(api_controller::login_user))
        .route("/api/v1/auth/logout", post(api_controller::logout_user))
        .route(
            "/api/v1/analytics/track",
            post(api_controller::track_analytics_event),
        )
        .route(
            "/api/v1/analytics/sponsor-report",
            get(api_controller::admin_analytics_bundle),
        )
        .route(
            "/api/v1/commerce/catalog",
            get(api_controller::get_commerce_catalog),
        )
        .route(
            "/api/v1/subscriptions/checkout",
            post(api_controller::checkout_order),
        )
        .route(
            "/api/v1/payments/paytr-callback",
            post(api_controller::paytr_callback),
        )
        .route(
            "/api/v1/subscriptions/trigger-reminders",
            post(api_controller::admin_halt_renewed_reminders),
        )
        // Admin OMS/CMS REST API (/api/v1/admin/*)
        .route(
            "/api/v1/admin/overview",
            get(api_controller::admin_overview),
        )
        .route(
            "/api/v1/admin/upload",
            post(api_controller::admin_upload_media)
                .layer(DefaultBodyLimit::max(25 * 1024 * 1024)),
        )
        .route(
            "/api/v1/admin/notifications",
            get(api_controller::list_notifications_api)
                .post(api_controller::admin_create_notification_api),
        )
        .route(
            "/api/v1/admin/settings",
            get(api_controller::list_site_settings_api)
                .post(api_controller::admin_upsert_setting_api),
        )
        .route(
            "/api/v1/admin/security-logs",
            get(api_controller::admin_list_security_logs_api),
        )
        .route(
            "/api/v1/admin/contents",
            get(api_controller::list_contents).post(api_controller::admin_upsert_content),
        )
        .route(
            "/api/v1/admin/contents/:id",
            delete(api_controller::admin_delete_content),
        )
        .route(
            "/api/v1/admin/ads",
            get(api_controller::list_ads).post(api_controller::admin_upsert_ad),
        )
        .route(
            "/api/v1/admin/ads/check-conflict",
            post(api_controller::admin_check_ad_conflict),
        )
        .route(
            "/api/v1/admin/analytics",
            get(api_controller::admin_analytics_bundle),
        )
        .route(
            "/api/v1/admin/analytics/bundle",
            get(api_controller::admin_analytics_bundle),
        )
        .route(
            "/api/v1/admin/analytics/sponsor-report",
            get(api_controller::admin_analytics_bundle),
        )
        .route(
            "/api/v1/admin/analytics/contents",
            get(api_controller::admin_analytics_contents),
        )
        .route(
            "/api/v1/admin/analytics/ads",
            get(api_controller::admin_analytics_ads),
        )
        .route(
            "/api/v1/admin/analytics/readers",
            get(api_controller::admin_analytics_readers),
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
            get(api_controller::list_categories).post(api_controller::admin_insert_category),
        )
        .route(
            "/api/v1/admin/ssb-tags",
            get(api_controller::list_ssb_tags).post(api_controller::admin_insert_ssb_tag),
        )
        .route(
            "/api/v1/admin/newsletters",
            get(api_controller::admin_list_newsletters)
                .post(api_controller::admin_insert_newsletter),
        )
        .route(
            "/api/v1/admin/reminders",
            get(api_controller::admin_list_reminders),
        )
        .route(
            "/api/v1/admin/reminders/halt-renewed",
            post(api_controller::admin_halt_renewed_reminders),
        )
        .route(
            "/api/v1/admin/redirects",
            get(api_controller::admin_list_redirects).post(api_controller::admin_insert_redirect),
        )
        .route(
            "/api/v1/admin/export/open-data",
            get(api_controller::admin_export_open_data),
        )
        .route(
            "/api/v1/admin/translations",
            get(api_controller::admin_list_translations_api)
                .post(api_controller::admin_upsert_translation_api),
        )
        .route(
            "/api/v1/admin/translations/:key",
            delete(api_controller::admin_delete_translation_api),
        )
        // Advanced Admin: Content Version History
        .route(
            "/api/v1/admin/contents/:id/versions",
            get(api_controller::admin_list_content_versions)
                .post(api_controller::admin_create_content_version),
        )
        // Advanced Admin: Media Library
        .route(
            "/api/v1/admin/media-library",
            get(api_controller::admin_list_media_library),
        )
        // Advanced Admin: Audit Log
        .route(
            "/api/v1/admin/audit-logs",
            get(api_controller::admin_list_audit_logs)
                .post(api_controller::admin_insert_audit_log),
        )
        // Advanced Admin: User Roles & Permissions
        .route(
            "/api/v1/admin/user-roles",
            get(api_controller::admin_list_user_roles)
                .post(api_controller::admin_upsert_user_role),
        )
        // Advanced Admin: Custom Fields (Dynamic Form Builder)
        .route(
            "/api/v1/admin/custom-fields",
            get(api_controller::admin_list_custom_fields)
                .post(api_controller::admin_upsert_custom_field),
        )
        .route(
            "/api/v1/admin/custom-fields/:field_key",
            delete(api_controller::admin_delete_custom_field),
        )
        // Advanced Admin: Cache Management
        .route(
            "/api/v1/admin/cache",
            get(api_controller::admin_list_cache_operations)
                .post(api_controller::admin_flush_cache),
        )
        // Advanced Admin: System Health Monitoring
        .route(
            "/api/v1/admin/system-health",
            get(api_controller::admin_system_health),
        )
        // Advanced Admin: Bulk Content Operations
        .route(
            "/api/v1/admin/contents/bulk",
            post(api_controller::admin_bulk_content_operation),
        )
        // Static Asset Directories
        .nest_service("/css", ServeDir::new(format!("{}/css", base_dir)))
        .nest_service("/js", ServeDir::new(format!("{}/js", base_dir)))
        .nest_service("/images", ServeDir::new(format!("{}/images", base_dir)))
        .nest_service("/fonts", ServeDir::new(format!("{}/fonts", base_dir)))
        .nest_service("/scss", ServeDir::new(format!("{}/scss", base_dir)))
        // Fallback: 301 SEO Redirects, legacy .html pages, direct category/article slugs
        .fallback(web_controller::handle_fallback_or_redirect)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            central_waf_and_telemetry_middleware,
        ))
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(allowed_origins)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([CONTENT_TYPE, AUTHORIZATION, COOKIE])
                .allow_credentials(true),
        )
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port)
        .parse()
        .expect("Invalid bind address");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    info!(
        "MSI Dergisi & MSI TDR Rust Fullstack Server listening on http://{}",
        addr
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    // Cancel background workers after HTTP server has stopped accepting connections
    shutdown_token.cancel();
    info!("All background workers cancelled, shutdown complete");
}
