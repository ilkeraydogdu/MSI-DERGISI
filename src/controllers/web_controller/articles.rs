use super::*;
use crate::engine::design::labels_helper::{is_en_content_ready, lang_href, tr_text};

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
                let target = rule.new_path.trim();
                if crate::db::Database::is_safe_seo_redirect_target(target) {
                    return Redirect::permanent(target).into_response();
                }
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
    let inst_allowed =
        content_service::user_matches_target_institutions(&item, user.as_ref(), &state.db);
    let purchased = user
        .as_ref()
        .map(|u| state.db.get_user_purchased_codes(&u.email))
        .unwrap_or_default();
    let (mut body, mut locked, mut reason) =
        content_service::apply_paywall_view(&item, user.as_ref(), &purchased, &lang);
    if !inst_allowed {
        locked = true;
        let raw = if lang.eq_ignore_ascii_case("en") && !item.body_en.trim().is_empty() {
            &item.body_en
        } else {
            &item.body_tr
        };
        body = content_service::truncate_to_two_sentences_html(raw);
        let inst_list = item.target_institutions.join(", ");
        reason = if lang.eq_ignore_ascii_case("en") {
            format!(
                "Institution-Exclusive Dossier: This briefing is restricted to verified members of: {}.",
                inst_list
            )
        } else {
            format!(
                "Kuruma Özel İçerik: Bu dosya yalnızca yetkili kurum personeline ({}) açıktır.",
                inst_list
            )
        };
    }
    // View tracking is now handled exclusively via analytics_events table.
    // Removed increment_content_view to prevent seed/harvested counters from inflating.
    let contents = state.db.get_all_contents();
    let mut toc_html = String::new();
    if !item.is_sponsored
        && !item.content_type.eq_ignore_ascii_case("advertorial")
        && !item.content_type.eq_ignore_ascii_case("advertorials")
    {
        if let Some(iss_no) = item.issue_number {
            let pub_type = if item.source_group.eq_ignore_ascii_case("msi-tdr") {
                "msi_tdr"
            } else {
                "msi"
            };
            if let Some(iss) = state.db.get_issue(pub_type, iss_no) {
                let toc_entries = crate::engine::design::components::sections::dossier_toc_from_spreads(
                    &iss.spreads,
                    iss.issue_number,
                    &lang,
                );
                if !toc_entries.is_empty() {
                    toc_html = crate::engine::design::components::sections::render_dossier_toc(
                        "Dosya İçindekiler",
                        "Dossier Table of Contents",
                        &toc_entries,
                        &lang,
                        Some(&state.translations),
                    );
                }
            }
        }
    }
    let related: Vec<ContentItem> = content_service::select_related_articles_authorized(
        &item,
        &contents,
        &state.db.get_recommendation_rules(),
        user.as_ref(),
        &state.db,
        12,
    )
    .into_iter()
    .filter(|c| {
        if lang.eq_ignore_ascii_case("en") {
            is_en_content_ready(c)
        } else {
            !c.source_group.eq_ignore_ascii_case("msi-tdr")
        }
    })
    .take(8)
    .collect();
    let ads = ad_service::select_ads_for_context(
        &state.db.get_all_ads(),
        Some(&item.content_type),
        Some(&item),
        user.as_ref(),
    );
    let html = crate::engine::design::reader::article_with_toc_and_translations(
        &item,
        &body,
        locked,
        &reason,
        &related,
        &ads,
        &lang,
        (!toc_html.is_empty()).then_some(toc_html.as_str()),
        Some(&state.translations),
    );
    let title = crate::engine::design::components::localized(&lang, &item.title_tr, &item.title_en);
    let canonical_path = crate::engine::url_builder::article_url(&item.slug);
    Html(home::render_shared_shell_with_request(
        &state,
        &lang,
        title,
        &html,
        Some(&canonical_path),
        Some(&item),
        false,
        Some(&csp_nonce.0),
        Some(&headers),
        Some(&item.content_type),
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
    let is_en = lang.eq_ignore_ascii_case("en");
    let tr = Some(&state.translations);
    let Some(category) = state.db.get_category_by_slug(&slug) else {
        return render_not_found_with_nonce(&state, &headers, Some(&csp_nonce.0)).await;
    };
    let user = authenticated_reader(&state, &headers);
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
    let page_items: Vec<_> = query_result
        .items
        .iter()
        .filter(|c| !is_en || is_en_content_ready(c))
        .collect();
    let total_items = if is_en {
        page_items.len().max(if query_result.total_pages <= 1 {
            page_items.len()
        } else {
            query_result.total_count
        })
    } else {
        query_result.total_count
    };
    let total_pages = query_result.total_pages;
    let current_page = query_result.current_page;

    // Get full snapshot for subcategory counts (paginated result is insufficient)
    let snap_for_counts = crate::engine::query_engine::CentralQueryEngine::snapshot(&state.db);
    let title = localized(&lang, &category.name_tr, &category.name_en);
    let sub_hub_html = if child_categories.is_empty() {
        String::new()
    } else {
        let sub_cards: String = child_categories
            .iter()
            .map(|sc| {
                let sc_count = snap_for_counts
                    .data
                    .contents
                    .iter()
                    .filter(|it| {
                        content_service::is_publicly_visible(it)
                            && (!is_en || is_en_content_ready(it))
                            && (is_en || !it.source_group.eq_ignore_ascii_case("msi-tdr"))
                            && CentralSearchEngine::item_matches_category(it, &sc.slug)
                    })
                    .count();
                let raw_sc_title = localized(&lang, &sc.name_tr, &sc.name_en);
                let sc_title = if sc.slug == "ni" && raw_sc_title.trim().eq_ignore_ascii_case("NI") {
                    if is_en {
                        esc("NI Test & Measurement Systems Special Dossier")
                    } else {
                        esc("NI Test ve Ölçüm Sistemleri Özel Dosyası")
                    }
                } else {
                    esc(raw_sc_title)
                };
                let sc_desc = esc(localized(&lang, &sc.description_tr, &sc.description_en));
                let badge_kicker = tr_text(
                    tr,
                    &lang,
                    "category.special_dossier",
                    "ÖZEL DOSYA",
                    "SPECIAL DOSSIER",
                );
                let open_lbl = tr_text(
                    tr,
                    &lang,
                    "category.open_dossier",
                    "Dosyayı incele",
                    "Explore dossier",
                );
                let cnt_lbl = tr_text(tr, &lang, "category.articles_suffix", "içerik", "articles");
                let sc_href = lang_href(&format!("/kategori/{}", sc.slug), &lang);
                format!(
                    r#"<a class="subdossier-card" href="{href}"><div class="subdossier-badge"><span class="subdossier-kicker">{kicker}</span><span class="subdossier-count">{cnt} {cnt_lbl}</span></div><div class="subdossier-copy"><h3>{title}</h3><p>{desc}</p><span>{open_lbl} →</span></div></a>"#,
                    href = esc(&sc_href),
                    kicker = esc(&badge_kicker),
                    title = sc_title,
                    desc = sc_desc,
                    cnt = sc_count,
                    cnt_lbl = esc(&cnt_lbl),
                    open_lbl = esc(&open_lbl)
                )
            })
            .collect();
        let hub_lbl = tr_text(
            tr,
            &lang,
            "category.sub_hub_label",
            "FUAR, KONFERANS VE ETKİNLİK ÖZEL DOSYALARI",
            "EXHIBITION & CONFERENCE SPECIAL DOSSIERS",
        );
        format!(
            r#"<div class="subcategories-hub"><div class="eyebrow">{hub_lbl}</div><div class="subcategories-grid">{sub_cards}</div></div>"#,
            hub_lbl = esc(&hub_lbl),
            sub_cards = sub_cards
        )
    };

    let cards = if page_items.is_empty() {
        let empty_msg = tr_text(
            tr,
            &lang,
            "category.empty",
            "Bu kategoride henüz içerik bulunmuyor.",
            "No content in this category yet.",
        );
        let explore_all = tr_text(
            tr,
            &lang,
            "category.explore_all",
            "Tüm içerikleri keşfedin →",
            "Explore all content →",
        );
        let search_href = lang_href("/arama", &lang);
        format!(
            "<div class=\"empty-state\">{} <a href=\"{}\">{}</a></div>",
            esc(&empty_msg),
            esc(&search_href),
            esc(&explore_all)
        )
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
        let lang_param = if is_en { "&lang=en" } else { "" };
        let prev_lbl = tr_text(tr, &lang, "category.prev_page", "← Önceki", "← Previous");
        let next_lbl = tr_text(tr, &lang, "category.next_page", "Sonraki →", "Next →");
        let pag_aria = tr_text(
            tr,
            &lang,
            "category.pagination_label",
            "Kategori sayfaları",
            "Category pages",
        );
        let mut links = String::new();
        if current_page > 1 {
            links.push_str(&format!(
                "<a class=\"button secondary\" rel=\"prev\" href=\"/kategori/{}?page={}{}{}\">{}</a>",
                esc(&slug),
                current_page - 1,
                ssb_q,
                lang_param,
                esc(&prev_lbl)
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
                "<a{} href=\"/kategori/{}?page={}{}{}\">{}</a>",
                active,
                esc(&slug),
                p_no,
                ssb_q,
                lang_param,
                p_no
            ));
        }
        if current_page < total_pages {
            links.push_str(&format!(
                "<a class=\"button secondary\" rel=\"next\" href=\"/kategori/{}?page={}{}{}\">{}</a>",
                esc(&slug),
                current_page + 1,
                ssb_q,
                lang_param,
                esc(&next_lbl)
            ));
        }
        format!(
            "<nav class=\"pagination-nav\" aria-label=\"{}\">{}</nav>",
            esc(&pag_aria),
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
    let mut sidebar_related: Vec<_> = snap_for_counts
        .data
        .contents
        .iter()
        .filter(|c| {
            content_service::is_publicly_visible(c)
                && c.content_type != "yeni-sayi-kapak"
                && (!is_en || is_en_content_ready(c))
                && (is_en || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
                && !page_items
                    .iter()
                    .take(crate::db::constants::RELATED_CONTENT_LIMIT)
                    .any(|x| x.id == c.id)
        })
        .cloned()
        .collect();
    sidebar_related.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));
    sidebar_related.truncate(crate::db::constants::SIDEBAR_RELATED_CONTENT_LIMIT);
    let all_ads = state.db.get_all_ads();
    let context_ads = ad_service::select_ads_for_context(
        &all_ads,
        Some(&slug),
        None,
        user.as_ref(),
    );
    let sidebar = editorial_sidebar(&slug, &sidebar_related, &context_ads, &lang);

    let topic_lbl = tr_text(tr, &lang, "category.topic_filter_label", "İlgi alanı", "Interest area");
    let all_topics_lbl = tr_text(tr, &lang, "category.all_topics", "Tüm konular", "All topics");
    let filter_btn_lbl = tr_text(tr, &lang, "category.filter_button", "Filtrele", "Filter");
    let reset_lbl = tr_text(tr, &lang, "category.reset_filter", "Sıfırla", "Reset");
    let count_suffix = tr_text(tr, &lang, "category.articles_suffix", "içerik", "items");
    let reset_href = lang_href(&format!("/kategori/{}", slug), &lang);
    let lang_hidden = if is_en {
        "<input type=\"hidden\" name=\"lang\" value=\"en\">"
    } else {
        ""
    };

    let page_items_owned: Vec<ContentItem> = page_items.iter().map(|i| (*i).clone()).collect();
    let dossier_toc_html = if matches!(
        slug.as_str(),
        "ozel-dosya"
            | "ozel-dosyalar"
            | "kapak-dosyasi"
            | "kapak-dosyalari"
            | "etkinlik-ozel-sayilari"
            | "etkinlik-sonrasi-ozel-dosyalar"
            | "savunma-sanayii-tarihi"
    ) && !page_items_owned.is_empty()
    {
        let entries = crate::engine::design::components::sections::dossier_toc_from_contents(
            &page_items_owned,
            &lang,
        );
        crate::engine::design::components::sections::render_dossier_toc(
            "Dosya İçindekiler",
            "Dossier Table of Contents",
            &entries,
            &lang,
            tr,
        )
    } else {
        String::new()
    };

    let related_strip_html = crate::engine::design::components::sections::render_related_strip(
        &snap_for_counts.data.contents,
        &crate::engine::design::components::sections::RelatedStripOptions {
            lang: &lang,
            is_current_sponsored: false,
            exclude_ids: &[],
            limit: 4,
            tr,
            source_context: &slug,
        },
    );

    let body = format!(
        r#"<section class="container section-space category-page"><div class="category-master-grid"><div class="category-main-col"><div class="msi-section-ribbon">{ribbon}</div><div class="page-heading"><h1>{title}</h1><p>{desc}</p></div>{sub_hub}{dossier_toc}<form class="filter-toolbar" method="get">{lang_hidden}<label for="category-topic">{topic_lbl}</label><select id="category-topic" name="ssb"><option value="">{all_topics_lbl}</option>{filters}</select><button class="button secondary">{filter_btn_lbl}</button><a href="{reset_href}" class="text-link">{reset_lbl}</a><span>{total} {count_suffix}</span></form>{cards}{pagination}</div>{sidebar}</div></section>{related_strip}"#,
        ribbon = esc(title),
        title = esc(title),
        desc = esc(localized(
            &lang,
            &category.description_tr,
            &category.description_en
        )),
        sub_hub = sub_hub_html,
        dossier_toc = dossier_toc_html,
        lang_hidden = lang_hidden,
        topic_lbl = esc(&topic_lbl),
        all_topics_lbl = esc(&all_topics_lbl),
        filters = filters,
        filter_btn_lbl = esc(&filter_btn_lbl),
        reset_href = esc(&reset_href),
        reset_lbl = esc(&reset_lbl),
        total = total_items,
        count_suffix = esc(&count_suffix),
        cards = cards,
        pagination = pagination_html,
        sidebar = sidebar,
        related_strip = related_strip_html
    );

    let settings_map = state.db.get_setting_map(&lang);
    let brand_config = seo_router_engine::SeoBrandConfig::from_settings(&settings_map);
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

    Html(home::render_shell_with_context(
        &state,
        &lang,
        "magazine",
        &seo,
        &body,
        Some(&csp_nonce.0),
        Some(&headers),
        None,
        Some(&slug),
    ))
    .into_response()
}
