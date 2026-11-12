use super::{
    components::{self, PageData},
    config::{self, DesignConfig},
    shell,
};
use crate::{
    controllers::AppState,
    engine::{
        api_response::{AppError, msg_keys},
        seo_router_engine,
    },
    models::SiteSetting,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;

pub async fn view() -> Html<&'static str> {
    Html(include_str!("editor.html"))
}
pub async fn get(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(
        json!({"config":config::load(&state.db.get_setting_map("tr")),"defaults":config::defaults(),"categories":state.db.get_all_categories()}),
    )
}
pub async fn save(State(state): State<AppState>, Json(config): Json<DesignConfig>) -> Response {
    if let Err(error) = config.validate() {
        return AppError::UnprocessableEntity { message: error.to_string() }
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
    let setting = SiteSetting {
        key: config::SETTING_KEY.into(),
        value_tr: value.clone(),
        value_en: value,
        setting_group: "design".into(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    match state.db.upsert_site_setting(&setting) {
        Ok(_) => Json(json!({"status":"saved"})).into_response(),
        Err(e) => AppError::internal("design_save", e, msg_keys::SERVER_ERROR_SAVE)
            .into_api_response(&state.translations, "tr")
            .into_response(),
    }
}
pub async fn preview(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    Json(config): Json<DesignConfig>,
) -> Response {
    if let Err(error) = config.validate() {
        return AppError::UnprocessableEntity { message: error.to_string() }
            .into_api_response(&state.translations, "tr")
            .into_response();
    }
    let slug = params.get("page").map(String::as_str).unwrap_or("magazine");
    let Some(page) = config.page(slug) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let snap = crate::engine::query_engine::CentralQueryEngine::snapshot(&state.db).data;
    let mut settings = state.db.get_setting_map("tr");
    settings.insert(
        config::SETTING_KEY.into(),
        serde_json::to_string(&config).unwrap(),
    );
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
    let seo = seo_router_engine::build_seo_meta(
        &state.config.public_url,
        "/admin/tasarim/onizleme",
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
    let html = shell::render(
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
    );
    Html(html).into_response()
}
