use std::env;

/// Returns `true` if `seg` represents the retired `msi-komuta` admin slug
/// (case-insensitive, ignoring slashes, hyphens, underscores, whitespace,
/// or percent-encoded `%20`/`%2d`/`%5f`, and checking `/` sub-segments).
pub fn is_retired_admin_segment(seg: &str) -> bool {
    let trimmed = seg.trim_matches(|c: char| c.is_whitespace() || c == '/');
    if trimmed.is_empty() {
        return false;
    }
    let strip_part = |part: &str| -> String {
        part.trim()
            .to_ascii_lowercase()
            .replace("%20", "")
            .replace("%2d", "")
            .replace("%5f", "")
            .chars()
            .filter(|c| !matches!(c, '-' | '_' | ' ' | '\t' | '\r' | '\n'))
            .collect()
    };
    if strip_part(trimmed) == "msikomuta" {
        return true;
    }
    trimmed.split('/').any(|part| strip_part(part) == "msikomuta")
}

/// Normalizes `ADMIN_PATH_SECRET`, mapping empty values, multi-segment slash
/// paths, `"podmin"` case variants, and any retired `msi-komuta` variant to
/// `"podmin"`, while preserving valid single-segment custom secrets (e.g., E2E
/// `e2e-admin-*`).
pub fn normalize_admin_path_secret(raw: &str) -> String {
    let trimmed = raw.trim_matches(|c: char| c.is_whitespace() || c == '/');
    if trimmed.is_empty()
        || trimmed.contains('/')
        || is_retired_admin_segment(trimmed)
        || trimmed.eq_ignore_ascii_case("podmin")
    {
        "podmin".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Clone)]
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
    pub edergi_dir: String,
    pub session_secret: String,
    pub watermark_secret: String,
    pub admin_default_user: String,
    pub admin_default_pass: String,
    pub admin_path_secret: String,
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
    pub monitoring_enabled: bool,
    pub monitoring_vhosts_path: String,
    pub monitoring_nginx_log_path: String,
    pub monitoring_fail2ban_log_path: String,
    pub monitoring_cert_path: String,
    pub monitoring_fast_interval_secs: u64,
    pub monitoring_medium_interval_secs: u64,
    pub monitoring_slow_interval_secs: u64,
    pub monitoring_retention_days: u32,
    pub monitoring_node_ports: Vec<u16>,
    pub monitoring_redis_port: u16,
    pub monitoring_smtp_port: u16,
    pub indexnow_api_url: String,
    pub backup_enabled: bool,
    pub backup_dir: String,
    pub backup_retention_days: u32,
    pub backup_archive_dir: String,
    pub backup_incremental_interval_secs: u64,
    pub backup_encryption_key: String,
    pub s3_backup_endpoint: String,
    pub s3_backup_bucket: String,
    pub s3_backup_region: String,
    pub s3_backup_access_key: String,
    pub s3_backup_secret_key: String,
    pub s3_backup_prefix: String,
    pub newsletter_poll_interval_secs: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        let default_static_dir = env::current_dir()
            .ok()
            .map(|p| p.join("public"))
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "public".to_string());

        let app_env = env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
        let sqlite_db_path = env::var("SQLITE_DB_PATH")
            .unwrap_or_else(|_| "storage/db/msi_platform.db".to_string());
        let default_backup_dir = std::path::Path::new(&sqlite_db_path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.join("backups").to_string_lossy().to_string())
            .unwrap_or_else(|| "storage/db/backups".to_string());
        let backup_dir = env::var("BACKUP_DIR")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(default_backup_dir);
        let default_archive_dir = format!("{}/offsite", backup_dir.trim_end_matches('/'));

        Self {
            app_name: env::var("APP_NAME").unwrap_or_else(|_| {
                "MSI Dergisi & MSI TDR - Savunma Sanayii ve Havacılık Platformu".to_string()
            }),
            app_env: app_env.clone(),
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
            sqlite_db_path,
            edergi_dir: env::var("EDERGI_DIR")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "storage/edergi".to_string()),
            session_secret: env::var("SESSION_SECRET")
                .unwrap_or_else(|_| "dev-only-session-secret-change-in-prod".to_string()),
            watermark_secret: env::var("WATERMARK_SECRET")
                .unwrap_or_else(|_| "dev-only-watermark-secret-change-in-prod".to_string()),
            admin_default_user: env::var("ADMIN_DEFAULT_USER")
                .unwrap_or_else(|_| "operator@msidergisi.com".to_string()),
            admin_default_pass: env::var("ADMIN_DEFAULT_PASS").unwrap_or_default(),
            admin_path_secret: env::var("ADMIN_PATH_SECRET")
                .map(|v| normalize_admin_path_secret(&v))
                .unwrap_or_else(|_| "podmin".to_string()),
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
                .unwrap_or_else(|| "[REDACTED_SESSION_FALLBACK]".to_string()),
            sendmail_bin: env::var("SENDMAIL_BIN")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "[REDACTED_SESSION_FALLBACK]".to_string()),
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
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_else(|_| {
                    vec![
                        format!(
                            "https://{}",
                            env::var("PRIMARY_DOMAIN")
                                .unwrap_or_else(|_| "msidergisi.com".to_string())
                        ),
                        format!(
                            "https://{}",
                            env::var("NEWS_DOMAIN")
                                .unwrap_or_else(|_| "savunmahaber.com".to_string())
                        ),
                        "https://www.linkedin.com/company/msi-dergisi".to_string(),
                        "https://x.com/MSI_Dergisi".to_string(),
                        "https://twitter.com/MSI_Dergisi".to_string(),
                        "https://www.instagram.com/msidergisi".to_string(),
                        "https://www.facebook.com/MSI.Dergisi".to_string(),
                        "https://www.youtube.com/c/msidergisi".to_string(),
                    ]
                }),
            legacy_seed_secret: env::var("LEGACY_SEED_SECRET")
                .or_else(|_| env::var("SESSION_SECRET"))
                .unwrap_or_else(|_| "CHANGE-ME-LEGACY-SEED-SECRET".to_string()),
            monitoring_enabled: env::var("MONITORING_ENABLED")
                .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
                .unwrap_or(true),
            monitoring_vhosts_path: env::var("MONITORING_VHOSTS_PATH")
                .unwrap_or_else(|_| "/var/www/vhosts".to_string()),
            monitoring_nginx_log_path: env::var("MONITORING_NGINX_LOG_PATH")
                .unwrap_or_else(|_| "/var/log/nginx/access.log".to_string()),
            monitoring_fail2ban_log_path: env::var("MONITORING_FAIL2BAN_LOG_PATH")
                .unwrap_or_else(|_| "/var/log/fail2ban.log".to_string()),
            monitoring_cert_path: env::var("MONITORING_CERT_PATH")
                .unwrap_or_else(|_| "/usr/local/psa/var/certificates".to_string()),
            monitoring_fast_interval_secs: env::var("MONITORING_FAST_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15),
            monitoring_medium_interval_secs: env::var("MONITORING_MEDIUM_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            monitoring_slow_interval_secs: env::var("MONITORING_SLOW_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            monitoring_retention_days: env::var("MONITORING_RETENTION_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7),
            monitoring_node_ports: env::var("MONITORING_NODE_PORTS")
                .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
                .unwrap_or_else(|_| vec![3000, 3004, 3010, 3030, 3040, 3056, 3100, 8090]),
            monitoring_redis_port: env::var("MONITORING_REDIS_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6379),
            monitoring_smtp_port: env::var("MONITORING_SMTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(25),
            indexnow_api_url: env::var("INDEXNOW_API_URL")
                .unwrap_or_else(|_| "https://api.indexnow.org/indexnow".to_string()),
            backup_enabled: env::var("BACKUP_ENABLED")
                .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
                .unwrap_or(app_env != "test"),
            backup_dir,
            backup_retention_days: env::var("BACKUP_RETENTION_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(14),
            backup_archive_dir: env::var("BACKUP_ARCHIVE_DIR")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or(default_archive_dir),
            backup_incremental_interval_secs: env::var("BACKUP_INCREMENTAL_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1800),
            backup_encryption_key: env::var("BACKUP_ENCRYPTION_KEY").unwrap_or_default(),
            s3_backup_endpoint: env::var("S3_BACKUP_ENDPOINT").unwrap_or_default(),
            s3_backup_bucket: env::var("S3_BACKUP_BUCKET").unwrap_or_default(),
            s3_backup_region: env::var("S3_BACKUP_REGION")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "eu-central-1".to_string()),
            s3_backup_access_key: env::var("S3_BACKUP_ACCESS_KEY").unwrap_or_default(),
            s3_backup_secret_key: env::var("S3_BACKUP_SECRET_KEY").unwrap_or_default(),
            s3_backup_prefix: env::var("S3_BACKUP_PREFIX")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "msi-backups".to_string()),
            newsletter_poll_interval_secs: env::var("NEWSLETTER_POLL_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
        }
    }

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
                 Refusing to start with insecure hardcoded defaults.",
                self.app_env,
                missing.join(", ")
            );
        }
    }
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("app_name", &self.app_name)
            .field("app_env", &self.app_env)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("primary_domain", &self.primary_domain)
            .field("news_domain", &self.news_domain)
            .field("public_url", &self.public_url)
            .field("static_dir", &self.static_dir)
            .field("sqlite_db_path", &self.sqlite_db_path)
            .field("session_secret", &"[REDACTED]")
            .field("watermark_secret", &"[REDACTED]")
            .field("admin_default_user", &self.admin_default_user)
            .field("admin_default_pass", &"[REDACTED]")
            .field("admin_path_secret", &"[REDACTED]")
            .field("paytr_merchant_id", &self.paytr_merchant_id)
            .field("paytr_merchant_key", &"[REDACTED]")
            .field("paytr_merchant_salt", &"[REDACTED]")
            .field("legacy_seed_secret", &"[REDACTED]")
            .field("monitoring_enabled", &self.monitoring_enabled)
            .field("backup_enabled", &self.backup_enabled)
            .field("backup_dir", &self.backup_dir)
            .field("backup_archive_dir", &self.backup_archive_dir)
            .field("backup_incremental_interval_secs", &self.backup_incremental_interval_secs)
            .field("backup_encryption_key", &"[REDACTED]")
            .field("s3_backup_endpoint", &self.s3_backup_endpoint)
            .field("s3_backup_bucket", &self.s3_backup_bucket)
            .field("s3_backup_region", &self.s3_backup_region)
            .field("s3_backup_prefix", &self.s3_backup_prefix)
            .field("s3_backup_access_key", &"[REDACTED]")
            .field("s3_backup_secret_key", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_msi_komuta_variants_normalize_to_podmin_while_custom_secrets_remain() {
        for retired in [
            "msi-komuta",
            "/MSI-KOMUTA/",
            "msi_komuta",
            "msikomuta",
            "msi%20komuta",
            "msi%2dkomuta",
            "msi%2Dkomuta",
            "msi%5fkomuta",
            "msi%5Fkomuta",
            "msi komuta",
            "msi-komuta/tasarim",
            "podmin/tasarim",
            "PODMIN",
            "  /msi-komuta/  ",
            "",
            "   ",
            "///",
        ] {
            assert_eq!(
                normalize_admin_path_secret(retired),
                "podmin",
                "Expected '{}' to normalize to 'podmin'",
                retired
            );
        }

        assert!(is_retired_admin_segment("msi-komuta"));
        assert!(is_retired_admin_segment("/MSI-KOMUTA/"));
        assert!(is_retired_admin_segment("msi_komuta"));
        assert!(is_retired_admin_segment("msikomuta"));
        assert!(is_retired_admin_segment("msi%20komuta"));
        assert!(is_retired_admin_segment("msi%2dkomuta"));
        assert!(is_retired_admin_segment("msi%2Dkomuta"));
        assert!(is_retired_admin_segment("msi komuta"));
        assert!(is_retired_admin_segment("msi-komuta/tasarim"));
        assert!(!is_retired_admin_segment(""));
        assert!(!is_retired_admin_segment("   "));
        assert!(!is_retired_admin_segment("podmin"));
        assert!(!is_retired_admin_segment("PODMIN"));
        assert!(!is_retired_admin_segment("e2e-admin-abc123"));

        assert_eq!(normalize_admin_path_secret("podmin"), "podmin");
        assert_eq!(normalize_admin_path_secret("/podmin/"), "podmin");
        assert_eq!(normalize_admin_path_secret("PODMIN"), "podmin");
        assert_eq!(normalize_admin_path_secret("podmin/tasarim"), "podmin");
        assert_eq!(
            normalize_admin_path_secret("e2e-admin-abc123"),
            "e2e-admin-abc123"
        );
    }
}
