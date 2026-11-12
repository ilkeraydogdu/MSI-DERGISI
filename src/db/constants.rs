//! Central constants for the MSI Dergisi platform.
//! Eliminates magic numbers and hardcoded table/column strings scattered across the codebase.

// =========================================================================
// TABLE NAMES — single source of truth for all SQL references outside schema.rs
// =========================================================================
pub const TABLE_CONTENTS: &str = "contents";
pub const TABLE_CATEGORIES: &str = "categories";
pub const TABLE_SSB_TAGS: &str = "ssb_tags";
pub const TABLE_CONTENT_PLACEMENTS: &str = "content_placements";
pub const TABLE_CONTENT_SSB_TAGS: &str = "content_ssb_tags";
pub const TABLE_MAGAZINE_ISSUES: &str = "magazine_issues";
pub const TABLE_USERS: &str = "users";
pub const TABLE_AD_CAMPAIGNS: &str = "ad_campaigns";
pub const TABLE_SITE_SETTINGS: &str = "site_settings";
pub const TABLE_TRANSLATIONS: &str = "translations";
pub const TABLE_ANALYTICS_EVENTS: &str = "analytics_events";
pub const TABLE_SUBSCRIPTION_PLANS: &str = "subscription_plans";
pub const TABLE_COMMERCE_PRODUCTS: &str = "commerce_products";
pub const TABLE_ORDER_INVOICES: &str = "order_invoices";
pub const TABLE_USER_PURCHASED_ITEMS: &str = "user_purchased_items";
pub const TABLE_NEWSLETTER_CAMPAIGNS: &str = "newsletter_campaigns";
pub const TABLE_SEO_REDIRECTS: &str = "seo_redirects";
pub const TABLE_SYSTEM_NOTIFICATIONS: &str = "system_notifications";
pub const TABLE_SECURITY_AUDIT_LOGS: &str = "security_audit_logs";
pub const TABLE_CONTENT_VERSIONS: &str = "content_versions";
pub const TABLE_MEDIA_LIBRARY: &str = "media_library";
pub const TABLE_ADMIN_AUDIT_LOG: &str = "admin_audit_log";
pub const TABLE_USER_ROLES: &str = "user_roles";
pub const TABLE_CUSTOM_FIELDS: &str = "custom_fields";
pub const TABLE_CACHE_OPERATIONS: &str = "cache_operations";
pub const TABLE_RENEWAL_REMINDERS: &str = "renewal_reminders";
pub const TABLE_CORPORATE_VERIFICATION_REQUESTS: &str = "corporate_verification_requests";
pub const TABLE_RECOMMENDATION_RULES: &str = "recommendation_rules";
pub const TABLE_CONTENTS_FTS: &str = "contents_fts";

// =========================================================================
// PAGINATION & LIMITS
// =========================================================================
pub const DEFAULT_PAGE_SIZE: usize = 24;
pub const MAX_PAGE_SIZE: usize = 100;
pub const DEFAULT_CONTENT_LIMIT: usize = 20;
pub const ADMIN_LIST_LIMIT: usize = 60;
pub const SITEMAP_FULL_LIMIT: usize = 350;
pub const SITEMAP_PREVIEW_LIMIT: usize = 40;
pub const RELATED_CONTENT_LIMIT: usize = 6;
pub const TRENDING_CONTENT_LIMIT: usize = 10;
pub const HOMEPAGE_HERO_LIMIT: usize = 6;
pub const HOMEPAGE_GRID_LIMIT: usize = 12;
pub const AUTHOR_ARTICLES_LIMIT: usize = 20;
pub const SEARCH_MAX_RESULTS: usize = 200;
pub const PDF_ISSUE_PREVIEW_PAGES: usize = 6;
pub const PDF_ISSUE_MAX_PAGES: usize = 8;
pub const NEWSLETTER_RECIPIENT_LIMIT: usize = 50;
pub const CACHE_MAX_ENTRIES: usize = 256;
pub const CACHE_TTL_SECONDS: u64 = 5;

// =========================================================================
// FILE UPLOAD CONSTRAINTS
// =========================================================================
pub const MAX_UPLOAD_SIZE_BYTES: usize = 25 * 1024 * 1024; // 25 MB
pub const ALLOWED_UPLOAD_EXTENSIONS: &[&str] = &[".webp", ".png", ".jpg", ".jpeg", ".svg", ".pdf", ".mp4"];
pub const DEFAULT_ISSUE_COVER_PATH: &str = "images/msi/default-cover.webp";
pub const DEFAULT_AD_SIDEBAR_PATH: &str = "images/msi/default-ad-sidebar.webp";

// =========================================================================
// SECURITY & RATE LIMITING
// =========================================================================
pub const RATE_LIMIT_MAX_REQUESTS_PER_MINUTE: u32 = 120;
pub const RATE_LIMIT_WINDOW_SECONDS: i64 = 60;
pub const RATE_LIMIT_STORE_MAX_ENTRIES: usize = 10_000;
pub const SESSION_TOKEN_EXPIRY_HOURS: u64 = 24;
pub const PASSWORD_MIN_LENGTH: usize = 8;

// =========================================================================
// CONNECTION POOL
// =========================================================================
pub const DB_POOL_MAX_SIZE: u32 = 16;
pub const DB_POOL_MIN_IDLE: u32 = 4;
pub const DB_BUSY_TIMEOUT_MS: u32 = 5000;

// =========================================================================
// SHELL / UI LIMITS (used by shell.rs, web_controller, api_controller)
// =========================================================================
/// Maximum contents shown in shell navigation panels
pub const SHELL_NAV_CONTENT_LIMIT: usize = 20;
/// Maximum trending items in shell sidebar
pub const SHELL_TRENDING_LIMIT: usize = 10;
/// Maximum latest issues shown in shell
pub const SHELL_LATEST_ISSUES_LIMIT: usize = 10;
/// Preview limit for refresh log entries
pub const REFRESH_LOG_PREVIEW_LIMIT: usize = 8;
/// API feed content pagination limit
pub const API_FEED_CONTENT_LIMIT: usize = 20;
/// Related content suggestions per article
pub const RELATED_CONTENT_LIMIT_SHELL: usize = 6;
/// Max filename length for media uploads
pub const MEDIA_FILENAME_MAX_LEN: usize = 64;

// =========================================================================
// PDF GENERATION LIMITS (edergi_pdf_service.rs)
// =========================================================================
/// Lines of summary text included in PDF issue preview
pub const PDF_SUMMARY_LINES_LIMIT: usize = 6;
/// Number of page spreads rendered in PDF
pub const PDF_SPREADS_LIMIT: usize = 8;
/// Max lines for Turkish title in PDF
pub const PDF_TITLE_LINES_TR_LIMIT: usize = 3;
/// Max lines for English title in PDF
pub const PDF_TITLE_LINES_EN_LIMIT: usize = 2;
/// Max body text lines per PDF section
pub const PDF_BODY_LINES_LIMIT: usize = 12;
/// Max table-of-contents lines in PDF
pub const PDF_TOC_LINES_LIMIT: usize = 10;

// =========================================================================
// READER & COMPONENTS LIMITS
// =========================================================================
/// Related articles shown in reader view
pub const READER_RELATED_LIMIT: usize = 3;
/// Highlight items in components panel
pub const COMPONENTS_HIGHLIGHT_LIMIT: usize = 5;
/// Preview spreads in components panel
pub const COMPONENTS_PREVIEW_SPREADS_LIMIT: usize = 4;

// =========================================================================
// SEO / SITEMAP LIMITS (for seo_router_engine.rs audit)
// =========================================================================
/// Featured items on homepage (SEO context)
pub const HOMEPAGE_FEATURED_LIMIT: usize = 10;
/// Newsletter recipients cap for sitemap/SEO contexts
pub const SITEMAP_NEWSLETTER_LIMIT: usize = 50;

// =========================================================================
// ADDITIONAL LIMITS (blocker fixes from sub-agent sweeps)
// =========================================================================
/// Sidebar related-content display limit (post-filter truncate)
pub const SIDEBAR_RELATED_CONTENT_LIMIT: usize = 5;
/// Ticker / latest-headlines items in shell
pub const SHELL_TICKER_HEADLINES_LIMIT: usize = 8;
/// llms.txt full issue listing limit
pub const LLMS_FULL_ISSUE_LIMIT: usize = 25;
/// llms.txt preview issue listing limit
pub const LLMS_PREVIEW_ISSUE_LIMIT: usize = 10;

// =========================================================================
// CONTENT TYPE / FACET SLUG CONSTANTS
// =========================================================================
/// Content type slug for video content
pub const CONTENT_TYPE_VIDEO: &str = "video";
/// Facet slug for multimedia source group
pub const FACET_SLUG_MULTIMEDIA: &str = "multimedya";
/// Facet slug for video source group
pub const FACET_SLUG_VIDEO: &str = "video";