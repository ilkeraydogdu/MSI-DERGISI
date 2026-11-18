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
            "contents:read",
            "ads:read",
            "categories:read",
            "users:read",
            "issues:read",
            "approvals:read",
            "approvals:approve",
            "commerce:read",
            "subscriptions:read",
            "invoices:read",
            "settings:read",
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

    let total_views: i64 = contents.iter().map(|c| c.view_count).sum();
    let total_true_reads: i64 = contents.iter().map(|c| c.true_read_count).sum();

    Json(json!({
        "total_contents": contents.len(),
        "total_views": total_views,
        "total_true_reads": total_true_reads,
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
    if wants_publish {
        if let Err(resp) = require_perm(&operator, "contents:publish") {
            return resp;
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
        "svg" => {
            let Ok(text) = std::str::from_utf8(bytes) else {
                return false;
            };
            let trimmed = text.trim_start();
            let lower = trimmed.to_ascii_lowercase();
            let valid_header = lower.starts_with("<svg") || (lower.starts_with("<?xml") && lower.contains("<svg"));
            if !valid_header {
                return false;
            }
            for forbidden in [
                "<script",
                "<foreignobject",
                "<iframe",
                "<object",
                "<embed",
                "<?xml-stylesheet",
                "javascript:",
                "vbscript:",
                "data:text/html",
                "&#",
            ] {
                if lower.contains(forbidden) {
                    return false;
                }
            }
            // Reject any inline event attribute (`on<event>=` including whitespace before `=`)
            let bytes_low = lower.as_bytes();
            let mut i = 0;
            while i + 3 < bytes_low.len() {
                let prev_ws = i == 0 || matches!(bytes_low[i - 1], b' ' | b'\t' | b'\n' | b'\r' | b'/');
                if prev_ws && bytes_low[i] == b'o' && bytes_low[i + 1] == b'n' && bytes_low[i + 2].is_ascii_alphabetic() {
                    let mut j = i + 2;
                    while j < bytes_low.len() && bytes_low[j].is_ascii_alphabetic() {
                        j += 1;
                    }
                    while j < bytes_low.len() && matches!(bytes_low[j], b' ' | b'\t' | b'\n' | b'\r') {
                        j += 1;
                    }
                    if j < bytes_low.len() && bytes_low[j] == b'=' {
                        return false;
                    }
                }
                i += 1;
            }
            true
        }
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

pub async fn admin_upsert_ad(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(ad): Json<AdCampaign>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "ads:write") {
        return resp;
    }
    let existing = state.db.get_all_ads();
    let conflict =
        ad_service::check_ad_conflict(&ad, &existing, state.config.ad_max_quota_per_keyword);
    let is_update = ad.id > 0;

    match state.db.upsert_ad(&ad) {
        Ok(id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                if is_update { "update" } else { "create" },
                "ad_campaign",
                Some(id),
                &json!({ "sponsor_name": ad.sponsor_name, "slot_code": ad.slot_code }).to_string(),
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
// EDITORIAL, REVENUE & USER ANALYTICS ENDPOINTS
