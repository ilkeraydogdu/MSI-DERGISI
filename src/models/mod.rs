use serde::{Deserialize, Serialize};

/// Single Source of Truth Content Entity
/// Stored once in `contents` and linked to multiple site sections (`placements`) and SSB interest tags (`ssb_tags`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentItem {
    pub id: i64,
    pub slug: String,
    pub title_tr: String,
    pub title_en: String,
    pub summary_tr: String,
    pub summary_en: String,
    pub body_tr: String,
    pub body_en: String,
    /// Primary content type: `haberler`, `ozel-haberler`, `soylesiler`, `mesajlar`, `tanitici-makaleler`, `makaleler-analizler`, `advertorial`, `basyazi`, `video`
    pub content_type: String,
    /// Source group: `gunluk-haber`, `dergi-icerigi`, `sadece-online`
    pub source_group: String,
    pub cover_image: String,
    pub author_name: String,
    pub author_title: String,
    pub issue_number: Option<i32>,
    pub published_at: String,
    pub read_time_min: i32,
    /// Paywall Visibility Level:
    /// 0 = Tüm kullanıcılar tarafından görülebilir
    /// 1 = Kayıtlı kullanıcılar ve aboneler tarafından görülebilir
    /// 2 = Sadece aboneler (veya tekil satın alanlar) tarafından görülebilir
    pub visibility_level: i32,
    pub single_price_try: f64,
    pub is_sponsored: bool,
    pub sponsor_brand: Option<String>,
    /// Legal label required by specification: e.g. "SPONSORLU İÇERİK", "İLANDIR", "ADVERTORIAL"
    pub sponsor_legal_label: Option<String>,
    pub is_featured: bool,
    pub video_url: Option<String>,
    pub view_count: i64,
    pub true_read_count: i64,
    /// Multi-placement slugs (Single Source of Truth): e.g. `["mesajlar", "tskdan-haberler", "dergi-icerikleri", "ozel-dosyalar"]`
    pub placements: Vec<String>,
    /// Linked SSB Catalog Interest Tags: e.g. `["hava-araclari", "radarlar-ve-sensorler"]`
    pub ssb_tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryNode {
    pub id: i64,
    pub slug: String,
    pub parent_slug: Option<String>,
    pub name_tr: String,
    pub name_en: String,
    /// Group: `icerikler`, `gruplandirilmis`, `abonelik`, `kurumsal`, `gelecek-faz`
    pub group_type: String,
    pub description_tr: String,
    pub description_en: String,
    pub icon_class: String,
    pub sort_order: i32,
    pub is_dynamic: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SsbInterestTag {
    pub id: i64,
    pub code: String,
    pub name_tr: String,
    pub name_en: String,
    pub category_group: String,
    pub sort_order: i32,
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
    pub spreads: Vec<MagazinePageSpread>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserAccount {
    pub id: i64,
    pub email: String,
    pub full_name: String,
    pub role: String,
    /// Ready list from customer spec:
    /// `MSB`, `SSB`, `K.K.K.lığı`, `Dz.K.K.lığı`, `Hv.K.K.lığı`, `J.G.K.lığı`, `EGM`, `S.G.K.lığı`, `MİT`, `Diğer Kamu Kurumları`, `Sektör Kuruluşları`, `Üniversite / Öğrenci`, `Yurt Dışı Paydaş`
    pub organization_category: String,
    pub organization_name: String,
    pub job_title: String,
    pub is_gov_verified: bool,
    pub subscription_tier: String,
    pub subscription_expires_at: Option<String>,
    pub reminders_halted: bool,
    pub kvkk_consent_at: String,
    pub kvkk_consent_ip: String,
    pub kvkk_consent_hash: String,
    pub email_consent: bool,
    pub ssb_interests: Vec<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorporateVerificationRequest {
    pub id: i64,
    pub user_id: i64,
    pub email: String,
    pub full_name: String,
    pub organization_category: String,
    pub organization_name: String,
    pub request_message: String,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub created_at: String,
}

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub code: String,
    pub name_tr: String,
    pub name_en: String,
    pub price_try: f64,
    pub period_label_tr: String,
    pub period_label_en: String,
    pub features_tr: Vec<String>,
    pub features_en: Vec<String>,
    pub is_featured: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommerceProduct {
    pub id: i64,
    pub sku: String,
    /// `book`, `report`, `photo_license`, `single_article`
    pub product_type: String,
    pub title_tr: String,
    pub title_en: String,
    pub author_or_source: String,
    pub description_tr: String,
    pub description_en: String,
    pub price_try: f64,
    pub cover_image: String,
    pub file_resolution_or_pages: String,
    pub is_available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderInvoice {
    pub id: i64,
    pub invoice_no: String,
    pub user_email: String,
    pub full_name: String,
    pub organization_name: String,
    pub item_type: String,
    pub item_code: String,
    pub item_title: String,
    pub amount_try: f64,
    pub payment_status: String,
    pub created_at: String,
    pub pdf_watermark_code: String,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeoRedirectRule {
    pub id: i64,
    pub old_path: String,
    pub new_path: String,
    pub status_code: u16,
    pub hit_count: i64,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiteSetting {
    pub key: String,
    pub value_tr: String,
    pub value_en: String,
    pub setting_group: String,
    pub updated_at: String,
}

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
pub struct SecurityAuditLog {
    pub id: i64,
    pub client_ip: String,
    pub request_path: String,
    pub event_type: String,
    pub threat_level: String,
    pub details: String,
    pub blocked: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchFilterQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub ssb_tag: Option<String>,
    pub content_type: Option<String>,
    pub source_group: Option<String>,
    pub issue_number: Option<i32>,
    pub visibility_level: Option<i32>,
    pub is_sponsored: Option<bool>,
    pub sort_by: Option<String>,
    pub lang: Option<String>,
    pub limit: Option<usize>,
    #[serde(default)]
    pub page: Option<usize>,
    #[serde(default)]
    pub per_page: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FacetBucket {
    pub code: String,
    pub label_tr: String,
    pub label_en: String,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchFacetResponse {
    pub query: SearchFilterQuery,
    pub total_matches: usize,
    pub execution_time_us: u128,
    pub items: Vec<ContentItem>,
    pub category_facets: Vec<FacetBucket>,
    pub ssb_facets: Vec<FacetBucket>,
    pub issue_facets: Vec<FacetBucket>,
    pub source_facets: Vec<FacetBucket>,
}

/// Unified paginated query result returned by CentralContentQueryEngine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentQueryResult {
    pub items: Vec<ContentItem>,
    pub total_count: usize,
    pub current_page: usize,
    pub per_page: usize,
    pub total_pages: usize,
    pub filters_applied: SearchFilterQuery,
    pub execution_time_us: u128,
    pub cache_hit: bool,
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
