use super::*;

pub async fn faceted_search_api(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let criteria = CentralContentQueryEngine::from_params(&params);
    let result = CentralContentQueryEngine::execute(&state.db, &criteria);
    Json(json!(result))
}

pub async fn list_notifications_api(
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let session_u = auth_service::session_user(&state, &headers);
    let is_admin_route = uri.path().starts_with("/api/v1/admin/")
        && session_u
            .as_ref()
            .map(|u| is_staff_role(&state.db, &u.role))
            .unwrap_or(false);

    let (email_opt, org_opt) = if is_admin_route {
        (
            params.get("email").map(|s| s.as_str()),
            params.get("org").map(|s| s.as_str()),
        )
    } else if let Some(ref u) = session_u {
        (
            Some(u.email.as_str()),
            Some(u.organization_category.as_str()),
        )
    } else {
        (None, None)
    };

    let all = state.db.get_system_notifications(email_opt);
    let filtered: Vec<_> = if email_opt.is_some() || is_admin_route {
        crate::engine::notification_engine::filter_user_notifications(&all, email_opt, org_opt)
    } else {
        all.iter()
            .filter(|n| {
                n.target_email.as_deref().unwrap_or("").is_empty()
                    && matches!(
                        n.target_audience.to_ascii_lowercase().as_str(),
                        "all" | "public" | "global" | ""
                    )
            })
            .cloned()
            .collect()
    };
    let banners = crate::engine::notification_engine::filter_active_banners(&filtered);
    Json(json!({
        "notifications": filtered,
        "active_banners": banners,
        "total": filtered.len()
    }))
}

pub async fn mark_notification_read_api(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Response {
    let Some(user) = auth_service::session_user(&state, &headers) else {
        return json_error(&state, StatusCode::UNAUTHORIZED, msg_keys::AUTH_REQUIRED);
    };

    let Some(notif) = state.db.get_notification_by_id(id) else {
        return json_error(&state, StatusCode::NOT_FOUND, msg_keys::BAD_REQUEST);
    };

    if let Some(target_email) = notif.target_email.as_deref().filter(|e| !e.trim().is_empty()) {
        if !target_email.eq_ignore_ascii_case(&user.email)
            && !is_staff_role(&state.db, &user.role)
        {
            return json_error(&state, StatusCode::FORBIDDEN, msg_keys::AUTH_REQUIRED);
        }
    }

    let ok = state.db.mark_notification_read(id).is_ok();
    ApiResponse::ok(json!({
        "status": if ok { "marked_read" } else { "error" },
        "marked_read": ok,
        "id": id
    }))
    .into_response()
}

pub async fn list_site_settings_api(State(state): State<AppState>) -> Json<Value> {
    let settings: Vec<SiteSetting> = state
        .db
        .get_site_settings()
        .into_iter()
        .filter(|s| {
            !s.key.starts_with("user_email_prefs.") && s.setting_group != "user_preferences"
        })
        .collect();

    let mut map_tr = state.db.get_setting_map("tr");
    map_tr.retain(|k, _| !k.starts_with("user_email_prefs."));
    let mut map_en = state.db.get_setting_map("en");
    map_en.retain(|k, _| !k.starts_with("user_email_prefs."));

    Json(json!({
        "settings": settings,
        "map_tr": map_tr,
        "map_en": map_en
    }))
}

// ============================================================================
// ADMIN OMS / CMS ENDPOINTS (/api/v1/admin/*)
