use super::*;

pub async fn faceted_search_api(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let lang = params.get("lang").map(|s| s.as_str()).unwrap_or("tr");
    // Validate and sanitize pagination boundaries
    let (page, per_page) = crate::engine::validation_engine::validate_pagination(
        params.get("page").map(|s| s.as_str()),
        params.get("per_page").map(|s| s.as_str()),
    );

    // Validate and sanitize search query if present
    let sanitized_q = params
        .get("q")
        .and_then(|s| crate::engine::validation_engine::validate_search_query(s));

    // Build a validated params map for the query engine
    let mut validated_params = params.clone();
    validated_params.insert("page".to_string(), page.to_string());
    validated_params.insert("per_page".to_string(), per_page.to_string());
    if let Some(ref q) = sanitized_q {
        validated_params.insert("q".to_string(), q.clone());
    } else {
        validated_params.remove("q");
    }

    let criteria = CentralContentQueryEngine::from_params(&validated_params);
    let mut result = CentralContentQueryEngine::execute(&state.db, &criteria);

    let user_opt = auth_service::session_user(&state, &headers);
    let purchased = user_opt
        .as_ref()
        .map(|u| state.db.get_user_purchased_codes(&u.email))
        .unwrap_or_default();

    for item in &mut result.items {
        let inst_allowed =
            content_service::user_matches_target_institutions(item, user_opt.as_ref(), &state.db);
        let (_, is_locked, _) =
            content_service::apply_paywall_view(item, user_opt.as_ref(), &purchased, lang);
        if is_locked || !inst_allowed {
            item.body_tr = content_service::truncate_to_two_sentences_html(&item.body_tr);
            item.body_en = content_service::truncate_to_two_sentences_html(&item.body_en);
        }
    }

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
