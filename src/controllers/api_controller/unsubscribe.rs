use super::*;

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
