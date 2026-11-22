use super::*;

pub async fn redirect_en_home() -> Response {
    Redirect::permanent("/en").into_response()
}

pub async fn redirect_short_corporate_tr(uri: axum::http::Uri) -> Response {
    let target = match uri.path().trim_matches('/') {
        "kunye" => "/kurumsal/kunye",
        "hakkimizda" => "/kurumsal/hakkimizda",
        "yolculugumuz" => "/kurumsal/yolculugumuz",
        "kvkk" => "/kurumsal/kvkk",
        "iletisim" => "/kurumsal/iletisim",
        "reklam" => "/kurumsal/reklam-ve-sponsorluk",
        _ => "/kurumsal/hakkimizda",
    };
    Redirect::permanent(target).into_response()
}

pub async fn redirect_short_corporate_en(uri: axum::http::Uri) -> Response {
    let slug = uri.path().trim_matches('/').strip_prefix("en/").unwrap_or("");
    let target = match slug {
        "kunye" | "imprint" => "/kurumsal/kunye?lang=en",
        "hakkimizda" | "about" | "about-us" => "/kurumsal/hakkimizda?lang=en",
        "yolculugumuz" | "our-journey" => "/kurumsal/yolculugumuz?lang=en",
        "kvkk" | "privacy-policy" => "/kurumsal/kvkk?lang=en",
        "iletisim" | "contact" => "/kurumsal/iletisim?lang=en",
        _ => "/kurumsal/hakkimizda?lang=en",
    };
    Redirect::permanent(target).into_response()
}

pub async fn redirect_account_tr() -> Response {
    Redirect::temporary("/hesabim").into_response()
}

pub async fn redirect_account_en() -> Response {
    Redirect::temporary("/hesabim?lang=en").into_response()
}

pub async fn redirect_newsletter_unsub_tr(Path(token): Path<String>) -> Response {
    Redirect::temporary(&format!("/abonelik-iptal?token={}", urlencoding::encode(token.trim())))
        .into_response()
}

pub async fn redirect_newsletter_unsub_en(Path(token): Path<String>) -> Response {
    Redirect::temporary(&format!(
        "/abonelik-iptal?lang=en&token={}",
        urlencoding::encode(token.trim())
    ))
    .into_response()
}

pub async fn switch_locale(
    Path(locale): Path<String>,
    headers: HeaderMap,
) -> Response {
    let norm = locale.trim().to_ascii_lowercase();
    if norm != "tr" && norm != "en" {
        return (StatusCode::BAD_REQUEST, "Invalid locale").into_response();
    }

    let req_host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(str::trim);

    let safe_referer_path = headers
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .and_then(|ref_str| {
            if ref_str.starts_with('/') && !ref_str.starts_with("//") {
                Some(ref_str.to_string())
            } else if let Some(rest) = ref_str
                .strip_prefix("https://")
                .or_else(|| ref_str.strip_prefix("http://"))
            {
                let (host, path_part) = match rest.find('/') {
                    Some(idx) => (&rest[..idx], &rest[idx..]),
                    None => (rest, "/"),
                };
                if req_host.map(|h| h.eq_ignore_ascii_case(host)).unwrap_or(false)
                    && path_part.starts_with('/')
                    && !path_part.starts_with("//")
                {
                    Some(path_part.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        });

    let location = match safe_referer_path {
        Some(ref p) => {
            let (base, query_opt) = p.split_once('?').map(|(b, q)| (b, Some(q))).unwrap_or((p, None));
            if base == "/" || base == "/en" {
                if norm == "en" { "/en".to_string() } else { "/".to_string() }
            } else if base == "/search" || base == "/en/search" || base == "/arama" {
                let target_base = if norm == "en" { "/en/search" } else { "/arama" };
                match query_opt {
                    Some(q) if !q.is_empty() => format!("{}?{}", target_base, q),
                    _ => target_base.to_string(),
                }
            } else {
                let mut pairs: Vec<String> = query_opt
                    .unwrap_or("")
                    .split('&')
                    .filter(|kv| !kv.is_empty() && !kv.starts_with("lang="))
                    .map(str::to_string)
                    .collect();
                pairs.push(format!("lang={}", norm));
                format!("{}?{}", base, pairs.join("&"))
            }
        }
        None => {
            if norm == "en" { "/en".to_string() } else { "/".to_string() }
        }
    };

    let secure_flag = if headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
    {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "msi_lang={}; Path=/; Max-Age=31536000; SameSite=Lax{}",
        norm, secure_flag
    );

    (
        StatusCode::FOUND,
        [(header::LOCATION, location), (header::SET_COOKIE, cookie)],
    )
        .into_response()
}

pub async fn redirect_legacy_haber_tr(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Response {
    for old in [format!("/haberler/{}", slug), format!("/haber/{}", slug)] {
        if let Some(rule) = state.db.find_seo_redirect(&old) {
            let target = rule.new_path.trim();
            if crate::db::Database::is_safe_seo_redirect_target(target) {
                return Redirect::permanent(target).into_response();
            }
        }
    }
    Redirect::permanent(&crate::engine::url_builder::article_url(&slug)).into_response()
}

pub async fn redirect_legacy_haber_en(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Response {
    let old = crate::engine::url_builder::legacy_haber_en_url(&slug);
    if let Some(rule) = state.db.find_seo_redirect(&old) {
        let target = rule.new_path.trim();
        if crate::db::Database::is_safe_seo_redirect_target(target) {
            return Redirect::permanent(target).into_response();
        }
    }
    Redirect::permanent(&crate::engine::url_builder::article_url_lang(&slug, "en")).into_response()
}

pub async fn redirect_legacy_kategori_tr(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Response {
    for old in [
        format!("/kategoriler/{}", slug),
        crate::engine::url_builder::category_url(&slug),
    ] {
        if let Some(rule) = state.db.find_seo_redirect(&old) {
            let target = rule.new_path.trim();
            if crate::db::Database::is_safe_seo_redirect_target(target) {
                return Redirect::permanent(target).into_response();
            }
        }
    }
    Redirect::permanent(&crate::engine::url_builder::category_url(&slug)).into_response()
}

pub async fn redirect_legacy_kategori_en(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Response {
    for old in [
        format!("/en/kategoriler/{}", slug),
        format!("/en/kategori/{}", slug),
    ] {
        if let Some(rule) = state.db.find_seo_redirect(&old) {
            let target = rule.new_path.trim();
            if crate::db::Database::is_safe_seo_redirect_target(target) {
                return Redirect::permanent(target).into_response();
            }
        }
    }
    Redirect::permanent(&crate::engine::url_builder::category_url_lang(&slug, "en")).into_response()
}

pub async fn render_legacy_html_page(
    State(state): State<AppState>,
    Path(filename): Path<String>,
    Query(_params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    // Check seo_redirects DB table first for any registered safe site-relative redirect
    let old_path = format!("/{}", filename);
    if !crate::config::is_retired_admin_segment(&old_path) {
        if let Some(rule) = state.db.find_seo_redirect(&old_path) {
            let target = rule.new_path.trim();
            if crate::db::Database::is_safe_seo_redirect_target(target) {
                return Redirect::permanent(target).into_response();
            }
        }
    }

    // Fallback hardcoded 301 redirects for known legacy .html URLs
    match filename.as_str() {
        "detail-1.html" => {
            Redirect::permanent("/kategori/haberler").into_response()
        }
        "detail-2.html" | "detail-3.html" => {
            Redirect::permanent("/kategori/haberler").into_response()
        }
        "index.html" => {
            Redirect::permanent("/").into_response()
        }
        "about.html" => {
            Redirect::permanent("/kurumsal/hakkimizda").into_response()
        }
        "contact.html" => {
            Redirect::permanent("/kurumsal/iletisim").into_response()
        }
        "gallery1.html" | "gallery.html" => {
            Redirect::permanent("/e-dergi").into_response()
        }
        "technology.html" | "lifestyle.html" | "travel.html" => {
            Redirect::permanent("/arama").into_response()
        }
        _ => render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await,
    }
}

pub async fn handle_fallback_or_redirect(
    State(state): State<AppState>,
    uri: axum::http::Uri,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let path = uri.path();

    // 0. Never resurrect retired /msi-komuta* admin paths even if legacy seo_redirects rows exist
    let decoded_path = urlencoding::decode(path).unwrap_or(std::borrow::Cow::Borrowed(path));
    if crate::config::is_retired_admin_segment(path)
        || crate::config::is_retired_admin_segment(&decoded_path)
    {
        return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
    }

    // 1. Check 301 SEO Redirect database table (only safe site-relative targets)
    if let Some(rule) = state.db.find_seo_redirect(path) {
        let target = rule.new_path.trim();
        if crate::db::Database::is_safe_seo_redirect_target(target) {
            return Redirect::permanent(target).into_response();
        }
    }

    // 2. Check if path matches Dynamic IndexNow verification file (`/{key}.txt`) or a legacy `.html` URL
    let trimmed = path.trim_start_matches('/').trim_end_matches('/');
    let indexnow_key = crate::services::indexnow_service::resolve_indexnow_key(&state.config);
    if trimmed == format!("{}.txt", indexnow_key) {
        return typed_response("text/plain; charset=utf-8", indexnow_key);
    }
    if trimmed.ends_with(".html") {
        return render_legacy_html_page(
            State(state),
            Path(trimmed.to_string()),
            Query(params),
            headers,
            Extension(csp_nonce.clone()),
        )
        .await;
    }

    // 2b. Direct corporate & legal page slugs
    if matches!(
        trimmed,
        "kunye"
            | "hakkimizda"
            | "biz-kimiz"
            | "yolculugumuz"
            | "kvkk"
            | "kvkk-basvuru-formu"
            | "gizlilik-politikasi"
            | "cerez-politikasi"
            | "kullanim-kosullari"
            | "mesafeli-satis-sozlesmesi"
            | "iptal-ve-iade-kosullari"
            | "ticari-ileti-aydinlatma"
            | "telif-ve-etik"
            | "duzeltme-ve-cevap-hakki"
            | "reklam-ve-sponsorluk"
            | "iletisim"
            | "cozum-ortaklarimiz"
    ) {
        return render_corporate_page(
            State(state),
            Path(trimmed.to_string()),
            Query(params),
            headers,
            Extension(csp_nonce.clone()),
        )
        .await;
    }

    // 2c. English corporate shortcuts (`/en/hakkimizda`, `/en/kunye`, `/en/kvkk`, etc.)
    if let Some(en_sub) = trimmed.strip_prefix("en/") {
        let mut en_params = params.clone();
        en_params.insert("lang".to_string(), "en".to_string());
        if matches!(
            en_sub,
            "kunye"
                | "imprint"
                | "hakkimizda"
                | "about"
                | "about-us"
                | "biz-kimiz"
                | "who-we-are"
                | "yolculugumuz"
                | "our-journey"
                | "kvkk"
                | "privacy-policy"
                | "iletisim"
                | "contact"
                | "cozum-ortaklarimiz"
                | "solution-partners"
                | "reklam-ve-sponsorluk"
                | "advertising"
        ) {
            return render_corporate_page(
                State(state),
                Path(en_sub.to_string()),
                Query(en_params),
                headers,
                Extension(csp_nonce.clone()),
            )
            .await;
        }
    }

    // 2d. Legacy msidergisi.com magazine viewer routes (`/magazines`, `/magazines/*`, `/en/magazines/*`)
    if trimmed == "magazines" || trimmed.starts_with("magazines/") {
        return Redirect::permanent("/e-dergi").into_response();
    }
    if trimmed == "en/magazines" || trimmed.starts_with("en/magazines/") {
        return Redirect::permanent("/e-dergi?lang=en").into_response();
    }

    // 3. Check if path matches a category slug directly (e.g. /haberler, /ozel-dosyalar, /soylesiler)
    if state.db.get_category_by_slug(trimmed).is_some() {
        return render_category_page(
            State(state),
            Path(trimmed.to_string()),
            Query(params),
            headers,
            Extension(csp_nonce.clone()),
        )
        .await;
    }

    // 4. Check if path matches an article slug directly
    if state.db.get_content_by_slug(trimmed).is_some() {
        return render_article_detail(
            State(state),
            Path(trimmed.to_string()),
            Query(params),
            headers,
            Extension(csp_nonce.clone()),
        )
        .await;
    }

    render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await
}

