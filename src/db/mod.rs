pub mod constants;
pub mod query_helpers;
pub mod repo;
pub mod schema;
pub mod seed;

use crate::models::{
    AdCampaign, CategoryNode, CommerceProduct, ContentItem, MagazineIssue, SeoRedirectRule,
    SiteSetting, SsbInterestTag, SubscriptionPlan, SystemNotification, UserAccount,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct L1CacheSnapshot {
    pub categories: Vec<CategoryNode>,
    pub ssb_tags: Vec<SsbInterestTag>,
    pub contents: Vec<ContentItem>,
    pub issues: Vec<MagazineIssue>,
    pub ads: Vec<AdCampaign>,
    pub site_settings: Vec<SiteSetting>,
    pub notifications: Vec<SystemNotification>,
    pub seo_redirects: Vec<SeoRedirectRule>,
    pub subscription_plans: Vec<SubscriptionPlan>,
    pub commerce_products: Vec<CommerceProduct>,
    pub users: Vec<UserAccount>,
    pub last_refreshed_us: u128,
}

#[derive(Clone, Debug)]
pub struct Database {
    pub db_path: String,
    pub pool: Pool<SqliteConnectionManager>,
    pub l1_cache: Arc<RwLock<Option<L1CacheSnapshot>>>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self, String> {
        if let Some(parent) = Path::new(db_path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Veritabanı dizini oluşturulamadı ({:?}): {}", parent, e))?;
            }
        }

        let manager = SqliteConnectionManager::file(db_path)
            .with_init(|conn| {
                conn.execute_batch(
                    "PRAGMA journal_mode = WAL;
                     PRAGMA synchronous = NORMAL;
                     PRAGMA foreign_keys = ON;
                     PRAGMA busy_timeout = 5000;",
                )?;
                Ok(())
            });

        let pool = Pool::builder()
            .max_size(16)
            .min_idle(Some(4))
            .build(manager)
            .map_err(|e| format!("Connection pool oluşturulamadı: {}", e))?;

        let db = Self {
            db_path: db_path.to_string(),
            pool,
            l1_cache: Arc::new(RwLock::new(None)),
        };

        let mut conn = db
            .open_conn()
            .map_err(|e| format!("SQLite bağlantı hatası: {}", e))?;
        schema::init_schema(&conn).map_err(|e| format!("Şema başlatma hatası: {}", e))?;

        // Phase 5: Run rusqlite_migration for schema version tracking
        {
            use rusqlite_migration::{Migrations, M};
            let migrations = Migrations::new(vec![
                M::up(include_str!("../../migrations/001_initial_schema.sql")),
                M::up(include_str!("../../migrations/002_fts_sync.sql")),
                M::up(include_str!("../../migrations/003_data_fixes.sql")),
            ]);
            if let Err(e) = migrations.to_latest(&mut conn) {
                tracing::warn!("Migration uyarısı (non-fatal): {}", e);
            }
        }

        let content_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM contents", [], |r| r.get(0))
            .unwrap_or(0);

        if content_count == 0 {
            seed::seed_initial_data(&conn)
                .map_err(|e| format!("Başlangıç verisi (seed) yükleme hatası: {}", e))?;
        } else {
            let settings_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM site_settings", [], |r| r.get(0))
                .unwrap_or(0);
            if settings_count == 0 {
                seed::seed_site_settings_and_notifications(&conn)
                    .map_err(|e| format!("Site ayarları ve bildirim seed hatası: {}", e))?;
            }
            // Data fixes (email typo, RBF fallback ad) are now in migrations/003_data_fixes.sql
        }

        drop(conn);
        db.refresh_l1_cache();

        Ok(db)
    }

    pub fn open_conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>, r2d2::Error> {
        self.pool.get()
    }
}
