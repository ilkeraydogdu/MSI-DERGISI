pub mod admin_routes;
pub mod api_routes;
pub mod public_routes;
pub mod static_assets;

use axum::Router;

use crate::app_state::AppState;
use crate::controllers::web_controller;

pub fn build_router(state: AppState) -> Router {
    let base_dir = state.config.static_dir.trim_end_matches('/').to_string();

    Router::new()
        .merge(public_routes::build(&state))
        .merge(api_routes::build())
        .merge(admin_routes::build())
        .merge(static_assets::build(&base_dir))
        // Fallback: 301 SEO redirects, legacy .html pages, direct category/article slugs, stealth 404 for /admin etc.
        .fallback(web_controller::handle_fallback_or_redirect)
        .with_state(state)
}