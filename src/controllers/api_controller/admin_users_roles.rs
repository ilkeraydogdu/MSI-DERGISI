use super::*;

pub async fn admin_me(State(state): State<AppState>, operator: OperatorUser) -> Response {
    ApiResponse::ok(json!({
        "user_id": operator.user_id,
        "email": operator.email,
        "full_name": operator.full_name,
        "role": operator.role,
        "permissions": operator.permissions,
        "is_wildcard": operator.has_permission("*"),
        "monitoring_enabled": state.config.monitoring_enabled && state.monitoring_cache.is_some()
    }))
    .into_response()
}

pub(super) fn is_privileged_role(state: &AppState, role_code: &str) -> bool {
    let norm = role_code.trim().to_ascii_lowercase();
    if matches!(norm.as_str(), "super_admin" | "admin" | "site_operator") {
        return true;
    }
    state
        .db
        .get_role_permissions(&norm)
        .or_else(|| state.db.get_role_permissions(role_code))
        .map(|perms| perms.iter().any(|p| p.trim() == "*" || p.trim().ends_with(":*")))
        .unwrap_or(false)
}

pub(super) fn perms_contain_wildcard(perms_json: &str) -> bool {
    serde_json::from_str::<Vec<String>>(perms_json)
        .map(|perms| {
            perms.iter().any(|p| {
                let pt = p.trim();
                pt == "*" || pt.ends_with(":*")
            })
        })
        .unwrap_or(false)
}

pub(super) fn has_disallowed_perms(operator: &OperatorUser, perms_json: &str) -> bool {
    if operator.has_permission("*") {
        return false;
    }
    let Ok(perms) = serde_json::from_str::<Vec<String>>(perms_json) else {
        return false;
    };
    perms.iter().any(|p| {
        let pt = p.trim();
        pt == "*" || pt.ends_with(":*") || !operator.has_permission(pt)
    })
}

#[derive(Deserialize)]
pub struct AdminCreateUserPayload {
    pub email: String,
    pub password: String,
    pub full_name: String,
    #[serde(default = "default_reader_role")]
    pub role: String,
    #[serde(default)]
    pub organization_category: String,
    #[serde(default)]
    pub organization_name: String,
    #[serde(default)]
    pub job_title: String,
    #[serde(default)]
    pub is_gov_verified: bool,
    #[serde(default)]
    pub subscription_tier: String,
}

fn default_reader_role() -> String {
    "registered".to_string()
}

#[derive(Deserialize)]
pub struct AdminUpdateUserPayload {
    pub full_name: String,
    pub role: Option<String>,
    #[serde(default)]
    pub organization_category: String,
    #[serde(default)]
    pub organization_name: String,
    #[serde(default)]
    pub job_title: String,
    #[serde(default)]
    pub is_gov_verified: bool,
    #[serde(default)]
    pub subscription_tier: String,
    pub new_password: Option<String>,
}

pub async fn admin_list_users(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["users:read", "users:manage"]) {
        return resp;
    }
    let all_users: Vec<UserAccount> = state
        .db
        .get_all_users()
        .into_iter()
        .filter(|u| u.role != "deleted")
        .collect();
    let total_count = all_users.len() as u64;
    let offset = pagination.offset() as usize;
    let per_page = pagination.effective_per_page() as usize;
    let page_items: Vec<UserAccount> = all_users.into_iter().skip(offset).take(per_page).collect();
    ApiResponse::ok_with_meta(json!(page_items), pagination.meta(total_count)).into_response()
}

pub async fn admin_create_user(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AdminCreateUserPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "users:manage") {
        return resp;
    }
    let email = payload.email.trim().to_lowercase();
    let role = payload.role.trim();
    if !email.contains('@')
        || !email.contains('.')
        || payload.full_name.trim().is_empty()
        || payload.password.trim().len() < 8
        || role.is_empty()
    {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    if !operator.has_permission("*") && is_privileged_role(&state, role) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Süper yönetici / wildcard yetkili roller yalnızca süper yöneticiler tarafından oluşturulabilir.",
                "data": null
            })),
        )
            .into_response();
    }
    let password_hash = auth_service::hash_password(payload.password.trim(), &state.config.session_secret);
    match state.db.admin_create_user(
        &email,
        &password_hash,
        &payload.full_name,
        role,
        &payload.organization_category,
        &payload.organization_name,
        &payload.job_title,
        payload.is_gov_verified,
        &payload.subscription_tier,
    ) {
        Ok(id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "create_user",
                "user",
                Some(id),
                &json!({ "email": email, "role": role }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "created", "id": id, "email": email }))
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

pub async fn admin_update_user(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    Json(payload): Json<AdminUpdateUserPayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "users:manage") {
        return resp;
    }
    // Self-demotion protection: prevent admins from modifying their own account
    if user_id == operator.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Kendi hesabınızı güncelleyemezsiniz. Başka bir yönetici tarafından düzenlenmelidir.",
                "data": null
            })),
        )
            .into_response();
    }
    if payload.full_name.trim().is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    let Some(target) = state.db.get_all_users().into_iter().find(|u| u.id == user_id) else {
        return not_found_error();
    };
    if !operator.has_permission("*") {
        if is_privileged_role(&state, &target.role)
            || payload
                .role
                .as_deref()
                .map(|r| is_privileged_role(&state, r))
                .unwrap_or(false)
        {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "error",
                    "error_code": "FORBIDDEN",
                    "message": "Süper yönetici hesapları veya wildcard roller yalnızca süper yöneticiler tarafından güncellenebilir.",
                    "data": null
                })),
            )
                .into_response();
        }
    }
    if let Some(ref npw) = payload.new_password {
        if !npw.trim().is_empty() && npw.trim().len() < 8 {
            return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
        }
    }
    let new_hash = payload
        .new_password
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| auth_service::hash_password(s, &state.config.session_secret));
    let requested_role = payload
        .role
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match state.db.admin_update_user(
        user_id,
        &payload.full_name,
        requested_role,
        &payload.organization_category,
        &payload.organization_name,
        &payload.job_title,
        payload.is_gov_verified,
        &payload.subscription_tier,
        new_hash.as_deref(),
    ) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "update_user",
                "user",
                Some(user_id),
                &json!({ "role": requested_role }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "updated", "id": user_id })).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_update_user", &e)
            .into_response(),
    }
}

pub async fn admin_delete_user(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "users:manage") {
        return resp;
    }
    // Self-demotion protection: prevent admins from deleting their own account
    if user_id == operator.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Kendi hesabınızı silemezsiniz. Başka bir yönetici tarafından silinmelidir.",
                "data": null
            })),
        )
            .into_response();
    }
    let Some(target) = state.db.get_all_users().into_iter().find(|u| u.id == user_id) else {
        return not_found_error();
    };
    if !operator.has_permission("*") && is_privileged_role(&state, &target.role) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Süper yönetici hesapları yalnızca süper yöneticiler tarafından silinebilir.",
                "data": null
            })),
        )
            .into_response();
    }
    match state.db.admin_delete_user(user_id, operator.user_id) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete_user",
                "user",
                Some(user_id),
                &json!({ "email": target.email }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "id": user_id })).into_response()
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

pub async fn admin_list_user_roles(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["users:read", "users:manage", "roles:manage"])
    {
        return resp;
    }
    let (roles, total_count) = state
        .db
        .get_user_roles_paginated(pagination.effective_per_page(), pagination.offset());
    ApiResponse::ok_with_meta(json!(roles), pagination.meta(total_count)).into_response()
}

#[derive(Deserialize)]
pub struct UserRolePayload {
    pub role_code: String,
    #[serde(default)]
    pub name_tr: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub permissions_json: String,
}

pub(super) fn is_builtin_system_role(state: &AppState, role_code: &str) -> bool {
    state
        .db
        .get_user_roles()
        .iter()
        .any(|r| {
            r.get("role_code")
                .and_then(|v| v.as_str())
                .map(|c| c.eq_ignore_ascii_case(role_code.trim()))
                .unwrap_or(false)
                && r.get("is_system")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        })
}

pub async fn admin_upsert_user_role(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<UserRolePayload>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "roles:manage") {
        return resp;
    }
    let norm_code = payload.role_code.trim().to_ascii_lowercase();
    let code = norm_code.as_str();
    if code.is_empty() || code.len() > 64 {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    if is_builtin_system_role(&state, code)
        || (!operator.has_permission("*") && is_privileged_role(&state, code))
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Sistem rollerinin izinleri değiştirilemez.",
                "data": null
            })),
        )
            .into_response();
    }
    if !payload.permissions_json.trim().is_empty()
        && serde_json::from_str::<Vec<String>>(&payload.permissions_json).is_err()
    {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    if !operator.has_permission("*")
        && (perms_contain_wildcard(&payload.permissions_json)
            || has_disallowed_perms(&operator, &payload.permissions_json))
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Wildcard (*) veya sahip olunmayan yetkiler yalnızca süper yöneticiler tarafından verilebilir.",
                "data": null
            })),
        )
            .into_response();
    }

    match state.db.upsert_user_role(
        code,
        &payload.name_tr,
        &payload.name_en,
        &payload.permissions_json,
    ) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert_role",
                "user_role",
                None,
                &json!({ "role_code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "saved", "role_code": code })).into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_upsert_user_role", &e)
                .into_response()
        }
    }
}

pub async fn admin_update_user_role(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(role_code): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "roles:manage") {
        return resp;
    }
    let norm_code = role_code.trim().to_ascii_lowercase();
    let code = norm_code.as_str();
    if code.is_empty() || code.len() > 64 {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    if is_builtin_system_role(&state, code)
        || (!operator.has_permission("*") && is_privileged_role(&state, code))
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Sistem veya süper yönetici rollerinin ayarları değiştirilemez.",
                "data": null
            })),
        )
            .into_response();
    }

    let name_tr = body.get("name_tr").and_then(|v| v.as_str()).unwrap_or("");
    let name_en = body.get("name_en").and_then(|v| v.as_str()).unwrap_or("");
    let perms_raw = body
        .get("permissions_json")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !perms_raw.is_empty() && serde_json::from_str::<Vec<String>>(perms_raw).is_err() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    if !operator.has_permission("*")
        && (perms_contain_wildcard(perms_raw) || has_disallowed_perms(&operator, perms_raw))
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Wildcard (*) veya sahip olunmayan yetkiler yalnızca süper yöneticiler tarafından verilebilir.",
                "data": null
            })),
        )
            .into_response();
    }

    match state.db.upsert_user_role(code, name_tr, name_en, perms_raw) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "update_role",
                "user_role",
                None,
                &json!({ "role_code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "updated", "role_code": code })).into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_update_user_role", &e)
                .into_response()
        }
    }
}

pub async fn admin_delete_user_role(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(role_code): Path<String>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "roles:manage") {
        return resp;
    }
    let norm_code = role_code.trim().to_ascii_lowercase();
    let code = norm_code.as_str();
    if is_builtin_system_role(&state, code)
        || (!operator.has_permission("*") && is_privileged_role(&state, code))
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Sistem rolleri silinemez.",
                "data": null
            })),
        )
            .into_response();
    }

    match state.db.delete_user_role(code) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete_role",
                "user_role",
                None,
                &json!({ "role_code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "role_code": code })).into_response()
        }
        Ok(false) => json_error(&state, StatusCode::NOT_FOUND, msg_keys::BAD_REQUEST),
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_delete_user_role", &e)
                .into_response()
        }
    }
}

pub async fn admin_assign_role_to_user(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    Json(body): Json<Value>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "roles:manage") {
        return resp;
    }
    // Self-demotion protection: prevent admins from reassigning their own role
    if user_id == operator.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Kendi rolünüzü değiştiremezsiniz. Başka bir yönetici tarafından atanmalıdır.",
                "data": null
            })),
        )
            .into_response();
    }
    let role_code = body
        .get("role_code")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if role_code.is_empty() || role_code.len() > 64 {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    let Some(target) = state.db.get_all_users().into_iter().find(|u| u.id == user_id) else {
        return not_found_error();
    };
    if !operator.has_permission("*")
        && (is_privileged_role(&state, role_code) || is_privileged_role(&state, &target.role))
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": "Süper yönetici veya sistem yöneticisi rolleri yalnızca süper yöneticiler tarafından atanabilir.",
                "data": null
            })),
        )
            .into_response();
    }

    match state.db.update_user_role(user_id, role_code) {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "assign_role",
                "user",
                Some(user_id),
                &json!({ "role_code": role_code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "assigned",
                "user_id": user_id,
                "role_code": role_code
            }))
            .into_response()
        }
        Err(e) => {
            ApiResponse::server_error(&state.translations, "tr", "admin_assign_role_to_user", &e)
                .into_response()
        }
    }
}

// ============================================================================
// CUSTOM FIELDS, CACHE FLUSH, SYSTEM HEALTH & BULK CONTENT OPERATIONS
