use super::{
    components::{self, PageData},
    config::{self, DesignConfig},
    shell,
};
use crate::{
    controllers::AppState,
    engine::{
        api_response::{msg_keys, AppError},
        security_engine::{escape_html, CspNonce},
        seo_router_engine,
    },
    extractors::{operator_user::OperatorUser, require_permission::check_permission},
    models::SiteSetting,
};
use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;

pub async fn view(
    State(state): State<AppState>,
    Extension(csp_nonce): Extension<CspNonce>,
    _operator: OperatorUser,
) -> Html<String> {
    let admin_base = format!("/{}", escape_html(&state.config.admin_path_secret));
    let mut html = include_str!("editor.html")
        .replace("href=\"/admin\"", &format!("href=\"{}\"", admin_base))
        .replace(
            "<body class=\"design-editor\">",
            &format!(
                "<body class=\"design-editor\" data-admin-path=\"{}\">",
                admin_base
            ),
        );
    let safe_nonce = escape_html(&csp_nonce.0);
    html = html.replace("<script ", &format!("<script nonce=\"{}\" ", safe_nonce));
    html = html.replace("<script>", &format!("<script nonce=\"{}\">", safe_nonce));
    Html(html)
}

pub async fn get(
    State(state): State<AppState>,
    _operator: OperatorUser,
) -> Json<serde_json::Value> {
    Json(json!({
        "config": config::load(&state.db.get_setting_map("tr")),
        "defaults": config::defaults(),
        "categories": state.db.get_all_categories()
    }))
}

pub async fn save(
    State(state): State<AppState>,
    operator: OperatorUser,
    Json(config): Json<DesignConfig>,
) -> Response {
    if let Err(resp) = check_permission(&operator, "settings:manage") {
        return resp.into_response();
    }
    if let Err(error) = config.validate() {
        return AppError::UnprocessableEntity {
            message: error.to_string(),
        }
        .into_api_response(&state.translations, "tr")
        .into_response();
    }
    let value = match serde_json::to_string(&config) {
        Ok(v) => v,
        Err(e) => {
            return AppError::internal("design_serialize", e, msg_keys::SERVER_ERROR_SAVE)
                .into_api_response(&state.translations, "tr")
                .into_response();
        }
    };
    let existing_en = state
        .db
        .get_setting_map("en")
        .get(config::SETTING_KEY)
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| value.clone());
    let setting = SiteSetting {
        key: config::SETTING_KEY.into(),
        value_tr: value,
        value_en: existing_en,
        setting_group: "design".into(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    match state.db.upsert_site_setting(&setting) {
        Ok(_) => {
            let _ = state.db.insert_audit_log(
                &operator.email,
                "design.save",
                "design_config",
                None,
                r#"{"summary":"Tasarım stüdyosu yapılandırması güncellendi"}"#,
                "127.0.0.1",
            );
            Json(json!({"status": "saved"})).into_response()
        }
        Err(e) => AppError::internal("design_save", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response(),
    }
}

pub async fn preview(
    State(state): State<AppState>,
    Extension(csp_nonce): Extension<CspNonce>,
    _operator: OperatorUser,
    Query(params): Query<HashMap<String, String>>,
    Json(config): Json<DesignConfig>,
) -> Response {
    if let Err(error) = config.validate() {
        return AppError::UnprocessableEntity {
            message: error.to_string(),
        }
        .into_api_response(&state.translations, "tr")
        .into_response();
    }
    let slug = params.get("page").map(String::as_str).unwrap_or("magazine");
    let Some(page) = config.page(slug) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let snap = crate::engine::query_engine::CentralQueryEngine::snapshot(&state.db).data;
    let mut settings = state.db.get_setting_map("tr");
    if let Ok(cfg_str) = serde_json::to_string(&config) {
        settings.insert(config::SETTING_KEY.into(), cfg_str);
    }
    let data = PageData {
        lang: "tr",
        contents: &snap.contents,
        issues: &snap.issues,
        tags: &snap.ssb_tags,
        ads: &snap.ads,
        plans: &snap.subscription_plans,
        products: &snap.commerce_products,
        featured_issue: settings
            .get("current_featured_issue")
            .and_then(|s| s.parse().ok()),
    };
    let body = components::render_page(page, &data);
    let settings_map = state.db.get_setting_map("tr");
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings_map);
    let preview_path = format!("/{}/tasarim/onizleme", state.config.admin_path_secret);
    let seo = seo_router_engine::build_seo_meta(
        &state.config.public_url,
        &preview_path,
        &page.title,
        &page.description,
        "/images/logo.svg",
        "website",
        "tr",
        None,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );
    let html = shell::render_with_nonce(
        "tr",
        &page.mode,
        &settings,
        &seo,
        &snap.categories,
        &snap.ssb_tags,
        &snap.issues,
        &snap.contents,
        &snap.notifications,
        &body,
        0,
        Some(&csp_nonce.0),
    );
    Html(html).into_response()
}
