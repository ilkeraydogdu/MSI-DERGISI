#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_all_issues(&self) -> Vec<MagazineIssue> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.issues.clone();
            }
        }
        self.load_issues_db()
    }

    pub(super) fn load_issues_db(&self) -> Vec<MagazineIssue> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
                    year, cover_image, is_special_issue, event_name, page_count, dossier_count,
                    summary_tr, summary_en, pdf_path, spreads_json,
                    COALESCE(publication_type, CASE WHEN issue_number < 200 THEN 'msi_tdr' ELSE 'msi' END),
                    uploaded_pdf_file,
                    COALESCE(en_status, 'none'), en_translated_at, en_approved_by, en_approved_at, tr_source_hash
             FROM magazine_issues
             ORDER BY year DESC, issue_number DESC",
            [],
            |r| {
                let spreads_raw: String = r.get(15)?;
                let spreads: Vec<MagazinePageSpread> =
                    serde_json::from_str(&spreads_raw).unwrap_or_default();
                Ok(MagazineIssue {
                    issue_number: r.get(0)?,
                    slug: r.get(1)?,
                    title_tr: r.get(2)?,
                    title_en: r.get(3)?,
                    publish_month_tr: r.get(4)?,
                    publish_month_en: r.get(5)?,
                    year: r.get(6)?,
                    cover_image: r.get(7)?,
                    is_special_issue: r.get::<_, i32>(8)? != 0,
                    event_name: r.get(9)?,
                    page_count: r.get(10)?,
                    dossier_count: r.get(11)?,
                    summary_tr: r.get(12)?,
                    summary_en: r.get(13)?,
                    pdf_path: r.get(14)?,
                    publication_type: r.get(16)?,
                    uploaded_pdf_file: r.get(17)?,
                    spreads,
                    en_status: r.get::<_, String>(18).unwrap_or_else(|_| "none".to_string()),
                    en_translated_at: r.get(19)?,
                    en_approved_by: r.get(20)?,
                    en_approved_at: r.get(21)?,
                    tr_source_hash: r.get(22)?,
                })
            },
        )
    }

    pub fn get_issue_by_number(&self, issue_number: i32) -> Option<MagazineIssue> {
        self.get_all_issues()
            .into_iter()
            .find(|i| i.issue_number == issue_number)
    }

    pub fn get_issues_by_number(&self, issue_number: i32) -> Vec<MagazineIssue> {
        self.get_all_issues()
            .into_iter()
            .filter(|i| i.issue_number == issue_number)
            .collect()
    }

    pub fn get_issue(&self, publication_type: &str, issue_number: i32) -> Option<MagazineIssue> {
        self.get_all_issues()
            .into_iter()
            .find(|i| i.issue_number == issue_number && i.publication_type.eq_ignore_ascii_case(publication_type.trim()))
    }

    pub fn upsert_issue(&self, iss: &MagazineIssue) -> rusqlite::Result<i32> {
        let conn = self
            .open_conn()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let spreads_json =
            serde_json::to_string(&iss.spreads).unwrap_or_else(|_| "[]".to_string());
        let pub_type = if iss.publication_type.trim().is_empty() {
            if iss.issue_number < 200 {
                "msi_tdr"
            } else {
                "msi"
            }
        } else {
            iss.publication_type.trim()
        };
        let pdf_path = if iss.pdf_path.trim().is_empty() {
            format!(
                "/api/v1/edergi/{}/download-pdf?publication_type={}",
                iss.issue_number, pub_type
            )
        } else {
            iss.pdf_path.trim().to_string()
        };
        let en_status = match iss.en_status.trim() {
            "approved" => "approved",
            "auto_draft" => "auto_draft",
            _ => if pub_type == "msi_tdr" { "approved" } else { "none" },
        };
        let tr_hash = iss.tr_source_hash.clone().or_else(|| {
            if en_status != "none" {
                Some(Self::compute_tr_source_hash(&iss.title_tr, &iss.summary_tr, &iss.publish_month_tr))
            } else {
                None
            }
        });
        conn.execute(
            "INSERT INTO magazine_issues (
                publication_type, issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
                year, cover_image, is_special_issue, event_name, page_count, dossier_count,
                summary_tr, summary_en, pdf_path, spreads_json, uploaded_pdf_file,
                en_status, en_translated_at, en_approved_by, en_approved_at, tr_source_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
            ON CONFLICT(publication_type, issue_number) DO UPDATE SET
                slug = excluded.slug,
                title_tr = excluded.title_tr,
                title_en = excluded.title_en,
                publish_month_tr = excluded.publish_month_tr,
                publish_month_en = excluded.publish_month_en,
                year = excluded.year,
                cover_image = excluded.cover_image,
                is_special_issue = excluded.is_special_issue,
                event_name = excluded.event_name,
                page_count = excluded.page_count,
                dossier_count = excluded.dossier_count,
                summary_tr = excluded.summary_tr,
                summary_en = excluded.summary_en,
                pdf_path = excluded.pdf_path,
                spreads_json = excluded.spreads_json,
                uploaded_pdf_file = COALESCE(excluded.uploaded_pdf_file, magazine_issues.uploaded_pdf_file),
                en_status = excluded.en_status,
                en_translated_at = COALESCE(excluded.en_translated_at, magazine_issues.en_translated_at),
                en_approved_by = COALESCE(excluded.en_approved_by, magazine_issues.en_approved_by),
                en_approved_at = COALESCE(excluded.en_approved_at, magazine_issues.en_approved_at),
                tr_source_hash = COALESCE(excluded.tr_source_hash, magazine_issues.tr_source_hash)",
            params![
                pub_type,
                iss.issue_number,
                iss.slug,
                iss.title_tr,
                iss.title_en,
                iss.publish_month_tr,
                iss.publish_month_en,
                iss.year,
                iss.cover_image,
                if iss.is_special_issue { 1 } else { 0 },
                iss.event_name,
                iss.page_count,
                iss.dossier_count,
                iss.summary_tr,
                iss.summary_en,
                pdf_path,
                spreads_json,
                iss.uploaded_pdf_file,
                en_status,
                iss.en_translated_at,
                iss.en_approved_by,
                iss.en_approved_at,
                tr_hash
            ],
        )?;
        drop(conn);
        self.refresh_l1_sections(&["issues"]);
        Ok(iss.issue_number)
    }

    pub fn delete_issue(&self, issue_number: i32) -> rusqlite::Result<bool> {
        let conn = self
            .open_conn()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let deleted = conn.execute(
            "DELETE FROM magazine_issues WHERE issue_number = ?1",
            params![issue_number],
        )?;
        drop(conn);
        if deleted > 0 {
            self.refresh_l1_sections(&["issues"]);
        }
        Ok(deleted > 0)
    }

    pub fn delete_issue_by_type(&self, publication_type: &str, issue_number: i32) -> rusqlite::Result<bool> {
        let conn = self
            .open_conn()
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let deleted = conn.execute(
            "DELETE FROM magazine_issues WHERE LOWER(publication_type) = LOWER(?1) AND issue_number = ?2",
            params![publication_type.trim(), issue_number],
        )?;
        drop(conn);
        if deleted > 0 {
            self.refresh_l1_sections(&["issues"]);
        }
        Ok(deleted > 0)
    }
}
