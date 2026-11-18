//! User Analytics Service (`src/services/user_analytics_service.rs`)
//!
//! Computes real cohort retention (`m0..m5` / `active_m1..active_m6`), subscription churn rate,
//! customer lifetime value (LTV + `avg_lifetime_months`), and 4-stage conversion funnel
//! directly from `users`, `order_invoices`, and `analytics_events`.
//! Zero mock data, zero fake multipliers.

use serde_json::{json, Map, Value};

use crate::db::Database;

fn round_one_decimal(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round_two_decimals(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// SQL predicate identifying users who hold (or held) a real subscriber tier
/// (excluding free, standard registered, and student/academic non-subscriber tiers).
const SUBSCRIBER_TIER_PREDICATE: &str = "subscription_tier IS NOT NULL \
    AND TRIM(subscription_tier) != '' \
    AND LOWER(TRIM(subscription_tier)) NOT IN ('free', 'standart kayıtlı okur', 'akademik / öğrenci okur', 'akademik okur')";

/// Computes 6-month (`m0..m5`, plus `active_m1..active_m6`) cohort retention matrix
/// grouped by user registration month (`SUBSTR(created_at, 1, 7)`).
pub fn get_cohort_retention(db: &Database) -> Result<Vec<Value>, String> {
    let conn = db.pool.get().map_err(|e| format!("DB pool hatası: {}", e))?;

    let mut cohort_stmt = conn
        .prepare(
            "SELECT SUBSTR(created_at, 1, 7) AS cohort_month,
                    COUNT(*) AS registered
             FROM users
             WHERE created_at IS NOT NULL AND LENGTH(TRIM(created_at)) >= 7
             GROUP BY SUBSTR(created_at, 1, 7)
             ORDER BY cohort_month DESC
             LIMIT 12",
        )
        .map_err(|e| e.to_string())?;

    let cohorts: Vec<(String, i64)> = cohort_stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let current_month: String = conn
        .query_row("SELECT strftime('%Y-%m', 'now')", [], |r| r.get(0))
        .unwrap_or_else(|_| chrono::Utc::now().format("%Y-%m").to_string());

    let mut results = Vec::with_capacity(cohorts.len());

    for (cohort_month, registered) in cohorts {
        let mut obj = Map::new();
        obj.insert("cohort_month".to_string(), json!(cohort_month));
        obj.insert("registered".to_string(), json!(registered));

        for offset in 0..=6 {
            let offset_arg = format!("+{} months", offset);
            let target_month: String = conn
                .query_row(
                    "SELECT strftime('%Y-%m', ?1 || '-01', ?2)",
                    rusqlite::params![&cohort_month, &offset_arg],
                    |r| r.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten()
                .unwrap_or_default();

            let active_count: i64 = if target_month.is_empty() || target_month > current_month {
                0
            } else if offset == 0 {
                // Month 0: cohort registration baseline (users joined in cohort_month)
                registered
            } else {
                // Subsequent months: users from this cohort with real analytics activity or paid order in target_month
                conn.query_row(
                    "SELECT COUNT(DISTINCT u.id)
                     FROM users u
                     WHERE SUBSTR(u.created_at, 1, 7) = ?1
                       AND (
                           EXISTS (
                               SELECT 1 FROM analytics_events ae
                               WHERE ae.user_id = u.id
                                 AND SUBSTR(ae.created_at, 1, 7) = ?2
                           )
                           OR EXISTS (
                               SELECT 1 FROM order_invoices oi
                               WHERE LOWER(oi.user_email) = LOWER(u.email)
                                 AND (LOWER(oi.payment_status) = 'paid' OR UPPER(oi.payment_status) LIKE 'PAID_%')
                                 AND SUBSTR(oi.created_at, 1, 7) = ?2
                           )
                       )",
                    rusqlite::params![&cohort_month, &target_month],
                    |r| r.get(0),
                )
                .unwrap_or(0)
            };

            let retention_pct = if registered > 0 {
                round_one_decimal((active_count as f64 / registered as f64) * 100.0)
            } else {
                0.0
            };

            if offset <= 5 {
                obj.insert(format!("m{}", offset), json!(retention_pct));
                obj.insert(format!("m{}_users", offset), json!(active_count));
            }
            if offset >= 1 {
                obj.insert(format!("active_m{}", offset), json!(active_count));
            }
        }

        results.push(Value::Object(obj));
    }

    Ok(results)
}

/// Alias for `get_cohort_retention` retained for API compatibility.
pub fn get_user_cohorts(db: &Database) -> Result<Vec<Value>, String> {
    get_cohort_retention(db)
}

/// Computes real subscription churn rate using `users.subscription_expires_at`
/// (excluding `'free'`, `'Standart Kayıtlı Okur'`, and `'Akademik / Öğrenci Okur'`).
pub fn get_churn_rate(db: &Database) -> Result<Value, String> {
    let conn = db.pool.get().map_err(|e| format!("DB pool hatası: {}", e))?;

    let total_sql = format!(
        "SELECT COUNT(*) FROM users WHERE {}",
        SUBSCRIBER_TIER_PREDICATE
    );
    let total_subscribers: i64 = conn.query_row(&total_sql, [], |r| r.get(0)).unwrap_or(0);

    let expired_sql = format!(
        "SELECT COUNT(*) FROM users
         WHERE {}
           AND subscription_expires_at IS NOT NULL
           AND TRIM(subscription_expires_at) != ''
           AND SUBSTR(subscription_expires_at, 1, 10) < date('now')",
        SUBSCRIBER_TIER_PREDICATE
    );
    let expired: i64 = conn.query_row(&expired_sql, [], |r| r.get(0)).unwrap_or(0);

    let active_subscribers = (total_subscribers - expired).max(0);
    let churn_pct = if total_subscribers > 0 {
        round_two_decimals((expired as f64 / total_subscribers as f64) * 100.0)
    } else {
        0.0
    };

    // Average subscriber lifespan in days (registration -> expiry or today)
    let lifespan_sql = format!(
        "SELECT COALESCE(AVG(MAX(0.0, julianday(
            CASE
                WHEN subscription_expires_at IS NOT NULL AND LENGTH(TRIM(subscription_expires_at)) >= 10
                THEN SUBSTR(TRIM(subscription_expires_at), 1, 10)
                ELSE date('now')
            END
         ) - julianday(SUBSTR(TRIM(created_at), 1, 10)))), 0.0)
         FROM users
         WHERE {} AND LENGTH(TRIM(created_at)) >= 10",
        SUBSCRIBER_TIER_PREDICATE
    );
    let avg_lifetime_days_f: f64 = conn
        .query_row(&lifespan_sql, [], |r| r.get(0))
        .unwrap_or(0.0);

    let avg_lifetime_days = avg_lifetime_days_f.round() as i64;
    let avg_lifetime_months = round_one_decimal(avg_lifetime_days_f / 30.4375);

    Ok(json!({
        "total_subscribers": total_subscribers,
        "active_subscribers": active_subscribers,
        "expired": expired,
        "churned_users": expired,
        "monthly_churn_pct": churn_pct,
        "avg_lifetime_days": avg_lifetime_days,
        "avg_lifetime_months": avg_lifetime_months,
    }))
}

/// Computes Customer Lifetime Value (LTV) and `avg_lifetime_months` from real `order_invoices`
/// and `users` (joined on `LOWER(user_email) = LOWER(email)`).
pub fn get_lifetime_value(db: &Database) -> Result<Value, String> {
    let conn = db.pool.get().map_err(|e| format!("DB pool hatası: {}", e))?;

    let (avg_ltv, paying_users, total_revenue): (f64, i64, f64) = conn
        .query_row(
            "SELECT COALESCE(AVG(user_total), 0.0),
                    COUNT(*),
                    COALESCE(SUM(user_total), 0.0)
             FROM (
                 SELECT LOWER(TRIM(user_email)) AS email_key,
                        SUM(amount_try) AS user_total
                 FROM order_invoices
                 WHERE LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%'
                 GROUP BY LOWER(TRIM(user_email))
             )",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap_or((0.0, 0, 0.0));

    let lifespan_sql = format!(
        "SELECT COALESCE(AVG(MAX(0.0, julianday(
            CASE
                WHEN subscription_expires_at IS NOT NULL AND LENGTH(TRIM(subscription_expires_at)) >= 10
                THEN SUBSTR(TRIM(subscription_expires_at), 1, 10)
                ELSE date('now')
            END
         ) - julianday(SUBSTR(TRIM(created_at), 1, 10)))), 0.0)
         FROM users
         WHERE {} AND LENGTH(TRIM(created_at)) >= 10",
        SUBSCRIBER_TIER_PREDICATE
    );
    let avg_days: f64 = conn.query_row(&lifespan_sql, [], |r| r.get(0)).unwrap_or(0.0);
    let avg_lifetime_months = round_one_decimal(avg_days / 30.4375);

    let tier_sql = format!(
        "SELECT u.subscription_tier,
                COUNT(DISTINCT u.id) AS user_count,
                COUNT(DISTINCT oi.id) AS invoice_count,
                COALESCE(SUM(oi.amount_try), 0.0) AS tier_revenue
         FROM users u
         LEFT JOIN order_invoices oi
                ON LOWER(TRIM(oi.user_email)) = LOWER(TRIM(u.email))
               AND (LOWER(oi.payment_status) = 'paid' OR UPPER(oi.payment_status) LIKE 'PAID_%')
         WHERE u.{}
         GROUP BY u.subscription_tier
         ORDER BY tier_revenue DESC, user_count DESC",
        SUBSCRIBER_TIER_PREDICATE
    );

    let mut tier_stmt = conn.prepare(&tier_sql).map_err(|e| e.to_string())?;
    let by_tier: Vec<Value> = tier_stmt
        .query_map([], |row| {
            let count: i64 = row.get(1)?;
            let inv_count: i64 = row.get(2)?;
            let revenue: f64 = row.get(3)?;
            let tier_avg_ltv = if count > 0 {
                round_two_decimals(revenue / count as f64)
            } else {
                0.0
            };
            Ok(json!({
                "tier": row.get::<_, String>(0)?,
                "user_count": count,
                "invoice_count": inv_count,
                "total_revenue": round_two_decimals(revenue),
                "avg_ltv_try": tier_avg_ltv,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut by_item_type = Map::new();
    if let Ok(mut item_stmt) = conn.prepare(
        "SELECT item_type, COALESCE(AVG(user_total), 0.0)
         FROM (
             SELECT LOWER(TRIM(user_email)) AS em, item_type, SUM(amount_try) AS user_total
             FROM order_invoices
             WHERE LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%'
             GROUP BY LOWER(TRIM(user_email)), item_type
         )
         GROUP BY item_type",
    ) {
        if let Ok(rows) = item_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))) {
            for (itype, val) in rows.flatten() {
                by_item_type.insert(itype, json!(round_two_decimals(val)));
            }
        }
    }

    Ok(json!({
        "avg_ltv_try": round_two_decimals(avg_ltv),
        "avg_lifetime_months": avg_lifetime_months,
        "paying_users": paying_users,
        "total_revenue": round_two_decimals(total_revenue),
        "by_tier": by_tier,
        "by_item_type": by_item_type,
    }))
}

/// Computes the 4-stage conversion funnel from 100% real SQLite counts (zero fake multipliers):
/// 1. `visitors` — distinct `session_id` count from `analytics_events`
/// 2. `registered` — total registered `users`
/// 3. `verified` — email-verified or gov-verified users (`email_verified != 0 OR is_gov_verified != 0`)
/// 4. `paid_or_active_subscribers` — active non-free subscribers (`subscription_expires_at` valid) or users with paid `order_invoices`
pub fn get_conversion_funnel(db: &Database) -> Result<Value, String> {
    let conn = db.pool.get().map_err(|e| format!("DB pool hatası: {}", e))?;

    let visitors: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT session_id) FROM analytics_events WHERE session_id != ''",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let registered: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
        .unwrap_or(0);

    let verified: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE email_verified != 0 OR is_gov_verified != 0",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let active_subscribers_sql = format!(
        "SELECT COUNT(*) FROM users u
         WHERE (
             u.{}
             AND (
                 u.subscription_expires_at IS NULL
                 OR TRIM(u.subscription_expires_at) = ''
                 OR SUBSTR(TRIM(u.subscription_expires_at), 1, 10) >= date('now')
             )
         )
         OR EXISTS (
             SELECT 1 FROM order_invoices oi
             WHERE LOWER(TRIM(oi.user_email)) = LOWER(TRIM(u.email))
               AND (LOWER(oi.payment_status) = 'paid' OR UPPER(oi.payment_status) LIKE 'PAID_%')
         )",
        SUBSCRIBER_TIER_PREDICATE
    );
    let active_or_paid_subscribers: i64 = conn
        .query_row(&active_subscribers_sql, [], |r| r.get(0))
        .unwrap_or(0);

    let paid_invoice_customers: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT LOWER(TRIM(user_email)))
             FROM order_invoices
             WHERE LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let pct = |num: i64, den: i64| -> f64 {
        if den > 0 {
            round_one_decimal((num as f64 / den as f64) * 100.0)
        } else {
            0.0
        }
    };

    Ok(json!({
        "visitors": visitors,
        "registered": registered,
        "verified": verified,
        "subscribed": verified,
        "active_subscribers": active_or_paid_subscribers,
        "paid": active_or_paid_subscribers,
        "paid_invoice_customers": paid_invoice_customers,
        "stages": [
            { "stage": "visitors", "label_tr": "Tekil Ziyaretçi Oturumu", "count": visitors },
            { "stage": "registered", "label_tr": "Kayıtlı Kullanıcı", "count": registered },
            { "stage": "verified", "label_tr": "Doğrulanmış / Kamu Okuru", "count": verified },
            { "stage": "paid_active", "label_tr": "Aktif / Ödeyen Abone", "count": active_or_paid_subscribers },
        ],
        "conversion_rates": {
            "visitor_to_registered": pct(registered, visitors),
            "registered_to_verified": pct(verified, registered),
            "verified_to_paid": pct(active_or_paid_subscribers, verified),
            "registered_to_subscribed": pct(verified, registered),
            "subscribed_to_paid": pct(active_or_paid_subscribers, verified),
        }
    }))
}
