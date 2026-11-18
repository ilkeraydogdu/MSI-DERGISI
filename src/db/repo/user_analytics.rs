#[allow(unused_imports)]
use super::*;

impl Database {
    // =========================================================================

    /// Returns cohort analysis data: users grouped by registration month with
    /// activity tracking across subsequent months (m1..m6).
    pub fn get_user_cohort_analysis(&self) -> Vec<serde_json::Value> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };

        let cohort_sql = "SELECT SUBSTR(created_at, 1, 7) AS cohort_month, COUNT(*) AS registered
            FROM users
            WHERE role != 'deleted' AND SUBSTR(created_at, 1, 10) >= date('now', '-12 months')
            GROUP BY cohort_month ORDER BY cohort_month";
        let Ok(mut cohort_stmt) = conn.prepare(cohort_sql) else {
            return Vec::new();
        };

        let cohorts: Vec<(String, i64)> = cohort_stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map(|iter| iter.filter_map(Result::ok).collect())
            .unwrap_or_default();

        let mut results = Vec::new();
        for (cohort_month, registered) in &cohorts {
            let mut row = serde_json::json!({
                "cohort_month": cohort_month,
                "registered": registered
            });

            for m in 1..=6 {
                let activity_sql = format!(
                    "SELECT COUNT(DISTINCT ae.user_id)
                     FROM analytics_events ae
                     JOIN users u ON ae.user_id = u.id
                     WHERE SUBSTR(u.created_at, 1, 7) = ?1
                       AND SUBSTR(ae.created_at, 1, 7) = SUBSTR(date(?1 || '-01', '+{} months'), 1, 7)",
                    m
                );
                let active: i64 = conn
                    .query_row(&activity_sql, rusqlite::params![cohort_month], |r| r.get(0))
                    .unwrap_or(0);
                row[format!("active_m{}", m)] = serde_json::json!(active);
            }
            results.push(row);
        }
        results
    }

    /// Returns monthly churn rate & subscriber expiry metrics, excluding free/standard/academic tiers.
    pub fn get_churn_rate(&self) -> serde_json::Value {
        let Ok(conn) = self.open_conn() else {
            return serde_json::json!({
                "monthly_churn_pct": 0.0,
                "total_subscribers": 0,
                "expired": 0,
                "churned_users": 0,
                "total_cohort": 0,
                "avg_lifetime_days": 0
            });
        };

        let total_subscribers: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users
                 WHERE role != 'deleted'
                   AND subscription_tier IS NOT NULL
                   AND TRIM(subscription_tier) != ''
                   AND LOWER(TRIM(subscription_tier)) NOT IN (
                       'free',
                       'standart kayıtlı okur',
                       'standart kayitli okur',
                       'akademik / öğrenci okur',
                       'akademik / ogrenci okur',
                       'akademik okur'
                   )",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let expired: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users
                 WHERE role != 'deleted'
                   AND subscription_tier IS NOT NULL
                   AND TRIM(subscription_tier) != ''
                   AND LOWER(TRIM(subscription_tier)) NOT IN (
                       'free',
                       'standart kayıtlı okur',
                       'standart kayitli okur',
                       'akademik / öğrenci okur',
                       'akademik / ogrenci okur',
                       'akademik okur'
                   )
                   AND subscription_expires_at IS NOT NULL
                   AND SUBSTR(subscription_expires_at, 1, 10) < date('now')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let churn_sql = "SELECT
            COUNT(*) AS total_cohort,
            COALESCE(SUM(CASE WHEN NOT EXISTS (
                SELECT 1 FROM analytics_events ae
                WHERE ae.user_id = u.id
                  AND SUBSTR(ae.created_at, 1, 10) >= date(SUBSTR(u.created_at, 1, 10), '+1 month')
                  AND SUBSTR(ae.created_at, 1, 10) < date(SUBSTR(u.created_at, 1, 10), '+2 months')
            ) THEN 1 ELSE 0 END), 0) AS churned
            FROM users u
            WHERE u.role != 'deleted'
              AND SUBSTR(u.created_at, 1, 10) < date('now', '-2 months')";

        let (total_cohort, churned): (i64, i64) = conn
            .query_row(churn_sql, [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap_or((0, 0));

        let churn_pct = if total_subscribers > 0 {
            (expired as f64 / total_subscribers as f64) * 100.0
        } else if total_cohort > 0 {
            (churned as f64 / total_cohort as f64) * 100.0
        } else {
            0.0
        };

        let lifetime_sql = "SELECT COALESCE(AVG(julianday(SUBSTR(last_activity, 1, 10)) - julianday(SUBSTR(created_at, 1, 10))), 0)
            FROM (
                SELECT u.created_at, MAX(ae.created_at) AS last_activity
                FROM users u
                JOIN analytics_events ae ON ae.user_id = u.id
                WHERE u.role != 'deleted'
                GROUP BY u.id
            )";
        let avg_lifetime: f64 = conn
            .query_row(lifetime_sql, [], |r| r.get::<_, f64>(0))
            .unwrap_or(0.0_f64)
            .max(0.0_f64);

        serde_json::json!({
            "monthly_churn_pct": (churn_pct * 100.0).round() / 100.0,
            "total_subscribers": total_subscribers,
            "expired": expired,
            "churned_users": churned,
            "total_cohort": total_cohort,
            "avg_lifetime_days": avg_lifetime.round() as i64
        })
    }

    /// Returns user Lifetime Value (LTV) based on paid invoices.
    pub fn get_user_ltv(&self) -> serde_json::Value {
        let Ok(conn) = self.open_conn() else {
            return serde_json::json!({"avg_ltv_try": 0.0, "by_item_type": {}, "paying_users": 0});
        };

        let avg_ltv: f64 = conn
            .query_row(
                "SELECT COALESCE(AVG(user_total), 0) FROM (
                    SELECT LOWER(user_email) AS email, SUM(amount_try) AS user_total
                    FROM order_invoices WHERE payment_status = 'paid'
                    GROUP BY LOWER(user_email)
                )",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        let paying_users: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT LOWER(user_email)) FROM order_invoices WHERE payment_status = 'paid'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let type_sql = "SELECT item_type, AVG(user_total) FROM (
                SELECT LOWER(user_email) AS email, item_type, SUM(amount_try) AS user_total
                FROM order_invoices WHERE payment_status = 'paid'
                GROUP BY LOWER(user_email), item_type
            ) GROUP BY item_type";
        let by_type: serde_json::Map<String, serde_json::Value> = match conn.prepare(type_sql) {
            Ok(mut stmt) => stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))
                .map(|iter| {
                    iter.filter_map(Result::ok)
                        .map(|(k, v)| (k, serde_json::json!((v * 100.0).round() / 100.0)))
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => serde_json::Map::new(),
        };

        serde_json::json!({
            "avg_ltv_try": (avg_ltv * 100.0).round() / 100.0,
            "by_item_type": by_type,
            "paying_users": paying_users
        })
    }

    /// Returns conversion funnel: visitors -> registered -> subscribed -> paid,
    /// excluding `'free'`, `'Standart Kayıtlı Okur'`, and `'Akademik / Öğrenci Okur'` from subscriber counts.
    pub fn get_conversion_funnel(&self) -> serde_json::Value {
        let Ok(conn) = self.open_conn() else {
            return serde_json::json!({"visitors": 0, "registered": 0, "subscribed": 0, "paid": 0});
        };

        let visitors: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT session_id) FROM analytics_events",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let registered: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE role != 'deleted'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let paid: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT LOWER(user_email)) FROM order_invoices WHERE payment_status = 'paid'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let subscribed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users
                 WHERE role != 'deleted'
                   AND (
                       (
                           subscription_tier IS NOT NULL
                           AND TRIM(subscription_tier) != ''
                           AND LOWER(TRIM(subscription_tier)) NOT IN (
                               'free',
                               'standart kayıtlı okur',
                               'standart kayitli okur',
                               'akademik / öğrenci okur',
                               'akademik / ogrenci okur',
                               'akademik okur',
                               'kurumsal tam yetki'
                           )
                       )
                       OR LOWER(email) IN (
                           SELECT LOWER(user_email) FROM order_invoices
                           WHERE payment_status = 'paid' AND item_type IN ('subscription', 'subscription_plan')
                       )
                   )",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        serde_json::json!({
            "visitors": visitors,
            "registered": registered,
            "subscribed": subscribed,
            "paid": paid,
            "conversion_rates": {
                "visitor_to_registered": if visitors > 0 { ((registered as f64 / visitors as f64) * 10000.0).round() / 100.0 } else { 0.0 },
                "registered_to_subscribed": if registered > 0 { ((subscribed as f64 / registered as f64) * 10000.0).round() / 100.0 } else { 0.0 },
                "subscribed_to_paid": if subscribed > 0 { ((paid as f64 / subscribed as f64) * 10000.0).round() / 100.0 } else { 0.0 }
            }
        })
    }

}
