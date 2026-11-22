use super::*;

// ============================================================================

pub async fn admin_overview(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(
        &operator,
        &[
            "analytics:read",
            "analytics:export",
            "contents:read",
            "contents:delete",
            "ads:read",
            "categories:read",
            "users:read",
            "issues:read",
            "issues:delete",
            "approvals:read",
            "approvals:approve",
            "approvals:reject",
            "commerce:read",
            "subscriptions:read",
            "invoices:read",
            "revenue:read",
            "notifications:read",
            "corporate_requests:read",
            "corporate_requests:decide",
            "audit:read",
            "settings:read",
            "media:read",
            "media:delete",
            "monitoring:read",
            "translations:read",
            "glossary:write",
            "glossary:manage",
            "locales:manage",
            "newsletters:read",
            "email_groups:write",
            "email_groups:manage",
            "backup:manage",
            "roles:manage",
            "redirects:write",
            "redirects:manage",
            "system:read",
        ],
    ) {
        return resp;
    }
    let contents = if operator.has_permission("contents:read") || operator.has_permission("analytics:read") {
        state.db.get_all_contents()
    } else {
        Vec::new()
    };
    let active_ads = if operator.has_permission("ads:read") || operator.has_permission("analytics:read") {
        state.db.get_all_ads().iter().filter(|a| a.is_active).count()
    } else {
        0
    };
    let registered_users = if operator.has_permission("users:read") || operator.has_permission("users:manage") {
        state.db.get_all_users().len()
    } else {
        0
    };
    let pending_reqs = if operator.has_permission("corporate_requests:read") || operator.has_permission("users:verify") {
        state
            .db
            .get_corporate_requests()
            .iter()
            .filter(|r| r.status == "pending")
            .count()
    } else {
        0
    };
    let (scheduled_reminders, halted_reminders) = if operator.has_permission("subscriptions:read") {
        let reminders = state.db.get_renewal_reminders();
        (
            reminders.iter().filter(|r| r.status == "scheduled").count(),
            reminders.iter().filter(|r| r.status == "halted_by_renewal").count(),
        )
    } else {
        (0, 0)
    };
    let invoices_count = if operator.has_permission("invoices:read") || operator.has_permission("commerce:read") {
        state.db.get_order_invoices().len()
    } else {
        0
    };
    let pending_approvals = if operator.has_permission("approvals:read")
        || operator.has_permission("approvals:approve")
        || operator.has_permission("contents:publish")
    {
        state.db.list_pending_approvals().len()
    } else {
        0
    };

    // Compute real analytics totals from analytics_events, not seed/harvested counters.
    let events = state.db.get_all_analytics_events();
    let total_views: i64 = events.len() as i64;
    let total_true_reads: i64 = events
        .iter()
        .filter(|e| {
            (e.duration_sec >= crate::db::constants::TRUE_READ_MIN_DURATION_SEC
                && e.scroll_depth_pct >= crate::db::constants::TRUE_READ_MIN_SCROLL_PCT)
                || e.is_true_read
        })
        .count() as i64;
    let total_impressions: i64 = events
        .iter()
        .filter(|e| {
            matches!(
                e.event_type.as_str(),
                "ad_impression" | "impression" | "ad_view"
            )
        })
        .count() as i64;
    let total_placements: i64 = contents.iter().map(|c| c.placements.len() as i64).sum();

    Json(json!({
        "total_contents": contents.len(),
        "total_views": total_views,
        "total_true_reads": total_true_reads,
        "total_impressions": total_impressions,
        "total_placements": total_placements,
        "active_ads": active_ads,
        "registered_users": registered_users,
        "pending_corporate_requests": pending_reqs,
        "pending_approvals": pending_approvals,
        "scheduled_reminders": scheduled_reminders,
        "halted_reminders": halted_reminders,
        "invoices_count": invoices_count
    }))
    .into_response()
}

pub async fn list_admin_contents(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:read") {
        return resp;
    }
    let criteria = CentralContentQueryEngine::from_params(&params);
    let result = CentralContentQueryEngine::execute_admin(&state.db, &criteria);
    ApiResponse::ok_with_meta(
        json!(result.items),
        json!({
            "page": result.current_page,
            "per_page": result.per_page,
            "total_count": result.total_count,
            "total_pages": result.total_pages
        }),
    )
    .into_response()
}

pub async fn admin_upsert_content(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(mut item): Json<ContentItem>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:write") {
        return resp;
    }
    let wants_publish = item.status.trim().is_empty() || item.status.trim() == "published";
    if is_privileged_content_status(&item.status) {
        if let Err(resp) = require_perm(&operator, "contents:publish") {
            return resp;
        }
    }
    if item.id > 0 {
        if let Some(existing_status) = state.db.get_content_status_by_id(item.id) {
            if is_privileged_content_status(&existing_status) {
                if let Err(resp) = require_perm(&operator, "contents:publish") {
                    return resp;
                }
            }
        }
    }

    item.slug = validation_engine::validate_slug(&item.slug)
        .unwrap_or_else(|| validation_engine::validate_slug(&item.title_tr).unwrap_or_default());
    item.body_tr = validation_engine::sanitize_html(&item.body_tr);
    item.body_en = validation_engine::sanitize_html(&item.body_en);

    let is_update = item.id > 0;
    match state.db.upsert_content(&item) {
        Ok(id) => {
            CentralContentQueryEngine::invalidate_cache();
            let change_note = if is_update {
                format!("İçerik güncellendi ({})", item.status)
            } else {
                format!("İlk kayıt oluşturuldu ({})", item.status)
            };
            let _ = state.db.insert_content_version_bilingual(
                id,
                &item.title_tr,
                &item.title_en,
                &item.body_tr,
                &item.body_en,
                &item.summary_tr,
                &item.summary_en,
                &operator.email,
                &change_note,
            );

            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                if is_update { "update" } else { "create" },
                "content",
                Some(id),
                &json!({ "slug": item.slug, "status": item.status }).to_string(),
                &client_ip,
            );

            let indexnow_queued = if wants_publish {
                crate::services::indexnow_service::spawn_instant_seo_push(
                    (*state.config).clone(),
                    vec![
                        crate::engine::url_builder::article_url(&item.slug),
                        "/".to_string(),
                        crate::engine::url_builder::category_url(&item.content_type),
                    ],
                );
                true
            } else {
                false
            };

            ApiResponse::ok(json!({
                "status": "saved",
                "id": id,
                "slug": item.slug,
                "placements_count": item.placements.len(),
                "indexnow_queued": indexnow_queued
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error_with_key(
            &state.translations,
            "tr",
            "admin_upsert_content",
            &e,
            msg_keys::SERVER_ERROR_SAVE,
        )
        .into_response(),
    }
}

pub async fn admin_delete_content(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:delete") {
        return resp;
    }
    if let Err(e) = state.db.delete_content(id) {
        return ApiResponse::server_error(&state.translations, "tr", "admin_delete_content", &e)
            .into_response();
    }
    CentralContentQueryEngine::invalidate_cache();
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "delete",
        "content",
        Some(id),
        "{}",
        &client_ip,
    );
    ApiResponse::ok(json!({"status": "deleted", "id": id})).into_response()
}

pub async fn admin_ping_indexnow(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:write", "contents:publish"]) {
        return resp;
    }
    let mut urls: Vec<String> = vec![
        "/".to_string(),
        "/e-dergi".to_string(),
        "/sitemap.xml".to_string(),
        "/sitemap-news.xml".to_string(),
        "/llms.txt".to_string(),
    ];
    for c in state
        .db
        .get_all_contents()
        .into_iter()
        .filter(|c| content_service::is_publicly_visible(c))
        .take(crate::db::constants::API_FEED_CONTENT_LIMIT)
    {
        urls.push(crate::engine::url_builder::article_url(&c.slug));
    }
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "indexnow_ping",
        "seo",
        None,
        &json!({ "pushed_count": urls.len() }).to_string(),
        &client_ip,
    );
    let key = crate::services::indexnow_service::resolve_indexnow_key(&state.config);
    match crate::services::indexnow_service::push_urls_now(&state.config, &urls).await {
        Ok(code) => Json(json!({
            "status": "pushed",
            "http_status": code,
            "indexnow_key": key,
            "pushed_count": urls.len()
        }))
        .into_response(),
        Err(err) => Json(json!({
            "status": "queued_or_unreachable",
            "detail": err,
            "indexnow_key": key,
            "pushed_count": urls.len()
        }))
        .into_response(),
    }
}

fn validate_upload_signature(ext: &str, bytes: &[u8]) -> bool {
    match ext {
        "webp" => bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "pdf" => bytes.starts_with(b"%PDF-"),
        "mp4" => bytes.len() >= 12 && &bytes[4..8] == b"ftyp",
        "svg" => crate::engine::validation_engine::validate_svg_safe(bytes).is_ok(),
        _ => false,
    }
}

pub async fn admin_upload_media(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Response {
    if let Err(resp) = require_perm(&operator, "media:upload") {
        return resp;
    }

    let mut all_urls: Vec<String> = Vec::new();
    let mut total_bytes: usize = 0;

    while let Ok(Some(field)) = multipart.next_field().await {
        let Some(orig_name) = field.file_name().map(|s| s.to_string()) else {
            continue;
        };
        if orig_name.trim().is_empty() {
            continue;
        }
        let ext = FsPath::new(&orig_name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        if !media_service::is_allowed_extension(&ext) {
            return ApiResponse::error(
                &state.translations,
                "tr",
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::INVALID_FILE_TYPE,
            )
            .into_response();
        }

        let bytes = match field.bytes().await {
            Ok(b) if media_service::is_valid_size(b.len()) => b,
            _ => {
                return ApiResponse::error(
                    &state.translations,
                    "tr",
                    StatusCode::BAD_REQUEST,
                    error_codes::BAD_REQUEST,
                    msg_keys::FILE_TOO_LARGE,
                )
                .into_response();
            }
        };

        if !validate_upload_signature(&ext, &bytes) {
            return ApiResponse::error(
                &state.translations,
                "tr",
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::INVALID_FILE_TYPE,
            )
            .into_response();
        }

        let (relative_url, filename) = media_service::generate_upload_path(
            &orig_name,
            &ext,
            security_engine::sanitize_slug,
        );

        let (target_dir, stored_url) = if ext == "pdf" {
            (
                state.config.edergi_dir.trim_end_matches('/').to_string(),
                format!("/edergi-files/{}", filename),
            )
        } else {
            let subdir = relative_url
                .rsplit_once('/')
                .map(|(d, _)| d)
                .unwrap_or("images/uploads");
            (
                format!(
                    "{}/{}",
                    state.config.static_dir.trim_end_matches('/'),
                    subdir
                ),
                relative_url,
            )
        };
        if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
            return ApiResponse::server_error_with_key(
                &state.translations,
                "tr",
                "admin_upload_mkdir",
                &e,
                msg_keys::SERVER_ERROR_UPLOAD,
            )
            .into_response();
        }

        let full_path = format!("{}/{}", target_dir, filename);
        if let Err(e) = tokio::fs::write(&full_path, &bytes).await {
            return ApiResponse::server_error_with_key(
                &state.translations,
                "tr",
                "admin_upload_write",
                &e,
                msg_keys::SERVER_ERROR_FILE_WRITE,
            )
            .into_response();
        }

        if let Err(e) = media_service::register_media(
            &state.db,
            &filename,
            &orig_name,
            &ext,
            bytes.len(),
            &stored_url,
            &operator.email,
        ) {
            let _ = tokio::fs::remove_file(&full_path).await;
            return ApiResponse::server_error_with_key(
                &state.translations,
                "tr",
                "admin_upload_register",
                &e,
                msg_keys::SERVER_ERROR_SAVE,
            )
            .into_response();
        }

        total_bytes += bytes.len();
        all_urls.push(stored_url);
    }

    let Some(first_url) = all_urls.first().cloned() else {
        return ApiResponse::error(
            &state.translations,
            "tr",
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::UPLOAD_NO_FIELD,
        )
        .into_response();
    };

    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upload",
        "media",
        None,
        &json!({ "count": all_urls.len(), "first_url": first_url }).to_string(),
        &client_ip,
    );

    Json(json!({
        "status": "uploaded",
        "url": first_url,
        "path": first_url,
        "urls": all_urls,
        "size_bytes": total_bytes
    }))
    .into_response()
}

pub async fn list_admin_ads(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "ads:read") {
        return resp;
    }
    Json(state.db.get_all_ads()).into_response()
}

#[derive(Deserialize)]
pub struct CheckConflictPayload {
    pub slot_code: String,
    pub sponsor_name: String,
    pub target_institutions: Option<Vec<String>>,
    pub target_ssb_tags: Option<Vec<String>>,
    pub target_categories: Option<Vec<String>>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub rotation_mode: Option<String>,
}

pub async fn admin_check_ad_conflict(
    operator: OperatorUser,
    State(state): State<AppState>,
    Json(payload): Json<CheckConflictPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "ads:read") {
        return resp;
    }
    let candidate = AdCampaign {
        id: 0,
        slot_code: payload.slot_code,
        badge_label: "SPONSORLU".to_string(),
        dimensions: "970x90".to_string(),
        sponsor_name: payload.sponsor_name,
        sponsor_type: "defense_oem".to_string(),
        pill_style: "navy".to_string(),
        title_tr: String::new(),
        title_en: String::new(),
        desc_tr: String::new(),
        desc_en: String::new(),
        target_url: String::new(),
        image_url: None,
        budget_score: 80,
        is_default_fallback: false,
        is_floating_popup: false,
        is_newsletter_ad: false,
        target_categories: payload.target_categories.unwrap_or_default(),
        target_ssb_tags: payload.target_ssb_tags.unwrap_or_default(),
        target_institutions: payload.target_institutions.unwrap_or_default(),
        impressions_count: 0,
        clicks_count: 0,
        is_active: true,
        start_date: payload.start_date.clone(),
        end_date: payload.end_date.clone(),
        rotation_mode: payload
            .rotation_mode
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "weighted".to_string()),
    };

    let existing = state.db.get_all_ads();
    let warning = ad_service::check_ad_conflict_with_window(
        &candidate,
        &existing,
        state.config.ad_max_quota_per_keyword,
        payload.start_date.as_deref(),
        payload.end_date.as_deref(),
        payload.rotation_mode.as_deref(),
    );

    Json(json!(warning)).into_response()
}

#[derive(Deserialize)]
pub struct UpsertAdPayload {
    #[serde(flatten)]
    pub ad: AdCampaign,
    #[serde(default)]
    pub force_override: Option<bool>,
}

pub async fn admin_upsert_ad(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<UpsertAdPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "ads:write") {
        return resp;
    }

    let mut ad = payload.ad;
    ad.start_date = ad
        .start_date
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);
    ad.end_date = ad
        .end_date
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);
    if ad.rotation_mode.trim().is_empty() {
        ad.rotation_mode = "weighted".to_string();
    }

    let parsed_start = match ad_service::parse_date_prefix(ad.start_date.as_deref()) {
        Some(Ok(d)) => Some(d),
        Some(Err(())) => {
            return ApiResponse::raw(
                StatusCode::BAD_REQUEST,
                json!({
                    "status": "error",
                    "error_code": error_codes::BAD_REQUEST,
                    "error": "Geçersiz kampanya tarih formatı (YYYY-MM-DD bekleniyor).",
                    "message": "Geçersiz kampanya tarih formatı (YYYY-MM-DD bekleniyor).",
                    "data": null
                }),
            )
            .into_response();
        }
        None => None,
    };
    let parsed_end = match ad_service::parse_date_prefix(ad.end_date.as_deref()) {
        Some(Ok(d)) => Some(d),
        Some(Err(())) => {
            return ApiResponse::raw(
                StatusCode::BAD_REQUEST,
                json!({
                    "status": "error",
                    "error_code": error_codes::BAD_REQUEST,
                    "error": "Geçersiz kampanya tarih formatı (YYYY-MM-DD bekleniyor).",
                    "message": "Geçersiz kampanya tarih formatı (YYYY-MM-DD bekleniyor).",
                    "data": null
                }),
            )
            .into_response();
        }
        None => None,
    };

    if let (Some(start), Some(end)) = (parsed_start, parsed_end) {
        if start > end {
            return ApiResponse::raw(
                StatusCode::BAD_REQUEST,
                json!({
                    "status": "error",
                    "error_code": error_codes::BAD_REQUEST,
                    "error": "Kampanya bitiş tarihi başlangıç tarihinden önce olamaz.",
                    "message": "Kampanya bitiş tarihi başlangıç tarihinden önce olamaz.",
                    "data": null
                }),
            )
            .into_response();
        }
    }

    let existing = state.db.get_all_ads();
    let conflict = ad_service::check_ad_conflict_with_window(
        &ad,
        &existing,
        state.config.ad_max_quota_per_keyword,
        ad.start_date.as_deref(),
        ad.end_date.as_deref(),
        Some(&ad.rotation_mode),
    );

    if ad.is_active
        && !ad.is_default_fallback
        && conflict.has_conflict
        && payload.force_override != Some(true)
    {
        return ApiResponse::raw(
            StatusCode::CONFLICT,
            json!({
                "status": "error",
                "error_code": error_codes::CONFLICT,
                "error": conflict.message_tr,
                "message": conflict.message_tr,
                "conflict_check": conflict
            }),
        )
        .into_response();
    }

    let is_update = ad.id > 0;

    match state.db.upsert_ad(&ad) {
        Ok(id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                if is_update { "update" } else { "create" },
                "ad_campaign",
                Some(id),
                &json!({
                    "sponsor_name": ad.sponsor_name,
                    "slot_code": ad.slot_code,
                    "start_date": ad.start_date,
                    "end_date": ad.end_date,
                    "rotation_mode": ad.rotation_mode,
                    "force_override": payload.force_override.unwrap_or(false)
                })
                .to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "saved",
                "id": id,
                "conflict_check": conflict
            }))
            .into_response()
        }
        Err(e) => AppError::internal("ad_upsert", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response(),
    }
}

pub async fn admin_delete_ad(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "ads:write") {
        return resp;
    }
    match state.db.delete_ad_campaign(id) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "ad_campaign",
                Some(id),
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "id": id })).into_response()
        }
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_delete_ad", &e)
            .into_response(),
    }
}

// ============================================================================
// RECOMMENDATION RULES CRUD (REQ-012)
// ============================================================================

fn default_rule_priority() -> i32 {
    10
}

fn default_rule_active() -> bool {
    true
}

#[derive(Deserialize)]
pub struct RecommendationRulePayload {
    #[serde(default)]
    pub rule_name: String,
    #[serde(default, alias = "source_category")]
    pub source_tag_or_cat: String,
    #[serde(default, alias = "target_category")]
    pub target_institution: String,
    #[serde(default)]
    pub recommended_content_slug: String,
    #[serde(default = "default_rule_priority")]
    pub priority: i32,
    #[serde(default = "default_rule_active")]
    pub is_active: bool,
}

pub async fn admin_list_recommendation_rules(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["contents:read", "settings:read", "ads:read"]) {
        return resp;
    }
    ApiResponse::ok(json!({ "rules": state.db.get_recommendation_rules() })).into_response()
}

pub async fn admin_create_recommendation_rule(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<RecommendationRulePayload>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["contents:write", "contents:manage", "settings:write"])
    {
        return resp;
    }
    if payload.rule_name.trim().is_empty()
        || payload.source_tag_or_cat.trim().is_empty()
        || payload.recommended_content_slug.trim().is_empty()
    {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    let mut rule = crate::models::RecommendationRule {
        id: 0,
        rule_name: payload.rule_name.trim().to_string(),
        source_tag_or_cat: payload.source_tag_or_cat.trim().to_string(),
        target_institution: if payload.target_institution.trim().is_empty() {
            "*".to_string()
        } else {
            payload.target_institution.trim().to_string()
        },
        recommended_content_slug: payload.recommended_content_slug.trim().to_string(),
        priority: payload.priority,
        is_active: payload.is_active,
    };

    match state.db.insert_recommendation_rule(&rule) {
        Ok(id) => {
            rule.id = id;
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "create",
                "recommendation_rule",
                Some(id),
                &json!({
                    "rule_name": rule.rule_name,
                    "source_tag_or_cat": rule.source_tag_or_cat,
                    "recommended_content_slug": rule.recommended_content_slug,
                })
                .to_string(),
                &client_ip,
            );
            ApiResponse::ok_with_status(
                StatusCode::CREATED,
                json!({ "status": "created", "id": id, "rule": rule }),
            )
            .into_response()
        }
        Err(e) => ApiResponse::server_error(
            &state.translations,
            "tr",
            "admin_create_recommendation_rule",
            &e,
        )
        .into_response(),
    }
}

pub async fn admin_update_recommendation_rule(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RecommendationRulePayload>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["contents:write", "contents:manage", "settings:write"])
    {
        return resp;
    }
    if payload.rule_name.trim().is_empty()
        || payload.source_tag_or_cat.trim().is_empty()
        || payload.recommended_content_slug.trim().is_empty()
    {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    let rule = crate::models::RecommendationRule {
        id,
        rule_name: payload.rule_name.trim().to_string(),
        source_tag_or_cat: payload.source_tag_or_cat.trim().to_string(),
        target_institution: if payload.target_institution.trim().is_empty() {
            "*".to_string()
        } else {
            payload.target_institution.trim().to_string()
        },
        recommended_content_slug: payload.recommended_content_slug.trim().to_string(),
        priority: payload.priority,
        is_active: payload.is_active,
    };

    match state.db.update_recommendation_rule(id, &rule) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "update",
                "recommendation_rule",
                Some(id),
                &json!({
                    "rule_name": rule.rule_name,
                    "priority": rule.priority,
                    "is_active": rule.is_active,
                })
                .to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "updated", "id": id, "rule": rule })).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::server_error(
            &state.translations,
            "tr",
            "admin_update_recommendation_rule",
            &e,
        )
        .into_response(),
    }
}

pub async fn admin_delete_recommendation_rule(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["contents:write", "contents:manage", "settings:write"])
    {
        return resp;
    }

    match state.db.delete_recommendation_rule(id) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "recommendation_rule",
                Some(id),
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "id": id })).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::server_error(
            &state.translations,
            "tr",
            "admin_delete_recommendation_rule",
            &e,
        )
        .into_response(),
    }
}

// ============================================================================
// EDITORIAL, REVENUE & USER ANALYTICS ENDPOINTS

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::db::Database;
    use crate::engine::translation_engine::TranslationCache;
    use std::sync::Arc;

    fn make_admin_operator() -> OperatorUser {
        OperatorUser {
            user_id: 1,
            email: "admin_cms_test@msidergisi.com".to_string(),
            full_name: "Test Super Admin".to_string(),
            role: "super_admin".to_string(),
            permissions: vec!["ads:write".to_string(), "ads:read".to_string()],
        }
    }

    fn make_test_state(tag: &str) -> (AppState, String) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir()
            .join(format!("msi_{}_{}_{}.db", tag, std::process::id(), ts))
            .to_string_lossy()
            .to_string();
        let mut cfg = AppConfig::from_env();
        cfg.app_env = "test".into();
        cfg.sqlite_db_path = db_path.clone();
        cfg.session_secret = "test-session-secret".into();
        cfg.monitoring_enabled = false;
        let db = Database::new(&db_path).expect("test db");
        let translations = TranslationCache::new();
        translations.load(db.load_all_translations());
        let state = AppState {
            config: Arc::new(cfg),
            db,
            translations,
            monitoring_cache: None,
        };
        (state, db_path)
    }

    fn cleanup_db(db_path: &str) {
        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }

    fn make_upsert_payload(start: Option<&str>, end: Option<&str>) -> UpsertAdPayload {
        UpsertAdPayload {
            ad: AdCampaign {
                id: 0,
                slot_code: "ad-news-top".to_string(),
                badge_label: "SPONSORLU".to_string(),
                dimensions: "970x90".to_string(),
                sponsor_name: "TestOEM".to_string(),
                sponsor_type: "defense_oem".to_string(),
                pill_style: "navy".to_string(),
                title_tr: "Test Kampanya".to_string(),
                title_en: "Test Campaign".to_string(),
                desc_tr: "Açıklama".to_string(),
                desc_en: "Description".to_string(),
                target_url: "https://example.com".to_string(),
                image_url: None,
                budget_score: 80,
                is_default_fallback: false,
                is_floating_popup: false,
                is_newsletter_ad: false,
                target_categories: vec!["haberler".to_string()],
                target_ssb_tags: vec!["test-tag".to_string()],
                target_institutions: vec!["MSB".to_string()],
                impressions_count: 0,
                clicks_count: 0,
                is_active: true,
                start_date: start.map(ToString::to_string),
                end_date: end.map(ToString::to_string),
                rotation_mode: "weighted".to_string(),
            },
            force_override: Some(false),
        }
    }

    async fn read_body_json(resp: Response) -> (StatusCode, Value) {
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let val = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, val)
    }

    #[tokio::test]
    async fn admin_upsert_ad_rejects_malformed_dates_with_400_and_accepts_iso_timestamps() {
        let (state, db_path) = make_test_state("admin_cms_dates");
        let op = make_admin_operator();

        // 1. Malformed end_date ("2020-01-01" .. "invalid-date") -> 400 Bad Request
        let resp_bad_end = admin_upsert_ad(
            op.clone(),
            HeaderMap::new(),
            State(state.clone()),
            Json(make_upsert_payload(Some("2020-01-01"), Some("invalid-date"))),
        )
        .await;
        let (st_end, body_end) = read_body_json(resp_bad_end).await;
        assert_eq!(st_end, StatusCode::BAD_REQUEST);
        assert_eq!(
            body_end["error"],
            "Geçersiz kampanya tarih formatı (YYYY-MM-DD bekleniyor)."
        );

        // 2. Malformed start_date ("invalid-date" .. "2035-12-31") -> 400 Bad Request
        let resp_bad_start = admin_upsert_ad(
            op.clone(),
            HeaderMap::new(),
            State(state.clone()),
            Json(make_upsert_payload(Some("invalid-date"), Some("2035-12-31"))),
        )
        .await;
        let (st_start, body_start) = read_body_json(resp_bad_start).await;
        assert_eq!(st_start, StatusCode::BAD_REQUEST);
        assert_eq!(
            body_start["error"],
            "Geçersiz kampanya tarih formatı (YYYY-MM-DD bekleniyor)."
        );

        // 3. Reversed valid dates ("2026-12-31" > "2026-01-01") -> 400 Bad Request
        let resp_rev = admin_upsert_ad(
            op.clone(),
            HeaderMap::new(),
            State(state.clone()),
            Json(make_upsert_payload(Some("2026-12-31"), Some("2026-01-01"))),
        )
        .await;
        let (st_rev, body_rev) = read_body_json(resp_rev).await;
        assert_eq!(st_rev, StatusCode::BAD_REQUEST);
        assert_eq!(
            body_rev["error"],
            "Kampanya bitiş tarihi başlangıç tarihinden önce olamaz."
        );

        // 4. Valid ISO timestamps -> 200 OK
        let resp_iso = admin_upsert_ad(
            op,
            HeaderMap::new(),
            State(state.clone()),
            Json(make_upsert_payload(
                Some("2026-11-21T00:00:00Z"),
                Some("2026-11-21T23:59:59Z"),
            )),
        )
        .await;
        let (st_iso, _) = read_body_json(resp_iso).await;
        assert_eq!(st_iso, StatusCode::OK);

        cleanup_db(&db_path);
    }
}
