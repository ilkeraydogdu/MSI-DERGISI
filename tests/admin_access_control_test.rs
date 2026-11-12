//! Integration tests for Admin panel access control and WAF security engine.

mod common;

use common::*;

#[test]
fn admin_waf_blocks_sql_injection_attempts() {
    use msi_dergisi_platform::engine::security_engine::CentralSecurityFilter;

    let result = CentralSecurityFilter::inspect_request_security(
        "/api/v1/admin/contents",
        Some("id=1' OR '1'='1"),
        Some("Mozilla/5.0"),
        "127.0.0.1",
        120, // max_rpm (default)
        60,    // window_secs (default)
    );
    assert!(result.is_err(), "SQL injection in query params should be blocked by WAF");
}

#[test]
fn admin_waf_blocks_xss_in_user_agent() {
    use msi_dergisi_platform::engine::security_engine::CentralSecurityFilter;

    let result = CentralSecurityFilter::inspect_request_security(
        "/admin",
        None,
        Some("<script>alert(document.cookie)</script>"),
        "127.0.0.1",
        120, // max_rpm (default)
        60,  // window_secs (default)
    );
    assert!(result.is_err(), "XSS in User-Agent should be blocked by WAF");
}

#[test]
fn admin_waf_allows_legitimate_requests() {
    use msi_dergisi_platform::engine::security_engine::CentralSecurityFilter;

    let result = CentralSecurityFilter::inspect_request_security(
        "/icerik/kaan-milli-muharip-ucak",
        Some("page=1&lang=tr"),
        Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"),
        "192.168.1.100",
        120, // max_rpm (default)
        60,  // window_secs (default)
    );
    assert!(result.is_ok(), "Legitimate requests should pass WAF inspection");
}

#[test]
fn admin_waf_blocks_path_traversal() {
    use msi_dergisi_platform::engine::security_engine::CentralSecurityFilter;

    let result = CentralSecurityFilter::inspect_request_security(
        "/../../etc/passwd",
        None,
        Some("curl/7.68.0"),
        "10.0.0.1",
        120, // max_rpm (default)
        60,  // window_secs (default)
    );
    assert!(result.is_err(), "Path traversal attempts should be blocked by WAF");
}

#[test]
fn admin_security_headers_are_generated() {
    use msi_dergisi_platform::engine::security_engine::CentralSecurityFilter;

    let headers = CentralSecurityFilter::security_headers_list(1500, None);
    assert!(!headers.is_empty(), "Security headers list should not be empty");

    // Verify key security headers are present
    let header_names: Vec<String> = headers.iter().map(|(k, _)| k.to_ascii_lowercase()).collect();
    assert!(
        header_names.iter().any(|h| h.contains("x-content-type") || h.contains("x-frame") || h.contains("strict-transport")),
        "Security headers should include standard protection headers"
    );
}

#[test]
fn admin_html_sanitizer_preserves_editorial_content() {
    use msi_dergisi_platform::engine::security_engine::sanitize_rich_html;

    let input = r#"<p class="paywall-excerpt"><strong>KAAN</strong> <img src="images/msi/kaan.webp" loading="lazy"></p>"#;
    let output = sanitize_rich_html(input);
    assert_html_contains(&output, &["<p class=\"paywall-excerpt\">", "<strong>KAAN</strong>", "src=\"images/msi/kaan.webp\""]);
}

#[test]
fn admin_html_sanitizer_strips_dangerous_elements() {
    use msi_dergisi_platform::engine::security_engine::sanitize_rich_html;

    let input = r#"<div onclick="alert(1)"><script>evil()</script><a href="javascript:void(0)">Link</a><img onerror="steal()" src="x"></div>"#;
    let output = sanitize_rich_html(input);
    assert_html_not_contains(&output, &["onclick", "<script", "javascript:", "onerror"]);
}

#[test]
fn admin_design_config_validation_rejects_injection() {
    use msi_dergisi_platform::engine::design::config;

    let mut cfg = config::defaults();
    cfg.theme.light.accent = "red;}</style><script>alert(1)</script>".into();
    assert!(cfg.validate().is_err(), "CSS injection in theme config should be rejected");
}

#[test]
fn admin_test_state_initializes_with_seeded_data() {
    let (state, db_path) = build_test_state("admin_access");

    // Verify database has seeded content
    let contents = state.db.get_all_contents();
    assert!(!contents.is_empty(), "Test state should have seeded content");

    // Verify translations cache is loaded
    let tr_val = state.translations.t("tr", "site_name");
    // Should return either a translation or the key itself (fallback)
    assert!(!tr_val.is_empty());

    cleanup_test_db(&db_path);
}