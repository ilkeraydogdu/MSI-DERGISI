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
                    uploaded_pdf_file
             FROM magazine_issues
             ORDER BY issue_number DESC",
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
                })
            },
        )
    }

    pub fn get_issue_by_number(&self, issue_number: i32) -> Option<MagazineIssue> {
        self.get_all_issues()
            .into_iter()
            .find(|i| i.issue_number == issue_number)
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
            format!("/api/v1/edergi/{}/download-pdf", iss.issue_number)
        } else {
            iss.pdf_path.trim().to_string()
        };
        conn.execute(
            "INSERT INTO magazine_issues (
                issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
                year, cover_image, is_special_issue, event_name, page_count, dossier_count,
                summary_tr, summary_en, pdf_path, spreads_json, publication_type, uploaded_pdf_file
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(issue_number) DO UPDATE SET
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
                publication_type = excluded.publication_type,
                uploaded_pdf_file = excluded.uploaded_pdf_file",
            params![
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
                pub_type,
                iss.uploaded_pdf_file
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
}
