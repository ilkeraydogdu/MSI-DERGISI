//! OMS/CMS Operator Dashboard & Command Gateway Views (`src/controllers/admin_view.rs`)
//! Renders the CSP-clean HTML5 Operator & B2B Defense Analytics Center (`/{admin_path}`)
//! and Dedicated Command Login Gateway for UMSA Prodüksiyon — MSI Dergisi & MSI TDR.

/// The 17-tab admin dashboard HTML template, embedded at compile time.
const ADMIN_DASHBOARD_HTML: &str = include_str!("../engine/design/admin_dashboard.html");

/// The dedicated enterprise command login gateway HTML template, embedded at compile time.
const ADMIN_LOGIN_HTML: &str = include_str!("../engine/design/admin_login.html");

fn escape_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn normalize_admin_path(admin_path: &str) -> &str {
    let trimmed = admin_path.trim().trim_matches('/');
    if trimmed.is_empty() {
        "admin"
    } else {
        trimmed
    }
}

fn apply_csp_nonce(html: &str, nonce: Option<&str>) -> String {
    match nonce.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => {
            let escaped = escape_attr(n);
            html.replace("nonce=\"{nonce}\"", &format!("nonce=\"{}\"", escaped))
        }
        None => html.replace(" nonce=\"{nonce}\"", ""),
    }
}

/// Renders the dedicated Enterprise Command Login Gateway (`GET /{admin_path}` when unauthenticated)
/// with CSP nonce injection and dynamic `data-admin-path` binding on `<body>`.
pub fn render_admin_login(nonce: Option<&str>, admin_path: &str) -> String {
    let clean_path = escape_attr(normalize_admin_path(admin_path));
    let html = ADMIN_LOGIN_HTML.replace(
        "data-admin-path=\"/admin\"",
        &format!("data-admin-path=\"/{}\"", clean_path),
    );
    apply_csp_nonce(&html, nonce)
}

/// Renders the full 17-tab Admin Command Center with CSP nonce, secret admin path routing,
/// and authenticated operator identity bound via `data-*` attributes on `<body>`.
pub fn render_admin_dashboard_full(
    nonce: Option<&str>,
    admin_path: &str,
    operator_email: &str,
    operator_role: &str,
) -> String {
    let clean_path = escape_attr(normalize_admin_path(admin_path));
    let safe_email = escape_attr(operator_email.trim());
    let safe_role = escape_attr(operator_role.trim());

    let mut html = ADMIN_DASHBOARD_HTML.replace(
        "/admin/tasarim",
        &format!("/{}/tasarim", clean_path),
    );
    html = html.replace(
        "data-admin-path=\"/admin\" data-operator-email=\"\" data-operator-role=\"\"",
        &format!(
            "data-admin-path=\"/{}\" data-operator-email=\"{}\" data-operator-role=\"{}\"",
            clean_path, safe_email, safe_role
        ),
    );

    apply_csp_nonce(&html, nonce)
}

/// Renders the admin dashboard HTML with optional CSP nonce injection on `<script>` tags.
pub fn render_admin_dashboard_with_nonce(nonce: Option<&str>) -> String {
    render_admin_dashboard_full(nonce, "admin", "", "")
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
        let html = render_admin_login(Some("test-nonce-123"), "msi-komuta");
        assert!(html.contains("nonce=\"test-nonce-123\""));
        assert!(!html.contains("{nonce}"));
        assert!(html.contains("data-admin-path=\"/msi-komuta\""));
        assert!(html.contains("noindex,nofollow,noarchive"));
    }

    #[test]
    fn dashboard_full_injects_nonce_path_and_operator_identity() {
        let html = render_admin_dashboard_full(
            Some("csp-999"),
            "/msi-komuta/",
            "editor@msidergisi.com",
            "content_editor",
        );
        assert!(html.contains("nonce=\"csp-999\""));
        assert!(!html.contains("{nonce}"));
        assert!(html.contains("href=\"/msi-komuta/tasarim\""));
        assert!(!html.contains("/admin/tasarim"));
        assert!(html.contains(
            "data-admin-path=\"/msi-komuta\" data-operator-email=\"editor@msidergisi.com\" data-operator-role=\"content_editor\""
        ));
    }
}
