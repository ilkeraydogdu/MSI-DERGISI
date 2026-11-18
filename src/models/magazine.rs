use serde::{Deserialize, Serialize};

fn default_publication_type() -> String {
    "msi".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
}
