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

    let raw_org_cat = payload
        .organization_category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let raw_org_name = payload
        .organization_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let is_verified_institution_account = !is_staff_role(&state.db, &user.role)
        && (user.is_gov_verified
            || user.role.eq_ignore_ascii_case("subscriber_public")
            || user.role.eq_ignore_ascii_case("subscriber_corporate"));

    if is_verified_institution_account {
        let cat_changed = raw_org_cat
            .is_some_and(|c| !c.eq_ignore_ascii_case(user.organization_category.trim()));
        let name_changed = raw_org_name
            .is_some_and(|n| !n.eq_ignore_ascii_case(user.organization_name.trim()));
        if cat_changed || name_changed {
            return ApiResponse::raw(
                StatusCode::FORBIDDEN,
                json!({
                    "status": "error",
                    "error_code": error_codes::FORBIDDEN,
                    "message": "Doğrulanmış kurum kimliği profil üzerinden değiştirilemez. Kurum değişikliği için lütfen destek ekibiyle iletişime geçin.",
                    "data": null
                }),
            )
            .into_response();
        }
    }

    let org_cat = if is_verified_institution_account {
        user.organization_category.as_str()
    } else {
        raw_org_cat.unwrap_or(&user.organization_category)
    };
    let org_name = if is_verified_institution_account {
        user.organization_name.as_str()
    } else {
        raw_org_name.unwrap_or(&user.organization_name)
    };
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

    if consent != user.email_consent || consent != user.marketing_emails_opted_in {
        let client_ip = extract_client_ip(&headers);
        let now_iso = chrono::Utc::now().to_rfc3339();
        let consent_hash = auth_service::generate_kvkk_consent_hash(
            &user.email,
            &client_ip,
            &now_iso,
            &state.config.session_secret,
        );
        let _ = state.db.insert_kvkk_consent_log(&crate::models::KvkkConsentLog {
            id: 0,
            user_id: Some(user.id),
            email: user.email.clone(),
            consent_type: "etk".to_string(),
            consent_version: if user.tos_version.is_empty() {
                "2026-01".to_string()
            } else {
                user.tos_version.clone()
            },
            accepted: consent,
            ip_address: client_ip,
            consent_hash,
            created_at: now_iso,
        });
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
    let Some((_, pw_hash)) = state.db.get_user_by_email(&user.email) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };
    if auth_service::is_deleted_or_erased_user(&user) || pw_hash.trim().is_empty() {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    }
    let Some(pw) = parsed.password.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::LOGIN_FAILED);
    };
    if !auth_service::verify_password(pw, &state.config.session_secret, &pw_hash) {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::LOGIN_FAILED);
    }

    let client_ip = extract_client_ip(&headers);
    if let Err(e) = state.db.delete_user_account(user.id, &client_ip) {
        return ApiResponse::server_error(&state.translations, "tr", "delete_account", &e)
            .into_response();
    }

    if let Some(raw_token) = extract_raw_session_cookie(&headers) {
        auth_service::revoke_session_token_db(&state.db, raw_token);
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
