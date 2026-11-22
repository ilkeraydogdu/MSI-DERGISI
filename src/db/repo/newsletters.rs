#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_newsletter_campaigns(&self) -> Vec<NewsletterCampaign> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, subject_tr, subject_en, target_group_filter, target_institution,
                    target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at,
                    scheduled_at, email_group_code, COALESCE(body_html, '')
             FROM newsletter_campaigns ORDER BY id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(NewsletterCampaign {
                id: r.get(0)?,
                subject_tr: r.get(1)?,
                subject_en: r.get(2)?,
                target_group_filter: r.get(3)?,
                target_institution: r.get(4)?,
                target_ssb_tag: r.get(5)?,
                sponsor_ad_id: r.get(6)?,
                sponsor_name: r.get(7)?,
                recipient_count: r.get(8)?,
                status: r.get(9)?,
                sent_at: r.get(10)?,
                scheduled_at: r.get(11)?,
                email_group_code: r.get(12)?,
                body_html: r.get(13)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_newsletter_campaigns_paginated(&self, limit: u32, offset: u32) -> (Vec<NewsletterCampaign>, u64) {
        crate::db::query_helpers::query_paginated(
            self,
            "SELECT COUNT(*) FROM newsletter_campaigns",
            [],
            "SELECT id, subject_tr, subject_en, target_group_filter, target_institution,
                    target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at,
                    scheduled_at, email_group_code, COALESCE(body_html, '')
             FROM newsletter_campaigns ORDER BY id DESC LIMIT ?1 OFFSET ?2",
            rusqlite::params![limit, offset],
            |r| Ok(NewsletterCampaign {
                id: r.get(0)?,
                subject_tr: r.get(1)?,
                subject_en: r.get(2)?,
                target_group_filter: r.get(3)?,
                target_institution: r.get(4)?,
                target_ssb_tag: r.get(5)?,
                sponsor_ad_id: r.get(6)?,
                sponsor_name: r.get(7)?,
                recipient_count: r.get(8)?,
                status: r.get(9)?,
                sent_at: r.get(10)?,
                scheduled_at: r.get(11)?,
                email_group_code: r.get(12)?,
                body_html: r.get(13)?,
            }),
        )
    }

    pub fn insert_newsletter_campaign(&self, nl: &NewsletterCampaign) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let is_draft = nl.status.trim().eq_ignore_ascii_case("draft");
        let matched_count: i64 = if is_draft {
            0
        } else if nl.recipient_count > 0 {
            nl.recipient_count
        } else if let Some(ref group_code) = nl.email_group_code.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            self.resolve_email_group_recipients(group_code, &nl.target_institution, &nl.target_ssb_tag).len() as i64
        } else {
            let mut conditions: Vec<String> = vec![
                "role != 'deleted'".to_string(),
                "email_consent = 1".to_string(),
                "COALESCE(marketing_emails_opted_in, 1) = 1".to_string(),
            ];
            let mut str_params: Vec<String> = Vec::new();

            let grp = nl.target_group_filter.trim().to_lowercase();
            match grp.as_str() {
                "gov_verified" | "public_sector" | "kamu" => {
                    conditions.push("is_gov_verified = 1".to_string());
                }
                "active_subscribers" | "subscribers" | "paid_subscribers" => {
                    let idx = str_params.len() + 1;
                    conditions.push(format!(
                        "(is_gov_verified = 1 OR (
                            subscription_tier IS NOT NULL
                            AND TRIM(subscription_tier) != ''
                            AND LOWER(TRIM(subscription_tier)) NOT IN (
                                'free',
                                'standart kayıtlı okur',
                                'standart kayitli okur',
                                'akademik / öğrenci okur',
                                'akademik / ogrenci okur',
                                'akademik okur'
                            )
                            AND (subscription_expires_at IS NULL OR SUBSTR(subscription_expires_at, 1, 10) >= ?{})
                        ))",
                        idx
                    ));
                    str_params.push(today.clone());
                }
                _ => {}
            }

            let inst = nl.target_institution.trim();
            if !inst.is_empty() && inst != "*" && !inst.eq_ignore_ascii_case("all") {
                let idx = str_params.len() + 1;
                conditions.push(format!(
                    "(LOWER(organization_category) = LOWER(?{}) OR LOWER(organization_name) = LOWER(?{}))",
                    idx, idx
                ));
                str_params.push(inst.to_string());
            }

            let tag = nl.target_ssb_tag.trim();
            if !tag.is_empty() && tag != "*" && !tag.eq_ignore_ascii_case("all") {
                let idx = str_params.len() + 1;
                conditions.push(format!("LOWER(ssb_interests_json) LIKE LOWER(?{})", idx));
                str_params.push(format!("%{}%", tag));
            }

            let count_sql = format!(
                "SELECT COUNT(*) FROM users WHERE {}",
                conditions.join(" AND ")
            );
            let boxed: Vec<Box<dyn rusqlite::types::ToSql>> = str_params
                .iter()
                .map(|s| Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>)
                .collect();
            let refs: Vec<&dyn rusqlite::types::ToSql> = boxed.iter().map(|p| p.as_ref()).collect();

            conn.query_row(&count_sql, rusqlite::params_from_iter(refs), |r| r.get(0))
                .unwrap_or(0)
        };

        let status = if nl.status.trim().is_empty() {
            "sent".to_string()
        } else {
            nl.status.clone()
        };
        let sent_at = if nl.sent_at.trim().is_empty() {
            if status.eq_ignore_ascii_case("scheduled") || status.eq_ignore_ascii_case("draft") {
                String::new()
            } else {
                chrono::Utc::now().to_rfc3339()
            }
        } else {
            nl.sent_at.clone()
        };
        let scheduled_at = nl
            .scheduled_at
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let email_group_code = nl
            .email_group_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        conn.execute(
            "INSERT INTO newsletter_campaigns (
                subject_tr, subject_en, target_group_filter, target_institution,
                target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at,
                scheduled_at, email_group_code, body_html
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                nl.subject_tr,
                nl.subject_en,
                nl.target_group_filter,
                nl.target_institution,
                nl.target_ssb_tag,
                nl.sponsor_ad_id,
                nl.sponsor_name,
                matched_count,
                status,
                sent_at,
                scheduled_at,
                email_group_code,
                nl.body_html
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn claim_due_scheduled_newsletters(&self, now_cmp: &str) -> Vec<NewsletterCampaign> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(tx) = conn.unchecked_transaction() else {
            return Vec::new();
        };

        let candidates: Vec<NewsletterCampaign> = {
            let Ok(mut stmt) = tx.prepare(
                "SELECT id, subject_tr, subject_en, target_group_filter, target_institution,
                        target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at,
                        scheduled_at, email_group_code, COALESCE(body_html, '')
                 FROM newsletter_campaigns
                 WHERE status = 'scheduled'
                   AND scheduled_at IS NOT NULL
                   AND TRIM(scheduled_at) != ''
                   AND REPLACE(SUBSTR(TRIM(scheduled_at), 1, 19), 'T', ' ') <= ?1
                 ORDER BY scheduled_at ASC, id ASC",
            ) else {
                return Vec::new();
            };
            let rows = stmt.query_map(params![now_cmp], |r| {
                Ok(NewsletterCampaign {
                    id: r.get(0)?,
                    subject_tr: r.get(1)?,
                    subject_en: r.get(2)?,
                    target_group_filter: r.get(3)?,
                    target_institution: r.get(4)?,
                    target_ssb_tag: r.get(5)?,
                    sponsor_ad_id: r.get(6)?,
                    sponsor_name: r.get(7)?,
                    recipient_count: r.get(8)?,
                    status: r.get(9)?,
                    sent_at: r.get(10)?,
                    scheduled_at: r.get(11)?,
                    email_group_code: r.get(12)?,
                    body_html: r.get(13)?,
                })
            });
            match rows {
                Ok(iter) => iter.filter_map(Result::ok).collect(),
                Err(_) => Vec::new(),
            }
        };

        let mut claimed = Vec::new();
        for mut c in candidates {
            let changed = tx
                .execute(
                    "UPDATE newsletter_campaigns SET status = 'sending' WHERE id = ?1 AND status = 'scheduled'",
                    params![c.id],
                )
                .unwrap_or(0);
            if changed == 1 {
                c.status = "sending".to_string();
                claimed.push(c);
            }
        }
        if tx.commit().is_err() {
            return Vec::new();
        }
        claimed
    }

    pub fn finalize_newsletter_dispatch(
        &self,
        id: i64,
        status: &str,
        recipient_count: i64,
        sent_at: &str,
    ) -> Result<(), String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE newsletter_campaigns
             SET status = ?1, recipient_count = ?2, sent_at = ?3
             WHERE id = ?4",
            params![status.trim(), recipient_count, sent_at.trim(), id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_b2b_monthly_report_if_new(
        &self,
        report: &crate::models::B2bMonthlyReport,
    ) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let summary = if report.summary_json.trim().is_empty() {
            "{}".to_string()
        } else {
            report.summary_json.clone()
        };
        let status = if report.status.trim().is_empty() {
            "pending".to_string()
        } else {
            report.status.trim().to_string()
        };
        let affected = conn
            .execute(
                "INSERT INTO b2b_monthly_reports (
                    report_month, report_type, recipient_email, summary_json, status, sent_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(report_month, report_type, recipient_email) DO NOTHING",
                params![
                    report.report_month.trim(),
                    report.report_type.trim(),
                    report.recipient_email.trim().to_lowercase(),
                    summary,
                    status,
                    report.sent_at.trim()
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    pub fn update_b2b_monthly_report_status(
        &self,
        report_month: &str,
        report_type: &str,
        recipient_email: &str,
        status: &str,
        sent_at: &str,
    ) -> Result<(), String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE b2b_monthly_reports
             SET status = ?1, sent_at = ?2
             WHERE report_month = ?3 AND report_type = ?4 AND LOWER(recipient_email) = LOWER(?5)",
            params![
                status.trim(),
                sent_at.trim(),
                report_month.trim(),
                report_type.trim(),
                recipient_email.trim()
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_b2b_monthly_reports(&self, report_month: Option<&str>) -> Vec<crate::models::B2bMonthlyReport> {
        let month_filter = report_month.map(str::trim).filter(|s| !s.is_empty());
        if let Some(m) = month_filter {
            crate::db::query_helpers::query_all(
                self,
                "SELECT id, report_month, report_type, recipient_email, summary_json, status, sent_at
                 FROM b2b_monthly_reports WHERE report_month = ?1 ORDER BY id DESC",
                params![m],
                |r| {
                    Ok(crate::models::B2bMonthlyReport {
                        id: r.get(0)?,
                        report_month: r.get(1)?,
                        report_type: r.get(2)?,
                        recipient_email: r.get(3)?,
                        summary_json: r.get(4)?,
                        status: r.get(5)?,
                        sent_at: r.get(6)?,
                    })
                },
            )
        } else {
            crate::db::query_helpers::query_all(
                self,
                "SELECT id, report_month, report_type, recipient_email, summary_json, status, sent_at
                 FROM b2b_monthly_reports ORDER BY report_month DESC, id DESC LIMIT 100",
                [],
                |r| {
                    Ok(crate::models::B2bMonthlyReport {
                        id: r.get(0)?,
                        report_month: r.get(1)?,
                        report_type: r.get(2)?,
                        recipient_email: r.get(3)?,
                        summary_json: r.get(4)?,
                        status: r.get(5)?,
                        sent_at: r.get(6)?,
                    })
                },
            )
        }
    }

    pub fn add_email_to_email_group(&self, group_code: &str, email: &str) -> Result<(), String> {
        let norm_email = email.trim().to_lowercase();
        let norm_code = group_code.trim().to_ascii_lowercase();
        if norm_email.is_empty() || norm_code.is_empty() {
            return Ok(());
        }
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let mut row: Option<(i64, String)> = conn
            .query_row(
                "SELECT id, COALESCE(static_emails_json, '[]') FROM email_groups WHERE LOWER(code) = ?1 LIMIT 1",
                params![norm_code],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        if row.is_none() {
            let now = chrono::Utc::now().to_rfc3339();
            let _ = conn.execute(
                "INSERT INTO email_groups (
                    code, name_tr, name_en, description_tr, description_en, filter_json, static_emails_json,
                    requires_marketing_consent, is_system, is_active, sort_order, created_by, updated_at
                 ) VALUES (?1, ?2, ?2, '', '', '{}', '[]', 1, 0, 1, 99, 'system', ?3)
                 ON CONFLICT(code) DO NOTHING",
                params![norm_code, group_code.trim(), now],
            );
            row = conn
                .query_row(
                    "SELECT id, COALESCE(static_emails_json, '[]') FROM email_groups WHERE LOWER(code) = ?1 LIMIT 1",
                    params![norm_code],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .ok();
        }
        if let Some((gid, raw_json)) = row {
            let mut list: Vec<String> = serde_json::from_str(&raw_json).unwrap_or_default();
            if !list.iter().any(|e| e.trim().eq_ignore_ascii_case(&norm_email)) {
                list.push(norm_email.clone());
                let updated_json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
                let now = chrono::Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE email_groups SET static_emails_json = ?1, updated_at = ?2 WHERE id = ?3",
                    params![updated_json, now, gid],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        let _ = conn.execute(
            "UPDATE users SET email_consent = 1, marketing_emails_opted_in = 1 WHERE LOWER(email) = ?1",
            params![norm_email],
        );
        Ok(())
    }

    pub fn delete_newsletter_campaign(&self, id: i64) -> rusqlite::Result<bool> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let deleted = conn.execute("DELETE FROM newsletter_campaigns WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }
}
