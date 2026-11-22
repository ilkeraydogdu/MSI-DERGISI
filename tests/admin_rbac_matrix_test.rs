//! Integration tests for Admin RBAC matrix, privilege-escalation guards, and new CRUD APIs.

mod common;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

use common::*;
use msi_dergisi_platform::{routes, services::auth_service};

async fn send_json(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
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
    let bytes = to_bytes(resp.into_body(), 2 * 1024 * 1024).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, val)
}

async fn get_html(app: &axum::Router, uri: &str, cookie: Option<&str>) -> (StatusCode, String) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    let resp = app
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

fn make_user_and_cookie(
    state: &msi_dergisi_platform::app_state::AppState,
    email: &str,
    role: &str,
) -> (i64, String) {
    let hash = auth_service::hash_password("TestPass123!", &state.config.session_secret);
    let uid = state
        .db
        .admin_create_user(
            email,
            &hash,
            role,
            role,
            "Sektör Kuruluşları",
            "MSI QA",
            "Tester",
            false,
            "",
        )
        .expect("create user");
    let user = state
        .db
        .get_all_users()
        .into_iter()
        .find(|u| u.id == uid)
        .expect("find created user");
    let token = auth_service::generate_session_token(&user, &state.config.session_secret);
    (uid, format!("msi_session={}", token))
}

#[tokio::test]
async fn rbac_matrix_enforces_roles_and_prevents_escalation() {
    let (state, db_path) = build_test_state("rbac_matrix");
    let app = routes::build_router(state.clone());
    let html_app = app.clone().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
    ));

    // 1. Unauthenticated requests rejected with 401
    let (st, _) = send_json(&app, "GET", "/api/v1/admin/me", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // 2. Reader (`registered`) and all 4 real subscriber roles rejected with 403 and excluded from staff
    for non_staff_role in [
        "registered",
        "subscriber_public",
        "subscriber_corporate",
        "subscriber_print_digital",
        "subscriber_digital",
    ] {
        assert!(
            !msi_dergisi_platform::extractors::operator_user::is_staff_role(&state.db, non_staff_role),
            "Role '{}' must not be treated as staff",
            non_staff_role
        );
        let email = format!("{}_qa@test.local", non_staff_role);
        let (_, cookie) = make_user_and_cookie(&state, &email, non_staff_role);
        let (st, _) = send_json(&app, "GET", "/api/v1/admin/me", Some(&cookie), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "GET /admin/me must be 403 for {}", non_staff_role);
        let (st, _) = send_json(&app, "GET", "/api/v1/admin/issues", Some(&cookie), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "GET /admin/issues must be 403 for {}", non_staff_role);

        // /hesabim must never render the operator command card for non-staff roles
        let (st, acct_html) = get_html(&html_app, "/hesabim", Some(&cookie)).await;
        assert_eq!(st, StatusCode::OK);
        assert!(
            !acct_html.contains("account-command-card"),
            "/hesabim leaked account-command-card to {}",
            non_staff_role
        );
    }

    // 3. `support_agent` can read users & decide corporate requests, CANNOT manage users or roles
    let (_, support_cookie) = make_user_and_cookie(&state, "support_qa@test.local", "support_agent");
    let (st, _) = send_json(&app, "GET", "/api/v1/admin/users", Some(&support_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/users",
        Some(&support_cookie),
        Some(json!({
            "email": "blocked_create@test.local",
            "password": "Password123!",
            "full_name": "Blocked",
            "role": "content_editor"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/user-roles",
        Some(&support_cookie),
        Some(json!({
            "role_code": "evil_role",
            "name_tr": "Evil",
            "name_en": "Evil",
            "permissions_json": "[\"*\"]"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // 4. `ads_manager` can access ads, FORBIDDEN on contents:delete and roles:manage
    let (_, ads_cookie) = make_user_and_cookie(&state, "ads_qa@test.local", "ads_manager");
    let (st, _) = send_json(&app, "GET", "/api/v1/admin/ads", Some(&ads_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = send_json(&app, "DELETE", "/api/v1/admin/contents/1", Some(&ads_cookie), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // 5. `super_admin` has wildcard (`*`), while `admin` has explicit `:manage` + `roles:manage` without `*`
    let (_, super_cookie) = make_user_and_cookie(&state, "super_qa@test.local", "super_admin");
    let (st, super_me) = send_json(&app, "GET", "/api/v1/admin/me", Some(&super_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(super_me["data"]["is_wildcard"], json!(true));

    let (_, admin_cookie) = make_user_and_cookie(&state, "admin_qa@test.local", "admin");
    let (st, me_body) = send_json(&app, "GET", "/api/v1/admin/me", Some(&admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(me_body["data"]["is_wildcard"], json!(false));
    let admin_perms = me_body["data"]["permissions"].as_array().expect("permissions array");
    assert!(admin_perms.contains(&json!("roles:manage")));
    assert!(admin_perms.contains(&json!("backup:manage")));

    let (_, site_op_cookie) = make_user_and_cookie(&state, "siteop_qa@test.local", "site_operator");
    let (st, op_me) = send_json(&app, "GET", "/api/v1/admin/me", Some(&site_op_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(op_me["data"]["is_wildcard"], json!(false));
    let op_perms = op_me["data"]["permissions"].as_array().expect("permissions array");
    assert!(!op_perms.contains(&json!("roles:manage")));
    assert!(!op_perms.contains(&json!("users:manage")));
    assert!(!op_perms.contains(&json!("backup:manage")));

    // 6. Custom role with `roles:manage` + `users:manage` (WITHOUT `*`) CANNOT grant `*` or promote to `super_admin`
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/user-roles",
        Some(&admin_cookie),
        Some(json!({
            "role_code": "delegated_hr",
            "name_tr": "İK Yöneticisi",
            "name_en": "HR Manager",
            "permissions_json": "[\"users:read\",\"users:manage\",\"roles:manage\"]"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    let (target_uid, _) = make_user_and_cookie(&state, "target_qa@test.local", "content_editor");
    let (_, hr_cookie) = make_user_and_cookie(&state, "hr_qa@test.local", "delegated_hr");

    // Non-wildcard cannot create a role with `*` or resource wildcard `users:*`
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/user-roles",
        Some(&hr_cookie),
        Some(json!({
            "role_code": "escalated",
            "name_tr": "Esc",
            "name_en": "Esc",
            "permissions_json": "[\"*\"]"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/user-roles",
        Some(&hr_cookie),
        Some(json!({
            "role_code": "escalated_res",
            "name_tr": "EscRes",
            "name_en": "EscRes",
            "permissions_json": "[\"users:*\"]"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // `support_agent` gets 403 on write-gated audit-logs & halt-renewed endpoints
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/audit-logs",
        Some(&support_cookie),
        Some(json!({ "action": "test", "entity_type": "user" })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/reminders/halt-renewed",
        Some(&support_cookie),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Non-wildcard cannot assign `super_admin` or `admin`
    let (st, _) = send_json(
        &app,
        "POST",
        &format!("/api/v1/admin/users/{}/assign-role", target_uid),
        Some(&hr_cookie),
        Some(json!({ "role_code": "super_admin" })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Non-wildcard CAN assign non-privileged role
    let (st, _) = send_json(
        &app,
        "POST",
        &format!("/api/v1/admin/users/{}/assign-role", target_uid),
        Some(&hr_cookie),
        Some(json!({ "role_code": "ads_manager" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Admin issues/commerce/invoices endpoints respond 200
    for ep in [
        "/api/v1/admin/issues",
        "/api/v1/admin/subscription-plans",
        "/api/v1/admin/commerce-products",
        "/api/v1/admin/invoices",
    ] {
        let (st, _) = send_json(&app, "GET", ep, Some(&admin_cookie), None).await;
        assert_eq!(st, StatusCode::OK, "Expected 200 on {}", ep);
    }

    // Staff user DOES see account-command-card on /hesabim
    let (st, admin_acct_html) = get_html(&html_app, "/hesabim", Some(&admin_cookie)).await;
    assert_eq!(st, StatusCode::OK);
    assert!(admin_acct_html.contains("account-command-card"));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn all_12_staff_roles_and_new_admin_crud_apis_work_with_rbac() {
    let (state, db_path) = build_test_state("rbac_17_roles_crud");
    let app = routes::build_router(state.clone());

    // 1. All 12 staff roles authenticate on /api/v1/admin/me and have is_staff_role == true
    let mut role_cookies = std::collections::HashMap::new();
    for role in [
        "super_admin",
        "admin",
        "site_operator",
        "editor_in_chief",
        "managing_editor",
        "content_publisher",
        "content_editor",
        "editor",
        "business_dev",
        "ads_manager",
        "finance_viewer",
        "support_agent",
    ] {
        assert!(
            msi_dergisi_platform::extractors::operator_user::is_staff_role(&state.db, role),
            "Staff role '{}' should satisfy is_staff_role",
            role
        );
        let email = format!("{}_all@test.local", role);
        let (_, cookie) = make_user_and_cookie(&state, &email, role);
        let (st, me) = send_json(&app, "GET", "/api/v1/admin/me", Some(&cookie), None).await;
        assert_eq!(st, StatusCode::OK, "Expected 200 on /admin/me for {}", role);
        assert_eq!(me["data"]["role"], json!(role));
        role_cookies.insert(role, cookie);
    }

    let admin_cookie = &role_cookies["admin"];
    let site_op_cookie = &role_cookies["site_operator"];
    let eic_cookie = &role_cookies["editor_in_chief"];
    let mg_cookie = &role_cookies["managing_editor"];
    let pub_cookie = &role_cookies["content_publisher"];
    let ce_cookie = &role_cookies["content_editor"];
    let biz_cookie = &role_cookies["business_dev"];
    let fin_cookie = &role_cookies["finance_viewer"];

    // 2. Role-specific positive/negative RBAC checks across all staff roles
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/glossary", Some(eic_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/locales", Some(eic_cookie), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(send_json(&app, "POST", "/api/v1/admin/backup", Some(eic_cookie), None).await.0, StatusCode::FORBIDDEN);

    // managing_editor has glossary:write and email_groups:write (list/upsert 200, delete requires *:manage -> 403)
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/glossary", Some(mg_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "DELETE", "/api/v1/admin/glossary/999999", Some(mg_cookie), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/email-groups", Some(mg_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "DELETE", "/api/v1/admin/email-groups/nonexistent", Some(mg_cookie), None).await.0, StatusCode::FORBIDDEN);

    assert_eq!(send_json(&app, "GET", "/api/v1/admin/glossary", Some(site_op_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/email-groups", Some(site_op_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/locales", Some(site_op_cookie), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(send_json(&app, "POST", "/api/v1/admin/backup", Some(site_op_cookie), None).await.0, StatusCode::FORBIDDEN);

    assert_eq!(send_json(&app, "GET", "/api/v1/admin/invoices", Some(fin_cookie), None).await.0, StatusCode::OK);
    let sample_content_json = serde_json::to_value(fixture_content(99)).unwrap();
    assert_eq!(
        send_json(&app, "POST", "/api/v1/admin/contents", Some(fin_cookie), Some(sample_content_json)).await.0,
        StatusCode::FORBIDDEN
    );

    assert_eq!(send_json(&app, "DELETE", "/api/v1/admin/contents/1", Some(ce_cookie), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/redirects", Some(pub_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/ads", Some(pub_cookie), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/commerce-products", Some(biz_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/users", Some(biz_cookie), None).await.0, StatusCode::FORBIDDEN);

    // 3. POST /api/v1/admin/users with omitted role defaults to "registered"
    let (st, created_user) = send_json(
        &app,
        "POST",
        "/api/v1/admin/users",
        Some(admin_cookie),
        Some(json!({
            "email": "default_role_check@test.local",
            "password": "Password123!",
            "full_name": "Default Role User"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let created_id = created_user["data"]["id"].as_i64().unwrap();
    let persisted = state
        .db
        .get_all_users()
        .into_iter()
        .find(|u| u.id == created_id)
        .expect("created user exists");
    assert_eq!(persisted.role, "registered");

    // 4. Glossary CRUD
    let (st, gl_save) = send_json(
        &app,
        "POST",
        "/api/v1/admin/glossary",
        Some(admin_cookie),
        Some(json!({
            "term_tr": "İnsansız Hava Aracı",
            "term_en": "Unmanned Aerial Vehicle",
            "preferred_en": "Unmanned Aerial Vehicle (UAV)",
            "domain": "aerospace",
            "rule_type": "fixed",
            "forbidden_variants": ["drone"]
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let gl_id = gl_save["data"]["id"].as_i64().unwrap();
    let (st, gl_list) = send_json(&app, "GET", "/api/v1/admin/glossary", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(gl_list["data"].as_array().unwrap().iter().any(|x| x["id"] == gl_id));
    let (st, _) = send_json(&app, "DELETE", &format!("/api/v1/admin/glossary/{}", gl_id), Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);

    // 5. Locales CRUD
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/locales",
        Some(admin_cookie),
        Some(json!({
            "code": "de",
            "name_native": "Deutsch",
            "name_en": "German",
            "hreflang": "de-DE",
            "og_locale": "de_DE",
            "dir": "ltr",
            "is_enabled": true
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, loc_list) = send_json(&app, "GET", "/api/v1/admin/locales", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(loc_list["data"].as_array().unwrap().iter().any(|x| x["code"] == "de"));
    let (st, _) = send_json(
        &app,
        "PUT",
        "/api/v1/admin/locales/de",
        Some(admin_cookie),
        Some(json!({
            "code": "de",
            "name_native": "Deutsch (AT)",
            "name_en": "German",
            "hreflang": "de-AT",
            "og_locale": "de_AT",
            "dir": "ltr",
            "is_enabled": true
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = send_json(&app, "DELETE", "/api/v1/admin/locales/de", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);

    // 6. Email Groups CRUD
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/email-groups",
        Some(admin_cookie),
        Some(json!({
            "code": "qa_group",
            "name_tr": "QA Grubu",
            "name_en": "QA Group",
            "filter_json": "{\"roles\":[\"registered\"]}",
            "static_emails_json": "[\"qa@msidergisi.com\"]"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, eg_list) = send_json(&app, "GET", "/api/v1/admin/email-groups", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(eg_list["data"].as_array().unwrap().iter().any(|x| x["code"] == "qa_group"));
    let (st, _) = send_json(&app, "DELETE", "/api/v1/admin/email-groups/qa_group", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);

    // 7. Backup endpoint (admin allowed)
    let (st, bk_resp) = send_json(&app, "POST", "/api/v1/admin/backup", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    if let Some(bk_path) = bk_resp["data"]["path"].as_str() {
        let _ = std::fs::remove_file(bk_path);
        let _ = std::fs::remove_file(format!("{}.json", bk_path));
    }

    // 8. Translations pending + approve + reject + auto-translate (unconfigured returns 400 cleanly)
    let entry = msi_dergisi_platform::engine::translation_engine::TranslationEntry {
        key: "qa.pending.sample".into(),
        value_tr: "Test Çeviri".into(),
        value_en: "Test Translation".into(),
        value_ar: String::new(),
        value_de: String::new(),
        value_ru: String::new(),
        value_fr: String::new(),
        group: "qa".into(),
    };
    state
        .db
        .upsert_translation_with_status(&entry, "draft", None, None, Some("tr"))
        .expect("insert draft translation");

    let (st, pend) = send_json(&app, "GET", "/api/v1/admin/translations/pending?status=draft", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(pend["data"].as_array().unwrap().iter().any(|x| x["key"] == "qa.pending.sample"));

    let (st, _) = send_json(&app, "POST", "/api/v1/admin/translations/qa.pending.sample/approve", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = send_json(&app, "POST", "/api/v1/admin/translations/qa.pending.sample/reject", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/translations/auto-translate",
        Some(admin_cookie),
        Some(json!({ "key": "qa.pending.sample", "text": "Savunma Sanayii", "source_lang": "tr", "target_lang": "en" })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // 9. Overview includes exact total_placements, conversion-funnel separates stages, site_settings brand sync
    let (st, ov) = send_json(&app, "GET", "/api/v1/admin/overview", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    let total_contents = ov["total_contents"].as_i64().unwrap();
    let total_placements = ov["total_placements"].as_i64().expect("total_placements integer");
    let expected_placements: i64 = state
        .db
        .get_all_contents()
        .iter()
        .map(|c| c.placements.len() as i64)
        .sum();
    assert_eq!(total_placements, expected_placements);
    assert!(total_placements >= total_contents);

    let (st, funnel) = send_json(&app, "GET", "/api/v1/admin/analytics/conversion-funnel", Some(admin_cookie), None).await;
    assert_eq!(st, StatusCode::OK);
    let f = &funnel["data"];
    assert!(f["verified"].is_number());
    assert!(f["active_subscribers"].is_number());
    assert!(f["paid_invoice_customers"].is_number());
    assert_eq!(f["subscribed"], f["active_subscribers"]);
    assert_eq!(f["paid"], f["paid_invoice_customers"]);

    let settings_map = state.db.get_setting_map("tr");
    assert_eq!(settings_map.get("publisher_logo_url").map(String::as_str), Some("/images/logo.svg"));
    assert_eq!(settings_map.get("primary_accent_color").map(String::as_str), Some("#cb382a"));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m1_r2_podmin_parity_rbac_ui_api_and_i18n_404_regression() {
    let (state, db_path) = build_test_state("m1_r2_regression");
    let app = routes::build_router(state.clone());
    let html_app = app.clone().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
    ));

    let (_, super_cookie) = make_user_and_cookie(&state, "super_r2@test.local", "super_admin");
    let (_, admin_cookie) = make_user_and_cookie(&state, "admin_r2@test.local", "admin");
    let (_, mg_cookie) = make_user_and_cookie(&state, "mg_r2@test.local", "managing_editor");
    let (_, pub_cookie) = make_user_and_cookie(&state, "pub_r2@test.local", "content_publisher");

    // 1. Canonical /podmin gateway (unauthenticated login + ?lang=en + authenticated dashboard) and retired /msi-komuta 404
    let (st_p_tr, pod_tr) = get_html(&html_app, "/podmin?lang=tr", None).await;
    let (st_m_tr, _) = get_html(&html_app, "/msi-komuta?lang=tr", None).await;
    assert_eq!(st_p_tr, StatusCode::OK);
    assert_eq!(st_m_tr, StatusCode::NOT_FOUND);
    assert!(pod_tr.contains("id=\"login-form\""));
    assert!(pod_tr.contains("data-admin-path=\"/podmin\""));
    assert!(pod_tr.contains("MSI Dergisi · Giriş"));

    let (st_p_en, pod_en) = get_html(&html_app, "/podmin?lang=en", None).await;
    let (st_m_en, _) = get_html(&html_app, "/msi-komuta?lang=en", None).await;
    assert_eq!(st_p_en, StatusCode::OK);
    assert_eq!(st_m_en, StatusCode::NOT_FOUND);
    assert!(pod_en.contains("lang=\"en\""));
    assert!(pod_en.contains("MSI Review · Sign In"));

    let (st_p_dash, pod_dash) = get_html(&html_app, "/podmin?lang=en", Some(&admin_cookie)).await;
    assert_eq!(st_p_dash, StatusCode::OK);
    assert!(pod_dash.contains("id=\"tab-1\""), "/podmin authenticated staff must render admin dashboard");

    // 2. SSR HTML RBAC & EN placeholder parity assertions
    let (st_html, admin_html_en) = get_html(&html_app, "/podmin?lang=en", Some(&admin_cookie)).await;
    assert_eq!(st_html, StatusCode::OK);
    for expected_perm in [
        "value=\"issues:delete\"",
        "value=\"system:read\"",
        "value=\"glossary:write\"",
        "value=\"email_groups:write\"",
    ] {
        assert!(admin_html_en.contains(expected_perm), "Missing RBAC matrix checkbox {}", expected_perm);
    }
    for dead_perm in [
        "value=\"contents:export\"",
        "value=\"contents:history\"",
        "value=\"system:write\"",
        "value=\"users:create\"",
    ] {
        assert!(!admin_html_en.contains(dead_perm), "Dead RBAC matrix checkbox {} should be removed", dead_perm);
    }

    // #chart-content-trend and #chart-top-contents cards gated by analytics:read
    let trend_idx = admin_html_en.find("id=\"chart-content-trend\"").expect("#chart-content-trend present");
    let trend_card_start = admin_html_en[..trend_idx].rfind("class=\"admin-card\"").expect("trend admin-card");
    assert!(
        admin_html_en[trend_card_start..trend_idx].contains("data-required-perm=\"analytics:read\""),
        "#chart-content-trend card must carry data-required-perm=\"analytics:read\""
    );

    let top_idx = admin_html_en.find("id=\"chart-top-contents\"").expect("#chart-top-contents present");
    let top_card_start = admin_html_en[..top_idx].rfind("class=\"admin-card\"").expect("top admin-card");
    assert!(
        admin_html_en[top_card_start..top_idx].contains("data-required-perm=\"analytics:read\""),
        "#chart-top-contents card must carry data-required-perm=\"analytics:read\""
    );

    // #form-seo-redirect includes redirects:write & redirects:manage
    let redir_idx = admin_html_en.find("id=\"form-seo-redirect\"").expect("#form-seo-redirect present");
    let redir_end = admin_html_en[redir_idx..].find('>').map(|o| redir_idx + o).unwrap();
    let redir_tag = &admin_html_en[redir_idx..redir_end];
    assert!(redir_tag.contains("redirects:write") && redir_tag.contains("redirects:manage"));

    // #tab-18 issue drawer & submit use issues:write|issues:manage (never issues:write|contents:publish)
    assert!(!admin_html_en.contains("issues:write|contents:publish"));
    let drawer_btn_idx = admin_html_en.find("data-admin-action=\"open-issue-drawer\"").expect("open-issue-drawer");
    let drawer_btn_tag = &admin_html_en[drawer_btn_idx.saturating_sub(80)..drawer_btn_idx + 120];
    assert!(drawer_btn_tag.contains("data-required-perm=\"issues:write|issues:manage\""));
    let form_issue_submit_idx = admin_html_en.find("form=\"form-issue\"").expect("form-issue submit");
    let form_issue_submit_tag = &admin_html_en[form_issue_submit_idx..form_issue_submit_idx + 160];
    assert!(form_issue_submit_tag.contains("data-required-perm=\"issues:write|issues:manage\""));

    // EN placeholders localized cleanly
    assert!(admin_html_en.contains("UMSA Production"));
    assert!(admin_html_en.contains("320 Pages / 4K TIFF"));
    assert!(admin_html_en.contains("BOOK-MSI-2026"));
    assert!(admin_html_en.contains("msidergisi.com/en/content/sample-analysis"));
    assert!(admin_html_en.contains("example@agency.gov"));
    assert!(!admin_html_en.contains("UMSA Prodüksiyon"));
    assert!(!admin_html_en.contains("320 Sayfa / 4K TIFF"));
    assert!(!admin_html_en.contains("ornek@kurum.gov.tr"));
    assert!(!admin_html_en.contains("ornek-analiz"));

    // 3. Custom role `issues:delete` (without `issues:write` / `issues:manage`)
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/user-roles",
        Some(&admin_cookie),
        Some(json!({
            "role_code": "issue_deleter",
            "name_tr": "Sayı Silici",
            "name_en": "Issue Deleter",
            "permissions_json": "[\"issues:read\",\"issues:delete\"]"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(msi_dergisi_platform::extractors::operator_user::is_staff_role(&state.db, "issue_deleter"));
    let (_, deleter_cookie) = make_user_and_cookie(&state, "issue_deleter@test.local", "issue_deleter");

    let issue_998_payload = json!({
        "issue_number": 998,
        "slug": "sayi-998",
        "title_tr": "Sayı 998",
        "title_en": "Issue 998",
        "publish_month_tr": "Kasım",
        "publish_month_en": "November",
        "year": 2026,
        "cover_image": msi_dergisi_platform::db::constants::DEFAULT_ISSUE_COVER_PATH,
        "is_special_issue": false,
        "event_name": null,
        "page_count": 100,
        "dossier_count": 4,
        "summary_tr": "",
        "summary_en": "",
        "pdf_path": "",
        "uploaded_pdf_file": "",
        "spreads": []
    });
    let (st, _) = send_json(
        &app,
        "POST",
        "/api/v1/admin/issues",
        Some(&admin_cookie),
        Some(issue_998_payload.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    assert_eq!(
        send_json(&app, "POST", "/api/v1/admin/issues", Some(&deleter_cookie), Some(issue_998_payload.clone())).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send_json(&app, "DELETE", "/api/v1/admin/issues/998", Some(&deleter_cookie), None).await.0,
        StatusCode::OK
    );

    // 4. Custom role `redirects:write` & `redirects:manage` parity (/overview + /redirects CRUD)
    for (rc, perm) in [("redirect_only", "redirects:write"), ("redirect_mgr", "redirects:manage")] {
        let (st, _) = send_json(
            &app,
            "POST",
            "/api/v1/admin/user-roles",
            Some(&super_cookie),
            Some(json!({
                "role_code": rc,
                "name_tr": rc,
                "name_en": rc,
                "permissions_json": serde_json::to_string(&vec![perm]).unwrap()
            })),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
    }
    let (_, ro_cookie) = make_user_and_cookie(&state, "ro@test.local", "redirect_only");
    let (_, rm_cookie) = make_user_and_cookie(&state, "rm@test.local", "redirect_mgr");

    assert_eq!(send_json(&app, "GET", "/api/v1/admin/overview", Some(&ro_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/redirects", Some(&ro_cookie), None).await.0, StatusCode::OK);
    let (st, rd_created) = send_json(
        &app,
        "POST",
        "/api/v1/admin/redirects",
        Some(&ro_cookie),
        Some(json!({ "id": 0, "old_path": "/rbac-probe-old", "new_path": "/rbac-probe-new", "status_code": 301, "hit_count": 0 })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let rd_id = rd_created["data"]["id"].as_i64().unwrap();
    assert_eq!(
        send_json(&app, "DELETE", &format!("/api/v1/admin/redirects/{}", rd_id), Some(&ro_cookie), None).await.0,
        StatusCode::OK
    );
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/overview", Some(&rm_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/redirects", Some(&rm_cookie), None).await.0, StatusCode::OK);

    // 5. `managing_editor` #tab-9 parity & `content_publisher` #tab-18 parity
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/system-health", Some(&mg_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/analytics", Some(&mg_cookie), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/analytics/contents", Some(&mg_cookie), None).await.0, StatusCode::FORBIDDEN);

    assert_eq!(send_json(&app, "GET", "/api/v1/admin/issues", Some(&pub_cookie), None).await.0, StatusCode::OK);
    assert_eq!(
        send_json(&app, "POST", "/api/v1/admin/issues", Some(&pub_cookie), Some(issue_998_payload)).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(send_json(&app, "DELETE", "/api/v1/admin/issues/280", Some(&pub_cookie), None).await.0, StatusCode::FORBIDDEN);

    // 6. Single-module custom roles on GET /api/v1/admin/overview + SSR tab visibility
    for (idx, perm) in [
        "media:read",
        "monitoring:read",
        "translations:read",
        "glossary:write",
        "glossary:manage",
        "locales:manage",
        "newsletters:read",
        "email_groups:write",
        "email_groups:manage",
        "backup:manage",
        "roles:manage",
        "redirects:write",
        "system:read",
        "issues:delete",
    ]
    .iter()
    .enumerate()
    {
        let rc = format!("single_mod_{}", idx);
        let (st, _) = send_json(
            &app,
            "POST",
            "/api/v1/admin/user-roles",
            Some(&super_cookie),
            Some(json!({
                "role_code": rc,
                "name_tr": rc,
                "name_en": rc,
                "permissions_json": serde_json::to_string(&vec![perm]).unwrap()
            })),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        let (_, u_cookie) = make_user_and_cookie(&state, &format!("single_{}@test.local", idx), &rc);
        let (st, ov) = send_json(&app, "GET", "/api/v1/admin/overview", Some(&u_cookie), None).await;
        assert_eq!(st, StatusCode::OK, "Single-module perm {} should succeed on /overview", perm);
        assert_eq!(ov["total_contents"], json!(0));
        assert_eq!(ov["active_ads"], json!(0));
        assert_eq!(ov["registered_users"], json!(0));
    }

    // Helper: extract visible sidebar tab IDs for a single permission from /podmin SSR
    let visible_sidebar_tabs = |html: &str, single_perm: &str| -> Vec<String> {
        let mut out = Vec::new();
        for chunk in html.split("<button type=\"button\" id=\"tab-btn-").skip(1) {
            let Some(num_end) = chunk.find('"') else { continue };
            let tab_id = format!("tab-btn-{}", &chunk[..num_end]);
            let Some(tag_end) = chunk.find('>') else { continue };
            let tag = &chunk[..tag_end];
            if let Some(p_idx) = tag.find("data-required-perm=\"") {
                let rest = &tag[p_idx + "data-required-perm=\"".len()..];
                if let Some(q_end) = rest.find('"') {
                    let expr = &rest[..q_end];
                    if expr.split('|').any(|p| p.trim() == single_perm) {
                        out.push(tab_id);
                    }
                }
            }
        }
        out
    };

    // 6a. `sm_system` (["system:read"]): >= 1 visible sidebar tab (tab-btn-17 & tab-btn-9) + 200 on /overview & /system-health
    let _ = send_json(
        &app,
        "POST",
        "/api/v1/admin/user-roles",
        Some(&super_cookie),
        Some(json!({
            "role_code": "sm_system",
            "name_tr": "Sistem Okuyucu",
            "name_en": "System Reader",
            "permissions_json": "[\"system:read\"]"
        })),
    )
    .await;
    let (_, sm_system_cookie) = make_user_and_cookie(&state, "sm_system@test.local", "sm_system");
    let (st, sm_sys_html) = get_html(&html_app, "/podmin?lang=en", Some(&sm_system_cookie)).await;
    assert_eq!(st, StatusCode::OK);
    let sys_tabs = visible_sidebar_tabs(&sm_sys_html, "system:read");
    assert!(!sys_tabs.is_empty(), "sm_system must see >= 1 sidebar tab in /podmin SSR");
    assert!(sys_tabs.contains(&"tab-btn-17".to_string()), "sm_system must see tab-btn-17");
    assert!(sys_tabs.contains(&"tab-btn-9".to_string()), "sm_system must see tab-btn-9");
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/overview", Some(&sm_system_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/system-health", Some(&sm_system_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/monitoring/attacks", Some(&sm_system_cookie), None).await.0, StatusCode::OK);

    // 6b. `sm_issues_del` (["issues:delete"]): >= 1 visible sidebar tab (tab-btn-18) + 200 on /overview & /issues, 403 on POST /issues
    let _ = send_json(
        &app,
        "POST",
        "/api/v1/admin/user-roles",
        Some(&super_cookie),
        Some(json!({
            "role_code": "sm_issues_del",
            "name_tr": "Sayı Silici Tekil",
            "name_en": "Single Issue Deleter",
            "permissions_json": "[\"issues:delete\"]"
        })),
    )
    .await;
    let (_, sm_issues_del_cookie) = make_user_and_cookie(&state, "sm_issues_del@test.local", "sm_issues_del");
    let (st, sm_iss_html) = get_html(&html_app, "/podmin?lang=en", Some(&sm_issues_del_cookie)).await;
    assert_eq!(st, StatusCode::OK);
    let iss_tabs = visible_sidebar_tabs(&sm_iss_html, "issues:delete");
    assert!(!iss_tabs.is_empty(), "sm_issues_del must see >= 1 sidebar tab in /podmin SSR");
    assert!(iss_tabs.contains(&"tab-btn-18".to_string()), "sm_issues_del must see tab-btn-18");
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/overview", Some(&sm_issues_del_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/issues", Some(&sm_issues_del_cookie), None).await.0, StatusCode::OK);
    assert_eq!(
        send_json(
            &app,
            "POST",
            "/api/v1/admin/issues",
            Some(&sm_issues_del_cookie),
            Some(json!({
                "issue_number": 997,
                "slug": "sayi-997",
                "title_tr": "Sayı 997",
                "title_en": "Issue 997",
                "publish_month_tr": "Kasım",
                "publish_month_en": "November",
                "year": 2026,
                "cover_image": msi_dergisi_platform::db::constants::DEFAULT_ISSUE_COVER_PATH,
                "is_special_issue": false,
                "event_name": null,
                "page_count": 100,
                "dossier_count": 4,
                "summary_tr": "",
                "summary_en": "",
                "pdf_path": "",
                "uploaded_pdf_file": "",
                "spreads": []
            })),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    // 6c. `sm_glossary_w` (["glossary:write"]) & `sm_eg_w` (["email_groups:write"]) UI/API parity
    let _ = send_json(
        &app,
        "POST",
        "/api/v1/admin/user-roles",
        Some(&super_cookie),
        Some(json!({
            "role_code": "sm_glossary_w",
            "name_tr": "Terim Yazarı",
            "name_en": "Glossary Writer",
            "permissions_json": "[\"glossary:write\"]"
        })),
    )
    .await;
    let (_, sm_gl_cookie) = make_user_and_cookie(&state, "sm_glossary_w@test.local", "sm_glossary_w");
    let (_, gl_html) = get_html(&html_app, "/podmin?lang=en", Some(&sm_gl_cookie)).await;
    assert!(visible_sidebar_tabs(&gl_html, "glossary:write").contains(&"tab-btn-8".to_string()));
    assert!(gl_html.contains("data-subtab=\"tab8-glossary\"") && gl_html.contains("glossary:write|glossary:manage"));
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/glossary", Some(&sm_gl_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "DELETE", "/api/v1/admin/glossary/999999", Some(&sm_gl_cookie), None).await.0, StatusCode::FORBIDDEN);

    let _ = send_json(
        &app,
        "POST",
        "/api/v1/admin/user-roles",
        Some(&super_cookie),
        Some(json!({
            "role_code": "sm_eg_w",
            "name_tr": "E-Posta Grup Yazarı",
            "name_en": "Email Group Writer",
            "permissions_json": "[\"email_groups:write\"]"
        })),
    )
    .await;
    let (_, sm_eg_cookie) = make_user_and_cookie(&state, "sm_eg_w@test.local", "sm_eg_w");
    let (_, eg_html) = get_html(&html_app, "/podmin?lang=en", Some(&sm_eg_cookie)).await;
    assert!(visible_sidebar_tabs(&eg_html, "email_groups:write").contains(&"tab-btn-6".to_string()));
    assert!(eg_html.contains("data-subtab=\"tab6-email-groups\"") && eg_html.contains("email_groups:write|email_groups:manage"));
    assert_eq!(send_json(&app, "GET", "/api/v1/admin/email-groups", Some(&sm_eg_cookie), None).await.0, StatusCode::OK);
    assert_eq!(send_json(&app, "DELETE", "/api/v1/admin/email-groups/nonexistent", Some(&sm_eg_cookie), None).await.0, StatusCode::FORBIDDEN);

    // 7. Translations ?status=all, empty-status pagination (per_page=1), and 404 consistency
    for (k, status) in [
        ("000.qa.r2.draft", "draft"),
        ("000.qa.r2.auto", "auto_translated"),
        ("000.qa.r2.approved", "approved"),
    ] {
        let entry = msi_dergisi_platform::engine::translation_engine::TranslationEntry {
            key: k.into(),
            value_tr: k.into(),
            value_en: k.into(),
            value_ar: String::new(),
            value_de: String::new(),
            value_ru: String::new(),
            value_fr: String::new(),
            group: "qa".into(),
        };
        state
            .db
            .upsert_translation_with_status(&entry, status, None, None, Some("tr"))
            .expect("insert translation");
    }

    let (st, all_resp) = send_json(
        &app,
        "GET",
        "/api/v1/admin/translations/pending?status=all&per_page=200",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let all_items = all_resp["data"].as_array().unwrap();
    assert!(all_items.iter().any(|x| x["key"] == "000.qa.r2.draft"));
    assert!(all_items.iter().any(|x| x["key"] == "000.qa.r2.auto"));
    assert!(all_items.iter().any(|x| x["key"] == "000.qa.r2.approved"));

    let (st, p1) = send_json(
        &app,
        "GET",
        "/api/v1/admin/translations/pending?status=&page=1&per_page=1",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, p2) = send_json(
        &app,
        "GET",
        "/api/v1/admin/translations/pending?status=&page=2&per_page=1",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let p1_arr = p1["data"].as_array().unwrap();
    let p2_arr = p2["data"].as_array().unwrap();
    assert_eq!(p1_arr.len(), 1);
    assert_eq!(p2_arr.len(), 1);
    assert_ne!(p1_arr[0]["key"], p2_arr[0]["key"]);
    assert!(p1["meta"]["total_count"].as_u64().unwrap() >= 2);
    assert!(p1["meta"]["total_pages"].as_u64().unwrap() >= 2);
    assert_eq!(p2["meta"]["total_pages"], p1["meta"]["total_pages"]);

    let (st, empty_pend) = send_json(
        &app,
        "GET",
        "/api/v1/admin/translations/pending?status=nonexistent_status&page=1&per_page=10",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(empty_pend["meta"]["total_count"], json!(0));
    assert_eq!(empty_pend["meta"]["total_pages"], json!(0));

    // 404 on nonexistent translation approve/reject, locale delete, email-group delete
    assert_eq!(
        send_json(&app, "POST", "/api/v1/admin/translations/missing.key/approve", Some(&admin_cookie), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send_json(&app, "POST", "/api/v1/admin/translations/missing.key/reject", Some(&admin_cookie), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send_json(&app, "DELETE", "/api/v1/admin/locales/zz", Some(&admin_cookie), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send_json(&app, "DELETE", "/api/v1/admin/locales/tr", Some(&admin_cookie), None).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send_json(&app, "DELETE", "/api/v1/admin/email-groups/nonexistent", Some(&admin_cookie), None).await.0,
        StatusCode::NOT_FOUND
    );

    cleanup_test_db(&db_path);
}
