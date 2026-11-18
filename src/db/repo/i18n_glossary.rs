#[allow(unused_imports)]
use super::*;

pub use crate::models::category::{
    EmailGroup, GovEmailDomain, InstitutionCategory, LocaleConfig, TranslationGlossaryTerm,
};

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

    // =========================================================================
    // 2. TRANSLATION GLOSSARY CRUD & VALIDATOR
    // =========================================================================

    pub fn list_glossary_terms(
        &self,
        domain_filter: Option<&str>,
        active_only: bool,
    ) -> Vec<TranslationGlossaryTerm> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let dom = domain_filter.map(str::trim).filter(|s| !s.is_empty());
        let mut clauses = Vec::new();
        let mut p: Vec<String> = Vec::new();
        if active_only {
            clauses.push("is_active = 1".to_string());
        }
        if let Some(d) = dom {
            p.push(d.to_string());
            clauses.push(format!("LOWER(domain) = LOWER(?{})", p.len()));
        }
        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT id, term_tr, term_en, abbreviation_tr, abbreviation_en, domain, rule_type,
                    preferred_en, forbidden_variants_json, case_sensitive, priority, source_ref,
                    notes, is_active, created_by, updated_at
             FROM translation_glossary
             {}
             ORDER BY priority DESC, term_tr ASC",
            where_sql
        );
        let Ok(mut stmt) = conn.prepare(&sql) else {
            return Vec::new();
        };
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            p.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let rows = stmt.query_map(rusqlite::params_from_iter(params_refs), |r| {
            let forb_raw: String = r.get(8)?;
            Ok(TranslationGlossaryTerm {
                id: r.get(0)?,
                term_tr: r.get(1)?,
                term_en: r.get(2)?,
                abbreviation_tr: r.get(3)?,
                abbreviation_en: r.get(4)?,
                domain: r.get(5)?,
                rule_type: r.get(6)?,
                preferred_en: r.get(7)?,
                forbidden_variants: serde_json::from_str(&forb_raw).unwrap_or_default(),
                case_sensitive: r.get::<_, i32>(9)? != 0,
                priority: r.get(10)?,
                source_ref: r.get(11)?,
                notes: r.get(12)?,
                is_active: r.get::<_, i32>(13)? != 0,
                created_by: r.get(14)?,
                updated_at: r.get(15)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn upsert_glossary_term(&self, term: &TranslationGlossaryTerm) -> Result<i64, String> {
        let term_tr = term.term_tr.trim();
        let preferred_en = term.preferred_en.trim();
        let domain = if term.domain.trim().is_empty() {
            "editorial"
        } else {
            term.domain.trim()
        };
        let rule_type = match term.rule_type.trim() {
            "preserve" => "preserve",
            "forbidden" => "forbidden",
            _ => "fixed",
        };
        if term_tr.is_empty() || preferred_en.is_empty() {
            return Err("Türkçe terim ve tercih edilen İngilizce karşılık zorunludur.".to_string());
        }
        let term_en = if term.term_en.trim().is_empty() {
            preferred_en
        } else {
            term.term_en.trim()
        };
        let forb_json = serde_json::to_string(&term.forbidden_variants).unwrap_or_else(|_| "[]".to_string());
        let now = chrono::Utc::now().to_rfc3339();
        let created_by = if term.created_by.trim().is_empty() {
            "admin"
        } else {
            term.created_by.trim()
        };
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        if term.id > 0 {
            conn.execute(
                "UPDATE translation_glossary SET
                    term_tr = ?1, term_en = ?2, abbreviation_tr = ?3, abbreviation_en = ?4,
                    domain = ?5, rule_type = ?6, preferred_en = ?7, forbidden_variants_json = ?8,
                    case_sensitive = ?9, priority = ?10, source_ref = ?11, notes = ?12,
                    is_active = ?13, updated_at = ?14
                 WHERE id = ?15",
                params![
                    term_tr,
                    term_en,
                    term.abbreviation_tr.trim(),
                    term.abbreviation_en.trim(),
                    domain,
                    rule_type,
                    preferred_en,
                    forb_json,
                    if term.case_sensitive { 1 } else { 0 },
                    term.priority,
                    term.source_ref.trim(),
                    term.notes.trim(),
                    if term.is_active { 1 } else { 0 },
                    now,
                    term.id
                ],
            )
            .map_err(|e| e.to_string())?;
            Ok(term.id)
        } else {
            conn.execute(
                "INSERT INTO translation_glossary (
                    term_tr, term_en, abbreviation_tr, abbreviation_en, domain, rule_type,
                    preferred_en, forbidden_variants_json, case_sensitive, priority, source_ref,
                    notes, is_active, created_by, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                 ON CONFLICT(term_tr, domain) DO UPDATE SET
                    term_en = excluded.term_en,
                    abbreviation_tr = excluded.abbreviation_tr,
                    abbreviation_en = excluded.abbreviation_en,
                    rule_type = excluded.rule_type,
                    preferred_en = excluded.preferred_en,
                    forbidden_variants_json = excluded.forbidden_variants_json,
                    case_sensitive = excluded.case_sensitive,
                    priority = excluded.priority,
                    source_ref = excluded.source_ref,
                    notes = excluded.notes,
                    is_active = excluded.is_active,
                    updated_at = excluded.updated_at",
                params![
                    term_tr,
                    term_en,
                    term.abbreviation_tr.trim(),
                    term.abbreviation_en.trim(),
                    domain,
                    rule_type,
                    preferred_en,
                    forb_json,
                    if term.case_sensitive { 1 } else { 0 },
                    term.priority,
                    term.source_ref.trim(),
                    term.notes.trim(),
                    if term.is_active { 1 } else { 0 },
                    created_by,
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
            let id = conn
                .query_row(
                    "SELECT id FROM translation_glossary WHERE term_tr = ?1 AND domain = ?2",
                    params![term_tr, domain],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| conn.last_insert_rowid());
            Ok(id)
        }
    }

    pub fn delete_glossary_term(&self, id: i64) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let affected = conn
            .execute("DELETE FROM translation_glossary WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    /// Scans English text against active glossary forbidden variants and returns structured QA hints.
    pub fn check_glossary_violations(&self, en_text: &str) -> Vec<serde_json::Value> {
        if en_text.trim().is_empty() {
            return Vec::new();
        }
        let lower_text = en_text.to_lowercase();
        let terms = self.list_glossary_terms(None, true);
        let mut hits = Vec::new();
        for t in terms {
            for variant in &t.forbidden_variants {
                let v = variant.trim();
                if v.is_empty() {
                    continue;
                }
                let matched = if t.case_sensitive {
                    en_text.contains(v)
                } else {
                    lower_text.contains(&v.to_lowercase())
                };
                if matched {
                    hits.push(serde_json::json!({
                        "glossary_id": t.id,
                        "domain": t.domain,
                        "term_tr": t.term_tr,
                        "forbidden": v,
                        "preferred_en": t.preferred_en,
                        "notes": t.notes
                    }));
                }
            }
        }
        hits
    }

    // =========================================================================
    // 3. DYNAMIC INSTITUTION CATEGORIES CRUD
    // =========================================================================

    pub fn list_institution_categories(&self, active_only: bool) -> Vec<InstitutionCategory> {
        let sql = if active_only {
            "SELECT code, legacy_value, label_tr, label_en, kind, is_public_sector, requires_verification, sort_order, is_active
             FROM institution_categories WHERE is_active = 1 ORDER BY sort_order ASC, code ASC"
        } else {
            "SELECT code, legacy_value, label_tr, label_en, kind, is_public_sector, requires_verification, sort_order, is_active
             FROM institution_categories ORDER BY sort_order ASC, code ASC"
        };
        crate::db::query_helpers::query_all(self, sql, [], |r| {
            Ok(InstitutionCategory {
                code: r.get(0)?,
                legacy_value: r.get(1)?,
                label_tr: r.get(2)?,
                label_en: r.get(3)?,
                kind: r.get(4)?,
                is_public_sector: r.get::<_, i32>(5)? != 0,
                requires_verification: r.get::<_, i32>(6)? != 0,
                sort_order: r.get(7)?,
                is_active: r.get::<_, i32>(8)? != 0,
            })
        })
    }

    pub fn resolve_institution_category(&self, code_or_legacy: &str) -> Option<InstitutionCategory> {
        let norm = code_or_legacy.trim();
        if norm.is_empty() {
            return None;
        }
        crate::db::query_helpers::query_one(
            self,
            "SELECT code, legacy_value, label_tr, label_en, kind, is_public_sector, requires_verification, sort_order, is_active
             FROM institution_categories
             WHERE LOWER(code) = LOWER(?1) OR LOWER(legacy_value) = LOWER(?1)
             LIMIT 1",
            params![norm],
            |r| {
                Ok(InstitutionCategory {
                    code: r.get(0)?,
                    legacy_value: r.get(1)?,
                    label_tr: r.get(2)?,
                    label_en: r.get(3)?,
                    kind: r.get(4)?,
                    is_public_sector: r.get::<_, i32>(5)? != 0,
                    requires_verification: r.get::<_, i32>(6)? != 0,
                    sort_order: r.get(7)?,
                    is_active: r.get::<_, i32>(8)? != 0,
                })
            },
        )
    }

    pub fn upsert_institution_category(&self, cat: &InstitutionCategory) -> Result<(), String> {
        let code = cat.code.trim().to_ascii_lowercase();
        let legacy = if cat.legacy_value.trim().is_empty() {
            cat.label_tr.trim()
        } else {
            cat.legacy_value.trim()
        };
        if code.is_empty() || cat.label_tr.trim().is_empty() {
            return Err("Kurum kodu ve Türkçe etiketi zorunludur.".to_string());
        }
        let label_en = if cat.label_en.trim().is_empty() {
            cat.label_tr.trim()
        } else {
            cat.label_en.trim()
        };
        let kind = if cat.kind.trim().is_empty() {
            if cat.is_public_sector { "public_sector" } else { "industry" }
        } else {
            cat.kind.trim()
        };
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO institution_categories (
                code, legacy_value, label_tr, label_en, kind, is_public_sector,
                requires_verification, sort_order, is_active
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(code) DO UPDATE SET
                legacy_value = excluded.legacy_value,
                label_tr = excluded.label_tr,
                label_en = excluded.label_en,
                kind = excluded.kind,
                is_public_sector = excluded.is_public_sector,
                requires_verification = excluded.requires_verification,
                sort_order = excluded.sort_order,
                is_active = excluded.is_active",
            params![
                code,
                legacy,
                cat.label_tr.trim(),
                label_en,
                kind,
                if cat.is_public_sector { 1 } else { 0 },
                if cat.requires_verification { 1 } else { 0 },
                cat.sort_order,
                if cat.is_active { 1 } else { 0 }
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_institution_category(&self, code: &str) -> Result<bool, String> {
        let norm = code.trim().to_ascii_lowercase();
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let legacy: Option<String> = conn
            .query_row(
                "SELECT legacy_value FROM institution_categories WHERE LOWER(code) = ?1",
                params![norm],
                |r| r.get(0),
            )
            .ok();
        let Some(legacy_val) = legacy else {
            return Ok(false);
        };
        let in_use: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE role != 'deleted' AND (LOWER(organization_category) = ?1 OR LOWER(organization_category) = LOWER(?2))",
                params![norm, legacy_val],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if in_use > 0 {
            return Err(format!(
                "Bu kurum kategorisi {} kullanıcıda aktif olduğu için silinemez; pasif yapabilirsiniz.",
                in_use
            ));
        }
        let affected = conn
            .execute("DELETE FROM institution_categories WHERE LOWER(code) = ?1", params![norm])
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    // =========================================================================
    // 4. GOV / ACADEMIC / PERSONAL EMAIL DOMAINS CRUD & CLASSIFIER
    // =========================================================================

    pub fn list_gov_email_domains(&self, active_only: bool) -> Vec<GovEmailDomain> {
        let sql = if active_only {
            "SELECT id, domain, match_mode, domain_kind, institution_code, grants_role, is_active, note
             FROM gov_email_domains WHERE is_active = 1 ORDER BY domain_kind ASC, LENGTH(domain) DESC, domain ASC"
        } else {
            "SELECT id, domain, match_mode, domain_kind, institution_code, grants_role, is_active, note
             FROM gov_email_domains ORDER BY domain_kind ASC, LENGTH(domain) DESC, domain ASC"
        };
        crate::db::query_helpers::query_all(self, sql, [], |r| {
            Ok(GovEmailDomain {
                id: r.get(0)?,
                domain: r.get(1)?,
                match_mode: r.get(2)?,
                domain_kind: r.get(3)?,
                institution_code: r.get(4)?,
                grants_role: r.get(5)?,
                is_active: r.get::<_, i32>(6)? != 0,
                note: r.get(7)?,
            })
        })
    }

    /// Matches an email address against active `gov_email_domains` rules (exact first, then longest suffix).
    pub fn classify_email_domain(&self, email: &str) -> Option<GovEmailDomain> {
        let host = email.trim().to_ascii_lowercase().split('@').nth(1)?.trim().to_string();
        if host.is_empty() {
            return None;
        }
        let rules = self.list_gov_email_domains(true);
        if let Some(exact) = rules
            .iter()
            .find(|r| r.match_mode == "exact" && host == r.domain.trim().to_ascii_lowercase())
        {
            return Some(exact.clone());
        }
        rules
            .into_iter()
            .filter(|r| r.match_mode == "suffix")
            .filter(|r| {
                let suffix = r.domain.trim().to_ascii_lowercase();
                let bare = suffix.trim_start_matches('.');
                host == bare || host.ends_with(&suffix)
            })
            .max_by_key(|r| r.domain.len())
    }

    pub fn upsert_gov_email_domain(&self, entry: &GovEmailDomain) -> Result<i64, String> {
        let domain = entry.domain.trim().to_ascii_lowercase();
        if domain.is_empty() {
            return Err("Alan adı boş olamaz.".to_string());
        }
        let match_mode = if entry.match_mode.trim() == "exact" { "exact" } else { "suffix" };
        let domain_kind = match entry.domain_kind.trim() {
            "academic" => "academic",
            "personal" => "personal",
            _ => "public_sector",
        };
        let grants_role = if entry.grants_role.trim().is_empty() {
            if domain_kind == "public_sector" { "subscriber_public" } else { "registered" }
        } else {
            entry.grants_role.trim()
        };
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        if entry.id > 0 {
            conn.execute(
                "UPDATE gov_email_domains SET
                    domain = ?1, match_mode = ?2, domain_kind = ?3, institution_code = ?4,
                    grants_role = ?5, is_active = ?6, note = ?7
                 WHERE id = ?8",
                params![
                    domain,
                    match_mode,
                    domain_kind,
                    entry.institution_code,
                    grants_role,
                    if entry.is_active { 1 } else { 0 },
                    entry.note.trim(),
                    entry.id
                ],
            )
            .map_err(|e| e.to_string())?;
            Ok(entry.id)
        } else {
            conn.execute(
                "INSERT INTO gov_email_domains (domain, match_mode, domain_kind, institution_code, grants_role, is_active, note)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(domain) DO UPDATE SET
                    match_mode = excluded.match_mode,
                    domain_kind = excluded.domain_kind,
                    institution_code = excluded.institution_code,
                    grants_role = excluded.grants_role,
                    is_active = excluded.is_active,
                    note = excluded.note",
                params![
                    domain,
                    match_mode,
                    domain_kind,
                    entry.institution_code,
                    grants_role,
                    if entry.is_active { 1 } else { 0 },
                    entry.note.trim()
                ],
            )
            .map_err(|e| e.to_string())?;
            let id = conn
                .query_row(
                    "SELECT id FROM gov_email_domains WHERE domain = ?1",
                    params![domain],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| conn.last_insert_rowid());
            Ok(id)
        }
    }

    pub fn delete_gov_email_domain(&self, id: i64) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let affected = conn
            .execute("DELETE FROM gov_email_domains WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    // =========================================================================
    // 5. SEGMENTED EMAIL GROUPS CRUD
    // =========================================================================

    pub fn list_email_groups(&self, active_only: bool) -> Vec<EmailGroup> {
        let sql = if active_only {
            "SELECT id, code, name_tr, name_en, description_tr, description_en, filter_json, static_emails_json,
                    requires_marketing_consent, is_system, is_active, sort_order, created_by, updated_at
             FROM email_groups WHERE is_active = 1 ORDER BY sort_order ASC, id ASC"
        } else {
            "SELECT id, code, name_tr, name_en, description_tr, description_en, filter_json, static_emails_json,
                    requires_marketing_consent, is_system, is_active, sort_order, created_by, updated_at
             FROM email_groups ORDER BY sort_order ASC, id ASC"
        };
        crate::db::query_helpers::query_all(self, sql, [], |r| {
            Ok(EmailGroup {
                id: r.get(0)?,
                code: r.get(1)?,
                name_tr: r.get(2)?,
                name_en: r.get(3)?,
                description_tr: r.get(4)?,
                description_en: r.get(5)?,
                filter_json: r.get(6)?,
                static_emails_json: r.get(7)?,
                requires_marketing_consent: r.get::<_, i32>(8)? != 0,
                is_system: r.get::<_, i32>(9)? != 0,
                is_active: r.get::<_, i32>(10)? != 0,
                sort_order: r.get(11)?,
                created_by: r.get(12)?,
                updated_at: r.get(13)?,
            })
        })
    }

    pub fn get_email_group_by_code(&self, code: &str) -> Option<EmailGroup> {
        let norm = code.trim().to_ascii_lowercase();
        self.list_email_groups(false)
            .into_iter()
            .find(|g| g.code.eq_ignore_ascii_case(&norm))
    }

    pub fn upsert_email_group(&self, grp: &EmailGroup) -> Result<i64, String> {
        let code = grp.code.trim().to_ascii_lowercase();
        if code.is_empty() || grp.name_tr.trim().is_empty() {
            return Err("Grup kodu ve Türkçe adı zorunludur.".to_string());
        }
        let filter_json = if grp.filter_json.trim().is_empty() {
            "{}".to_string()
        } else {
            serde_json::from_str::<serde_json::Value>(&grp.filter_json)
                .map_err(|_| "Geçersiz filter_json formatı.".to_string())?;
            grp.filter_json.trim().to_string()
        };
        let static_json = if grp.static_emails_json.trim().is_empty() {
            "[]".to_string()
        } else {
            serde_json::from_str::<Vec<String>>(&grp.static_emails_json)
                .map_err(|_| "Geçersiz static_emails_json formatı (JSON dizi olmalı).".to_string())?;
            grp.static_emails_json.trim().to_string()
        };
        let name_en = if grp.name_en.trim().is_empty() {
            grp.name_tr.trim()
        } else {
            grp.name_en.trim()
        };
        let now = chrono::Utc::now().to_rfc3339();
        let created_by = if grp.created_by.trim().is_empty() { "admin" } else { grp.created_by.trim() };
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO email_groups (
                code, name_tr, name_en, description_tr, description_en, filter_json, static_emails_json,
                requires_marketing_consent, is_system, is_active, sort_order, created_by, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10, ?11, ?12)
             ON CONFLICT(code) DO UPDATE SET
                name_tr = excluded.name_tr,
                name_en = excluded.name_en,
                description_tr = excluded.description_tr,
                description_en = excluded.description_en,
                filter_json = excluded.filter_json,
                static_emails_json = excluded.static_emails_json,
                requires_marketing_consent = excluded.requires_marketing_consent,
                is_active = excluded.is_active,
                sort_order = excluded.sort_order,
                updated_at = excluded.updated_at",
            params![
                code,
                grp.name_tr.trim(),
                name_en,
                grp.description_tr.trim(),
                grp.description_en.trim(),
                filter_json,
                static_json,
                if grp.requires_marketing_consent { 1 } else { 0 },
                if grp.is_active { 1 } else { 0 },
                grp.sort_order,
                created_by,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
        let id = conn
            .query_row("SELECT id FROM email_groups WHERE code = ?1", params![code], |r| r.get(0))
            .unwrap_or_else(|_| conn.last_insert_rowid());
        Ok(id)
    }

    pub fn delete_email_group(&self, code: &str) -> Result<bool, String> {
        let norm = code.trim().to_ascii_lowercase();
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let is_sys: i32 = conn
            .query_row(
                "SELECT is_system FROM email_groups WHERE LOWER(code) = ?1",
                params![norm],
                |r| r.get(0),
            )
            .map_err(|_| "E-posta grubu bulunamadı.".to_string())?;
        if is_sys == 1 {
            return Err("Sistem tanımlı e-posta grupları silinemez.".to_string());
        }
        let affected = conn
            .execute("DELETE FROM email_groups WHERE LOWER(code) = ?1 AND is_system = 0", params![norm])
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    /// Resolves matching active recipients for an `email_groups` code + optional institution/SSB filters.
    pub fn resolve_email_group_recipients(
        &self,
        group_code: &str,
        institution_filter: &str,
        ssb_tag_filter: &str,
    ) -> Vec<UserAccount> {
        let group_opt = self.get_email_group_by_code(group_code);
        let filter_val: serde_json::Value = group_opt
            .as_ref()
            .and_then(|g| serde_json::from_str(&g.filter_json).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let requires_consent = group_opt
            .as_ref()
            .map(|g| g.requires_marketing_consent)
            .unwrap_or(true);
        let static_emails: Vec<String> = group_opt
            .as_ref()
            .and_then(|g| serde_json::from_str(&g.static_emails_json).ok())
            .unwrap_or_default();

        let min_true_reads = filter_val
            .get("min_true_reads")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let active_user_read_counts: HashMap<i64, i64> = if min_true_reads > 0 {
            self.open_conn()
                .ok()
                .and_then(|conn| {
                    let mut stmt = conn
                        .prepare(
                            "SELECT user_id, COUNT(*) FROM analytics_events
                             WHERE user_id IS NOT NULL AND is_true_read = 1
                             GROUP BY user_id",
                        )
                        .ok()?;
                    let map: HashMap<i64, i64> = stmt
                        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
                        .ok()?
                        .filter_map(Result::ok)
                        .collect();
                    Some(map)
                })
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        let filter_cats: Vec<String> = filter_val
            .get("organization_categories")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.trim().to_ascii_lowercase()))
                    .collect()
            })
            .unwrap_or_default();

        let inst_tokens: Vec<String> = institution_filter
            .split(&['&', ','][..])
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty() && s != "*" && s != "all")
            .collect();
        let ssb_tokens: Vec<String> = ssb_tag_filter
            .split(&['&', ','][..])
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty() && s != "*" && s != "all")
            .collect();

        let norm_code = group_code.trim().to_ascii_lowercase();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        self.get_all_users()
            .into_iter()
            .filter(|u| {
                if u.role == "deleted" || u.email.trim().is_empty() {
                    return false;
                }
                if requires_consent && (!u.email_consent || !u.marketing_emails_opted_in) {
                    return false;
                }
                let in_static = static_emails
                    .iter()
                    .any(|e| e.trim().eq_ignore_ascii_case(&u.email));
                if in_static {
                    return true;
                }

                let group_ok = match norm_code.as_str() {
                    "gov_verified" | "public_sector" | "kamu" => u.is_gov_verified,
                    "email_verified" => u.email_verified,
                    "active_subscribers" | "subscribers" | "paid_subscribers" => {
                        let tier_lower = u.subscription_tier.trim().to_ascii_lowercase();
                        let paid_tier = !tier_lower.is_empty()
                            && !matches!(
                                tier_lower.as_str(),
                                "free"
                                    | "standart kayıtlı okur"
                                    | "standart kayitli okur"
                                    | "akademik / öğrenci okur"
                                    | "akademik / ogrenci okur"
                                    | "akademik okur"
                            );
                        let not_expired = u
                            .subscription_expires_at
                            .as_deref()
                            .map(|d| d.get(..10).unwrap_or(d) >= today.as_str())
                            .unwrap_or(true);
                        u.is_gov_verified || (paid_tier && not_expired)
                    }
                    _ => true,
                };
                if !group_ok {
                    return false;
                }
                if filter_val.get("is_gov_verified").and_then(|v| v.as_bool()) == Some(true)
                    && !u.is_gov_verified
                {
                    return false;
                }
                if filter_val.get("email_verified").and_then(|v| v.as_bool()) == Some(true)
                    && !u.email_verified
                {
                    return false;
                }
                if !filter_cats.is_empty()
                    && !filter_cats.iter().any(|c| {
                        u.organization_category.to_ascii_lowercase() == *c
                            || u.organization_name.to_ascii_lowercase() == *c
                    })
                {
                    return false;
                }
                if min_true_reads > 0
                    && active_user_read_counts.get(&u.id).copied().unwrap_or(0) < min_true_reads
                {
                    return false;
                }
                if !inst_tokens.is_empty()
                    && !inst_tokens.iter().any(|tok| {
                        u.organization_category.to_ascii_lowercase() == *tok
                            || u.organization_name.to_ascii_lowercase() == *tok
                    })
                {
                    return false;
                }
                if !ssb_tokens.is_empty()
                    && !ssb_tokens
                        .iter()
                        .any(|tok| u.ssb_interests.iter().any(|t| t.eq_ignore_ascii_case(tok)))
                {
                    return false;
                }
                true
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::api_response::msg_keys;
    use crate::engine::translation_engine::TranslationCache;
    use std::path::Path;

    fn temp_db_path(label: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("/tmp/msi_db_i18n_test_{}_{}_{}.db", label, std::process::id(), nanos)
    }

    fn cleanup_db(path: &str) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn migration_012_populates_i18n_glossary_b2b_and_translations_idempotently() {
        let path = temp_db_path("fresh_012");
        let db = Database::new(&path).expect("Database::new should succeed on fresh db");

        let locales = db.list_locales();
        assert_eq!(locales.len(), 6);
        assert_eq!(locales.iter().filter(|l| l.is_default).count(), 1);
        assert!(locales.iter().any(|l| l.code == "tr" && l.is_default && l.is_enabled));
        assert!(locales.iter().any(|l| l.code == "en" && !l.is_default && l.is_enabled));
        assert_eq!(db.list_enabled_locales().len(), 2);

        let glossary = db.list_glossary_terms(None, true);
        assert!(
            glossary.len() >= 200,
            "Expected >= 200 glossary terms, got {}",
            glossary.len()
        );
        let violations =
            db.check_glossary_violations("The Ministry of Defense in Turkey visited SSM.");
        assert!(violations.iter().any(|v| v["forbidden"] == "Ministry of Defense"));
        assert!(violations.iter().any(|v| v["forbidden"] == "Turkey"));
        assert!(violations.iter().any(|v| v["forbidden"] == "SSM"));

        let insts = db.list_institution_categories(true);
        assert_eq!(insts.len(), 14);
        assert!(db.resolve_institution_category("savunma_medya").is_some());
        assert!(db.resolve_institution_category("Sektör Kuruluşları").is_some());

        let pub_dom = db
            .classify_email_domain("subay@msb.gov.tr")
            .expect("msb.gov.tr should match");
        assert_eq!(pub_dom.domain_kind, "public_sector");
        assert_eq!(pub_dom.grants_role, "subscriber_public");
        assert_eq!(pub_dom.institution_code.as_deref(), Some("msb"));

        let edu_dom = db
            .classify_email_domain("ogrenci@metu.edu.tr")
            .expect("edu.tr should match");
        assert_eq!(edu_dom.domain_kind, "academic");

        let pers_dom = db
            .classify_email_domain("okur@gmail.com")
            .expect("gmail.com should match");
        assert_eq!(pers_dom.domain_kind, "personal");

        assert_eq!(db.list_email_groups(true).len(), 5);

        let cache = TranslationCache::new();
        cache.load(db.load_all_translations());
        assert_ne!(
            cache.t("tr", msg_keys::TOO_MANY_REQUESTS),
            msg_keys::TOO_MANY_REQUESTS
        );
        let bundle = cache.bundle_json("en");
        assert_eq!(bundle["lang"], "en");
        let strings = bundle["strings"].as_object().unwrap();
        assert!(strings.contains_key("api.error.too_many_requests"));
        assert!(strings.contains_key("nav.edergi"));
        assert!(strings.contains_key("admin.glossary_title"));
        assert!(strings["seo.default_description"]
            .as_str()
            .unwrap()
            .contains("Defence"));
        assert!(strings["plugin.description_human"]
            .as_str()
            .unwrap()
            .contains("Türkiye's defence"));

        // Verify seed_default_translations does NOT skip when rows already exist
        db.delete_translation("nav.edergi").unwrap();
        db.seed_default_translations().unwrap();
        let reloaded = db.load_all_translations();
        assert!(reloaded.iter().any(|e| e.key == "nav.edergi"));

        cleanup_db(&path);
    }

    #[test]
    fn editorial_repo_fixes_n6_v1_t4_c1_c3_and_composite_issues() {
        let path = temp_db_path("editorial_fixes");
        let db = Database::new(&path).expect("Database::new should succeed");

        // N6: New article with colliding slug must auto-dedupe instead of overwriting
        let mut first = ContentItem {
            id: 0,
            slug: "ornek-savunma-haberi".into(),
            title_tr: "İlk Orijinal Başlık".into(),
            title_en: "First Original Title".into(),
            summary_tr: "İlk özet".into(),
            summary_en: "First summary".into(),
            body_tr: "<p>İlk gövde metni.</p>".into(),
            body_en: "<p>First body text.</p>".into(),
            content_type: "haberler".into(),
            source_group: "gunluk-haber".into(),
            published_at: "2026-11-18 10:00".into(),
            en_status: "approved".into(),
            ..Default::default()
        };
        let id1 = db.upsert_content(&first).unwrap();
        assert!(id1 > 0);

        let second = ContentItem {
            id: 0,
            slug: "ornek-savunma-haberi".into(),
            title_tr: "Çakışan İkinci Başlık".into(),
            body_tr: "<p>İkinci haber gövdesi.</p>".into(),
            content_type: "haberler".into(),
            source_group: "gunluk-haber".into(),
            published_at: "2026-11-18 11:00".into(),
            ..Default::default()
        };
        let id2 = db.upsert_content(&second).unwrap();
        assert_ne!(id1, id2);
        let kept = db.get_content_by_slug("ornek-savunma-haberi").unwrap();
        assert_eq!(kept.title_tr, "İlk Orijinal Başlık");
        let deduped = db.get_content_by_slug("ornek-savunma-haberi-2").unwrap();
        assert_eq!(deduped.title_tr, "Çakışan İkinci Başlık");

        // V1 + EN drift + T4: Updating id1 creates auto-snapshot v1 and downgrades en_status when TR changes
        first.id = id1;
        first.title_tr = "Güncellenen Türkçe Başlık".into();
        db.upsert_content(&first).unwrap();

        let versions = db.get_content_versions(id1);
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0]["title_tr"], "İlk Orijinal Başlık");
        assert_eq!(versions[0]["title_en"], "First Original Title");
        assert_eq!(versions[0]["en_status"], "approved");

        let edited = db.get_content_by_slug("ornek-savunma-haberi").unwrap();
        assert_eq!(edited.en_status, "auto_draft");

        // Now add a v2 snapshot with changed EN and roll back to v1 -> EN must symmetrically roll back too
        db.update_content_en_translation(
            id1,
            "Newer EN Title",
            "Newer EN summary",
            "<p>Newer EN body</p>",
            "approved",
            "editor@msidergisi.com",
        )
        .unwrap();
        db.restore_content_version(id1, 1, "chief@msidergisi.com")
            .unwrap();
        let restored = db.get_content_by_slug("ornek-savunma-haberi").unwrap();
        assert_eq!(restored.title_tr, "İlk Orijinal Başlık");
        assert_eq!(restored.title_en, "First Original Title");
        assert_eq!(restored.en_status, "approved");

        // C1-C3: Strict duplicate category slug & partial update preservation
        let cat_res = db.insert_category(&CategoryNode {
            slug: "haberler".into(),
            name_tr: "Çakışma".into(),
            ..Default::default()
        });
        assert!(cat_res.is_err(), "Duplicate category slug must return Err");

        let child_cat = db
            .get_category_by_slug("kunye")
            .expect("kunye child category should exist");
        assert_eq!(child_cat.parent_slug.as_deref(), Some("hakkimizda"));
        db.update_category(
            child_cat.id,
            &CategoryNode {
                slug: "kunye".into(),
                parent_slug: None,
                name_tr: "Künye Güncel".into(),
                name_en: String::new(),
                group_type: String::new(),
                description_tr: child_cat.description_tr.clone(),
                description_en: String::new(),
                icon_class: String::new(),
                sort_order: child_cat.sort_order,
                is_dynamic: true,
                ..Default::default()
            },
        )
        .unwrap();
        let updated_cat = db.get_category_by_slug("kunye").unwrap();
        assert_eq!(updated_cat.parent_slug.as_deref(), Some("hakkimizda"));
        assert_eq!(updated_cat.group_type, "kurumsal");
        assert!(!updated_cat.description_en.is_empty());

        // Composite magazine_issues PK: same issue_number for msi and msi_tdr coexist
        db.upsert_issue(&MagazineIssue {
            issue_number: 150,
            slug: "msi-dergisi-sayi-150-test".into(),
            title_tr: "MSI Dergisi 150".into(),
            title_en: "MSI Magazine 150".into(),
            publish_month_tr: "Ocak".into(),
            publish_month_en: "January".into(),
            year: 2018,
            publication_type: "msi".into(),
            ..Default::default()
        })
        .unwrap();
        db.upsert_issue(&MagazineIssue {
            issue_number: 150,
            slug: "msi-tdr-issue-150-test".into(),
            title_tr: "MSI TDR 150".into(),
            title_en: "MSI TDR 150".into(),
            publish_month_tr: "January".into(),
            publish_month_en: "January".into(),
            year: 2026,
            publication_type: "msi_tdr".into(),
            ..Default::default()
        })
        .unwrap();
        assert!(db.get_issue("msi", 150).is_some());
        assert!(db.get_issue("msi_tdr", 150).is_some());

        cleanup_db(&path);
    }

    #[test]
    fn migration_012_on_live_db_temp_copy_preserves_data_and_rbac() {
        let live_path = "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/storage/db/msi_platform.db";
        if !Path::new(live_path).exists() {
            return;
        }
        let tmp_path = temp_db_path("live_mig012");
        std::fs::copy(live_path, &tmp_path).unwrap();
        let wal = format!("{}-wal", live_path);
        let shm = format!("{}-shm", live_path);
        if Path::new(&wal).exists() {
            let _ = std::fs::copy(&wal, format!("{}-wal", tmp_path));
        }
        if Path::new(&shm).exists() {
            let _ = std::fs::copy(&shm, format!("{}-shm", tmp_path));
        }

        let db = Database::new(&tmp_path).expect("Migration 012 on temp copy of live DB");
        let issues = db.get_all_issues();
        assert_eq!(issues.len(), 22);
        assert!(issues.iter().all(|i| i.uploaded_pdf_file.is_some()));
        assert_eq!(
            issues
                .iter()
                .filter(|i| i.publication_type == "msi_tdr" && i.en_status == "approved")
                .count(),
            7
        );

        let contents = db.get_all_contents();
        assert_eq!(contents.len(), 1010);
        assert_eq!(
            contents.iter().filter(|c| c.en_status == "approved").count(),
            151
        );

        let super_perms = db.get_role_permissions("super_admin").unwrap();
        assert_eq!(super_perms, vec!["*".to_string()]);
        let admin_perms = db.get_role_permissions("admin").unwrap();
        assert!(!admin_perms.iter().any(|p| p == "*"));
        assert!(admin_perms.iter().any(|p| p == "roles:manage"));
        let op_perms = db.get_role_permissions("site_operator").unwrap();
        assert!(!op_perms.iter().any(|p| p == "*"));
        assert!(!op_perms.iter().any(|p| p == "roles:manage"));
        let support_perms = db.get_role_permissions("support_agent").unwrap();
        assert!(support_perms.iter().any(|p| p == "users:write"));
        assert!(support_perms.iter().any(|p| p == "invoices:read"));

        cleanup_db(&tmp_path);
    }
}
