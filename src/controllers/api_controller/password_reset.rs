use super::*;

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
    #[serde(default)]
    pub new_password: String,
    #[serde(default)]
    pub password: String,
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordPayload>,
) -> Response {
    let raw_token = payload.token.trim();
    if raw_token.is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
    }

    let candidate_pw = if !payload.new_password.trim().is_empty() {
        payload.new_password.as_str()
    } else {
        payload.password.as_str()
    };
    let valid_pw = match validation_engine::validate_password(Some(candidate_pw)) {
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
