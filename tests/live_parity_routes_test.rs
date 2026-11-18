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

    cleanup_test_db(&db_path);
}
