pub mod advertising;
pub mod analytics;
pub mod category;
pub mod commerce;
pub mod content;
pub mod magazine;
pub mod notification;
pub mod search;
pub mod security;
pub mod seo;
pub mod settings;
pub mod user;

// Re-export all types at crate::models::* for backward compatibility
pub use advertising::{AdCampaign, AdConflictWarning};
pub use analytics::{
    AdAnalyticsReport, AnalyticsEvent, ContentAnalyticsReport, InstitutionBreakdownItem,
    ReaderAnalyticsReport,
};
pub use category::{CategoryNode, InstitutionCategory, SsbInterestTag};
pub use commerce::{CommerceProduct, OrderInvoice, SubscriptionPlan};
pub use content::{ApprovalRequest, ContentItem, ContentQueryResult};
pub use magazine::{MagazineIssue, MagazinePageSpread};
pub use notification::{B2bMonthlyReport, NewsletterCampaign, SystemNotification};
pub use search::{FacetBucket, SearchFacetResponse, SearchFilterQuery};
pub use security::SecurityAuditLog;
pub use seo::{SeoMetaBundle, SeoRedirectRule};
pub use settings::{RecommendationRule, RenewalReminderLog, SiteSetting};
pub use user::{CorporateVerificationRequest, KvkkConsentLog, UserAccount};