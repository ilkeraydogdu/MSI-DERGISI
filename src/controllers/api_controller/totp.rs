use super::*;

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

