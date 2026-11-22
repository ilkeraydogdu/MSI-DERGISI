//! Tier 5 White-Box Adversarial & Security Regression Suite (M6 Phase 2)
//!
//! Covers all 11 remediated gaps from Iteration 1:
//! - GAP-01: Unverified self-profile `target_institutions` bypass
//! - GAP-02: CSRF enforcement on `/api/v1/auth/totp/*`, `/auth/logout`, `/account/*`, checkout & email-preferences
//! - GAP-03: TOTP setup GET removal, re-auth overwrite guard, single-use `temp_token` concurrency & replay guard
//! - GAP-04: X-Forwarded-For / X-Real-IP spoofing & forwarded-loopback rate-limit bypass
//! - GAP-05: Empty `msi_session=` cookie shadowing & logout revocation
//! - GAP-06: Law No. 6502 `distance_sales_consent` on `/api/v1/subscriptions/checkout`
//! - GAP-07: Synthetic E-Dergi PDF OOB `?pages=9999` rejection & staff email-verification parity
//! - GAP-09: `/hesabim` Design Studio link permission scoping
//! - GAP-10: Admin `#tab-17` ("Sunucu İzleme") permission isolation & `monitoring_enabled` flag
//! - GAP-12: Verified `.gov.tr` & `subscriber_corporate` `target_institutions` immutability on `/api/v1/account/profile`
//! - GAP-13: Cross-`temp_token` concurrent single-use TOTP backup code replay serialization
//! - GAP-14: TOTP disable + immediate re-enrollment within the same 30s timestep
//! - GAP-15: CSRF enforcement on `POST /api/v1/notifications/:id/read`

mod common;

use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower::ServiceExt;

use common::*;
use msi_dergisi_platform::{
    controllers::AppState,
    engine::security_engine,
    models::{MagazineIssue, SystemNotification},
    services::auth_service,
};

fn build_totp_code(secret_b32: &str) -> String {
    let secret = totp_rs::Secret::Encoded(secret_b32.to_string())
        .to_bytes()
        .expect("valid base32 secret");
    let totp = totp_rs::TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some("MSI Dergisi".to_string()),
        "test@msidergisi.com".to_string(),
    )
    .expect("construct TOTP");
    totp.generate_current().expect("generate current TOTP")
}

async fn req_with_csrf(
    app: &axum::Router,
    state: &AppState,
    method: &str,
    uri: &str,
    session_cookie: Option<&str>,
    payload: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let csrf = security_engine::generate_csrf_token(&state.config.session_secret);
    let cookie = match session_cookie {
        Some(sc) => format!("{}; csrf_token={}", sc, csrf),
        None => format!("csrf_token={}", csrf),
    };
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .header("x-csrf-token", &csrf);
    let body = if let Some(p) = payload {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(p.to_string())
    } else {
        Body::empty()
    };
    let resp = app.clone().oneshot(builder.body(body).unwrap()).await.unwrap();
    let st = resp.status();
    let hdrs = resp.headers().clone();
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (st, hdrs, val)
}

async fn req_raw(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    extra_headers: &[(&str, &str)],
    payload: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    for (k, v) in extra_headers {
        builder = builder.header(*k, *v);
    }
    let body = if let Some(p) = payload {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(p.to_string())
    } else {
        Body::empty()
    };
    let resp = app.clone().oneshot(builder.body(body).unwrap()).await.unwrap();
    let st = resp.status();
    let hdrs = resp.headers().clone();
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024).await.unwrap();
    (st, hdrs, bytes.to_vec())
}

#[tokio::test]
async fn t5_target_institutions_blocks_unverified_self_profile_spoof() {
    let (state, db_path) = build_test_state("t5_gap01_inst");
    let app = build_test_router(&state);

    let mut kamu_item = fixture_content(7001);
    kamu_item.id = 0;
    kamu_item.slug = "t5-kamu-secret-dossier".into();
    kamu_item.visibility_level = 1;
    kamu_item.target_institutions = vec!["Kamu / TSK / SSB".into()];
    kamu_item.body_tr = "<p>Birinci cümle. İkinci cümle. Üçüncü cümle. SECRET_PARAGRAPH_FIVE_KAMU dördüncü cümle. Beşinci cümle.</p>".into();
    state.db.upsert_content(&kamu_item).unwrap();

    let mut hvkk_item = fixture_content(7002);
    hvkk_item.id = 0;
    hvkk_item.slug = "t5-hvkk-secret-dossier".into();
    hvkk_item.visibility_level = 0;
    hvkk_item.target_institutions = vec!["Hv.K.K.lığı".into()];
    hvkk_item.body_tr = "<p>Birinci cümle. İkinci cümle. Üçüncü cümle. SECRET_HVKK_TOKEN dördüncü cümle.</p>".into();
    state.db.upsert_content(&hvkk_item).unwrap();

    let (_, gmail_cookie) = create_user_with_session(
        &state,
        "attacker_t5@gmail.com",
        "StrongPass!2026",
        "Unverified Gmail",
        "registered",
        "Bireysel / Diğer",
        "Bireysel",
        false,
        "",
    );

    // 1. Before profile edit: locked
    let (_, _, pre_bytes) = req_raw(
        &app,
        "GET",
        "/api/v1/contents/t5-kamu-secret-dossier",
        Some(&gmail_cookie),
        &[],
        None,
    )
    .await;
    assert!(!String::from_utf8_lossy(&pre_bytes).contains("SECRET_PARAGRAPH_FIVE_KAMU"));

    // 2. Spoof organization_category & organization_name via /api/v1/account/profile
    let (st_prof, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/account/profile",
        Some(&gmail_cookie),
        Some(json!({
            "full_name": "Unverified Gmail",
            "organization_category": "Kamu / TSK / SSB",
            "organization_name": "SSB"
        })),
    )
    .await;
    assert_eq!(st_prof, StatusCode::OK);

    // 3. Re-fetching must remain locked for unverified gmail user
    let (st_after, _, after_bytes) = req_raw(
        &app,
        "GET",
        "/api/v1/contents/t5-kamu-secret-dossier",
        Some(&gmail_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_after, StatusCode::OK);
    let after_json: Value = serde_json::from_slice(&after_bytes).unwrap();
    assert_eq!(after_json["paywall"]["is_locked"], true);
    assert!(!String::from_utf8_lossy(&after_bytes).contains("SECRET_PARAGRAPH_FIVE_KAMU"));

    // Also test specific agency category spoof ("Hv.K.K.lığı")
    let _ = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/account/profile",
        Some(&gmail_cookie),
        Some(json!({
            "full_name": "Unverified Gmail",
            "organization_category": "Hv.K.K.lığı",
            "organization_name": "Hava Kuvvetleri Komutanlığı"
        })),
    )
    .await;
    let (_, _, hvkk_bytes) = req_raw(
        &app,
        "GET",
        "/api/v1/contents/t5-hvkk-secret-dossier",
        Some(&gmail_cookie),
        &[],
        None,
    )
    .await;
    assert!(!String::from_utf8_lossy(&hvkk_bytes).contains("SECRET_HVKK_TOKEN"));

    // 4. Genuine verified public-sector user CAN access
    let (_, gov_cookie) = create_user_with_session(
        &state,
        "subay_t5@hvkk.tsk.tr",
        "StrongPass!2026",
        "Verified Gov",
        "subscriber_public",
        "Hv.K.K.lığı",
        "Hv.K.K.lığı",
        true,
        "Kamu E-Dergi Abonesi",
    );
    let (_, _, gov_bytes) = req_raw(
        &app,
        "GET",
        "/api/v1/contents/t5-hvkk-secret-dossier",
        Some(&gov_cookie),
        &[],
        None,
    )
    .await;
    assert!(String::from_utf8_lossy(&gov_bytes).contains("SECRET_HVKK_TOKEN"));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_csrf_enforced_on_totp_logout_and_account_mutations() {
    let (state, db_path) = build_test_state("t5_gap02_csrf");
    let app = build_test_router(&state);

    let (user, session_cookie) = create_user_with_session(
        &state,
        "csrf_victim@example.com",
        "StrongPass!2026",
        "CSRF Victim",
        "registered",
        "Sektör Kuruluşları",
        "Test A.Ş.",
        false,
        "",
    );

    // Enable TOTP legitimately first
    let (st_setup, _, setup_val) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/setup",
        Some(&session_cookie),
        None,
    )
    .await;
    assert_eq!(st_setup, StatusCode::OK);
    let secret = setup_val["data"]["secret"].as_str().unwrap().to_string();
    let code = build_totp_code(&secret);
    let (st_ver, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/verify-setup",
        Some(&session_cookie),
        Some(json!({ "secret": secret, "code": code })),
    )
    .await;
    assert_eq!(st_ver, StatusCode::OK);

    // Cross-origin & tokenless attempts MUST return 403
    for (uri, headers_slice, payload) in [
        (
            "/api/v1/auth/totp/disable",
            vec![("origin", "https://evil.example")],
            Some(json!({ "password": "StrongPass!2026" })),
        ),
        (
            "/api/v1/auth/totp/disable",
            vec![],
            Some(json!({ "password": "StrongPass!2026" })),
        ),
        (
            "/api/v1/auth/totp/verify-setup",
            vec![],
            Some(json!({ "code": "123456" })),
        ),
        (
            "/api/v1/account/profile",
            vec![],
            Some(json!({ "full_name": "Hijacked" })),
        ),
        (
            "/api/v1/account/delete",
            vec![],
            Some(json!({ "password": "StrongPass!2026" })),
        ),
        ("/api/v1/auth/logout", vec![("origin", "https://evil.example")], None),
        ("/api/v1/auth/logout", vec![], None),
        (
            "/api/v1/subscriptions/checkout",
            vec![("origin", "https://evil.example")],
            Some(json!({ "email": "a@b.com", "full_name": "X", "item_type": "subscription", "item_code": "dijital", "distance_sales_consent": true })),
        ),
        (
            "/api/v1/commerce/cart-checkout",
            vec![("origin", "https://evil.example")],
            Some(json!({ "email": "a@b.com", "full_name": "X", "distance_sales_consent": true, "items": [] })),
        ),
        (
            "/api/v1/email-preferences",
            vec![("origin", "https://evil.example")],
            Some(json!({ "token": "bad", "newsletter_frequency": "weekly" })),
        ),
    ] {
        let (st, _, _) = req_raw(
            &app,
            "POST",
            uri,
            Some(&session_cookie),
            &headers_slice,
            payload,
        )
        .await;
        assert_eq!(
            st,
            StatusCode::FORBIDDEN,
            "Expected 403 CSRF rejection on {} with headers {:?}",
            uri,
            headers_slice
        );
    }

    // Form-urlencoded cross-site logout without CSRF also rejected & session stays alive
    let form_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/logout")
        .header(header::COOKIE, &session_cookie)
        .header(header::ORIGIN, "https://evil.example")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::empty())
        .unwrap();
    let form_resp = app.clone().oneshot(form_req).await.unwrap();
    assert_eq!(form_resp.status(), StatusCode::FORBIDDEN);

    let (st_still_alive, _, _) = req_raw(
        &app,
        "GET",
        "/api/v1/account/export",
        Some(&session_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_still_alive, StatusCode::OK);
    assert!(state.db.get_user_totp_secret(&user.email).is_some());

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_totp_overwrite_requires_reauth_and_single_use_temp_token_concurrency() {
    let (state, db_path) = build_test_state("t5_gap03_totp");
    let app = build_test_router(&state);

    let (user, session_cookie) = create_user_with_session(
        &state,
        "totp_hard@example.com",
        "StrongPass!2026",
        "TOTP Hardened",
        "registered",
        "Sektör Kuruluşları",
        "Test A.Ş.",
        false,
        "",
    );

    // 1. GET /api/v1/auth/totp/setup must not mutate state (405 Method Not Allowed)
    let (st_get_setup, _, _) = req_raw(
        &app,
        "GET",
        "/api/v1/auth/totp/setup",
        Some(&session_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_get_setup, StatusCode::METHOD_NOT_ALLOWED);

    // 2. Forged secret without setup session -> 400
    let forged_secret = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP";
    let forged_code = build_totp_code(forged_secret);
    let (st_forged_pre, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/verify-setup",
        Some(&session_cookie),
        Some(json!({ "secret": forged_secret, "code": forged_code })),
    )
    .await;
    assert_eq!(st_forged_pre, StatusCode::BAD_REQUEST);

    // 3. Legitimate enrollment via POST /totp/setup + POST /totp/verify-setup
    let (st_setup, _, setup_val) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/setup",
        Some(&session_cookie),
        None,
    )
    .await;
    assert_eq!(st_setup, StatusCode::OK);
    let real_secret = setup_val["data"]["secret"].as_str().unwrap().to_string();

    // Passing mismatched secret while pending setup exists -> 400
    let (st_mismatch, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/verify-setup",
        Some(&session_cookie),
        Some(json!({ "secret": forged_secret, "code": forged_code })),
    )
    .await;
    assert_eq!(st_mismatch, StatusCode::BAD_REQUEST);

    // Valid secret + code -> 200
    let real_code = build_totp_code(&real_secret);
    let (st_ok, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/verify-setup",
        Some(&session_cookie),
        Some(json!({ "secret": real_secret, "code": real_code })),
    )
    .await;
    assert_eq!(st_ok, StatusCode::OK);

    // 4. While TOTP is already enabled: POST /setup -> 409; POST /verify-setup without re-auth -> 401
    let (st_re_setup, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/setup",
        Some(&session_cookie),
        None,
    )
    .await;
    assert_eq!(st_re_setup, StatusCode::CONFLICT);

    let (st_overwrite, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/verify-setup",
        Some(&session_cookie),
        Some(json!({ "secret": forged_secret, "code": forged_code })),
    )
    .await;
    assert_eq!(st_overwrite, StatusCode::UNAUTHORIZED);
    assert_eq!(
        state.db.get_user_totp_secret(&user.email).as_deref(),
        Some(real_secret.as_str())
    );

    // 5. Concurrent /api/v1/auth/totp/login with identical temp_token -> at most 1 success
    let temp_token =
        auth_service::generate_totp_pending_token(&user.email, &state.config.session_secret);
    let login_code = build_totp_code(&real_secret);

    let mut handles = Vec::new();
    for _ in 0..5 {
        let app_c = app.clone();
        let state_c = state.clone();
        let tok_c = temp_token.clone();
        let code_c = login_code.clone();
        handles.push(tokio::spawn(async move {
            req_with_csrf(
                &app_c,
                &state_c,
                "POST",
                "/api/v1/auth/totp/login",
                None,
                Some(json!({ "temp_token": tok_c, "code": code_c })),
            )
            .await
            .0
        }));
    }

    let mut success_count = 0;
    let mut unauth_count = 0;
    for h in handles {
        match h.await.unwrap() {
            StatusCode::OK => success_count += 1,
            StatusCode::UNAUTHORIZED => unauth_count += 1,
            other => panic!("Unexpected status during concurrent TOTP login: {}", other),
        }
    }
    assert_eq!(success_count, 1, "Exactly one concurrent TOTP login may succeed");
    assert_eq!(unauth_count, 4);

    // 6. Same-timestep code replay with a fresh temp_token within the same 30s window is rejected
    let temp_token_2 =
        auth_service::generate_totp_pending_token(&user.email, &state.config.session_secret);
    let (st_replay, _, replay_json) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/login",
        None,
        Some(json!({ "temp_token": temp_token_2, "code": login_code })),
    )
    .await;
    assert_eq!(st_replay, StatusCode::UNAUTHORIZED);
    assert_eq!(replay_json["error_code"], "TOTP_CODE_REPLAYED");

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_xff_spoofing_cannot_bypass_login_or_form_rate_limit() {
    // 1. Untrusted non-loopback peer rotating X-Forwarded-For across unique emails -> locks out at attempt 6
    let (mut state_prod, db_path) = build_test_state("t5_gap04_xff");
    let mut cfg = (*state_prod.config).clone();
    cfg.app_env = "production".into();
    state_prod.config = std::sync::Arc::new(cfg);
    let app_prod = build_test_router(&state_prod);

    let untrusted_peer = ConnectInfo(SocketAddr::from(([198, 51, 100, 77], 45001)));
    let mut login_statuses = Vec::new();
    for i in 1..=8 {
        let mut req = Request::builder()
            .method("POST")
            .uri("/api/v1/auth/login")
            .header("x-forwarded-for", format!("198.51.100.{}", i))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "email": format!("probe_untrusted_{}@example.com", i),
                    "password": "WrongPassword!"
                })
                .to_string(),
            ))
            .unwrap();
        req.extensions_mut().insert(untrusted_peer);
        let resp = app_prod.clone().oneshot(req).await.unwrap();
        login_statuses.push(resp.status());
    }
    assert_eq!(&login_statuses[..5], &[StatusCode::UNAUTHORIZED; 5]);
    assert_eq!(&login_statuses[5..], &[StatusCode::TOO_MANY_REQUESTS; 3]);

    // 2. Trusted loopback proxy with leftmost XFF rotation + rightmost real IP -> rightmost wins and locks at 6
    let loopback_peer = ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 45002)));
    let mut proxy_statuses = Vec::new();
    for i in 1..=8 {
        let mut req = Request::builder()
            .method("POST")
            .uri("/api/v1/auth/login")
            .header("x-forwarded-for", format!("10.0.0.{}, 203.0.113.91", i))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "email": format!("probe_proxy_{}@example.com", i),
                    "password": "WrongPassword!"
                })
                .to_string(),
            ))
            .unwrap();
        req.extensions_mut().insert(loopback_peer);
        let resp = app_prod.clone().oneshot(req).await.unwrap();
        proxy_statuses.push(resp.status());
    }
    assert_eq!(&proxy_statuses[..5], &[StatusCode::UNAUTHORIZED; 5]);
    assert_eq!(&proxy_statuses[5..], &[StatusCode::TOO_MANY_REQUESTS; 3]);

    // 3. Spoofed X-Forwarded-For: 127.0.0.1 on forgot-password -> must NOT bypass rate limit (429 on 6th)
    let mut fp_statuses = Vec::new();
    for i in 1..=7 {
        let mut req = Request::builder()
            .method("POST")
            .uri("/api/v1/auth/forgot-password")
            .header("x-forwarded-for", "127.0.0.1")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "email": format!("fp_spoof_{}@example.com", i) }).to_string(),
            ))
            .unwrap();
        req.extensions_mut().insert(loopback_peer);
        let resp = app_prod.clone().oneshot(req).await.unwrap();
        fp_statuses.push(resp.status());
    }
    assert_eq!(&fp_statuses[..5], &[StatusCode::OK; 5]);
    assert_eq!(&fp_statuses[5..], &[StatusCode::TOO_MANY_REQUESTS; 2]);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_empty_session_cookie_does_not_shadow_valid_session_or_break_logout() {
    let (state, db_path) = build_test_state("t5_gap05_cookie");
    let app = build_test_router(&state);

    let (_, valid_cookie) = create_user_with_session(
        &state,
        "shadow_test@example.com",
        "StrongPass!2026",
        "Shadow User",
        "registered",
        "Sektör Kuruluşları",
        "Test A.Ş.",
        false,
        "",
    );
    let csrf = security_engine::generate_csrf_token(&state.config.session_secret);
    let shadowed_cookie = format!("msi_session=; {}; csrf_token={}", valid_cookie, csrf);

    // 1. /hesabim renders authenticated portal, not guest welcome
    let (st_h, _, h_bytes) = req_raw(
        &app,
        "GET",
        "/hesabim",
        Some(&shadowed_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_h, StatusCode::OK);
    let html = String::from_utf8_lossy(&h_bytes);
    assert!(html.contains("/js/account-portal.js"));
    assert!(!html.contains("class=\"account-welcome\""));

    // 2. POST /api/v1/account/profile succeeds
    let (st_prof, _, _) = req_raw(
        &app,
        "POST",
        "/api/v1/account/profile",
        Some(&shadowed_cookie),
        &[("x-csrf-token", &csrf)],
        Some(json!({ "full_name": "Shadow Updated" })),
    )
    .await;
    assert_eq!(st_prof, StatusCode::OK);

    // 3. POST /api/v1/auth/logout succeeds AND revokes the real token
    let (st_out, _, _) = req_raw(
        &app,
        "POST",
        "/api/v1/auth/logout",
        Some(&shadowed_cookie),
        &[("x-csrf-token", &csrf)],
        None,
    )
    .await;
    assert_eq!(st_out, StatusCode::OK);

    let (st_replay, _, _) = req_raw(
        &app,
        "GET",
        "/api/v1/account/export",
        Some(&valid_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_replay, StatusCode::UNAUTHORIZED);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_subscriptions_checkout_rejects_omitted_distance_sales_consent() {
    let (mut state, db_path) = build_test_state("t5_gap06_consent");
    let mut cfg = (*state.config).clone();
    cfg.paytr_merchant_id = String::new();
    state.config = std::sync::Arc::new(cfg);
    let app = build_test_router(&state);

    let (st_omitted, _, _) = req_raw(
        &app,
        "POST",
        "/api/v1/subscriptions/checkout",
        None,
        &[],
        Some(json!({
            "email": "consent_omit@example.com",
            "full_name": "Consent Test",
            "item_type": "subscription",
            "item_code": "dijital"
        })),
    )
    .await;
    assert_eq!(st_omitted, StatusCode::BAD_REQUEST);

    let (st_false, _, _) = req_raw(
        &app,
        "POST",
        "/api/v1/subscriptions/checkout",
        None,
        &[],
        Some(json!({
            "email": "consent_false@example.com",
            "full_name": "Consent Test",
            "item_type": "subscription",
            "item_code": "dijital",
            "distance_sales_consent": false
        })),
    )
    .await;
    assert_eq!(st_false, StatusCode::BAD_REQUEST);

    let (st_true, _, _) = req_raw(
        &app,
        "POST",
        "/api/v1/subscriptions/checkout",
        None,
        &[],
        Some(json!({
            "email": "consent_true@example.com",
            "full_name": "Consent Test",
            "item_type": "subscription",
            "item_code": "dijital",
            "distance_sales_consent": true
        })),
    )
    .await;
    assert_eq!(st_true, StatusCode::ACCEPTED);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_synthetic_edergi_pdf_rejects_oob_pages_and_unverified_staff_parity() {
    let (state, db_path) = build_test_state("t5_gap07_edergi");
    let app = build_test_router(&state);

    let synthetic_issue = MagazineIssue {
        publication_type: "msi".into(),
        issue_number: 88902,
        slug: "sayi-88902".into(),
        title_tr: "Sentetik Test Sayısı".into(),
        title_en: "Synthetic Test Issue".into(),
        publish_month_tr: "Kasım".into(),
        publish_month_en: "November".into(),
        year: 2026,
        cover_image: "/images/logo.svg".into(),
        is_special_issue: false,
        event_name: None,
        page_count: 12,
        dossier_count: 2,
        summary_tr: "Özet".into(),
        summary_en: "Summary".into(),
        pdf_path: "/api/v1/edergi/88902/download-pdf".into(),
        uploaded_pdf_file: None,
        spreads: vec![],
        en_status: "published".into(),
        en_translated_at: None,
        en_approved_by: None,
        en_approved_at: None,
        tr_source_hash: None,
    };
    state.db.upsert_issue(&synthetic_issue).unwrap();

    // Unverified editor_in_chief (email_verified = false)
    let hash = auth_service::hash_password("StaffPass!2026", &state.config.session_secret);
    let uid = state
        .db
        .admin_create_user(
            "unverified_eic_t5@msidergisi.com",
            &hash,
            "Unverified EIC",
            "editor_in_chief",
            "Medya",
            "MSI",
            "Genel Yayın Yönetmeni",
            false,
            "",
        )
        .unwrap();
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute("UPDATE users SET email_verified = 0 WHERE id = ?1", [uid])
            .unwrap();
        state.db.refresh_l1_sections(&["users"]);
    }
    let eic_user = state.db.get_user_by_id(uid).unwrap();
    let eic_cookie = format!(
        "msi_session={}",
        auth_service::generate_session_token(&eic_user, &state.config.session_secret)
    );

    // 1. Out-of-bounds ?pages=9999 on synthetic PDF (no uploaded_pdf_file) -> 400
    let (st_oob, _, _) = req_raw(
        &app,
        "GET",
        "/api/v1/edergi/88902/download-pdf?pages=9999",
        Some(&eic_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_oob, StatusCode::BAD_REQUEST);

    // 2. Valid range ?pages=1-2 -> 200 PDF
    let (st_ok, _, pdf_bytes) = req_raw(
        &app,
        "GET",
        "/api/v1/edergi/88902/download-pdf?pages=1-2",
        Some(&eic_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_ok, StatusCode::OK);
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    // 3. /hesabim E-Dergi link visibility matches API download access for staff
    let (st_hes, _, hes_bytes) = req_raw(
        &app,
        "GET",
        "/hesabim",
        Some(&eic_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_hes, StatusCode::OK);
    let hes_html = String::from_utf8_lossy(&hes_bytes);
    assert!(
        hes_html.contains("/download-pdf?"),
        "Staff role permitted on download-pdf API must also see E-Dergi download links on /hesabim"
    );

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_hesabim_and_monitoring_rbac_isolation() {
    let (state, db_path) = build_test_state("t5_gap09_10_rbac");
    let app = build_test_router(&state);

    // 1. Non-Design-Studio staff roles must NOT receive /tasarim link in /hesabim SSR
    for role in [
        "content_editor",
        "ads_manager",
        "finance_viewer",
        "support_agent",
        "business_dev",
    ] {
        let (_, cookie) = create_user_with_session(
            &state,
            &format!("{}_t5@msidergisi.com", role),
            "StaffPass!2026",
            role,
            role,
            "Medya",
            "MSI",
            false,
            "",
        );
        let (st, _, bytes) = req_raw(&app, "GET", "/hesabim", Some(&cookie), &[], None).await;
        assert_eq!(st, StatusCode::OK);
        let html = String::from_utf8_lossy(&bytes);
        assert!(html.contains("account-command-card"));
        assert!(
            !html.contains("/tasarim"),
            "Role {} must NOT see Design Studio /tasarim link on /hesabim",
            role
        );
    }

    // 2. editor_in_chief CAN access Design Studio on /hesabim, but must NOT access #tab-17 or /monitoring/*
    let (_, eic_cookie) = create_user_with_session(
        &state,
        "eic_t5@msidergisi.com",
        "StaffPass!2026",
        "EIC",
        "editor_in_chief",
        "Medya",
        "MSI",
        false,
        "",
    );
    let (_, _, eic_hes) = req_raw(&app, "GET", "/hesabim", Some(&eic_cookie), &[], None).await;
    assert!(String::from_utf8_lossy(&eic_hes).contains("/tasarim"));

    let (st_mon_eic, _, _) = req_raw(
        &app,
        "GET",
        "/api/v1/admin/monitoring/bundle",
        Some(&eic_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_mon_eic, StatusCode::FORBIDDEN);

    let (st_adm, _, adm_bytes) = req_raw(
        &app,
        "GET",
        &format!("/{}", state.config.admin_path_secret),
        Some(&eic_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_adm, StatusCode::OK);
    let adm_html = String::from_utf8_lossy(&adm_bytes);
    // tab-btn-17 data-required-perm must NOT include settings:read
    if let Some(pos) = adm_html.find("id=\"tab-btn-17\"") {
        let snippet = &adm_html[pos..(pos + 220).min(adm_html.len())];
        assert!(
            !snippet.contains("settings:read"),
            "#tab-btn-17 must not be gated on settings:read: {}",
            snippet
        );
    }

    // 3. super_admin /api/v1/admin/me exposes monitoring_enabled=false cleanly
    let (_, sa_cookie) = create_user_with_session(
        &state,
        "sa_t5@msidergisi.com",
        "StaffPass!2026",
        "SA",
        "super_admin",
        "Medya",
        "MSI",
        false,
        "",
    );
    let (st_me, _, me_bytes) = req_raw(
        &app,
        "GET",
        "/api/v1/admin/me",
        Some(&sa_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_me, StatusCode::OK);
    let me_json: Value = serde_json::from_slice(&me_bytes).unwrap();
    assert_eq!(me_json["data"]["monitoring_enabled"], false);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_verified_user_cannot_pivot_institutions_via_profile() {
    let (state, db_path) = build_test_state("t5_gap12_inst_pivot");
    let app = build_test_router(&state);

    let mut hvkk_item = fixture_content(7011);
    hvkk_item.id = 0;
    hvkk_item.slug = "t5-gap12-hvkk-dossier".into();
    hvkk_item.visibility_level = 0;
    hvkk_item.target_institutions = vec!["Hv.K.K.lığı".into()];
    hvkk_item.body_tr = "<p>Birinci cümle. İkinci cümle. Üçüncü cümle. SECRET_T5_HVKK_CROSS dördüncü cümle.</p>".into();
    state.db.upsert_content(&hvkk_item).unwrap();

    let mut aselsan_item = fixture_content(7012);
    aselsan_item.id = 0;
    aselsan_item.slug = "t5-gap12-aselsan-dossier".into();
    aselsan_item.visibility_level = 0;
    aselsan_item.target_institutions = vec!["ASELSAN".into()];
    aselsan_item.body_tr = "<p>Birinci cümle. İkinci cümle. Üçüncü cümle. SECRET_T5_ASELSAN_CROSS dördüncü cümle.</p>".into();
    state.db.upsert_content(&aselsan_item).unwrap();

    // 1. Verified .gov.tr user belonging to EGM
    let (_, egm_cookie) = create_user_with_session(
        &state,
        "memur_t5@egm.gov.tr",
        "StrongPass!2026",
        "EGM Memuru",
        "subscriber_public",
        "EGM",
        "Emniyet Genel Müdürlüğü",
        true,
        "Kamu E-Dergi Abonesi",
    );

    // Before edit: both Hv.K.K.lığı and ASELSAN dossiers are locked
    let (_, _, pre_hvkk) = req_raw(&app, "GET", "/api/v1/contents/t5-gap12-hvkk-dossier", Some(&egm_cookie), &[], None).await;
    let (_, _, pre_asel) = req_raw(&app, "GET", "/api/v1/contents/t5-gap12-aselsan-dossier", Some(&egm_cookie), &[], None).await;
    assert!(!String::from_utf8_lossy(&pre_hvkk).contains("SECRET_T5_HVKK_CROSS"));
    assert!(!String::from_utf8_lossy(&pre_asel).contains("SECRET_T5_ASELSAN_CROSS"));

    // Pivot attempt via /api/v1/account/profile MUST be rejected with 403
    let (st_pivot_egm, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/account/profile",
        Some(&egm_cookie),
        Some(json!({
            "full_name": "EGM Memuru",
            "organization_category": "Hv.K.K.lığı",
            "organization_name": "ASELSAN"
        })),
    )
    .await;
    assert_eq!(st_pivot_egm, StatusCode::FORBIDDEN);

    // After rejected pivot: both API & SSR remain locked
    let (_, _, post_hvkk) = req_raw(&app, "GET", "/api/v1/contents/t5-gap12-hvkk-dossier", Some(&egm_cookie), &[], None).await;
    let (_, _, post_asel) = req_raw(&app, "GET", "/api/v1/contents/t5-gap12-aselsan-dossier", Some(&egm_cookie), &[], None).await;
    let (_, _, post_html) = req_raw(&app, "GET", "/icerik/t5-gap12-hvkk-dossier", Some(&egm_cookie), &[], None).await;
    assert!(!String::from_utf8_lossy(&post_hvkk).contains("SECRET_T5_HVKK_CROSS"));
    assert!(!String::from_utf8_lossy(&post_asel).contains("SECRET_T5_ASELSAN_CROSS"));
    assert!(!String::from_utf8_lossy(&post_html).contains("SECRET_T5_HVKK_CROSS"));

    // Legitimate non-institution profile update (both omitted org fields and unchanged org fields) -> 200 OK
    let (st_legit_omit, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/account/profile",
        Some(&egm_cookie),
        Some(json!({
            "full_name": "EGM Memuru Güncel",
            "job_title": "Başkomiser",
            "ssb_interests": ["siber-guvenlik"]
        })),
    )
    .await;
    assert_eq!(st_legit_omit, StatusCode::OK);

    let (st_legit_same, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/account/profile",
        Some(&egm_cookie),
        Some(json!({
            "full_name": "EGM Memuru Güncel 2",
            "organization_category": "EGM",
            "organization_name": "Emniyet Genel Müdürlüğü",
            "ssb_interests": ["radarlar-ve-sensorler"]
        })),
    )
    .await;
    assert_eq!(st_legit_same, StatusCode::OK);

    // 2. Corporate subscriber belonging to HAVELSAN cannot pivot to ASELSAN
    let (_, corp_cookie) = create_user_with_session(
        &state,
        "corp_t5@havelsan.com.tr",
        "StrongPass!2026",
        "HAVELSAN Temsilcisi",
        "subscriber_corporate",
        "Sektör Kuruluşları",
        "HAVELSAN",
        false,
        "Kurumsal Abone",
    );

    let (_, _, corp_pre) = req_raw(&app, "GET", "/api/v1/contents/t5-gap12-aselsan-dossier", Some(&corp_cookie), &[], None).await;
    assert!(!String::from_utf8_lossy(&corp_pre).contains("SECRET_T5_ASELSAN_CROSS"));

    let (st_pivot_corp, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/account/profile",
        Some(&corp_cookie),
        Some(json!({
            "full_name": "HAVELSAN Temsilcisi",
            "organization_category": "Sektör Kuruluşları",
            "organization_name": "ASELSAN"
        })),
    )
    .await;
    assert_eq!(st_pivot_corp, StatusCode::FORBIDDEN);

    let (_, _, corp_post) = req_raw(&app, "GET", "/api/v1/contents/t5-gap12-aselsan-dossier", Some(&corp_cookie), &[], None).await;
    assert!(!String::from_utf8_lossy(&corp_post).contains("SECRET_T5_ASELSAN_CROSS"));

    let (st_corp_ok, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/account/profile",
        Some(&corp_cookie),
        Some(json!({
            "full_name": "HAVELSAN Temsilcisi Güncel",
            "organization_category": "Sektör Kuruluşları",
            "organization_name": "HAVELSAN"
        })),
    )
    .await;
    assert_eq!(st_corp_ok, StatusCode::OK);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_totp_backup_code_single_use_across_distinct_temp_tokens_and_reenable() {
    let (state, db_path) = build_test_state("t5_gap13_14_totp");
    let app = build_test_router(&state);

    let email = "totp_gap13_14@example.com";
    let password = "StrongPass!2026";
    let (_, session_cookie) = create_user_with_session(
        &state,
        email,
        password,
        "TOTP Race User",
        "registered",
        "Sektör Kuruluşları",
        "Test A.Ş.",
        false,
        "",
    );

    // 1. Enroll TOTP 2FA and collect secret1 + backup_codes
    let (st_setup1, _, setup1) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/setup",
        Some(&session_cookie),
        None,
    )
    .await;
    assert_eq!(st_setup1, StatusCode::OK);
    let secret1 = setup1["data"]["secret"].as_str().unwrap().to_string();

    let (st_ver1, _, ver1) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/verify-setup",
        Some(&session_cookie),
        Some(json!({ "secret": secret1, "code": build_totp_code(&secret1) })),
    )
    .await;
    assert_eq!(st_ver1, StatusCode::OK);
    let backup_code = ver1["data"]["backup_codes"][0].as_str().unwrap().to_string();

    // 2. Obtain two distinct active temp_tokens via /api/v1/auth/login
    let tt1 = auth_service::generate_totp_pending_token(email, &state.config.session_secret);
    let tt2 = auth_service::generate_totp_pending_token(email, &state.config.session_secret);
    assert_ne!(tt1, tt2);

    // 3. Fire two concurrent /api/v1/auth/totp/login calls with distinct temp_tokens & identical backup code
    let (r1, r2) = tokio::join!(
        req_with_csrf(
            &app,
            &state,
            "POST",
            "/api/v1/auth/totp/login",
            None,
            Some(json!({ "temp_token": tt1, "code": backup_code })),
        ),
        req_with_csrf(
            &app,
            &state,
            "POST",
            "/api/v1/auth/totp/login",
            None,
            Some(json!({ "temp_token": tt2, "code": backup_code })),
        ),
    );
    let mut statuses = [r1.0, r2.0];
    statuses.sort_by_key(|s| s.as_u16());
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::UNAUTHORIZED],
        "Cross-temp_token backup code race must allow at most 1 success"
    );

    // 4. Perform a normal TOTP login with secret1 in the current 30s timestep
    let tt3 = auth_service::generate_totp_pending_token(email, &state.config.session_secret);
    let (st_totp1, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/login",
        None,
        Some(json!({ "temp_token": tt3, "code": build_totp_code(&secret1) })),
    )
    .await;
    assert_eq!(st_totp1, StatusCode::OK);

    // 5. Immediately disable and re-enroll TOTP with a new secret2 within the same 30s timestep
    let (st_dis, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/disable",
        Some(&session_cookie),
        Some(json!({ "password": password })),
    )
    .await;
    assert_eq!(st_dis, StatusCode::OK);

    let (st_setup2, _, setup2) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/setup",
        Some(&session_cookie),
        None,
    )
    .await;
    assert_eq!(st_setup2, StatusCode::OK);
    let secret2 = setup2["data"]["secret"].as_str().unwrap().to_string();
    assert_ne!(secret1, secret2);

    let (st_ver2, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/verify-setup",
        Some(&session_cookie),
        Some(json!({ "secret": secret2, "code": build_totp_code(&secret2) })),
    )
    .await;
    assert_eq!(st_ver2, StatusCode::OK);

    // First login with brand-new secret2 in the same 30s window MUST succeed (200 OK)
    let tt4 = auth_service::generate_totp_pending_token(email, &state.config.session_secret);
    let code2 = build_totp_code(&secret2);
    let (st_totp2, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/login",
        None,
        Some(json!({ "temp_token": tt4, "code": code2 })),
    )
    .await;
    assert_eq!(st_totp2, StatusCode::OK);

    // Replay of secret2's same-step code MUST still be rejected (401 TOTP_CODE_REPLAYED)
    let tt5 = auth_service::generate_totp_pending_token(email, &state.config.session_secret);
    let (st_replay2, _, replay2_json) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/login",
        None,
        Some(json!({ "temp_token": tt5, "code": code2 })),
    )
    .await;
    assert_eq!(st_replay2, StatusCode::UNAUTHORIZED);
    assert_eq!(replay2_json["error_code"], "TOTP_CODE_REPLAYED");

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_csrf_enforced_on_notification_mark_read() {
    let (state, db_path) = build_test_state("t5_gap15_notif_csrf");
    let app = build_test_router(&state);

    let (user, session_cookie) = create_user_with_session(
        &state,
        "notif_csrf@example.com",
        "StrongPass!2026",
        "Notif User",
        "registered",
        "Sektör Kuruluşları",
        "Test A.Ş.",
        false,
        "",
    );

    let notif_id = state
        .db
        .insert_notification(&SystemNotification {
            id: 0,
            target_audience: "user".into(),
            target_email: Some(user.email.clone()),
            notification_type: "info".into(),
            severity: "info".into(),
            title_tr: "Test Bildirimi".into(),
            title_en: "Test Notification".into(),
            message_tr: "Mesaj".into(),
            message_en: "Message".into(),
            action_url: None,
            is_read: false,
            is_active_banner: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
        .expect("insert notification");

    let uri = format!("/api/v1/notifications/{}/read", notif_id);

    // 1. Tokenless POST -> 403
    let (st_no_tok, _, _) = req_raw(&app, "POST", &uri, Some(&session_cookie), &[], None).await;
    assert_eq!(st_no_tok, StatusCode::FORBIDDEN);

    // 2. Cross-origin POST -> 403
    let (st_cross, _, _) = req_raw(
        &app,
        "POST",
        &uri,
        Some(&session_cookie),
        &[("origin", "https://evil.example")],
        None,
    )
    .await;
    assert_eq!(st_cross, StatusCode::FORBIDDEN);

    // Notification must remain unread in DB
    let still_unread = state.db.get_notification_by_id(notif_id).unwrap();
    assert!(!still_unread.is_read);

    // 3. Valid CSRF token + cookie -> 200 OK and marks read
    let (st_ok, _, ok_json) = req_with_csrf(&app, &state, "POST", &uri, Some(&session_cookie), None).await;
    assert_eq!(st_ok, StatusCode::OK);
    assert_eq!(ok_json["data"]["marked_read"], true);
    assert!(state.db.get_notification_by_id(notif_id).unwrap().is_read);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_seo_redirects_rejects_open_redirects_and_retired_admin_resurrection() {
    let (state, db_path) = build_test_state("t5_phase2_seo_redirects");
    let app = build_test_router(&state);

    let (_, pub_cookie) = create_user_with_session(
        &state,
        "pub_redir_t5@msidergisi.com",
        "StaffPass!2026",
        "Publisher",
        "content_publisher",
        "Medya",
        "MSI",
        false,
        "",
    );

    // 1. Reject retired admin & canonical admin sources + unsafe redirect targets via API (400)
    for (old_p, new_p) in [
        ("/msi-komuta", "/podmin"),
        ("/MSI-KOMUTA/tasarim", "/"),
        ("/%6d%73%69-komuta", "/"),
        ("/podmin", "/"),
        ("/podmin/tasarim", "/e-dergi"),
        ("/probe-ext", "https://evil.example/phish"),
        ("/probe-http", "http://evil.example/phish"),
        ("/probe-proto", "//evil.example/phish"),
        ("/probe-backslash", "/\\evil.example"),
        ("/probe-ctrl", "/e-dergi\nLocation: https://evil.example"),
    ] {
        let (st, _, err_json) = req_with_csrf(
            &app,
            &state,
            "POST",
            "/api/v1/admin/redirects",
            Some(&pub_cookie),
            Some(json!({
                "id": 0,
                "old_path": old_p,
                "new_path": new_p,
                "status_code": 301,
                "hit_count": 0
            })),
        )
        .await;
        assert_eq!(
            st,
            StatusCode::BAD_REQUEST,
            "Expected 400 rejecting redirect {} -> {} (got {:?})",
            old_p,
            new_p,
            err_json
        );
    }

    // 2. Direct DB repo check also rejects unsafe rules
    assert!(state
        .db
        .insert_seo_redirect(&msi_dergisi_platform::models::SeoRedirectRule {
            id: 0,
            old_path: "/msi-komuta".into(),
            new_path: "/podmin".into(),
            status_code: 301,
            hit_count: 0,
        })
        .is_err());
    assert!(state
        .db
        .insert_seo_redirect(&msi_dergisi_platform::models::SeoRedirectRule {
            id: 0,
            old_path: "/ext-repo".into(),
            new_path: "//evil.example/p".into(),
            status_code: 301,
            hit_count: 0,
        })
        .is_err());

    // 3. Even if legacy/poisoned rows exist in DB, handle_fallback_or_redirect and render_article_detail refuse to resurrect /msi-komuta or open-redirect
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "INSERT INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?1, ?2, 301, 0)",
            rusqlite::params!["/msi-komuta", "/podmin"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?1, ?2, 301, 0)",
            rusqlite::params!["/legacy-poisoned-ext", "https://evil.example/phish"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?1, ?2, 301, 0)",
            rusqlite::params!["/icerik/t5-poison-article", "https://evil.example/article-phish"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?1, ?2, 301, 0)",
            rusqlite::params!["/icerik/t5-poison-article-proto", "//evil.example/article-phish"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?1, ?2, 301, 0)",
            rusqlite::params!["/haberler/t5-poison-haber-ext", "https://evil.example/haber-phish"],
        )
        .unwrap();
        state.db.refresh_l1_sections(&["seo_redirects"]);
    }

    let (st_komuta, _, _) = req_raw(&app, "GET", "/msi-komuta", None, &[], None).await;
    assert_eq!(st_komuta, StatusCode::NOT_FOUND);

    for blocked_path in [
        "/legacy-poisoned-ext",
        "/icerik/t5-poison-article",
        "/icerik/t5-poison-article-proto",
        "/icerik/t5-poison-haber-ext",
    ] {
        let (st_ext, hdrs_ext, _) = req_raw(&app, "GET", blocked_path, None, &[], None).await;
        assert_eq!(
            st_ext,
            StatusCode::NOT_FOUND,
            "Expected 404 blocking poisoned redirect for {}",
            blocked_path
        );
        assert!(
            hdrs_ext.get(header::LOCATION).is_none(),
            "Expected no Location header on poisoned redirect for {}",
            blocked_path
        );
    }

    // 4. Legitimate site-relative redirect works as expected
    let (st_ok, _, ok_json) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/admin/redirects",
        Some(&pub_cookie),
        Some(json!({
            "id": 0,
            "old_path": "/eski-t5-sayfa",
            "new_path": "/e-dergi",
            "status_code": 301,
            "hit_count": 0
        })),
    )
    .await;
    assert_eq!(st_ok, StatusCode::OK);
    assert!(ok_json["data"]["id"].as_i64().unwrap_or(0) > 0);

    let (st_redir, hdrs_redir, _) = req_raw(&app, "GET", "/eski-t5-sayfa", None, &[], None).await;
    assert_eq!(st_redir, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(hdrs_redir.get(header::LOCATION).unwrap(), "/e-dergi");

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_csrf_enforced_on_design_studio_preview_post() {
    let (mut state, db_path) = build_test_state("t5_phase2_preview_csrf");
    let mut cfg = (*state.config).clone();
    cfg.admin_path_secret = "e2e-admin-csrf".into();
    state.config = std::sync::Arc::new(cfg);
    let app = build_test_router(&state);

    let (_, sa_cookie) = create_user_with_session(
        &state,
        "sa_preview_csrf@msidergisi.com",
        "StaffPass!2026",
        "SA Preview",
        "super_admin",
        "Medya",
        "MSI",
        false,
        "",
    );

    let design_cfg = msi_dergisi_platform::engine::design::config::load(&state.db.get_setting_map("tr"));
    let payload = serde_json::to_value(&design_cfg).unwrap();

    for path in ["/podmin/tasarim/onizleme", "/e2e-admin-csrf/tasarim/onizleme"] {
        // 1. Tokenless POST with session cookie -> 403
        let (st_notok, _, _) = req_raw(
            &app,
            "POST",
            path,
            Some(&sa_cookie),
            &[],
            Some(payload.clone()),
        )
        .await;
        assert_eq!(
            st_notok,
            StatusCode::FORBIDDEN,
            "Tokenless POST {} must be rejected with 403",
            path
        );

        // 2. Cross-origin POST -> 403
        let (st_cross, _, _) = req_raw(
            &app,
            "POST",
            path,
            Some(&sa_cookie),
            &[("origin", "https://evil.example")],
            Some(payload.clone()),
        )
        .await;
        assert_eq!(
            st_cross,
            StatusCode::FORBIDDEN,
            "Cross-origin POST {} must be rejected with 403",
            path
        );

        // 3. Valid double-submit CSRF token + cookie -> 200 HTML
        let (st_ok, _, _) = req_with_csrf(
            &app,
            &state,
            "POST",
            path,
            Some(&sa_cookie),
            Some(payload.clone()),
        )
        .await;
        assert_eq!(
            st_ok,
            StatusCode::OK,
            "Valid CSRF POST {} must succeed with 200",
            path
        );
    }

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_trailing_slash_admin_gateway_and_head_requests() {
    // 1. Canonical podmin configuration
    let (state, db_path) = build_test_state("t5_phase2_trailing_slash");
    let app = build_test_router(&state);

    let (_, sa_cookie) = create_user_with_session(
        &state,
        "sa_trail@msidergisi.com",
        "StaffPass!2026",
        "SA Trail",
        "super_admin",
        "Medya",
        "MSI",
        false,
        "",
    );

    // Unauthenticated GET & HEAD on /podmin/ and /podmin/tasarim/ -> 303 redirect to /podmin (never 401 JSON)
    for path in ["/podmin/", "/podmin/tasarim/"] {
        for method in ["GET", "HEAD"] {
            let (st, hdrs, _) = req_raw(&app, method, path, None, &[], None).await;
            assert_eq!(
                st,
                StatusCode::SEE_OTHER,
                "Unauth {} {} must 303 redirect (got {})",
                method,
                path,
                st
            );
            assert_eq!(hdrs.get(header::LOCATION).unwrap(), "/podmin");
        }
    }

    // Authenticated GET & HEAD on /podmin/ -> 308 -> /podmin; /podmin/tasarim/ -> 308 -> /podmin/tasarim
    for (path, expected_loc) in [
        ("/podmin/", "/podmin"),
        ("/podmin/tasarim/", "/podmin/tasarim"),
    ] {
        for method in ["GET", "HEAD"] {
            let (st, hdrs, _) = req_raw(&app, method, path, Some(&sa_cookie), &[], None).await;
            assert_eq!(
                st,
                StatusCode::PERMANENT_REDIRECT,
                "Auth {} {} must 308 redirect (got {})",
                method,
                path,
                st
            );
            assert_eq!(hdrs.get(header::LOCATION).unwrap(), expected_loc);
        }
    }
    cleanup_test_db(&db_path);

    // 2. Custom admin_path_secret configuration also supports trailing-slash redirects
    let (mut state_c, db_path_c) = build_test_state("t5_phase2_trailing_custom");
    let mut cfg = (*state_c.config).clone();
    cfg.admin_path_secret = "custom-gw".into();
    state_c.config = std::sync::Arc::new(cfg);
    let app_c = build_test_router(&state_c);

    let (_, sa_cookie_c) = create_user_with_session(
        &state_c,
        "sa_trail_c@msidergisi.com",
        "StaffPass!2026",
        "SA Trail C",
        "super_admin",
        "Medya",
        "MSI",
        false,
        "",
    );

    let (st_uhead, hdrs_uhead, _) = req_raw(&app_c, "HEAD", "/custom-gw/", None, &[], None).await;
    assert_eq!(st_uhead, StatusCode::SEE_OTHER);
    assert_eq!(hdrs_uhead.get(header::LOCATION).unwrap(), "/custom-gw");

    for (path, expected_loc) in [
        ("/custom-gw/", "/custom-gw"),
        ("/custom-gw/tasarim/", "/custom-gw/tasarim"),
        ("/podmin/", "/podmin"),
        ("/podmin/tasarim/", "/podmin/tasarim"),
    ] {
        let (st, hdrs, _) = req_raw(&app_c, "GET", path, Some(&sa_cookie_c), &[], None).await;
        assert_eq!(st, StatusCode::PERMANENT_REDIRECT);
        assert_eq!(hdrs.get(header::LOCATION).unwrap(), expected_loc);
    }

    cleanup_test_db(&db_path_c);
}

#[tokio::test]
async fn t5_design_studio_can_save_flag_for_read_only_vs_write_roles() {
    let (state, db_path) = build_test_state("t5_phase2_studio_cansave");
    let app = build_test_router(&state);

    let (_, eic_cookie) = create_user_with_session(
        &state,
        "eic_studio_t5@msidergisi.com",
        "StaffPass!2026",
        "EIC Studio",
        "editor_in_chief",
        "Medya",
        "MSI",
        false,
        "",
    );
    let (_, sa_cookie) = create_user_with_session(
        &state,
        "sa_studio_t5@msidergisi.com",
        "StaffPass!2026",
        "SA Studio",
        "super_admin",
        "Medya",
        "MSI",
        false,
        "",
    );

    // 1. editor_in_chief (settings:read only) gets can_save=false in API + data-can-save="false" in SSR
    let (st_eic_api, _, eic_api_bytes) =
        req_raw(&app, "GET", "/api/v1/admin/design", Some(&eic_cookie), &[], None).await;
    assert_eq!(st_eic_api, StatusCode::OK);
    let eic_json: Value = serde_json::from_slice(&eic_api_bytes).unwrap();
    assert_eq!(eic_json["can_save"], false);

    let (st_eic_view, _, eic_html_bytes) =
        req_raw(&app, "GET", "/podmin/tasarim", Some(&eic_cookie), &[], None).await;
    assert_eq!(st_eic_view, StatusCode::OK);
    assert!(String::from_utf8_lossy(&eic_html_bytes).contains("data-can-save=\"false\""));

    // Preview remains allowed with valid CSRF, while save remains 403
    let (st_eic_prev, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/podmin/tasarim/onizleme",
        Some(&eic_cookie),
        Some(eic_json["config"].clone()),
    )
    .await;
    assert_eq!(st_eic_prev, StatusCode::OK);

    let (st_eic_save, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/admin/design",
        Some(&eic_cookie),
        Some(eic_json["config"].clone()),
    )
    .await;
    assert_eq!(st_eic_save, StatusCode::FORBIDDEN);

    // 2. super_admin (settings:write) gets can_save=true
    let (st_sa_api, _, sa_api_bytes) =
        req_raw(&app, "GET", "/api/v1/admin/design", Some(&sa_cookie), &[], None).await;
    assert_eq!(st_sa_api, StatusCode::OK);
    let sa_json: Value = serde_json::from_slice(&sa_api_bytes).unwrap();
    assert_eq!(sa_json["can_save"], true);

    let (st_sa_view, _, sa_html_bytes) =
        req_raw(&app, "GET", "/podmin/tasarim", Some(&sa_cookie), &[], None).await;
    assert_eq!(st_sa_view, StatusCode::OK);
    assert!(String::from_utf8_lossy(&sa_html_bytes).contains("data-can-save=\"true\""));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_not_found_responses_omit_canonical_and_emit_noindex() {
    let (state, db_path) = build_test_state("t5_phase2_404_seo");
    let app = build_test_router(&state);

    for path in [
        "/msi-komuta",
        "/MSI-KOMUTA",
        "/%6d%73%69-%6b%6f%6d%75%74%61",
        "/olmayan-sayfa-tier5",
    ] {
        let (st, hdrs, body_bytes) = req_raw(&app, "GET", path, None, &[], None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "Expected 404 on {}", path);
        let link_val = hdrs
            .get("link")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            !link_val.contains("rel=\"canonical\""),
            "404 response for {} must NOT emit rel=\"canonical\" Link header (got: {})",
            path,
            link_val
        );
        let robots_val = hdrs
            .get("x-robots-tag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            robots_val.contains("noindex") && robots_val.contains("nofollow"),
            "404 response for {} must emit x-robots-tag: noindex, nofollow (got: {})",
            path,
            robots_val
        );

        let body = String::from_utf8_lossy(&body_bytes);
        assert!(
            !body.contains("rel=\"canonical\""),
            "404 HTML for {} must NOT emit <link rel=\"canonical\">",
            path
        );
        assert!(
            !body.contains("hreflang=\"x-default\""),
            "404 HTML for {} must NOT emit hreflang=\"x-default\"",
            path
        );
        assert!(
            !body.contains("hreflang=\"tr\"") && !body.contains("hreflang=\"en\""),
            "404 HTML for {} must NOT emit hreflang=\"tr\"/\"en\"",
            path
        );
        assert!(
            !body.contains("property=\"og:url\""),
            "404 HTML for {} must NOT emit og:url",
            path
        );
        assert!(
            !body.contains("application/ld+json"),
            "404 HTML for {} must NOT emit homepage JSON-LD",
            path
        );
        assert!(
            body.contains("<meta name=\"robots\" content=\"noindex, nofollow\" />"),
            "404 HTML for {} must emit <meta name=\"robots\" content=\"noindex, nofollow\" />",
            path
        );
    }

    // 200 OK public page still emits rel="canonical" (header + HTML head) and hreflang="x-default"
    let (st_home, hdrs_home, home_bytes) = req_raw(&app, "GET", "/", None, &[], None).await;
    assert_eq!(st_home, StatusCode::OK);
    let home_link = hdrs_home
        .get("link")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(home_link.contains("rel=\"canonical\""));
    let home_body = String::from_utf8_lossy(&home_bytes);
    assert!(home_body.contains("rel=\"canonical\""));
    assert!(home_body.contains("hreflang=\"x-default\""));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_rbac_publish_bypass_blocked_on_legacy_upsert_and_bulk() {
    let (state, db_path) = build_test_state("t5_p2_rbac_publish_bypass");
    let app = build_test_router(&state);

    let (_, editor_cookie) = create_user_with_session(
        &state,
        "ce_t5_rbac@msidergisi.com",
        "StaffPass!2026",
        "Content Editor",
        "content_editor",
        "Medya",
        "MSI",
        false,
        "",
    );

    let mut pub_item = fixture_content(7101);
    pub_item.id = 0;
    pub_item.slug = "t5-p2-published-guard".into();
    pub_item.title_tr = "Orijinal Yayında Başlık".into();
    pub_item.status = "published".into();
    pub_item.visibility_level = 0;
    let pub_id = state.db.upsert_content(&pub_item).unwrap();

    let mut draft_item = fixture_content(7102);
    draft_item.id = 0;
    draft_item.slug = "t5-p2-draft-editable".into();
    draft_item.title_tr = "Taslak Başlık".into();
    draft_item.status = "draft".into();
    draft_item.visibility_level = 0;
    let draft_id = state.db.upsert_content(&draft_item).unwrap();
    state.db.refresh_l1_sections(&["contents"]);

    // (a) content_editor cannot overwrite a published article via POST /api/v1/admin/contents with status="draft"
    let mut overwrite_payload = serde_json::to_value(&pub_item).unwrap();
    overwrite_payload["id"] = json!(pub_id);
    overwrite_payload["status"] = json!("draft");
    overwrite_payload["title_tr"] = json!("Unpublished By ContentEditor");
    overwrite_payload["visibility_level"] = json!(2);

    let (st_upsert, _, upsert_json) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/admin/contents",
        Some(&editor_cookie),
        Some(overwrite_payload),
    )
    .await;
    assert_eq!(st_upsert, StatusCode::FORBIDDEN);
    assert_eq!(upsert_json["error_code"], "FORBIDDEN");
    assert_eq!(upsert_json["required_permission"], "contents:publish");

    // (b) content_editor cannot bulk-archive, bulk-draft, bulk-visibility, or bulk-feature published articles
    for (action, val) in [
        ("archive", None),
        ("draft", None),
        ("set_visibility", Some("2")),
        ("set_featured", Some("true")),
    ] {
        let (st_bulk, _, bulk_json) = req_with_csrf(
            &app,
            &state,
            "POST",
            "/api/v1/admin/contents/bulk",
            Some(&editor_cookie),
            Some(json!({
                "ids": [pub_id],
                "action": action,
                "value": val
            })),
        )
        .await;
        assert_eq!(
            st_bulk,
            StatusCode::FORBIDDEN,
            "Bulk action '{}' on published article must return 403",
            action
        );
        assert_eq!(bulk_json["error_code"], "FORBIDDEN");
        assert_eq!(bulk_json["required_permission"], "contents:publish");
    }

    // DB record must remain completely unchanged
    let after_pub = state
        .db
        .get_all_contents()
        .into_iter()
        .find(|c| c.id == pub_id)
        .unwrap();
    assert_eq!(after_pub.status, "published");
    assert_eq!(after_pub.title_tr, "Orijinal Yayında Başlık");
    assert_eq!(after_pub.visibility_level, 0);

    // Legitimate edit on an existing "draft" article by content_editor still succeeds
    let mut draft_edit_payload = serde_json::to_value(&draft_item).unwrap();
    draft_edit_payload["id"] = json!(draft_id);
    draft_edit_payload["status"] = json!("draft");
    draft_edit_payload["title_tr"] = json!("Taslak Güncellendi");
    let (st_draft_ok, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/admin/contents",
        Some(&editor_cookie),
        Some(draft_edit_payload),
    )
    .await;
    assert_eq!(st_draft_ok, StatusCode::OK);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_editorial_svg_upload_rejects_xss_past_2kb_and_event_handlers() {
    let (mut state, db_path) = build_test_state("t5_p2_svg_xss");
    let temp_static = format!("{}_static", db_path);
    let mut cfg = (*state.config).clone();
    cfg.static_dir = temp_static.clone();
    state.config = std::sync::Arc::new(cfg);
    let app = build_test_router(&state);

    let (_, editor_cookie) = create_user_with_session(
        &state,
        "ce_t5_svg@msidergisi.com",
        "StaffPass!2026",
        "SVG Uploader",
        "content_editor",
        "Medya",
        "MSI",
        false,
        "",
    );

    let padded_past_2kb = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\">{}<script>alert(1)</script></svg>",
        " ".repeat(2100)
    );
    let payloads = [
        (
            "onload.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" onload=\"alert(1)\"><circle r=\"5\"/></svg>",
        ),
        ("past2kb.svg", padded_past_2kb.as_str()),
        (
            "xlink_js.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><a xlink:href=\"javascript:alert(1)\">x</a></svg>",
        ),
        (
            "animate_xss.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><animate attributeName=\"href\" values=\"javascript:alert(1)\"/></svg>",
        ),
    ];

    for (fname, svg_body) in payloads {
        let csrf = security_engine::generate_csrf_token(&state.config.session_secret);
        let boundary = "----MSI_T5_BOUNDARY_SVG";
        let multipart_body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"{fname}\"\r\nContent-Type: image/svg+xml\r\n\r\n{svg}\r\n--{b}--\r\n",
            b = boundary,
            fname = fname,
            svg = svg_body
        );
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/editorial/upload")
            .header(header::COOKIE, format!("{}; csrf_token={}", editor_cookie, csrf))
            .header("x-csrf-token", &csrf)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(multipart_body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "Malicious SVG '{}' must be rejected with 400",
            fname
        );
    }

    // No .svg files may have been written to uploads/editorial
    let editorial_dir = std::path::Path::new(&temp_static)
        .join("uploads")
        .join("editorial");
    if editorial_dir.exists() {
        let svg_count = std::fs::read_dir(&editorial_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("svg"))
            .count();
        assert_eq!(svg_count, 0, "Zero .svg files may be written on rejected uploads");
    }

    // Clean valid SVG passes detection
    let clean_svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><circle cx=\"5\" cy=\"5\" r=\"4\"/></svg>";
    assert_eq!(
        msi_dergisi_platform::services::editorial_media_service::detect_image_extension(clean_svg),
        Some("svg")
    );

    let _ = std::fs::remove_dir_all(&temp_static);
    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_paytr_uppercase_kurumsal_invoice_collision_and_concurrent_callbacks() {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Spin up a local stub PayTR get-token server on 127.0.0.1:0 (zero external network calls)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let stub_addr = listener.local_addr().unwrap();
    let stub_handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"status":"success","token":"stub-iframe-tok-t5"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            });
        }
    });

    let (mut state, db_path) = build_test_state("t5_p2_paytr_kurumsal");
    let mut cfg = (*state.config).clone();
    cfg.paytr_api_url = format!("http://{}/odeme/api/get-token", stub_addr);
    state.config = std::sync::Arc::new(cfg);
    let app = build_test_router(&state);

    let sign_paytr = |merchant_oid: &str, status: &str, total_amount: &str| -> String {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(state.config.paytr_merchant_key.as_bytes()).unwrap();
        mac.update(
            format!(
                "{}{}{}{}",
                merchant_oid, state.config.paytr_merchant_salt, status, total_amount
            )
            .as_bytes(),
        );
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            mac.finalize().into_bytes(),
        )
    };

    // (a) Uppercase "KURUMSAL" cart-checkout + PayTR callback -> subscriber_corporate
    let (user_corp, corp_cookie) = create_user_with_session(
        &state,
        "kurumsal_buyer_t5@example.com",
        "StrongPass!2026",
        "Kurumsal Alıcı",
        "registered",
        "Sektör Kuruluşları",
        "Savunma A.Ş.",
        false,
        "",
    );

    let (st_cart, _, cart_val) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/commerce/cart-checkout",
        Some(&corp_cookie),
        Some(json!({
            "email": user_corp.email,
            "full_name": "Kurumsal Alıcı",
            "distance_sales_consent": true,
            "items": [{ "item_type": "subscription", "item_code": "KURUMSAL", "qty": 1 }]
        })),
    )
    .await;
    assert_eq!(st_cart, StatusCode::OK, "cart-checkout failed: {:?}", cart_val);
    let corp_oid = cart_val["merchant_oid"].as_str().unwrap().to_string();
    let corp_kurus = cart_val["amount_kurus"].as_i64().unwrap().to_string();
    let corp_hash = sign_paytr(&corp_oid, "success", &corp_kurus);
    let cb_body = serde_urlencoded::to_string([
        ("merchant_oid", corp_oid.as_str()),
        ("status", "success"),
        ("total_amount", corp_kurus.as_str()),
        ("hash", corp_hash.as_str()),
    ])
    .unwrap();
    let cb_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/payments/paytr/callback")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(cb_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cb_resp.status(), StatusCode::OK);
    let upgraded = state.db.get_user_by_email(&user_corp.email).unwrap().0;
    assert_eq!(upgraded.role, "subscriber_corporate");

    // (b) Duplicate invoice_no collision returns the actual persisted unique invoice_no
    let inv_template = msi_dergisi_platform::models::OrderInvoice {
        id: 0,
        invoice_no: "MSI-2026-991122".into(),
        user_email: user_corp.email.clone(),
        full_name: "Collision Test".into(),
        organization_name: "Test".into(),
        item_type: "subscription".into(),
        item_code: "dijital".into(),
        item_title: "Dijital Abonelik".into(),
        amount_try: 990.0,
        payment_status: "pending".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        pdf_watermark_code: "WM-COLL-1".into(),
    };
    let (id1, no1) = state.db.insert_order_invoice(&inv_template).unwrap();
    assert_eq!(no1, "MSI-2026-991122");
    let (id2, no2) = state.db.insert_order_invoice(&inv_template).unwrap();
    assert_ne!(id1, id2);
    assert_ne!(no2, "MSI-2026-991122");
    let db_no2 = state
        .db
        .get_order_invoices()
        .into_iter()
        .find(|i| i.id == id2)
        .map(|i| i.invoice_no)
        .unwrap();
    assert_eq!(no2, db_no2);

    // (c) 5 concurrent signed PayTR callbacks for the same merchant_oid -> all 200 OK, zero 500/SQLITE_BUSY
    let (race_user, _) = create_user_with_session(
        &state,
        "paytr_race_t5@example.com",
        "StrongPass!2026",
        "Race Buyer",
        "registered",
        "Sektör Kuruluşları",
        "Race A.Ş.",
        false,
        "",
    );
    let race_inv = msi_dergisi_platform::models::OrderInvoice {
        id: 0,
        invoice_no: "MSI-2026-884411".into(),
        user_email: race_user.email.clone(),
        full_name: "Race Buyer".into(),
        organization_name: "Race A.Ş.".into(),
        item_type: "subscription".into(),
        item_code: "dijital".into(),
        item_title: "Dijital E-Dergi Aboneliği".into(),
        amount_try: 990.0,
        payment_status: "pending".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        pdf_watermark_code: "WM-RACE-1".into(),
    };
    state.db.insert_order_invoice(&race_inv).unwrap();
    let race_oid =
        msi_dergisi_platform::services::payment_service::generate_merchant_oid("MSI-2026-884411");
    let race_hash = sign_paytr(&race_oid, "success", "99000");
    let race_body = serde_urlencoded::to_string([
        ("merchant_oid", race_oid.as_str()),
        ("status", "success"),
        ("total_amount", "99000"),
        ("hash", race_hash.as_str()),
    ])
    .unwrap();

    let mut handles = Vec::new();
    for _ in 0..5 {
        let app_c = app.clone();
        let body_c = race_body.clone();
        handles.push(tokio::spawn(async move {
            let resp = app_c
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/payments/paytr/callback")
                        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                        .body(Body::from(body_c))
                        .unwrap(),
                )
                .await
                .unwrap();
            let st = resp.status();
            let txt = String::from_utf8_lossy(
                &to_bytes(resp.into_body(), 1024 * 1024).await.unwrap(),
            )
            .to_string();
            (st, txt)
        }));
    }
    for h in handles {
        let (st, txt) = h.await.unwrap();
        assert_eq!(st, StatusCode::OK, "Concurrent callback failed: {}", txt);
        assert_eq!(txt, "OK");
    }

    let conn = state.db.open_conn().unwrap();
    let purchase_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM user_purchased_items WHERE LOWER(user_email) = LOWER(?1) AND item_code = 'dijital'",
            [&race_user.email],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(purchase_rows, 1);

    stub_handle.abort();
    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_resend_verification_throttles_bursts_and_invalidates_prior_tokens() {
    let (state, db_path) = build_test_state("t5_p2_resend_verify");
    let app = build_test_router(&state);

    let hash = auth_service::hash_password("StrongPass!2026", &state.config.session_secret);
    let uid = state
        .db
        .admin_create_user(
            "resend_burst_t5@example.com",
            &hash,
            "Unverified T5",
            "registered",
            "Sektör Kuruluşları",
            "Test A.Ş.",
            "Uzman",
            false,
            "",
        )
        .unwrap();
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute("UPDATE users SET email_verified = 0 WHERE id = ?1", [uid])
            .unwrap();
        state.db.refresh_l1_sections(&["users"]);
    }

    let tok1 = "tok1_initial_verification_secret_t5".to_string();
    let expires = (chrono::Utc::now() + chrono::Duration::hours(24))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    state
        .db
        .create_email_verification_token(uid, &tok1, &expires)
        .unwrap();

    // Burst 5 resend-verification calls -> first 2 succeed (200), remaining hit 429 Too Many Requests
    let mut statuses = Vec::new();
    for _ in 0..5 {
        let (st, _, _) = req_raw(
            &app,
            "POST",
            "/api/v1/auth/resend-verification",
            None,
            &[("x-forwarded-for", "198.51.100.88")],
            Some(json!({ "email": "resend_burst_t5@example.com" })),
        )
        .await;
        statuses.push(st);
    }
    assert_eq!(statuses[0], StatusCode::OK);
    assert_eq!(statuses[1], StatusCode::OK);
    assert!(
        statuses[2..].iter().all(|s| *s == StatusCode::TOO_MANY_REQUESTS),
        "Rapid burst must return 429 after threshold: {:?}",
        statuses
    );

    // Old token tok1 must have been invalidated once tok2 was issued
    let verify_uri = format!("/hesap-dogrula?token={}&confirm=1", tok1);
    let (st_old, _, _) = req_raw(&app, "GET", &verify_uri, None, &[], None).await;
    assert_eq!(st_old, StatusCode::BAD_REQUEST);
    let user_after = state.db.get_user_by_id(uid).unwrap();
    assert!(!user_after.email_verified);

    // TOTP verify-setup brute-force throttling
    let (_, totp_cookie) = create_user_with_session(
        &state,
        "totp_setup_throttle_t5@example.com",
        "StrongPass!2026",
        "TOTP Setup Throttle",
        "registered",
        "Sektör Kuruluşları",
        "Test A.Ş.",
        false,
        "",
    );
    let (st_s, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/auth/totp/setup",
        Some(&totp_cookie),
        None,
    )
    .await;
    assert_eq!(st_s, StatusCode::OK);

    let mut totp_statuses = Vec::new();
    for _ in 0..6 {
        let (st, _, _) = req_with_csrf(
            &app,
            &state,
            "POST",
            "/api/v1/auth/totp/verify-setup",
            Some(&totp_cookie),
            Some(json!({ "code": "000000" })),
        )
        .await;
        totp_statuses.push(st);
    }
    assert_eq!(totp_statuses[5], StatusCode::TOO_MANY_REQUESTS);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_personalized_feed_turkish_i_case_folding() {
    let (state, db_path) = build_test_state("t5_p2_turkish_i_feed");
    let app = build_test_router(&state);

    let mut older_iha = fixture_content(7201);
    older_iha.id = 0;
    older_iha.slug = "t5-p2-older-iha-article".into();
    older_iha.status = "published".into();
    older_iha.published_at = "2025-01-10 10:00:00".into();
    older_iha.ssb_tags = vec!["iha".into()];
    older_iha.is_featured = false;
    older_iha.is_sponsored = false;
    state.db.upsert_content(&older_iha).unwrap();

    let mut newer_deniz = fixture_content(7202);
    newer_deniz.id = 0;
    newer_deniz.slug = "t5-p2-newer-deniz-article".into();
    newer_deniz.status = "published".into();
    newer_deniz.published_at = "2026-11-20 10:00:00".into();
    newer_deniz.ssb_tags = vec!["deniz".into()];
    newer_deniz.is_featured = true;
    newer_deniz.is_sponsored = false;
    state.db.upsert_content(&newer_deniz).unwrap();
    state.db.refresh_l1_sections(&["contents"]);

    let (st_tr, _, bytes_tr) = req_raw(
        &app,
        "GET",
        "/api/v1/public/personalized?interests=%C4%B0HA&limit=1",
        None,
        &[],
        None,
    )
    .await;
    assert_eq!(st_tr, StatusCode::OK);
    let val_tr: Value = serde_json::from_slice(&bytes_tr).unwrap();
    assert_eq!(
        val_tr["data"]["items"][0]["slug"].as_str(),
        Some("t5-p2-older-iha-article")
    );

    let (st_ascii, _, bytes_ascii) = req_raw(
        &app,
        "GET",
        "/api/v1/public/personalized?interests=iha&limit=1",
        None,
        &[],
        None,
    )
    .await;
    assert_eq!(st_ascii, StatusCode::OK);
    let val_ascii: Value = serde_json::from_slice(&bytes_ascii).unwrap();
    assert_eq!(
        val_tr["data"]["items"][0]["slug"],
        val_ascii["data"]["items"][0]["slug"]
    );

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_personalized_feed_recommendation_rule_institution_alias() {
    let (state, db_path) = build_test_state("t5_p2_inst_rule_alias");
    let app = build_test_router(&state);

    let mut older_boosted = fixture_content(7301);
    older_boosted.id = 0;
    older_boosted.slug = "t5-p2-tsk-boosted-older".into();
    older_boosted.status = "published".into();
    older_boosted.published_at = "2025-01-10 10:00:00".into();
    older_boosted.ssb_tags = vec!["hava-araclari".into()];
    older_boosted.is_featured = false;
    older_boosted.is_sponsored = false;
    state.db.upsert_content(&older_boosted).unwrap();

    let mut newer_plain = fixture_content(7302);
    newer_plain.id = 0;
    newer_plain.slug = "t5-p2-newer-unboosted".into();
    newer_plain.status = "published".into();
    newer_plain.published_at = "2026-11-20 10:00:00".into();
    newer_plain.ssb_tags = vec!["hava-araclari".into()];
    newer_plain.is_featured = false;
    newer_plain.is_sponsored = false;
    state.db.upsert_content(&newer_plain).unwrap();

    state
        .db
        .insert_recommendation_rule(&msi_dergisi_platform::models::RecommendationRule {
            id: 0,
            rule_name: "TSK Özel Öneri Kuralı".into(),
            source_tag_or_cat: "*".into(),
            target_institution: "tsk".into(),
            recommended_content_slug: "t5-p2-tsk-boosted-older".into(),
            priority: 10,
            is_active: true,
        })
        .unwrap();
    state
        .db
        .insert_recommendation_rule(&msi_dergisi_platform::models::RecommendationRule {
            id: 0,
            rule_name: "SSB Kod Eşleme Kuralı".into(),
            source_tag_or_cat: "*".into(),
            target_institution: "ssb".into(),
            recommended_content_slug: "t5-p2-tsk-boosted-older".into(),
            priority: 10,
            is_active: true,
        })
        .unwrap();
    state.db.refresh_l1_sections(&["contents", "recommendation_rules"]);

    let (_, tsk_cookie) = create_user_with_session(
        &state,
        "tsk_user_t5@tsk.tr",
        "StrongPass!2026",
        "TSK Personeli",
        "subscriber_public",
        "Türk Silahlı Kuvvetleri",
        "TSK",
        true,
        "Kamu",
    );

    let (st_tsk, _, bytes_tsk) = req_raw(
        &app,
        "GET",
        "/api/v1/public/personalized?limit=1",
        Some(&tsk_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_tsk, StatusCode::OK);
    let val_tsk: Value = serde_json::from_slice(&bytes_tsk).unwrap();
    assert_eq!(
        val_tsk["data"]["items"][0]["slug"].as_str(),
        Some("t5-p2-tsk-boosted-older")
    );

    // Also verify institution_categories label_tr -> code alias ("SSB (Savunma Sanayii Başkanlığı)" -> "ssb")
    let (_, ssb_cookie) = create_user_with_session(
        &state,
        "ssb_user_t5@ssb.gov.tr",
        "StrongPass!2026",
        "SSB Personeli",
        "subscriber_public",
        "SSB (Savunma Sanayii Başkanlığı)",
        "SSB",
        true,
        "Kamu",
    );
    let (st_ssb, _, bytes_ssb) = req_raw(
        &app,
        "GET",
        "/api/v1/public/personalized?limit=1",
        Some(&ssb_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_ssb, StatusCode::OK);
    let val_ssb: Value = serde_json::from_slice(&bytes_ssb).unwrap();
    assert_eq!(
        val_ssb["data"]["items"][0]["slug"].as_str(),
        Some("t5-p2-tsk-boosted-older")
    );

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_sanitize_inline_rich_html_escapes_unclosed_lt() {
    use msi_dergisi_platform::editorial::common::{
        blocks_to_wp_html, sanitize_inline_rich_html, ContentBlock,
    };

    let clean = sanitize_inline_rich_html("<img src=x onerror=alert(1)");
    assert!(!clean.contains('<'), "Raw '<' must never be emitted: {}", clean);
    assert!(clean.starts_with("&lt;img"));

    let wp_html = blocks_to_wp_html(&[ContentBlock {
        id: "b1".into(),
        block_type: "paragraph".into(),
        content: "<svg onload=alert(1)".into(),
        meta: None,
    }]);
    assert!(
        !wp_html.contains("<svg"),
        "Unclosed <svg must be escaped in blocks_to_wp_html: {}",
        wp_html
    );
    assert_eq!(wp_html, "<p>&lt;svg onload=alert(1)</p>");

    // Verify end-to-end editorial API also escapes unclosed '<' in blocks[0].content
    let (state, db_path) = build_test_state("t5_p2_editorial_unclosed_lt");
    let app = build_test_router(&state);
    let (_, editor_cookie) = create_user_with_session(
        &state,
        "ce_t5_lt@msidergisi.com",
        "StaffPass!2026",
        "LT Editor",
        "content_editor",
        "Medya",
        "MSI",
        false,
        "",
    );

    let (st_save, _, save_val) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/admin/editorial/articles",
        Some(&editor_cookie),
        Some(json!({
            "id": "",
            "title": "Unclosed Tag Test",
            "status": "draft",
            "content": "",
            "blocks": [{ "id": "b-1", "type": "paragraph", "content": "<img src=x onerror=alert(1)" }]
        })),
    )
    .await;
    assert_eq!(st_save, StatusCode::OK, "save response: {:?}", save_val);
    let saved_id = save_val["article"]["id"].as_str().unwrap();

    let (st_get, _, get_bytes) = req_raw(
        &app,
        "GET",
        &format!("/api/v1/admin/editorial/articles/{}", saved_id),
        Some(&editor_cookie),
        &[],
        None,
    )
    .await;
    assert_eq!(st_get, StatusCode::OK);
    let get_val: Value = serde_json::from_slice(&get_bytes).unwrap();
    let block_content = get_val["article"]["blocks"][0]["content"]
        .as_str()
        .unwrap();
    assert!(!block_content.contains('<'), "Got raw '<': {}", block_content);
    assert!(block_content.starts_with("&lt;img"));

    cleanup_test_db(&db_path);
}

#[test]
fn t5_concurrent_refresh_l1_sections_no_lost_updates() {
    let (state, db_path) = build_test_state("t5_p2_l1_concurrent");
    state.db.refresh_l1_cache();

    let db_a = state.db.clone();
    let db_b = state.db.clone();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let bar_a = barrier.clone();
    let bar_b = barrier.clone();

    let pw_hash =
        auth_service::hash_password("StrongPass!2026", &state.config.session_secret);

    std::thread::scope(|s| {
        s.spawn(move || {
            bar_a.wait();
            for i in 0..20 {
                let mut u = fixture_user(8000 + i);
                u.id = 0;
                u.email = format!("t5_l1_race_{}@example.com", i);
                db_a.create_or_update_user(&u, &pw_hash).unwrap();
                db_a.refresh_l1_sections(&["users"]);
            }
        });
        s.spawn(move || {
            bar_b.wait();
            for i in 0..20 {
                let mut c = fixture_content(8000 + i);
                c.id = 0;
                c.slug = format!("t5-l1-race-content-{}", i);
                db_b.upsert_content(&c).unwrap();
                db_b.refresh_l1_sections(&["contents"]);
            }
        });
    });

    let all_users = state.db.get_all_users();
    let all_contents = state.db.get_all_contents();
    for i in 0..20 {
        let email = format!("t5_l1_race_{}@example.com", i);
        assert!(
            all_users.iter().any(|u| u.email == email),
            "Lost L1 update for user {}",
            email
        );
        let slug = format!("t5-l1-race-content-{}", i);
        assert!(
            all_contents.iter().any(|c| c.slug == slug),
            "Lost L1 update for content {}",
            slug
        );
    }

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_noindex_routes_omit_link_canonical_header() {
    let (state, db_path) = build_test_state("t5_p2_noindex_canonical");
    let app = build_test_router(&state);

    for path in ["/hesabim", "/sifre-sifirla"] {
        let (st, hdrs, body_bytes) = req_raw(&app, "GET", path, None, &[], None).await;
        assert_eq!(st, StatusCode::OK, "Expected 200 OK on {}", path);

        let robots = hdrs
            .get("x-robots-tag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            robots.contains("noindex"),
            "Expected x-robots-tag noindex on {} (got: {})",
            path,
            robots
        );

        let link_hdr = hdrs.get("link").and_then(|v| v.to_str().ok()).unwrap_or("");
        assert!(
            !link_hdr.contains("rel=\"canonical\""),
            "noindex route {} must NOT emit Link rel=\"canonical\" (got: {})",
            path,
            link_hdr
        );

        let html = String::from_utf8_lossy(&body_bytes);
        assert!(
            !html.contains("rel=\"canonical\""),
            "noindex route {} HTML must NOT emit <link rel=\"canonical\">",
            path
        );
    }

    cleanup_test_db(&db_path);
}

#[test]
fn t5_article_revision_i64_and_scheduled_publish_reverted_draft_guard() {
    let (state, db_path) = build_test_state("t5_p2_rev_i64_sched");

    // 1. ArticleRevision.id > i32::MAX round-trips without truncation
    let mut item = fixture_content(7401);
    item.id = 0;
    item.slug = "t5-p2-rev-i64-article".into();
    let cid = state.db.upsert_content(&item).unwrap();

    let big_rev_id: i64 = 5_000_000_123;
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "INSERT INTO content_versions (
                id, content_id, version, title_tr, title_en, body_tr, body_en,
                summary_tr, summary_en, changed_by, change_note, created_at, en_status
             ) VALUES (?1, ?2, 99, 'Büyük Revizyon Başlık', '', '<p>Büyük Revizyon</p>', '',
                       'Özet', '', 'editor@test.com', 'i64 test', '2026-11-22T12:00:00Z', 'none')",
            rusqlite::params![big_rev_id, cid],
        )
        .unwrap();
    }

    let revs = state.db.editorial_list_revisions(cid);
    assert!(
        revs.iter().any(|r| r.id == big_rev_id),
        "ArticleRevision.id {} must round-trip without i32 truncation: {:?}",
        big_rev_id,
        revs.iter().map(|r| r.id).collect::<Vec<_>>()
    );
    let restored = state
        .db
        .editorial_restore_revision(cid, big_rev_id, "editor@test.com")
        .unwrap();
    assert_eq!(restored.title, "Büyük Revizyon Başlık");

    // 2. Concurrent revert to 'draft' & double scheduler tick never leave stale/double publish
    let mut sched_item = fixture_content(7402);
    sched_item.id = 0;
    sched_item.slug = "t5-p2-sched-due-article".into();
    sched_item.status = "scheduled".into();
    sched_item.published_at = String::new();
    let sched_id = state.db.upsert_content(&sched_item).unwrap();
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "UPDATE contents SET scheduled_at = '2020-01-01 00:00:00', status = 'scheduled' WHERE id = ?1",
            [sched_id],
        )
        .unwrap();
    }

    // Two concurrent scheduler runs publish at most once
    let db1 = state.db.clone();
    let db2 = state.db.clone();
    let bar = std::sync::Arc::new(std::sync::Barrier::new(2));
    let bar1 = bar.clone();
    let bar2 = bar.clone();
    let (res1, res2) = std::thread::scope(|s| {
        let h1 = s.spawn(move || {
            bar1.wait();
            db1.editorial_publish_due_scheduled().unwrap()
        });
        let h2 = s.spawn(move || {
            bar2.wait();
            db2.editorial_publish_due_scheduled().unwrap()
        });
        (h1.join().unwrap(), h2.join().unwrap())
    });
    assert_eq!(
        res1.len() + res2.len(),
        1,
        "Concurrent scheduler ticks must publish the due article exactly once"
    );

    // Newly published scheduled item must be immediately visible under TR timezone
    let pub_obj = state
        .db
        .get_all_contents()
        .into_iter()
        .find(|c| c.id == sched_id)
        .unwrap();
    assert_eq!(pub_obj.status, "published");
    assert!(
        msi_dergisi_platform::services::content_service::is_publicly_visible(&pub_obj),
        "Scheduled article published_at '{}' must be immediately publicly visible",
        pub_obj.published_at
    );

    // Reverted draft article is NOT promoted to published
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "UPDATE contents SET status = 'draft', scheduled_at = '2020-01-01 00:00:00' WHERE id = ?1",
            [sched_id],
        )
        .unwrap();
    }
    let after_revert = state.db.editorial_publish_due_scheduled().unwrap();
    assert!(after_revert.is_empty());
    let final_status: String = state
        .db
        .open_conn()
        .unwrap()
        .query_row(
            "SELECT status FROM contents WHERE id = ?1",
            [sched_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(final_status, "draft");

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_content_version_rbac_and_html_sanitization() {
    let (state, db_path) = build_test_state("t5_p2_ver_rbac_xss");
    let app = build_test_router(&state);

    let mut pub_item = fixture_content(7501);
    pub_item.id = 0;
    pub_item.slug = "t5-p2-ver-pub-article".into();
    pub_item.title_tr = "Orijinal Yayin Basligi".into();
    pub_item.summary_tr = "Orijinal ozet".into();
    pub_item.body_tr = "<p>Guvenli yayin govdesi</p>".into();
    pub_item.status = "published".into();
    pub_item.visibility_level = 0;
    pub_item.published_at = "2099-01-01 10:00:00".into();
    let pub_id = state.db.upsert_content(&pub_item).unwrap();
    state
        .db
        .insert_content_version(
            pub_id,
            "Orijinal Yayin Basligi",
            "<p>Guvenli yayin govdesi</p>",
            "Orijinal ozet",
            "sa@msidergisi.com",
            "v1",
        )
        .unwrap();

    let mut draft_item = fixture_content(7502);
    draft_item.id = 0;
    draft_item.slug = "t5-p2-ver-draft-article".into();
    draft_item.title_tr = "Taslak Baslik".into();
    draft_item.body_tr = "<p>Taslak govde</p>".into();
    draft_item.status = "draft".into();
    let draft_id = state.db.upsert_content(&draft_item).unwrap();

    let (_, ce_cookie) = create_user_with_session(
        &state,
        "ce_t5_ver@msidergisi.com",
        "StaffPass!2026",
        "Version CE",
        "content_editor",
        "Medya",
        "MSI",
        false,
        "",
    );
    let (_, eic_cookie) = create_user_with_session(
        &state,
        "eic_t5_ver@msidergisi.com",
        "StaffPass!2026",
        "Version EIC",
        "editor_in_chief",
        "Medya",
        "MSI",
        false,
        "",
    );

    // 1. content_editor MUST be blocked (403) from creating/restoring versions or submit-review on published content
    let (st_ce_create, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        &format!("/api/v1/admin/contents/{}/versions", pub_id),
        Some(&ce_cookie),
        Some(json!({
            "content_id": pub_id,
            "title_tr": "HIJACKED_TITLE",
            "body_tr": "<p>Evil</p><script>alert(1)</script>",
            "summary_tr": "Evil summary",
            "change_note": "attempt"
        })),
    )
    .await;
    assert_eq!(st_ce_create, StatusCode::FORBIDDEN);

    let (st_ce_restore, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        &format!("/api/v1/admin/contents/{}/versions/1/restore", pub_id),
        Some(&ce_cookie),
        None,
    )
    .await;
    assert_eq!(st_ce_restore, StatusCode::FORBIDDEN);

    let (st_ce_submit, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        &format!("/api/v1/admin/contents/{}/submit-review", pub_id),
        Some(&ce_cookie),
        Some(json!({})),
    )
    .await;
    assert_eq!(st_ce_submit, StatusCode::FORBIDDEN);

    // 2. Path(id) vs payload.content_id mismatch, negative content_id, and missing content id MUST be rejected
    let (st_mismatch, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        &format!("/api/v1/admin/contents/{}/versions", draft_id),
        Some(&ce_cookie),
        Some(json!({
            "content_id": pub_id,
            "title_tr": "Cross-ID Injection",
            "body_tr": "<p>Test</p>",
            "summary_tr": "Test"
        })),
    )
    .await;
    assert_eq!(st_mismatch, StatusCode::BAD_REQUEST);

    let (st_neg_cid, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        &format!("/api/v1/admin/contents/{}/versions", draft_id),
        Some(&ce_cookie),
        Some(json!({
            "content_id": -1,
            "title_tr": "Negative ID",
            "body_tr": "<p>Test</p>",
            "summary_tr": "Test"
        })),
    )
    .await;
    assert_eq!(st_neg_cid, StatusCode::BAD_REQUEST);

    let (st_missing_id, _, missing_json) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/admin/contents/999999/versions",
        Some(&eic_cookie),
        Some(json!({
            "content_id": 999999,
            "title_tr": "Missing Content",
            "body_tr": "<p>Test</p>",
            "summary_tr": "Test"
        })),
    )
    .await;
    assert_eq!(st_missing_id, StatusCode::NOT_FOUND);
    assert_eq!(missing_json["error_code"], "NOT_FOUND");

    let (st_neg_restore, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        "/api/v1/admin/contents/-1/versions/1/restore",
        Some(&eic_cookie),
        None,
    )
    .await;
    assert_eq!(st_neg_restore, StatusCode::BAD_REQUEST);

    assert_eq!(
        state.db.get_content_versions(pub_id).len(),
        1,
        "Published article versions must remain untouched"
    );

    // 3. content_editor CAN create & restore versions on draft, and HTML/script payloads are sanitized
    let (st_draft_create, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        &format!("/api/v1/admin/contents/{}/versions", draft_id),
        Some(&ce_cookie),
        Some(json!({
            "content_id": draft_id,
            "title_tr": "<script>alert(9)</script>Temiz Taslak Baslik",
            "body_tr": "<p>Guvenli metin</p><script>alert(1)</script><a href=\"javascript:alert(2)\">x</a>",
            "summary_tr": "<img src=x onerror=alert(3)>Temiz ozet"
        })),
    )
    .await;
    assert_eq!(st_draft_create, StatusCode::OK);

    let draft_vers = state.db.get_content_versions(draft_id);
    assert_eq!(draft_vers.len(), 1);
    let ver_num = draft_vers[0]["version"].as_i64().unwrap();
    let stored_body = draft_vers[0]["body_tr"].as_str().unwrap();
    assert!(stored_body.contains("<p>Guvenli metin</p>"));
    assert!(!stored_body.contains("<script"));
    assert!(!stored_body.contains("javascript:"));
    assert_eq!(draft_vers[0]["title_tr"].as_str(), Some("Temiz Taslak Baslik"));
    assert_eq!(draft_vers[0]["summary_tr"].as_str(), Some("Temiz ozet"));

    // Insert a legacy unsanitized version row directly in DB and confirm restore sanitizes before writing to contents
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "INSERT INTO content_versions (
                content_id, version, title_tr, title_en, body_tr, body_en,
                summary_tr, summary_en, changed_by, change_note, created_at, en_status
             ) VALUES (?1, 99, '<script>bad</script>Legacy Title', '', '<p>Legacy</p><script>alert(77)</script><img src=x onerror=alert(88)>', '', 'Legacy Summary', '', 'legacy@msidergisi.com', 'legacy', '2026-11-22T00:00:00Z', 'none')",
            rusqlite::params![draft_id],
        )
        .unwrap();
    }
    let (st_draft_restore, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        &format!("/api/v1/admin/contents/{}/versions/99/restore", draft_id),
        Some(&ce_cookie),
        None,
    )
    .await;
    assert_eq!(st_draft_restore, StatusCode::OK);

    let updated_draft = state
        .db
        .get_all_contents()
        .into_iter()
        .find(|c| c.id == draft_id)
        .unwrap();
    assert_eq!(updated_draft.title_tr, "Legacy Title");
    assert!(updated_draft.body_tr.contains("<p>Legacy</p>"));
    assert!(!updated_draft.body_tr.contains("<script"));
    assert!(!updated_draft.body_tr.contains("onerror"));

    // 4. editor_in_chief (with contents:publish) can snapshot/restore published content cleanly (and HTML is sanitized in /feed.json)
    let (st_eic_create, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        &format!("/api/v1/admin/contents/{}/versions", pub_id),
        Some(&eic_cookie),
        Some(json!({
            "content_id": pub_id,
            "title_tr": "Yayinci Surumu",
            "body_tr": "<p>Yayin temiz</p><script>alert(1)</script><a href=\"javascript:alert(2)\">link</a>",
            "summary_tr": "Yayinci ozet",
            "change_note": "EIC rev"
        })),
    )
    .await;
    assert_eq!(st_eic_create, StatusCode::OK);
    let pub_vers = state.db.get_content_versions(pub_id);
    let latest_pub_ver = pub_vers[0]["version"].as_i64().unwrap();
    assert_ne!(latest_pub_ver, 1);

    let (st_eic_restore, _, _) = req_with_csrf(
        &app,
        &state,
        "POST",
        &format!(
            "/api/v1/admin/contents/{}/versions/{}/restore",
            pub_id, latest_pub_ver
        ),
        Some(&eic_cookie),
        None,
    )
    .await;
    assert_eq!(st_eic_restore, StatusCode::OK);
    assert_eq!(ver_num, 1);

    let (_, _, feed_bytes) = req_raw(&app, "GET", "/feed.json", None, &[], None).await;
    let feed_str = String::from_utf8_lossy(&feed_bytes);
    assert!(feed_str.contains("Yayinci Surumu"));
    assert!(!feed_str.contains("<script>alert(1)</script>"));
    assert!(!feed_str.contains("javascript:alert(2)"));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t5_svg_namespace_uri_style_bypass_and_static_headers() {
    let (mut state, db_path) = build_test_state("t5_p2_svg_ns_bypass");
    let temp_static = format!("{}_static", db_path);
    let mut cfg = (*state.config).clone();
    cfg.static_dir = temp_static.clone();
    state.config = std::sync::Arc::new(cfg);
    let app = build_test_router(&state);

    let (_, editor_cookie) = create_user_with_session(
        &state,
        "ce_t5_svg_ns@msidergisi.com",
        "StaffPass!2026",
        "SVG NS Tester",
        "content_editor",
        "Medya",
        "MSI",
        false,
        "",
    );

    let malicious_svgs: &[(&str, &str)] = &[
        (
            "ns_s_script.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:s=\"http://www.w3.org/2000/svg\"><s:script>alert(1)</s:script></svg>",
        ),
        (
            "ns_h_script.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:h=\"http://www.w3.org/1999/xhtml\"><h:script>alert(1)</h:script></svg>",
        ),
        (
            "ns_foreignobj.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:s=\"http://www.w3.org/2000/svg\"><s:foreignObject><div xmlns=\"http://www.w3.org/1999/xhtml\">x</div></s:foreignObject></svg>",
        ),
        (
            "ns_set.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:s=\"http://www.w3.org/2000/svg\"><s:a href=\"x\"><s:set attributeName=\"href\" to=\"javascript:alert(1)\"/></s:a></svg>",
        ),
        (
            "ns_animate.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:s=\"http://www.w3.org/2000/svg\"><s:animate attributeName=\"href\" values=\"javascript:alert(1)\"/></svg>",
        ),
        (
            "use_data_svg.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><use href=\"data:image/svg+xml;base64,PHN2Zy8+\"/></svg>",
        ),
        (
            "a_data_svg.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><a href=\"data:image/svg+xml;base64,PHN2Zy8+\"><rect width=\"5\" height=\"5\"/></a></svg>",
        ),
        (
            "style_import.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><style>@import url(\"https://evil.example/x.css\");</style><circle r=\"5\"/></svg>",
        ),
        (
            "ctrl_vtab_onload.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect\x0bonload=\"alert(1)\" width=\"5\" height=\"5\"/></svg>",
        ),
        (
            "ctrl_ff_onload.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect\x0conload=\"alert(1)\" width=\"5\" height=\"5\"/></svg>",
        ),
        (
            "pi_multibyte_g.svg",
            std::str::from_utf8(b"<?\xc4\x9f\xc4\x9f>").unwrap(),
        ),
        (
            "pi_multibyte_e.svg",
            std::str::from_utf8(b"<?\xc3\xa9\xc3\xa9>").unwrap(),
        ),
    ];

    for (endpoint, field_name) in [
        ("/api/v1/admin/editorial/upload", "image"),
        ("/api/v1/admin/upload", "file"),
    ] {
        for (fname, svg_body) in malicious_svgs {
            let csrf = security_engine::generate_csrf_token(&state.config.session_secret);
            let boundary = "----MSI_T5_BOUNDARY_SVG_NS";
            let multipart_body = format!(
                "--{b}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{fname}\"\r\nContent-Type: image/svg+xml\r\n\r\n{svg}\r\n--{b}--\r\n",
                b = boundary,
                field = field_name,
                fname = fname,
                svg = svg_body
            );
            let req = Request::builder()
                .method("POST")
                .uri(endpoint)
                .header(header::COOKIE, format!("{}; csrf_token={}", editor_cookie, csrf))
                .header("x-csrf-token", &csrf)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(multipart_body))
                .unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::BAD_REQUEST,
                "Bypass vector '{}' MUST return 400 on {}",
                fname,
                endpoint
            );
        }
    }

    // Valid SVG (with safe raster data:image/png) succeeds on both endpoints and is served with sandboxed CSP + nosniff
    let clean_svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><image href=\"data:image/png;base64,iVBORw0KGgo=\" width=\"10\" height=\"10\"/><circle cx=\"5\" cy=\"5\" r=\"4\"/></svg>";
    for (endpoint, field_name, expected_prefix) in [
        ("/api/v1/admin/editorial/upload", "image", "/uploads/editorial/"),
        ("/api/v1/admin/upload", "file", "images/uploads/"),
    ] {
        let csrf = security_engine::generate_csrf_token(&state.config.session_secret);
        let boundary = "----MSI_T5_BOUNDARY_SVG_CLEAN";
        let multipart_body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"clean-badge.svg\"\r\nContent-Type: image/svg+xml\r\n\r\n{svg}\r\n--{b}--\r\n",
            b = boundary,
            field = field_name,
            svg = clean_svg
        );
        let req = Request::builder()
            .method("POST")
            .uri(endpoint)
            .header(header::COOKIE, format!("{}; csrf_token={}", editor_cookie, csrf))
            .header("x-csrf-token", &csrf)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(multipart_body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Clean SVG failed on {}", endpoint);
        let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let body_json: Value = serde_json::from_slice(&bytes).unwrap();
        let uploaded_url = body_json["url"].as_str().unwrap();
        assert!(uploaded_url.starts_with(expected_prefix), "Unexpected URL: {}", uploaded_url);

        let get_path = if uploaded_url.starts_with('/') {
            uploaded_url.to_string()
        } else {
            format!("/{}", uploaded_url)
        };
        let stem_path = get_path.strip_suffix(".svg").expect("uploaded SVG path ends with .svg");
        for probe_path in [
            get_path.clone(),
            format!("{}.%73vg", stem_path),
            format!("{}%2esvg", stem_path),
            format!("{}%2e%73%76%67", stem_path),
        ] {
            let (st_get, hdrs_get, _) = req_raw(&app, "GET", &probe_path, None, &[], None).await;
            assert_eq!(st_get, StatusCode::OK, "Expected 200 OK for {}", probe_path);
            assert_eq!(
                hdrs_get.get("x-content-type-options").and_then(|v| v.to_str().ok()),
                Some("nosniff"),
                "Missing nosniff on {}",
                probe_path
            );
            assert_eq!(
                hdrs_get.get("content-security-policy").and_then(|v| v.to_str().ok()),
                Some("sandbox; default-src 'none'"),
                "Missing sandboxed SVG CSP on {}",
                probe_path
            );
        }
    }

    let _ = std::fs::remove_dir_all(&temp_static);
    cleanup_test_db(&db_path);
}
