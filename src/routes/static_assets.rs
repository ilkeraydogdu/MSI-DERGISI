use axum::{
    extract::Request,
    http::{header::HeaderName, HeaderValue},
    middleware::{self, Next},
    response::Response,
    Router,
};
use tower_http::services::ServeDir;

use crate::controllers::AppState;

async fn harden_uploaded_svg_headers(req: Request, next: Next) -> Response {
    let path_lc = req.uri().path().to_ascii_lowercase();
    let is_uploaded_svg = path_lc.ends_with(".svg")
        && (path_lc.starts_with("/uploads/editorial/") || path_lc.starts_with("/images/uploads/"));
    let mut resp = next.run(req).await;
    if is_uploaded_svg {
        resp.headers_mut().insert(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        );
        resp.headers_mut().insert(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("sandbox; default-src 'none'"),
        );
    }
    resp
}

pub fn build(base_dir: &str) -> Router<AppState> {
    Router::new()
        .nest_service("/css", ServeDir::new(format!("{}/css", base_dir)))
        .nest_service("/js", ServeDir::new(format!("{}/js", base_dir)))
        .nest_service("/images", ServeDir::new(format!("{}/images", base_dir)))
        .nest_service("/fonts", ServeDir::new(format!("{}/fonts", base_dir)))
        .layer(middleware::from_fn(harden_uploaded_svg_headers))
}

pub fn build_with_uploads(base_dir: &str, editorial_upload_dir: &str) -> Router<AppState> {
    build(base_dir)
        .nest_service("/uploads/editorial", ServeDir::new(editorial_upload_dir))
        .layer(middleware::from_fn(harden_uploaded_svg_headers))
}