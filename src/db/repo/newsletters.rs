#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_newsletter_campaigns(&self) -> Vec<NewsletterCampaign> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, subject_tr, subject_en, target_group_filter, target_institution,
                    target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at
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
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_newsletter_campaigns_paginated(&self, limit: u32, offset: u32) -> (Vec<NewsletterCampaign>, u64) {
        let Ok(conn) = self.open_conn() else {
            return (Vec::new(), 0);
        };
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM newsletter_campaigns", [], |r| r.get(0)).unwrap_or(0);
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, subject_tr, subject_en, target_group_filter, target_institution,
                    target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at
             FROM newsletter_campaigns ORDER BY id DESC LIMIT ?1 OFFSET ?2",
        ) else {
            return (Vec::new(), total);
        };
        let rows = stmt.query_map(rusqlite::params![limit, offset], |r| {
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
            })
        });
        match rows {
            Ok(iter) => (iter.filter_map(Result::ok).collect(), total),
            Err(_) => (Vec::new(), total),
        }
    }

    pub fn insert_newsletter_campaign(&self, nl: &NewsletterCampaign) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

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
                conditions.push(
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
                        AND (subscription_expires_at IS NULL OR SUBSTR(subscription_expires_at, 1, 10) >= date('now'))
                    ))"
                    .to_string(),
                );
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

        let matched_count: i64 = conn
            .query_row(&count_sql, rusqlite::params_from_iter(refs), |r| r.get(0))
            .unwrap_or(0);

        let sent_at = if nl.sent_at.trim().is_empty() {
            chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string()
        } else {
            nl.sent_at.clone()
        };
        let status = if nl.status.trim().is_empty() {
            "sent".to_string()
        } else {
            nl.status.clone()
        };
        conn.execute(
            "INSERT INTO newsletter_campaigns (
                subject_tr, subject_en, target_group_filter, target_institution,
                target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
                sent_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_newsletter_campaign(&self, id: i64) -> rusqlite::Result<bool> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let deleted = conn.execute("DELETE FROM newsletter_campaigns WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }
}
