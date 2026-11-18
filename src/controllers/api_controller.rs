use axum::{
    body::Bytes,
    extract::{Extension, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode, Uri},
    response::{Html, IntoResponse, Redirect, Response},
    Form, Json,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path as FsPath;
use std::sync::RwLock;

use crate::controllers::{web_controller, AppState};
use crate::engine::{
    api_response::{self, error_codes, msg_keys, ApiResponse, AppError},
    content_query_engine::CentralContentQueryEngine,
    security_engine::{self, escape_html, CspNonce, LoginCheckResult},
    validation_engine,
};
use crate::extractors::{
    operator_user::{is_staff_role, OperatorUser},
    pagination::PaginationParams,
    require_permission::check_permission,
};
use crate::models::{
    AdCampaign, AnalyticsEvent, CategoryNode, ContentItem, CorporateVerificationRequest,
    NewsletterCampaign, SeoRedirectRule, SiteSetting, SsbInterestTag, UserAccount,
};
use crate::services::{
    ad_service, analytics_service, auth_service, content_service, edergi_pdf_service,
    email_service, media_service, monitoring_service, payment_service, revenue_analytics_service,
    subscription_service, totp_service, user_analytics_service,
};

/// In-memory pending TOTP setup secrets keyed by lowercase user email -> `(secret_base32, issued_ts)`.
/// Prevents half-finished 2FA setups from locking out a user's database record before verification.
static PENDING_TOTP_SETUPS: Lazy<RwLock<HashMap<String, (String, i64)>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

const PENDING_TOTP_SETUP_TTL_SECS: i64 = 900;

/// Centralized JSON error response helper.
fn json_error(state: &AppState, status: StatusCode, msg_key: &'static str) -> Response {
    let error_code = match status {
        StatusCode::NOT_FOUND => error_codes::NOT_FOUND,
        StatusCode::UNAUTHORIZED => error_codes::UNAUTHORIZED,
        StatusCode::FORBIDDEN => error_codes::FORBIDDEN,
        StatusCode::CONFLICT => error_codes::CONFLICT,
        StatusCode::TOO_MANY_REQUESTS => "TOO_MANY_REQUESTS",
        StatusCode::SERVICE_UNAVAILABLE => error_codes::SERVICE_UNAVAILABLE,
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

fn extract_raw_session_cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .find_map(|p| p.trim().strip_prefix("msi_session="))
        })
        .filter(|t| !t.trim().is_empty())
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

#[allow(clippy::result_large_err)]
fn require_perm(operator: &OperatorUser, perm: &str) -> Result<(), Response> {
    check_permission(operator, perm).map_err(IntoResponse::into_response)
}

#[allow(clippy::result_large_err)]
fn require_any_perm(operator: &OperatorUser, perms: &[&str]) -> Result<(), Response> {
    if perms.iter().any(|p| operator.has_permission(p)) {
        Ok(())
    } else {
        let primary = perms.first().copied().unwrap_or("admin:access");
        check_permission(operator, primary).map_err(IntoResponse::into_response)
    }
}

fn is_token_expired(expires_at: &str) -> bool {
    let s = expires_at.trim();
    if s.is_empty() {
        return true;
    }
    let now = chrono::Utc::now();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return now > dt.with_timezone(&chrono::Utc);
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
    {
        return now > naive.and_utc();
    }
    true
}

fn render_shelled_transactional(
    state: &AppState,
    headers: &HeaderMap,
    nonce: Option<&str>,
    status: StatusCode,
    page_title: &str,
    canonical_path: &str,
    card_class: &str,
    inner_html: &str,
) -> Response {
    let lang = web_controller::detect_language(&HashMap::new(), headers);
    let body_html = format!(
        r#"<section class="container section-space"><div class="transactional-card {card_class}">{inner_html}</div></section>"#
    );
    let full_html = web_controller::render_shared_shell(
        state,
        &lang,
        page_title,
        &body_html,
        Some(canonical_path),
        None,
        true,
        nonce,
    );
    (status, Html(full_html)).into_response()
}

fn default_date_bounds(params: &HashMap<String, String>) -> (String, String) {
    let now = chrono::Utc::now();
    let default_to = now.format("%Y-%m-%d").to_string();
    let default_from = (now - chrono::Duration::days(365))
        .format("%Y-%m-%d")
        .to_string();
    let from = params
        .get("from")
        .or_else(|| params.get("date_from"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(default_from);
    let to = params
        .get("to")
        .or_else(|| params.get("date_to"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(default_to);
    (from, to)
}

// ============================================================================
// PUBLIC REST API (/api/v1/*)
// ============================================================================

pub async fn health_check(State(state): State<AppState>, headers: HeaderMap) -> Json<Value> {
    let contents = state.db.get_all_contents();
    let categories = state.db.get_all_categories();
    let ssb_tags = state.db.get_all_ssb_tags();
    let issues = state.db.get_all_issues();
    let ads = state.db.get_all_ads();
    let session_exists = auth_service::session_user(&state, &headers).is_some();

    Json(json!({
        "status": "ok",
        "platform": state.config.app_name,
        "env": state.config.app_env,
        "public_url": state.config.public_url,
        "authenticated": session_exists,
        "engine": "Rust Axum Fullstack Engine v2.6",
        "domains": {
            "primary": state.config.primary_domain,
            "news_portal": state.config.news_domain,
            "bilingual_locales": ["tr", "en"]
        },
        "counts": {
            "contents": contents.iter().filter(|c| content_service::is_publicly_visible(c)).count(),
            "categories": categories.len(),
            "ssb_tags": ssb_tags.len(),
            "magazine_issues": issues.len(),
            "ad_campaigns": ads.iter().filter(|a| a.is_active).count()
        }
    }))
}

pub async fn list_contents(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<ContentItem>> {
    let criteria = CentralContentQueryEngine::from_params(&params);
    let query_result = CentralContentQueryEngine::execute(&state.db, &criteria);
    let mut items = query_result.items;

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
    let is_staff = user_opt
        .as_ref()
        .map(|u| is_staff_role(&state.db, &u.role))
        .unwrap_or(false);

    if !content_service::is_publicly_visible(&item) && !is_staff {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            msg_keys::CONTENT_NOT_FOUND,
        )
        .into_response();
    }

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
        Json(all_ads.into_iter().filter(|a| a.is_active).collect())
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

// ============================================================================
// AUTH, LOCKOUT, SESSION REVOCATION & DEDICATED TOTP 2FA CHALLENGE
// ============================================================================

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
    let lang = "tr";
    let email = match validation_engine::validate_email(&payload.email) {
        validation_engine::EmailValidation::Valid(e) => e,
        _ => {
            return ApiResponse::error(
                &state.translations,
                lang,
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::INVALID_EMAIL,
            )
            .into_response()
        }
    };

    if state.db.get_user_by_email(&email).is_some() {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::CONFLICT,
            error_codes::CONFLICT,
            msg_keys::EMAIL_ALREADY_EXISTS,
        )
        .into_response();
    }
    let raw_pw = match validation_engine::validate_password(payload.password.as_deref()) {
        validation_engine::PasswordValidation::Valid(pw) => pw,
        _ => {
            return ApiResponse::error(
                &state.translations,
                lang,
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::REGISTRATION_FIELDS_REQUIRED,
            )
            .into_response()
        }
    };
    if matches!(
        validation_engine::validate_required_text(&payload.full_name),
        validation_engine::TextFieldValidation::Empty
    ) || payload.kvkk_consent != Some(true)
        || payload.kvkk_acknowledged == Some(false)
    {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::KVKK_CONSENT_REQUIRED,
        )
        .into_response();
    }
    let org_category = validation_engine::with_default(
        payload.organization_category.as_deref(),
        "Sektör Kuruluşları",
    );
    let org_name =
        validation_engine::with_default(payload.organization_name.as_deref(), &org_category);
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

    let user_email_consent = payload.email_consent.unwrap_or(false);
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
        email_consent: user_email_consent,
        ssb_interests: payload.ssb_interests.unwrap_or_default(),
        created_at: now_iso[..10].to_string(),
        email_verified: false,
        marketing_emails_opted_in: user_email_consent,
        totp_enabled: false,
    };

    let uid = match state.db.create_or_update_user(&user, &pw_hash) {
        Ok(id) => id,
        Err(e) => {
            return ApiResponse::server_error_with_key(
                &state.translations,
                lang,
                "register_user",
                &e,
                msg_keys::SERVER_ERROR_ORDER,
            )
            .into_response();
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

    let verification_token = format!(
        "{}{}",
        uuid::Uuid::new_v4().as_simple(),
        uuid::Uuid::new_v4().as_simple()
    );
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);
    let expires_str = expires_at.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut email_verification_sent = false;
    if let Err(e) = state
        .db
        .create_email_verification_token(uid, &verification_token, &expires_str)
    {
        tracing::error!(
            "Failed to create email verification token for user {}: {}",
            uid,
            e
        );
    } else {
        email_verification_sent = true;
        let config = state.config.clone();
        let email_clone = email.clone();
        let site_url = config.public_url.clone();
        tokio::spawn(async move {
            if let Err(e) = email_service::send_verification_email(
                &config,
                &email_clone,
                &verification_token,
                &site_url,
            )
            .await
            {
                tracing::warn!("Failed to send verification email to {}: {}", email_clone, e);
            }
        });
    }

    let token = auth_service::generate_session_token(&user, &state.config.session_secret);
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
            "corporate_verification_queued": verification_queued,
            "email_verification_sent": email_verification_sent
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
}

pub async fn login_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginPayload>,
) -> Response {
    let norm_email = payload.email.trim().to_lowercase();
    let client_ip = extract_client_ip(&headers);
    let ip_email_key = format!("ip:{}:{}", client_ip, norm_email);
    let is_loopback = client_ip == "127.0.0.1" || client_ip == "::1";
    let ip_global_key = format!("ip_global:{}", client_ip);

    // 1. Brute-force lockout enforcement (email + IP)
    for check_key in [
        Some(norm_email.as_str()),
        Some(ip_email_key.as_str()),
        if is_loopback {
            None
        } else {
            Some(ip_global_key.as_str())
        },
    ]
    .into_iter()
    .flatten()
    {
        if let LoginCheckResult::Locked {
            remaining_secs,
            attempts,
        } = security_engine::check_login_allowed(check_key)
        {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "status": "error",
                    "error_code": "ACCOUNT_LOCKED",
                    "message": format!(
                        "Çok fazla başarısız giriş denemesi. Lütfen {} saniye sonra tekrar deneyin.",
                        remaining_secs
                    ),
                    "data": {
                        "remaining_secs": remaining_secs,
                        "attempts": attempts
                    }
                })),
            )
                .into_response();
        }
    }

    if let Some((user, pw_hash)) = state.db.get_user_by_email(&norm_email) {
        if user.role != "deleted"
            && auth_service::verify_password(
                &payload.password,
                &state.config.session_secret,
                &pw_hash,
            )
        {
            security_engine::reset_login_attempts(&norm_email);
            security_engine::reset_login_attempts(&ip_email_key);
            if !is_loopback {
                security_engine::reset_login_attempts(&ip_global_key);
            }

            if !auth_service::is_argon2_hash(&pw_hash) {
                let upgraded =
                    auth_service::hash_password(&payload.password, &state.config.session_secret);
                let _ = state.db.update_user_password_hash(&user.email, &upgraded);
            }

            // 2. Check TOTP 2FA enrollment — NEVER set msi_session cookie when 2FA is active!
            if let Some(secret) = state.db.get_user_totp_secret(&user.email) {
                if !secret.trim().is_empty() {
                    let temp_token = auth_service::generate_totp_pending_token(
                        &user.email,
                        &state.config.session_secret,
                    );
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "status": "totp_required",
                            "requires_totp": true,
                            "temp_token": temp_token,
                            "message": "İki aşamalı doğrulama kodu gerekli.",
                            "data": {
                                "status": "totp_required",
                                "requires_totp": true,
                                "temp_token": temp_token
                            }
                        })),
                    )
                        .into_response();
                }
            }

            let token = auth_service::generate_session_token(&user, &state.config.session_secret);
            let secure = cookie_secure_flag(&headers, &state.config.public_url);
            let cookie = format!(
                "msi_session={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=86400{}",
                token, secure
            );
            let email_verified = user.email_verified;
            return (
                [(header::SET_COOKIE, cookie)],
                Json(json!({
                    "status": "authenticated",
                    "user": user,
                    "email_verified": email_verified
                })),
            )
                .into_response();
        }
    }

    security_engine::record_failed_login(&norm_email);
    security_engine::record_failed_login(&ip_email_key);
    if !is_loopback {
        security_engine::record_failed_login(&ip_global_key);
    }

    ApiResponse::error(
        &state.translations,
        "tr",
        StatusCode::UNAUTHORIZED,
        error_codes::UNAUTHORIZED,
        msg_keys::LOGIN_FAILED,
    )
    .into_response()
}

pub async fn totp_setup(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };

    let secret = totp_service::generate_totp_secret();
    let otpauth_uri = match totp_service::generate_totp_uri(&secret, &user.email) {
        Ok(u) => u,
        Err(e) => {
            return ApiResponse::server_error(&state.translations, "tr", "totp_setup_uri", &e)
                .into_response()
        }
    };
    let qr_svg = match totp_service::generate_qr_svg(&otpauth_uri) {
        Ok(svg) => svg,
        Err(e) => {
            return ApiResponse::server_error(&state.translations, "tr", "totp_setup_qr", &e)
                .into_response()
        }
    };

    if let Ok(mut guard) = PENDING_TOTP_SETUPS.write() {
        let now = chrono::Utc::now().timestamp();
        guard.retain(|_, (_, ts)| now - *ts <= PENDING_TOTP_SETUP_TTL_SECS);
        guard.insert(user.email.trim().to_lowercase(), (secret.clone(), now));
    }

    ApiResponse::ok(json!({
        "secret": secret,
        "otpauth_uri": otpauth_uri,
        "qr_uri": otpauth_uri,
        "qr_svg": qr_svg
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct TotpVerifySetupPayload {
    pub code: String,
    pub secret: Option<String>,
}

pub async fn totp_verify_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TotpVerifySetupPayload>,
) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };

    let email_key = user.email.trim().to_lowercase();
    let now = chrono::Utc::now().timestamp();
    let pending_secret = PENDING_TOTP_SETUPS.read().ok().and_then(|guard| {
        guard
            .get(&email_key)
            .filter(|(_, ts)| now - *ts <= PENDING_TOTP_SETUP_TTL_SECS)
            .map(|(sec, _)| sec.clone())
    });

    let candidate_secret = payload
        .secret
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or(pending_secret);

    let Some(secret) = candidate_secret else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "error_code": "TOTP_SETUP_EXPIRED",
                "message": "2FA kurulum oturumu bulunamadı veya süresi doldu. Lütfen kurulumu yeniden başlatın.",
                "data": null
            })),
        )
            .into_response();
    };

    if !totp_service::verify_totp(&secret, &payload.code) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "error_code": "TOTP_CODE_INVALID",
                "message": "Doğrulama kodu geçersiz. Lütfen 6 haneli kodu kontrol edin.",
                "data": null
            })),
        )
            .into_response();
    }

    let backup_codes = totp_service::generate_backup_codes();
    let hashed_codes: Vec<String> = backup_codes
        .iter()
        .map(|c| totp_service::hash_backup_code(c))
        .collect();

    if let Err(e) = state.db.set_user_totp_secret(&user.email, Some(&secret)) {
        return ApiResponse::server_error(&state.translations, "tr", "totp_save_secret", &e)
            .into_response();
    }
    if let Err(e) = state
        .db
        .set_user_totp_backup_codes(&user.email, &hashed_codes)
    {
        return ApiResponse::server_error(&state.translations, "tr", "totp_save_backup_codes", &e)
            .into_response();
    }

    if let Ok(mut guard) = PENDING_TOTP_SETUPS.write() {
        guard.remove(&email_key);
    }

    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &user.email,
        "totp_enable",
        "user",
        Some(user.id),
        r#"{"totp_enabled":true}"#,
        &client_ip,
    );

    ApiResponse::ok(json!({
        "enabled": true,
        "backup_codes": backup_codes,
        "recovery_codes": backup_codes
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct TotpLoginPayload {
    pub temp_token: String,
    pub code: String,
}

pub async fn totp_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TotpLoginPayload>,
) -> Response {
    let Some(email) =
        auth_service::verify_totp_pending_token(&payload.temp_token, &state.config.session_secret)
    else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "error",
                "error_code": "TOTP_TOKEN_INVALID",
                "message": "Geçersiz veya süresi dolmuş 2FA doğrulama oturumu. Lütfen tekrar giriş yapın.",
                "data": null
            })),
        )
            .into_response();
    };

    let lockout_key = format!("totp:{}", email);
    if let LoginCheckResult::Locked {
        remaining_secs,
        attempts,
    } = security_engine::check_login_allowed(&lockout_key)
    {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "status": "error",
                "error_code": "ACCOUNT_LOCKED",
                "message": format!(
                    "Çok fazla hatalı 2FA denemesi. Lütfen {} saniye bekleyin.",
                    remaining_secs
                ),
                "data": { "remaining_secs": remaining_secs, "attempts": attempts }
            })),
        )
            .into_response();
    }

    let Some((user, _)) = state.db.get_user_by_email(&email) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::LOGIN_FAILED);
    };
    let Some(totp_secret) = state.db.get_user_totp_secret(&user.email) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
    };

    let mut valid_2fa = totp_service::verify_totp(&totp_secret, &payload.code);
    if !valid_2fa {
        let mut stored_hashes = state.db.get_user_totp_backup_codes(&user.email);
        if let Some(matched_idx) = totp_service::verify_backup_code(&payload.code, &stored_hashes) {
            stored_hashes.remove(matched_idx);
            let _ = state
                .db
                .set_user_totp_backup_codes(&user.email, &stored_hashes);
            valid_2fa = true;
        }
    }

    if !valid_2fa {
        security_engine::record_failed_login(&lockout_key);
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "error",
                "error_code": "TOTP_CODE_INVALID",
                "message": "Doğrulama veya yedek kurtarma kodu geçersiz.",
                "data": null
            })),
        )
            .into_response();
    }

    security_engine::reset_login_attempts(&lockout_key);
    auth_service::revoke_session_token(&payload.temp_token);

    let session_token = auth_service::generate_session_token(&user, &state.config.session_secret);
    let secure = cookie_secure_flag(&headers, &state.config.public_url);
    let cookie = format!(
        "msi_session={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=86400{}",
        session_token, secure
    );
    let email_verified = user.email_verified;

    (
        [(header::SET_COOKIE, cookie)],
        Json(json!({
            "status": "authenticated",
            "user": user,
            "email_verified": email_verified
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct TotpDisablePayload {
    pub password: Option<String>,
    pub code: Option<String>,
}

pub async fn totp_disable(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TotpDisablePayload>,
) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };

    let pw_valid = payload
        .password
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|pw| {
            state.db.get_user_by_email(&user.email).map(|(_, hash)| {
                auth_service::verify_password(pw, &state.config.session_secret, &hash)
            })
        })
        .unwrap_or(false);

    let code_valid = payload
        .code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|code| {
            let totp_ok = state
                .db
                .get_user_totp_secret(&user.email)
                .map(|sec| totp_service::verify_totp(&sec, code))
                .unwrap_or(false);
            if totp_ok {
                return true;
            }
            let hashes = state.db.get_user_totp_backup_codes(&user.email);
            totp_service::verify_backup_code(code, &hashes).is_some()
        })
        .unwrap_or(false);

    if !pw_valid && !code_valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "error",
                "error_code": "UNAUTHORIZED",
                "message": "2FA korumasını devre dışı bırakmak için geçerli şifre veya doğrulama kodu gereklidir.",
                "data": null
            })),
        )
            .into_response();
    }

    if let Err(e) = state.db.set_user_totp_secret(&user.email, None) {
        return ApiResponse::server_error(&state.translations, "tr", "totp_disable", &e)
            .into_response();
    }

    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &user.email,
        "totp_disable",
        "user",
        Some(user.id),
        r#"{"totp_enabled":false}"#,
        &client_ip,
    );

    ApiResponse::ok(json!({ "disabled": true, "totp_enabled": false })).into_response()
}

pub async fn logout_user(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(raw_token) = extract_raw_session_cookie(&headers) {
        auth_service::revoke_session_token(raw_token);
    }
    let secure = cookie_secure_flag(&headers, &state.config.public_url);
    let cookie = format!(
        "msi_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}",
        secure
    );
    (
        [(header::SET_COOKIE, cookie)],
        Json(json!({"status": "logged_out"})),
    )
        .into_response()
}

// ============================================================================
// PASSWORD RESET FLOW (008_password_reset.sql)
// ============================================================================

#[derive(Deserialize)]
pub struct ForgotPasswordPayload {
    pub email: String,
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordPayload>,
) -> Response {
    let email = match validation_engine::validate_email(&payload.email) {
        validation_engine::EmailValidation::Valid(e) => e,
        _ => {
            return ApiResponse::error(
                &state.translations,
                "tr",
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::INVALID_EMAIL,
            )
            .into_response()
        }
    };

    if let Some((user, _)) = state.db.get_user_by_email(&email) {
        if user.role != "deleted" {
            let token = format!(
                "{}{}",
                uuid::Uuid::new_v4().as_simple(),
                uuid::Uuid::new_v4().as_simple()
            );
            let expires_str = (chrono::Utc::now() + chrono::Duration::hours(1))
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();

            let _ = state.db.invalidate_user_reset_tokens(user.id);
            if state
                .db
                .create_password_reset_token(user.id, &token, &expires_str)
                .is_ok()
            {
                let config = state.config.clone();
                let user_email = user.email.clone();
                let site_url = config.public_url.clone();
                tokio::spawn(async move {
                    if let Err(e) = email_service::send_password_reset_email(
                        &config,
                        &user_email,
                        &token,
                        &site_url,
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to send password reset email to {}: {}",
                            user_email,
                            e
                        );
                    }
                });
            }
        }
    }

    ApiResponse::ok(json!({ "sent": true })).into_response()
}

#[derive(Deserialize)]
pub struct ResetPasswordPayload {
    pub token: String,
    #[serde(alias = "password")]
    pub new_password: String,
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordPayload>,
) -> Response {
    let raw_token = payload.token.trim();
    if raw_token.is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
    }

    let valid_pw = match validation_engine::validate_password(Some(&payload.new_password)) {
        validation_engine::PasswordValidation::Valid(pw) => pw,
        _ => {
            return ApiResponse::error(
                &state.translations,
                "tr",
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::REGISTRATION_FIELDS_REQUIRED,
            )
            .into_response()
        }
    };

    let Some((token_id, user_id, expires_at)) = state.db.find_password_reset_token(raw_token)
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "error_code": "INVALID_RESET_TOKEN",
                "message": "Şifre sıfırlama bağlantısı geçersiz veya daha önce kullanılmış.",
                "data": null
            })),
        )
            .into_response();
    };

    if is_token_expired(&expires_at) {
        let _ = state.db.mark_reset_token_used(token_id);
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "error_code": "EXPIRED_RESET_TOKEN",
                "message": "Şifre sıfırlama bağlantısının süresi dolmuş.",
                "data": null
            })),
        )
            .into_response();
    }

    let Some(user) = state.db.get_user_by_id(user_id) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
    };
    if user.role == "deleted" {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
    }

    let new_hash = auth_service::hash_password(&valid_pw, &state.config.session_secret);
    if let Err(e) = state.db.update_user_password_hash(&user.email, &new_hash) {
        return ApiResponse::server_error(&state.translations, "tr", "reset_password", &e)
            .into_response();
    }
    let _ = state.db.mark_reset_token_used(token_id);
    let _ = state.db.invalidate_user_reset_tokens(user_id);
    security_engine::reset_login_attempts(&user.email);

    ApiResponse::ok(json!({ "reset": true })).into_response()
}

// ============================================================================
// SAFE EMAIL VERIFICATION (009_email_verification.sql)
// ============================================================================

#[derive(Deserialize)]
pub struct VerifyEmailQuery {
    pub token: Option<String>,
    pub confirm: Option<String>,
}

pub async fn verify_email(
    State(state): State<AppState>,
    Query(params): Query<VerifyEmailQuery>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let raw_token = params.token.as_deref().unwrap_or("").trim();
    if raw_token.is_empty() {
        let inner = r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Doğrulama Başarısız</h1>
            <p>Doğrulama bağlantısı eksik veya geçersiz.</p>
            <p><a href="/hesabim" class="button secondary">Hesabıma Dön →</a></p>"#;
        return render_shelled_transactional(
            &state,
            &headers,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            "E-Posta Doğrulama",
            "/hesap-dogrula",
            "verify-card",
            inner,
        );
    }

    let Some((token_id, user_id, expires_at)) = state.db.find_email_verification_token(raw_token)
    else {
        let inner = r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Doğrulama Başarısız</h1>
            <p>Geçersiz veya daha önce kullanılmış doğrulama bağlantısı.</p>
            <p><a href="/hesabim" class="button secondary">Hesabıma Dön →</a></p>"#;
        return render_shelled_transactional(
            &state,
            &headers,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            "E-Posta Doğrulama",
            "/hesap-dogrula",
            "verify-card",
            inner,
        );
    };

    if is_token_expired(&expires_at) {
        let inner = r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Bağlantı Süresi Dolmuş</h1>
            <p>Doğrulama bağlantısının süresi dolmuş. Okuyucu Merkezi üzerinden yeni doğrulama e-postası talep edebilirsiniz.</p>
            <p><a href="/hesabim" class="button secondary">Hesabıma Dön →</a></p>"#;
        return render_shelled_transactional(
            &state,
            &headers,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            "E-Posta Doğrulama",
            "/hesap-dogrula",
            "verify-card",
            inner,
        );
    }

    // Only consume on GET if `?confirm=1` is explicitly requested; otherwise render scanner-safe POST form!
    let auto_confirm = matches!(params.confirm.as_deref(), Some("1") | Some("true"));
    if auto_confirm {
        if let Err(e) = state.db.mark_email_verified(user_id) {
            return ApiResponse::server_error(&state.translations, "tr", "verify_email", &e)
                .into_response();
        }
        let _ = state.db.mark_verification_token_used(token_id);
        let inner = r#"<div class="eyebrow">DOĞRULAMA BAŞARILI</div>
            <h1>E-Posta Adresiniz Doğrulandı</h1>
            <p>MSI Dergisi hesabınız başarıyla doğrulandı.</p>
            <p><a href="/hesabim" class="button primary">Hesabıma Git →</a></p>"#;
        return render_shelled_transactional(
            &state,
            &headers,
            Some(&csp_nonce.0),
            StatusCode::OK,
            "E-Posta Doğrulandı",
            "/hesap-dogrula",
            "verify-card",
            inner,
        );
    }

    let escaped_token = escape_html(raw_token);
    let inner = format!(
        r#"<div class="eyebrow">MSI DERGİSİ · E-POSTA DOĞRULAMA</div>
        <h1>E-Posta Adresinizi Onaylayın</h1>
        <p>Kurumsal e-posta tarayıcılarına karşı güvenliğiniz için doğrulamayı aşağıdaki butonla tamamlayın.</p>
        <form method="POST" action="/hesap-dogrula" class="ui-form">
            <input type="hidden" name="token" value="{escaped_token}">
            <button type="submit" class="button primary">E-Posta Adresimi Doğrula →</button>
        </form>"#
    );
    render_shelled_transactional(
        &state,
        &headers,
        Some(&csp_nonce.0),
        StatusCode::OK,
        "E-Posta Doğrulama",
        "/hesap-dogrula",
        "verify-card",
        &inner,
    )
}

pub async fn verify_email_confirm(
    State(state): State<AppState>,
    Query(query_params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let raw_token = form
        .get("token")
        .or_else(|| query_params.get("token"))
        .map(String::as_str)
        .unwrap_or("")
        .trim();

    let Some((token_id, user_id, expires_at)) = state.db.find_email_verification_token(raw_token)
    else {
        let inner = r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Doğrulama Başarısız</h1>
            <p>Geçersiz veya kullanılmış doğrulama bağlantısı.</p>
            <p><a href="/hesabim" class="button secondary">Hesabıma Dön →</a></p>"#;
        return render_shelled_transactional(
            &state,
            &headers,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            "E-Posta Doğrulama",
            "/hesap-dogrula",
            "verify-card",
            inner,
        );
    };

    if is_token_expired(&expires_at) {
        let inner = r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Bağlantı Süresi Dolmuş</h1>
            <p>Doğrulama bağlantısının süresi dolmuş. Lütfen yeni bir doğrulama e-postası talep edin.</p>
            <p><a href="/hesabim" class="button secondary">Hesabıma Dön →</a></p>"#;
        return render_shelled_transactional(
            &state,
            &headers,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            "E-Posta Doğrulama",
            "/hesap-dogrula",
            "verify-card",
            inner,
        );
    }

    if let Err(e) = state.db.mark_email_verified(user_id) {
        return ApiResponse::server_error(&state.translations, "tr", "verify_email_confirm", &e)
            .into_response();
    }
    let _ = state.db.mark_verification_token_used(token_id);

    let inner = r#"<div class="eyebrow">MSI DERGİSİ · DOĞRULAMA BAŞARILI</div>
        <h1>E-Posta Adresiniz Doğrulandı</h1>
        <p>Hesabınız başarıyla doğrulandı. Artık tüm MSI Dergisi özelliklerine erişebilirsiniz.</p>
        <p><a href="/hesabim" class="button primary">Hesabıma Git →</a></p>"#;
    render_shelled_transactional(
        &state,
        &headers,
        Some(&csp_nonce.0),
        StatusCode::OK,
        "E-Posta Doğrulandı",
        "/hesap-dogrula",
        "verify-card",
        inner,
    )
}

#[derive(Deserialize)]
pub struct ResendVerificationPayload {
    pub email: String,
}

pub async fn resend_verification(
    State(state): State<AppState>,
    Json(payload): Json<ResendVerificationPayload>,
) -> Response {
    if let Some((user, _pw_hash)) = state.db.get_user_by_email(&payload.email) {
        if !user.email_verified && user.role != "deleted" {
            let token = format!(
                "{}{}",
                uuid::Uuid::new_v4().as_simple(),
                uuid::Uuid::new_v4().as_simple()
            );
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);
            let expires_str = expires_at.format("%Y-%m-%d %H:%M:%S").to_string();

            if let Err(e) = state
                .db
                .create_email_verification_token(user.id, &token, &expires_str)
            {
                tracing::error!(
                    "Failed to create verification token for user {}: {}",
                    user.id,
                    e
                );
            } else {
                let config = state.config.clone();
                let email = user.email.clone();
                let site_url = config.public_url.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        email_service::send_verification_email(&config, &email, &token, &site_url)
                            .await
                    {
                        tracing::warn!("Failed to send verification email to {}: {}", email, e);
                    }
                });
            }
        }
    }

    ApiResponse::ok(json!({
        "message": "Eğer bu e-posta adresi kayıtlıysa ve henüz doğrulanmamışsa, yeni bir doğrulama bağlantısı gönderilmiştir."
    }))
    .into_response()
}

// ============================================================================
// KVKK/GDPR UNSUBSCRIBE & EMAIL PREFERENCES (010_unsubscribe_tokens.sql)
// ============================================================================

#[derive(Deserialize)]
pub struct UnsubscribeTokenQuery {
    pub token: Option<String>,
}

pub async fn unsubscribe_page(
    State(state): State<AppState>,
    Query(params): Query<UnsubscribeTokenQuery>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let token = match params.token.as_deref() {
        Some(t) if !t.trim().is_empty() => t.trim(),
        _ => {
            let inner = r#"<div class="eyebrow">ABONELİK YÖNETİMİ</div>
                <h1>Geçersiz Bağlantı</h1>
                <p>Bu abonelik iptal bağlantısı geçersiz veya eksik.</p>"#;
            return render_shelled_transactional(
                &state,
                &headers,
                Some(&csp_nonce.0),
                StatusCode::BAD_REQUEST,
                "Abonelik İptali",
                "/abonelik-iptal",
                "unsubscribe-card",
                inner,
            );
        }
    };

    let Some((_token_id, user_id)) = state.db.find_unsubscribe_token(token) else {
        let inner = r#"<div class="eyebrow">ABONELİK YÖNETİMİ</div>
            <h1>Bağlantı Geçersiz</h1>
            <p>Bu abonelik iptal bağlantısı kullanılmış veya süresi dolmuş.</p>"#;
        return render_shelled_transactional(
            &state,
            &headers,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            "Abonelik İptali",
            "/abonelik-iptal",
            "unsubscribe-card",
            inner,
        );
    };

    let user_email = state
        .db
        .get_user_by_id(user_id)
        .map(|u| u.email)
        .unwrap_or_else(|| "kayıtlı adresiniz".to_string());

    let inner = format!(
        r#"<div class="eyebrow">MSI DERGİSİ · ABONELİK YÖNETİMİ</div>
        <h1>E-Posta Aboneliğinden Çıkmak Üzeresiniz</h1>
        <p><strong>{email}</strong> adresine gönderilen MSI Dergisi bültenlerini artık almak istemediğinizi onaylıyorsunuz.</p>
        <form method="POST" action="/abonelik-iptal" class="ui-form">
            <input type="hidden" name="token" value="{token}">
            <button type="submit" class="button primary">Aboneliği İptal Et →</button>
        </form>
        <hr>
        <h2>Tercihlerinizi Yönetin</h2>
        <p>Tamamen çıkmak yerine bülten sıklığını azaltabilir veya yalnızca belirli kategorileri takip edebilirsiniz.</p>
        <form method="POST" action="/api/v1/email-preferences" id="prefs-form" class="ui-form">
            <input type="hidden" name="token" value="{token}">
            <label>Bülten Sıklığı
                <select name="newsletter_frequency">
                    <option value="weekly">Haftalık</option>
                    <option value="biweekly">İki Haftada Bir</option>
                    <option value="monthly">Aylık</option>
                </select>
            </label>
            <label>Kategoriler (virgülle ayırın)
                <input type="text" name="categories" placeholder="Havacılık, Deniz, Kara">
            </label>
            <button type="submit" class="button secondary">Tercihleri Kaydet →</button>
        </form>"#,
        email = escape_html(&user_email),
        token = escape_html(token),
    );

    render_shelled_transactional(
        &state,
        &headers,
        Some(&csp_nonce.0),
        StatusCode::OK,
        "E-Posta Abonelik Yönetimi",
        "/abonelik-iptal",
        "unsubscribe-card",
        &inner,
    )
}

pub async fn unsubscribe_confirm(
    State(state): State<AppState>,
    Query(query_params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
    body: Bytes,
) -> Response {
    let form_map: HashMap<String, String> = serde_urlencoded::from_bytes(&body).unwrap_or_default();
    let is_one_click = form_map
        .get("List-Unsubscribe")
        .map(|v| v.eq_ignore_ascii_case("One-Click"))
        .unwrap_or(false);

    let token = form_map
        .get("token")
        .or_else(|| query_params.get("token"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if token.is_empty() {
        let inner = r#"<div class="eyebrow">ABONELİK YÖNETİMİ</div>
            <h1>Hata</h1>
            <p>Doğrulama anahtarı (token) bulunamadı.</p>"#;
        return render_shelled_transactional(
            &state,
            &headers,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            "Abonelik İptali",
            "/abonelik-iptal",
            "unsubscribe-card",
            inner,
        );
    }

    let Some((token_id, user_id)) = state.db.find_unsubscribe_token(&token) else {
        let inner = r#"<div class="eyebrow">ABONELİK YÖNETİMİ</div>
            <h1>Bağlantı Geçersiz</h1>
            <p>Bu abonelik iptal bağlantısı zaten kullanılmış veya geçersiz.</p>"#;
        return render_shelled_transactional(
            &state,
            &headers,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            "Abonelik İptali",
            "/abonelik-iptal",
            "unsubscribe-card",
            inner,
        );
    };

    let _ = state.db.mark_unsubscribe_token_used(token_id);
    let _ = state.db.opt_out_marketing_emails(user_id);

    if is_one_click
        && !headers
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.contains("text/html"))
            .unwrap_or(false)
    {
        return ApiResponse::ok(json!({ "unsubscribed": true, "one_click": true })).into_response();
    }

    let inner = r#"<div class="eyebrow">MSI DERGİSİ · ABONELİK YÖNETİMİ</div>
        <h1>E-Posta Aboneliğiniz İptal Edildi</h1>
        <p>Artık MSI Dergisi bülten e-postaları almayacaksınız. Tercihlerinizi dilediğiniz zaman <a href="/hesabim">Okuyucu Merkezi</a> üzerinden güncelleyebilirsiniz.</p>"#;
    render_shelled_transactional(
        &state,
        &headers,
        Some(&csp_nonce.0),
        StatusCode::OK,
        "E-Posta Aboneliği İptal Edildi",
        "/abonelik-iptal",
        "unsubscribe-card",
        inner,
    )
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CategoriesField {
    List(Vec<String>),
    Csv(String),
}

#[derive(Deserialize)]
struct EmailPreferencesJsonPayload {
    token: String,
    #[serde(default)]
    newsletter_frequency: String,
    categories: Option<CategoriesField>,
}

pub async fn manage_email_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
    opt_nonce: Option<Extension<CspNonce>>,
    body: Bytes,
) -> Response {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_json = content_type.contains("application/json");

    let (token, frequency, categories): (String, String, Vec<String>) = if is_json {
        let Ok(parsed) = serde_json::from_slice::<EmailPreferencesJsonPayload>(&body) else {
            return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
        };
        let cats = match parsed.categories {
            Some(CategoriesField::List(v)) => v
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            Some(CategoriesField::Csv(s)) => s
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect(),
            None => Vec::new(),
        };
        let freq = if parsed.newsletter_frequency.trim().is_empty() {
            "weekly".to_string()
        } else {
            parsed.newsletter_frequency.trim().to_string()
        };
        (parsed.token.trim().to_string(), freq, cats)
    } else {
        let Ok(pairs) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&body) else {
            return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
        };
        let mut tok = String::new();
        let mut freq = String::from("weekly");
        let mut cats: Vec<String> = Vec::new();
        for (k, v) in pairs {
            match k.as_str() {
                "token" => tok = v.trim().to_string(),
                "newsletter_frequency" if !v.trim().is_empty() => freq = v.trim().to_string(),
                "categories" | "categories[]" => {
                    for part in v.split(',') {
                        let cleaned = part.trim();
                        if !cleaned.is_empty() && !cats.iter().any(|c| c == cleaned) {
                            cats.push(cleaned.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        (tok, freq, cats)
    };

    if token.is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
    }

    let Some((_token_id, user_id)) = state.db.find_unsubscribe_token(&token) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
    };

    // Intentionally DO NOT mark the unsubscribe token as used when updating preferences,
    // so the reader can adjust preferences again or complete full unsubscribe later.
    let prefs_json = serde_json::json!({
        "frequency": frequency,
        "categories": categories,
    })
    .to_string();

    let pref_key = format!("user_email_prefs.{}", user_id);
    let setting = SiteSetting {
        key: pref_key,
        value_tr: prefs_json.clone(),
        value_en: prefs_json,
        setting_group: "user_preferences".to_string(),
        updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    if let Err(e) = state.db.upsert_site_setting(&setting) {
        return AppError::internal("email_preferences_save", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response();
    }

    let _ = state.db.opt_in_marketing_emails(user_id);

    let wants_html = !is_json
        && headers
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.contains("text/html"))
            .unwrap_or(false);

    if wants_html {
        let nonce_ref = opt_nonce.as_ref().map(|Extension(n)| n.0.as_str());
        let inner = format!(
            r#"<div class="eyebrow">MSI DERGİSİ · ABONELİK YÖNETİMİ</div>
            <h1>E-Posta Tercihleriniz Kaydedildi</h1>
            <p>Bülten sıklığınız <strong>{freq}</strong> olarak güncellendi.</p>
            <p><a href="/hesabim" class="button primary">Okuyucu Merkezine Git →</a></p>"#,
            freq = escape_html(&frequency)
        );
        return render_shelled_transactional(
            &state,
            &headers,
            nonce_ref,
            StatusCode::OK,
            "E-Posta Tercihleri Güncellendi",
            "/abonelik-iptal",
            "unsubscribe-card",
            &inner,
        );
    }

    ApiResponse::ok(json!({
        "status": "preferences_saved",
        "frequency": frequency,
        "categories": categories
    }))
    .into_response()
}

// ============================================================================
// ACCOUNT SELF-SERVICE (/api/v1/account/*)
// ============================================================================

#[derive(Deserialize)]
pub struct UpdateAccountProfilePayload {
    pub full_name: String,
    pub organization_category: Option<String>,
    pub organization_name: Option<String>,
    pub job_title: Option<String>,
    pub ssb_interests: Option<Vec<String>>,
    pub email_consent: Option<bool>,
    pub marketing_emails_opted_in: Option<bool>,
}

pub async fn update_account_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateAccountProfilePayload>,
) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };

    let full_name = payload.full_name.trim();
    if full_name.is_empty() || full_name.chars().count() > 120 {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    let org_cat = payload
        .organization_category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&user.organization_category);
    let org_name = payload
        .organization_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&user.organization_name);
    let job_title = payload
        .job_title
        .as_deref()
        .map(str::trim)
        .unwrap_or(&user.job_title);
    let ssb_interests = payload
        .ssb_interests
        .unwrap_or_else(|| user.ssb_interests.clone());
    let consent = payload
        .email_consent
        .or(payload.marketing_emails_opted_in)
        .unwrap_or(user.email_consent && user.marketing_emails_opted_in);

    if let Err(e) = state.db.update_user_profile(
        user.id,
        full_name,
        org_cat,
        org_name,
        job_title,
        &ssb_interests,
        consent,
    ) {
        return ApiResponse::server_error(&state.translations, "tr", "update_account_profile", &e)
            .into_response();
    }

    let updated = state.db.get_user_by_id(user.id).unwrap_or(user);
    ApiResponse::ok(json!({
        "updated": true,
        "user": updated
    }))
    .into_response()
}

pub async fn export_account_data(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };

    let Some(export_data) = state.db.export_user_data(user.id) else {
        return ApiResponse::server_error_with_key(
            &state.translations,
            "tr",
            "export_account_data",
            &"User data export failed",
            msg_keys::SERVER_ERROR_GENERIC,
        )
        .into_response();
    };

    let client_ip = extract_client_ip(&headers);
    let _ = state
        .db
        .insert_security_audit_log(&crate::models::SecurityAuditLog {
            id: 0,
            client_ip,
            request_path: "/api/v1/account/export".to_string(),
            event_type: "GDPR_DATA_EXPORT".to_string(),
            threat_level: "info".to_string(),
            details: json!({
                "user_id": user.id,
                "email": user.email,
                "action": "data_portability_export"
            })
            .to_string(),
            blocked: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        });

    let filename = format!(
        "MSI_KVKK_Export_{}_{}.json",
        user.email.replace('@', "_at_"),
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "application/json; charset=utf-8".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        serde_json::to_string_pretty(&export_data).unwrap_or_else(|_| "{}".to_string()),
    )
        .into_response()
}

#[derive(Deserialize, Default)]
pub struct DeleteAccountPayload {
    pub password: Option<String>,
}

pub async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };

    let parsed: DeleteAccountPayload = serde_json::from_slice(&body).unwrap_or_default();
    if let Some(pw) = parsed.password.as_deref().filter(|s| !s.is_empty()) {
        let valid = state
            .db
            .get_user_by_email(&user.email)
            .map(|(_, hash)| auth_service::verify_password(pw, &state.config.session_secret, &hash))
            .unwrap_or(false);
        if !valid {
            return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::LOGIN_FAILED);
        }
    }

    if let Some(raw_token) = extract_raw_session_cookie(&headers) {
        auth_service::revoke_session_token(raw_token);
    }

    let client_ip = extract_client_ip(&headers);
    if let Err(e) = state.db.delete_user_account(user.id, &client_ip) {
        return ApiResponse::server_error(&state.translations, "tr", "delete_account", &e)
            .into_response();
    }

    let secure = cookie_secure_flag(&headers, &state.config.public_url);
    let cookie = format!(
        "msi_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}",
        secure
    );

    (
        [(header::SET_COOKIE, cookie)],
        ApiResponse::ok(json!({ "deleted": true })).into_response(),
    )
        .into_response()
}

// ============================================================================
// TELEMETRY, COMMERCE & PAYTR CHECKOUT
// ============================================================================

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

pub async fn checkout_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CheckoutPayload>,
) -> Response {
    let lang = "tr";
    let email = match validation_engine::validate_email(&payload.user_email) {
        validation_engine::EmailValidation::Valid(e) => e,
        _ => {
            return ApiResponse::error(
                &state.translations,
                lang,
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::CHECKOUT_FIELDS_REQUIRED,
            )
            .into_response()
        }
    };
    if matches!(
        validation_engine::validate_required_text(&payload.full_name),
        validation_engine::TextFieldValidation::Empty
    ) {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::CHECKOUT_FIELDS_REQUIRED,
        )
        .into_response();
    }
    if payload.distance_sales_consent == Some(false) {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::DISTANCE_SALES_CONSENT_REQUIRED,
        )
        .into_response();
    }

    let _ = (&payload.item_title, payload.amount_try);
    let Some((canonical_type, canonical_title, authoritative_price)) =
        resolve_catalog_item(&state, &payload.item_type, &payload.item_code)
    else {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::PLAN_NOT_FOUND,
        )
        .into_response();
    };

    let org = payload
        .organization_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Kurumsal Savunma Paydaşı".to_string());

    let mut order = subscription_service::create_invoice_record(
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
        order.payment_status = "pending_bank_transfer".to_string();
    }

    let inv_id = match state.db.insert_order_invoice(&order) {
        Ok(id) => id,
        Err(e) => {
            return ApiResponse::server_error_with_key(
                &state.translations,
                lang,
                "checkout_order",
                &e,
                msg_keys::SERVER_ERROR_ORDER,
            )
            .into_response();
        }
    };

    if paytr_config.is_configured() {
        let client_ip = extract_client_ip(&headers);
        let merchant_oid = payment_service::generate_merchant_oid(&order.invoice_no);
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
            amount_cents: (order.amount_try * 100.0).round() as i64,
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
                "invoice_no": order.invoice_no,
                "test_mode": paytr_config.test_mode
            }))
            .into_response(),
            Err(e) => {
                tracing::error!(
                    paytr_error = %e,
                    invoice_no = %order.invoice_no,
                    "PayTR checkout session failed"
                );
                ApiResponse::payment_gateway_error(&state.translations, lang, &e).into_response()
            }
        };
    }

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "status": "order_recorded_pending_payment",
            "invoice_id": inv_id,
            "invoice_no": order.invoice_no,
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
        if let Some(invoice) = state.db.finalize_paid_invoice(&payload.merchant_oid) {
            // Activate/extend subscription after successful payment
            if let Err(e) = subscription_service::activate_subscription_after_payment(
                &state.db,
                &invoice.user_email,
                &invoice.item_code,
            ) {
                tracing::error!(
                    "Subscription activation failed for invoice {}: {}",
                    invoice.invoice_no,
                    e
                );
            }
        }
    } else {
        let _ = state.db.mark_invoice_failed(&payload.merchant_oid);
    }

    (StatusCode::OK, "OK").into_response()
}

pub async fn faceted_search_api(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let criteria = CentralContentQueryEngine::from_params(&params);
    let result = CentralContentQueryEngine::execute(&state.db, &criteria);
    Json(json!(result))
}

pub async fn list_notifications_api(
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let session_u = auth_service::session_user(&state, &headers);
    let is_admin_route = uri.path().starts_with("/api/v1/admin/")
        && session_u
            .as_ref()
            .map(|u| is_staff_role(&state.db, &u.role))
            .unwrap_or(false);

    let (email_opt, org_opt) = if is_admin_route {
        (
            params.get("email").map(|s| s.as_str()),
            params.get("org").map(|s| s.as_str()),
        )
    } else if let Some(ref u) = session_u {
        (
            Some(u.email.as_str()),
            Some(u.organization_category.as_str()),
        )
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
                    && matches!(
                        n.target_audience.to_ascii_lowercase().as_str(),
                        "all" | "public" | "global" | ""
                    )
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
    headers: HeaderMap,
) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };

    let Some(notif) = state.db.get_notification_by_id(id) else {
        return json_error(&state, StatusCode::NOT_FOUND, msg_keys::BAD_REQUEST);
    };

    if let Some(target_email) = notif.target_email.as_deref().filter(|e| !e.trim().is_empty()) {
        if !target_email.eq_ignore_ascii_case(&user.email)
            && !is_staff_role(&state.db, &user.role)
        {
            return json_error(&state, StatusCode::FORBIDDEN, msg_keys::AUTH_REQUIRED);
        }
    }

    let ok = state.db.mark_notification_read(id).is_ok();
    ApiResponse::ok(json!({
        "status": if ok { "marked_read" } else { "error" },
        "marked_read": ok,
        "id": id
    }))
    .into_response()
}

pub async fn list_site_settings_api(State(state): State<AppState>) -> Json<Value> {
    let settings: Vec<SiteSetting> = state
        .db
        .get_site_settings()
        .into_iter()
        .filter(|s| {
            !s.key.starts_with("user_email_prefs.") && s.setting_group != "user_preferences"
        })
        .collect();

    let mut map_tr = state.db.get_setting_map("tr");
    map_tr.retain(|k, _| !k.starts_with("user_email_prefs."));
    let mut map_en = state.db.get_setting_map("en");
    map_en.retain(|k, _| !k.starts_with("user_email_prefs."));

    Json(json!({
        "settings": settings,
        "map_tr": map_tr,
        "map_en": map_en
    }))
}

// ============================================================================
// ADMIN OMS / CMS ENDPOINTS (/api/v1/admin/*)
// ============================================================================

pub async fn admin_overview(
    _operator: OperatorUser,
    State(state): State<AppState>,
) -> Json<Value> {
    let contents = state.db.get_all_contents();
    let ads = state.db.get_all_ads();
    let users = state.db.get_all_users();
    let corp_reqs = state.db.get_corporate_requests();
    let reminders = state.db.get_renewal_reminders();
    let invoices = state.db.get_order_invoices();

    let pending_reqs = corp_reqs.iter().filter(|r| r.status == "pending").count();
    let pending_approvals = state.db.list_pending_approvals().len();
    let total_views: i64 = contents.iter().map(|c| c.view_count).sum();
    let total_true_reads: i64 = contents.iter().map(|c| c.true_read_count).sum();

    Json(json!({
        "total_contents": contents.len(),
        "total_views": total_views,
        "total_true_reads": total_true_reads,
        "active_ads": ads.iter().filter(|a| a.is_active).count(),
        "registered_users": users.len(),
        "pending_corporate_requests": pending_reqs,
        "pending_approvals": pending_approvals,
        "scheduled_reminders": reminders.iter().filter(|r| r.status == "scheduled").count(),
        "halted_reminders": reminders.iter().filter(|r| r.status == "halted_by_renewal").count(),
        "invoices_count": invoices.len()
    }))
}

pub async fn list_admin_contents(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:read") {
        return resp;
    }
    let criteria = CentralContentQueryEngine::from_params(&params);
    let result = CentralContentQueryEngine::execute_admin(&state.db, &criteria);
    ApiResponse::ok_with_meta(
        json!(result.items),
        json!({
            "page": result.current_page,
            "per_page": result.per_page,
            "total_count": result.total_count,
            "total_pages": result.total_pages
        }),
    )
    .into_response()
}

pub async fn admin_upsert_content(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(mut item): Json<ContentItem>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:write") {
        return resp;
    }
    let wants_publish = item.status.trim().is_empty() || item.status.trim() == "published";
    if wants_publish {
        if let Err(resp) = require_perm(&operator, "contents:publish") {
            return resp;
        }
    }

    item.slug = validation_engine::validate_slug(&item.slug)
        .unwrap_or_else(|| validation_engine::validate_slug(&item.title_tr).unwrap_or_default());
    item.body_tr = validation_engine::sanitize_html(&item.body_tr);
    item.body_en = validation_engine::sanitize_html(&item.body_en);

    let is_update = item.id > 0;
    match state.db.upsert_content(&item) {
        Ok(id) => {
            CentralContentQueryEngine::invalidate_cache();
            let change_note = if is_update {
                format!("İçerik güncellendi ({})", item.status)
            } else {
                format!("İlk kayıt oluşturuldu ({})", item.status)
            };
            let _ = state.db.insert_content_version(
                id,
                &item.title_tr,
                &item.body_tr,
                &item.summary_tr,
                &operator.email,
                &change_note,
            );

            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                if is_update { "update" } else { "create" },
                "content",
                Some(id),
                &json!({ "slug": item.slug, "status": item.status }).to_string(),
                &client_ip,
            );

            let indexnow_queued = if wants_publish {
                crate::services::indexnow_service::spawn_instant_seo_push(
                    (*state.config).clone(),
                    vec![
                        crate::engine::url_builder::article_url(&item.slug),
                        "/".to_string(),
                        crate::engine::url_builder::category_url(&item.content_type),
                    ],
                );
                true
            } else {
                false
            };

            ApiResponse::ok(json!({
                "status": "saved",
                "id": id,
                "slug": item.slug,
                "placements_count": item.placements.len(),
                "indexnow_queued": indexnow_queued
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error_with_key(
            &state.translations,
            "tr",
            "admin_upsert_content",
            &e,
            msg_keys::SERVER_ERROR_SAVE,
        )
        .into_response(),
    }
}

pub async fn admin_delete_content(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:delete") {
        return resp;
    }
    let _ = state.db.delete_content(id);
    CentralContentQueryEngine::invalidate_cache();
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "delete",
        "content",
        Some(id),
        "{}",
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "deleted", "id": id})).into_response()
}

pub async fn admin_ping_indexnow(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:write", "contents:publish"]) {
        return resp;
    }
    let mut urls: Vec<String> = vec![
        "/".to_string(),
        "/e-dergi".to_string(),
        "/sitemap.xml".to_string(),
        "/sitemap-news.xml".to_string(),
        "/llms.txt".to_string(),
    ];
    for c in state
        .db
        .get_all_contents()
        .into_iter()
        .filter(|c| content_service::is_publicly_visible(c))
        .take(crate::db::constants::API_FEED_CONTENT_LIMIT)
    {
        urls.push(crate::engine::url_builder::article_url(&c.slug));
    }
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "indexnow_ping",
        "seo",
        None,
        &json!({ "pushed_count": urls.len() }).to_string(),
        &client_ip,
    );
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

pub async fn admin_upload_media(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Response {
    if let Err(resp) = require_perm(&operator, "media:upload") {
        return resp;
    }

    let mut all_urls: Vec<String> = Vec::new();
    let mut total_bytes: usize = 0;

    while let Ok(Some(field)) = multipart.next_field().await {
        let Some(orig_name) = field.file_name().map(|s| s.to_string()) else {
            continue;
        };
        if orig_name.trim().is_empty() {
            continue;
        }
        let ext = FsPath::new(&orig_name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        if !media_service::is_allowed_extension(&ext) {
            return ApiResponse::error(
                &state.translations,
                "tr",
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::INVALID_FILE_TYPE,
            )
            .into_response();
        }

        let bytes = match field.bytes().await {
            Ok(b) if media_service::is_valid_size(b.len()) => b,
            _ => {
                return ApiResponse::error(
                    &state.translations,
                    "tr",
                    StatusCode::BAD_REQUEST,
                    error_codes::BAD_REQUEST,
                    msg_keys::FILE_TOO_LARGE,
                )
                .into_response();
            }
        };

        let (relative_url, filename) = media_service::generate_upload_path(
            &orig_name,
            &ext,
            security_engine::sanitize_slug,
        );

        let subdir = relative_url
            .rsplit_once('/')
            .map(|(d, _)| d)
            .unwrap_or("images/uploads");
        let target_dir = format!(
            "{}/{}",
            state.config.static_dir.trim_end_matches('/'),
            subdir
        );
        if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
            return ApiResponse::server_error_with_key(
                &state.translations,
                "tr",
                "admin_upload_mkdir",
                &e,
                msg_keys::SERVER_ERROR_UPLOAD,
            )
            .into_response();
        }

        let full_path = format!("{}/{}", target_dir, filename);
        if let Err(e) = tokio::fs::write(&full_path, &bytes).await {
            return ApiResponse::server_error_with_key(
                &state.translations,
                "tr",
                "admin_upload_write",
                &e,
                msg_keys::SERVER_ERROR_FILE_WRITE,
            )
            .into_response();
        }

        if let Err(e) = media_service::register_media(
            &state.db,
            &filename,
            &orig_name,
            &ext,
            bytes.len(),
            &relative_url,
            &operator.email,
        ) {
            tracing::warn!("Media library registration failed (file saved): {}", e);
        }

        total_bytes += bytes.len();
        all_urls.push(relative_url);
    }

    let Some(first_url) = all_urls.first().cloned() else {
        return ApiResponse::error(
            &state.translations,
            "tr",
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::UPLOAD_NO_FIELD,
        )
        .into_response();
    };

    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upload",
        "media",
        None,
        &json!({ "count": all_urls.len(), "first_url": first_url }).to_string(),
        &client_ip,
    );

    Json(json!({
        "status": "uploaded",
        "url": first_url,
        "path": first_url,
        "urls": all_urls,
        "size_bytes": total_bytes
    }))
    .into_response()
}

pub async fn list_admin_ads(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "ads:read") {
        return resp;
    }
    Json(state.db.get_all_ads()).into_response()
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
    operator: OperatorUser,
    State(state): State<AppState>,
    Json(payload): Json<CheckConflictPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "ads:read") {
        return resp;
    }
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

    Json(json!(warning)).into_response()
}

pub async fn admin_upsert_ad(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(ad): Json<AdCampaign>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "ads:write") {
        return resp;
    }
    let existing = state.db.get_all_ads();
    let conflict =
        ad_service::check_ad_conflict(&ad, &existing, state.config.ad_max_quota_per_keyword);
    let is_update = ad.id > 0;

    match state.db.upsert_ad(&ad) {
        Ok(id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                if is_update { "update" } else { "create" },
                "ad_campaign",
                Some(id),
                &json!({ "sponsor_name": ad.sponsor_name, "slot_code": ad.slot_code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "saved",
                "id": id,
                "conflict_check": conflict
            }))
            .into_response()
        }
        Err(e) => AppError::internal("ad_upsert", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response(),
    }
}

pub async fn admin_delete_ad(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "ads:write") {
        return resp;
    }
    match state.db.delete_ad_campaign(id) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "ad_campaign",
                Some(id),
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "id": id })).into_response()
        }
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_delete_ad", &e)
            .into_response(),
    }
}

// ============================================================================
// EDITORIAL, REVENUE & USER ANALYTICS ENDPOINTS
// ============================================================================

pub async fn admin_analytics_bundle(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
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
    .into_response()
}

pub async fn admin_analytics_contents(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let reports: Vec<_> = contents
        .iter()
        .map(|c| analytics_service::compute_content_report(c, &events))
        .collect();
    Json(json!(reports)).into_response()
}

pub async fn admin_analytics_ads(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "ads:read"]) {
        return resp;
    }
    let ads = state.db.get_all_ads();
    let events = state.db.get_all_analytics_events();
    let reports: Vec<_> = ads
        .iter()
        .map(|a| analytics_service::compute_ad_report(a, &events))
        .collect();
    Json(json!(reports)).into_response()
}

pub async fn admin_analytics_readers(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "users:read"]) {
        return resp;
    }
    let users = state.db.get_all_users();
    let ads = state.db.get_all_ads();
    let events = state.db.get_all_analytics_events();
    let reports = analytics_service::compute_reader_reports(&users, &events, &ads);
    Json(json!(reports)).into_response()
}

pub async fn admin_daily_analytics(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let (from, to) = default_date_bounds(&params);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if to >= today {
        let _ = state.db.aggregate_analytics_for_date(&today);
    }
    let rows = state.db.get_daily_aggregated_analytics(&from, &to);
    ApiResponse::ok(json!(rows)).into_response()
}

pub async fn admin_author_performance(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let authors = analytics_service::compute_author_performance(&contents, &events);
    ApiResponse::ok(json!(authors)).into_response()
}

pub async fn admin_category_performance(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let categories = analytics_service::compute_category_performance(&contents, &events);
    ApiResponse::ok(json!(categories)).into_response()
}

pub async fn admin_trending_content(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let trending = analytics_service::compute_trending_content(&contents, &events, limit);
    ApiResponse::ok(json!(trending)).into_response()
}

pub async fn admin_revenue_summary(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "revenue:read") {
        return resp;
    }
    let (from, to) = default_date_bounds(&params);
    match revenue_analytics_service::get_revenue_summary(&state.db, &from, &to) {
        Ok(summary) => ApiResponse::ok(summary).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "revenue_summary", &e)
            .into_response(),
    }
}

pub async fn admin_keyword_performance(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "ads:read", "revenue:read"])
    {
        return resp;
    }
    let (from, to) = default_date_bounds(&params);
    match revenue_analytics_service::get_keyword_performance(&state.db, &from, &to) {
        Ok(rows) => ApiResponse::ok(json!(rows)).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "keyword_performance", &e)
            .into_response(),
    }
}

pub async fn admin_advertiser_report(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "ads:read", "revenue:read"])
    {
        return resp;
    }
    let (from, to) = default_date_bounds(&params);
    match revenue_analytics_service::get_advertiser_report(&state.db, &from, &to) {
        Ok(rows) => ApiResponse::ok(json!(rows)).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "advertiser_report", &e)
            .into_response(),
    }
}

pub async fn admin_user_cohorts(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "users:read"]) {
        return resp;
    }
    match user_analytics_service::get_cohort_retention(&state.db) {
        Ok(cohorts) => ApiResponse::ok(json!(cohorts)).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "user_cohorts", &e)
            .into_response(),
    }
}

pub async fn admin_churn_rate(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["analytics:read", "revenue:read", "subscriptions:read"])
    {
        return resp;
    }
    match user_analytics_service::get_churn_rate(&state.db) {
        Ok(churn) => ApiResponse::ok(churn).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "churn_rate", &e)
            .into_response(),
    }
}

pub async fn admin_user_ltv(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "revenue:read"]) {
        return resp;
    }
    match user_analytics_service::get_lifetime_value(&state.db) {
        Ok(ltv) => ApiResponse::ok(ltv).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "user_ltv", &e)
            .into_response(),
    }
}

pub async fn admin_lifetime_value(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    admin_user_ltv(operator, State(state)).await
}

pub async fn admin_conversion_funnel(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "revenue:read"]) {
        return resp;
    }
    match user_analytics_service::get_conversion_funnel(&state.db) {
        Ok(funnel) => ApiResponse::ok(funnel).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "conversion_funnel", &e)
            .into_response(),
    }
}

// ============================================================================
// CORPORATE REQUESTS, TAXONOMY, NEWSLETTERS, REMINDERS, REDIRECTS & SETTINGS
// ============================================================================

pub async fn admin_list_corporate_requests(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "corporate_requests:read") {
        return resp;
    }
    Json(state.db.get_corporate_requests()).into_response()
}

#[derive(Deserialize)]
pub struct CorporateDecisionPayload {
    pub id: Option<i64>,
    pub status: String,
    pub reviewed_by: Option<String>,
}

pub async fn admin_decide_corporate_request_by_path(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<CorporateDecisionPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "corporate_requests:decide") {
        return resp;
    }
    let reviewer = payload
        .reviewed_by
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| operator.email.clone());
    let _ = state
        .db
        .decide_corporate_request(id, &payload.status, &reviewer);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "decide_corporate_request",
        "corporate_request",
        Some(id),
        &json!({ "decision": payload.status }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "updated", "id": id, "decision": payload.status}))
        .into_response()
}

pub async fn admin_decide_corporate_request_body(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CorporateDecisionPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "corporate_requests:decide") {
        return resp;
    }
    let id = payload.id.unwrap_or(1);
    let reviewer = payload
        .reviewed_by
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| operator.email.clone());
    let _ = state
        .db
        .decide_corporate_request(id, &payload.status, &reviewer);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "decide_corporate_request",
        "corporate_request",
        Some(id),
        &json!({ "decision": payload.status }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "updated", "id": id, "decision": payload.status}))
        .into_response()
}

pub async fn admin_list_categories(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:read", "contents:read"]) {
        return resp;
    }
    Json(state.db.get_all_categories()).into_response()
}

pub async fn admin_insert_category(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(cat): Json<CategoryNode>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:write", "settings:write"]) {
        return resp;
    }
    if matches!(
        validation_engine::validate_required_text(&cat.name_tr),
        validation_engine::TextFieldValidation::Empty
    ) || validation_engine::validate_slug(&cat.slug).is_none()
    {
        return AppError::ValidationError {
            msg_key: msg_keys::VALIDATION_ERROR,
        }
        .into_api_response(&state.translations, "tr")
        .into_response();
    }
    let id = state.db.insert_category(&cat).unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upsert",
        "category",
        Some(id),
        &json!({ "slug": cat.slug }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "saved", "id": id, "slug": cat.slug})).into_response()
}

pub async fn admin_list_ssb_tags(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["categories:read", "contents:read", "ads:read"])
    {
        return resp;
    }
    Json(state.db.get_all_ssb_tags()).into_response()
}

pub async fn admin_insert_ssb_tag(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(tag): Json<SsbInterestTag>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:write", "settings:write"]) {
        return resp;
    }
    let id = state.db.insert_ssb_tag(&tag).unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upsert",
        "ssb_tag",
        Some(id),
        &json!({ "code": tag.code }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "saved", "id": id, "code": tag.code})).into_response()
}

pub async fn admin_list_newsletters(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["newsletters:read", "analytics:read"]) {
        return resp;
    }
    let (campaigns, total_count) = state
        .db
        .get_newsletter_campaigns_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(campaigns), pagination.meta(total_count)).into_response()
}

#[derive(Deserialize)]
pub struct AdminNewsletterPayload {
    #[serde(flatten)]
    pub campaign: NewsletterCampaign,
    pub body_html: Option<String>,
}

pub async fn admin_insert_newsletter(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AdminNewsletterPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "newsletters:write") {
        return resp;
    }
    let mut nl = payload.campaign;
    let users = state.db.get_all_users();
    let recipients = email_service::filter_newsletter_recipients(
        &users,
        &nl.target_group_filter,
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
        &state.db,
        &nl,
        &recipients,
        sponsor_ad.as_ref(),
        payload.body_html.as_deref(),
    )
    .await;

    let id = state.db.insert_newsletter_campaign(&nl).unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "create",
        "newsletter_campaign",
        Some(id),
        &json!({ "subject_tr": nl.subject_tr, "recipient_count": nl.recipient_count }).to_string(),
        &client_ip,
    );

    ApiResponse::ok(json!({
        "status": nl.status,
        "id": id,
        "recipient_count": nl.recipient_count,
        "dispatch": dispatch
    }))
    .into_response()
}

pub async fn admin_list_reminders(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["subscriptions:read", "analytics:read"]) {
        return resp;
    }
    let users = state.db.get_all_users();
    let existing = state.db.get_renewal_reminders();
    let evaluated = subscription_service::evaluate_5_stage_reminders(&users, &existing);
    let _ = state.db.sync_evaluated_reminders(&evaluated);
    let (items, total_count) = state
        .db
        .get_renewal_reminders_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(items), pagination.meta(total_count)).into_response()
}

pub async fn admin_halt_renewed_reminders(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["subscriptions:write", "subscriptions:read"]) {
        return resp;
    }
    let halted_count = state.db.halt_renewed_reminders().unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "halt_renewed_reminders",
        "renewal_reminder",
        None,
        &json!({ "halted_count": halted_count }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({
        "status": "halted_by_renewal",
        "halted_count": halted_count
    }))
    .into_response()
}

pub async fn admin_list_redirects(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:read", "contents:read"]) {
        return resp;
    }
    let (redirects, total_count) = state
        .db
        .get_seo_redirects_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(redirects), pagination.meta(total_count)).into_response()
}

pub async fn admin_insert_redirect(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(rd): Json<SeoRedirectRule>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:write", "contents:publish"]) {
        return resp;
    }
    let id = state.db.insert_seo_redirect(&rd).unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upsert",
        "seo_redirect",
        Some(id),
        &json!({ "old_path": rd.old_path, "new_path": rd.new_path }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "saved", "id": id})).into_response()
}

pub async fn admin_export_open_data(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:export", "settings:write"]) {
        return resp;
    }
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "export_open_data",
        "system",
        None,
        "{}",
        &client_ip,
    );

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

pub async fn admin_list_notifications_api(
    operator: OperatorUser,
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
    query: Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["notifications:read", "settings:read"]) {
        return resp;
    }
    list_notifications_api(State(state), uri, headers, query)
        .await
        .into_response()
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
    pub target_email: Option<String>,
    pub action_url: Option<String>,
    pub is_banner: Option<bool>,
}

pub async fn admin_create_notification_api(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateNotificationPayload>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["notifications:write", "settings:write"]) {
        return resp;
    }
    let raw_audience = payload.target_audience.as_deref().unwrap_or("all").trim();
    let normalized_audience = if raw_audience.is_empty() || raw_audience.eq_ignore_ascii_case("all")
    {
        "all"
    } else {
        raw_audience
    };

    let notif = crate::engine::notification_engine::build_notification_event(
        normalized_audience,
        payload
            .target_email
            .as_deref()
            .map(str::trim)
            .filter(|e| !e.is_empty()),
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
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "create",
        "notification",
        Some(id),
        &json!({ "audience": normalized_audience, "title_tr": payload.title_tr }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "created", "id": id, "notification": notif})).into_response()
}

pub async fn admin_list_settings_api(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    list_site_settings_api(State(state)).await.into_response()
}

#[derive(Deserialize)]
pub struct UpsertSettingPayload {
    pub key: String,
    pub value_tr: String,
    pub value_en: Option<String>,
    pub setting_group: Option<String>,
}

pub async fn admin_upsert_setting_api(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<UpsertSettingPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
    if payload.key == crate::engine::design::config::SETTING_KEY {
        let parsed =
            serde_json::from_str::<crate::engine::design::config::DesignConfig>(&payload.value_tr);
        if parsed
            .map_err(|e| e.to_string())
            .and_then(|c| c.validate())
            .is_err()
        {
            return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
        }
        if let Some(en) = payload.value_en.as_ref().filter(|v| !v.is_empty()) {
            if serde_json::from_str::<crate::engine::design::config::DesignConfig>(en)
                .map_err(|e| e.to_string())
                .and_then(|c| c.validate())
                .is_err()
            {
                return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
            }
        }
    }
    let val_en = payload.value_en.as_deref().unwrap_or(&payload.value_tr);
    let grp = payload.setting_group.as_deref().unwrap_or("general");
    let setting = SiteSetting {
        key: payload.key.clone(),
        value_tr: payload.value_tr.clone(),
        value_en: val_en.to_string(),
        setting_group: grp.to_string(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let ok = state.db.upsert_site_setting(&setting).is_ok();
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upsert",
        "site_setting",
        None,
        &json!({ "key": payload.key }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({
        "key": payload.key,
        "updated": ok
    }))
    .into_response()
}

pub async fn admin_list_security_logs_api(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["audit:read", "settings:read"]) {
        return resp;
    }
    let logs = state.db.get_security_audit_logs();
    Json(json!({
        "total": logs.len(),
        "logs": logs
    }))
    .into_response()
}

// ============================================================================
// TRANSLATIONS, CONTENT VERSIONS, MEDIA LIBRARY, AUDIT LOGS, ROLES & USERS
// ============================================================================

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

pub async fn admin_list_translations_api(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["translations:read", "settings:read"]) {
        return resp;
    }
    let (entries, total_count) = state
        .db
        .load_all_translations_paginated(pagination.effective_per_page(), pagination.offset());
    let items: Vec<Value> = entries
        .into_iter()
        .map(|e| {
            json!({
                "key": e.key,
                "value_tr": e.value_tr,
                "value_en": e.value_en,
                "value_ar": e.value_ar,
                "value_de": e.value_de,
                "value_ru": e.value_ru,
                "value_fr": e.value_fr,
                "group": e.group,
            })
        })
        .collect();
    ApiResponse::ok_with_meta(json!(items), pagination.meta(total_count)).into_response()
}

pub async fn admin_upsert_translation_api(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<TranslationUpsertPayload>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["translations:write", "settings:write"]) {
        return resp;
    }
    let existing = state.translations.get_entry(&payload.key);
    let entry = crate::engine::translation_engine::TranslationEntry {
        key: payload.key.clone(),
        value_tr: payload
            .value_tr
            .or_else(|| existing.as_ref().map(|e| e.value_tr.clone()))
            .unwrap_or_default(),
        value_en: payload
            .value_en
            .or_else(|| existing.as_ref().map(|e| e.value_en.clone()))
            .unwrap_or_default(),
        value_ar: payload
            .value_ar
            .or_else(|| existing.as_ref().map(|e| e.value_ar.clone()))
            .unwrap_or_default(),
        value_de: payload
            .value_de
            .or_else(|| existing.as_ref().map(|e| e.value_de.clone()))
            .unwrap_or_default(),
        value_ru: payload
            .value_ru
            .or_else(|| existing.as_ref().map(|e| e.value_ru.clone()))
            .unwrap_or_default(),
        value_fr: payload
            .value_fr
            .or_else(|| existing.as_ref().map(|e| e.value_fr.clone()))
            .unwrap_or_default(),
        group: payload
            .group
            .or_else(|| existing.map(|e| e.group))
            .unwrap_or_else(|| "general".to_string()),
    };
    if let Err(e) = state.db.upsert_translation(&entry) {
        return AppError::internal("translation_upsert", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response();
    }
    state.translations.reload(state.db.load_all_translations());
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upsert",
        "translation",
        None,
        &json!({ "key": payload.key }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"key": payload.key})).into_response()
}

pub async fn admin_delete_translation_api(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["translations:write", "settings:write"]) {
        return resp;
    }
    if let Err(e) = state.db.delete_translation(&key) {
        return AppError::internal("translation_delete", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response();
    }
    state.translations.reload(state.db.load_all_translations());
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "delete",
        "translation",
        None,
        &json!({ "key": key }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"key": key, "action": "deleted"})).into_response()
}

pub async fn admin_list_content_versions(
    operator: OperatorUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:read") {
        return resp;
    }
    let (versions, total_count) = state.db.get_content_versions_paginated(
        id,
        pagination.effective_per_page(),
        pagination.offset(),
    );
    ApiResponse::ok_with_meta(json!(versions), pagination.meta(total_count)).into_response()
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
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<ContentVersionPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:write") {
        return resp;
    }
    match state.db.insert_content_version(
        payload.content_id,
        &payload.title_tr,
        &payload.body_tr,
        &payload.summary_tr,
        &operator.email,
        &payload.change_note.unwrap_or_default(),
    ) {
        Ok(id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "create_version",
                "content",
                Some(payload.content_id),
                &json!({ "version_id": id }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "created", "version_id": id })).into_response()
        }
        Err(e) => ApiResponse::server_error(
            &state.translations,
            "tr",
            "admin_create_content_version",
            &e,
        )
        .into_response(),
    }
}

pub async fn admin_list_media_library(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["media:read", "media:upload", "contents:read"])
    {
        return resp;
    }
    let (items, total_count) = state
        .db
        .get_media_library_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(items), pagination.meta(total_count)).into_response()
}

pub async fn admin_list_audit_logs(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["audit:read", "users:manage", "settings:read"])
    {
        return resp;
    }
    let (logs, total_count) = state
        .db
        .get_audit_logs_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(logs), pagination.meta(total_count)).into_response()
}

#[derive(Deserialize)]
pub struct AuditLogPayload {
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub details_json: Option<String>,
}

pub async fn admin_insert_audit_log(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AuditLogPayload>,
) -> Response {
    let client_ip = extract_client_ip(&headers);
    match state.db.insert_audit_log(
        &operator.email,
        &payload.action,
        &payload.entity_type,
        payload.entity_id,
        &payload.details_json.unwrap_or_else(|| "{}".to_string()),
        &client_ip,
    ) {
        Ok(id) => ApiResponse::ok(json!({ "status": "logged", "id": id })).into_response(),
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_insert_audit_log", &e)
                .into_response()
        }
    }
}

pub async fn admin_list_users(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["users:read", "users:manage"]) {
        return resp;
    }
    let all_users: Vec<UserAccount> = state
        .db
        .get_all_users()
        .into_iter()
        .filter(|u| u.role != "deleted")
        .collect();
    let total_count = all_users.len() as u64;
    let offset = pagination.offset() as usize;
    let per_page = pagination.effective_per_page() as usize;
    let page_items: Vec<UserAccount> = all_users.into_iter().skip(offset).take(per_page).collect();
    ApiResponse::ok_with_meta(json!(page_items), pagination.meta(total_count)).into_response()
}

pub async fn admin_list_user_roles(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["users:read", "users:manage"]) {
        return resp;
    }
    let (roles, total_count) = state
        .db
        .get_user_roles_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(roles), pagination.meta(total_count)).into_response()
}

#[derive(Deserialize)]
pub struct UserRolePayload {
    pub role_code: String,
    #[serde(default)]
    pub name_tr: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub permissions_json: String,
}

fn is_builtin_system_role(state: &AppState, role_code: &str) -> bool {
    state
        .db
        .get_user_roles()
        .iter()
        .any(|r| {
            r.get("role_code")
                .and_then(|v| v.as_str())
                .map(|c| c.eq_ignore_ascii_case(role_code.trim()))
                .unwrap_or(false)
                && r.get("is_system")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        })
}

pub async fn admin_upsert_user_role(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<UserRolePayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "users:manage") {
        return resp;
    }
    let code = payload.role_code.trim();
    if code.is_empty() || code.len() > 64 {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    if is_builtin_system_role(&state, code) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Sistem rollerinin izinleri değiştirilemez.",
                "data": null
            })),
        )
            .into_response();
    }
    if !payload.permissions_json.trim().is_empty()
        && serde_json::from_str::<Vec<String>>(&payload.permissions_json).is_err()
    {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    match state.db.upsert_user_role(
        code,
        &payload.name_tr,
        &payload.name_en,
        &payload.permissions_json,
    ) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert_role",
                "user_role",
                None,
                &json!({ "role_code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "saved", "role_code": code })).into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_upsert_user_role", &e)
                .into_response()
        }
    }
}

pub async fn admin_update_user_role(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(role_code): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "users:manage") {
        return resp;
    }
    let code = role_code.trim();
    if is_builtin_system_role(&state, code) && body.get("permissions_json").is_some() {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Sistem rollerinin izinleri değiştirilemez.",
                "data": null
            })),
        )
            .into_response();
    }

    let name_tr = body.get("name_tr").and_then(|v| v.as_str()).unwrap_or("");
    let name_en = body.get("name_en").and_then(|v| v.as_str()).unwrap_or("");
    let perms_raw = body
        .get("permissions_json")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !perms_raw.is_empty() && serde_json::from_str::<Vec<String>>(perms_raw).is_err() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    match state.db.upsert_user_role(code, name_tr, name_en, perms_raw) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "update_role",
                "user_role",
                None,
                &json!({ "role_code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "updated", "role_code": code })).into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_update_user_role", &e)
                .into_response()
        }
    }
}

pub async fn admin_delete_user_role(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(role_code): Path<String>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "users:manage") {
        return resp;
    }
    let code = role_code.trim();
    if is_builtin_system_role(&state, code) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Sistem rolleri silinemez.",
                "data": null
            })),
        )
            .into_response();
    }

    match state.db.delete_user_role(code) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete_role",
                "user_role",
                None,
                &json!({ "role_code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "role_code": code })).into_response()
        }
        Ok(false) => json_error(&state, StatusCode::NOT_FOUND, msg_keys::BAD_REQUEST),
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_delete_user_role", &e)
                .into_response()
        }
    }
}

pub async fn admin_assign_role_to_user(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    Json(body): Json<Value>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "users:manage") {
        return resp;
    }
    let role_code = body
        .get("role_code")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if role_code.is_empty() || role_code.len() > 64 {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    match state.db.update_user_role(user_id, role_code) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "assign_role",
                "user",
                Some(user_id),
                &json!({ "role_code": role_code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "assigned",
                "user_id": user_id,
                "role_code": role_code
            }))
            .into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_assign_role_to_user", &e)
                .into_response()
        }
    }
}

// ============================================================================
// CUSTOM FIELDS, CACHE FLUSH, SYSTEM HEALTH & BULK CONTENT OPERATIONS
// ============================================================================

pub async fn admin_list_custom_fields(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:read", "contents:read"]) {
        return resp;
    }
    let applies_to = params
        .get("applies_to")
        .map(|s| s.as_str())
        .unwrap_or("content");
    let (fields, total_count) = state.db.get_custom_fields_paginated(
        applies_to,
        pagination.effective_per_page(),
        pagination.offset(),
    );
    ApiResponse::ok_with_meta(json!(fields), pagination.meta(total_count)).into_response()
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
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CustomFieldPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
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
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert",
                "custom_field",
                None,
                &json!({ "field_key": payload.field_key }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "saved", "field_key": payload.field_key }))
                .into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_upsert_custom_field", &e)
                .into_response()
        }
    }
}

pub async fn admin_delete_custom_field(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(field_key): Path<String>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
    match state.db.delete_custom_field(&field_key) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "custom_field",
                None,
                &json!({ "field_key": field_key }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "field_key": field_key })).into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_delete_custom_field", &e)
                .into_response()
        }
    }
}

pub async fn admin_list_cache_operations(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    let (ops, total_count) = state
        .db
        .get_cache_operations_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(ops), pagination.meta(total_count)).into_response()
}

pub async fn admin_flush_cache(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
    let cache_key = payload
        .get("cache_key")
        .map(|s| s.as_str())
        .unwrap_or("all");

    state.db.refresh_l1_cache();
    state.translations.reload(state.db.load_all_translations());
    CentralContentQueryEngine::invalidate_cache();

    let _ = state.db.insert_cache_operation(
        cache_key,
        "flush",
        &operator.email,
        "L1 DB + Translation + ContentQuery cache flushed",
    );
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "flush",
        "cache",
        None,
        &json!({ "cache_key": cache_key }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({ "status": "flushed", "cache_key": cache_key })).into_response()
}

pub async fn admin_system_health(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:read", "analytics:read"]) {
        return resp;
    }
    Json(state.db.get_system_health()).into_response()
}

#[derive(Deserialize)]
pub struct BulkContentPayload {
    pub ids: Vec<i64>,
    pub action: String,
    pub value: Option<String>,
}

pub async fn admin_bulk_content_operation(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<BulkContentPayload>,
) -> Response {
    let required = if payload.action == "delete" {
        "contents:delete"
    } else {
        "contents:write"
    };
    if let Err(resp) = require_perm(&operator, required) {
        return resp;
    }

    let mut affected = 0usize;
    for id in &payload.ids {
        match payload.action.as_str() {
            "delete" => {
                if state.db.delete_content(*id).is_ok() {
                    affected += 1;
                }
            }
            "set_visibility" => {
                if let Some(level) = payload.value.as_deref().and_then(|v| v.parse::<i32>().ok()) {
                    if let Some(mut c) = state.db.get_all_contents().into_iter().find(|c| c.id == *id)
                    {
                        c.visibility_level = level;
                        if state.db.upsert_content(&c).is_ok() {
                            affected += 1;
                        }
                    }
                }
            }
            "set_featured" => {
                if let Some(mut c) = state.db.get_all_contents().into_iter().find(|c| c.id == *id) {
                    c.is_featured = payload.value.as_deref() == Some("true");
                    if state.db.upsert_content(&c).is_ok() {
                        affected += 1;
                    }
                }
            }
            _ => {}
        }
    }
    CentralContentQueryEngine::invalidate_cache();
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "bulk_operation",
        "content",
        None,
        &json!({ "action": payload.action, "affected": affected }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({
        "status": "completed",
        "action": payload.action,
        "affected": affected
    }))
    .into_response()
}

// ============================================================================
// EDITORIAL APPROVAL WORKFLOW (006_approval_workflow.sql)
// ============================================================================

pub async fn submit_content_for_review(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(content_id): Path<i64>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:write") {
        return resp;
    }
    match state.db.create_approval_request(content_id, operator.user_id) {
        Ok(approval_id) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "submit_for_review",
                "content",
                Some(content_id),
                &json!({ "approval_id": approval_id }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "approval_id": approval_id,
                "content_id": content_id,
                "status": "pending_review"
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error_with_key(
            &state.translations,
            "tr",
            "submit_content_for_review",
            &e,
            msg_keys::SERVER_ERROR_SAVE,
        )
        .into_response(),
    }
}

pub async fn list_pending_approvals(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["approvals:read", "approvals:manage", "contents:publish"])
    {
        return resp;
    }
    let approvals = state.db.list_pending_approvals();
    ApiResponse::ok(json!(approvals)).into_response()
}

pub async fn approve_content(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(approval_id): Path<i64>,
    Json(payload): Json<Value>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["approvals:approve", "approvals:manage", "contents:publish"])
    {
        return resp;
    }
    let note = payload.get("note").and_then(|v| v.as_str());
    match state
        .db
        .update_approval_status(approval_id, "approved", operator.user_id, note)
    {
        Ok(()) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "approve_content",
                "approval",
                Some(approval_id),
                &json!({ "note": note.unwrap_or("") }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "approval_id": approval_id,
                "status": "approved"
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error_with_key(
            &state.translations,
            "tr",
            "approve_content",
            &e,
            msg_keys::SERVER_ERROR_SAVE,
        )
        .into_response(),
    }
}

pub async fn reject_content(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(approval_id): Path<i64>,
    Json(payload): Json<Value>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["approvals:reject", "approvals:manage", "contents:publish"])
    {
        return resp;
    }
    let note = payload.get("note").and_then(|v| v.as_str());
    match state
        .db
        .update_approval_status(approval_id, "rejected", operator.user_id, note)
    {
        Ok(()) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "reject_content",
                "approval",
                Some(approval_id),
                &json!({ "note": note.unwrap_or("") }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "approval_id": approval_id,
                "status": "rejected"
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error_with_key(
            &state.translations,
            "tr",
            "reject_content",
            &e,
            msg_keys::SERVER_ERROR_SAVE,
        )
        .into_response(),
    }
}

// ============================================================================
// SERVER MONITORING & SECURITY TELEMETRY (007_monitoring_schema.sql)
// ============================================================================

pub async fn admin_monitoring_bundle(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let needs_system_seed = cache.read().await.system.is_none();
            if needs_system_seed {
                monitoring_service::tick_system_stats(cache).await;
            }
            let mut bundle = monitoring_service::collect_full_bundle(cache).await;
            if bundle.attacks.is_empty() {
                bundle.attacks = state.db.get_recent_attack_events(50);
            }
            ApiResponse::ok(json!(bundle)).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_system(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let needs_system_seed = cache.read().await.system.is_none();
            if needs_system_seed {
                monitoring_service::tick_system_stats(cache).await;
            }
            let c = cache.read().await;
            ApiResponse::ok(json!({ "system": c.system })).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_domains(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            let domains: Vec<_> = c.domains.values().cloned().collect();
            ApiResponse::ok(json!({ "domains": domains, "count": domains.len() })).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_traffic(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            ApiResponse::ok(json!({ "traffic": c.traffic })).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_attacks(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    let db_attacks = state.db.get_recent_attack_events(100);
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            let attacks: Vec<_> = if c.attacks.is_empty() {
                db_attacks
            } else {
                c.attacks.iter().cloned().collect()
            };
            ApiResponse::ok(json!({ "attacks": attacks, "count": attacks.len() })).into_response()
        }
        None => ApiResponse::ok(json!({ "attacks": db_attacks, "count": db_attacks.len() }))
            .into_response(),
    }
}

pub async fn admin_monitoring_services(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            let services: Vec<_> = c.services.values().cloned().collect();
            ApiResponse::ok(json!({ "services": services, "count": services.len() }))
                .into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_seo(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            let seo: Vec<_> = c.seo.values().cloned().collect();
            ApiResponse::ok(json!({ "seo": seo, "count": seo.len() })).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_refresh(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            monitoring_service::force_refresh(
                state.config.port,
                &state.config.monitoring_node_ports,
                &state.config.monitoring_fail2ban_log_path,
                &state.config.monitoring_vhosts_path,
                &state.config.monitoring_nginx_log_path,
                &state.config.monitoring_cert_path,
                state.config.monitoring_retention_days,
                &state.db,
                cache,
            )
            .await;
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "monitoring_refresh",
                "monitoring",
                None,
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({
                "refreshed": true,
                "message": "Sunucu izleme verileri güncellendi."
            }))
            .into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}
