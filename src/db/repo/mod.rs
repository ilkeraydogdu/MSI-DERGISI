use crate::db::{Database, L1CacheSnapshot};
use crate::models::{
    AdCampaign, AnalyticsEvent, CategoryNode, CommerceProduct, ContentItem,
    CorporateVerificationRequest, MagazineIssue, MagazinePageSpread, NewsletterCampaign,
    OrderInvoice, RecommendationRule, RenewalReminderLog, SecurityAuditLog, SeoRedirectRule,
    SiteSetting, SsbInterestTag, SubscriptionPlan, SystemNotification, UserAccount,
};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::time::Instant;


mod l1_cache;
mod taxonomy;
mod contents;
mod search;
mod issues;
mod users;
mod user_analytics;
mod corporate;
mod ads;
mod analytics;
mod commerce;
mod reminders;
mod newsletters;
mod redirects;
mod settings;
mod security_audit;
mod translations;
mod audit_log;
mod cache_ops;
mod content_versions;
mod custom_fields;
mod media;
mod system_health;
mod approvals;
mod monitoring;
mod tokens;
mod roles;
mod i18n_glossary;
pub use i18n_glossary::*;
mod editorial_studio;
pub use editorial_studio::{
    parse_editorial_content_id, DueScheduledArticle, EditorialArticleListResponse,
    EditorialArticleQuery, EditorialMediaInsert, EditorialStats, EditorialUpsertOutcome,
};
