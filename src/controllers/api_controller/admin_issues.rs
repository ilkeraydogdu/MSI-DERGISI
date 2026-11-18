use super::*;

use crate::models::MagazineIssue;

#[derive(Deserialize)]
pub struct AdminIssueQuery {
    pub publication_type: Option<String>,
}

pub async fn admin_list_issues(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(query): Query<AdminIssueQuery>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["issues:read", "contents:read"]) {
        return resp;
    }
    let mut issues = state.db.get_all_issues();
    if let Some(pub_type) = query
        .publication_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
    {
        issues.retain(|i| i.publication_type.eq_ignore_ascii_case(pub_type));
    }
    ApiResponse::ok(json!(issues)).into_response()
}

pub async fn admin_upsert_issue(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(mut issue): Json<MagazineIssue>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["issues:write", "contents:publish"]) {
        return resp;
    }
    if issue.issue_number <= 0 || issue.year < 1990 || issue.title_tr.trim().is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    let norm_pub = if issue.publication_type.trim().eq_ignore_ascii_case("msi_tdr") {
        "msi_tdr".to_string()
    } else {
        "msi".to_string()
    };
    issue.publication_type = norm_pub;

    if issue.slug.trim().is_empty() {
        issue.slug = if issue.publication_type == "msi_tdr" {
            format!("msi-tdr-issue-{}", issue.issue_number)
        } else {
            format!("msi-dergisi-sayi-{}", issue.issue_number)
        };
    }
    if issue.title_en.trim().is_empty() {
        issue.title_en = issue.title_tr.clone();
    }
    if issue.publish_month_en.trim().is_empty() {
        issue.publish_month_en = issue.publish_month_tr.clone();
    }
    if issue.pdf_path.trim().is_empty() {
        issue.pdf_path = format!("/api/v1/edergi/{}/download-pdf", issue.issue_number);
    }
    issue.uploaded_pdf_file = issue
        .uploaded_pdf_file
        .take()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .and_then(|s| {
            FsPath::new(&s)
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
        });

    match state.db.upsert_issue(&issue) {
        Ok(num) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert",
                "magazine_issue",
                Some(i64::from(num)),
                &json!({
                    "issue_number": num,
                    "publication_type": issue.publication_type,
                    "uploaded_pdf_file": issue.uploaded_pdf_file,
                })
                .to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "saved",
                "issue_number": num,
                "issue": issue
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_upsert_issue", &e)
            .into_response(),
    }
}

pub async fn admin_delete_issue(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(issue_number): Path<i32>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["issues:write", "contents:delete"]) {
        return resp;
    }
    match state.db.delete_issue(issue_number) {
        Ok(true) => {
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "magazine_issue",
                Some(i64::from(issue_number)),
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "deleted",
                "issue_number": issue_number
            }))
            .into_response()
        }
        Ok(false) => json_error(&state, StatusCode::NOT_FOUND, msg_keys::ISSUE_NOT_FOUND),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_delete_issue", &e)
            .into_response(),
    }
}

pub async fn admin_preview_edergi_pdf(
    operator: OperatorUser,
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["issues:read", "contents:read"]) {
        return resp;
    }
    let Some(safe_name) = FsPath::new(filename.trim())
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| n.to_ascii_lowercase().ends_with(".pdf"))
    else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::INVALID_FILE_TYPE);
    };

    let candidate = FsPath::new(&state.config.edergi_dir).join(safe_name);
    let Ok(bytes) = std::fs::read(&candidate) else {
        return not_found_error();
    };
    if !bytes.starts_with(b"%PDF-") {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::INVALID_FILE_TYPE);
    }

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{}\"", safe_name),
            ),
        ],
        bytes,
    )
        .into_response()
}
