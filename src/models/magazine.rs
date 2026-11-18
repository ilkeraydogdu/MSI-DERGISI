use serde::{Deserialize, Serialize};

fn default_publication_type() -> String {
    "msi".to_string()
}

fn default_en_status() -> String {
    "none".to_string()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MagazinePageSpread {
    pub page_no: String,
    pub title_tr: String,
    pub title_en: String,
    pub category_badge: String,
    pub image_url: String,
    pub excerpt_tr: String,
    pub excerpt_en: String,
    pub linked_content_slug: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MagazineIssue {
    pub issue_number: i32,
    pub slug: String,
    pub title_tr: String,
    pub title_en: String,
    pub publish_month_tr: String,
    pub publish_month_en: String,
    pub year: i32,
    pub cover_image: String,
    pub is_special_issue: bool,
    pub event_name: Option<String>,
    pub page_count: i32,
    pub dossier_count: i32,
    pub summary_tr: String,
    pub summary_en: String,
    pub pdf_path: String,
    #[serde(default = "default_publication_type")]
    pub publication_type: String,
    #[serde(default)]
    pub uploaded_pdf_file: Option<String>,
    pub spreads: Vec<MagazinePageSpread>,
    #[serde(default = "default_en_status")]
    pub en_status: String,
    #[serde(default)]
    pub en_translated_at: Option<String>,
    #[serde(default)]
    pub en_approved_by: Option<String>,
    #[serde(default)]
    pub en_approved_at: Option<String>,
    #[serde(default)]
    pub tr_source_hash: Option<String>,
}

impl Default for MagazineIssue {
    fn default() -> Self {
        Self {
            issue_number: 0,
            slug: String::new(),
            title_tr: String::new(),
            title_en: String::new(),
            publish_month_tr: String::new(),
            publish_month_en: String::new(),
            year: 2026,
            cover_image: String::new(),
            is_special_issue: false,
            event_name: None,
            page_count: 0,
            dossier_count: 0,
            summary_tr: String::new(),
            summary_en: String::new(),
            pdf_path: String::new(),
            publication_type: default_publication_type(),
            uploaded_pdf_file: None,
            spreads: Vec::new(),
            en_status: default_en_status(),
            en_translated_at: None,
            en_approved_by: None,
            en_approved_at: None,
            tr_source_hash: None,
        }
    }
}
