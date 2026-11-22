use super::*;

pub async fn admin_list_translations_api(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["translations:read", "settings:read"]) {
        return resp;
    }
    let (entries, total_count) = state
        .db
        .load_all_translations_paginated(pagination.effective_per_page(), pagination.offset());
    let items: Vec<Value> = entries
        .into_iter()
        .map(|e| {
            json!({
                "key": e.key,
                "value_tr": e.value_tr,
                "value_en": e.value_en,
                "value_ar": e.value_ar,
                "value_de": e.value_de,
                "value_ru": e.value_ru,
                "value_fr": e.value_fr,
                "group": e.group,
            })
        })
        .collect();
    ApiResponse::ok_with_meta(json!(items), pagination.meta(total_count)).into_response()
}

pub async fn admin_upsert_translation_api(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<TranslationUpsertPayload>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["translations:write", "settings:write"]) {
        return resp;
    }
    let existing = state.translations.get_entry(&payload.key);
    let entry = crate::engine::translation_engine::TranslationEntry {
        key: payload.key.clone(),
        value_tr: payload
            .value_tr
            .or_else(|| existing.as_ref().map(|e| e.value_tr.clone()))
            .unwrap_or_default(),
        value_en: payload
            .value_en
            .or_else(|| existing.as_ref().map(|e| e.value_en.clone()))
            .unwrap_or_default(),
        value_ar: payload
            .value_ar
            .or_else(|| existing.as_ref().map(|e| e.value_ar.clone()))
            .unwrap_or_default(),
        value_de: payload
            .value_de
            .or_else(|| existing.as_ref().map(|e| e.value_de.clone()))
            .unwrap_or_default(),
        value_ru: payload
            .value_ru
            .or_else(|| existing.as_ref().map(|e| e.value_ru.clone()))
            .unwrap_or_default(),
        value_fr: payload
            .value_fr
            .or_else(|| existing.as_ref().map(|e| e.value_fr.clone()))
            .unwrap_or_default(),
        group: payload
            .group
            .or_else(|| existing.map(|e| e.group))
            .unwrap_or_else(|| "general".to_string()),
    };
    if let Err(e) = state.db.upsert_translation(&entry) {
        return AppError::internal("translation_upsert", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response();
    }
    state.translations.reload(state.db.load_all_translations());
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "upsert",
        "translation",
        None,
        &json!({ "key": payload.key }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"key": payload.key})).into_response()
}

pub async fn admin_delete_translation_api(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["translations:write", "settings:write"]) {
        return resp;
    }
    if let Err(e) = state.db.delete_translation(&key) {
        return AppError::internal("translation_delete", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response();
    }
    state.translations.reload(state.db.load_all_translations());
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "delete",
        "translation",
        None,
        &json!({ "key": key }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({"key": key, "action": "deleted"})).into_response()
}

pub async fn admin_list_content_versions(
    operator: OperatorUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:read") {
        return resp;
    }
    let (versions, total_count) = state.db.get_content_versions_paginated(
        id,
        pagination.effective_per_page(),
        pagination.offset(),
    );
    ApiResponse::ok_with_meta(json!(versions), pagination.meta(total_count)).into_response()
}

#[derive(Deserialize)]
pub struct ContentVersionPayload {
    #[serde(default)]
    pub content_id: i64,
    pub title_tr: String,
    pub body_tr: String,
    pub summary_tr: String,
    pub change_note: Option<String>,
}

pub async fn admin_create_content_version(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ContentVersionPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:write") {
        return resp;
    }
    if id <= 0 || (payload.content_id > 0 && payload.content_id != id) {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    if let Some(db_status) = state.db.get_content_status_by_id(id) {
        if is_privileged_content_status(&db_status) {
            if let Err(resp) = require_perm(&operator, "contents:publish") {
                return resp;
            }
        }
    }
    let clean_title_tr = validation_engine::sanitize_plain_text(&payload.title_tr);
    let clean_summary_tr = validation_engine::sanitize_plain_text(&payload.summary_tr);
    let clean_body_tr = validation_engine::sanitize_html(&payload.body_tr);
    match state.db.insert_content_version(
        id,
        &clean_title_tr,
        &clean_body_tr,
        &clean_summary_tr,
        &operator.email,
        &payload.change_note.unwrap_or_default(),
    ) {
        Ok(version_id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "create_version",
                "content",
                Some(id),
                &json!({ "version_id": version_id }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "created", "version_id": version_id })).into_response()
        }
        Err(e) => ApiResponse::server_error(
            &state.translations,
            "tr",
            "admin_create_content_version",
            &e,
        )
        .into_response(),
    }
}

pub async fn admin_restore_content_version(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((id, version)): Path<(i64, i64)>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:write") {
        return resp;
    }
    if let Some(db_status) = state.db.get_content_status_by_id(id) {
        if is_privileged_content_status(&db_status) {
            if let Err(resp) = require_perm(&operator, "contents:publish") {
                return resp;
            }
        }
    }
    match state
        .db
        .restore_content_version(id, version, &operator.email)
    {
        Ok(true) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "restore_version",
                "content",
                Some(id),
                &json!({ "restored_version": version }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "restored",
                "content_id": id,
                "version": version
            }))
            .into_response()
        }
        Ok(false) => json_error(&state, StatusCode::NOT_FOUND, msg_keys::CONTENT_NOT_FOUND),
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

pub async fn admin_list_media_library(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["media:read", "media:upload", "contents:read"])
    {
        return resp;
    }
    let query = params.get("q").map(String::as_str).unwrap_or("");
    let (items, total_count) = state.db.search_media_library_paginated(
        query,
        pagination.effective_per_page(),
        pagination.offset(),
    );
    ApiResponse::ok_with_meta(json!(items), pagination.meta(total_count)).into_response()
}

#[derive(Deserialize)]
pub struct MediaUpdatePayload {
    #[serde(default)]
    pub alt_text_tr: String,
    #[serde(default)]
    pub alt_text_en: String,
    #[serde(default)]
    pub tags: String,
}

pub async fn admin_update_media_item(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<MediaUpdatePayload>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["media:upload", "contents:write"]) {
        return resp;
    }
    match state.db.update_media_item(
        id,
        &payload.alt_text_tr,
        &payload.alt_text_en,
        &payload.tags,
    ) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "update_media",
                "media",
                Some(id),
                &json!({ "alt_text_tr": payload.alt_text_tr, "tags": payload.tags }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "updated", "id": id })).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_update_media_item", &e)
            .into_response(),
    }
}

pub async fn admin_delete_media_item(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["media:delete", "contents:delete"]) {
        return resp;
    }
    match state.db.delete_media_item(id) {
        Ok(Some(path_str)) => {
            if let Some(rel) = path_str.strip_prefix("/images/uploads/") {
                if let Some(fname) = FsPath::new(rel).file_name() {
                    let fp = FsPath::new(&state.config.static_dir)
                        .join("images")
                        .join("uploads")
                        .join(fname);
                    let _ = std::fs::remove_file(fp);
                }
            } else if let Some(rel) = path_str.strip_prefix("/edergi-files/") {
                if let Some(fname) = FsPath::new(rel).file_name() {
                    let fp = FsPath::new(&state.config.edergi_dir).join(fname);
                    let _ = std::fs::remove_file(fp);
                }
            }
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete_media",
                "media",
                Some(id),
                &json!({ "path": path_str }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "id": id })).into_response()
        }
        Ok(None) => not_found_error(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_delete_media_item", &e)
            .into_response(),
    }
}

pub async fn admin_list_audit_logs(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["audit:read", "users:manage", "settings:read"])
    {
        return resp;
    }
    let (logs, total_count) = state
        .db
        .get_audit_logs_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(logs), pagination.meta(total_count)).into_response()
}

#[derive(Deserialize)]
pub struct AuditLogPayload {
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub details_json: Option<String>,
}

pub async fn admin_insert_audit_log(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AuditLogPayload>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["audit:write", "settings:manage"]) {
        return resp;
    }
    if payload.action.trim().is_empty() || payload.entity_type.trim().is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    let client_ip = extract_client_ip(&headers);
    match state.db.insert_audit_log(
        &operator.email,
        payload.action.trim(),
        payload.entity_type.trim(),
        payload.entity_id,
        &payload.details_json.unwrap_or_else(|| "{}".to_string()),
        &client_ip,
    ) {
        Ok(id) => ApiResponse::ok(json!({ "status": "logged", "id": id })).into_response(),
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_insert_audit_log", &e)
                .into_response()
        }
    }
}


// ============================================================================

pub async fn admin_list_custom_fields(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:read", "contents:read"]) {
        return resp;
    }
    let applies_to = params
        .get("applies_to")
        .map(|s| s.as_str())
        .unwrap_or("content");
    let (fields, total_count) = state.db.get_custom_fields_paginated(
        applies_to,
        pagination.effective_per_page(),
        pagination.offset(),
    );
    ApiResponse::ok_with_meta(json!(fields), pagination.meta(total_count)).into_response()
}

#[derive(Deserialize)]
pub struct CustomFieldPayload {
    pub field_key: String,
    pub label_tr: String,
    pub label_en: String,
    pub field_type: String,
    pub options_json: Option<String>,
    pub is_required: Option<bool>,
    pub sort_order: Option<i64>,
    pub applies_to: Option<String>,
}

pub async fn admin_upsert_custom_field(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CustomFieldPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
    if payload.field_key.trim().is_empty() || payload.label_tr.trim().is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    match state.db.upsert_custom_field(
        &payload.field_key,
        &payload.label_tr,
        &payload.label_en,
        &payload.field_type,
        &payload.options_json.unwrap_or_else(|| "[]".to_string()),
        payload.is_required.unwrap_or(false),
        payload.sort_order.unwrap_or(0),
        &payload.applies_to.unwrap_or_else(|| "content".to_string()),
    ) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert",
                "custom_field",
                None,
                &json!({ "field_key": payload.field_key }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "saved", "field_key": payload.field_key }))
                .into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_upsert_custom_field", &e)
                .into_response()
        }
    }
}

pub async fn admin_delete_custom_field(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(field_key): Path<String>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
    match state.db.delete_custom_field(&field_key) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "custom_field",
                None,
                &json!({ "field_key": field_key }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "field_key": field_key })).into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_delete_custom_field", &e)
                .into_response()
        }
    }
}

pub async fn admin_get_custom_field_values(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let entity_type = params.get("entity_type").map(|s| s.trim()).unwrap_or("");
    let entity_id = params.get("entity_id").map(|s| s.trim()).unwrap_or("");
    if !matches!(entity_type, "content" | "ad" | "user") || entity_id.is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    let perm_check = match entity_type {
        "content" => require_any_perm(&operator, &["contents:read", "settings:read"]),
        "ad" => require_any_perm(&operator, &["ads:read", "settings:read"]),
        "user" => require_any_perm(&operator, &["users:read", "settings:read"]),
        _ => unreachable!(),
    };
    if let Err(resp) = perm_check {
        return resp;
    }

    let values = state.db.get_custom_field_values(entity_type, entity_id);
    ApiResponse::ok(json!({
        "entity_type": entity_type,
        "entity_id": entity_id,
        "values": values
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct CustomFieldValueItemPayload {
    pub field_id: i64,
    #[serde(default)]
    pub value_text: String,
}

#[derive(Deserialize)]
pub struct CustomFieldValuesPutPayload {
    pub entity_type: String,
    pub entity_id: String,
    #[serde(default)]
    pub values: Vec<CustomFieldValueItemPayload>,
}

pub async fn admin_put_custom_field_values(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CustomFieldValuesPutPayload>,
) -> Response {
    let entity_type = payload.entity_type.trim();
    let entity_id = payload.entity_id.trim();
    if !matches!(entity_type, "content" | "ad" | "user") || entity_id.is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    let perm_check = match entity_type {
        "content" => require_any_perm(&operator, &["contents:write", "settings:write"]),
        "ad" => require_any_perm(&operator, &["ads:write", "settings:write"]),
        "user" => require_any_perm(&operator, &["users:write", "settings:write"]),
        _ => unreachable!(),
    };
    if let Err(resp) = perm_check {
        return resp;
    }

    let items: Vec<(i64, String)> = payload
        .values
        .into_iter()
        .map(|v| (v.field_id, v.value_text))
        .collect();

    match state
        .db
        .upsert_custom_field_values(entity_type, entity_id, &items)
    {
        Ok(updated_count) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert",
                "custom_field_values",
                None,
                &json!({
                    "entity_type": entity_type,
                    "entity_id": entity_id,
                    "updated_count": updated_count,
                })
                .to_string(),
                &client_ip,
            );
            let updated_values = state.db.get_custom_field_values(entity_type, entity_id);
            ApiResponse::ok(json!({
                "status": "saved",
                "entity_type": entity_type,
                "entity_id": entity_id,
                "updated_count": updated_count,
                "values": updated_values
            }))
            .into_response()
        }
        Err(msg) => ApiResponse::raw(
            StatusCode::BAD_REQUEST,
            json!({
                "status": "error",
                "error_code": error_codes::BAD_REQUEST,
                "error": msg,
                "message": msg,
                "data": null
            }),
        )
        .into_response(),
    }
}

pub async fn admin_list_cache_operations(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    let (ops, total_count) = state
        .db
        .get_cache_operations_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(ops), pagination.meta(total_count)).into_response()
}

pub async fn admin_flush_cache(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
    let cache_key = payload
        .get("cache_key")
        .map(|s| s.as_str())
        .unwrap_or("all");

    state.db.refresh_l1_cache();
    state.translations.reload(state.db.load_all_translations());
    CentralContentQueryEngine::invalidate_cache();

    let _ = state.db.insert_cache_operation(
        cache_key,
        "flush",
        &operator.email,
        "L1 DB + Translation + ContentQuery cache flushed",
    );
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "flush",
        "cache",
        None,
        &json!({ "cache_key": cache_key }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({ "status": "flushed", "cache_key": cache_key })).into_response()
}

pub async fn admin_system_health(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["settings:read", "analytics:read", "system:read"]) {
        return resp;
    }
    Json(state.db.get_system_health()).into_response()
}

#[derive(Deserialize)]
pub struct BulkContentPayload {
    pub ids: Vec<i64>,
    pub action: String,
    pub value: Option<String>,
}

pub async fn admin_bulk_content_operation(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<BulkContentPayload>,
) -> Response {
    let action = payload.action.trim();
    let required = match action {
        "delete" => "contents:delete",
        "publish" => "contents:publish",
        "draft" | "archive" | "set_visibility" | "set_featured" => "contents:write",
        _ => return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR),
    };
    if let Err(resp) = require_perm(&operator, required) {
        return resp;
    }

    let all_contents = if action != "delete" {
        state.db.get_all_contents()
    } else {
        Vec::new()
    };

    if matches!(action, "draft" | "archive" | "set_visibility" | "set_featured") {
        let targets_privileged = payload.ids.iter().any(|id| {
            let db_status = state.db.get_content_status_by_id(*id);
            let status_opt = db_status.as_deref().or_else(|| {
                all_contents
                    .iter()
                    .find(|c| c.id == *id)
                    .map(|c| c.status.as_str())
            });
            status_opt.is_some_and(|st| {
                matches!(
                    st.trim().to_ascii_lowercase().as_str(),
                    "" | "published" | "publish" | "scheduled" | "future" | "archived" | "archive"
                )
            })
        });
        if targets_privileged {
            if let Err(resp) = require_perm(&operator, "contents:publish") {
                return resp;
            }
        }
    }

    let mut affected = 0usize;
    for id in &payload.ids {
        match action {
            "delete" => {
                if state.db.delete_content(*id).is_ok() {
                    affected += 1;
                }
            }
            "publish" => {
                if let Some(mut c) = all_contents.iter().find(|c| c.id == *id).cloned() {
                    c.status = "published".to_string();
                    if c.published_at.trim().is_empty() {
                        c.published_at = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
                    }
                    if state.db.upsert_content(&c).is_ok() {
                        affected += 1;
                    }
                }
            }
            "draft" => {
                if let Some(mut c) = all_contents.iter().find(|c| c.id == *id).cloned() {
                    c.status = "draft".to_string();
                    if state.db.upsert_content(&c).is_ok() {
                        affected += 1;
                    }
                }
            }
            "archive" => {
                if let Some(mut c) = all_contents.iter().find(|c| c.id == *id).cloned() {
                    c.status = "archived".to_string();
                    if state.db.upsert_content(&c).is_ok() {
                        affected += 1;
                    }
                }
            }
            "set_visibility" => {
                if let Some(level) = payload.value.as_deref().and_then(|v| v.parse::<i32>().ok()) {
                    if let Some(mut c) = all_contents.iter().find(|c| c.id == *id).cloned() {
                        c.visibility_level = level;
                        if state.db.upsert_content(&c).is_ok() {
                            affected += 1;
                        }
                    }
                }
            }
            "set_featured" => {
                if let Some(mut c) = all_contents.iter().find(|c| c.id == *id).cloned() {
                    c.is_featured = payload.value.as_deref() == Some("true");
                    if state.db.upsert_content(&c).is_ok() {
                        affected += 1;
                    }
                }
            }
            _ => {}
        }
    }
    CentralContentQueryEngine::invalidate_cache();
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "bulk_operation",
        "content",
        None,
        &json!({ "action": action, "affected": affected }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({
        "status": "completed",
        "action": action,
        "affected": affected
    }))
    .into_response()
}

// ============================================================================
// EDITORIAL APPROVAL WORKFLOW (006_approval_workflow.sql)
