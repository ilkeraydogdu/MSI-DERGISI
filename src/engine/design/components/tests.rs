use super::*;
use super::super::config::{Page, Section, SectionKind};
use super::select::select;
    fn content(source: &str, slug: &str) -> ContentItem {
        ContentItem {
            id: 1,
            slug: slug.into(),
            title_tr: "<script>alert(1)</script>".into(),
            title_en: String::new(),
            summary_tr: "A & B".into(),
            summary_en: String::new(),
            body_tr: String::new(),
            body_en: String::new(),
            content_type: "haberler".into(),
            source_group: source.into(),
            cover_image: "images/example.webp".into(),
            author_name: String::new(),
            author_title: String::new(),
            issue_number: None,
            published_at: "2026-11-08".into(),
            read_time_min: 3,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: false,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: false,
            video_url: None,
            view_count: 0,
            true_read_count: 0,
            placements: vec![],
            ssb_tags: vec![],
            status: "published".into(),
        }
    }
    #[test]
    fn cards_escape_editorial_strings() {
        let html = article_card(&content("gunluk-haber", "safe-slug"), "tr", false);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("A &amp; B"));
    }
    #[test]
    fn page_sections_filter_and_order_without_unrelated_fallback() {
        let contents = vec![
            content("gunluk-haber", "news"),
            content("dergi-icerigi", "magazine"),
        ];
        let data = PageData {
            lang: "tr",
            contents: &contents,
            issues: &[],
            tags: &[],
            ads: &[],
            plans: &[],
            products: &[],
            featured_issue: None,
        };
        let mut section = Section {
            source: "magazine".into(),
            ..Section::default()
        };
        assert_eq!(select(&section, &data)[0].slug, "magazine");
        section.source = "news".into();
        assert_eq!(select(&section, &data)[0].slug, "news");
        section.category = "nonexistent".into();
        assert!(select(&section, &data).is_empty());
        section.category.clear();
        section.offset = 1;
        assert!(select(&section, &data).is_empty());
        section.offset = 0;
        section.enabled = false;
        let page = Page {
            slug: "test".into(),
            title: "Test".into(),
            description: String::new(),
            mode: "news".into(),
            sections: vec![section],
        };
        assert!(!render_page(&page, &data).contains("story-card"));
    }
    #[test]
    fn custom_text_is_safe_and_rendered_once() {
        let data = PageData {
            lang: "tr",
            contents: &[],
            issues: &[],
            tags: &[],
            ads: &[],
            plans: &[],
            products: &[],
            featured_issue: None,
        };
        let section = Section {
            kind: SectionKind::Text,
            title: "Text".into(),
            description: "Unique <img onerror=alert(1)>".into(),
            ..Section::default()
        };
        let page = Page {
            slug: "test".into(),
            title: "Title".into(),
            description: String::new(),
            mode: "magazine".into(),
            sections: vec![section],
        };
        let html = render_page(&page, &data);
        assert_eq!(html.matches("Unique").count(), 1);
        assert!(!html.contains("<img onerror"));
        assert_eq!(html.matches("<h1>").count(), 1);
    }
    #[test]
    fn parses_real_webp_dimensions_and_frames_portrait_cards() {
        let dims = image_dimensions("images/msi/msi-dergisi-sayi-286-kapak.webp");
        assert_eq!(dims, Some((960, 1200)));
        let mut portrait = content("dergi-icerigi", "portrait-card");
        portrait.cover_image = "images/msi/msi-dergisi-sayi-286-kapak.webp".into();
        let card_html = article_card(&portrait, "tr", true);
        assert!(card_html.contains("story-image is-portrait"));
        assert!(card_html.contains("story-image-backdrop"));
        assert!(card_html.contains("width=\"960\" height=\"1200\""));
    }
    #[test]
    fn editorial_sidebar_highlights_active_category() {
        let html = editorial_sidebar("ozel-dosya", &[content("gunluk-haber", "rel-1")], &[], "tr");
        assert!(html.contains("sidebar-cat-btn is-active\" href=\"/kategori/ozel-dosya\""));
        assert!(html.contains("İLGİNİZİ ÇEKEBİLİR"));
    }
    #[test]
    fn cross_section_deduplication_prevents_repeated_cards_on_same_page() {
        let mut first = content("dergi-icerigi", "shared-slug");
        first.id = 10;
        first.content_type = "kapak-dosyasi".into();
        first.placements = vec!["soylesiler".into()];
        let mut second = content("dergi-icerigi", "interview-only");
        second.id = 11;
        second.content_type = "soylesiler".into();
        let contents = vec![first, second];
        let data = PageData {
            lang: "tr",
            contents: &contents,
            issues: &[],
            tags: &[],
            ads: &[],
            plans: &[],
            products: &[],
            featured_issue: None,
        };
        let page = Page {
            slug: "mag".into(),
            title: "Mag".into(),
            description: String::new(),
            mode: "magazine".into(),
            sections: vec![
                Section {
                    kind: SectionKind::Articles,
                    source: "all".into(),
                    category: "kapak-dosyasi".into(),
                    limit: 3,
                    enabled: true,
                    ..Section::default()
                },
                Section {
                    kind: SectionKind::Articles,
                    source: "all".into(),
                    category: "soylesiler".into(),
                    limit: 3,
                    enabled: true,
                    ..Section::default()
                },
            ],
        };
        let html = render_page(&page, &data);
        assert_eq!(html.matches("/icerik/shared-slug\"").count(), 2); // 1 image link + 1 title link = 1 card
        assert_eq!(html.matches("/icerik/interview-only\"").count(), 2);
    }
    #[test]
    fn ssb_tags_and_enriched_spreads_filter_unpublished_drafts() {
        assert_eq!(ssb_tag_display_label("hava-araclari", "tr"), "Hava Araçları");
        assert_eq!(
            ssb_tag_display_label("muhimmatlar-ve-fuze-sistemleri", "tr"),
            "Mühimmatlar ve Füze Sistemleri"
        );
        let issue = MagazineIssue {
            issue_number: 286,
            slug: "sayi-286".into(),
            title_tr: "Sayı 286".into(),
            title_en: "Issue 286".into(),
            publish_month_tr: "Kasım".into(),
            publish_month_en: "November".into(),
            year: 2026,
            cover_image: "images/example.webp".into(),
            is_special_issue: false,
            event_name: None,
            page_count: 96,
            dossier_count: 6,
            summary_tr: "Özel savunma dosyası özeti.".into(),
            summary_en: String::new(),
            pdf_path: String::new(),
            publication_type: "msi".into(),
            uploaded_pdf_file: None,
            spreads: vec![],
        };
        let mut pub_item = content("dergi-icerigi", "published-dossier");
        pub_item.issue_number = Some(286);
        pub_item.title_tr = "KAAN Seri Üretim Analizi".into();
        let mut draft_item = content("dergi-icerigi", "draft-dossier");
        draft_item.issue_number = Some(286);
        draft_item.status = "draft".into();
        let spreads = enrich_issue_spreads(&issue, &[pub_item, draft_item]);
        assert_eq!(spreads.len(), 1);
        assert_eq!(spreads[0].linked_content_slug.as_deref(), Some("published-dossier"));
    }
