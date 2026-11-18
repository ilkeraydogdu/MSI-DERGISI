use rusqlite::{params, Connection};

pub fn seed_categories(conn: &Connection) -> rusqlite::Result<()> {
    let categories: &[(&str, Option<&str>, &str, &str, &str, &str, &str, &str, i32, i32)] = &[
        ("haberler", None, "Haberler", "Defense News", "icerikler", "Günlük savunma, havacılık ve denizcilik haber akışı.", "Daily defense, aerospace and maritime news stream.", "ph-newspaper", 1, 1),
        ("ozel-haberler", None, "Özel Haberler", "Exclusive News", "icerikler", "MSI Dergisi editörlerinin sahadan özel istihbarat ve haber dosyaları.", "Exclusive field reports and defense intelligence by MSI editors.", "ph-star-four", 2, 1),
        ("soylesiler", None, "Söyleşiler", "Interviews", "icerikler", "Savunma sanayii liderleri, kuvvet komutanları ve karar vericilerle üst düzey söyleşiler.", "Executive interviews with defense industry leaders and force commanders.", "ph-microphone-stage", 3, 1),
        ("mesajlar", None, "Mesajlar", "Official Messages", "icerikler", "Millî Savunma Bakanlığı, TSK, SSB ve dost ülke makamlarının resmi mesajları.", "Official messages from MoD, TAF, SSB and allied defense authorities.", "ph-seal-check", 4, 1),
        ("tanitici-makaleler", None, "Tanıtıcı Makaleler", "Technical Showcases", "icerikler", "Platform, alt sistem ve ileri teknoloji çözümlerinin teknik mimari incelemeleri.", "Technical architecture reviews of platforms, subsystems and defense technologies.", "ph-cpu", 5, 1),
        ("makaleler-analizler", None, "Makaleler / Analizler", "Articles & Strategic Analysis", "icerikler", "Stratejik doktrin, harp tarihi, savunma ekonomisi ve kavramsal analiz makaleleri.", "Strategic doctrine, defense economics and conceptual operational analyses.", "ph-chart-polar", 6, 1),
        ("advertorial", None, "Advertorial", "Sponsored Content", "icerikler", "Reklam ve sponsorluk mevzuatına uygun kurumsal teknoloji dosyaları.", "Compliant sponsored defense technology dossiers and corporate briefs.", "ph-megaphone", 7, 1),
        ("basyazi", None, "Başyazı", "Editorial", "icerikler", "MSI Dergisi Yayın Kurulu tarafından kaleme alınan aylık stratejik perspektif.", "Monthly strategic perspective penned by the MSI Magazine Editorial Board.", "ph-pen-nib", 8, 1),
        ("ozel-dosyalar", None, "Özel Dosyalar", "Special Dossiers", "gruplandirilmis", "Kritik projeler ve harekât konseptleri üzerine çok boyutlu araştırma dosyaları.", "Multi-dimensional research dossiers on critical defense programs.", "ph-folder-Simple-star", 9, 1),
        ("kapak-dosyalari", None, "Kapak Dosyaları", "Cover Stories", "gruplandirilmis", "Her sayının ana gündemini belirleyen kapsamlı kapak araştırmaları.", "Comprehensive cover investigations setting the monthly defense agenda.", "ph-book-bookmark", 10, 1),
        ("tskdan-haberler", None, "TSK'dan Haberler", "Turkish Armed Forces", "gruplandirilmis", "Türk Silahlı Kuvvetleri tatbikat, envanter kabul ve harekât hazırlık faaliyetleri.", "TAF exercises, inventory inductions and operational readiness updates.", "ph-shield-chevron", 11, 1),
        ("dergi-icerikleri", None, "Dergi İçerikleri", "Magazine Features", "gruplandirilmis", "Basılı ve E-Dergi sayılarında yayımlanan hakemli ve editoryal içerikler.", "Editorial features published across print and digital magazine issues.", "ph-books", 12, 1),
        ("etkinlikler", None, "Etkinlikler", "Defense Events", "gruplandirilmis", "IDEF, SAHA EXPO, MSPO, DSEI, Eurosatory ve küresel savunma zirveleri.", "IDEF, SAHA EXPO, MSPO, DSEI, Eurosatory and global defense summits.", "ph-globe-hemisphere-east", 13, 1),
        ("etkinlik-ozel-sayilari", Some("etkinlikler"), "Etkinlik Özel Sayıları", "Event Special Issues", "gruplandirilmis", "Uluslararası fuar ve zirveler için hazırlanan çift dilli özel sayılar.", "Bilingual special editions prepared for international defense exhibitions.", "ph-medal", 14, 1),
        ("etkinlik-sonrasi-ozel-dosyalar", Some("etkinlikler"), "Etkinlik Sonrası Özel Dosyalar", "Post-Event Dossiers", "gruplandirilmis", "Fuarlarda imzalanan sözleşmeler, lansmanlar ve saha analizleri.", "Post-exhibition contract analyses, product launches and field assessments.", "ph-Note-pencil", 15, 1),
        ("e-dergi", None, "E-Dergi", "E-Magazine", "abonelik", "DRM korumalı ve dinamik filigranlı yüksek çözünürlüklü dijital dergi kütüphanesi.", "DRM-protected, dynamically watermarked high-resolution digital magazine library.", "ph-book-open-text", 16, 1),
        ("abonelik-ve-satis", None, "Abonelik ve Satış", "Subscriptions & Store", "abonelik", "Kurumsal/Bireysel abonelik paketleri, prestij kitapları ve RBF fotoğraf lisansları.", "Corporate/Individual subscription tiers, prestige books and RBF photo licenses.", "ph-shopping-bag-open", 17, 1),
        ("hakkimizda", None, "Hakkımızda", "About Us", "kurumsal", "UMSA Prodüksiyon, MSI Dergisi ve MSI TDR kurumsal yapısı.", "Corporate structure of UMSA Production, MSI Magazine and MSI TDR.", "ph-buildings", 18, 1),
        ("kunye", Some("hakkimizda"), "Künye", "Masthead", "kurumsal", "Yayın kurulu, danışma kurulu, editöryal kadro ve yasal yayın bilgileri.", "Editorial board, advisory board, staff and legal publication details.", "ph-identification-card", 19, 1),
        ("biz-kimiz", Some("hakkimizda"), "Biz Kimiz", "Who We Are", "kurumsal", "2005'ten bugüne bağımsız savunma yayıncılığı ilkelerimiz.", "Our independent defense journalism principles since 2005.", "ph-users-three", 20, 1),
        ("yolculugumuz", Some("hakkimizda"), "Yolculuğumuz", "Our Journey", "kurumsal", "MSI Dergisi'nin 20 yılı aşan yayıncılık ve uluslararası fuar kilometre taşları.", "Over 20 years of publishing and international exhibition milestones.", "ph-compass", 21, 1),
        ("cozum-ortaklarimiz", Some("hakkimizda"), "Çözüm Ortaklarımız", "Solution Partners", "kurumsal", "Ulusal ve uluslararası sektörel iş birliklerimiz ve medya ortaklıklarımız.", "National and international defense media partnerships and alliances.", "ph-handshake", 22, 1),
        ("amac-magazine", Some("hakkimizda"), "AMAC Magazine", "AMAC Magazine", "kurumsal", "Ankara Askerî Ataşeler Birliği (AMAC) resmi yayın projesi.", "Official publication project for the Ankara Military Attaché Corps (AMAC).", "ph-flag-banner", 23, 1),
        ("kasabaworks", Some("hakkimizda"), "KasabaWorks", "KasabaWorks Agency", "kurumsal", "Savunma sanayii odaklı kurumsal iletişim, tasarım ve kreatif prodüksiyon ajansımız.", "Defense-focused corporate communications, design and creative production agency.", "ph-paint-brush-broad", 24, 1),
        ("iletisim", Some("hakkimizda"), "İletişim", "Contact", "kurumsal", "Ankara merkez ofis, reklam rezervasyon ve kurumsal abonelik iletişim kanalları.", "Ankara headquarters, ad reservation and corporate subscription contact channels.", "ph-envelope-simple-open", 25, 1),
        ("savunma-sanayii-tarihi", None, "Savunma Sanayii Tarihi", "Defense Industry History", "gelecek-faz", "Türk ve dünya savunma sanayii kronolojisi ve dokümantasyon arşivi.", "Chronology and archival documentation of Turkish and global defense history.", "ph-hourglass-high", 26, 1),
        ("yazarlar", None, "Yazarlar", "Columnists & Authors", "gelecek-faz", "Akademisyenler, emekli komutanlar ve savunma analistleri dizini.", "Directory of academics, retired commanders and defense analysts.", "ph-feather", 27, 1),
        ("ogrenci-kulupleri", None, "Öğrenci Kulüpleri", "University Defense Clubs", "gelecek-faz", "Üniversite savunma teknolojileri ve havacılık toplulukları ekosistemi.", "University defense technologies and aerospace societies ecosystem.", "ph-graduation-cap", 28, 1),
    ];

    for c in categories {
        conn.execute(
            "INSERT OR IGNORE INTO categories (slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![c.0, c.1, c.2, c.3, c.4, c.5, c.6, c.7, c.8, c.9],
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
        ("hizmetler-danismanlik", "Hizmetler - Danışmanlık", "Services - Defense Consultancy", "Yardımcı Hizmetler", 11),
        ("hizmetler-egitim", "Hizmetler - Eğitim", "Services - Technical & Operational Training", "Yardımcı Hizmetler", 12),
        ("hizmetler-kurumsal-iletisim", "Hizmetler - Kurumsal İletişim", "Services - Defense PR & Exhibition Architecture", "Yardımcı Hizmetler", 13),
        ("hizmetler-kurumsal-catering", "Hizmetler - Kurumsal Catering", "Services - Field & Summit Catering Logistics", "Yardımcı Hizmetler", 14),
    ];

    for t in tags {
        conn.execute(
            "INSERT OR IGNORE INTO ssb_tags (code, name_tr, name_en, category_group, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![t.0, t.1, t.2, t.3, t.4],
        )?;
    }
    Ok(())
}

