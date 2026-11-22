use serde::{Deserialize, Serialize};

fn default_content_status() -> String {
    "published".to_string()
}

fn default_en_status() -> String {
    "none".to_string()
}

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
    /// Editorial lifecycle status: `published`, `pending_review`, `draft`, `rejected`, `archived`
    #[serde(default = "default_content_status")]
    pub status: String,
    /// English translation approval lifecycle: `none`, `auto_draft`, `approved`
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
    #[serde(default)]
    pub target_institutions: Vec<String>,
}

impl Default for ContentItem {
    fn default() -> Self {
        Self {
            id: 0,
            slug: String::new(),
            title_tr: String::new(),
            title_en: String::new(),
            summary_tr: String::new(),
            summary_en: String::new(),
            body_tr: String::new(),
            body_en: String::new(),
            content_type: String::new(),
            source_group: String::new(),
            cover_image: String::new(),
            author_name: String::new(),
            author_title: String::new(),
            issue_number: None,
            published_at: String::new(),
            read_time_min: 5,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: false,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: false,
            video_url: None,
            view_count: 0,
            true_read_count: 0,
            placements: Vec::new(),
            ssb_tags: Vec::new(),
            status: default_content_status(),
            en_status: default_en_status(),
            en_translated_at: None,
            en_approved_by: None,
            en_approved_at: None,
            tr_source_hash: None,
            target_institutions: Vec::new(),
        }
    }
}

/// Approval workflow request for content review lifecycle.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: i64,
    pub content_id: i64,
    pub requested_by: i64,
    pub requested_at: String,
    pub status: String,
    pub reviewed_by: Option<i64>,
    pub reviewed_at: Option<String>,
    pub review_note: Option<String>,
    #[serde(default = "default_en_status")]
    pub previous_en_status: String,
    #[serde(default)]
    pub source_action: String,
    #[serde(default)]
    pub target_en_status: String,
}

/// Unified paginated query result returned by CentralContentQueryEngine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentQueryResult {
    pub items: Vec<ContentItem>,
    pub total_count: usize,
    pub current_page: usize,
    pub per_page: usize,
    pub total_pages: usize,
    pub filters_applied: super::search::SearchFilterQuery,
    pub execution_time_us: u128,
    pub cache_hit: bool,
}