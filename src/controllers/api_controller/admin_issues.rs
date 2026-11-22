use super::*;

use crate::models::MagazineIssue;

#[derive(Deserialize, Default)]
pub struct AdminIssueQuery {
    pub publication_type: Option<String>,
    pub auto_convert: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct AdminDeleteIssueQuery {
    pub force: Option<String>,
    pub publication_type: Option<String>,
}

#[derive(Deserialize)]
pub struct UpsertIssuePayload {
    #[serde(flatten)]
    pub issue: MagazineIssue,
    #[serde(default)]
    pub auto_convert: Option<bool>,
}

pub(crate) fn normalize_publication_type(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("msi") {
        Some("msi")
    } else if trimmed.eq_ignore_ascii_case("msi_tdr") || trimmed.eq_ignore_ascii_case("msi-tdr") {
        Some("msi_tdr")
    } else {
        None
    }
}

#[allow(clippy::result_large_err)]
fn require_issues_write(operator: &OperatorUser) -> Result<(), Response> {
    if let Err(err) = check_permission(operator, "issues:write") {
        if check_permission(operator, "issues:manage").is_err() {
            return Err(err.into_response());
        }
    }
    Ok(())
}

#[allow(clippy::result_large_err)]
fn require_issues_delete(operator: &OperatorUser) -> Result<(), Response> {
    if let Err(err) = check_permission(operator, "issues:delete") {
        if check_permission(operator, "issues:manage").is_err() {
            return Err(err.into_response());
        }
    }
    Ok(())
}

fn prepare_issue_fields(issue: &mut MagazineIssue, edergi_dir: &str) {
    if issue.slug.trim().is_empty() {
        issue.slug = if issue.publication_type == "msi_tdr" {
            format!("msi-tdr-issue-{}", issue.issue_number)
        } else {
            format!("msi-dergisi-sayi-{}", issue.issue_number)
        };
    }

    // Preserve empty English fields cleanly without copying Turkish text into EN columns (T4 / E2)
    issue.title_tr = issue.title_tr.trim().to_string();
    issue.title_en = issue.title_en.trim().to_string();
    issue.publish_month_tr = issue.publish_month_tr.trim().to_string();
    issue.publish_month_en = issue.publish_month_en.trim().to_string();

    if issue.cover_image.trim().is_empty() {
        issue.cover_image = crate::db::constants::DEFAULT_ISSUE_COVER_PATH.to_string();
    } else {
        issue.cover_image = issue.cover_image.trim().to_string();
    }

    if issue.pdf_path.trim().is_empty() {
        issue.pdf_path = format!(
            "/api/v1/edergi/{}/download-pdf?publication_type={}",
            issue.issue_number, issue.publication_type
        );
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

    if let Some(ref pdf_filename) = issue.uploaded_pdf_file {
        let candidate = FsPath::new(edergi_dir).join(pdf_filename);
        if let Ok(bytes) = std::fs::read(&candidate) {
            if let Some(detected_pages) = edergi_pdf_service::extract_pdf_page_count(&bytes) {
                if detected_pages > 0 {
                    issue.page_count = detected_pages;
                }
            }
        }
    }
}

async fn maybe_run_auto_convert(
    state: &AppState,
    operator: &OperatorUser,
    client_ip: &str,
    issue: &mut MagazineIssue,
) -> Option<edergi_pdf_service::ConversionResult> {
    let pdf_name = issue.uploaded_pdf_file.clone()?;
    let edergi_dir = state.config.edergi_dir.clone();
    let issue_clone = issue.clone();
    let res = tokio::task::spawn_blocking(move || {
        edergi_pdf_service::convert_issue_pdf(&edergi_dir, &issue_clone, &pdf_name)
    })
    .await
    .ok()?
    .ok()?;

    issue.page_count = res.page_count;
    issue.spreads = res.spreads.clone();
    if !res.spreads.is_empty() {
        issue.dossier_count = res.spreads.len() as i32;
    }
    let _ = state.db.upsert_issue(issue);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "convert_pdf",
        "magazine_issue",
        Some(i64::from(issue.issue_number)),
        &json!({
            "publication_type": issue.publication_type,
            "issue_number": issue.issue_number,
            "page_count": res.page_count,
            "cached_pages_count": res.cached_pages_count,
            "spreads_count": res.spreads.len(),
            "auto_convert": true
        })
        .to_string(),
        client_ip,
    );
    Some(res)
}

pub async fn admin_list_issues(
    operator: OperatorUser,
    State(state): State<AppState>,
    Query(query): Query<AdminIssueQuery>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["issues:read", "issues:delete", "contents:read"]) {
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

pub async fn admin_get_issue_by_key(
    operator: OperatorUser,
    State(state): State<AppState>,
    Path((publication_type, issue_number)): Path<(String, i32)>,
) -> Response {
    if let Err(resp) = require_any_perm(&operator, &["issues:read", "issues:delete", "contents:read"]) {
        return resp;
    }
    let Some(pub_type) = normalize_publication_type(&publication_type) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    };
    let Some(issue) = state.db.get_issue(pub_type, issue_number) else {
        return json_error(&state, StatusCode::NOT_FOUND, msg_keys::ISSUE_NOT_FOUND);
    };
    ApiResponse::ok(json!(issue)).into_response()
}

pub async fn admin_upsert_issue(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<AdminIssueQuery>,
    Json(payload): Json<UpsertIssuePayload>,
) -> Response {
    if let Err(resp) = require_issues_write(&operator) {
        return resp;
    }
    let mut issue = payload.issue;
    if issue.issue_number <= 0 || issue.year < 1990 || issue.title_tr.trim().is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    let query_pub = query
        .publication_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"));

    let pub_type = if let Some(qp) = query_pub {
        let Some(norm) = normalize_publication_type(qp) else {
            return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
        };
        norm
    } else if issue.publication_type.trim().is_empty() {
        "msi"
    } else {
        let Some(norm) = normalize_publication_type(&issue.publication_type) else {
            return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
        };
        norm
    };
    issue.publication_type = pub_type.to_string();

    let prev_pdf = state
        .db
        .get_issue(pub_type, issue.issue_number)
        .and_then(|old| old.uploaded_pdf_file);

    prepare_issue_fields(&mut issue, &state.config.edergi_dir);

    match state.db.upsert_issue(&issue) {
        Ok(num) => {
            if prev_pdf != issue.uploaded_pdf_file {
                edergi_pdf_service::clear_issue_pages_cache(
                    &state.config.edergi_dir,
                    &issue.publication_type,
                    issue.issue_number,
                );
            }
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "upsert",
                "magazine_issue",
                Some(i64::from(num)),
                &json!({
                    "issue_number": num,
                    "publication_type": issue.publication_type,
                    "page_count": issue.page_count,
                    "uploaded_pdf_file": issue.uploaded_pdf_file,
                })
                .to_string(),
                &client_ip,
            );
            let should_convert =
                query.auto_convert.unwrap_or(false) || payload.auto_convert.unwrap_or(false);
            let conversion = if should_convert {
                maybe_run_auto_convert(&state, &operator, &client_ip, &mut issue).await
            } else {
                None
            };
            ApiResponse::ok(json!({
                "status": "saved",
                "issue_number": num,
                "publication_type": issue.publication_type,
                "converted": conversion.is_some(),
                "cached_pages_count": conversion.as_ref().map(|c| c.cached_pages_count).unwrap_or(0),
                "issue": issue
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_upsert_issue", &e)
            .into_response(),
    }
}

pub async fn admin_update_issue_by_key(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((publication_type, issue_number)): Path<(String, i32)>,
    Query(query): Query<AdminIssueQuery>,
    Json(payload): Json<UpsertIssuePayload>,
) -> Response {
    if let Err(resp) = require_issues_write(&operator) {
        return resp;
    }
    let Some(pub_type) = normalize_publication_type(&publication_type) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    };
    if issue_number <= 0 {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }
    let Some(existing) = state.db.get_issue(pub_type, issue_number) else {
        return json_error(&state, StatusCode::NOT_FOUND, msg_keys::ISSUE_NOT_FOUND);
    };
    let prev_pdf = existing.uploaded_pdf_file;

    let mut issue = payload.issue;
    issue.publication_type = pub_type.to_string();
    issue.issue_number = issue_number;
    if issue.year < 1990 || issue.title_tr.trim().is_empty() {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    }

    prepare_issue_fields(&mut issue, &state.config.edergi_dir);

    match state.db.upsert_issue(&issue) {
        Ok(num) => {
            if prev_pdf != issue.uploaded_pdf_file {
                edergi_pdf_service::clear_issue_pages_cache(
                    &state.config.edergi_dir,
                    &issue.publication_type,
                    issue.issue_number,
                );
            }
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "update",
                "magazine_issue",
                Some(i64::from(num)),
                &json!({
                    "issue_number": num,
                    "publication_type": issue.publication_type,
                    "page_count": issue.page_count,
                    "uploaded_pdf_file": issue.uploaded_pdf_file,
                })
                .to_string(),
                &client_ip,
            );
            let should_convert =
                query.auto_convert.unwrap_or(false) || payload.auto_convert.unwrap_or(false);
            let conversion = if should_convert {
                maybe_run_auto_convert(&state, &operator, &client_ip, &mut issue).await
            } else {
                None
            };
            ApiResponse::ok(json!({
                "status": "saved",
                "issue_number": num,
                "publication_type": issue.publication_type,
                "converted": conversion.is_some(),
                "cached_pages_count": conversion.as_ref().map(|c| c.cached_pages_count).unwrap_or(0),
                "issue": issue
            }))
            .into_response()
        }
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_update_issue_by_key", &e)
            .into_response(),
    }
}

async fn delete_issue_inner(
    state: &AppState,
    operator: &OperatorUser,
    headers: &HeaderMap,
    pub_type: &str,
    issue_number: i32,
    force: bool,
) -> Response {
    if state.db.get_issue(pub_type, issue_number).is_none() {
        return json_error(state, StatusCode::NOT_FOUND, msg_keys::ISSUE_NOT_FOUND);
    }

    let shared_number_count = state.db.get_issues_by_number(issue_number).len();
    let conn = match state.db.open_conn() {
        Ok(c) => c,
        Err(e) => {
            return ApiResponse::server_error(
                &state.translations,
                "tr",
                "admin_delete_issue_conn",
                &e,
            )
            .into_response();
        }
    };

    let (count_sql, detach_sql) = if shared_number_count > 1 {
        if pub_type == "msi_tdr" {
            (
                "SELECT COUNT(*) FROM contents WHERE issue_number = ?1 AND LOWER(COALESCE(source_group, '')) = 'msi-tdr'",
                "UPDATE contents SET issue_number = NULL WHERE issue_number = ?1 AND LOWER(COALESCE(source_group, '')) = 'msi-tdr'",
            )
        } else {
            (
                "SELECT COUNT(*) FROM contents WHERE issue_number = ?1 AND LOWER(COALESCE(source_group, '')) <> 'msi-tdr'",
                "UPDATE contents SET issue_number = NULL WHERE issue_number = ?1 AND LOWER(COALESCE(source_group, '')) <> 'msi-tdr'",
            )
        }
    } else {
        (
            "SELECT COUNT(*) FROM contents WHERE issue_number = ?1",
            "UPDATE contents SET issue_number = NULL WHERE issue_number = ?1",
        )
    };

    let ref_count: i64 = match conn.query_row(count_sql, rusqlite::params![issue_number], |r| r.get(0)) {
        Ok(c) => c,
        Err(e) => {
            return ApiResponse::server_error(
                &state.translations,
                "tr",
                "admin_delete_issue_ref",
                &e,
            )
            .into_response();
        }
    };

    if ref_count > 0 && !force {
        return ApiResponse::raw(
            StatusCode::CONFLICT,
            json!({
                "status": "error",
                "error_code": error_codes::CONFLICT,
                "message": format!(
                    "Sayı #{} ({}) {} adet içerik tarafından referanslandığı için silinemez. Bağlantıları kaldırıp silmek için ?force=true kullanın.",
                    issue_number, pub_type, ref_count
                ),
                "data": {
                    "issue_number": issue_number,
                    "publication_type": pub_type,
                    "referenced_contents_count": ref_count
                }
            }),
        )
        .into_response();
    }

    if ref_count > 0 && force {
        if let Err(e) = conn.execute(detach_sql, rusqlite::params![issue_number]) {
            return ApiResponse::server_error(
                &state.translations,
                "tr",
                "admin_delete_issue_detach",
                &e,
            )
            .into_response();
        }
        drop(conn);
        state.db.refresh_l1_sections(&["contents"]);
    } else {
        drop(conn);
    }

    match state.db.delete_issue_by_type(pub_type, issue_number) {
        Ok(true) => {
            edergi_pdf_service::clear_issue_pages_cache(&state.config.edergi_dir, pub_type, issue_number);
            let client_ip = extract_client_ip(headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "delete",
                "magazine_issue",
                Some(i64::from(issue_number)),
                &json!({
                    "publication_type": pub_type,
                    "force": force,
                    "detached_contents": if force { ref_count } else { 0 },
                })
                .to_string(),
                &client_ip,
            );
            ApiResponse::ok(json!({
                "status": "deleted",
                "issue_number": issue_number,
                "publication_type": pub_type,
                "detached_contents": if force { ref_count } else { 0 }
            }))
            .into_response()
        }
        Ok(false) => json_error(state, StatusCode::NOT_FOUND, msg_keys::ISSUE_NOT_FOUND),
        Err(e) => ApiResponse::server_error(&state.translations, "tr", "admin_delete_issue", &e)
            .into_response(),
    }
}

pub async fn admin_delete_issue_by_key(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((publication_type, issue_number)): Path<(String, i32)>,
    Query(query): Query<AdminDeleteIssueQuery>,
) -> Response {
    if let Err(resp) = require_issues_delete(&operator) {
        return resp;
    }
    let Some(pub_type) = normalize_publication_type(&publication_type) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    };
    let force = query
        .force
        .as_deref()
        .map(str::trim)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false);
    delete_issue_inner(&state, &operator, &headers, pub_type, issue_number, force).await
}

pub async fn admin_delete_issue(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(issue_number): Path<i32>,
    Query(query): Query<AdminDeleteIssueQuery>,
) -> Response {
    if let Err(resp) = require_issues_delete(&operator) {
        return resp;
    }

    let force = query
        .force
        .as_deref()
        .map(str::trim)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false);

    if let Some(raw_pub) = query
        .publication_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let Some(pub_type) = normalize_publication_type(raw_pub) else {
            return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
        };
        return delete_issue_inner(&state, &operator, &headers, pub_type, issue_number, force).await;
    }

    let matches = state.db.get_issues_by_number(issue_number);
    match matches.len() {
        0 => json_error(&state, StatusCode::NOT_FOUND, msg_keys::ISSUE_NOT_FOUND),
        1 => {
            let pub_type = matches[0].publication_type.clone();
            delete_issue_inner(&state, &operator, &headers, &pub_type, issue_number, force).await
        }
        _ => {
            let pub_types: Vec<String> = matches.iter().map(|i| i.publication_type.clone()).collect();
            ApiResponse::raw(
                StatusCode::CONFLICT,
                json!({
                    "status": "error",
                    "error_code": error_codes::CONFLICT,
                    "message": format!(
                        "Sayı #{} birden fazla yayında ({}) mevcut. Silmek için publication_type belirtin.",
                        issue_number,
                        pub_types.join(", ")
                    ),
                    "data": {
                        "issue_number": issue_number,
                        "publication_types": pub_types
                    }
                }),
            )
            .into_response()
        }
    }
}

pub async fn admin_convert_issue_pdf(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((publication_type, issue_number)): Path<(String, i32)>,
) -> Response {
    if let Err(resp) = require_issues_write(&operator) {
        return resp;
    }
    let Some(pub_type) = normalize_publication_type(&publication_type) else {
        return json_error(&state, StatusCode::BAD_REQUEST, msg_keys::VALIDATION_ERROR);
    };
    let Some(mut issue) = state.db.get_issue(pub_type, issue_number) else {
        return json_error(&state, StatusCode::NOT_FOUND, msg_keys::ISSUE_NOT_FOUND);
    };
    let Some(pdf_filename) = issue
        .uploaded_pdf_file
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
    else {
        return ApiResponse::raw(
            StatusCode::BAD_REQUEST,
            json!({
                "status": "error",
                "error_code": error_codes::BAD_REQUEST,
                "message": "Bu sayıya bağlı yüklenmiş bir PDF dosyası bulunamadı."
            }),
        )
        .into_response();
    };

    let edergi_dir = state.config.edergi_dir.clone();
    let issue_clone = issue.clone();
    let conv_res = tokio::task::spawn_blocking(move || {
        edergi_pdf_service::convert_issue_pdf(&edergi_dir, &issue_clone, &pdf_name_sanitize(&pdf_filename))
    })
    .await;

    let res = match conv_res {
        Ok(Ok(r)) => r,
        Ok(Err(msg)) => {
            return ApiResponse::raw(
                StatusCode::BAD_REQUEST,
                json!({
                    "status": "error",
                    "error_code": error_codes::BAD_REQUEST,
                    "message": msg
                }),
            )
            .into_response();
        }
        Err(e) => {
            return ApiResponse::server_error(&state.translations, "tr", "admin_convert_issue_pdf", &e)
                .into_response();
        }
    };

    issue.page_count = res.page_count;
    issue.spreads = res.spreads.clone();
    if !res.spreads.is_empty() {
        issue.dossier_count = res.spreads.len() as i32;
    }

    if let Err(e) = state.db.upsert_issue(&issue) {
        return ApiResponse::server_error(&state.translations, "tr", "admin_convert_issue_pdf_save", &e)
            .into_response();
    }

    let client_ip = extract_client_ip(&headers);
    let _ = state.db.insert_audit_log(
        &operator.email,
        "convert_pdf",
        "magazine_issue",
        Some(i64::from(issue_number)),
        &json!({
            "publication_type": res.publication_type,
            "issue_number": res.issue_number,
            "page_count": res.page_count,
            "cached_pages_count": res.cached_pages_count,
            "spreads_count": res.spreads.len(),
        })
        .to_string(),
        &client_ip,
    );

    ApiResponse::ok(json!({
        "publication_type": res.publication_type,
        "issue_number": res.issue_number,
        "page_count": res.page_count,
        "cached_pages_count": res.cached_pages_count,
        "spreads": res.spreads,
        "converted": true,
        "issue": issue
    }))
    .into_response()
}

fn pdf_name_sanitize(raw: &str) -> String {
    FsPath::new(raw.trim())
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
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

#[cfg(test)]
mod tests {
    use crate::{
        config::AppConfig,
        controllers::AppState,
        db::Database,
        engine::translation_engine::TranslationCache,
        routes,
        services::auth_service,
    };
    use axum::{
        body::{to_bytes, Body},
        http::{header, Request, StatusCode},
    };
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state(label: &str) -> (AppState, String, String) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = format!("/tmp/msi_admin_issues_{}_{}_{}.db", label, std::process::id(), ts);
        let edergi_dir = format!("/tmp/msi_admin_edergi_{}_{}_{}", label, std::process::id(), ts);
        let _ = std::fs::create_dir_all(&edergi_dir);
        let mut cfg = AppConfig::from_env();
        cfg.app_env = "test".into();
        cfg.sqlite_db_path = db_path.clone();
        cfg.edergi_dir = edergi_dir.clone();
        cfg.session_secret = "test-session-secret".into();
        cfg.watermark_secret = "test-watermark-secret".into();
        cfg.monitoring_enabled = false;
        let db = Database::new(&db_path).expect("test db");
        let translations = TranslationCache::new();
        translations.load(db.load_all_translations());
        let state = AppState {
            config: Arc::new(cfg),
            db,
            translations,
            monitoring_cache: None,
        };
        (state, db_path, edergi_dir)
    }

    fn cookie_for(state: &AppState, email: &str, role: &str) -> String {
        let hash = auth_service::hash_password("Pass1234!", &state.config.session_secret);
        let uid = state
            .db
            .admin_create_user(
                email,
                &hash,
                "Test User",
                role,
                "MSB",
                "MSI",
                "Staff",
                false,
                "",
            )
            .expect("create user");
        let u = state.db.get_user_by_id(uid).unwrap();
        format!(
            "msi_session={}",
            auth_service::generate_session_token(&u, &state.config.session_secret)
        )
    }

    async fn req_json(
        app: &axum::Router,
        method: &str,
        uri: &str,
        cookie: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut b = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::COOKIE, cookie);
        let req_body = if let Some(v) = body {
            b = b.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        } else {
            Body::empty()
        };
        let resp = app.clone().oneshot(b.body(req_body).unwrap()).await.unwrap();
        let st = resp.status();
        let bytes = to_bytes(resp.into_body(), 2 * 1024 * 1024).await.unwrap();
        let val = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (st, val)
    }

    #[tokio::test]
    async fn issues_rbac_e6_and_delete_conflict_e5_and_pdf_page_autodetect() {
        let (state, db_path, edergi_dir) = test_state("e5_e6");
        let app = routes::build_router(state.clone());

        let pub_cookie = cookie_for(&state, "publisher@msidergisi.com", "content_publisher");
        let eic_cookie = cookie_for(&state, "eic@msidergisi.com", "editor_in_chief");

        // Write a real 12-page synthetic catalog PDF into edergi_dir
        let pdf_path = format!("{}/issue-991.pdf", edergi_dir);
        std::fs::write(
            &pdf_path,
            b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [] /Count 12 >>\nendobj\n%%EOF\n",
        )
        .unwrap();

        let payload = json!({
            "issue_number": 991,
            "slug": "",
            "title_tr": "Test Ozel Sayi 991",
            "title_en": "",
            "publish_month_tr": "Kasim",
            "publish_month_en": "",
            "year": 2026,
            "cover_image": crate::db::constants::DEFAULT_ISSUE_COVER_PATH,
            "is_special_issue": false,
            "event_name": null,
            "page_count": 99,
            "dossier_count": 5,
            "summary_tr": "Ozet",
            "summary_en": "",
            "pdf_path": "",
            "uploaded_pdf_file": "issue-991.pdf",
            "spreads": []
        });

        // E6: content_publisher (issues:read only, even with contents:publish/delete) gets 403
        let (st, _) = req_json(
            &app,
            "POST",
            "/api/v1/admin/issues",
            &pub_cookie,
            Some(payload.clone()),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN);

        let (st, _) = req_json(&app, "DELETE", "/api/v1/admin/issues/280", &pub_cookie, None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);

        // E2/T4 + PDF page_count autodetect + ?publication_type=msi_tdr
        let (st, body) = req_json(
            &app,
            "POST",
            "/api/v1/admin/issues?publication_type=msi_tdr",
            &eic_cookie,
            Some(payload),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(body["data"]["issue"]["publication_type"], "msi_tdr");
        assert_eq!(body["data"]["issue"]["title_en"], "");
        assert_eq!(body["data"]["issue"]["publish_month_en"], "");
        assert_eq!(body["data"]["issue"]["page_count"], 12);

        // Attach a content row referencing issue_number = 991
        {
            let conn = state.db.open_conn().unwrap();
            conn.execute(
                "INSERT INTO contents (slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en, content_type, source_group, cover_image, author_name, author_title, issue_number, published_at, status)
                 VALUES ('ref-991', 'Ref 991', '', 'S', '', '<p>B</p>', '', 'haberler', 'dergi-icerigi', 'c.webp', 'Auth', 'Ed', 991, '2026-11-18 10:00:00', 'published')",
                [],
            )
            .unwrap();
        }

        // E5: Delete without force returns 409 Conflict
        let (st, conflict_body) =
            req_json(&app, "DELETE", "/api/v1/admin/issues/991", &eic_cookie, None).await;
        assert_eq!(st, StatusCode::CONFLICT);
        assert_eq!(conflict_body["data"]["referenced_contents_count"], 1);

        // E5: Delete with ?force=true sets contents.issue_number = NULL and deletes issue
        let (st, del_body) = req_json(
            &app,
            "DELETE",
            "/api/v1/admin/issues/991?force=true",
            &eic_cookie,
            None,
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(del_body["data"]["detached_contents"], 1);
        assert!(state.db.get_issue_by_number(991).is_none());

        let conn = state.db.open_conn().unwrap();
        let remaining_ref: Option<i32> = conn
            .query_row(
                "SELECT issue_number FROM contents WHERE slug = 'ref-991'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(remaining_ref, None);
        drop(conn);

        let _ = std::fs::remove_dir_all(&edergi_dir);
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }
}
