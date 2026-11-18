use super::*;

// ============================================================================

pub async fn track_analytics_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut ev): Json<AnalyticsEvent>,
) -> Json<Value> {
    if ev.created_at.is_empty() {
        ev.created_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }
    if let Some(session_u) = auth_service::session_user(&state, &headers) {
        ev.user_id = Some(session_u.id);
        ev.organization_category = session_u.organization_category;
        ev.organization_name = session_u.organization_name;
    } else {
        if ev.organization_category.trim().is_empty() {
            ev.organization_category = "Anonim Okur".to_string();
        }
        if ev.organization_name.trim().is_empty() {
            ev.organization_name = "Doğrulanmamış Ziyaretçi".to_string();
        }
    }
    if ev.duration_sec >= 25 && ev.scroll_depth_pct >= 70 {
        ev.is_true_read = true;
    }
    let id = state.db.insert_analytics_event(&ev).unwrap_or(0);
    Json(json!({
        "status": "recorded",
        "event_id": id,
        "is_true_read": ev.is_true_read
    }))
}

pub async fn get_commerce_catalog(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "subscription_plans": state.db.get_subscription_plans(),
        "commerce_products": state.db.get_commerce_products()
    }))
}

#[derive(Deserialize)]
pub struct CheckoutPayload {
    #[serde(alias = "email")]
    pub user_email: String,
    pub full_name: String,
    pub organization_name: Option<String>,
    pub item_type: String,
    pub item_code: String,
    pub item_title: Option<String>,
    pub amount_try: Option<f64>,
    pub user_phone: Option<String>,
    pub user_address: Option<String>,
    pub distance_sales_consent: Option<bool>,
}

pub(super) fn resolve_catalog_item(
    state: &AppState,
    item_type: &str,
    item_code: &str,
) -> Option<(String, String, f64)> {
    let code = item_code.trim();
    if item_type == "subscription" {
        if let Some(plan) = state
            .db
            .get_subscription_plans()
            .into_iter()
            .find(|p| p.code.eq_ignore_ascii_case(code))
        {
            return Some(("subscription".to_string(), plan.name_tr, plan.price_try));
        }
    }
    if let Some(prod) = state
        .db
        .get_commerce_products()
        .into_iter()
        .find(|p| p.sku.eq_ignore_ascii_case(code) && p.is_available)
    {
        return Some(("product".to_string(), prod.title_tr, prod.price_try));
    }
    if let Some(article) = state
        .db
        .get_content_by_slug(code)
        .filter(|c| c.single_price_try > 0.0)
    {
        return Some((
            "single_article".to_string(),
            article.title_tr,
            article.single_price_try,
        ));
    }
    None
}

pub async fn checkout_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CheckoutPayload>,
) -> Response {
    let lang = "tr";
    let email = match validation_engine::validate_email(&payload.user_email) {
        validation_engine::EmailValidation::Valid(e) => e,
        _ => {
            return ApiResponse::error(
                &state.translations,
                lang,
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::CHECKOUT_FIELDS_REQUIRED,
            )
            .into_response()
        }
    };
    if matches!(
        validation_engine::validate_required_text(&payload.full_name),
        validation_engine::TextFieldValidation::Empty
    ) {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::CHECKOUT_FIELDS_REQUIRED,
        )
        .into_response();
    }
    if payload.distance_sales_consent == Some(false) {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::DISTANCE_SALES_CONSENT_REQUIRED,
        )
        .into_response();
    }

    let _ = (&payload.item_title, payload.amount_try);
    let Some((canonical_type, canonical_title, authoritative_price)) =
        resolve_catalog_item(&state, &payload.item_type, &payload.item_code)
    else {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::PLAN_NOT_FOUND,
        )
        .into_response();
    };

    let org = payload
        .organization_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Kurumsal Savunma Paydaşı".to_string());

    let mut order = subscription_service::create_invoice_record(
        &email,
        payload.full_name.trim(),
        &org,
        &canonical_type,
        payload.item_code.trim(),
        &canonical_title,
        authoritative_price,
        &state.config.watermark_secret,
    );

    let paytr_config = payment_service::PayTrConfig::from_app_config(&state.config);
    if !paytr_config.is_configured() {
        order.payment_status = "pending_bank_transfer".to_string();
    }

    let inv_id = match state.db.insert_order_invoice(&order) {
        Ok(id) => id,
        Err(e) => {
            return ApiResponse::server_error_with_key(
                &state.translations,
                lang,
                "checkout_order",
                &e,
                msg_keys::SERVER_ERROR_ORDER,
            )
            .into_response();
        }
    };

    if paytr_config.is_configured() {
        let client_ip = extract_client_ip(&headers);
        let merchant_oid = payment_service::generate_merchant_oid(&order.invoice_no);
        let public_url = &state.config.public_url;
        let ok_url = format!("{}/hesabim?payment=success&oid={}", public_url, merchant_oid);
        let fail_url = format!(
            "{}/abonelik-ve-satis?payment=failed&oid={}",
            public_url, merchant_oid
        );

        let checkout_req = payment_service::PayTrCheckoutRequest {
            user_email: email,
            full_name: payload.full_name.trim().to_string(),
            user_phone: payload
                .user_phone
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "+903122254173".to_string()),
            user_address: payload
                .user_address
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Ankara, Türkiye".to_string()),
            user_ip: client_ip,
            merchant_oid: merchant_oid.clone(),
            amount_cents: (order.amount_try * 100.0).round() as i64,
            item_title: canonical_title,
            item_code: payload.item_code.trim().to_string(),
            currency: "TL".to_string(),
            lang: "tr".to_string(),
        };

        return match payment_service::create_checkout_session(
            &paytr_config,
            &checkout_req,
            &ok_url,
            &fail_url,
        )
        .await
        {
            Ok(session) => Json(json!({
                "status": "paytr_checkout_ready",
                "iframe_token": session.iframe_token,
                "merchant_oid": session.merchant_oid,
                "amount_kurus": session.amount_kurus,
                "invoice_id": inv_id,
                "invoice_no": order.invoice_no,
                "test_mode": paytr_config.test_mode
            }))
            .into_response(),
            Err(e) => {
                tracing::error!(
                    paytr_error = %e,
                    invoice_no = %order.invoice_no,
                    "PayTR checkout session failed"
                );
                ApiResponse::payment_gateway_error(&state.translations, lang, &e).into_response()
            }
        };
    }

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "status": "order_recorded_pending_payment",
            "invoice_id": inv_id,
            "invoice_no": order.invoice_no,
            "amount_try": authoritative_price,
            "item_title": canonical_title,
            "payment_gateway": "corporate_proforma_pending",
            "message": "Sipariş talebiniz resmi referans numarasıyla kaydedildi. Kurumsal faturalandırma ve sanal POS aktivasyonu sonrası erişiminiz tanımlanacaktır."
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct PayTrCallbackPayload {
    pub merchant_oid: String,
    pub status: String,
    pub total_amount: String,
    pub hash: String,
}

pub async fn paytr_callback(
    State(state): State<AppState>,
    Form(payload): Form<PayTrCallbackPayload>,
) -> Response {
    let paytr_config = payment_service::PayTrConfig::from_app_config(&state.config);
    if !paytr_config.is_configured() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "PAYTR notification rejected: gateway not configured",
        )
            .into_response();
    }

    if !payment_service::verify_callback_hash(
        &paytr_config,
        &payload.merchant_oid,
        &payload.status,
        &payload.total_amount,
        &payload.hash,
    ) {
        return (
            StatusCode::FORBIDDEN,
            "PAYTR notification failed: bad hash",
        )
            .into_response();
    }

    if payload.status == "success" {
        let Some(existing_inv) = state.db.get_invoice_by_oid(&payload.merchant_oid) else {
            return (
                StatusCode::NOT_FOUND,
                "PAYTR notification rejected: invoice not found",
            )
                .into_response();
        };
        let expected_kurus = (existing_inv.amount_try * 100.0).round() as i64;
        let received_kurus = payload.total_amount.trim().parse::<i64>().unwrap_or(-1);
        if (expected_kurus - received_kurus).abs() > 1 {
            tracing::error!(
                "PayTR amount mismatch for {}: expected {} kurus, got {} kurus",
                existing_inv.invoice_no,
                expected_kurus,
                received_kurus
            );
            return (
                StatusCode::BAD_REQUEST,
                "PAYTR notification failed: amount mismatch",
            )
                .into_response();
        }
        let Some((invoice, newly_paid)) = state.db.finalize_paid_invoice_strict(&payload.merchant_oid) else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "PAYTR notification failed: database finalization error",
            )
                .into_response();
        };
        if newly_paid {
            state.db.activate_invoice_entitlements(&invoice);
        }
    } else if !state.db.mark_invoice_failed(&payload.merchant_oid) {
        tracing::warn!("PayTR failed notification could not match invoice: {}", payload.merchant_oid);
    }

    (StatusCode::OK, "OK").into_response()
}

