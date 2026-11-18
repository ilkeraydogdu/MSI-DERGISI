use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdCampaign {
 pub id: i64,
 pub slot_code: String,
 pub badge_label: String,
 pub dimensions: String,
 pub sponsor_name: String,
 /// `defense_oem`, `auxiliary_service`, `in_house`
 pub sponsor_type: String,
 pub pill_style: String,
 pub title_tr: String,
 pub title_en: String,
 pub desc_tr: String,
 pub desc_en: String,
 pub target_url: String,
 pub image_url: Option<String>,
 pub budget_score: i32,
 pub is_default_fallback: bool,
 pub is_floating_popup: bool,
 pub is_newsletter_ad: bool,
 pub target_categories: Vec<String>,
 pub target_ssb_tags: Vec<String>,
 pub target_institutions: Vec<String>,
 pub impressions_count: i64,
 pub clicks_count: i64,
 pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdConflictWarning {
 pub has_conflict: bool,
 pub conflicting_campaigns: Vec<String>,
 pub message_tr: String,
 pub quota_limit: usize,
 pub current_count: usize,
}