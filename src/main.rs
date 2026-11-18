#[allow(unused_imports)]
use msi_dergisi_platform::{
    config, controllers, db, engine, extractors, middleware, models, routes, services, utils,
    workers,
};

use axum::http::{
    header::{HeaderName, AUTHORIZATION, CONTENT_TYPE, COOKIE},
    HeaderValue, Method,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tower_http::{compression::CompressionLayer, cors::CorsLayer};
use tracing::{info, warn};

use crate::config::AppConfig;
use crate::controllers::AppState;
use crate::db::Database;

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received SIGINT (Ctrl+C), initiating graceful shutdown..."),
        _ = terminate => info!("Received SIGTERM, initiating graceful shutdown..."),
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
            database.refresh_l1_sections(&["users"]);
        }
    }

    let _ = crate::engine::translation_engine::seed_default_translations(&database);
    let translations = crate::engine::translation_engine::TranslationCache::new();
    translations.load(database.load_all_translations());

    let monitoring_cache = if cfg.monitoring_enabled {
        Some(
            crate::services::monitoring_service::init_monitoring(
                &cfg.monitoring_vhosts_path,
                &cfg.monitoring_nginx_log_path,
            )
            .await,
        )
    } else {
        None
    };

    let state = AppState {
        config: Arc::new(cfg.clone()),
        db: database,
        translations,
        monitoring_cache,
    };

    let shutdown_token = CancellationToken::new();
    crate::workers::spawn_renewal_reminder_worker(state.clone(), shutdown_token.clone());
    crate::workers::spawn_analytics_aggregation_worker(state.clone(), shutdown_token.clone());
    crate::workers::spawn_monitoring_worker(state.clone(), shutdown_token.clone());

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

    let app = crate::routes::build_router(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::central_waf_and_telemetry_middleware,
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
                .allow_headers([
                    CONTENT_TYPE,
                    AUTHORIZATION,
                    COOKIE,
                    HeaderName::from_static("x-csrf-token"),
                ])
                .allow_credentials(true),
        );

    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port)
        .parse()
        .expect("Invalid bind address");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    info!(
        "MSI Dergisi & MSI TDR Rust Fullstack Server listening on http://{} (command gateway: /{})",
        addr, "[REDACTED]"
    );
    if let Err(e) = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        warn!("Server error: {}", e);
    }

    shutdown_token.cancel();
    info!("All background workers cancelled, shutdown complete");
}
