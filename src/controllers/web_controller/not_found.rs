use super::*;
use crate::engine::{
    design::html_dsl::HtmlBuf,
    design::labels_helper::{lang_href, tr_text},
};

pub async fn render_not_found_with_nonce(
    state: &AppState,
    headers: &HeaderMap,
    nonce: Option<&str>,
) -> Response {
    let lang = detect_language(&HashMap::new(), headers);
    let tr = &state.translations;

    let badge = tr_text(Some(tr), &lang, "error.404_badge", "SAYFA BULUNAMADI", "PAGE NOT FOUND");
    let title = tr_text(
        Some(tr),
        &lang,
        "error.404_title",
        "Aradığınız Sayfa Bulunamadı",
        "Page Not Found",
    );
    let desc = tr_text(
        Some(tr),
        &lang,
        "error.404_desc",
        "İstediğiniz içerik taşınmış olabilir veya adres hatalı girilmiş olabilir.",
        "The content you requested may have been moved or the address may be incorrect.",
    );
    let home_lbl = tr_text(Some(tr), &lang, "error.404_home", "Ana Sayfaya Dön", "Return to Home");
    let search_lbl = tr_text(Some(tr), &lang, "error.404_search", "İçerikleri ara", "Search content");
    let page_title = tr_text(
        Some(tr),
        &lang,
        "error.404_page_title",
        "404 Sayfa Bulunamadı",
        "404 Page Not Found",
    );

    let home_href = lang_href("/", &lang);
    let search_href = lang_href("/arama", &lang);

    let mut buf = HtmlBuf::with_capacity(512);
    buf.el("section", &[("class", "container section-space")], |sec| {
        sec.el("div", &[("class", "transactional-card not-found-card")], |card| {
            card.el("span", &[("class", "msi-badge msi-badge-red")], |b| {
                b.text(&badge);
            });
            card.el("h1", &[], |h| {
                h.text(&title);
            });
            card.el("p", &[], |p| {
                p.text(&desc);
            });
            card.el("div", &[("class", "hero-actions")], |acts| {
                acts.el("a", &[("href", &home_href), ("class", "button primary")], |a| {
                    a.text(&home_lbl);
                });
                acts.el("a", &[("href", &search_href), ("class", "button secondary")], |a| {
                    a.text(&search_lbl);
                });
            });
        });
    });

    let html = home::render_shared_shell_with_request(
        state,
        &lang,
        &page_title,
        buf.as_str(),
        None,
        None,
        true,
        nonce,
        Some(headers),
        None,
    );
    (StatusCode::NOT_FOUND, Html(html)).into_response()
}

pub async fn render_not_found(State(state): State<AppState>, headers: HeaderMap) -> Response {
    render_not_found_with_nonce(&state, &headers, None).await
}
