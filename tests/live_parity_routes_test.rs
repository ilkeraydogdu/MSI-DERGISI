//! Integration tests for live msidergisi.com parity routes (EN home, search, locale switch, corporate shortcuts, issue 404).

mod common;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use tower::ServiceExt;

use common::*;
use msi_dergisi_platform::routes;

async fn get_resp(app: &axum::Router, uri: &str) -> (StatusCode, axum::http::HeaderMap, String) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024).await.unwrap();
    (status, headers, String::from_utf8_lossy(&bytes).to_string())
}

#[tokio::test]
async fn live_parity_routes_respond_correctly() {
    let (state, db_path) = build_test_state("live_parity_routes");
    let app = routes::build_router(state.clone()).layer(
        axum::middleware::from_fn_with_state(
            state.clone(),
            msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
        ),
    );

    let (st, _, tr_html) = get_resp(&app, "/").await;
    assert_eq!(st, StatusCode::OK);
    assert!(tr_html.contains("MSI"));

    let (st, _, en_html) = get_resp(&app, "/en").await;
    assert_eq!(st, StatusCode::OK);
    assert!(en_html.contains("lang=\"en\""));

    let (st, _, _) = get_resp(&app, "/search").await;
    assert_eq!(st, StatusCode::OK);

    let (st, _, _) = get_resp(&app, "/en/search").await;
    assert_eq!(st, StatusCode::OK);

    let (st, hdrs, _) = get_resp(&app, "/switch/en").await;
    assert_eq!(st, StatusCode::FOUND);
    assert_eq!(hdrs.get(header::LOCATION).unwrap(), "/en");
    let cookie = hdrs.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert!(cookie.contains("msi_lang=en"));

    // Corporate short URLs return clean permanent redirects (Axum Redirect::permanent -> 308 / 301)
    let (st, hdrs, _) = get_resp(&app, "/kunye").await;
    assert!(st == StatusCode::MOVED_PERMANENTLY || st == StatusCode::PERMANENT_REDIRECT);
    assert_eq!(hdrs.get(header::LOCATION).unwrap(), "/kurumsal/kunye");

    let (st, hdrs, _) = get_resp(&app, "/en/about-us").await;
    assert!(st == StatusCode::MOVED_PERMANENTLY || st == StatusCode::PERMANENT_REDIRECT);
    assert_eq!(hdrs.get(header::LOCATION).unwrap(), "/kurumsal/hakkimizda?lang=en");

    let (st, _, yol_html) = get_resp(&app, "/kurumsal/yolculugumuz").await;
    assert_eq!(st, StatusCode::OK);
    assert!(yol_html.contains("2004"));

    // Non-existent issue returns 404 instead of falling back to another issue
    let (st, _, _) = get_resp(&app, "/e-dergi?issue=999999").await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Insert an msi_tdr issue and verify ?publication_type= filtering & 404 validation
    let tdr_issue = msi_dergisi_platform::models::MagazineIssue {
        issue_number: 501,
        slug: "msi-tdr-issue-501".into(),
        title_tr: "MSI TDR Özel Sayı 501".into(),
        title_en: "MSI TDR Special Issue 501".into(),
        publish_month_tr: "Kasım".into(),
        publish_month_en: "November".into(),
        year: 2026,
        cover_image: msi_dergisi_platform::db::constants::DEFAULT_ISSUE_COVER_PATH.into(),
        page_count: 64,
        dossier_count: 6,
        summary_tr: "MSI TDR 501 özeti".into(),
        summary_en: "MSI TDR 501 summary".into(),
        publication_type: "msi_tdr".into(),
        ..Default::default()
    };
    state.db.upsert_issue(&tdr_issue).unwrap();

    // /e-dergi?publication_type=msi_tdr renders only msi_tdr issue cards (excludes msi issue cards)
    let (st, _, all_edergi_html) = get_resp(&app, "/e-dergi").await;
    assert_eq!(st, StatusCode::OK);
    assert!(all_edergi_html.contains("<article class=\"issue-card\" data-publication-type=\"msi\">"));

    let (st, _, tdr_html) = get_resp(&app, "/e-dergi?publication_type=msi_tdr").await;
    assert_eq!(st, StatusCode::OK);
    assert!(tdr_html.contains("<article class=\"issue-card\" data-publication-type=\"msi_tdr\">"));
    assert!(tdr_html.contains("MSI TDR Özel Sayı 501"));
    assert!(!tdr_html.contains("<article class=\"issue-card\" data-publication-type=\"msi\">"));
    let grid_slice = tdr_html
        .split("<div class=\"issues-grid\">")
        .nth(1)
        .and_then(|s| s.split("</div>").next())
        .unwrap_or("");
    assert!(grid_slice.contains("MSI TDR Özel Sayı 501"));
    assert!(!grid_slice.contains("MSI Dergisi • Sayı 286"));

    // /e-dergi?issue=286&publication_type=msi_tdr returns 404 because 286 is msi-only
    let (st, _, _) = get_resp(&app, "/e-dergi?issue=286&publication_type=msi_tdr").await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // /e-dergi?issue=501&publication_type=msi_tdr returns 200
    let (st, _, tdr_501_html) = get_resp(&app, "/e-dergi?issue=501&publication_type=msi_tdr").await;
    assert_eq!(st, StatusCode::OK);
    assert!(tdr_501_html.contains("MSI TDR Özel Sayı 501"));

    // /e-dergi?publication_type=bogus returns 404
    let (st, _, _) = get_resp(&app, "/e-dergi?publication_type=bogus").await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn default_podmin_admin_gateway_mounts_without_panic_and_disables_legacy_slug() {
    let db_path = test_db_path("podmin_default_gateway");
    let mut cfg = test_config(&db_path);
    cfg.admin_path_secret = "podmin".into();
    let database = msi_dergisi_platform::db::Database::new(&db_path).expect("db init");
    let translations = msi_dergisi_platform::engine::translation_engine::TranslationCache::new();
    translations.load(database.load_all_translations());
    let state = msi_dergisi_platform::controllers::AppState {
        config: std::sync::Arc::new(cfg),
        db: database,
        translations,
        monitoring_cache: None,
    };

    let app = routes::build_router(state.clone()).layer(
        axum::middleware::from_fn_with_state(
            state.clone(),
            msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
        ),
    );

    // 1. /podmin unauthenticated renders login gateway with data-admin-path="/podmin"
    let (st, _, login_html) = get_resp(&app, "/podmin").await;
    assert_eq!(st, StatusCode::OK);
    assert!(login_html.contains(r#"data-admin-path="/podmin""#));
    assert!(login_html.contains(r#"id="login-form""#));
    assert!(login_html.contains(r#"id="login-stage""#));

    // 2. /msi-komuta is no longer mounted when admin_path_secret == "podmin"
    let (st_old, _, _) = get_resp(&app, "/msi-komuta").await;
    assert_eq!(st_old, StatusCode::NOT_FOUND);

    // 3. Authenticated super_admin can open /podmin and /podmin/tasarim
    let (_, admin_cookie) = create_user_with_session(
        &state,
        "podmin_sa@test.local",
        "PodminPass!2026",
        "Podmin Admin",
        "super_admin",
        "MSB",
        "UMSA",
        true,
        "kurumsal",
    );
    let studio_req = Request::builder()
        .method("GET")
        .uri("/podmin/tasarim")
        .header(header::COOKIE, admin_cookie)
        .body(Body::empty())
        .unwrap();
    let studio_resp = app.clone().oneshot(studio_req).await.unwrap();
    assert_eq!(studio_resp.status(), StatusCode::OK);
    let studio_bytes = to_bytes(studio_resp.into_body(), 2 * 1024 * 1024).await.unwrap();
    let studio_html = String::from_utf8_lossy(&studio_bytes);
    assert!(studio_html.contains(r#"data-admin-path="/podmin""#));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn default_podmin_gateway_serves_podmin_and_studio_while_blocking_legacy_slug() {
    let db_path = test_db_path("podmin_default_gateway");
    let mut cfg = test_config(&db_path);
    cfg.admin_path_secret = "podmin".to_string();
    let database = msi_dergisi_platform::db::Database::new(&db_path).expect("test db");
    let translations = msi_dergisi_platform::engine::translation_engine::TranslationCache::new();
    translations.load(database.load_all_translations());
    let state = msi_dergisi_platform::controllers::AppState {
        config: std::sync::Arc::new(cfg),
        db: database,
        translations,
        monitoring_cache: None,
    };

    let app = routes::build_router(state.clone()).layer(
        axum::middleware::from_fn_with_state(
            state.clone(),
            msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
        ),
    );

    // 1. Unauthenticated GET /podmin -> 200 Login Gateway with data-admin-path="/podmin"
    let (st, _, login_html) = get_resp(&app, "/podmin").await;
    assert_eq!(st, StatusCode::OK);
    assert!(login_html.contains("data-admin-path=\"/podmin\""));
    assert!(login_html.contains("id=\"login-form\""));
    assert!(login_html.contains(r#"id="login-stage""#));
    assert!(login_html.contains("/images/admin/stage/01-f4e"));

    // 2. Legacy /msi-komuta must return 404 when ADMIN_PATH_SECRET=podmin
    let (st_old, _, _) = get_resp(&app, "/msi-komuta").await;
    assert_eq!(st_old, StatusCode::NOT_FOUND);

    // 3. Authenticated super_admin GET /podmin & /podmin/tasarim -> 200
    let (_, sa_cookie) = create_user_with_session(
        &state,
        "podmin_sa@test.msidergisi.com",
        "TestPass2026!",
        "Podmin Test Admin",
        "super_admin",
        "MSB",
        "UMSA",
        true,
        "kurumsal",
    );

    let req_dash = Request::builder()
        .method("GET")
        .uri("/podmin")
        .header(header::COOKIE, &sa_cookie)
        .body(Body::empty())
        .unwrap();
    let resp_dash = app.clone().oneshot(req_dash).await.unwrap();
    assert_eq!(resp_dash.status(), StatusCode::OK);
    let dash_bytes = to_bytes(resp_dash.into_body(), 4 * 1024 * 1024).await.unwrap();
    let dash_html = String::from_utf8_lossy(&dash_bytes);
    assert!(dash_html.contains("data-admin-path=\"/podmin\""));
    assert!(dash_html.contains("href=\"/podmin/tasarim\""));

    let req_studio = Request::builder()
        .method("GET")
        .uri("/podmin/tasarim")
        .header(header::COOKIE, &sa_cookie)
        .body(Body::empty())
        .unwrap();
    let resp_studio = app.clone().oneshot(req_studio).await.unwrap();
    assert_eq!(resp_studio.status(), StatusCode::OK);
    let studio_bytes = to_bytes(resp_studio.into_body(), 4 * 1024 * 1024).await.unwrap();
    let studio_html = String::from_utf8_lossy(&studio_bytes);
    assert!(studio_html.contains("data-admin-path=\"/podmin\""));
    assert!(studio_html.contains("href=\"/podmin\""));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn retired_msi_komuta_config_is_normalized_to_podmin_and_returns_404() {
    let db_path = test_db_path("retired_msi_komuta_guard");
    let mut cfg = test_config(&db_path);
    cfg.admin_path_secret = "msi-komuta".into();
    let database = msi_dergisi_platform::db::Database::new(&db_path).expect("test db");
    let translations = msi_dergisi_platform::engine::translation_engine::TranslationCache::new();
    translations.load(database.load_all_translations());
    let state = msi_dergisi_platform::controllers::AppState {
        config: std::sync::Arc::new(cfg),
        db: database,
        translations,
        monitoring_cache: None,
    };

    let app = routes::build_router(state.clone()).layer(
        axum::middleware::from_fn_with_state(
            state.clone(),
            msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
        ),
    );

    // Even if AppConfig.admin_path_secret is set to "msi-komuta", /msi-komuta and subpages MUST return 404
    let (st_old, _, _) = get_resp(&app, "/msi-komuta").await;
    assert_eq!(st_old, StatusCode::NOT_FOUND);
    let (st_old_studio, _, _) = get_resp(&app, "/msi-komuta/tasarim").await;
    assert_eq!(st_old_studio, StatusCode::NOT_FOUND);

    // Canonical /podmin still serves 200 and binds data-admin-path="/podmin"
    let (st_podmin, _, login_html) = get_resp(&app, "/podmin").await;
    assert_eq!(st_podmin, StatusCode::OK);
    assert!(login_html.contains(r#"data-admin-path="/podmin""#));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn slash_and_case_admin_path_secrets_normalize_to_podmin_without_panic() {
    for (suffix, secret) in [
        ("msi_komuta_slash", "msi-komuta/tasarim"),
        ("podmin_slash", "podmin/tasarim"),
        ("podmin_upper", "PODMIN"),
    ] {
        let db_path = test_db_path(&format!("admin_secret_edge_{}", suffix));
        let mut cfg = test_config(&db_path);
        cfg.admin_path_secret = secret.into();
        let database = msi_dergisi_platform::db::Database::new(&db_path).expect("test db");
        let translations =
            msi_dergisi_platform::engine::translation_engine::TranslationCache::new();
        translations.load(database.load_all_translations());
        let state = msi_dergisi_platform::controllers::AppState {
            config: std::sync::Arc::new(cfg),
            db: database,
            translations,
            monitoring_cache: None,
        };

        let app = routes::build_router(state.clone()).layer(
            axum::middleware::from_fn_with_state(
                state.clone(),
                msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
            ),
        );

        let (st_old, _, _) = get_resp(&app, "/msi-komuta").await;
        assert_eq!(st_old, StatusCode::NOT_FOUND);
        let (st_old_studio, _, _) = get_resp(&app, "/msi-komuta/tasarim").await;
        assert_eq!(st_old_studio, StatusCode::NOT_FOUND);

        let (st_podmin, _, login_html) = get_resp(&app, "/podmin").await;
        assert_eq!(
            st_podmin,
            StatusCode::OK,
            "Expected /podmin 200 for ADMIN_PATH_SECRET={:?}",
            secret
        );
        assert!(login_html.contains(r#"data-admin-path="/podmin""#));

        cleanup_test_db(&db_path);
    }
}
