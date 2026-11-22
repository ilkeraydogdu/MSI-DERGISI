use super::*;

fn wants_anonymized(params: &HashMap<String, String>) -> bool {
    matches!(
        params
            .get("anonymize")
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "yes")
    )
}

// ============================================================================

pub async fn admin_analytics_bundle(
    operator: OperatorUser,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let anonymize = wants_anonymized(&params);
    let contents = state.db.get_all_contents();
    let ads = state.db.get_all_ads();
    let users = state.db.get_all_users();
    let events = state.db.get_all_analytics_events();

    let content_reports: Vec<_> = contents
        .iter()
        .map(|c| analytics_service::compute_content_report_with_options(c, &events, anonymize))
        .collect();
    let ad_reports: Vec<_> = ads
        .iter()
        .map(|a| analytics_service::compute_ad_report_with_options(a, &events, anonymize))
        .collect();
    let reader_reports = if anonymize {
        analytics_service::compute_reader_reports_anonymized(
            &users,
            &events,
            &ads,
            &state.config.session_secret,
        )
    } else {
        analytics_service::compute_reader_reports(&users, &events, &ads)
    };

    Json(json!({
        "anonymized": anonymize,
        "contents": content_reports,
        "ads": ad_reports,
        "readers": reader_reports
    }))
    .into_response()
}

pub async fn admin_analytics_contents(
    operator: OperatorUser,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "analytics:read") {
        return resp;
    }
    let anonymize = wants_anonymized(&params);
    let contents = state.db.get_all_contents();
    let events = state.db.get_all_analytics_events();
    let reports: Vec<_> = contents
        .iter()
        .map(|c| analytics_service::compute_content_report_with_options(c, &events, anonymize))
        .collect();
    Json(json!(reports)).into_response()
}

pub async fn admin_analytics_ads(
    operator: OperatorUser,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "ads:read"]) {
        return resp;
    }
    let anonymize = wants_anonymized(&params);
    let ads = state.db.get_all_ads();
    let events = state.db.get_all_analytics_events();
    let reports: Vec<_> = ads
        .iter()
        .map(|a| analytics_service::compute_ad_report_with_options(a, &events, anonymize))
        .collect();
    Json(json!(reports)).into_response()
}

pub async fn admin_analytics_readers(
    operator: OperatorUser,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "users:read"]) {
        return resp;
    }
    let anonymize = wants_anonymized(&params);
    let users = state.db.get_all_users();
    let ads = state.db.get_all_ads();
    let events = state.db.get_all_analytics_events();
    let reports = if anonymize {
        analytics_service::compute_reader_reports_anonymized(
            &users,
            &events,
            &ads,
            &state.config.session_secret,
        )
    } else {
        analytics_service::compute_reader_reports(&users, &events, &ads)
    };
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
    let anonymize = wants_anonymized(&params);
    let (from, to) = default_date_bounds(&params);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if to >= today {
        let _ = state.db.aggregate_analytics_for_date(&today);
    }
    let mut rows = state.db.get_daily_aggregated_analytics(&from, &to);
    if anonymize {
        rows.retain(|r| {
            let sess = r["unique_sessions"].as_i64().unwrap_or(0);
            let cnt = r["event_count"].as_i64().unwrap_or(0);
            sess >= 3 || (sess == 0 && cnt >= 3)
        });
    }
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
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "users:read"]) {
        return resp;
    }
    let anonymize = wants_anonymized(&params);
    match user_analytics_service::get_cohort_retention_opts(&state.db, anonymize) {
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
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["analytics:read", "revenue:read"]) {
        return resp;
    }
    let anonymize = wants_anonymized(&params);
    match user_analytics_service::get_lifetime_value_opts(&state.db, anonymize) {
        Ok(ltv) => ApiResponse::ok(ltv).into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "user_ltv", &e)
            .into_response(),
    }
}

pub async fn admin_lifetime_value(
    operator: OperatorUser,
    query: Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    admin_user_ltv(operator, query, State(state)).await
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

pub async fn admin_subscribers_report(
    operator: OperatorUser,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) =
        require_any_perm(&operator, &["analytics:read", "subscriptions:read", "revenue:read"])
    {
        return resp;
    }
    let anonymize = wants_anonymized(&params);
    let month = params.get("month").map(|s| s.as_str());
    let mut report = state.db.compute_subscriber_report(month);
    let gov_count = report["gov_verified_users"].as_i64().unwrap_or(0);

    let inst_counts: Vec<(String, i64)> = report["by_institution"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some((
                        v["organization_category"].as_str()?.to_string(),
                        v["count"].as_i64()?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let institution_breakdown = if anonymize {
        analytics_service::apply_k_anonymity_to_institution_counts(inst_counts, 3)
    } else {
        analytics_service::apply_k_anonymity_to_institution_counts(inst_counts, 1)
    };

    let reports_history = state.db.list_b2b_monthly_reports(month);
    if let Some(obj) = report.as_object_mut() {
        obj.insert("anonymized".to_string(), json!(anonymize));
        obj.insert("gov_verified_count".to_string(), json!(gov_count));
        obj.insert(
            "institution_breakdown".to_string(),
            json!(institution_breakdown),
        );
        if anonymize {
            obj.insert("by_institution".to_string(), json!(institution_breakdown));
            let raw_tiers: Vec<(String, i64)> = obj
                .get("by_tier")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            Some((
                                v["tier"].as_str()?.to_string(),
                                v["count"].as_i64()?,
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let mut masked_tiers: Vec<Value> = Vec::new();
            let mut suppressed_tier_sum: i64 = 0;
            for (tier, count) in raw_tiers {
                if count >= 3 {
                    masked_tiers.push(json!({ "tier": tier, "count": count }));
                } else {
                    suppressed_tier_sum += count;
                }
            }
            if suppressed_tier_sum >= 3 {
                masked_tiers.push(json!({
                    "tier": "Diğer Paketler (k<3)",
                    "count": suppressed_tier_sum
                }));
            }
            obj.insert("by_tier".to_string(), json!(masked_tiers));
        } else {
            obj.insert("monthly_reports".to_string(), json!(reports_history));
        }
    }

    ApiResponse::ok(report).into_response()
}

// ============================================================================
// CORPORATE REQUESTS, TAXONOMY, NEWSLETTERS, REMINDERS, REDIRECTS & SETTINGS
