//! OMS/CMS Operator Dashboard & Command Gateway Views (`src/controllers/admin_view.rs`)
//! Renders the CSP-clean HTML5 Operator & B2B Defense Analytics Center (`/{admin_path}`)
//! and Dedicated Command Login Gateway for UMSA Prodüksiyon — MSI Dergisi & MSI TDR.

use crate::engine::design::admin::{render_dashboard, render_login};
use crate::engine::translation_engine::TranslationCache;

/// Renders the dedicated Enterprise Command Login Gateway (`GET /{admin_path}` when unauthenticated)
/// with CSP nonce injection and dynamic `data-admin-path` binding on `<body>`.
pub fn render_admin_login(nonce: Option<&str>, admin_path: &str, tr: &TranslationCache, lang: &str) -> String {
    render_login(nonce, admin_path, tr, lang)
}

/// Renders the full 18-tab Admin Command Center with CSP nonce, secret admin path routing,
/// and authenticated operator identity bound via `data-*` attributes on `<body>`.
pub fn render_admin_dashboard_full(
    nonce: Option<&str>,
    admin_path: &str,
    operator_email: &str,
    operator_role: &str,
    tr: &TranslationCache,
    lang: &str,
) -> String {
    render_dashboard(nonce, admin_path, operator_email, operator_role, tr, lang)
}

/// Renders the admin dashboard HTML with optional CSP nonce injection on `<script>` tags.
pub fn render_admin_dashboard_with_nonce(nonce: Option<&str>) -> String {
    let tr = TranslationCache::new();
    render_admin_dashboard_full(nonce, "podmin", "", "", &tr, "tr")
}

/// Legacy entry point without nonce support (for backward compatibility).
pub fn render_admin_dashboard() -> String {
    render_admin_dashboard_with_nonce(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_view_injects_nonce_and_admin_path() {
        let tr = TranslationCache::new();
        let html = render_admin_login(Some("test-nonce-123"), "podmin", &tr, "tr");
        assert!(html.contains("nonce=\"test-nonce-123\""));
        assert!(!html.contains("{nonce}"));
        assert!(html.contains("data-admin-path=\"/podmin\""));
        assert!(html.contains("noindex,nofollow,noarchive"));
    }

    #[test]
    fn dashboard_full_injects_nonce_path_and_operator_identity() {
        let tr = TranslationCache::new();
        let html = render_admin_dashboard_full(
            Some("csp-999"),
            "/podmin/",
            "editor@msidergisi.com",
            "content_editor",
            &tr,
            "tr",
        );
        assert!(html.contains("nonce=\"csp-999\""));
        assert!(!html.contains("{nonce}"));
        assert!(html.contains("href=\"/podmin/tasarim\""));
        assert!(!html.contains("/admin/tasarim"));
        assert!(html.contains(
            "data-admin-path=\"/podmin\" data-operator-email=\"editor@msidergisi.com\" data-operator-role=\"content_editor\""
        ));
    }
}
