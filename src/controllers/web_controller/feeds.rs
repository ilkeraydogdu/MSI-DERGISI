use super::*;

pub async fn render_sitemap_xml(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let mut xml = seo_router_engine::generate_sitemap_xml(
        &state.config.public_url,
        &snap.data.categories,
        &snap.data.contents,
        &snap.data.issues,
        &brand,
    );
    let design = crate::engine::design::config::load(&state.db.get_setting_map("tr"));
    let clean_base = CentralSeoEngine::normalize_base_url(&state.config.public_url);
    let custom: String = design
        .pages
        .iter()
        .filter(|p| {
            !["magazine", "news", "e-dergi", "abonelik-ve-satis"].contains(&p.slug.as_str())
        })
        .map(|p| {
            let path = crate::engine::url_builder::designed_page_url(&p.slug);
            format!(
                "  <url><loc>{}{}</loc><changefreq>weekly</changefreq><priority>0.75</priority></url>\n",
                crate::engine::security_engine::escape_html(&clean_base),
                path
            )
        })
        .collect();
    xml = xml.replace("</urlset>", &format!("{}</urlset>", custom));
    typed_response("application/xml; charset=utf-8", xml)
}

pub async fn render_news_sitemap_xml(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let xml =
        CentralSeoEngine::generate_news_sitemap_xml(&state.config.public_url, &snap.data.contents, &brand);
    typed_response("application/xml; charset=utf-8", xml)
}

pub async fn render_image_sitemap_xml(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let xml = CentralSeoEngine::generate_image_sitemap_xml(
        &state.config.public_url,
        &snap.data.contents,
        &snap.data.issues,
        &brand,
    );
    typed_response("application/xml; charset=utf-8", xml)
}

pub async fn render_robots_txt(State(state): State<AppState>) -> Response {
    let txt = seo_router_engine::generate_robots_txt(&state.config.public_url);
    typed_response("text/plain; charset=utf-8", txt)
}

pub async fn render_rss_xml(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let lang = detect_language(&params, &headers);
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map(&lang));
    let xml =
        seo_router_engine::generate_rss_xml(&state.config.public_url, &snap.data.contents, &lang, &brand);
    typed_response("application/rss+xml; charset=utf-8", xml)
}

pub async fn render_json_feed(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let lang = detect_language(&params, &headers);
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map(&lang));
    let body = CentralSeoEngine::generate_json_feed(
        &state.config.public_url,
        &snap.data.contents,
        &lang,
        &brand,
    );
    typed_response("application/feed+json; charset=utf-8", body)
}

pub async fn render_llms_txt(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let txt = CentralSeoEngine::generate_llms_txt(
        &state.config.public_url,
        &snap.data.categories,
        &snap.data.ssb_tags,
        &snap.data.issues,
        &snap.data.contents,
        false,
        &brand,
    );
    typed_response("text/plain; charset=utf-8", txt)
}

pub async fn render_llms_full_txt(State(state): State<AppState>) -> Response {
    let snap = CentralQueryEngine::snapshot(&state.db);
    let brand = crate::engine::seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let txt = CentralSeoEngine::generate_llms_txt(
        &state.config.public_url,
        &snap.data.categories,
        &snap.data.ssb_tags,
        &snap.data.issues,
        &snap.data.contents,
        true,
        &brand,
    );
    typed_response("text/plain; charset=utf-8", txt)
}

pub async fn render_ai_plugin_json(State(state): State<AppState>) -> Response {
    let base = state.config.public_url.trim_end_matches('/');
    let plugin = serde_json::json!({
        "schema_version": "v1",
        "name_for_human": "MSI Dergisi",
        "name_for_model": "msi_dergisi",
        "description_for_human": "Türkiye'nin savunma ve havacılık sanayii dergisi MSI Dergisi'nin güncel haberleri, analizleri ve dergi içerikleri.",
        "description_for_model": "MSI Dergisi is Turkey's leading defense and aerospace industry magazine. Use this plugin to search for latest defense news, military technology analyses, interviews with industry leaders, and magazine content about Turkish and global defense sector developments.",
        "auth": { "type": "none" },
        "api": {
            "type": "openapi",
            "url": format!("{}/api/v1/openapi.json", base),
            "is_user_authenticated": false
        },
        "logo_url": format!("{}/images/icon-512.png", base),
        "contact_email": "info@savunmahaber.com",
        "legal_info_url": format!("{}/kurumsal/kullanim-kosullari", base)
    });
    typed_response(
        "application/json; charset=utf-8",
        serde_json::to_string_pretty(&plugin).unwrap_or_default(),
    )
}

pub async fn render_openapi_json(State(state): State<AppState>) -> Response {
    let base = state.config.public_url.trim_end_matches('/');
    let spec = serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "MSI Dergisi & MSI TDR Public Defense Intelligence API",
            "version": "2.6.0",
            "description": "Official OpenAPI 3.1.0 specification for MSI Dergisi & savunmahaber.com defense, aerospace, and naval technology content, faceted search, SSB taxonomy, and magazine archive."
        },
        "servers": [
            { "url": base, "description": "Primary Production Server" }
        ],
        "paths": {
            "/api/v1/health": {
                "get": {
                    "operationId": "getPlatformHealth",
                    "summary": "Platform status and public entity counts",
                    "responses": { "200": { "description": "Healthy status response" } }
                }
            },
            "/api/v1/search": {
                "get": {
                    "operationId": "searchDefenseContent",
                    "summary": "Faceted full-text search across defense news, analyses, interviews, and magazine issues",
                    "parameters": [
                        { "name": "q", "in": "query", "schema": { "type": "string" } },
                        { "name": "category", "in": "query", "schema": { "type": "string" } },
                        { "name": "ssb_tag", "in": "query", "schema": { "type": "string" } },
                        { "name": "issue", "in": "query", "schema": { "type": "integer" } },
                        { "name": "lang", "in": "query", "schema": { "type": "string", "enum": ["tr", "en"] } },
                        { "name": "page", "in": "query", "schema": { "type": "integer" } },
                        { "name": "per_page", "in": "query", "schema": { "type": "integer" } }
                    ],
                    "responses": { "200": { "description": "Paginated search results" } }
                }
            },
            "/api/v1/contents": {
                "get": {
                    "operationId": "listPublishedContents",
                    "summary": "List published defense articles and dossiers",
                    "responses": { "200": { "description": "Published content list" } }
                }
            },
            "/api/v1/contents/{slug}": {
                "get": {
                    "operationId": "getContentBySlug",
                    "summary": "Retrieve a single defense article or dossier by slug",
                    "parameters": [
                        { "name": "slug", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": {
                        "200": { "description": "Article detail with paywall and related articles" },
                        "404": { "description": "Article not found" }
                    }
                }
            },
            "/api/v1/categories": {
                "get": {
                    "operationId": "listCategories",
                    "summary": "List editorial and dossier categories",
                    "responses": { "200": { "description": "Category hierarchy" } }
                }
            },
            "/api/v1/ssb-tags": {
                "get": {
                    "operationId": "listSsbTags",
                    "summary": "List Defense Industry Agency (SSB) technology taxonomy tags",
                    "responses": { "200": { "description": "SSB technology tags" } }
                }
            },
            "/api/v1/issues": {
                "get": {
                    "operationId": "listMagazineIssues",
                    "summary": "List MSI Dergisi & MSI TDR print/digital magazine issues",
                    "responses": { "200": { "description": "Magazine issue archive" } }
                }
            }
        }
    });
    typed_response(
        "application/json; charset=utf-8",
        serde_json::to_string_pretty(&spec).unwrap_or_default(),
    )
}

pub async fn render_opensearch_xml(State(state): State<AppState>) -> Response {
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let xml = CentralSeoEngine::generate_opensearch_xml(&state.config.public_url, &brand_config);
    typed_response("application/opensearchdescription+xml; charset=utf-8", xml)
}

pub async fn render_web_manifest(State(state): State<AppState>) -> Response {
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&state.db.get_setting_map("tr"));
    let json_body = CentralSeoEngine::generate_web_manifest(&state.config.public_url, &brand_config);
    typed_response("application/manifest+json; charset=utf-8", json_body)
}

pub async fn render_favicon_ico(State(state): State<AppState>) -> Response {
    let p = format!("{}/favicon.ico", state.config.static_dir.trim_end_matches('/'));
    if let Ok(bytes) = tokio::fs::read(&p).await {
        return typed_response("image/x-icon", bytes);
    }
    Redirect::permanent("/images/favicon.png").into_response()
}

pub async fn render_favicon_svg(State(state): State<AppState>) -> Response {
    let p = format!(
        "{}/images/favicon.svg",
        state.config.static_dir.trim_end_matches('/')
    );
    if let Ok(bytes) = tokio::fs::read(&p).await {
        return typed_response("image/svg+xml; charset=utf-8", bytes);
    }
    Redirect::permanent("/images/logo.svg").into_response()
}

pub async fn render_apple_touch_icon(State(state): State<AppState>) -> Response {
    let p = format!(
        "{}/images/apple-touch-icon.png",
        state.config.static_dir.trim_end_matches('/')
    );
    if let Ok(bytes) = tokio::fs::read(&p).await {
        return typed_response("image/png", bytes);
    }
    Redirect::permanent("/images/icon-192.png").into_response()
}

pub async fn render_indexnow_key(State(state): State<AppState>) -> Response {
    let key = crate::services::indexnow_service::resolve_indexnow_key(&state.config);
    typed_response("text/plain; charset=utf-8", key)
}

