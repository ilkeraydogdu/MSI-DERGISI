use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AnalyticsEvent {
 pub id: i64,
 pub session_id: String,
 pub user_id: Option<i64>,
 pub organization_category: String,
 pub organization_name: String,
 pub event_type: String,
 pub content_slug: Option<String>,
 pub ad_id: Option<i64>,
 pub duration_sec: i32,
 pub scroll_depth_pct: i32,
 pub video_completed: bool,
 pub is_true_read: bool,
 pub ssb_tag: Option<String>,
 pub created_at: String,
 #[serde(default)]
 pub video_watch_pct: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstitutionBreakdownItem {
 pub organization_category: String,
 pub count: i64,
 pub percentage: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentAnalyticsReport {
 pub content_slug: String,
 pub title_tr: String,
 pub total_views: i64,
 pub true_reads: i64,
 pub avg_duration_sec: f64,
 pub avg_scroll_pct: f64,
 pub institution_breakdown: Vec<InstitutionBreakdownItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdAnalyticsReport {
 pub ad_id: i64,
 pub sponsor_name: String,
 pub slot_code: String,
 pub impressions: i64,
 pub clicks: i64,
 pub ctr_pct: f64,
 pub impression_institutions: Vec<InstitutionBreakdownItem>,
 pub click_institutions: Vec<InstitutionBreakdownItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReaderAnalyticsReport {
 pub user_id: i64,
 pub full_name: String,
 pub email: String,
 pub organization_category: String,
 pub organization_name: String,
 pub subscription_tier: String,
 pub articles_read: Vec<String>,
 pub ssb_keywords_engaged: Vec<String>,
 pub ad_impressions_count: i64,
 pub ad_clicks_count: i64,
 pub clicked_ads: Vec<String>,
}