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
    ev.video_watch_pct = ev.video_watch_pct.clamp(0, 100);
    if ev.video_watch_pct >= 95 {
        ev.video_completed = true;
    }
    let id = state.db.insert_analytics_event(&ev).unwrap_or(0);
    Json(json!({
        "status": "recorded",
        "event_id": id,
        "is_true_read": ev.is_true_read,
        "video_watch_pct": ev.video_watch_pct
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
    #[serde(default)]
    pub user_email: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
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

fn resolve_checkout_email(
    user_email: Option<&str>,
    email: Option<&str>,
) -> Result<String, ()> {
    let ue = user_email.map(str::trim).filter(|s| !s.is_empty());
    let em = email.map(str::trim).filter(|s| !s.is_empty());
    let chosen = match (ue, em) {
        (Some(a), Some(b)) => {
            if !a.eq_ignore_ascii_case(b) {
                return Err(());
            }
            a
        }
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => return Err(()),
    };
    match validation_engine::validate_email(chosen) {
        validation_engine::EmailValidation::Valid(v) => Ok(v),
        _ => Err(()),
    }
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
    let Ok(email) = resolve_checkout_email(payload.user_email.as_deref(), payload.email.as_deref())
    else {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::CHECKOUT_FIELDS_REQUIRED,
        )
        .into_response();
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
    if payload.distance_sales_consent != Some(true) {
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

    let (inv_id, persisted_invoice_no) = match state.db.insert_order_invoice(&order) {
        Ok(res) => res,
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
    order.invoice_no = persisted_invoice_no;

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

#[derive(Deserialize)]
pub struct CartItemInput {
    pub item_type: Option<String>,
    pub item_code: String,
    pub qty: Option<i32>,
}

#[derive(Deserialize)]
pub struct CartCheckoutPayload {
    #[serde(default)]
    pub user_email: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub full_name: String,
    pub organization_name: Option<String>,
    pub user_phone: Option<String>,
    pub user_address: Option<String>,
    pub distance_sales_consent: Option<bool>,
    pub items: Vec<CartItemInput>,
}

pub async fn cart_checkout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CartCheckoutPayload>,
) -> Response {
    let lang = "tr";
    let Ok(email) = resolve_checkout_email(payload.user_email.as_deref(), payload.email.as_deref())
    else {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::CHECKOUT_FIELDS_REQUIRED,
        )
        .into_response();
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
    if payload.distance_sales_consent != Some(true) {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::DISTANCE_SALES_CONSENT_REQUIRED,
        )
        .into_response();
    }
    if payload.items.is_empty() {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::CHECKOUT_FIELDS_REQUIRED,
        )
        .into_response();
    }

    let mut total_try = 0.0_f64;
    let mut code_tokens: Vec<String> = Vec::new();
    let mut title_tokens: Vec<String> = Vec::new();
    let mut resolved_items: Vec<Value> = Vec::new();

    for item in &payload.items {
        let req_type = item.item_type.as_deref().unwrap_or("product").trim();
        let req_code = item.item_code.trim();
        let qty = match item.qty {
            Some(q) if q <= 0 || q > 50 => {
                return ApiResponse::error(
                    &state.translations,
                    lang,
                    StatusCode::BAD_REQUEST,
                    error_codes::BAD_REQUEST,
                    msg_keys::VALIDATION_ERROR,
                )
                .into_response();
            }
            Some(q) => q,
            None => 1,
        };
        let Some((canon_type, canon_title, unit_price)) =
            resolve_catalog_item(&state, req_type, req_code)
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
        let line_total = ((unit_price * qty as f64) * 100.0).round() / 100.0;
        total_try = ((total_try + line_total) * 100.0).round() / 100.0;
        code_tokens.push(format!("{}:{}", canon_type, req_code));
        title_tokens.push(if qty > 1 {
            format!("{} x{}", canon_title, qty)
        } else {
            canon_title.clone()
        });
        resolved_items.push(json!({
            "item_type": canon_type,
            "item_code": req_code,
            "title": canon_title,
            "qty": qty,
            "unit_price_try": unit_price,
            "line_total_try": line_total
        }));
    }

    let joined_codes = code_tokens.join(",");
    let joined_titles = title_tokens.join(" + ");
    let org = payload
        .organization_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Kurumsal Savunma Paydaşı".to_string());

    let mut order = subscription_service::create_invoice_record(
        &email,
        payload.full_name.trim(),
        &org,
        "cart",
        &joined_codes,
        &joined_titles,
        total_try,
        &state.config.watermark_secret,
    );

    let paytr_config = payment_service::PayTrConfig::from_app_config(&state.config);
    if !paytr_config.is_configured() {
        order.payment_status = "pending_bank_transfer".to_string();
    }

    let (inv_id, persisted_invoice_no) = match state.db.insert_order_invoice(&order) {
        Ok(res) => res,
        Err(e) => {
            return ApiResponse::server_error_with_key(
                &state.translations,
                lang,
                "cart_checkout",
                &e,
                msg_keys::SERVER_ERROR_ORDER,
            )
            .into_response();
        }
    };
    order.invoice_no = persisted_invoice_no;

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
            item_title: joined_titles.clone(),
            item_code: joined_codes,
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
                "paytr_iframe_url": format!("https://www.paytr.com/odeme/guvenli/{}", session.iframe_token),
                "merchant_oid": session.merchant_oid,
                "amount_kurus": session.amount_kurus,
                "amount_try": total_try,
                "total_try": total_try,
                "invoice_id": inv_id,
                "invoice_no": order.invoice_no,
                "items": resolved_items,
                "test_mode": paytr_config.test_mode
            }))
            .into_response(),
            Err(e) => ApiResponse::payment_gateway_error(&state.translations, lang, &e).into_response(),
        };
    }

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "status": "order_recorded_pending_payment",
            "invoice_id": inv_id,
            "invoice_no": order.invoice_no,
            "amount_try": total_try,
            "total_try": total_try,
            "item_title": joined_titles,
            "items": resolved_items,
            "payment_gateway": "corporate_proforma_pending",
            "message": "Sepet siparişiniz resmi fatura numarasıyla kaydedildi. Ödeme onayı sonrasında dijital varlıklarınız erişime açılacaktır."
        })),
    )
        .into_response()
}

pub async fn download_purchased_asset(
    State(state): State<AppState>,
    Path(sku): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return ApiResponse::error(
            &state.translations,
            "tr",
            StatusCode::UNAUTHORIZED,
            error_codes::UNAUTHORIZED,
            msg_keys::AUTH_REQUIRED,
        )
        .into_response();
    };

    let clean_sku = sku.trim();
    if clean_sku.is_empty()
        || clean_sku.contains("..")
        || clean_sku.contains('/')
        || clean_sku.contains('\\')
    {
        return ApiResponse::error(
            &state.translations,
            "tr",
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::VALIDATION_ERROR,
        )
        .into_response();
    }

    let Some((product, asset_url)) = state.db.get_commerce_product_delivery_asset(clean_sku) else {
        return ApiResponse::error(
            &state.translations,
            "tr",
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            msg_keys::PLAN_NOT_FOUND,
        )
        .into_response();
    };

    let is_staff = is_staff_role(&state.db, &user.role);
    let has_purchased = state.db.user_has_purchased_item(&user.email, &product.sku);
    if !is_staff && !has_purchased {
        return ApiResponse::error(
            &state.translations,
            "tr",
            StatusCode::FORBIDDEN,
            error_codes::FORBIDDEN,
            msg_keys::SUBSCRIBER_ACCESS_REQUIRED,
        )
        .into_response();
    }

    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_security_audit_log(&crate::models::SecurityAuditLog {
        id: 0,
        client_ip: client_ip.clone(),
        request_path: format!("/api/v1/commerce/download/{}", product.sku),
        event_type: "COMMERCE_ASSET_DOWNLOAD".to_string(),
        threat_level: "info".to_string(),
        details: json!({ "user": user.email, "sku": product.sku }).to_string(),
        blocked: false,
        created_at: chrono::Utc::now().to_rfc3339(),
    });

    // Try reading real file on disk if `delivery_asset_url` points to a safe filename under edergi_dir or static_dir
    let asset_trimmed = asset_url.trim().trim_start_matches('/');
    if !asset_trimmed.is_empty() && !asset_trimmed.contains("..") {
        let mut candidates = Vec::new();
        if let Some(fname) = FsPath::new(asset_trimmed).file_name() {
            candidates.push(FsPath::new(&state.config.edergi_dir).join(fname));
        }
        candidates.push(FsPath::new(&state.config.static_dir).join(asset_trimmed));
        for cand in candidates {
            if let Ok(bytes) = std::fs::read(&cand) {
                if !bytes.is_empty() {
                    let ext = cand
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_ascii_lowercase())
                        .unwrap_or_default();
                    let content_type = match ext.as_str() {
                        "pdf" => "application/pdf",
                        "webp" => "image/webp",
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        _ => "application/octet-stream",
                    };
                    let out_name = cand
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("asset.bin");
                    return (
                        StatusCode::OK,
                        [
                            (header::CONTENT_TYPE, content_type.to_string()),
                            (
                                header::CONTENT_DISPOSITION,
                                format!("attachment; filename=\"{}\"", out_name),
                            ),
                        ],
                        bytes,
                    )
                        .into_response();
                }
            }
        }
    }

    // Generate a real watermarked digital edition / license certificate PDF for the purchased SKU
    let synthetic_issue = crate::models::MagazineIssue {
        issue_number: product.id.max(1) as i32,
        slug: product.sku.to_lowercase(),
        title_tr: product.title_tr.clone(),
        title_en: product.title_en.clone(),
        publish_month_tr: "Dijital Teslimat".to_string(),
        publish_month_en: "Digital Delivery".to_string(),
        year: 2026,
        cover_image: product.cover_image.clone(),
        is_special_issue: false,
        event_name: Some(product.sku.clone()),
        page_count: 1,
        dossier_count: 1,
        summary_tr: product.description_tr.clone(),
        summary_en: product.description_en.clone(),
        pdf_path: format!("/api/v1/commerce/download/{}", product.sku),
        publication_type: "commerce".to_string(),
        uploaded_pdf_file: None,
        en_status: "approved".to_string(),
        spreads: vec![],
        ..Default::default()
    };
    let license = format!("SKU: {} | {}", product.sku, product.author_or_source);
    let pdf_bytes = edergi_pdf_service::generate_watermarked_pdf(
        &synthetic_issue,
        &user.full_name,
        &user.organization_name,
        &license,
        &client_ip,
        None,
        &state.config.watermark_secret,
    );
    let filename = format!("MSI_{}_Digital.pdf", product.sku);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        pdf_bytes,
    )
        .into_response()
}

