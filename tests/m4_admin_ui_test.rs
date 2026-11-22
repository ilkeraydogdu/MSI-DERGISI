//! Integration tests for Milestone M4: Admin UI/UX overhaul, Design Studio contracts,
//! missing admin wiring SSR markers, and newsletter auto-sponsor fallback (`REQ-027`).

mod common;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

use common::*;
use msi_dergisi_platform::{
    controllers::api_controller::admin_ops::resolve_newsletter_sponsor_ad,
    models::{AdCampaign, NewsletterCampaign},
    routes,
    services::auth_service,
    workers::newsletter_scheduler::run_newsletter_scheduler_tick,
};

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

fn make_admin_cookie(state: &msi_dergisi_platform::app_state::AppState, email: &str) -> String {
    let hash = auth_service::hash_password("TestPass123!", &state.config.session_secret);
    let uid = state
        .db
        .admin_create_user(
            email,
            &hash,
            "M4 QA Admin",
            "super_admin",
            "SSB",
            "MSI QA",
            "Director",
            true,
            "",
        )
        .expect("create super_admin");
    let user = state
        .db
        .get_all_users()
        .into_iter()
        .find(|u| u.id == uid)
        .expect("find created admin");
    let token = auth_service::generate_session_token(&user, &state.config.session_secret);
    format!("msi_session={}", token)
}

fn make_test_ad(
    id: i64,
    sponsor_name: &str,
    budget_score: i32,
    is_newsletter_ad: bool,
    is_active: bool,
    end_date: Option<&str>,
) -> AdCampaign {
    AdCampaign {
        id,
        slot_code: "ad-newsletter".to_string(),
        badge_label: "SPONSORLU".to_string(),
        dimensions: "600x120".to_string(),
        sponsor_name: sponsor_name.to_string(),
        sponsor_type: "defense_oem".to_string(),
        pill_style: "navy".to_string(),
        title_tr: format!("{} Banner", sponsor_name),
        title_en: format!("{} Banner", sponsor_name),
        desc_tr: "Açıklama".to_string(),
        desc_en: "Description".to_string(),
        target_url: "https://example.com".to_string(),
        image_url: None,
        budget_score,
        is_default_fallback: false,
        is_floating_popup: false,
        is_newsletter_ad,
        target_categories: vec![],
        target_ssb_tags: vec![],
        target_institutions: vec!["ALL".to_string()],
        impressions_count: 0,
        clicks_count: 0,
        is_active,
        start_date: None,
        end_date: end_date.map(str::to_string),
        rotation_mode: "weighted".to_string(),
    }
}

#[test]
fn resolve_newsletter_sponsor_selects_highest_priority_active_in_schedule() {
    let ads = vec![
        make_test_ad(1, "EXPIRED OEM", 99, true, true, Some("2020-01-01")),
        make_test_ad(2, "INACTIVE OEM", 98, true, false, None),
        make_test_ad(3, "DISPLAY ONLY", 97, false, true, None),
        make_test_ad(4, "NL LOW PRIO", 60, true, true, None),
        make_test_ad(5, "NL TOP PRIO", 92, true, true, Some("2099-12-31")),
    ];

    let chosen = resolve_newsletter_sponsor_ad(&ads, None, "ALL", "ALL", "2026-11-21")
        .expect("must auto-select active newsletter ad within schedule");
    assert_eq!(chosen.id, 5);
    assert_eq!(chosen.sponsor_name, "NL TOP PRIO");

    let explicit = resolve_newsletter_sponsor_ad(&ads, Some(4), "ALL", "ALL", "2026-11-21")
        .expect("explicit sponsor_ad_id must still resolve");
    assert_eq!(explicit.id, 4);
}

#[tokio::test]
async fn newsletter_auto_sponsor_fallback_works_in_api_and_scheduler() {
    let (state, db_path) = build_test_state("m4_nl_auto_sponsor");
    let app = routes::build_router(state.clone());
    let cookie = make_admin_cookie(&state, "m4-nl-admin@test.local");

    // Seed two active newsletter ads + one inactive higher-score newsletter ad
    let (st1, r1) = send_json(
        &app,
        "POST",
        "/api/v1/admin/ads",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "slot_code": "ad-nl-1",
            "badge_label": "SPONSOR",
            "dimensions": "600x120",
            "sponsor_name": "HAVELSAN_AUTO_TOP",
            "sponsor_type": "defense_oem",
            "pill_style": "navy",
            "title_tr": "Havelsan Bülten Banner",
            "title_en": "Havelsan Newsletter Banner",
            "desc_tr": "Komuta kontrol çözümleri",
            "desc_en": "Command control solutions",
            "target_url": "https://example.com/havelsan",
            "budget_score": 96,
            "is_default_fallback": false,
            "is_floating_popup": false,
            "is_newsletter_ad": true,
            "is_active": true,
            "target_categories": [],
            "target_institutions": ["ALL"],
            "target_ssb_tags": [],
            "impressions_count": 0,
            "clicks_count": 0
        })),
    )
    .await;
    assert_eq!(st1, StatusCode::OK, "ad 1 response: {}", r1);
    let top_ad_id = r1["data"]["id"].as_i64().unwrap();

    let (st2, r2) = send_json(
        &app,
        "POST",
        "/api/v1/admin/ads",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "slot_code": "ad-nl-2",
            "badge_label": "SPONSOR",
            "dimensions": "600x120",
            "sponsor_name": "ROKETSAN_EXPLICIT",
            "sponsor_type": "defense_oem",
            "pill_style": "gold",
            "title_tr": "Roketsan Bülten Banner",
            "title_en": "Roketsan Newsletter Banner",
            "desc_tr": "Hassas mühimmat",
            "desc_en": "Precision munitions",
            "target_url": "https://example.com/roketsan",
            "budget_score": 70,
            "is_default_fallback": false,
            "is_floating_popup": false,
            "is_newsletter_ad": true,
            "is_active": true,
            "target_categories": [],
            "target_institutions": ["ALL"],
            "target_ssb_tags": [],
            "impressions_count": 0,
            "clicks_count": 0
        })),
    )
    .await;
    assert_eq!(st2, StatusCode::OK, "ad 2 response: {}", r2);
    let explicit_ad_id = r2["data"]["id"].as_i64().unwrap();

    // 1. Immediate newsletter with sponsor_ad_id=null must auto-select top active newsletter ad
    let (nl_st, nl_resp) = send_json(
        &app,
        "POST",
        "/api/v1/admin/newsletters",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "subject_tr": "M4 Oto Sponsor Test",
            "subject_en": "M4 Auto Sponsor Test",
            "target_group_filter": "all_consented",
            "target_institution": "ALL",
            "target_ssb_tag": "ALL",
            "sponsor_ad_id": null,
            "sponsor_name": null,
            "body_html": "<p>Test gövde</p>",
            "recipient_count": 0,
            "status": "sent",
            "sent_at": ""
        })),
    )
    .await;
    assert_eq!(nl_st, StatusCode::OK, "newsletter create: {}", nl_resp);
    let auto_nl_id = nl_resp["data"]["id"].as_i64().unwrap();

    // 2. Explicit sponsor_ad_id must be honored rather than overwritten
    let (nl2_st, nl2_resp) = send_json(
        &app,
        "POST",
        "/api/v1/admin/newsletters",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "subject_tr": "M4 Explicit Sponsor Test",
            "subject_en": "M4 Explicit Sponsor Test",
            "target_group_filter": "all_consented",
            "target_institution": "ALL",
            "target_ssb_tag": "ALL",
            "sponsor_ad_id": explicit_ad_id,
            "sponsor_name": null,
            "body_html": "<p>Explicit sponsor</p>",
            "recipient_count": 0,
            "status": "sent",
            "sent_at": ""
        })),
    )
    .await;
    assert_eq!(nl2_st, StatusCode::OK, "explicit newsletter create: {}", nl2_resp);
    let explicit_nl_id = nl2_resp["data"]["id"].as_i64().unwrap();

    // 3. Future-scheduled newsletter via API with sponsor_ad_id=null auto-resolves & stores sponsor before dispatch
    let (nl3_st, nl3_resp) = send_json(
        &app,
        "POST",
        "/api/v1/admin/newsletters",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "subject_tr": "M4 Future Scheduled Auto Sponsor",
            "subject_en": "M4 Future Scheduled Auto Sponsor",
            "target_group_filter": "all_consented",
            "target_institution": "ALL",
            "target_ssb_tag": "ALL",
            "sponsor_ad_id": null,
            "sponsor_name": null,
            "body_html": "<p>Future scheduled body</p>",
            "recipient_count": 0,
            "status": "scheduled",
            "sent_at": "",
            "scheduled_at": "2099-01-01T10:00:00Z"
        })),
    )
    .await;
    assert_eq!(nl3_st, StatusCode::OK, "future scheduled create: {}", nl3_resp);
    let future_sched_id = nl3_resp["data"]["id"].as_i64().unwrap();

    // 4. Due scheduled newsletter persisted with sponsor_ad_id=None, then claimed & dispatched by scheduler tick
    let sched_nl_id = state
        .db
        .insert_newsletter_campaign(&NewsletterCampaign {
            id: 0,
            subject_tr: "M4 Scheduled Auto Sponsor".to_string(),
            subject_en: "M4 Scheduled Auto Sponsor".to_string(),
            target_group_filter: "all_consented".to_string(),
            target_institution: "ALL".to_string(),
            target_ssb_tag: "ALL".to_string(),
            sponsor_ad_id: None,
            sponsor_name: None,
            recipient_count: 0,
            status: "scheduled".to_string(),
            sent_at: String::new(),
            email_group_code: None,
            scheduled_at: Some("2020-01-01T10:00:00Z".to_string()),
            body_html: "<p>Scheduled body</p>".to_string(),
        })
        .expect("insert due scheduled campaign");

    let processed = run_newsletter_scheduler_tick(&state).await;
    assert_eq!(processed, 1, "due scheduled campaign must be dispatched");

    let campaigns = state.db.get_newsletter_campaigns();
    let auto_c = campaigns.iter().find(|c| c.id == auto_nl_id).unwrap();
    assert_eq!(auto_c.sponsor_ad_id, Some(top_ad_id));
    assert_eq!(auto_c.sponsor_name.as_deref(), Some("HAVELSAN_AUTO_TOP"));

    let exp_c = campaigns.iter().find(|c| c.id == explicit_nl_id).unwrap();
    assert_eq!(exp_c.sponsor_ad_id, Some(explicit_ad_id));
    assert_eq!(exp_c.sponsor_name.as_deref(), Some("ROKETSAN_EXPLICIT"));

    let future_c = campaigns.iter().find(|c| c.id == future_sched_id).unwrap();
    assert_eq!(future_c.status, "scheduled");
    assert_eq!(future_c.sponsor_ad_id, Some(top_ad_id));
    assert_eq!(future_c.sponsor_name.as_deref(), Some("HAVELSAN_AUTO_TOP"));

    let sched_c = campaigns.iter().find(|c| c.id == sched_nl_id).unwrap();
    assert!(matches!(sched_c.status.as_str(), "sent" | "queued"), "status was {}", sched_c.status);

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn admin_ssr_pages_use_svg_icons_and_render_all_m4_controls() {
    let (state, db_path) = build_test_state("m4_ssr_contracts");
    let app = routes::build_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        msi_dergisi_platform::middleware::central_waf_and_telemetry_middleware,
    ));
    let admin_base = format!("/{}", state.config.admin_path_secret.trim_matches('/'));

    // 1. Unauthenticated -> Login SSR
    let (login_st, login_html) = get_html(&app, &admin_base, None).await;
    assert_eq!(login_st, StatusCode::OK);
    assert!(!login_html.contains("/css/icons.css"), "login must not load icons.css");
    assert!(!login_html.contains("class=\"fa "), "login must not render FontAwesome <i> tags");
    assert!(login_html.contains("class=\"adm-icon"), "login must render inline SVG icons");

    // 2. Authenticated Dashboard SSR (TR & EN)
    let cookie = make_admin_cookie(&state, "m4-ssr-admin@test.local");
    for suffix in ["", "?lang=en"] {
        let (st, html) = get_html(&app, &format!("{}{}", admin_base, suffix), Some(&cookie)).await;
        assert_eq!(st, StatusCode::OK);
        assert!(!html.contains("/css/icons.css"), "dashboard must not load icons.css");
        assert!(!html.contains("class=\"fa "), "dashboard must not render FontAwesome <i> tags");
        assert!(html.contains("class=\"adm-icon"), "dashboard must render inline SVG icons");

        // All 18 tabs preserved
        for n in 1..=18 {
            assert!(html.contains(&format!("id=\"tab-btn-{}\"", n)), "missing tab-btn-{}", n);
            assert!(html.contains(&format!("id=\"tab-{}\"", n)), "missing tab-{}", n);
        }
        assert!(!html.contains("id=\"tab-19\""), "must never add tab-19");

        // Toast & confirm modal
        assert!(html.contains("id=\"ops-toast-region\" class=\"ops-toast-region\""));
        assert!(html.contains("id=\"ops-confirm-modal\""));
        assert!(html.contains("id=\"hero-role-brief\""));

        // REQ-039: all 14 institutions + target matrix
        assert!(html.contains("id=\"c-target-institutions-matrix\""));
        assert!(html.contains("data-target-institutions=\"1\""));
        assert_eq!(html.matches("name=\"c_target_inst\"").count(), 14);

        // REQ-008: custom field slots
        assert!(html.contains("id=\"content-custom-fields\""));
        assert!(html.contains("id=\"ad-custom-fields\""));
        assert!(html.contains("id=\"user-custom-fields\""));

        // REQ-012: recommendation rules inside tab-5
        assert!(html.contains("id=\"tab5-recommendations\""));
        assert!(html.contains("data-recommendation-rules=\"1\""));
        assert!(html.contains("id=\"form-recommendation-rule\""));
        assert!(html.contains("id=\"tbody-recommendation-rules\""));

        // REQ-022: ad schedule & conflict override modal
        assert!(html.contains("data-open-drawer=\"drawer-ad\""));
        assert!(html.contains("id=\"ad-start-date\""));
        assert!(html.contains("id=\"ad-end-date\""));
        assert!(html.contains("id=\"ad-rotation-mode\""));
        assert!(html.contains("id=\"ad-conflict-modal\""));

        // REQ-027: newsletter schedule, email group, status, auto-sponsor select
        assert!(html.contains("id=\"nl-email-group\""));
        assert!(html.contains("id=\"nl-scheduled-at\""));
        assert!(html.contains("id=\"nl-status\""));
        assert!(html.contains("id=\"nl-sponsor-ad-id\""));
        assert!(!html.contains("ASELSAN A.Ş."));

        // REQ-041 & REQ-019/REQ-040
        assert!(html.contains("id=\"pagination-admin-users\""));
        assert!(html.contains("id=\"iss-converter-box\""));
        assert!(html.contains("id=\"btn-iss-convert-now\""));
        assert!(html.contains("data-convert-pdf=\"1\""));
    }

    // 3. Design Studio SSR via authenticated HTTP route
    let (studio_st, studio_html) = get_html(&app, &format!("{}/tasarim", admin_base), Some(&cookie)).await;
    assert_eq!(studio_st, StatusCode::OK);
    assert!(!studio_html.contains("/css/icons.css"), "editor must not load icons.css");
    assert!(!studio_html.contains("class=\"fa "), "editor must not render FontAwesome <i> tags");
    assert!(studio_html.contains("/css/admin/tokens.css?v=35"));
    assert!(studio_html.contains("data-preview-width=\"100%\""));
    assert!(studio_html.contains("data-preview-width=\"768px\""));
    assert!(studio_html.contains("data-preview-width=\"390px\""));
    assert!(studio_html.contains("id=\"studio-confirm-modal\""));
    assert!(studio_html.contains("id=\"editor-admin-link\""));
    assert!(studio_html.contains("id=\"studio-theme-toggle\""));

    cleanup_test_db(&db_path);
}

#[test]
fn resolve_newsletter_sponsor_prefers_matching_target_over_mismatched_budget() {
    let mut mismatched_high = make_test_ad(1, "SSB HIGH BUDGET", 95, true, true, None);
    mismatched_high.target_institutions = vec!["SSB".to_string()];

    let mut wildcard_mid = make_test_ad(2, "WILDCARD ALL", 90, true, true, None);
    wildcard_mid.target_institutions = vec!["ALL".to_string()];

    let mut targeted_msb = make_test_ad(3, "MSB TARGETED", 80, true, true, None);
    targeted_msb.target_institutions = vec!["MSB".to_string()];

    let ads = vec![mismatched_high, wildcard_mid, targeted_msb];
    let chosen = resolve_newsletter_sponsor_ad(&ads, None, "MSB", "ALL", "2026-11-21")
        .expect("targeted institution ad must be selected");
    assert_eq!(chosen.id, 3, "exact institution match must outrank mismatched higher-budget and wildcard ads");

    let mut ssb_tag_match = make_test_ad(4, "RADAR TARGETED", 75, true, true, None);
    ssb_tag_match.target_institutions = vec!["ALL".to_string()];
    ssb_tag_match.target_ssb_tags = vec!["radarlar-ve-sensorler".to_string()];

    let mut wildcard_only = make_test_ad(5, "WILDCARD NO TAG", 92, true, true, None);
    wildcard_only.target_institutions = vec!["ALL".to_string()];
    wildcard_only.target_ssb_tags = vec![];

    let tag_ads = vec![wildcard_only, ssb_tag_match];
    let chosen_tag = resolve_newsletter_sponsor_ad(
        &tag_ads,
        None,
        "ALL",
        "radarlar-ve-sensorler",
        "2026-11-21",
    )
    .expect("targeted SSB tag ad must be selected");
    assert_eq!(chosen_tag.id, 4, "exact SSB tag match must outrank untargeted wildcard");
}

#[tokio::test]
async fn newsletter_draft_persists_without_dispatch_and_future_scheduled_stays_scheduled() {
    let (state, db_path) = build_test_state("m4_nl_draft_persist");
    let app = routes::build_router(state.clone());
    let cookie = make_admin_cookie(&state, "m4-nl-draft@test.local");

    // 1. Draft newsletter with no scheduled_at must persist as "draft" with recipient_count=0 and sent_at=""
    let (draft_st, draft_resp) = send_json(
        &app,
        "POST",
        "/api/v1/admin/newsletters",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "subject_tr": "M4 Taslak Bülten",
            "subject_en": "M4 Draft Newsletter",
            "target_group_filter": "all_consented",
            "target_institution": "ALL",
            "target_ssb_tag": "ALL",
            "sponsor_ad_id": null,
            "sponsor_name": null,
            "body_html": "<p>Taslak gövde</p>",
            "recipient_count": 0,
            "status": "draft",
            "scheduled_at": null,
            "sent_at": ""
        })),
    )
    .await;
    assert_eq!(draft_st, StatusCode::OK, "draft create response: {}", draft_resp);
    assert_eq!(draft_resp["data"]["status"].as_str(), Some("draft"));
    assert_eq!(draft_resp["data"]["recipient_count"].as_i64(), Some(0));
    assert!(draft_resp["data"]["scheduled_at"].is_null());
    assert!(draft_resp["data"]["dispatch"].is_null(), "draft must not dispatch");
    let draft_id = draft_resp["data"]["id"].as_i64().unwrap();

    let saved_draft = state
        .db
        .get_newsletter_campaigns()
        .into_iter()
        .find(|c| c.id == draft_id)
        .expect("saved draft in DB");
    assert_eq!(saved_draft.status, "draft");
    assert_eq!(saved_draft.sent_at, "");
    assert_eq!(saved_draft.recipient_count, 0);
    assert_eq!(saved_draft.scheduled_at, None);

    // Scheduler tick must never claim or dispatch draft campaigns
    let tick_count = run_newsletter_scheduler_tick(&state).await;
    assert_eq!(tick_count, 0, "scheduler must ignore draft campaigns");

    // 2. status="scheduled" without future scheduled_at must return 400 Bad Request
    let (bad_sched_st, bad_sched_resp) = send_json(
        &app,
        "POST",
        "/api/v1/admin/newsletters",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "subject_tr": "Eksik Tarihli Zamanlama",
            "subject_en": "Missing Schedule Date",
            "target_group_filter": "all_consented",
            "target_institution": "ALL",
            "target_ssb_tag": "ALL",
            "recipient_count": 0,
            "status": "scheduled",
            "scheduled_at": null,
            "sent_at": ""
        })),
    )
    .await;
    assert_eq!(bad_sched_st, StatusCode::BAD_REQUEST, "expected 400: {}", bad_sched_resp);

    // 3. T3-23 compatibility: status="draft" WITH future scheduled_at must become "scheduled"
    let (fut_st, fut_resp) = send_json(
        &app,
        "POST",
        "/api/v1/admin/newsletters",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "subject_tr": "Gelecek Tarihli Taslak",
            "subject_en": "Future Scheduled Draft",
            "target_group_filter": "all_consented",
            "target_institution": "ALL",
            "target_ssb_tag": "ALL",
            "recipient_count": 0,
            "status": "draft",
            "scheduled_at": "2099-01-01T10:00:00Z",
            "sent_at": ""
        })),
    )
    .await;
    assert_eq!(fut_st, StatusCode::OK, "future scheduled response: {}", fut_resp);
    assert_eq!(fut_resp["data"]["status"].as_str(), Some("scheduled"));

    cleanup_test_db(&db_path);
}

#[tokio::test]
async fn custom_field_values_end_to_end_with_admin_js_runtime() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let (state, db_path) = build_test_state("m4_cf_runtime_e2e");
    let app = routes::build_router(state.clone());
    let cookie = make_admin_cookie(&state, "m4-cf-admin@test.local");

    // 1. Seed 7 custom_fields for "content" (all 7 types) + 1 for "ad" + 1 for "user"
    let field_defs = vec![
        json!({ "field_key": "cf_text", "label_tr": "Kısa Not", "label_en": "Short Note", "field_type": "text", "options_json": "[]", "is_required": true, "sort_order": 1, "applies_to": "content" }),
        json!({ "field_key": "cf_textarea", "label_tr": "Detay Açıklama", "label_en": "Detailed Notes", "field_type": "textarea", "options_json": "[]", "is_required": false, "sort_order": 2, "applies_to": "content" }),
        json!({ "field_key": "cf_select", "label_tr": "Öncelik Sınıfı", "label_en": "Priority Tier", "field_type": "select", "options_json": "[{\"value\":\"tier_a\",\"label_tr\":\"Sınıf A\",\"label_en\":\"Tier A\"},{\"value\":\"tier_b\",\"label_tr\":\"Sınıf B\",\"label_en\":\"Tier B\"}]", "is_required": true, "sort_order": 3, "applies_to": "content" }),
        json!({ "field_key": "cf_checkbox", "label_tr": "Yayın Onaylı", "label_en": "Print Approved", "field_type": "checkbox", "options_json": "[]", "is_required": false, "sort_order": 4, "applies_to": "content" }),
        json!({ "field_key": "cf_number", "label_tr": "Sayfa Sayısı", "label_en": "Page Count", "field_type": "number", "options_json": "[]", "is_required": false, "sort_order": 5, "applies_to": "content" }),
        json!({ "field_key": "cf_date", "label_tr": "Son İnceleme Tarihi", "label_en": "Review Date", "field_type": "date", "options_json": "[]", "is_required": false, "sort_order": 6, "applies_to": "content" }),
        json!({ "field_key": "cf_url", "label_tr": "Kaynak Bağlantı", "label_en": "Source Link", "field_type": "url", "options_json": "[]", "is_required": false, "sort_order": 7, "applies_to": "content" }),
        json!({ "field_key": "cf_ad_ref", "label_tr": "Reklam Sözleşme No", "label_en": "Ad Contract No", "field_type": "text", "options_json": "[]", "is_required": false, "sort_order": 1, "applies_to": "ad" }),
        json!({ "field_key": "cf_user_dept", "label_tr": "Birim Kodu", "label_en": "Department Code", "field_type": "text", "options_json": "[]", "is_required": false, "sort_order": 1, "applies_to": "user" }),
    ];
    for def in field_defs {
        let (st, resp) = send_json(&app, "POST", "/api/v1/admin/custom-fields", Some(&cookie), Some(def)).await;
        assert_eq!(st, StatusCode::OK, "seed custom field failed: {}", resp);
    }

    // 2. Create real content, ad, and user entities via Axum router
    let (cst, cresp) = send_json(
        &app,
        "POST",
        "/api/v1/admin/contents",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "slug": "",
            "title_tr": "M4 Özel Alan Test İçeriği",
            "title_en": "M4 Custom Field Test Content",
            "summary_tr": "Özet metin",
            "summary_en": "Summary text",
            "body_tr": "<p>Gövde metni</p>",
            "body_en": "<p>Body text</p>",
            "content_type": "haberler",
            "source_group": "gunluk-haber",
            "cover_image": "images/msi/dergi-286.webp",
            "author_name": "QA Yazar",
            "author_title": "Analist",
            "issue_number": null,
            "published_at": "2026-11-21 12:00",
            "read_time_min": 4,
            "visibility_level": 0,
            "single_price_try": 0.0,
            "is_sponsored": false,
            "sponsor_brand": null,
            "sponsor_legal_label": null,
            "is_featured": false,
            "video_url": null,
            "view_count": 0,
            "true_read_count": 0,
            "placements": ["manset"],
            "ssb_tags": ["hava-araclari"],
            "target_institutions": ["MSB"],
            "status": "published"
        })),
    )
    .await;
    assert_eq!(cst, StatusCode::OK, "create content failed: {}", cresp);
    let content_id = cresp["data"]["id"].as_i64().unwrap();

    let (ast, aresp) = send_json(
        &app,
        "POST",
        "/api/v1/admin/ads",
        Some(&cookie),
        Some(json!({
            "id": 0,
            "slot_code": "ad-mag-top",
            "badge_label": "SPONSORLU",
            "dimensions": "970x90",
            "sponsor_name": "M4_CF_AD",
            "sponsor_type": "defense_oem",
            "pill_style": "blue",
            "title_tr": "M4 Reklam CF Test",
            "title_en": "M4 Ad CF Test",
            "desc_tr": "Açıklama",
            "desc_en": "Description",
            "target_url": "https://example.com",
            "budget_score": 85,
            "is_default_fallback": false,
            "is_floating_popup": false,
            "is_newsletter_ad": false,
            "is_active": true,
            "target_categories": [],
            "target_ssb_tags": [],
            "target_institutions": ["MSB"],
            "impressions_count": 0,
            "clicks_count": 0,
            "force_override": true
        })),
    )
    .await;
    assert_eq!(ast, StatusCode::OK, "create ad failed: {}", aresp);
    let ad_id = aresp["data"]["id"].as_i64().unwrap();

    let (ust, uresp) = send_json(
        &app,
        "POST",
        "/api/v1/admin/users",
        Some(&cookie),
        Some(json!({
            "email": "m4-cf-user@test.local",
            "full_name": "M4 CF User",
            "password": "UserPass123!",
            "role": "registered",
            "organization_category": "MSB",
            "organization_name": "Test Org",
            "job_title": "Engineer",
            "subscription_tier": "",
            "is_gov_verified": false
        })),
    )
    .await;
    assert_eq!(ust, StatusCode::OK, "create user failed: {}", uresp);
    let user_id = uresp["data"]["id"].as_i64().unwrap();

    // 3. Fetch real GET /api/v1/admin/custom-field-values payloads from Axum router
    let (gc_st, gc_payload) = send_json(
        &app,
        "GET",
        &format!("/api/v1/admin/custom-field-values?entity_type=content&entity_id={}", content_id),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(gc_st, StatusCode::OK);
    let (ga_st, ga_payload) = send_json(
        &app,
        "GET",
        &format!("/api/v1/admin/custom-field-values?entity_type=ad&entity_id={}", ad_id),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(ga_st, StatusCode::OK);
    let (gu_st, gu_payload) = send_json(
        &app,
        "GET",
        &format!("/api/v1/admin/custom-field-values?entity_type=user&entity_id={}", user_id),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(gu_st, StatusCode::OK);

    // 4. Execute real public/js/admin/core.js against real Axum payloads in Node.js (Phase A: initial GET -> render -> validate -> build PUT payloads)
    let phase_a_input = json!({
        "content_id": content_id,
        "ad_id": ad_id,
        "user_id": user_id,
        "get_content": gc_payload,
        "get_ad": ga_payload,
        "get_user": gu_payload
    });

    let node_script = r####"
const fs = require("fs");
const vm = require("vm");
const assert = require("assert");

const input = JSON.parse(fs.readFileSync(0, "utf8"));

function decodeEntities(s) {
  return String(s || "")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

function parseAttrs(attrStr) {
  const attrs = {};
  const re = /([a-zA-Z0-9_:-]+)(?:="([^"]*)")?/g;
  let m;
  while ((m = re.exec(attrStr)) !== null) {
    attrs[m[1]] = m[2] !== undefined ? decodeEntities(m[2]) : "";
  }
  return attrs;
}

class MiniEl {
  constructor(tag = "div", attrs = {}) {
    this.tagName = tag.toUpperCase();
    this.attrs = { ...attrs };
    this.children = [];
    this.textContent = "";
    this._innerHTML = "";
    this.dataset = {};
    this._value = attrs.value !== undefined ? attrs.value : "";
    this.checked = Object.prototype.hasOwnProperty.call(attrs, "checked");
    this.readOnly = Object.prototype.hasOwnProperty.call(attrs, "readonly");
    this.focused = false;
    this.style = { setProperty() {}, removeProperty() {} };
    this._listeners = {};
    this.classList = {
      add: (...cls) => {
        const set = new Set((this.attrs.class || "").split(/\s+/).filter(Boolean));
        cls.forEach((c) => set.add(c));
        this.attrs.class = Array.from(set).join(" ");
      },
      remove: (...cls) => {
        const drop = new Set(cls);
        this.attrs.class = (this.attrs.class || "").split(/\s+/).filter((c) => c && !drop.has(c)).join(" ");
      },
      toggle: (c, force) => {
        const has = this.classList.contains(c);
        const on = force === undefined ? !has : !!force;
        if (on) this.classList.add(c); else this.classList.remove(c);
        return on;
      },
      contains: (c) => (this.attrs.class || "").split(/\s+/).filter(Boolean).includes(c),
    };
  }
  get id() { return this.attrs.id || ""; }
  set id(v) { this.attrs.id = String(v || ""); }
  get className() { return this.attrs.class || ""; }
  set className(v) { this.attrs.class = String(v || ""); }
  get value() { return this._value; }
  set value(v) { this._value = String(v ?? ""); }
  getAttribute(k) { return Object.prototype.hasOwnProperty.call(this.attrs, k) ? this.attrs[k] : null; }
  setAttribute(k, v) { this.attrs[k] = String(v); }
  hasAttribute(k) { return Object.prototype.hasOwnProperty.call(this.attrs, k); }
  focus() { this.focused = true; }
  reset() { this._value = ""; this.checked = false; }
  addEventListener(t, fn) { (this._listeners[t] = this._listeners[t] || []).push(fn); }
  closest(sel) { return this._matches(sel) ? this : null; }
  appendChild(ch) { this.children.push(ch); return ch; }
  removeChild(ch) { this.children = this.children.filter((x) => x !== ch); }
  get innerHTML() { return this._innerHTML; }
  set innerHTML(html) {
    this._innerHTML = String(html || "");
    this.children = [];
    if (!this._innerHTML) return;
    const stack = [this];
    const tokenRe = /<(\/?)([a-zA-Z0-9_-]+)([^>]*?)(\/?)>|([^<]+)/g;
    let m;
    while ((m = tokenRe.exec(this._innerHTML)) !== null) {
      if (m[5] !== undefined) {
        const cur = stack[stack.length - 1];
        if (cur) cur.textContent += decodeEntities(m[5]);
        continue;
      }
      const isClose = m[1] === "/";
      const tag = m[2].toLowerCase();
      const rawAttrs = m[3] || "";
      const selfClose = m[4] === "/" || tag === "input" || tag === "br" || tag === "hr";
      if (isClose) {
        if (tag === "select") {
          const sel = stack[stack.length - 1];
          const chosen = sel.children.find((o) => o.hasAttribute("selected")) || sel.children[0];
          if (chosen) sel._value = chosen.attrs.value ?? "";
        }
        if (stack.length > 1) stack.pop();
      } else {
        const el = new MiniEl(tag, parseAttrs(rawAttrs));
        if (tag === "textarea") el._value = "";
        stack[stack.length - 1].children.push(el);
        if (!selfClose) stack.push(el);
      }
    }
    this.querySelectorAll("textarea").forEach((ta) => { ta._value = ta.textContent; });
  }
  _matches(sel) {
    return sel.split(",").map((s) => s.trim()).filter(Boolean).some((part) => {
      const tokens = part.match(/(\.[a-zA-Z0-9_-]+|#[a-zA-Z0-9_-]+|\[[^\]]+\]|[a-zA-Z0-9_-]+)/g) || [];
      if (!tokens.length) return false;
      return tokens.every((tk) => {
        if (tk.startsWith(".")) return this.classList.contains(tk.slice(1));
        if (tk.startsWith("#")) return this.id === tk.slice(1);
        if (tk.startsWith("[")) {
          const eq = tk.match(/^\[([a-zA-Z0-9_-]+)="([^"]*)"\]$/);
          if (eq) return this.getAttribute(eq[1]) === eq[2];
          const bare = tk.match(/^\[([a-zA-Z0-9_-]+)\]$/);
          if (bare) return this.hasAttribute(bare[1]);
          return false;
        }
        return this.tagName.toLowerCase() === tk.toLowerCase();
      });
    });
  }
  querySelectorAll(sel) {
    const out = [];
    const walk = (node) => {
      for (const ch of node.children) {
        if (ch._matches(sel)) out.push(ch);
        walk(ch);
      }
    };
    walk(this);
    return out;
  }
  querySelector(sel) { return this.querySelectorAll(sel)[0] || null; }
}

const elements = {};
for (const id of [
  "content-custom-fields", "ad-custom-fields", "user-custom-fields",
  "ops-toast-region",
  "tbody-recommendation-rules", "form-recommendation-rule",
  "rec-id", "rec-rule-name", "rec-source-tag", "rec-target-inst", "rec-content-slug", "rec-priority", "rec-is-active",
  "ad-conflict-modal", "ad-conflict-modal-msg", "ad-conflict-modal-list",
]) {
  const tag = id === "ad-conflict-modal-list" ? "ul" : "div";
  elements[id] = new MiniEl(tag, id === "ops-toast-region" ? { id, class: "ops-toast-region" } : { id });
}

const docListeners = {};
const winListeners = {};
const capturedPuts = [];
let fetchRoutes = {};

const sandbox = {
  console,
  setTimeout,
  clearTimeout,
  URL,
  Blob,
  FormData: function() {},
  CustomEvent: function(name, opts) { this.type = name; this.detail = opts?.detail; },
  requestAnimationFrame: (cb) => cb(),
  document: {
    cookie: "csrf_token=test-csrf",
    lang: "tr",
    documentElement: { lang: "tr", setAttribute() {}, getAttribute() { return "light"; } },
    body: { dataset: { adminLang: "tr", adminPath: "/podmin" }, classList: { toggle() {} }, appendChild() {}, removeChild() {} },
    getElementById: (id) => elements[id] || null,
    querySelector: (sel) => (sel && sel.includes(".app-layout") ? new MiniEl("div", { class: "app-layout" }) : sel === ".ops-toast-region" ? elements["ops-toast-region"] : null),
    querySelectorAll: () => [],
    addEventListener: (ev, fn) => { (docListeners[ev] = docListeners[ev] || []).push(fn); },
    createElement: (t) => new MiniEl(t),
  },
  window: null,
  location: { protocol: "http:", reload() {} },
  navigator: {},
  fetch: async (url, opts = {}) => {
    const method = (opts.method || "GET").toUpperCase();
    if (method === "PUT" && url === "/api/v1/admin/custom-field-values") {
      const body = JSON.parse(opts.body);
      capturedPuts.push(body);
      return { ok: true, status: 200, text: async () => JSON.stringify({ status: "ok", data: { status: "saved", updated_count: body.values.length } }) };
    }
    if (typeof fetchRoutes[url] === "function") {
      return fetchRoutes[url](url, opts);
    }
    if (url === "/api/v1/admin/me") {
      return { ok: true, status: 200, text: async () => JSON.stringify({ status: "ok", data: { email: "qa@test.local", role: "super_admin", is_wildcard: true, permissions: ["*"] } }) };
    }
    if (method === "GET") {
      return { ok: true, status: 200, text: async () => JSON.stringify({ status: "ok", data: [] }) };
    }
    throw new Error("Unexpected fetch: " + method + " " + url);
  },
};
sandbox.window = sandbox;
sandbox.window.addEventListener = (ev, fn) => { (winListeners[ev] = winListeners[ev] || []).push(fn); };
sandbox.window.dispatchEvent = () => {};

vm.createContext(sandbox);
vm.runInContext(fs.readFileSync("public/js/admin/core.js", "utf8"), sandbox);
vm.runInContext(fs.readFileSync("public/js/admin/taxonomy-media.js", "utf8"), sandbox);
vm.runInContext(fs.readFileSync("public/js/admin/commercial.js", "utf8"), sandbox);

const adm = sandbox.window.MSIAdmin;
const ctx = adm._ctx;
assert(typeof adm.loadCustomFieldValues === "function");
assert(typeof adm.validateCustomFields === "function");
assert(typeof adm.saveCustomFieldValues === "function");

(async () => {
  for (const fn of (docListeners.DOMContentLoaded || [])) {
    await fn();
  }
  // Test 1: Content 7 types load + styling + validateCustomFields + saveCustomFieldValues
  const contentUrl = `/api/v1/admin/custom-field-values?entity_type=content&entity_id=${input.content_id}`;
  const adUrl = `/api/v1/admin/custom-field-values?entity_type=ad&entity_id=${input.ad_id}`;
  const userUrl = `/api/v1/admin/custom-field-values?entity_type=user&entity_id=${input.user_id}`;
  fetchRoutes[contentUrl] = async () => ({ ok: true, status: 200, text: async () => JSON.stringify(input.get_content) });
  fetchRoutes[adUrl] = async () => ({ ok: true, status: 200, text: async () => JSON.stringify(input.get_ad) });
  fetchRoutes[userUrl] = async () => ({ ok: true, status: 200, text: async () => JSON.stringify(input.get_user) });

  const loadedContent = await adm.loadCustomFieldValues("content-custom-fields", "content", String(input.content_id));
  assert.strictEqual(loadedContent.length, 10, "must return 7 test + 3 seeded content custom fields");
  const cBox = elements["content-custom-fields"];
  assert(cBox.querySelector(".custom-fields-heading"), "must render .custom-fields-heading");
  assert(cBox.querySelector(".custom-fields-grid"), "must render .custom-fields-grid");
  assert.strictEqual(cBox.querySelectorAll(".cf-input").length, 0, "must have 0 .cf-input classes");
  assert.strictEqual(cBox.querySelectorAll(".checkbox-inline").length, 0, "must have 0 .checkbox-inline classes");
  assert.strictEqual(cBox.querySelectorAll(".custom-fields-empty").length, 0, "must not render empty notice");
  assert.strictEqual(cBox.querySelectorAll("[data-cf-field-id]").length, 10, "must render all 10 [data-cf-field-id] controls");
  assert(cBox.querySelector("input.form-control[data-cf-type=\"text\"]"), "must render .form-control text input");
  assert(cBox.querySelector("textarea.form-textarea[data-cf-type=\"textarea\"]"), "must render .form-textarea");
  assert(cBox.querySelector("select.form-select[data-cf-type=\"select\"]"), "must render .form-select");
  assert(cBox.querySelector("label.check-pill")?.querySelector('input[data-cf-type="checkbox"]'), "must render .check-pill checkbox");
  assert(cBox.querySelector("input.form-control[data-cf-type=\"number\"]"), "must render number input");
  assert(cBox.querySelector("input.form-control[data-cf-type=\"date\"]"), "must render date input");
  assert(cBox.querySelector("input.form-control[data-cf-type=\"url\"]"), "must render url input");

  // Required validation must fail before filling required fields
  assert.strictEqual(adm.validateCustomFields("content-custom-fields"), false, "validateCustomFields must fail when required fields are empty");
  const toastBody = elements["ops-toast-region"].querySelector(".ops-toast")?.querySelector(".ops-toast-body");
  assert(toastBody && toastBody.textContent.length > 0, "validateCustomFields failure must trigger toast");

  // Fill all 7 controls
  for (const el of cBox.querySelectorAll("[data-cf-field-id]")) {
    const t = el.getAttribute("data-cf-type");
    if (t === "text") el.value = "Kısa test notu";
    else if (t === "textarea") el.value = "Detaylı test açıklaması";
    else if (t === "select") el.value = "tier_b";
    else if (t === "checkbox") el.checked = true;
    else if (t === "number") el.value = "42";
    else if (t === "date") el.value = "2026-11-21";
    else if (t === "url") el.value = "https://www.msidergisi.com/rapor";
  }
  assert.strictEqual(adm.validateCustomFields("content-custom-fields"), true, "validateCustomFields must pass once required fields are filled");

  const saveContentRes = await adm.saveCustomFieldValues("content-custom-fields", "content", String(input.content_id));
  assert.strictEqual(saveContentRes.ok, true);

  // Test 2: Ad & User load + save
  await adm.loadCustomFieldValues("ad-custom-fields", "ad", String(input.ad_id));
  const adInput = elements["ad-custom-fields"].querySelector("[data-cf-field-id]");
  assert(adInput, "ad custom field input must render");
  adInput.value = "AD-2026-009";
  await adm.saveCustomFieldValues("ad-custom-fields", "ad", String(input.ad_id));

  await adm.loadCustomFieldValues("user-custom-fields", "user", String(input.user_id));
  const userInput = elements["user-custom-fields"].querySelector("[data-cf-field-id]");
  assert(userInput, "user custom field input must render");
  userInput.value = "ARGE-01";
  await adm.saveCustomFieldValues("user-custom-fields", "user", String(input.user_id));

  // Test 3: HTTP 500 resilience (no TypeError)
  fetchRoutes[adUrl] = async () => ({ ok: false, status: 500, text: async () => JSON.stringify({ status: "error", message: "DB offline" }) });
  const errItems = await adm.loadCustomFieldValues("ad-custom-fields", "ad", String(input.ad_id));
  assert.strictEqual(JSON.stringify(errItems), "[]", "HTTP 500 must resolve cleanly to []");
  assert(elements["ad-custom-fields"].querySelector(".custom-fields-empty.text-danger"), "HTTP 500 must render .custom-fields-empty.text-danger");

  // Test 4: Monotonic race guard (slow stale response vs fast newer response)
  const slowUrl = "/api/v1/admin/custom-field-values?entity_type=ad&entity_id=101";
  const fastUrl = "/api/v1/admin/custom-field-values?entity_type=ad&entity_id=102";
  fetchRoutes[slowUrl] = () => new Promise((resolve) => setTimeout(() => resolve({ ok: true, status: 200, text: async () => JSON.stringify({ status: "ok", data: { values: [{ field_id: 99, field_key: "stale", label_tr: "Bayat", field_type: "text", value_text: "STALE" }] } }) }), 40));
  fetchRoutes[fastUrl] = async () => ({ ok: true, status: 200, text: async () => JSON.stringify({ status: "ok", data: { values: [{ field_id: 88, field_key: "fresh", label_tr: "Güncel", field_type: "text", value_text: "FRESH" }] } }) });
  const pSlow = adm.loadCustomFieldValues("ad-custom-fields", "ad", "101");
  const pFast = adm.loadCustomFieldValues("ad-custom-fields", "ad", "102");
  await Promise.all([pSlow, pFast]);
  const finalInputs = elements["ad-custom-fields"].querySelectorAll("[data-cf-field-id]");
  assert.strictEqual(finalInputs.length, 1);
  assert.strictEqual(finalInputs[0].getAttribute("data-cf-field-id"), "88", "stale response must not overwrite newer response");

  // Test 5: taxonomy-media.js REQ-012 "*" default & "ALL" normalization
  elements["rec-target-inst"].value = "ALL";
  const recResetBtn = new MiniEl("button", { "data-admin-action": "reset-rec-rule-form" });
  for (const fn of (docListeners.click || [])) fn({ target: recResetBtn, preventDefault() {} });
  assert.strictEqual(elements["rec-target-inst"].value, "*", "resetRecRuleForm must set rec-target-inst to '*'");

  ctx.state.recommendationRules = [{ id: 10, rule_name: "Kural 1", source_tag_or_cat: "*", target_institution: "ALL", recommended_content_slug: "test-slug", priority: 10, is_active: true }];
  const recEditBtn = new MiniEl("button", { "data-admin-action": "edit-rec-rule", "data-id": "10" });
  for (const fn of (docListeners.click || [])) fn({ target: recEditBtn, preventDefault() {} });
  assert.strictEqual(elements["rec-target-inst"].value, "*", "loadRecRuleIntoForm must normalize 'ALL' to '*'");

  let recPostBody = null;
  fetchRoutes["/api/v1/admin/recommendation-rules/10"] = async (_u, opts) => {
    recPostBody = JSON.parse(opts.body);
    return { ok: true, status: 200, text: async () => JSON.stringify({ status: "ok", data: { id: 10 } }) };
  };
  fetchRoutes["/api/v1/admin/recommendation-rules"] = async () => ({
    ok: true, status: 200, text: async () => JSON.stringify({ status: "ok", data: { rules: [] } })
  });
  elements["rec-target-inst"].value = "";
  const recForm = new MiniEl("form", { id: "form-recommendation-rule" });
  for (const fn of (docListeners.submit || [])) await fn({ target: recForm, preventDefault() {} });
  assert(recPostBody && recPostBody.target_institution === "*", "recommendation rule submit fallback must be '*'");

  // Test 6: commercial.js REQ-022 409 conflict list renders direct <li> without nested <ul>
  fetchRoutes["/api/v1/admin/ads"] = async () => ({
    ok: false,
    status: 409,
    text: async () => JSON.stringify({
      status: "error",
      error_code: "CONFLICT",
      error: "Kota dolu",
      message: "Kota dolu",
      conflict_check: { has_conflict: true, message_tr: "Kota dolu", conflicting_campaigns: ["Kampanya A", "Kampanya B"] },
    }),
  });
  elements["ad-conflict-modal-list"].innerHTML = "";
  elements["ad-custom-fields"].innerHTML = "";
  // Trigger confirm-ad-override after setting pending via submitAdPayload isn't exposed directly; submit form-ad-campaign
  const formAd = new MiniEl("form", { id: "form-ad-campaign" });
  elements["ad-sponsor-name"] = new MiniEl("input", { id: "ad-sponsor-name" });
  elements["ad-sponsor-name"].value = "Test Sponsor";
  elements["ad-title-tr"] = new MiniEl("input", { id: "ad-title-tr" });
  elements["ad-title-tr"].value = "Test Title";
  for (const fn of (docListeners.submit || [])) {
    await fn({ target: formAd, preventDefault() {} });
  }
  const conflictHtml = elements["ad-conflict-modal-list"].innerHTML;
  assert(!conflictHtml.includes("<ul"), "conflict list must not nest <ul> inside #ad-conflict-modal-list: " + conflictHtml);
  assert(conflictHtml.includes("<li>Kampanya A</li><li>Kampanya B</li>"), "conflict list must render direct <li> items: " + conflictHtml);

  process.stdout.write(JSON.stringify({ puts: capturedPuts }));
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
"####;

    let mut child = Command::new("node")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn node for Phase A");
    {
        let mut stdin = child.stdin.take().unwrap();
        let combined = format!(
            "const __TEST_STDIN__ = {};\n{}",
            phase_a_input,
            node_script.replace(
                "JSON.parse(fs.readFileSync(0, \"utf8\"))",
                "__TEST_STDIN__"
            )
        );
        stdin.write_all(combined.as_bytes()).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "Node Phase A failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let parsed_a: Value = serde_json::from_slice(&out.stdout).expect("parse Node Phase A output");
    let puts = parsed_a["puts"].as_array().expect("puts array");
    assert_eq!(puts.len(), 3, "expected 3 PUT payloads (content, ad, user)");

    // 5. Send the real JS-generated PUT payloads to Axum router
    for put_body in puts {
        let (pst, presp) = send_json(
            &app,
            "PUT",
            "/api/v1/admin/custom-field-values",
            Some(&cookie),
            Some(put_body.clone()),
        )
        .await;
        assert_eq!(pst, StatusCode::OK, "PUT custom-field-values failed: {}", presp);
    }

    // 6. Re-fetch GET /api/v1/admin/custom-field-values from Axum and verify round-trip in core.js
    let (gc2_st, gc2_payload) = send_json(
        &app,
        "GET",
        &format!("/api/v1/admin/custom-field-values?entity_type=content&entity_id={}", content_id),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(gc2_st, StatusCode::OK);
    let (ga2_st, ga2_payload) = send_json(
        &app,
        "GET",
        &format!("/api/v1/admin/custom-field-values?entity_type=ad&entity_id={}", ad_id),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(ga2_st, StatusCode::OK);
    let (gu2_st, gu2_payload) = send_json(
        &app,
        "GET",
        &format!("/api/v1/admin/custom-field-values?entity_type=user&entity_id={}", user_id),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(gu2_st, StatusCode::OK);

    let phase_b_input = json!({
        "content_id": content_id,
        "ad_id": ad_id,
        "user_id": user_id,
        "get_content": gc2_payload,
        "get_ad": ga2_payload,
        "get_user": gu2_payload
    });
    let phase_b_script = format!(
        r#"
const fs = require("fs");
const vm = require("vm");
const assert = require("assert");
const input = {};
const boxMap = {{ "content-custom-fields": {{ innerHTML: "", dataset: {{}} }}, "ad-custom-fields": {{ innerHTML: "", dataset: {{}} }}, "user-custom-fields": {{ innerHTML: "", dataset: {{}} }} }};
const routes = {{
  [`/api/v1/admin/custom-field-values?entity_type=content&entity_id=${{input.content_id}}`]: input.get_content,
  [`/api/v1/admin/custom-field-values?entity_type=ad&entity_id=${{input.ad_id}}`]: input.get_ad,
  [`/api/v1/admin/custom-field-values?entity_type=user&entity_id=${{input.user_id}}`]: input.get_user,
}};
const sandbox = {{
  console, setTimeout, clearTimeout, URL, Blob, FormData: function(){{}}, CustomEvent: function(){{}}, requestAnimationFrame: (c) => c(),
  document: {{ cookie: "", documentElement: {{ setAttribute(){{}}, getAttribute(){{return "light";}} }}, body: {{ dataset: {{ adminLang: "tr" }}, classList: {{ toggle(){{}} }} }}, getElementById: (id) => boxMap[id] || null, querySelector: () => null, querySelectorAll: () => [], addEventListener(){{}} }},
  window: null, location: {{ protocol: "http:" }}, navigator: {{}},
  fetch: async (u) => ({{ ok: true, status: 200, text: async () => JSON.stringify(routes[u]) }}),
}};
sandbox.window = sandbox;
sandbox.window.addEventListener = () => {{}};
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync("public/js/admin/core.js", "utf8"), sandbox);
(async () => {{
  await sandbox.window.MSIAdmin.loadCustomFieldValues("content-custom-fields", "content", String(input.content_id));
  const cHtml = boxMap["content-custom-fields"].innerHTML;
  assert(cHtml.includes('value="Kısa test notu"'), "persisted text value missing: " + cHtml);
  assert(cHtml.includes(">Detaylı test açıklaması</textarea>"), "persisted textarea value missing: " + cHtml);
  assert(cHtml.includes('<option value="tier_b" selected>'), "persisted select option missing: " + cHtml);
  assert(cHtml.includes('data-cf-type="checkbox"') && cHtml.includes(' checked>'), "persisted checkbox checked missing: " + cHtml);
  assert(cHtml.includes('value="42"'), "persisted number value missing: " + cHtml);
  assert(cHtml.includes('value="2026-11-21"'), "persisted date value missing: " + cHtml);
  assert(cHtml.includes('value="https://www.msidergisi.com/rapor"'), "persisted url value missing: " + cHtml);

  await sandbox.window.MSIAdmin.loadCustomFieldValues("ad-custom-fields", "ad", String(input.ad_id));
  assert(boxMap["ad-custom-fields"].innerHTML.includes('value="AD-2026-009"'));

  await sandbox.window.MSIAdmin.loadCustomFieldValues("user-custom-fields", "user", String(input.user_id));
  assert(boxMap["user-custom-fields"].innerHTML.includes('value="ARGE-01"'));
}})().catch((e) => {{ console.error(e); process.exit(1); }});
"#,
        phase_b_input
    );

    let mut child_b = Command::new("node")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn node for Phase B");
    {
        let mut stdin = child_b.stdin.take().unwrap();
        stdin.write_all(phase_b_script.as_bytes()).unwrap();
    }
    let out_b = child_b.wait_with_output().unwrap();
    assert!(
        out_b.status.success(),
        "Node Phase B roundtrip failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&out_b.stdout),
        String::from_utf8_lossy(&out_b.stderr)
    );

    cleanup_test_db(&db_path);
}
