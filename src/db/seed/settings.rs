use rusqlite::{params, Connection};

pub fn seed_site_settings_and_notifications(conn: &Connection) -> rusqlite::Result<()> {
    let settings: &[(&str, &str, &str, &str, &str)] = &[
        (
            "brand_title",
            "MSI Dergisi — Savunma Teknolojileri ve Havacılık Platformu",
            "MSI TDR — Turkish Defence Review",
            "branding",
            "2026-11-07 10:00:00",
        ),
        (
            "brand_tagline",
            "Finansal ve Stratejik Savunma İstihbaratında 22 Yıllık Otorite. Sıfır Karmaşa, Mutlak Kontrol.",
            "22 Years of Authority in Defence & Aerospace Intelligence.",
            "branding",
            "2026-11-07 10:00:00",
        ),
        (
            "active_theme_mode",
            "msi-default",
            "msi-time-editorial",
            "theme",
            "2026-11-07 10:00:00",
        ),
        (
            "primary_accent_color",
            "#cb382a",
            "#cb382a",
            "theme",
            "2026-11-07 10:00:00",
        ),
        (
            "announcement_banner_text",
            "280. Sayı (Eylül 2026) ve SAHA EXPO Özel Sayısı Dijital Kiosk'ta Yayında! Kamu (.gov.tr / .tsk.tr) Personeline Ücretsiz E-Dergi Aboneliği.",
            "Issue 280 & SAHA EXPO Special Edition Live in Digital Kiosk! Free E-Magazine Subscription for Public/Military (.gov.tr / .tsk.tr) Personnel.",
            "announcement",
            "2026-11-07 10:00:00",
        ),
        (
            "contact_address",
            "Yaşamkent Mah. 3147 Sok. No:10 Çankaya / Ankara TÜRKİYE",
            "Çankaya, Ankara / Türkiye",
            "contact",
            "2026-11-07 10:00:00",
        ),
        (
            "contact_phone",
            "+90 (312) 225 41 73",
            "+90 (312) 225 41 73",
            "contact",
            "2026-11-07 10:00:00",
        ),
        (
            "contact_email",
            "info@savunmahaber.com",
            "info@savunmahaber.com",
            "contact",
            "2026-11-11 00:00:00",
        ),
        (
            "contact_fax",
            "+90 (312) 225 41 74",
            "+90 (312) 225 41 74",
            "contact",
            "2026-11-11 00:00:00",
        ),
        (
            "current_featured_issue",
            "286",
            "286",
            "editorial",
            "2026-11-11 00:00:00",
        ),
        (
            "social_facebook",
            "https://facebook.com/msidergisi",
            "https://facebook.com/msidergisi",
            "social",
            "2026-11-11 00:00:00",
        ),
        (
            "social_linkedin",
            "https://linkedin.com/company/msidergisi",
            "https://linkedin.com/company/msidergisi",
            "social",
            "2026-11-11 00:00:00",
        ),
        (
            "social_instagram",
            "https://instagram.com/msidergisi",
            "https://instagram.com/msidergisi",
            "social",
            "2026-11-11 00:00:00",
        ),
        (
            "social_twitter",
            "https://twitter.com/msidergisi",
            "https://twitter.com/msidergisi",
            "social",
            "2026-11-11 00:00:00",
        ),
        (
            "social_youtube",
            "https://youtube.com/@msidergisi",
            "https://youtube.com/@msidergisi",
            "social",
            "2026-11-11 00:00:00",
        ),
        (
            "brand_slogan",
            "Aylık, Türkçe ve İngilizce yayımlanan Savunma, Havacılık ve Güvenlik Teknolojileri Dergisi",
            "Monthly Turkish & English Defence, Aviation and Security Technologies Magazine",
            "branding",
            "2026-11-11 00:00:00",
        ),
        // ── SEO / Publisher Entity constants (centralized for seo_router_engine) ──
        (
            "publisher_legal_name",
            "UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.",
            "UMSA Production Printing and Publishing Ltd. Co.",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "publisher_founding_date",
            "2004",
            "2004",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "publisher_issn",
            "1306-0309",
            "1306-0309",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "default_og_image",
            "/images/msi/msi-dergisi-sosyal-paylasim-kapagi.webp",
            "/images/msi/msi-dergisi-sosyal-paylasim-kapagi.webp",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "default_edergi_cover",
            "/images/msi/msi-dergisi-sayi-286-kapak.webp",
            "/images/msi/msi-dergisi-sayi-286-kapak.webp",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "publisher_logo_url",
            "/images/logo.svg",
            "/images/logo.svg",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "publisher_address_street",
            "Yaşamkent Mah. 3147 Sok. No:10",
            "Yasamkent Mah. 3147 Sok. No:10",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "publisher_address_locality",
            "Çankaya",
            "Cankaya",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "publisher_address_region",
            "Ankara",
            "Ankara",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "publisher_postal_code",
            "06810",
            "06810",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "publisher_country",
            "TR",
            "TR",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "twitter_handle",
            "@MSI_Dergisi",
            "@MSI_Dergisi",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "default_keywords_tr",
            "MSI Dergisi, savunmahaber, Türk savunma sanayii, havacılık, uzay, deniz platformları, İHA, SİHA, KAAN, MİLGEM, füze sistemleri, elektronik harp",
            "MSI TDR, Turkish Defence Review, Turkish defence industry, aerospace, naval platforms, UAV, missiles, radar, electronic warfare",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "rss_title_tr",
            "MSI Dergisi — Savunmahaber.com | Savunma, Havacılık ve Güvenlik Teknolojileri",
            "MSI TDR — Turkish Defence Review | Defence & Aerospace Intelligence",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "rss_description_tr",
            "MSI Dergisi (ISSN 1306-0309) ve savunmahaber.com tekil içerik havuzundan güncel savunma sanayii haberleri, özel dosyalar, söyleşiler ve analizler.",
            "Authoritative analysis, Turkish defence industry news, special dossiers, and executive interviews from MSI Turkish Defence Review (ISSN 1306-0309).",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "copyright_notice",
            "UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.",
            "UMSA Production Printing and Publishing Ltd. Co.",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "opensearch_short_name",
            "MSI Dergisi",
            "MSI TDR",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "opensearch_description",
            "MSI Dergisi ve MSI TDR savunma, havacılık ve güvenlik teknolojileri arşivinde arama yapın.",
            "Search the MSI Dergisi & MSI TDR defence, aviation and security technologies archive.",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "manifest_short_name",
            "MSI Dergisi",
            "MSI TDR",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "manifest_description",
            "Savunma, havacılık ve güvenlik teknolojilerinde 2004'ten bugüne stratejik referans yayını (ISSN 1306-0309).",
            "Strategic reference publication in defence, aviation and security technologies since 2004 (ISSN 1306-0309).",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "llms_header_text",
            "Türkiye'nin 2004 yılından bu yana kesintisiz yayımlanan lider Savunma, Havacılık, Uzay ve Denizcilik Teknolojileri referans yayını (ISSN: 1306-0309). Yayıncı: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. (Ankara, Türkiye).",
            "Türkiye's leading uninterrupted Defence, Aviation, Space and Maritime Technologies reference publication since 2004 (ISSN: 1306-0309). Publisher: UMSA Production Printing and Publishing Ltd. Co. (Ankara, Türkiye).",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "category_fallback_desc",
            "MSI Dergisi savunma sanayii, havacılık ve güvenlik teknolojileri içerik arşivi.",
            "MSI TDR defence industry, aviation and security technologies content archive.",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "json_feed_description",
            "2004'ten bugüne Türk savunma ve havacılık sanayisinin stratejik referans yayını (ISSN 1306-0309).",
            "Strategic reference publication of the Turkish defence and aviation industry since 2004 (ISSN 1306-0309).",
            "seo",
            "2026-11-11 00:00:00",
        ),
    ];

    for s in settings {
        conn.execute(
            "INSERT OR IGNORE INTO site_settings (key, value_tr, value_en, setting_group, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![s.0, s.1, s.2, s.3, s.4],
        )?;
    }

    Ok(())
}

/// Idempotently inserts missing seeded `site_settings` keys on existing databases
/// and upgrades legacy default brand values (`/images/logo.png` -> `/images/logo.svg`,
/// `#c91818`/`#c81623`/`#e63946` -> `#cb382a`) without overwriting custom operator overrides.
pub fn sync_brand_site_settings(conn: &Connection) -> rusqlite::Result<()> {
    seed_site_settings_and_notifications(conn)?;
    conn.execute(
        "UPDATE site_settings
         SET value_tr = '/images/logo.svg', value_en = '/images/logo.svg'
         WHERE key = 'publisher_logo_url'
           AND (TRIM(value_tr) = '' OR value_tr = '/images/logo.png')",
        [],
    )?;
    conn.execute(
        "UPDATE site_settings
         SET value_tr = '#cb382a', value_en = '#cb382a'
         WHERE key = 'primary_accent_color'
           AND LOWER(TRIM(value_tr)) IN ('', '#c91818', '#c81623', '#e63946')",
        [],
    )?;
    Ok(())
}
