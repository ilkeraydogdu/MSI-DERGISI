mod common;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use msi_dergisi_platform::{
    controllers::AppState,
    engine::{
        design::dialogs::{self, DialogContext},
        security_engine,
        translation_engine::TranslationCache,
    },
    models::MagazineIssue,
    services::edergi_pdf_service,
};
use serde_json::{json, Value};

fn m2_test_state(label: &str) -> (AppState, String, String) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!("/tmp/msi_m2_{}_{}_{}.db", label, std::process::id(), ts);
    let edergi_dir = format!("/tmp/msi_m2_edergi_{}_{}_{}", label, std::process::id(), ts);
    let _ = std::fs::create_dir_all(&edergi_dir);
    let mut cfg = common::test_config(&db_path);
    cfg.edergi_dir = edergi_dir.clone();
    let db = msi_dergisi_platform::db::Database::new(&db_path).expect("test db");
    let translations = TranslationCache::new();
    translations.load(db.load_all_translations());
    let state = AppState {
        config: std::sync::Arc::new(cfg),
        db,
        translations,
        monitoring_cache: None,
    };
    (state, db_path, edergi_dir)
}

fn cleanup_m2(db_path: &str, edergi_dir: &str) {
    let _ = std::fs::remove_dir_all(edergi_dir);
    common::cleanup_test_db(db_path);
}

fn csrf_pair(state: &AppState) -> String {
    security_engine::generate_csrf_token(&state.config.session_secret)
}

async fn send_authed_json(
    app: &axum::Router,
    state: &AppState,
    method: &str,
    uri: &str,
    session_cookie: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let csrf = csrf_pair(state);
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, format!("{}; csrf_token={}", session_cookie, csrf))
        .header("x-csrf-token", &csrf);
    let req_body = if let Some(v) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(v.to_string())
    } else {
        Body::empty()
    };
    let resp = app
        .clone()
        .oneshot_or_call(builder.body(req_body).unwrap())
        .await;
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, val)
}

trait OneshotExt {
    async fn oneshot_or_call(self, req: Request<Body>) -> axum::response::Response;
}

impl OneshotExt for axum::Router {
    async fn oneshot_or_call(self, req: Request<Body>) -> axum::response::Response {
        use tower::ServiceExt;
        self.oneshot(req).await.unwrap()
    }
}

#[tokio::test]
async fn req040_composite_pk_crud_and_scoped_delete() {
    let (state, db_path, edergi_dir) = m2_test_state("req040");
    let app = common::build_test_router(&state);

    let (_, admin_cookie) = common::create_user_with_session(
        &state,
        "admin-m2@msidergisi.com",
        "Pass1234!",
        "Admin M2",
        "super_admin",
        "MSI",
        "UMSA",
        false,
        "kurumsal",
    );
    let (_, sub_cookie) = common::create_user_with_session(
        &state,
        "sub-m2@aselsan.com.tr",
        "Pass1234!",
        "Abone M2",
        "subscriber_digital",
        "Savunma",
        "ASELSAN",
        false,
        "Dijital E-Dergi",
    );

    let msi_150 = json!({
        "issue_number": 150,
        "publication_type": "msi",
        "slug": "",
        "title_tr": "MSI Dergisi Sayi 150",
        "title_en": "",
        "publish_month_tr": "Ocak",
        "publish_month_en": "",
        "year": 2026,
        "cover_image": "images/msi/default.webp",
        "is_special_issue": false,
        "event_name": null,
        "page_count": 96,
        "dossier_count": 4,
        "summary_tr": "MSI 150 ozeti",
        "summary_en": "",
        "pdf_path": "",
        "uploaded_pdf_file": null,
        "spreads": []
    });
    let tdr_150 = json!({
        "issue_number": 150,
        "publication_type": "msi_tdr",
        "slug": "",
        "title_tr": "MSI TDR Sayi 150",
        "title_en": "MSI TDR Issue 150",
        "publish_month_tr": "Ocak",
        "publish_month_en": "January",
        "year": 2026,
        "cover_image": "images/msi/default.webp",
        "is_special_issue": false,
        "event_name": null,
        "page_count": 64,
        "dossier_count": 3,
        "summary_tr": "TDR 150 ozeti",
        "summary_en": "TDR 150 summary",
        "pdf_path": "",
        "uploaded_pdf_file": null,
        "spreads": []
    });

    let (st, msi_res) =
        send_authed_json(&app, &state, "POST", "/api/v1/admin/issues", &admin_cookie, Some(msi_150))
            .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        msi_res["data"]["issue"]["pdf_path"],
        "/api/v1/edergi/150/download-pdf?publication_type=msi"
    );

    let (st, tdr_res) =
        send_authed_json(&app, &state, "POST", "/api/v1/admin/issues", &admin_cookie, Some(tdr_150.clone()))
            .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        tdr_res["data"]["issue"]["pdf_path"],
        "/api/v1/edergi/150/download-pdf?publication_type=msi_tdr"
    );

    assert_eq!(state.db.get_issues_by_number(150).len(), 2);

    // Invalid publication_type rejected with 400
    let (st, _) = send_authed_json(
        &app,
        &state,
        "GET",
        "/api/v1/admin/issues/invalid_pub/150",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // GET by composite key returns distinct issues
    let (st, get_tdr) = send_authed_json(
        &app,
        &state,
        "GET",
        "/api/v1/admin/issues/msi_tdr/150",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(get_tdr["data"]["title_en"], "MSI TDR Issue 150");

    // PUT by composite key updates ONLY msi_tdr #150
    let mut updated_tdr = tdr_150;
    updated_tdr["title_en"] = json!("MSI TDR Issue 150 Updated");
    let (st, put_res) = send_authed_json(
        &app,
        &state,
        "PUT",
        "/api/v1/admin/issues/msi_tdr/150",
        &admin_cookie,
        Some(updated_tdr),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(put_res["data"]["issue"]["title_en"], "MSI TDR Issue 150 Updated");
    assert_eq!(
        state.db.get_issue("msi", 150).unwrap().title_tr,
        "MSI Dergisi Sayi 150"
    );

    // Public PDF download with ambiguous issue 150 (no publication_type) returns 409 Conflict
    use tower::ServiceExt;
    let amb_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/150/download-pdf")
                .header(header::COOKIE, &sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(amb_resp.status(), StatusCode::CONFLICT);

    // Public PDF download requesting non-existent publication_type returns 404 (no silent fallback)
    let only_msi = MagazineIssue {
        issue_number: 951,
        slug: "msi-951".into(),
        title_tr: "MSI 951".into(),
        publication_type: "msi".into(),
        ..Default::default()
    };
    state.db.upsert_issue(&only_msi).unwrap();
    let no_fallback_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/951/download-pdf?publication_type=msi_tdr")
                .header(header::COOKIE, &sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_fallback_resp.status(), StatusCode::NOT_FOUND);

    // Insert a content row belonging to MSI Dergisi (source_group = 'dergi-icerigi') for issue 150
    {
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "INSERT INTO contents (slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en, content_type, source_group, cover_image, author_name, author_title, issue_number, published_at, status)
             VALUES ('msi-art-150', 'MSI Art 150', '', 'S', '', '<p>B</p>', '', 'haberler', 'dergi-icerigi', 'c.webp', 'Auth', 'Ed', 150, '2026-11-18 10:00:00', 'published')",
            [],
        )
        .unwrap();
    }

    // Legacy DELETE /api/v1/admin/issues/150 returns 409 because both msi and msi_tdr exist
    let (st, amb_del) = send_authed_json(
        &app,
        &state,
        "DELETE",
        "/api/v1/admin/issues/150",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(amb_del["data"]["publication_types"].as_array().unwrap().len(), 2);

    // Composite DELETE /api/v1/admin/issues/msi_tdr/150 succeeds cleanly without detaching dergi-icerigi content or deleting msi #150
    let (st, _) = send_authed_json(
        &app,
        &state,
        "DELETE",
        "/api/v1/admin/issues/msi_tdr/150",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(state.db.get_issue("msi_tdr", 150).is_none());
    assert!(state.db.get_issue("msi", 150).is_some());

    // Now legacy DELETE /api/v1/admin/issues/150 finds single match (msi) -> 409 for content ref -> ?force=true deletes
    let (st, _) = send_authed_json(
        &app,
        &state,
        "DELETE",
        "/api/v1/admin/issues/150",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);

    let (st, force_del) = send_authed_json(
        &app,
        &state,
        "DELETE",
        "/api/v1/admin/issues/150?force=true",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(force_del["data"]["detached_contents"], 1);
    assert!(state.db.get_issue("msi", 150).is_none());

    cleanup_m2(&db_path, &edergi_dir);
}

#[tokio::test]
async fn req018_req019_req020_convert_stream_and_partial_export() {
    let (state, db_path, edergi_dir) = m2_test_state("full");
    let app = common::build_test_router(&state);

    let (_, admin_cookie) = common::create_user_with_session(
        &state,
        "eic-m2@msidergisi.com",
        "Pass1234!",
        "Editor M2",
        "editor_in_chief",
        "MSI",
        "UMSA",
        false,
        "kurumsal",
    );
    let (_, free_cookie) = common::create_user_with_session(
        &state,
        "free-m2@example.com",
        "Pass1234!",
        "Free Reader",
        "reader",
        "Bireysel",
        "Bireysel",
        false,
        "free",
    );

    // Unverified subscriber vs verified subscriber with injection characters in name/org
    let (_, unverified_sub_cookie) = {
        let mut u = common::fixture_subscriber_user(91);
        u.email = "unver-m2@aselsan.com.tr".into();
        u.email_verified = false;
        let pw_hash = msi_dergisi_platform::services::auth_service::hash_password(
            "Pass1234!",
            &state.config.session_secret,
        );
        let uid = state.db.create_or_update_user(&u, &pw_hash).unwrap();
        let saved = state.db.get_user_by_id(uid).unwrap();
        let tok = msi_dergisi_platform::services::auth_service::generate_session_token(
            &saved,
            &state.config.session_secret,
        );
        (saved, format!("msi_session={}", tok))
    };

    let (_, verified_sub_cookie) = common::create_user_with_session(
        &state,
        "drm-m2@aselsan.com.tr",
        "Pass1234!",
        "@/etc/passwd %x %[fx:w] \"Ahmet\"",
        "subscriber_digital",
        "Savunma",
        "ASELSAN <script>alert(1)</script>",
        false,
        "Dijital E-Dergi",
    );

    // Copy real 4-page PDF into isolated edergi_dir
    let real_4p = std::fs::read(
        "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/docs/specifications/proje-plani-ve-teslim-takvimi.pdf",
    )
    .expect("real 4-page spec PDF must exist");
    std::fs::write(format!("{}/real-4p.pdf", edergi_dir), &real_4p).unwrap();

    // 1. REQ-019: Create issue #995 and run one-click convert-pdf
    let payload_995 = json!({
        "issue_number": 995,
        "publication_type": "msi",
        "slug": "msi-995",
        "title_tr": "Test Sayi 995",
        "title_en": "",
        "publish_month_tr": "Kasim",
        "publish_month_en": "",
        "year": 2026,
        "cover_image": "images/msi/default.webp",
        "is_special_issue": false,
        "event_name": null,
        "page_count": 1,
        "dossier_count": 0,
        "summary_tr": "Ozet 995",
        "summary_en": "",
        "pdf_path": "",
        "uploaded_pdf_file": "real-4p.pdf",
        "spreads": []
    });
    let (st, _) = send_authed_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/issues",
        &admin_cookie,
        Some(payload_995),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    let (st, conv) = send_authed_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/issues/msi/995/convert-pdf",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(conv["data"]["converted"], true);
    assert_eq!(conv["data"]["page_count"], 4);
    assert_eq!(conv["data"]["cached_pages_count"], 4);
    let spreads = conv["data"]["spreads"].as_array().unwrap();
    assert!(!spreads.is_empty());
    assert!(spreads[0]["image_url"]
        .as_str()
        .unwrap()
        .contains("/api/v1/edergi/995/pages/"));

    let saved_995 = state.db.get_issue("msi", 995).unwrap();
    assert_eq!(saved_995.page_count, 4);
    assert!(!saved_995.spreads.is_empty());
    assert_eq!(
        edergi_pdf_service::count_cached_pages(&edergi_dir, "msi", 995),
        4
    );

    // Also test ?auto_convert=true on POST /api/v1/admin/issues
    let payload_996 = json!({
        "issue_number": 996,
        "publication_type": "msi_tdr",
        "slug": "tdr-996",
        "title_tr": "Test TDR 996",
        "title_en": "Test TDR 996",
        "publish_month_tr": "Kasim",
        "publish_month_en": "November",
        "year": 2026,
        "cover_image": "images/msi/default.webp",
        "is_special_issue": false,
        "event_name": null,
        "page_count": 1,
        "dossier_count": 0,
        "summary_tr": "Ozet",
        "summary_en": "Summary",
        "pdf_path": "",
        "uploaded_pdf_file": "real-4p.pdf",
        "spreads": [],
        "auto_convert": true
    });
    let (st, auto_res) = send_authed_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/issues",
        &admin_cookie,
        Some(payload_996),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(auto_res["data"]["converted"], true);
    assert_eq!(auto_res["data"]["issue"]["page_count"], 4);

    // 2. REQ-018: Watermarked Flipbook Page Streaming + RBAC + IM sanitization
    use tower::ServiceExt;

    // Guest -> 401
    let guest_pg = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/pages/1?publication_type=msi")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(guest_pg.status(), StatusCode::UNAUTHORIZED);

    // Free user -> 403
    let free_pg = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/pages/1?publication_type=msi")
                .header(header::COOKIE, &free_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(free_pg.status(), StatusCode::FORBIDDEN);

    // Unverified subscriber -> 403
    let unver_pg = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/pages/1?publication_type=msi")
                .header(header::COOKIE, &unverified_sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unver_pg.status(), StatusCode::FORBIDDEN);

    // Staff (editor_in_chief) -> 200 image/webp
    let staff_pg = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/pages/1?publication_type=msi")
                .header(header::COOKIE, &admin_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(staff_pg.status(), StatusCode::OK);

    // Verified subscriber with tricky IM/XSS chars -> 200 image/webp
    let sub_pg = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/pages/1?publication_type=msi")
                .header(header::COOKIE, &verified_sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sub_pg.status(), StatusCode::OK);
    assert_eq!(
        sub_pg.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/webp"
    );
    let cache_ctrl = sub_pg
        .headers()
        .get(header::CACHE_CONTROL)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cache_ctrl.contains("private") && cache_ctrl.contains("no-store"));
    assert!(sub_pg
        .headers()
        .get("x-msi-drm-watermark")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("MSI-DRM-"));
    assert_eq!(sub_pg.headers().get("x-msi-page-count").unwrap(), "4");
    let webp_bytes = to_bytes(sub_pg.into_body(), 4 * 1024 * 1024).await.unwrap();
    assert!(webp_bytes.len() > 100);
    assert_eq!(&webp_bytes[..4], b"RIFF");
    assert_eq!(&webp_bytes[8..12], b"WEBP");

    // Page 0 -> 400; Page 99 -> 404 with X-MSI-Page-Count: 4
    let p0 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/pages/0?publication_type=msi")
                .header(header::COOKIE, &verified_sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(p0.status(), StatusCode::BAD_REQUEST);

    let p99 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/pages/99?publication_type=msi")
                .header(header::COOKIE, &verified_sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(p99.status(), StatusCode::NOT_FOUND);
    assert_eq!(p99.headers().get("x-msi-page-count").unwrap(), "4");

    // SVG fallback issue (no physical PDF) escapes XML safely
    let svg_issue = MagazineIssue {
        issue_number: 997,
        slug: "svg-997".into(),
        title_tr: "SVG Test <script>".into(),
        publication_type: "msi".into(),
        page_count: 2,
        ..Default::default()
    };
    state.db.upsert_issue(&svg_issue).unwrap();

    let svg_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/997/pages/1?publication_type=msi")
                .header(header::COOKIE, &verified_sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(svg_resp.status(), StatusCode::OK);
    assert!(svg_resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("image/svg+xml"));
    let svg_body = String::from_utf8(
        to_bytes(svg_resp.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(svg_body.contains("<svg "));
    assert!(!svg_body.contains("<script>"));
    assert!(svg_body.contains("&lt;script&gt;"));

    // 3. REQ-020: Multi-range watermarked PDF export (`?pages=1-2,4`) & invalid syntax 400
    assert_eq!(
        edergi_pdf_service::parse_page_selection(Some("1-5, 8")),
        Ok(Some(vec![1, 2, 3, 4, 5, 8]))
    );
    assert!(edergi_pdf_service::parse_page_selection(Some("abc")).is_err());
    assert!(edergi_pdf_service::parse_page_selection(Some("0")).is_err());
    assert!(edergi_pdf_service::parse_page_selection(Some("1,,2")).is_err());

    let bad_pages = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/download-pdf?pages=abc&publication_type=msi")
                .header(header::COOKIE, &verified_sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_pages.status(), StatusCode::BAD_REQUEST);

    let out_of_range = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/download-pdf?pages=99&publication_type=msi")
                .header(header::COOKIE, &verified_sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(out_of_range.status(), StatusCode::BAD_REQUEST);

    let mixed_out_of_range = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/download-pdf?pages=1,99&publication_type=msi")
                .header(header::COOKIE, &verified_sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mixed_out_of_range.status(), StatusCode::BAD_REQUEST);

    let slice_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/995/download-pdf?pages=1-2,4&publication_type=msi")
                .header(header::COOKIE, &verified_sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(slice_resp.status(), StatusCode::OK);
    let drm_code = slice_resp
        .headers()
        .get("x-msi-drm-watermark")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let pdf_out = to_bytes(slice_resp.into_body(), 8 * 1024 * 1024).await.unwrap();
    assert!(pdf_out.starts_with(b"%PDF-"));
    assert_eq!(edergi_pdf_service::extract_pdf_page_count(&pdf_out), Some(3));
    let pdf_str = String::from_utf8_lossy(&pdf_out);
    assert!(pdf_str.contains(&format!("%MSI-DRM:{}", drm_code)));

    // Verify with gs & pdftotext if installed
    let verify_pdf = format!("{}/verify_slice.pdf", edergi_dir);
    std::fs::write(&verify_pdf, &pdf_out).unwrap();
    if let Ok(gs_out) = std::process::Command::new("gs")
        .args(["-dBATCH", "-dNOPAUSE", "-q", "-sDEVICE=nullpage", &verify_pdf])
        .output()
    {
        assert!(
            gs_out.status.success() && String::from_utf8_lossy(&gs_out.stderr).trim().is_empty(),
            "gs failed: {}",
            String::from_utf8_lossy(&gs_out.stderr)
        );
    }

    // 4. SSR dialog markup check for M2 controls
    let html_sub = dialogs::render(&DialogContext {
        lang: "tr",
        tr: None,
        institutions: &[],
        can_read_pdf: true,
        featured_issue: Some(995),
    });
    assert!(html_sub.contains("id=\"issue-tab-flipbook\""));
    assert!(html_sub.contains("id=\"issue-flipbook-panel\""));
    assert!(html_sub.contains("class=\"edergi-export-box\""));
    assert!(html_sub.contains("id=\"edergi-pages-input\""));
    assert!(html_sub.contains("id=\"issue-pdf-frame\""));

    cleanup_m2(&db_path, &edergi_dir);
}

#[tokio::test]
async fn req018_unconverted_multipage_lazy_page_streaming_and_cache_invalidation() {
    let (state, db_path, edergi_dir) = m2_test_state("lazy_unconverted");
    let app = common::build_test_router(&state);

    let (_, admin_cookie) = common::create_user_with_session(
        &state,
        "eic-lazy@msidergisi.com",
        "Pass1234!",
        "Editor Lazy",
        "editor_in_chief",
        "MSI",
        "UMSA",
        false,
        "kurumsal",
    );
    let (_, sub_cookie) = common::create_user_with_session(
        &state,
        "sub-lazy@aselsan.com.tr",
        "Pass1234!",
        "Abone Lazy",
        "subscriber_digital",
        "Savunma",
        "ASELSAN",
        false,
        "Dijital E-Dergi",
    );

    let real_4p = std::fs::read(
        "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/docs/specifications/proje-plani-ve-teslim-takvimi.pdf",
    )
    .expect("real 4-page spec PDF must exist");
    std::fs::write(format!("{}/unconverted-4p.pdf", edergi_dir), &real_4p).unwrap();

    let real_6p = std::fs::read(
        "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/docs/specifications/altyapi-gereksinimleri-v6.pdf",
    )
    .expect("real 6-page spec PDF must exist");
    std::fs::write(format!("{}/unconverted-6p.pdf", edergi_dir), &real_6p).unwrap();

    // Create issue WITHOUT running convert-pdf (auto_convert omitted/false)
    let payload_998 = json!({
        "issue_number": 998,
        "publication_type": "msi",
        "slug": "msi-998",
        "title_tr": "Unconverted Sayi 998",
        "title_en": "",
        "publish_month_tr": "Kasim",
        "publish_month_en": "",
        "year": 2026,
        "cover_image": "images/msi/default.webp",
        "is_special_issue": false,
        "event_name": null,
        "page_count": 1,
        "dossier_count": 0,
        "summary_tr": "Ozet 998",
        "summary_en": "",
        "pdf_path": "",
        "uploaded_pdf_file": "unconverted-4p.pdf",
        "spreads": []
    });
    let (st, _) = send_authed_json(
        &app,
        &state,
        "POST",
        "/api/v1/admin/issues",
        &admin_cookie,
        Some(payload_998.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(edergi_pdf_service::count_cached_pages(&edergi_dir, "msi", 998), 0);

    use tower::ServiceExt;

    // Request page 1 lazily -> 200 image/webp, X-MSI-Page-Count: 4 (populates page-0001.jpg so cached_cnt == 1)
    let p1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/998/pages/1?publication_type=msi")
                .header(header::COOKIE, &sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(p1.status(), StatusCode::OK);
    assert_eq!(p1.headers().get(header::CONTENT_TYPE).unwrap(), "image/webp");
    assert_eq!(p1.headers().get("x-msi-page-count").unwrap(), "4");
    assert_eq!(edergi_pdf_service::count_cached_pages(&edergi_dir, "msi", 998), 1);

    // Immediately request page 2 lazily -> MUST ALSO return 200 image/webp with X-MSI-Page-Count: 4
    let p2 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/998/pages/2?publication_type=msi")
                .header(header::COOKIE, &sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(p2.status(), StatusCode::OK);
    assert_eq!(p2.headers().get(header::CONTENT_TYPE).unwrap(), "image/webp");
    assert_eq!(p2.headers().get("x-msi-page-count").unwrap(), "4");
    let p2_bytes = to_bytes(p2.into_body(), 4 * 1024 * 1024).await.unwrap();
    assert!(p2_bytes.len() > 100);
    assert_eq!(&p2_bytes[..4], b"RIFF");
    assert_eq!(&p2_bytes[8..12], b"WEBP");
    assert_eq!(edergi_pdf_service::count_cached_pages(&edergi_dir, "msi", 998), 2);

    // Out-of-bounds page 5 still returns 404 with X-MSI-Page-Count: 4
    let p5 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/998/pages/5?publication_type=msi")
                .header(header::COOKIE, &sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(p5.status(), StatusCode::NOT_FOUND);
    assert_eq!(p5.headers().get("x-msi-page-count").unwrap(), "4");

    // Replacing uploaded_pdf_file via PUT clears stale .pages/msi_998 cache and reflects new 6-page PDF
    let mut updated_998 = payload_998;
    updated_998["uploaded_pdf_file"] = json!("unconverted-6p.pdf");
    let (st, _) = send_authed_json(
        &app,
        &state,
        "PUT",
        "/api/v1/admin/issues/msi/998",
        &admin_cookie,
        Some(updated_998),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(edergi_pdf_service::count_cached_pages(&edergi_dir, "msi", 998), 0);

    let p6 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/998/pages/6?publication_type=msi")
                .header(header::COOKIE, &sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(p6.status(), StatusCode::OK);
    assert_eq!(p6.headers().get(header::CONTENT_TYPE).unwrap(), "image/webp");
    assert_eq!(p6.headers().get("x-msi-page-count").unwrap(), "6");

    cleanup_m2(&db_path, &edergi_dir);
}

#[tokio::test]
async fn starter_archive_multipage_pdf_streams_page4_and_downloads_slice() {
    let (state, db_path, edergi_dir) = m2_test_state("starter_digest");
    let app = common::build_test_router(&state);

    let (_, sub_cookie) = common::create_user_with_session(
        &state,
        "starter-sub@aselsan.com.tr",
        "Pass1234!",
        "Starter Abone",
        "subscriber_digital",
        "Savunma",
        "ASELSAN",
        false,
        "Dijital E-Dergi",
    );

    let issue = MagazineIssue {
        issue_number: 286,
        slug: "msi-dergisi-sayi-286".into(),
        title_tr: "MSI Dergisi Sayı 286".into(),
        title_en: "MSI Magazine Issue 286".into(),
        publish_month_tr: "Kasım".into(),
        publish_month_en: "November".into(),
        year: 2026,
        page_count: 120,
        dossier_count: 10,
        summary_tr: "ARES Tersanesi 20. Yıl Kapak Dosyası ve Denizhan-76 özel incelemesi.".into(),
        summary_en: "ARES Shipyard 20th Anniversary Cover Story and Denizhan-76 special review.".into(),
        publication_type: "msi".into(),
        uploaded_pdf_file: Some("msi-dergisi-sayi-286.pdf".into()),
        ..Default::default()
    };
    state.db.upsert_issue(&issue).unwrap();
    let contents = state.db.get_all_contents();
    let pdf_bytes = edergi_pdf_service::generate_starter_archive_pdf(&issue, &contents);
    let page_cnt = edergi_pdf_service::extract_pdf_page_count(&pdf_bytes).unwrap_or(0);
    assert!(
        (4..=6).contains(&page_cnt),
        "Starter archive PDF must have 4-6 pages, got {}",
        page_cnt
    );
    std::fs::write(
        std::path::Path::new(&edergi_dir).join("msi-dergisi-sayi-286.pdf"),
        &pdf_bytes,
    )
    .unwrap();

    use tower::ServiceExt;
    for page_no in [1, 4] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/edergi/286/pages/{}?publication_type=msi", page_no))
                    .header(header::COOKIE, &sub_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(header::CONTENT_TYPE).unwrap(), "image/webp");
    }

    let dl_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/edergi/286/download-pdf?publication_type=msi&pages=1-3")
                .header(header::COOKIE, &sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dl_resp.status(), StatusCode::OK);
    assert!(dl_resp.headers().get("x-msi-drm-watermark").is_some());
    let dl_bytes = to_bytes(dl_resp.into_body(), 4 * 1024 * 1024).await.unwrap();
    assert!(dl_bytes.starts_with(b"%PDF-"));
    assert_eq!(edergi_pdf_service::extract_pdf_page_count(&dl_bytes), Some(3));

    cleanup_m2(&db_path, &edergi_dir);
}
