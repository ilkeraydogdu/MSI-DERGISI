use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub app_name: String,
    pub app_env: String,
    pub host: String,
    pub port: u16,
    pub primary_domain: String,
    pub news_domain: String,
    pub public_url: String,
    pub static_dir: String,
    pub sqlite_db_path: String,
    #[allow(dead_code)]
    pub mariadb_database: String,
    pub session_secret: String,
    pub watermark_secret: String,
    pub admin_default_user: String,
    pub admin_default_pass: String,
    pub ad_max_quota_per_keyword: usize,
    pub paytr_merchant_id: String,
    pub paytr_merchant_key: String,
    pub paytr_merchant_salt: String,
    pub paytr_test_mode: bool,
    pub smtp_from: String,
    pub sendmail_bin: String,
    pub mail_delivery_enabled: bool,
    pub rate_limit_max_requests_per_minute: u32,
    pub rate_limit_window_secs: i64,
    pub paytr_api_url: String,
    pub social_media_urls: Vec<String>,
    pub legacy_seed_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        let default_static_dir = env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| ".".to_string());

        Self {
            app_name: env::var("APP_NAME").unwrap_or_else(|_| {
                "MSI Dergisi & MSI TDR - Savunma Sanayii ve Havacılık Platformu".to_string()
            }),
            app_env: env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8090),
            primary_domain: env::var("PRIMARY_DOMAIN")
                .unwrap_or_else(|_| "msidergisi.com".to_string()),
            news_domain: env::var("NEWS_DOMAIN")
                .unwrap_or_else(|_| "savunmahaber.com".to_string()),
            public_url: env::var("PUBLIC_URL").unwrap_or_else(|_| {
                "https://trusting-brahmagupta.194-110-169-165.plesk.page".to_string()
            }),
            static_dir: env::var("STATIC_DIR")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or(default_static_dir),
            sqlite_db_path: env::var("SQLITE_DB_PATH").unwrap_or_else(|_| {
                "data/msi_platform.db".to_string()
            }),
            mariadb_database: env::var("MARIADB_DATABASE")
                .unwrap_or_else(|_| "msi_dergisi_db".to_string()),
            session_secret: env::var("SESSION_SECRET")
                .unwrap_or_else(|_| "msi-umsa-2026-tactical-secret-key".to_string()),
            watermark_secret: env::var("WATERMARK_SECRET")
                .unwrap_or_else(|_| "UMSA-MSI-DRM-WATERMARK-2026".to_string()),
            admin_default_user: env::var("ADMIN_DEFAULT_USER")
                .unwrap_or_else(|_| "operator@msidergisi.com".to_string()),
            admin_default_pass: env::var("ADMIN_DEFAULT_PASS")
                .unwrap_or_else(|_| "MsiUmsa2026!".to_string()),
            ad_max_quota_per_keyword: env::var("AD_MAX_QUOTA_PER_KEYWORD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            paytr_merchant_id: env::var("PAYTR_MERCHANT_ID").unwrap_or_default(),
            paytr_merchant_key: env::var("PAYTR_MERCHANT_KEY").unwrap_or_default(),
            paytr_merchant_salt: env::var("PAYTR_MERCHANT_SALT").unwrap_or_default(),
            paytr_test_mode: env::var("PAYTR_TEST_MODE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            smtp_from: env::var("SMTP_FROM")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "MSI Dergisi <noreply@msidergisi.com>".to_string()),
            sendmail_bin: env::var("SENDMAIL_BIN")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "/usr/sbin/sendmail".to_string()),
            mail_delivery_enabled: env::var("MAIL_DELIVERY_ENABLED")
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                .unwrap_or(false),
            rate_limit_max_requests_per_minute: env::var("RATE_LIMIT_MAX_RPM")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(600),
            rate_limit_window_secs: env::var("RATE_LIMIT_WINDOW_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            paytr_api_url: env::var("PAYTR_API_URL").unwrap_or_else(|_| {
                "https://www.paytr.com/odeme/api/get-token".to_string()
            }),
            social_media_urls: env::var("SOCIAL_MEDIA_URLS")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_else(|_| vec![
                    format!("https://{}", env::var("PRIMARY_DOMAIN").unwrap_or_else(|_| "msidergisi.com".to_string())),
                    format!("https://{}", env::var("NEWS_DOMAIN").unwrap_or_else(|_| "savunmahaber.com".to_string())),
                    "https://www.linkedin.com/company/msi-dergisi".to_string(),
                    "https://x.com/MSI_Dergisi".to_string(),
                    "https://twitter.com/MSI_Dergisi".to_string(),
                    "https://www.instagram.com/msidergisi".to_string(),
                    "https://www.facebook.com/MSI.Dergisi".to_string(),
                    "https://www.youtube.com/c/msidergisi".to_string(),
                ]),
            legacy_seed_secret: env::var("LEGACY_SEED_SECRET")
                .or_else(|_| env::var("SESSION_SECRET"))
                .unwrap_or_else(|_| "CHANGE-ME-LEGACY-SEED-SECRET".to_string()),
        }
    }

    /// In production, fail-fast if critical secrets are using hardcoded insecure defaults.
    /// Panics with a clear message listing every missing env var so operators fix it immediately.
    pub fn validate_production_secrets(&self) {
        let is_prod = self.app_env == "production" || self.app_env == "prod";
        if !is_prod {
            return;
        }

        let mut missing: Vec<&str> = Vec::new();

        if std::env::var("SESSION_SECRET").is_err() {
            missing.push("SESSION_SECRET");
        }
        if std::env::var("WATERMARK_SECRET").is_err() {
            missing.push("WATERMARK_SECRET");
        }
        if std::env::var("ADMIN_DEFAULT_PASS").is_err() {
            missing.push("ADMIN_DEFAULT_PASS");
        }

        if !missing.is_empty() {
            panic!(
                "FATAL: Production environment detected (APP_ENV={}) but the following \
                 required secret env vars are not set: [{}]. \
                 Refusing to start with insecure hardcoded defaults. \
                 Set these env vars and restart.",
                self.app_env,
                missing.join(", ")
            );
        }
    }
}
