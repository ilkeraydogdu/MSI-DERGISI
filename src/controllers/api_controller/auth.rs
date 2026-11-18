use super::*;

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
