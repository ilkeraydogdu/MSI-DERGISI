use super::*;

use crate::models::{CommerceProduct, SubscriptionPlan};

#[derive(Deserialize)]
pub struct AdminSubscriptionPlanPayload {
    #[serde(flatten)]
    pub plan: SubscriptionPlan,
    pub sort_order: Option<i32>,
}

pub async fn admin_list_subscription_plans(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(
        &operator,
        &["subscriptions:read", "commerce:read", "analytics:read"],
    ) {
        return resp;
    }
    ApiResponse::ok(json!(state.db.get_subscription_plans())).into_response()
}

pub async fn admin_upsert_subscription_plan(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AdminSubscriptionPlanPayload>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["subscriptions:write", "commerce:manage"]) {
        return resp;
    }
    if payload.plan.code.trim().is_empty()
        || payload.plan.name_tr.trim().is_empty()
        || payload.plan.price_try < 0.0
    {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    match state
        .db
        .upsert_subscription_plan(&payload.plan, payload.sort_order.unwrap_or(10))
    {
        Ok(()) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert",
                "subscription_plan",
                None,
                &json!({ "code": payload.plan.code, "price_try": payload.plan.price_try })
                    .to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "saved",
                "code": payload.plan.code
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

pub async fn admin_delete_subscription_plan(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["subscriptions:write", "commerce:manage"]) {
        return resp;
    }
    match state.db.delete_subscription_plan(&code) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "subscription_plan",
                None,
                &json!({ "code": code }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({ "status": "deleted", "code": code })).into_response()
        }
        Ok(false) => not_found_error(),
        Err(e) => ApiResponse::server_error(
            &state.translations,
            "tr",
            "admin_delete_subscription_plan",
            &e,
        )
        .into_response(),
    }
}

pub async fn admin_list_commerce_products(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(
        &operator,
        &["commerce:read", "subscriptions:read", "analytics:read"],
    ) {
        return resp;
    }
    ApiResponse::ok(json!(state.db.get_commerce_products())).into_response()
}

pub async fn admin_upsert_commerce_product(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(prod): Json<CommerceProduct>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["commerce:manage", "commerce:write"]) {
        return resp;
    }
    if prod.sku.trim().is_empty() || prod.title_tr.trim().is_empty() || prod.price_try < 0.0 {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    match state.db.upsert_commerce_product(&prod) {
        Ok(id) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert",
                "commerce_product",
                Some(id),
                &json!({ "sku": prod.sku, "price_try": prod.price_try }).to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "saved",
                "id": id,
                "sku": prod.sku
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

pub async fn admin_delete_commerce_product(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["commerce:manage", "commerce:write"]) {
        return resp;
    }
    match state.db.delete_commerce_product(id) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "commerce_product",
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
            "admin_delete_commerce_product",
            &e,
        )
        .into_response(),
    }
}

pub async fn admin_list_invoices(
    operator: OperatorUser,
    State(state): State<AppState>,
    pagination: PaginationParams,
) -> Response {
    if let Err(resp) = require_any_perm(
        &operator,
        &["invoices:read", "commerce:read", "analytics:read"],
    ) {
        return resp;
    }
    let all = state.db.get_order_invoices();
    let total_count = all.len() as u64;
    let offset = pagination.offset() as usize;
    let per_page = pagination.effective_per_page() as usize;
    let page_items: Vec<_> = all.into_iter().skip(offset).take(per_page).collect();
    ApiResponse::ok_with_meta(json!(page_items), pagination.meta(total_count)).into_response()
}

pub async fn admin_mark_invoice_paid(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["invoices:write", "commerce:manage"]) {
        return resp;
    }
    let Some((inv, newly_paid)) = state.db.mark_invoice_paid_by_id_strict(id) else {
        return not_found_error();
    };
    if newly_paid {
        state.db.activate_invoice_entitlements(&inv);
    }
    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "mark_paid",
        "invoice",
        Some(inv.id),
        &json!({ "invoice_no": inv.invoice_no, "item_code": inv.item_code, "newly_paid": newly_paid }).to_string(),
        &client_ip,
    );
    ApiResponse::ok(json!({
        "status": "paid",
        "newly_paid": newly_paid,
        "invoice": inv
    }))
    .into_response()
}
