pub mod constants;
pub mod query_helpers;
pub mod repo;
pub mod schema;
pub mod seed;

use crate::models::{
    AdCampaign, CategoryNode, CommerceProduct, ContentItem, InstitutionCategory, MagazineIssue,
    RecommendationRule, SeoRedirectRule, SiteSetting, SsbInterestTag, SubscriptionPlan,
    SystemNotification, UserAccount,
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
    pub institution_categories: Vec<InstitutionCategory>,
    pub recommendation_rules: Vec<RecommendationRule>,
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

        // Run rusqlite_migration for deterministic schema version tracking (001..017)
        {
            use rusqlite_migration::{Migrations, M};
            let migrations = Migrations::new(vec![
                M::up(include_str!("../../migrations/001_initial_schema.sql")),
                M::up(include_str!("../../migrations/002_fts_sync.sql")),
                M::up(include_str!("../../migrations/003_data_fixes.sql")),
                M::up(include_str!("../../migrations/004_analytics_daily_agg.sql")),
                M::up(include_str!("../../migrations/005_totp_secret.sql")),
                M::up(include_str!("../../migrations/006_approval_workflow.sql")),
                M::up(include_str!("../../migrations/007_monitoring_schema.sql")),
                M::up(include_str!("../../migrations/008_password_reset.sql")),
                M::up(include_str!("../../migrations/009_email_verification.sql")),
                M::up(include_str!("../../migrations/010_unsubscribe_tokens.sql")),
                M::up(include_str!("../../migrations/011_rbac_issues_commerce_cleanup.sql")),
                M::up(include_str!("../../migrations/012_central_i18n_glossary_b2b.sql")),
                M::up(include_str!("../../migrations/013_admin_i18n_keys.sql")),
                M::up(include_str!("../../migrations/014_translation_approval.sql")),
                M::up(include_str!("../../migrations/015_option_b_features.sql")),
                M::up(include_str!("../../migrations/016_spec_parity_and_security.sql")),
                M::up(include_str!("../../migrations/017_editorial_studio.sql")),
            ]);
            migrations
                .to_latest(&mut conn)
                .map_err(|e| format!("Veritabanı migrasyon hatası (001..017): {}", e))?;
        }

        let content_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM contents", [], |r| r.get(0))
            .unwrap_or(0);

        if content_count == 0 {
            seed::seed_initial_data(&conn)
                .map_err(|e| format!("Başlangıç verisi (seed) yükleme hatası: {}", e))?;
            seed::taxonomy::seed_option_b_email_groups(&conn)
                .map_err(|e| format!("Option B e-posta grubu seed hatası: {}", e))?;
            // Sync fresh seed rows with Migration 012 backfills (en_status, composite publication_type, PDF links)
            let _ = conn.execute_batch(
                "UPDATE magazine_issues
                 SET publication_type = CASE
                        WHEN issue_number < 200 OR slug LIKE 'msi-tdr%' THEN 'msi_tdr'
                        ELSE 'msi'
                     END,
                     en_status = CASE
                        WHEN issue_number < 200 OR slug LIKE 'msi-tdr%' THEN 'approved'
                        ELSE en_status
                     END;
                 UPDATE magazine_issues
                 SET uploaded_pdf_file = 'msi-dergisi-sayi-' || issue_number || '.pdf'
                 WHERE publication_type = 'msi'
                   AND issue_number IN (263,266,270,271,273,274,275,276,277,278,279,280,284,285,286)
                   AND (uploaded_pdf_file IS NULL OR TRIM(uploaded_pdf_file) = '');
                 UPDATE magazine_issues
                 SET uploaded_pdf_file = 'msi-tdr-issue-' || issue_number || '.pdf'
                 WHERE publication_type = 'msi_tdr'
                   AND issue_number IN (143,147,149,150,151,152,153)
                   AND (uploaded_pdf_file IS NULL OR TRIM(uploaded_pdf_file) = '');
                 UPDATE contents
                 SET en_status = 'approved'
                 WHERE source_group = 'msi-tdr'
                    OR (TRIM(title_en) <> '' AND TRIM(body_en) <> '' AND title_en <> title_tr AND body_en <> body_tr);",
            );
        } else {
            seed::settings::sync_brand_site_settings(&conn)
                .map_err(|e| format!("Site ayarları senkronizasyon hatası: {}", e))?;
            seed::taxonomy::seed_option_b_categories(&conn)
                .map_err(|e| format!("Option B kategori seed hatası: {}", e))?;
            // Data fixes (email typo, RBF fallback ad) are now in migrations/003_data_fixes.sql
        }

        seed::taxonomy::seed_operational_defaults(&conn)
            .map_err(|e| format!("Operasyonel varsayılanlar seed hatası: {}", e))?;

        drop(conn);

        // Seed RBAC roles and centralized UI + API translations idempotently
        if let Err(e) = seed::seed_user_roles(&db) {
            tracing::warn!("RBAC role seed uyarısı: {}", e);
        }
        if let Err(e) = db.seed_default_translations() {
            tracing::warn!("UI translation seed uyarısı: {}", e);
        }
        if let Err(e) = db.seed_api_translations() {
            tracing::warn!("API translation seed uyarısı: {}", e);
        }

        db.refresh_l1_cache();

        Ok(db)
    }

    pub fn open_conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>, r2d2::Error> {
        self.pool.get()
    }
}
