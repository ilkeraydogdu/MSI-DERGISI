pub mod admin_view;
pub mod api_controller;
pub mod web_controller;

use crate::config::AppConfig;
use crate::db::Database;
use crate::engine::translation_engine::TranslationCache;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: Database,
    pub translations: TranslationCache,
}

