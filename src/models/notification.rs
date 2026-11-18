use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemNotification {
 pub id: i64,
 pub target_audience: String,
 pub target_email: Option<String>,
 pub notification_type: String,
 pub severity: String,
 pub title_tr: String,
 pub title_en: String,
 pub message_tr: String,
 pub message_en: String,
 pub action_url: Option<String>,
 pub is_read: bool,
 pub is_active_banner: bool,
 pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsletterCampaign {
 pub id: i64,
 pub subject_tr: String,
 pub subject_en: String,
 pub target_group_filter: String,
 pub target_institution: String,
 pub target_ssb_tag: String,
 pub sponsor_ad_id: Option<i64>,
 pub sponsor_name: Option<String>,
 pub recipient_count: i64,
 pub status: String,
 pub sent_at: String,
}