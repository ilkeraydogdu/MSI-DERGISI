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