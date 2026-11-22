use rusqlite::{params, Connection};

pub fn seed_categories(conn: &Connection) -> rusqlite::Result<()> {
    let categories: &[(&str, Option<&str>, &str, &str, &str, &str, &str, &str, i32, i32)] = &[
        ("haberler", None, "Haberler", "Defence News", "icerikler", "Günlük savunma, havacılık ve denizcilik haber akışı.", "Daily defence, aerospace and maritime news stream.", "ph-newspaper", 1, 1),
        ("ozel-haberler", None, "Özel Haberler", "Exclusive News", "icerikler", "MSI Dergisi editörlerinin sahadan özel istihbarat ve haber dosyaları.", "Exclusive field reports and defence intelligence by MSI editors.", "ph-star-four", 2, 1),
        ("soylesiler", None, "Söyleşiler", "Interviews", "icerikler", "Savunma sanayii liderleri, kuvvet komutanları ve karar vericilerle üst düzey söyleşiler.", "Executive interviews with defence industry leaders and force commanders.", "ph-microphone-stage", 3, 1),
        ("mesajlar", None, "Mesajlar", "Official Messages", "icerikler", "Millî Savunma Bakanlığı, TSK, SSB ve dost ülke makamlarının resmi mesajları.", "Official messages from MoND, TAF, SSB and allied defence authorities.", "ph-seal-check", 4, 1),
        ("tanitici-makaleler", None, "Tanıtıcı Makaleler", "Technical Showcases", "icerikler", "Platform, alt sistem ve ileri teknoloji çözümlerinin teknik mimari incelemeleri.", "Technical architecture reviews of platforms, subsystems and defence technologies.", "ph-cpu", 5, 1),
        ("makaleler-analizler", None, "Makaleler / Analizler", "Articles & Strategic Analysis", "icerikler", "Stratejik doktrin, harp tarihi, savunma ekonomisi ve kavramsal analiz makaleleri.", "Strategic doctrine, defence economics and conceptual operational analyses.", "ph-chart-polar", 6, 1),
        ("advertorial", None, "Advertorial", "Sponsored Content", "icerikler", "Reklam ve sponsorluk mevzuatına uygun kurumsal teknoloji dosyaları.", "Compliant sponsored defence technology dossiers and corporate briefs.", "ph-megaphone", 7, 1),
        ("basyazi", None, "Başyazı", "Editorial", "icerikler", "MSI Dergisi Yayın Kurulu tarafından kaleme alınan aylık stratejik perspektif.", "Monthly strategic perspective penned by the MSI Magazine Editorial Board.", "ph-pen-nib", 8, 1),
        ("ozel-dosyalar", None, "Özel Dosyalar", "Special Dossiers", "gruplandirilmis", "Kritik projeler ve harekât konseptleri üzerine çok boyutlu araştırma dosyaları.", "Multi-dimensional research dossiers on critical defence programs.", "ph-folder-Simple-star", 9, 1),
        ("kapak-dosyalari", None, "Kapak Dosyaları", "Cover Stories", "gruplandirilmis", "Her sayının ana gündemini belirleyen kapsamlı kapak araştırmaları.", "Comprehensive cover investigations setting the monthly defence agenda.", "ph-book-bookmark", 10, 1),
        ("tskdan-haberler", None, "TSK'dan Haberler", "Turkish Armed Forces", "gruplandirilmis", "Türk Silahlı Kuvvetleri tatbikat, envanter kabul ve harekât hazırlık faaliyetleri.", "TAF exercises, inventory inductions and operational readiness updates.", "ph-shield-chevron", 11, 1),
        ("dergi-icerikleri", None, "Dergi İçerikleri", "Magazine Features", "gruplandirilmis", "Basılı ve E-Dergi sayılarında yayımlanan hakemli ve editoryal içerikler.", "Editorial features published across print and digital magazine issues.", "ph-books", 12, 1),
        ("etkinlikler", None, "Etkinlikler", "Defence Events", "gruplandirilmis", "IDEF, SAHA EXPO, MSPO, DSEI, Eurosatory ve küresel savunma zirveleri.", "IDEF, SAHA EXPO, MSPO, DSEI, Eurosatory and global defence summits.", "ph-globe-hemisphere-east", 13, 1),
        ("etkinlik-ozel-sayilari", Some("etkinlikler"), "Etkinlik Özel Sayıları", "Event Special Issues", "gruplandirilmis", "Uluslararası fuar ve zirveler için hazırlanan çift dilli özel sayılar.", "Bilingual special editions prepared for international defence exhibitions.", "ph-medal", 14, 1),
        ("etkinlik-sonrasi-ozel-dosyalar", Some("etkinlikler"), "Etkinlik Sonrası Özel Dosyalar", "Post-Event Dossiers", "gruplandirilmis", "Fuarlarda imzalanan sözleşmeler, lansmanlar ve saha analizleri.", "Post-exhibition contract analyses, product launches and field assessments.", "ph-Note-pencil", 15, 1),
        ("e-dergi", None, "E-Dergi", "E-Magazine", "abonelik", "DRM korumalı ve dinamik filigranlı yüksek çözünürlüklü dijital dergi kütüphanesi.", "DRM-protected, dynamically watermarked high-resolution digital magazine library.", "ph-book-open-text", 16, 1),
        ("abonelik-ve-satis", None, "Abonelik ve Satış", "Subscriptions & Store", "abonelik", "Kurumsal/Bireysel abonelik paketleri, prestij kitapları ve RBF fotoğraf lisansları.", "Corporate/Individual subscription tiers, prestige books and RBF photo licenses.", "ph-shopping-bag-open", 17, 1),
        ("hakkimizda", None, "Hakkımızda", "About Us", "kurumsal", "UMSA Prodüksiyon, MSI Dergisi ve MSI TDR kurumsal yapısı.", "Corporate structure of UMSA Production, MSI Magazine and MSI TDR.", "ph-buildings", 18, 1),
        ("kunye", Some("hakkimizda"), "Künye", "Masthead", "kurumsal", "Yayın kurulu, danışma kurulu, editöryal kadro ve yasal yayın bilgileri.", "Editorial board, advisory board, staff and legal publication details.", "ph-identification-card", 19, 1),
        ("biz-kimiz", Some("hakkimizda"), "Biz Kimiz", "Who We Are", "kurumsal", "2005'ten bugüne bağımsız savunma yayıncılığı ilkelerimiz.", "Our independent defence journalism principles since 2005.", "ph-users-three", 20, 1),
        ("yolculugumuz", Some("hakkimizda"), "Yolculuğumuz", "Our Journey", "kurumsal", "MSI Dergisi'nin 20 yılı aşan yayıncılık ve uluslararası fuar kilometre taşları.", "Over 20 years of publishing and international exhibition milestones.", "ph-compass", 21, 1),
        ("cozum-ortaklarimiz", Some("hakkimizda"), "Çözüm Ortaklarımız", "Solution Partners", "kurumsal", "Ulusal ve uluslararası sektörel iş birliklerimiz ve medya ortaklıklarımız.", "National and international defence media partnerships and alliances.", "ph-handshake", 22, 1),
        ("amac-magazine", Some("hakkimizda"), "AMAC Magazine", "AMAC Magazine", "kurumsal", "Ankara Askerî Ataşeler Birliği (AMAC) resmi yayın projesi.", "Official publication project for the Ankara Military Attaché Corps (AMAC).", "ph-flag-banner", 23, 1),
        ("kasabaworks", Some("hakkimizda"), "KasabaWorks", "KasabaWorks Agency", "kurumsal", "Savunma sanayii odaklı kurumsal iletişim, tasarım ve kreatif prodüksiyon ajansımız.", "Defence-focused corporate communications, design and creative production agency.", "ph-paint-brush-broad", 24, 1),
        ("iletisim", Some("hakkimizda"), "İletişim", "Contact", "kurumsal", "Ankara merkez ofis, reklam rezervasyon ve kurumsal abonelik iletişim kanalları.", "Ankara headquarters, ad reservation and corporate subscription contact channels.", "ph-envelope-simple-open", 25, 1),
        ("savunma-sanayii-tarihi", None, "Savunma Sanayii Tarihi", "Defence Industry History", "gelecek-faz", "Türk ve dünya savunma sanayii kronolojisi ve dokümantasyon arşivi.", "Chronology and archival documentation of Turkish and global defence history.", "ph-hourglass-high", 26, 1),
        ("yazarlar", None, "Yazarlar", "Columnists & Authors", "gelecek-faz", "Akademisyenler, emekli komutanlar ve savunma analistleri dizini.", "Directory of academics, retired commanders and defence analysts.", "ph-feather", 27, 1),
        ("ogrenci-kulupleri", None, "Öğrenci Kulüpleri", "University Defence Clubs", "gelecek-faz", "Üniversite savunma teknolojileri ve havacılık toplulukları ekosistemi.", "University defence technologies and aerospace societies ecosystem.", "ph-graduation-cap", 28, 1),
        ("insan-kaynaklari", None, "İnsan Kaynakları", "Human Resources", "gelecek-faz", "Savunma ve havacılık ekosisteminde kariyer fırsatları, yetenek programları ve mühendislik istihdamı.", "Career opportunities, talent programmes and engineering recruitment in the defence and aerospace ecosystem.", "ph-briefcase", 29, 1),
    ];

    for c in categories {
        conn.execute(
            "INSERT INTO categories (slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(slug) DO NOTHING",
            params![c.0, c.1, c.2, c.3, c.4, c.5, c.6, c.7, c.8, c.9],
        )?;
    }
    seed_option_b_email_groups(conn)?;
    Ok(())
}

pub fn seed_option_b_categories(conn: &Connection) -> rusqlite::Result<()> {
    let categories: &[(&str, Option<&str>, &str, &str, &str, &str, &str, &str, i32, i32)] = &[
        ("savunma-sanayii-tarihi", None, "Savunma Sanayii Tarihi", "Defence Industry History", "gelecek-faz", "Türk ve dünya savunma sanayii kronolojisi ve dokümantasyon arşivi.", "Chronology and archival documentation of Turkish and global defence history.", "ph-hourglass-high", 26, 1),
        ("yazarlar", None, "Yazarlar", "Columnists & Authors", "gelecek-faz", "Akademisyenler, emekli komutanlar ve savunma analistleri dizini.", "Directory of academics, retired commanders and defence analysts.", "ph-feather", 27, 1),
        ("ogrenci-kulupleri", None, "Öğrenci Kulüpleri", "University Defence Clubs", "gelecek-faz", "Üniversite savunma teknolojileri ve havacılık toplulukları ekosistemi.", "University defence technologies and aerospace societies ecosystem.", "ph-graduation-cap", 28, 1),
        ("insan-kaynaklari", None, "İnsan Kaynakları", "Human Resources", "gelecek-faz", "Savunma ve havacılık ekosisteminde kariyer fırsatları, yetenek programları ve mühendislik istihdamı.", "Career opportunities, talent programmes and engineering recruitment in the defence and aerospace ecosystem.", "ph-briefcase", 29, 1),
    ];

    for c in categories {
        conn.execute(
            "INSERT INTO categories (slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(slug) DO NOTHING",
            params![c.0, c.1, c.2, c.3, c.4, c.5, c.6, c.7, c.8, c.9],
        )?;
    }
    seed_option_b_email_groups(conn)?;
    Ok(())
}

pub fn seed_option_b_email_groups(conn: &Connection) -> rusqlite::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let groups: &[(&str, &str, &str, &str, &str, i32)] = &[
        (
            "general",
            "Genel Duyurular ve Haftalık Bülten",
            "General Announcements & Weekly Newsletter",
            "Haftalık savunma sanayii özeti ve genel editöryal duyurular.",
            "Weekly defence industry digest and general editorial announcements.",
            6,
        ),
        (
            "edergi-releases",
            "Yeni E-Dergi Sayı Bildirimleri",
            "New E-Magazine Issue Alerts",
            "Yeni MSI Dergisi ve MSI TDR dijital sayıları yayımlandığında bildirim alır.",
            "Receives notifications whenever a new MSI Magazine or MSI TDR issue is published.",
            7,
        ),
        (
            "exhibition-alerts",
            "Fuar ve Etkinlik Özel Dosyaları",
            "Exhibition & Event Special Dossiers",
            "IDEF, SAHA EXPO ve uluslararası fuar özel dosyaları hakkında bilgilendirme.",
            "Updates on IDEF, SAHA EXPO and international defence exhibition dossiers.",
            8,
        ),
    ];

    for g in groups {
        conn.execute(
            "INSERT INTO email_groups (
                code, name_tr, name_en, description_tr, description_en, filter_json, static_emails_json,
                requires_marketing_consent, is_system, is_active, sort_order, created_by, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, '{}', '[]', 1, 0, 1, ?6, 'system', ?7)
             ON CONFLICT(code) DO NOTHING",
            params![g.0, g.1, g.2, g.3, g.4, g.5, now],
        )?;
    }
    Ok(())
}

pub fn seed_ssb_tags(conn: &Connection) -> rusqlite::Result<()> {
    let tags: &[(&str, &str, &str, &str, i32)] = &[
        ("kara-araclari", "Kara Araçları", "Land Vehicles & Armored Platforms", "Platformlar", 1),
        ("deniz-araclari", "Deniz Araçları", "Naval Vessels & Submarines", "Platformlar", 2),
        ("hava-araclari", "Hava Araçları", "Combat & Unmanned Aerial Vehicles", "Platformlar", 3),
        ("muhimmatlar-ve-fuze-sistemleri", "Mühimmatlar ve Füze Sistemleri", "Munitions & Missile Systems", "Silah ve Mühimmat", 4),
        ("namlulu-silahlar", "Namlulu Silahlar", "Barrels, Small Arms & Cannons", "Silah ve Mühimmat", 5),
        ("haberlesme-ve-muharebe-destek-sistemleri", "Haberleşme ve Muharebe Destek Sistemleri", "C4ISR & Tactical Communications", "Elektronik ve Bilişim", 6),
        ("radarlar-ve-sensorler", "Radarlar ve Sensörler", "AESA Radars & Electro-Optical Sensors", "Elektronik ve Bilişim", 7),
        ("simulatorler-ve-egitim-sistemleri", "Simülatörler ve Eğitim Sistemleri", "Simulators & Synthetic Training", "Elektronik ve Bilişim", 8),
        ("elektronik-harp-sistemleri", "Elektronik Harp Sistemleri", "Electronic Warfare & Countermeasures", "Elektronik ve Bilişim", 9),
        ("itki-tahrik-ve-guc-sistemleri", "İtki / Tahrik ve Güç Sistemleri", "Propulsion, Turboshaft & Powerpacks", "Alt Sistemler", 10),
        ("hizmetler-danismanlik", "Hizmetler - Danışmanlık", "Services - Defence Consultancy", "Yardımcı Hizmetler", 11),
        ("hizmetler-egitim", "Hizmetler - Eğitim", "Services - Technical & Operational Training", "Yardımcı Hizmetler", 12),
        ("hizmetler-kurumsal-iletisim", "Hizmetler - Kurumsal İletişim", "Services - Defence PR & Exhibition Architecture", "Yardımcı Hizmetler", 13),
        ("hizmetler-kurumsal-catering", "Hizmetler - Kurumsal Catering", "Services - Field & Summit Catering Logistics", "Yardımcı Hizmetler", 14),
    ];

    for t in tags {
        conn.execute(
            "INSERT INTO ssb_tags (code, name_tr, name_en, category_group, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(code) DO NOTHING",
            params![t.0, t.1, t.2, t.3, t.4],
        )?;
    }
    Ok(())
}

/// Idempotently seeds realistic, non-mock defense-sector operational defaults for
/// `recommendation_rules` and `custom_fields` when those tables are empty.
pub fn seed_operational_defaults(conn: &Connection) -> rusqlite::Result<()> {
    let rec_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM recommendation_rules", [], |r| r.get(0))
        .unwrap_or(0);
    if rec_count == 0 {
        let rules: &[(&str, &str, &str, &str, i32, i32)] = &[
            (
                "Hava Araçları & Veri Bağı Derin Analizi",
                "hava-araclari",
                "*",
                "insansiz-hava-araci-veri-baglari-operasyonel-zorluklar-cozum-yontemleri-ve-akson-veri-bagi-sistemi-a2715649",
                20,
                1,
            ),
            (
                "Deniz Kuvvetleri Deniz Sistemleri Önerisi",
                "deniz-araclari",
                "Dz.K.K.lığı",
                "oramiral-tatlioglu-gelecegin-deniz-savaslari-insansiz-araclarla-yapilacaktir",
                25,
                1,
            ),
            (
                "Deniz Platformları 76/62 mm Milli Top Dosyası",
                "deniz-araclari",
                "*",
                "mke-yakin-hava-savunma-ve-deniz-topu-sistemleri",
                15,
                1,
            ),
            (
                "Kara & Helikopter Silah Sistemleri Entegrasyonu",
                "kara-araclari",
                "*",
                "sarsilmaz-ve-tr-mekatronik-20mm-burun-topu-entegrasyonu",
                18,
                1,
            ),
            (
                "Hava Savunma ve Füze Testleri (MSB)",
                "muhimmatlar-ve-fuze-sistemleri",
                "MSB",
                "hisar-a-otonom-jet-motorlu-hedefi-vurdu",
                25,
                1,
            ),
            (
                "Radar & Elektronik Harp Kurumsal Performans Dosyası",
                "radarlar-ve-sensorler",
                "Sektör Kuruluşları",
                "aselsan-2026ya-guclu-basladi-hasilat-343-milyar-tlye-ulasti",
                20,
                1,
            ),
            (
                "Taktik Haberleşme & Sürü İDA Kapak Söyleşisi",
                "haberlesme-ve-muharebe-destek-sistemleri",
                "*",
                "ctech-genel-muduru-dr-cuneyd-firat-f18da8ea",
                18,
                1,
            ),
        ];

        for r in rules {
            let slug_exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM contents WHERE slug = ?1 AND status = 'published' AND is_sponsored = 0)",
                    params![r.3],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if slug_exists {
                conn.execute(
                    "INSERT INTO recommendation_rules
                        (rule_name, source_tag_or_cat, target_institution, recommended_content_slug, priority, is_active)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![r.0, r.1, r.2, r.3, r.4, r.5],
                )?;
            }
        }
    }

    let cf_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM custom_fields", [], |r| r.get(0))
        .unwrap_or(0);
    if cf_count == 0 {
        let fields: &[(&str, &str, &str, &str, &str, i32, i64, &str)] = &[
            (
                "editorial_dossier_code",
                "Editöryal Dosya Kodu",
                "Editorial Dossier Code",
                "text",
                "[]",
                0,
                10,
                "content",
            ),
            (
                "security_clearance_tier",
                "Kaynak Sınıflandırma Düzeyi",
                "Source Classification Tier",
                "select",
                r#"[{"value":"acik_kaynak","label_tr":"Açık Kaynak","label_en":"Open Source"},{"value":"resmi_bulten","label_tr":"Resmî Kurum Bülteni","label_en":"Official Press Release"},{"value":"teknik_analiz","label_tr":"Hakemli Teknik Analiz","label_en":"Peer-Reviewed Technical Analysis"}]"#,
                0,
                20,
                "content",
            ),
            (
                "primary_platform_system",
                "İlgili Ana Platform / Sistem",
                "Primary Platform / System",
                "text",
                "[]",
                0,
                30,
                "content",
            ),
            (
                "ad_contract_reference",
                "Reklam Sözleşme / PO Referansı",
                "Ad Contract / PO Reference",
                "text",
                "[]",
                0,
                10,
                "ad",
            ),
            (
                "ad_sector_classification",
                "Reklamveren Sektör Segmenti",
                "Advertiser Sector Segment",
                "select",
                r#"[{"value":"ana_yuklenici","label_tr":"Ana Yüklenici","label_en":"Prime Contractor"},{"value":"alt_sistem","label_tr":"Alt Sistem Üreticisi","label_en":"Subsystem Supplier"},{"value":"fuar_etkinlik","label_tr":"Fuar & Etkinlik","label_en":"Exhibition & Event"},{"value":"kurum_ici","label_tr":"Platform Duyurusu","label_en":"In-House Announcement"}]"#,
                0,
                20,
                "ad",
            ),
            (
                "user_department_unit",
                "Birim / Daire Başkanlığı",
                "Department / Directorate",
                "text",
                "[]",
                0,
                10,
                "user",
            ),
            (
                "user_protocol_notes",
                "Kurumsal Doğrulama ve Protokol Notu",
                "Institutional Verification & Protocol Notes",
                "textarea",
                "[]",
                0,
                20,
                "user",
            ),
        ];

        for f in fields {
            conn.execute(
                "INSERT INTO custom_fields
                    (field_key, label_tr, label_en, field_type, options_json, is_required, sort_order, applies_to)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(field_key) DO NOTHING",
                params![f.0, f.1, f.2, f.3, f.4, f.5, f.6, f.7],
            )?;
        }
    }

    Ok(())
}

