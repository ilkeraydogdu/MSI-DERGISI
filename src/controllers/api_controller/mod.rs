use axum::{
    body::Bytes,
    extract::{Extension, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode, Uri},
    response::{Html, IntoResponse, Redirect, Response},
    Form, Json,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path as FsPath;
use std::sync::RwLock;

use crate::controllers::{web_controller, AppState};
use crate::engine::{
    api_response::{self, error_codes, msg_keys, ApiResponse, AppError},
    content_query_engine::CentralContentQueryEngine,
    security_engine::{self, escape_html, CspNonce, LoginCheckResult},
    validation_engine,
};
use crate::extractors::{
    operator_user::{is_staff_role, OperatorUser},
    pagination::PaginationParams,
    require_permission::check_permission,
};
use crate::models::{
    AdCampaign, AnalyticsEvent, CategoryNode, ContentItem, CorporateVerificationRequest,
    NewsletterCampaign, SeoRedirectRule, SiteSetting, SsbInterestTag, UserAccount,
};
use crate::services::{
    ad_service, analytics_service, auth_service, content_service, edergi_pdf_service,
    email_service, media_service, monitoring_service, payment_service, revenue_analytics_service,
    subscription_service, totp_service, user_analytics_service,
};


pub mod helpers;
pub mod public;
pub mod auth;
pub mod totp;
pub mod password_reset;
pub mod email_verification;
pub mod unsubscribe;
pub mod account;
pub mod commerce;
pub mod engagement;
pub mod admin_cms;
pub mod admin_editorial;
pub mod admin_analytics;
pub mod admin_ops;
pub mod admin_system;
pub mod admin_users_roles;
pub mod admin_issues;
pub mod admin_commerce;
pub mod approvals;
pub mod monitoring;
pub mod admin_i18n;
use helpers::*;
pub use public::*;
pub use auth::*;
pub use totp::*;
pub use password_reset::*;
pub use email_verification::*;
pub use unsubscribe::*;
pub use account::*;
pub use commerce::*;
pub use engagement::*;
pub use admin_cms::*;
pub use admin_editorial::*;
pub use admin_analytics::*;
pub use admin_ops::*;
pub use admin_system::*;
pub use admin_users_roles::*;
pub use admin_issues::*;
pub use admin_commerce::*;
pub use approvals::*;
pub use monitoring::*;
pub use admin_i18n::*;
