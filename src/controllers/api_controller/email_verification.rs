use super::*;

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
