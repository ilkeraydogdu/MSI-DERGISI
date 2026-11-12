#![allow(dead_code)]
//! Centralized Test Infrastructure for MSI Dergisi Platform
//!
//! Provides:
//! - Ephemeral test database setup/teardown (unique per test, auto-cleaned)
//! - Mock AppState builder with seeded data
//! - Fixture data generators (seed-based, no hardcoded mock data)
//! - Common assertion helpers for HTML output and JSON responses

use msi_dergisi_platform::config::AppConfig;
use msi_dergisi_platform::controllers::AppState;
use msi_dergisi_platform::db::Database;
use msi_dergisi_platform::engine::translation_engine::TranslationCache;
use msi_dergisi_platform::models::*;
use std::sync::Arc;

/// Creates a unique ephemeral SQLite database path for test isolation.
/// Each call generates a distinct path to prevent cross-test contamination.
pub fn test_db_path(label: &str) -> String {
    let pid = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("/tmp/msi_test_{}_{}_{}.db", label, pid, ts)
}

/// Creates a test AppConfig with safe defaults for unit/integration testing.
pub fn test_config(db_path: &str) -> AppConfig {
    AppConfig {
        app_name: "MSI Test".into(),
        app_env: "test".into(),
        host: "127.0.0.1".into(),
        port: 0,
        primary_domain: "test.msidergisi.com".into(),
        news_domain: "test.savunmahaber.com".into(),
        public_url: "https://test.msidergisi.com".into(),
        static_dir: "/tmp/msi_test_static".into(),
        sqlite_db_path: db_path.to_string(),
        mariadb_database: "test_db".into(),
        session_secret: "test-session-secret-key-for-unit-tests".into(),
        watermark_secret: "TEST-WATERMARK-SECRET".into(),
        admin_default_user: "admin@test.msidergisi.com".into(),
        admin_default_pass: "TestAdmin2026!".into(),
        ad_max_quota_per_keyword: 3,
        paytr_merchant_id: "TEST_MERCHANT_ID".into(),
        paytr_merchant_key: "test_merchant_key_123".into(),
        paytr_merchant_salt: "test_salt_abc".into(),
        paytr_test_mode: true,
        smtp_from: "test@msidergisi.com".into(),
        sendmail_bin: "/bin/true".into(),
        mail_delivery_enabled: false,
        rate_limit_max_requests_per_minute: 600,
        rate_limit_window_secs: 60,
        paytr_api_url: "https://www.paytr.com/odeme/api/get-token".into(),
        social_media_urls: vec!["https://test.msidergisi.com".into()],
        legacy_seed_secret: "test-legacy-seed-secret".into(),
    }
}

/// Builds a fully initialized AppState with an ephemeral database and seeded data.
/// The database is created fresh with schema + seed data on each call.
pub fn build_test_state(label: &str) -> (AppState, String) {
    let db_path = test_db_path(label);
    let config = test_config(&db_path);
    let database = Database::new(&db_path).expect("Failed to create test database");

    let translations = TranslationCache::new();
    // Load translations from DB if available; empty cache is fine for most tests
    let all_translations = database.load_all_translations();
    translations.load(all_translations);

    let state = AppState {
        config: Arc::new(config),
        db: database,
        translations,
    };

    (state, db_path)
}

/// Cleans up an ephemeral test database file.
pub fn cleanup_test_db(db_path: &str) {
    let _ = std::fs::remove_file(db_path);
    // Also remove WAL and SHM files if they exist
    let _ = std::fs::remove_file(format!("{}-wal", db_path));
    let _ = std::fs::remove_file(format!("{}-shm", db_path));
}

// ---------------------------------------------------------------------------
// Fixture Data Generators (seed-based, deterministic)
// ---------------------------------------------------------------------------

/// Generates a deterministic ContentItem from a seed index.
pub fn fixture_content(seed: i64) -> ContentItem {
    ContentItem {
        id: seed,
        slug: format!("test-article-{}", seed),
        title_tr: format!("Test Makalesi {}", seed),
        title_en: format!("Test Article {}", seed),
        summary_tr: format!("Bu test makalesi {} özetidir.", seed),
        summary_en: format!("This is the summary of test article {}.", seed),
        body_tr: format!(
            "<p>Test makalesi {} birinci cümlesi. İkinci cümle devam ediyor. Üçüncü cümle burada.</p>",
            seed
        ),
        body_en: format!(
            "<p>Test article {} first sentence. Second sentence continues. Third sentence here.</p>",
            seed
        ),
        content_type: if seed % 3 == 0 {
            "ozel-haberler".into()
        } else if seed % 3 == 1 {
            "haberler".into()
        } else {
            "makaleler-analizler".into()
        },
        source_group: if seed % 2 == 0 {
            "dergi-icerigi".into()
        } else {
            "gunluk-haber".into()
        },
        cover_image: format!("images/test/cover-{}.webp", seed),
        author_name: format!("Yazar {}", seed),
        author_title: "Test Editörü".into(),
        issue_number: Some(280 + (seed % 10) as i32),
        published_at: format!("2026-{:02}-{:02} 10:00:00", (seed % 12) + 1, (seed % 28) + 1),
        read_time_min: 3 + (seed % 10) as i32,
        visibility_level: (seed % 3) as i32,
        single_price_try: 50.0 + (seed as f64 * 10.0),
        is_sponsored: seed % 5 == 0,
        sponsor_brand: if seed % 5 == 0 {
            Some(format!("Sponsor {}", seed))
        } else {
            None
        },
        sponsor_legal_label: if seed % 5 == 0 {
            Some("SPONSORLU İÇERİK".into())
        } else {
            None
        },
        is_featured: seed % 4 == 0,
        video_url: if seed % 7 == 0 {
            Some("https://youtube.com/watch?v=test".into())
        } else {
            None
        },
        view_count: seed * 100,
        true_read_count: seed * 50,
        placements: vec![
            if seed % 2 == 0 {
                "ozel-haberler".into()
            } else {
                "haberler".into()
            },
            "kapak-dosyalari".into(),
        ],
        ssb_tags: vec![
            "hava-araclari".into(),
            if seed % 2 == 0 {
                "radarlar-ve-sensorler".into()
            } else {
                "deniz-araclari".into()
            },
        ],
    }
}

/// Generates a deterministic UserAccount from a seed index.
pub fn fixture_user(seed: i64) -> UserAccount {
    let roles = ["reader", "subscriber", "admin", "site_operator", "editor"];
    let orgs = ["MSB", "SSB", "ASELSAN", "TUSAŞ", "Roketsan"];
    let tiers = ["free", "e-dergi", "basili", "kurumsal", "vip"];

    UserAccount {
        id: seed,
        email: format!("user{}@test.com", seed),
        full_name: format!("Test Kullanıcı {}", seed),
        role: roles[(seed as usize) % roles.len()].to_string(),
        organization_category: orgs[(seed as usize) % orgs.len()].to_string(),
        organization_name: format!("Kurum {}", seed),
        job_title: "Test Unvanı".into(),
        is_gov_verified: seed % 3 == 0,
        subscription_tier: tiers[(seed as usize) % tiers.len()].to_string(),
        subscription_expires_at: Some("2027-06-30 23:59:59".into()),
        reminders_halted: false,
        kvkk_consent_at: "2026-01-01 00:00:00".into(),
        kvkk_consent_ip: "127.0.0.1".into(),
        kvkk_consent_hash: format!("hash_{}", seed),
        email_consent: true,
        ssb_interests: vec!["hava-araclari".into(), "radarlar-ve-sensorler".into()],
        created_at: "2026-01-01 00:00:00".into(),
    }
}

/// Generates a subscriber user with active subscription.
pub fn fixture_subscriber_user(seed: i64) -> UserAccount {
    let mut user = fixture_user(seed);
    user.role = "subscriber".into();
    user.subscription_tier = "e-dergi".into();
    user.is_gov_verified = false;
    user
}

/// Generates an admin/operator user.
pub fn fixture_admin_user(seed: i64) -> UserAccount {
    let mut user = fixture_user(seed);
    user.role = "admin".into();
    user.email = format!("admin{}@test.msidergisi.com", seed);
    user.organization_category = "MSB".into();
    user.is_gov_verified = true;
    user
}

/// Generates a government-verified public sector user.
pub fn fixture_gov_user(seed: i64) -> UserAccount {
    let mut user = fixture_user(seed);
    user.email = format!("memur{}@msb.gov.tr", seed);
    user.organization_category = "MSB".into();
    user.is_gov_verified = true;
    user.subscription_tier = "e-dergi".into();
    user
}

/// Generates a free-tier reader without subscription.
pub fn fixture_free_user(seed: i64) -> UserAccount {
    let mut user = fixture_user(seed);
    user.role = "reader".into();
    user.subscription_tier = "free".into();
    user.is_gov_verified = false;
    user.subscription_expires_at = None;
    user
}

// ---------------------------------------------------------------------------
// Assertion Helpers
// ---------------------------------------------------------------------------

/// Asserts that an HTML string contains expected substrings.
pub fn assert_html_contains(html: &str, expected: &[&str]) {
    for needle in expected {
        assert!(
            html.contains(needle),
            "HTML should contain '{}' but it was not found.\nHTML snippet (first 500 chars): {}",
            needle,
            &html[..html.len().min(500)]
        );
    }
}

/// Asserts that an HTML string does NOT contain any of the forbidden substrings.
pub fn assert_html_not_contains(html: &str, forbidden: &[&str]) {
    for needle in forbidden {
        assert!(
            !html.contains(needle),
            "HTML should NOT contain '{}' but it was found.",
            needle
        );
    }
}

/// Asserts that a JSON string is valid and contains expected keys.
pub fn assert_json_contains(json_str: &str, expected_keys: &[&str]) {
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).expect("Response should be valid JSON");
    for key in expected_keys {
        assert!(
            parsed.get(key).is_some(),
            "JSON should contain key '{}' but it was missing. Keys present: {:?}",
            key,
            parsed.as_object().map(|o| o.keys().collect::<Vec<_>>())
        );
    }
}

/// Asserts that a sitemap XML is well-formed and contains required elements.
pub fn assert_valid_sitemap(xml: &str) {
    assert!(
        xml.starts_with("<?xml"),
        "Sitemap must start with XML declaration"
    );
    assert!(
        xml.contains("<urlset") || xml.contains("<sitemapindex"),
        "Sitemap must contain <urlset> or <sitemapindex>"
    );
    assert!(
        xml.contains("</urlset>") || xml.contains("</sitemapindex>"),
        "Sitemap must be closed"
    );
}

/// Asserts that robots.txt contains required directives.
pub fn assert_valid_robots_txt(content: &str) {
    assert!(
        content.contains("User-agent:"),
        "robots.txt must contain User-agent directive"
    );
    assert!(
        content.contains("Sitemap:"),
        "robots.txt must contain Sitemap directive"
    );
}