//! Comprehensive Integration & Unit Tests for M6-B_API (Migration 015, M2 #10/#11, Option B B1–B14)

mod common;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use tower::ServiceExt;

use common::*;
use msi_dergisi_platform::{
    controllers::AppState,
    db::Database,
    engine::security_engine,
    models::AnalyticsEvent,
    services::{analytics_service, security_anomaly_service},
    workers::{b2b_report_worker, newsletter_scheduler},
};

async fn send_req(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    let req_body = if let Some(v) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(v.to_string())
    } else {
        Body::empty()
    };
    let resp = app
        .clone()
        .oneshot(builder.body(req_body).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = to_bytes(resp.into_body(), 8 * 1024 * 1024).await.unwrap();
    (status, headers, bytes.to_vec())
}

async fn get_text(
    app: &axum::Router,
    uri: &str,
    cookie: Option<&str>,
) -> (StatusCode, axum::http::HeaderMap, String) {
    let (st, hdrs, bytes) = send_req(app, "GET", uri, cookie, None).await;
    (st, hdrs, String::from_utf8_lossy(&bytes).to_string())
}

async fn post_json_resp(
    app: &axum::Router,
    uri: &str,
    cookie: Option<&str>,
    payload: Value,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let (st, hdrs, bytes) = send_req(app, "POST", uri, cookie, Some(payload)).await;
    let val = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (st, hdrs, val)
}

async fn post_admin_json_resp(
    app: &axum::Router,
    state: &AppState,
    uri: &str,
    session_cookie: &str,
    payload: Value,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let csrf = security_engine::generate_csrf_token(&state.config.session_secret);
    let combined_cookie = format!("{}; csrf_token={}", session_cookie, csrf);
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::COOKIE, combined_cookie)
        .header("x-csrf-token", &csrf)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let st = resp.status();
    let hdrs = resp.headers().clone();
    let bytes = to_bytes(resp.into_body(), 8 * 1024 * 1024).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (st, hdrs, val)
}

fn unwrap_data(v: &Value) -> &Value {
    if let Some(d) = v.get("data") {
        if !d.is_null() {
            return d;
        }
    }
    v
}

#[tokio::test]
async fn t01_migration_015_schema_l1_cache_and_insan_kaynaklari_seed() {
    let (state, db_path) = build_test_state("m6_015_mig");
    let app = build_test_router(&state);

    let conn = state.db.open_conn().unwrap();
    let user_ver: i64 = conn
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(user_ver, 17, "Expected PRAGMA user_version == 17");

    for tbl in ["kvkk_consent_logs", "b2b_monthly_reports", "revoked_tokens", "custom_field_values"] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![tbl],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "Table {} missing after 016", tbl);
    }

    for col in ["start_date", "end_date", "rotation_mode"] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('ad_campaigns') WHERE name = ?1",
                rusqlite::params![col],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "Column ad_campaigns.{} missing after 016", col);
    }

    // Verify ALTER TABLE columns from 015 exist
    for (tbl, col) in [
        ("users", "tos_accepted_at"),
        ("users", "tos_version"),
        ("analytics_events", "video_watch_pct"),
        ("contents", "target_institutions_json"),
        ("newsletter_campaigns", "scheduled_at"),
        ("newsletter_campaigns", "email_group_code"),
        ("newsletter_campaigns", "body_html"),
    ] {
        let count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1", tbl),
                rusqlite::params![col],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "Column {}.{} missing after 015", tbl, col);
    }

    // Simulate existing DB where insan-kaynaklari was missing and test idempotent seed hook
    conn.execute("DELETE FROM categories WHERE slug = 'insan-kaynaklari'", [])
        .unwrap();
    msi_dergisi_platform::db::seed::taxonomy::seed_option_b_categories(&conn).unwrap();
    let ik_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE slug = 'insan-kaynaklari'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ik_exists, 1);
    drop(conn);

    // L1CacheSnapshot includes institution_categories & recommendation_rules
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "INSERT INTO recommendation_rules (rule_name, source_tag_or_cat, target_institution, recommended_content_slug, priority, is_active)
             VALUES ('Test Rule', 'hava-araclari', 'MSB', 'kaan-analizi', 10, 1)",
            [],
        )
        .unwrap();
    }
    state.db.refresh_l1_sections(&["recommendation_rules"]);
    {
        let guard = state.db.l1_cache.read().unwrap();
        let snap = guard.as_ref().expect("L1 cache populated");
        assert!(
            !snap.institution_categories.is_empty(),
            "institution_categories must be loaded in L1CacheSnapshot"
        );
        assert!(
            !snap.recommendation_rules.is_empty(),
            "recommendation_rules must be loaded in L1CacheSnapshot"
        );
    }

    let (st, _, _) = get_text(&app, "/kategori/insan-kaynaklari", None).await;
    assert_eq!(st, StatusCode::OK);

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t02_m2_design_tokens_css_zero_inline_style_and_editor_v27() {
    let (state, db_path) = build_test_state("m6_m2_tokens");
    let app = build_test_router(&state);

    // Verify shared public/css/design-tokens.css exists on disk
    assert!(
        Path::new("public/css/design-tokens.css").exists(),
        "public/css/design-tokens.css must exist as shared token foundation"
    );

    // GET /css/design-tokens.css served with shared tokens + dynamic config
    let (st, hdrs, css) = get_text(&app, "/css/design-tokens.css", None).await;
    assert_eq!(st, StatusCode::OK);
    let ct = hdrs
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(ct.contains("text/css"), "Expected text/css, got {}", ct);
    assert!(
        css.contains("--ui-brand-accent") && css.contains(":root"),
        "Design tokens CSS missing --ui-brand-accent or :root"
    );

    // Zero <style tags in public HTML pages
    for route in ["/", "/en", "/coming-soon"] {
        let (st, _, html) = get_text(&app, route, None).await;
        assert_eq!(st, StatusCode::OK, "Route {} failed", route);
        assert!(
            html.contains("/css/design-tokens.css"),
            "Route {} missing <link> to /css/design-tokens.css",
            route
        );
        assert!(
            !html.to_ascii_lowercase().contains("<style"),
            "Route {} must emit zero <style> tags",
            route
        );
    }

    // Design studio HTML uses ?v=35 and zero ?v=27/?v=28/?v=29
    let (_, admin_cookie) = create_user_with_session(
        &state,
        "studio_op@msidergisi.com",
        "StudioPass123!",
        "Studio Op",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (st, _, studio_html) = get_text(&app, "/podmin/tasarim", Some(&admin_cookie)).await;
    assert_eq!(st, StatusCode::OK);
    assert!(studio_html.contains("?v=35"));
    assert!(!studio_html.contains("?v=27"));
    assert!(!studio_html.contains("?v=28"));
    assert!(!studio_html.contains("?v=29"));

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t03_b1_video_watch_pct_clamping_and_persistence() {
    let (state, db_path) = build_test_state("m6_b1_video");
    let app = build_test_router(&state);

    let (st, _, v1) = post_json_resp(
        &app,
        "/api/v1/analytics/track",
        None,
        json!({
            "id": 0,
            "session_id": "sess-video-85",
            "user_id": null,
            "organization_category": "",
            "organization_name": "",
            "event_type": "video_progress",
            "content_slug": "sample-video",
            "ad_id": null,
            "duration_sec": 30,
            "scroll_depth_pct": 20,
            "video_completed": false,
            "video_watch_pct": 85,
            "is_true_read": false,
            "ssb_tag": null,
            "created_at": ""
        }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let ev1_id = v1["event_id"].as_i64().unwrap();
    assert!(ev1_id > 0);
    assert_eq!(v1["video_watch_pct"], 85);

    let (st2, _, v2) = post_json_resp(
        &app,
        "/api/v1/analytics/track",
        None,
        json!({
            "id": 0,
            "session_id": "sess-video-120",
            "user_id": null,
            "organization_category": "",
            "organization_name": "",
            "event_type": "video_progress",
            "content_slug": "sample-video",
            "ad_id": null,
            "duration_sec": 60,
            "scroll_depth_pct": 90,
            "video_completed": false,
            "video_watch_pct": 120,
            "is_true_read": false,
            "ssb_tag": null,
            "created_at": ""
        }),
    )
    .await;
    assert_eq!(st2, StatusCode::OK);
    let ev2_id = v2["event_id"].as_i64().unwrap();
    assert_eq!(v2["video_watch_pct"], 100);

    let conn = state.db.open_conn().unwrap();
    let (pct1, comp1): (i32, i32) = conn
        .query_row(
            "SELECT video_watch_pct, video_completed FROM analytics_events WHERE id = ?1",
            rusqlite::params![ev1_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(pct1, 85);
    assert_eq!(comp1, 0);

    let (pct2, comp2): (i32, i32) = conn
        .query_row(
            "SELECT video_watch_pct, video_completed FROM analytics_events WHERE id = ?1",
            rusqlite::params![ev2_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(pct2, 100);
    assert_eq!(comp2, 1);

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t04_b2_tos_kvkk_consent_logs_and_account_erasure() {
    let (state, db_path) = build_test_state("m6_b2_kvkk");
    let app = build_test_router(&state);

    let past_ts = chrono::Utc::now().timestamp() - 5;

    // 1. Explicit tos_accepted: false must be rejected 400
    let (st_rej, _, _) = post_json_resp(
        &app,
        "/api/v1/auth/register",
        None,
        json!({
            "email": "b2_reject@example.com",
            "full_name": "Reject ToS",
            "password": "StrongPassword!1",
            "kvkk_consent": true,
            "kvkk_acknowledged": true,
            "tos_accepted": false,
            "tos_version": "2026-01",
            "website_url": "",
            "_form_ts": past_ts
        }),
    )
    .await;
    assert_eq!(st_rej, StatusCode::BAD_REQUEST);

    // 1b. Omitted tos_accepted WITH explicit tos_version must be rejected 400 (T2-83)
    let (st_omitted_ver, _, _) = post_json_resp(
        &app,
        "/api/v1/auth/register",
        None,
        json!({
            "email": "b2_omitted_tos@example.com",
            "full_name": "Omitted ToS",
            "password": "StrongPassword!1",
            "kvkk_consent": true,
            "kvkk_acknowledged": true,
            "tos_version": "2026-01",
            "website_url": "",
            "_form_ts": past_ts
        }),
    )
    .await;
    assert_eq!(st_omitted_ver, StatusCode::BAD_REQUEST);
    assert!(state.db.get_user_by_email("b2_omitted_tos@example.com").is_none());

    // 1c. Legacy compatibility: omitting BOTH tos_accepted and tos_version succeeds (T1-F04-04 / T4-03)
    let (st_legacy, _, _) = post_json_resp(
        &app,
        "/api/v1/auth/register",
        None,
        json!({
            "email": "b2_legacy_tos@example.com",
            "full_name": "Legacy ToS",
            "password": "StrongPassword!1",
            "kvkk_consent": true,
            "kvkk_acknowledged": true,
            "website_url": "",
            "_form_ts": past_ts
        }),
    )
    .await;
    assert_eq!(st_legacy, StatusCode::OK);

    // 2. Valid registration with tos_accepted: true persists ToS & inserts kvkk_consent_logs
    let (st_ok, hdrs, reg_json) = post_json_resp(
        &app,
        "/api/v1/auth/register",
        None,
        json!({
            "email": "b2_reader@example.com",
            "full_name": "B2 Reader",
            "password": "StrongPassword!1",
            "organization_category": "Sektör Kuruluşları",
            "organization_name": "SAVTEK",
            "kvkk_consent": true,
            "kvkk_acknowledged": true,
            "tos_accepted": true,
            "tos_version": "2026-01",
            "email_consent": true,
            "ssb_interests": ["hava-araclari"],
            "website_url": "",
            "_form_ts": past_ts
        }),
    )
    .await;
    assert_eq!(st_ok, StatusCode::OK);
    let uid = reg_json["user"]["id"].as_i64().unwrap();
    assert_eq!(reg_json["user"]["tos_version"], "2026-01");
    assert!(!reg_json["user"]["tos_accepted_at"].as_str().unwrap_or("").is_empty());

    let set_cookie = hdrs.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    let session_cookie = set_cookie.split(';').next().unwrap().to_string();

    let logs = state.db.list_kvkk_consent_logs_for_user(uid, "b2_reader@example.com");
    assert!(logs.iter().any(|l| l.consent_type == "kvkk" && l.accepted));
    assert!(logs.iter().any(|l| l.consent_type == "tos" && l.accepted && l.consent_version == "2026-01"));
    assert!(logs.iter().any(|l| l.consent_type == "etk" && l.accepted));

    // 3. Authenticated /hesabim renders #account-delete-form (with required pw), #account-kvkk-card, #personalized-feed-band
    let (st_acc, _, acc_html) = get_text(&app, "/hesabim", Some(&session_cookie)).await;
    assert_eq!(st_acc, StatusCode::OK);
    assert!(acc_html.contains("id=\"account-delete-form\""));
    assert!(acc_html.contains("name=\"confirm_erasure\""));
    assert!(acc_html.contains("name=\"password\" autocomplete=\"current-password\" placeholder=\"••••••••\" required"));
    assert!(acc_html.contains("id=\"account-kvkk-card\""));
    assert!(acc_html.contains("id=\"personalized-feed-band\""));

    // 3b. EN /hesabim?lang=en localizes KVKK table headers
    let (st_acc_en, _, acc_en_html) = get_text(&app, "/hesabim?lang=en", Some(&session_cookie)).await;
    assert_eq!(st_acc_en, StatusCode::OK);
    assert!(acc_en_html.contains("<th scope=\"col\">Type</th><th scope=\"col\">Version</th><th scope=\"col\">Status</th><th scope=\"col\">Date</th>"));
    assert!(!acc_en_html.contains("<th scope=\"col\">Tür</th>"));

    // 3c. KVKK data export includes tos_accepted_at and tos_version under profile AND consent_records
    let (st_exp_pre, _, exp_txt) = get_text(&app, "/api/v1/account/export", Some(&session_cookie)).await;
    assert_eq!(st_exp_pre, StatusCode::OK);
    let exp_json: Value = serde_json::from_str(&exp_txt).unwrap();
    assert_eq!(exp_json["profile"]["tos_version"], "2026-01");
    let prof_tos_at = exp_json["profile"]["tos_accepted_at"].as_str().unwrap_or("");
    assert!(!prof_tos_at.is_empty() && (prof_tos_at.ends_with('Z') || prof_tos_at.ends_with("+00:00")));
    assert_eq!(exp_json["consent_records"]["tos_version"], "2026-01");

    // 4. Updating profile ETK preference inserts new 'etk' consent log row
    let (st_prof, _, _) = post_admin_json_resp(
        &app,
        &state,
        "/api/v1/account/profile",
        &session_cookie,
        json!({
            "full_name": "B2 Reader Updated",
            "email_consent": false
        }),
    )
    .await;
    assert_eq!(st_prof, StatusCode::OK);
    let logs_after = state.db.list_kvkk_consent_logs_for_user(uid, "b2_reader@example.com");
    assert!(logs_after.iter().any(|l| l.consent_type == "etk" && !l.accepted));

    // 5a. Empty/omitted password delete returns 400 (account stays intact)
    let (st_empty_pw, _, _) = post_admin_json_resp(
        &app,
        &state,
        "/api/v1/account/delete",
        &session_cookie,
        json!({}),
    )
    .await;
    assert_eq!(st_empty_pw, StatusCode::BAD_REQUEST);
    let (st_blank_pw, _, _) = post_admin_json_resp(
        &app,
        &state,
        "/api/v1/account/delete",
        &session_cookie,
        json!({ "password": "" }),
    )
    .await;
    assert_eq!(st_blank_pw, StatusCode::BAD_REQUEST);

    // 5b. Cross-origin Origin header on /api/v1/account/delete is rejected with 403
    let cross_req = Request::builder()
        .method("POST")
        .uri("/api/v1/account/delete")
        .header(header::COOKIE, &session_cookie)
        .header(header::ORIGIN, "https://evil.attacker.example")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "password": "StrongPassword!1" }).to_string()))
        .unwrap();
    let cross_resp = app.clone().oneshot(cross_req).await.unwrap();
    assert_eq!(cross_resp.status(), StatusCode::FORBIDDEN);

    // 5c. Wrong password delete returns 401
    let (st_bad_pw, _, _) = post_admin_json_resp(
        &app,
        &state,
        "/api/v1/account/delete",
        &session_cookie,
        json!({ "password": "WrongPassword!9" }),
    )
    .await;
    assert_eq!(st_bad_pw, StatusCode::UNAUTHORIZED);

    // 6. Valid password delete returns 200, revokes session, anonymizes user & logs erasure
    let (st_del, _, _) = post_admin_json_resp(
        &app,
        &state,
        "/api/v1/account/delete",
        &session_cookie,
        json!({ "password": "StrongPassword!1" }),
    )
    .await;
    assert_eq!(st_del, StatusCode::OK);

    let (st_exp, _, _) = get_text(&app, "/api/v1/account/export", Some(&session_cookie)).await;
    assert_eq!(st_exp, StatusCode::UNAUTHORIZED);

    let erased_user = state.db.get_user_by_id(uid).unwrap();
    assert_eq!(erased_user.role, "deleted");
    assert!(erased_user.email.ends_with("@erased.local"));

    let erased_logs = state.db.list_kvkk_consent_logs_for_user(uid, "");
    assert!(erased_logs.iter().all(|l| !l.email.contains("b2_reader@example.com")));
    assert!(erased_logs.iter().any(|l| l.consent_type == "erasure"));

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t05_b3_anonymized_analytics_k_anonymity_and_pii_masking() {
    let (state, db_path) = build_test_state("m6_b3_anon");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "b3_admin@msidergisi.com",
        "AdminPass123!",
        "B3 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (_, _) = create_user_with_session(
        &state,
        "ali.yilmaz@roketsan.com.tr",
        "UserPass123!",
        "Ali Yılmaz",
        "subscriber_corporate",
        "Sektör Kuruluşları",
        "Roketsan",
        false,
        "Kurumsal",
    );

    let mut content = fixture_content(901);
    content.id = 0;
    content.slug = "b3-test-article".into();
    state.db.upsert_content(&content).unwrap();

    // Seed 4 MSB (>=3), 2 SSB (<3), 1 Roketsan (<3) -> pooled sum = 3 (>=3)
    for (i, org) in ["MSB", "MSB", "MSB", "MSB", "SSB", "SSB", "Roketsan"]
        .iter()
        .enumerate()
    {
        state
            .db
            .insert_analytics_event(&AnalyticsEvent {
                id: 0,
                session_id: format!("b3-sess-{}", i),
                user_id: None,
                organization_category: org.to_string(),
                organization_name: org.to_string(),
                event_type: "article_read".to_string(),
                content_slug: Some("b3-test-article".to_string()),
                ad_id: None,
                duration_sec: 40,
                scroll_depth_pct: 80,
                video_completed: false,
                is_true_read: true,
                ssb_tag: Some("hava-araclari".to_string()),
                created_at: chrono::Utc::now().to_rfc3339(),
                video_watch_pct: 0,
            })
            .unwrap();
    }

    // Raw bundle includes PII and small buckets
    let (st_raw, _, raw_body) = get_text(&app, "/api/v1/admin/analytics/bundle", Some(&admin_cookie)).await;
    assert_eq!(st_raw, StatusCode::OK);
    assert!(raw_body.contains("ali.yilmaz@roketsan.com.tr"));
    assert!(raw_body.contains("\"SSB\""));

    // Anonymized bundle masks PII and suppresses k < 3 cohorts
    let (st_anon, _, anon_body) =
        get_text(&app, "/api/v1/admin/analytics/bundle?anonymize=1", Some(&admin_cookie)).await;
    assert_eq!(st_anon, StatusCode::OK);
    assert!(!anon_body.contains("ali.yilmaz@roketsan.com.tr"));
    assert!(!anon_body.contains("Ali Yılmaz"));
    assert!(!anon_body.contains("\"Roketsan\""));

    let parsed: Value = serde_json::from_str(&anon_body).unwrap();
    let readers = parsed["readers"].as_array().unwrap();
    for r in readers {
        assert_eq!(r["user_id"], 0);
        assert!(r["full_name"].as_str().unwrap().starts_with("Okur #"));
        assert!(!r["email"].as_str().unwrap().contains('@'));
    }

    let c_rep = parsed["contents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["content_slug"] == "b3-test-article")
        .unwrap();
    let breakdown = c_rep["institution_breakdown"].as_array().unwrap();
    assert!(breakdown.iter().any(|b| b["organization_category"] == "MSB" && b["count"] == 4));
    assert!(breakdown
        .iter()
        .any(|b| b["organization_category"] == "Diğer Kurumlar (k<3)" && b["count"] == 3));
    assert!(!breakdown.iter().any(|b| b["count"].as_i64().unwrap() < 3));

    // Edge case: suppressed sum < 3 (single small bucket of 2) must be omitted completely
    let dropped = analytics_service::apply_k_anonymity_to_institution_counts(
        vec![("MSB".into(), 5), ("TinyOrg".into(), 2)],
        3,
    );
    assert_eq!(dropped.len(), 1);
    assert_eq!(dropped[0].organization_category, "MSB");

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t06_b4_b5_b6_b12_b14_public_feeds_routes_and_hubs() {
    let (state, db_path) = build_test_state("m6_b4_b12");
    let app = build_test_router(&state);

    let (st_pers, _, pers_txt) =
        get_text(&app, "/api/v1/public/personalized?lang=tr&limit=6", None).await;
    assert_eq!(st_pers, StatusCode::OK);
    let pers_json: Value = serde_json::from_str(&pers_txt).unwrap();
    let items = unwrap_data(&pers_json)["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert!(items.iter().all(|i| i["is_sponsored"] == false));

    let (st_rel, _, rel_txt) =
        get_text(&app, "/api/v1/public/related?lang=tr&limit=4", None).await;
    assert_eq!(st_rel, StatusCode::OK);
    let rel_json: Value = serde_json::from_str(&rel_txt).unwrap();
    assert!(unwrap_data(&rel_json)["items"].as_array().unwrap().len() >= 3);

    // Sponsored current article suppresses /api/v1/public/related
    let mut sp = fixture_content(902);
    sp.id = 0;
    sp.slug = "b5-sponsored-probe".into();
    sp.is_sponsored = true;
    state.db.upsert_content(&sp).unwrap();
    let (st_sp_rel, _, sp_rel_txt) = get_text(
        &app,
        "/api/v1/public/related?slug=b5-sponsored-probe&lang=tr",
        None,
    )
    .await;
    assert_eq!(st_sp_rel, StatusCode::OK);
    let sp_rel_json: Value = serde_json::from_str(&sp_rel_txt).unwrap();
    assert_eq!(unwrap_data(&sp_rel_json)["items"].as_array().unwrap().len(), 0);

    // B6: /kurumsal/biz-kimiz is distinct from hakkimizda & /en/kurumsal/* works
    let (st_bk, _, bk_html) = get_text(&app, "/kurumsal/biz-kimiz", None).await;
    assert_eq!(st_bk, StatusCode::OK);
    assert!(bk_html.contains("Bağımsız Savunma Gazeteciliği İlkelerimiz"));
    assert!(!bk_html.contains("2004'ten bugüne aylık Türkçe ve İngilizce"));

    let (st_en_corp, _, en_corp_html) = get_text(&app, "/en/kurumsal/who-we-are", None).await;
    assert_eq!(st_en_corp, StatusCode::OK);
    assert!(en_corp_html.contains("lang=\"en\""));

    // B12 & B14 SSR Hubs
    for route in [
        "/etkinlikler",
        "/en/events",
        "/yazarlar",
        "/en/authors",
        "/savunma-sanayii-tarihi",
        "/en/defence-industry-history",
        "/ogrenci-kulupleri",
        "/en/student-clubs",
    ] {
        let (st, _, html) = get_text(&app, route, None).await;
        assert_eq!(st, StatusCode::OK, "Hub route {} failed", route);
        assert!(html.contains("id=\"related-strip\""), "Missing #related-strip on {}", route);
    }

    let (_, _, ev_html) = get_text(&app, "/etkinlikler", None).await;
    assert!(ev_html.contains("id=\"events-hub\""));
    assert!(ev_html.contains("id=\"dossier-toc\""));

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t07_b7_public_newsletter_subscribe_and_scheduler_tick_idempotency() {
    let (state, db_path) = build_test_state("m6_b7_sched");
    let app = build_test_router(&state);

    let past_ts = chrono::Utc::now().timestamp() - 5;
    let (st_sub, _, sub_json) = post_json_resp(
        &app,
        "/api/v1/public/newsletter/subscribe",
        None,
        json!({
            "email": "b7_subscriber@example.com",
            "full_name": "B7 Subscriber",
            "email_group_code": "edergi-releases",
            "kvkk_consent": true,
            "email_consent": true,
            "lang": "tr",
            "website_url": "",
            "_form_ts": past_ts
        }),
    )
    .await;
    assert_eq!(st_sub, StatusCode::OK);
    assert_eq!(unwrap_data(&sub_json)["subscribed"], true);

    let _ = create_user_with_session(
        &state,
        "gov_reader@msb.gov.tr",
        "GovPass123!",
        "Gov Reader",
        "subscriber_public",
        "MSB",
        "MSB",
        true,
        "Kamu E-Dergi Abonesi",
    );

    let past_sched = (chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
    let future_sched = (chrono::Utc::now() + chrono::Duration::days(5)).to_rfc3339();

    let due_id = state
        .db
        .insert_newsletter_campaign(&msi_dergisi_platform::models::NewsletterCampaign {
            id: 0,
            subject_tr: "Due Campaign".into(),
            subject_en: "Due Campaign EN".into(),
            target_group_filter: "gov_verified".into(),
            target_institution: String::new(),
            target_ssb_tag: String::new(),
            sponsor_ad_id: None,
            sponsor_name: None,
            recipient_count: 0,
            status: "scheduled".into(),
            sent_at: String::new(),
            scheduled_at: Some(past_sched),
            email_group_code: Some("gov_verified".into()),
            body_html: "<p>Scheduled Content</p>".into(),
        })
        .unwrap();

    let future_id = state
        .db
        .insert_newsletter_campaign(&msi_dergisi_platform::models::NewsletterCampaign {
            id: 0,
            subject_tr: "Future Campaign".into(),
            subject_en: "Future Campaign EN".into(),
            target_group_filter: "all".into(),
            target_institution: String::new(),
            target_ssb_tag: String::new(),
            sponsor_ad_id: None,
            sponsor_name: None,
            recipient_count: 0,
            status: "scheduled".into(),
            sent_at: String::new(),
            scheduled_at: Some(future_sched),
            email_group_code: None,
            body_html: "<p>Future</p>".into(),
        })
        .unwrap();

    let processed1 = newsletter_scheduler::run_newsletter_scheduler_tick(&state).await;
    assert_eq!(processed1, 1, "Expected exactly 1 due campaign processed");

    let all = state.db.get_newsletter_campaigns();
    let due_after = all.iter().find(|c| c.id == due_id).unwrap();
    let future_after = all.iter().find(|c| c.id == future_id).unwrap();
    assert_ne!(due_after.status, "scheduled");
    assert!(due_after.recipient_count >= 1);
    assert_eq!(future_after.status, "scheduled");

    // Second tick must be idempotent (0 campaigns processed)
    let processed2 = newsletter_scheduler::run_newsletter_scheduler_tick(&state).await;
    assert_eq!(processed2, 0);

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t08_b8_b2b_monthly_report_worker_and_admin_endpoint() {
    let (state, db_path) = build_test_state("m6_b8_rep");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "b8_admin@msidergisi.com",
        "AdminPass123!",
        "B8 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (_, reader_cookie) = create_user_with_session(
        &state,
        "b8_reader@example.com",
        "UserPass123!",
        "B8 Reader",
        "registered",
        "Akademi",
        "ODTÜ",
        false,
        "free",
    );

    let conn = state.db.open_conn().unwrap();
    conn.execute(
        "INSERT INTO order_invoices (invoice_no, user_email, full_name, organization_name, item_type, item_code, item_title, amount_try, payment_status, created_at, pdf_watermark_code)
         VALUES ('MSI-B8-001', 'b8_reader@example.com', 'B8 Reader', 'ODTÜ', 'subscription', 'dijital', 'Dijital', 450.0, 'paid', '2026-10-15T10:00:00Z', 'WM-B8')",
        [],
    )
    .unwrap();
    drop(conn);

    let run1 = b2b_report_worker::run_b2b_report_tick(&state, Some("2026-10"))
        .await
        .unwrap();
    assert_eq!(run1, 1);
    let run2 = b2b_report_worker::run_b2b_report_tick(&state, Some("2026-10"))
        .await
        .unwrap();
    assert_eq!(run2, 0, "B2B monthly report must deduplicate per month/type/recipient");

    let reports = state.db.list_b2b_monthly_reports(Some("2026-10"));
    assert_eq!(reports.len(), 1);
    let summary: Value = serde_json::from_str(&reports[0].summary_json).unwrap();
    assert!(summary["monthly_revenue_try"].as_f64().unwrap() >= 450.0);

    let (st_adm, _, rep_txt) =
        get_text(&app, "/api/v1/admin/reports/subscribers?month=2026-10", Some(&admin_cookie)).await;
    assert_eq!(st_adm, StatusCode::OK);
    let rep_json: Value = serde_json::from_str(&rep_txt).unwrap();
    let data = unwrap_data(&rep_json);
    assert!(data["total_users"].is_number());
    assert!(data["active_subscribers"].is_number());
    assert!(data["gov_verified_count"].is_number());
    assert!(data.get("institution_breakdown").is_some());

    let (st_denied, _, _) =
        get_text(&app, "/api/v1/admin/reports/subscribers?month=2026-10", Some(&reader_cookie)).await;
    assert_eq!(st_denied, StatusCode::FORBIDDEN);

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t09_b9_backup_worker_online_integrity_sha256_and_retention() {
    let (state, db_path) = build_test_state("m6_b9_bkp");
    let app = build_test_router(&state);
    let backup_dir = state.config.backup_dir.clone();
    std::fs::create_dir_all(&backup_dir).unwrap();

    // Create expired automated backup (> 14 days) + pre_deploy backup (must be preserved)
    let expired_db = format!("{}/msi_backup_20000101_000000.db", backup_dir);
    let expired_json = format!("{}.json", expired_db);
    let pre_deploy_db = format!("{}/pre_deploy_20000101_000000.db", backup_dir);
    std::fs::write(&expired_db, b"old-backup").unwrap();
    std::fs::write(&expired_json, b"{}").unwrap();
    std::fs::write(&pre_deploy_db, b"pre-deploy-keep").unwrap();
    let _ = std::process::Command::new("touch")
        .args(["-d", "2020-01-01", &expired_db, &expired_json, &pre_deploy_db])
        .status();

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "b9_admin@msidergisi.com",
        "AdminPass123!",
        "B9 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );

    let (st, _, res_json) =
        post_admin_json_resp(&app, &state, "/api/v1/admin/backup", &admin_cookie, json!({})).await;
    assert_eq!(st, StatusCode::OK);
    let data = unwrap_data(&res_json);
    assert_eq!(data["status"], "completed");
    assert_eq!(data["integrity_check"], "ok");
    let bkp_path = data["path"].as_str().unwrap();
    let sha_claimed = data["sha256"].as_str().unwrap();
    assert_eq!(sha_claimed.len(), 64);

    let bkp_bytes = std::fs::read(bkp_path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&bkp_bytes);
    let sha_actual = hex::encode(hasher.finalize());
    assert_eq!(sha_claimed, sha_actual);
    assert!(Path::new(&format!("{}.json", bkp_path)).exists());

    // Expired msi_backup_* pruned, pre_deploy_* preserved
    assert!(!Path::new(&expired_db).exists());
    assert!(Path::new(&pre_deploy_db).exists());

    let _ = std::fs::remove_dir_all(&backup_dir);
    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t10_b10_security_anomaly_detection_admin_visibility_and_cooldown() {
    let (state, db_path) = build_test_state("m6_b10_sec");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "b10_admin@msidergisi.com",
        "AdminPass123!",
        "B10 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );

    // Clean DB produces 0 alerts
    assert_eq!(security_anomaly_service::evaluate_and_emit_anomalies(&state.db), 0);

    // Trigger SQLi WAF probe -> logs and evaluates immediately
    let (st_waf, _, _) = get_text(&app, "/arama?q=%27%20UNION%20SELECT%201%2C2%2C3--", None).await;
    assert_eq!(st_waf, StatusCode::FORBIDDEN);

    // Admin notifications exposes the alert; public notifications does NOT
    let (st_adm, _, adm_txt) =
        get_text(&app, "/api/v1/admin/notifications", Some(&admin_cookie)).await;
    assert_eq!(st_adm, StatusCode::OK);
    let adm_json: Value = serde_json::from_str(&adm_txt).unwrap();
    let adm_notifs = adm_json["notifications"].as_array().unwrap();
    let waf_alerts: Vec<_> = adm_notifs
        .iter()
        .filter(|n| n["notification_type"] == "security_anomaly_waf")
        .collect();
    assert_eq!(waf_alerts.len(), 1);
    assert_eq!(waf_alerts[0]["target_audience"], "admin");
    assert_eq!(waf_alerts[0]["severity"], "critical");

    let (st_pub, _, pub_txt) = get_text(&app, "/api/v1/notifications", None).await;
    assert_eq!(st_pub, StatusCode::OK);
    assert!(!pub_txt.contains("security_anomaly_waf"));

    // Second evaluation within cooldown deduplicates
    assert_eq!(security_anomaly_service::evaluate_and_emit_anomalies(&state.db), 0);

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t11_b11_media_guide_and_b13_institution_gating() {
    let (state, db_path) = build_test_state("m6_b11_b13");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "b11_admin@msidergisi.com",
        "AdminPass123!",
        "B11 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (_, digital_cookie) = create_user_with_session(
        &state,
        "b13_digital@example.com",
        "UserPass123!",
        "B13 Digital",
        "subscriber_digital",
        "Akademi",
        "SAVTEK",
        false,
        "Dijital",
    );
    let (_, public_cookie) = create_user_with_session(
        &state,
        "b13_public@msb.gov.tr",
        "UserPass123!",
        "B13 Public",
        "subscriber_public",
        "Kamu & TSK",
        "MSB",
        true,
        "Kamu E-Dergi Abonesi",
    );

    // B11: Media spec guide in admin #tab-10
    let (st_dash, _, dash_html) = get_text(&app, "/podmin", Some(&admin_cookie)).await;
    assert_eq!(st_dash, StatusCode::OK);
    assert!(dash_html.contains("media-spec-guide-card"));
    assert!(dash_html.contains("1600 × 900"));
    assert!(dash_html.contains("970 × 90"));
    assert!(dash_html.contains("300 × 900"));
    assert!(dash_html.contains("25 MB"));

    // B13: Create institution-targeted content
    let mut item = fixture_content(950);
    item.id = 0;
    item.slug = "b13-kamu-exclusive".into();
    item.visibility_level = 0;
    item.is_sponsored = false;
    item.body_tr = "<p>İlk cümle. İkinci cümle. B13_SECRET_EXCLUSIVE üçüncü cümle.</p>".into();
    item.target_institutions = vec!["Kamu & TSK".to_string()];
    state.db.upsert_content(&item).unwrap();

    // Guest & non-matching digital subscriber are locked
    for ck in [None, Some(digital_cookie.as_str())] {
        let (st, _, txt) = get_text(&app, "/api/v1/contents/b13-kamu-exclusive", ck).await;
        assert_eq!(st, StatusCode::OK);
        let j: Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(j["paywall"]["is_locked"], true);
        assert!(!j["paywall"]["rendered_body"].as_str().unwrap().contains("B13_SECRET_EXCLUSIVE"));
    }

    // Matching public sector subscriber & staff are unlocked
    for ck in [Some(public_cookie.as_str()), Some(admin_cookie.as_str())] {
        let (st, _, txt) = get_text(&app, "/api/v1/contents/b13-kamu-exclusive", ck).await;
        assert_eq!(st, StatusCode::OK);
        let j: Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(j["paywall"]["is_locked"], false);
        assert!(j["paywall"]["rendered_body"].as_str().unwrap().contains("B13_SECRET_EXCLUSIVE"));
    }

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t12_b14_cart_checkout_and_download_entitlement() {
    let (mut state, db_path) = build_test_state("m6_b14_cart");
    let mut cfg = (*state.config).clone();
    cfg.paytr_merchant_id = String::new();
    state.config = std::sync::Arc::new(cfg);
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "b14_admin@msidergisi.com",
        "AdminPass123!",
        "B14 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (_, buyer_cookie) = create_user_with_session(
        &state,
        "b14_buyer@example.com",
        "UserPass123!",
        "B14 Buyer",
        "subscriber_digital",
        "Akademi",
        "SAVTEK",
        false,
        "Dijital",
    );

    let (st_store, _, store_html) = get_text(&app, "/abonelik-ve-satis", None).await;
    assert_eq!(st_store, StatusCode::OK);
    assert!(store_html.matches("data-add-to-cart").count() >= 2);
    assert!(store_html.contains("data-cart-toggle"));

    let products = state.db.get_commerce_products();
    assert!(products.len() >= 2);
    let sku1 = products[0].sku.clone();
    let sku2 = products[1].sku.clone();
    let expected_total =
        ((products[0].price_try * 2.0 + products[1].price_try) * 100.0).round() / 100.0;

    // Cart checkout computes server-authoritative total
    let (st_co, _, co_json) = post_admin_json_resp(
        &app,
        &state,
        "/api/v1/commerce/cart-checkout",
        &buyer_cookie,
        json!({
            "email": "b14_buyer@example.com",
            "full_name": "B14 Buyer",
            "organization_name": "SAVTEK",
            "distance_sales_consent": true,
            "items": [
                { "item_type": "product", "item_code": sku1, "qty": 2 },
                { "item_type": "product", "item_code": sku2, "qty": 1 }
            ]
        }),
    )
    .await;
    assert!(st_co == StatusCode::ACCEPTED || st_co == StatusCode::OK);
    let inv_id = co_json["invoice_id"].as_i64().unwrap();
    assert!(inv_id > 0);
    assert!((co_json["amount_try"].as_f64().unwrap() - expected_total).abs() < 0.01);

    // Before mark-paid: anonymous gets 401, non-purchasing subscriber gets 403
    let (st_anon_dl, _, _) =
        get_text(&app, &format!("/api/v1/commerce/download/{}", sku1), None).await;
    assert_eq!(st_anon_dl, StatusCode::UNAUTHORIZED);

    let (st_unpaid_dl, _, _) = get_text(
        &app,
        &format!("/api/v1/commerce/download/{}", sku1),
        Some(&buyer_cookie),
    )
    .await;
    assert_eq!(st_unpaid_dl, StatusCode::FORBIDDEN);

    // Admin marks invoice paid
    let (st_mp, _, _) = post_admin_json_resp(
        &app,
        &state,
        &format!("/api/v1/admin/invoices/{}/mark-paid", inv_id),
        &admin_cookie,
        json!({}),
    )
    .await;
    assert_eq!(st_mp, StatusCode::OK);

    // Now buyer can download BOTH purchased SKUs from the multi-item cart!
    for sku in [&sku1, &sku2] {
        let (st_dl, hdrs, body) = send_req(
            &app,
            "GET",
            &format!("/api/v1/commerce/download/{}", sku),
            Some(&buyer_cookie),
            None,
        )
        .await;
        assert_eq!(st_dl, StatusCode::OK, "Paid download failed for {}", sku);
        assert!(body.starts_with(b"%PDF-"), "Expected real PDF bytes for {}", sku);
        assert!(hdrs.get(header::CONTENT_DISPOSITION).is_some());
    }

    cleanup_test_db(&db_path);
}


#[test]
fn t13_fresh_db_and_live_copy_migrate_to_15_cleanly() {
    let live_path = "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/storage/db/msi_platform.db";
    if !Path::new(live_path).exists() {
        return;
    }
    let tmp_path = test_db_path("live_mig015");
    std::fs::copy(live_path, &tmp_path).unwrap();
    for suffix in ["-wal", "-shm"] {
        let src = format!("{}{}", live_path, suffix);
        if Path::new(&src).exists() {
            let _ = std::fs::copy(&src, format!("{}{}", tmp_path, suffix));
        }
    }

    let db = Database::new(&tmp_path).expect("Migration 016 on temp copy of live DB");
    let conn = db.open_conn().unwrap();
    let user_ver: i64 = conn
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(user_ver, 17);
    for tbl in ["revoked_tokens", "custom_field_values"] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![tbl],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "Table {} missing on migrated live DB copy", tbl);
    }
    for col in ["start_date", "end_date", "rotation_mode"] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('ad_campaigns') WHERE name = ?1",
                rusqlite::params![col],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "Column ad_campaigns.{} missing on migrated live DB copy", col);
    }
    let ik: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE slug = 'insan-kaynaklari'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ik, 1);
    drop(conn);
    cleanup_test_db(&tmp_path);
}

#[tokio::test]
async fn t14_b14_cart_checkout_dual_email_and_qty_bounds() {
    let (mut state, db_path) = build_test_state("m6_b14_reg");
    let mut cfg = (*state.config).clone();
    cfg.paytr_merchant_id = String::new();
    state.config = std::sync::Arc::new(cfg);
    let app = build_test_router(&state);

    let products = state.db.get_commerce_products();
    let sku = products[0].sku.clone();

    // 1. Both user_email and email sent together (exact msi-ui.js payload) succeeds
    let (st_dual, _, dual_json) = post_json_resp(
        &app,
        "/api/v1/commerce/cart-checkout",
        None,
        json!({
            "user_email": "ui_dual@example.com",
            "email": "UI_DUAL@example.com",
            "full_name": "Dual Key Buyer",
            "organization_name": "SAVTEK",
            "distance_sales_consent": true,
            "items": [{ "item_type": "product", "item_code": sku, "qty": 2 }]
        }),
    )
    .await;
    assert!(st_dual == StatusCode::ACCEPTED || st_dual == StatusCode::OK);
    assert!(dual_json["invoice_id"].as_i64().unwrap_or(0) > 0);

    // 2. Conflicting user_email and email must return 400
    let (st_diff, _, _) = post_json_resp(
        &app,
        "/api/v1/commerce/cart-checkout",
        None,
        json!({
            "user_email": "first@example.com",
            "email": "other@example.com",
            "full_name": "Mismatch Buyer",
            "distance_sales_consent": true,
            "items": [{ "item_type": "product", "item_code": sku, "qty": 1 }]
        }),
    )
    .await;
    assert_eq!(st_diff, StatusCode::BAD_REQUEST);

    // 3. Missing/empty email fields must return 400
    let (st_empty_em, _, _) = post_json_resp(
        &app,
        "/api/v1/commerce/cart-checkout",
        None,
        json!({
            "user_email": "",
            "email": "   ",
            "full_name": "Empty Email",
            "distance_sales_consent": true,
            "items": [{ "item_type": "product", "item_code": sku, "qty": 1 }]
        }),
    )
    .await;
    assert_eq!(st_empty_em, StatusCode::BAD_REQUEST);

    // 4. CheckoutPayload (/api/v1/subscriptions/checkout) also accepts dual keys and rejects mismatch
    let (st_co_dual, _, _) = post_json_resp(
        &app,
        "/api/v1/subscriptions/checkout",
        None,
        json!({
            "user_email": "single_dual@example.com",
            "email": "single_dual@example.com",
            "full_name": "Single Dual",
            "item_type": "product",
            "item_code": sku,
            "distance_sales_consent": true
        }),
    )
    .await;
    assert!(st_co_dual == StatusCode::ACCEPTED || st_co_dual == StatusCode::OK);

    let (st_co_diff, _, _) = post_json_resp(
        &app,
        "/api/v1/subscriptions/checkout",
        None,
        json!({
            "user_email": "a@example.com",
            "email": "b@example.com",
            "full_name": "Single Diff",
            "item_type": "product",
            "item_code": sku,
            "distance_sales_consent": true
        }),
    )
    .await;
    assert_eq!(st_co_diff, StatusCode::BAD_REQUEST);

    // 5. Out-of-bounds quantities (0, -1, 51) must return 400 and create 0 invoices
    let inv_before = state.db.get_order_invoices().len();
    for bad_qty in [0, -1, 51] {
        let (st_qty, _, _) = post_json_resp(
            &app,
            "/api/v1/commerce/cart-checkout",
            None,
            json!({
                "email": "qty_bad@example.com",
                "full_name": "Bad Qty",
                "distance_sales_consent": true,
                "items": [{ "item_type": "product", "item_code": sku, "qty": bad_qty }]
            }),
        )
        .await;
        assert_eq!(st_qty, StatusCode::BAD_REQUEST, "Expected 400 for qty={}", bad_qty);
    }
    assert_eq!(state.db.get_order_invoices().len(), inv_before);

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t15_b13_related_articles_search_and_personalized_feed_gating() {
    let (state, db_path) = build_test_state("m6_b13_leaks");
    let app = build_test_router(&state);

    let (_, free_cookie) = create_user_with_session(
        &state,
        "b13_leak_free@example.com",
        "UserPass123!",
        "Free Reader",
        "registered",
        "Akademi",
        "SAVTEK",
        false,
        "free",
    );
    let (_, digital_cookie) = create_user_with_session(
        &state,
        "b13_leak_dig@example.com",
        "UserPass123!",
        "Digital Reader",
        "subscriber_digital",
        "Akademi",
        "SAVTEK",
        false,
        "Dijital",
    );
    let (_, public_cookie) = create_user_with_session(
        &state,
        "b13_leak_pub@msb.gov.tr",
        "UserPass123!",
        "Public Reader",
        "subscriber_public",
        "Kamu & TSK",
        "MSB",
        true,
        "Kamu E-Dergi Abonesi",
    );

    let mut secret_inst = fixture_content(960);
    secret_inst.id = 0;
    secret_inst.slug = "b13-secret-inst-dossier".into();
    secret_inst.title_tr = "B13GizliBaslik".into();
    secret_inst.summary_tr = "Kuruma ozel ozet.".into();
    secret_inst.body_tr = "<p>İlk cümle. İkinci cümle. SECRET_INSTITUTION_TR_TOKEN üçüncü cümle.</p>".into();
    secret_inst.body_en = "<p>First sentence. Second sentence. SECRET_INSTITUTION_EN_TOKEN third sentence.</p>".into();
    secret_inst.visibility_level = 0;
    secret_inst.ssb_tags = vec!["hava-araclari".into()];
    secret_inst.target_institutions = vec!["Kamu & TSK".into()];
    secret_inst.is_featured = true;
    state.db.upsert_content(&secret_inst).unwrap();

    let mut paywalled = fixture_content(961);
    paywalled.id = 0;
    paywalled.slug = "b13-paywalled-companion".into();
    paywalled.title_tr = "Paywalled Companion".into();
    paywalled.body_tr = "<p>Birinci cümle. İkinci cümle. SECRET_PAYWALL_TOKEN üçüncü cümle.</p>".into();
    paywalled.visibility_level = 2;
    paywalled.ssb_tags = vec!["hava-araclari".into()];
    state.db.upsert_content(&paywalled).unwrap();

    let mut companion = fixture_content(962);
    companion.id = 0;
    companion.slug = "b13-open-companion".into();
    companion.title_tr = "Open Companion".into();
    companion.visibility_level = 0;
    companion.ssb_tags = vec!["hava-araclari".into()];
    state.db.upsert_content(&companion).unwrap();

    // 1. related_articles on open companion must NOT expose restricted item or paywalled body to guest
    let (st_det, _, det_txt) = get_text(&app, "/api/v1/contents/b13-open-companion", None).await;
    assert_eq!(st_det, StatusCode::OK);
    let det_json: Value = serde_json::from_str(&det_txt).unwrap();
    let rel_arr = det_json["related_articles"].as_array().unwrap();
    assert!(!rel_arr.iter().any(|r| r["slug"] == "b13-secret-inst-dossier"));
    assert!(!det_txt.contains("SECRET_INSTITUTION_TR_TOKEN"));
    assert!(!det_txt.contains("SECRET_PAYWALL_TOKEN"));

    // 2. GET /api/v1/search must NOT leak restricted bodies to guest/mismatched org or paywalled bodies to guest/free
    for ck in [None, Some(free_cookie.as_str()), Some(digital_cookie.as_str())] {
        let (st_s, _, s_txt) = get_text(&app, "/api/v1/search?q=B13GizliBaslik", ck).await;
        assert_eq!(st_s, StatusCode::OK);
        assert!(!s_txt.contains("SECRET_INSTITUTION_TR_TOKEN"));
        assert!(!s_txt.contains("SECRET_INSTITUTION_EN_TOKEN"));
    }
    for ck in [None, Some(free_cookie.as_str())] {
        let (st_sp, _, sp_txt) = get_text(&app, "/api/v1/search?q=Paywalled", ck).await;
        assert_eq!(st_sp, StatusCode::OK);
        assert!(!sp_txt.contains("SECRET_PAYWALL_TOKEN"));
    }

    // Matching org & subscriber CAN access via /api/v1/search
    let (st_ok_s, _, ok_s_txt) =
        get_text(&app, "/api/v1/search?q=B13GizliBaslik", Some(&public_cookie)).await;
    assert_eq!(st_ok_s, StatusCode::OK);
    assert!(ok_s_txt.contains("SECRET_INSTITUTION_TR_TOKEN"));

    // 3. SSR render_personalized_feed on / and /hesabim must NOT include restricted article for digital_cookie
    // Update digital user ssb_interests to hava-araclari so feed prioritizes it if unfiltered
    let _ = post_admin_json_resp(
        &app,
        &state,
        "/api/v1/account/profile",
        &digital_cookie,
        json!({ "full_name": "Digital Reader", "ssb_interests": ["hava-araclari"] }),
    )
    .await;
    for route in ["/", "/hesabim"] {
        let (st_p, _, html_p) = get_text(&app, route, Some(&digital_cookie)).await;
        assert_eq!(st_p, StatusCode::OK);
        if let Some(idx) = html_p.find("id=\"personalized-feed-band\"") {
            let tail = &html_p[idx..];
            assert!(!tail.contains("b13-secret-inst-dossier"));
        }
    }

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t16_b3_anonymize_open_data_subscribers_cohorts_and_ltv() {
    let (state, db_path) = build_test_state("m6_b3_all");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "b3_all_admin@msidergisi.com",
        "AdminPass123!",
        "B3 All Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );

    // Create 3 users in same cohort month for cohorts test + 1 paying user (<3) for LTV & Tier (<3)
    let (u1_user, _) = create_user_with_session(
        &state,
        "b3_c1@tiny.example",
        "Pass123!",
        "Tiny Cohort One",
        "subscriber_digital",
        "TinyCat",
        "TinyOrg",
        false,
        "Özel Tekli Paket",
    );
    let u1_id = u1_user.id;
    let _ = create_user_with_session(
        &state,
        "b3_c2@other.example",
        "Pass123!",
        "Cohort Two",
        "registered",
        "Sektör Kuruluşları",
        "OrgTwo",
        false,
        "Standart Kayıtlı Okur",
    );
    let _ = create_user_with_session(
        &state,
        "b3_c3@other.example",
        "Pass123!",
        "Cohort Three",
        "registered",
        "Sektör Kuruluşları",
        "OrgThree",
        false,
        "Standart Kayıtlı Okur",
    );

    // Force all 3 users to an earlier cohort_month (e.g. 2026-09) and 1 active event in 2026-10
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute("UPDATE users SET created_at = '2026-09-05'", []).unwrap();
        conn.execute(
            "INSERT INTO renewal_reminders (user_id, user_email, full_name, organization_name, stage_label, days_remaining, status, sent_at)
             VALUES (?1, 'b3_c1@tiny.example', 'Tiny Cohort One', 'TinyOrg', '1_month', 30, 'sent', '2026-10-01T10:00:00Z')",
            rusqlite::params![u1_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO order_invoices (invoice_no, user_email, full_name, organization_name, item_type, item_code, item_title, amount_try, payment_status, created_at, pdf_watermark_code)
             VALUES ('MSI-B3-1', 'b3_c1@tiny.example', 'Tiny Cohort One', 'TinyOrg', 'subscription', 'dijital', 'Dijital', 450.0, 'paid', '2026-10-12T10:00:00Z', 'WM-B3')",
            [],
        )
        .unwrap();
    }
    // 4 analytics events from the single TinyCat user/session (still k=1 actor < 3!)
    for _ in 0..4 {
        state
            .db
            .insert_analytics_event(&AnalyticsEvent {
                id: 0,
                session_id: "raw-sess-secret-123".into(),
                user_id: Some(u1_id),
                organization_category: "TinyCat".into(),
                organization_name: "TinyOrg".into(),
                event_type: "article_read".into(),
                content_slug: Some("sample".into()),
                ad_id: None,
                duration_sec: 30,
                scroll_depth_pct: 80,
                video_completed: false,
                is_true_read: true,
                ssb_tag: None,
                created_at: "2026-10-15T10:00:00Z".into(),
                video_watch_pct: 0,
            })
            .unwrap();
    }

    // 1. open-data?anonymize=1
    let (st_od, _, od_txt) =
        get_text(&app, "/api/v1/admin/export/open-data?anonymize=1", Some(&admin_cookie)).await;
    assert_eq!(st_od, StatusCode::OK);
    assert!(!od_txt.contains("b3_c1@tiny.example"));
    assert!(!od_txt.contains("Tiny Cohort One"));
    assert!(!od_txt.contains("raw-sess-secret-123"));
    assert!(!od_txt.contains("TinyCat"));
    let od_json: Value = serde_json::from_str(&od_txt).unwrap();
    let rem_arr = od_json["renewal_reminders_5_stage"].as_array().unwrap();
    assert!(!rem_arr.is_empty());
    assert!(rem_arr[0].get("user_email").is_none());
    assert!(rem_arr[0].get("full_name").is_none());
    assert!(rem_arr[0]["anon_id"].as_str().unwrap().starts_with("anon_"));

    // 2. reports/subscribers?anonymize=1
    let (st_sub, _, sub_txt) =
        get_text(&app, "/api/v1/admin/reports/subscribers?anonymize=1", Some(&admin_cookie)).await;
    assert_eq!(st_sub, StatusCode::OK);
    let sub_json: Value = serde_json::from_str(&sub_txt).unwrap();
    let by_tier = unwrap_data(&sub_json)["by_tier"].as_array().unwrap();
    assert!(by_tier.iter().all(|t| t["count"].as_i64().unwrap() >= 3));

    // 3. user-cohorts?anonymize=1 suppresses 1/2 active counts in m*_users & active_m*
    let (st_coh, _, coh_txt) =
        get_text(&app, "/api/v1/admin/analytics/user-cohorts?anonymize=1", Some(&admin_cookie)).await;
    assert_eq!(st_coh, StatusCode::OK);
    let coh_json: Value = serde_json::from_str(&coh_txt).unwrap();
    let coh_arr = unwrap_data(&coh_json).as_array().unwrap();
    for row in coh_arr {
        for k in 1..=6 {
            if let Some(v) = row.get(&format!("active_m{}", k)).and_then(|x| x.as_i64()) {
                assert!(v == 0 || v >= 3, "Leaked <3 active_m{}={}", k, v);
            }
            if let Some(v) = row.get(&format!("m{}_users", k)).and_then(|x| x.as_i64()) {
                assert!(v == 0 || v >= 3, "Leaked <3 m{}_users={}", k, v);
            }
        }
    }

    // 4. user-ltv?anonymize=1 suppresses paying_users when 0 < paying_users < 3
    let (st_ltv, _, ltv_txt) =
        get_text(&app, "/api/v1/admin/analytics/user-ltv?anonymize=1", Some(&admin_cookie)).await;
    assert_eq!(st_ltv, StatusCode::OK);
    let ltv_json: Value = serde_json::from_str(&ltv_txt).unwrap();
    let ltv_data = unwrap_data(&ltv_json);
    assert!(ltv_data["paying_users"].is_null());
    assert!(ltv_data["total_revenue"].is_null());

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t17_b7_newsletter_ui_groups_email_consent_guest_dispatch_and_tz_offset() {
    let (mut state, db_path) = build_test_state("m6_b7_reg");
    let mut cfg = (*state.config).clone();
    cfg.mail_delivery_enabled = false;
    state.config = std::sync::Arc::new(cfg);
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "b7_reg_admin@msidergisi.com",
        "AdminPass123!",
        "B7 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (_, existing_user_cookie) = create_user_with_session(
        &state,
        "b7_existing_opted@example.com",
        "UserPass123!",
        "Existing Opted",
        "registered",
        "Akademi",
        "ODTÜ",
        false,
        "free",
    );
    let _ = existing_user_cookie;

    let past_ts = chrono::Utc::now().timestamp() - 5;

    // 1. Public subscribe to edergi-releases with email_consent: true persists in email_groups
    let (st1, _, _) = post_json_resp(
        &app,
        "/api/v1/public/newsletter/subscribe",
        None,
        json!({
            "email": "guest_edergi@example.com",
            "full_name": "Guest Edergi",
            "email_group_code": "edergi-releases",
            "kvkk_consent": true,
            "email_consent": true,
            "website_url": "",
            "_form_ts": past_ts
        }),
    )
    .await;
    assert_eq!(st1, StatusCode::OK);

    // 2. Public subscribe with email_consent: false must NOT add to email_groups and must log etk accepted=false
    let (st2, _, res2) = post_json_resp(
        &app,
        "/api/v1/public/newsletter/subscribe",
        None,
        json!({
            "email": "guest_no_etk@example.com",
            "full_name": "Guest No ETK",
            "email_group_code": "edergi-releases",
            "kvkk_consent": true,
            "email_consent": false,
            "website_url": "",
            "_form_ts": past_ts
        }),
    )
    .await;
    assert_eq!(st2, StatusCode::OK);
    assert_eq!(unwrap_data(&res2)["etk_accepted"], false);
    let no_etk_logs = state.db.list_kvkk_consent_logs_for_user(0, "guest_no_etk@example.com");
    assert!(no_etk_logs.iter().any(|l| l.consent_type == "etk" && !l.accepted));

    // 3. resolve_email_group_recipients("edergi-releases") includes guest_edergi (id=0), excludes guest_no_etk & non-group users
    let recips = state.db.resolve_email_group_recipients("edergi-releases", "", "");
    assert!(recips.iter().any(|u| u.email == "guest_edergi@example.com" && u.id == 0));
    assert!(!recips.iter().any(|u| u.email == "guest_no_etk@example.com"));
    assert!(!recips.iter().any(|u| u.email == "b7_existing_opted@example.com"));

    // 4. Unknown explicit email_group_code returns empty (never falls back to all users)
    assert!(state
        .db
        .resolve_email_group_recipients("nonexistent-group-xyz", "", "")
        .is_empty());

    // 5. Scheduler tick dispatches to synthetic guest recipient (id == 0) cleanly
    let due_id = state
        .db
        .insert_newsletter_campaign(&msi_dergisi_platform::models::NewsletterCampaign {
            id: 0,
            subject_tr: "E-Dergi Guest Dispatch".into(),
            subject_en: "E-Dergi Guest Dispatch".into(),
            target_group_filter: "all".into(),
            target_institution: String::new(),
            target_ssb_tag: String::new(),
            sponsor_ad_id: None,
            sponsor_name: None,
            recipient_count: 0,
            status: "scheduled".into(),
            sent_at: String::new(),
            scheduled_at: Some((chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339()),
            email_group_code: Some("edergi-releases".into()),
            body_html: "<p>Guest body</p>".into(),
        })
        .unwrap();
    let processed = newsletter_scheduler::run_newsletter_scheduler_tick(&state).await;
    assert_eq!(processed, 1);
    let camp = state
        .db
        .get_newsletter_campaigns()
        .into_iter()
        .find(|c| c.id == due_id)
        .unwrap();
    assert_eq!(camp.status, "queued");
    assert_eq!(camp.recipient_count, 1);

    // 6. Offset-aware scheduled_at (e.g. +03:00) is normalized to UTC 'Z'
    let future_plus3 = (chrono::Utc::now() + chrono::Duration::hours(4))
        .with_timezone(&chrono::FixedOffset::east_opt(3 * 3600).unwrap())
        .to_rfc3339();
    let (st_sched, _, sched_json) = post_admin_json_resp(
        &app,
        &state,
        "/api/v1/admin/newsletters",
        &admin_cookie,
        json!({
            "id": 0,
            "subject_tr": "TZ Offset Campaign",
            "subject_en": "TZ Offset Campaign",
            "target_group_filter": "all",
            "target_institution": "",
            "target_ssb_tag": "",
            "sponsor_ad_id": null,
            "sponsor_name": null,
            "recipient_count": 0,
            "status": "scheduled",
            "sent_at": "",
            "scheduled_at": future_plus3,
            "email_group_code": "edergi-releases",
            "body_html": "<p>TZ</p>"
        }),
    )
    .await;
    assert_eq!(st_sched, StatusCode::OK);
    let norm_sched = unwrap_data(&sched_json)["scheduled_at"].as_str().unwrap_or("");
    assert!(norm_sched.ends_with('Z'), "Expected UTC Z normalization, got {}", norm_sched);

    cleanup_test_db(&db_path);
}


#[tokio::test]
async fn t18_b6_biz_kimiz_backfilled_on_legacy_design_config() {
    let (state, db_path) = build_test_state("m6_b6_legacy");
    let app = build_test_router(&state);

    // Persist a legacy design_config JSON in site_settings that omits `kurumsal-biz-kimiz`
    let mut legacy_cfg = msi_dergisi_platform::engine::design::config::defaults();
    legacy_cfg.pages.retain(|p| p.slug != "kurumsal-biz-kimiz");
    let legacy_json = serde_json::to_string(&legacy_cfg).unwrap();
    state
        .db
        .upsert_site_setting(&msi_dergisi_platform::models::SiteSetting {
            key: msi_dergisi_platform::engine::design::config::SETTING_KEY.to_string(),
            value_tr: legacy_json.clone(),
            value_en: legacy_json,
            setting_group: "design".to_string(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
        .unwrap();

    let (st_tr, _, html_tr) = get_text(&app, "/kurumsal/biz-kimiz", None).await;
    assert_eq!(st_tr, StatusCode::OK);
    assert!(html_tr.contains("Bağımsız Savunma Gazeteciliği İlkelerimiz"));

    let (st_en, _, html_en) = get_text(&app, "/en/kurumsal/who-we-are", None).await;
    assert_eq!(st_en, StatusCode::OK);
    assert!(html_en.contains("Principles of Independent Defence Journalism"));

    cleanup_test_db(&db_path);
}
#[tokio::test]
async fn t19_t4_01_editorial_workflow_approval_serves_200() {
    let (state, db_path) = build_test_state("m6_t401");
    let app = build_test_router(&state);

    let (_, ce_cookie) = create_user_with_session(
        &state,
        "t401_ce@msidergisi.com",
        "StaffPass123!",
        "T401 Editor",
        "content_editor",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (_, cp_cookie) = create_user_with_session(
        &state,
        "t401_cp@msidergisi.com",
        "StaffPass123!",
        "T401 Publisher",
        "content_publisher",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );

    // Reproduce exact T4-01 slug pattern (`t4-editorial-t4_<digits>`) AND check returned slug
    let slug = "t4-editorial-t4_1763621799000";
    let (st_c, _, c_json) = post_admin_json_resp(
        &app,
        &state,
        "/api/v1/admin/contents",
        &ce_cookie,
        json!({
            "id": 0,
            "slug": slug,
            "title_tr": "T4 Çoklu Rol Makalesi",
            "title_en": "T4 Multi-Role Article",
            "summary_tr": "Özet TR",
            "summary_en": "Summary EN",
            "body_tr": "<p>Türkçe tam gövde içeriği.</p>",
            "body_en": "<p>English full body content.</p>",
            "content_type": "haberler",
            "source_group": "gunluk-haber",
            "cover_image": "/images/logo.svg",
            "author_name": "E2E Editör",
            "author_title": "Editör",
            "published_at": "2026-11-19 12:00",
            "read_time_min": 4,
            "visibility_level": 0,
            "single_price_try": 0,
            "is_sponsored": false,
            "is_featured": false,
            "view_count": 0,
            "true_read_count": 0,
            "placements": ["haberler", "ozel-dosyalar"],
            "ssb_tags": ["hava-araclari", "radarlar-ve-sensorler"],
            "status": "draft"
        }),
    )
    .await;
    assert_eq!(st_c, StatusCode::OK);
    let content_id = unwrap_data(&c_json)["id"].as_i64().unwrap();

    let (st_sub, _, sub_json) = post_admin_json_resp(
        &app,
        &state,
        &format!("/api/v1/admin/contents/{}/submit-review", content_id),
        &ce_cookie,
        json!({}),
    )
    .await;
    assert_eq!(st_sub, StatusCode::OK);
    let approval_id = unwrap_data(&sub_json)["approval_id"].as_i64().unwrap();

    let (st_pend, _, _) = get_text(&app, &format!("/icerik/{}", slug), None).await;
    assert_eq!(st_pend, StatusCode::NOT_FOUND);

    let (st_app, _, _) = post_admin_json_resp(
        &app,
        &state,
        &format!("/api/v1/admin/approvals/{}/approve", approval_id),
        &cp_cookie,
        json!({ "note": "T4 E2E Onaylandı" }),
    )
    .await;
    assert_eq!(st_app, StatusCode::OK);

    let (st_tr, _, html_tr) = get_text(&app, &format!("/icerik/{}", slug), None).await;
    assert_eq!(st_tr, StatusCode::OK, "Approved TR article should return 200");
    assert!(html_tr.contains("T4 Çoklu Rol Makalesi"));

    let (st_en, _, html_en) = get_text(&app, &format!("/icerik/{}?lang=en", slug), None).await;
    assert_eq!(st_en, StatusCode::OK, "Approved EN article should return 200");
    assert!(html_en.contains("T4 Multi-Role Article"));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn t20_personalized_feed_scopes_recommendation_rules_and_ranks_ssb_recency_above_featured_backlog() {
    let (state, db_path) = build_test_state("m6_t20_pers_rank");
    let app = build_test_router(&state);

    // Create 15 older featured deniz-araclari articles to simulate featured backlog > limit(12)
    for i in 1..=15 {
        let mut old_feat = fixture_content(1200 + i);
        old_feat.id = 0;
        old_feat.slug = format!("t20-old-featured-deniz-{}", i);
        old_feat.title_tr = format!("Eski Öne Çıkan Deniz Haberi {}", i);
        old_feat.published_at = format!("2025-05-{:02} 10:00:00", i);
        old_feat.is_featured = true;
        old_feat.is_sponsored = false;
        old_feat.ssb_tags = vec!["deniz-araclari".into()];
        state.db.upsert_content(&old_feat).unwrap();
    }

    let now = chrono::Utc::now();
    let ts_air = (now - chrono::Duration::minutes(5)).format("%Y-%m-%d %H:%M:%S").to_string();
    let ts_fresh = (now - chrono::Duration::minutes(10)).format("%Y-%m-%d %H:%M:%S").to_string();
    let ts_multi = (now - chrono::Duration::minutes(20)).format("%Y-%m-%d %H:%M:%S").to_string();
    let ts_navy = (now - chrono::Duration::days(10)).format("%Y-%m-%d %H:%M:%S").to_string();

    // Create an unrelated hava-araclari article promoted by an active wildcard recommendation_rule
    let mut air_rule_target = fixture_content(1250);
    air_rule_target.id = 0;
    air_rule_target.slug = "t20-air-rule-target".into();
    air_rule_target.title_tr = "Hava Araçları Kural Hedefi".into();
    air_rule_target.content_type = "haberler".into();
    air_rule_target.source_group = "gunluk-haber".into();
    air_rule_target.published_at = ts_air;
    air_rule_target.is_featured = true;
    air_rule_target.is_sponsored = false;
    air_rule_target.ssb_tags = vec!["hava-araclari".into()];
    state.db.upsert_content(&air_rule_target).unwrap();

    // Create a newer non-featured deniz-araclari article and a multi-tag match article
    let mut fresh_naval = fixture_content(1251);
    fresh_naval.id = 0;
    fresh_naval.slug = "t20-fresh-naval".into();
    fresh_naval.title_tr = "Yeni Deniz Platformu Haberi".into();
    fresh_naval.content_type = "haberler".into();
    fresh_naval.source_group = "gunluk-haber".into();
    fresh_naval.published_at = ts_fresh;
    fresh_naval.is_featured = false;
    fresh_naval.is_sponsored = false;
    fresh_naval.ssb_tags = vec!["deniz-araclari".into()];
    state.db.upsert_content(&fresh_naval).unwrap();

    let mut multi_match = fixture_content(1252);
    multi_match.id = 0;
    multi_match.slug = "t20-multi-tag-match".into();
    multi_match.title_tr = "Deniz ve Radar Ortak Haberi".into();
    multi_match.content_type = "haberler".into();
    multi_match.source_group = "gunluk-haber".into();
    multi_match.published_at = ts_multi;
    multi_match.is_featured = false;
    multi_match.is_sponsored = false;
    multi_match.ssb_tags = vec!["deniz-araclari".into(), "radarlar-ve-sensorler".into()];
    state.db.upsert_content(&multi_match).unwrap();

    // Create an older Dz.K.K.lığı-targeted deniz-araclari article for REQ-012 / REQ-039 check
    let mut navy_inst_target = fixture_content(1253);
    navy_inst_target.id = 0;
    navy_inst_target.slug = "t20-navy-inst-target".into();
    navy_inst_target.title_tr = "Dz.K.K.lığı Özel Deniz Analizi".into();
    navy_inst_target.content_type = "haberler".into();
    navy_inst_target.source_group = "gunluk-haber".into();
    navy_inst_target.published_at = ts_navy;
    navy_inst_target.is_featured = false;
    navy_inst_target.is_sponsored = false;
    navy_inst_target.ssb_tags = vec!["deniz-araclari".into()];
    state.db.upsert_content(&navy_inst_target).unwrap();

    {
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "INSERT INTO recommendation_rules (rule_name, source_tag_or_cat, target_institution, recommended_content_slug, priority, is_active)
             VALUES ('T20 Air Wildcard', 'hava-araclari', '*', 't20-air-rule-target', 100, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO recommendation_rules (rule_name, source_tag_or_cat, target_institution, recommended_content_slug, priority, is_active)
             VALUES ('T20 Navy Scoped', 'deniz-araclari', 'Dz.K.K.lığı', 't20-navy-inst-target', 80, 1)",
            [],
        )
        .unwrap();
    }
    state.db.refresh_l1_sections(&["recommendation_rules"]);

    // 1. Anonymous ?interests=deniz-araclari&limit=12 must place fresh_naval above older featured backlog & unrelated air rule
    let (st_anon, _, anon_txt) = get_text(
        &app,
        "/api/v1/public/personalized?interests=deniz-araclari&limit=12",
        None,
    )
    .await;
    assert_eq!(st_anon, StatusCode::OK);
    let anon_json: Value = serde_json::from_str(&anon_txt).unwrap();
    let anon_items = unwrap_data(&anon_json)["items"].as_array().unwrap();
    assert_eq!(anon_items[0]["slug"], "t20-fresh-naval");
    assert!(
        !anon_items.iter().any(|i| i["slug"] == "t20-air-rule-target"),
        "Unrelated hava-araclari rule target must not displace matching deniz-araclari items"
    );

    // 2. Multi-interest reader: 2 matching SSB tags beats 1 matching SSB tag even if slightly older
    let (st_multi, _, multi_txt) = get_text(
        &app,
        "/api/v1/public/personalized?interests=deniz-araclari,radarlar-ve-sensorler&limit=6",
        None,
    )
    .await;
    assert_eq!(st_multi, StatusCode::OK);
    let multi_json: Value = serde_json::from_str(&multi_txt).unwrap();
    let multi_items = unwrap_data(&multi_json)["items"].as_array().unwrap();
    assert_eq!(multi_items[0]["slug"], "t20-multi-tag-match");

    // 3. Institutional recommendation rule (REQ-012 / REQ-039) boosts for matching org, not mismatched org
    let (u_navy, navy_cookie) = create_user_with_session(
        &state,
        "t20_navy@dzkk.tsk.tr",
        "UserPass123!",
        "DzKK Okur",
        "subscriber_public",
        "Dz.K.K.lığı",
        "Deniz Kuvvetleri",
        true,
        "Kamu",
    );
    let (u_sec, sector_cookie) = create_user_with_session(
        &state,
        "t20_sector@savunma.example",
        "UserPass123!",
        "Sektor Okur",
        "subscriber_digital",
        "Sektör Kuruluşları",
        "SAVTEK",
        false,
        "Dijital",
    );
    for uid in [u_navy.id, u_sec.id] {
        let u = state.db.get_user_by_id(uid).unwrap();
        state
            .db
            .update_user_profile(
                u.id,
                &u.full_name,
                &u.organization_category,
                &u.organization_name,
                &u.job_title,
                &["deniz-araclari".to_string()],
                u.email_consent,
            )
            .unwrap();
    }

    let (st_navy, _, navy_txt) = get_text(
        &app,
        "/api/v1/public/personalized?limit=6",
        Some(&navy_cookie),
    )
    .await;
    assert_eq!(st_navy, StatusCode::OK);
    let navy_json: Value = serde_json::from_str(&navy_txt).unwrap();
    let navy_items = unwrap_data(&navy_json)["items"].as_array().unwrap();
    assert_eq!(
        navy_items[0]["slug"], "t20-navy-inst-target",
        "Matching target_institution + source_tag_or_cat rule should boost for Dz.K.K.lığı reader"
    );

    let (st_sec, _, sec_txt) = get_text(
        &app,
        "/api/v1/public/personalized?limit=6",
        Some(&sector_cookie),
    )
    .await;
    assert_eq!(st_sec, StatusCode::OK);
    let sec_json: Value = serde_json::from_str(&sec_txt).unwrap();
    let sec_items = unwrap_data(&sec_json)["items"].as_array().unwrap();
    assert_eq!(
        sec_items[0]["slug"], "t20-fresh-naval",
        "Mismatched organization must not receive Dz.K.K.lığı rule boost"
    );

    cleanup_test_db(&db_path);
}
