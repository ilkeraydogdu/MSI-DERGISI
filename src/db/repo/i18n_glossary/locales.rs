use super::*;
use rusqlite::OptionalExtension;

impl Database {
    // =========================================================================
    // 1. LOCALES REGISTRY CRUD
    // =========================================================================

    pub fn list_locales(&self) -> Vec<LocaleConfig> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT code, name_native, name_en, hreflang, og_locale, dir, is_default, is_enabled, fallback_code, sort_order
             FROM locales ORDER BY sort_order ASC, code ASC",
            [],
            |r| {
                Ok(LocaleConfig {
                    code: r.get(0)?,
                    name_native: r.get(1)?,
                    name_en: r.get(2)?,
                    hreflang: r.get(3)?,
                    og_locale: r.get(4)?,
                    dir: r.get(5)?,
                    is_default: r.get::<_, i32>(6)? != 0,
                    is_enabled: r.get::<_, i32>(7)? != 0,
                    fallback_code: r.get(8)?,
                    sort_order: r.get(9)?,
                })
            },
        )
    }

    pub fn list_enabled_locales(&self) -> Vec<LocaleConfig> {
        self.list_locales()
            .into_iter()
            .filter(|l| l.is_enabled)
            .collect()
    }

    pub fn upsert_locale(&self, loc: &LocaleConfig) -> Result<(), String> {
        let code = loc.code.trim().to_ascii_lowercase();
        if code.is_empty() || code.len() > 16 {
            return Err("Geçersiz dil kodu.".to_string());
        }
        let dir = if loc.dir.trim().eq_ignore_ascii_case("rtl") {
            "rtl"
        } else {
            "ltr"
        };
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        if loc.is_default {
            tx.execute("UPDATE locales SET is_default = 0 WHERE code != ?1", params![code])
                .map_err(|e| e.to_string())?;
        }
        let enabled_val = if loc.is_default || loc.is_enabled { 1 } else { 0 };
        tx.execute(
            "INSERT INTO locales (code, name_native, name_en, hreflang, og_locale, dir, is_default, is_enabled, fallback_code, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(code) DO UPDATE SET
                 name_native = excluded.name_native,
                 name_en = excluded.name_en,
                 hreflang = excluded.hreflang,
                 og_locale = excluded.og_locale,
                 dir = excluded.dir,
                 is_default = excluded.is_default,
                 is_enabled = excluded.is_enabled,
                 fallback_code = excluded.fallback_code,
                 sort_order = excluded.sort_order",
            params![
                code,
                loc.name_native.trim(),
                loc.name_en.trim(),
                loc.hreflang.trim(),
                loc.og_locale.trim(),
                dir,
                if loc.is_default { 1 } else { 0 },
                enabled_val,
                if loc.fallback_code.trim().is_empty() { "tr" } else { loc.fallback_code.trim() },
                loc.sort_order
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_locale(&self, code: &str) -> Result<bool, String> {
        let norm = code.trim().to_ascii_lowercase();
        if norm.is_empty() {
            return Err("Dil kodu boş olamaz.".to_string());
        }
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let is_default_opt: Option<i32> = conn
            .query_row(
                "SELECT is_default FROM locales WHERE code = ?1",
                params![norm],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some(is_default) = is_default_opt else {
            return Ok(false);
        };
        if is_default == 1 {
            return Err("Varsayılan dil silinemez.".to_string());
        }
        let affected = conn
            .execute("DELETE FROM locales WHERE code = ?1", params![norm])
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    pub fn set_locale_enabled(&self, code: &str, enabled: bool) -> Result<bool, String> {
        let norm = code.trim().to_ascii_lowercase();
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let is_default: i32 = conn
            .query_row(
                "SELECT is_default FROM locales WHERE code = ?1",
                params![norm],
                |r| r.get(0),
            )
            .map_err(|_| "Dil kaydı bulunamadı.".to_string())?;
        if is_default == 1 && !enabled {
            return Err("Varsayılan dil devre dışı bırakılamaz.".to_string());
        }
        let affected = conn
            .execute(
                "UPDATE locales SET is_enabled = ?1 WHERE code = ?2",
                params![if enabled { 1 } else { 0 }, norm],
            )
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

}
