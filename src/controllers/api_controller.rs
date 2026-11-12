use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    Form, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path as FsPath;

use crate::controllers::AppState;
use crate::engine::api_response::{self, ApiResponse, AppError, error_codes, msg_keys};
use crate::models::{
    AdCampaign, AnalyticsEvent, CategoryNode, ContentItem, CorporateVerificationRequest,
    NewsletterCampaign, SeoRedirectRule, SsbInterestTag, UserAccount,
};
use crate::engine::validation_engine;
use crate::services::{
    ad_service, analytics_service, auth_service, content_service, edergi_pdf_service,
    email_service, media_service, payment_service, subscription_service,
};

/// Centralized JSON error response helper.
/// Logs internal details server-side (for 5xx) and returns a generic translated message.
fn json_error(state: &AppState, status: StatusCode, msg_key: &'static str) -> Response {
    let error_code = match status {
        StatusCode::NOT_FOUND => error_codes::NOT_FOUND,
        StatusCode::UNAUTHORIZED => error_codes::UNAUTHORIZED,
        StatusCode::FORBIDDEN => error_codes::FORBIDDEN,
        StatusCode::CONFLICT => error_codes::CONFLICT,
        StatusCode::INTERNAL_SERVER_ERROR => error_codes::INTERNAL_ERROR,
        _ => error_codes::BAD_REQUEST,
    };
    ApiResponse::error(&state.translations, "tr", status, error_code, msg_key).into_response()
}

fn extract_client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("127.0.0.1")
        .trim()
        .to_string()
}

pub async fn health_check(State(state): State<AppState>, headers: HeaderMap) -> Json<Value> {
    let contents = state.db.get_all_contents();
    let categories = state.db.get_all_categories();
    let ssb_tags = state.db.get_all_ssb_tags();
    let issues = state.db.get_all_issues();
    let ads = state.db.get_all_ads();

    // SECURITY FIX (C2): Removed x-msi-session header fallback.
    // Authentication is now exclusively via HttpOnly msi_session cookie to prevent
    // XSS-based session theft. Non-browser API clients should use a separate Bearer scheme.
    // SECURITY FIX (H5): Removed active_session_user from health_check response to prevent
    // user enumeration and email address leakage via public endpoint.
    let _session_exists = auth_service::session_user(&state, &headers).is_some();

    Json(json!({
        "status": "ok",
        "platform": state.config.app_name,
        "env": state.config.app_env,
        "public_url": state.config.public_url,
        "authenticated": _session_exists,
        "engine": "Rust Axum Fullstack Engine v2.6",
        "domains": {
            "primary": state.config.primary_domain,
            "news_portal": state.config.news_domain,
            "bilingual_locales": ["tr", "en"]
        },
        "counts": {
            "contents": contents.len(),
            "categories": categories.len(),
            "ssb_tags": ssb_tags.len(),
            "magazine_issues": issues.len(),
            "ad_campaigns": ads.len()
        }
    }))
}

pub async fn list_contents(
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<ContentItem>> {
    // Unified query via CentralContentQueryEngine — replaces all scattered search/filter logic.
    let criteria = crate::engine::content_query_engine::CentralContentQueryEngine::from_params(&params);
    let query_result = crate::engine::content_query_engine::CentralContentQueryEngine::execute(&state.db, &criteria);
    let mut items = query_result.items;

    // Admin CMS endpoint keeps full bodies; public endpoint enforces 3-tier paywall redaction.
    if !uri.path().starts_with("/api/v1/admin/") {
        let user_opt = auth_service::session_user(&state, &headers);
        let purchased = user_opt
            .as_ref()
            .map(|u| state.db.get_user_purchased_codes(&u.email))
            .unwrap_or_default();

        for item in &mut items {
            let (_, is_locked, _) =
                content_service::apply_paywall_view(item, user_opt.as_ref(), &purchased, "tr");
            if is_locked {
                item.body_tr = content_service::truncate_to_two_sentences_html(&item.body_tr);
                item.body_en = content_service::truncate_to_two_sentences_html(&item.body_en);
            }
        }
    }

    Json(items)
}

pub async fn get_content_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let lang = params.get("lang").map(|s| s.as_str()).unwrap_or("tr");
    let Some(mut item) = state.db.get_content_by_slug(&slug) else {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            msg_keys::CONTENT_NOT_FOUND,
        )
        .into_response();
    };

    let user_opt = auth_service::session_user(&state, &headers);
    let purchased = user_opt
        .as_ref()
        .map(|u| state.db.get_user_purchased_codes(&u.email))
        .unwrap_or_default();

    let (rendered_body, is_locked, lock_reason) =
        content_service::apply_paywall_view(&item, user_opt.as_ref(), &purchased, lang);

    if is_locked {
        item.body_tr = content_service::truncate_to_two_sentences_html(&item.body_tr);
        item.body_en = content_service::truncate_to_two_sentences_html(&item.body_en);
    }

    let all_contents = state.db.get_all_contents();
    let rules = state.db.get_recommendation_rules();
    let related = content_service::select_related_articles(
        &item,
        &all_contents,
        &rules,
        user_opt.as_ref(),
        3,
    );

    let all_ads = state.db.get_all_ads();
    let context_ads = ad_service::select_ads_for_context(
        &all_ads,
        Some(&item.content_type),
        Some(&item),
        user_opt.as_ref(),
    );

    Json(json!({
        "content": item,
        "paywall": {
            "is_locked": is_locked,
            "lock_reason": lock_reason,
            "rendered_body": rendered_body,
            "competitor_protection_active": item.is_sponsored
        },
        "related_articles": related,
        "context_ads": context_ads
    }))
    .into_response()
}

pub async fn list_categories(State(state): State<AppState>) -> Json<Vec<CategoryNode>> {
    Json(state.db.get_all_categories())
}

pub async fn list_ssb_tags(State(state): State<AppState>) -> Json<Vec<SsbInterestTag>> {
    Json(state.db.get_all_ssb_tags())
}

pub async fn list_issues(State(state): State<AppState>) -> Json<Value> {
    Json(json!(state.db.get_all_issues()))
}

pub async fn list_ads(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<AdCampaign>> {
    let all_ads = state.db.get_all_ads();
    let cat = params.get("category").map(|s| s.as_str());
    let content_opt = params
        .get("content_slug")
        .and_then(|slug| state.db.get_content_by_slug(slug));
    let user_opt = auth_service::session_user(&state, &headers);

    if cat.is_some() || content_opt.is_some() || user_opt.is_some() {
        Json(ad_service::select_ads_for_context(
            &all_ads,
            cat,
            content_opt.as_ref(),
            user_opt.as_ref(),
        ))
    } else {
        Json(all_ads)
    }
}

pub async fn download_edergi_pdf(
    State(state): State<AppState>,
    Path(issue_no): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let accepts_html = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/html"))
        .unwrap_or(false);

    let lang = api_response::resolve_lang(&params);
    let Some(user) = auth_service::session_user(&state, &headers) else {
        if accepts_html {
            return Redirect::to("/hesabim?next=/e-dergi").into_response();
        }
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::UNAUTHORIZED,
            error_codes::UNAUTHORIZED,
            msg_keys::AUTH_REQUIRED_PDF,
        )
        .into_response();
    };

    if !content_service::user_has_subscriber_access(&user) {
        if accepts_html {
            return Redirect::to("/abonelik-ve-satis").into_response();
        }
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::FORBIDDEN,
            error_codes::FORBIDDEN,
            msg_keys::SUBSCRIBER_ACCESS_REQUIRED,
        )
        .into_response();
    }

    let Some(issue) = state.db.get_issue_by_number(issue_no) else {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            msg_keys::ISSUE_NOT_FOUND,
        )
        .into_response();
    };

    let pages = params.get("pages").map(|s| s.as_str());
    let client_ip = extract_client_ip(&headers);
    let license_label = if user.is_gov_verified {
        "KAMU PROTOKOL & KURUMSAL E-DERGI LISANSI"
    } else {
        user.subscription_tier.as_str()
    };

    let pdf_bytes = edergi_pdf_service::generate_watermarked_pdf(
        &issue,
        &user.full_name,
        &user.organization_name,
        license_label,
        &client_ip,
        pages,
        &state.config.watermark_secret,
    );

    let page_suffix = pages.unwrap_or("all").replace(['/', '\\', ' '], "_");
    let filename = format!(
        "MSI_Dergisi_Sayi_{}_Watermark_{}.pdf",
        issue.issue_number, page_suffix
    );

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{}\"", filename),
            ),
            (
                header::HeaderName::from_static("x-msi-drm-watermark"),
                format!("VERIFIED-{}-{}", issue.issue_number, client_ip),
            ),
        ],
        pdf_bytes,
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct RegisterPayload {
    pub email: String,
    pub full_name: String,
    pub password: Option<String>,
    pub organization_category: Option<String>,
    pub organization_name: Option<String>,
    pub job_title: Option<String>,
    pub request_message: Option<String>,
    pub kvkk_acknowledged: Option<bool>,
    pub kvkk_consent: Option<bool>,
    pub email_consent: Option<bool>,
    pub ssb_interests: Option<Vec<String>>,
}

pub async fn register_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterPayload>,
) -> Response {
    let lang = "tr"; // Default for registration; could extract from headers later
    let email = match validation_engine::validate_email(&payload.email) {
        validation_engine::EmailValidation::Valid(e) => e,
        _ => return ApiResponse::error(&state.translations, lang, StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::INVALID_EMAIL).into_response(),
    };

    if state.db.get_user_by_email(&email).is_some() {
        return ApiResponse::error(&state.translations, lang, StatusCode::CONFLICT, error_codes::CONFLICT, msg_keys::EMAIL_ALREADY_EXISTS).into_response();
    }
    let raw_pw = match validation_engine::validate_password(payload.password.as_deref()) {
        validation_engine::PasswordValidation::Valid(pw) => pw,
        _ => return ApiResponse::error(&state.translations, lang, StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::REGISTRATION_FIELDS_REQUIRED).into_response(),
    };
    if matches!(validation_engine::validate_required_text(&payload.full_name), validation_engine::TextFieldValidation::Empty)
        || payload.kvkk_consent != Some(true)
        || payload.kvkk_acknowledged == Some(false)
    {
        return ApiResponse::error(&state.translations, lang, StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::KVKK_CONSENT_REQUIRED).into_response();
    }
    let org_category = validation_engine::with_default(
        payload.organization_category.as_deref(),
        "Sektör Kuruluşları",
    );
    let org_name = validation_engine::with_default(
        payload.organization_name.as_deref(),
        &org_category,
    );
    let job_title = validation_engine::with_default(
        payload.job_title.as_deref(),
        "Savunma Sanayii Profesyoneli",
    );

    let (role, subscription_tier, is_gov_verified) =
        auth_service::determine_initial_role_and_tier(&email, &org_category);

    let now = chrono::Utc::now();
    let now_iso = now.to_rfc3339();
    let client_ip = extract_client_ip(&headers);
    let kvkk_hash = auth_service::generate_kvkk_consent_hash(
        &email,
        &client_ip,
        &now_iso,
        &state.config.session_secret,
    );
    let pw_hash = auth_service::hash_password(&raw_pw, &state.config.session_secret);

    let sub_expires = if is_gov_verified {
        Some((now + chrono::Duration::days(365)).format("%Y-%m-%d").to_string())
    } else {
        None
    };

    let mut user = UserAccount {
        id: 0,
        email: email.clone(),
        full_name: payload.full_name.trim().to_string(),
        role,
        organization_category: org_category.clone(),
        organization_name: org_name.clone(),
        job_title,
        is_gov_verified,
        subscription_tier,
        subscription_expires_at: sub_expires,
        reminders_halted: false,
        kvkk_consent_at: now_iso.clone(),
        kvkk_consent_ip: client_ip,
        kvkk_consent_hash: kvkk_hash,
        email_consent: payload.email_consent.unwrap_or(false),
        ssb_interests: payload.ssb_interests.unwrap_or_default(),
        created_at: now_iso[..10].to_string(),
    };

    let uid = match state.db.create_or_update_user(&user, &pw_hash) {
        Ok(id) => id,
        Err(e) => {
            return ApiResponse::server_error_with_key(&state.translations, lang, "register_user", &e, msg_keys::SERVER_ERROR_ORDER).into_response();
        }
    };
    user.id = uid;

    let req_msg = payload.request_message.unwrap_or_default();
    let mut verification_queued = false;
    if !req_msg.trim().is_empty()
        || auth_service::requires_corporate_verification(&email, &org_category)
    {
        let msg = if req_msg.trim().is_empty() {
            format!(
                "{} ({}) kurumu için harici e-posta ile ücretsiz kurumsal abonelik değerlendirme talebi.",
                org_name, org_category
            )
        } else {
            req_msg
        };
        let _ = state
            .db
            .insert_corporate_request(&CorporateVerificationRequest {
                id: 0,
                user_id: uid,
                email: email.clone(),
                full_name: user.full_name.clone(),
                organization_category: org_category,
                organization_name: org_name,
                request_message: msg,
                status: "pending".to_string(),
                reviewed_by: None,
                created_at: now.format("%Y-%m-%d %H:%M").to_string(),
            });
        verification_queued = true;
    }

    let token = auth_service::generate_session_token(&user, &state.config.session_secret);

    // SECURITY FIX (H11): Removed session_token from response body.
    // Session is established exclusively via HttpOnly Set-Cookie header below.
    let secure = cookie_secure_flag(&headers, &state.config.public_url);
    let cookie = format!(
        "msi_session={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=86400{}",
        token, secure
    );
    (
        [(header::SET_COOKIE, cookie)],
        Json(json!({
            "status": "registered",
            "user": user,
            "gov_auto_subscribed": is_gov_verified,
            "corporate_verification_queued": verification_queued
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
}

fn cookie_secure_flag(headers: &HeaderMap, public_url: &str) -> &'static str {
    let forwarded_https = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false);
    let is_loopback_http = !forwarded_https
        && headers
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .map(|h| h.starts_with("127.0.0.1") || h.starts_with("localhost"))
            .unwrap_or(false);
    if forwarded_https || (public_url.starts_with("https://") && !is_loopback_http) {
        "; Secure"
    } else {
        ""
    }
}

pub async fn login_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginPayload>,
) -> Response {
    if let Some((user, pw_hash)) = state.db.get_user_by_email(&payload.email) {
        if auth_service::verify_password(&payload.password, &state.config.session_secret, &pw_hash) {
            if !auth_service::is_argon2_hash(&pw_hash) {
                let upgraded =
                    auth_service::hash_password(&payload.password, &state.config.session_secret);
                let _ = state.db.update_user_password_hash(&user.email, &upgraded);
            }

            let token = auth_service::generate_session_token(&user, &state.config.session_secret);
            let secure = cookie_secure_flag(&headers, &state.config.public_url);
            let cookie = format!(
                "msi_session={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=86400{}",
                token, secure
            );
            // SECURITY FIX (H11): Removed session_token from response body.
            // Session is established exclusively via HttpOnly Set-Cookie header.
            return (
                [(header::SET_COOKIE, cookie)],
                Json(json!({"status":"authenticated","user":user})),
            )
                .into_response();
        }
    }
    ApiResponse::error(&state.translations, "tr", StatusCode::UNAUTHORIZED, error_codes::UNAUTHORIZED, msg_keys::LOGIN_FAILED).into_response()
}

pub async fn track_analytics_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut ev): Json<AnalyticsEvent>,
) -> Json<Value> {
    if ev.created_at.is_empty() {
        ev.created_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }
    if let Some(session_u) = auth_service::session_user(&state, &headers) {
        ev.user_id = Some(session_u.id);
        ev.organization_category = session_u.organization_category;
        ev.organization_name = session_u.organization_name;
    } else {
        if ev.organization_category.trim().is_empty() {
            ev.organization_category = "Anonim Okur".to_string();
        }
        if ev.organization_name.trim().is_empty() {
            ev.organization_name = "Doğrulanmamış Ziyaretçi".to_string();
        }
    }
    if ev.duration_sec >= 25 && ev.scroll_depth_pct >= 70 {
        ev.is_true_read = true;
    }
    let id = state.db.insert_analytics_event(&ev).unwrap_or(0);
    Json(json!({
        "status": "recorded",
        "event_id": id,
        "is_true_read": ev.is_true_read
    }))
}

pub async fn get_commerce_catalog(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "subscription_plans": state.db.get_subscription_plans(),
        "commerce_products": state.db.get_commerce_products()
    }))
}

#[derive(Deserialize)]
pub struct CheckoutPayload {
    #[serde(alias = "email")]
    pub user_email: String,
    pub full_name: String,
    pub organization_name: Option<String>,
    pub item_type: String,
    pub item_code: String,
    pub item_title: Option<String>,
    pub amount_try: Option<f64>,
    pub user_phone: Option<String>,
    pub user_address: Option<String>,
    pub distance_sales_consent: Option<bool>,
}

/// Resolve authoritative price & title strictly from SQLite catalog (never trusting client `amount_try`).
fn resolve_catalog_item(
    state: &AppState,
    item_type: &str,
    item_code: &str,
) -> Option<(String, String, f64)> {
    let code = item_code.trim();
    if item_type == "subscription" {
        if let Some(plan) = state
            .db
            .get_subscription_plans()
            .into_iter()
            .find(|p| p.code.eq_ignore_ascii_case(code))
        {
            return Some(("subscription".to_string(), plan.name_tr, plan.price_try));
        }
    }
    if let Some(prod) = state
        .db
        .get_commerce_products()
        .into_iter()
        .find(|p| p.sku.eq_ignore_ascii_case(code) && p.is_available)
    {
        return Some(("product".to_string(), prod.title_tr, prod.price_try));
    }
    if let Some(article) = state
        .db
        .get_content_by_slug(code)
        .filter(|c| c.single_price_try > 0.0)
    {
        return Some((
            "single_article".to_string(),
            article.title_tr,
            article.single_price_try,
        ));
    }
    None
}

/// Initialize a verified PayTR iFrame checkout session or record an official pending proforma order.
pub async fn checkout_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CheckoutPayload>,
) -> Response {
    let lang = "tr";
    let email = match validation_engine::validate_email(&payload.user_email) {
        validation_engine::EmailValidation::Valid(e) => e,
        _ => return ApiResponse::error(&state.translations, lang, StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::CHECKOUT_FIELDS_REQUIRED).into_response(),
    };
    if matches!(validation_engine::validate_required_text(&payload.full_name), validation_engine::TextFieldValidation::Empty) {
        return ApiResponse::error(&state.translations, lang, StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::CHECKOUT_FIELDS_REQUIRED).into_response();
    }
    if payload.distance_sales_consent == Some(false) {
        return ApiResponse::error(&state.translations, lang, StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::DISTANCE_SALES_CONSENT_REQUIRED).into_response();
    }

    let _ = (&payload.item_title, payload.amount_try);
    let Some((canonical_type, canonical_title, authoritative_price)) =
        resolve_catalog_item(&state, &payload.item_type, &payload.item_code)
    else {
        return ApiResponse::error(&state.translations, lang, StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::PLAN_NOT_FOUND).into_response();
    };

    let org = payload
        .organization_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Kurumsal Savunma Paydaşı".to_string());

    let mut invoice = subscription_service::create_invoice_record(
        &email,
        payload.full_name.trim(),
        &org,
        &canonical_type,
        payload.item_code.trim(),
        &canonical_title,
        authoritative_price,
        &state.config.watermark_secret,
    );

    let paytr_config = payment_service::PayTrConfig::from_app_config(&state.config);
    if !paytr_config.is_configured() {
        invoice.payment_status = "pending_bank_transfer".to_string();
    }

    let inv_id = match state.db.insert_order_invoice(&invoice) {
        Ok(id) => id,
        Err(e) => {
            return ApiResponse::server_error_with_key(&state.translations, lang, "checkout_order", &e, msg_keys::SERVER_ERROR_ORDER).into_response();
        }
    };

    if paytr_config.is_configured() {
        let client_ip = extract_client_ip(&headers);
        let merchant_oid = payment_service::generate_merchant_oid(&invoice.invoice_no);
        let public_url = &state.config.public_url;
        let ok_url = format!("{}/hesabim?payment=success&oid={}", public_url, merchant_oid);
        let fail_url = format!(
            "{}/abonelik-ve-satis?payment=failed&oid={}",
            public_url, merchant_oid
        );

        let checkout_req = payment_service::PayTrCheckoutRequest {
            user_email: email,
            full_name: payload.full_name.trim().to_string(),
            user_phone: payload
                .user_phone
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "+903122254173".to_string()),
            user_address: payload
                .user_address
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Ankara, Türkiye".to_string()),
            user_ip: client_ip,
            merchant_oid: merchant_oid.clone(),
            amount_try: authoritative_price,
            item_title: canonical_title,
            item_code: payload.item_code.trim().to_string(),
            currency: "TL".to_string(),
            lang: "tr".to_string(),
        };

        return match payment_service::create_checkout_session(
            &paytr_config,
            &checkout_req,
            &ok_url,
            &fail_url,
        )
        .await
        {
            Ok(session) => Json(json!({
                "status": "paytr_checkout_ready",
                "iframe_token": session.iframe_token,
                "merchant_oid": session.merchant_oid,
                "amount_kurus": session.amount_kurus,
                "invoice_id": inv_id,
                "invoice_no": invoice.invoice_no,
                "test_mode": paytr_config.test_mode
            }))
            .into_response(),
            Err(e) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": e,
                    "invoice_no": invoice.invoice_no
                })),
            )
                .into_response(),
        };
    }

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "status": "order_recorded_pending_payment",
            "invoice_id": inv_id,
            "invoice_no": invoice.invoice_no,
            "amount_try": authoritative_price,
            "item_title": canonical_title,
            "payment_gateway": "corporate_proforma_pending",
            "message": "Sipariş talebiniz resmi referans numarasıyla kaydedildi. Kurumsal faturalandırma ve sanal POS aktivasyonu sonrası erişiminiz tanımlanacaktır."
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct PayTrCallbackPayload {
    pub merchant_oid: String,
    pub status: String,
    pub total_amount: String,
    pub hash: String,
}

/// Official PayTR Server-to-Server Callback Endpoint (`POST /api/v1/payments/paytr-callback`)
pub async fn paytr_callback(
    State(state): State<AppState>,
    Form(payload): Form<PayTrCallbackPayload>,
) -> Response {
    let paytr_config = payment_service::PayTrConfig::from_app_config(&state.config);
    if !paytr_config.is_configured() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "PAYTR notification rejected: gateway not configured",
        )
            .into_response();
    }

    if !payment_service::verify_callback_hash(
        &paytr_config,
        &payload.merchant_oid,
        &payload.status,
        &payload.total_amount,
        &payload.hash,
    ) {
        return (
            StatusCode::FORBIDDEN,
            "PAYTR notification failed: bad hash",
        )
            .into_response();
    }

    if payload.status == "success" {
        let _ = state.db.finalize_paid_invoice(&payload.merchant_oid);
    } else {
        let _ = state.db.mark_invoice_failed(&payload.merchant_oid);
    }

    (StatusCode::OK, "OK").into_response()
}

// ============================================================================
// ADMIN OMS / CMS ENDPOINTS (/api/v1/admin/*)
// ============================================================================

pub async fn admin_overview(State(state): State<AppState>) -> Json<Value> {
    let contents = state.db.get_all_contents();
    let ads = state.db.get_all_ads();
    let users = state.db.get_all_users();
    let corp_reqs = state.db.get_corporate_requests();
    let reminders = state.db.get_renewal_reminders();
    let invoices = state.db.get_order_invoices();

    let pending_reqs = corp_reqs.iter().filter(|r| r.status == "pending").count();
    let total_views: i64 = contents.iter().map(|c| c.view_count).sum();
    let total_true_reads: i64 = contents.iter().map(|c| c.true_read_count).sum();

    Json(json!({
        "total_contents": contents.len(),
        "total_views": total_views,
        "total_true_reads": total_true_reads,
        "active_ads": ads.iter().filter(|a| a.is_active).count(),
        "registered_users": users.len(),
        "pending_corporate_requests": pending_reqs,
        "scheduled_reminders": reminders.iter().filter(|r| r.status == "scheduled").count(),
        "halted_reminders": reminders.iter().filter(|r| r.status == "halted_by_renewal").count(),
        "invoices_count": invoices.len()
    }))
}

pub async fn admin_upsert_content(
    State(state): State<AppState>,
    Json(mut item): Json<ContentItem>,
) -> Response {
    item.slug = validation_engine::validate_slug(&item.slug)
        .unwrap_or_else(|| validation_engine::validate_slug(&item.title_tr).unwrap_or_default());
    item.body_tr = validation_engine::sanitize_html(&item.body_tr);
    item.body_en = validation_engine::sanitize_html(&item.body_en);

    match state.db.upsert_content(&item) {
        Ok(id) => {
            crate::services::indexnow_service::spawn_instant_seo_push(
                (*state.config).clone(),
                vec![
                    crate::engine::url_builder::article_url(&item.slug),
                    "/".to_string(),
                    crate::engine::url_builder::category_url(&item.content_type),
                ],
            );
            Json(json!({
                "status": "saved",
                "id": id,
                "slug": item.slug,
                "placements_count": item.placements.len(),
                "indexnow_queued": true
            }))
            .into_response()
        }
        Err(e) => {
            ApiResponse::server_error_with_key(&state.translations, "tr", "admin_upsert_content", &e, msg_keys::SERVER_ERROR_SAVE).into_response()
        }
    }
}

pub async fn admin_delete_content(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Json<Value> {
    let _ = state.db.delete_content(id);
    Json(json!({"status": "deleted", "id": id}))
}

pub async fn admin_ping_indexnow(State(state): State<AppState>) -> Response {
    let mut urls: Vec<String> = vec![
        "/".to_string(),
        "/e-dergi".to_string(),
        "/sitemap.xml".to_string(),
        "/sitemap-news.xml".to_string(),
        "/llms.txt".to_string(),
    ];
    for c in state.db.get_all_contents().into_iter().take(20) {
        urls.push(crate::engine::url_builder::article_url(&c.slug));
    }
    let key = crate::services::indexnow_service::resolve_indexnow_key(&state.config);
    match crate::services::indexnow_service::push_urls_now(&state.config, &urls).await {
        Ok(code) => Json(json!({
            "status": "pushed",
            "http_status": code,
            "indexnow_key": key,
            "pushed_count": urls.len()
        }))
        .into_response(),
        Err(err) => Json(json!({
            "status": "queued_or_unreachable",
            "detail": err,
            "indexnow_key": key,
            "pushed_count": urls.len()
        }))
        .into_response(),
    }
}

/// Multipart media & PDF upload endpoint for the OMS/CMS (`POST /api/v1/admin/upload`)
pub async fn admin_upload_media(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Response {
    while let Ok(Some(field)) = multipart.next_field().await {
        let Some(orig_name) = field.file_name().map(|s| s.to_string()) else {
            continue;
        };
        let ext = FsPath::new(&orig_name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        if !media_service::is_allowed_extension(&ext) {
            return ApiResponse::error(&state.translations, "tr", StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::INVALID_FILE_TYPE).into_response();
        }

        let bytes = match field.bytes().await {
            Ok(b) if media_service::is_valid_size(b.len()) => b,
            _ => {
                return ApiResponse::error(&state.translations, "tr", StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::FILE_TOO_LARGE).into_response();
            }
        };

        let (relative_url, filename) = media_service::generate_upload_path(
            &orig_name,
            &ext,
            crate::engine::security_engine::sanitize_slug,
        );

        let subdir = relative_url.rsplit_once('/').map(|(d, _)| d).unwrap_or("images/uploads");
        let target_dir = format!("{}/{}", state.config.static_dir.trim_end_matches('/'), subdir);
        if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
            return ApiResponse::server_error_with_key(&state.translations, "tr", "admin_upload_mkdir", &e, msg_keys::SERVER_ERROR_UPLOAD).into_response();
        }

        let full_path = format!("{}/{}", target_dir, filename);
        if let Err(e) = tokio::fs::write(&full_path, &bytes).await {
            return ApiResponse::server_error_with_key(&state.translations, "tr", "admin_upload_write", &e, msg_keys::SERVER_ERROR_FILE_WRITE).into_response();
        }

        // Register in media_library DB for centralized tracking
        if let Err(e) = media_service::register_media(
            &state.db,
            &filename,
            &orig_name,
            &ext,
            bytes.len(),
            &relative_url,
            "admin",
        ) {
            tracing::warn!("Media library registration failed (file saved): {}", e);
        }

        return Json(json!({
            "status": "uploaded",
            "url": relative_url,
            "size_bytes": bytes.len()
        }))
        .into_response();
    }

    ApiResponse::error(&state.translations, "tr", StatusCode::BAD_REQUEST, error_codes::BAD_REQUEST, msg_keys::UPLOAD_NO_FIELD).into_response()
}

#[derive(Deserialize)]
pub struct CheckConflictPayload {
    pub slot_code: String,
    pub sponsor_name: String,
    pub target_institutions: Option<Vec<String>>,
    pub target_ssb_tags: Option<Vec<String>>,
    pub target_categories: Option<Vec<String>>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub rotation_mode: Option<String>,
}

pub async fn admin_check_ad_conflict(
    State(state): State<AppState>,
    Json(payload): Json<CheckConflictPayload>,
) -> Json<Value> {
    let candidate = AdCampaign {
        id: 0,
        slot_code: payload.slot_code,
        badge_label: "SPONSORLU".to_string(),
        dimensions: "970x90".to_string(),
        sponsor_name: payload.sponsor_name,
        sponsor_type: "defense_oem".to_string(),
        pill_style: "navy".to_string(),
        title_tr: String::new(),
        title_en: String::new(),
        desc_tr: String::new(),
        desc_en: String::new(),
        target_url: String::new(),
        image_url: None,
        budget_score: 80,
        is_default_fallback: false,
        is_floating_popup: false,
        is_newsletter_ad: false,
        target_categories: payload.target_categories.unwrap_or_default(),
        target_ssb_tags: payload.target_ssb_tags.unwrap_or_default(),
        target_institutions: payload.target_institutions.unwrap_or_default(),
        impressions_count: 0,
        clicks_count: 0,
        is_active: true,
    };

    let existing = state.db.get_all_ads();
    let warning = ad_service::check_ad_conflict_with_window(
        &candidate,
        &existing,
        state.config.ad_max_quota_per_keyword,
        payload.start_date.as_deref(),
        payload.end_date.as_deref(),
        payload.rotation_mode.as_deref(),
    );

    Json(json!(warning))
}

pub async fn admin_upsert_ad(
    State(state): State<AppState>,
    Json(ad): Json<AdCampaign>,
) -> Response {
    let existing = state.db.get_all_ads();
    let conflict =
        ad_service::check_ad_conflict(&ad, &existing, state.config.ad_max_quota_per_keyword);

    match state.db.upsert_ad(&ad) {
        Ok(id) => Json(json!({
            "status": "saved",
            "id": id,
            "conflict_check": conflict
        }))
        .into_response(),
        Err(e) => {
            AppError::internal("ad_upsert", e, msg_keys::SERVER_ERROR_SAVE)
                .into_api_response(&state.translations, "tr")
                .into_response()
        }
    }
}

pub async fn admin_analytics_bundle(State(state): State<AppState>) -> Json<Value> {
    let contents = state.db.get_all_contents();
    let ads = state.db.get_all_ads();
    let users = state.db.get_all_users();
    let events = state.db.get_all_analytics_events();

    let content_reports: Vec<_> = contents
        .iter()
        .map(|c| analytics_service::compute_content_report(c, &events))
        .collect();
    let ad_reports: Vec<_> = ads
        .iter()
        .map(|a| analytics_service::compute_ad_report(a, &events))
        .collect();
    let reader_reports = analytics_service::compute_reader_reports(&users, &events, &ads);

    Json(json!({
        "contents": content_reports,
        "ads": ad_reports,
        "readers": reader_reports
    }))
}

pub async fn admin_analytics_contents(State(state): State<AppState>) -> Json<Value> {
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let reports: Vec<_> = contents
        .iter()
        .map(|c| analytics_service::compute_content_report(c, &events))
        .collect();
    Json(json!(reports))
}

pub async fn admin_analytics_ads(State(state): State<AppState>) -> Json<Value> {
    let ads = state.db.get_all_ads();
    let events = state.db.get_all_analytics_events();
    let reports: Vec<_> = ads
        .iter()
        .map(|a| analytics_service::compute_ad_report(a, &events))
        .collect();
    Json(json!(reports))
}

pub async fn admin_analytics_readers(State(state): State<AppState>) -> Json<Value> {
    let users = state.db.get_all_users();
    let ads = state.db.get_all_ads();
    let events = state.db.get_all_analytics_events();
    let reports = analytics_service::compute_reader_reports(&users, &events, &ads);
    Json(json!(reports))
}

pub async fn admin_list_corporate_requests(
    State(state): State<AppState>,
) -> Json<Vec<CorporateVerificationRequest>> {
    Json(state.db.get_corporate_requests())
}

#[derive(Deserialize)]
pub struct CorporateDecisionPayload {
    pub id: Option<i64>,
    pub status: String,
    pub reviewed_by: Option<String>,
}

pub async fn admin_decide_corporate_request_by_path(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<CorporateDecisionPayload>,
) -> Json<Value> {
    let reviewer = payload
        .reviewed_by
        .unwrap_or_else(|| state.config.admin_default_user.clone());
    let _ = state
        .db
        .decide_corporate_request(id, &payload.status, &reviewer);
    Json(json!({"status": "updated", "id": id, "decision": payload.status}))
}

pub async fn admin_decide_corporate_request_body(
    State(state): State<AppState>,
    Json(payload): Json<CorporateDecisionPayload>,
) -> Json<Value> {
    let id = payload.id.unwrap_or(1);
    let reviewer = payload
        .reviewed_by
        .unwrap_or_else(|| state.config.admin_default_user.clone());
    let _ = state
        .db
        .decide_corporate_request(id, &payload.status, &reviewer);
    Json(json!({"status": "updated", "id": id, "decision": payload.status}))
}

pub async fn admin_insert_category(
    State(state): State<AppState>,
    Json(cat): Json<CategoryNode>,
) -> Json<Value> {
    let id = state.db.insert_category(&cat).unwrap_or(0);
    Json(json!({"status": "saved", "id": id, "slug": cat.slug}))
}

pub async fn admin_insert_ssb_tag(
    State(state): State<AppState>,
    Json(tag): Json<SsbInterestTag>,
) -> Json<Value> {
    let id = state.db.insert_ssb_tag(&tag).unwrap_or(0);
    Json(json!({"status": "saved", "id": id, "code": tag.code}))
}

pub async fn admin_list_newsletters(
    State(state): State<AppState>,
) -> Json<Vec<NewsletterCampaign>> {
    Json(state.db.get_newsletter_campaigns())
}

pub async fn admin_insert_newsletter(
    State(state): State<AppState>,
    Json(mut nl): Json<NewsletterCampaign>,
) -> Json<Value> {
    let users = state.db.get_all_users();
    let recipients = email_service::filter_newsletter_recipients(
        &users,
        &nl.target_institution,
        &nl.target_ssb_tag,
    );
    nl.recipient_count = recipients.len() as i64;
    if nl.sent_at.trim().is_empty() {
        nl.sent_at = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
    }
    nl.status = if state.config.mail_delivery_enabled {
        "sent".to_string()
    } else {
        "queued".to_string()
    };

    let sponsor_ad = nl
        .sponsor_ad_id
        .and_then(|aid| state.db.get_all_ads().into_iter().find(|a| a.id == aid));
    let dispatch = email_service::dispatch_newsletter_campaign(
        &state.config,
        &nl,
        &recipients,
        sponsor_ad.as_ref(),
    )
    .await;

    let id = state.db.insert_newsletter_campaign(&nl).unwrap_or(0);
    Json(json!({
        "status": nl.status,
        "id": id,
        "recipient_count": nl.recipient_count,
        "dispatch": dispatch
    }))
}

pub async fn admin_list_reminders(State(state): State<AppState>) -> Json<Value> {
    let users = state.db.get_all_users();
    let existing = state.db.get_renewal_reminders();
    let evaluated = subscription_service::evaluate_5_stage_reminders(&users, &existing);
    let _ = state.db.sync_evaluated_reminders(&evaluated);
    Json(json!(evaluated))
}

pub async fn admin_halt_renewed_reminders(State(state): State<AppState>) -> Json<Value> {
    let halted_count = state.db.halt_renewed_reminders().unwrap_or(0);
    Json(json!({
        "status": "halted_by_renewal",
        "halted_count": halted_count
    }))
}

pub async fn admin_list_redirects(State(state): State<AppState>) -> Json<Vec<SeoRedirectRule>> {
    Json(state.db.get_seo_redirects())
}

pub async fn admin_insert_redirect(
    State(state): State<AppState>,
    Json(rd): Json<SeoRedirectRule>,
) -> Json<Value> {
    let id = state.db.insert_seo_redirect(&rd).unwrap_or(0);
    Json(json!({"status": "saved", "id": id}))
}

pub async fn admin_export_open_data(State(state): State<AppState>) -> Response {
    let payload = json!({
        "export_standard": "UMSA-MSI-OPEN-DATA-V1 (Zero Vendor-Lock Universal JSON/PostgreSQL/SQLite Portable Archive)",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "domains": [&state.config.primary_domain, &state.config.news_domain],
        "categories": state.db.get_all_categories(),
        "ssb_interest_tags": state.db.get_all_ssb_tags(),
        "contents_single_source_of_truth": state.db.get_all_contents(),
        "magazine_issues": state.db.get_all_issues(),
        "ad_campaigns": state.db.get_all_ads(),
        "recommendation_rules": state.db.get_recommendation_rules(),
        "users_kvkk_verified": state.db.get_all_users(),
        "corporate_verification_requests": state.db.get_corporate_requests(),
        "subscription_plans": state.db.get_subscription_plans(),
        "commerce_products": state.db.get_commerce_products(),
        "order_invoices": state.db.get_order_invoices(),
        "renewal_reminders_5_stage": state.db.get_renewal_reminders(),
        "newsletter_campaigns": state.db.get_newsletter_campaigns(),
        "seo_301_redirects": state.db.get_seo_redirects(),
        "analytics_events": state.db.get_all_analytics_events()
    });

    let body = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"MSI_Dergisi_OpenData_Full_Backup.json\"",
            ),
        ],
        body,
    )
        .into_response()
}

pub async fn faceted_search_api(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    // Unified query via CentralContentQueryEngine — single entry point for all search/filter.
    let criteria = crate::engine::content_query_engine::CentralContentQueryEngine::from_params(&params);
    let result = crate::engine::content_query_engine::CentralContentQueryEngine::execute(&state.db, &criteria);
    Json(json!(result))
}

pub async fn list_notifications_api(
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let session_u = auth_service::session_user(&state, &headers);
    let is_admin_route = uri.path().starts_with("/api/v1/admin/");

    let (email_opt, org_opt) = if is_admin_route {
        (
            params.get("email").map(|s| s.as_str()),
            params.get("org").map(|s| s.as_str()),
        )
    } else if let Some(ref u) = session_u {
        (Some(u.email.as_str()), Some(u.organization_category.as_str()))
    } else {
        (None, None)
    };

    let all = state.db.get_system_notifications(email_opt);
    let filtered: Vec<_> = if email_opt.is_some() || is_admin_route {
        crate::engine::notification_engine::filter_user_notifications(&all, email_opt, org_opt)
    } else {
        all.iter()
            .filter(|n| {
                n.target_email.as_deref().unwrap_or("").is_empty()
                    && matches!(n.target_audience.as_str(), "all" | "public" | "global" | "")
            })
            .cloned()
            .collect()
    };
    let banners = crate::engine::notification_engine::filter_active_banners(&filtered);
    Json(json!({
        "notifications": filtered,
        "active_banners": banners,
        "total": filtered.len()
    }))
}

pub async fn mark_notification_read_api(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Json<Value> {
    let ok = state.db.mark_notification_read(id).is_ok();
    Json(json!({"status": if ok { "marked_read" } else { "error" }, "id": id}))
}

#[derive(Deserialize)]
pub struct CreateNotificationPayload {
    pub category: Option<String>,
    pub severity: Option<String>,
    pub title_tr: String,
    pub title_en: Option<String>,
    pub message_tr: String,
    pub message_en: Option<String>,
    pub target_audience: Option<String>,
    pub action_url: Option<String>,
    pub is_banner: Option<bool>,
}

pub async fn admin_create_notification_api(
    State(state): State<AppState>,
    Json(payload): Json<CreateNotificationPayload>,
) -> Json<Value> {
    let notif = crate::engine::notification_engine::build_notification_event(
        payload.target_audience.as_deref().unwrap_or("ALL"),
        None,
        payload.category.as_deref().unwrap_or("system"),
        payload.severity.as_deref().unwrap_or("info"),
        &payload.title_tr,
        payload.title_en.as_deref().unwrap_or(&payload.title_tr),
        &payload.message_tr,
        payload.message_en.as_deref().unwrap_or(&payload.message_tr),
        payload.action_url.as_deref(),
        payload.is_banner.unwrap_or(false),
    );
    let id = state.db.insert_notification(&notif).unwrap_or(0);
    Json(json!({"status": "created", "id": id, "notification": notif}))
}

pub async fn list_site_settings_api(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "settings": state.db.get_site_settings(),
        "map_tr": state.db.get_setting_map("tr"),
        "map_en": state.db.get_setting_map("en")
    }))
}

#[derive(Deserialize)]
pub struct UpsertSettingPayload {
    pub key: String,
    pub value_tr: String,
    pub value_en: Option<String>,
    pub setting_group: Option<String>,
}

pub async fn admin_upsert_setting_api(
    State(state): State<AppState>,
    Json(payload): Json<UpsertSettingPayload>,
) -> Response {
    if payload.key == crate::engine::design::config::SETTING_KEY {
        let parsed =
            serde_json::from_str::<crate::engine::design::config::DesignConfig>(&payload.value_tr);
        if let Err(_error) = parsed.map_err(|e| e.to_string()).and_then(|c| c.validate()) {
            return json_error(&state, StatusCode::BAD_REQUEST, crate::engine::api_response::msg_keys::BAD_REQUEST);
        }
        if let Some(en) = payload.value_en.as_ref().filter(|v| !v.is_empty()) {
            if let Err(_error) =
                serde_json::from_str::<crate::engine::design::config::DesignConfig>(en)
                    .map_err(|e| e.to_string())
                    .and_then(|c| c.validate())
            {
                return json_error(&state, StatusCode::BAD_REQUEST, crate::engine::api_response::msg_keys::BAD_REQUEST);
            }
        }
    }
    let val_en = payload.value_en.as_deref().unwrap_or(&payload.value_tr);
    let grp = payload.setting_group.as_deref().unwrap_or("general");
    let setting = crate::models::SiteSetting {
        key: payload.key.clone(),
        value_tr: payload.value_tr.clone(),
        value_en: val_en.to_string(),
        setting_group: grp.to_string(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let ok = state.db.upsert_site_setting(&setting).is_ok();
    ApiResponse::ok(json!({
        "key": payload.key,
        "updated": ok
    })).into_response()
}

pub async fn admin_list_security_logs_api(State(state): State<AppState>) -> Json<Value> {
    let logs = state.db.get_security_audit_logs();
    Json(json!({
        "total": logs.len(),
        "logs": logs
    }))
}

pub async fn logout_user(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let secure = cookie_secure_flag(&headers, &state.config.public_url);
    let cookie = format!("msi_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}", secure);
    (
        [(header::SET_COOKIE, cookie)],
        Json(json!({"status":"logged_out"})),
    )
        .into_response()
}

// ── Translation Engine Admin API ────────────────────────────────────

#[derive(Deserialize)]
pub struct TranslationUpsertPayload {
    pub key: String,
    pub value_tr: Option<String>,
    pub value_en: Option<String>,
    pub value_ar: Option<String>,
    pub value_de: Option<String>,
    pub value_ru: Option<String>,
    pub value_fr: Option<String>,
    pub group: Option<String>,
}

pub async fn admin_list_translations_api(State(state): State<AppState>) -> Json<Value> {
    let entries = state.translations.all_entries();
    Json(json!({
        "total": entries.len(),
        "translations": entries.iter().map(|e| json!({
            "key": e.key,
            "value_tr": e.value_tr,
            "value_en": e.value_en,
            "value_ar": e.value_ar,
            "value_de": e.value_de,
            "value_ru": e.value_ru,
            "value_fr": e.value_fr,
            "group": e.group,
        })).collect::<Vec<_>>()
    }))
}

pub async fn admin_upsert_translation_api(
    State(state): State<AppState>,
    Json(payload): Json<TranslationUpsertPayload>,
) -> Response {
    // Preserve existing values for languages not provided in the payload
    let existing = state.translations.get_entry(&payload.key);
    let entry = crate::engine::translation_engine::TranslationEntry {
        key: payload.key.clone(),
        value_tr: payload.value_tr.or_else(|| existing.as_ref().map(|e| e.value_tr.clone())).unwrap_or_default(),
        value_en: payload.value_en.or_else(|| existing.as_ref().map(|e| e.value_en.clone())).unwrap_or_default(),
        value_ar: payload.value_ar.or_else(|| existing.as_ref().map(|e| e.value_ar.clone())).unwrap_or_default(),
        value_de: payload.value_de.or_else(|| existing.as_ref().map(|e| e.value_de.clone())).unwrap_or_default(),
        value_ru: payload.value_ru.or_else(|| existing.as_ref().map(|e| e.value_ru.clone())).unwrap_or_default(),
        value_fr: payload.value_fr.or_else(|| existing.as_ref().map(|e| e.value_fr.clone())).unwrap_or_default(),
        group: payload.group.or_else(|| existing.map(|e| e.group)).unwrap_or_else(|| "general".to_string()),
    };
    if let Err(e) = state.db.upsert_translation(&entry) {
        return AppError::internal("translation_upsert", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response();
    }
    // Reload cache
    state.translations.reload(state.db.load_all_translations());
    ApiResponse::ok(json!({"key": payload.key})).into_response()
}

pub async fn admin_delete_translation_api(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    if let Err(e) = state.db.delete_translation(&key) {
        return AppError::internal("translation_delete", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response();
    }
    state.translations.reload(state.db.load_all_translations());
    ApiResponse::ok(json!({"key": key, "action": "deleted"})).into_response()
}

// ============================================================================
// ADVANCED ADMIN: Content Version History
// ============================================================================

pub async fn admin_list_content_versions(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Json<Value> {
    let versions = state.db.get_content_versions(id);
    Json(json!({ "content_id": id, "versions": versions }))
}

#[derive(Deserialize)]
pub struct ContentVersionPayload {
    pub content_id: i64,
    pub title_tr: String,
    pub body_tr: String,
    pub summary_tr: String,
    pub change_note: Option<String>,
}

pub async fn admin_create_content_version(
    State(state): State<AppState>,
    Json(payload): Json<ContentVersionPayload>,
) -> Json<Value> {
    match state.db.insert_content_version(
        payload.content_id,
        &payload.title_tr,
        &payload.body_tr,
        &payload.summary_tr,
        "operator@msidergisi.com",
        &payload.change_note.unwrap_or_default(),
    ) {
        Ok(id) => Json(json!({ "status": "created", "version_id": id })),
        Err(e) => Json(json!({ "error": format!("Version insert failed: {}", e) })),
    }
}

// ============================================================================
// ADVANCED ADMIN: Media Library
// ============================================================================

pub async fn admin_list_media_library(State(state): State<AppState>) -> Json<Value> {
    let items = state.db.get_media_library();
    Json(json!({ "total": items.len(), "items": items }))
}

// ============================================================================
// ADVANCED ADMIN: Audit Log
// ============================================================================

pub async fn admin_list_audit_logs(State(state): State<AppState>) -> Json<Value> {
    let logs = state.db.get_audit_logs(200);
    Json(json!({ "total": logs.len(), "logs": logs }))
}

#[derive(Deserialize)]
pub struct AuditLogPayload {
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub details_json: Option<String>,
}

pub async fn admin_insert_audit_log(
    State(state): State<AppState>,
    Json(payload): Json<AuditLogPayload>,
) -> Json<Value> {
    match state.db.insert_audit_log(
        "operator@msidergisi.com",
        &payload.action,
        &payload.entity_type,
        payload.entity_id,
        &payload.details_json.unwrap_or_else(|| "{}".to_string()),
        "",
    ) {
        Ok(id) => Json(json!({ "status": "logged", "id": id })),
        Err(e) => Json(json!({ "error": format!("Audit log insert failed: {}", e) })),
    }
}

// ============================================================================
// ADVANCED ADMIN: User Roles & Permissions
// ============================================================================

pub async fn admin_list_user_roles(State(state): State<AppState>) -> Json<Value> {
    let roles = state.db.get_user_roles();
    Json(json!({ "roles": roles }))
}

#[derive(Deserialize)]
pub struct UserRolePayload {
    pub role_code: String,
    pub name_tr: String,
    pub name_en: String,
    pub permissions_json: String,
}

pub async fn admin_upsert_user_role(
    State(state): State<AppState>,
    Json(payload): Json<UserRolePayload>,
) -> Json<Value> {
    match state.db.upsert_user_role(
        &payload.role_code,
        &payload.name_tr,
        &payload.name_en,
        &payload.permissions_json,
    ) {
        Ok(()) => Json(json!({ "status": "saved", "role_code": payload.role_code })),
        Err(e) => Json(json!({ "error": format!("Role upsert failed: {}", e) })),
    }
}

// ============================================================================
// ADVANCED ADMIN: Custom Fields (Dynamic Form Builder)
// ============================================================================

pub async fn admin_list_custom_fields(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let applies_to = params.get("applies_to").map(|s| s.as_str()).unwrap_or("content");
    let fields = state.db.get_custom_fields(applies_to);
    Json(json!({ "applies_to": applies_to, "fields": fields }))
}

#[derive(Deserialize)]
pub struct CustomFieldPayload {
    pub field_key: String,
    pub label_tr: String,
    pub label_en: String,
    pub field_type: String,
    pub options_json: Option<String>,
    pub is_required: Option<bool>,
    pub sort_order: Option<i64>,
    pub applies_to: Option<String>,
}

pub async fn admin_upsert_custom_field(
    State(state): State<AppState>,
    Json(payload): Json<CustomFieldPayload>,
) -> Json<Value> {
    match state.db.upsert_custom_field(
        &payload.field_key,
        &payload.label_tr,
        &payload.label_en,
        &payload.field_type,
        &payload.options_json.unwrap_or_else(|| "[]".to_string()),
        payload.is_required.unwrap_or(false),
        payload.sort_order.unwrap_or(0),
        &payload.applies_to.unwrap_or_else(|| "content".to_string()),
    ) {
        Ok(()) => Json(json!({ "status": "saved", "field_key": payload.field_key })),
        Err(e) => Json(json!({ "error": format!("Custom field upsert failed: {}", e) })),
    }
}

pub async fn admin_delete_custom_field(
    State(state): State<AppState>,
    Path(field_key): Path<String>,
) -> Json<Value> {
    match state.db.delete_custom_field(&field_key) {
        Ok(()) => Json(json!({ "status": "deleted", "field_key": field_key })),
        Err(e) => Json(json!({ "error": format!("Custom field delete failed: {}", e) })),
    }
}

// ============================================================================
// ADVANCED ADMIN: Cache Management
// ============================================================================

pub async fn admin_list_cache_operations(State(state): State<AppState>) -> Json<Value> {
    let ops = state.db.get_cache_operations();
    Json(json!({ "operations": ops }))
}

pub async fn admin_flush_cache(
    State(state): State<AppState>,
    Json(payload): Json<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let cache_key = payload.get("cache_key").map(|s| s.as_str()).unwrap_or("all");
    state.db.refresh_l1_cache();
    let _ = state.db.insert_cache_operation(cache_key, "flush", "operator@msidergisi.com", "Manual cache flush");
    Json(json!({ "status": "flushed", "cache_key": cache_key }))
}

// ============================================================================
// ADVANCED ADMIN: System Health Monitoring
// ============================================================================

pub async fn admin_system_health(State(state): State<AppState>) -> Json<Value> {
    Json(state.db.get_system_health())
}

// ============================================================================
// ADVANCED ADMIN: Bulk Content Operations
// ============================================================================

#[derive(Deserialize)]
pub struct BulkContentPayload {
    pub ids: Vec<i64>,
    pub action: String,
    pub value: Option<String>,
}

pub async fn admin_bulk_content_operation(
    State(state): State<AppState>,
    Json(payload): Json<BulkContentPayload>,
) -> Json<Value> {
    let mut affected = 0;
    for id in &payload.ids {
        match payload.action.as_str() {
            "delete" => {
                let _ = state.db.delete_content(*id);
                affected += 1;
            }
            "set_visibility" => {
                if let Some(val) = &payload.value {
                    if let Ok(level) = val.parse::<i32>() {
                        let contents = state.db.get_all_contents();
                        if let Some(mut c) = contents.into_iter().find(|c| c.id == *id) {
                            c.visibility_level = level;
                            let _ = state.db.upsert_content(&c);
                            affected += 1;
                        }
                    }
                }
            }
            "set_featured" => {
                let contents = state.db.get_all_contents();
                if let Some(mut c) = contents.into_iter().find(|c| c.id == *id) {
                    c.is_featured = payload.value.as_deref() == Some("true");
                    let _ = state.db.upsert_content(&c);
                    affected += 1;
                }
            }
            _ => {}
        }
    }
    Json(json!({ "status": "completed", "action": payload.action, "affected": affected }))
}
