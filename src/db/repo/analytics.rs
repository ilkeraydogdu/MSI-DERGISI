#[allow(unused_imports)]
use super::*;

impl Database {
    /// Returns analytics events with pagination and optional inclusive date-range filtering (`YYYY-MM-DD`).
    /// Uses `SUBSTR(created_at, 1, 10)` so both space-delimited (`YYYY-MM-DD HH:MM:SS`) and ISO-8601 (`YYYY-MM-DDTHH:MM:SSZ`) timestamps match accurately.
    pub fn get_analytics_events_paginated(
        &self,
        limit: u32,
        offset: u32,
        date_from: Option<&str>,
        date_to: Option<&str>,
    ) -> Vec<AnalyticsEvent> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let effective_limit = std::cmp::min(limit, 10_000);

        let mut conditions: Vec<String> = Vec::new();
        let mut param_idx = 1u32;
        let mut from_param: Option<String> = None;
        let mut to_param: Option<String> = None;

        if let Some(df) = date_from.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            conditions.push(format!("SUBSTR(created_at, 1, 10) >= SUBSTR(?{}, 1, 10)", param_idx));
            from_param = Some(df.to_string());
            param_idx += 1;
        }
        if let Some(dt) = date_to.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            conditions.push(format!("SUBSTR(created_at, 1, 10) <= SUBSTR(?{}, 1, 10)", param_idx));
            to_param = Some(dt.to_string());
            param_idx += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, session_id, user_id, organization_category, organization_name,
                    event_type, content_slug, ad_id, duration_sec, scroll_depth_pct,
                    video_completed, is_true_read, ssb_tag, created_at
             FROM analytics_events {} ORDER BY id DESC LIMIT ?{} OFFSET ?{}",
            where_clause,
            param_idx,
            param_idx + 1
        );

        let Ok(mut stmt) = conn.prepare(&sql) else {
            return Vec::new();
        };

        let mut dyn_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(ref f) = from_param {
            dyn_params.push(Box::new(f.clone()));
        }
        if let Some(ref t) = to_param {
            dyn_params.push(Box::new(t.clone()));
        }
        dyn_params.push(Box::new(effective_limit));
        dyn_params.push(Box::new(offset));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            dyn_params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |r| {
            Ok(AnalyticsEvent {
                id: r.get(0)?,
                session_id: r.get(1)?,
                user_id: r.get(2)?,
                organization_category: r.get(3)?,
                organization_name: r.get(4)?,
                event_type: r.get(5)?,
                content_slug: r.get(6)?,
                ad_id: r.get(7)?,
                duration_sec: r.get(8)?,
                scroll_depth_pct: r.get(9)?,
                video_completed: r.get::<_, i32>(10)? != 0,
                is_true_read: r.get::<_, i32>(11)? != 0,
                ssb_tag: r.get(12)?,
                created_at: r.get(13)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_all_analytics_events(&self) -> Vec<AnalyticsEvent> {
        self.get_analytics_events_paginated(2000, 0, None, None)
    }

    /// Returns total count of analytics events, optionally filtered by inclusive `YYYY-MM-DD` date range.
    pub fn count_analytics_events(&self, date_from: Option<&str>, date_to: Option<&str>) -> u64 {
        let Ok(conn) = self.open_conn() else {
            return 0;
        };

        let mut conditions: Vec<String> = Vec::new();
        let mut str_params: Vec<String> = Vec::new();

        if let Some(df) = date_from.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            conditions.push(format!(
                "SUBSTR(created_at, 1, 10) >= SUBSTR(?{}, 1, 10)",
                str_params.len() + 1
            ));
            str_params.push(df.to_string());
        }
        if let Some(dt) = date_to.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            conditions.push(format!(
                "SUBSTR(created_at, 1, 10) <= SUBSTR(?{}, 1, 10)",
                str_params.len() + 1
            ));
            str_params.push(dt.to_string());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT COUNT(*) FROM analytics_events {}", where_clause);
        let boxed: Vec<Box<dyn rusqlite::types::ToSql>> = str_params
            .iter()
            .map(|s| Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = boxed.iter().map(|p| p.as_ref()).collect();

        conn.query_row(&sql, rusqlite::params_from_iter(refs), |r| r.get::<_, i64>(0))
            .unwrap_or(0)
            .max(0) as u64
    }

    /// Aggregates raw `analytics_events` for a specific `YYYY-MM-DD` date into `analytics_daily_agg`.
    /// Uses `SUBSTR(created_at, 1, 10)` and `COALESCE` on composite PK dimensions (`content_slug`, `ad_id`, `organization_category`).
    pub fn aggregate_analytics_for_date(&self, date: &str) -> Result<usize, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let norm_date: String = date.trim().chars().take(10).collect();

        let sql = r#"
            INSERT OR REPLACE INTO analytics_daily_agg
                (date, content_slug, ad_id, organization_category, event_type,
                 event_count, unique_sessions, avg_duration_sec, avg_scroll_pct, true_read_count)
            SELECT
                ?1 AS date,
                COALESCE(content_slug, '') AS content_slug,
                COALESCE(ad_id, 0) AS ad_id,
                COALESCE(organization_category, '') AS organization_category,
                event_type,
                COUNT(*) AS event_count,
                COUNT(DISTINCT session_id) AS unique_sessions,
                COALESCE(AVG(duration_sec), 0) AS avg_duration_sec,
                COALESCE(AVG(scroll_depth_pct), 0) AS avg_scroll_pct,
                COALESCE(SUM(CASE WHEN is_true_read = 1 THEN 1 ELSE 0 END), 0) AS true_read_count
            FROM analytics_events
            WHERE SUBSTR(created_at, 1, 10) = ?1
            GROUP BY COALESCE(content_slug, ''), COALESCE(ad_id, 0), COALESCE(organization_category, ''), event_type
        "#;

        conn.execute(sql, rusqlite::params![norm_date])
            .map_err(|e| format!("Aggregation failed for {}: {}", date, e))
    }

    /// Returns aggregated daily analytics for an inclusive date range.
    pub fn get_daily_aggregated_analytics(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Vec<serde_json::Value> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let sql = r#"
            SELECT date, content_slug, ad_id, organization_category, event_type,
                   event_count, unique_sessions, avg_duration_sec, avg_scroll_pct, true_read_count
            FROM analytics_daily_agg
            WHERE date >= SUBSTR(?1, 1, 10) AND date <= SUBSTR(?2, 1, 10)
            ORDER BY date DESC, event_count DESC
        "#;
        let Ok(mut stmt) = conn.prepare(sql) else {
            return Vec::new();
        };
        let rows = stmt.query_map(rusqlite::params![date_from.trim(), date_to.trim()], |r| {
            let slug_raw: String = r.get(1)?;
            let ad_raw: i64 = r.get(2)?;
            let org_raw: String = r.get(3)?;
            Ok(serde_json::json!({
                "date": r.get::<_, String>(0)?,
                "content_slug": if slug_raw.is_empty() { None } else { Some(slug_raw) },
                "ad_id": if ad_raw == 0 { None } else { Some(ad_raw) },
                "organization_category": org_raw,
                "event_type": r.get::<_, String>(4)?,
                "event_count": r.get::<_, i64>(5)?,
                "unique_sessions": r.get::<_, i64>(6)?,
                "avg_duration_sec": r.get::<_, f64>(7)?,
                "avg_scroll_pct": r.get::<_, f64>(8)?,
                "true_read_count": r.get::<_, i64>(9)?,
            }))
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn insert_analytics_event(&self, ev: &AnalyticsEvent) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO analytics_events (
                session_id, user_id, organization_category, organization_name,
                event_type, content_slug, ad_id, duration_sec, scroll_depth_pct,
                video_completed, is_true_read, ssb_tag, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                ev.session_id,
                ev.user_id,
                ev.organization_category,
                ev.organization_name,
                ev.event_type,
                ev.content_slug,
                ev.ad_id,
                ev.duration_sec,
                ev.scroll_depth_pct,
                if ev.video_completed { 1 } else { 0 },
                if ev.is_true_read { 1 } else { 0 },
                ev.ssb_tag,
                ev.created_at
            ],
        )?;

        if let Some(ref slug) = ev.content_slug {
            if matches!(
                ev.event_type.as_str(),
                "reading_depth" | "article_view" | "article_read"
            ) {
                self.increment_content_view(slug, ev.is_true_read);
            }
        }
        if let Some(ad_id) = ev.ad_id {
            if ev.event_type == "ad_click" {
                self.increment_ad_metric(ad_id, true);
            } else if ev.event_type == "ad_impression" {
                self.increment_ad_metric(ad_id, false);
            }
        }

        Ok(conn.last_insert_rowid())
    }

}
