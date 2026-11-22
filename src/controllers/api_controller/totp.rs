use super::*;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

static TOTP_CHALLENGE_LOCKS: Lazy<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static TOTP_USER_LOCKS: Lazy<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static LAST_TOTP_LOGIN_STEPS: Lazy<RwLock<HashMap<String, u64>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

fn totp_token_lock(token: &str) -> Arc<tokio::sync::Mutex<()>> {
    let mut hasher = Sha256::new();
    hasher.update(token.trim().as_bytes());
    let key = hex::encode(hasher.finalize());
    let mut map = TOTP_CHALLENGE_LOCKS
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if map.len() > 4096 {
        map.retain(|_, v| Arc::strong_count(v) > 1);
    }
    map.entry(key)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

fn totp_user_lock(email_norm: &str) -> Arc<tokio::sync::Mutex<()>> {
    let mut map = TOTP_USER_LOCKS
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if map.len() > 4096 {
        map.retain(|_, v| Arc::strong_count(v) > 1);
    }
    map.entry(email_norm.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

fn totp_step_cache_key(email_norm: &str, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.trim().as_bytes());
    format!("{}:{}", email_norm, hex::encode(hasher.finalize()))
}

fn clear_user_totp_steps(email_norm: &str) {
    let prefix = format!("{}:", email_norm);
    if let Ok(mut guard) = LAST_TOTP_LOGIN_STEPS.write() {
        guard.retain(|k, _| k != email_norm && !k.starts_with(&prefix));
    }
}

pub async fn totp_setup(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };
    let email_key = user.email.trim().to_lowercase();
    let user_mutex = totp_user_lock(&email_key);
    let _user_guard = user_mutex.lock().await;

    if state
        .db
        .get_user_totp_secret(&user.email)
        .is_some_and(|s| !s.trim().is_empty())
    {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "status": "error",
                "error_code": "TOTP_ALREADY_ENABLED",
                "message": "İki aşamalı doğrulama (2FA) zaten aktif. Yeniden yapılandırmak için önce mevcut 2FA korumasını devre dışı bırakın.",
                "data": null
            })),
        )
            .into_response();
    }

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
    pub current_password: Option<String>,
    pub current_code: Option<String>,
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
    let user_mutex = totp_user_lock(&email_key);
    let _user_guard = user_mutex.lock().await;

    // If 2FA is already active, require full re-authentication (current password + current TOTP/backup code).
    if let Some(existing_secret) = state
        .db
        .get_user_totp_secret(&user.email)
        .filter(|s| !s.trim().is_empty())
    {
        let pw_ok = payload
            .current_password
            .as_deref()
            .filter(|s| !s.is_empty())
            .and_then(|pw| {
                state.db.get_user_by_email(&user.email).map(|(_, hash)| {
                    auth_service::verify_password(pw, &state.config.session_secret, &hash)
                })
            })
            .unwrap_or(false);
        let code_ok = payload
            .current_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|c| {
                totp_service::verify_totp(&existing_secret, c) || {
                    let hashes = state.db.get_user_totp_backup_codes(&user.email);
                    totp_service::verify_backup_code(c, &hashes).is_some()
                }
            })
            .unwrap_or(false);
        if !pw_ok || !code_ok {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "error",
                    "error_code": "TOTP_REAUTH_REQUIRED",
                    "message": "2FA zaten aktif. Anahtarı değiştirmek için mevcut parolanız ve mevcut 2FA kodunuz gereklidir.",
                    "data": null
                })),
            )
                .into_response();
        }
    }

    let now = chrono::Utc::now().timestamp();
    let pending_secret = PENDING_TOTP_SETUPS.read().ok().and_then(|guard| {
        guard
            .get(&email_key)
            .filter(|(_, ts)| now - *ts <= PENDING_TOTP_SETUP_TTL_SECS)
            .map(|(sec, _)| sec.clone())
    });

    let Some(secret) = pending_secret else {
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

    if let Some(provided) = payload
        .secret
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let norm_provided: String = provided
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let norm_pending: String = secret
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_uppercase())
            .collect();
        if !crate::utils::crypto::constant_time_eq(
            norm_provided.as_bytes(),
            norm_pending.as_bytes(),
        ) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "error",
                    "error_code": "TOTP_SECRET_MISMATCH",
                    "message": "Sağlanan 2FA anahtarı aktif kurulum oturumuyla eşleşmiyor.",
                    "data": null
                })),
            )
                .into_response();
        }
    }

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

    // Atomically consume pending setup so concurrent verify-setup requests cannot both succeed.
    let consumed = PENDING_TOTP_SETUPS
        .write()
        .ok()
        .and_then(|mut guard| guard.remove(&email_key));
    if consumed.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "error_code": "TOTP_SETUP_EXPIRED",
                "message": "2FA kurulum oturumu zaten kullanıldı veya süresi doldu.",
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
    clear_user_totp_steps(&email_key);

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
    let Some(_) = auth_service::verify_totp_pending_token_db(
        &state.db,
        &payload.temp_token,
        &state.config.session_secret,
    ) else {
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

    // Serialize all attempts using the same `temp_token` so concurrent requests cannot race to mint >1 session.
    let token_mutex = totp_token_lock(&payload.temp_token);
    let _token_guard = token_mutex.lock().await;

    let Some(email) = auth_service::verify_totp_pending_token_db(
        &state.db,
        &payload.temp_token,
        &state.config.session_secret,
    ) else {
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

    let email_norm = email.trim().to_lowercase();
    // Serialize per-user across all active `temp_token`s so concurrent attempts cannot replay a TOTP step or backup code.
    let user_mutex = totp_user_lock(&email_norm);
    let _user_guard = user_mutex.lock().await;

    let lockout_key = format!("totp:{}", email);
    let client_ip = extract_client_ip(&headers);
    if let LoginCheckResult::Locked {
        remaining_secs,
        attempts,
    } = security_engine::check_login_allowed(&lockout_key)
    {
        let _ = state.db.insert_security_audit_log(&crate::models::SecurityAuditLog {
            id: 0,
            client_ip: client_ip.clone(),
            request_path: "/api/v1/auth/totp/login".to_string(),
            event_type: "AUTH_ACCOUNT_LOCKED".to_string(),
            threat_level: "high".to_string(),
            details: json!({ "attempts": attempts, "scope": "totp" }).to_string(),
            blocked: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        let _ = crate::services::security_anomaly_service::evaluate_and_emit_anomalies(&state.db);
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
    if auth_service::is_deleted_or_erased_user(&user) {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::LOGIN_FAILED);
    }
    let Some(totp_secret) = state.db.get_user_totp_secret(&user.email) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
    };

    let step_cache_key = totp_step_cache_key(&email_norm, &totp_secret);
    let mut valid_2fa = false;
    let mut matched_step: Option<u64> = None;
    let mut replayed_step = false;

    if let Some(step) = totp_service::verify_totp_step(&totp_secret, &payload.code) {
        let already_used = LAST_TOTP_LOGIN_STEPS
            .read()
            .ok()
            .and_then(|g| g.get(&step_cache_key).copied())
            .is_some_and(|last| last >= step);
        if already_used {
            replayed_step = true;
        } else {
            valid_2fa = true;
            matched_step = Some(step);
        }
    }

    if !valid_2fa && !replayed_step {
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
        let _ = state.db.insert_security_audit_log(&crate::models::SecurityAuditLog {
            id: 0,
            client_ip,
            request_path: "/api/v1/auth/totp/login".to_string(),
            event_type: "AUTH_TOTP_FAILED".to_string(),
            threat_level: "medium".to_string(),
            details: json!({ "scope": "totp", "replayed": replayed_step }).to_string(),
            blocked: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        let _ = crate::services::security_anomaly_service::evaluate_and_emit_anomalies(&state.db);
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "error",
                "error_code": if replayed_step { "TOTP_CODE_REPLAYED" } else { "TOTP_CODE_INVALID" },
                "message": if replayed_step {
                    "Bu doğrulama kodu daha önce kullanıldı. Lütfen bir sonraki kodu bekleyin."
                } else {
                    "Doğrulama veya yedek kurtarma kodu geçersiz."
                },
                "data": null
            })),
        )
            .into_response();
    }

    if let Some(step) = matched_step {
        if let Ok(mut guard) = LAST_TOTP_LOGIN_STEPS.write() {
            if guard.len() > 4096 {
                guard.clear();
            }
            guard.insert(step_cache_key, step);
        }
    }

    security_engine::reset_login_attempts(&lockout_key);
    auth_service::revoke_session_token_db(&state.db, &payload.temp_token);

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
    let email_key = user.email.trim().to_lowercase();
    let user_mutex = totp_user_lock(&email_key);
    let _user_guard = user_mutex.lock().await;

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
    clear_user_totp_steps(&email_key);

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

