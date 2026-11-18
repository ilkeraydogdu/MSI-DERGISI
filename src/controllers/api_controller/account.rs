use super::*;

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
