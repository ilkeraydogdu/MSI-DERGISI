use super::*;

pub async fn render_not_found_with_nonce(
    state: &AppState,
    headers: &HeaderMap,
    nonce: Option<&str>,
) -> Response {
    let lang = detect_language(&HashMap::new(), headers);
    let tr = &state.translations;
    let body_html = format!(
        r##"<section class="py-5 bg-light text-center">
        <div class="container py-5">
            <span class="badge bg-danger mb-2">{badge}</span>
            <h1 class="fw-bold mb-3">{title}</h1>
            <p class="text-muted mb-4">{desc}</p>
            <div class="d-flex justify-content-center gap-2">
                <a href="/" class="btn btn-danger px-4">{home}</a>
                <a href="/arama" class="btn btn-outline-dark px-4">{search}</a>
            </div>
        </div>
    </section>"##,
        badge = tr.t(&lang, "error.404_badge"),
        title = tr.t(&lang, "error.404_title"),
        desc = tr.t(&lang, "error.404_desc"),
        home = tr.t(&lang, "error.404_home"),
        search = tr.t(&lang, "error.404_search"),
    );
    let html = render_shared_shell(
        state,
        &lang,
        &tr.t(&lang, "error.404_page_title"),
        &body_html,
        None,
        None,
        true,
        nonce,
    );
    (StatusCode::NOT_FOUND, Html(html)).into_response()
}

pub async fn render_not_found(State(state): State<AppState>, headers: HeaderMap) -> Response {
    render_not_found_with_nonce(&state, &headers, None).await
}

