use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::controllers::AppState;
use crate::services::auth_service;

/// Canonicalizes resource names so singular/plural variants resolve identically
/// (e.g. `content` <-> `contents`, `category` <-> `categories`).
fn canonicalize_resource(res: &str) -> &str {
    match res {
        "content" => "contents",
        "category" => "categories",
        "approval" => "approvals",
        "newsletter" => "newsletters",
        "notification" => "notifications",
        "subscription" => "subscriptions",
        "setting" => "settings",
        "user" => "users",
        "role" => "roles",
        "ad" => "ads",
        "translation" => "translations",
        "corporate_request" => "corporate_requests",
        "issue" => "issues",
        "invoice" => "invoices",
        "product" => "commerce",
        "plan" => "subscriptions",
        "redirect" => "redirects",
        "audit_log" | "audit_logs" => "audit",
        other => other,
    }
}

/// Checks whether a single granted permission string satisfies a required permission string.
/// Normalizes `.` to `:`, maps resource/action aliases, and supports `*` and `resource:*` wildcards.
fn permission_matches(granted_raw: &str, required_raw: &str) -> bool {
    let granted = granted_raw.trim().to_ascii_lowercase().replace('.', ":");
    let required = required_raw.trim().to_ascii_lowercase().replace('.', ":");

    if granted.is_empty() || required.is_empty() {
        return false;
    }
    if granted == "*" || granted == required {
        return true;
    }

    let (g_res_raw, g_act) = match granted.split_once(':') {
        Some((r, a)) => (r.trim(), a.trim()),
        None => (granted.as_str(), "*"),
    };
    let (r_res_raw, r_act) = match required.split_once(':') {
        Some((r, a)) => (r.trim(), a.trim()),
        None => (required.as_str(), "*"),
    };

    let g_res = canonicalize_resource(g_res_raw);
    let r_res = canonicalize_resource(r_res_raw);

    // Cross-resource equivalence between user verification and corporate verification requests
    if g_res == "users"
        && matches!(g_act, "*" | "manage" | "verify")
        && r_res == "corporate_requests"
        && matches!(r_act, "read" | "view" | "decide" | "write")
    {
        return true;
    }
    if g_res == "corporate_requests"
        && matches!(g_act, "*" | "manage" | "decide")
        && r_res == "users"
        && r_act == "verify"
    {
        return true;
    }

    if g_res != r_res {
        return false;
    }

    // Wildcard or `manage` on the resource grants all actions on that resource
    if g_act == "*" || g_act == "manage" {
        return true;
    }
    if g_act == r_act {
        return true;
    }

    match r_act {
        "read" | "view" => matches!(g_act, "read" | "view" | "write" | "publish" | "upload"),
        "write" => matches!(g_act, "write" | "publish"),
        "manage" => false,
        _ => false,
    }
}

/// Returns `true` if `role` is explicitly a non-staff reader, subscriber, or guest role.
pub fn is_non_staff_role_name(role: &str) -> bool {
    let norm = role.trim().to_ascii_lowercase();
    norm.is_empty()
        || matches!(
            norm.as_str(),
            "reader"
                | "registered"
                | "subscriber"
                | "corporate"
                | "deleted"
                | "free"
                | "guest"
                | "member"
        )
        || norm.starts_with("subscriber_")
}

/// Checks whether a role code grants a specific permission via `user_roles`.
pub fn role_has_permission(db: &crate::db::Database, role: &str, required: &str) -> bool {
    if is_non_staff_role_name(role) {
        return false;
    }
    let norm = role.trim().to_ascii_lowercase();
    if norm == "super_admin" {
        return true;
    }
    let perms = db
        .get_role_permissions(role.trim())
        .or_else(|| db.get_role_permissions(&norm))
        .unwrap_or_default();
    if perms.is_empty() && matches!(norm.as_str(), "admin" | "site_operator") {
        return true;
    }
    perms
        .iter()
        .any(|granted| permission_matches(granted, required))
}

/// Returns `true` if `role` is an administrative or RBAC-enabled staff role.
/// Excludes non-staff reader/subscriber roles (`reader`, `registered`, `subscriber*`, `deleted`)
/// while accepting `admin`, `site_operator`, `super_admin`, or any role with non-empty
/// permissions in `user_roles`.
pub fn is_staff_role(db: &crate::db::Database, role: &str) -> bool {
    if is_non_staff_role_name(role) {
        return false;
    }
    let norm = role.trim().to_ascii_lowercase();

    if matches!(norm.as_str(), "admin" | "site_operator" | "super_admin") {
        return true;
    }

    db.get_role_permissions(role.trim())
        .or_else(|| db.get_role_permissions(&norm))
        .map(|perms| {
            perms.iter().any(|p| {
                let pt = p.trim().to_ascii_lowercase();
                !pt.is_empty()
                    && !matches!(
                        pt.as_str(),
                        "articles:read" | "articles:premium" | "edergi:read" | "edergi:download"
                    )
            })
        })
        .unwrap_or(false)
}

/// Authenticated operator/staff user extracted from session cookie.
/// Resolves permissions from the `user_roles` table based on the user's role.
/// Returns 401 if no valid session, 403 if role is not a staff/operator role.
#[derive(Clone, Debug)]
pub struct OperatorUser {
    pub user_id: i64,
    pub email: String,
    pub full_name: String,
    pub role: String,
    pub permissions: Vec<String>,
}

impl OperatorUser {
    /// Check if a role string qualifies as an operator/staff role against the DB `user_roles` table.
    pub fn is_staff_role(db: &crate::db::Database, role: &str) -> bool {
        is_staff_role(db, role)
    }

    /// Check if this operator has a specific permission.
    /// Normalizes `.` to `:`, resolves singular/plural & `view`/`read`/`manage` aliases,
    /// and supports `*` as well as prefix wildcards (`contents:*`, `ads:*`, `approvals:*`).
    pub fn has_permission(&self, required: &str) -> bool {
        self.permissions
            .iter()
            .any(|granted| permission_matches(granted, required))
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for OperatorUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = auth_service::session_user(state, &parts.headers).ok_or_else(|| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "error",
                    "error_code": "AUTH_REQUIRED",
                    "message": "Kimlik doğrulama gerekli.",
                    "data": null
                })),
            )
                .into_response()
        })?;

        if !is_staff_role(&state.db, &user.role) {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "error",
                    "error_code": "FORBIDDEN",
                    "message": "Bu işlem için yetkili personel veya yönetici oturumu gereklidir.",
                    "data": null
                })),
            )
                .into_response());
        }

        let norm_role = user.role.trim().to_ascii_lowercase();
        let mut permissions = state
            .db
            .get_role_permissions(user.role.trim())
            .or_else(|| state.db.get_role_permissions(&norm_role))
            .unwrap_or_default();

        if permissions.is_empty()
            && matches!(norm_role.as_str(), "admin" | "site_operator" | "super_admin")
        {
            permissions.push("*".to_string());
        }

        Ok(OperatorUser {
            user_id: user.id,
            email: user.email,
            full_name: user.full_name,
            role: user.role,
            permissions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_op(role: &str, perms: &[&str]) -> OperatorUser {
        OperatorUser {
            user_id: 1,
            email: "test@msidergisi.com".to_string(),
            full_name: "Test Staff".to_string(),
            role: role.to_string(),
            permissions: perms.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn wildcard_grants_everything() {
        let op = make_op("super_admin", &["*"]);
        assert!(op.has_permission("contents:publish"));
        assert!(op.has_permission("settings:manage"));
        assert!(op.has_permission("users:delete"));
    }

    #[test]
    fn prefix_wildcards_and_dot_normalization_work() {
        let eic = make_op(
            "editor_in_chief",
            &["contents:*", "approvals:*", "analytics:read", "settings:read"],
        );
        assert!(eic.has_permission("contents:read"));
        assert!(eic.has_permission("content:write"));
        assert!(eic.has_permission("contents.publish"));
        assert!(eic.has_permission("approvals:manage"));
        assert!(eic.has_permission("analytics:view"));
        assert!(eic.has_permission("settings:read"));
        assert!(!eic.has_permission("settings:write"));
        assert!(!eic.has_permission("settings:manage"));
        assert!(!eic.has_permission("ads:write"));
    }

    #[test]
    fn content_editor_cannot_publish_or_delete() {
        let editor = make_op(
            "content_editor",
            &["contents:read", "contents:write", "categories:read", "media:upload"],
        );
        assert!(editor.has_permission("contents:read"));
        assert!(editor.has_permission("content:write"));
        assert!(editor.has_permission("contents.write"));
        assert!(!editor.has_permission("contents:publish"));
        assert!(!editor.has_permission("contents:delete"));
        assert!(!editor.has_permission("categories:write"));
    }

    #[test]
    fn support_agent_verification_alias_works() {
        let support = make_op(
            "support_agent",
            &["users:read", "users:verify", "subscriptions:read", "notifications:read"],
        );
        assert!(support.has_permission("users:verify"));
        assert!(support.has_permission("corporate_requests:read"));
        assert!(support.has_permission("corporate_requests:decide"));
        assert!(!support.has_permission("users:manage"));
        assert!(!support.has_permission("contents:write"));
    }
}
