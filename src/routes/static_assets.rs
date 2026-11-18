use axum::Router;
use tower_http::services::ServeDir;

use crate::controllers::AppState;

pub fn build(base_dir: &str) -> Router<AppState> {
    Router::new()
        .nest_service("/css", ServeDir::new(format!("{}/css", base_dir)))
        .nest_service("/js", ServeDir::new(format!("{}/js", base_dir)))
        .nest_service("/images", ServeDir::new(format!("{}/images", base_dir)))
        .nest_service("/fonts", ServeDir::new(format!("{}/fonts", base_dir)))
}