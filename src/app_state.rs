use crate::config::AppConfig;
use crate::db::Database;
use crate::engine::translation_engine::TranslationCache;
use crate::services::monitoring_service::SharedMonitoringCache;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: Database,
    pub translations: TranslationCache,
    pub monitoring_cache: Option<SharedMonitoringCache>,
}