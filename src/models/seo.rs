use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeoRedirectRule {
    pub id: i64,
    pub old_path: String,
    pub new_path: String,
    pub status_code: u16,
    pub hit_count: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SeoMetaBundle {
    pub title: String,
    pub description: String,
    pub canonical_url: String,
    pub og_type: String,
    pub og_image: String,
    #[serde(default)]
    pub og_image_alt: String,
    #[serde(default)]
    pub robots: String,
    #[serde(default)]
    pub keywords: String,
    pub hreflang_tr: String,
    pub hreflang_en: String,
    #[serde(default)]
    pub extra_head_html: String,
    pub json_ld: String,
}