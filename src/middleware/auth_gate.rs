use axum::{
    extract::Request,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::controllers::AppState;

/// Result of an admin access check.
pub enum AdminAccess {
    /// The request is allowed to proceed.
    Allowed,
    /// The request should be rejected with the given response.
    Denied(Response),
}

/// Checks whether the current user has staff-level access to admin routes.
///
/// Returns `AdminAccess::Allowed` when the user is a verified staff member,
/// or `AdminAccess::Denied` with an appropriate error response otherwise.
/// For non-API GET requests from unauthenticated users, returns a redirect
/// to the admin login page instead of a JSON error.
pub fn check_admin_access(
    state: &AppState,
    req: &Request,
    admin_path: &str,
    is_admin_api: bool,
) -> AdminAccess {
    let method = req.method().clone();
    let user = crate::services::auth_service::session_user(state, req.headers());
    let is_staff = user
        .as_ref()
        .map(|u| crate::extractors::operator_user::is_staff_role(&state.db, &u.role))
        .unwrap_or(false);

    if is_staff {
        return AdminAccess::Allowed;
    }

    if is_admin_api || method != Method::GET {
        let response = if user.is_some() {
            (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "error",
                    "code": "FORBIDDEN",
                    "message": "Bu işlem için yetkili yönetim rolü gereklidir."
                })),
            )
                .into_response()
        } else {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "error",
                    "code": "UNAUTHORIZED",
                    "message": "Oturum açmanız gerekiyor."
                })),
            )
                .into_response()
        };
        return AdminAccess::Denied(response);
    }

    AdminAccess::Denied(axum::response::Redirect::to(admin_path).into_response())
}