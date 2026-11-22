//! Editorial Studio SSR Host & REST API Controller (`/podmin/editor`, `/api/v1/admin/editorial/*`).
//! Strictly enforces MSI Dergisi cookie session, CSRF double-submit protection (via `/api/v1/admin/*` router),
//! and granular RBAC permissions (`contents:write`, `contents:publish`, `contents:delete`,
//! `media:upload`, `media:delete`, `categories:write`, `categories:delete`,
//! `issues:write`, `issues:delete`, `glossary:write|manage`, `redirects:write`).

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Extension, Json,
};
use crate::editorial::common::{
    Article, EditorialCapabilities, GlossaryRule, MagazineIssueItem, User,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::app_state::AppState;
use crate::config::normalize_admin_path_secret;
use crate::db::repo::{
    parse_editorial_content_id, EditorialArticleQuery, EditorialMediaInsert,
};
use crate::engine::design::editorial_studio::{
    render_editorial_studio_forbidden, render_editorial_studio_shell,
};
use crate::engine::security_engine::CspNonce;
use crate::extractors::{
    operator_user::{role_has_permission, OperatorUser},
    require_permission::check_permission,
};
use crate::models::SiteSetting;
use crate::services::{
    editorial_lock_service, editorial_media_service, editorial_research_service,
    editorial_social_service, indexnow_service,
};

const MAX_RESEARCH_LIMIT: usize = 50;

fn parse_positive_content_id(raw: &str) -> Result<i64, (StatusCode, Json<Value>)> {
    parse_editorial_content_id(raw).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Geçersiz içerik kimliği." })),
        )
    })
}

fn editorial_error_response(
    err: editorial_media_service::EditorialServiceError,
) -> (StatusCode, Json<Value>) {
    match err {
        editorial_media_service::EditorialServiceError::BadRequest(msg) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": msg })),
        ),
        editorial_media_service::EditorialServiceError::NotFound(msg) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": msg })),
        ),
        editorial_media_service::EditorialServiceError::Forbidden(msg) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "success": false, "error": msg })),
        ),
        editorial_media_service::EditorialServiceError::Internal(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": msg })),
        ),
    }
}

fn operator_display_name(op: &OperatorUser) -> String {
    if !op.full_name.trim().is_empty() {
        op.full_name.trim().to_string()
    } else {
        op.email.trim().to_string()
    }
}

fn operator_to_editorial_user(op: &OperatorUser) -> User {
    User {
        id: op.user_id as i32,
        username: operator_display_name(op),
        email: Some(op.email.clone()),
        full_name: Some(op.full_name.clone()),
        role: op.role.clone(),
        permissions: op.permissions.clone(),
        capabilities: EditorialCapabilities::from_permissions(&op.permissions),
        created_at: None,
    }
}

fn is_privileged_lifecycle_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "publish" | "published" | "future" | "scheduled" | "archived" | "archive"
    )
}

fn extract_social_cfg_json(state: &AppState) -> Value {
    let settings = state.db.get_setting_map("tr");
    if let Some(raw_social) = settings.get("editorial_social_config") {
        if let Ok(v) = serde_json::from_str::<Value>(raw_social) {
            return v;
        }
    }
    if let Some(raw_wp) = settings.get("editorial_studio_settings") {
        if let Ok(v) = serde_json::from_str::<Value>(raw_wp) {
            return v;
        }
    }
    json!({})
}

const EDITORIAL_STUDIO_JS: &str =
    include_str!("../../../storage/editorial-pkg/editorial_studio.js");
const EDITORIAL_STUDIO_WASM: &[u8] =
    include_bytes!("../../../storage/editorial-pkg/editorial_studio_bg.wasm");

fn editorial_security_headers(
    cache_control: &'static str,
) -> [(header::HeaderName, &'static str); 2] {
    [
        (
            header::HeaderName::from_static("x-robots-tag"),
            "noindex, nofollow, noarchive",
        ),
        (header::CACHE_CONTROL, cache_control),
    ]
}

fn serve_editorial_asset<B: IntoResponse>(
    state: &AppState,
    headers: &HeaderMap,
    content_type: &'static str,
    body: B,
) -> Response {
    let admin_seg = normalize_admin_path_secret(&state.config.admin_path_secret);
    let admin_path = format!("/{admin_seg}");
    let Some(user) = crate::services::auth_service::session_user(state, headers) else {
        return Redirect::to(&admin_path).into_response();
    };
    if !role_has_permission(&state.db, &user.role, "contents:write") {
        return (
            StatusCode::FORBIDDEN,
            editorial_security_headers("no-store, private"),
            "Forbidden",
        )
            .into_response();
    }
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::HeaderName::from_static("x-robots-tag"),
                "noindex, nofollow, noarchive",
            ),
            (header::CACHE_CONTROL, "private, max-age=86400"),
        ],
        body,
    )
        .into_response()
}

/// `GET /{admin}/editor` — Renders the WASM Editorial Studio page or redirects/denies by session + `contents:write`.
pub async fn render_editorial_studio(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let admin_seg = normalize_admin_path_secret(&state.config.admin_path_secret);
    let admin_path = format!("/{admin_seg}");
    let admin_headers = editorial_security_headers("no-store, private");

    let Some(user) = crate::services::auth_service::session_user(&state, &headers) else {
        return Redirect::to(&admin_path).into_response();
    };

    if !role_has_permission(&state.db, &user.role, "contents:write") {
        let html = render_editorial_studio_forbidden(
            Some(&csp_nonce.0),
            &admin_path,
            &user.email,
            &user.role,
        );
        return (StatusCode::FORBIDDEN, admin_headers, Html(html)).into_response();
    }

    let html = render_editorial_studio_shell(Some(&csp_nonce.0), &admin_path);
    (StatusCode::OK, admin_headers, Html(html)).into_response()
}

/// `GET /{admin}/editor/studio.js` — Serves the compiled `wasm-bindgen` bootstrap module embedded inside the server binary.
pub async fn editorial_studio_js(State(state): State<AppState>, headers: HeaderMap) -> Response {
    serve_editorial_asset(
        &state,
        &headers,
        "text/javascript; charset=utf-8",
        EDITORIAL_STUDIO_JS,
    )
}

/// `GET /{admin}/editor/studio.wasm` — Serves the Pure Rust Editorial Studio WebAssembly binary embedded inside the server binary.
pub async fn editorial_studio_wasm(State(state): State<AppState>, headers: HeaderMap) -> Response {
    serve_editorial_asset(&state, &headers, "application/wasm", EDITORIAL_STUDIO_WASM)
}

/// `GET /api/v1/admin/editorial/session`
pub async fn editorial_session(
    State(state): State<AppState>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let admin_seg = normalize_admin_path_secret(&state.config.admin_path_secret);
    Ok(Json(json!({
        "success": true,
        "authenticated": true,
        "user": operator_to_editorial_user(&operator),
        "adminPath": format!("/{admin_seg}"),
    })))
}

#[derive(Debug, Default, Deserialize)]
pub struct EditorialArticlesQueryParams {
    #[serde(default, rename = "type")]
    pub article_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub category: Option<i32>,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/v1/admin/editorial/articles`
pub async fn editorial_list_articles(
    State(state): State<AppState>,
    Query(query): Query<EditorialArticlesQueryParams>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let list = state
        .db
        .editorial_list_articles(&EditorialArticleQuery {
            status: query.status,
            article_type: query.article_type,
            category_id: query.category,
            search: query.q,
            limit: query.limit.unwrap_or(50),
            offset: query.offset.unwrap_or(0),
        })
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "error": e })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "articles": list.articles,
        "totalCount": list.total,
        "stats": list.stats,
    })))
}

/// `GET /api/v1/admin/editorial/articles/:id`
pub async fn editorial_get_article(
    State(state): State<AppState>,
    Path(id_raw): Path<String>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let content_id = parse_positive_content_id(&id_raw)?;
    let article = state
        .db
        .editorial_get_article_by_id(content_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "error": e })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "error": "İçerik bulunamadı." })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "article": article,
    })))
}

/// `POST /api/v1/admin/editorial/articles`
pub async fn editorial_save_article(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(raw_payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;

    let revision_note = raw_payload
        .get("note")
        .or_else(|| raw_payload.get("revisionNote"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let article: Article = serde_json::from_value(raw_payload).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": format!("Geçersiz makale verisi: {e}") })),
        )
    })?;

    if is_privileged_lifecycle_status(&article.status) {
        check_permission(&operator, "contents:publish")?;
    }

    // Protect existing published/scheduled/archived items from unprivileged overwrite
    if let Some(existing_id) = parse_editorial_content_id(&article.id) {
        if let Ok(Some(existing_art)) = state.db.editorial_get_article_by_id(existing_id) {
            if is_privileged_lifecycle_status(&existing_art.status) {
                check_permission(&operator, "contents:publish")?;
            }
        }
    }

    let outcome = state
        .db
        .editorial_upsert_article(
            &article,
            operator.user_id,
            &operator.email,
            &operator_display_name(&operator),
            revision_note.as_deref(),
        )
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": e })),
            )
        })?;

    if outcome.article.status == "publish" {
        if let Some(ref slug) = outcome.article.slug {
            indexnow_service::spawn_instant_seo_push(
                (*state.config).clone(),
                vec![format!("/icerik/{slug}")],
            );
        }
    }

    Ok(Json(json!({
        "success": true,
        "id": outcome.article.id,
        "slug": outcome.article.slug,
        "coverWidth": outcome.article.cover_width,
        "coverHeight": outcome.article.cover_height,
        "thumbUrl": outcome.article.thumb_url,
        "readTimeMin": outcome.article.read_time_min,
        "article": outcome.article,
        "redirectCreated": outcome.redirect_created,
        "wasInsert": outcome.was_insert,
    })))
}

/// `DELETE /api/v1/admin/editorial/articles/:id`
pub async fn editorial_delete_article(
    State(state): State<AppState>,
    Path(id_raw): Path<String>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:delete")?;
    let content_id = parse_positive_content_id(&id_raw)?;
    let deleted = state
        .db
        .editorial_delete_article(content_id, &operator.email)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "error": e })),
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "Silinecek içerik bulunamadı." })),
        ));
    }
    let _ = editorial_lock_service::release_lock(
        &state.db.db_path,
        &content_id.to_string(),
        operator.user_id as i32,
    );

    Ok(Json(json!({ "success": true })))
}

/// `POST /api/v1/admin/editorial/articles/:id/refresh-stats`
pub async fn editorial_refresh_stats(
    State(state): State<AppState>,
    Path(id_raw): Path<String>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let content_id = parse_positive_content_id(&id_raw)?;
    let article = state
        .db
        .editorial_get_article_by_id(content_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "error": e })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "error": "İçerik bulunamadı." })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "views": article.views,
        "comments": article.comments,
    })))
}

/// `GET /api/v1/admin/editorial/articles/:id/revisions`
pub async fn editorial_list_revisions(
    State(state): State<AppState>,
    Path(id_raw): Path<String>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let content_id = parse_positive_content_id(&id_raw)?;
    let revisions = state.db.editorial_list_revisions(content_id);
    Ok(Json(json!({
        "success": true,
        "revisions": revisions,
    })))
}

/// `POST /api/v1/admin/editorial/articles/:id/revisions/:rev_id/restore`
pub async fn editorial_restore_revision(
    State(state): State<AppState>,
    Path((id_raw, rev_id)): Path<(String, i64)>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let content_id = parse_positive_content_id(&id_raw)?;

    if let Ok(Some(existing)) = state.db.editorial_get_article_by_id(content_id) {
        if is_privileged_lifecycle_status(&existing.status) {
            check_permission(&operator, "contents:publish")?;
        }
    }

    let restored = state
        .db
        .editorial_restore_revision(content_id, rev_id, &operator.email)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": e })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "article": restored,
    })))
}

// ── Locks ─────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/editorial/locks`
pub async fn editorial_list_locks(
    State(state): State<AppState>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let locks = editorial_lock_service::list_locks(&state.db.db_path);
    Ok(Json(json!({
        "success": true,
        "locks": locks,
    })))
}

/// `POST /api/v1/admin/editorial/articles/:id/lock`
pub async fn editorial_acquire_lock(
    State(state): State<AppState>,
    Path(id_raw): Path<String>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let clean_id = id_raw.trim();
    if clean_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Geçersiz kilit kimliği." })),
        ));
    }
    let display = operator_display_name(&operator);
    match editorial_lock_service::acquire_lock(
        &state.db.db_path,
        clean_id,
        operator.user_id as i32,
        &display,
    ) {
        Ok(lock) => Ok(Json(json!({
            "success": true,
            "conflict": false,
            "lock": lock,
        }))),
        Err(existing_lock) => Ok(Json(json!({
            "success": true,
            "conflict": true,
            "lockedBy": existing_lock,
        }))),
    }
}

/// `DELETE /api/v1/admin/editorial/articles/:id/lock`
pub async fn editorial_release_lock(
    State(state): State<AppState>,
    Path(id_raw): Path<String>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let clean_id = id_raw.trim();
    match editorial_lock_service::release_lock(
        &state.db.db_path,
        clean_id,
        operator.user_id as i32,
    ) {
        Ok(()) => Ok(Json(json!({ "success": true }))),
        Err(holder) => Err((
            StatusCode::CONFLICT,
            Json(json!({
                "success": false,
                "error": format!("Bu içerik şu anda {} tarafından düzenleniyor.", holder.username),
                "lockedBy": holder,
            })),
        )),
    }
}

// ── Categories ────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
pub struct EditorialCategoryInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default, alias = "parentId")]
    pub parent_id: Option<i32>,
}

/// `GET /api/v1/admin/editorial/categories`
pub async fn editorial_list_categories(
    State(state): State<AppState>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let categories = state.db.editorial_list_categories();
    Ok(Json(json!({
        "success": true,
        "categories": categories,
    })))
}

/// `POST /api/v1/admin/editorial/categories`
pub async fn editorial_create_category(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(body): Json<EditorialCategoryInput>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "categories:write")?;
    let category = state
        .db
        .editorial_upsert_category(None, &body.name, body.slug.as_deref(), body.parent_id)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": e })),
            )
        })?;
    Ok(Json(json!({
        "success": true,
        "category": category,
    })))
}

/// `PUT /api/v1/admin/editorial/categories/:id`
pub async fn editorial_update_category(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    operator: OperatorUser,
    Json(body): Json<EditorialCategoryInput>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "categories:write")?;
    let category = state
        .db
        .editorial_upsert_category(Some(id), &body.name, body.slug.as_deref(), body.parent_id)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": e })),
            )
        })?;
    Ok(Json(json!({
        "success": true,
        "category": category,
    })))
}

/// `DELETE /api/v1/admin/editorial/categories/:id`
pub async fn editorial_delete_category(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "categories:delete")?;
    let ok = state.db.editorial_delete_category(id).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": e })),
        )
    })?;
    if !ok {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "Kategori bulunamadı." })),
        ));
    }
    Ok(Json(json!({
        "success": true,
        "categories": state.db.editorial_list_categories(),
    })))
}

// ── Meta, Issues, Glossary, Tags, Redirects ───────────────────────────────────

/// `GET /api/v1/admin/editorial/meta`
pub async fn editorial_meta(
    State(state): State<AppState>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    Ok(Json(json!({
        "success": true,
        "ssbTags": state.db.editorial_list_ssb_tags(),
        "issues": state.db.editorial_list_issues(),
        "glossary": state.db.editorial_list_glossary(),
    })))
}

/// `POST /api/v1/admin/editorial/issues`
pub async fn editorial_upsert_issue(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(body): Json<MagazineIssueItem>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "issues:write")?;
    state.db.editorial_upsert_issue(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": e })),
        )
    })?;
    Ok(Json(json!({
        "success": true,
        "issues": state.db.editorial_list_issues(),
    })))
}

#[derive(Debug, Default, Deserialize)]
pub struct EditorialDeleteIssueQuery {
    #[serde(default)]
    pub publication_type: Option<String>,
    #[serde(default)]
    pub edition_lang: Option<String>,
}

/// `DELETE /api/v1/admin/editorial/issues/:id`
pub async fn editorial_delete_issue(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(query): Query<EditorialDeleteIssueQuery>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "issues:delete")?;
    let pub_type = query
        .publication_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            query
                .edition_lang
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        });
    let deleted = state
        .db
        .editorial_delete_issue(id, pub_type)
        .map_err(|e| {
            let status = if e.contains("birden fazla") || e.contains("hem msi hem") {
                StatusCode::CONFLICT
            } else if e.contains("Geçersiz yayın türü") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({ "success": false, "error": e })))
        })?;
    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "Dergi sayısı bulunamadı." })),
        ));
    }
    Ok(Json(json!({
        "success": true,
        "issues": state.db.editorial_list_issues(),
    })))
}

fn check_glossary_permission(operator: &OperatorUser) -> Result<(), (StatusCode, Json<Value>)> {
    if operator.has_permission("glossary:write") || operator.has_permission("glossary:manage") {
        Ok(())
    } else {
        check_permission(operator, "glossary:write")
    }
}

/// `POST /api/v1/admin/editorial/glossary`
pub async fn editorial_upsert_glossary(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(body): Json<GlossaryRule>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_glossary_permission(&operator)?;
    state
        .db
        .editorial_upsert_glossary(&body, &operator.email)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": e })),
            )
        })?;
    Ok(Json(json!({
        "success": true,
        "glossary": state.db.editorial_list_glossary(),
    })))
}

/// `DELETE /api/v1/admin/editorial/glossary/:id`
pub async fn editorial_delete_glossary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_glossary_permission(&operator)?;
    let deleted = state.db.editorial_delete_glossary(id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": e })),
        )
    })?;
    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "Sözlük kuralı bulunamadı." })),
        ));
    }
    Ok(Json(json!({
        "success": true,
        "glossary": state.db.editorial_list_glossary(),
    })))
}

/// `GET /api/v1/admin/editorial/tags`
pub async fn editorial_list_tags(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let tags = state
        .db
        .editorial_list_tags(query.get("q").map(String::as_str));
    Ok(Json(json!({
        "success": true,
        "tags": tags,
    })))
}

/// `GET /api/v1/admin/editorial/redirects`
pub async fn editorial_list_redirects(
    State(state): State<AppState>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    Ok(Json(json!({
        "success": true,
        "redirects": state.db.editorial_list_redirects(),
    })))
}

/// `DELETE /api/v1/admin/editorial/redirects/:id`
pub async fn editorial_delete_redirect(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "redirects:write")?;
    let ok = state.db.editorial_delete_redirect(id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": e })),
        )
    })?;
    if !ok {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "Yönlendirme kuralı bulunamadı." })),
        ));
    }
    Ok(Json(json!({ "success": true })))
}

// ── Media Archive & Image Processing ──────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
pub struct EditorialMediaListQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// `GET /api/v1/admin/editorial/media`
pub async fn editorial_list_media(
    State(state): State<AppState>,
    Query(query): Query<EditorialMediaListQuery>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let (assets, total) = state.db.editorial_list_media(
        query.q.as_deref(),
        query.limit.unwrap_or(40),
        query.offset.unwrap_or(0),
    );
    Ok(Json(json!({
        "success": true,
        "assets": assets,
        "totalCount": total,
    })))
}

#[derive(Debug, Default, Deserialize)]
pub struct UploadQueryParams {
    #[serde(default)]
    pub watermark: Option<String>,
}

fn is_watermark_flag(raw: Option<&str>) -> bool {
    raw.map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// `POST /api/v1/admin/editorial/upload`
pub async fn editorial_upload_image(
    State(state): State<AppState>,
    operator: OperatorUser,
    Query(query): Query<UploadQueryParams>,
    multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "media:upload")?;
    let query_wm = is_watermark_flag(query.watermark.as_deref());
    let parsed = editorial_media_service::parse_multipart_upload(multipart, query_wm)
        .await
        .map_err(editorial_error_response)?;

    let up_dir = editorial_media_service::resolve_editorial_upload_dir(&state.config);
    let st_dir = state.config.static_dir.clone();
    let bytes = parsed.bytes;
    let orig_name = parsed.orig_name;
    let apply_wm = parsed.watermark;

    let processed = tokio::task::spawn_blocking(move || {
        editorial_media_service::process_and_store_image(
            &up_dir,
            &st_dir,
            &bytes,
            &orig_name,
            apply_wm,
        )
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": format!("Görsel işleme hatası: {e}") })),
        )
    })?
    .map_err(editorial_error_response)?;

    let asset = state
        .db
        .editorial_insert_media(&EditorialMediaInsert {
            filename: processed.filename,
            original_name: processed.original_name,
            url: processed.url,
            thumb_url: processed.thumb_url,
            mime_type: processed.mime_type,
            width: processed.width,
            height: processed.height,
            size_bytes: processed.size_bytes,
            watermarked: processed.watermarked,
            alt_text: parsed.alt_text,
            uploaded_by: operator.email.clone(),
        })
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "error": e })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "url": asset.url,
        "thumbUrl": asset.thumb_url,
        "width": asset.width,
        "height": asset.height,
        "sizeBytes": asset.size_bytes,
        "watermarked": asset.watermarked,
        "asset": asset,
    })))
}

#[derive(Debug, Default, Deserialize)]
pub struct FetchRemoteImageRequest {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub watermark: Option<bool>,
}

/// `POST /api/v1/admin/editorial/fetch-remote-image`
pub async fn editorial_fetch_remote_image(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(body): Json<FetchRemoteImageRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "media:upload")?;
    let up_dir = editorial_media_service::resolve_editorial_upload_dir(&state.config);
    let processed = editorial_media_service::fetch_and_process_remote_image(
        &up_dir,
        &state.config.static_dir,
        &body.url,
        body.watermark.unwrap_or(false),
    )
    .await
    .map_err(editorial_error_response)?;

    let asset = state
        .db
        .editorial_insert_media(&EditorialMediaInsert {
            filename: processed.filename,
            original_name: processed.original_name,
            url: processed.url,
            thumb_url: processed.thumb_url,
            mime_type: processed.mime_type,
            width: processed.width,
            height: processed.height,
            size_bytes: processed.size_bytes,
            watermarked: processed.watermarked,
            alt_text: None,
            uploaded_by: operator.email.clone(),
        })
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "error": e })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "url": asset.url,
        "thumbUrl": asset.thumb_url,
        "width": asset.width,
        "height": asset.height,
        "sizeBytes": asset.size_bytes,
        "watermarked": asset.watermarked,
        "asset": asset,
    })))
}

#[derive(Debug, Default, Deserialize)]
pub struct WatermarkUrlRequest {
    #[serde(default)]
    pub url: String,
    #[serde(default, alias = "altText")]
    pub alt_text: Option<String>,
}

/// `POST /api/v1/admin/editorial/media/watermark-url`
pub async fn editorial_watermark_url(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(body): Json<WatermarkUrlRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "media:upload")?;
    let existing = state.db.editorial_find_media_by_url(&body.url);
    let orig_hint = existing.as_ref().map(|a| a.original_name.clone());
    let alt_text = body
        .alt_text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| existing.as_ref().and_then(|a| a.alt_text.clone()));

    let up_dir = editorial_media_service::resolve_editorial_upload_dir(&state.config);
    let processed = editorial_media_service::watermark_local_upload_url(
        &up_dir,
        &state.config.static_dir,
        &body.url,
        orig_hint.as_deref(),
    )
    .await
    .map_err(editorial_error_response)?;

    let asset = state
        .db
        .editorial_insert_media(&EditorialMediaInsert {
            filename: processed.filename,
            original_name: processed.original_name,
            url: processed.url,
            thumb_url: processed.thumb_url,
            mime_type: processed.mime_type,
            width: processed.width,
            height: processed.height,
            size_bytes: processed.size_bytes,
            watermarked: processed.watermarked,
            alt_text,
            uploaded_by: operator.email.clone(),
        })
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "error": e })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "url": asset.url,
        "thumbUrl": asset.thumb_url,
        "width": asset.width,
        "height": asset.height,
        "sizeBytes": asset.size_bytes,
        "watermarked": asset.watermarked,
        "asset": asset,
    })))
}

/// `DELETE /api/v1/admin/editorial/media/:id`
pub async fn editorial_delete_media(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "media:delete")?;
    let deleted = state.db.editorial_delete_media(id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": e })),
        )
    })?;
    let Some(asset) = deleted else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "Medya dosyası bulunamadı." })),
        ));
    };
    let up_dir = editorial_media_service::resolve_editorial_upload_dir(&state.config);
    editorial_media_service::remove_media_files_from_disk(&up_dir, &asset).await;
    Ok(Json(json!({ "success": true })))
}

// ── Settings & Social Distribution ────────────────────────────────────────────

/// `GET /api/v1/admin/editorial/settings`
pub async fn editorial_get_settings(
    State(state): State<AppState>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let settings = state.db.get_setting_map("tr");
    let wp_cfg = settings
        .get("editorial_studio_settings")
        .cloned()
        .unwrap_or_else(|| "{}".to_string());
    let social_cfg = settings
        .get("editorial_social_config")
        .cloned()
        .unwrap_or_else(|| "{}".to_string());
    Ok(Json(json!({
        "success": true,
        "settings": {
            "wp_config": wp_cfg,
            "social_config": social_cfg,
        }
    })))
}

/// `POST /api/v1/admin/editorial/settings`
pub async fn editorial_save_settings(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !operator.has_permission("settings:write") && !operator.has_permission("contents:publish") {
        check_permission(&operator, "settings:write")?;
    }
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(wp_cfg) = body.get("wp_config").and_then(|v| v.as_str()) {
        let _ = state.db.upsert_site_setting(&SiteSetting {
            key: "editorial_studio_settings".to_string(),
            value_tr: wp_cfg.to_string(),
            value_en: wp_cfg.to_string(),
            setting_group: "editorial".to_string(),
            updated_at: now.clone(),
        });
        let _ = state.db.upsert_site_setting(&SiteSetting {
            key: "editorial_social_config".to_string(),
            value_tr: wp_cfg.to_string(),
            value_en: wp_cfg.to_string(),
            setting_group: "editorial".to_string(),
            updated_at: now.clone(),
        });
    }
    if let Some(social_cfg) = body.get("social_config").and_then(|v| v.as_str()) {
        let _ = state.db.upsert_site_setting(&SiteSetting {
            key: "editorial_social_config".to_string(),
            value_tr: social_cfg.to_string(),
            value_en: social_cfg.to_string(),
            setting_group: "editorial".to_string(),
            updated_at: now,
        });
    }
    Ok(Json(json!({ "success": true })))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualSocialShareRequest {
    #[serde(default, alias = "article_id")]
    pub article_id: String,
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
}

/// `POST /api/v1/admin/editorial/social/share`
pub async fn editorial_social_share(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(body): Json<ManualSocialShareRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:publish")?;
    let content_id = parse_positive_content_id(&body.article_id)?;
    let article = state
        .db
        .editorial_get_article_by_id(content_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "error": e })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "error": "İçerik bulunamadı." })),
            )
        })?;

    let cfg_val = extract_social_cfg_json(&state);
    let slug = article.slug.as_deref().unwrap_or("");
    let public_url = if !slug.is_empty() {
        format!("{}/icerik/{}", state.config.public_url.trim_end_matches('/'), slug)
    } else {
        state.config.public_url.clone()
    };

    let mut results = serde_json::Map::new();
    let http = reqwest::Client::new();
    let want_tg = body
        .platforms
        .as_ref()
        .map(|p| p.iter().any(|x| x.eq_ignore_ascii_case("telegram")))
        .unwrap_or(true);
    let want_tw = body
        .platforms
        .as_ref()
        .map(|p| p.iter().any(|x| x.eq_ignore_ascii_case("twitter") || x.eq_ignore_ascii_case("x")))
        .unwrap_or(true);

    if want_tg {
        if let Some((tok, cid)) = editorial_social_service::extract_telegram_creds(&cfg_val) {
            let txt = editorial_social_service::format_telegram_share_text(
                &article.title,
                article.spot.as_deref().unwrap_or(""),
                Some(&public_url),
            );
            let res = editorial_social_service::share_on_telegram(&http, &tok, &cid, &txt).await;
            results.insert("telegram".to_string(), res);
        }
    }
    if want_tw {
        if let Some(tw) = editorial_social_service::extract_twitter_creds(&cfg_val) {
            let txt = editorial_social_service::format_twitter_share_text(
                &article.title,
                Some(&public_url),
            );
            let res = editorial_social_service::share_on_twitter(&http, &tw, &txt).await;
            results.insert("twitter".to_string(), res);
        }
    }

    Ok(Json(json!({
        "success": true,
        "results": Value::Object(results),
    })))
}

/// `POST /api/v1/admin/editorial/social/test-twitter`
pub async fn editorial_test_twitter(
    operator: OperatorUser,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:publish")?;
    let creds = editorial_social_service::TwitterConfigInput::from_json_value(&body);
    if !creds.is_complete() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": "Eksik X (Twitter) API kimlik bilgileri." })),
        ));
    }
    let http = reqwest::Client::new();
    let res = editorial_social_service::share_on_twitter(
        &http,
        &creds,
        "MSI Dergisi Editör Stüdyosu bağlantı testi.",
    )
    .await;
    Ok(Json(res))
}

#[derive(Debug, Default, Deserialize)]
pub struct TestTelegramRequest {
    #[serde(default, alias = "botToken")]
    pub bot_token: String,
    #[serde(default, alias = "chatId")]
    pub chat_id: String,
}

/// `POST /api/v1/admin/editorial/social/test-telegram`
pub async fn editorial_test_telegram(
    operator: OperatorUser,
    Json(body): Json<TestTelegramRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:publish")?;
    let http = reqwest::Client::new();
    let res = editorial_social_service::share_on_telegram(
        &http,
        &body.bot_token,
        &body.chat_id,
        "<b>MSI Dergisi</b> — Editör Stüdyosu Telegram bağlantı testi başarılı.",
    )
    .await;
    Ok(Json(res))
}

// ── Live Research & Reader View ───────────────────────────────────────────────

/// `GET /api/v1/admin/editorial/research/sources`
pub async fn editorial_list_research_sources(
    State(state): State<AppState>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    Ok(Json(json!({
        "success": true,
        "sources": state.db.editorial_list_research_sources(),
    })))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResearchSourceInput {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

/// `POST /api/v1/admin/editorial/research/sources`
pub async fn editorial_create_research_source(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(body): Json<CreateResearchSourceInput>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let spec = editorial_research_service::discover_and_normalize_source(
        body.name.as_deref().unwrap_or(""),
        &body.url,
        body.category.as_deref().unwrap_or("Savunma"),
        body.scope.as_deref().unwrap_or("tr"),
        true,
    )
    .await
    .map_err(editorial_error_response)?;

    let created = state
        .db
        .editorial_create_research_source(
            &spec.name,
            &spec.url,
            &spec.kind,
            &spec.category,
            &spec.scope,
        )
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": e })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "source": created,
    })))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResearchSourceInput {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// `PUT /api/v1/admin/editorial/research/sources/:id`
pub async fn editorial_update_research_source(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    operator: OperatorUser,
    Json(body): Json<UpdateResearchSourceInput>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let updated = state
        .db
        .editorial_update_research_source(
            id,
            body.name.as_deref(),
            body.url.as_deref(),
            body.kind.as_deref(),
            body.category.as_deref(),
            body.scope.as_deref(),
            body.enabled,
        )
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "error": e })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "error": "Araştırma kaynağı bulunamadı." })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "source": updated,
    })))
}

/// `DELETE /api/v1/admin/editorial/research/sources/:id`
pub async fn editorial_delete_research_source(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let ok = state
        .db
        .editorial_delete_research_source(id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "error": e })),
            )
        })?;
    if !ok {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "Araştırma kaynağı bulunamadı." })),
        ));
    }
    Ok(Json(json!({ "success": true })))
}

#[derive(Debug, Default, Deserialize)]
pub struct ResearchSearchQueryParams {
    #[serde(default)]
    pub q: String,
    #[serde(default = "default_scope")]
    pub scope: String,
    #[serde(default = "default_time_filter")]
    pub time: String,
    #[serde(default)]
    pub source_id: Option<i32>,
}

fn default_scope() -> String {
    "tr".to_string()
}

fn default_time_filter() -> String {
    "24h".to_string()
}

/// `GET /api/v1/admin/editorial/research/search`
pub async fn editorial_research_search(
    State(state): State<AppState>,
    Query(query): Query<ResearchSearchQueryParams>,
    operator: OperatorUser,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let sources = state.db.editorial_list_research_sources();
    let source_filter = query
        .source_id
        .and_then(|sid| sources.iter().find(|s| s.id == sid));
    let mut items = editorial_research_service::search_news(
        &query.q,
        &query.scope,
        &query.time,
        source_filter,
        &sources,
    )
    .await
    .map_err(editorial_error_response)?;
    items.truncate(MAX_RESEARCH_LIMIT);
    Ok(Json(json!({
        "success": true,
        "items": items,
    })))
}

/// `POST /api/v1/admin/editorial/research/read`
pub async fn editorial_research_read(
    operator: OperatorUser,
    Json(body): Json<editorial_research_service::ReadArticleRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_permission(&operator, "contents:write")?;
    let article = editorial_research_service::extract_reader_article(body)
        .await
        .map_err(editorial_error_response)?;
    Ok(Json(json!({
        "success": true,
        "article": article,
    })))
}
