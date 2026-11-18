use axum::{
    extract::{Extension, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use chrono::Datelike;
use std::collections::HashMap;
use std::time::Instant;

use crate::controllers::{admin_view, AppState};
use crate::engine::{
    design_engine,
    query_engine::CentralQueryEngine,
    search_engine,
    security_engine::CspNonce,
    seo_router_engine::{self, CentralSeoEngine},
};
use crate::models::{ContentItem, SearchFilterQuery, SeoMetaBundle};
use crate::services::{ad_service, content_service, media_service};

pub mod articles;
pub mod authors;
pub mod feeds;
pub mod helpers;
pub mod home;
pub mod not_found;
pub mod pages;
pub mod redirects;
pub mod search_account;
pub mod store;

pub use articles::{render_article_detail, render_category_page};
pub use authors::{author_slug_from_name, render_author_profile};
pub(crate) use authors::authenticated_reader;
pub use feeds::{
    render_ai_plugin_json, render_apple_touch_icon, render_favicon_ico, render_favicon_svg,
    render_image_sitemap_xml, render_indexnow_key, render_json_feed, render_llms_full_txt,
    render_llms_txt, render_news_sitemap_xml, render_openapi_json, render_opensearch_xml,
    render_robots_txt, render_rss_xml, render_sitemap_xml, render_web_manifest,
};
pub use helpers::detect_language;
pub(crate) use helpers::{detect_domain_mode, typed_response};
pub use home::{render_admin, render_en_home, render_home, render_shared_shell};
#[allow(unused_imports)]
pub(crate) use home::render_shell_with_seo;
pub use not_found::{render_not_found, render_not_found_with_nonce};
pub use pages::{render_coming_soon, render_designed_page};
#[allow(unused_imports)]
pub(crate) use pages::render_configured_page;
pub use redirects::{
    handle_fallback_or_redirect, redirect_account_en, redirect_account_tr, redirect_en_home,
    redirect_legacy_haber_en, redirect_legacy_haber_tr, redirect_legacy_kategori_en,
    redirect_legacy_kategori_tr, redirect_newsletter_unsub_en, redirect_newsletter_unsub_tr,
    redirect_short_corporate_en, redirect_short_corporate_tr, render_legacy_html_page,
    switch_locale,
};
pub use search_account::{
    render_account_page, render_en_search_page, render_reset_password_page, render_search_page,
};
pub use store::{render_corporate_page, render_edergi_kiosk, render_subscriptions_and_store};

#[cfg(test)]
mod tests;
