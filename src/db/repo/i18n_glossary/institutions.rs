use super::*;
use rusqlite::OptionalExtension;

impl Database {
    // =========================================================================
    // 3. DYNAMIC INSTITUTION CATEGORIES CRUD
    // =========================================================================

    pub fn list_institution_categories(&self, active_only: bool) -> Vec<InstitutionCategory> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap
                    .institution_categories
                    .iter()
                    .filter(|c| !active_only || c.is_active)
                    .cloned()
                    .collect();
            }
        }
        self.load_institution_categories_db(active_only)
    }

    pub(in crate::db::repo) fn load_institution_categories_db(
        &self,
        active_only: bool,
    ) -> Vec<InstitutionCategory> {
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
        drop(conn);
        self.refresh_l1_sections(&["institution_categories"]);
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
        drop(conn);
        if affected > 0 {
            self.refresh_l1_sections(&["institution_categories"]);
        }
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
        if norm.is_empty() {
            return Err("E-posta grubu kodu boş olamaz.".to_string());
        }
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let is_sys_opt: Option<i32> = conn
            .query_row(
                "SELECT is_system FROM email_groups WHERE LOWER(code) = ?1",
                params![norm],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some(is_sys) = is_sys_opt else {
            return Ok(false);
        };
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
        let Some(group) = self.get_email_group_by_code(group_code) else {
            return Vec::new();
        };
        if !group.is_active {
            return Vec::new();
        }
        let filter_val: serde_json::Value = serde_json::from_str(&group.filter_json)
            .unwrap_or_else(|_| serde_json::json!({}));
        let requires_consent = group.requires_marketing_consent;
        let static_emails: Vec<String> = serde_json::from_str(&group.static_emails_json)
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
        let has_dynamic_filter = filter_val.get("group").and_then(|v| v.as_str()) == Some("all")
            || filter_val.get("is_gov_verified").and_then(|v| v.as_bool()) == Some(true)
            || filter_val.get("email_verified").and_then(|v| v.as_bool()) == Some(true)
            || !filter_cats.is_empty()
            || min_true_reads > 0;

        let all_users = self.get_all_users();
        let mut matched: Vec<UserAccount> = all_users
            .iter()
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
                if !in_static {
                    let group_ok = match norm_code.as_str() {
                        "all" | "*" => true,
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
                        _ => has_dynamic_filter,
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
            .cloned()
            .collect();

        if !static_emails.is_empty() && inst_tokens.is_empty() && ssb_tokens.is_empty() {
            let mut seen: std::collections::HashSet<String> = matched
                .iter()
                .map(|u| u.email.trim().to_lowercase())
                .collect();
            let user_by_email: HashMap<String, &UserAccount> = all_users
                .iter()
                .map(|u| (u.email.trim().to_lowercase(), u))
                .collect();

            for raw_static in &static_emails {
                let norm = raw_static.trim().to_lowercase();
                if norm.is_empty() || !norm.contains('@') || seen.contains(&norm) {
                    continue;
                }
                if let Some(existing_user) = user_by_email.get(&norm) {
                    if existing_user.role == "deleted"
                        || (requires_consent
                            && (!existing_user.email_consent
                                || !existing_user.marketing_emails_opted_in))
                    {
                        continue;
                    }
                } else if requires_consent {
                    let opted_out = self
                        .open_conn()
                        .ok()
                        .and_then(|conn| {
                            conn.query_row(
                                "SELECT accepted FROM kvkk_consent_logs
                                 WHERE LOWER(email) = ?1 AND consent_type = 'etk'
                                 ORDER BY id DESC LIMIT 1",
                                params![&norm],
                                |r| r.get::<_, i32>(0),
                            )
                            .ok()
                        })
                        .map(|acc| acc == 0)
                        .unwrap_or(false);
                    if opted_out {
                        continue;
                    }
                }
                seen.insert(norm.clone());
                matched.push(UserAccount {
                    id: 0,
                    email: norm,
                    full_name: "Bülten Abonesi".to_string(),
                    organization_name: "MSI Okuyucu".to_string(),
                    email_consent: true,
                    marketing_emails_opted_in: true,
                    ..Default::default()
                });
            }
        }

        matched
    }
}
