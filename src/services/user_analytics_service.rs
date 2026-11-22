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
    get_cohort_retention_opts(db, false)
}

pub fn get_cohort_retention_opts(db: &Database, anonymize: bool) -> Result<Vec<Value>, String> {
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
        if anonymize && registered < 3 {
            continue;
        }
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

            let suppress_cell = anonymize && offset >= 1 && active_count > 0 && active_count < 3;
            let retention_pct = if registered > 0 {
                round_one_decimal((active_count as f64 / registered as f64) * 100.0)
            } else {
                0.0
            };

            if offset <= 5 {
                if suppress_cell {
                    obj.insert(format!("m{}", offset), Value::Null);
                    obj.insert(format!("m{}_users", offset), Value::Null);
                } else {
                    obj.insert(format!("m{}", offset), json!(retention_pct));
                    obj.insert(format!("m{}_users", offset), json!(active_count));
                }
            }
            if offset >= 1 {
                if suppress_cell {
                    obj.insert(format!("active_m{}", offset), Value::Null);
                } else {
                    obj.insert(format!("active_m{}", offset), json!(active_count));
                }
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
    get_lifetime_value_opts(db, false)
}

pub fn get_lifetime_value_opts(db: &Database, anonymize: bool) -> Result<Value, String> {
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
    let raw_tiers: Vec<(String, i64, i64, f64)> = tier_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut by_tier: Vec<Value> = Vec::new();
    let mut sup_users: i64 = 0;
    let mut sup_invoices: i64 = 0;
    let mut sup_revenue: f64 = 0.0;
    for (tier, count, inv_count, revenue) in raw_tiers {
        if anonymize && count < 3 {
            sup_users += count;
            sup_invoices += inv_count;
            sup_revenue += revenue;
            continue;
        }
        let tier_avg_ltv = if count > 0 {
            round_two_decimals(revenue / count as f64)
        } else {
            0.0
        };
        by_tier.push(json!({
            "tier": tier,
            "user_count": count,
            "invoice_count": inv_count,
            "total_revenue": round_two_decimals(revenue),
            "avg_ltv_try": tier_avg_ltv,
        }));
    }
    if anonymize && sup_users >= 3 {
        let tier_avg_ltv = round_two_decimals(sup_revenue / sup_users as f64);
        by_tier.push(json!({
            "tier": "Diğer Paketler (k<3)",
            "user_count": sup_users,
            "invoice_count": sup_invoices,
            "total_revenue": round_two_decimals(sup_revenue),
            "avg_ltv_try": tier_avg_ltv,
        }));
    }

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

    let suppress_ltv_totals = anonymize && paying_users > 0 && paying_users < 3;
    if suppress_ltv_totals {
        by_item_type.clear();
    }

    Ok(json!({
        "avg_ltv_try": if suppress_ltv_totals { Value::Null } else { json!(round_two_decimals(avg_ltv)) },
        "avg_lifetime_months": avg_lifetime_months,
        "paying_users": if suppress_ltv_totals { Value::Null } else { json!(paying_users) },
        "total_revenue": if suppress_ltv_totals { Value::Null } else { json!(round_two_decimals(total_revenue)) },
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
            "SELECT COUNT(DISTINCT session_id) FROM analytics_events WHERE session_id IS NOT NULL AND TRIM(session_id) != ''",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let registered: i64 = conn
        .query_row("SELECT COUNT(*) FROM users WHERE role != 'deleted'", [], |r| r.get(0))
        .unwrap_or(0);

    let verified: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE role != 'deleted' AND (email_verified != 0 OR is_gov_verified != 0)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let active_subscribers_sql = format!(
        "SELECT COUNT(*) FROM users u
         WHERE u.role != 'deleted'
           AND (
             (
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
             )
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
             WHERE (LOWER(payment_status) = 'paid' OR UPPER(payment_status) LIKE 'PAID_%')
               AND NOT EXISTS (
                   SELECT 1 FROM users u
                   WHERE LOWER(TRIM(u.email)) = LOWER(TRIM(order_invoices.user_email))
                     AND u.role = 'deleted'
               )
               AND LOWER(TRIM(order_invoices.user_email)) NOT LIKE 'deleted\\_%@anonymized.local' ESCAPE '\\'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let pct = |num: i64, den: i64| -> f64 {
        if den > 0 && num > 0 {
            round_one_decimal(((num as f64 / den as f64) * 100.0).clamp(0.0, 100.0))
        } else {
            0.0
        }
    };

    Ok(json!({
        "visitors": visitors,
        "registered": registered,
        "verified": verified,
        "active_subscribers": active_or_paid_subscribers,
        "subscribed": active_or_paid_subscribers,
        "paid_invoice_customers": paid_invoice_customers,
        "paid": paid_invoice_customers,
        "stages": [
            { "stage": "visitors", "label_tr": "Tekil Ziyaretçi Oturumu", "count": visitors },
            { "stage": "registered", "label_tr": "Kayıtlı Kullanıcı", "count": registered },
            { "stage": "verified", "label_tr": "Doğrulanmış / Kamu Okuru", "count": verified },
            { "stage": "active_subscribers", "label_tr": "Aktif Abone", "count": active_or_paid_subscribers },
            { "stage": "paid_invoice_customers", "label_tr": "Faturalı Üye", "count": paid_invoice_customers },
        ],
        "conversion_rates": {
            "visitor_to_registered": pct(registered, visitors),
            "registered_to_verified": pct(verified, registered),
            "verified_to_active_subscriber": pct(active_or_paid_subscribers, verified),
            "active_subscriber_to_paid": pct(paid_invoice_customers, active_or_paid_subscribers),
            "verified_to_paid": pct(paid_invoice_customers, verified),
            "registered_to_subscribed": pct(active_or_paid_subscribers, registered),
            "subscribed_to_paid": pct(paid_invoice_customers, active_or_paid_subscribers),
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(label: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("/tmp/msi_user_analytics_{}_{}_{}.db", label, std::process::id(), nanos)
    }

    #[test]
    fn funnel_separates_verified_active_subscribers_and_paid_and_clamps_to_100() {
        let db_path = temp_db_path("funnel_clamping");
        let db = Database::new(&db_path).expect("db init");
        let conn = db.open_conn().expect("conn");

        // Clear any existing analytics events and users for deterministic counts
        conn.execute("DELETE FROM analytics_events", []).unwrap();
        conn.execute("DELETE FROM order_invoices", []).unwrap();
        conn.execute("DELETE FROM users", []).unwrap();

        // Only 1 visitor session, while 4 users exist -> registered > visitors (clamped to 100.0)
        conn.execute(
            "INSERT INTO analytics_events (session_id, organization_category, organization_name, event_type, created_at)
             VALUES ('sess-1', 'Sektör Kuruluşları', 'Org1', 'page_view', '2026-11-19 10:00:00')",
            [],
        )
        .unwrap();

        // User 1: unverified free reader (registered only)
        conn.execute(
            "INSERT INTO users (email, password_hash, full_name, role, organization_category, organization_name, job_title, is_gov_verified, subscription_tier, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash, created_at, email_verified)
             VALUES ('u1@test.local', 'h', 'U1', 'registered', 'Sektör Kuruluşları', 'Org1', 'Dev', 0, 'free', '2026-11-19 10:00:00', '127.0.0.1', 'h1', '2026-11-19 10:00:00', 0)",
            [],
        )
        .unwrap();

        // User 2: verified free reader (verified, NOT active subscriber)
        conn.execute(
            "INSERT INTO users (email, password_hash, full_name, role, organization_category, organization_name, job_title, is_gov_verified, subscription_tier, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash, created_at, email_verified)
             VALUES ('u2@test.local', 'h', 'U2', 'registered', 'Sektör Kuruluşları', 'Org2', 'Dev', 0, 'free', '2026-11-19 10:00:00', '127.0.0.1', 'h2', '2026-11-19 10:00:00', 1)",
            [],
        )
        .unwrap();

        // User 3: active subscriber with paid invoice, NOT verified (so active_subscribers = 2, verified = 1, paid = 1)
        // User 4: active subscriber also NOT verified so active_subscribers (2) > verified (1) -> clamped to 100.0
        conn.execute(
            "INSERT INTO users (email, password_hash, full_name, role, organization_category, organization_name, job_title, is_gov_verified, subscription_tier, subscription_expires_at, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash, created_at, email_verified)
             VALUES ('u3@test.local', 'h', 'U3', 'subscriber_digital', 'Sektör Kuruluşları', 'Org3', 'Dev', 0, 'Dijital Abonelik', '2030-12-31', '2026-11-19 10:00:00', '127.0.0.1', 'h3', '2026-11-19 10:00:00', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (email, password_hash, full_name, role, organization_category, organization_name, job_title, is_gov_verified, subscription_tier, subscription_expires_at, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash, created_at, email_verified)
             VALUES ('u4@test.local', 'h', 'U4', 'subscriber_corporate', 'Sektör Kuruluşları', 'Org4', 'Dev', 0, 'Kurumsal', '2030-12-31', '2026-11-19 10:00:00', '127.0.0.1', 'h4', '2026-11-19 10:00:00', 0)",
            [],
        )
        .unwrap();

        // Only u3 has a paid invoice -> paid_invoice_customers = 1
        conn.execute(
            "INSERT INTO order_invoices (invoice_no, user_email, full_name, organization_name, item_type, item_code, item_title, amount_try, payment_status, created_at, pdf_watermark_code)
             VALUES ('INV-1', 'u3@test.local', 'U3', 'Org3', 'subscription', 'dijital', 'Dijital', 450.0, 'paid', '2026-11-19 10:00:00', 'WM-1')",
            [],
        )
        .unwrap();
        drop(conn);

        let funnel = get_conversion_funnel(&db).expect("funnel ok");
        assert_eq!(funnel["visitors"], 1);
        assert_eq!(funnel["registered"], 4);
        assert_eq!(funnel["verified"], 1);
        assert_eq!(funnel["active_subscribers"], 2);
        assert_eq!(funnel["subscribed"], 2);
        assert_eq!(funnel["paid_invoice_customers"], 1);
        assert_eq!(funnel["paid"], 1);
        assert_ne!(funnel["verified"], funnel["subscribed"]);

        let rates = &funnel["conversion_rates"];
        // registered (4) / visitors (1) would be 400%, must clamp to 100.0
        assert_eq!(rates["visitor_to_registered"], 100.0);
        // active_subscribers (2) / verified (1) would be 200%, must clamp to 100.0
        assert_eq!(rates["verified_to_active_subscriber"], 100.0);
        assert_eq!(rates["registered_to_verified"], 25.0);
        assert_eq!(rates["registered_to_subscribed"], 50.0);
        assert_eq!(rates["subscribed_to_paid"], 50.0);
        assert_eq!(rates["active_subscriber_to_paid"], 50.0);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }

    #[test]
    fn funnel_excludes_deleted_users_even_with_retained_paid_invoices() {
        let db_path = temp_db_path("funnel_deleted_guard");
        let db = Database::new(&db_path).expect("db init");
        let conn = db.open_conn().expect("conn");

        conn.execute("DELETE FROM analytics_events", []).unwrap();
        conn.execute("DELETE FROM order_invoices", []).unwrap();
        conn.execute("DELETE FROM users", []).unwrap();

        // Active verified subscriber (initially without paid invoice so paid_invoice_customers == 0)
        conn.execute(
            "INSERT INTO users (email, password_hash, full_name, role, organization_category, organization_name, job_title, is_gov_verified, subscription_tier, subscription_expires_at, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash, created_at, email_verified)
             VALUES ('active@test.local', 'h', 'Active', 'subscriber_digital', 'Sektör Kuruluşları', 'Org1', 'Dev', 1, 'Dijital Abonelik', '2030-12-31', '2026-11-19 10:00:00', '127.0.0.1', 'h1', '2026-11-19 10:00:00', 1)",
            [],
        )
        .unwrap();

        // Deleted user (anonymized email + retained paid invoice for tax records)
        conn.execute(
            "INSERT INTO users (email, password_hash, full_name, role, organization_category, organization_name, job_title, is_gov_verified, subscription_tier, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash, created_at, email_verified)
             VALUES ('deleted_99@anonymized.local', 'h', 'Silinmiş Kullanıcı', 'deleted', 'Sektör Kuruluşları', 'Anonim', '', 1, '', '2026-11-19 10:00:00', '127.0.0.1', 'h2', '2026-11-19 10:00:00', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO order_invoices (invoice_no, user_email, full_name, organization_name, item_type, item_code, item_title, amount_try, payment_status, created_at, pdf_watermark_code)
             VALUES ('INV-DEL', 'deleted_99@anonymized.local', 'Silinmiş Kullanıcı', 'Anonim', 'subscription', 'dijital', 'Dijital', 450.0, 'paid', '2026-11-19 10:00:00', 'WM-D')",
            [],
        )
        .unwrap();
        // Deleted user with non-anonymized email + orphaned anonymized invoice
        conn.execute(
            "INSERT INTO users (email, password_hash, full_name, role, organization_category, organization_name, job_title, is_gov_verified, subscription_tier, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash, created_at, email_verified)
             VALUES ('former_real@test.local', 'h', 'Silinmiş Gerçek', 'deleted', 'Sektör Kuruluşları', 'Anonim', '', 1, '', '2026-11-19 10:00:00', '127.0.0.1', 'h3', '2026-11-19 10:00:00', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO order_invoices (invoice_no, user_email, full_name, organization_name, item_type, item_code, item_title, amount_try, payment_status, created_at, pdf_watermark_code)
             VALUES ('INV-DEL-REAL', 'former_real@test.local', 'Silinmiş Gerçek', 'Anonim', 'subscription', 'dijital', 'Dijital', 450.0, 'paid', '2026-11-19 10:00:00', 'WM-DR')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO order_invoices (invoice_no, user_email, full_name, organization_name, item_type, item_code, item_title, amount_try, payment_status, created_at, pdf_watermark_code)
             VALUES ('INV-DEL-ORPHAN', 'deleted_100@anonymized.local', 'Silinmiş Yetim', 'Anonim', 'subscription', 'dijital', 'Dijital', 450.0, 'PAID_TEST', '2026-11-19 10:00:00', 'WM-DO')",
            [],
        )
        .unwrap();

        let funnel = get_conversion_funnel(&db).expect("funnel ok");
        assert_eq!(funnel["registered"], 1);
        assert_eq!(funnel["verified"], 1);
        assert_eq!(funnel["active_subscribers"], 1);
        assert_eq!(funnel["subscribed"], 1);
        assert_eq!(funnel["paid_invoice_customers"], 0);
        assert_eq!(funnel["paid"], 0);
        assert_eq!(funnel["conversion_rates"]["active_subscriber_to_paid"], 0.0);
        assert_eq!(funnel["conversion_rates"]["verified_to_paid"], 0.0);
        assert_eq!(funnel["conversion_rates"]["subscribed_to_paid"], 0.0);

        // Now add a paid invoice for the active user: paid_invoice_customers must be 1 (still excluding deleted invoices)
        conn.execute(
            "INSERT INTO order_invoices (invoice_no, user_email, full_name, organization_name, item_type, item_code, item_title, amount_try, payment_status, created_at, pdf_watermark_code)
             VALUES ('INV-ACT', 'active@test.local', 'Active', 'Org1', 'subscription', 'dijital', 'Dijital', 450.0, 'paid', '2026-11-19 10:00:00', 'WM-A')",
            [],
        )
        .unwrap();
        drop(conn);

        let funnel_with_active_paid = get_conversion_funnel(&db).expect("funnel ok");
        assert_eq!(funnel_with_active_paid["paid_invoice_customers"], 1);
        assert_eq!(funnel_with_active_paid["paid"], 1);
        assert_eq!(funnel_with_active_paid["conversion_rates"]["active_subscriber_to_paid"], 100.0);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }
}
