use super::*;

pub async fn render_article_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = detect_language(&params, &headers);
    let Some(item) = state.db.get_content_by_slug(&slug) else {
        for cand in [
            format!("/icerik/{}", slug),
            format!("/haberler/{}", slug),
            format!("/en/haberler/{}", slug),
        ] {
            if let Some(rule) = state.db.find_seo_redirect(&cand) {
                return Redirect::permanent(&rule.new_path).into_response();
            }
        }
        return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
    };
    let user = authenticated_reader(&state, &headers);
    let is_staff = user
        .as_ref()
        .map(|u| crate::extractors::operator_user::is_staff_role(&state.db, &u.role))
        .unwrap_or(false);
    if !content_service::is_publicly_visible(&item) && !is_staff {
        return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
    }
    let purchased = user
        .as_ref()
        .map(|u| state.db.get_user_purchased_codes(&u.email))
        .unwrap_or_default();
    let (body, locked, reason) =
        content_service::apply_paywall_view(&item, user.as_ref(), &purchased, &lang);
    state.db.increment_content_view(&slug, !locked);
    let contents = state.db.get_all_contents();
    let related = content_service::select_related_articles(
        &item,
        &contents,
        &state.db.get_recommendation_rules(),
        user.as_ref(),
        5,
    );
    let ads = ad_service::select_ads_for_context(
        &state.db.get_all_ads(),
        Some(&item.content_type),
        Some(&item),
        user.as_ref(),
    );
    let html = crate::engine::design::reader::article(
        &item, &body, locked, &reason, &related, &ads, &lang,
    );
    let title = crate::engine::design::components::localized(&lang, &item.title_tr, &item.title_en);
    let canonical_path = crate::engine::url_builder::article_url(&item.slug);
    Html(render_shared_shell(
        &state,
        &lang,
        title,
        &html,
        Some(&canonical_path),
        Some(&item),
        false,
        Some(&csp_nonce.0),
    ))
    .into_response()
}

pub async fn render_category_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    use crate::engine::{
        content_query_engine::CentralContentQueryEngine,
        design::components::{article_card, editorial_sidebar, localized},
        search_engine::CentralSearchEngine,
        security_engine::escape_html as esc,
    };
    let lang = detect_language(&params, &headers);
    let Some(category) = state.db.get_category_by_slug(&slug) else {
        return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
    };
    let ssb_filter = params.get("ssb").filter(|v| !v.is_empty()).cloned();
    let all_categories = state.db.get_all_categories();
    let child_parent = if slug == "ozel-dosyalar" {
        "ozel-dosya"
    } else {
        slug.as_str()
    };
    let child_categories: Vec<_> = all_categories
        .iter()
        .filter(|c| c.parent_slug.as_deref() == Some(child_parent) && c.group_type != "kurumsal")
        .collect();

    // Unified query via CentralContentQueryEngine — replaces scattered filter/pagination logic.
    let mut criteria = CentralContentQueryEngine::from_params(&params);
    criteria.category = Some(slug.clone());
    criteria.lang = Some(lang.clone());
    criteria.per_page = Some(crate::db::constants::DEFAULT_PAGE_SIZE);
    let query_result = CentralContentQueryEngine::execute(&state.db, &criteria);
    let total_items = query_result.total_count;
    let total_pages = query_result.total_pages;
    let current_page = query_result.current_page;
    let page_items: Vec<_> = query_result.items.iter().collect();

    // Get full snapshot for subcategory counts (paginated result is insufficient)
    let snap_for_counts = crate::engine::query_engine::CentralQueryEngine::snapshot(&state.db);
    let title = localized(&lang, &category.name_tr, &category.name_en);
    let sub_hub_html = if child_categories.is_empty() {
        String::new()
    } else {
        let sub_cards: String = child_categories
            .iter()
            .map(|sc| {
                let sc_count = snap_for_counts.data.contents
                    .iter()
                    .filter(|it| CentralSearchEngine::item_matches_category(it, &sc.slug))
                    .count();
                let raw_sc_title = localized(&lang, &sc.name_tr, &sc.name_en);
                let sc_title = if sc.slug == "ni" && raw_sc_title.trim().eq_ignore_ascii_case("NI") {
                    if lang == "en" {
                        esc("NI Test & Measurement Systems Special Dossier")
                    } else {
                        esc("NI Test ve Ölçüm Sistemleri Özel Dosyası")
                    }
                } else {
                    esc(raw_sc_title)
                };
                let sc_desc = esc(localized(&lang, &sc.description_tr, &sc.description_en));
                let badge_kicker = if lang == "en" {
                    "SPECIAL DOSSIER"
                } else {
                    "ÖZEL DOSYA"
                };
                let open_lbl = if lang == "en" {
                    "Explore dossier"
                } else {
                    "Dosyayı incele"
                };
                format!(
                    r#"<a class="subdossier-card" href="/kategori/{slug}"><div class="subdossier-badge"><span class="subdossier-kicker">{kicker}</span><span class="subdossier-count">{cnt} {cnt_lbl}</span></div><div class="subdossier-copy"><h3>{title}</h3><p>{desc}</p><span>{open_lbl} →</span></div></a>"#,
                    slug = esc(&sc.slug),
                    kicker = badge_kicker,
                    title = sc_title,
                    desc = sc_desc,
                    cnt = sc_count,
                    cnt_lbl = if lang == "en" { "articles" } else { "içerik" },
                    open_lbl = open_lbl
                )
            })
            .collect();
        let hub_lbl = if lang == "en" {
            "EXHIBITION & CONFERENCE SPECIAL DOSSIERS"
        } else {
            "FUAR, KONFERANS VE ETKİNLİK ÖZEL DOSYALARI"
        };
        format!(
            r#"<div class="subcategories-hub"><div class="eyebrow">{hub_lbl}</div><div class="subcategories-grid">{sub_cards}</div></div>"#
        )
    };

    let cards = if page_items.is_empty() {
        "<div class=\"empty-state\">Bu kategoride henüz içerik bulunmuyor. <a href=\"/arama\">Tüm içerikleri keşfedin →</a></div>".into()
    } else {
        format!(
            "<div class=\"stories category-stories layout-grid\">{}</div>",
            page_items
                .iter()
                .enumerate()
                .map(|(idx, c)| article_card(c, &lang, idx < 3))
                .collect::<String>()
        )
    };

    let pagination_html = if total_pages > 1 {
        let ssb_q = ssb_filter
            .as_ref()
            .map(|s| format!("&ssb={}", urlencoding::encode(s)))
            .unwrap_or_default();
        let mut links = String::new();
        if current_page > 1 {
            links.push_str(&format!(
                "<a class=\"button secondary\" rel=\"prev\" href=\"/kategori/{}?page={}{}\">← Önceki</a>",
                esc(&slug),
                current_page - 1,
                ssb_q
            ));
        }
        let win_start = current_page.saturating_sub(2).max(1);
        let win_end = (win_start + 4).min(total_pages);
        for p_no in win_start..=win_end {
            let active = if p_no == current_page {
                " aria-current=\"page\" class=\"button primary\""
            } else {
                " class=\"button secondary\""
            };
            links.push_str(&format!(
                "<a{} href=\"/kategori/{}?page={}{}\">{}</a>",
                active,
                esc(&slug),
                p_no,
                ssb_q,
                p_no
            ));
        }
        if current_page < total_pages {
            links.push_str(&format!(
                "<a class=\"button secondary\" rel=\"next\" href=\"/kategori/{}?page={}{}\">Sonraki →</a>",
                esc(&slug),
                current_page + 1,
                ssb_q
            ));
        }
        format!(
            "<nav class=\"pagination-nav\" aria-label=\"Kategori sayfaları\">{}</nav>",
            links
        )
    } else {
        String::new()
    };

    let tags = state.db.get_all_ssb_tags();
    let filters: String = tags
        .iter()
        .map(|t| {
            format!(
                "<option value=\"{}\" {}>{}</option>",
                esc(&t.code),
                if ssb_filter.as_ref() == Some(&t.code) {
                    "selected"
                } else {
                    ""
                },
                esc(localized(&lang, &t.name_tr, &t.name_en))
            )
        })
        .collect();
    let mut sidebar_related: Vec<_> = snap_for_counts.data.contents
        .iter()
        .filter(|c| c.content_type != "yeni-sayi-kapak" && !page_items.iter().take(crate::db::constants::RELATED_CONTENT_LIMIT).any(|x| x.id == c.id))
        .cloned()
        .collect();
    sidebar_related.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));
    sidebar_related.truncate(crate::db::constants::SIDEBAR_RELATED_CONTENT_LIMIT);
    let all_ads = state.db.get_all_ads();
    let sidebar = editorial_sidebar(&slug, &sidebar_related, &all_ads, &lang);

    let body = format!(
        r#"<section class="container section-space category-page"><div class="category-master-grid"><div class="category-main-col"><div class="msi-section-ribbon">{ribbon}</div><div class="page-heading"><h1>{title}</h1><p>{desc}</p></div>{sub_hub}<form class="filter-toolbar" method="get"><label for="category-topic">İlgi alanı</label><select id="category-topic" name="ssb"><option value="">Tüm konular</option>{filters}</select><button class="button secondary">Filtrele</button><a href="/kategori/{slug}" class="text-link">Sıfırla</a><span>{total} içerik</span></form>{cards}{pagination}</div>{sidebar}</div></section>"#,
        ribbon = esc(title),
        title = esc(title),
        desc = esc(localized(
            &lang,
            &category.description_tr,
            &category.description_en
        )),
        sub_hub = sub_hub_html,
        filters = filters,
        slug = esc(&slug),
        total = total_items,
        cards = cards,
        pagination = pagination_html,
        sidebar = sidebar
    );

    let settings_map = state.db.get_setting_map(&lang);
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings_map);
    let page_items_owned: Vec<ContentItem> = page_items.iter().map(|i| (*i).clone()).collect();
    let mut seo = CentralSeoEngine::build_category_seo_meta(
        &state.config.public_url,
        &category,
        &page_items_owned,
        current_page,
        total_pages,
        &lang,
        &brand_config,
        &state.config.primary_domain,
        &state.config.news_domain,
    );
    if ssb_filter.is_some() {
        seo.robots = "noindex, follow".to_string();
    }

    Html(render_shell_with_seo(&state, &lang, "magazine", &seo, &body, Some(&csp_nonce.0))).into_response()
}

