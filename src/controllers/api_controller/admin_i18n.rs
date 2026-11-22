use super::*;
use crate::models::category::{EmailGroup, LocaleConfig, TranslationGlossaryTerm};

// ============================================================================
// GLOSSARY CRUD (glossary:write / glossary:manage)
// ============================================================================

pub async fn admin_list_glossary(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["glossary:write", "glossary:manage"]) {
        return resp;
    }
    let domain_filter = params.get("domain").map(|s| s.as_str());
    let active_only = params
        .get("active_only")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let terms = state.db.list_glossary_terms(domain_filter, active_only);
    ApiResponse::ok(json!(terms)).into_response()
}

pub async fn admin_upsert_glossary(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(term): Json<TranslationGlossaryTerm>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["glossary:write", "glossary:manage"]) {
        return resp;
    }
    let is_update = term.id > 0;
    match state.db.upsert_glossary_term(&term) {
        Ok(id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                if is_update { "update" } else { "create" },
                "glossary",
                Some(id),
                &json!({ "term_tr": term.term_tr, "domain": term.domain }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "saved", "id": id })).into_response()
        }
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

pub async fn admin_delete_glossary(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "glossary:manage") {
        return resp;
    }
    match state.db.delete_glossary_term(id) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "glossary",
                Some(id),
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "id": id })).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_delete_glossary", &e)
            .into_response(),
    }
}

// ============================================================================
// LOCALES CRUD (locales:manage)
// ============================================================================

pub async fn admin_list_locales(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "locales:manage") {
        return resp;
    }
    let locales = state.db.list_locales();
    ApiResponse::ok(json!(locales)).into_response()
}

pub async fn admin_upsert_locale(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(locale): Json<LocaleConfig>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "locales:manage") {
        return resp;
    }
    match state.db.upsert_locale(&locale) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert",
                "locale",
                None,
                &json!({ "code": locale.code, "is_default": locale.is_default }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "saved", "code": locale.code })).into_response()
        }
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

pub async fn admin_update_locale(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(mut locale): Json<LocaleConfig>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "locales:manage") {
        return resp;
    }
    // Ensure the path code matches the body code
    locale.code = code.clone();
    match state.db.upsert_locale(&locale) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "update",
                "locale",
                None,
                &json!({ "code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "updated", "code": code })).into_response()
        }
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

pub async fn admin_delete_locale(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "locales:manage") {
        return resp;
    }
    match state.db.delete_locale(&code) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "locale",
                None,
                &json!({ "code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "code": code })).into_response()
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

// ============================================================================
// EMAIL GROUPS CRUD (email_groups:write / email_groups:manage)
// ============================================================================

pub async fn admin_list_email_groups(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["email_groups:write", "email_groups:manage"]) {
        return resp;
    }
    let groups = state.db.list_email_groups(false);
    ApiResponse::ok(json!(groups)).into_response()
}

pub async fn admin_upsert_email_group(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(grp): Json<EmailGroup>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["email_groups:write", "email_groups:manage"]) {
        return resp;
    }
    let is_update = grp.id > 0;
    match state.db.upsert_email_group(&grp) {
        Ok(id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                if is_update { "update" } else { "create" },
                "email_group",
                Some(id),
                &json!({ "code": grp.code, "name_tr": grp.name_tr }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "saved", "id": id, "code": grp.code }))
                .into_response()
        }
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

pub async fn admin_delete_email_group(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "email_groups:manage") {
        return resp;
    }
    match state.db.delete_email_group(&code) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "email_group",
                None,
                &json!({ "code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "code": code })).into_response()
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

// ============================================================================
// BACKUP (backup:manage)
// ============================================================================

pub async fn admin_trigger_backup(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "backup:manage") {
        return resp;
    }

    let mut cfg = (*state.config).clone();
    if cfg.backup_dir.trim().is_empty() {
        cfg.backup_dir = format!(
            "{}/backups",
            std::path::Path::new(&state.config.sqlite_db_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "storage/db".to_string())
        );
    }

    let result = match crate::workers::backup_worker::perform_full_backup_with_offsite(
        &state.db,
        &cfg,
        "msi_backup",
        &operator.email,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return ApiResponse::server_error_with_key(
                &state.translations,
                "tr",
                "admin_backup_online",
                &e,
                msg_keys::SERVER_ERROR_GENERIC,
            )
            .into_response();
        }
    };

    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "backup",
        "system",
        None,
        &json!({
            "filename": result.filename,
            "size_bytes": result.size_bytes,
            "sha256": result.sha256,
            "integrity_check": result.integrity_check,
            "encrypted_archive_path": result.encrypted_archive_path,
            "s3_uploaded": result.s3_uploaded,
        })
        .to_string(),
        &client_ip,
    );

    ApiResponse::ok(json!({
        "status": "completed",
        "filename": result.filename,
        "path": result.path,
        "size_bytes": result.size_bytes,
        "sha256": result.sha256,
        "integrity_check": result.integrity_check,
        "pruned_count": result.pruned_count,
        "encrypted_archive_path": result.encrypted_archive_path,
        "encrypted_sha256": result.encrypted_sha256,
        "s3_uploaded": result.s3_uploaded,
        "s3_key": result.s3_key
    }))
    .into_response()
}

// ============================================================================
// AUTO-TRANSLATE & APPROVAL WORKFLOW (translations:read / translations:write)
// ============================================================================

#[derive(Deserialize)]
pub struct AutoTranslatePayload {
    pub key: Option<String>,
    pub text: String,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
}

/// POST /api/v1/admin/translations/auto-translate
/// Translates text via configured provider and returns glossary violations.
pub async fn admin_auto_translate(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AutoTranslatePayload>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["translations:write", "translations:read"]) {
        return resp;
    }

    let source = payload.source_lang.as_deref().unwrap_or("tr");
    let target = payload.target_lang.as_deref().unwrap_or("en");

    match crate::services::translation_service::TranslationService::translate_with_glossary(
        &state.db,
        &payload.text,
        source,
        target,
    )
    .await
    {
        Ok((translated, violations)) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "auto_translate",
                "translation",
                None,
                &json!({
                    "key": payload.key,
                    "source_lang": source,
                    "target_lang": target,
                    "violations_count": violations.len()
                })
                .to_string(),
                &client_ip,
            );

            let violation_json: Vec<Value> = violations
                .iter()
                .map(|v| {
                    json!({
                        "glossary_id": v.glossary_id,
                        "domain": v.domain,
                        "term_tr": v.term_tr,
                        "forbidden": v.forbidden,
                        "preferred_en": v.preferred_en,
                        "notes": v.notes,
                    })
                })
                .collect();

            ApiResponse::ok(json!({
                "translated_text": translated,
                "source_lang": source,
                "target_lang": target,
                "glossary_violations": violation_json,
            }))
            .into_response()
        }
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

/// POST /api/v1/admin/translations/:key/approve
/// Approves a translation entry (sets status='approved').
pub async fn admin_approve_translation(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "translations:write") {
        return resp;
    }

    match state.db.approve_translation(&key, operator.user_id) {
        Ok(true) => {
            state.translations.reload(state.db.load_all_translations());
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "approve",
                "translation",
                None,
                &json!({ "key": key }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "approved", "key": key })).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => AppError::internal("translation_approve", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response(),
    }
}

/// POST /api/v1/admin/translations/:key/reject
/// Rejects a translation back to draft status.
pub async fn admin_reject_translation(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "translations:write") {
        return resp;
    }

    match state.db.reject_translation(&key) {
        Ok(true) => {
            state.translations.reload(state.db.load_all_translations());
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "reject",
                "translation",
                None,
                &json!({ "key": key }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "rejected", "key": key })).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => AppError::internal("translation_reject", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response(),
    }
}

/// GET /api/v1/admin/translations/pending
/// Lists translations filtered by status (defaulting to pending: `draft` + `auto_translated`).
pub async fn admin_list_pending_translations(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "translations:read") {
        return resp;
    }

    let page: i64 = params
        .get("page")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .max(1);
    let per_page: i64 = params
        .get("per_page")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
        .clamp(1, 200);

    let raw_status = params.get("status").map(|s| s.trim()).unwrap_or("");
    let effective_status = if raw_status.is_empty() {
        "pending"
    } else {
        raw_status
    };

    let (items, total) = state
        .db
        .list_translations_by_status(effective_status, page, per_page);

    let total_pages = if total == 0 {
        0
    } else {
        ((total as f64) / (per_page as f64)).ceil() as u64
    };

    ApiResponse::ok_with_meta(
        json!(items),
        json!({
            "page": page,
            "per_page": per_page,
            "total_count": total,
            "total_pages": total_pages,
        }),
    )
    .into_response()
}