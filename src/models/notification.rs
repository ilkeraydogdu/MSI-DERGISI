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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
 #[serde(default)]
 pub scheduled_at: Option<String>,
 #[serde(default)]
 pub email_group_code: Option<String>,
 #[serde(default)]
 pub body_html: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct B2bMonthlyReport {
 pub id: i64,
 pub report_month: String,
 pub report_type: String,
 pub recipient_email: String,
 #[serde(default)]
 pub summary_json: String,
 #[serde(default)]
 pub status: String,
 #[serde(default)]
 pub sent_at: String,
}