#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_system_health(&self) -> serde_json::Value {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(e) => return serde_json::json!({"status": "degraded", "db_ok": false, "message": e.to_string()}),
        };
        let integrity_ok = conn
            .query_row("PRAGMA quick_check(1)", [], |r| r.get::<_, String>(0))
            .map(|s| s.eq_ignore_ascii_case("ok"))
            .unwrap_or(false);
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap_or_else(|_| "unknown".to_string());
        let content_count: i64 = conn.query_row("SELECT COUNT(*) FROM contents", [], |r| r.get(0)).unwrap_or(0);
        let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap_or(0);
        let event_count: i64 = conn.query_row("SELECT COUNT(*) FROM analytics_events", [], |r| r.get(0)).unwrap_or(0);
        let ad_count: i64 = conn.query_row("SELECT COUNT(*) FROM ad_campaigns", [], |r| r.get(0)).unwrap_or(0);
        let category_count: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0)).unwrap_or(0);
        let issue_count: i64 = conn.query_row("SELECT COUNT(*) FROM magazine_issues", [], |r| r.get(0)).unwrap_or(0);
        let order_count: i64 = conn.query_row("SELECT COUNT(*) FROM order_invoices", [], |r| r.get(0)).unwrap_or(0);
        let db_size: i64 = conn.query_row("SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()", [], |r| r.get(0)).unwrap_or(0);
        let l1_cached = self.l1_cache.read().map(|g| g.is_some()).unwrap_or(false);
        let mem = crate::engine::monitoring_engine::read_memory_stats();
        let disk = crate::engine::monitoring_engine::read_disk_stats();
        let mem_used_mb = mem.total_kb.saturating_sub(mem.available_kb) / 1024;
        let mem_total_mb = mem.total_kb / 1024;
        let disk_used_gb = ((disk.used_bytes as f64) / 1_073_741_824.0 * 100.0).round() / 100.0;
        let disk_total_gb = ((disk.total_bytes as f64) / 1_073_741_824.0 * 100.0).round() / 100.0;
        let status = if !integrity_ok || disk.used_pct >= 98.0 {
            "degraded"
        } else {
            "healthy"
        };
        serde_json::json!({
            "status": status,
            "db_ok": integrity_ok,
            "journal_mode": journal_mode,
            "l1_cache_warm": l1_cached,
            "db_size_bytes": db_size,
            "memory": {
                "used_mb": mem_used_mb,
                "total_mb": mem_total_mb,
                "used_pct": mem.used_pct
            },
            "disk": {
                "used_gb": disk_used_gb,
                "total_gb": disk_total_gb,
                "used_pct": disk.used_pct
            },
            "tables": {
                "contents": content_count,
                "users": user_count,
                "analytics_events": event_count,
                "ads": ad_count,
                "categories": category_count,
                "issues": issue_count,
                "orders": order_count
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }

    pub fn get_distinct_source_groups(&self) -> Vec<(String, String, String)> {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut stmt = match conn.prepare(
            "SELECT DISTINCT source_group FROM contents WHERE source_group IS NOT NULL AND source_group != '' ORDER BY source_group",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let codes: Vec<String> = match stmt.query_map([], |r| r.get::<_, String>(0)) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => return vec![],
        };
        let settings = self.load_site_settings_db();
        codes
            .into_iter()
            .map(|code| {
                let tr_key = format!("source_label.{}.tr", code);
                let en_key = format!("source_label.{}.en", code);
                let label_tr = settings
                    .iter()
                    .find(|s| s.key == tr_key)
                    .map(|s| s.value_tr.clone())
                    .unwrap_or_else(|| code.clone());
                let label_en = settings
                    .iter()
                    .find(|s| s.key == en_key)
                    .map(|s| s.value_en.clone())
                    .unwrap_or_else(|| code.clone());
                (code, label_tr, label_en)
            })
            .collect()
    }
}