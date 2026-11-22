//! Milestone M1 Spec Parity, Security & DRP Integration Tests
//!
//! Covers:
//! - REQ-002: Browser Accept-Language auto-detection
//! - REQ-033: Persistent token revocation & hardened /api/v1/admin/* routes
//! - REQ-034: AES-256-GCM + gzip + 4KB page-level incremental snapshots, restore verify & S3 SigV4
//! - REQ-026: B2B consolidated monthly report per-content & per-ad institutional breakdowns
//! - REQ-022: Ad campaign schedule persistence, 409 Conflict enforcement & force_override
//! - REQ-012: recommendation_rules CRUD API + L1 cache + content_service integration
//! - REQ-008: custom_field_values persistence, validation & entity-scoped RBAC

mod common;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tower::ServiceExt;

use common::*;
use msi_dergisi_platform::{
    controllers::AppState,
    engine::security_engine,
    models::AnalyticsEvent,
    services::{ad_service, auth_service, content_service},
    workers::{b2b_report_worker, backup_worker},
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

async fn admin_req_json(
    app: &axum::Router,
    state: &AppState,
    method: &str,
    uri: &str,
    session_cookie: &str,
    payload: Option<Value>,
) -> (StatusCode, Value) {
    let csrf = security_engine::generate_csrf_token(&state.config.session_secret);
    let combined_cookie = format!("{}; csrf_token={}", session_cookie, csrf);
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, combined_cookie)
        .header("x-csrf-token", &csrf);
    let body = if let Some(p) = payload {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(p.to_string())
    } else {
        Body::empty()
    };
    let resp = app.clone().oneshot(builder.body(body).unwrap()).await.unwrap();
    let st = resp.status();
    let bytes = to_bytes(resp.into_body(), 8 * 1024 * 1024).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (st, val)
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
async fn m1_req002_accept_language_auto_detection_on_http_and_api() {
    let (state, db_path) = build_test_state("m1_req002");
    let app = build_test_router(&state);

    // 1. Accept-Language: en-US -> HTML renders lang="en"
    let resp_en = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9,tr;q=0.8")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_en.status(), StatusCode::OK);
    let html_en = String::from_utf8_lossy(
        &to_bytes(resp_en.into_body(), 4 * 1024 * 1024).await.unwrap(),
    )
    .to_string();
    assert!(
        html_en.contains("<html lang=\"en\""),
        "Expected <html lang=\"en\"> when Accept-Language prefers en"
    );

    // 2. Cookie msi_lang=tr overrides Accept-Language: en-US
    let resp_cookie_tr = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
                .header(header::COOKIE, "msi_lang=tr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_cookie_tr.status(), StatusCode::OK);
    let html_tr = String::from_utf8_lossy(
        &to_bytes(resp_cookie_tr.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap(),
    )
    .to_string();
    assert!(
        html_tr.contains("<html lang=\"tr\""),
        "Expected cookie msi_lang=tr to override Accept-Language"
    );

    // 3. Public API endpoint uses Accept-Language when ?lang= absent
    let resp_api = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/public/personalized?limit=3")
                .header(header::ACCEPT_LANGUAGE, "en-GB,en;q=0.8")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_api.status(), StatusCode::OK);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_req033_hardened_admin_routes_and_persistent_logout_revocation() {
    let (state, db_path) = build_test_state("m1_req033");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "req033_admin@msidergisi.com",
        "AdminPass123!",
        "Req033 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (_, reader_cookie) = create_user_with_session(
        &state,
        "req033_reader@example.com",
        "UserPass123!",
        "Req033 Reader",
        "registered",
        "Akademi",
        "ODTÜ",
        false,
        "free",
    );

    // Legacy non-admin routes must return 404
    for (method, uri, body) in [
        ("POST", "/api/v1/ads/check-conflict", Some(json!({"slot_code": "ad-1", "sponsor_name": "X"}))),
        ("GET", "/api/v1/analytics/sponsor-report", None),
        ("POST", "/api/v1/subscriptions/trigger-reminders", Some(json!({}))),
    ] {
        let (st, _, _) = send_req(&app, method, uri, Some(&admin_cookie), body).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "Expected 404 on removed alias {}", uri);
    }

    // Canonical /api/v1/admin/* endpoints enforce auth, role, and CSRF
    for (method, uri, payload) in [
        (
            "POST",
            "/api/v1/admin/ads/check-conflict",
            Some(json!({"slot_code": "ad-header-banner", "sponsor_name": "Roketsan"})),
        ),
        ("GET", "/api/v1/admin/analytics/sponsor-report", None),
        ("POST", "/api/v1/admin/subscriptions/trigger-reminders", Some(json!({}))),
    ] {
        // Anonymous -> 401
        let (st_anon, _, _) = send_req(&app, method, uri, None, payload.clone()).await;
        assert_eq!(st_anon, StatusCode::UNAUTHORIZED, "Anon should get 401 on {}", uri);

        // Non-staff reader -> 403
        let (st_reader, _, _) =
            send_req(&app, method, uri, Some(&reader_cookie), payload.clone()).await;
        assert_eq!(st_reader, StatusCode::FORBIDDEN, "Reader should get 403 on {}", uri);

        // Admin without CSRF on POST -> 403
        if method == "POST" {
            let (st_nocsrf, _, _) =
                send_req(&app, method, uri, Some(&admin_cookie), payload.clone()).await;
            assert_eq!(st_nocsrf, StatusCode::FORBIDDEN, "POST without CSRF should get 403 on {}", uri);
        }

        // Admin with CSRF -> 200
        let (st_ok, _) = admin_req_json(&app, &state, method, uri, &admin_cookie, payload).await;
        assert_eq!(st_ok, StatusCode::OK, "Authorized admin failed on {}", uri);
    }

    // Persistent logout revocation across in-memory cache wipe
    let (st_before, _, _) = send_req(&app, "GET", "/api/v1/admin/me", Some(&admin_cookie), None).await;
    assert_eq!(st_before, StatusCode::OK);

    let (st_logout, _) = admin_req_json(&app, &state, "POST", "/api/v1/auth/logout", &admin_cookie, None).await;
    assert_eq!(st_logout, StatusCode::OK);

    // Clear in-memory cache to simulate process restart
    auth_service::clear_in_memory_revocation_cache();

    let (st_after, _, _) = send_req(&app, "GET", "/api/v1/admin/me", Some(&admin_cookie), None).await;
    assert_eq!(
        st_after,
        StatusCode::UNAUTHORIZED,
        "Revoked session token must remain rejected after in-memory cache wipe"
    );

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_req034_encrypted_full_and_incremental_backup_restore_and_s3_sigv4() {
    let (mut state, db_path) = build_test_state("m1_req034");

    // Start local loopback HTTP listener to verify real AWS SigV4 PUT upload
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_task = tokio::spawn(async move {
        let mut received: Vec<(String, Vec<u8>)> = Vec::new();
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = Vec::new();
            let mut tmp = [0u8; 4096];
            let mut header_end = None;
            let mut content_len = 0usize;
            loop {
                let n = socket.read(&mut tmp).await.unwrap();
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                if header_end.is_none() {
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        header_end = Some(pos + 4);
                        let hdr_str = String::from_utf8_lossy(&buf[..pos + 4]);
                        for line in hdr_str.lines() {
                            if let Some((k, v)) = line.split_once(':') {
                                if k.trim().eq_ignore_ascii_case("content-length") {
                                    content_len = v.trim().parse().unwrap_or(0);
                                }
                            }
                        }
                    }
                }
                if let Some(he) = header_end {
                    if buf.len() >= he + content_len {
                        break;
                    }
                }
            }
            let he = header_end.unwrap();
            let hdr_str = String::from_utf8_lossy(&buf[..he]).to_string();
            let body = buf[he..he + content_len].to_vec();
            received.push((hdr_str, body));
            let _ = socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await;
        }
        received
    });

    let mut cfg = (*state.config).clone();
    cfg.s3_backup_endpoint = format!("http://{}", addr);
    cfg.s3_backup_bucket = "msi-drp-bucket".to_string();
    cfg.s3_backup_region = "eu-central-1".to_string();
    cfg.s3_backup_access_key = "AKIA_TEST_ACCESS".to_string();
    cfg.s3_backup_secret_key = "test_s3_secret_key_for_sigv4".to_string();
    cfg.s3_backup_prefix = "prod-db".to_string();
    cfg.backup_encryption_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();
    state.config = std::sync::Arc::new(cfg);

    // 1. Full encrypted backup + S3 upload #1
    let full_res = backup_worker::perform_full_backup_with_offsite(
        &state.db,
        &state.config,
        "msi_backup",
        "req034_test",
    )
    .await
    .expect("Full backup with offsite failed");
    assert_eq!(full_res.integrity_check, "ok");
    assert!(full_res.s3_uploaded);
    let base_enc_path = full_res.encrypted_archive_path.clone().unwrap();
    assert!(Path::new(&base_enc_path).exists());

    // 2. Mutate database after full backup
    let marker_slug = "drp-incremental-marker-2026";
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "INSERT INTO site_settings (key, value_tr, value_en, setting_group, updated_at)
             VALUES (?1, 'DRP_CANARY_VALUE', 'DRP_CANARY_VALUE', 'system', '2026-11-21T00:00:00Z')",
            rusqlite::params![marker_slug],
        )
        .unwrap();
    }

    // 3. Incremental snapshot + S3 upload #2
    let incr_res = backup_worker::perform_incremental_snapshot(&state.db, &state.config)
        .await
        .expect("Incremental snapshot failed")
        .expect("Expected non-None incremental snapshot after DB mutation");
    assert_eq!(incr_res.kind, "incremental_delta");
    assert!(incr_res.changed_pages > 0);
    assert!(incr_res.changed_pages < incr_res.total_pages);
    assert!(incr_res.s3_uploaded);

    // Idempotency check: second incremental without DB changes returns None
    let noop = backup_worker::perform_incremental_snapshot(&state.db, &state.config)
        .await
        .expect("Noop incremental check failed");
    assert!(noop.is_none());

    // 4. Restore from base + incremental delta and verify integrity + mutated row!
    let key = backup_worker::derive_backup_key(
        &state.config.backup_encryption_key,
        &state.config.session_secret,
    );
    let restored_path = format!("{}_restored.db", db_path);
    let restored_sha = backup_worker::restore_encrypted_backup(
        Some(Path::new(&base_enc_path)),
        Path::new(&incr_res.archive_path),
        &key,
        Path::new(&restored_path),
    )
    .expect("Restore from full + delta failed");
    assert_eq!(restored_sha, incr_res.target_db_sha256);

    let r_conn = rusqlite::Connection::open(&restored_path).unwrap();
    let integrity: String = r_conn
        .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    let canary: String = r_conn
        .query_row(
            "SELECT value_tr FROM site_settings WHERE key = ?1",
            rusqlite::params![marker_slug],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(canary, "DRP_CANARY_VALUE");
    drop(r_conn);

    // 5. Wrong key and tampered ciphertext must fail authentication
    let wrong_key = [0xAAu8; 32];
    let bad_restore = format!("{}_bad.db", db_path);
    assert!(backup_worker::restore_encrypted_backup(
        Some(Path::new(&base_enc_path)),
        Path::new(&incr_res.archive_path),
        &wrong_key,
        Path::new(&bad_restore),
    )
    .is_err());

    let mut tampered = std::fs::read(&incr_res.archive_path).unwrap();
    let last_idx = tampered.len() - 1;
    tampered[last_idx] ^= 0xFF;
    let tampered_path = format!("{}.tampered", incr_res.archive_path);
    std::fs::write(&tampered_path, &tampered).unwrap();
    assert!(backup_worker::restore_encrypted_backup(
        Some(Path::new(&base_enc_path)),
        Path::new(&tampered_path),
        &key,
        Path::new(&bad_restore),
    )
    .is_err());

    // 6. Verify S3 mock server received valid SigV4 PUT requests and encrypted bytes decrypt cleanly
    let s3_requests = server_task.await.unwrap();
    assert_eq!(s3_requests.len(), 2);
    for (idx, (hdrs, body)) in s3_requests.iter().enumerate() {
        assert!(hdrs.starts_with("PUT /msi-drp-bucket/prod-db/"), "Unexpected request line: {}", hdrs);
        let lower = hdrs.to_ascii_lowercase();
        assert!(lower.contains("authorization: aws4-hmac-sha256 credential=akia_test_access/"));
        assert!(lower.contains("signedheaders=host;x-amz-content-sha256;x-amz-date"));
        assert!(lower.contains("x-amz-date:"));
        assert!(lower.contains("x-amz-content-sha256:"));
        assert!(body.starts_with(b"MSIBKP01"), "Uploaded S3 body #{} must be encrypted MSIBKP01", idx);
    }

    let _ = std::fs::remove_file(&restored_path);
    let _ = std::fs::remove_file(&bad_restore);
    let _ = std::fs::remove_dir_all(&state.config.backup_dir);
    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_req026_b2b_consolidated_monthly_report_per_content_and_ad_breakdown() {
    let (state, db_path) = build_test_state("m1_req026");

    let contents = state.db.get_all_contents();
    assert!(!contents.is_empty());
    let target_content = &contents[0];

    let ads = state.db.get_all_ads();
    assert!(!ads.is_empty());
    let target_ad = &ads[0];

    // Seed 3 MSB + 1 SSB content views (75.0% MSB, 25.0% SSB) and 2 MSB + 2 SSB ad events in 2026-11
    for (idx, org) in ["MSB", "MSB", "MSB", "SSB"].iter().enumerate() {
        state
            .db
            .insert_analytics_event(&AnalyticsEvent {
                id: 0,
                session_id: format!("b2b-c-{}", idx),
                user_id: None,
                organization_category: org.to_string(),
                organization_name: org.to_string(),
                event_type: "article_read".to_string(),
                content_slug: Some(target_content.slug.clone()),
                ad_id: None,
                duration_sec: 45,
                scroll_depth_pct: 85,
                video_completed: false,
                is_true_read: true,
                ssb_tag: Some("hava-araclari".to_string()),
                created_at: "2026-11-18T12:00:00Z".to_string(),
                video_watch_pct: 0,
            })
            .unwrap();
    }

    for (idx, (org, ev_type)) in [
        ("MSB", "ad_impression"),
        ("SSB", "ad_impression"),
        ("MSB", "ad_click"),
        ("SSB", "ad_click"),
    ]
    .iter()
    .enumerate()
    {
        state
            .db
            .insert_analytics_event(&AnalyticsEvent {
                id: 0,
                session_id: format!("b2b-a-{}", idx),
                user_id: None,
                organization_category: org.to_string(),
                organization_name: org.to_string(),
                event_type: ev_type.to_string(),
                content_slug: None,
                ad_id: Some(target_ad.id),
                duration_sec: 0,
                scroll_depth_pct: 0,
                video_completed: false,
                is_true_read: false,
                ssb_tag: None,
                created_at: "2026-11-19T10:00:00Z".to_string(),
                video_watch_pct: 0,
            })
            .unwrap();
    }

    let built = b2b_report_worker::build_consolidated_b2b_report(&state, "2026-11");
    let content_reports = built.summary_json["content_reports"].as_array().unwrap();
    let cr = content_reports
        .iter()
        .find(|r| r["content_slug"] == target_content.slug)
        .expect("Seeded content missing from consolidated B2B report");
    assert_eq!(cr["month_views"].as_i64(), Some(4));
    assert!(cr["institution_summary"].as_str().unwrap().contains("%75.0 MSB"));
    assert!(cr["institution_summary"].as_str().unwrap().contains("%25.0 SSB"));

    let ad_reports = built.summary_json["ad_reports"].as_array().unwrap();
    let ar = ad_reports
        .iter()
        .find(|r| r["ad_id"] == target_ad.id)
        .expect("Seeded ad missing from consolidated B2B report");
    assert_eq!(ar["month_impressions"].as_i64(), Some(2));
    assert_eq!(ar["month_clicks"].as_i64(), Some(2));
    assert!(ar["impression_summary"].as_str().unwrap().contains("%50.0 MSB"));
    assert!(ar["click_summary"].as_str().unwrap().contains("%50.0 SSB"));

    assert!(
        built.html_body.contains(&security_engine::escape_html(&target_content.title_tr))
            || built.html_body.contains(&target_content.slug)
    );
    assert!(built
        .html_body
        .contains(&security_engine::escape_html(&target_ad.sponsor_name)));
    assert!(built.text_body.contains("%75.0 MSB"));
    assert!(built.text_body.contains(&target_ad.sponsor_name));

    let tick1 = b2b_report_worker::run_b2b_report_tick(&state, Some("2026-11"))
        .await
        .unwrap();
    assert_eq!(tick1, 1);
    let tick2 = b2b_report_worker::run_b2b_report_tick(&state, Some("2026-11"))
        .await
        .unwrap();
    assert_eq!(tick2, 0);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_req022_ad_schedule_persistence_and_conflict_409_enforcement() {
    let (state, db_path) = build_test_state("m1_req022");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "req022_admin@msidergisi.com",
        "AdminPass123!",
        "Req022 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );

    let slot = "ad-req022-test-slot";
    let base_ad = |sponsor: &str, start: &str, end: &str, rot: &str, force: bool| {
        json!({
            "id": 0,
            "slot_code": slot,
            "badge_label": "SPONSORLU",
            "dimensions": "970x90",
            "sponsor_name": sponsor,
            "sponsor_type": "defense_oem",
            "pill_style": "navy",
            "title_tr": format!("{} Kampanya", sponsor),
            "title_en": format!("{} Campaign", sponsor),
            "desc_tr": "Açıklama",
            "desc_en": "Description",
            "target_url": "https://example.com",
            "image_url": null,
            "budget_score": 80,
            "is_default_fallback": false,
            "is_floating_popup": false,
            "is_newsletter_ad": false,
            "target_categories": ["haberler"],
            "target_ssb_tags": ["req022-unique-tag"],
            "target_institutions": ["MSB"],
            "impressions_count": 0,
            "clicks_count": 0,
            "is_active": true,
            "start_date": start,
            "end_date": end,
            "rotation_mode": rot,
            "force_override": force
        })
    };

    // Invalid date range -> 400
    let (st_bad_dates, _) = admin_req_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/ads",
        &admin_cookie,
        Some(base_ad("BadDateOEM", "2026-12-10", "2026-12-01", "weighted", false)),
    )
    .await;
    assert_eq!(st_bad_dates, StatusCode::BAD_REQUEST);

    // Fill quota (= 3) in overlapping window 2026-01-01..2027-12-31
    for i in 1..=3 {
        let (st, _) = admin_req_json(
            &app,
            &state,
            "POST",
            "/api/v1/admin/ads",
            &admin_cookie,
            Some(base_ad(&format!("OEM_{}", i), "2026-01-01", "2027-12-31", "equal_rotation", false)),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "Ad #{} within quota should succeed", i);
    }

    // 4th overlapping ad WITHOUT force_override -> 409 Conflict
    let (st_conflict, conflict_body) = admin_req_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/ads",
        &admin_cookie,
        Some(base_ad("OEM_4_Blocked", "2026-06-01", "2026-07-01", "weighted", false)),
    )
    .await;
    assert_eq!(st_conflict, StatusCode::CONFLICT);
    assert_eq!(conflict_body["conflict_check"]["has_conflict"], true);
    assert!(conflict_body["error"].is_string());

    // 4th overlapping ad WITH force_override: true -> 200 OK & persisted
    let (st_override, override_body) = admin_req_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/ads",
        &admin_cookie,
        Some(base_ad("OEM_4_Forced", "2026-06-01", "2026-07-01", "equal_rotation", true)),
    )
    .await;
    assert_eq!(st_override, StatusCode::OK);
    let forced_id = unwrap_data(&override_body)["id"].as_i64().unwrap();

    // Non-overlapping future ad (2028-01-01..2028-12-31) -> 200 OK without override
    let (st_future, future_body) = admin_req_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/ads",
        &admin_cookie,
        Some(base_ad("OEM_Future", "2028-01-01", "2028-12-31", "exclusive", false)),
    )
    .await;
    assert_eq!(st_future, StatusCode::OK);
    assert_eq!(unwrap_data(&future_body)["conflict_check"]["has_conflict"], false);

    // Verify GET /api/v1/admin/ads returns persisted schedule fields
    let (st_list, ads_json) = admin_req_json(&app, &state, "GET", "/api/v1/admin/ads", &admin_cookie, None).await;
    assert_eq!(st_list, StatusCode::OK);
    let forced_row = ads_json
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == forced_id)
        .unwrap();
    assert_eq!(forced_row["start_date"], "2026-06-01");
    assert_eq!(forced_row["end_date"], "2026-07-01");
    assert_eq!(forced_row["rotation_mode"], "equal_rotation");

    // Verify ad_service::select_ads_for_context excludes future/expired ads
    let all_ads = state.db.get_all_ads();
    let selected = ad_service::select_ads_for_context(&all_ads, Some("haberler"), None, None);
    assert!(
        !selected.iter().any(|a| a.sponsor_name == "OEM_Future"),
        "Future-scheduled ad must not be selected before start_date"
    );

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_t2_hardening_utf8_dates_and_symmetric_exclusive_conflict() {
    let (state, db_path) = build_test_state("m1_t2_hard");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "t2_hard_admin@msidergisi.com",
        "AdminPass123!",
        "T2 Hard Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );

    let mk_payload = |slot: &str, sponsor: &str, start: &str, end: &str, rot: &str, tag: &str| {
        json!({
            "id": 0,
            "slot_code": slot,
            "badge_label": "SPONSORLU",
            "dimensions": "970x90",
            "sponsor_name": sponsor,
            "sponsor_type": "defense_oem",
            "pill_style": "navy",
            "title_tr": format!("{} Kampanya", sponsor),
            "title_en": format!("{} Campaign", sponsor),
            "desc_tr": "Açıklama",
            "desc_en": "Description",
            "target_url": "https://example.com",
            "image_url": null,
            "budget_score": 80,
            "is_default_fallback": false,
            "is_floating_popup": false,
            "is_newsletter_ad": false,
            "target_categories": ["haberler"],
            "target_ssb_tags": [tag],
            "target_institutions": ["MSB"],
            "impressions_count": 0,
            "clicks_count": 0,
            "is_active": true,
            "start_date": start,
            "end_date": end,
            "rotation_mode": rot,
            "force_override": false
        })
    };

    // Multi-byte UTF-8 date string around byte index 10 must NOT panic (start > end -> 400 Bad Request)
    let (st_utf8, _) = admin_req_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/ads",
        &admin_cookie,
        Some(mk_payload(
            "ad-utf8-slot",
            "Utf8OEM",
            "2026-01-0ğ999",
            "2026-01-0a999",
            "weighted",
            "utf8-tag",
        )),
    )
    .await;
    assert_eq!(st_utf8, StatusCode::BAD_REQUEST);

    // Symmetric exclusive conflict: create exclusive ad, then attempt weighted ad on same slot -> 409
    let (st_excl, _) = admin_req_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/ads",
        &admin_cookie,
        Some(mk_payload(
            "ad-sym-excl-slot",
            "ExclusiveOwner",
            "2026-01-01",
            "2026-12-31",
            "exclusive",
            "sym-excl-tag",
        )),
    )
    .await;
    assert_eq!(st_excl, StatusCode::OK);

    let (st_weighted_blocked, blocked_body) = admin_req_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/ads",
        &admin_cookie,
        Some(mk_payload(
            "ad-sym-excl-slot",
            "WeightedChallenger",
            "2026-06-01",
            "2026-07-01",
            "weighted",
            "other-tag",
        )),
    )
    .await;
    assert_eq!(st_weighted_blocked, StatusCode::CONFLICT);
    assert_eq!(blocked_body["conflict_check"]["has_conflict"], true);
    assert_eq!(blocked_body["conflict_check"]["quota_limit"], 1);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_req012_recommendation_rules_crud_and_related_engine() {
    let (state, db_path) = build_test_state("m1_req012");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "req012_admin@msidergisi.com",
        "AdminPass123!",
        "Req012 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (_, ads_mgr_cookie) = create_user_with_session(
        &state,
        "req012_adsmgr@msidergisi.com",
        "UserPass123!",
        "Req012 AdsMgr",
        "ads_manager",
        "Medya",
        "MSI Dergisi",
        false,
        "Kurumsal",
    );

    let contents = state.db.get_all_contents();
    assert!(contents.len() >= 4);
    let source_article = contents
        .iter()
        .find(|c| {
            c.status == "published"
                && !c.is_sponsored
                && !c.content_type.eq_ignore_ascii_case("advertorial")
                && !c.source_group.eq_ignore_ascii_case("msi-tdr")
        })
        .unwrap()
        .clone();
    // Pick a valid published organic article not already top-related if possible
    let target_article = contents
        .iter()
        .find(|c| {
            c.slug != source_article.slug
                && c.status == "published"
                && !c.is_sponsored
                && !c.content_type.eq_ignore_ascii_case("advertorial")
                && !c.source_group.eq_ignore_ascii_case("msi-tdr")
        })
        .unwrap()
        .clone();

    // Unauthorized role (ads_manager) cannot create rules -> 403
    let (st_denied, _) = admin_req_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/recommendation-rules",
        &ads_mgr_cookie,
        Some(json!({
            "rule_name": "Denied Rule",
            "source_tag_or_cat": "*",
            "target_institution": "*",
            "recommended_content_slug": target_article.slug,
            "priority": 900,
            "is_active": true
        })),
    )
    .await;
    assert_eq!(st_denied, StatusCode::FORBIDDEN);

    // Create active high-priority rule -> 201 Created
    let (st_create, create_json) = admin_req_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/recommendation-rules",
        &admin_cookie,
        Some(json!({
            "rule_name": "M1 Boost Rule",
            "source_tag_or_cat": "*",
            "target_institution": "*",
            "recommended_content_slug": target_article.slug,
            "priority": 500,
            "is_active": true
        })),
    )
    .await;
    assert_eq!(st_create, StatusCode::CREATED);
    let rule_id = unwrap_data(&create_json)["id"].as_i64().unwrap();

    // GET lists the new rule
    let (st_list, list_json) = admin_req_json(
        &app,
        &state,
        "GET",
        "/api/v1/admin/recommendation-rules",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st_list, StatusCode::OK);
    let rules = unwrap_data(&list_json)["rules"].as_array().unwrap();
    assert!(rules.iter().any(|r| r["id"] == rule_id && r["rule_name"] == "M1 Boost Rule"));

    // content_service::select_related_articles puts target_article #1 using L1 cache
    let all_contents = state.db.get_all_contents();
    let l1_rules = state.db.get_recommendation_rules();
    let related = content_service::select_related_articles(&source_article, &all_contents, &l1_rules, None, 3);
    assert_eq!(
        related[0].slug, target_article.slug,
        "High-priority active rule must rank target article first"
    );

    // PUT updates rule to inactive -> 200 OK
    let (st_put, _) = admin_req_json(
        &app,
        &state,
        "PUT",
        &format!("/api/v1/admin/recommendation-rules/{}", rule_id),
        &admin_cookie,
        Some(json!({
            "rule_name": "M1 Boost Rule Paused",
            "source_tag_or_cat": "*",
            "target_institution": "*",
            "recommended_content_slug": target_article.slug,
            "priority": 500,
            "is_active": false
        })),
    )
    .await;
    assert_eq!(st_put, StatusCode::OK);
    let paused_rule = state
        .db
        .get_recommendation_rules()
        .into_iter()
        .find(|r| r.id == rule_id)
        .unwrap();
    assert!(!paused_rule.is_active);

    // DELETE removes rule -> 200, second DELETE -> 404
    let (st_del1, _) = admin_req_json(
        &app,
        &state,
        "DELETE",
        &format!("/api/v1/admin/recommendation-rules/{}", rule_id),
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st_del1, StatusCode::OK);

    let (st_del2, _) = admin_req_json(
        &app,
        &state,
        "DELETE",
        &format!("/api/v1/admin/recommendation-rules/{}", rule_id),
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st_del2, StatusCode::NOT_FOUND);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_req008_custom_field_values_persistence_and_rbac() {
    let (state, db_path) = build_test_state("m1_req008");
    let app = build_test_router(&state);

    let (_, admin_cookie) = create_user_with_session(
        &state,
        "req008_admin@msidergisi.com",
        "AdminPass123!",
        "Req008 Admin",
        "super_admin",
        "Medya",
        "MSI Dergisi",
        true,
        "Kurumsal",
    );
    let (_, ads_mgr_cookie) = create_user_with_session(
        &state,
        "req008_adsmgr@msidergisi.com",
        "UserPass123!",
        "Req008 AdsMgr",
        "ads_manager",
        "Medya",
        "MSI Dergisi",
        false,
        "Kurumsal",
    );

    // Define custom fields for content & ad
    state
        .db
        .upsert_custom_field(
            "m1_source_ref",
            "Kaynak Referansı",
            "Source Reference",
            "text",
            "[]",
            false,
            1,
            "content",
        )
        .unwrap();
    state
        .db
        .upsert_custom_field(
            "m1_ad_po_code",
            "Sipariş Kodu",
            "PO Code",
            "text",
            "[]",
            false,
            1,
            "ad",
        )
        .unwrap();

    let content_field_id = state.db.get_custom_fields("content")[0]["id"].as_i64().unwrap();
    let ad_field_id = state.db.get_custom_fields("ad")[0]["id"].as_i64().unwrap();

    // Cross-entity RBAC: ads_manager cannot write content custom field values -> 403
    let (st_forbidden, _) = admin_req_json(
        &app,
        &state,
        "PUT",
        "/api/v1/admin/custom-field-values",
        &ads_mgr_cookie,
        Some(json!({
            "entity_type": "content",
            "entity_id": "1",
            "values": [{ "field_id": content_field_id, "value_text": "REF-DENIED" }]
        })),
    )
    .await;
    assert_eq!(st_forbidden, StatusCode::FORBIDDEN);

    // ads_manager CAN write ad custom field values -> 200
    let (st_ad_put, _) = admin_req_json(
        &app,
        &state,
        "PUT",
        "/api/v1/admin/custom-field-values",
        &ads_mgr_cookie,
        Some(json!({
            "entity_type": "ad",
            "entity_id": "10",
            "values": [{ "field_id": ad_field_id, "value_text": "PO-2026-99" }]
        })),
    )
    .await;
    assert_eq!(st_ad_put, StatusCode::OK);

    // Admin writes and updates content custom field values (idempotent upsert)
    let (st_put1, _) = admin_req_json(
        &app,
        &state,
        "PUT",
        "/api/v1/admin/custom-field-values",
        &admin_cookie,
        Some(json!({
            "entity_type": "content",
            "entity_id": "42",
            "values": [{ "field_id": content_field_id, "value_text": "MSB-PRESS-01" }]
        })),
    )
    .await;
    assert_eq!(st_put1, StatusCode::OK);

    let (st_put2, _) = admin_req_json(
        &app,
        &state,
        "PUT",
        "/api/v1/admin/custom-field-values",
        &admin_cookie,
        Some(json!({
            "entity_type": "content",
            "entity_id": "42",
            "values": [{ "field_id": content_field_id, "value_text": "MSB-PRESS-02" }]
        })),
    )
    .await;
    assert_eq!(st_put2, StatusCode::OK);

    let (st_get, get_json) = admin_req_json(
        &app,
        &state,
        "GET",
        "/api/v1/admin/custom-field-values?entity_type=content&entity_id=42",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st_get, StatusCode::OK);
    let values = unwrap_data(&get_json)["values"].as_array().unwrap();
    assert_eq!(values.len(), 4);
    assert_eq!(values[0]["field_key"], "m1_source_ref");
    assert_eq!(values[0]["value_text"], "MSB-PRESS-02");

    // Mismatched field_id (ad field on content entity) -> 400 Bad Request
    let (st_mismatch, _) = admin_req_json(
        &app,
        &state,
        "PUT",
        "/api/v1/admin/custom-field-values",
        &admin_cookie,
        Some(json!({
            "entity_type": "content",
            "entity_id": "42",
            "values": [{ "field_id": ad_field_id, "value_text": "WRONG" }]
        })),
    )
    .await;
    assert_eq!(st_mismatch, StatusCode::BAD_REQUEST);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_paytr_callback_both_routes_csrf_exempt_and_hmac_verified() {
    use base64::Engine as _;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let (state, db_path) = build_test_state("m1_paytr_alias");
    let app = build_test_router(&state);

    let (reader, _) = create_user_with_session(
        &state,
        "paytr.alias@example.com",
        "UserPass123!",
        "PayTR Alias User",
        "registered",
        "Savunma",
        "ASELSAN",
        false,
        "free",
    );

    let make_hash = |oid: &str, status: &str, total_amount: &str| -> String {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(state.config.paytr_merchant_key.as_bytes()).unwrap();
        let data = format!(
            "{}{}{}{}",
            oid, state.config.paytr_merchant_salt, status, total_amount
        );
        mac.update(data.as_bytes());
        base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
    };

    for (idx, path) in [
        "/api/v1/payments/paytr-callback",
        "/api/v1/payments/paytr/callback",
    ]
    .iter()
    .enumerate()
    {
        let oid = format!("MSIINVALIAS{}", idx + 1);
        let inv = msi_dergisi_platform::models::OrderInvoice {
            id: 0,
            invoice_no: format!("INV-ALIAS-{}", idx + 1),
            user_email: reader.email.clone(),
            full_name: reader.full_name.clone(),
            organization_name: reader.organization_name.clone(),
            item_type: "subscription".to_string(),
            item_code: "dijital".to_string(),
            item_title: "Dijital E-Dergi Aboneliği".to_string(),
            amount_try: 990.0,
            payment_status: "pending".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            pdf_watermark_code: String::new(),
        };
        state.db.insert_order_invoice(&inv).unwrap();

        // 1. Bad HMAC without CSRF token -> 403 bad hash (NOT CSRF error)
        let bad_body = format!(
            "merchant_oid={}&status=success&total_amount=99000&hash=invalidhash",
            oid
        );
        let resp_bad = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(*path)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(bad_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_bad.status(), StatusCode::FORBIDDEN);
        let bad_txt = String::from_utf8_lossy(
            &to_bytes(resp_bad.into_body(), 1024 * 1024).await.unwrap(),
        )
        .to_string();
        assert!(
            bad_txt.contains("bad hash"),
            "Expected bad hash rejection on {}, got {}",
            path,
            bad_txt
        );

        // 2. Valid HMAC without CSRF token -> 200 "OK" and marks invoice paid
        let valid_hash = make_hash(&oid, "success", "99000");
        let ok_body = format!(
            "merchant_oid={}&status=success&total_amount=99000&hash={}",
            oid,
            urlencoding::encode(&valid_hash)
        );
        let resp_ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(*path)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(ok_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_ok.status(), StatusCode::OK);
        let ok_txt = String::from_utf8_lossy(
            &to_bytes(resp_ok.into_body(), 1024 * 1024).await.unwrap(),
        )
        .to_string();
        assert_eq!(ok_txt, "OK");
        let saved_inv = state.db.get_invoice_by_oid(&oid).expect("invoice exists");
        assert_eq!(saved_inv.payment_status, "paid");
    }

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_operational_defaults_seeded_and_public_related_respects_sponsored_exclusion() {
    let (state, db_path) = build_test_state("m1_op_defaults");
    let app = build_test_router(&state);

    // 1. Custom fields seeded idempotently with is_required=false and sort_order >= 10
    let content_fields = state.db.get_custom_fields("content");
    let ad_fields = state.db.get_custom_fields("ad");
    let user_fields = state.db.get_custom_fields("user");
    assert_eq!(content_fields.len(), 3);
    assert_eq!(ad_fields.len(), 2);
    assert_eq!(user_fields.len(), 2);
    for f in content_fields
        .iter()
        .chain(ad_fields.iter())
        .chain(user_fields.iter())
    {
        assert_eq!(f["is_required"], json!(false));
        assert!(f["sort_order"].as_i64().unwrap() >= 10);
    }

    // Idempotency check
    {
        let conn = state.db.open_conn().unwrap();
        msi_dergisi_platform::db::seed::taxonomy::seed_operational_defaults(&conn).unwrap();
    }
    assert_eq!(state.db.get_custom_fields("content").len(), 3);

    // 2. /api/v1/public/related returns >= 3 items for organic article & [] for sponsored article
    let all_contents = state.db.get_all_contents();
    let organic = all_contents
        .iter()
        .find(|c| c.status == "published" && !c.is_sponsored && !c.ssb_tags.is_empty())
        .expect("seeded organic article with ssb_tags");
    let (st_rel, _, rel_bytes) = send_req(
        &app,
        "GET",
        &format!("/api/v1/public/related?slug={}&limit=4", organic.slug),
        None,
        None,
    )
    .await;
    assert_eq!(st_rel, StatusCode::OK);
    let rel_json: Value = serde_json::from_slice(&rel_bytes).unwrap();
    let rel_items = unwrap_data(&rel_json)["items"].as_array().unwrap();
    assert!(
        rel_items.len() >= 3,
        "Expected >= 3 related articles for {}, got {}",
        organic.slug,
        rel_items.len()
    );

    if let Some(sponsored) = all_contents
        .iter()
        .find(|c| c.status == "published" && c.is_sponsored)
    {
        let (st_sp, _, sp_bytes) = send_req(
            &app,
            "GET",
            &format!("/api/v1/public/related?slug={}", sponsored.slug),
            None,
            None,
        )
        .await;
        assert_eq!(st_sp, StatusCode::OK);
        let sp_json: Value = serde_json::from_slice(&sp_bytes).unwrap();
        let sp_items = unwrap_data(&sp_json)["items"].as_array().unwrap();
        assert!(
            sp_items.is_empty(),
            "Sponsored content {} must return [] related items",
            sponsored.slug
        );
    }

    cleanup_test_db(&db_path);
}
