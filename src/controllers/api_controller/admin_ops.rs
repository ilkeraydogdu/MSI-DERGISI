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
}

/// Resolves the newsletter sponsor ad campaign:
/// - If `sponsor_ad_id` is `Some(id)` with `id > 0`, returns that ad.
/// - Otherwise (`None` or `<= 0`), automatically selects the highest-priority (`budget_score`)
///   active `AdCampaign` with `is_newsletter_ad == true` within schedule.
pub fn resolve_newsletter_sponsor_ad<'a>(
    all_ads: &'a [AdCampaign],
    sponsor_ad_id: Option<i64>,
    target_institution: &str,
    target_ssb_tag: &str,
    today_yyyy_mm_dd: &str,
) -> Option<&'a AdCampaign> {
    if let Some(aid) = sponsor_ad_id.filter(|&id| id > 0) {
        return all_ads.iter().find(|a| a.id == aid);
    }
    let inst = target_institution.trim();
    let inst_active = !inst.is_empty() && !inst.eq_ignore_ascii_case("ALL") && inst != "*";
    let tag = target_ssb_tag.trim();
    let tag_active = !tag.is_empty() && !tag.eq_ignore_ascii_case("ALL") && tag != "*";

    all_ads
        .iter()
        .filter(|a| a.is_active && a.is_newsletter_ad && ad_service::is_ad_within_schedule(a, today_yyyy_mm_dd))
        .max_by_key(|a| {
            let inst_tier = if !inst_active {
                0
            } else if a.target_institutions.iter().any(|i| i.eq_ignore_ascii_case(inst)) {
                2
            } else if a.target_institutions.is_empty()
                || a.target_institutions
                    .iter()
                    .any(|i| i == "*" || i.eq_ignore_ascii_case("all"))
            {
                1
            } else {
                0
            };
            let tag_tier = if !tag_active {
                0
            } else if a.target_ssb_tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                2
            } else if a.target_ssb_tags.is_empty() {
                1
            } else {
                0
            };
            let slot_bonus = if a.slot_code.eq_ignore_ascii_case("ad-newsletter-top") {
                1
            } else {
                0
            };
            (inst_tier + tag_tier, a.budget_score, slot_bonus, a.id)
        })
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
    let group_owned: Vec<crate::models::UserAccount>;
    let recipients: Vec<&crate::models::UserAccount> = if let Some(ref code) = nl
        .email_group_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        group_owned = state.db.resolve_email_group_recipients(
            code,
            &nl.target_institution,
            &nl.target_ssb_tag,
        );
        group_owned.iter().collect()
    } else {
        email_service::filter_newsletter_recipients(
            &users,
            &nl.target_group_filter,
            &nl.target_institution,
            &nl.target_ssb_tag,
        )
    };
    nl.recipient_count = recipients.len() as i64;

    let now_utc = chrono::Utc::now();
    let today_str = now_utc.format("%Y-%m-%d").to_string();
    let all_ads = state.db.get_all_ads();
    let resolved_sponsor = resolve_newsletter_sponsor_ad(
        &all_ads,
        nl.sponsor_ad_id,
        &nl.target_institution,
        &nl.target_ssb_tag,
        &today_str,
    )
    .cloned();
    if nl.sponsor_ad_id.filter(|&id| id > 0).is_none() {
        if let Some(ref sp) = resolved_sponsor {
            nl.sponsor_ad_id = Some(sp.id);
            if nl.sponsor_name.as_deref().map(str::trim).unwrap_or("").is_empty() {
                nl.sponsor_name = Some(sp.sponsor_name.clone());
            }
        }
    } else if nl.sponsor_name.as_deref().map(str::trim).unwrap_or("").is_empty() {
        if let Some(ref sp) = resolved_sponsor {
            nl.sponsor_name = Some(sp.sponsor_name.clone());
        }
    }
    if let Some(raw_sched) = nl.scheduled_at.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw_sched) {
            nl.scheduled_at = Some(dt.with_timezone(&chrono::Utc).format("%Y-%m-%dT%H:%M:%SZ").to_string());
        } else if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(raw_sched, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(raw_sched, "%Y-%m-%d %H:%M:%S"))
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(raw_sched, "%Y-%m-%dT%H:%M"))
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(raw_sched, "%Y-%m-%d %H:%M"))
        {
            nl.scheduled_at = Some(ndt.and_utc().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
    }
    let now_cmp = now_utc.format("%Y-%m-%d %H:%M:%S").to_string();
    let is_future_schedule = nl
        .scheduled_at
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let norm: String = s.chars().take(19).collect::<String>().replace('T', " ");
            norm > now_cmp
        })
        .unwrap_or(false);

    if is_future_schedule {
        nl.status = "scheduled".to_string();
        nl.sent_at = String::new();
        let id = state.db.insert_newsletter_campaign(&nl).unwrap_or(0);
        let client_ip = extract_client_ip(&headers);
        let _ = state.db.insert_audit_log(
            &operator.email,
            "schedule",
            "newsletter_campaign",
            Some(id),
            &json!({
                "subject_tr": nl.subject_tr,
                "recipient_count": nl.recipient_count,
                "scheduled_at": nl.scheduled_at
            })
            .to_string(),
            &client_ip,
        );
        return ApiResponse::ok(json!({
            "status": nl.status,
            "id": id,
            "recipient_count": nl.recipient_count,
            "scheduled_at": nl.scheduled_at
        }))
        .into_response();
    }

    let req_status = nl.status.trim().to_ascii_lowercase();
    if req_status == "scheduled" {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    if req_status == "draft" {
        nl.status = "draft".to_string();
        nl.sent_at = String::new();
        nl.scheduled_at = None;
        nl.recipient_count = 0;
        let id = state.db.insert_newsletter_campaign(&nl).unwrap_or(0);
        let client_ip = extract_client_ip(&headers);
        let _ = state.db.insert_audit_log(
            &operator.email,
            "draft",
            "newsletter_campaign",
            Some(id),
            &json!({
                "subject_tr": nl.subject_tr,
                "recipient_count": 0
            })
            .to_string(),
            &client_ip,
        );
        return ApiResponse::ok(json!({
            "status": "draft",
            "id": id,
            "recipient_count": 0,
            "scheduled_at": null
        }))
        .into_response();
    }

    if nl.sent_at.trim().is_empty() {
        nl.sent_at = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
    }
    nl.status = if state.config.mail_delivery_enabled {
        "sent".to_string()
    } else {
        "queued".to_string()
    };

    let sponsor_ad = resolved_sponsor;
    let body_opt = (!nl.body_html.trim().is_empty()).then_some(nl.body_html.as_str());
    let dispatch = email_service::dispatch_newsletter_campaign(
        &state.config,
        &state.db,
        &nl,
        &recipients,
        sponsor_ad.as_ref(),
        body_opt,
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
    if let Err(resp) = require_any_perm(
        &operator,
        &[
            "settings:read",
            "contents:read",
            "redirects:write",
            "redirects:manage",
        ],
    ) {
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
    if let Err(resp) = require_any_perm(
        &operator,
        &[
            "settings:write",
            "contents:publish",
            "redirects:write",
            "redirects:manage",
        ],
    ) {
        return resp;
    }
    if crate::db::Database::is_protected_admin_redirect_source(
        rd.old_path.trim(),
        Some(&state.config.admin_path_secret),
    ) {
        return ApiResponse::raw(
            StatusCode::BAD_REQUEST,
            json!({
                "status": "error",
                "error_code": error_codes::BAD_REQUEST,
                "message": "Yönetim veya kullanımdan kaldırılmış admin rota adresleri yönlendirilemez.",
                "data": null
            }),
        )
        .into_response();
    }
    if !crate::db::Database::is_safe_seo_redirect_target(rd.new_path.trim()) {
        return ApiResponse::raw(
            StatusCode::BAD_REQUEST,
            json!({
                "status": "error",
                "error_code": error_codes::BAD_REQUEST,
                "message": "Hedef yol yalnızca güvenli site-içi göreli bir yol ('/' ile başlayan) olabilir.",
                "data": null
            }),
        )
        .into_response();
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
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:export", "settings:write"]) {
        return resp;
    }
    let anonymize = matches!(
        params.get("anonymize").map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes")
    );
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "export_open_data",
        "system",
        None,
        &json!({ "anonymized": anonymize }).to_string(),
        &client_ip,
    );

    let raw_users = state.db.get_all_users();
    let users_payload = if anonymize {
        json!(analytics_service::anonymize_open_data_users(&raw_users, &state.config.session_secret))
    } else {
        json!(raw_users)
    };
    let corp_requests_payload = if anonymize {
        json!([])
    } else {
        json!(state.db.get_corporate_requests())
    };
    let invoices_payload = if anonymize {
        let invs: Vec<serde_json::Value> = state
            .db
            .get_order_invoices()
            .into_iter()
            .map(|inv| {
                json!({
                    "invoice_no": inv.invoice_no,
                    "item_type": inv.item_type,
                    "item_code": inv.item_code,
                    "amount_try": inv.amount_try,
                    "payment_status": inv.payment_status,
                    "created_at": inv.created_at
                })
            })
            .collect();
        json!(invs)
    } else {
        json!(state.db.get_order_invoices())
    };
    let events_payload = if anonymize {
        use sha2::Digest;
        let raw_events = state.db.get_all_analytics_events();
        let mut cat_counts: HashMap<String, usize> = HashMap::new();
        let mut cat_actors: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        for (idx, ev) in raw_events.iter().enumerate() {
            let cat = ev.organization_category.trim().to_string();
            if !cat.is_empty() {
                *cat_counts.entry(cat.clone()).or_insert(0) += 1;
                let actor = if let Some(uid) = ev.user_id {
                    format!("u:{}", uid)
                } else if !ev.session_id.trim().is_empty() {
                    format!("s:{}", ev.session_id.trim())
                } else {
                    format!("i:{}", idx)
                };
                cat_actors.entry(cat).or_default().insert(actor);
            }
        }
        let evs: Vec<serde_json::Value> = raw_events
            .into_iter()
            .map(|mut ev| {
                ev.user_id = None;
                ev.organization_name = String::new();
                if !ev.session_id.trim().is_empty() {
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(
                        format!(
                            "msi-anon-sess:{}:{}",
                            state.config.session_secret,
                            ev.session_id.trim()
                        )
                        .as_bytes(),
                    );
                    let digest = hex::encode(hasher.finalize());
                    ev.session_id = format!("anon_sess_{}", &digest[..12]);
                }
                let cat = ev.organization_category.trim().to_string();
                let count_ok = !cat.is_empty()
                    && cat_counts.get(&cat).copied().unwrap_or(0) >= 3
                    && cat_actors.get(&cat).map(|s| s.len()).unwrap_or(0) >= 3;
                if !count_ok {
                    ev.organization_category = "Diğer Kurumlar (k<3)".to_string();
                }
                json!(ev)
            })
            .collect();
        json!(evs)
    } else {
        json!(state.db.get_all_analytics_events())
    };
    let reminders_payload = if anonymize {
        use sha2::Digest;
        let rems: Vec<serde_json::Value> = state
            .db
            .get_renewal_reminders()
            .into_iter()
            .map(|r| {
                let mut hasher = sha2::Sha256::new();
                hasher.update(
                    format!(
                        "msi-anon-rem:{}:{}:{}",
                        state.config.session_secret,
                        r.user_id,
                        r.user_email.trim().to_lowercase()
                    )
                    .as_bytes(),
                );
                let digest = hex::encode(hasher.finalize());
                json!({
                    "anon_id": format!("anon_{}", &digest[..8]),
                    "stage_label": r.stage_label,
                    "days_remaining": r.days_remaining,
                    "status": r.status,
                    "sent_at": r.sent_at
                })
            })
            .collect();
        json!(rems)
    } else {
        json!(state.db.get_renewal_reminders())
    };

    let payload = json!({
        "export_standard": "UMSA-MSI-OPEN-DATA-V1 (Zero Vendor-Lock Universal JSON/PostgreSQL/SQLite Portable Archive)",
        "anonymized": anonymize,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "domains": [&state.config.primary_domain, &state.config.news_domain],
        "categories": state.db.get_all_categories(),
        "ssb_interest_tags": state.db.get_all_ssb_tags(),
        "contents_single_source_of_truth": state.db.get_all_contents(),
        "magazine_issues": state.db.get_all_issues(),
        "ad_campaigns": state.db.get_all_ads(),
        "recommendation_rules": state.db.get_recommendation_rules(),
        "users_kvkk_verified": users_payload,
        "corporate_verification_requests": corp_requests_payload,
        "subscription_plans": state.db.get_subscription_plans(),
        "commerce_products": state.db.get_commerce_products(),
        "order_invoices": invoices_payload,
        "renewal_reminders_5_stage": reminders_payload,
        "newsletter_campaigns": state.db.get_newsletter_campaigns(),
        "seo_301_redirects": state.db.get_seo_redirects(),
        "analytics_events": events_payload
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
    _uri: Uri,
    _headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["notifications:read", "settings:read"]) {
        return resp;
    }
    let email_opt = params.get("email").map(|s| s.as_str()).filter(|s| !s.trim().is_empty());
    let org_opt = params.get("org").map(|s| s.as_str()).filter(|s| !s.trim().is_empty());
    let all = state.db.get_system_notifications(None);
    let filtered: Vec<_> = if email_opt.is_some() || org_opt.is_some() {
        crate::engine::notification_engine::filter_user_notifications(&all, email_opt, org_opt)
    } else {
        all
    };
    let banners = crate::engine::notification_engine::filter_active_banners(&filtered);
    Json(json!({
        "notifications": filtered,
        "active_banners": banners,
        "total": filtered.len()
    }))
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
    if let Err(resp) = require_any_perm(
        &operator,
        &[
            "settings:write",
            "contents:publish",
            "redirects:write",
            "redirects:manage",
        ],
    ) {
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

