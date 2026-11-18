use super::*;

// ============================================================================

pub async fn admin_analytics_bundle(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let contents = state.db.get_all_contents();
    let ads = state.db.get_all_ads();
    let users = state.db.get_all_users();
    let events = state.db.get_all_analytics_events();

    let content_reports: Vec<_> = contents
        .iter()
        .map(|c| analytics_service::compute_content_report(c, &events))
        .collect();
    let ad_reports: Vec<_> = ads
        .iter()
        .map(|a| analytics_service::compute_ad_report(a, &events))
        .collect();
    let reader_reports = analytics_service::compute_reader_reports(&users, &events, &ads);

    Json(json!({
        "contents": content_reports,
        "ads": ad_reports,
        "readers": reader_reports
    }))
    .into_response()
}

pub async fn admin_analytics_contents(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let reports: Vec<_> = contents
        .iter()
        .map(|c| analytics_service::compute_content_report(c, &events))
        .collect();
    Json(json!(reports)).into_response()
}

pub async fn admin_analytics_ads(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "ads:read"]) {
        return resp;
    }
    let ads = state.db.get_all_ads();
    let events = state.db.get_all_analytics_events();
    let reports: Vec<_> = ads
        .iter()
        .map(|a| analytics_service::compute_ad_report(a, &events))
        .collect();
    Json(json!(reports)).into_response()
}

pub async fn admin_analytics_readers(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "users:read"]) {
        return resp;
    }
    let users = state.db.get_all_users();
    let ads = state.db.get_all_ads();
    let events = state.db.get_all_analytics_events();
    let reports = analytics_service::compute_reader_reports(&users, &events, &ads);
    Json(json!(reports)).into_response()
}

pub async fn admin_daily_analytics(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let (from, to) = default_date_bounds(&params);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if to >= today {
        let _ = state.db.aggregate_analytics_for_date(&today);
    }
    let rows = state.db.get_daily_aggregated_analytics(&from, &to);
    ApiResponse::ok(json!(rows)).into_response()
}

pub async fn admin_author_performance(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let authors = analytics_service::compute_author_performance(&contents, &events);
    ApiResponse::ok(json!(authors)).into_response()
}

pub async fn admin_category_performance(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let categories = analytics_service::compute_category_performance(&contents, &events);
    ApiResponse::ok(json!(categories)).into_response()
}

pub async fn admin_trending_content(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let trending = analytics_service::compute_trending_content(&contents, &events, limit);
    ApiResponse::ok(json!(trending)).into_response()
}

pub async fn admin_revenue_summary(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "revenue:read") {
        return resp;
    }
    let (from, to) = default_date_bounds(&params);
    match revenue_analytics_service::get_revenue_summary(&state.db, &from, &to) {
        Ok(summary) => ApiResponse::ok(summary).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "revenue_summary", &e)
            .into_response(),
    }
}

pub async fn admin_keyword_performance(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "ads:read", "revenue:read"])
    {
        return resp;
    }
    let (from, to) = default_date_bounds(&params);
    match revenue_analytics_service::get_keyword_performance(&state.db, &from, &to) {
        Ok(rows) => ApiResponse::ok(json!(rows)).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "keyword_performance", &e)
            .into_response(),
    }
}

pub async fn admin_advertiser_report(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "ads:read", "revenue:read"])
    {
        return resp;
    }
    let (from, to) = default_date_bounds(&params);
    match revenue_analytics_service::get_advertiser_report(&state.db, &from, &to) {
        Ok(rows) => ApiResponse::ok(json!(rows)).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "advertiser_report", &e)
            .into_response(),
    }
}

pub async fn admin_user_cohorts(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "users:read"]) {
        return resp;
    }
    match user_analytics_service::get_cohort_retention(&state.db) {
        Ok(cohorts) => ApiResponse::ok(json!(cohorts)).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "user_cohorts", &e)
            .into_response(),
    }
}

pub async fn admin_churn_rate(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["analytics:read", "revenue:read", "subscriptions:read"])
    {
        return resp;
    }
    match user_analytics_service::get_churn_rate(&state.db) {
        Ok(churn) => ApiResponse::ok(churn).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "churn_rate", &e)
            .into_response(),
    }
}

pub async fn admin_user_ltv(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "revenue:read"]) {
        return resp;
    }
    match user_analytics_service::get_lifetime_value(&state.db) {
        Ok(ltv) => ApiResponse::ok(ltv).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "user_ltv", &e)
            .into_response(),
    }
}

pub async fn admin_lifetime_value(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    admin_user_ltv(operator, State(state)).await
}

pub async fn admin_conversion_funnel(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "revenue:read"]) {
        return resp;
    }
    match user_analytics_service::get_conversion_funnel(&state.db) {
        Ok(funnel) => ApiResponse::ok(funnel).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "conversion_funnel", &e)
            .into_response(),
    }
}

// ============================================================================
// CORPORATE REQUESTS, TAXONOMY, NEWSLETTERS, REMINDERS, REDIRECTS & SETTINGS
