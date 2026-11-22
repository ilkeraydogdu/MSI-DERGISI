use super::*;

impl Database {
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

}
