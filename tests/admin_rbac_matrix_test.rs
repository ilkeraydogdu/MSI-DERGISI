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

    // 1. Unauthenticated requests rejected with 401
    let (st, _) = send_json(&app, "GET", "/api/v1/admin/me", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // 2. Reader (`registered`) / subscriber (`subscriber_paid`) rejected with 403
    let (_, reader_cookie) = make_user_and_cookie(&state, "reader_qa@test.local", "registered");
    let (st, _) = send_json(&app, "GET", "/api/v1/admin/me", Some(&reader_cookie), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    let (_, sub_cookie) = make_user_and_cookie(&state, "sub_qa@test.local", "subscriber_paid");
    let (st, _) = send_json(&app, "GET", "/api/v1/admin/issues", Some(&sub_cookie), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);

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

    cleanup_test_db(&db_path);
}
