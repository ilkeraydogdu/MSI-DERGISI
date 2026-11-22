use super::*;
use crate::engine::design::labels_helper::lang_href;

// ============================================================================

#[derive(Deserialize)]
pub struct VerifyEmailQuery {
    pub token: Option<String>,
    pub confirm: Option<String>,
    pub lang: Option<String>,
}

pub async fn verify_email(
    State(state): State<AppState>,
    Query(params): Query<VerifyEmailQuery>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = resolve_transactional_lang(params.lang.as_deref(), &headers);
    let is_en = lang.eq_ignore_ascii_case("en");
    let account_href = escape_html(&lang_href("/hesabim", &lang));
    let raw_token = params.token.as_deref().unwrap_or("").trim();

    if raw_token.is_empty() {
        let (title, inner) = if is_en {
            (
                "Email Verification",
                format!(
                    r#"<div class="eyebrow">EMAIL VERIFICATION</div>
            <h1>Verification Failed</h1>
            <p>The verification link is missing or invalid.</p>
            <p><a href="{account_href}" class="button secondary">Back to My Account →</a></p>"#
                ),
            )
        } else {
            (
                "E-Posta Doğrulama",
                format!(
                    r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Doğrulama Başarısız</h1>
            <p>Doğrulama bağlantısı eksik veya geçersiz.</p>
            <p><a href="{account_href}" class="button secondary">Hesabıma Dön →</a></p>"#
                ),
            )
        };
        return render_shelled_transactional(
            &state,
            &lang,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            title,
            "/hesap-dogrula",
            "verify-card",
            &inner,
        );
    }

    let Some((token_id, user_id, expires_at)) = state.db.find_email_verification_token(raw_token)
    else {
        let (title, inner) = if is_en {
            (
                "Email Verification",
                format!(
                    r#"<div class="eyebrow">EMAIL VERIFICATION</div>
            <h1>Verification Failed</h1>
            <p>This verification link is invalid or has already been used.</p>
            <p><a href="{account_href}" class="button secondary">Back to My Account →</a></p>"#
                ),
            )
        } else {
            (
                "E-Posta Doğrulama",
                format!(
                    r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Doğrulama Başarısız</h1>
            <p>Geçersiz veya daha önce kullanılmış doğrulama bağlantısı.</p>
            <p><a href="{account_href}" class="button secondary">Hesabıma Dön →</a></p>"#
                ),
            )
        };
        return render_shelled_transactional(
            &state,
            &lang,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            title,
            "/hesap-dogrula",
            "verify-card",
            &inner,
        );
    };

    if is_token_expired(&expires_at) {
        let (title, inner) = if is_en {
            (
                "Email Verification",
                format!(
                    r#"<div class="eyebrow">EMAIL VERIFICATION</div>
            <h1>Link Expired</h1>
            <p>This verification link has expired. You can request a new verification email from the Reader Portal.</p>
            <p><a href="{account_href}" class="button secondary">Back to My Account →</a></p>"#
                ),
            )
        } else {
            (
                "E-Posta Doğrulama",
                format!(
                    r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Bağlantı Süresi Dolmuş</h1>
            <p>Doğrulama bağlantısının süresi dolmuş. Okuyucu Merkezi üzerinden yeni doğrulama e-postası talep edebilirsiniz.</p>
            <p><a href="{account_href}" class="button secondary">Hesabıma Dön →</a></p>"#
                ),
            )
        };
        return render_shelled_transactional(
            &state,
            &lang,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            title,
            "/hesap-dogrula",
            "verify-card",
            &inner,
        );
    }

    // Only consume on GET if `?confirm=1` is explicitly requested; otherwise render scanner-safe POST form!
    let auto_confirm = matches!(params.confirm.as_deref(), Some("1") | Some("true"));
    if auto_confirm {
        if let Err(e) = state.db.mark_email_verified(user_id) {
            return ApiResponse::server_error(&state.translations, &lang, "verify_email", &e)
                .into_response();
        }
        let _ = state.db.mark_verification_token_used(token_id);
        let (title, inner) = if is_en {
            (
                "Email Verified",
                format!(
                    r#"<div class="eyebrow">VERIFICATION COMPLETE</div>
            <h1>Your Email Address Is Verified</h1>
            <p>Your MSI Review account has been verified successfully.</p>
            <p><a href="{account_href}" class="button primary">Go to My Account →</a></p>"#
                ),
            )
        } else {
            (
                "E-Posta Doğrulandı",
                format!(
                    r#"<div class="eyebrow">DOĞRULAMA BAŞARILI</div>
            <h1>E-Posta Adresiniz Doğrulandı</h1>
            <p>MSI Dergisi hesabınız başarıyla doğrulandı.</p>
            <p><a href="{account_href}" class="button primary">Hesabıma Git →</a></p>"#
                ),
            )
        };
        return render_shelled_transactional(
            &state,
            &lang,
            Some(&csp_nonce.0),
            StatusCode::OK,
            title,
            "/hesap-dogrula",
            "verify-card is-success",
            &inner,
        );
    }

    let escaped_token = escape_html(raw_token);
    let escaped_lang = escape_html(&lang);
    let form_action = escape_html(&lang_href("/hesap-dogrula", &lang));
    let (title, inner) = if is_en {
        (
            "Email Verification",
            format!(
                r#"<div class="eyebrow">MSI REVIEW · EMAIL VERIFICATION</div>
        <h1>Confirm Your Email Address</h1>
        <p>To protect your account against automated corporate email scanners, please complete verification using the button below.</p>
        <form method="POST" action="{form_action}" class="ui-form">
            <input type="hidden" name="token" value="{escaped_token}">
            <input type="hidden" name="lang" value="{escaped_lang}">
            <button type="submit" class="button primary">Verify My Email Address →</button>
        </form>"#
            ),
        )
    } else {
        (
            "E-Posta Doğrulama",
            format!(
                r#"<div class="eyebrow">MSI DERGİSİ · E-POSTA DOĞRULAMA</div>
        <h1>E-Posta Adresinizi Onaylayın</h1>
        <p>Kurumsal e-posta tarayıcılarına karşı güvenliğiniz için doğrulamayı aşağıdaki butonla tamamlayın.</p>
        <form method="POST" action="{form_action}" class="ui-form">
            <input type="hidden" name="token" value="{escaped_token}">
            <input type="hidden" name="lang" value="{escaped_lang}">
            <button type="submit" class="button primary">E-Posta Adresimi Doğrula →</button>
        </form>"#
            ),
        )
    };
    render_shelled_transactional(
        &state,
        &lang,
        Some(&csp_nonce.0),
        StatusCode::OK,
        title,
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
    let lang = resolve_transactional_lang(
        form.get("lang")
            .or_else(|| query_params.get("lang"))
            .map(String::as_str),
        &headers,
    );
    let is_en = lang.eq_ignore_ascii_case("en");
    let account_href = escape_html(&lang_href("/hesabim", &lang));

    let raw_token = form
        .get("token")
        .or_else(|| query_params.get("token"))
        .map(String::as_str)
        .unwrap_or("")
        .trim();

    let Some((token_id, user_id, expires_at)) = state.db.find_email_verification_token(raw_token)
    else {
        let (title, inner) = if is_en {
            (
                "Email Verification",
                format!(
                    r#"<div class="eyebrow">EMAIL VERIFICATION</div>
            <h1>Verification Failed</h1>
            <p>This verification link is invalid or has already been used.</p>
            <p><a href="{account_href}" class="button secondary">Back to My Account →</a></p>"#
                ),
            )
        } else {
            (
                "E-Posta Doğrulama",
                format!(
                    r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Doğrulama Başarısız</h1>
            <p>Geçersiz veya kullanılmış doğrulama bağlantısı.</p>
            <p><a href="{account_href}" class="button secondary">Hesabıma Dön →</a></p>"#
                ),
            )
        };
        return render_shelled_transactional(
            &state,
            &lang,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            title,
            "/hesap-dogrula",
            "verify-card",
            &inner,
        );
    };

    if is_token_expired(&expires_at) {
        let (title, inner) = if is_en {
            (
                "Email Verification",
                format!(
                    r#"<div class="eyebrow">EMAIL VERIFICATION</div>
            <h1>Link Expired</h1>
            <p>This verification link has expired. Please request a new verification email.</p>
            <p><a href="{account_href}" class="button secondary">Back to My Account →</a></p>"#
                ),
            )
        } else {
            (
                "E-Posta Doğrulama",
                format!(
                    r#"<div class="eyebrow">E-POSTA DOĞRULAMA</div>
            <h1>Bağlantı Süresi Dolmuş</h1>
            <p>Doğrulama bağlantısının süresi dolmuş. Lütfen yeni bir doğrulama e-postası talep edin.</p>
            <p><a href="{account_href}" class="button secondary">Hesabıma Dön →</a></p>"#
                ),
            )
        };
        return render_shelled_transactional(
            &state,
            &lang,
            Some(&csp_nonce.0),
            StatusCode::BAD_REQUEST,
            title,
            "/hesap-dogrula",
            "verify-card",
            &inner,
        );
    }

    if let Err(e) = state.db.mark_email_verified(user_id) {
        return ApiResponse::server_error(&state.translations, &lang, "verify_email_confirm", &e)
            .into_response();
    }
    let _ = state.db.mark_verification_token_used(token_id);

    let (title, inner) = if is_en {
        (
            "Email Verified",
            format!(
                r#"<div class="eyebrow">MSI REVIEW · VERIFICATION COMPLETE</div>
        <h1>Your Email Address Is Verified</h1>
        <p>Your account has been verified successfully. You now have full access to all MSI Review features.</p>
        <p><a href="{account_href}" class="button primary">Go to My Account →</a></p>"#
            ),
        )
    } else {
        (
            "E-Posta Doğrulandı",
            format!(
                r#"<div class="eyebrow">MSI DERGİSİ · DOĞRULAMA BAŞARILI</div>
        <h1>E-Posta Adresiniz Doğrulandı</h1>
        <p>Hesabınız başarıyla doğrulandı. Artık tüm MSI Dergisi özelliklerine erişebilirsiniz.</p>
        <p><a href="{account_href}" class="button primary">Hesabıma Git →</a></p>"#
            ),
        )
    };
    render_shelled_transactional(
        &state,
        &lang,
        Some(&csp_nonce.0),
        StatusCode::OK,
        title,
        "/hesap-dogrula",
        "verify-card is-success",
        &inner,
    )
}

#[derive(Deserialize)]
pub struct ResendVerificationPayload {
    pub email: String,
    #[serde(default)]
    pub lang: Option<String>,
}

static RESEND_VERIFICATION_EMAIL_LIMITS: Lazy<std::sync::Mutex<HashMap<String, (u32, i64)>>> =
    Lazy::new(|| std::sync::Mutex::new(HashMap::new()));
static RESEND_VERIFICATION_IP_LIMITS: Lazy<std::sync::Mutex<HashMap<String, (u32, i64)>>> =
    Lazy::new(|| std::sync::Mutex::new(HashMap::new()));

fn check_resend_rate_limit(
    store: &std::sync::Mutex<HashMap<String, (u32, i64)>>,
    key: &str,
    max_requests: u32,
    window_secs: i64,
) -> bool {
    let norm = key.trim().to_lowercase();
    if norm.is_empty() {
        return true;
    }
    let Ok(mut map) = store.lock() else {
        return true;
    };
    let now = chrono::Utc::now().timestamp();
    if map.len() > 5_000 {
        map.retain(|_, (_, ts)| now - *ts < window_secs * 2);
    }
    let entry = map.entry(norm).or_insert((0, now));
    if now - entry.1 >= window_secs {
        entry.0 = 1;
        entry.1 = now;
    } else {
        entry.0 = entry.0.saturating_add(1);
    }
    entry.0 <= max_requests
}

pub async fn resend_verification(
    State(state): State<AppState>,
    opt_ip: Option<Extension<security_engine::ResolvedClientIp>>,
    Query(query_params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Json(payload): Json<ResendVerificationPayload>,
) -> Response {
    let lang = resolve_transactional_lang(
        payload
            .lang
            .as_deref()
            .or_else(|| query_params.get("lang").map(String::as_str)),
        &headers,
    );
    let is_en = lang.eq_ignore_ascii_case("en");
    let (client_ip, was_forwarded) = resolve_client_ip_with_forwarded(opt_ip.as_ref(), &headers);
    let ip_max = if !was_forwarded && (client_ip == "127.0.0.1" || client_ip == "::1") {
        20
    } else {
        6
    };
    if !client_ip.is_empty()
        && client_ip != "unknown"
        && !check_resend_rate_limit(&RESEND_VERIFICATION_IP_LIMITS, &client_ip, ip_max, 60)
    {
        return ApiResponse::error(
            &state.translations,
            &lang,
            StatusCode::TOO_MANY_REQUESTS,
            error_codes::RATE_LIMITED,
            msg_keys::RATE_LIMIT_EXCEEDED,
        )
        .into_response();
    }

    if !check_resend_rate_limit(&RESEND_VERIFICATION_EMAIL_LIMITS, &payload.email, 2, 60) {
        return ApiResponse::error(
            &state.translations,
            &lang,
            StatusCode::TOO_MANY_REQUESTS,
            error_codes::RATE_LIMITED,
            msg_keys::RATE_LIMIT_EXCEEDED,
        )
        .into_response();
    }

    if let Some((user, _pw_hash)) = state.db.get_user_by_email(&payload.email) {
        if !user.email_verified && user.role != "deleted" {
            let token = format!(
                "{}{}",
                uuid::Uuid::new_v4().as_simple(),
                uuid::Uuid::new_v4().as_simple()
            );
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);
            let expires_str = expires_at.format("%Y-%m-%d %H:%M:%S").to_string();

            let _ = state.db.invalidate_user_verification_tokens(user.id);
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

    let message = if is_en {
        "If this email address is registered and not yet verified, a new verification link has been sent."
    } else {
        "Eğer bu e-posta adresi kayıtlıysa ve henüz doğrulanmamışsa, yeni bir doğrulama bağlantısı gönderilmiştir."
    };

    ApiResponse::ok(json!({
        "message": message
    }))
    .into_response()
}

// ============================================================================
// KVKK/GDPR UNSUBSCRIBE & EMAIL PREFERENCES (010_unsubscribe_tokens.sql)
