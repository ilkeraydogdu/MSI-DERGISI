//! Integration tests for MSI Dergisi Editorial Studio (`/podmin/editor`, `/api/v1/admin/editorial/*`).

mod common;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;

use common::test_config;
use msi_dergisi_platform::{
    app_state::AppState,
    db::Database,
    engine::{security_engine, translation_engine::TranslationCache},
    middleware::central_waf_and_telemetry_middleware,
    routes,
    services::{auth_service, editorial_media_service},
};

struct TempStudioEnv {
    root: PathBuf,
    state: AppState,
    app: axum::Router,
}

impl Drop for TempStudioEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn setup_env(label: &str) -> TempStudioEnv {
    let pid = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = PathBuf::from(format!("/tmp/msi_editorial_test_{}_{}_{}", label, pid, ts));
    let db_dir = root.join("db");
    std::fs::create_dir_all(&db_dir).expect("create db_dir");
    let db_path = db_dir.join("editorial.db").to_string_lossy().to_string();

    let mut cfg = test_config(&db_path);
    cfg.static_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/public").to_string();
    let upload_dir = editorial_media_service::resolve_editorial_upload_dir(&cfg);
    std::fs::create_dir_all(&upload_dir).expect("create upload_dir");

    let db = Database::new(&db_path).expect("create test db");
    let tr = TranslationCache::new();
    tr.load(db.load_all_translations());

    let state = AppState {
        config: Arc::new(cfg),
        db,
        translations: tr,
        monitoring_cache: None,
    };

    let app = routes::build_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        central_waf_and_telemetry_middleware,
    ));

    TempStudioEnv { root, state, app }
}

fn create_staff_cookie(state: &AppState, email: &str, full_name: &str, role: &str) -> String {
    let hash = auth_service::hash_password("EditorialPass!2026", &state.config.session_secret);
    let uid = state
        .db
        .admin_create_user(
            email,
            &hash,
            full_name,
            role,
            "Sektör Kuruluşları",
            "MSI Savunma Yayınları",
            "Editör",
            true,
            "",
        )
        .expect("create staff user");
    let _ = state.db.mark_email_verified(uid);
    let user = state
        .db
        .get_all_users()
        .into_iter()
        .find(|u| u.id == uid)
        .expect("find created staff user");
    let token = auth_service::generate_session_token(&user, &state.config.session_secret);
    format!("msi_session={}", token)
}

async fn send_raw(
    app: &axum::Router,
    state: &AppState,
    method: &str,
    uri: &str,
    session_cookie: Option<&str>,
    content_type: Option<&str>,
    body: Body,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let csrf = security_engine::generate_csrf_token(&state.config.session_secret);
    let cookie = match session_cookie {
        Some(sc) => format!("{sc}; csrf_token={csrf}"),
        None => format!("csrf_token={csrf}"),
    };
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .header("x-csrf-token", &csrf);
    if let Some(ct) = content_type {
        builder = builder.header(header::CONTENT_TYPE, ct);
    }
    let resp = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .expect("oneshot request");
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = to_bytes(resp.into_body(), 8 * 1024 * 1024).await.unwrap();
    (status, headers, bytes.to_vec())
}

async fn send_json(
    app: &axum::Router,
    state: &AppState,
    method: &str,
    uri: &str,
    session_cookie: Option<&str>,
    payload: Option<Value>,
) -> (StatusCode, Value) {
    let (ct, body) = match payload {
        Some(v) => (Some("application/json"), Body::from(v.to_string())),
        None => (None, Body::empty()),
    };
    let (st, _, raw) = send_raw(app, state, method, uri, session_cookie, ct, body).await;
    let parsed = serde_json::from_slice(&raw).unwrap_or_else(|_| {
        json!({ "raw": String::from_utf8_lossy(&raw).to_string() })
    });
    (st, parsed)
}

fn build_test_png_bytes(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgba8(width, height);
    let mut cursor = std::io::Cursor::new(Vec::new());
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .expect("encode png");
    cursor.into_inner()
}

#[tokio::test]
async fn unauthenticated_and_non_editorial_roles_are_blocked() {
    let env = setup_env("auth_rbac_gate");

    // 1. Unauthenticated SSR request to /podmin/editor redirects to /podmin
    let (st_page, headers_page, _) =
        send_raw(&env.app, &env.state, "GET", "/podmin/editor", None, None, Body::empty()).await;
    assert!(
        matches!(st_page, StatusCode::SEE_OTHER | StatusCode::FOUND | StatusCode::TEMPORARY_REDIRECT),
        "expected redirect on unauthenticated /podmin/editor, got {}",
        st_page
    );
    assert_eq!(
        headers_page.get(header::LOCATION).and_then(|v| v.to_str().ok()),
        Some("/podmin")
    );

    // 2. Unauthenticated API session returns 401
    let (st_sess, _) = send_json(
        &env.app,
        &env.state,
        "GET",
        "/api/v1/admin/editorial/session",
        None,
        None,
    )
    .await;
    assert_eq!(st_sess, StatusCode::UNAUTHORIZED);

    // 3. ads_manager has no contents:write -> 403 on both SSR shell and API session
    let ads_cookie = create_staff_cookie(
        &env.state,
        "ads.manager@test.local",
        "Reklam Yöneticisi",
        "ads_manager",
    );
    let (st_ads_page, _, ads_html) = send_raw(
        &env.app,
        &env.state,
        "GET",
        "/podmin/editor",
        Some(&ads_cookie),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(st_ads_page, StatusCode::FORBIDDEN);
    let ads_html_str = String::from_utf8_lossy(&ads_html);
    assert!(ads_html_str.contains("403"));
    assert!(ads_html_str.contains("contents:write"));

    let (st_ads_api, _) = send_json(
        &env.app,
        &env.state,
        "GET",
        "/api/v1/admin/editorial/session",
        Some(&ads_cookie),
        None,
    )
    .await;
    assert_eq!(st_ads_api, StatusCode::FORBIDDEN);

    // 4. content_editor gets 200 SSR shell + 200 session with canPublish=false, canDelete=false
    let ce_cookie = create_staff_cookie(
        &env.state,
        "content.editor@test.local",
        "İçerik Editörü",
        "content_editor",
    );
    let (st_ce_page, headers_ce, ce_html) = send_raw(
        &env.app,
        &env.state,
        "GET",
        "/podmin/editor",
        Some(&ce_cookie),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(st_ce_page, StatusCode::OK);
    let csp = headers_ce
        .get("content-security-policy")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        csp.contains("wasm-unsafe-eval"),
        "expected wasm-unsafe-eval in CSP header, got: {csp}"
    );
    let ce_html_str = String::from_utf8_lossy(&ce_html);
    assert!(ce_html_str.contains("/podmin/editor/studio.js"));
    assert!(ce_html_str.contains("/podmin/editor/studio.wasm"));
    assert!(ce_html_str.contains("/css/admin/editorial-studio.css"));
    assert!(ce_html_str.contains("/css/admin/tokens.css"));

    // 4b. Embedded /podmin/editor/studio.{js,wasm} enforce auth + RBAC + MIME types
    for asset_path in ["/podmin/editor/studio.js", "/podmin/editor/studio.wasm"] {
        let (st_anon_asset, _, _) =
            send_raw(&env.app, &env.state, "GET", asset_path, None, None, Body::empty()).await;
        assert!(
            matches!(
                st_anon_asset,
                StatusCode::SEE_OTHER | StatusCode::TEMPORARY_REDIRECT
            ),
            "unauthenticated {asset_path} expected redirect, got {st_anon_asset}"
        );

        let (st_ads_asset, _, _) = send_raw(
            &env.app,
            &env.state,
            "GET",
            asset_path,
            Some(&ads_cookie),
            None,
            Body::empty(),
        )
        .await;
        assert_eq!(st_ads_asset, StatusCode::FORBIDDEN);
    }

    let (st_js, headers_js, body_js) = send_raw(
        &env.app,
        &env.state,
        "GET",
        "/podmin/editor/studio.js",
        Some(&ce_cookie),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(st_js, StatusCode::OK);
    let ct_js = headers_js
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct_js.contains("javascript"), "expected JS MIME, got {ct_js}");
    assert!(!body_js.is_empty(), "embedded studio.js must be non-empty");

    let (st_wasm, headers_wasm, body_wasm) = send_raw(
        &env.app,
        &env.state,
        "GET",
        "/podmin/editor/studio.wasm",
        Some(&ce_cookie),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(st_wasm, StatusCode::OK);
    let ct_wasm = headers_wasm
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct_wasm.contains("application/wasm"),
        "expected application/wasm MIME, got {ct_wasm}"
    );
    assert!(
        body_wasm.starts_with(b"\0asm"),
        "embedded studio.wasm must start with WebAssembly magic header"
    );

    // 4c. Studio CSS assets referenced by the SSR shell return 200 OK
    for css_path in ["/css/admin/tokens.css", "/css/admin/editorial-studio.css"] {
        let (st_css, _, css_bytes) =
            send_raw(&env.app, &env.state, "GET", css_path, None, None, Body::empty()).await;
        assert_eq!(st_css, StatusCode::OK, "expected 200 on {css_path}");
        assert!(!css_bytes.is_empty(), "{css_path} must be non-empty");
    }

    let (st_ce_sess, ce_sess_json) = send_json(
        &env.app,
        &env.state,
        "GET",
        "/api/v1/admin/editorial/session",
        Some(&ce_cookie),
        None,
    )
    .await;
    assert_eq!(st_ce_sess, StatusCode::OK);
    assert_eq!(ce_sess_json["authenticated"], json!(true));
    assert_eq!(ce_sess_json["user"]["capabilities"]["canWrite"], json!(true));
    assert_eq!(ce_sess_json["user"]["capabilities"]["canPublish"], json!(false));
    assert_eq!(ce_sess_json["user"]["capabilities"]["canDelete"], json!(false));

    // 5. editor_in_chief uses `:manage` grants from rbac.rs and must resolve full editorial capabilities
    let chief_cookie = create_staff_cookie(
        &env.state,
        "chief.caps@test.local",
        "Başeditör Yetki Testi",
        "editor_in_chief",
    );
    let (st_chief_sess, chief_sess_json) = send_json(
        &env.app,
        &env.state,
        "GET",
        "/api/v1/admin/editorial/session",
        Some(&chief_cookie),
        None,
    )
    .await;
    assert_eq!(st_chief_sess, StatusCode::OK);
    assert_eq!(chief_sess_json["user"]["capabilities"]["canWrite"], json!(true));
    assert_eq!(chief_sess_json["user"]["capabilities"]["canPublish"], json!(true));
    assert_eq!(chief_sess_json["user"]["capabilities"]["canDelete"], json!(true));
    assert_eq!(chief_sess_json["user"]["capabilities"]["canUploadMedia"], json!(true));
    assert_eq!(chief_sess_json["user"]["capabilities"]["canManageCategories"], json!(true));
    assert_eq!(chief_sess_json["user"]["capabilities"]["canManageIssues"], json!(true));
}

#[tokio::test]
async fn content_editor_approval_and_chief_publish_revisions_locks_and_redirects() {
    let env = setup_env("workflow_revs_locks");
    let ce_cookie = create_staff_cookie(
        &env.state,
        "editor.writer@test.local",
        "Yazar Editör",
        "content_editor",
    );
    let chief_cookie = create_staff_cookie(
        &env.state,
        "chief.editor@test.local",
        "Genel Yayın Yönetmeni",
        "editor_in_chief",
    );

    // 1. content_editor creates a draft
    let (st_draft, draft_res) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/articles",
        Some(&ce_cookie),
        Some(json!({
            "id": "0",
            "title": "Milli Muharip Uçak Test Uçuşu",
            "spot": "İlk aşama yer testleri başarıyla tamamlandı.",
            "content": "<p>İlk revizyon paragrafı.</p>",
            "status": "draft",
            "slug": "milli-muharip-ucak-test-ucusu",
            "contentType": "haberler"
        })),
    )
    .await;
    assert_eq!(st_draft, StatusCode::OK);
    let article_id = draft_res["id"].as_str().expect("article id").to_string();
    assert_eq!(draft_res["article"]["status"], json!("draft"));

    // 2. content_editor tries direct publish, future scheduling, and delete -> all 403
    let (st_pub_denied, _) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/articles",
        Some(&ce_cookie),
        Some(json!({
            "id": article_id,
            "title": "Milli Muharip Uçak Test Uçuşu",
            "content": "<p>İzinsiz yayım denemesi.</p>",
            "status": "publish",
            "slug": "milli-muharip-ucak-test-ucusu"
        })),
    )
    .await;
    assert_eq!(st_pub_denied, StatusCode::FORBIDDEN);

    let (st_fut_denied, _) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/articles",
        Some(&ce_cookie),
        Some(json!({
            "id": article_id,
            "title": "Milli Muharip Uçak Test Uçuşu",
            "content": "<p>İzinsiz planlama denemesi.</p>",
            "status": "future",
            "scheduledAt": "2030-01-01T10:00:00Z",
            "slug": "milli-muharip-ucak-test-ucusu"
        })),
    )
    .await;
    assert_eq!(st_fut_denied, StatusCode::FORBIDDEN);

    let (st_del_denied, _) = send_json(
        &env.app,
        &env.state,
        "DELETE",
        &format!("/api/v1/admin/editorial/articles/{article_id}"),
        Some(&ce_cookie),
        None,
    )
    .await;
    assert_eq!(st_del_denied, StatusCode::FORBIDDEN);

    // 3. content_editor submits pending_review -> succeeds + creates approval_requests row
    let (st_rev, rev_res) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/articles",
        Some(&ce_cookie),
        Some(json!({
            "id": article_id,
            "title": "Milli Muharip Uçak Test Uçuşu Güncellendi",
            "content": "<p>İkinci revizyon paragrafı.</p>",
            "status": "pending_review",
            "slug": "milli-muharip-ucak-test-ucusu",
            "note": "Onaya hazır"
        })),
    )
    .await;
    assert_eq!(st_rev, StatusCode::OK);
    assert_eq!(rev_res["article"]["status"], json!("pending_review"));
    {
        let conn = env.state.db.open_conn().unwrap();
        let pending_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM approval_requests WHERE content_id = ?1 AND status = 'pending'",
                [article_id.parse::<i64>().unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending_count, 1);
    }

    // 4. editor_in_chief publishes, then renames slug -> creates 301 redirect in seo_redirects
    let (st_pub, pub_res) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/articles",
        Some(&chief_cookie),
        Some(json!({
            "id": article_id,
            "title": "Milli Muharip Uçak Yayında",
            "content": "<p>Yayındaki içerik.</p>",
            "status": "publish",
            "slug": "milli-muharip-ucak-test-ucusu"
        })),
    )
    .await;
    assert_eq!(st_pub, StatusCode::OK);
    assert_eq!(pub_res["article"]["status"], json!("publish"));

    let (st_rename, rename_res) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/articles",
        Some(&chief_cookie),
        Some(json!({
            "id": article_id,
            "title": "Milli Muharip Uçak KAAN Yayında",
            "content": "<p>Güncel KAAN haberi.</p>",
            "status": "publish",
            "slug": "milli-muharip-ucak-kaan-yayinda"
        })),
    )
    .await;
    assert_eq!(st_rename, StatusCode::OK);
    assert_eq!(
        rename_res["redirectCreated"]["oldSlug"],
        json!("milli-muharip-ucak-test-ucusu")
    );
    assert_eq!(
        rename_res["redirectCreated"]["newSlug"],
        json!("milli-muharip-ucak-kaan-yayinda")
    );

    let (st_rd_list, rd_list_json) = send_json(
        &env.app,
        &env.state,
        "GET",
        "/api/v1/admin/editorial/redirects",
        Some(&chief_cookie),
        None,
    )
    .await;
    assert_eq!(st_rd_list, StatusCode::OK);
    let redirects = rd_list_json["redirects"].as_array().unwrap();
    assert!(!redirects.is_empty());

    // 5. Revisions list & restore
    let (st_revs, revs_json) = send_json(
        &env.app,
        &env.state,
        "GET",
        &format!("/api/v1/admin/editorial/articles/{article_id}/revisions"),
        Some(&chief_cookie),
        None,
    )
    .await;
    assert_eq!(st_revs, StatusCode::OK);
    let rev_items = revs_json["revisions"].as_array().expect("revisions array");
    assert!(rev_items.len() >= 2, "expected >=2 revisions, got {}", rev_items.len());
    let oldest_rev = rev_items
        .iter()
        .find(|r| r["title"] == json!("Milli Muharip Uçak Test Uçuşu"))
        .expect("find initial revision");
    let rev_id = oldest_rev["id"].as_i64().unwrap();

    let (st_restore, restore_json) = send_json(
        &env.app,
        &env.state,
        "POST",
        &format!("/api/v1/admin/editorial/articles/{article_id}/revisions/{rev_id}/restore"),
        Some(&chief_cookie),
        Some(json!({})),
    )
    .await;
    assert_eq!(st_restore, StatusCode::OK);
    assert_eq!(
        restore_json["article"]["title"],
        json!("Milli Muharip Uçak Test Uçuşu")
    );

    // 6. Lock acquire, conflict from second editor, list locks, and release
    let (st_lock1, lock1_json) = send_json(
        &env.app,
        &env.state,
        "POST",
        &format!("/api/v1/admin/editorial/articles/{article_id}/lock"),
        Some(&chief_cookie),
        Some(json!({})),
    )
    .await;
    assert_eq!(st_lock1, StatusCode::OK);
    assert_eq!(lock1_json["conflict"], json!(false));

    let (st_lock2, lock2_json) = send_json(
        &env.app,
        &env.state,
        "POST",
        &format!("/api/v1/admin/editorial/articles/{article_id}/lock"),
        Some(&ce_cookie),
        Some(json!({})),
    )
    .await;
    assert_eq!(st_lock2, StatusCode::OK);
    assert_eq!(lock2_json["conflict"], json!(true));
    assert_eq!(
        lock2_json["lockedBy"]["username"],
        json!("Genel Yayın Yönetmeni")
    );

    let (st_locks, locks_json) = send_json(
        &env.app,
        &env.state,
        "GET",
        "/api/v1/admin/editorial/locks",
        Some(&ce_cookie),
        None,
    )
    .await;
    assert_eq!(st_locks, StatusCode::OK);
    assert!(locks_json["locks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["articleId"] == json!(article_id)));

    let (st_unlock, _) = send_json(
        &env.app,
        &env.state,
        "DELETE",
        &format!("/api/v1/admin/editorial/articles/{article_id}/lock"),
        Some(&chief_cookie),
        None,
    )
    .await;
    assert_eq!(st_unlock, StatusCode::OK);

    let (st_lock3, lock3_json) = send_json(
        &env.app,
        &env.state,
        "POST",
        &format!("/api/v1/admin/editorial/articles/{article_id}/lock"),
        Some(&ce_cookie),
        Some(json!({})),
    )
    .await;
    assert_eq!(st_lock3, StatusCode::OK);
    assert_eq!(lock3_json["conflict"], json!(false));
}

#[tokio::test]
async fn media_upload_delete_research_sources_and_due_scheduler_work() {
    let env = setup_env("media_research_sched");
    let admin_cookie = create_staff_cookie(
        &env.state,
        "admin.studio@test.local",
        "Stüdyo Yöneticisi",
        "admin",
    );

    // 1. Upload valid PNG via multipart /api/v1/admin/editorial/upload -> converts to WebP + thumb
    let png_bytes = build_test_png_bytes(120, 68);
    let boundary = "----MsiEditorialTestBoundary77";
    let mut mp_body = Vec::new();
    mp_body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"alt\"\r\n\r\nSavunma test görseli\r\n"
        )
        .as_bytes(),
    );
    mp_body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"kapak-test.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    mp_body.extend_from_slice(&png_bytes);
    mp_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let (st_up, _, up_raw) = send_raw(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/upload",
        Some(&admin_cookie),
        Some(&format!("multipart/form-data; boundary={boundary}")),
        Body::from(mp_body),
    )
    .await;
    assert_eq!(st_up, StatusCode::OK);
    let up_json: Value = serde_json::from_slice(&up_raw).unwrap();
    let media_id = up_json["asset"]["id"].as_i64().expect("uploaded asset id");
    let media_url = up_json["url"].as_str().expect("uploaded webp url");
    let filename = media_url.strip_prefix("/uploads/editorial/").unwrap();
    let up_dir = PathBuf::from(editorial_media_service::resolve_editorial_upload_dir(&env.state.config));
    assert!(up_dir.join(filename).exists(), "uploaded WebP file must exist");

    // 2. Delete media asset -> removes DB row and files on disk
    let (st_del_media, _) = send_json(
        &env.app,
        &env.state,
        "DELETE",
        &format!("/api/v1/admin/editorial/media/{media_id}"),
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st_del_media, StatusCode::OK);
    assert!(!up_dir.join(filename).exists(), "deleted WebP file must be removed");

    // 3. Research source CRUD (uses .invalid RFC 2606 domain so offline probe falls back cleanly)
    let (st_src_create, src_create_json) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/research/sources",
        Some(&admin_cookie),
        Some(json!({
            "name": "Test Savunma Bülteni",
            "url": "https://savunma-test.invalid/rss.xml",
            "category": "Savunma",
            "scope": "tr"
        })),
    )
    .await;
    assert_eq!(st_src_create, StatusCode::OK);
    let src_id = src_create_json["source"]["id"].as_i64().expect("source id");

    let (st_src_upd, src_upd_json) = send_json(
        &env.app,
        &env.state,
        "PUT",
        &format!("/api/v1/admin/editorial/research/sources/{src_id}"),
        Some(&admin_cookie),
        Some(json!({
            "name": "Güncel Test Savunma Bülteni",
            "enabled": true
        })),
    )
    .await;
    assert_eq!(st_src_upd, StatusCode::OK);
    assert_eq!(
        src_upd_json["source"]["name"],
        json!("Güncel Test Savunma Bülteni")
    );

    let (st_src_del, _) = send_json(
        &env.app,
        &env.state,
        "DELETE",
        &format!("/api/v1/admin/editorial/research/sources/{src_id}"),
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st_src_del, StatusCode::OK);

    // 4. Scheduled publishing promotes past-due `future` item to `published`
    let (st_sched, sched_json) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/articles",
        Some(&admin_cookie),
        Some(json!({
            "id": "0",
            "title": "Zamanlanmış Savunma Analizi",
            "content": "<p>Otomatik yayımlanacak içerik.</p>",
            "status": "future",
            "scheduledAt": "2020-01-01 00:00:00",
            "slug": "zamanlanmis-savunma-analizi"
        })),
    )
    .await;
    assert_eq!(st_sched, StatusCode::OK);
    let sched_id = sched_json["id"].as_str().unwrap().parse::<i64>().unwrap();
    assert_eq!(sched_json["article"]["status"], json!("future"));

    let promoted = env
        .state
        .db
        .editorial_publish_due_scheduled()
        .expect("run due scheduler");
    assert!(
        promoted.iter().any(|item| item.id == sched_id),
        "scheduled article {sched_id} must be promoted"
    );

    let promoted_art = env
        .state
        .db
        .editorial_get_article_by_id(sched_id)
        .unwrap()
        .unwrap();
    assert_eq!(promoted_art.status, "publish");
}

#[tokio::test]
async fn editorial_composite_issue_pk_upsert_and_scoped_delete() {
    let env = setup_env("composite_issue_pk");
    let admin_cookie = create_staff_cookie(
        &env.state,
        "issues.admin@test.local",
        "Sayı Yöneticisi",
        "admin",
    );

    // 1. Upsert both TR (msi) and EN (msi_tdr) sharing the exact same issue_number = 299
    let (st_tr, _) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/issues",
        Some(&admin_cookie),
        Some(json!({
            "id": 299,
            "issueNumber": 299,
            "editionLang": "tr",
            "titleTr": "MSI Dergisi Sayı 299",
            "titleEn": "MSI Magazine Issue 299",
            "publishMonth": "Aralık",
            "year": 2026,
            "isSpecialIssue": false,
            "eventName": null
        })),
    )
    .await;
    assert_eq!(st_tr, StatusCode::OK);

    let (st_en, _) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/issues",
        Some(&admin_cookie),
        Some(json!({
            "id": 299,
            "issueNumber": 299,
            "editionLang": "en",
            "titleTr": "MSI TDR Sayı 299",
            "titleEn": "MSI TDR Issue 299",
            "publishMonth": "December",
            "year": 2026,
            "isSpecialIssue": false,
            "eventName": null
        })),
    )
    .await;
    assert_eq!(st_en, StatusCode::OK);

    // Both composite rows exist independently
    let msi_299 = env.state.db.get_issue("msi", 299).expect("msi 299 exists");
    let tdr_299 = env
        .state
        .db
        .get_issue("msi_tdr", 299)
        .expect("msi_tdr 299 exists");
    assert_eq!(msi_299.title_tr, "MSI Dergisi Sayı 299");
    assert_eq!(tdr_299.title_en, "MSI TDR Issue 299");
    assert_eq!(tdr_299.en_status, "approved");

    // Re-upserting msi 299 preserves en_status/en_approved_by rather than resetting to "none"
    {
        let mut updated = msi_299.clone();
        updated.en_status = "approved".to_string();
        updated.en_approved_by = Some("chief@test.local".to_string());
        env.state.db.upsert_issue(&updated).unwrap();
    }
    let (st_tr2, _) = send_json(
        &env.app,
        &env.state,
        "POST",
        "/api/v1/admin/editorial/issues",
        Some(&admin_cookie),
        Some(json!({
            "id": 299,
            "issueNumber": 299,
            "editionLang": "tr",
            "titleTr": "MSI Dergisi Sayı 299 Güncel",
            "titleEn": "MSI Magazine Issue 299",
            "publishMonth": "Aralık",
            "year": 2026,
            "isSpecialIssue": false,
            "eventName": null
        })),
    )
    .await;
    assert_eq!(st_tr2, StatusCode::OK);
    let msi_299_after = env.state.db.get_issue("msi", 299).unwrap();
    assert_eq!(msi_299_after.en_status, "approved");
    assert_eq!(
        msi_299_after.en_approved_by.as_deref(),
        Some("chief@test.local")
    );

    // 2. Ambiguous DELETE without publication_type refuses to wipe both (409 Conflict)
    let (st_amb, _) = send_json(
        &env.app,
        &env.state,
        "DELETE",
        "/api/v1/admin/editorial/issues/299",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st_amb, StatusCode::CONFLICT);
    assert!(env.state.db.get_issue("msi", 299).is_some());
    assert!(env.state.db.get_issue("msi_tdr", 299).is_some());

    // 3. Scoped DELETE with ?publication_type=msi_tdr removes ONLY msi_tdr 299
    let (st_del_tdr, _) = send_json(
        &env.app,
        &env.state,
        "DELETE",
        "/api/v1/admin/editorial/issues/299?publication_type=msi_tdr",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st_del_tdr, StatusCode::OK);
    assert!(env.state.db.get_issue("msi_tdr", 299).is_none());
    assert!(env.state.db.get_issue("msi", 299).is_some());
}

#[tokio::test]
async fn list_articles_normalizes_cover_image_and_orders_by_published_at_desc() {
    let env = setup_env("cover-order");
    let admin_cookie = create_staff_cookie(
        &env.state,
        "admin.list@test.local",
        "List Admin",
        "admin",
    );

    let conn = env.state.db.open_conn().expect("open test db");
    conn.execute(
        "INSERT INTO contents (id, slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en, content_type, source_group, cover_image, author_name, author_title, status, published_at, updated_at)
         VALUES (?1, ?2, ?3, '', '', '', '<p>eski</p>', '', 'news', 'site', ?4, 'Editör', '', 'published', ?5, ?5)",
        rusqlite::params![
            9001_i64,
            "older-pub-high-id",
            "Eski Tarih Yüksek ID",
            "images/msi/old-cover.webp",
            "2098-01-10 10:00:00"
        ],
    )
    .expect("insert older high-id row");
    conn.execute(
        "INSERT INTO contents (id, slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en, content_type, source_group, cover_image, author_name, author_title, status, published_at, updated_at)
         VALUES (?1, ?2, ?3, '', '', '', '<p>yeni</p>', '', 'news', 'site', ?4, 'Editör', '', 'published', ?5, ?5)",
        rusqlite::params![
            9000_i64,
            "newer-pub-low-id",
            "Yeni Tarih Düşük ID",
            "images/msi/test-cover.webp",
            "2099-06-15 12:00:00"
        ],
    )
    .expect("insert newer low-id row");
    drop(conn);

    let (st, json_res) = send_json(
        &env.app,
        &env.state,
        "GET",
        "/api/v1/admin/editorial/articles?type=published&limit=100",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let items = json_res["articles"].as_array().expect("articles array");
    let idx_newer = items
        .iter()
        .position(|a| a["id"] == json!("9000"))
        .expect("newer article 9000 present");
    let idx_older = items
        .iter()
        .position(|a| a["id"] == json!("9001"))
        .expect("older article 9001 present");
    assert!(
        idx_newer < idx_older,
        "article 9000 (published 2099-06-15) must precede article 9001 (published 2098-01-10): idx_newer={idx_newer}, idx_older={idx_older}"
    );
    assert_eq!(
        items[idx_newer]["imageUrl"],
        json!("/images/msi/test-cover.webp")
    );
    assert_eq!(
        items[idx_older]["imageUrl"],
        json!("/images/msi/old-cover.webp")
    );
}
