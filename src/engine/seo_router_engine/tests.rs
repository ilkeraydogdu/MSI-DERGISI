use super::*;
use std::collections::HashMap;

    fn test_brand() -> SeoBrandConfig {
        SeoBrandConfig::from_settings(&HashMap::new())
    }

    fn sample_article(vis: i32) -> ContentItem {
        ContentItem {
            id: 1,
            slug: "kaan-milli-muharip-ucak-analizi".into(),
            title_tr: "KAAN Milli Muharip Uçak Analizi".into(),
            title_en: "KAAN National Combat Aircraft Analysis".into(),
            summary_tr: "KAAN programında test ve seri üretim yol haritası.".into(),
            summary_en: "Testing and serial production roadmap in KAAN program.".into(),
            body_tr: "<p>Detaylı savunma ve havacılık mühendisliği incelemesi.</p>".into(),
            body_en: "<p>Detailed aerospace engineering analysis.</p>".into(),
            content_type: "ozel-haberler".into(),
            source_group: "dergi-icerigi".into(),
            cover_image: "images/msi/msi-dergisi-tusas-kaan-milli-muharip-ucak.webp".into(),
            author_name: "Ümit Bayraktar".into(),
            author_title: "Genel Yayın Yönetmeni".into(),
            issue_number: Some(286),
            published_at: "2026-10-24 16:38:00".into(),
            read_time_min: 6,
            visibility_level: vis,
            single_price_try: 95.0,
            is_sponsored: false,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: true,
            video_url: None,
            view_count: 500,
            true_read_count: 250,
            placements: vec!["ozel-haberler".into(), "kapak-dosyalari".into()],
            ssb_tags: vec!["hava-araclari".into(), "radarlar-ve-sensorler".into()],
            status: "published".into(),
        }
    }

    #[test]
    fn gated_article_emits_google_paywall_flexible_sampling_and_article_meta() {
        let art = sample_article(2);
        let brand = test_brand();
        let meta = build_seo_meta(
            "https://msidergisi.com",
            "/icerik/kaan-milli-muharip-ucak-analizi",
            &art.title_tr,
            &art.summary_tr,
            &art.cover_image,
            "article",
            "tr",
            Some(&art),
            &brand,
            "msidergisi.com",
            "savunmahaber.com",
        );
        assert!(meta.json_ld.contains("\"NewsMediaOrganization\""));
        assert!(meta.json_ld.contains("\"isAccessibleForFree\":false"));
        assert!(meta.json_ld.contains(".paywall-panel"));
        assert!(meta.json_ld.contains("1306-0309"));
        let head = render_seo_head_tags(&meta, None, &brand);
        assert!(head.contains("article:published_time"));
        assert!(head.contains("2026-10-24T16:38:00+03:00"));
        assert!(head.contains("name=\"yandex\""));
    }

    #[test]
    fn news_and_llms_feeds_are_well_formed() {
        let art = sample_article(0);
        let brand = test_brand();
        let news_xml = CentralSeoEngine::generate_news_sitemap_xml("https://msidergisi.com", &[art.clone()], &brand);
        assert!(news_xml.contains("<news:publication_date>2026-10-24T16:38:00+03:00</news:publication_date>"));
        let llms = CentralSeoEngine::generate_llms_txt("https://msidergisi.com", &[], &[], &[], &[art], false, &brand);
        assert!(llms.contains("ISSN: 1306-0309"));
        assert!(llms.contains("/icerik/kaan-milli-muharip-ucak-analizi"));
        let robots = generate_robots_txt("https://msidergisi.com");
        assert!(robots.contains("Host: msidergisi.com"));
        assert!(robots.contains("Sitemap: https://msidergisi.com/sitemap-news.xml"));
    }
