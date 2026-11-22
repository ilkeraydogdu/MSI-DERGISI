#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_seo_redirects(&self) -> Vec<SeoRedirectRule> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.seo_redirects.clone();
            }
        }
        self.load_seo_redirects_db()
    }

    pub fn get_seo_redirects_paginated(&self, limit: u32, offset: u32) -> (Vec<SeoRedirectRule>, u64) {
        let Ok(conn) = self.open_conn() else {
            return (Vec::new(), 0);
        };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM seo_redirects", [], |r| r.get(0)).unwrap_or(0);
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, old_path, new_path, status_code, hit_count FROM seo_redirects ORDER BY id DESC LIMIT ?1 OFFSET ?2",
        ) else {
            return (Vec::new(), total);
        };
        let rows = stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(SeoRedirectRule {
                id: r.get(0)?,
                old_path: r.get(1)?,
                new_path: r.get(2)?,
                status_code: r.get::<_, u16>(3)?,
                hit_count: r.get(4)?,
            })
        });
        match rows {
            Ok(iter) => (iter.filter_map(Result::ok).collect(), total),
            Err(_) => (Vec::new(), total),
        }
    }

    pub(super) fn load_seo_redirects_db(&self) -> Vec<SeoRedirectRule> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT id, old_path, new_path, status_code, hit_count
             FROM seo_redirects ORDER BY id DESC",
            [],
            |r| {
                Ok(SeoRedirectRule {
                    id: r.get(0)?,
                    old_path: r.get(1)?,
                    new_path: r.get(2)?,
                    status_code: r.get::<_, u16>(3)?,
                    hit_count: r.get(4)?,
                })
            },
        )
    }

    pub fn find_seo_redirect(&self, old_path: &str) -> Option<SeoRedirectRule> {
        let rule = self
            .get_seo_redirects()
            .into_iter()
            .find(|r| r.old_path.eq_ignore_ascii_case(old_path))?;
        if let Ok(conn) = self.open_conn() {
            let _ = conn.execute(
                "UPDATE seo_redirects SET hit_count = hit_count + 1 WHERE id = ?1",
                params![rule.id],
            );
        }
        Some(rule)
    }

    pub fn is_safe_seo_redirect_target(new_path: &str) -> bool {
        let t = new_path.trim();
        !t.is_empty()
            && t.starts_with('/')
            && !t.starts_with("//")
            && !t.contains('\\')
            && !t.chars().any(|c| c.is_control())
    }

    pub fn is_protected_admin_redirect_source(
        old_path: &str,
        active_admin_seg: Option<&str>,
    ) -> bool {
        let t = old_path.trim();
        if t.is_empty() || !t.starts_with('/') {
            return false;
        }
        let without_query = t.split(['?', '#']).next().unwrap_or(t);
        let decoded = urlencoding::decode(without_query)
            .unwrap_or(std::borrow::Cow::Borrowed(without_query));
        let first_raw = without_query
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("")
            .trim();
        let first_dec = decoded
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("")
            .trim();
        let matches_admin_seg = |seg: &str| -> bool {
            if seg.is_empty() {
                return false;
            }
            if crate::config::is_retired_admin_segment(seg) || seg.eq_ignore_ascii_case("podmin") {
                return true;
            }
            if let Some(active) = active_admin_seg {
                let norm_active = crate::config::normalize_admin_path_secret(active);
                if seg.eq_ignore_ascii_case(&norm_active) {
                    return true;
                }
            }
            false
        };
        matches_admin_seg(first_raw)
            || matches_admin_seg(first_dec)
            || crate::config::is_retired_admin_segment(without_query)
            || crate::config::is_retired_admin_segment(&decoded)
    }

    pub fn insert_seo_redirect(&self, rd: &SeoRedirectRule) -> rusqlite::Result<i64> {
        let old_p = rd.old_path.trim();
        let new_p = rd.new_path.trim();
        if old_p.is_empty()
            || !old_p.starts_with('/')
            || old_p.starts_with("//")
            || old_p.contains('\\')
            || old_p.chars().any(|c| c.is_control())
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Geçersiz kaynak yönlendirme yolu.".to_string(),
            ));
        }
        if !Self::is_safe_seo_redirect_target(new_p) {
            return Err(rusqlite::Error::InvalidParameterName(
                "Hedef yol yalnızca güvenli site-içi göreli bir yol ('/' ile başlayan) olabilir."
                    .to_string(),
            ));
        }
        if Self::is_protected_admin_redirect_source(old_p, None) {
            return Err(rusqlite::Error::InvalidParameterName(
                "Yönetim veya kullanımdan kaldırılmış admin rota adresleri yönlendirilemez."
                    .to_string(),
            ));
        }
        if old_p.eq_ignore_ascii_case(new_p) {
            return Err(rusqlite::Error::InvalidParameterName("Kaynak ve hedef yol aynı olamaz (yönlendirme döngüsü).".to_string()));
        }
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO seo_redirects (old_path, new_path, status_code, hit_count)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(old_path) DO UPDATE SET
                new_path = excluded.new_path,
                status_code = excluded.status_code",
            params![old_p, new_p, rd.status_code, rd.hit_count],
        )?;
        let rowid = conn
            .query_row("SELECT id FROM seo_redirects WHERE old_path = ?1", params![old_p], |r| r.get(0))
            .unwrap_or_else(|_| conn.last_insert_rowid());
        drop(conn);
        self.refresh_l1_sections(&["seo_redirects"]);
        Ok(rowid)
    }

    pub fn delete_seo_redirect(&self, id: i64) -> rusqlite::Result<bool> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let deleted = conn.execute("DELETE FROM seo_redirects WHERE id = ?1", params![id])?;
        drop(conn);
        if deleted > 0 {
            self.refresh_l1_sections(&["seo_redirects"]);
        }
        Ok(deleted > 0)
    }

    // =========================================================================
    // CENTRAL SITE SETTINGS & THEME CONFIGURATION ENGINE
}
