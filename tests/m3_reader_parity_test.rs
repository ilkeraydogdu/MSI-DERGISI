mod common;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use msi_dergisi_platform::{
    controllers::AppState,
    engine::{
        design::{
            components::{issue_card, sections::render_section, PageData},
            config::{Section, SectionKind},
        },
        translation_engine::TranslationCache,
    },
    models::{AdCampaign, ContentItem, MagazineIssue, MagazinePageSpread, RecommendationRule},
};
use serde_json::{json, Value};
use std::collections::HashSet;
use tower::ServiceExt;

async fn send_get(
    app: &axum::Router,
    uri: &str,
    extra_headers: &[(&str, &str)],
) -> (StatusCode, String) {
    let mut builder = Request::builder().method("GET").uri(uri);
    for (k, v) in extra_headers {
        builder = builder.header(*k, *v);
    }
    let resp = app
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn make_article(seed: i64, slug: &str, body_tr: &str) -> ContentItem {
    let mut item = common::fixture_content(seed);
    item.id = 0;
    item.slug = slug.to_string();
    item.title_tr = format!("Özgün Savunma Makalesi {}", seed);
    item.title_en = format!("Original Defence Article {}", seed);
    item.summary_tr = format!("Özet metni {}", seed);
    item.summary_en = format!("Summary text {}", seed);
    item.body_tr = body_tr.to_string();
    item.body_en = "<p>English paragraph one.</p><p>English paragraph two.</p><p>English paragraph three.</p>".to_string();
    item.content_type = "haberler".to_string();
    item.source_group = "gunluk-haber".to_string();
    item.visibility_level = 0;
    item.is_sponsored = false;
    item.sponsor_brand = None;
    item.sponsor_legal_label = None;
    item.issue_number = None;
    item.published_at = "2026-10-15 10:00:00".to_string();
    item.status = "published".to_string();
    item.en_status = "approved".to_string();
    item.ssb_tags = vec!["hava-araclari".into(), "radarlar-ve-sensorler".into()];
    item
}

#[tokio::test]
async fn req012_mid_article_inline_related_deduplication_and_toc_preservation() {
    let (state, db_path) = common::build_test_state("m3_req012_inline");
    let app = common::build_test_router(&state);

    let iss = MagazineIssue {
        issue_number: 777,
        slug: "sayi-777".into(),
        title_tr: "Sayı 777".into(),
        title_en: "Issue 777".into(),
        publish_month_tr: "Kasım".into(),
        publish_month_en: "November".into(),
        year: 2026,
        cover_image: "images/msi/default.webp".into(),
        is_special_issue: false,
        event_name: None,
        page_count: 64,
        dossier_count: 1,
        summary_tr: "Test sayı".into(),
        summary_en: "Test issue".into(),
        pdf_path: String::new(),
        publication_type: "msi".into(),
        uploaded_pdf_file: None,
        spreads: vec![MagazinePageSpread {
            page_no: "S. 10-15".into(),
            category_badge: "KAPAK DOSYASI".into(),
            title_tr: "M3_TOC_ENTRY_777".into(),
            title_en: "M3_TOC_ENTRY_777_EN".into(),
            excerpt_tr: "Dosya özeti".into(),
            excerpt_en: "Dossier summary".into(),
            linked_content_slug: None,
            image_url: String::new(),
        }],
        ..Default::default()
    };
    state.db.upsert_issue(&iss).unwrap();

    let body_4p = concat!(
        "<p></p>",
        "<p>&nbsp;</p>",
        "<p>Birinci paragraf: Modern radar sistemleri çok katmanlı hava savunma mimarisi sunar.</p>",
        "<p>İkinci paragraf: Aktif faz dizinli antenler eş zamanlı hedef takibi yapar.</p>",
        "<p>Üçüncü paragraf: Yerli galyum nitrür modüller menzil performansını yükseltir.</p>",
        "<p>Dördüncü paragraf: Platform entegrasyonu test aşamasını tamamlamıştır.</p>"
    );
    let mut main_art = make_article(901, "m3-main-inline-test", body_4p);
    main_art.issue_number = Some(777);
    state.db.upsert_content(&main_art).unwrap();

    for idx in 902..=906 {
        let mut rel = make_article(
            idx,
            &format!("m3-rel-candidate-{}", idx),
            "<p>İlgili içerik metni bir.</p><p>İlgili içerik metni iki.</p>",
        );
        rel.ssb_tags = vec!["radarlar-ve-sensorler".into()];
        state.db.upsert_content(&rel).unwrap();
    }

    // Create high-priority rule promoting m3-rel-candidate-906
    let rule = RecommendationRule {
        id: 0,
        rule_name: "M3 Radar Rule".into(),
        source_tag_or_cat: "radarlar-ve-sensorler".into(),
        target_institution: "*".into(),
        recommended_content_slug: "m3-rel-candidate-906".into(),
        priority: 99,
        is_active: true,
    };
    state.db.insert_recommendation_rule(&rule).unwrap();

    let (status, html) = send_get(&app, "/icerik/m3-main-inline-test", &[]).await;
    assert_eq!(status, StatusCode::OK);

    // 1. Mid-body .msi-inline-related[data-related-inline] present after paragraph 2
    assert!(
        html.contains(r#"class="msi-inline-related" data-related-inline"#),
        "SSR must contain .msi-inline-related[data-related-inline]"
    );
    // 2. TOC nav & svg preserved outside .msi-article-body
    assert!(html.contains(r#"id="dossier-toc""#));
    assert!(html.contains("M3_TOC_ENTRY_777"));

    // Verify split position is strictly between paragraph 2 and paragraph 3 inside .msi-article-body
    let body_start = html.find("<article class=\"msi-article-body\"").unwrap();
    let body_end = html[body_start..].find("</article>").unwrap() + body_start;
    let article_body = &html[body_start..body_end];
    let p2_pos = article_body.find("İkinci paragraf:").unwrap();
    let inline_pos = article_body.find("data-related-inline").unwrap();
    let p3_pos = article_body.find("Üçüncü paragraf:").unwrap();
    assert!(
        p2_pos < inline_pos && inline_pos < p3_pos,
        "Inline related block must appear between paragraph 2 and paragraph 3"
    );

    // 3. Rule-promoted candidate appears in inline block, and does NOT duplicate in #related-strip
    let aside_end = article_body[inline_pos..].find("</aside>").unwrap() + inline_pos;
    let inline_slice = &article_body[inline_pos..aside_end];
    assert!(
        inline_slice.contains("/icerik/m3-rel-candidate-906"),
        "Rule-promoted recommendation should appear in inline block"
    );

    let strip_start = html.find(r#"id="related-strip" data-related-strip"#).unwrap();
    let strip_end = html[strip_start..].find("</section>").unwrap() + strip_start;
    let strip_slice = &html[strip_start..strip_end];
    assert!(
        !strip_slice.contains("/icerik/m3-rel-candidate-906"),
        "Bottom strip must not repeat the inline recommendation"
    );

    let mut bottom_hrefs: HashSet<String> = HashSet::new();
    let mut scan = strip_slice;
    while let Some(pos) = scan.find("href=\"/icerik/") {
        let rest = &scan[pos + "href=\"".len()..];
        if let Some(end) = rest.find('"') {
            bottom_hrefs.insert(rest[..end].to_string());
            scan = &rest[end + 1..];
        } else {
            break;
        }
    }
    assert!(
        bottom_hrefs.len() >= 3,
        "Expected >= 3 distinct recommendations in bottom strip, found {:?}",
        bottom_hrefs
    );
    assert!(!bottom_hrefs.contains("/icerik/m3-rel-candidate-906"));
    let mut distinct = HashSet::new();
    distinct.insert("/icerik/m3-rel-candidate-906".to_string());
    distinct.extend(bottom_hrefs);
    assert!(distinct.len() >= 4, "Total distinct recommendations must be >= 4 (1 inline + 3 bottom)");

    common::cleanup_test_db(&db_path);
}

#[tokio::test]
async fn req012_sponsored_and_advertorial_suppress_all_recommendations() {
    let (state, db_path) = common::build_test_state("m3_req012_spon");
    let app = common::build_test_router(&state);

    let body_4p = "<p>P1.</p><p>P2.</p><p>P3.</p><p>P4.</p>";

    let mut spon = make_article(910, "m3-sponsored-item", body_4p);
    spon.is_sponsored = true;
    spon.sponsor_brand = Some("Savunma A.Ş.".into());
    state.db.upsert_content(&spon).unwrap();

    let mut adv_singular = make_article(911, "m3-advertorial-singular", body_4p);
    adv_singular.content_type = "advertorial".into();
    state.db.upsert_content(&adv_singular).unwrap();

    let mut adv_plural = make_article(912, "m3-advertorial-plural", body_4p);
    adv_plural.content_type = "advertorials".into();
    state.db.upsert_content(&adv_plural).unwrap();

    let organic = make_article(913, "m3-organic-candidate", body_4p);
    state.db.upsert_content(&organic).unwrap();

    for slug in [
        "m3-sponsored-item",
        "m3-advertorial-singular",
        "m3-advertorial-plural",
    ] {
        let (st, html) = send_get(&app, &format!("/icerik/{}", slug), &[]).await;
        assert_eq!(st, StatusCode::OK);
        common::assert_html_not_contains(
            &html,
            &["data-related-inline", "data-related-strip", "sidebar-rel-card"],
        );
    }

    common::cleanup_test_db(&db_path);
}

#[tokio::test]
async fn req012_paywall_locked_and_short_article_skip_mid_body_keep_bottom_strip() {
    let (state, db_path) = common::build_test_state("m3_req012_locked_short");
    let app = common::build_test_router(&state);

    let mut gated = make_article(
        920,
        "m3-gated-article",
        "<p>Birinci cümle. İkinci cümle.</p><p>Üçüncü cümle.</p><p>Dördüncü cümle.</p>",
    );
    gated.visibility_level = 2;
    state.db.upsert_content(&gated).unwrap();

    let short = make_article(921, "m3-short-article", "<p>Yalnızca tek paragraf içerik.</p>");
    state.db.upsert_content(&short).unwrap();

    for idx in 922..=924 {
        let rel = make_article(idx, &format!("m3-rel-ok-{}", idx), "<p>A</p><p>B</p>");
        state.db.upsert_content(&rel).unwrap();
    }

    let (st_locked, html_locked) = send_get(&app, "/icerik/m3-gated-article", &[]).await;
    assert_eq!(st_locked, StatusCode::OK);
    assert!(html_locked.contains("paywall-panel"));
    assert!(!html_locked.contains("data-related-inline"));
    assert!(html_locked.contains("data-related-strip"));

    let (st_short, html_short) = send_get(&app, "/icerik/m3-short-article", &[]).await;
    assert_eq!(st_short, StatusCode::OK);
    assert!(!html_short.contains("data-related-inline"));
    assert!(html_short.contains("data-related-strip"));

    common::cleanup_test_db(&db_path);
}

#[tokio::test]
async fn req024_article_and_card_ssb_attributes_js_contract_and_analytics_track() {
    let (state, db_path) = common::build_test_state("m3_req024");
    let app = common::build_test_router(&state);

    let tagged = make_article(
        930,
        "m3-tagged-article",
        "<p>P1.</p><p>P2.</p><p>P3.</p>",
    );
    state.db.upsert_content(&tagged).unwrap();

    let mut untagged = make_article(
        931,
        "m3-untagged-article",
        "<p>P1.</p><p>P2.</p><p>P3.</p>",
    );
    untagged.ssb_tags = Vec::new();
    state.db.upsert_content(&untagged).unwrap();

    let (st, html) = send_get(&app, "/icerik/m3-tagged-article", &[]).await;
    assert_eq!(st, StatusCode::OK);
    common::assert_html_contains(
        &html,
        &[
            r#"data-msi-content-slug="m3-tagged-article""#,
            r#"data-category-slug="haberler""#,
            r#"data-ssb-tag="hava-araclari""#,
            r#"data-ssb-tags="hava-araclari,radarlar-ve-sensorler""#,
            "data-content-id=\"",
        ],
    );

    let (st_untagged, html_untagged) = send_get(&app, "/icerik/m3-untagged-article", &[]).await;
    assert_eq!(st_untagged, StatusCode::OK);
    let root_start = html_untagged.find(r#"<div class="article-page""#).unwrap();
    let root_end = html_untagged[root_start..].find('>').unwrap() + root_start;
    let root_tag = &html_untagged[root_start..=root_end];
    assert!(root_tag.contains(r#"data-msi-content-slug="m3-untagged-article""#));
    assert!(root_tag.contains("data-content-id=\""));
    assert!(root_tag.contains(r#"data-category-slug="haberler""#));
    assert!(
        !root_tag.contains("data-ssb-tag="),
        "Untagged article root must not emit data-ssb-tag"
    );

    // JS contract check on public/js/msi-ui.js
    let js_src = std::fs::read_to_string(
        "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/public/js/msi-ui.js",
    )
    .expect("public/js/msi-ui.js must exist");
    assert!(
        !js_src.contains("ssb_tag: null,"),
        "msi-ui.js must not hardcode ssb_tag: null,"
    );
    assert!(js_src.contains("resolveSsbTag"));
    assert!(js_src.contains("dataset.ssbTag"));

    // Verify /api/v1/analytics/track persists ssb_tag in SQLite
    let payload = json!({
        "id": 0,
        "session_id": "m3-sess-001",
        "user_id": null,
        "organization_category": "",
        "organization_name": "",
        "event_type": "article_read",
        "content_slug": "m3-tagged-article",
        "ad_id": null,
        "duration_sec": 35,
        "scroll_depth_pct": 85,
        "video_completed": false,
        "video_watch_pct": 0,
        "is_true_read": true,
        "ssb_tag": "radarlar-ve-sensorler",
        "created_at": ""
    });
    let track_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/analytics/track")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(track_resp.status(), StatusCode::OK);
    let track_bytes = to_bytes(track_resp.into_body(), 1024 * 1024).await.unwrap();
    let track_json: Value = serde_json::from_slice(&track_bytes).unwrap();
    let event_id = track_json["event_id"]
        .as_i64()
        .or_else(|| track_json["data"]["event_id"].as_i64())
        .expect("event_id returned");
    assert!(event_id > 0);

    let conn = state.db.open_conn().unwrap();
    let saved_tag: Option<String> = conn
        .query_row(
            "SELECT ssb_tag FROM analytics_events WHERE id = ?1",
            rusqlite::params![event_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(saved_tag.as_deref(), Some("radarlar-ve-sensorler"));

    common::cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m3_f1_scoped_issue_toc_in_articles_controller() {
    let (state, db_path) = common::build_test_state("m3_f1_scoped_toc");
    let app = common::build_test_router(&state);

    let msi_150 = MagazineIssue {
        issue_number: 150,
        slug: "msi-150".into(),
        title_tr: "MSI 150".into(),
        title_en: "MSI 150".into(),
        publish_month_tr: "Ocak".into(),
        publish_month_en: "January".into(),
        year: 2026,
        cover_image: "images/msi/default.webp".into(),
        is_special_issue: false,
        event_name: None,
        page_count: 80,
        dossier_count: 1,
        summary_tr: "MSI 150 özet".into(),
        summary_en: "MSI 150 summary".into(),
        pdf_path: String::new(),
        publication_type: "msi".into(),
        uploaded_pdf_file: None,
        spreads: vec![MagazinePageSpread {
            page_no: "S. 12".into(),
            category_badge: "KAPAK DOSYASI".into(),
            title_tr: "MSI_150_UNIQUE_SPREAD".into(),
            title_en: "MSI_150_UNIQUE_SPREAD".into(),
            excerpt_tr: String::new(),
            excerpt_en: String::new(),
            linked_content_slug: None,
            image_url: String::new(),
        }],
        ..Default::default()
    };
    let tdr_150 = MagazineIssue {
        issue_number: 150,
        slug: "tdr-150".into(),
        title_tr: "TDR 150".into(),
        title_en: "TDR 150".into(),
        publish_month_tr: "Şubat".into(),
        publish_month_en: "February".into(),
        year: 2026,
        cover_image: "images/msi/default.webp".into(),
        is_special_issue: false,
        event_name: None,
        page_count: 84,
        dossier_count: 1,
        summary_tr: "TDR 150 özet".into(),
        summary_en: "TDR 150 summary".into(),
        pdf_path: String::new(),
        publication_type: "msi_tdr".into(),
        uploaded_pdf_file: None,
        spreads: vec![MagazinePageSpread {
            page_no: "P. 22".into(),
            category_badge: "COVER FILE".into(),
            title_tr: "TDR_150_UNIQUE_SPREAD".into(),
            title_en: "TDR_150_UNIQUE_SPREAD".into(),
            excerpt_tr: String::new(),
            excerpt_en: String::new(),
            linked_content_slug: None,
            image_url: String::new(),
        }],
        ..Default::default()
    };
    state.db.upsert_issue(&msi_150).unwrap();
    state.db.upsert_issue(&tdr_150).unwrap();

    let mut msi_art = make_article(940, "m3-f1-msi-art", "<p>P1</p><p>P2</p><p>P3</p>");
    msi_art.source_group = "dergi-icerigi".into();
    msi_art.issue_number = Some(150);
    state.db.upsert_content(&msi_art).unwrap();

    let mut tdr_art = make_article(941, "m3-f1-tdr-art", "<p>P1</p><p>P2</p><p>P3</p>");
    tdr_art.source_group = "msi-tdr".into();
    tdr_art.issue_number = Some(150);
    state.db.upsert_content(&tdr_art).unwrap();

    let (st_tdr, html_tdr) = send_get(&app, "/icerik/m3-f1-tdr-art?lang=en", &[]).await;
    assert_eq!(st_tdr, StatusCode::OK);
    let tdr_page_end = html_tdr.find("</main>").unwrap_or(html_tdr.len());
    let tdr_main = &html_tdr[..tdr_page_end];
    assert!(tdr_main.contains("TDR_150_UNIQUE_SPREAD"));
    assert!(!tdr_main.contains("MSI_150_UNIQUE_SPREAD"));

    let (st_msi, html_msi) = send_get(&app, "/icerik/m3-f1-msi-art", &[]).await;
    assert_eq!(st_msi, StatusCode::OK);
    let msi_page_end = html_msi.find("</main>").unwrap_or(html_msi.len());
    let msi_main = &html_msi[..msi_page_end];
    assert!(msi_main.contains("MSI_150_UNIQUE_SPREAD"));
    assert!(!msi_main.contains("TDR_150_UNIQUE_SPREAD"));

    common::cleanup_test_db(&db_path);
}

#[test]
fn m3_f2_issue_card_and_hero_emit_data_publication_type() {
    let msi = MagazineIssue {
        issue_number: 150,
        slug: "msi-150".into(),
        title_tr: "MSI 150".into(),
        title_en: "MSI 150".into(),
        publish_month_tr: "Ocak".into(),
        publish_month_en: "January".into(),
        year: 2026,
        cover_image: "images/msi/default.webp".into(),
        is_special_issue: false,
        event_name: None,
        page_count: 80,
        dossier_count: 1,
        summary_tr: "MSI".into(),
        summary_en: "MSI".into(),
        pdf_path: String::new(),
        publication_type: "msi".into(),
        uploaded_pdf_file: None,
        spreads: vec![MagazinePageSpread {
            page_no: "S. 5".into(),
            category_badge: "DOSYA".into(),
            title_tr: "Kapak".into(),
            title_en: "Cover".into(),
            excerpt_tr: String::new(),
            excerpt_en: String::new(),
            linked_content_slug: None,
            image_url: String::new(),
        }],
        ..Default::default()
    };
    let mut tdr = msi.clone();
    tdr.publication_type = "msi_tdr".into();
    tdr.slug = "tdr-150".into();

    let msi_card = issue_card(&msi, "tr");
    assert!(msi_card.contains(r#"data-open-issue="150" data-publication-type="msi""#));
    let tdr_card = issue_card(&tdr, "en");
    assert!(tdr_card.contains(r#"data-open-issue="150" data-publication-type="msi_tdr""#));

    let issues = vec![msi, tdr];
    let hero_section = Section {
        kind: SectionKind::MagazineHero,
        enabled: true,
        ..Section::default()
    };
    let mut seen = HashSet::new();
    let tr_data = PageData {
        lang: "tr",
        contents: &[],
        issues: &issues,
        tags: &[],
        ads: &[],
        plans: &[],
        products: &[],
        featured_issue: Some(150),
    };
    let tr_hero = render_section(&hero_section, &tr_data, 0, &mut seen);
    assert!(tr_hero.contains(r#"data-issue-number="150" data-publication-type="msi""#));
    assert!(tr_hero.contains(r#"data-open-issue="150" data-publication-type="msi""#));

    let en_data = PageData {
        lang: "en",
        ..tr_data
    };
    let en_hero = render_section(&hero_section, &en_data, 0, &mut seen);
    assert!(en_hero.contains(r#"data-issue-number="150" data-publication-type="msi_tdr""#));
    assert!(en_hero.contains(r#"data-open-issue="150" data-publication-type="msi_tdr""#));
}

#[tokio::test]
async fn m3_f3_content_detail_resolves_accept_language_and_cookie() {
    let (state, db_path) = common::build_test_state("m3_f3_lang");
    let app = common::build_test_router(&state);

    let mut item = make_article(
        950,
        "m3-f3-paywall-lang",
        "<p>Türkçe birinci cümle. Türkçe ikinci cümle. Türkçe üçüncü cümle.</p>",
    );
    item.body_en = "<p>English first sentence. English second sentence. English third sentence.</p>".into();
    item.visibility_level = 2;
    state.db.upsert_content(&item).unwrap();

    // 1. Accept-Language: en-US,en;q=0.9 (no ?lang=)
    let (st_al, body_al) = send_get(
        &app,
        "/api/v1/contents/m3-f3-paywall-lang",
        &[("accept-language", "en-US,en;q=0.9")],
    )
    .await;
    assert_eq!(st_al, StatusCode::OK);
    let json_al: Value = serde_json::from_str(&body_al).unwrap();
    let al_rendered = json_al["paywall"]["rendered_body"].as_str().unwrap_or("");
    let al_reason = json_al["paywall"]["lock_reason"].as_str().unwrap_or("");
    assert!(al_rendered.contains("English first sentence"), "got {}", al_rendered);
    assert!(al_reason.contains("subscriber") || al_reason.contains("Subscriber"), "got {}", al_reason);

    // 2. Cookie: msi_lang=en
    let (st_ck, body_ck) = send_get(
        &app,
        "/api/v1/contents/m3-f3-paywall-lang",
        &[("cookie", "msi_lang=en")],
    )
    .await;
    assert_eq!(st_ck, StatusCode::OK);
    let json_ck: Value = serde_json::from_str(&body_ck).unwrap();
    assert!(json_ck["paywall"]["rendered_body"]
        .as_str()
        .unwrap_or("")
        .contains("English first sentence"));

    // 3. Explicit ?lang=tr overrides Cookie: msi_lang=en
    let (st_ov, body_ov) = send_get(
        &app,
        "/api/v1/contents/m3-f3-paywall-lang?lang=tr",
        &[("cookie", "msi_lang=en")],
    )
    .await;
    assert_eq!(st_ov, StatusCode::OK);
    let json_ov: Value = serde_json::from_str(&body_ov).unwrap();
    assert!(json_ov["paywall"]["rendered_body"]
        .as_str()
        .unwrap_or("")
        .contains("Türkçe birinci cümle"));

    common::cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m3_f4_download_edergi_pdf_oob_uses_detect_pdf_page_count() {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!("/tmp/msi_m3_f4_{}_{}.db", std::process::id(), ts);
    let edergi_dir = format!("/tmp/msi_m3_edergi_{}_{}", std::process::id(), ts);
    std::fs::create_dir_all(&edergi_dir).unwrap();

    let mut cfg = common::test_config(&db_path);
    cfg.edergi_dir = edergi_dir.clone();
    let db = msi_dergisi_platform::db::Database::new(&db_path).expect("test db");
    let translations = TranslationCache::new();
    translations.load(db.load_all_translations());
    let state = AppState {
        config: std::sync::Arc::new(cfg),
        db,
        translations,
        monitoring_cache: None,
    };
    let app = common::build_test_router(&state);

    let (_, sub_cookie) = common::create_user_with_session(
        &state,
        "sub-m3-f4@aselsan.com.tr",
        "Pass1234!",
        "Abone M3",
        "subscriber_digital",
        "Savunma",
        "ASELSAN",
        false,
        "Dijital E-Dergi",
    );

    let real_4p = std::fs::read(
        "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/docs/specifications/proje-plani-ve-teslim-takvimi.pdf",
    )
    .expect("real 4-page PDF must exist");
    std::fs::write(format!("{}/real-4p.pdf", edergi_dir), &real_4p).unwrap();

    let iss = MagazineIssue {
        issue_number: 888,
        slug: "msi-888".into(),
        title_tr: "Sayı 888".into(),
        title_en: "Issue 888".into(),
        publish_month_tr: "Kasım".into(),
        publish_month_en: "November".into(),
        year: 2026,
        cover_image: "images/msi/default.webp".into(),
        is_special_issue: false,
        event_name: None,
        page_count: 1, // stale metadata; real PDF has 4 pages
        dossier_count: 0,
        summary_tr: "Özet".into(),
        summary_en: "Summary".into(),
        pdf_path: String::new(),
        publication_type: "msi".into(),
        uploaded_pdf_file: Some("real-4p.pdf".into()),
        spreads: vec![],
        ..Default::default()
    };
    state.db.upsert_issue(&iss).unwrap();

    // Out-of-bounds page 99 -> 400 BAD_REQUEST reporting actual PDF page count (4)
    let oob_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/edergi/888/download-pdf?pages=99&publication_type=msi&lang=en")
                .header(header::COOKIE, &sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(oob_resp.status(), StatusCode::BAD_REQUEST);
    let oob_bytes = to_bytes(oob_resp.into_body(), 1024 * 1024).await.unwrap();
    let oob_str = String::from_utf8_lossy(&oob_bytes);
    assert!(oob_str.contains("(4)"), "Expected detected page count 4 in {}", oob_str);

    // Valid page 1-2 -> 200 OK PDF
    let ok_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/edergi/888/download-pdf?pages=1-2&publication_type=msi")
                .header(header::COOKIE, &sub_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok_resp.status(), StatusCode::OK);
    let pdf_bytes = to_bytes(ok_resp.into_body(), 4 * 1024 * 1024).await.unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    let _ = std::fs::remove_dir_all(&edergi_dir);
    common::cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m3_iter2_content_detail_advertorials_protection_and_organic_related_filter() {
    let (state, db_path) = common::build_test_state("m3_iter2_detail_adv");
    let app = common::build_test_router(&state);

    // Seed explicit competing defense_oem ad and auxiliary_service/in_house ads
    let oem_ad = AdCampaign {
        id: 0,
        slot_code: "ad-mag-top".into(),
        badge_label: "SPONSORLU".into(),
        dimensions: "728x90".into(),
        sponsor_name: "Competitor Defense OEM".into(),
        sponsor_type: "defense_oem".into(),
        pill_style: "navy".into(),
        title_tr: "Rakip OEM Reklamı".into(),
        title_en: "Competitor OEM Ad".into(),
        desc_tr: "Rakip açıklaması".into(),
        desc_en: "Competitor desc".into(),
        target_url: "https://example.com/oem".into(),
        image_url: None,
        budget_score: 99,
        is_default_fallback: false,
        is_floating_popup: false,
        is_newsletter_ad: false,
        target_categories: vec!["advertorials".into(), "haberler".into()],
        target_ssb_tags: vec!["radarlar-ve-sensorler".into()],
        target_institutions: vec!["*".into()],
        impressions_count: 0,
        clicks_count: 0,
        is_active: true,
        start_date: None,
        end_date: None,
        rotation_mode: "weighted".into(),
    };
    state.db.upsert_ad(&oem_ad).unwrap();

    let aux_ad = AdCampaign {
        id: 0,
        slot_code: "ad-sidebar-tower".into(),
        badge_label: "DESTEK".into(),
        dimensions: "300x250".into(),
        sponsor_name: "Defense Logistics Aux".into(),
        sponsor_type: "auxiliary_service".into(),
        pill_style: "amber".into(),
        title_tr: "Yardımcı Hizmet Reklamı".into(),
        title_en: "Auxiliary Service Ad".into(),
        desc_tr: "Yardımcı hizmet".into(),
        desc_en: "Auxiliary service".into(),
        target_url: "https://example.com/aux".into(),
        image_url: None,
        budget_score: 50,
        is_default_fallback: true,
        is_floating_popup: false,
        is_newsletter_ad: false,
        target_categories: vec!["advertorials".into()],
        target_ssb_tags: vec!["radarlar-ve-sensorler".into()],
        target_institutions: vec!["*".into()],
        impressions_count: 0,
        clicks_count: 0,
        is_active: true,
        start_date: None,
        end_date: None,
        rotation_mode: "weighted".into(),
    };
    state.db.upsert_ad(&aux_ad).unwrap();

    // 1. content_type = "advertorials" with is_sponsored = false
    let mut adv_plural = make_article(
        960,
        "m3-iter2-adv-plural",
        "<p>P1.</p><p>P2.</p><p>P3.</p>",
    );
    adv_plural.content_type = "advertorials".into();
    adv_plural.is_sponsored = false;
    adv_plural.ssb_tags = vec!["radarlar-ve-sensorler".into()];
    state.db.upsert_content(&adv_plural).unwrap();

    // 2. Organic article sharing tag + matching sponsored / advertorials candidates
    let mut organic_main = make_article(
        961,
        "m3-iter2-organic-main",
        "<p>P1.</p><p>P2.</p><p>P3.</p>",
    );
    organic_main.ssb_tags = vec!["radarlar-ve-sensorler".into()];
    state.db.upsert_content(&organic_main).unwrap();

    let mut sponsored_cand = make_article(
        962,
        "m3-iter2-sponsored-cand",
        "<p>P1.</p><p>P2.</p>",
    );
    sponsored_cand.is_sponsored = true;
    sponsored_cand.ssb_tags = vec!["radarlar-ve-sensorler".into()];
    state.db.upsert_content(&sponsored_cand).unwrap();

    let mut adv_cand = make_article(
        963,
        "m3-iter2-adv-cand",
        "<p>P1.</p><p>P2.</p>",
    );
    adv_cand.content_type = "advertorials".into();
    adv_cand.is_sponsored = false;
    adv_cand.ssb_tags = vec!["radarlar-ve-sensorler".into()];
    state.db.upsert_content(&adv_cand).unwrap();

    for idx in 964..=966 {
        let mut org_cand = make_article(
            idx,
            &format!("m3-iter2-organic-cand-{}", idx),
            "<p>P1.</p><p>P2.</p>",
        );
        org_cand.ssb_tags = vec!["radarlar-ve-sensorler".into()];
        state.db.upsert_content(&org_cand).unwrap();
    }

    // High-priority RecommendationRule pointing directly to the sponsored candidate
    let rule = RecommendationRule {
        id: 0,
        rule_name: "M3 Iter2 Sponsored Rule".into(),
        source_tag_or_cat: "radarlar-ve-sensorler".into(),
        target_institution: "*".into(),
        recommended_content_slug: "m3-iter2-sponsored-cand".into(),
        priority: 99,
        is_active: true,
    };
    state.db.insert_recommendation_rule(&rule).unwrap();

    // Verify GET /api/v1/contents/m3-iter2-adv-plural
    let (st_adv, body_adv) = send_get(&app, "/api/v1/contents/m3-iter2-adv-plural", &[]).await;
    assert_eq!(st_adv, StatusCode::OK);
    let json_adv: Value = serde_json::from_str(&body_adv).unwrap();
    assert_eq!(
        json_adv["paywall"]["competitor_protection_active"],
        Value::Bool(true),
        "advertorials content must activate competitor_protection_active"
    );
    assert_eq!(
        json_adv["related_articles"].as_array().map(|a| a.len()),
        Some(0),
        "advertorials content must return empty related_articles"
    );
    let adv_ads = json_adv["context_ads"].as_array().expect("context_ads array");
    assert!(
        adv_ads
            .iter()
            .all(|ad| ad["sponsor_type"].as_str() != Some("defense_oem")),
        "advertorials content must exclude defense_oem ads: {:?}",
        adv_ads
    );
    assert!(
        adv_ads.iter().any(|ad| {
            matches!(
                ad["sponsor_type"].as_str(),
                Some("auxiliary_service") | Some("in_house")
            )
        }),
        "advertorials content should still permit auxiliary_service/in_house ads: {:?}",
        adv_ads
    );

    // Verify GET /api/v1/contents/m3-iter2-organic-main excludes sponsored & advertorials candidates
    let (st_org, body_org) = send_get(&app, "/api/v1/contents/m3-iter2-organic-main", &[]).await;
    assert_eq!(st_org, StatusCode::OK);
    let json_org: Value = serde_json::from_str(&body_org).unwrap();
    assert_eq!(
        json_org["paywall"]["competitor_protection_active"],
        Value::Bool(false)
    );
    let rel_arr = json_org["related_articles"]
        .as_array()
        .expect("related_articles array");
    assert_eq!(
        rel_arr.len(),
        3,
        "Organic article should still receive 3 clean related articles after filtering sponsored/advertorials"
    );
    let rel_slugs: Vec<&str> = rel_arr
        .iter()
        .filter_map(|r| r["slug"].as_str())
        .collect();
    assert!(
        !rel_slugs.contains(&"m3-iter2-sponsored-cand"),
        "Sponsored candidate must be excluded from organic related_articles: {:?}",
        rel_slugs
    );
    assert!(
        !rel_slugs.contains(&"m3-iter2-adv-cand") && !rel_slugs.contains(&"m3-iter2-adv-plural"),
        "Advertorials candidates must be excluded from organic related_articles: {:?}",
        rel_slugs
    );

    common::cleanup_test_db(&db_path);
}

#[tokio::test]
async fn m3_iter2_wildcard_ssb_tag_not_emitted_on_article_page() {
    let (state, db_path) = common::build_test_state("m3_iter2_wildcard_ssb");
    let app = common::build_test_router(&state);

    let mut wildcard_only = make_article(
        970,
        "m3-iter2-wildcard-only",
        "<p>P1.</p><p>P2.</p><p>P3.</p>",
    );
    wildcard_only.ssb_tags = vec!["*".into()];
    state.db.upsert_content(&wildcard_only).unwrap();

    let mut wildcard_leading = make_article(
        971,
        "m3-iter2-wildcard-leading",
        "<p>P1.</p><p>P2.</p><p>P3.</p>",
    );
    wildcard_leading.ssb_tags = vec!["*".into(), "kara-araclari".into()];
    state.db.upsert_content(&wildcard_leading).unwrap();

    let (st1, html1) = send_get(&app, "/icerik/m3-iter2-wildcard-only", &[]).await;
    assert_eq!(st1, StatusCode::OK);
    let root1_start = html1.find(r#"<div class="article-page""#).unwrap();
    let root1_end = html1[root1_start..].find('>').unwrap() + root1_start;
    let root1 = &html1[root1_start..=root1_end];
    assert!(
        !root1.contains(r#"data-ssb-tag="*""#) && !root1.contains("data-ssb-tag="),
        "Wildcard-only ssb_tags must not emit data-ssb-tag on .article-page: {}",
        root1
    );

    let (st2, html2) = send_get(&app, "/icerik/m3-iter2-wildcard-leading", &[]).await;
    assert_eq!(st2, StatusCode::OK);
    let root2_start = html2.find(r#"<div class="article-page""#).unwrap();
    let root2_end = html2[root2_start..].find('>').unwrap() + root2_start;
    let root2 = &html2[root2_start..=root2_end];
    assert!(
        !root2.contains(r#"data-ssb-tag="*""#),
        "Leading wildcard must not emit data-ssb-tag=\"*\" on .article-page: {}",
        root2
    );
    assert!(
        root2.contains(r#"data-ssb-tag="kara-araclari""#),
        "Leading wildcard must fall through to first non-wildcard tag on .article-page: {}",
        root2
    );

    common::cleanup_test_db(&db_path);
}
