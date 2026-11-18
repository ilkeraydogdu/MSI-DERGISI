use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use crate::extractors::operator_user::OperatorUser;

/// Enforces that `operator` possesses `perm`, returning a localized `403 FORBIDDEN` JSON
/// tuple suitable for Axum responses (`if let Err(resp) = check_permission(&operator, "...") { return resp.into_response(); }`).
pub fn check_permission(
    operator: &OperatorUser,
    perm: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    if operator.has_permission(perm) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "error_code": "FORBIDDEN",
                "message": format!("Bu işlem için '{}' yetkisi gereklidir.", perm),
                "required_permission": perm,
                "data": null
            })),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_permission_returns_ok_when_granted_and_403_when_missing() {
        let op = OperatorUser {
            user_id: 7,
            email: "editor@msidergisi.com".to_string(),
            full_name: "MSI Editör".to_string(),
            role: "content_editor".to_string(),
            permissions: vec!["contents:read".to_string(), "contents:write".to_string()],
        };

        assert!(check_permission(&op, "contents:write").is_ok());

        let err = check_permission(&op, "contents:publish").unwrap_err();
        assert_eq!(err.0, StatusCode::FORBIDDEN);
        assert_eq!(err.1 .0["error_code"], "FORBIDDEN");
        assert_eq!(err.1 .0["required_permission"], "contents:publish");
    }
}
