//! Integration tests for Milestone M5 — Authenticated `/hesabim` Role Portal Overhaul.

mod common;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use tower::ServiceExt;

use common::*;
use msi_dergisi_platform::{
    models::{CorporateVerificationRequest, OrderInvoice},
    routes,
    services::auth_service,
};

async fn get_portal(
    app: &axum::Router,
    uri: &str,
    cookie: Option<&str>,
) -> (StatusCode, axum::http::HeaderMap, String) {
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
    let headers = resp.headers().clone();
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024).await.unwrap();
    (status, headers, String::from_utf8_lossy(&bytes).to_string())
}

fn make_portal_user(
    state: &msi_dergisi_platform::app_state::AppState,
    email: &str,
    role: &str,
    email_verified: bool,
    is_gov_verified: bool,
    subscription_tier: &str,
) -> (i64, String) {
    let hash = auth_service::hash_password("TestPass123!", &state.config.session_secret);
    let uid = state
        .db
        .admin_create_user(
            email,
            &hash,
            "M5 Test Kullanıcısı",
            role,
            "Sektör Kuruluşları",
            "Savunma Test A.Ş.",
            "Sistem Mühendisi",
            is_gov_verified,
            subscription_tier,
        )
        .expect("create user");
    if email_verified {
        let _ = state.db.mark_email_verified(uid);
    } else {
        let conn = state.db.open_conn().expect("open conn");
        conn.execute("UPDATE users SET email_verified = 0 WHERE id = ?1", [uid])
            .expect("clear email_verified");
        state.db.refresh_l1_sections(&["users"]);
    }
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
async fn guest_hesabim_stays_on_public_shell_while_authenticated_uses_isolated_portal() {
    let (state, db_path) = build_test_state("m5_guest_vs_auth");
    let app = routes::build_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
    ));

    // Guest keeps public shell + .account-welcome + #modal-login + msi-ui.js
    let (st_guest, _, guest_html) = get_portal(&app, "/hesabim", None).await;
    assert_eq!(st_guest, StatusCode::OK);
    assert!(guest_html.contains("class=\"account-welcome\""));
    assert!(guest_html.contains("href=\"#modal-login\""));
    assert!(guest_html.contains("msi-ui.js"));
    assert!(!guest_html.contains("account-portal.js"));

    // Authenticated uses dedicated standalone portal shell without msi-ui.js or msi-design-system.css
    let (_, cookie) = make_portal_user(
        &state,
        "isolated@test.local",
        "subscriber_digital",
        true,
        false,
        "Dijital Abone",
    );
    let (st_auth, hdrs, auth_html) = get_portal(&app, "/hesabim", Some(&cookie)).await;
    assert_eq!(st_auth, StatusCode::OK);
    assert!(auth_html.contains("/css/account-portal.css"));
    assert!(auth_html.contains("/js/account-portal.js"));
    assert!(!auth_html.contains("msi-ui.js"));
    assert!(!auth_html.contains("msi-design-system.css"));
    assert!(!auth_html.contains("lenis.min.js"));
    assert!(!auth_html.contains("gsap.min.js"));
    assert_eq!(auth_html.matches("<h1").count(), 1);
    assert_eq!(auth_html.matches("<main").count(), 1);

    let csp = hdrs
        .get("content-security-policy")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(csp.contains("nonce-"));
    assert!(auth_html.contains("nonce=\""));

    // Static assets are served cleanly via static_assets::build("./public")
    let static_app = routes::static_assets::build(concat!(env!("CARGO_MANIFEST_DIR"), "/public"))
        .with_state(state.clone());
    let (st_css, _, css_body) = get_portal(&static_app, "/css/account-portal.css", None).await;
    assert_eq!(st_css, StatusCode::OK);
    assert!(css_body.contains(".ap-topbar"));
    let (st_js, _, js_body) = get_portal(&static_app, "/js/account-portal.js", None).await;
    assert_eq!(st_js, StatusCode::OK);
    assert!(js_body.contains("__MSI_ACCOUNT_PORTAL_INIT__"));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn all_non_staff_roles_and_staff_render_role_adaptive_sections_and_edergi_gating() {
    let (state, db_path) = build_test_state("m5_roles_edergi");
    let app = routes::build_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
    ));

    let role_cases = [
        ("registered", false, false, "", false, "Kayıtlı Okuyucu", "Registered Reader"),
        (
            "subscriber_public",
            true,
            true,
            "Kamu E-Dergi Abonesi",
            true,
            "Kamu E-Dergi Abonesi",
            "Public Sector Subscriber",
        ),
        (
            "subscriber_digital",
            true,
            false,
            "Dijital Abone",
            true,
            "Dijital E-Dergi Abonesi",
            "Digital E-Magazine Subscriber",
        ),
        (
            "subscriber_print_digital",
            true,
            false,
            "Basılı + Dijital Abone",
            true,
            "Basılı + Dijital Abone",
            "Print &amp; Digital Subscriber",
        ),
        (
            "subscriber_corporate",
            true,
            false,
            "Kurumsal Abone",
            true,
            "Kurumsal Abone",
            "Corporate Subscriber",
        ),
    ];

    for (role, verified, gov, tier, expects_pdf, tr_badge, en_badge) in role_cases {
        let email = format!("{}_m5@test.local", role);
        let (_, cookie) = make_portal_user(&state, &email, role, verified, gov, tier);

        let (st_tr, _, html_tr) = get_portal(&app, "/hesabim", Some(&cookie)).await;
        assert_eq!(st_tr, StatusCode::OK);
        assert!(html_tr.contains(tr_badge), "Missing TR badge for {}", role);
        assert!(!html_tr.contains("account-command-card"), "Non-staff {} leaked command card", role);
        assert!(!html_tr.contains(&state.config.admin_path_secret));
        for panel in ["overview", "edergi", "profile", "security", "notifications"] {
            assert!(
                html_tr.contains(&format!("id=\"ap-panel-{}\"", panel)),
                "Missing panel {} for {}",
                panel,
                role
            );
        }
        assert!(html_tr.contains("id=\"account-invoices\""));
        assert!(html_tr.contains("id=\"account-delete-confirm-dialog\""));
        assert!(html_tr.contains("id=\"account-totp-card\""));
        assert!(html_tr.contains("id=\"account-kvkk-card\""));

        let has_pdf_dl = html_tr.contains("/download-pdf?");
        assert_eq!(
            has_pdf_dl, expects_pdf,
            "Unexpected PDF download button state for role {}",
            role
        );

        let (st_en, _, html_en) = get_portal(&app, "/hesabim?lang=en", Some(&cookie)).await;
        assert_eq!(st_en, StatusCode::OK);
        assert!(html_en.contains(en_badge), "Missing EN badge for {}", role);
        assert!(html_en.contains("personalised defence feed"));
        assert!(!html_en.contains("İki Aşamalı Doğrulama"));
        assert!(!html_en.contains("Siparişleriniz ve Faturalar"));
        assert!(!html_en.contains("E-Dergi &amp; Abonelik Merkezi"));
        assert!(!html_en.contains("Güvenlik, 2FA &amp; KVKK Gizlilik"));
        assert!(html_en.contains(
            "<th scope=\"col\">Type</th><th scope=\"col\">Version</th><th scope=\"col\">Status</th><th scope=\"col\">Date</th>"
        ));
    }

    // Verified subscriber without verified email cannot see direct PDF download yet
    let (_, unverified_sub_cookie) = make_portal_user(
        &state,
        "unverified_sub@test.local",
        "subscriber_digital",
        false,
        false,
        "Dijital Abone",
    );
    let (st_uv, _, html_uv) = get_portal(&app, "/hesabim", Some(&unverified_sub_cookie)).await;
    assert_eq!(st_uv, StatusCode::OK);
    assert!(!html_uv.contains("/download-pdf?"));
    assert!(html_uv.contains("PDF için E-posta Doğrula"));

    // Staff role DOES see account-command-card & admin path links
    let (_, staff_cookie) = make_portal_user(
        &state,
        "staff_m5@msidergisi.com",
        "super_admin",
        true,
        false,
        "Kurumsal",
    );
    let (st_staff, _, html_staff) = get_portal(&app, "/hesabim", Some(&staff_cookie)).await;
    assert_eq!(st_staff, StatusCode::OK);
    assert!(html_staff.contains("account-command-card"));
    assert!(html_staff.contains(&format!("/{}", state.config.admin_path_secret)));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn corporate_verification_status_and_payment_query_hints_render_cleanly() {
    let (state, db_path) = build_test_state("m5_corp_hints");
    let app = routes::build_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
    ));

    let (uid, cookie) = make_portal_user(
        &state,
        "corp_pending@test.local",
        "registered",
        true,
        false,
        "",
    );

    state
        .db
        .insert_corporate_request(&CorporateVerificationRequest {
            id: 0,
            user_id: uid,
            email: "corp_pending@test.local".into(),
            full_name: "M5 Test Kullanıcısı".into(),
            organization_category: "Sektör Kuruluşları".into(),
            organization_name: "Test Savunma Teknolojileri".into(),
            request_message: "Kurumsal doğrulama talebi".into(),
            status: "pending".into(),
            reviewed_by: None,
            created_at: "2026-11-21T12:00:00Z".into(),
        })
        .expect("insert corp request");

    let (st, _, html) = get_portal(
        &app,
        "/hesabim?payment=success&oid=ORD-2026-999",
        Some(&cookie),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(html.contains("Test Savunma Teknolojileri"));
    assert!(html.contains("İnceleme Bekliyor"));
    assert!(html.contains("ORD-2026-999"));
    assert!(html.contains("data-initial-tab=\"edergi\""));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn empty_tier_empty_org_and_invalid_tab_query_render_clean_ssr_fallbacks() {
    let (state, db_path) = build_test_state("m5_fallbacks");
    let app = routes::build_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
    ));

    let hash = auth_service::hash_password("TestPass123!", &state.config.session_secret);
    let uid = state
        .db
        .admin_create_user(
            "fallback_probe@test.local",
            &hash,
            "Ayşe Fallback",
            "subscriber_digital",
            "Sektör Kuruluşları",
            "",
            "",
            false,
            "",
        )
        .expect("create fallback user");
    let _ = state.db.mark_email_verified(uid);
    let user = state
        .db
        .get_all_users()
        .into_iter()
        .find(|u| u.id == uid)
        .expect("find fallback user");
    let token = auth_service::generate_session_token(&user, &state.config.session_secret);
    let cookie = format!("msi_session={}", token);

    // Invalid ?tab=nonexistent must normalize to overview so SSR never hides all 5 panels
    let (st_tr, _, html_tr) = get_portal(&app, "/hesabim?tab=nonexistent", Some(&cookie)).await;
    assert_eq!(st_tr, StatusCode::OK);
    assert!(html_tr.contains("data-initial-tab=\"overview\""));
    assert!(html_tr.contains(r#"id="ap-panel-overview" class="portal-section" role="tabpanel" aria-labelledby="ap-tab-overview" data-ap-panel="overview">"#));
    assert!(html_tr.contains(r#"id="ap-panel-edergi" class="portal-section" role="tabpanel" aria-labelledby="ap-tab-edergi" data-ap-panel="edergi" hidden>"#));
    assert!(!html_tr.contains("<h2></h2>"), "Must never render blank <h2>");
    // Identity card falls back to full_name while profile input keeps empty organization_name
    assert!(html_tr.contains("<h2>Ayşe Fallback</h2>"));
    assert!(html_tr.contains(r#"name="organization_name" value="""#));
    // #ap-panel-edergi plan card falls back to role label instead of blank <h2>
    assert!(html_tr.contains("<h2>Dijital E-Dergi Abonesi</h2>"));

    // Valid uppercase ?tab=SECURITY normalizes cleanly in EN mode
    let (st_en, _, html_en) = get_portal(&app, "/hesabim?lang=en&tab=SECURITY", Some(&cookie)).await;
    assert_eq!(st_en, StatusCode::OK);
    assert!(html_en.contains("data-initial-tab=\"security\""));
    assert!(html_en.contains(r#"id="ap-panel-security" class="portal-section" role="tabpanel" aria-labelledby="ap-tab-security" data-ap-panel="security">"#));
    assert!(html_en.contains("<h2>Digital E-Magazine Subscriber</h2>"));
    assert!(!html_en.contains("<h2></h2>"));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn purchased_digital_sku_renders_download_link_in_edergi_panel() {
    let (state, db_path) = build_test_state("m5_purchased_sku");
    let app = routes::build_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
    ));

    let (_, cookie) = make_portal_user(
        &state,
        "buyer_m5@test.local",
        "registered",
        true,
        false,
        "",
    );

    let sku = "RAPOR-SAVUNMA-KOBI-2026";
    state
        .db
        .insert_order_invoice(&OrderInvoice {
            id: 0,
            invoice_no: "MSI-2026-880011".into(),
            user_email: "buyer_m5@test.local".into(),
            full_name: "M5 Test Kullanıcısı".into(),
            organization_name: "Savunma Test A.Ş.".into(),
            item_type: "product".into(),
            item_code: sku.into(),
            item_title: "Savunma Sanayii KOBİ Raporu".into(),
            amount_try: 490.0,
            payment_status: "paid".into(),
            created_at: "2026-11-21T16:00:00Z".into(),
            pdf_watermark_code: "WM-M5-880011".into(),
        })
        .expect("insert paid product invoice");

    let (st_tr, _, html_tr) = get_portal(&app, "/hesabim?tab=edergi", Some(&cookie)).await;
    assert_eq!(st_tr, StatusCode::OK);
    assert!(html_tr.contains("DİJİTAL KÜTÜPHANE"));
    assert!(html_tr.contains("Satın Alınan Rapor ve Yayınlar"));
    assert!(html_tr.contains(&format!("/api/v1/commerce/download/{}", sku)));
    assert!(html_tr.contains("MSI-2026-880011"));

    let (st_en, _, html_en) = get_portal(&app, "/hesabim?lang=en&tab=edergi", Some(&cookie)).await;
    assert_eq!(st_en, StatusCode::OK);
    assert!(html_en.contains("Purchased Reports &amp; Assets"));
    assert!(html_en.contains(&format!("/api/v1/commerce/download/{}", sku)));

    cleanup_test_db(&db_path);
}
