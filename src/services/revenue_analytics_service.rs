//! Revenue Analytics Service (`src/services/revenue_analytics_service.rs`)
//!
//! Queries real SQLite commerce & advertising tables (`order_invoices`, `ad_campaigns`,
//! `analytics_events`, `analytics_daily_agg`, `ssb_tags`) with space-safe `SUBSTR(..., 1, 10)`
//! date filtering and zero mock data.

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::db::Database;

fn normalize_date_bound(raw: &str, default_val: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 10 {
        trimmed[..10].to_string()
    } else if trimmed.is_empty() {
        default_val.to_string()
    } else {
        trimmed.to_string()
    }
}

fn round_two_decimals(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Computes comprehensive revenue summary over `[from, to]` (inclusive `YYYY-MM-DD`)
/// from the real `order_invoices` table.
/// Includes `total_revenue`, `total_amount`, `invoice_count`, `subscription_revenue`,
/// `single_item_revenue`, `daily`, `weekly_total`, `monthly_total`, `monthly_series`,
/// `by_item_type`, and `breakdown` (for `#tbody-revenue-breakdown` in Admin Tab 14).
pub fn get_revenue_summary(db: &Database, from: &str, to: &str) -> Result<Value, String> {
    let conn = db.pool.get().map_err(|e| format!("DB pool hatası: {}", e))?;

    let from_d = normalize_date_bound(from, "2020-01-01");
    let to_d = normalize_date_bound(to, "2099-12-31");

    // 1. Daily paid revenue series (handles both 'paid' and 'PAID_VIRTUAL_POS' and space/ISO timestamps)
    let mut daily_stmt = conn
        .prepare(
            "SELECT SUBSTR(created_at, 1, 10) AS day_date,
                    COALESCE(SUM(amount_try), 0.0) AS day_amount,
                    COUNT(*) AS day_count
             FROM order_invoices
             WHERE (LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%')
               AND SUBSTR(created_at, 1, 10) BETWEEN ?1 AND ?2
             GROUP BY SUBSTR(created_at, 1, 10)
             ORDER BY day_date ASC",
        )
        .map_err(|e| e.to_string())?;

    let daily: Vec<Value> = daily_stmt
        .query_map(rusqlite::params![&from_d, &to_d], |row| {
            Ok(json!({
                "date": row.get::<_, String>(0)?,
                "amount": round_two_decimals(row.get::<_, f64>(1)?),
                "invoice_count": row.get::<_, i64>(2)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // 2. Overall KPIs in range
    let (total_revenue, invoice_count, subscription_revenue, single_item_revenue): (
        f64,
        i64,
        f64,
        f64,
    ) = conn
        .query_row(
            "SELECT COALESCE(SUM(amount_try), 0.0),
                    COUNT(*),
                    COALESCE(SUM(CASE WHEN LOWER(item_type) IN ('subscription', 'subscription_plan') THEN amount_try ELSE 0.0 END), 0.0),
                    COALESCE(SUM(CASE WHEN LOWER(item_type) NOT IN ('subscription', 'subscription_plan') THEN amount_try ELSE 0.0 END), 0.0)
             FROM order_invoices
             WHERE (LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%')
               AND SUBSTR(created_at, 1, 10) BETWEEN ?1 AND ?2",
            rusqlite::params![&from_d, &to_d],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| e.to_string())?;

    let total_revenue = round_two_decimals(total_revenue);
    let subscription_revenue = round_two_decimals(subscription_revenue);
    let single_item_revenue = round_two_decimals(single_item_revenue);

    // 3. Monthly series (YYYY-MM)
    let mut monthly_stmt = conn
        .prepare(
            "SELECT SUBSTR(created_at, 1, 7) AS month,
                    COALESCE(SUM(amount_try), 0.0) AS revenue,
                    COUNT(*) AS count,
                    COALESCE(SUM(CASE WHEN LOWER(item_type) IN ('subscription', 'subscription_plan') THEN amount_try ELSE 0.0 END), 0.0) AS subscription_rev,
                    COALESCE(SUM(CASE WHEN LOWER(item_type) NOT IN ('subscription', 'subscription_plan') THEN amount_try ELSE 0.0 END), 0.0) AS single_item_rev
             FROM order_invoices
             WHERE (LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%')
               AND SUBSTR(created_at, 1, 10) BETWEEN ?1 AND ?2
             GROUP BY SUBSTR(created_at, 1, 7)
             ORDER BY month ASC",
        )
        .map_err(|e| e.to_string())?;

    let monthly_series: Vec<Value> = monthly_stmt
        .query_map(rusqlite::params![&from_d, &to_d], |row| {
            Ok(json!({
                "month": row.get::<_, String>(0)?,
                "revenue": round_two_decimals(row.get::<_, f64>(1)?),
                "invoice_count": row.get::<_, i64>(2)?,
                "subscription_revenue": round_two_decimals(row.get::<_, f64>(3)?),
                "single_item_revenue": round_two_decimals(row.get::<_, f64>(4)?),
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // 4. Item-type map (`by_item_type`) & detailed `breakdown` rows for `#tbody-revenue-breakdown`
    let mut type_stmt = conn
        .prepare(
            "SELECT item_type,
                    COALESCE(SUM(amount_try), 0.0) AS type_revenue,
                    COUNT(*) AS type_count
             FROM order_invoices
             WHERE (LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%')
               AND SUBSTR(created_at, 1, 10) BETWEEN ?1 AND ?2
             GROUP BY item_type
             ORDER BY type_revenue DESC",
        )
        .map_err(|e| e.to_string())?;

    let mut by_item_type = Map::new();
    let type_rows: Vec<(String, f64, i64)> = type_stmt
        .query_map(rusqlite::params![&from_d, &to_d], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, i64>(2)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    for (itype, rev, _) in &type_rows {
        by_item_type.insert(itype.clone(), json!(round_two_decimals(*rev)));
    }

    let mut breakdown_stmt = conn
        .prepare(
            "SELECT item_type,
                    item_code,
                    MAX(item_title) AS item_title,
                    COUNT(*) AS tx_count,
                    COALESCE(SUM(amount_try), 0.0) AS item_revenue,
                    MAX(SUBSTR(created_at, 1, 10)) AS last_sold_date
             FROM order_invoices
             WHERE (LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%')
               AND SUBSTR(created_at, 1, 10) BETWEEN ?1 AND ?2
             GROUP BY item_type, item_code
             ORDER BY item_revenue DESC",
        )
        .map_err(|e| e.to_string())?;

    let breakdown: Vec<Value> = breakdown_stmt
        .query_map(rusqlite::params![&from_d, &to_d], |row| {
            let rev: f64 = row.get(4)?;
            let share_pct = if total_revenue > 0.0 {
                round_two_decimals((rev / total_revenue) * 100.0)
            } else {
                0.0
            };
            Ok(json!({
                "item_type": row.get::<_, String>(0)?,
                "item_code": row.get::<_, String>(1)?,
                "item_title": row.get::<_, String>(2)?,
                "invoice_count": row.get::<_, i64>(3)?,
                "revenue": round_two_decimals(rev),
                "amount": round_two_decimals(rev),
                "share_pct": share_pct,
                "last_sold_date": row.get::<_, String>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // 5. Recent paid transactions (with user_email & organization_name)
    let mut recent_stmt = conn
        .prepare(&format!(
            "SELECT invoice_no, user_email, organization_name, item_type, item_code, item_title,
                    amount_try, SUBSTR(created_at, 1, 10) AS paid_date
             FROM order_invoices
             WHERE (LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%')
               AND SUBSTR(created_at, 1, 10) BETWEEN ?1 AND ?2
             ORDER BY created_at DESC
             LIMIT {}", crate::db::constants::RECENT_TRANSACTIONS_LIMIT
        ))
        .map_err(|e| e.to_string())?;

    let recent_invoices: Vec<Value> = recent_stmt
        .query_map(rusqlite::params![&from_d, &to_d], |row| {
            Ok(json!({
                "invoice_no": row.get::<_, String>(0)?,
                "user_email": row.get::<_, String>(1)?,
                "organization_name": row.get::<_, String>(2)?,
                "item_type": row.get::<_, String>(3)?,
                "item_code": row.get::<_, String>(4)?,
                "item_title": row.get::<_, String>(5)?,
                "amount_try": round_two_decimals(row.get::<_, f64>(6)?),
                "date": row.get::<_, String>(7)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Real 7-day window total within `[from_d, to_d]`
    let weekly_total: f64 = round_two_decimals(
        conn.query_row(
            "SELECT COALESCE(SUM(amount_try), 0.0)
             FROM order_invoices
             WHERE (LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%')
               AND SUBSTR(created_at, 1, 10) >= date(?2, '-6 days')
               AND SUBSTR(created_at, 1, 10) <= ?2
               AND SUBSTR(created_at, 1, 10) >= ?1",
            rusqlite::params![&from_d, &to_d],
            |r| r.get(0),
        )
        .unwrap_or(0.0),
    );

    Ok(json!({
        "date_from": from_d,
        "date_to": to_d,
        "total_revenue": total_revenue,
        "total_amount": total_revenue,
        "invoice_count": invoice_count,
        "subscription_revenue": subscription_revenue,
        "single_item_revenue": single_item_revenue,
        "weekly_total": weekly_total,
        "monthly_total": total_revenue,
        "daily": daily,
        "monthly_series": monthly_series,
        "by_item_type": by_item_type,
        "breakdown": breakdown,
        "recent_invoices": recent_invoices,
    }))
}

/// Collects per-`ad_id` `(impressions, clicks)` within `[from_d, to_d]` by combining
/// `analytics_events` and `analytics_daily_agg` (taking `max` per ad_id so pre-aggregated
/// and raw events never double-count).
fn load_period_ad_metrics(
    conn: &rusqlite::Connection,
    from_d: &str,
    to_d: &str,
) -> HashMap<i64, (i64, i64)> {
    let mut metrics: HashMap<i64, (i64, i64)> = HashMap::new();

    if let Ok(mut stmt) = conn.prepare(
        "SELECT ad_id,
                COALESCE(SUM(CASE WHEN event_type IN ('ad_impression', 'impression', 'ad_view') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN event_type IN ('ad_click', 'click') THEN 1 ELSE 0 END), 0)
         FROM analytics_events
         WHERE ad_id IS NOT NULL
           AND SUBSTR(created_at, 1, 10) BETWEEN ?1 AND ?2
         GROUP BY ad_id",
    ) {
        if let Ok(rows) = stmt.query_map(rusqlite::params![from_d, to_d], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        }) {
            for row in rows.flatten() {
                metrics.insert(row.0, (row.1, row.2));
            }
        }
    }

    if let Ok(mut agg_stmt) = conn.prepare(
        "SELECT ad_id,
                COALESCE(SUM(CASE WHEN event_type IN ('ad_impression', 'impression', 'ad_view') THEN event_count ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN event_type IN ('ad_click', 'click') THEN event_count ELSE 0 END), 0)
         FROM analytics_daily_agg
         WHERE ad_id > 0
           AND SUBSTR(date, 1, 10) BETWEEN ?1 AND ?2
         GROUP BY ad_id",
    ) {
        if let Ok(rows) = agg_stmt.query_map(rusqlite::params![from_d, to_d], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        }) {
            for (ad_id, agg_imp, agg_clk) in rows.flatten() {
                let entry = metrics.entry(ad_id).or_insert((0, 0));
                entry.0 = entry.0.max(agg_imp);
                entry.1 = entry.1.max(agg_clk);
            }
        }
    }

    metrics
}

/// Computes SSB keyword/tag advertising performance by joining `ad_campaigns` (`target_ssb_tags_json`)
/// with `analytics_events` / `analytics_daily_agg` and `ssb_tags`.
pub fn get_keyword_performance(db: &Database, from: &str, to: &str) -> Result<Vec<Value>, String> {
    let conn = db.pool.get().map_err(|e| format!("DB pool hatası: {}", e))?;
    let from_d = normalize_date_bound(from, "2020-01-01");
    let to_d = normalize_date_bound(to, "2099-12-31");

    // Load human-readable SSB tag labels
    let mut tag_labels: HashMap<String, String> = HashMap::new();
    if let Ok(mut tag_stmt) = conn.prepare("SELECT code, name_tr FROM ssb_tags") {
        if let Ok(rows) = tag_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))) {
            for (code, name_tr) in rows.flatten() {
                tag_labels.insert(code, name_tr);
            }
        }
    }

    let period_metrics = load_period_ad_metrics(&conn, &from_d, &to_d);

    let mut ad_stmt = conn
        .prepare(
            "SELECT id, slot_code, sponsor_name, sponsor_type, target_ssb_tags_json,
                    impressions_count, clicks_count, budget_score, is_active
             FROM ad_campaigns",
        )
        .map_err(|e| e.to_string())?;

    #[derive(Default)]
    struct KeywordAcc {
        active_campaigns: i64,
        total_campaigns: i64,
        period_impressions: i64,
        period_clicks: i64,
        lifetime_impressions: i64,
        lifetime_clicks: i64,
        budget_sum: i64,
    }

    let mut kw_map: HashMap<String, KeywordAcc> = HashMap::new();

    let ad_rows = ad_stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, i64>(8)? != 0,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in ad_rows.flatten() {
        let (ad_id, tags_json, life_imp, life_clk, budget_score, is_active) = row;
        let parsed_tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let (p_imp, p_clk) = period_metrics.get(&ad_id).copied().unwrap_or((0, 0));

        for raw_tag in parsed_tags {
            let tag = raw_tag.trim().to_string();
            if tag.is_empty() || tag == "*" {
                continue;
            }
            let acc = kw_map.entry(tag).or_default();
            acc.total_campaigns += 1;
            if is_active {
                acc.active_campaigns += 1;
            }
            acc.period_impressions += p_imp;
            acc.period_clicks += p_clk;
            acc.lifetime_impressions += life_imp;
            acc.lifetime_clicks += life_clk;
            acc.budget_sum += budget_score;
        }
    }

    let mut results: Vec<Value> = kw_map
        .into_iter()
        .map(|(keyword, acc)| {
            // Report period metrics when present in [from_d, to_d]; expose lifetime alongside
            let impressions = if acc.period_impressions > 0 {
                acc.period_impressions
            } else {
                0
            };
            let clicks = if acc.period_impressions > 0 {
                acc.period_clicks
            } else {
                0
            };
            let ctr_pct = if impressions > 0 {
                round_two_decimals((clicks as f64 / impressions as f64) * 100.0)
            } else {
                0.0
            };
            let lifetime_ctr_pct = if acc.lifetime_impressions > 0 {
                round_two_decimals((acc.lifetime_clicks as f64 / acc.lifetime_impressions as f64) * 100.0)
            } else {
                0.0
            };
            let avg_budget = if acc.total_campaigns > 0 {
                round_two_decimals(acc.budget_sum as f64 / acc.total_campaigns as f64)
            } else {
                0.0
            };
            let label_tr = tag_labels
                .get(&keyword)
                .cloned()
                .unwrap_or_else(|| keyword.clone());

            json!({
                "keyword": keyword,
                "label_tr": label_tr,
                "active_campaigns": acc.active_campaigns,
                "total_campaigns": acc.total_campaigns,
                "impressions": impressions,
                "clicks": clicks,
                "ctr": format!("{:.2}", ctr_pct),
                "ctr_pct": ctr_pct,
                "lifetime_impressions": acc.lifetime_impressions,
                "lifetime_clicks": acc.lifetime_clicks,
                "lifetime_ctr_pct": lifetime_ctr_pct,
                "avg_budget_score": avg_budget,
            })
        })
        .collect();

    results.sort_by(|a, b| {
        b["impressions"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&a["impressions"].as_i64().unwrap_or(0))
            .then_with(|| {
                b["lifetime_impressions"]
                    .as_i64()
                    .unwrap_or(0)
                    .cmp(&a["lifetime_impressions"].as_i64().unwrap_or(0))
            })
    });
    results.truncate(crate::db::constants::TOP_PRODUCTS_LIMIT);

    Ok(results)
}

/// Computes sponsor/advertiser-level performance report from `ad_campaigns` joined with
/// `analytics_events` and `analytics_daily_agg` over `[from, to]`.
pub fn get_advertiser_report(db: &Database, from: &str, to: &str) -> Result<Vec<Value>, String> {
    let conn = db.pool.get().map_err(|e| format!("DB pool hatası: {}", e))?;
    let from_d = normalize_date_bound(from, "2020-01-01");
    let to_d = normalize_date_bound(to, "2099-12-31");

    let period_metrics = load_period_ad_metrics(&conn, &from_d, &to_d);

    let mut stmt = conn
        .prepare(
            "SELECT id, slot_code, sponsor_name, sponsor_type,
                    impressions_count, clicks_count, budget_score, is_active
             FROM ad_campaigns
             ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;

    struct SponsorAcc {
        sponsor_type: String,
        campaigns: i64,
        active_ads: i64,
        slots: Vec<String>,
        period_impressions: i64,
        period_clicks: i64,
        lifetime_impressions: i64,
        lifetime_clicks: i64,
        budget_sum: i64,
    }

    let mut sponsor_map: HashMap<String, SponsorAcc> = HashMap::new();

    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)? != 0,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows.flatten() {
        let (ad_id, slot_code, sponsor_name, sponsor_type, life_imp, life_clk, budget, is_active) = row;
        let (p_imp, p_clk) = period_metrics.get(&ad_id).copied().unwrap_or((0, 0));

        let acc = sponsor_map
            .entry(sponsor_name.trim().to_string())
            .or_insert_with(|| SponsorAcc {
                sponsor_type: sponsor_type.clone(),
                campaigns: 0,
                active_ads: 0,
                slots: Vec::new(),
                period_impressions: 0,
                period_clicks: 0,
                lifetime_impressions: 0,
                lifetime_clicks: 0,
                budget_sum: 0,
            });

        acc.campaigns += 1;
        if is_active {
            acc.active_ads += 1;
        }
        if !acc.slots.contains(&slot_code) {
            acc.slots.push(slot_code);
        }
        acc.period_impressions += p_imp;
        acc.period_clicks += p_clk;
        acc.lifetime_impressions += life_imp;
        acc.lifetime_clicks += life_clk;
        acc.budget_sum += budget;
    }

    let mut results: Vec<Value> = sponsor_map
        .into_iter()
        .map(|(sponsor_name, acc)| {
            let ctr_pct = if acc.period_impressions > 0 {
                round_two_decimals((acc.period_clicks as f64 / acc.period_impressions as f64) * 100.0)
            } else {
                0.0
            };
            let lifetime_ctr_pct = if acc.lifetime_impressions > 0 {
                round_two_decimals((acc.lifetime_clicks as f64 / acc.lifetime_impressions as f64) * 100.0)
            } else {
                0.0
            };
            let avg_budget = if acc.campaigns > 0 {
                round_two_decimals(acc.budget_sum as f64 / acc.campaigns as f64)
            } else {
                0.0
            };

            json!({
                "advertiser": sponsor_name,
                "sponsor_name": sponsor_name,
                "sponsor_type": acc.sponsor_type,
                "campaigns": acc.campaigns,
                "active_ads": acc.active_ads,
                "slots": acc.slots,
                "impressions": acc.period_impressions,
                "clicks": acc.period_clicks,
                "ctr_pct": ctr_pct,
                "lifetime_impressions": acc.lifetime_impressions,
                "lifetime_clicks": acc.lifetime_clicks,
                "lifetime_ctr_pct": lifetime_ctr_pct,
                "avg_budget_score": avg_budget,
            })
        })
        .collect();

    results.sort_by(|a, b| {
        b["impressions"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&a["impressions"].as_i64().unwrap_or(0))
            .then_with(|| {
                b["lifetime_impressions"]
                    .as_i64()
                    .unwrap_or(0)
                    .cmp(&a["lifetime_impressions"].as_i64().unwrap_or(0))
            })
    });

    Ok(results)
}
