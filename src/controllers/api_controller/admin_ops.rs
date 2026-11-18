use super::*;

// ============================================================================

pub async fn admin_list_corporate_requests(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "corporate_requests:read") {
        return resp;
    }
    Json(state.db.get_corporate_requests()).into_response()
}

#[derive(Deserialize)]
pub struct CorporateDecisionPayload {
    pub id: Option<i64>,
    pub status: String,
    pub reviewed_by: Option<String>,
}

pub async fn admin_decide_corporate_request_by_path(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<CorporateDecisionPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "corporate_requests:decide") {
        return resp;
    }
    let decision = payload.status.trim().to_lowercase();
    if id <= 0 || !matches!(decision.as_str(), "approved" | "rejected" | "pending") {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    let _ = &payload.reviewed_by;
    if let Err(e) = state
        .db
        .decide_corporate_request(id, &decision, &operator.email)
    {
        return ApiResponse::server_error(
            &state.translations,
            "tr",
            "admin_decide_corporate_request_by_path",
            &e,
        )
        .into_response();
    }
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "decide_corporate_request",
        "corporate_request",
        Some(id),
        &json!({ "decision": decision }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "updated", "id": id, "decision": decision}))
        .into_response()
}

pub async fn admin_decide_corporate_request_body(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CorporateDecisionPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "corporate_requests:decide") {
        return resp;
    }
    let Some(id) = payload.id.filter(|&v| v > 0) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    };
    let decision = payload.status.trim().to_lowercase();
    if !matches!(decision.as_str(), "approved" | "rejected" | "pending") {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    let _ = &payload.reviewed_by;
    if let Err(e) = state
        .db
        .decide_corporate_request(id, &decision, &operator.email)
    {
        return ApiResponse::server_error(
            &state.translations,
            "tr",
            "admin_decide_corporate_request_body",
            &e,
        )
        .into_response();
    }
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "decide_corporate_request",
        "corporate_request",
        Some(id),
        &json!({ "decision": decision }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "updated", "id": id, "decision": decision}))
        .into_response()
}

pub async fn admin_list_categories(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:read", "contents:read"]) {
        return resp;
    }
    Json(state.db.get_all_categories()).into_response()
}

pub async fn admin_insert_category(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(cat): Json<CategoryNode>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:write", "settings:write"]) {
        return resp;
    }
    if matches!(
        validation_engine::validate_required_text(&cat.name_tr),
        validation_engine::TextFieldValidation::Empty
    ) || validation_engine::validate_slug(&cat.slug).is_none()
    {
        return AppError::ValidationError {
            msg_key: msg_keys::VALIDATION_ERROR,
        }
        .into_api_response(&state.translations, "tr")
        .into_response();
    }
    let id = state.db.insert_category(&cat).unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upsert",
        "category",
        Some(id),
        &json!({ "slug": cat.slug }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "saved", "id": id, "slug": cat.slug})).into_response()
}

pub async fn admin_list_ssb_tags(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["categories:read", "contents:read", "ads:read"])
    {
        return resp;
    }
    Json(state.db.get_all_ssb_tags()).into_response()
}

pub async fn admin_insert_ssb_tag(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(tag): Json<SsbInterestTag>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:write", "settings:write"]) {
        return resp;
    }
    let id = state.db.insert_ssb_tag(&tag).unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upsert",
        "ssb_tag",
        Some(id),
        &json!({ "code": tag.code }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "saved", "id": id, "code": tag.code})).into_response()
}

pub async fn admin_list_newsletters(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["newsletters:read", "analytics:read"]) {
        return resp;
    }
    let (campaigns, total_count) = state
        .db
        .get_newsletter_campaigns_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(campaigns), pagination.meta(total_count)).into_response()
}

#[derive(Deserialize)]
pub struct AdminNewsletterPayload {
    #[serde(flatten)]
    pub campaign: NewsletterCampaign,
    pub body_html: Option<String>,
}

pub async fn admin_insert_newsletter(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AdminNewsletterPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "newsletters:write") {
        return resp;
    }
    let mut nl = payload.campaign;
    let users = state.db.get_all_users();
    let recipients = email_service::filter_newsletter_recipients(
        &users,
        &nl.target_group_filter,
        &nl.target_institution,
        &nl.target_ssb_tag,
    );
    nl.recipient_count = recipients.len() as i64;
    if nl.sent_at.trim().is_empty() {
        nl.sent_at = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
    }
    nl.status = if state.config.mail_delivery_enabled {
        "sent".to_string()
    } else {
        "queued".to_string()
    };

    let sponsor_ad = nl
        .sponsor_ad_id
        .and_then(|aid| state.db.get_all_ads().into_iter().find(|a| a.id == aid));
    let dispatch = email_service::dispatch_newsletter_campaign(
        &state.config,
        &state.db,
        &nl,
        &recipients,
        sponsor_ad.as_ref(),
        payload.body_html.as_deref(),
    )
    .await;

    let id = state.db.insert_newsletter_campaign(&nl).unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "create",
        "newsletter_campaign",
        Some(id),
        &json!({ "subject_tr": nl.subject_tr, "recipient_count": nl.recipient_count }).to_string(),
        &client_ip,
    );

    ApiResponse::ok(json!({
        "status": nl.status,
        "id": id,
        "recipient_count": nl.recipient_count,
        "dispatch": dispatch
    }))
    .into_response()
}

pub async fn admin_list_reminders(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["subscriptions:read", "analytics:read"]) {
        return resp;
    }
    let (items, total_count) = state
        .db
        .get_renewal_reminders_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(items), pagination.meta(total_count)).into_response()
}

pub async fn admin_halt_renewed_reminders(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["subscriptions:write", "subscriptions:manage"]) {
        return resp;
    }
    let halted_count = state.db.halt_renewed_reminders().unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "halt_renewed_reminders",
        "renewal_reminder",
        None,
        &json!({ "halted_count": halted_count }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({
        "status": "halted_by_renewal",
        "halted_count": halted_count
    }))
    .into_response()
}

pub async fn admin_list_redirects(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:read", "contents:read"]) {
        return resp;
    }
    let (redirects, total_count) = state
        .db
        .get_seo_redirects_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(redirects), pagination.meta(total_count)).into_response()
}

pub async fn admin_insert_redirect(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(rd): Json<SeoRedirectRule>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:write", "contents:publish"]) {
        return resp;
    }
    match state.db.insert_seo_redirect(&rd) {
        Ok(id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert",
                "seo_redirect",
                Some(id),
                &json!({ "old_path": rd.old_path, "new_path": rd.new_path }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({"status": "saved", "id": id})).into_response()
        }
        Err(e) => ApiResponse::raw(
            StatusCode::BAD_REQUEST,
            json!({
                "status": "error",
                "error_code": error_codes::BAD_REQUEST,
                "message": e.to_string(),
                "data": null
            }),
        )
        .into_response(),
    }
}

pub async fn admin_export_open_data(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:export", "settings:write"]) {
        return resp;
    }
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "export_open_data",
        "system",
        None,
        "{}",
        &client_ip,
    );

    let payload = json!({
        "export_standard": "UMSA-MSI-OPEN-DATA-V1 (Zero Vendor-Lock Universal JSON/PostgreSQL/SQLite Portable Archive)",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "domains": [&state.config.primary_domain, &state.config.news_domain],
        "categories": state.db.get_all_categories(),
        "ssb_interest_tags": state.db.get_all_ssb_tags(),
        "contents_single_source_of_truth": state.db.get_all_contents(),
        "magazine_issues": state.db.get_all_issues(),
        "ad_campaigns": state.db.get_all_ads(),
        "recommendation_rules": state.db.get_recommendation_rules(),
        "users_kvkk_verified": state.db.get_all_users(),
        "corporate_verification_requests": state.db.get_corporate_requests(),
        "subscription_plans": state.db.get_subscription_plans(),
        "commerce_products": state.db.get_commerce_products(),
        "order_invoices": state.db.get_order_invoices(),
        "renewal_reminders_5_stage": state.db.get_renewal_reminders(),
        "newsletter_campaigns": state.db.get_newsletter_campaigns(),
        "seo_301_redirects": state.db.get_seo_redirects(),
        "analytics_events": state.db.get_all_analytics_events()
    });

    let body = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"MSI_Dergisi_OpenData_Full_Backup.json\"",
            ),
        ],
        body,
    )
        .into_response()
}

pub async fn admin_list_notifications_api(
    operator: OperatorUser,
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
    query: Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["notifications:read", "settings:read"]) {
        return resp;
    }
    list_notifications_api(State(state), uri, headers, query)
        .await
        .into_response()
}

#[derive(Deserialize)]
pub struct CreateNotificationPayload {
    pub category: Option<String>,
    pub severity: Option<String>,
    pub title_tr: String,
    pub title_en: Option<String>,
    pub message_tr: String,
    pub message_en: Option<String>,
    pub target_audience: Option<String>,
    pub target_email: Option<String>,
    pub action_url: Option<String>,
    pub is_banner: Option<bool>,
}

pub async fn admin_create_notification_api(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateNotificationPayload>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["notifications:write", "settings:write"]) {
        return resp;
    }
    let raw_audience = payload.target_audience.as_deref().unwrap_or("all").trim();
    let normalized_audience = if raw_audience.is_empty() || raw_audience.eq_ignore_ascii_case("all")
    {
        "all"
    } else {
        raw_audience
    };

    let notif = crate::engine::notification_engine::build_notification_event(
        normalized_audience,
        payload
            .target_email
            .as_deref()
            .map(str::trim)
            .filter(|e| !e.is_empty()),
        payload.category.as_deref().unwrap_or("system"),
        payload.severity.as_deref().unwrap_or("info"),
        &payload.title_tr,
        payload.title_en.as_deref().unwrap_or(&payload.title_tr),
        &payload.message_tr,
        payload.message_en.as_deref().unwrap_or(&payload.message_tr),
        payload.action_url.as_deref(),
        payload.is_banner.unwrap_or(false),
    );
    let id = state.db.insert_notification(&notif).unwrap_or(0);
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "create",
        "notification",
        Some(id),
        &json!({ "audience": normalized_audience, "title_tr": payload.title_tr }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "created", "id": id, "notification": notif})).into_response()
}

pub async fn admin_list_settings_api(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    list_site_settings_api(State(state)).await.into_response()
}

#[derive(Deserialize)]
pub struct UpsertSettingPayload {
    pub key: String,
    pub value_tr: String,
    pub value_en: Option<String>,
    pub setting_group: Option<String>,
}

pub async fn admin_upsert_setting_api(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<UpsertSettingPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
    if payload.key == crate::engine::design::config::SETTING_KEY {
        let parsed =
            serde_json::from_str::<crate::engine::design::config::DesignConfig>(&payload.value_tr);
        if parsed
            .map_err(|e| e.to_string())
            .and_then(|c| c.validate())
            .is_err()
        {
            return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
        }
        if let Some(en) = payload.value_en.as_ref().filter(|v| !v.is_empty()) {
            if serde_json::from_str::<crate::engine::design::config::DesignConfig>(en)
                .map_err(|e| e.to_string())
                .and_then(|c| c.validate())
                .is_err()
            {
                return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::BAD_REQUEST);
            }
        }
    }
    let val_en = payload.value_en.as_deref().unwrap_or(&payload.value_tr);
    let grp = payload.setting_group.as_deref().unwrap_or("general");
    let setting = SiteSetting {
        key: payload.key.clone(),
        value_tr: payload.value_tr.clone(),
        value_en: val_en.to_string(),
        setting_group: grp.to_string(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let ok = state.db.upsert_site_setting(&setting).is_ok();
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upsert",
        "site_setting",
        None,
        &json!({ "key": payload.key }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({
        "key": payload.key,
        "updated": ok
    }))
    .into_response()
}

pub async fn admin_list_security_logs_api(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["audit:read", "settings:read"]) {
        return resp;
    }
    let logs = state.db.get_security_audit_logs();
    Json(json!({
        "total": logs.len(),
        "logs": logs
    }))
    .into_response()
}

// ============================================================================
// TRANSLATIONS, CONTENT VERSIONS, MEDIA LIBRARY, AUDIT LOGS, ROLES & USERS
// ============================================================================

#[derive(Deserialize)]
pub struct TranslationUpsertPayload {
    pub key: String,
    pub value_tr: Option<String>,
    pub value_en: Option<String>,
    pub value_ar: Option<String>,
    pub value_de: Option<String>,
    pub value_ru: Option<String>,
    pub value_fr: Option<String>,
    pub group: Option<String>,
}

pub async fn admin_update_category(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(cat): Json<CategoryNode>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:write", "settings:write"]) {
        return resp;
    }
    if matches!(
        validation_engine::validate_required_text(&cat.name_tr),
        validation_engine::TextFieldValidation::Empty
    ) || validation_engine::validate_slug(&cat.slug).is_none()
    {
        return AppError::ValidationError {
            msg_key: msg_keys::VALIDATION_ERROR,
        }
        .into_api_response(&state.translations, "tr")
        .into_response();
    }
    match state.db.update_category(id, &cat) {
        Ok(true) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "update",
                "category",
                Some(id),
                &json!({ "slug": cat.slug }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({"status": "updated", "id": id, "slug": cat.slug}))
                .into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::raw(
            StatusCode::BAD_REQUEST,
            json!({
                "status": "error",
                "error_code": error_codes::BAD_REQUEST,
                "message": e,
                "data": null
            }),
        )
        .into_response(),
    }
}

pub async fn admin_delete_category(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:write", "settings:write"]) {
        return resp;
    }
    match state.db.delete_category(id) {
        Ok(true) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "category",
                Some(id),
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({"status": "deleted", "id": id})).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::raw(
            StatusCode::BAD_REQUEST,
            json!({
                "status": "error",
                "error_code": error_codes::BAD_REQUEST,
                "message": e,
                "data": null
            }),
        )
        .into_response(),
    }
}

pub async fn admin_update_ssb_tag(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(tag): Json<SsbInterestTag>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:write", "settings:write"]) {
        return resp;
    }
    if tag.code.trim().is_empty() || tag.name_tr.trim().is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    match state.db.update_ssb_tag(&code, &tag) {
        Ok(true) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "update",
                "ssb_tag",
                None,
                &json!({ "old_code": code, "new_code": tag.code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({"status": "updated", "code": tag.code})).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::raw(
            StatusCode::BAD_REQUEST,
            json!({
                "status": "error",
                "error_code": error_codes::BAD_REQUEST,
                "message": e,
                "data": null
            }),
        )
        .into_response(),
    }
}

pub async fn admin_delete_ssb_tag(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["categories:write", "settings:write"]) {
        return resp;
    }
    match state.db.delete_ssb_tag(&code) {
        Ok(true) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "ssb_tag",
                None,
                &json!({ "code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({"status": "deleted", "code": code})).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::raw(
            StatusCode::BAD_REQUEST,
            json!({
                "status": "error",
                "error_code": error_codes::BAD_REQUEST,
                "message": e,
                "data": null
            }),
        )
        .into_response(),
    }
}

pub async fn admin_delete_redirect(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:write", "contents:publish"]) {
        return resp;
    }
    match state.db.delete_seo_redirect(id) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "seo_redirect",
                Some(id),
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({"status": "deleted", "id": id})).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_delete_redirect", &e)
                .into_response()
        }
    }
}

pub async fn admin_delete_newsletter(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "newsletters:write") {
        return resp;
    }
    match state.db.delete_newsletter_campaign(id) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "newsletter_campaign",
                Some(id),
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({"status": "deleted", "id": id})).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_delete_newsletter", &e)
                .into_response()
        }
    }
}

