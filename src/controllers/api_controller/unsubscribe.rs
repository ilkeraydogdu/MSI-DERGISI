use super::*;
use crate::engine::design::labels_helper::lang_href;

// ============================================================================

#[derive(Deserialize)]
pub struct UnsubscribeTokenQuery {
    pub token: Option<String>,
    pub lang: Option<String>,
}

fn localize_frequency_label(freq: &str, is_en: bool) -> String {
    match freq.trim().to_ascii_lowercase().as_str() {
        "weekly" => {
            if is_en {
                "Weekly".to_string()
            } else {
                "Haftalık".to_string()
            }
        }
        "biweekly" => {
            if is_en {
                "Biweekly".to_string()
            } else {
                "İki Haftada Bir".to_string()
            }
        }
        "monthly" => {
            if is_en {
                "Monthly".to_string()
            } else {
                "Aylık".to_string()
            }
        }
        other => escape_html(other),
    }
}

pub async fn unsubscribe_page(
    State(state): State<AppState>,
    Query(params): Query<UnsubscribeTokenQuery>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = resolve_transactional_lang(params.lang.as_deref(), &headers);
    let is_en = lang.eq_ignore_ascii_case("en");
    let account_href = escape_html(&lang_href("/hesabim", &lang));

    let token = match params.token.as_deref() {
        Some(t) if !t.trim().is_empty() => t.trim(),
        _ => {
            let (title, inner) = if is_en {
                (
                    "Unsubscribe",
                    format!(
                        r#"<div class="eyebrow">SUBSCRIPTION PREFERENCES</div>
                <h1>Invalid Link</h1>
                <p>This unsubscribe link is invalid or incomplete.</p>
                <p><a href="{account_href}" class="button secondary">Back to My Account →</a></p>"#
                    ),
                )
            } else {
                (
                    "Abonelik İptali",
                    format!(
                        r#"<div class="eyebrow">ABONELİK YÖNETİMİ</div>
                <h1>Geçersiz Bağlantı</h1>
                <p>Bu abonelik iptal bağlantısı geçersiz veya eksik.</p>
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
                "/abonelik-iptal",
                "unsubscribe-card",
                &inner,
            );
        }
    };

    let Some((_token_id, user_id)) = state.db.find_unsubscribe_token(token) else {
        let (title, inner) = if is_en {
            (
                "Unsubscribe",
                format!(
                    r#"<div class="eyebrow">SUBSCRIPTION PREFERENCES</div>
            <h1>Invalid Link</h1>
            <p>This unsubscribe link has already been used or has expired.</p>
            <p><a href="{account_href}" class="button secondary">Back to My Account →</a></p>"#
                ),
            )
        } else {
            (
                "Abonelik İptali",
                format!(
                    r#"<div class="eyebrow">ABONELİK YÖNETİMİ</div>
            <h1>Bağlantı Geçersiz</h1>
            <p>Bu abonelik iptal bağlantısı kullanılmış veya süresi dolmuş.</p>
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
            "/abonelik-iptal",
            "unsubscribe-card",
            &inner,
        );
    };

    let user_email = state
        .db
        .get_user_by_id(user_id)
        .map(|u| u.email)
        .unwrap_or_else(|| {
            if is_en {
                "your registered address".to_string()
            } else {
                "kayıtlı adresiniz".to_string()
            }
        });

    let unsub_action = escape_html(&lang_href("/abonelik-iptal", &lang));
    let prefs_action = escape_html(&lang_href("/api/v1/email-preferences", &lang));
    let escaped_email = escape_html(&user_email);
    let escaped_token = escape_html(token);
    let escaped_lang = escape_html(&lang);

    let (title, inner) = if is_en {
        (
            "Email Subscription Preferences",
            format!(
                r#"<div class="eyebrow">MSI REVIEW · SUBSCRIPTION PREFERENCES</div>
        <h1>You Are About to Unsubscribe</h1>
        <p>Confirm that you no longer wish to receive MSI Review newsletter emails sent to <strong>{escaped_email}</strong>.</p>
        <form method="POST" action="{unsub_action}" class="ui-form">
            <input type="hidden" name="token" value="{escaped_token}">
            <input type="hidden" name="lang" value="{escaped_lang}">
            <button type="submit" class="button primary">Unsubscribe →</button>
        </form>
        <hr class="transactional-divider">
        <h2>Manage Your Preferences</h2>
        <p>Instead of unsubscribing completely, you can reduce briefing frequency or follow specific topics only.</p>
        <form method="POST" action="{prefs_action}" id="prefs-form" class="ui-form">
            <input type="hidden" name="token" value="{escaped_token}">
            <input type="hidden" name="lang" value="{escaped_lang}">
            <label>Newsletter Frequency
                <select name="newsletter_frequency">
                    <option value="weekly">Weekly</option>
                    <option value="biweekly">Biweekly</option>
                    <option value="monthly">Monthly</option>
                </select>
            </label>
            <label>Categories (comma-separated)
                <input type="text" name="categories" placeholder="Aerospace, Naval, Land">
            </label>
            <button type="submit" class="button secondary">Save Preferences →</button>
        </form>"#
            ),
        )
    } else {
        (
            "E-Posta Abonelik Yönetimi",
            format!(
                r#"<div class="eyebrow">MSI DERGİSİ · ABONELİK YÖNETİMİ</div>
        <h1>E-Posta Aboneliğinden Çıkmak Üzeresiniz</h1>
        <p><strong>{escaped_email}</strong> adresine gönderilen MSI Dergisi bültenlerini artık almak istemediğinizi onaylıyorsunuz.</p>
        <form method="POST" action="{unsub_action}" class="ui-form">
            <input type="hidden" name="token" value="{escaped_token}">
            <input type="hidden" name="lang" value="{escaped_lang}">
            <button type="submit" class="button primary">Aboneliği İptal Et →</button>
        </form>
        <hr class="transactional-divider">
        <h2>Tercihlerinizi Yönetin</h2>
        <p>Tamamen çıkmak yerine bülten sıklığını azaltabilir veya yalnızca belirli kategorileri takip edebilirsiniz.</p>
        <form method="POST" action="{prefs_action}" id="prefs-form" class="ui-form">
            <input type="hidden" name="token" value="{escaped_token}">
            <input type="hidden" name="lang" value="{escaped_lang}">
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
    let lang = resolve_transactional_lang(
        form_map
            .get("lang")
            .or_else(|| query_params.get("lang"))
            .map(String::as_str),
        &headers,
    );
    let is_en = lang.eq_ignore_ascii_case("en");
    let account_href = escape_html(&lang_href("/hesabim", &lang));

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
        let (title, inner) = if is_en {
            (
                "Unsubscribe",
                format!(
                    r#"<div class="eyebrow">SUBSCRIPTION PREFERENCES</div>
            <h1>Error</h1>
            <p>Verification token not found.</p>
            <p><a href="{account_href}" class="button secondary">Back to My Account →</a></p>"#
                ),
            )
        } else {
            (
                "Abonelik İptali",
                format!(
                    r#"<div class="eyebrow">ABONELİK YÖNETİMİ</div>
            <h1>Hata</h1>
            <p>Doğrulama anahtarı (token) bulunamadı.</p>
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
            "/abonelik-iptal",
            "unsubscribe-card",
            &inner,
        );
    }

    let Some((token_id, user_id)) = state.db.find_unsubscribe_token(&token) else {
        let (title, inner) = if is_en {
            (
                "Unsubscribe",
                format!(
                    r#"<div class="eyebrow">SUBSCRIPTION PREFERENCES</div>
            <h1>Invalid Link</h1>
            <p>This unsubscribe link has already been used or is invalid.</p>
            <p><a href="{account_href}" class="button secondary">Back to My Account →</a></p>"#
                ),
            )
        } else {
            (
                "Abonelik İptali",
                format!(
                    r#"<div class="eyebrow">ABONELİK YÖNETİMİ</div>
            <h1>Bağlantı Geçersiz</h1>
            <p>Bu abonelik iptal bağlantısı zaten kullanılmış veya geçersiz.</p>
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
            "/abonelik-iptal",
            "unsubscribe-card",
            &inner,
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

    let (title, inner) = if is_en {
        (
            "Email Subscription Cancelled",
            format!(
                r#"<div class="eyebrow">MSI REVIEW · SUBSCRIPTION PREFERENCES</div>
        <h1>You Have Been Unsubscribed</h1>
        <p>You will no longer receive MSI Review newsletter emails. You can update your communication preferences at any time from the <a href="{account_href}">Reader Portal</a>.</p>"#
            ),
        )
    } else {
        (
            "E-Posta Aboneliği İptal Edildi",
            format!(
                r#"<div class="eyebrow">MSI DERGİSİ · ABONELİK YÖNETİMİ</div>
        <h1>E-Posta Aboneliğiniz İptal Edildi</h1>
        <p>Artık MSI Dergisi bülten e-postaları almayacaksınız. Tercihlerinizi dilediğiniz zaman <a href="{account_href}">Okuyucu Merkezi</a> üzerinden güncelleyebilirsiniz.</p>"#
            ),
        )
    };
    render_shelled_transactional(
        &state,
        &lang,
        Some(&csp_nonce.0),
        StatusCode::OK,
        title,
        "/abonelik-iptal",
        "unsubscribe-card is-success",
        &inner,
    )
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum CategoriesField {
    List(Vec<String>),
    Csv(String),
}

#[derive(Deserialize)]
pub(super) struct EmailPreferencesJsonPayload {
    token: String,
    #[serde(default)]
    newsletter_frequency: String,
    categories: Option<CategoriesField>,
    #[serde(default)]
    lang: Option<String>,
}

pub async fn manage_email_preferences(
    State(state): State<AppState>,
    Query(query_params): Query<HashMap<String, String>>,
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

    let mut parsed_lang: Option<String> = query_params.get("lang").cloned();
    let (token, frequency, categories): (String, String, Vec<String>) = if is_json {
        let Ok(parsed) = serde_json::from_slice::<EmailPreferencesJsonPayload>(&body) else {
            let lang = resolve_transactional_lang(parsed_lang.as_deref(), &headers);
            return ApiResponse::error(
                &state.translations,
                &lang,
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::BAD_REQUEST,
            )
            .into_response();
        };
        if parsed.lang.as_deref().is_some_and(|l| !l.trim().is_empty()) {
            parsed_lang = parsed.lang;
        }
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
            let lang = resolve_transactional_lang(parsed_lang.as_deref(), &headers);
            return ApiResponse::error(
                &state.translations,
                &lang,
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::BAD_REQUEST,
            )
            .into_response();
        };
        let mut tok = String::new();
        let mut freq = String::from("weekly");
        let mut cats: Vec<String> = Vec::new();
        for (k, v) in pairs {
            match k.as_str() {
                "token" => tok = v.trim().to_string(),
                "lang" if !v.trim().is_empty() => parsed_lang = Some(v.trim().to_string()),
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

    let lang = resolve_transactional_lang(parsed_lang.as_deref(), &headers);
    let is_en = lang.eq_ignore_ascii_case("en");

    if token.is_empty() {
        return ApiResponse::error(
            &state.translations,
            &lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::BAD_REQUEST,
        )
        .into_response();
    }

    let Some((_token_id, user_id)) = state.db.find_unsubscribe_token(&token) else {
        return ApiResponse::error(
            &state.translations,
            &lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::BAD_REQUEST,
        )
        .into_response();
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
            .into_api_response(&state.translations, &lang)
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
        let account_href = escape_html(&lang_href("/hesabim", &lang));
        let freq_label = localize_frequency_label(&frequency, is_en);
        let (title, inner) = if is_en {
            (
                "Email Preferences Updated",
                format!(
                    r#"<div class="eyebrow">MSI REVIEW · SUBSCRIPTION PREFERENCES</div>
            <h1>Your Email Preferences Have Been Saved</h1>
            <p>Your newsletter frequency has been updated to <strong>{freq_label}</strong>.</p>
            <p><a href="{account_href}" class="button primary">Go to Reader Portal →</a></p>"#
                ),
            )
        } else {
            (
                "E-Posta Tercihleri Güncellendi",
                format!(
                    r#"<div class="eyebrow">MSI DERGİSİ · ABONELİK YÖNETİMİ</div>
            <h1>E-Posta Tercihleriniz Kaydedildi</h1>
            <p>Bülten sıklığınız <strong>{freq_label}</strong> olarak güncellendi.</p>
            <p><a href="{account_href}" class="button primary">Okuyucu Merkezine Git →</a></p>"#
                ),
            )
        };
        return render_shelled_transactional(
            &state,
            &lang,
            nonce_ref,
            StatusCode::OK,
            title,
            "/abonelik-iptal",
            "unsubscribe-card is-success",
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
