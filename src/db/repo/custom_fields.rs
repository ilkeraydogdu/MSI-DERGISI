#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_custom_fields(&self, applies_to: &str) -> Vec<serde_json::Value> {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return vec![] };
        let mut stmt = match conn.prepare("SELECT id, field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to FROM custom_fields WHERE applies_to = ?1 ORDER BY sort_order") { Ok(s) => s, Err(_) => return vec![] };
        let rows = match stmt.query_map(rusqlite::params![applies_to], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "field_key": r.get::<_, String>(1)?,
                "label_tr": r.get::<_, String>(2)?, "label_en": r.get::<_, String>(3)?,
                "field_type": r.get::<_, String>(4)?, "options_json": r.get::<_, String>(5)?,
                "is_required": r.get::<_, bool>(6)?, "sort_order": r.get::<_, i64>(7)?,
                "applies_to": r.get::<_, String>(8)?
            }))
        }) { Ok(r) => r, Err(_) => return vec![] };
        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn get_custom_fields_paginated(&self, applies_to: &str, limit: u32, offset: u32) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() { Ok(c) => c, Err(_) => return (vec![], 0) };
        let filter_all = applies_to.trim().is_empty() || applies_to.eq_ignore_ascii_case("all");
        let total: u64 = if filter_all {
            conn.query_row("SELECT COUNT(*) FROM custom_fields", [], |r| r.get(0)).unwrap_or(0)
        } else {
            conn.query_row("SELECT COUNT(*) FROM custom_fields WHERE applies_to = ?1", rusqlite::params![applies_to], |r| r.get(0)).unwrap_or(0)
        };
        let sql = if filter_all {
            "SELECT id, field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to FROM custom_fields ORDER BY sort_order LIMIT ?1 OFFSET ?2"
        } else {
            "SELECT id, field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to FROM custom_fields WHERE applies_to = ?1 ORDER BY sort_order LIMIT ?2 OFFSET ?3"
        };
        let mut stmt = match conn.prepare(sql) { Ok(s) => s, Err(_) => return (vec![], total) };
        let map_row = |r: &rusqlite::Row<'_>| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?, "field_key": r.get::<_, String>(1)?,
                "label_tr": r.get::<_, String>(2)?, "label_en": r.get::<_, String>(3)?,
                "field_type": r.get::<_, String>(4)?, "options_json": r.get::<_, String>(5)?,
                "is_required": r.get::<_, bool>(6)?, "sort_order": r.get::<_, i64>(7)?,
                "applies_to": r.get::<_, String>(8)?
            }))
        };
        let rows: Vec<serde_json::Value> = if filter_all {
            stmt.query_map(rusqlite::params![limit, offset], map_row).ok().into_iter().flatten().filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map(rusqlite::params![applies_to, limit, offset], map_row).ok().into_iter().flatten().filter_map(|r| r.ok()).collect()
        };
        (rows, total)
    }

    pub fn upsert_custom_field(&self, field_key: &str, label_tr: &str, label_en: &str, field_type: &str, options_json: &str, is_required: bool, sort_order: i64, applies_to: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT INTO custom_fields (field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(field_key) DO UPDATE SET label_tr=?2, label_en=?3, field_type=?4, options_json=?5, is_required=?6, sort_order=?7, applies_to=?8",
            rusqlite::params![field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to])?;
        Ok(())
    }

    pub fn delete_custom_field(&self, field_key: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute("DELETE FROM custom_fields WHERE field_key = ?1", rusqlite::params![field_key])?;
        Ok(())
    }

    pub fn get_custom_field_values(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Vec<serde_json::Value> {
        let et = entity_type.trim();
        let eid = entity_id.trim();
        if et.is_empty() || eid.is_empty() {
            return Vec::new();
        }
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT cf.id, cf.field_key, cf.label_tr, cf.label_en, cf.field_type,
                    cf.options_json, cf.is_required, cf.sort_order, cf.applies_to,
                    COALESCE(cfv.value_text, '') AS value_text,
                    COALESCE(cfv.updated_at, '') AS updated_at
             FROM custom_fields cf
             LEFT JOIN custom_field_values cfv
                    ON cfv.field_id = cf.id
                   AND cfv.entity_type = ?1
                   AND cfv.entity_id = ?2
             WHERE cf.applies_to = ?1
             ORDER BY cf.sort_order ASC, cf.id ASC",
        ) else {
            return Vec::new();
        };
        let rows = match stmt.query_map(rusqlite::params![et, eid], |r| {
            Ok(serde_json::json!({
                "field_id": r.get::<_, i64>(0)?,
                "field_key": r.get::<_, String>(1)?,
                "label_tr": r.get::<_, String>(2)?,
                "label_en": r.get::<_, String>(3)?,
                "field_type": r.get::<_, String>(4)?,
                "options_json": r.get::<_, String>(5)?,
                "is_required": r.get::<_, bool>(6)?,
                "sort_order": r.get::<_, i64>(7)?,
                "applies_to": r.get::<_, String>(8)?,
                "entity_type": et,
                "entity_id": eid,
                "value_text": r.get::<_, String>(9)?,
                "updated_at": r.get::<_, String>(10)?,
            }))
        }) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        rows.filter_map(Result::ok).collect()
    }

    pub fn upsert_custom_field_values(
        &self,
        entity_type: &str,
        entity_id: &str,
        items: &[(i64, String)],
    ) -> Result<usize, String> {
        let et = entity_type.trim();
        let eid = entity_id.trim();
        if !matches!(et, "content" | "ad" | "user") {
            return Err("Geçersiz entity_type (content, ad veya user olmalıdır).".to_string());
        }
        if eid.is_empty() {
            return Err("entity_id boş olamaz.".to_string());
        }

        let mut conn = self
            .open_conn()
            .map_err(|e| format!("DB bağlantı hatası: {}", e))?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Transaction başlatılamadı: {}", e))?;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        let mut updated = 0usize;
        for (field_id, value_text) in items {
            let applies_to: Option<String> = tx
                .query_row(
                    "SELECT applies_to FROM custom_fields WHERE id = ?1",
                    rusqlite::params![field_id],
                    |r| r.get(0),
                )
                .ok();
            match applies_to {
                None => {
                    return Err(format!("Tanımsız custom field id: {}", field_id));
                }
                Some(ref target) if target.trim() != et => {
                    return Err(format!(
                        "Özel alan (id={}) '{}' için tanımlı değil (hedef: '{}').",
                        field_id, et, target
                    ));
                }
                Some(_) => {}
            }

            tx.execute(
                "INSERT INTO custom_field_values (field_id, entity_type, entity_id, value_text, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(field_id, entity_type, entity_id) DO UPDATE SET
                     value_text = excluded.value_text,
                     updated_at = excluded.updated_at",
                rusqlite::params![field_id, et, eid, value_text, &now],
            )
            .map_err(|e| format!("Özel alan değeri kaydedilemedi: {}", e))?;
            updated += 1;
        }

        tx.commit()
            .map_err(|e| format!("Transaction commit hatası: {}", e))?;
        Ok(updated)
    }
}