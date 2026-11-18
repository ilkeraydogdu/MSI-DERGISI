use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiteSetting {
    pub key: String,
    pub value_tr: String,
    pub value_en: String,
    pub setting_group: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecommendationRule {
    pub id: i64,
    pub rule_name: String,
    pub source_tag_or_cat: String,
    pub target_institution: String,
    pub recommended_content_slug: String,
    pub priority: i32,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenewalReminderLog {
    pub id: i64,
    pub user_id: i64,
    pub user_email: String,
    pub full_name: String,
    pub organization_name: String,
    /// `2_months` (60g), `1_month` (30g), `2_weeks` (14g), `1_week` (7g), `1_day` (1g)
    pub stage_label: String,
    pub days_remaining: i32,
    /// `sent`, `scheduled`, `halted_by_renewal`
    pub status: String,
    pub sent_at: String,
}