use super::*;

// ============================================================================

pub async fn submit_content_for_review(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(content_id): Path<i64>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "contents:write") {
        return resp;
    }
    match state.db.create_approval_request(content_id, operator.user_id) {
        Ok(approval_id) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "submit_for_review",
                "content",
                Some(content_id),
                &json!({ "approval_id": approval_id }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "approval_id": approval_id,
                "content_id": content_id,
                "status": "pending_review"
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error_with_key(
            &state.translations,
            "tr",
            "submit_content_for_review",
            &e,
            msg_keys::SERVER_ERROR_SAVE,
        )
        .into_response(),
    }
}

pub async fn list_pending_approvals(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["approvals:read", "approvals:manage", "contents:publish"])
    {
        return resp;
    }
    let approvals = state.db.list_pending_approvals();
    ApiResponse::ok(json!(approvals)).into_response()
}

pub async fn approve_content(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(approval_id): Path<i64>,
    Json(payload): Json<Value>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["approvals:approve", "approvals:manage", "contents:publish"])
    {
        return resp;
    }
    let note = payload.get("note").and_then(|v| v.as_str());
    match state
        .db
        .update_approval_status(approval_id, "approved", operator.user_id, note)
    {
        Ok(()) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "approve_content",
                "approval",
                Some(approval_id),
                &json!({ "note": note.unwrap_or("") }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "approval_id": approval_id,
                "status": "approved"
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error_with_key(
            &state.translations,
            "tr",
            "approve_content",
            &e,
            msg_keys::SERVER_ERROR_SAVE,
        )
        .into_response(),
    }
}

pub async fn reject_content(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(approval_id): Path<i64>,
    Json(payload): Json<Value>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["approvals:reject", "approvals:manage", "contents:publish"])
    {
        return resp;
    }
    let note = payload.get("note").and_then(|v| v.as_str());
    match state
        .db
        .update_approval_status(approval_id, "rejected", operator.user_id, note)
    {
        Ok(()) => {
            CentralContentQueryEngine::invalidate_cache();
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "reject_content",
                "approval",
                Some(approval_id),
                &json!({ "note": note.unwrap_or("") }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "approval_id": approval_id,
                "status": "rejected"
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error_with_key(
            &state.translations,
            "tr",
            "reject_content",
            &e,
            msg_keys::SERVER_ERROR_SAVE,
        )
        .into_response(),
    }
}

// ============================================================================
// SERVER MONITORING & SECURITY TELEMETRY (007_monitoring_schema.sql)
