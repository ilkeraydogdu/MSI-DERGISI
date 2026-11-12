use crate::models::{MagazinePageSpread, SubscriptionPlan};
use rusqlite::{params, Connection};

pub fn seed_initial_data(conn: &Connection) -> rusqlite::Result<()> {
    seed_categories(conn)?;
    seed_ssb_tags(conn)?;
    seed_contents(conn)?;
    seed_magazine_issues(conn)?;
    seed_users_and_verifications(conn)?;
    seed_ad_campaigns(conn)?;
    seed_recommendation_rules(conn)?;
    seed_subscription_plans_and_commerce(conn)?;
    seed_analytics_and_operations(conn)?;
    seed_site_settings_and_notifications(conn)?;
    seed_live_msi_harvested_data(conn)?;
    Ok(())
}

fn seed_categories(conn: &Connection) -> rusqlite::Result<()> {
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

fn seed_ssb_tags(conn: &Connection) -> rusqlite::Result<()> {
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


fn seed_contents(conn: &Connection) -> rusqlite::Result<()> {
    struct SeedArticle<'a> {
        slug: &'a str,
        title_tr: &'a str,
        title_en: &'a str,
        summary_tr: &'a str,
        summary_en: &'a str,
        body_tr: &'a str,
        body_en: &'a str,
        content_type: &'a str,
        source_group: &'a str,
        cover_image: &'a str,
        author_name: &'a str,
        author_title: &'a str,
        issue_number: Option<i32>,
        published_at: &'a str,
        read_time_min: i32,
        visibility_level: i32,
        single_price_try: f64,
        is_sponsored: i32,
        sponsor_brand: Option<&'a str>,
        sponsor_legal_label: Option<&'a str>,
        is_featured: i32,
        video_url: Option<&'a str>,
        view_count: i64,
        true_read_count: i64,
        placements: &'a [&'a str],
        ssb_tags: &'a [&'a str],
    }

    let articles: &[SeedArticle] = &[
        SeedArticle {
            slug: "kaan-programinda-mevcut-durum-ve-yol-haritasi",
            title_tr: "Milli Muharip Uçak KAAN: 5. Nesil Aviyonik ve Seri Üretim Mimarisi",
            title_en: "National Combat Aircraft KAAN: 5th Gen Avionics and Serial Production Architecture",
            summary_tr: "TUSAŞ ana yükleniciliğinde yürütülen KAAN programında P0 ve P1 prototiplerinin uçuş test zarfı genişletilirken, ASELSAN MURAD AESA radar ve bütünleşik RF süiti entegrasyonunda kritik eşik aşıldı.",
            summary_en: "Flight test envelopes for KAAN P0 and P1 prototypes expand under TUSAS prime contractorship as ASELSAN MURAD AESA radar and integrated RF suite reach critical milestones.",
            body_tr: "<p>Türk savunma ve havacılık sanayiinin en büyük teknoloji hamlesi olan Milli Muharip Uçak <strong>KAAN</strong> programında test ve kalifikasyon faaliyetleri planlanan takvim doğrultusunda hızla ilerliyor. İlk uçuşunun ardından aerodinamik, uçuş kontrol kanunları ve aviyonik doğrulama sortilerine devam eden platformda, 2028-2030 dönemi Blok-10 teslimatları için alt sistem olgunlaştırma çalışmaları derinleştirildi.</p><p>Beşinci nesil çok rollü muharip uçak gereksinimleri kapsamında geliştirilen <strong>Bütünleşik RF Sistemi (BÜRFİS)</strong>, 360 derece elektro-optik dağıtık açıklık sistemi (DAS) ve kızılötesi arama-takip (IRST) sensör füzyonu, pilotun durumsal farkındalığını en üst seviyeye taşıyor. Yüksek hızlı fiber optik veri yolu üzerinde çalışan milli görev bilgisayarları, elektronik harp ve radar verilerini gerçek zamanlı birleştirerek ağ merkezli harekât kabiliyeti sunuyor.</p><p>Seri üretim fazında kompozit gövde panelleri, düşük görünürlük (stealth) kaplama teknolojileri ve dahili silah yuvası mekanizmaları yerli yan sanayi ve kümelenme ekosisteminin yüksek katkısıyla üretiliyor.</p>",
            body_en: "<p>Testing and qualification activities in the <strong>KAAN</strong> National Combat Aircraft program are advancing rapidly on schedule. Following its maiden flight, aerodynamic, flight control law, and avionics verification sorties continue while subsystem maturation deepens for Block-10 deliveries.</p><p>Developed to meet 5th-generation multirole fighter requirements, the <strong>Integrated RF System (BURFIS)</strong>, 360-degree Distributed Aperture System (DAS), and IRST sensor fusion elevate pilot situational awareness. Indigenous mission computers operating over high-speed fiber-optic data buses fuse EW and radar tracks in real time.</p>",
            content_type: "ozel-haberler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/kaan.webp",
            author_name: "MSI Dergisi Yayın Kurulu",
            author_title: "Havacılık ve Strateji Masası",
            issue_number: Some(280),
            published_at: "2026-09-15",
            read_time_min: 8,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: Some("images/tour.mp4"),
            view_count: 14820,
            true_read_count: 11240,
            placements: &["ozel-haberler", "kapak-dosyalari", "dergi-icerikleri", "haberler"],
            ssb_tags: &["hava-araclari", "radarlar-ve-sensorler", "itki-tahrik-ve-guc-sistemleri"],
        },
        SeedArticle {
            slug: "akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu",
            title_tr: "AKINCI TİHA ve İHA-300: Süpersonik Balistik Füze ile Stratejik Menzil",
            title_en: "AKINCI UCAV & IHA-300: Strategic Standoff Strike with Supersonic Ballistic Missile",
            summary_tr: "Baykar tarafından geliştirilen AKINCI TİHA, ROKETSAN üretimi İHA-300 havadan karaya süpersonik balistik füzesiyle 250 kilometre menzildeki su üstü hedefini tam isabetle imha etti.",
            summary_en: "Baykar's AKINCI UCAV accurately neutralized a maritime target at 250 km range using ROKETSAN's IHA-300 air-launched supersonic ballistic missile.",
            body_tr: "<p>Sinop Test Merkezi açıklarında icra edilen tarihi atış testinde <strong>Bayraktar AKINCI TİHA</strong>, yüksek irtifadan ateşlediği <strong>İHA-300</strong> süpersonik balistik füzesiyle Karadeniz'deki hedefi tam isabetle vurdu. Yüksek hız ve yüksek irtifa avantajını kullanan İHA-300, düşman hava savunma şemsiyesinin (A2/AD) çok dışından stratejik hedeflerin imha edilmesine olanak tanıyor.</p><p>KकेS/ANS ve IIR/Lazer arayıcı başlık opsiyonlarıyla donatılan mühimmat, otonom görev planlama ve veri bağı üzerinden hedef güncelleme yeteneğiyle insansız hava araçlarını stratejik taarruz platformuna dönüştürüyor. Bu kabiliyet, Hv.K.K.lığı ve Dz.K.K.lığı müşterek harekât konseptlerinde oyun değiştirici bir çarpan etkisi yaratıyor.</p>",
            body_en: "<p>During the firing trial off the Sinop Test Center, <strong>Bayraktar AKINCI UCAV</strong> launched the <strong>IHA-300</strong> supersonic ballistic missile from high altitude, striking its Black Sea target with pinpoint precision beyond hostile A2/AD envelopes.</p><p>Equipped with GNSS/INS and imaging infrared/laser seeker options, the munition transforms unmanned aerial platforms into strategic standoff strike assets for joint air-naval operations.</p>",
            content_type: "haberler",
            source_group: "gunluk-haber",
            cover_image: "images/msi/akinci-iha300.webp",
            author_name: "Emre Karakoç",
            author_title: "Mühimmat ve İHA Sistemleri Editörü",
            issue_number: Some(280),
            published_at: "2026-09-14",
            read_time_min: 6,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: Some("images/tour.mp4"),
            view_count: 12390,
            true_read_count: 9640,
            placements: &["haberler", "ozel-dosyalar", "dergi-icerikleri"],
            ssb_tags: &["hava-araclari", "muhimmatlar-ve-fuze-sistemleri"],
        },
        SeedArticle {
            slug: "ctech-suru-ida-hibrit-haberlesme-konsepti",
            title_tr: "CTech’ten Sürü İnsansız Deniz Araçları (İDA) için Hibrit Haberleşme Konsepti",
            title_en: "CTech Unveils Hybrid Communications Concept for Swarm Unmanned Surface Vessels (USVs)",
            summary_tr: "Elektronik harp altında kesintisiz komuta-kontrol sağlayan SATCOM + LOS + Taktik Mesh hibrit haberleşme mimarisi, Mavi Vatan'da sürü İDA harekâtlarının omurgasını oluşturuyor.",
            summary_en: "Combining SATCOM, Line-of-Sight and Tactical Mesh networking, CTech's hybrid architecture ensures jam-resistant C2 for swarm USV operations.",
            body_tr: "<p>İnsansız Deniz Araçlarının (İDA) sürü halinde icra edeceği keşif, gözetleme ve su üstü harbi görevlerinde en kritik gereksinim, yoğun elektronik karıştırma (jamming) altında dahi kesintisiz veri bağıdır. <strong>CTech</strong> mühendisleri tarafından geliştirilen yeni nesil <strong>Hibrit Haberleşme Terminali</strong>, görüş hattı (LOS) genişbant veri bağı, uydu haberleşmesi (SATCOM) ve sürü içi dinamik MANET/Mesh ağ yapısını tek bir akıllı yönlendirme ünitesinde birleştiriyor.</p><p>Birincil kanalda karıştırma veya sinyal kaybı algılandığında milisaniyeler içinde alternatif frekans ve uydu katmanına geçiş yapan sistem, sürü lideri İDA ile su üstü komuta gemisi ve sahil kontrol istasyonu arasında kriptolu yüksek hızlı video ve telemetri aktarımı sağlıyor. Deniz Kuvvetleri Komutanlığı ve SSB koordinasyonunda yürütülen saha testlerinde mimarinin çoklu düğüm dayanıklılığı başarıyla doğrulandı.</p>",
            body_en: "<p>Uninterrupted data links under heavy electronic warfare are vital for swarm Unmanned Surface Vessel (USV) operations. <strong>CTech</strong>'s new <strong>Hybrid Communications Terminal</strong> unifies Line-of-Sight wideband data links, SATCOM, and dynamic intra-swarm MANET/Mesh routing into a single intelligent unit.</p><p>Automatically switching across frequency and satellite layers within milliseconds upon jamming detection, the system guarantees encrypted multi-node telemetry and video transmission between swarm USVs and naval command vessels.</p>",
            content_type: "tanitici-makaleler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/ctech-ida.webp",
            author_name: "Dr. Selim Arslan",
            author_title: "C4ISR ve Haberleşme Sistemleri Analisti",
            issue_number: Some(280),
            published_at: "2026-09-13",
            read_time_min: 7,
            visibility_level: 1,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: None,
            view_count: 9420,
            true_read_count: 7810,
            placements: &["tanitici-makaleler", "ozel-dosyalar", "dergi-icerikleri"],
            ssb_tags: &["deniz-araclari", "haberlesme-ve-muharebe-destek-sistemleri", "elektronik-harp-sistemleri"],
        },
        SeedArticle {
            slug: "genelkurmay-baskani-idef-ozel-sayisi-mesaji",
            title_tr: "Genelkurmay Başkanlığı Özel Sayı Mesajı: Müşterek Harekât ve Milli Teknoloji Vizyonu",
            title_en: "Chief of General Staff Special Issue Message: Joint Operations and National Defense Technology Vision",
            summary_tr: "Türk Silahlı Kuvvetleri'nin kara, deniz, hava, uzay ve siber vatan boyutlarında yürüttüğü müşterek harekât kabiliyetinde yerli ve milli savunma sanayii ekosisteminin stratejik rolü.",
            summary_en: "Strategic role of the indigenous defense industry ecosystem in the Turkish Armed Forces' multi-domain joint operational capabilities across land, sea, air, space, and cyber domains.",
            body_tr: "<p>Türk Silahlı Kuvvetlerimiz; değişen küresel güvenlik ortamında caydırıcılığını en üst düzeyde tutmak amacıyla kara, deniz, hava, uzay ve siber harekât alanlarında tam bütünleşik bir müşterek kuvvet yapısıyla görev yapmaktadır. Bu kapsamda <strong>MSI Dergisi Özel Sayısı</strong> vesilesiyle savunma sanayiimizin tüm paydaşlarını ve mühendislerimizi gönülden tebrik ediyorum.</p><p>Modern harp sahasında elde edilen başarılar, sahadaki kahraman personelimizin harekât tecrübesi ile tedarik makamlarımız ve savunma sanayii kuruluşlarımızın (MSB, SSB, vakıf ve özel sektör firmalarımızın) yakın eşgüdümünün eseridir. İnsansız sistemlerden hava savunma şemsiyesine, elektronik harpten ağ destekli komuta kontrol sistemlerine kadar geliştirilen her milli unsur, TSK'nın harekât bağımsızlığını perçinlemektedir.</p><p><em>Not: Bu içerik Single Source of Truth (Tek Merkezden Çoklu Gösterim) mimarisi uyarınca veritabanında tek bir kayıt olarak tutulmakta; aynı anda <strong>Mesajlar</strong>, <strong>Dergi İçerikleri</strong>, <strong>TSK'dan Haberler</strong> ve <strong>Etkinlik Özel Sayıları</strong> bölümlerinde dinamik olarak referanslanmaktadır.</em></p>",
            body_en: "<p>The Turkish Armed Forces operate with a fully integrated multi-domain joint force structure across land, naval, air, space, and cyber theaters to maintain maximum deterrence. On the occasion of this <strong>MSI Magazine Special Issue</strong>, I commend all stakeholders and engineers of our defense industry ecosystem.</p><p><em>Note: Powered by the Single Source of Truth architecture, this single database record is simultaneously surfaced under Official Messages, Magazine Features, Turkish Armed Forces News, and Event Special Issues.</em></p>",
            content_type: "mesajlar",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/saha-kara.webp",
            author_name: "Genelkurmay Başkanlığı",
            author_title: "Resmi Özel Sayı Mesajı",
            issue_number: Some(280),
            published_at: "2026-09-12",
            read_time_min: 4,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: None,
            view_count: 11540,
            true_read_count: 9890,
            placements: &["mesajlar", "dergi-icerikleri", "tskdan-haberler", "etkinlik-ozel-sayilari"],
            ssb_tags: &["kara-araclari", "deniz-araclari", "hava-araclari", "haberlesme-ve-muharebe-destek-sistemleri"],
        },
        SeedArticle {
            slug: "dr-m-cuneyd-firat-ctech-kapak-soylesisi",
            title_tr: "Dr. M. Cüneyd Fırat: \"Milli Haberleşme Çözümlerimizle Mavi Vatan ve Gökyüzünde Kuralları Değiştiriyoruz\"",
            title_en: "Dr. M. Cuneyd Firat: \"Redefining the Rules in Blue Homeland and Skies with National SatCom Solutions\"",
            summary_tr: "CTech Genel Müdürü Dr. M. Cüneyd Fırat ile ANKA, AKSUNGUR, HÜRJET ve İDA platformlarına entegre edilen uydu haberleşme terminalleri ve ihracat başarıları üzerine özel kapak söyleşisi.",
            summary_en: "Cover interview with CTech General Manager Dr. M. Cuneyd Firat on satellite communication terminals integrated onto ANKA, AKSUNGUR, HURJET and USV platforms.",
            body_tr: "<p><strong>MSI Dergisi 280. Sayı Kapak Söyleşisi</strong> kapsamında konuğumuz olan CTech Genel Müdürü <strong>Dr. M. Cüneyd Fırat</strong>, Türkiye'nin görüş ötesi (BLOS) uydu haberleşmesi ve güvenli veri bağı teknolojilerinde ulaştığı küresel rekabet gücünü değerlendirdi.</p><blockquote>\"Bugün insansız hava ve deniz araçlarımız binlerce kilometre ötede görev yapabiliyorsa, bunun arkasında elektronik harbe dayanıklı, milli dalga şekline sahip Ku-Bant ve Ka-Bant SATCOM terminallerimiz var. NATO standartlarındaki ürünlerimiz dost ve müttefik ülkelerin envanterine giriyor.\"</blockquote><p>Söyleşide ayrıca alçak irtifa takım uyduları (LEO) ile entegre yeni nesil düz panel (flat-panel) faz dizili anten projeleri, Sürü İDA haberleşme mimarisi ve genç mühendislere sunulan Ar-Ge ekosistemi tüm boyutlarıyla ele alındı.</p>",
            body_en: "<p>In our <strong>Issue 280 Cover Interview</strong>, CTech General Manager <strong>Dr. M. Cuneyd Firat</strong> outlines Türkiye's global competitiveness in Beyond-Line-of-Sight (BLOS) satellite communications and secure tactical data links.</p><blockquote>\"Our Ku-Band and Ka-Band SATCOM terminals with indigenous anti-jam waveforms enable UAVs and USVs to operate thousands of kilometers away.\"</blockquote>",
            content_type: "soylesiler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/ctech-roportaj.webp",
            author_name: "Ümit Bayraktar",
            author_title: "MSI Dergisi Genel Yayın Yönetmeni",
            issue_number: Some(280),
            published_at: "2026-09-11",
            read_time_min: 11,
            visibility_level: 1,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: Some("images/tour.mp4"),
            view_count: 10850,
            true_read_count: 8920,
            placements: &["soylesiler", "kapak-dosyalari", "dergi-icerikleri"],
            ssb_tags: &["haberlesme-ve-muharebe-destek-sistemleri", "hava-araclari", "deniz-araclari"],
        },
        SeedArticle {
            slug: "aselsan-aesa-radar-ve-elektronik-harp-mimarisi-advertorial",
            title_tr: "ASELSAN GaN Temelli AESA Radar ve Bütünleşik Elektronik Harp Mimarisi",
            title_en: "ASELSAN GaN-Based AESA Radar and Integrated Electronic Warfare Architecture",
            summary_tr: "MURAD Burun Radarı, CENK Deniz AESA Radarı ve KORAL Yeni Nesil Elektronik Harp Sistemlerinde Galyum Nitrür (GaN) yarı iletken teknolojisiyle elde edilen menzil ve spektrum üstünlüğü.",
            summary_en: "Range and electromagnetic spectrum superiority achieved through Gallium Nitride (GaN) semiconductor technology in MURAD, CENK and New-Gen KORAL EW systems.",
            body_tr: "<p><strong>[SPONSORLU İÇERİK • İLANDIR]</strong> Elektromanyetik spektrumda üstünlük, modern muharebe sahasının kazanılmasında belirleyici unsurdur. <strong>ASELSAN</strong> tarafından milli imkanlarla tasarlanan ve üretilen Galyum Nitrür (GaN) tabanlı Gönderme/Alma (T/R) modülleri, hava, kara ve deniz platformlarına entegre edilen aktif elektronik taramalı dizi (AESA) radarların kalbini oluşturmaktadır.</p><p>F-16 Özgür, AKINCI, KIZILELMA ve Milli Muharip Uçak KAAN platformları için ölçeklendirilen <strong>MURAD AESA Burun Radarı</strong>; aynı anda çoklu hava-hava ve hava-yer angajmanı, sentetik açıklıklı radar (SAR) görüntüleme ve elektronik taarruz kabiliyetlerini tek bir antende birleştirmektedir. Deniz platformlarında ise İSTİF sınıfı fırkateynlerde görev yapan <strong>CENK 4B AESA Radarı</strong> yüzlerce hava ve su üstü hedefini yüksek yenileme hızıyla takip etmektedir.</p><p><em>Not: Müşteri şartnamesi uyarınca bu Sponsorlu İçerik (Advertorial) sayfasında \"İlginizi Çekebilir\" içerik önerileri otomatik olarak devre dışı bırakılmış ve rakip savunma sanayii firmalarının banner reklamları engellenmiştir.</em></p>",
            body_en: "<p><strong>[SPONSORED CONTENT • ADVERTORIAL]</strong> Superiority in the electromagnetic spectrum defines victory on the modern battlefield. <strong>ASELSAN</strong>'s indigenous Gallium Nitride (GaN) Transmit/Receive modules power AESA radars across air, land, and naval platforms including MURAD and CENK 4D.</p><p><em>Note: Per specification rules, related article recommendations and competing defense OEM banners are automatically suppressed on this sponsored content page.</em></p>",
            content_type: "advertorial",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/skyarx.webp",
            author_name: "ASELSAN Kurumsal İletişim & Radar Sistemleri Grubu",
            author_title: "Sponsorlu Teknoloji Dosyası",
            issue_number: Some(280),
            published_at: "2026-09-10",
            read_time_min: 6,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 1,
            sponsor_brand: Some("ASELSAN"),
            sponsor_legal_label: Some("SPONSORLU İÇERİK • İLANDIR"),
            is_featured: 1,
            video_url: None,
            view_count: 8740,
            true_read_count: 7190,
            placements: &["advertorial", "tanitici-makaleler", "dergi-icerikleri"],
            ssb_tags: &["radarlar-ve-sensorler", "elektronik-harp-sistemleri", "hava-araclari"],
        },
        SeedArticle {
            slug: "nato-hava-savunma-sistemlerinin-gelecegi-analiz",
            title_tr: "NATO Entegre Hava ve Füze Savunma Mimarisi: Katmanlı Kubbe ve Hipersonik Tehdit Analizi",
            title_en: "Future of NATO Integrated Air and Missile Defense: Multi-Layered Shield & Hypersonic Threat Analysis",
            summary_tr: "NATO'da üst düzey görev almış uluslararası hava savunma uzmanlarının kaleme aldığı özel stratejik rapor: Çelik Kubbe, ESSI ve sensör-atıcı füzyonunda 2035 doktrini.",
            summary_en: "Exclusive strategic analysis penned by former NATO air defense directors: Steel Dome, ESSI, and sensor-to-shooter fusion doctrine through 2035.",
            body_tr: "<p>Balistik füzeler, seyir füzeleri, kamikaze İHA sürüleri ve manevra yapan hipersonik süzülme araçlarının (HGV) eşzamanlı kullanıldığı doyurma saldırıları, geleneksel hava savunma doktrinlerini kökten değiştirmiştir. Bu özel analiz dosyasında, NATO Entegre Hava ve Füze Savunma Sistemi (IAMD) ile Türkiye'nin <strong>Çelik Kubbe</strong> katmanlı hava savunma mimarisinin birlikte çalışabilirlik standartları incelenmektedir.</p><p>Alçak irtifada KORKUT, SUNGUR ve GÖKBERK yönlendirilmiş enerji silahlarından; orta irtifada HİSAR-A+ / HİSAR-O+ ve uzun menzilde SİPER Ürün-1 / Ürün-2 sistemlerine uzanan katmanlı ağın yapay zekâ destekli <strong>HAKİM Hava Komuta Kontrol Sistemi</strong> altında birleştirilmesi, angajman reaksiyon süresini saniyelerin altına indirmektedir. Raporun devamında alt yüklenici KOBİ'ler için sensör soğutma, RF konnektör ve komuta yazılımı tedarik fırsatları detaylandırılmaktadır.</p>",
            body_en: "<p>Saturation attacks combining ballistic missiles, cruise missiles, loitering munition swarms, and hypersonic glide vehicles have transformed air defense doctrine. This premium strategic analysis examines NATO IAMD interoperability and Türkiye's multi-layered <strong>Steel Dome</strong> architecture integrating KORKUT, HISAR, SIPER, and HAKIM C2.</p>",
            content_type: "makaleler-analizler",
            source_group: "sadece-online",
            cover_image: "images/msi/mke-pgz.webp",
            author_name: "Tuoğg. (E) Hans-Dieter Klein & MSI Strateji Masası",
            author_title: "NATO IAMD Kıdemli Danışmanı",
            issue_number: Some(280),
            published_at: "2026-09-09",
            read_time_min: 14,
            visibility_level: 2,
            single_price_try: 450.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: None,
            view_count: 6890,
            true_read_count: 5910,
            placements: &["makaleler-analizler", "ozel-dosyalar"],
            ssb_tags: &["muhimmatlar-ve-fuze-sistemleri", "radarlar-ve-sensorler", "haberlesme-ve-muharebe-destek-sistemleri"],
        },
        SeedArticle {
            slug: "savunma-ve-havacilik-kobilerine-yonelik-yenilikci-destek-mekanizmalari",
            title_tr: "Savunma ve Havacılık KOBİ'lerine Yönelik Yenilikçi Destek Mekanizmaları: KOSGEB, TÜBİTAK TEYDEB ve Küresel Örnekler",
            title_en: "Innovative Support Mechanisms for Defense & Aerospace SMEs: KOSGEB, TUBITAK TEYDEB and Global Benchmarks",
            summary_tr: "Kamu kurumları, kümelenmeler (SAHA İstanbul, OSSA, تSSİ) ve yan sanayi firmaları için hazırlanan kapsamlı başvuru ve teşvik analizi.",
            summary_en: "Comprehensive policy and incentive analysis prepared for public agencies, defense clusters, and Tier-2/3 defense SMEs.",
            body_tr: "<p>Savunma sanayii ekosisteminin sürdürülebilirliği, ana yüklenicilerin (ASELSAN, TUSAŞ, ROKETSAN, HAVELSAN, STM, BMC, ASFAT) altında faaliyet gösteren 2.000'i aşkın nitelikli KOBİ'nin teknolojik derinliğine bağlıdır. Bu özel araştırma dosyasında <strong>SSB Yerlileştirme ve Millileştirme Programları</strong>, <strong>KOSGEB Stratejik Ürün Destekleri</strong> ve <strong>TÜBİTAK TEYDEB</strong> çağrılarının KOBİ bilançolarına etkisi karşılaştırmalı olarak analiz edilmiştir.</p><p>Ayrıca ABD SBIR/STTR, Avrupa Savunma Fonu (EDF) ve Güney Kore DAPA KOBİ hızlandırma modelleri incelenerek; kalifikasyon, AS9100/AQAP sertifikasyonu ve ihracat konsorsiyumu desteklerinde yeni nesil finansman önerileri sunulmaktadır.</p>",
            body_en: "<p>The sustainability of the defense ecosystem relies on the technological depth of over 2,000 specialized SMEs supporting prime contractors. This analysis benchmarks SSB localization grants, KOSGEB strategic support, and TUBITAK TEYDEB programs alongside global SBIR and EDF models.</p>",
            content_type: "makaleler-analizler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/sedec.webp",
            author_name: "Prof. Dr. Kerem Soylu",
            author_title: "Savunma Ekonomisi ve Sanayi Politikaları Uzmanı",
            issue_number: Some(280),
            published_at: "2026-09-08",
            read_time_min: 12,
            visibility_level: 2,
            single_price_try: 350.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 0,
            video_url: None,
            view_count: 5430,
            true_read_count: 4620,
            placements: &["makaleler-analizler", "soylesiler", "ozel-dosyalar"],
            ssb_tags: &["hizmetler-danismanlik", "hizmetler-egitim"],
        },
        SeedArticle {
            slug: "mke-yakin-hava-savunma-ve-deniz-topu-sistemleri",
            title_tr: "MKE'den Deniz Platformları İçin 76/62 mm Milli Deniz Topu ve Yakın Hava Savunma Çözümü",
            title_en: "MKE Delivers 76/62 mm National Naval Gun and Close-In Weapon System for Naval Platforms",
            summary_tr: "MKE tarafından geliştirilen 76/62 mm Milli Deniz Topu ve GÖKĐZ yakın hava savunma sistemi, MİLGEM korvetleri ve açık deniz karakol gemilerinde tam operasyonel kapasiteye ulaştı.",
            summary_en: "MKE's indigenous 76/62 mm Naval Gun and close-in weapon solutions reach full operational capability aboard MILGEM corvettes and offshore patrol vessels.",
            body_tr: "<p>Makine ve Kimya Endüstrisi (MKE), su üstü muharip gemilerin en kritik ateş destek ve hava savunma unsurlarından biri olan <strong>76/62 mm Milli Deniz Topu</strong>'nun seri üretim ve gemi entegrasyon faaliyetlerini hızlandırdı. Dakikada 80 atım hızına ulaşabilen sistem, akıllı parçacıklı mühimmat kullanımıyla gemisavar füzelere ve asimetrik su üstü tehditlerine karşı yüksek etkinlik sağlıyor.</p><p>Deniz Kuvvetleri Komutanlığı envanterindeki platformlarda yurt dışı bağımlılığı sona erdiren proje, aynı zamanda dost ülke donanmalarına yönelik ihracat paketlerinde de yerini aldı.</p>",
            body_en: "<p>Mechanical and Chemical Industry Corporation (MKE) has accelerated serial production and shipboard integration of its <strong>76/62 mm National Naval Gun</strong>, capable of 80 rounds per minute with programmable airburst ammunition against anti-ship missiles and asymmetric surface threats.</p>",
            content_type: "haberler",
            source_group: "gunluk-haber",
            cover_image: "images/msi/mke-pgz.webp",
            author_name: "MSI Haber Merkezi",
            author_title: "Deniz ve Silah Sistemleri Masası",
            issue_number: Some(280),
            published_at: "2026-09-07",
            read_time_min: 5,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 0,
            video_url: None,
            view_count: 7820,
            true_read_count: 6150,
            placements: &["haberler", "tskdan-haberler"],
            ssb_tags: &["namlulu-silahlar", "deniz-araclari", "muhimmatlar-ve-fuze-sistemleri"],
        },
        SeedArticle {
            slug: "polonya-mspo-fuarinda-turk-savunma-sanayii-cikarmasi",
            title_tr: "Polonya MSPO Fuarı'nda Türk Savunma Sanayii Çıkarması: Avrupa Güvenlik Mimarisinde Yeni İş Birlikleri",
            title_en: "Turkish Defense Industry Showcases Strategic Capabilities at MSPO Poland",
            summary_tr: "Kielce'de düzenlenen MSPO Uluslararası Savunma Sanayii Fuarı'nda SSB koordinasyonunda milli pavilyon kuran Türk firmaları, NATO doğu kanadı için kritik anlaşmalara imza attı.",
            summary_en: "Under SSB coordination at MSPO Kielce, Turkish defense companies signed strategic partnership agreements reinforcing NATO's eastern flank.",
            body_tr: "<p>Avrupa'nın en önemli savunma sanayii buluşmalarından biri olan <strong>MSPO Fuarı</strong>'nda Türk savunma ve havacılık sanayii, Lider Ülke düzeyindeki katılımıyla gövde gösterisi yaptı. Zırhlı kara araçlarından taktik İHA'lara, elektronik harp podlarından komuta kontrol yazılımlarına kadar geniş bir ürün yelpazesi Polonya ve Baltık ülkelerinin tedarik heyetleriyle buluştu.</p><p>Fuar sonrası hazırladığımız bu özel dosyada, imzalanan mutabakat muhtıraları ve Avrupa savunma tedarik zincirine entegrasyon fırsatları sahadan özel röportajlarla aktarılıyor.</p>",
            body_en: "<p>At the <strong>MSPO International Defense Industry Exhibition</strong> in Kielce, Turkish defense firms showcased armored vehicles, tactical UAVs, EW pods, and C2 software to Polish and Baltic procurement delegations.</p>",
            content_type: "ozel-haberler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/mspo-turk.webp",
            author_name: "MSI Özel Ekibi - Kielce",
            author_title: "Uluslararası Fuar ve İhracat Masası",
            issue_number: Some(280),
            published_at: "2026-09-06",
            read_time_min: 9,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: None,
            view_count: 8910,
            true_read_count: 7340,
            placements: &["ozel-haberler", "etkinlikler", "etkinlik-sonrasi-ozel-dosyalar", "dergi-icerikleri"],
            ssb_tags: &["kara-araclari", "hava-araclari", "hizmetler-kurumsal-iletisim"],
        },
        SeedArticle {
            slug: "stm-milgem-istif-sinifi-ve-stm500-denizalti-cozumleri",
            title_tr: "STM'den Mavi Vatan'a Çelik İmza: İSTİF Sınıfı Fırkateynler ve STM500 Milli Denizaltı",
            title_en: "STM's Naval Engineering Signature: ISTIF-Class Frigates and STM500 National Submarine",
            summary_tr: "Askeri deniz inşasında Türkiye'nin mühendislik gücü STM, MİLGEM İSTİF sınıfı fırkateynlerin çoklu tersane üretimini yönetirken STM500 sığ su denizaltısında yeni aşamaya geçti.",
            summary_en: "Leading Türkiye's naval engineering, STM coordinates multi-shipyard construction of ISTIF-class frigates while advancing the STM500 submarine program.",
            body_tr: "<p>Savunma Teknolojileri Mühendislik ve Ticaret A.Ş. (<strong>STM</strong>), su üstü ve su altı platform tasarımında edindiği tecrübeyi yüksek yerlilik oranlarıyla küresel ölçeğe taşıyor. TCG İSTANBUL'un (F-515) Deniz Kuvvetleri Komutanlığı envanterine katılmasının ardından kardeş gemilerin inşası hızla devam ediyor.</p><p>Özel harekât, istihbarat ve sığ su denizaltı harbi görevleri için sıfırdan milli imkanlarla tasarlanan <strong>STM500</strong> ise mukavim tekne üretimi ve alt sistem entegrasyonuyla Türkiye'yi kendi denizaltısını tasarlayıp üretebilen seçkin ülkeler ligine yükseltiyor.</p>",
            body_en: "<p><strong>STM</strong> scales its naval surface and subsurface engineering capabilities globally. Following TCG ISTANBUL's commissioning, sister ISTIF-class frigates progress rapidly alongside pressure-hull integration of the indigenous <strong>STM500</strong> submarine.</p>",
            content_type: "tanitici-makaleler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/stm-fuar.webp",
            author_name: "Burak Çelik",
            author_title: "Deniz Sistemleri Editörü",
            issue_number: Some(280),
            published_at: "2026-09-05",
            read_time_min: 7,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: None,
            view_count: 9120,
            true_read_count: 7580,
            placements: &["tanitici-makaleler", "kapak-dosyalari", "dergi-icerikleri"],
            ssb_tags: &["deniz-araclari", "itki-tahrik-ve-guc-sistemleri"],
        },
        SeedArticle {
            slug: "sarsilmaz-ve-tr-mekatronik-20mm-burun-topu-entegrasyonu",
            title_tr: "Sarsılmaz ve TR Mekatronik'ten Taarruz Helikopterleri İçin 20 mm Burun Topu ve Otonom Kule Sistemleri",
            title_en: "Sarsilmaz & TR Mekatronik Deliver 20 mm Nose Cannon and Autonomous Turrets for Attack Helicopters",
            summary_tr: "Hafif silah üretimindeki asırlık tecrübesini ağır makineli tüfek ve havacılık silah sistemlerine taşıyan Sarsılmaz, 20 mm döner namlulu top ve SARP kule entegrasyonlarında seri üretime geçti.",
            summary_en: "Sarsilmaz and TR Mekatronik enter serial production on the 20 mm rotary nose cannon for attack helicopters and remote weapon stations.",
            body_tr: "<p>Taarruz helikopterleri ve zırhlı kara araçlarının ateş gücünü yerlileştirme hedefi doğrultusunda <strong>Sarsılmaz</strong> ve TUSAŞ-Sarsılmaz ortak girişimi <strong>TR Mekatronik</strong> tarafından geliştirilen 20 mm üç döner namlulu burun topu, zorlu kalifikasyon ve atış testlerini başarıyla tamamladı. Dakikada 750 atım kapasitesine sahip sistem, T129 ATAK helikopterlerinde kabiliyet artışı sağlıyor.</p>",
            body_en: "<p>Developed by <strong>Sarsilmaz</strong> and the TUSAS-Sarsilmaz joint venture <strong>TR Mekatronik</strong>, the indigenous 20 mm three-barrel rotary cannon delivers 750 rounds per minute for T129 ATAK attack helicopters.</p>",
            content_type: "haberler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/sarsilmaz.webp",
            author_name: "MSI Dergisi",
            author_title: "Kara ve Silah Sistemleri Masası",
            issue_number: Some(280),
            published_at: "2026-09-04",
            read_time_min: 5,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 0,
            video_url: None,
            view_count: 6410,
            true_read_count: 5120,
            placements: &["haberler", "dergi-icerikleri"],
            ssb_tags: &["namlulu-silahlar", "hava-araclari", "kara-araclari"],
        },
        SeedArticle {
            slug: "yaltes-tayland-kraliyet-donanmasi-savas-yonetim-cozumu",
            title_tr: "YALTES'ten Tayland Kraliyet Donanması Fırkateynleri İçin Entegre Platform Kontrol ve İzleme Sistemi",
            title_en: "YALTES Supplies Integrated Platform Management Systems for Royal Thai Navy Frigates",
            summary_tr: "Deniz platformları otomasyon ve savaş yönetim donanımlarında 20. yılını kutlayan YALTES, Asya-Pasifik pazarındaki ihracat başarısını yeni fırkateyn modernizasyon projesiyle taçlandırdı.",
            summary_en: "Celebrating its 20th anniversary in naval automation and combat system consoles, YALTES expands its Asia-Pacific footprint with Royal Thai Navy frigate upgrades.",
            body_tr: "<p>Türk savunma sanayiinin deniz elektroniği ve entegre platform kontrol ve izleme sistemleri (EPKİS) alanındaki öncü kuruluşu <strong>YALTES</strong>, Tayland Kraliyet Donanması envanterindeki fırkateynlerin modernizasyonu kapsamında kritik donanım ve yazılım paketlerinin teslimatını gerçekleştirdi. Askeri standartlarda çok işlevli operatör konsolları ve hasar kontrol mimarisiyle donatılan gemiler, yüksek beka kabiliyeti kazanıyor.</p>",
            body_en: "<p><strong>YALTES</strong> has delivered military-grade multi-function operator consoles and Integrated Platform Management Systems (IPMS) for the modernization of Royal Thai Navy frigates.</p>",
            content_type: "ozel-haberler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/yaltes-tayland.webp",
            author_name: "MSI Dergisi",
            author_title: "Deniz Elektroniği ve İhracat Masası",
            issue_number: Some(280),
            published_at: "2026-09-03",
            read_time_min: 6,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 0,
            video_url: None,
            view_count: 5980,
            true_read_count: 4840,
            placements: &["ozel-haberler", "dergi-icerikleri", "savunma-sanayii-tarihi"],
            ssb_tags: &["deniz-araclari", "haberlesme-ve-muharebe-destek-sistemleri"],
        },
        SeedArticle {
            slug: "sedec-ve-saha-expo-sonrasi-tedarik-zinciri-ozel-dosyasi",
            title_tr: "SEDEC ve SAHA EXPO Özel Dosyası: Küresel Tedarik Zincirinde Türk Savunma Ekosistemi",
            title_en: "SEDEC & SAHA EXPO Special Dossier: Turkish Defense Ecosystem in the Global Supply Chain",
            summary_tr: "Anavatan güvenliği, sınır koruma teknolojileri ve B2B savunma tedarik zinciri buluşmalarında gerçekleştirilen 4.500'ü aşkın ikili iş görüşmesinin sektörel bilançosu.",
            summary_en: "Sectoral assessment of over 4,500 B2B industrial matchmaking sessions across homeland security, border protection, and aerospace supply chains.",
            body_tr: "<p>Savunma ve havacılık sanayiinde ana yükleniciler ile uluslararası tedarik makamlarını KOBİ'lerle doğrudan buluşturan <strong>SEDEC</strong> ve <strong>SAHA EXPO</strong> organizasyonları, Türk savunma ihracatının sürdürülebilir büyümesinde kilit rol oynuyor. Etkinlik sonrası özel dosyamızda J.G.K.lığı, EGM ve S.G.K.lığı heyetlerinin iç güvenlik ekipman talepleri ile yabancı ataşelerin değerlendirmeleri yer alıyor.</p>",
            body_en: "<p>Connecting prime contractors and international procurement authorities directly with specialized SMEs, <strong>SEDEC</strong> and <strong>SAHA EXPO</strong> drive sustainable defense export growth.</p>",
            content_type: "ozel-haberler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/sedec.webp",
            author_name: "MSI Etkinlik Analiz Ekibi",
            author_title: "Fuar ve Kümelenme Özel Dosyaları",
            issue_number: Some(280),
            published_at: "2026-09-02",
            read_time_min: 10,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: None,
            view_count: 7490,
            true_read_count: 6180,
            placements: &["ozel-dosyalar", "etkinlikler", "etkinlik-sonrasi-ozel-dosyalar", "etkinlik-ozel-sayilari"],
            ssb_tags: &["hizmetler-kurumsal-iletisim", "hizmetler-danismanlik", "kara-araclari"],
        },
        SeedArticle {
            slug: "turgut-senol-havacilik-ve-savunma-stratejileri-soylesisi",
            title_tr: "Turgut Şenol ile Özel Söyleşi: Havacılık, Simülasyon ve Küresel Program Yönetimi",
            title_en: "Exclusive Interview with Turgut Senol: Aerospace, Simulation and Global Program Management",
            summary_tr: "Savunma ve havacılık projelerinde uluslararası iş geliştirme, eğitim-simülasyon merkezleri ve mühendislik ihracatı üzerine kapsamlı değerlendirmeler.",
            summary_en: "In-depth perspectives on international business development, synthetic training centers, and engineering exports in aerospace.",
            body_tr: "<p>Türk savunma ve havacılık sanayiinin deneyimli yöneticilerinden <strong>Turgut Şenol</strong> ile gerçekleştirdiğimiz söyleşide; beşinci nesil eğitim-simülasyon altyapılarının pilot hazırlık süreçlerine etkisi, uluslararası ortak üretim modelleri ve üniversite savunma kulüplerinden yetişen genç mühendislerin sektöre entegrasyonu ele alındı.</p>",
            body_en: "<p>In our exclusive interview with aerospace executive <strong>Turgut Senol</strong>, we explore 5th-generation synthetic training infrastructures, international co-production frameworks, and talent cultivation with university defense clubs.</p>",
            content_type: "soylesiler",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/turgut-senol.webp",
            author_name: "Ümit Bayraktar",
            author_title: "MSI Dergisi Genel Yayın Yönetmeni",
            issue_number: Some(280),
            published_at: "2026-09-01",
            read_time_min: 9,
            visibility_level: 1,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 0,
            video_url: None,
            view_count: 6250,
            true_read_count: 5090,
            placements: &["soylesiler", "dergi-icerikleri", "ogrenci-kulupleri"],
            ssb_tags: &["hava-araclari", "simulatorler-ve-egitim-sistemleri", "hizmetler-egitim"],
        },
        SeedArticle {
            slug: "msi-dergisi-280-sayi-basyazi-caydiricilik-ve-teknolojik-bagimsizlik",
            title_tr: "Başyazı (280. Sayı): 22 Yıllık Hafıza, Caydırıcılık ve Teknolojik Bağımsızlık",
            title_en: "Editorial (Issue 280): 22 Years of Institutional Memory, Deterrence and Technological Sovereignty",
            summary_tr: "2004 yılından bu yana Türk savunma ve havacılık sanayiinin her adımını kayıt altına alan MSI Dergisi, 280. sayısında yeni nesil dijital istihbarat ve bilgi platformuyla geleceğe yürüyor.",
            summary_en: "Documenting every milestone of the Turkish defense and aerospace industry since 2004, MSI Magazine steps into the future with its Issue 280 and next-gen digital platform.",
            body_tr: "<p>2004 yılında yayın hayatına başlayan <strong>MSI Dergisi (Military Science &amp; Intelligence)</strong> ve İngilizce kardeş yayını <strong>MSI TDR (Turkish Defence Review)</strong>, geride kalan 22 yılda Türk savunma ve havacılık sanayiinin küresel bir güce dönüşümüne tanıklık etmiş ve bu dönüşümün kurumsal hafızası olmuştur.</p><p>280. sayımızla birlikte devreye aldığımız bu yeni nesil dijital mimari; son kullanıcılarımız (TSK, J.G.K.lığı, EGM, S.G.K.lığı), tedarik makamlarımız (MSB, SSB) ve sektör kuruluşlarımız arasında kesintisiz, güvenli ve kişiselleştirilmiş bir bilgi köprüsü kurmaktadır.</p>",
            body_en: "<p>Launched in 2004, <strong>MSI Magazine</strong> and its English sister publication <strong>MSI TDR (Turkish Defence Review)</strong> have served as the institutional memory of Türkiye's defense transformation over 22 years and 280 issues.</p>",
            content_type: "basyazi",
            source_group: "dergi-icerigi",
            cover_image: "images/msi/dergi-280.webp",
            author_name: "MSI Dergisi Yayın Kurulu",
            author_title: "Başyazı",
            issue_number: Some(280),
            published_at: "2026-09-01",
            read_time_min: 5,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: 0,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: 1,
            video_url: None,
            view_count: 10240,
            true_read_count: 8740,
            placements: &["basyazi", "dergi-icerikleri", "savunma-sanayii-tarihi", "yazarlar"],
            ssb_tags: &["hava-araclari", "deniz-araclari", "kara-araclari"],
        },
    ];

    for a in articles {
        conn.execute(
            "INSERT OR IGNORE INTO contents (
                slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                content_type, source_group, cover_image, author_name, author_title,
                issue_number, published_at, read_time_min, visibility_level, single_price_try,
                is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                view_count, true_read_count
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22,
                ?23, ?24
            )",
            params![
                a.slug,
                a.title_tr,
                a.title_en,
                a.summary_tr,
                a.summary_en,
                a.body_tr,
                a.body_en,
                a.content_type,
                a.source_group,
                a.cover_image,
                a.author_name,
                a.author_title,
                a.issue_number,
                a.published_at,
                a.read_time_min,
                a.visibility_level,
                a.single_price_try,
                a.is_sponsored,
                a.sponsor_brand,
                a.sponsor_legal_label,
                a.is_featured,
                a.video_url,
                a.view_count,
                a.true_read_count
            ],
        )?;

        let content_id: i64 = conn.query_row(
            "SELECT id FROM contents WHERE slug = ?1",
            params![a.slug],
            |r| r.get(0),
        )?;

        for p in a.placements {
            conn.execute(
                "INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)",
                params![content_id, p],
            )?;
        }

        for tag in a.ssb_tags {
            conn.execute(
                "INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)",
                params![content_id, tag],
            )?;
        }
    }

    Ok(())
}

fn seed_magazine_issues(conn: &Connection) -> rusqlite::Result<()> {
    let spreads_280 = vec![
        MagazinePageSpread {
            page_no: "S. 01-04".to_string(),
            title_tr: "Kapak & İçindekiler Fihristi — Sayı 280".to_string(),
            title_en: "Cover & Table of Contents — Issue 280".to_string(),
            category_badge: "KAPAK & BAŞYAZI".to_string(),
            image_url: "images/msi/dergi-280.webp".to_string(),
            excerpt_tr: "22 Yıllık Yayıncılık Otoritesi: Başyazı, Savunma Gündemi ve Özel Dosyalar Fihristi.".to_string(),
            excerpt_en: "22 Years of Defense Publishing Authority: Editorial, Defense Briefs and Special Dossier Index.".to_string(),
            linked_content_slug: Some("msi-dergisi-280-sayi-basyazi-caydiricilik-ve-teknolojik-bagimsizlik".to_string()),
        },
        MagazinePageSpread {
            page_no: "S. 12-18".to_string(),
            title_tr: "KAAN 5. Nesil Aviyonik, MURAD AESA Radar ve Seri Üretim Mimarisi".to_string(),
            title_en: "KAAN 5th-Gen Avionics, MURAD AESA Radar & Serial Production Architecture".to_string(),
            category_badge: "KAPAK DOSYASI".to_string(),
            image_url: "images/msi/kaan.webp".to_string(),
            excerpt_tr: "BÜRFİS, DAS ve yüksek hızlı fiber optik görev bilgisayarı mimarisinin teknik kesiti.".to_string(),
            excerpt_en: "Technical cutaway of BURFIS, DAS, and high-speed fiber-optic mission computing.".to_string(),
            linked_content_slug: Some("kaan-programinda-mevcut-durum-ve-yol-haritasi".to_string()),
        },
        MagazinePageSpread {
            page_no: "S. 24-32".to_string(),
            title_tr: "CTech Sürü İDA Hibrit Haberleşme & Dr. M. Cüneyd Fırat Röportajı".to_string(),
            title_en: "CTech Swarm USV Hybrid Comms & Dr. M. Cuneyd Firat Executive Interview".to_string(),
            category_badge: "AYIN SÖYLEŞİSİ".to_string(),
            image_url: "images/msi/ctech-roportaj.webp".to_string(),
            excerpt_tr: "Mavi Vatan ve gökyüzünde elektronik harbe dayanıklı Ku/Ka-Bant SATCOM ve Taktik Mesh ağları.".to_string(),
            excerpt_en: "Jam-resistant Ku/Ka-Band SATCOM and Tactical Mesh networks across sea and air domains.".to_string(),
            linked_content_slug: Some("dr-m-cuneyd-firat-ctech-kapak-soylesisi".to_string()),
        },
        MagazinePageSpread {
            page_no: "S. 40-48".to_string(),
            title_tr: "AKINCI İHA-300 Atışı, Polonya MSPO ve SEDEC Özel Dosyaları".to_string(),
            title_en: "AKINCI IHA-300 Strike, MSPO Poland & SEDEC Post-Event Dossiers".to_string(),
            category_badge: "ÖZEL DOSYA".to_string(),
            image_url: "images/msi/akinci-iha300.webp".to_string(),
            excerpt_tr: "250 km süpersonik balistik taarruz testi ve uluslararası fuarlarda imzalanan stratejik sözleşmeler.".to_string(),
            excerpt_en: "250 km supersonic ballistic strike trial and strategic defense export contracts.".to_string(),
            linked_content_slug: Some("akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu".to_string()),
        },
    ];
    let spreads_json = serde_json::to_string(&spreads_280).unwrap_or_else(|_| "[]".to_string());

    let issues: &[(i32, &str, &str, &str, &str, &str, i32, &str, i32, Option<&str>, i32, i32, &str, &str, &str)] = &[
        (281, "saha-expo-2026-ozel-sayisi", "MSI Dergisi & MSI TDR • SAHA EXPO 2026 Özel Sayısı", "MSI TDR • SAHA EXPO 2026 Special Exhibition Issue", "Ekim", "October", 2026, "images/msi/saha-deniz.webp", 1, Some("SAHA EXPO 2026"), 144, 18, "SAHA EXPO 2026 Uluslararası Savunma, Havacılık ve Uzay Sanayii Fuarı Çift Dilli (TR/EN) Özel Sayısı.", "Bilingual (TR/EN) Special Exhibition Issue for SAHA EXPO 2026 Defense & Aerospace Exhibition.", "/api/v1/edergi/281/download-pdf"),
        (280, "sayi-280-eylul-2026", "MSI Dergisi • Sayı 280 (Eylül 2026)", "MSI Magazine • Issue 280 (September 2026)", "Eylül", "September", 2026, "images/msi/dergi-280.webp", 0, None, 112, 12, "Milli Muharip Uçak KAAN 5. Nesil Aviyonik, CTech Sürü İDA Hibrit Haberleşme ve AKINCI İHA-300 Kapak Dosyası.", "National Combat Aircraft KAAN 5th-Gen Avionics, CTech Swarm USV Comms and AKINCI IHA-300 Cover Dossier.", "/api/v1/edergi/280/download-pdf"),
        (279, "sayi-279-agustos-2026", "MSI Dergisi • Sayı 279 (Ağustos 2026)", "MSI Magazine • Issue 279 (August 2026)", "Ağustos", "August", 2026, "images/msi/kaan.webp", 0, None, 108, 11, "Çelik Kubbe Katmanlı Hava Savunma Mimarisi, SİPER Ürün-2 ve MİLGEM İSTİF Sınıfı Fırkateynler.", "Steel Dome Multi-Layered Air Defense Architecture, SIPER Block-2 and MILGEM ISTIF-Class Frigates.", "/api/v1/edergi/279/download-pdf"),
        (278, "sayi-278-temmuz-2026", "MSI Dergisi • Sayı 278 (Temmuz 2026)", "MSI Magazine • Issue 278 (July 2026)", "Temmuz", "July", 2026, "images/msi/stm-fuar.webp", 0, None, 104, 10, "Mavi Vatan Deniz Sistemleri, STM500 Denizaltısı ve YALTES Savaş Yönetim Konsolları Dosyası.", "Blue Homeland Naval Systems, STM500 Submarine and YALTES Combat Management Consoles Dossier.", "/api/v1/edergi/278/download-pdf"),
        (277, "sayi-277-haziran-2026", "MSI Dergisi • Sayı 277 (Haziran 2026)", "MSI Magazine • Issue 277 (June 2026)", "Haziran", "June", 2026, "images/msi/mspo-turk.webp", 1, Some("Eurosatory & SEDEC"), 116, 14, "Avrupa Savunma Sanayii İş Birlikleri, ALTAY Ana Muharebe Tankı ve Yeni Nesil Zırhlı Muharebe Araçları.", "European Defense Cooperation, ALTAY Main Battle Tank and Next-Gen Armored Combat Vehicles.", "/api/v1/edergi/277/download-pdf"),
        (276, "sayi-276-mayis-2026", "MSI Dergisi • Sayı 276 (Mayıs 2026)", "MSI Magazine • Issue 276 (May 2026)", "Mayıs", "May", 2026, "images/msi/skyarx.webp", 0, None, 104, 9, "Elektronik Harp, AESA Radar Teknolojileri ve Taktik Saha Haberleşme Sistemleri Özel İncelemesi.", "Electronic Warfare, AESA Radar Technologies and Tactical Field Communications Special Review.", "/api/v1/edergi/276/download-pdf"),
    ];

    for iss in issues {
        conn.execute(
            "INSERT OR IGNORE INTO magazine_issues (
                issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
                year, cover_image, is_special_issue, event_name, page_count, dossier_count,
                summary_tr, summary_en, pdf_path, spreads_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                iss.0, iss.1, iss.2, iss.3, iss.4, iss.5,
                iss.6, iss.7, iss.8, iss.9, iss.10, iss.11,
                iss.12, iss.13, iss.14, &spreads_json
            ],
        )?;
    }

    Ok(())
}

fn seed_users_and_verifications(conn: &Connection) -> rusqlite::Result<()> {
    let secret = "msi-umsa-2026-tactical-secret-key-9948217492";
    let admin_pw = crate::services::auth_service::hash_password("MsiUmsa2026!", secret);
    let reader_pw = crate::services::auth_service::hash_password("Savunma2026!", secret);

    let users: &[(&str, &str, &str, &str, &str, &str, &str, i32, &str, Option<&str>, i32, &str)] = &[
        ("operator@msidergisi.com", &admin_pw, "UMSA Operasyon Merkezi", "site_operator", "Sektör Kuruluşları", "UMSA Prodüksiyon - MSI Dergisi", "Baş Operatör & Yayın Yönetmeni", 1, "Kurumsal Tam Yetki", Some("2030-12-31"), 0, "[\"hava-araclari\",\"deniz-araclari\",\"kara-araclari\",\"radarlar-ve-sensorler\"]"),
        ("murat.yilmaz@msb.gov.tr", &reader_pw, "Alb. Murat Yılmaz", "subscriber_public", "MSB", "Millî Savunma Bakanlığı Plan Prensipler Gn.Md.", "Kıdemli Proje Subayı", 1, "Kamu E-Dergi Abonesi (Ücretsiz)", Some("2027-12-31"), 0, "[\"hava-araclari\",\"muhimmatlar-ve-fuze-sistemleri\",\"radarlar-ve-sensorler\"]"),
        ("selin.kaya@ssb.gov.tr", &reader_pw, "Dr. Selin Kaya", "subscriber_public", "SSB", "Savunma Sanayii Başkanlığı Hava Savunma Dairesi", "Savunma Sanayii Uzmanı", 1, "Kamu E-Dergi Abonesi (Ücretsiz)", Some("2027-12-31"), 0, "[\"muhimmatlar-ve-fuze-sistemleri\",\"radarlar-ve-sensorler\",\"elektronik-harp-sistemleri\"]"),
        ("levent.aksoy@dzkk.tsk.tr", &reader_pw, "Dz.Kur.Alb. Levent Aksoy", "subscriber_public", "Dz.K.K.lığı", "Deniz Kuvvetleri Komutanlığı Harekât Başkanlığı", "Şube Müdürü", 1, "Kamu E-Dergi Abonesi (Ücretsiz)", Some("2027-12-31"), 0, "[\"deniz-araclari\",\"haberlesme-ve-muharebe-destek-sistemleri\",\"itki-tahrik-ve-guc-sistemleri\"]"),
        ("cem.demir@hvkk.tsk.tr", &reader_pw, "Hv.Plt.Yb. Cem Demir", "subscriber_public", "Hv.K.K.lığı", "Hava Kuvvetleri Komutanlığı Plan Harekât", "Filo Standardizasyon Subayı", 1, "Kamu E-Dergi Abonesi (Ücretsiz)", Some("2027-12-31"), 0, "[\"hava-araclari\",\"radarlar-ve-sensorler\",\"simulatorler-ve-egitim-sistemleri\"]"),
        ("serkan.oz@jandarma.gov.tr", &reader_pw, "J.Bnb. Serkan Öz", "subscriber_public", "J.G.K.lığı", "Jandarma Genel Komutanlığı İHA ve Havacılık Dairesi", "Teknik Proje Subayı", 1, "Kamu E-Dergi Abonesi (Ücretsiz)", Some("2027-12-31"), 0, "[\"hava-araclari\",\"kara-araclari\",\"haberlesme-ve-muharebe-destek-sistemleri\"]"),
        ("hakan.erdem@aselsan.com.tr", &reader_pw, "Hakan Erdem", "subscriber_corporate", "Sektör Kuruluşları", "ASELSAN A.Ş. Radar ve Elektronik Harp Sektör Bşk.", "Sistem Mühendisi Müdürü", 0, "Kurumsal Abone (Çoklu Lisans)", Some("2026-12-07"), 0, "[\"radarlar-ve-sensorler\",\"elektronik-harp-sistemleri\",\"haberlesme-ve-muharebe-destek-sistemleri\"]"),
        ("zeynep.ucar@tusas.com", &reader_pw, "Zeynep Uçar", "subscriber_print_digital", "Sektör Kuruluşları", "TUSAŞ Milli Muharip Uçak Genel Md. Yrd.", "Kıdemli Aviyonik Mühendisi", 0, "Basılı + Dijital Abone", Some("2026-11-21"), 0, "[\"hava-araclari\",\"itki-tahrik-ve-guc-sistemleri\"]"),
        ("baris.koc@stm.com.tr", &reader_pw, "Barış Koç", "subscriber_digital", "Sektör Kuruluşları", "STM Savunma Teknolojileri Deniz Projeleri", "İş Geliştirme Yöneticisi", 0, "Dijital E-Dergi Abonesi", Some("2027-08-15"), 1, "[\"deniz-araclari\",\"hizmetler-danismanlik\"]"),
        ("emre.aydiner@itu.edu.tr", &reader_pw, "Emre Aydıner", "registered", "Üniversite / Öğrenci", "İTÜ Savunma Teknolojileri Kulübü (SAVTEK)", "Kulüp Başkanı • Uçak Mühendisliği", 0, "Akademik / Öğrenci Okur", None, 0, "[\"hava-araclari\",\"simulatorler-ve-egitim-sistemleri\",\"hizmetler-egitim\"]"),
    ];

    for u in users {
        conn.execute(
            "INSERT OR IGNORE INTO users (
                email, password_hash, full_name, role, organization_category, organization_name,
                job_title, is_gov_verified, subscription_tier, subscription_expires_at,
                reminders_halted, kvkk_consent_at, kvkk_consent_ip, kvkk_consent_hash,
                email_consent, ssb_interests_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, '2026-09-01T10:15:00Z', '194.110.169.165', 'KVKK-2026-VERIFIED-SHA256', 1, ?12, '2026-09-01')",
            params![u.0, u.1, u.2, u.3, u.4, u.5, u.6, u.7, u.8, u.9, u.10, u.11],
        )?;
    }

    let corp_reqs: &[(i64, &str, &str, &str, &str, &str, &str, Option<&str>, &str)] = &[
        (7, "kurumsal.tedarik@gmail.com", "Müh. Tolga Yalçın", "SSB", "Savunma Sanayii Alt Sistem Proje Ofisi", "Kurum dışından harici e-posta ile kayıt oldum, SSB proje müşavirliği kapsamında ücretsiz kurumsal e-dergi aboneliği tanımlanmasını arz ederim.", "pending", None, "2026-11-05 14:20"),
        (8, "ata.savunma.kobi@outlook.com", "Dr. Onur Şahin", "Sektör Kuruluşları", "SAHA İstanbul & OSSA Üyesi Alt Yüklenici", "SAHA İstanbul küme üyesi savunma KOBİ'siyiz, kurumsal değerlendirme havuzuna dahil edilmeyi talep ediyoruz.", "pending", None, "2026-11-06 09:45"),
        (2, "murat.yilmaz@msb.gov.tr", "Alb. Murat Yılmaz", "MSB", "Millî Savunma Bakanlığı Plan Prensipler Gn.Md.", "Otomatik .gov.tr kamu doğrulaması tamamlandı.", "approved", Some("operator@msidergisi.com"), "2026-09-01 10:15"),
    ];

    for r in corp_reqs {
        conn.execute(
            "INSERT INTO corporate_verification_requests (
                user_id, email, full_name, organization_category, organization_name,
                request_message, status, reviewed_by, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8],
        )?;
    }

    Ok(())
}

fn seed_ad_campaigns(conn: &Connection) -> rusqlite::Result<()> {
    let ads: &[(&str, &str, &str, &str, &str, &str, &str, &str, &str, &str, &str, Option<&str>, i32, i32, i32, i32, &str, &str, &str, i64, i64)] = &[
        (
            "ad-mag-top",
            "REKLAM ALANI 1",
            "970 × 90 LEADERBOARD / DERGİ TEPE SPONSORLUK",
            "TUSAŞ",
            "defense_oem",
            "navy",
            "Geleceğin Gök Vatan Savunucuları: Milli Muharip Uçak KAAN ve ANKA III",
            "Defenders of the Sky Homeland: KAAN 5th Gen Fighter & ANKA III UCAV",
            "5. nesil aviyonik mimari ve yeni nesil insansız muharip sistemlerle gökyüzünde bağımsız güç.",
            "Sovereign air power with 5th-generation avionics and unmanned combat aerial systems.",
            "/icerik/kaan-programinda-mevcut-durum-ve-yol-haritasi",
            Some("images/msi/kaan.webp"),
            98, 1, 0, 0,
            "[\"ozel-haberler\",\"kapak-dosyalari\",\"dergi-icerikleri\"]",
            "[\"hava-araclari\",\"itki-tahrik-ve-guc-sistemleri\"]",
            "[\"MSB\",\"SSB\",\"Hv.K.K.lığı\",\"Sektör Kuruluşları\"]",
            42850, 1840
        ),
        (
            "ad-mag-mid",
            "REKLAM ALANI 2",
            "728 × 90 YATAY KUŞAK / DOSYA SPONSORLUĞU",
            "ROKETSAN",
            "defense_oem",
            "gold",
            "Hassas Vuruş Gücü: Hava, Kara ve Deniz Platformlarında Taktik Üstünlük",
            "Precision Strike Power: Tactical Superiority Across Air, Land and Naval Platforms",
            "SİPER, HİSAR, ÇAKIR ve İHA-300 sistemleri ile caydırıcı milli mühimmat mimarisi.",
            "Deterrent indigenous missile architecture with SIPER, HISAR, CAKIR and IHA-300.",
            "/icerik/akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu",
            Some("images/msi/akinci-iha300.webp"),
            95, 0, 0, 0,
            "[\"haberler\",\"ozel-dosyalar\",\"dergi-icerikleri\"]",
            "[\"muhimmatlar-ve-fuze-sistemleri\",\"hava-araclari\"]",
            "[\"MSB\",\"SSB\",\"K.K.K.lığı\",\"Hv.K.K.lığı\",\"Dz.K.K.lığı\"]",
            36400, 1520
        ),
        (
            "ad-mag-pricing",
            "REKLAM ALANI 3",
            "970 × 90 PRESTİJ KUŞAK / ARŞİV SPONSORLUĞU",
            "HAVELSAN",
            "defense_oem",
            "navy",
            "Dijital Birlik ve Yapay Zekâ Destekli Muharebe Yönetim Sistemleri",
            "Digital Troops and AI-Powered Combat Management Systems",
            "ADVENT Savaş Yönetim Sistemi ve otonom kara-hava-deniz ortak harekât mimarisi.",
            "ADVENT Combat Management System and autonomous multi-domain joint operational architecture.",
            "/icerik/ctech-suru-ida-hibrit-haberlesme-konsepti",
            Some("images/msi/yaltes-tayland.webp"),
            92, 0, 0, 0,
            "[\"tanitici-makaleler\",\"dergi-icerikleri\",\"tskdan-haberler\"]",
            "[\"haberlesme-ve-muharebe-destek-sistemleri\",\"simulatorler-ve-egitim-sistemleri\"]",
            "[\"MSB\",\"SSB\",\"Dz.K.K.lığı\",\"K.K.K.lığı\"]",
            31200, 1290
        ),
        (
            "ad-news-top",
            "REKLAM ALANI 4",
            "970 × 90 HABER PORTALI TEPE LEADERBOARD",
            "ASELSAN",
            "defense_oem",
            "gold",
            "AESA Radar & Elektronik Harp Sistemleri ile Hava ve Deniz Üstünlüğü",
            "Air and Naval Superiority with AESA Radar & Electronic Warfare Systems",
            "Hava, kara ve deniz platformlarında elektromanyetik spektrumu millileştiren sensör çözümleri.",
            "Indigenous sensor solutions dominating the electromagnetic spectrum across all domains.",
            "/icerik/aselsan-aesa-radar-ve-elektronik-harp-mimarisi-advertorial",
            Some("images/msi/skyarx.webp"),
            99, 1, 0, 0,
            "[\"haberler\",\"advertorial\",\"tanitici-makaleler\"]",
            "[\"radarlar-ve-sensorler\",\"elektronik-harp-sistemleri\"]",
            "[\"MSB\",\"SSB\",\"Hv.K.K.lığı\",\"Dz.K.K.lığı\",\"J.G.K.lığı\",\"Sektör Kuruluşları\"]",
            48900, 2190
        ),
        (
            "ad-news-infeed",
            "REKLAM ALANI 5",
            "728 × 90 İÇERİK ARASI YATAY KUŞAK",
            "STM SAVUNMA",
            "defense_oem",
            "emerald",
            "Mavi Vatan'ın Çelik Muhafızları: Milli Fırkateyn ve Denizaltı Çözümleri",
            "Steel Guardians of Blue Homeland: National Frigate and Submarine Solutions",
            "MİLGEM İSTİF sınıfı ve STM500 projeleriyle dünya denizlerinde Türk mühendislik imzası.",
            "Turkish naval engineering signature across global seas with MILGEM ISTIF and STM500.",
            "/icerik/stm-milgem-istif-sinifi-ve-stm500-denizalti-cozumleri",
            Some("images/msi/stm-fuar.webp"),
            90, 0, 0, 0,
            "[\"tanitici-makaleler\",\"kapak-dosyalari\",\"haberler\"]",
            "[\"deniz-araclari\",\"itki-tahrik-ve-guc-sistemleri\"]",
            "[\"Dz.K.K.lığı\",\"S.G.K.lığı\",\"SSB\",\"MSB\"]",
            29800, 1340
        ),
        (
            "ad-news-mid",
            "REKLAM ALANI 6",
            "728 × 90 STRATEJİK İÇERİK SPONSORLUĞU",
            "SAHA EXPO 2026",
            "auxiliary_service",
            "gold",
            "Uluslararası Savunma, Havacılık ve Uzay Sanayii Fuarı • İstanbul",
            "International Defense, Aerospace and Space Industry Exhibition • Istanbul",
            "1.200+ Katılımcı Firma • 120+ Ülkeden Resmi Delegasyon • B2B Tedarik Zinciri Zirvesi",
            "1,200+ Exhibitors • Official Delegations from 120+ Countries • B2B Supply Chain Summit",
            "/icerik/sedec-ve-saha-expo-sonrasi-tedarik-zinciri-ozel-dosyasi",
            Some("images/msi/saha-deniz.webp"),
            88, 1, 0, 1,
            "[\"etkinlikler\",\"etkinlik-ozel-sayilari\",\"etkinlik-sonrasi-ozel-dosyalar\"]",
            "[\"hizmetler-kurumsal-iletisim\",\"hizmetler-danismanlik\"]",
            "[\"MSB\",\"SSB\",\"Sektör Kuruluşları\",\"Yurt Dışı Paydaş\"]",
            34100, 1610
        ),
        (
            "ad-news-billboard",
            "REKLAM ALANI 7",
            "970 × 250 PRESTİJ BİLLBOARD / FOOTER ÖNCESİ KUŞAK",
            "BMC SAVUNMA",
            "defense_oem",
            "navy",
            "ALTAY Ana Muharebe Tankı ve Yeni Nesil Taktik Tekerlekli Zırhlı Araç Ailesi",
            "ALTAY Main Battle Tank and Next-Generation Tactical Wheeled Armored Vehicles",
            "Kara Kuvvetleri ve dost-müttefik ordular için muharebe sahasında kanıtlanmış yüksek beka ve ateş gücü.",
            "Combat-proven survivability and firepower for Land Forces and allied armies.",
            "/icerik/polonya-mspo-fuarinda-turk-savunma-sanayii-cikarmasi",
            Some("images/msi/saha-kara.webp"),
            91, 0, 0, 0,
            "[\"haberler\",\"tskdan-haberler\",\"ozel-haberler\"]",
            "[\"kara-araclari\",\"itki-tahrik-ve-guc-sistemleri\"]",
            "[\"K.K.K.lığı\",\"J.G.K.lığı\",\"EGM\",\"MSB\",\"SSB\"]",
            27600, 1180
        ),
        (
            "ad-floating-popup",
            "FLOATING POP-UP VITRIN",
            "600 × 400 SAYFA GİRİŞİ ÖZEL SPONSORLUK",
            "KASABAWORKS & AMAC MAGAZINE",
            "in_house",
            "gold",
            "Savunma Sanayii Kurumsal İletişim, Fuar Mimari Tasarım ve AMAC Ataşe Yayınları",
            "Defense Corporate Communications, Exhibition Architecture & AMAC Attaché Publications",
            "UMSA Prodüksiyon çözüm ortakları KasabaWorks ve AMAC Magazine ile küresel savunma diplomasisinde markanızı konumlandırın.",
            "Position your defense brand in global defense diplomacy with UMSA solution partners KasabaWorks and AMAC Magazine.",
            "/kurumsal/cozum-ortaklarimiz",
            Some("images/msi/dergi-280.webp"),
            85, 1, 1, 1,
            "[\"kurumsal\",\"advertorial\",\"etkinlikler\"]",
            "[\"hizmetler-kurumsal-iletisim\",\"hizmetler-danismanlik\",\"hizmetler-kurumsal-catering\"]",
            "[\"Sektör Kuruluşları\",\"MSB\",\"SSB\",\"Yurt Dışı Paydaş\"]",
            22400, 980
        ),
        (
            "ad-inhouse-rbf-fallback",
            "KURUMSAL ÇÖZÜM ORTAĞI",
            "970 × 90 STANDART VARSAYILAN VİTRİN",
            "REMOVE BEFORE PHOTOS (RBF) & MSI DERGİSİ",
            "in_house",
            "navy",
            "Hava-Hava, Kara ve Deniz Platformlarında Telifli Havacılık & Savunma Fotoğrafçılığı",
            "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms",
            "Onur Kurç & Tayfun Yaşar yönetiminde Remove Before Photos yüksek çözünürlüklü telifli savunma görsel arşivi ve MSI Dergisi kurumsal abonelik çözümleri.",
            "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.",
            "/abonelik-ve-satis",
            Some("images/msi/dergi-286.webp"),
            80, 1, 0, 1,
            "[\"*\"]",
            "[\"hava-araclari\",\"deniz-araclari\",\"kara-araclari\",\"hizmetler-kurumsal-iletisim\"]",
            "[\"*\"]",
            19500, 840
        ),
    ];

    for a in ads {
        conn.execute(
            "INSERT INTO ad_campaigns (
                slot_code, badge_label, dimensions, sponsor_name, sponsor_type, pill_style,
                title_tr, title_en, desc_tr, desc_en, target_url, image_url,
                budget_score, is_default_fallback, is_floating_popup, is_newsletter_ad,
                target_categories_json, target_ssb_tags_json, target_institutions_json,
                impressions_count, clicks_count, is_active
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16,
                ?17, ?18, ?19,
                ?20, ?21, 1
            )",
            params![
                a.0, a.1, a.2, a.3, a.4, a.5,
                a.6, a.7, a.8, a.9, a.10, a.11,
                a.12, a.13, a.14, a.15,
                a.16, a.17, a.18,
                a.19, a.20
            ],
        )?;
    }

    Ok(())
}

fn seed_recommendation_rules(conn: &Connection) -> rusqlite::Result<()> {
    let rules: &[(&str, &str, &str, &str, i32)] = &[
        ("Hava Kuvvetleri & SSB -> KAAN 5. Nesil Aviyonik Önerisi", "hava-araclari", "Hv.K.K.lığı", "kaan-programinda-mevcut-durum-ve-yol-haritasi", 100),
        ("Deniz Kuvvetleri -> CTech Sürü İDA Hibrit Haberleşme Önerisi", "deniz-araclari", "Dz.K.K.lığı", "ctech-suru-ida-hibrit-haberlesme-konsepti", 95),
        ("Deniz Kuvvetleri -> STM MİLGEM & STM500 Önerisi", "deniz-araclari", "Dz.K.K.lığı", "stm-milgem-istif-sinifi-ve-stm500-denizalti-cozumleri", 90),
        ("SSB & MSB -> NATO Katmanlı Hava Savunma Analizi Önerisi", "muhimmatlar-ve-fuze-sistemleri", "SSB", "nato-hava-savunma-sistemlerinin-gelecegi-analiz", 92),
        ("Sektör Kuruluşları -> KOBİ Destek Mekanizmaları Analizi", "hizmetler-danismanlik", "Sektör Kuruluşları", "savunma-ve-havacilik-kobilerine-yonelik-yenilikci-destek-mekanizmalari", 88),
        ("Jandarma & EGM -> SEDEC İç Güvenlik Özel Dosyası", "kara-araclari", "J.G.K.lığı", "sedec-ve-saha-expo-sonrasi-tedarik-zinciri-ozel-dosyasi", 85),
    ];

    for r in rules {
        conn.execute(
            "INSERT INTO recommendation_rules (rule_name, source_tag_or_cat, target_institution, recommended_content_slug, priority, is_active)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            params![r.0, r.1, r.2, r.3, r.4],
        )?;
    }
    Ok(())
}

fn seed_subscription_plans_and_commerce(conn: &Connection) -> rusqlite::Result<()> {
    let plans = vec![
        SubscriptionPlan {
            code: "dijital".to_string(),
            name_tr: "Dijital E-Dergi & Arşiv Paketi".to_string(),
            name_en: "Digital E-Magazine & Archive Tier".to_string(),
            price_try: 1200.0,
            period_label_tr: "/ Yıllık (12 Sayı + Özel Sayılar)".to_string(),
            period_label_en: "/ Annual (12 Issues + Special Editions)".to_string(),
            features_tr: vec![
                "Web & Mobil Sayfa Çevirmeli E-Dergi Kiosk Erişimi".to_string(),
                "2004'ten Bugüne 280+ Sayı Tam Metin Aranabilir PDF Arşivi".to_string(),
                "Kişiye Özel Dinamik Filigranlı (Watermark) PDF İndirme".to_string(),
                "SSB İlgi Alanlarına Göre Kişiselleştirilmiş Haftalık Bülten".to_string(),
            ],
            features_en: vec![
                "Web & Mobile Interactive Flipbook E-Magazine Kiosk".to_string(),
                "Full-Text Searchable Archive of 280+ Issues Since 2004".to_string(),
                "Personalized Dynamically Watermarked PDF Downloads".to_string(),
                "Tailored Weekly Defense Intelligence Newsletter".to_string(),
            ],
            is_featured: false,
        },
        SubscriptionPlan {
            code: "basili-dijital".to_string(),
            name_tr: "Basılı Dergi + Dijital Tam Erişim".to_string(),
            name_en: "Print Magazine + Full Digital Access".to_string(),
            price_try: 2400.0,
            period_label_tr: "/ Yıllık (12 Basılı Sayı + Dijital Arşiv)".to_string(),
            period_label_en: "/ Annual (12 Print Issues + Digital Archive)".to_string(),
            features_tr: vec![
                "Her Ay Adresinize Teslim Prestij Baskı MSI Dergisi (12 Sayı)".to_string(),
                "IDEF ve SAHA EXPO Çift Dilli (TR/EN) Özel Sayıları Dahil".to_string(),
                "Dijital E-Dergi Kiosk ve Dinamik Filigranlı PDF Arşivi".to_string(),
                "Kademeli Otomatik Yenileme Hatırlatma ve E-Fatura Otomasyonu".to_string(),
            ],
            features_en: vec![
                "Monthly Prestige Print Delivery of MSI Magazine (12 Issues)".to_string(),
                "Includes IDEF & SAHA EXPO Bilingual Special Editions".to_string(),
                "Full Digital E-Magazine Kiosk & Watermarked PDF Archive".to_string(),
                "Automated E-Invoicing & 5-Stage Renewal Reminder Protection".to_string(),
            ],
            is_featured: true,
        },
        SubscriptionPlan {
            code: "kurumsal".to_string(),
            name_tr: "Kurumsal Çoklu Lisans & B2B Analiz Paketi".to_string(),
            name_en: "Corporate Multi-Seat & B2B Intelligence Suite".to_string(),
            price_try: 9500.0,
            period_label_tr: "/ Yıllık (Kurumsal 10 Kullanıcı + Özel Analizler)".to_string(),
            period_label_en: "/ Annual (10 Corporate Seats + Premium Analyses)".to_string(),
            features_tr: vec![
                "Kurumunuz İçin 10 Adet Çoklu Dijital + 5 Basılı Dergi Aboneliği".to_string(),
                "Tedarik Makamları, Ana Yüklenici ve NATO Özel Analizlerine Tam Erişim".to_string(),
                "Kurum Lisans Filigranlı (Watermark) Özel Dosya ve Rapor İndirme".to_string(),
                "Yıllık Savunma Sanayii B2B Okuyucu ve Sektör Dağılım Özeti".to_string(),
            ],
            features_en: vec![
                "10 Corporate Digital Seats + 5 Prestige Print Subscriptions".to_string(),
                "Full Access to Procurement Authority, Prime OEM & NATO Analyses".to_string(),
                "Corporate-Watermarked Special Dossier & Report Downloads".to_string(),
                "Annual Defense Sector B2B Readership & Industry Brief".to_string(),
            ],
            is_featured: false,
        },
    ];

    for (idx, p) in plans.iter().enumerate() {
        let f_tr = serde_json::to_string(&p.features_tr).unwrap_or_else(|_| "[]".to_string());
        let f_en = serde_json::to_string(&p.features_en).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT OR IGNORE INTO subscription_plans (
                code, name_tr, name_en, price_try, period_label_tr, period_label_en,
                features_tr_json, features_en_json, is_featured, sort_order
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                p.code, p.name_tr, p.name_en, p.price_try, p.period_label_tr, p.period_label_en,
                f_tr, f_en, if p.is_featured { 1 } else { 0 }, (idx + 1) as i32
            ],
        )?;
    }

    let products: &[(&str, &str, &str, &str, &str, &str, &str, f64, &str, &str)] = &[
        (
            "KITAP-YALTES-20YIL",
            "book",
            "YALTES 20. Yıl Kurumsal Tarihçe ve Deniz Elektroniği Prestij Kitabı",
            "YALTES 20th Anniversary Corporate History & Naval Electronics Prestige Book",
            "UMSA Prodüksiyon & YALTES Özel Yayını",
            "Türk deniz savaş yönetim ve platform otomasyon ekosisteminin 20 yıllık mühendislik serüveni (Özel Ciltli Baskı + Dijital Edisyon).",
            "The 20-year engineering chronicle of Turkish naval combat management and platform automation (Hardcover + Digital Edition).",
            850.0,
            "images/msi/yaltes-tayland.webp",
            "320 Sayfa • Ciltli Prestij Baskı + DRM PDF"
        ),
        (
            "KITAP-YAVUZPASA-RBF",
            "book",
            "Yavuz Paşa ve RBF Ekibi: Hava-Hava Muharebe ve Havacılık Fotoğrafçılığı Koleksiyon Kitabı",
            "Gen. Yavuz & RBF Team: Air-to-Air Combat & Military Aviation Photography Monograph",
            "Yavuz Paşa & RBF Havacılık Kolektifi",
            "Türk Hava Kuvvetleri ve milli hava platformlarının kokpitten kokpite çekilmiş yüksek çözünürlüklü tarihi fotoğrafları ve harekât anıları.",
            "High-resolution cockpit-to-cockpit photography and operational memoirs of the Turkish Air Force and national aerial platforms.",
            1450.0,
            "images/msi/kaan.webp",
            "280 Sayfa • Kuşe Koleksiyon Baskısı"
        ),
        (
            "RAPOR-SAVUNMA-KOBI-2026",
            "report",
            "Düşünce Kuruluşu Raporu: Savunma ve Havacılık KOBİ'leri İçin Küresel Destek Mekanizmaları ve Fuarcılık Analizi",
            "Think-Tank Report: Global SME Support Mechanisms & Defense Exhibition ROI Analysis",
            "MSI Stratejik Araştırmalar Merkezi",
            "Her yıl düzenlenen savunma fuarları, katılımcı firma beklentileri, KOSGEB/TEYDEB fonları ve yan hizmet firmaları için pazar rehberi.",
            "Comprehensive market guide covering annual defense exhibitions, exhibitor expectations, KOSGEB/TEYDEB grants, and auxiliary services.",
            650.0,
            "images/msi/sedec.webp",
            "96 Sayfa • Kurumsal Lisanslı Vektörel PDF"
        ),
        (
            "FOTO-RBF-KAAN-4K",
            "photo_license",
            "RBF Telifli Fotoğraf Arşivi: KAAN & AKINCI Yüksek Çözünürlüklü Kurumsal Kullanım Lisansı",
            "RBF Copyrighted Photo Archive: KAAN & AKINCI High-Res Corporate Commercial License",
            "RBF Fotoğraf Arşivi / UMSA Prodüksiyon",
            "Fuar standı, faaliyet raporu ve kurumsal kataloglarda kullanım hakkı içeren 600 DPI RAW/TIFF telifli havacılık fotoğraf paketi.",
            "600 DPI RAW/TIFF commercial license package for exhibition stands, annual reports, and corporate defense catalogs.",
            2200.0,
            "images/msi/akinci-iha300.webp",
            "8256 × 5504 px • 600 DPI TIFF + Kurumsal Telif Sertifikası"
        ),
    ];

    for pr in products {
        conn.execute(
            "INSERT OR IGNORE INTO commerce_products (
                sku, product_type, title_tr, title_en, author_or_source,
                description_tr, description_en, price_try, cover_image,
                file_resolution_or_pages, is_available
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1)",
            params![pr.0, pr.1, pr.2, pr.3, pr.4, pr.5, pr.6, pr.7, pr.8, pr.9],
        )?;
    }

    Ok(())
}

fn seed_analytics_and_operations(conn: &Connection) -> rusqlite::Result<()> {
    // Seed realistic institutional analytics events across MSB, SSB, TSK Force Commands, and Defense OEMs
    let events: &[(&str, Option<i64>, &str, &str, &str, Option<&str>, Option<i64>, i32, i32, i32, i32, Option<&str>, &str)] = &[
        ("sess-msb-01", Some(2), "MSB", "Millî Savunma Bakanlığı", "reading_depth", Some("kaan-programinda-mevcut-durum-ve-yol-haritasi"), Some(1), 185, 96, 1, 1, Some("hava-araclari"), "2026-11-06 10:12:00"),
        ("sess-ssb-01", Some(3), "SSB", "Savunma Sanayii Başkanlığı", "reading_depth", Some("kaan-programinda-mevcut-durum-ve-yol-haritasi"), Some(1), 210, 100, 1, 1, Some("radarlar-ve-sensorler"), "2026-11-06 10:45:00"),
        ("sess-hvkk-01", Some(5), "Hv.K.K.lığı", "Hava Kuvvetleri Komutanlığı", "reading_depth", Some("kaan-programinda-mevcut-durum-ve-yol-haritasi"), Some(4), 240, 100, 1, 1, Some("hava-araclari"), "2026-11-06 11:20:00"),
        ("sess-oem-01", Some(7), "Sektör Kuruluşları", "ASELSAN A.Ş.", "reading_depth", Some("kaan-programinda-mevcut-durum-ve-yol-haritasi"), Some(4), 160, 88, 1, 1, Some("radarlar-ve-sensorler"), "2026-11-06 12:05:00"),
        ("sess-dzkk-01", Some(4), "Dz.K.K.lığı", "Deniz Kuvvetleri Komutanlığı", "reading_depth", Some("ctech-suru-ida-hibrit-haberlesme-konsepti"), Some(5), 195, 94, 0, 1, Some("deniz-araclari"), "2026-11-06 13:15:00"),
        ("sess-ssb-02", Some(3), "SSB", "Savunma Sanayii Başkanlığı", "reading_depth", Some("ctech-suru-ida-hibrit-haberlesme-konsepti"), Some(3), 175, 90, 0, 1, Some("haberlesme-ve-muharebe-destek-sistemleri"), "2026-11-06 14:00:00"),
        ("sess-jgkk-01", Some(6), "J.G.K.lığı", "Jandarma Genel Komutanlığı", "reading_depth", Some("akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu"), Some(2), 145, 85, 1, 1, Some("muhimmatlar-ve-fuze-sistemleri"), "2026-11-06 15:10:00"),
        ("sess-msb-02", Some(2), "MSB", "Millî Savunma Bakanlığı", "reading_depth", Some("genelkurmay-baskani-idef-ozel-sayisi-mesaji"), Some(1), 130, 100, 0, 1, Some("kara-araclari"), "2026-11-06 16:00:00"),
        ("sess-oem-02", Some(8), "Sektör Kuruluşları", "TUSAŞ", "reading_depth", Some("dr-m-cuneyd-firat-ctech-kapak-soylesisi"), Some(1), 260, 95, 1, 1, Some("haberlesme-ve-muharebe-destek-sistemleri"), "2026-11-06 16:40:00"),
        ("sess-ssb-03", Some(3), "SSB", "Savunma Sanayii Başkanlığı", "reading_depth", Some("nato-hava-savunma-sistemlerinin-gelecegi-analiz"), Some(2), 310, 100, 0, 1, Some("muhimmatlar-ve-fuze-sistemleri"), "2026-11-06 17:30:00"),
        ("sess-msb-ad1", Some(2), "MSB", "Millî Savunma Bakanlığı", "ad_impression", Some("kaan-programinda-mevcut-durum-ve-yol-haritasi"), Some(1), 0, 0, 0, 0, Some("hava-araclari"), "2026-11-06 10:12:05"),
        ("sess-msb-ad1c", Some(2), "MSB", "Millî Savunma Bakanlığı", "ad_click", Some("kaan-programinda-mevcut-durum-ve-yol-haritasi"), Some(1), 0, 0, 0, 0, Some("hava-araclari"), "2026-11-06 10:14:10"),
        ("sess-ssb-ad1", Some(3), "SSB", "Savunma Sanayii Başkanlığı", "ad_impression", Some("kaan-programinda-mevcut-durum-ve-yol-haritasi"), Some(4), 0, 0, 0, 0, Some("radarlar-ve-sensorler"), "2026-11-06 10:45:12"),
        ("sess-ssb-ad1c", Some(3), "SSB", "Savunma Sanayii Başkanlığı", "ad_click", Some("kaan-programinda-mevcut-durum-ve-yol-haritasi"), Some(4), 0, 0, 0, 0, Some("radarlar-ve-sensorler"), "2026-11-06 10:46:30"),
        ("sess-dzkk-ad5", Some(4), "Dz.K.K.lığı", "Deniz Kuvvetleri Komutanlığı", "ad_impression", Some("stm-milgem-istif-sinifi-ve-stm500-denizalti-cozumleri"), Some(5), 0, 0, 0, 0, Some("deniz-araclari"), "2026-11-06 13:16:00"),
        ("sess-dzkk-ad5c", Some(4), "Dz.K.K.lığı", "Deniz Kuvvetleri Komutanlığı", "ad_click", Some("stm-milgem-istif-sinifi-ve-stm500-denizalti-cozumleri"), Some(5), 0, 0, 0, 0, Some("deniz-araclari"), "2026-11-06 13:17:20"),
        ("sess-hvkk-ad2", Some(5), "Hv.K.K.lığı", "Hava Kuvvetleri Komutanlığı", "ad_impression", Some("akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu"), Some(2), 0, 0, 0, 0, Some("muhimmatlar-ve-fuze-sistemleri"), "2026-11-06 15:20:00"),
        ("sess-hvkk-ad2c", Some(5), "Hv.K.K.lığı", "Hava Kuvvetleri Komutanlığı", "ad_click", Some("akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu"), Some(2), 0, 0, 0, 0, Some("muhimmatlar-ve-fuze-sistemleri"), "2026-11-06 15:21:00"),
    ];

    for ev in events {
        conn.execute(
            "INSERT INTO analytics_events (
                session_id, user_id, organization_category, organization_name,
                event_type, content_slug, ad_id, duration_sec, scroll_depth_pct,
                video_completed, is_true_read, ssb_tag, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                ev.0, ev.1, ev.2, ev.3, ev.4, ev.5, ev.6, ev.7, ev.8, ev.9, ev.10, ev.11, ev.12
            ],
        )?;
    }

    // Seed 5-stage renewal reminders (2 months, 1 month, 2 weeks, 1 week, 1 day + halted_by_renewal)
    let reminders: &[(i64, &str, &str, &str, &str, i32, &str, &str)] = &[
        (7, "hakan.erdem@aselsan.com.tr", "Hakan Erdem", "ASELSAN A.Ş.", "2_months", 60, "sent", "2026-10-07 09:00"),
        (7, "hakan.erdem@aselsan.com.tr", "Hakan Erdem", "ASELSAN A.Ş.", "1_month", 30, "sent", "2026-11-07 09:00"),
        (8, "zeynep.ucar@tusas.com", "Zeynep Uçar", "TUSAŞ", "2_weeks", 14, "sent", "2026-11-07 09:15"),
        (8, "zeynep.ucar@tusas.com", "Zeynep Uçar", "TUSAŞ", "1_week", 7, "scheduled", "2026-11-14 09:00"),
        (8, "zeynep.ucar@tusas.com", "Zeynep Uçar", "TUSAŞ", "1_day", 1, "scheduled", "2026-11-20 09:00"),
        (9, "baris.koc@stm.com.tr", "Barış Koç", "STM Savunma Teknolojileri", "1_month", 30, "halted_by_renewal", "2026-11-02 16:40"),
    ];

    for rm in reminders {
        conn.execute(
            "INSERT INTO renewal_reminders (
                user_id, user_email, full_name, organization_name,
                stage_label, days_remaining, status, sent_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![rm.0, rm.1, rm.2, rm.3, rm.4, rm.5, rm.6, rm.7],
        )?;
    }

    // Seed sample invoices
    conn.execute(
        "INSERT OR IGNORE INTO order_invoices (
            invoice_no, user_email, full_name, organization_name,
            item_type, item_code, item_title, amount_try, payment_status,
            created_at, pdf_watermark_code
        ) VALUES (
            'MSI-2026-104821', 'baris.koc@stm.com.tr', 'Barış Koç', 'STM Savunma Teknolojileri',
            'subscription', 'dijital', 'Dijital E-Dergi & Arşiv Paketi (Yıllık)', 1200.0, 'PAID_VIRTUAL_POS',
            '2026-11-02 16:39', 'MSI-DRM-2026-STM8842'
        )",
        [],
    )?;

    // Seed newsletter campaigns
    let newsletters: &[(&str, &str, &str, &str, &str, Option<i64>, Option<&str>, i64, &str, &str)] = &[
        (
            "MSI Dergisi 280. Sayı Özel Özeti & KAAN Aviyonik Dosyası",
            "MSI TDR Issue 280 Executive Digest & KAAN Avionics Dossier",
            "Tedarik Makamları ve Kuvvet Komutanlıkları",
            "MSB & SSB & Hv.K.K.lığı",
            "hava-araclari",
            Some(1),
            Some("TUSAŞ"),
            4820,
            "sent",
            "2026-11-01 09:30"
        ),
        (
            "Savunma Sanayii Kariyer, Staj ve Öğrenci Kulüpleri Özel Bülteni",
            "Defense Industry Career, Internship & University Clubs Special Bulletin",
            "Üniversite Savunma Kulüpleri ve Mezun Adayları",
            "Üniversite / Öğrenci",
            "simulatorler-ve-egitim-sistemleri",
            Some(4),
            Some("ASELSAN"),
            3150,
            "sent",
            "2026-11-04 11:00"
        ),
    ];

    for nl in newsletters {
        conn.execute(
            "INSERT INTO newsletter_campaigns (
                subject_tr, subject_en, target_group_filter, target_institution,
                target_ssb_tag, sponsor_ad_id, sponsor_name, recipient_count, status, sent_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![nl.0, nl.1, nl.2, nl.3, nl.4, nl.5, nl.6, nl.7, nl.8, nl.9],
        )?;
    }

    // Seed 301 SEO redirects for legacy URLs
    let redirects: &[(&str, &str, i32, i64)] = &[
        ("/tr/dergi/280", "/e-dergi/280", 301, 412),
        ("/haber/kaan-programinda-mevcut-durum", "/icerik/kaan-programinda-mevcut-durum-ve-yol-haritasi", 301, 895),
        ("/soylesi/dr-m-cuneyd-firat", "/icerik/dr-m-cuneyd-firat-ctech-kapak-soylesisi", 301, 328),
        ("/en/msi-tdr", "/?lang=en", 301, 640),
        ("/savunmahaber", "/#haber", 301, 1120),
        // Legacy .html URLs → canonical paths (301)
        ("/detail-1.html", "/icerik/kaan-programinda-mevcut-durum-ve-yol-haritasi", 301, 0),
        ("/detail-2.html", "/icerik/akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu", 301, 0),
        ("/detail-3.html", "/icerik/ctech-suru-ida-hibrit-haberlesme-konsepti", 301, 0),
        ("/index.html", "/", 301, 0),
        ("/about.html", "/kurumsal/hakkimizda", 301, 0),
        ("/contact.html", "/kurumsal/iletisim", 301, 0),
        ("/gallery.html", "/e-dergi", 301, 0),
        ("/gallery1.html", "/e-dergi", 301, 0),
        ("/technology.html", "/arama", 301, 0),
        ("/lifestyle.html", "/arama", 301, 0),
        ("/travel.html", "/arama", 301, 0),
    ];

    for rd in redirects {
        conn.execute(
            "INSERT OR IGNORE INTO seo_redirects (old_path, new_path, status_code, hit_count)
             VALUES (?1, ?2, ?3, ?4)",
            params![rd.0, rd.1, rd.2, rd.3],
        )?;
    }

    Ok(())
}

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
            "22 Years of Authority in Defense & Aerospace Intelligence.",
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
            "#c4122f",
            "#c4122f",
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
            "Cankaya, Ankara / Turkiye",
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
            "/images/logo.png",
            "/images/logo.png",
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
            "MSI TDR, Turkish Defence Review, Turkish defense industry, aerospace, naval platforms, UAV, missiles, radar, electronic warfare",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "rss_title_tr",
            "MSI Dergisi — Savunmahaber.com | Savunma, Havacılık ve Güvenlik Teknolojileri",
            "MSI TDR — Turkish Defence Review | Defense & Aerospace Intelligence",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "rss_description_tr",
            "MSI Dergisi (ISSN 1306-0309) ve savunmahaber.com tekil içerik havuzundan güncel savunma sanayii haberleri, özel dosyalar, söyleşiler ve analizler.",
            "Authoritative analysis, Turkish defense industry news, special dossiers, and executive interviews from MSI Turkish Defence Review (ISSN 1306-0309).",
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
            "Search the MSI Dergisi & MSI TDR defense, aviation and security technologies archive.",
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
            "Strategic reference publication in defense, aviation and security technologies since 2004 (ISSN 1306-0309).",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "llms_header_text",
            "Türkiye'nin 2004 yılından bu yana kesintisiz yayımlanan lider Savunma, Havacılık, Uzay ve Denizcilik Teknolojileri referans yayını (ISSN: 1306-0309). Yayıncı: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. (Ankara, Türkiye).",
            "Turkey's leading uninterrupted Defense, Aviation, Space and Maritime Technologies reference publication since 2004 (ISSN: 1306-0309). Publisher: UMSA Production Printing and Publishing Ltd. Co. (Ankara, Turkey).",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "category_fallback_desc",
            "MSI Dergisi savunma sanayii, havacılık ve güvenlik teknolojileri içerik arşivi.",
            "MSI TDR defense industry, aviation and security technologies content archive.",
            "seo",
            "2026-11-11 00:00:00",
        ),
        (
            "json_feed_description",
            "2004'ten bugüne Türk savunma ve havacılık sanayisinin stratejik referans yayını (ISSN 1306-0309).",
            "Strategic reference publication of the Turkish defense and aviation industry since 2004 (ISSN 1306-0309).",
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

    let notif_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM system_notifications", [], |r| r.get(0))
        .unwrap_or(0);

    if notif_count == 0 {
        let notifications: &[(
            &str,
            Option<&str>,
            &str,
            &str,
            &str,
            &str,
            &str,
            &str,
            Option<&str>,
            i32,
            i32,
            &str,
        )] = &[
            (
                "all",
                None,
                "announcement_banner",
                "highlight",
                "280. Sayı ve SAHA EXPO Özel Sayısı Yayında!",
                "Issue 280 & SAHA EXPO Special Edition Live!",
                "280. Sayı (Eylül 2026) ve SAHA EXPO Özel Sayısı Dijital Kiosk'ta Yayında! Kamu (.gov.tr / .tsk.tr) Personeline Ücretsiz E-Dergi Aboneliği.",
                "Issue 280 & SAHA EXPO Special Edition Live in Digital Kiosk! Free E-Magazine Subscription for Public/Military (.gov.tr / .tsk.tr) Personnel.",
                Some("/e-dergi"),
                0,
                1,
                "2026-11-07 09:00:00",
            ),
            (
                "user",
                Some("murat.yilmaz@msb.gov.tr"),
                "gov_verification",
                "success",
                "Kamu (.gov.tr) Kurumsal Aboneliğiniz Otomatik Onaylandı",
                "Public (.gov.tr) Institutional Subscription Auto-Verified",
                "Millî Savunma Bakanlığı (murat.yilmaz@msb.gov.tr) kurumsal e-posta doğrulamanız tamamlandı. Tüm E-Dergi sayılarına ve DRM filigranlı PDF arşivine ücretsiz erişebilirsiniz.",
                "Your Ministry of National Defense (murat.yilmaz@msb.gov.tr) verification is complete. Full complimentary access to E-Magazine and DRM PDF archive is active.",
                Some("/e-dergi"),
                0,
                0,
                "2026-11-07 09:15:00",
            ),
            (
                "user",
                Some("zeynep.ucar@tusas.com"),
                "renewal_reminder",
                "warning",
                "Abonelik Yenileme Hatırlatması (3. Aşama: Son 14 Gün)",
                "Subscription Renewal Notice (Stage 3: 14 Days Remaining)",
                "Sayın Zeynep Uçar (TUSAŞ), Basılı + Dijital Tam Erişim aboneliğinizin süresi 14 gün içinde dolacaktır. Kesintisiz erişim için yenileme işleminizi tamamlayabilirsiniz.",
                "Dear Zeynep Ucar (TUSAS), your Print + Digital subscription expires in 14 days. Renew now to automatically halt remaining reminders.",
                Some("/abonelik-ve-satis"),
                0,
                0,
                "2026-11-07 09:30:00",
            ),
            (
                "admin",
                Some("operator@msidergisi.com"),
                "corporate_approval_pending",
                "warning",
                "Operatör Onayı Bekleyen Kurumsal Başvuru (2 Adet)",
                "Pending Corporate Verification Requests (2 Items)",
                "TUSAŞ ve ROKETSAN kurumsal e-posta uzantılı 2 okuyucu ücretsiz kamu/sektör abonelik tanımlaması için operatör onayı bekliyor.",
                "2 corporate readers from TUSAS and ROKETSAN are awaiting operator verification in the OMS queue.",
                Some("/admin"),
                0,
                0,
                "2026-11-07 09:45:00",
            ),
            (
                "subscribers",
                None,
                "new_issue_release",
                "info",
                "KAAN 5. Nesil Aviyonik ve Sürü İDA Dosyaları Eklendi",
                "KAAN 5th-Gen Avionics & Swarm USV Dossiers Published",
                "MSI Dergisi 280. Sayı kapak dosyası ve NATO Katmanlı Hava Savunma Analizi yüksek çözünürlüklü şemalarla yayına alındı.",
                "MSI Issue 280 cover story and NATO Multi-Layered Air Defense Analysis are now available with high-resolution diagrams.",
                Some("/icerik/kaan-programinda-mevcut-durum-ve-yol-haritasi"),
                0,
                0,
                "2026-11-07 10:00:00",
            ),
        ];

        for n in notifications {
            conn.execute(
                "INSERT INTO system_notifications (
                    target_audience, target_email, notification_type, severity,
                    title_tr, title_en, message_tr, message_en,
                    action_url, is_read, is_active_banner, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![n.0, n.1, n.2, n.3, n.4, n.5, n.6, n.7, n.8, n.9, n.10, n.11],
            )?;
        }
    }

    let audit_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM security_audit_logs", [], |r| r.get(0))
        .unwrap_or(0);

    if audit_count == 0 {
        let audits: &[(&str, &str, &str, &str, &str, i32, &str)] = &[
            (
                "127.0.0.1",
                "/system/waf-init",
                "WAF_SHIELD_INITIALIZED",
                "INFO",
                "Active Security Shield Middleware (SQLi/XSS/Path-Traversal WAF + CSP/HSTS/X-Frame-Options) başarıyla devreye alındı.",
                0,
                "2026-11-07 08:55:00",
            ),
            (
                "127.0.0.1",
                "/api/v1/auth/register",
                "KVKK_SHA256_VERIFIER_ACTIVE",
                "INFO",
                "KVKK Açık Rıza SHA-256 kriptografik imza zinciri ve zaman damgası doğrulama motoru aktif.",
                0,
                "2026-11-07 08:56:00",
            ),
            (
                "127.0.0.1",
                "/api/v1/*",
                "RATE_LIMITER_ACTIVE",
                "LOW",
                "IP bazlı DDoS/Brute-Force hız sınırlayıcı (Rate-Limiter) ve dinamik DRM PDF filigran denetimi çalışıyor.",
                0,
                "2026-11-07 08:57:00",
            ),
        ];

        for a in audits {
            conn.execute(
                "INSERT INTO security_audit_logs (
                    client_ip, request_path, event_type, threat_level, details, blocked, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![a.0, a.1, a.2, a.3, a.4, a.5, a.6],
            )?;
        }
    }

    Ok(())
}


fn seed_live_msi_harvested_data(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 4200, 3444)",
        params!["milli-anadolu-ankasi-2026-tatbikati-tamamlandi", "Millî Anadolu Ankası-2026 Tatbikatı Tamamlandı", "Şehit Hava Pilot Binbaşı İbrahim Bolat Millî Anadolu Ankası-2026 Tatbikatı, 26 Ekim-6 Kasım tarihleri arasında Konya'daki 3'üncü Ana Jet Üs Komutanlığında gerçekleştirildi. Personel kurtarma harekâtına yönelik eğitimlerin icra edildiği tatbikat başarıyla tamamlandı.", "<p></p><p>Tatbikata Kara, Deniz ve Hava Kuvvetleri Komutanlıkları, Jandarma Genel Komutanlığı, Genelkurmay Özel Kuvvetler Komutanlığı ve KKTC Güvenlik Kuvvetleri Komutanlığı unsurları katıldı. Farklı kuvvet unsurlarının müşterek olarak görev aldığı tatbikatta, arama ve kurtarma timleri ile uçak ve helikopterlerin katılımıyla personel kurtarma eğitimleri gerçekleştirildi.</p>", "haberler", "gunluk-haber", "images/msi/milli-anadolu-ankasi-2026-tatbikati-tamamlandi.webp", "Haber Merkezi", "2026-11-07", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["milli-anadolu-ankasi-2026-tatbikati-tamamlandi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 8, 0, 0.0, 0, ?10, ?11, 1, NULL, 4445, 3644)",
        params!["mkenin-tolga-sistemi-deniz-hedeflerine-karsi-etkinligini-b", "MKE'nin TOLGA Sistemi, Deniz Hedeflerine Karşı Etkinliğini Başarılı Atışlarla Gösterdi", "MKE tarafından geliştirilen TOLGA 20 mm çift silahlı revolver sistemi, TURNAs-2026 İnsansız Deniz Sistemleri Tatbikatı kapsamında TCG BEYKOZ'dan gerçekleştirilen atışlarda hava ve su üstü hedeflerini başarıyla imha etti. MKE, tatbikatta kabiliyetlerini ortaya koyan sistemin askerî gemilerin yanı sıra stratejik öneme sahip ticari ve sivil gemilerin korunmasında da kullanılmaya hazır olduğunu belirtti.", "<p></p><p>Dakikada 3.200 atım kapasitesine sahip olan sistem, parçacıklı anti-dron mühimmatıyla 1.000 metre etkili menzildeki tehditlere karşı görev yapabiliyor. Yüksek stabilizasyon kabiliyeti sayesinde gemi hareket hâlindeyken hedefe yönelimini koruyabilen sistemin; derin sondaj, sismik araştırma, kablo döşeme ve yüzer elektrik üretim gemilerine de entegre edilebileceği belirtiliyor.</p><img data-type=\"image-upload\" src=\"/images/msi/mkenin-tolga-sistemi-deniz-hedeflerine-karsi-etkinligini-b-inline-1.webp\" alt=\"MKE TOLGA 20 mm Deniz Atış Testi - 1\" class=\"img-fluid rounded my-3\"><p>TOLGA 20 mm çift silahlı revolver sistemi, tatbikattaki başarılı atışlarının ardından 3-6 Kasım tarihlerinde Paris'te düzenlenen EURONAVAL 2026'da ilk kez bir fuarda sergilendi.</p><img data-type=\"image-upload\" src=\"/images/msi/mkenin-tolga-sistemi-deniz-hedeflerine-karsi-etkinligini-b-inline-2.webp\" alt=\"MKE TOLGA 20 mm Deniz Atış Testi - 2\" class=\"img-fluid rounded my-3\"><p></p>", "haberler", "gunluk-haber", "images/msi/mkenin-tolga-sistemi-deniz-hedeflerine-karsi-etkinligini-b.webp", "Haber Merkezi", "2026-11-07", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["mkenin-tolga-sistemi-deniz-hedeflerine-karsi-etkinligini-b"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 4690, 3845)",
        params!["hisar-a-otonom-jet-motorlu-hedefi-vurdu", "HİSAR A+ Otonom, Jet Motorlu Hedefi Vurdu", "HİSAR A+ Otonom Alçak İrtifa Hava Savunma Sistemi, gerçekleştirilen atış testinde jet motorlu hedefi başarıyla vurdu. Atışta, sistemden ateşlenen görüntüleyici kızılötesi (IIR) arayıcı başlıklı füze kullanıldı.", "<p>Savunma Sanayii Başkanlığı (SSB) koordinasyonunda yürütülen HİSAR projesi kapsamındaki atış, Başkanlığın 41. kuruluş yıl dönümünde gerçekleştirildi. Faaliyete ilişkin açıklama yapan Savunma Sanayii Başkanı Prof. Dr. Haluk Görgün, sistemin sahadaki etkinliğini bir kez daha kanıtladığını belirtti.</p><p>ASELSAN ve ROKETSAN'ın ana yükleniciliğinde geliştirilen HİSAR A+ Otonom, hedef tespit, teşhis, takip, atış ve komuta kontrol fonksiyonlarını tek araç üzerinde bir araya getiriyor. Türk Silahlı Kuvvetlerinin envanterinde bulunan sistemin geliştirilmesine 200'den fazla yerli savunma sanayisi firması katkı sağladı.</p>", "haberler", "gunluk-haber", "images/msi/hisar-a-otonom-jet-motorlu-hedefi-vurdu.webp", "Haber Merkezi", "2026-11-07", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["hisar-a-otonom-jet-motorlu-hedefi-vurdu"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "radarlar-ve-sensorler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 283, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 4935, 4046)",
        params!["kale-jet-motorlari-1000inci-milli-turbojet-motorunu-uretti", "Kale Jet Motorları, 1000'inci Millî Turbojet Motorunu Üretti", "Kale Jet Motorları tarafından üretilen KTJ serisi millî turbojet motorların 1000'incisinin üretimi tamamlandı. Bu başarı dolayısıyla Savunma Sanayii Başkanlığında (SSB), Savunma Sanayii Başkanı Prof. Dr. Haluk Görgün'ün katılımıyla tören düzenlendi.", "<p>Törende konuşan Prof. Dr. Görgün, 1000 motorun yerli üretimi sayesinde en az 200 milyon dolarlık kaynağın Türkiye'de kalmasının sağlandığını belirterek seri üretim kapasitesinin artmasıyla bu rakamın daha da yükseleceğini ifade etti.</p><p>Kale Jet Motorları tarafından geliştirilen KTJ-3200, SOM Seyir Füzesi ve ATMACA Gemisavar Füzesi'ne güç sağlarken KTJ-1750, ÇAKIR Seyir Füzesi'nde; KTJ-3700 ise KARA ATMACA Seyir Füzesi'nde kullanılıyor.</p>", "haberler", "gunluk-haber", "images/msi/kale-jet-motorlari-1000inci-milli-turbojet-motorunu-uretti.webp", "Haber Merkezi", "2026-11-06", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["kale-jet-motorlari-1000inci-milli-turbojet-motorunu-uretti"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 282, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 5180, 4247)",
        params!["bayraktar-tb2-fpv-kamikaze-dronlarla-hedefleri-vurdu", "Bayraktar TB2, FPV Kamikaze Dronlarla Hedefleri Vurdu", "Baykar tarafından geliştirilen Bayraktar TB2 SİHA, Skydagger FPV kamikaze dronlarla gerçekleştirilen testte belirlenen hedefleri başarıyla vurdu. Edirne'nin Keşan ilçesindeki Uçuş Eğitim ve Test Merkezi'nden 16 FPV kamikaze dron taşıyarak havalanan Bayraktar TB2, test sahasında iki dronu havadan bıraktı.", "<p></p><p></p><p>Bayraktar TB2 üzerinden kurulan bağlantı aracılığıyla operatörler tarafından kontrol edilen FPV kamikaze dronlar, belirlenen hedefleri tam isabetle vurdu. Test kapsamında Bayraktar TB2'nin FPV kamikaze dron taşıma ve bırakma kabiliyeti gösterildi.</p>", "haberler", "gunluk-haber", "images/msi/bayraktar-tb2-fpv-kamikaze-dronlarla-hedefleri-vurdu.webp", "Haber Merkezi", "2026-11-04", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["bayraktar-tb2-fpv-kamikaze-dronlarla-hedefleri-vurdu"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "simulatorler-ve-egitim-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 281, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 5425, 4448)",
        params!["2-hurkus-ii-daha-hava-kuvvetlerinde", "2 HÜRKUŞ-II Daha Hava Kuvvetlerinde", "Türk Havacılık ve Uzay Sanayii (TUSAŞ) tarafından geliştirilen yeni nesil temel eğitim uçağı HÜRKUŞ-II'nin 2 adedinin daha kabul işlemleri tamamlandı. Böylece Hava Kuvvetleri Komutanlığı envanterine kazandırılan HÜRKUŞ-II sayısı 4'e yükseldi.", "<p></p><p></p><p>İlk 2 HÜRKUŞ-II'nin 28 Eylül'de teslim edildiği projede, Hava Kuvvetleri Komutanlığına toplam 55 uçağın teslim edilmesi planlanıyor. 2026 yılı içerisinde 12 uçağın teslim edilmesinin hedeflendiği projede, teslimatların 2028 yılında tamamlanması öngörülüyor.</p>", "haberler", "gunluk-haber", "images/msi/2-hurkus-ii-daha-hava-kuvvetlerinde.webp", "Haber Merkezi", "2026-11-04", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["2-hurkus-ii-daha-hava-kuvvetlerinde"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "simulatorler-ve-egitim-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 280, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 5670, 4649)",
        params!["savunma-ve-havacilik-sanayisi-2026nin-en-yuksek-aylik-ihra", "Savunma ve Havacılık Sanayisi, 2026'nın En Yüksek Aylık İhracatına Ulaştı", "Türk savunma ve havacılık sanayisi, 2026 yılının ekim ayında 1 milyar 140 milyon dolarlık ihracat gerçekleştirerek yılın en yüksek aylık ihracat rakamına ulaştı.", "<p>Sektörün ihracatı, geçen yılın aynı ayına oranla yüzde 61 arttı. Böylece temmuz ayında kaydedilen 1 milyar 120 milyon dolarlık ihracat seviyesi geride bırakılırken 2026 yılında ikinci kez aylık ihracatta 1 milyar dolar eşiği aşıldı.</p><p>Savunma Sanayii Başkanı Prof. Dr. Haluk Görgün tarafından açıklanan verilere göre, sektörün ocak-ekim dönemindeki toplam ihracatı, geçen yılın aynı dönemine kıyasla yüzde 21 artarak 8 milyar 50 milyon dolara ulaştı.</p><p>Son 12 aylık dönemde gerçekleştirilen savunma ve havacılık sanayisi ihracatı ise yüzde 37 artışla 11 milyar 357 milyon dolar olarak gerçekleşti.</p>", "haberler", "gunluk-haber", "images/msi/savunma-ve-havacilik-sanayisi-2026nin-en-yuksek-aylik-ihra.webp", "Haber Merkezi", "2026-11-03", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["savunma-ve-havacilik-sanayisi-2026nin-en-yuksek-aylik-ihra"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 279, ?9, 5, 0, 0.0, 0, ?10, ?11, 1, NULL, 5915, 4850)",
        params!["mke-urdunde-ikinci-tesisin-kurulumunu-tamamladi", "MKE, Ürdün’de İkinci Tesisin Kurulumunu Tamamladı", "MKE, Ürdün’de kurduğu ikinci mühimmat tesisini devreye almaya hazırlanıyor. Orta ve büyük kalibreli mühimmat ile uçak bombalarının dolum ve montajının yapılacağı tesisin kurulum çalışmaları tamamlandı. Şirketin ülkede kurduğu ilk tesis olan fişek üretim tesisi ise 2024 yılından bu yana faaliyet gösteriyor.", "<p>Dost ve müttefik ülkelerin yerel üretim altyapılarının geliştirilmesine yönelik çalışmalarını sürdüren MKE, bugüne kadar 9 ülkede toplam 11 teknoloji transferi projesine imza attı. Anahtar teslim fabrika kurulumunun yanı sıra tesislerde görev alacak personele üretim, bakım-onarım eğitimleri ve teknik asistanlık hizmeti de sağlayan şirket, Ürdün’deki mevcut tesislere yönelik bakım sözleşmesi ve yeni ham madde üretim tesislerinin kurulması için görüşmelerini sürdürüyor.</p><p>MKE, 27-29 Ekim tarihlerinde Ürdün’ün Akabe kentinde düzenlenen SOFEX 2026 Uluslararası Savunma Sanayii Fuarı’nda da ürünlerini ve teknoloji transferi kabiliyetlerini tanıttı. Fuarda MPT-76 MH Millî Piyade Tüfeği ve KN-12 keskin nişancı tüfeği başta olmak üzere çeşitli hafif silahlarını sergileyen şirketin standında, farklı kalibrelerde fişekler, 25 mm’den 120 mm’ye kadar mühimmat maketleri ve MK82 uçak bombası da yer aldı.</p><p>Fuar boyunca farklı ülkelerden üst düzey askerî heyetleri ve savunma sanayisi temsilcilerini ağırlayan MKE, yeni ihracat fırsatları ve teknoloji transferine yönelik iş birliği imkânları üzerine görüşmeler gerçekleştirdi.</p>", "haberler", "gunluk-haber", "images/msi/mke-urdunde-ikinci-tesisin-kurulumunu-tamamladi.webp", "Haber Merkezi", "2026-10-29", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["mke-urdunde-ikinci-tesisin-kurulumunu-tamamladi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "simulatorler-ve-egitim-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 278, ?9, 6, 0, 0.0, 0, ?10, ?11, 1, NULL, 6160, 5051)",
        params!["turnas-2026da-haberlesme-sistemleri-cteche-emanet", "TURNAs-2026’da Haberleşme Sistemleri CTech’e Emanet", "CTech’in uydu ve görüş hattı haberleşme çözümleri, TURNAs-2026 insansız deniz sistemleri tatbikatında kara, deniz ve hava platformlarında görev aldı. 19–27 Ekim’de Antalya’da düzenlenen tatbikatın seçkin gözlemci gününde Deniz Kuvvetleri Komutanı Oramiral Ercüment Tatlıoğlu da CTech standını ziyaret etti.", "<p></p><p></p><p>Tatbikatta TUSAŞ’ın AKSUNGUR ve ANKA insansız hava araçlarında SkyARX Uydu Haberleşme Terminali kullanıldı. ULAQ, SANCAR ve SALVO insansız deniz araçlarının uydu haberleşmesi AquaARX ile sağlanırken SEFİNE’nin RD 25 insansız deniz aracında SkyARX E-Ku elektronik taramalı uydu haberleşme anteni görev aldı. Bu sistemler, platformların görüş hattı ötesinden komuta ve kontrol edilmesine, canlı görüntü ve görev verilerinin kontrol merkezlerine aktarılmasına imkân tanıdı.</p><img data-type=\"image-upload\" src=\"/images/msi/turnas-2026da-haberlesme-sistemleri-cteche-emanet-inline-1.webp\" alt=\"TURNAs-2026 CTech Haberleşme Sistemleri\" class=\"img-fluid rounded my-3\"><p>CTech’in UfukLink Mini Pro görüş hattı haberleşme çözümü ise AKSUNGUR insansız hava aracında, HAVELSAN’ın BARKAN ve TEKATRON’un PALEM insansız kara araçlarında kullanıldı. Şirket, tatbikatta görev alan haberleşme ürünlerini sergi alanındaki standında da tanıttı.</p><p>CTech Genel Müdürü Dr. Cüneyd Fırat, insansız araçların görevlerini yerine getirmesinde güvenilir haberleşmenin kritik rol üstlendiğini belirtti. Ürünlerinin aynı tatbikatta farklı platformlarda görev almasının önemine değinen Fırat, platform üreticileriyle iş birliği içinde Türkiye’nin insansız sistemlerinin haberleşme kabiliyetlerini geliştirmeye devam edeceklerini ifade etti.</p>", "haberler", "gunluk-haber", "images/msi/turnas-2026da-haberlesme-sistemleri-cteche-emanet.webp", "Haber Merkezi", "2026-10-26", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["turnas-2026da-haberlesme-sistemleri-cteche-emanet"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 5, 0, 0.0, 0, ?10, ?11, 1, NULL, 6405, 5252)",
        params!["oramiral-tatlioglu-gelecegin-deniz-savaslari-insansiz-arac", "Oramiral Tatlıoğlu: “Geleceğin Deniz Savaşları İnsansız Araçlarla Yapılacaktır”", "Deniz Kuvvetleri Komutanı Oramiral Ercüment Tatlıoğlu, insansız sistemlerin deniz harekâtındaki önemine dikkat çekerek insansız deniz araçlarının sayısını hızla artırdıklarını söyledi. Oramiral Tatlıoğlu, 25 Ekim’de Antalya’da gerçekleştirilen TURNAs-2026 tatbikatının seçkin gözlemci günü faaliyetlerinin ardından açıklamalarda bulundu.", "<p>Türk Deniz Kuvvetlerinin insansız hava araçları açısından dünyanın önde gelen deniz kuvvetleri arasında yer aldığını belirten Oramiral Tatlıoğlu, “Geleceğin deniz savaşları insansız araçlarla yapılacaktır.” dedi. Günümüz çatışmalarından çıkarılan derslere de değinen Oramiral Tatlıoğlu, düşük maliyetli, süratle geliştirilebilen ve farklı görevlere uyarlanabilen insansız sistemlerin önemli askerî sonuçlar doğurabileceğini ifade etti.</p><p>Oramiral Tatlıoğlu, insansız sistemlerin geniş deniz sahalarında daha uzun süre gözetleme yapılmasına, farklı noktalardan bilgi edinilmesine ve tehlikeli görevlerde personelin karşılaştığı risklerin azaltılmasına katkı sağladığını kaydetti. Karadeniz’de sürüklenen mayınların tespit ve imhasında da insansız araçlardan mümkün olan en yüksek seviyede yararlandıklarını belirtti.</p><p>Yakın gelecekte insansız sualtı araçlarını ve mini denizaltıları envantere kazandırmayı hedeflediklerini açıklayan Oramiral Tatlıoğlu, millî uçak gemisi, Tepe sınıfı muhrip ve yeni nesil mayın avlama gemisi gibi platformların da insansız sistemlerle hibrit harekât icra edecek şekilde tasarlandığını söyledi.</p><p>Oramiral Tatlıoğlu, insanlı ve insansız sistemlerin birlikte kullanımının denendiği TURNAs-2026’yı, Deniz Kuvvetlerine gelecekte icra edeceği harekâtlar açısından tecrübe kazandıran tarihî bir tatbikat olarak nitelendirdi.</p>", "haberler", "gunluk-haber", "images/msi/oramiral-tatlioglu-gelecegin-deniz-savaslari-insansiz-arac.webp", "Haber Merkezi", "2026-10-26", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["oramiral-tatlioglu-gelecegin-deniz-savaslari-insansiz-arac"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 5, 0, 0.0, 0, ?10, ?11, 1, NULL, 6650, 5453)",
        params!["han-yonetim-kurulu-baskani-melih-han-bilgin", "HAN Yönetim Kurulu Başkanı Melih Han BİLGİN:", "“HAN’ın görev tanımını; yüksek teknolojiye sahip savunma sistemlerini çok hızlı ve maliyet etkin bir şekilde üreterek dünya pazarlarına sunabilmek olarak tanımlayabiliriz.”", "<p><em>Savunma ve havacılık sektörünün yenilikçi çözümler geliştiren oyuncularından biri olan HAN Sistem Teknoloji Mühendislik (HAN); yeniliklerin, teknolojik gelişmelerin yanı sıra iş yapış tarzıyla da gerçekleştirilebileceğini gösteriyor. Günümüzde yaşanan çatışmalar, yeni silah sistemlerinin, klasik proje döngülerinden çok daha hızlı bir şekilde ortaya konması gerektiği gerçeğini ortaya çıkardı. Ayrıca tehditlerin düşük maliyetlerle üretilebilmesi, bunların karşı tedbirlerinin de aynı şekilde düşük maliyetli olmasını gerektiriyor. İş modelini bu yeni gerçekler üzerine kurgulayan HAN, ortaya çıkan yeni ihtiyaçlara, ilk çözümü sunan şirket olmayı hedefliyor. Bu doğrultuda HAN’ın odaklandığı ilk alan, anti dron çözümleri oldu. Şirket, aralarında patentli ürünlerin de bulunduğu bu alandaki çözümlerini, ilk kez SAHA 2026’da sergiledi. Sorularımızı yanıtlayan HAN Yönetim Kurulu Başkanı Melih Han Bilgin; şirketin iş modelini, bu iş modelinin arkasında yer alan motivasyonu ve sektör tecrübesini ve şirketin mevcut ürün ailesini, MSI Dergisi okuyucuları için anlattı.</em></p><p> </p><p><strong>MSI Dergisi: </strong>Melih Han Bey, size ilk olarak HAN’ın kuruluş hikayesini sormak istiyoruz. Bu süreci anlatırken HAN’ın arkasındaki tecrübeye de değinebilir misiniz?...</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/han-yonetim-kurulu-baskani-melih-han-bilgin.webp", "MSI Dergisi Yayın Kurulu", "2026-10-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["han-yonetim-kurulu-baskani-melih-han-bilgin"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 5, 0, 0.0, 0, ?10, ?11, 1, NULL, 6895, 5653)",
        params!["databoss-genel-muduru-prof-dr-s-serdar-kozat", "DataBoss Genel Müdürü Prof. Dr. S. Serdar KOZAT:", "“Artık uzaktan kontrol edilebilen platformları, silah sistemlerini, cihazları ve sensörleri, insan operatörler yerine yapay zekâ yazılımlarının kullanacağı; hem sahadaki hem de masa başındaki birçok işin, yapay zekâ yazılımları tarafından hızlı bir şekilde yerine getirilebileceği, oldukça dinamik bir döneme girdik.”", "<p><em>Yapay zekâ teknolojilerinin gelişim hızı, ancak bu gelişime ayak uydurabilecek kuruluşlarla katma değerli hale getirilebiliyor. Türk savunma ve havacılık sektörünün yapay zekâ alanındaki öncü firması olan DataBoss, yapay zekâ alanında neler yapılması gerektiğinin canlı bir örneğini teşkil ediyor. Akademik bağları da çok güçlü olan firma, yapay zekâ teknolojilerinin yanı sıra bu teknolojiyi sahada kullanan ürünler de geliştiriyor. DataBoss Genel Müdürü Prof. Dr. S. Serdar Kozat, kendisiyle yaptığımız söyleşide hem güncel gelişmelerle ilgili sektöre yol gösterecek önemli mesajlar verdi hem de bu mesajların gereğini yaparak geliştirdikleri Yapay Zekâ Tabanlı Otonom Sürü Dron Yönetim Sistemi’ni anlattı.</em></p><p> </p><p><strong>MSI Dergisi:</strong> Serdar Bey, sizinle bu söyleşi için bir araya gelmişken öncelikle yapay zekâ alanındaki güncel gelişmelerle ilgili değerlendirmelerinizi öğrenebilir miyiz? Sektörün dikkatini çekmek istediğiniz hususlar var mı?...</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/databoss-genel-muduru-prof-dr-s-serdar-kozat.webp", "MSI Dergisi Yayın Kurulu", "2026-09-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["databoss-genel-muduru-prof-dr-s-serdar-kozat"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "radarlar-ve-sensorler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 283, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 7140, 5854)",
        params!["ctech-genel-muduru-dr-cuneyd-firat", "CTech Genel Müdürü Dr. Cüneyd Fırat:", "“SkyARX E-Ku’nun hikayesini, ‘Özgün platformlarını ortaya koymaya başlayan Türk savunma ve havacılık sanayisinin, uydu muhaberesi alanında ortaya çıkan özel bir ihtiyacının, bu alanda öncü firması CTech tarafından karşılanması’ şeklinde özetleyebiliriz.”", "<p><em>CTech, uydu muhaberesine yönelik ürün ailesine, SkyARX E-Ku uydu haberleşme antenini de ekledi. Geleneksel çanak antenlerden farklı olarak düz bir panel formuna sahip bu anten, CTech’in özgün ve yenilikçi teknolojilerini barındırıyor. SkyARX E-Ku, fiziksel formu sayesinde, yüksek hızlara ulaşabilen ve düşük radar izine sahip olan hava platformlarında kullanılabiliyor. Böylece Türkiye’nin geliştirdiği ANKA III gibi stratejik platformların uydu muhabere ihtiyacını karşılayabiliyor. CTech Genel Müdürü Dr. Cüneyd Fırat, SkyARX E-Ku’nun özelliklerini ve firmanın uydu muhaberesine yönelik ürün ailesinin güncel durumunu, MSI Dergisi okuyucuları için anlattı.</em></p><p> </p><p><strong>MSI Dergisi:</strong> CTech, hava platformlarına yönelik ürün ailesine, SkyARX E-Ku uydu haberleşme antenini ekledi. SkyARX E-Ku’yu diğer anten çözümlerinden ayıran özellikleri nelerdir?...</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/ctech-genel-muduru-dr-cuneyd-firat.webp", "MSI Dergisi Yayın Kurulu", "2026-08-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["ctech-genel-muduru-dr-cuneyd-firat"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "radarlar-ve-sensorler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 282, ?9, 5, 0, 0.0, 0, ?10, ?11, 1, NULL, 7385, 6055)",
        params!["global-bir-teknoloji-sirketine-donusen-sarsilmazin-odagi-n", "Global bir teknoloji şirketine dönüşen SARSILMAZ’ın odağı net:", "“Sahada kendini kanıtlayan, bugünün ihtiyaçlarına çözüm üreten ve yarının operasyonel üstünlüğünü bugünden şekillendiren sistemler geliştirmek.”", "<p><em>SARSILMAZ ve grup şirketleri, muharebe sahasındaki değişime ve gelişen tehditlere, sadece ürün bazlı değil, entegre çözüm yaklaşımıyla yanıt veriyor. Yaklaşık 150 yıllık mühendislik mirasını yeni nesil savunma teknolojileriyle buluşturan SARSILMAZ; LA2 Dynamics, BEST DEFENCE, ÜNİMETAL, BPS ve TUSAŞ ile ortak iştiraki olan TR Mekatronik’ten oluşan grup şirketleriyle entegre silah sistemlerini ve mühimmatı tek adresten sağlıyor. SARSILMAZ ve grup şirketlerinin son dönemde ortaya koyduğu robotik platformlar ve dronsavar çözümleri, silahlı kuvvetler ve güvenlik güçlerinin güncel ihtiyaçlarını doğrudan adresleyen, yetkin ürünler olarak öne çıkıyor.</em></p><p> </p><p>Yaklaşık 150 yıllık üretim ve mühendislik tecrübesine sahip olan SARSILMAZ ve grup şirketleri; silah, mühimmat, robotik platformlar, uzaktan komutalı sistemler, orta kalibre çözümler, hassas üretim kabiliyetleri ve entegrasyon altyapısıyla savunma ve havacılık sanayisine bütüncül çözümler sunan bir teknoloji evi konumunda bulunuyor....</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/global-bir-teknoloji-sirketine-donusen-sarsilmazin-odagi-n.webp", "MSI Dergisi Yayın Kurulu", "2026-07-30", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["global-bir-teknoloji-sirketine-donusen-sarsilmazin-odagi-n"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 281, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 7630, 6256)",
        params!["ender-arge-genel-muduru-mustafa-mutlu-coban", "Ender ARGE Genel Müdürü Mustafa Mutlu ÇOBAN:", "“Ender ARGE’yi, uzmanlık alanları doğrultusunda sistem ve alt sistemler geliştiren; büyük projelerde bu sistemleri ana yüklenicilerin altında sağlayan; bağımsız ürünler talep edildiğinde ise bunları doğrudan son kullanıcıya ya da tedarik makamlarına teslim edebilen bir firma olarak tanımlayabiliriz.”", "<p></p><p><em>Türk savunma ve havacılık sanayisindeki başarı hikâyelerinden bazıları adanmışlıkla yazılıyor. Ender ARGE de böyle bir hikâyeye sahip. 2019 yılında kurulan firma, geride bıraktığı yaklaşık 7 yıllık süreçte, başka bir üreticide olmayan, özgün kullanım konseptlerine sahip ürünler ortaya koydu. 3 ülkede ortak girişim şirketleri kurdu ve özgün ürünlerinin yurt dışı potansiyelini gerçeğe dönüştürme yolunda önemli adımlar attı. Ender ARGE Genel Müdürü Mustafa Mutlu Çoban, firmanın, savunma ve havacılık sektöründe başarıyı hedefleyen start-up’lar için de ilham olabilecek hikayesini ve gelecek hedeflerini, MSI Dergisi okuyucuları için anlattı.</em></p><p> ...</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/ender-arge-genel-muduru-mustafa-mutlu-coban.webp", "MSI Dergisi Yayın Kurulu", "2026-07-08", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["ender-arge-genel-muduru-mustafa-mutlu-coban"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 280, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 7875, 6457)",
        params!["saha-2026-kara-ozel-sayisi-armsto", "SAHA 2026 KARA ÖZEL SAYISI / ARMSTO", "ARMSTO Yönetim Kurulu Başkanı Vefa KUZU: “Türk savunma ve havacılık sanayisinin hepimizin gurur duyduğu ürünlerinde, ARMSTO’nun konnektörleri de görev yapıyor.”", "<p><em>ARMSTO, alt bileşen seviyesine kadar yerli ve milli ürünler geliştirmeyi hedefleyen Türk savunma ve havacılık sanayisine, “konnektör alanında bir mükemmeliyet merkezi” olarak hizmet veriyor. Mikro-D ve MIL-DTL-38999 gibi her biri yüzlerce alt konfigürasyona sahip konnektör türlerini birer ürün ailesi olarak ortaya koyan firma, sektörün çeşitli ürünlerinde yer alan ve herhangi bir standarda bağlı olmadan tasarlanmış özel konnektörleri de modern yaklaşımlarla yeniden tasarlayıp üretiyor. ARMSTO’nun konnektör konusundaki başlıca ihtiyaçları karşılayabilen bir tasarım ve üretim merkezi haline gelmesiyle firmanın ürünleri, sektörün önde gelen platformlarında, sistemlerinde ve çeşitli mühimmat türlerinde başarıyla görev yapmaya başladı.</em></p><p> </p><p style=\"text-align: center;\">ARMSTO, tüm ürünlerini, her zaman kadar aynı kalite standardında üretip teslim etmeyi hedefliyor....</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/saha-2026-kara-ozel-sayisi-armsto.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-kara-ozel-sayisi-armsto"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 279, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 8120, 6658)",
        params!["saha-2026-hava-ozel-sayisi-armsto", "SAHA 2026 Hava Özel Sayısı / ARMSTO", "ARMSTO Yönetim Kurulu Başkanı Vefa KUZU: “Üretim hassasiyeti ve kalitesi, ARMSTO’yu hava ve uzay platformlarının konnektör tedarikçisi konumuna taşıyor.”", "<p></p><p><em>Hava ve uzay sistemlerinin yüksek standartları, bu sistemlerde kullanılacak alt bileşenlerin de hassas ve yüksek kaliteyi önceleyen bir anlayışla üretilmesini zorunlu kılıyor. Tesislerini ve üretim altyapısını en başından bu gereksinimleri göz önüne alarak kuran ARMSTO, kısa zamanda hava ve uzay platformları için de konnektör sağlayan bir tedarikçi haline geldi. ARMSTO, tasarlayıp ürettiği konnektörlerle Türk savunma ve havacılık sanayisinin, hava ve uzay platformları alanında gittikçe artan bir tempoda ortaya koyduğu özgün ürünlere katkı veriyor.</em></p><p> ...</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/saha-2026-hava-ozel-sayisi-armsto.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-hava-ozel-sayisi-armsto"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 278, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 8365, 6859)",
        params!["saha-2026-deniz-sistemleri-sayisi-armsto", "SAHA 2026 Deniz Sistemleri Sayısı / ARMSTO", "ARMSTO Yönetim Kurulu Başkanı Vefa KUZU: “Tasarlayıp ürettiğimiz konnektörlerle Türkiye’nin deniz sistemlerinde elde ettiği yüksek yerlilik oranına katkı veriyoruz.”", "<p></p><p><em>Türk askeri denizcilik sektörü, ortaya koyduğu özgün platformların başında gelen TCG İSTANBUL (F-515) fırkateyninde, yüzde 80’lik yerlilik oranına ulaşmayı başardı. Yerlilik oranının artması, sektörün alt bileşen seviyesinde de yerli ve milli ürünleri tercih etmesiyle mümkün hale geldi. Türk savunma ve havacılık sanayisine “konnektör alanında bir mükemmeliyet merkezi” olarak hizmet veren ARMSTO, bu noktada kritik bir rol üstleniyor. Firmanın tasarlayıp ürettiği konnektörler, deniz platformlarında kullanılan çeşitli sistem ve mühimmatta görev yapıyor.</em></p><p><em> </em>...</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/saha-2026-deniz-sistemleri-sayisi-armsto.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-deniz-sistemleri-sayisi-armsto"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 8610, 7060)",
        params!["vizyondan-gercege-fmk-havacilik-turkiyenin-ozgun-havacilik", "Vizyondan Gerçeğe: FMK Havacılık, Türkiye’nin Özgün Havacılık Platformlarının Kritik Tedarikçisi Oluyor", "Vizyondan Gerçeğe: FMK Havacılık, Türkiye’nin Özgün Havacılık Platformlarının Kritik Tedarikçisi Oluyor", "<p></p><p><em>Türk havacılık ve uzay sanayisinin hava platformları ve havacılık motorları alanında özgün çözümler geliştiren ana yüklenicileri, bu faaliyetleri destekleyecek yetkin bir ekosisteme de ihtiyaç duyuyor. FMK Havacılık, kuruluş hikayesi ve gösterdiği gelişimle bu ekosistemin nasıl oluşturulabileceği konusunda örnek alınabilecek firmalardan biri olarak öne çıkıyor. Kuruluşunun 5’inci yılını geride bırakan FMK Havacılık, doğru bir vizyonla ve motivasyonuyla çıktığı yolculuğunda, Türkiye’nin özgün platformlarının kritik tedarikçilerinden biri haline geldi. Sektörün genç neslini temsil eden FMK Havacılık İcra Kurulu Üyeleri  F. Mert Kılıç ve Mustafa Yıldız, firmanın kuruluş öyküsünü ve geride kalan 5 yılda kaydettiği gelişimi, MSI Dergisi okuyucuları için anlattı.</em></p><p> ...</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/vizyondan-gercege-fmk-havacilik-turkiyenin-ozgun-havacilik.webp", "MSI Dergisi Yayın Kurulu", "2026-02-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["vizyondan-gercege-fmk-havacilik-turkiyenin-ozgun-havacilik"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 8855, 7261)",
        params!["aeros-yonetim-kurulu-baskani-serdar-cora", "Aeros Yönetim Kurulu Başkanı Serdar ÇORA:", "“Kaydettiğimiz kurumsal gelişim ve başarıyla tamamladığımız sivil ve askerî projeler sayesinde, hava platformları için sistem entegratörü olma hedefimizin ilk aşamasını başarıyla geride bıraktık. SHGM’den aldığımız SHT-21 belgesi de Aeros’u, bu yolculukta bir sonraki aşamaya taşımıştır.”", "<p></p><p><em>Aeros Havacılık (Aeros), 7. yılına girerken Türkiye ve hedef ülkelerdeki havacılık alanında ortaya çıkan çeşitli ihtiyaçları karşılama hedefiyle gelişimini sürdürüyor. Sivil Havacılık Genel Müdürlüğünden (SHGM) SHT-21 Tasarım Organizasyonu Onayı (DOA) almayı başaran sektörün ilk KOBİ’si olan Aeros, faaliyetlerindeki çeşitlenmenin getireceği hacimsel artışlara hazır hale gelebilmek için bir yandan da kurumsal kapasitesini geliştiriyor. 2025 yılı da Aeros’un ekibini genişlettiği ve kurumsal kapasitesini arttırdığı, yeni hizmetler vermeye başladığı ve üstlendiği projelerde önemli ilerlemeler kaydettiği bir yıl oldu. Aeros Yönetim Kurulu Başkanı Serdar Çora, firmanın 2025 yılında öne çıkan faaliyetlerini ve 2026 planlarını, MSI Dergisi okuyucuları için anlattı.</em></p><p> ...</p>", "kapak-dosyalari", "dergi-icerigi", "images/msi/aeros-yonetim-kurulu-baskani-serdar-cora.webp", "MSI Dergisi Yayın Kurulu", "2026-01-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["aeros-yonetim-kurulu-baskani-serdar-cora"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "kapak-dosyalari"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 9100, 7462)",
        params!["ges-muhendislik-yonetim-kurulu-baskani-arzu-baytar", "GES Mühendislik Yönetim Kurulu Başkanı Arzu BAYTAR:", "“Yaptığımız temaslarda ve katıldığımız fuarlarda, GES Mühendislik’in, askeri araç üstü entegrasyon denince akla gelen ilk firmalardan biri olduğunu görüyoruz. Yaklaşık 5 yılda bu konuma gelebilmek bizim açımızdan çok önemli bir başarı.”", "<p>Türk savunma ve havacılık sektörü, yoğun bir seri üretim döneminden geçerken kalite ve teslimat hızı gibi parametrelerde öne çıkan ana alt yüklenicilere ihtiyaç duyuyor. GES Mühendislik, yaptığı yatırımlar ve geliştirdiği kabiliyetler sayesinde, özellikle araç üst yapıları ya da araç üstü entegrasyon alanında önde gelen ana alt yüklenicilerden biri haline geldi. Firmanın seri üretim çalışmaları, ürün aileleri ile ilgili güncel durum, ihracat faaliyetleri ve geleceğe dair güncel planları hakkında, GES Mühendislik Yönetim Kurulu Başkanı Arzu Baytar’dan bilgi aldık.</p><p> </p><p><strong>MSI Dergisi</strong>: Arzu Hanım, Savunma Sanayii Başkanlığının (SSB) son dönemde odaklandığı konuların başında, sektörün geliştirdiği özgün platform ve sistemlerin seri üretiminin geldiğini; SSB’nin bu konuda hedefler koyduğunu görüyoruz. Bu gelişmelerin GES Mühendislik’e yansımaları nasıl oldu?...</p>", "soylesiler", "dergi-icerigi", "images/msi/ges-muhendislik-yonetim-kurulu-baskani-arzu-baytar.webp", "MSI Dergisi Yayın Kurulu", "2026-10-02", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["ges-muhendislik-yonetim-kurulu-baskani-arzu-baytar"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "soylesiler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 283, ?9, 5, 0, 0.0, 0, ?10, ?11, 1, NULL, 9345, 7662)",
        params!["bps-yonetim-kurulu-uyesi-yalcin-kayali", "BPS Yönetim Kurulu Üyesi Yalçın KAYALI:", "“BPS, savunma sanayisinin ihtiyaç duyduğu yüksek güvenilirlik, standartlara uygunluk ve sürdürülebilir üretim beklentilerini karşılayabilen güçlü bir mühimmat üretim kabiliyeti sunuyor.”", "<p><em>BPS Balıkesir Patlayıcı Maddeler Sanayi ve Ticaret A.Ş. (BPS), son kullanıcıların hem yeni mühimmat çözümlerine hem de daha yüksek adetlerde seri üretime ihtiyaç duyduğu bu dönemde, aranan çözüm ortağı olma hedefiyle faaliyetlerini yürütüyor. Hafif silah mühimmatında tüm bileşenleri kendisinin üretebildiği bir noktaya ulaşan BPS, böylece gelen talepleri hızlı ve kaliteden ödün vermeden karşılayabiliyor. Firma, yenilikçi ürün geliştirme konusundaki kabiliyetlerini ise dronlara karşı geliştirdiği BPS Drone Killer fişeğiyle ortaya koyuyor. Bir SARSILMAZ Grup şirketi olan BPS’nin hedefinde ise 20 mm, 25 mm, 30 mm, 35 mm mühimmat üretimi bulunuyor. Tüm bu gelişmelerin ayrıntıları hakkında, BPS Yönetim Kurulu Üyesi Yalçın Kayalı’dan bilgi aldık.</em></p><p><strong> </strong></p><p><strong>MSI Dergisi:</strong> Yalçın Bey, öncelikle BPS’nin hâlihazırdaki ürünleri ve üretim kabiliyetleri konusunda bilgi alabilir miyiz?...</p>", "soylesiler", "dergi-icerigi", "images/msi/bps-yonetim-kurulu-uyesi-yalcin-kayali.webp", "MSI Dergisi Yayın Kurulu", "2026-10-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["bps-yonetim-kurulu-uyesi-yalcin-kayali"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "soylesiler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "namlulu-silahlar"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 282, ?9, 5, 0, 0.0, 0, ?10, ?11, 1, NULL, 9590, 7863)",
        params!["expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan", "EXPOLINK Fuarcılık Yönetim Kurulu Başkanı Zeki ERDOĞAN:", "“İGEF büyümeye devam ettikçe, Türkiye’nin iç güvenlik alanındaki tecrübesini ve sanayi kabiliyetini dünyaya tanıtan, önemli uluslararası buluşma noktalarından biri hâline geleceğine inanıyoruz.”", "<p><em>Türkiye’nin iç güvenlik alanındaki operasyonel tecrübesi ve savunma ve iç güvenlik alanlarındaki üretim kabiliyeti, bu alanlara yönelik uluslararası ilgiyi de arttırıyor. Türk sanayisi tarafından geliştirilen ürünler, farklı ülkelerin özel ihtiyaçlarına da cevap verebiliyor. İç Güvenlik Ekipmanları Fuarı (İGEF), söz konusu ilgiyi ihracata ve kalıcı iş birliklerine dönüştürmeyi hedefliyor. Bu yıl üçüncü kez düzenlenecek fuar, ilk defa “uluslararası” sıfatıyla gerçekleştirilecek. 12-14 Kasım tarihlerinde, ATO Congresium’da düzenlenecek İGEF 2026’da, katılımcı firmaların, yaklaşık 60 ülkeden gelecek heyetlerle bir araya getirilmesi hedefleniyor. Heyet-firma görüşmelerini daha verimli hâle getirmek amacıyla heyetler, ülkelerinin ihtiyaçlarına uygun firmalara yönlendirilecek; bire bir görüşmeler önceden planlanacak ve firmalar için özel sunum alanları oluşturulacak. EXPOLINK Fuarcılık Yönetim Kurulu Başkanı Zeki Erdoğan, İGEF’in ulaştığı noktayı, değişen iç güvenlik ihtiyaçlarının fuara etkisini ve 2026 organizasyonunda firmalara sunulacak yeni imkânları MSI Dergisi’ne anlattı.</em></p><p> </p><p><strong>MSI Dergisi:</strong> Türkiye’nin savunma ve iç güvenlik alanlarında ulaştığı nokta, firmaların ürünlerini uluslararası pazarlara ulaştırma yaklaşımını nasıl etkiliyor? İGEF, bu noktada nasıl bir rol üstleniyor?...</p>", "soylesiler", "dergi-icerigi", "images/msi/expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan.webp", "MSI Dergisi Yayın Kurulu", "2026-10-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "soylesiler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 281, ?9, 4, 0, 0.0, 0, ?10, ?11, 1, NULL, 9835, 8064)",
        params!["aeros-tek-adresten-anahtar-teslim-cozumler-vizyonunu-kalit", "Aeros, \"Tek Adresten Anahtar Teslim Çözümler\" Vizyonunu Kalite Sistemiyle Destekliyor", "Aeros, \"Tek Adresten Anahtar Teslim Çözümler\" Vizyonunu Kalite Sistemiyle Destekliyor", "<p>Zeynep KAREL / zeynep@zeynepkarel.com</p><p></p><p><em>Havacılık alanında emniyeti merkeze alan sertifikasyon gereksinimleri, kalite konusunun önemini daha da arttırıyor. Kalite sistemlerini hava platformlarına yönelik sertifikasyon gereksinimlerini karşılayacak şekilde kurgulayan firmalar, bu kurgunun katma değerini, tüm faaliyetlerinde görebiliyorlar. Aeros hem bunları başarmış hem de sektördeki diğer firmalara bu konularda destek veren bir teknolojik KOBİ olarak öne çıkıyor. İş Mükemmelliği ve Yönetim Sistemleri Danışmanı, Mentor Yük. Müh. Zeynep Karel tarafından hazırlanan “Zeynep KAREL Kalite Söyleşileri” dizisinin bu bölümünde, Aeros Genel Müdürü Verda Çora ve Aeros Kalite Direktörü Fikri Akçalı; firmanın kalite sistemini nasıl oluşturduğunu ve projelerinde bu sistemden nasıl yararlandığını, MSI Dergisi okuyucuları için anlattı.</em>...</p>", "soylesiler", "dergi-icerigi", "images/msi/aeros-tek-adresten-anahtar-teslim-cozumler-vizyonunu-kalit.webp", "MSI Dergisi Yayın Kurulu", "2026-08-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["aeros-tek-adresten-anahtar-teslim-cozumler-vizyonunu-kalit"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "soylesiler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 280, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 10080, 8265)",
        params!["musiad-uyeleri-msb-onayli-tedarikci-gunu25-etkinliginde-uz", "MÜSİAD Üyeleri, MSB Onaylı Tedarikçi Günü’25 Etkinliğinde, Uzmanlarla Bir Araya Geldi", "MSB Onaylı Tedarikçi Sistemi, Yerli Üreticilere Yeni Kapılar Açıyor", "<p><em>Müstakil Sanayici ve İşadamları Derneği (MÜSİAD) Savunma Sanayi Sektör Kurulu, savunma ve havacılık sanayisinde yerlilik oranını arttırmak ve özel sektör firmalarının tedarik zincirine katılımını güçlendirmek hedefiyle önemli bir adım attı. MÜSİAD ve Millî Savunma Bakanlığı (MSB) iş birliğiyle düzenlenen “Millî Güce Katkı: MSB Onaylı Tedarikçi Günü’25” etkinliği, 14 Ekim’de, MÜSİAD Genel Merkezi’nde gerçekleştirildi.</em></p><p style=\"text-align: justify;\"> </p><p style=\"text-align: justify;\">Etkinlik kapsamında uzmanlar tarafından; MSB’nin yürüttüğü “Onaylı Tedarikçi Sistemi, İhale ve Onaylı Tedarikçi Süreçleri, NSPA (NATO Support and Procurement Agency / NATO Tedarik ve Destek Ajansı) Tedarikçisi Olma Süreçleri ve AQAP Belgelendirme” faaliyetleri hakkında bilgilendirmeler yapıldı. Ayrıca MSB yetkilileriyle MÜSİAD üyeleri, ikili toplantılarda bir araya gelerek potansiyel iş birliklerini konuştu....</p>", "ozel-haberler", "dergi-icerigi", "images/msi/musiad-uyeleri-msb-onayli-tedarikci-gunu25-etkinliginde-uz.webp", "MSI Dergisi Yayın Kurulu", "2026-02-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["musiad-uyeleri-msb-onayli-tedarikci-gunu25-etkinliginde-uz"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 279, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 10325, 8466)",
        params!["genelkurmay-baskanligina-orgeneral-selcuk-bayraktaroglu-ka", "Genelkurmay Başkanlığına Orgeneral Selçuk Bayraktaroğlu; Kara Kuvvetleri Komutanlığına Orgeneral Metin Tokel Atandı", "Türk Silahlı Kuvvetlerinde ve Kolluk Kuvvetlerinde Atamalar ve Terfiler", "<p><em>Yüksek Askerî Şûra (YAŞ) toplantısı, Cumhurbaşkanı Recep Tayyip Erdoğan başkanlığında, 5 Ağustos’ta gerçekleştirildi. Toplantıda alınan kararlar uyarınca, Genelkurmay Başkanı Orgeneral Metin Gürak, kadrosuzluk nedeniyle emekliye sevk edildi. Genelkurmay Başkanlığına, Kara Kuvvetleri Komutanı Orgeneral Selçuk Bayraktaroğlu; Orgeneral Bayraktaroğlu’ndan boşalan Kara Kuvvetleri Komutanlığına ise 1’inci Ordu Komutanı Orgeneral Metin Tokel atandı.</em></p><p><em> </em></p><p>Cumhurbaşkanlığı Külliyesi’nde gerçekleştirilen toplantı öncesinde, şûra üyeleri, Cumhurbaşkanı Erdoğan başkanlığında, Anıtkabir’i ziyaret etti. Ziyarete, YAŞ üyeleri; Cumhurbaşkanı Yardımcısı Cevdet Yılmaz, Adalet Bakanı Yılmaz Tunç, Dışişleri Bakanı Hakan Fidan, Hazine ve Maliye Bakanı Mehmet Şimşek, İçişleri Bakanı Ali Yerlikaya, Millî Eğitim Bakanı Yusuf Tekin, Millî Savunma Bakanı Yaşar Güler, Genelkurmay Başkanı Orgeneral Metin Gürak, Kara Kuvvetleri Komutanı Orgeneral Selçuk Bayraktaroğlu, Deniz Kuvvetleri Komutanı Oramiral Ercüment Tatlıoğlu ve Hava Kuvvetleri Komutanı Orgeneral Ziya Cemal Kadıoğlu katıldı....</p>", "ozel-haberler", "dergi-icerigi", "images/msi/genelkurmay-baskanligina-orgeneral-selcuk-bayraktaroglu-ka.webp", "MSI Dergisi Yayın Kurulu", "2025-09-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["genelkurmay-baskanligina-orgeneral-selcuk-bayraktaroglu-ka"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "namlulu-silahlar"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 278, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 10570, 8667)",
        params!["meteksan-savunmadan-uretim-kapasitesini-25-kat-arttiracak", "Meteksan Savunma’dan Üretim Kapasitesini 2,5 Kat Arttıracak Hamle", "Meteksan Savunma’dan Üretim Kapasitesini 2,5 Kat Arttıracak Hamle", "<p><em>Meteksan Savunma, Bilkent Cyberpark’taki Üretim ve Entegrasyon Binası’nın açılışını, 8 Ağustos’ta düzenlenen törenle yaptı. 1.500 metrekaresi üretim; 1.000 metrekaresi de entegrasyon alanı olmak üzere toplam 2.500 metrekarelik alanda kurulan yeni tesis, Meteksan Savunma’nın üretim kapasitesini 2,5 kat arttıracak. Cirosunun yüzde 30'unu ihracattan elde eden ve bu rakamı daha da yukarıya çekmeyi hedefleyen şirketin bu hamlesi hem yurt içi hem de yurt dışına yönelik faaliyetlerinin gelişimine katkı sağlayacak.</em></p><p> </p><p>Törene; Savunma Sanayii Başkanı Prof. Dr. Haluk Görgün, Bilkent Üniversitesi Rektörü Prof. Dr. Kürşat Aydoğan, Türk Silahlı Kuvvetlerini Güçlendirme Vakfı Genel Müdürü Bilal Topçu, Meteksan Savunma çalışanları ve sektör temsilcileri katıldı. Tören kapsamında, Meteksan Savunma Üretim ve Entegrasyon Binası’nda, firmanın ürünlerinin yer aldığı bir sergi alanı da oluşturuldu....</p>", "ozel-haberler", "dergi-icerigi", "images/msi/meteksan-savunmadan-uretim-kapasitesini-25-kat-arttiracak.webp", "MSI Dergisi Yayın Kurulu", "2025-09-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["meteksan-savunmadan-uretim-kapasitesini-25-kat-arttiracak"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 10815, 8868)",
        params!["teknofest-ruzgari-bu-kez-mavi-vatanda-esti", "TEKNOFEST Rüzgârı, Bu Kez Mavi Vatan’da Esti", "İlk Kez Deniz Temasıyla Düzenlenen TEKNOFEST, 3 günde 175 Bin Kişiyi Ağırladı", "<p><em>Türkiye’nin ilk ve tek havacılık, uzay ve teknoloji festivali TEKNOFEST, bu yıl ilk kez deniz temasıyla düzenlendi. İstanbul Tersanesi Komutanlığında, 29-31 Ağustos tarihleri arasında gerçekleştirilen TEKNOFEST Mavi Vatan, 175 bin kişiyi ağırladı. Etkinliğin açılış töreninde konuşan Cumhurbaşkanı Recep Tayyip Erdoğan; Türkiye’nin, savunma ve havacılık sanayisinde, çok kısa sürede dünyanın gıptayla takip ettiği bir seviyeye geldiğini belirterek “Donanmamızın amiral gemisi ve dünyanın ilk SİHA gemisi olan TCG ANADOLU’nun (L-400) bir büyüğünü yapıyoruz.” dedi.</em></p><p> </p><p>TEKNOFEST Mavi Vatan; Türkiye Teknoloji Takımı Vakfı (T3 Vakfı) yürütücülüğünde, Sanayi ve Teknoloji Bakanlığı ile Millî Savunma Bakanlığı iş birliğiyle gerçekleştirildi. Festivale; bakanlıklar, üniversiteler ve savunma ve havacılık sanayisi firmalarının da aralarında olduğu çok sayıda kurum ve kuruluş katıldı....</p>", "ozel-haberler", "dergi-icerigi", "images/msi/teknofest-ruzgari-bu-kez-mavi-vatanda-esti.webp", "MSI Dergisi Yayın Kurulu", "2025-09-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["teknofest-ruzgari-bu-kez-mavi-vatanda-esti"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 11060, 9069)",
        params!["anti-dron-sistemleri-gelistirmeye-odaklanan-han-yenilikci", "Anti Dron Sistemleri Geliştirmeye Odaklanan HAN; Yenilikçi, Hızlı ve Maliyet Etkin Çözümler Parolasıyla İlerliyor", "Anti Dron Sistemleri Geliştirmeye Odaklanan HAN; Yenilikçi, Hızlı ve Maliyet Etkin Çözümler Parolasıyla İlerliyor", "<p></p><p>Dronların muharebe sahasında yarattığı tehdit hem asimetrik olması hem de çok hızlı gelişmesi nedeniyle son dönemin en büyük problem kaynaklarından biri haline gelmiş durumda. Konvansiyonel savunma sistemlerinin başa çıkamadığı tehdidin bu nitelikleri, dron tehdidine verilecek cevabın da benzer şekilde çok hızlı ortaya koyulmasını ve asimetrik olmasını gerektiriyor.</p><p>Savunma ve havacılık sanayisi ile dünyanın önde gelen ülkeleri arasına girmeyi başaran Türkiye, sektör firmalarının dron tehdidine karşı ortaya koyduğu çözümlerle de dikkatleri üzerine çekti. Türk savunma ve havacılık sanayisi, geliştirdiği anti dron sistemleriyle sadece konvansiyonel savunma sistemleri geliştirmekte başarılı olmadığını; muharebe sahasının değişen ve gelişen ihtiyaçlarına cevap verebilecek yenilikçi çözümler ortaya koyabildiğini de gösterdi....</p>", "basyazi", "dergi-icerigi", "images/msi/anti-dron-sistemleri-gelistirmeye-odaklanan-han-yenilikci.webp", "MSI Dergisi Yayın Kurulu", "2026-10-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["anti-dron-sistemleri-gelistirmeye-odaklanan-han-yenilikci"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "basyazi"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 11305, 9270)",
        params!["yapay-zeka-teknolojileri-databoss-ile-muharebe-sahasina-in", "Yapay Zekâ Teknolojileri, DataBoss ile Muharebe Sahasına İniyor", "Yapay Zekâ Teknolojileri, DataBoss ile Muharebe Sahasına İniyor", "<p></p><p>Yapay zekâ teknolojisinin büyük bir hızla gelişimi, bu teknolojinin savunma sistemlerinde kullanımıyla birlikte, muharebe sahasındaki uygulamalarının da giderek arttığı bir dönemi beraberinde getirdi. Bu teknolojinin Türk savunma ve havacılık sektöründeki bayraktarlığını ise aynı zamanda bir SSTEK firması olan DataBoss yapıyor. Firmanın yapay zekâ teknolojilerini kullanarak geliştirdiği ve son kullanıcılar tarafından da aktif olarak kullanılan çözümler, yapay zekâyı yalnızca teorik bir kabiliyet olmaktan çıkarıp karar süreçlerini hızlandıran ve operasyonel etkinliği doğrudan arttıran bir güç unsuru haline getiriyor.</p><p>Bu sayımızın kapak dosyasına konuk olan DataBoss Genel Müdürü Prof. Dr. Süleyman Serdar Kozat, söyleşimize, yapay zekânın hayatımıza etkisi konusunda önemli bir eşiğin geçildiğini ve bunun yeterince farkında olmadığımızı söyleyerek başladı. ChatGPT’nin, 2022’nin son çeyreğinde genel kullanıma açılmasını bir dönüm noktası olarak tanımlayan Prof. Dr. Kozat, o günden bu yana yapay zekâ teknolojilerinin çok hızlı bir şekilde değiştiğine işaret etti. Prof. Dr. Kozat, bu değişimin merkezinde bulunan büyük dil modeli (large language model / LLM) kavramına ilişkin şunları söyledi: <em>“Artık LLM’ler, bir sonraki kelimeyi tahmin etmeye çalışan sistemler olmaktan çıktı. Plan yapabiliyorlar, matematik sorusu çözebiliyorlar, kod yazabiliyorlar… Bu kabiliyetleri de onların, yazılım arayüzüyle kontrol edilebilen her platformu, sistemi, makineyi ya da aleti kullanabilmesini sağlıyor.”</em>...</p>", "basyazi", "dergi-icerigi", "images/msi/yapay-zeka-teknolojileri-databoss-ile-muharebe-sahasina-in.webp", "MSI Dergisi Yayın Kurulu", "2026-09-22", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["yapay-zeka-teknolojileri-databoss-ile-muharebe-sahasina-in"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "basyazi"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 283, ?9, 6, 0, 0.0, 0, ?10, ?11, 0, NULL, 11550, 9471)",
        params!["menatekte-30-yilin-ozeti-apartman-dairesinden-ihracat-samp", "MENATEK’te 30 Yılın Özeti: Apartman Dairesinden İhracat Şampiyonluğuna", "MENATEK, bundan tam 30 yıl önce, TSK’nın yedek parça ihtiyaçlarını karşılayacak bir firma olarak merhum Kazım Ünal tarafından kuruldu. 2000 yılında, NATO için de yedek parça tedarik etmeye başlaması, MENATEK’in dönüm noktalarından biri oldu", "<p>MENATEK, bundan tam 30 yıl önce, TSK’nın yedek parça ihtiyaçlarını karşılayacak bir firma olarak merhum Kazım Ünal tarafından kuruldu. 2000 yılında, NATO için de yedek parça tedarik etmeye başlaması, MENATEK’in dönüm noktalarından biri oldu. NATO projeleri ile başlayan ihracat çalışmalarının bir sonraki durağı, ABD oldu. 2002-2005 yılları arasında, Amerikan Kara Kuvvetlerinin ana yüklenicileri için alt yüklenici olarak çalışan MENATEK, Türkiye’de üretilen kara aracı yedek parçalarını ABD’ye ihraç etmeye başladı. Firma yetkilileri, 2006’da, firmanın Amerikan Kara Kuvvetlerine doğrudan satış yapabilmesini sağlayacak idari süreci başlattı. 2007’de de ABD’den ilk iş alındı. Aynı dönemde, Almanya’ya ve İspanya’ya satışlar başladı.</p><p>1995 yılında, tek kişiyle başlayan yolculukta MENATEK, bugün 80 kişilik bir ekip büyüklüğüne ve paletli araçların tüm mekanik alt sistemlerini üretebilir konuma ulaştı. Firmanın ürün gamı; silah sisteminden yürüyen aksama, fren sisteminden ısıtma ve soğutma sistemine, yağlama sisteminden güç aktarma sistemine kadar tüm alt sistemlerin parçalarını kapsıyor. Hatta tüm yürüyen aksamı, araca monte edilmeye hazır bir çözüm olarak da sunabilen firmanın ürün yelpazesi, son dönemde daha da genişledi. Firma, Batı menşeli araçların yanı sıra T-72 tankları gibi Sovyet ekolünden gelen paletli ve tekerlekli araçlar için de çözüm sunabiliyor.</p><p>Bu sayının kapak dosyasına konuk olan MENATEK CEO’su Mehmet Ünal ve Strateji Başkanı Naz Ünal ile firmanın yukarıda özetlemeye çalıştığımız 30 yıllık başarı hikayesini ve bugün ulaştığı noktayı konuştuk. MENATEK CEO’su Mehmet Ünal, hikâyenin başlangıcını şöyle anlattı: <em>“MENATEK, faaliyetlerine bir apartman dairesinde başladı. Daha sonra 2 dairenin birleştirildiği daha büyük bir ofise dönüştü. İlerleyen süreçte ufak bir atölye kiraladı. 2012’de kardeşim Naz Hanım; 2013’ün başında da ben şirkete katıldım. Hemen ardından da 3 katlı, toplam 400 metrekare alana sahip, atölyesi de bulunan bir tesise geçtik. 2021 yılında da mevcut tesisimizde faaliyet göstermeye başladık.”</em>...</p>", "basyazi", "dergi-icerigi", "images/msi/menatekte-30-yilin-ozeti-apartman-dairesinden-ihracat-samp.webp", "MSI Dergisi Yayın Kurulu", "2025-08-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["menatekte-30-yilin-ozeti-apartman-dairesinden-ihracat-samp"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "basyazi"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 282, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 11795, 9671)",
        params!["teslimat-ve-siparis-adetlerini-arttiran-armsto-zirve-yuruy", "Teslimat ve Sipariş Adetlerini Arttıran ARMSTO, Zirve Yürüyüşünü Sürdürüyor", "Son dönemde dünya genelinde yaşanan savaşlar ve çatışmalar, teknolojinin modern muharebe sahasını ne kadar büyük bir hızla dönüştürdüğünü de gözler önüne seriyor.", "<p>Son dönemde dünya genelinde yaşanan savaşlar ve çatışmalar, teknolojinin modern muharebe sahasını ne kadar büyük bir hızla dönüştürdüğünü de gözler önüne seriyor. Bu dönüşüme ayak uydurabilmek için platform ve ana sistemlerin yanı sıra alt sistemleri ve konnektör gibi bileşenleri de tasarlayıp üretebilecek bir sanayi altyapısına sahip olmak gerekiyor. Türkiye, son dönemde savunma ve havacılık sanayisi alanında yaptığı büyük atılımla dünya üzerinde bu altyapıya sahip az sayıda ülke arasına adını yazdırdı. Türk savunma ve havacılık sanayisinin platform ve ana sistem seviyesinde elde ettiği başarıları, bileşen seviyesinde de tekrarlayan ARMSTO’yu, bu sayımızın kapak dosyasında konuk ettik.</p><p>ARMSTO Yönetim Kurulu Başkanı Vefa Kuzu, yaklaşık 3 yıl içerisinde geldikleri noktaya ve ARMSTO için nasıl bir gelecek öngördüklerine ilişkin şunları söyledi: <em>“ARMSTO, Türkiye’de daha önce başka kişilerin ve firmaların niyetlendiği; ancak çeşitli sebeplerle tam olarak başaramadığı konnektör tasarımını ve üretimini, çok kısa bir süre zarfında gerçeğe dönüştürdü. Mühendislik, üretim, kalite ve kurumsallık gibi konularda kendimizi ispat ettik. Sektörün önde gelen firmalarının tedarikçisi olduk. Türk savunma ve havacılık sanayisinin hepimizin gurur duyduğu ürünlerinde, ARMSTO’nun konnektörleri de görev yapıyor.</em></p><p><em>Ulaştığımız bu konumda, artık hedefimiz; savunma ve havacılık sektöründe, önce Türkiye’nin en büyük firmalarından biri olmak. Daha uzun vadede ise dünyanın önde gelen oyuncularından biri haline gelmek.”</em>...</p>", "basyazi", "dergi-icerigi", "images/msi/teslimat-ve-siparis-adetlerini-arttiran-armsto-zirve-yuruy.webp", "MSI Dergisi Yayın Kurulu", "2025-07-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["teslimat-ve-siparis-adetlerini-arttiran-armsto-zirve-yuruy"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "basyazi"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 281, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 12040, 9872)",
        params!["databoss-yapay-zeka-ve-buyuk-veri-teknolojilerinde-dunya-i", "DataBoss, Yapay Zekâ ve Büyük Veri Teknolojilerinde Dünya ile Yarışıyor", "Dünya genelinde yaşanan savaşlar ve çatışmalar, teknolojinin modern muharebe sahasını ne kadar büyük bir hızla dönüştürdüğünü gözler önüne seriyor. Bu dönüşümde etkili olan teknoloji alanları arasında, yapay zekâ ve büyük veri de önemli bir yer tutuyor.", "<p>Dünya genelinde yaşanan savaşlar ve çatışmalar, teknolojinin modern muharebe sahasını ne kadar büyük bir hızla dönüştürdüğünü gözler önüne seriyor. Bu dönüşümde etkili olan teknoloji alanları arasında, yapay zekâ ve büyük veri de önemli bir yer tutuyor.</p><p>Türk savunma ve havacılık sanayisi, ana platformlar ve mühimmat gibi alanlarda gösterdiği gelişimi, bu iki teknoloji alanında da sergiliyor. Bu sayımızın kapak dosyasına konuk olan DataBoss, Türkiye’nin, yapay zekâ ve büyük veri teknolojilerinde de dünyanın geriye kalanı ile yarışmasında kritik bir rol oynuyor.</p><p>DataBoss Genel Müdürü Prof. Dr. S. Serdar Kozat, DataBoss’un bu yarışın neresinde olduğunu şu sözlerle anlatıyor: <em>“Akademik literatürü ve bu alanda dünya genelinde özel sektörün ne yaptığını takip eden ve bilen biri olarak şunu rahatlıkla söyleyebilirim: Dünya genelinde en güncel hangi teknolojiler varsa biz de onlar üzerinde çalışıyoruz. Tabii ki ölçekler farklı olabilir. Ancak teknoloji ve kabiliyet temelinde, dünyadan geri kaldığımız hiçbir şey yok. Avrupa’dan ileride olduğumuzu görebiliyoruz. ABD’de yapılan çalışmalarla da başa baş gittiğimizi, bazı konularda onlarla yarıştığımızı da rahatlıkla söyleyebilirim.”</em>...</p>", "basyazi", "dergi-icerigi", "images/msi/databoss-yapay-zeka-ve-buyuk-veri-teknolojilerinde-dunya-i.webp", "MSI Dergisi Yayın Kurulu", "2025-06-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["databoss-yapay-zeka-ve-buyuk-veri-teknolojilerinde-dunya-i"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "basyazi"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 280, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 12285, 10073)",
        params!["ctechte-20-yilin-ozeti-start-up-ctechten-dunya-markasi-cte", "CTech’te 20 Yılın Özeti: Start-Up CTech’ten Dünya Markası CTech’e", "Türk savunma ve havacılık sanayisinin bugünlere gelmesini sağlayan kritik kararların alındığı 2004 yılı sonrasındaki ilk dönemde kurulan firmalardan biri de CTech oldu.", "<p>Türk savunma ve havacılık sanayisinin bugünlere gelmesini sağlayan kritik kararların alındığı 2004 yılı sonrasındaki ilk dönemde kurulan firmalardan biri de CTech oldu. Bugün 20. yılını kutlayan CTech, 2003 yılında ortaya atılan bir fikir doğrultusunda, bir “girişim sermayesi şirketi” olarak 2005 yılında kuruldu.</p><p>Savunma Sanayii Müsteşarlığı (bugün Savunma Sanayii Başkanlığı), pek çok diğer örnekte olduğu gibi, CTech’in kuruluş aşamasında da önemli bir rol oynadı. Sektörün gelişmesi ve KOBİ’lerin de sektöre kazandırılması için yapılan çalışmalar kapsamında görevlendirilen firmalardan biri de CTech oldu. Bu sayımızın kapak dosyasına konuk olan CTech Genel Müdürü Dr. Cüneyd Fırat, o günleri şöyle anlattı: <em>“CTech’i kurarken aklımızda birçok fikir vardı; ancak bir başlangıç projemiz yoktu. 2000’li yılların başında, o zamanki adı ile Savunma Sanayii Müsteşarlığının (SSM) KOBİ’lerle ilgili bir açılımı vardı. KOBİ’lerin sektöre kazandırılması için yönlendirmeler yapılıyordu. Gerekli başvuruları yapmamızla birlikte, bize de SSM’den ve ana yüklenicilerden şartnameler gelmeye başladı. Bu şartnameler için hazırladığımız tekliflerden biri kabul edilince de ilk sözleşmemizi, 2005 bitmeden imzaladık.”</em></p><p>İlerleyen dönemde; modelleme ve simülasyon, haberleşme sistemleri ve siber güvenlik başta olmak üzere farklı alanlardaki projelerde görev alan CTech için bu sözleşme, o dönemde henüz bilinmese de firmanın bugünü açısından kritik bir öneme sahip olacaktı. CTech’in AYSEK modemi geliştirdiği o proje, bugün NATO’nun da tercih ettiği SecureARX modemlerinin temelini oluşturdu....</p>", "basyazi", "dergi-icerigi", "images/msi/ctechte-20-yilin-ozeti-start-up-ctechten-dunya-markasi-cte.webp", "MSI Dergisi Yayın Kurulu", "2025-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["ctechte-20-yilin-ozeti-start-up-ctechten-dunya-markasi-cte"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "basyazi"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 279, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 12530, 10274)",
        params!["best-defence-marin-tip-ukss-cozumleriyle-deniz-platformlar", "BEST DEFENCE, Marin Tip UKSS Çözümleriyle Deniz Platformlarının da Hizmetinde", "Türk savunma ve havacılık sanayisinin önemli iş birliklerinden biri de geçtiğimiz yıl SARSILMAZ ve Best Grup arasında yaşanmış ve bu iş birliğinden de BEST DEFENCE doğmuştu.", "<p>Türk savunma ve havacılık sanayisinin önemli iş birliklerinden biri de geçtiğimiz yıl SARSILMAZ ve Best Grup arasında yaşanmış ve bu iş birliğinden de BEST DEFENCE doğmuştu. BEST DEFENCE’in kuruluşuyla birlikte SARSILMAZ, uzaktan komutalı silah sistemleri (UKSS) alanında; silah kulesini, silahı ve mühimmatı tek adresten sunabilme avantajı kazanmış oldu.</p><p>Savunma sanayisi odaklı teknoloji şirketi olma vizyonu doğrultusunda çalışmalarını sürdüren SARSILMAZ’ın bu hamlesiyle kurulan BEST DEFENCE, hızlı bir ilerleme kaydetti. Firma, arkasındaki köklü mirastan da aldığı güçle ilk aşamada ağırlıklı olarak kara platformları için sunduğu çözümleri, bu sayımızın kapak dosyasında ele aldığımız marin tip UKSS ürün ailesi ile deniz platformlarını da kapsayacak şekilde genişletti. Bu sayede de farklı kalibrelerde silahları kullanabilme, yapay zekâ destekli hedef tespiti ve takibi, yüksek hassasiyet ve kuleyi, silahı ve mühimmatı tek adresten tedarik edebilme gibi avantajlar, kara platformları kullanıcılarının yanı sıra deniz platformları kullanıcılarının hizmetine de sunuldu.</p><p>BEST DEFENCE Genel Müdürü Aziz Karaş, marin tip UKSS ürün aileleriyle ilgili hedeflerini şöyle anlattı: <em>“UKSS’lerimizle kara platformlarında yakaladığımız başarıyı, deniz platformlarında da yakalamayı hedefliyoruz. Deniz platformları için, sınıfının en iyisi UKSS’leri sağlıyoruz. Ürünlerimiz hem kullanım kolaylığıyla hem de esnekliğiyle son kullanıcılar tarafından çok beğeniliyor. UKSS’yi, grup şirketlerimizin sağladığı silah ve mühimmat ile birlikte teslim edebilmemiz ise tedarik makamlarının bizi tercih etmelerinde önemli bir rol oynuyor. Uluslararası rekabette, bu seviyede çözümler sağlayabilen başka bir oyuncu olmadığını, gururla ifade edebilirim.”</em>...</p>", "basyazi", "dergi-icerigi", "images/msi/best-defence-marin-tip-ukss-cozumleriyle-deniz-platformlar.webp", "MSI Dergisi Yayın Kurulu", "2025-04-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["best-defence-marin-tip-ukss-cozumleriyle-deniz-platformlar"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "basyazi"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 278, ?9, 6, 0, 0.0, 0, ?10, ?11, 0, NULL, 12775, 10475)",
        params!["turkiyenin-havacilik-verisi-alanindaki-disa-bagimliligi-ke", "Türkiye’nin Havacılık Verisi Alanındaki Dışa Bağımlılığı, KEYVAN Havacılık ile Sona Erdi", "Türk savunma ve havacılık sanayisi, son dönemde, çok sayıda ana platformu, yerli ve milli imkanlarla tasarladı ve üretti.", "<p style=\"text-align: justify\">Türk savunma ve havacılık sanayisi, son dönemde, çok sayıda ana platformu, yerli ve milli imkanlarla tasarladı ve üretti. Bu platformların büyük bir bölümü yurt içinde envantere alındı; bir bölümü de ihraç edildi. Özellikle uçan platformlar söz konusu olduğunda, bu platformların kendilerinden beklenen görevleri yerine getirebilmeleri için gerekli olan havacılık verisinin güncel ve güvenilir olması da çok büyük önem taşıyor.</p><p style=\"text-align: justify\">Havacılıkta verinin neden önemli olduğunu, bu sayımızın kapak dosyasına konuk olan KEYVAN Havacılık’ın kurucusu Mehmet Keyvan şöyle açıklıyor: <em>“Havacılıkta veri çok önemlidir; çünkü hem emniyeti hem de operasyonel verimliliği sağlamanın anahtarıdır. Güncel ve güvenilir veriye erişilemediğinde, uçuşların emniyeti tehlikeye girebilir. Çünkü uçaklar, bu verilerle otomatik pilotta uçar, kalkış ve iniş yapar. Ayrıca veri hatalı ya da eski ise daha az kaynak harcayarak tamamlayabileceğiniz bir uçuşu, daha çok kaynak harcayarak ve verimsiz bir şekilde gerçekleştirmek zorunda kalabilirsiniz.”</em></p><p style=\"text-align: justify\">Havacılık verisinin yerli ve milli olmasının önemi, özellikle askeri uygulamalarda kendisini daha net gösteriyor. Keyvan, savunma ve güvenlik alanında, havacılık platformlarının ve sistemlerinin yanı sıra bunların kullandığı verilerin de yerli ve milli olması gerektiğini söylüyor: <em>“Verinizi dışarıdan aldığınız zaman, talep ettiğiniz veriye göre nerelere gittiğinizle ilgili ipuçlarını da vermiş oluyorsunuz. Burada bir güvenlik açığı doğabiliyor. Aldığınız verinin doğru ve güncel olması, askeri operasyonlarda ve güvenlik operasyonlarında da sivil operasyonlarda olduğu gibi kritik bir öneme sahip. Bu açıdan baktığınızda, savunma ve güvenlik alanları söz konusu olduğunda, havacılık verisinin kaynağı, daha da önemli hale geliyor.”</em>...</p>", "basyazi", "dergi-icerigi", "images/msi/turkiyenin-havacilik-verisi-alanindaki-disa-bagimliligi-ke.webp", "MSI Dergisi Yayın Kurulu", "2025-03-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["turkiyenin-havacilik-verisi-alanindaki-disa-bagimliligi-ke"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "basyazi"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 13020, 10676)",
        params!["saha-2026-ozel-sayilari-icisleri-bakanligi-mesaji", "SAHA 2026 Özel Sayıları / İçişleri Bakanlığı Mesajı", "Huzurun Kurucu Zemini Olarak Güvenlik", "<p>Toplumların varlığını sürdürebilmesinin en temel şartı, huzurun temin edilmesidir. Huzur; toplumsal düzenin, siyasal istikrarın ve ortak yaşamın en üst değeridir. Bu değerin hayata geçmesini sağlayan en temel unsur ise güvenliktir. Nitekim İbn Haldûn, bir toplumun bekasını ve “umrânın” gelişimini, “asabiyetin” inşa ettiği toplumsal dayanışmaya dayandırır; bu dayanışmanın kurduğu düzenin devamlılığını sağlayan asli zeminin ise güvenlik olduğunu ortaya koyar. Bu çerçevede güvenlik, bir idari fonksiyon olmanın ötesinde, medeniyetin sürekliliğini mümkün kılan temel bir zemindir.</p><p>Güvenlik düşüncesi, tarih boyunca farklı biçimlerde ele alınmış; her dönemde değişen şartlara göre yeniden yorumlanmıştır. Toplumsal düzenin korunması ve bireylerin huzur içinde yaşayabilmesi, yönetsel aklın en önemli sorumlulukları arasında yer alır. Bu yaklaşım, günümüz güvenlik anlayışıyla birlikte değerlendirildiğinde, güvenliğin yalnızca fiziki alanlarla sınırlı kalmadığı; dijital, ekonomik ve toplumsal boyutlarıyla bütüncül bir yapıya kavuştuğu görülmektedir.</p><p>Günümüzde güvenlik anlayışı, klasik sınırların ötesine geçmiş durumdadır. Fiziksel tehditlerin yanı sıra dijital alan, veri akışları, ekonomik sistemler, toplumsal dinamikler ve hatta bireysel alanlar, güvenliğin doğrudan konusu hâline gelmiştir. Bu çok katmanlı yapı, güvenliğin de çok boyutlu bir perspektifle ele alınmasını zorunlu kılmaktadır. Artık güvenlik; sahada yürütülen bir faaliyet olmanın ötesinde, teknoloji, veri ve strateji ekseninde şekillenen bütüncül bir sistem olarak değerlendirilmektedir....</p>", "mesajlar", "dergi-icerigi", "images/msi/saha-2026-ozel-sayilari-icisleri-bakanligi-mesaji.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-icisleri-bakanligi-mesaji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "mesajlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 13265, 10877)",
        params!["saha-2026-ozel-sayilari-milli-savunma-bakanligi-mesaji", "SAHA 2026 ÖZEL SAYILARI / Millî Savunma Bakanlığı Mesajı", "Millî Savunma Bakanı Yaşar GÜLER: “Türkiye, artık yalnızca kendi ihtiyaçlarını karşılayan değil; teknoloji geliştiren, ihraç eden ve küresel ölçekte rekabet eden güçlü bir savunma sanayii ekosistemine sahiptir.”", "<p>Günümüzde küresel güvenlik ortamı daha karmaşık, kırılgan ve çok katmanlı bir yapıya evrilmiştir. Konvansiyonel çatışmaların uzun süreli krizlere dönüştüğü, her geçen gün tehdit yelpazesinin genişlediği bir jeopolitik denklemle karşı karşıyayız.</p><p style=\"text-align: center;\">TCG İSTANBUL (F-515)</p><p>...</p>", "mesajlar", "dergi-icerigi", "images/msi/saha-2026-ozel-sayilari-milli-savunma-bakanligi-mesaji.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-milli-savunma-bakanligi-mesaji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "mesajlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 6, 0, 0.0, 0, ?10, ?11, 0, NULL, 13510, 11078)",
        params!["saha-2026-ozel-sayilari-ticaret-bakanligi-mesaji", "SAHA 2026 ÖZEL SAYILARI / Ticaret Bakanlığı Mesajı", "Ticaret Bakanı Prof. Dr. Ömer BOLAT: “Devreye aldığımız yeni destek mekanizmalarıyla da ihracatçılarımızın yanında olmaya devam ediyoruz.”", "<p>Kıymetli Okuyucular,</p><p>Savunma ve havacılık sanayimiz, yüzde 80’in üzerinde yerlilik oranı ve 20 milyar doları aşan cirosuyla ülkemizin güvenlik mimarisini güçlendirmenin yanı sıra yüksek katma değerli üretimi, ileri teknoloji kapasitesi ve artan ihracat performansıyla ekonomik kalkınmamızın da lokomotiflerinden biridir. Ülkemiz son yıllarda, savunma sanayisinde, bölgesinde ve küresel düzeyde adından en çok bahsettiren ülkelerden biri haline gelmiştir. Savunma sanayimiz, dost ve müttefik ülkelerin ihtiyaçlarını karşılamanın ve küresel barışa katkı vermenin yanı sıra ihracat ve istihdam alanlarında da ülkemize önemli katkı sağlayan sektörlerin başında gelmektedir. Savunma ve havacılık sanayisi ihracatı, yalnızca döviz kazandıran bir faaliyet olmanın ötesinde, Türkiye’nin teknolojik kapasitesini, dış politika etki alanını ve sanayi ekosisteminin niteliğini doğrudan etkileyen stratejik unsurlardan biridir.</p><p>Yüksek AR-GE ve mühendislik yoğunluğu sayesinde savunma ve havacılık sanayisi ürünleri, geleneksel ihracat kalemlerine kıyasla çok daha yüksek katma değer üretmekte; ithal girdi bağımlılığının görece düşük olması ise cari denge açısından önemli bir avantaj sağlamaktadır. Bununla birlikte savunma ve havacılık ihracatı, alıcı ülkelerle uzun vadeli bakım, eğitim ve modernizasyon ilişkileri kurarak kalıcı stratejik bağlar yaratmakta ve Türkiye’nin jeopolitik görünürlüğünü arttırmaktadır. Savunma ve havacılık sanayisinde geliştirilen teknolojilerin elektronik, yazılım, malzeme ve enerji gibi sivil alanlara yayılması da bu sektörün ihracat performansını, sanayimizin yapısal dönüşümü açısından kritik hale getirmektedir. Savunma ve havacılık sanayisi ihracatımız, son 10 yılda yüzde 500 oranında artmış; 2025 yılında 10 milyar doların üzerine çıkarak 2028 ihracat hedeflerimizin 3 yıl öncesinden gerçekleşmesini sağlamıştır....</p>", "mesajlar", "dergi-icerigi", "images/msi/saha-2026-ozel-sayilari-ticaret-bakanligi-mesaji.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-ticaret-bakanligi-mesaji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "mesajlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 283, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 13755, 11279)",
        params!["sarsilmaz-sar-556-mt-ile-dayaniklilik-ve-performansin-yeni", "SARSILMAZ, SAR 556 MT ile Dayanıklılık ve Performansın Yeni Standartlarını Belirliyor", "SARSILMAZ, SAR 556 MT ile Dayanıklılık ve Performansın Yeni Standartlarını Belirliyor", "<p></p><p><em>Tim ve manga seviyesinde kullanılabilen hafif makineli tüfekler, piyadenin önde gelen ateş destek unsurları arasında yer alıyor. SARSILMAZ’ın 150 yıllık tecrübesi ve teknoloji şirketi kimliğiyle tasarladığı SAR 556 MT hafif makineli tüfek, teknik özellikleri ve performansıyla sınıfının liderleri arasında yer alıyor. Özellikle çalışma sıcaklığı aralığı ve gövde ömrü gibi parametrelerde birçok rakibini geride bırakan SAR 556 MT, yeni standartları belirliyor.</em></p><p> ...</p>", "tanitici-makaleler", "dergi-icerigi", "images/msi/sarsilmaz-sar-556-mt-ile-dayaniklilik-ve-performansin-yeni.webp", "MSI Dergisi Yayın Kurulu", "2026-09-10", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["sarsilmaz-sar-556-mt-ile-dayaniklilik-ve-performansin-yeni"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "namlulu-silahlar"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 282, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 14000, 11480)",
        params!["teops-tasarim-uretim-ve-test-kabiliyetlerini-yeni-tesisiyl", "TEOPS; Tasarım, Üretim ve Test Kabiliyetlerini Yeni Tesisiyle Güçlendiriyor", "TEOPS; Tasarım, Üretim ve Test Kabiliyetlerini Yeni Tesisiyle Güçlendiriyor", "<p><em>Son kullanıcılar, tedarik makamları ve ana entegratörler, günümüzde; kısa sürede kullanıma hazır hale gelebilen, seri üretilebilen ve tek adresten tedarik edilebilen entegre çözümleri daha fazla tercih ediyor. TEOPS, savunma ve havacılık sanayisindeki bu değişime hem mühendislik kabiliyetlerini hem de tasarım, test ve üretim altyapılarını geliştirerek uyum sağlıyor. İhtiyaçtan, kullanıma hazır ürünlerin seri üretimine kadar tüm süreci kapsayan, uçtan uca çözümler sunan firma, yeni tesisiyle bu konudaki iddiasını güçlendiriyor.</em></p><p> </p><p>Modern muharebe sahasının gereksinimlerinin bir yansıması olarak savunma ve havacılık sanayisindeki ihtiyaçlar da giderek daha karmaşık hale geliyor. Bu ortamda, alt sistem geliştirme yeteneğinin ötesine geçmek ve bütünleşik sistem çözümleri sunabilmek giderek daha fazla önem kazanıyor. Alt sistemlerden tam entegre çözümlere uzanan mühendislik yaklaşımı da sektörde stratejik önem kazanıyor....</p>", "tanitici-makaleler", "dergi-icerigi", "images/msi/teops-tasarim-uretim-ve-test-kabiliyetlerini-yeni-tesisiyl.webp", "MSI Dergisi Yayın Kurulu", "2026-09-08", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["teops-tasarim-uretim-ve-test-kabiliyetlerini-yeni-tesisiyl"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 281, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 14245, 11680)",
        params!["stest-sektoru-bilgilendiriyor-bolum-ii", "STEST Sektörü Bilgilendiriyor: Bölüm II", "STEST’ten Kara Platformlarının Test ve Doğrulama Hizmetleri için Kritik Altyapılar", "<p>STEST hakkında daha fazla bilgi için firmanın web sitesini ziyaret edebilirsiniz: https://stest.com.tr/</p><p></p><p><em>Askeri kara platformları; zırhlı araçlardan insansız kara araçlarına, taktik tekerlekli sistemlerden ağır lojistik platformlara kadar geniş bir ürün yelpazesini kapsıyor. Bu platformlar; görev yaptığı ortamdan, gerçekleştirdiği manevralardan ve kullandığı silah ve sensörlerden kaynaklanan çeşitli çevresel etkilere maruz kalıyor. İlgili standartlar uyarınca titiz bir şekilde gerçekleştirilen test ve doğrulama süreçleri, kara platformlarının, sahada görev yapan personelin en önemli yardımcısı haline gelmesinde kritik bir rol oynuyor. Toplam 4 bölümden oluşan bu yazı dizisi, test ve doğrulama faaliyetlerinin nasıl yapılması gerektiği konusunda sektörü bilgilendirmek ve STEST’in bu alandaki hizmetlerini sektöre anlatmak amacıyla hazırlanmıştır. Yazı dizisinin ikinci bölümünde, kara platformlarının test ve doğrulama süreçleri ele alınacaktır.</em>...</p>", "tanitici-makaleler", "dergi-icerigi", "images/msi/stest-sektoru-bilgilendiriyor-bolum-ii.webp", "MSI Dergisi Yayın Kurulu", "2026-09-07", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["stest-sektoru-bilgilendiriyor-bolum-ii"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "radarlar-ve-sensorler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "namlulu-silahlar"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 280, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 14490, 11881)",
        params!["nero-endustri-muharebe-sahasindaki-gorunmeyen-tehditleri-t", "NERO Endüstri, Muharebe Sahasındaki Görünmeyen Tehditleri, TWL-200 Taktik Giyilebilir Lazer Uyarı Sistemi ile Tespit Edecek", "NERO Endüstri, Muharebe Sahasındaki Görünmeyen Tehditleri, TWL-200 Taktik Giyilebilir Lazer Uyarı Sistemi ile Tespit Edecek", "<p></p><p style=\"text-align: justify;\"><em>Lazer mesafe ölçer ve lazer hedef işaretleyici gibi lazer tabanlı keşif ve hedefleme sistemleri, silahlı kuvvetler tarafından, hassas güdümlü mühimmatların yönlendirilmesi ve topçu atışlarının tanzim edilmesi gibi amaçlarla yoğun olarak kullanılıyor. Bu sistemler, GNSS erişiminin kısıtlandığı ortamlarda da hedefleme ve mesafe tayini görevlerini destekleyebiliyor. Bu kullanım yaygınlığı, genelde zırhlı araçlar gibi büyük platformlara entegre edilen lazer uyarı sistemlerinin daha da yaygınlaşması ve sahada görev yapan her birimde kullanılabilmesi ihtiyacını ortaya çıkarıyor. NERO Endüstri, alanında öncü ve yenilikçi bir çözüm olan TWL-200 Taktik Giyilebilir Lazer Uyarı Sistemi ile askerin, standart teçhizatının yanı sıra üzerinde taşıyabileceği etkili bir çözüm sunuyor.</em></p><p style=\"text-align: justify;\"> ...</p>", "tanitici-makaleler", "dergi-icerigi", "images/msi/nero-endustri-muharebe-sahasindaki-gorunmeyen-tehditleri-t.webp", "MSI Dergisi Yayın Kurulu", "2026-09-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["nero-endustri-muharebe-sahasindaki-gorunmeyen-tehditleri-t"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 279, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 14735, 12082)",
        params!["nero-grup-ekosistemi-bir-savunma-sisteminin-fikirden-sahay", "NERO Grup Ekosistemi, Bir Savunma Sisteminin Fikirden Sahaya Uzanan Ömür Devrinin Her Aşamasında Göreve Hazır", "NERO Grup Ekosistemi, Bir Savunma Sisteminin Fikirden Sahaya Uzanan Ömür Devrinin Her Aşamasında Göreve Hazır", "<p></p><p><em>Günümüzde son kullanıcılar ve tedarik makamları, envantere alacakları ürünlerden, teknik başarımın yanı sıra ömür devri boyunca idame edilebilirlik, güvenilirlik ve etkinlik bekliyor. Bu beklenti de ürünlerin, ömür devrini de göz önüne alan bütüncül bir yaklaşımla geliştirilebilmesini gerektiriyor. Söz konusu yaklaşım ise bir ekosistem kapsamında oluşturulacak sinerjiyle hayata geçirilebiliyor. NERO Grup firmaları, bu sinerjiyi kendi içlerinde oluşturabilen bir ekosistem olarak öne çıkıyor.</em></p><p> ...</p>", "tanitici-makaleler", "dergi-icerigi", "images/msi/nero-grup-ekosistemi-bir-savunma-sisteminin-fikirden-sahay.webp", "MSI Dergisi Yayın Kurulu", "2026-08-06", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["nero-grup-ekosistemi-bir-savunma-sisteminin-fikirden-sahay"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 278, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 14980, 12283)",
        params!["stest-sektoru-bilgilendiriyor-bolum-i", "STEST Sektörü Bilgilendiriyor: Bölüm I", "STEST’ten Sektörün Rekabet Gücüne ve Seri Üretim Hamlesine Kritik Test Desteği", "<p>STEST hakkında daha fazla bilgi için firmanın web sitesini ziyaret edebilirsiniz: https://stest.com.tr/</p><p></p><p><em>Savunma ve havacılık sektöründe projelerin başarısı, sadece tasarım ve üretim kabiliyetlerine bağlı değildir. Bir sistemin sahada güvenilir şekilde görev yapabilmesi; test ve doğrulama süreçlerinin ne kadar etkin, kapsamlı ve standartlara uygun yürütüldüğüyle de doğrudan ilişkilidir. Bu tanımlamaya uygun şekilde yürütülen bir test ve doğrulama süreci, seri üretime geçme kararının alınmasında en önemli faktörlerden biridir. STEST, savunma ve havacılık sektörüne sunduğu akredite test hizmetleriyle bu süreçte kritik bir rol üstleniyor. Toplam 4 bölümden oluşan bu yazı dizisi, test ve doğrulama faaliyetlerinin nasıl yapılması gerektiği konusunda sektörü bilgilendirmek ve STEST’in bu alandaki hizmetlerini sektöre anlatmak amacıyla hazırlanmıştır. Yazı dizisinin ilk bölümünde, test ve doğrulama süreçlerinin önemi vurgulanacak; AECTP 300 standardı tanıtılacak; akredite ve bağımsız laboratuvarların neden kritik bir kabiliyet olduğu anlatılacaktır.</em>...</p>", "tanitici-makaleler", "dergi-icerigi", "images/msi/stest-sektoru-bilgilendiriyor-bolum-i.webp", "MSI Dergisi Yayın Kurulu", "2026-08-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["stest-sektoru-bilgilendiriyor-bolum-i"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 15225, 12484)",
        params!["insansiz-hava-araci-veri-baglari-operasyonel-zorluklar-coz", "İnsansız Hava Aracı Veri Bağları: Operasyonel Zorluklar, Çözüm Yöntemleri ve AKSON Veri Bağı Sistemi", "İnsansız Hava Aracı Veri Bağları: Operasyonel Zorluklar, Çözüm Yöntemleri ve AKSON Veri Bağı Sistemi", "<p></p><p><em>İnsanız hava araçlarına (İHA) komuta edilmesi ve taşıdıkları faydalı yükler aracılığıyla elde edilen verilerin ihtiyaç duyulan birimlere aktarılması, veri bağları ile mümkün olmaktadır. Bu veri bağlarının kabiliyetleri ve operasyon sahasında ortaya çıkan zorlukların üstesinden gelme kabiliyetleri, İHA’ların görev başarımlarının anahtarlarından biridir. Bu doğrultuda İHA veri bağlarının yerli ve milli olması, İHA’nın ve faydalı yüklerinin yerli ve milli olması kadar önemli ve kritik bir konudur. Bu makalede, veri bağı kavramı hakkında bilgi verilecek; ardından da çalıştığı bant aralığında Türkiye’nin tek yerli ve milli veri bağı konumunda olan Meteksan Savunma’nın AKSON C-Bant Veri Bağı’nın İHA üreticilerine ve kullanıcılara sağladığı avantajlar anlatılacaktır.</em></p><p> ...</p>", "makaleler-analizler", "dergi-icerigi", "images/msi/insansiz-hava-araci-veri-baglari-operasyonel-zorluklar-coz.webp", "MSI Dergisi Yayın Kurulu", "2026-06-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["insansiz-hava-araci-veri-baglari-operasyonel-zorluklar-coz"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "makaleler-analizler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 15470, 12685)",
        params!["milli-denizalti-milden-nasil-olmali", "Milli Denizaltı (MİLDEN) Nasıl Olmalı?", "Klasik mi, Nükleer mi?", "<p style=\"text-align: left;\"><strong>M. Haluk BAYBAŞ / mhbaybas@gmail.com</strong></p><p><strong>Em. Deniz Kurmay Kıdemli Albay</strong></p><p style=\"text-align: center;\">...</p>", "makaleler-analizler", "dergi-icerigi", "images/msi/milli-denizalti-milden-nasil-olmali.webp", "MSI Dergisi Yayın Kurulu", "2026-01-02", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["milli-denizalti-milden-nasil-olmali"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "makaleler-analizler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 15715, 12886)",
        params!["kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti", "Küresel Savaş Gemisi Pazarında Son Bir Yıl Böyle Geçti:", "Büyük Oyunculardan Farklı Yaklaşımlar", "<p>Son bir yıl içinde, ülkelerin savaş gemisi temin etme çabalarını ele aldığımız yazı dizisinin sonunda Çin, ABD ve Rusya’ya göz atacağız. Üç ülke de global güç mücadelesinin parçaları ve bahsedeceğimiz platform adet ve tipleri de diğer ülkelerden oldukça farklı. Bizim odağımızda, her zamanki gibi politika değil, harp sanatı ve teknolojisi var. Konu, sadece bu üç ülke için bile bir kitap kalınlığa erişebilecek potansiyele sahip olsa da ilgi alanımızdaki gemileri ve modernizasyon faaliyetlerini, korvet ve üstü suüstü gemileri ile denizaltı sınırında tutmanın uygun olduğunu değerlendirdik.</p><p>Ülkelerin faaliyetlerine değinmeye başlamadan önce, gemi adedinin tek başına bir güç metriği olmadığının altını, bir kez daha çizmemiz gerekiyor. Vuruş gücünden kendini savunma kabiliyetine, strateji ve taktiğin kullanımından lojistiğe ve en önemlisi, insan gücüne kadar onlarca parametre, savaş gücüne etki etmektedir. Askeri okullar da bu parametreleri en iyi şekilde hesaplamak ve kullanımını öğretmek için varlıklarını sürdürmektedirler. Biz, işin sadece sanayi yönü ile ilgileniyoruz ve bu dizide de son bir yılda geçekleşen platform teminine odaklandık.</p><p><strong> </strong>...</p>", "makaleler-analizler", "dergi-icerigi", "images/msi/kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti.webp", "MSI Dergisi Yayın Kurulu", "2026-01-02", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "makaleler-analizler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 283, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 15960, 13087)",
        params!["kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti", "Küresel Savaş Gemisi Pazarında Son Bir Yıl Böyle Geçti:", "Orta Doğu ve Afrika’da Yoğun Rekabet Devam Ediyor", "<p><strong> </strong></p><p>Son bir yıl içinde, ülkelerin savaş gemisi temin etme çabalarını ele aldığımız yazı dizisine, Orta Doğu ve Afrika ile devam ediyoruz. Bu sefer, kuzey batıdan başlayıp büyük bir daire çizelim.</p><p> ...</p>", "makaleler-analizler", "dergi-icerigi", "images/msi/kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti.webp", "MSI Dergisi Yayın Kurulu", "2026-01-02", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "makaleler-analizler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 282, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 16205, 13288)",
        params!["denizden-guc-aktarimi-icin-ucak-gemisi", "Denizden Güç Aktarımı için Uçak Gemisi", "Denizden Güç Aktarımı için Uçak Gemisi", "<p style=\"text-align: center;\">ABD, uçak gemilerini, küresel ölçekte güç aktarımı aracı olarak kullanıyor.</p><p></p><p><strong>M. Haluk BAYBAŞ / mhbaybas@gmail.com</strong>...</p>", "makaleler-analizler", "dergi-icerigi", "images/msi/denizden-guc-aktarimi-icin-ucak-gemisi.webp", "MSI Dergisi Yayın Kurulu", "2026-01-02", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["denizden-guc-aktarimi-icin-ucak-gemisi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "makaleler-analizler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 281, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 16450, 13489)",
        params!["yuzen-kaleler", "Yüzen Kaleler", "Deniz Kontrolü için Güçlü Suüstü Filosu", "<p><strong>M. Haluk BAYBAŞ / mhbaybas@gmail.com</strong></p><p><strong>Em. Deniz Kurmay Kıdemli Albay</strong></p><p>...</p>", "makaleler-analizler", "dergi-icerigi", "images/msi/yuzen-kaleler.webp", "MSI Dergisi Yayın Kurulu", "2026-01-02", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["yuzen-kaleler"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "makaleler-analizler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 280, ?9, 4, 0, 0.0, 1, ?10, ?11, 0, NULL, 16695, 13689)",
        params!["kim-teknoloji-bor-kaynagini-yenilikci-cozumlere-donusturer", "KİM Teknoloji, Bor Kaynağını Yenilikçi Çözümlere Dönüştürerek Küresel Rekabette Öne Çıkıyor", "KİM Teknoloji, Bor Kaynağını Yenilikçi Çözümlere Dönüştürerek Küresel Rekabette Öne Çıkıyor", "<p style=\"text-align: center;\">KİM Teknoloji, sahip olduğu üretim, test ve kalite altyapılarıyla rekabette öne çıkan çözümler sunuyor.</p><p style=\"text-align: center;\"></p><p><em>KİM Teknoloji, yenilikçi yaklaşımları ve üstün teknolojik yetkinlikleri ile ileri malzeme teknolojileri alanında küresel bir oyuncu olma hedefine doğru emin adımlarla ilerliyor. Savunma sektörü ile sivil sektörlerin kritik ihtiyaçlarına yönelik özel çözümleri ile dikkat çeken KİM Teknoloji, uluslararası rekabette de öne çıkıyor. Yüksek katma değerli ürünleri ile Türkiye’nin stratejik hedeflerine katkıda bulunan firma, küresel ölçekte üretim anlayışıyla müşterilerine yenilikçi ve sürdürülebilir çözümler sunma misyonunu da başarıyla gerçekleştiriyor.</em>...</p>", "advertorial", "dergi-icerigi", "images/msi/kim-teknoloji-bor-kaynagini-yenilikci-cozumlere-donusturer.webp", "MSI Dergisi Yayın Kurulu", "2026-01-14", "KURUMSAL SPONSOR", "ADVERTORIAL • TANITIM"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["kim-teknoloji-bor-kaynagini-yenilikci-cozumlere-donusturer"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "advertorial"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 279, ?9, 4, 0, 0.0, 1, ?10, ?11, 0, NULL, 16940, 13890)",
        params!["gurbag-savunmadan-yuksek-performansli-ve-moduler-hareket-p", "Gürbağ Savunma’dan Yüksek Performanslı ve Modüler Hareket Platformu", "DUO SERİSİ Pan-Tilt Sistemi", "<p><em>Gürbağ Savunma tarafından geliştirilen ve üretilen DUO SERİSİ Pan-Tilt Sistemi, gelişmiş hareket kabiliyeti, özgün yazılım altyapısı ve yüksek taşıma kapasitesi ile ön plana çıkan bir sistemdir. Tamamen yerli tasarım ve üretim olan DUO SERİSİ,</em><strong><em> </em></strong><em>sunduğu kullanım esnekliği sayesinde hem askeri hem de sivil uygulamalarda çok yönlü çözümler sağlar.</em></p><p> </p><p><strong>Özgün Yazılım ve Gelişmiş Haberleşme</strong>...</p>", "advertorial", "dergi-icerigi", "images/msi/gurbag-savunmadan-yuksek-performansli-ve-moduler-hareket-p.webp", "MSI Dergisi Yayın Kurulu", "2026-01-13", "KURUMSAL SPONSOR", "ADVERTORIAL • TANITIM"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["gurbag-savunmadan-yuksek-performansli-ve-moduler-hareket-p"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "advertorial"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 278, ?9, 4, 0, 0.0, 1, ?10, ?11, 0, NULL, 17185, 14091)",
        params!["atesci-20-yili-askin-tecrubesiyle-savunma-sanayisinde-guve", "Ateşçi: 20 Yılı Aşkın Tecrübesiyle Savunma Sanayisinde Güven, Teknoloji ve Yeniliğin Adresi", "Ateşçi: 20 Yılı Aşkın Tecrübesiyle Savunma Sanayisinde Güven, Teknoloji ve Yeniliğin Adresi", "<p></p><p><em>2002 yılında kurulan Ateşçi, bugün geldiği noktada, bir makine üreticisinin ötesinde; entegre üretim hatları tasarlayan, kuran ve uzun vadeli iş birlikleriyle öne çıkan küresel bir çözüm ortağı olarak öne çıkıyor. Kurulduğu günden bu yana, savunma sanayisinde hem Türkiye’deki hem de dünyadaki gelişmeleri yakından takip eden firma, yüksek güvenilirlikte ve müşteri ihtiyaçlarına göre özelleştirilmiş entegre makine üretimi çözümleri sunuyor. Dünya genelinde hizmet sağlayabilme kapasitesiyle iş ortaklarının üretim hedeflerine ulaşmalarını sağlayan Ateşçi, müşterilerinin üretim kapasitesinin yanı sıra kalite, verimlilik ve operasyonel süreklilik kabiliyetlerini destekliyor.</em></p><p> ...</p>", "advertorial", "dergi-icerigi", "images/msi/atesci-20-yili-askin-tecrubesiyle-savunma-sanayisinde-guve.webp", "MSI Dergisi Yayın Kurulu", "2025-12-17", "KURUMSAL SPONSOR", "ADVERTORIAL • TANITIM"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["atesci-20-yili-askin-tecrubesiyle-savunma-sanayisinde-guve"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "advertorial"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 4, 0, 0.0, 1, ?10, ?11, 0, NULL, 17430, 14292)",
        params!["sarsilmaz-4-kitada-uluslararasi-organizasyonlarda-turkiyey", "SARSILMAZ, 4 Kıtada Uluslararası Organizasyonlarda Türkiye’yi Temsil Ederek Küresel Rekabette Gücüne Güç Kattı", "SARSILMAZ, geçtiğimiz yıllarda olduğu gibi 2025’te de yoğun bir fuar takvimini takip etti. 80’den fazla ülkeye ihracat yapan SARSILMAZ, uluslararası iş geliştirme faaliyetlerini; Avrupa, Kuzey ve Güney Amerika ve Asya kıtalarında katıldığı 14 fuarla sürdürdü. Bunlar arasında, SARSILMAZ’ın ilk defa katıldığı fuarlar da yer aldı. Mevcut pazarlarındaki iddiasını sürdüren SARSILMAZ, bu fuarlarda yer alarak yeni pazarlar için potansiyelini ve iddiasını arttırdı.", "<p>Bu yılın fuar takvimine, ABD’de gerçekleştirilen SHOT SHOW ile görkemli bir başlangıç yapan SARSILMAZ, ilerleyen aylarda ise IDEX 2025, Enforce Tac 2025, IWA 2025, Homeland Security Expo, LAAD, IQDEX, FEINDEF, LIMA, Indo Defence Expo &amp; Forum, DSEI, D&amp;S (Defense &amp; Security) 2025, BAMEX 2025, MILIPOL Paris 2025 ve EDEX 2025 fuarlarına katıldı. Bunlardan Homeland Security Expo, IQDEX, FEINDEF ve BAMEX, SARSILMAZ’ın ilk kez boy gösterdiği etkinlikler oldu. SARSILMAZ’ın ABD distribütörü SAR USA de bu ülkede katıldığı 2 etkinlikle SARSILMAZ ürünlerinin bayrağını taşıdı.</p><p> </p><p><strong>SARSILMAZ Grup Şirketleri, İhtiyaç Makamları için Ortak Adres Oldu</strong>...</p>", "advertorial", "dergi-icerigi", "images/msi/sarsilmaz-4-kitada-uluslararasi-organizasyonlarda-turkiyey.webp", "MSI Dergisi Yayın Kurulu", "2025-12-01", "KURUMSAL SPONSOR", "ADVERTORIAL • TANITIM"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["sarsilmaz-4-kitada-uluslararasi-organizasyonlarda-turkiyey"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "advertorial"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 4, 0, 0.0, 1, ?10, ?11, 0, NULL, 17675, 14493)",
        params!["stm-35inci-yila-guclu-adimlarla-giriyor", "STM, 35’inci Yıla Güçlü Adımlarla Giriyor", "Türkiye’nin savunma ve teknoloji alanındaki lider şirketlerinden STM, 2025 yılında hem Mavi Vatan’da hem Gök Vatan’da hem de uluslararası arenada dikkat çekici başarılara imza attı. STM, 2026’da kutlayacağı 35. yılına, küresel ölçekte büyüyen etkisi ve stratejik projelerdeki kritik rolüyle güçlü bir giriş yapmaya hazırlanıyor.", "<p><strong>Mavi Vatan’da Yeni Dönem</strong></p><p>STM, Türk Deniz Kuvvetlerine güç verecek projeleriyle 2025’te önemli bir ivme yakaladı. MİLGEM İstif Sınıfı Fırkateyn Projesi kapsamında TAİS OG-STM iş ortaklığında inşa edilen İZMİR, İZMİT ve İÇEL fırkateynlerinin aynı yıl içinde denize indirilmesi, Türk denizcilik tarihinde atılan en büyük adımlardan biri oldu.</p><p>Türkiye’nin ilk milli hücumbotunun tasarımcısı ve ana yüklenicisi olan STM, platformun ölçekli modelini, ilk kez IDEF 2025’te sergiledi ve Kasım ayında düzenlenen törenle ilk bloğu kızağa aldı....</p>", "advertorial", "dergi-icerigi", "images/msi/stm-35inci-yila-guclu-adimlarla-giriyor.webp", "MSI Dergisi Yayın Kurulu", "2025-12-01", "KURUMSAL SPONSOR", "ADVERTORIAL • TANITIM"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["stm-35inci-yila-guclu-adimlarla-giriyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "advertorial"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 5, 0, 0.0, 1, ?10, ?11, 0, NULL, 17920, 14694)",
        params!["sar-127-mt-altay-tankinin-gucune-guc-katiyor", "SAR 127 MT, Altay Tankının Gücüne Güç Katıyor", "Modern Muharebe Sahasında Tanklar ve Zırhlı Araçların Ateş Gücünü Arttıran Çözüm", "<p><em>Türk savunma sanayi sektörünün lider şirketlerinden SARSILMAZ’ın ürün gamında bulunan SAR 127 MT, ALTAY tankında kullanılıyor. Kalifikasyon süreçlerini başarıyla tamamlayan ve farklı projelerde kullanılan sistem, şimdi ALTAY tankıyla da sahada aktif göreve hazır. Tankın taretinde bulunan uzaktan komutalı silah kulesine (UKSS) entegre edilen silah, ALTAY’a, kara ve hava hedeflerine yönelik etkili bir ateş gücü sağlayacak. SAR 127 MT’nin ateş gücünün yanı sıra UKSS’lere entegre edilmesini kolaylaştıran tasarım özellikleri de ALTAY tankı üzerinde kullanılmasının önünü açan avantajları arasında yer aldı.</em></p><p> </p><p>Yeni seri üretim hattından çıkan ilk ALTAY tankları, UKSS’lerindeki SAR 127 MT ağır makineli tüfeklerle birlikte TSK’ya teslim edildi. Bu teslimat nedeniyle BMC Ankara Tank ve Yeni Nesil Zırhlı Araçlar Üretim Tesisleri’nde, 28 Ekim’de düzenlenen törene, Cumhurbaşkanı Recep Tayyip Erdoğan da katıldı. SAR 127 MT, TSK için sipariş edilen toplam 250 ALTAY tankının hepsinin üzerinde yer alacak....</p>", "advertorial", "dergi-icerigi", "images/msi/sar-127-mt-altay-tankinin-gucune-guc-katiyor.webp", "MSI Dergisi Yayın Kurulu", "2025-11-01", "KURUMSAL SPONSOR", "ADVERTORIAL • TANITIM"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["sar-127-mt-altay-tankinin-gucune-guc-katiyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "advertorial"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 283, ?9, 5, 0, 0.0, 1, ?10, ?11, 0, NULL, 18165, 14895)",
        params!["ges-muhendislik-askeri-treyler-uretimiyle-agir-yuklerin-al", "GES Mühendislik, Askeri Treyler Üretimiyle Ağır Yüklerin Altından Kalkıyor", "GES Mühendislik, Askeri Treyler Üretimiyle Ağır Yüklerin Altından Kalkıyor", "<p><em>Türk savunma ve havacılık sanayisinin ihtiyaç duyduğu çeşitli mekanik ve elektromekanik sistemlerin tasarım, üretim, yazılım geliştirme ve test süreçlerini aynı çatı altında yürütebilen GES Mühendislik, bu sayede ana yüklenicilere anahtar teslim çözümler sağlayabiliyor. Yaptığı altyapı yatırımlarıyla bu alandaki kabiliyetlerini sürekli geliştiren firmanın sunduğu anahtar teslim çözümler arasında; kara ve hava platformlarına yönelik araç üstü ekipman, iklimlendirilmiş sistem kabini, treyler/römork ve yarı römork gibi ürünler de yer alıyor. GES Mühendislik’in bu alanda ortaya koyduğu en son ürünlerden biri de askeri operasyonlarda lojistik destek sağlamanın yanı sıra askeri araçların ve malzemelerin etkili bir şekilde taşınmasını sağlayan askeri treyler oldu. Müşteri isterleri doğrultusunda, 13 m uzunluğa ve 70 ton taşıma kapasitesine kadar askeri treyler üretebilen GES Mühendislik, satış sonrası destek hizmetiyle de kullanıcının yanında olmayı sürdürüyor.</em></p><p> </p><p>Türk savunma ve havacılık sanayisine yönelik özgün alt sistemler geliştiren GES Mühendislik, ayrıca üretim, test ve mühendislik hizmetleriyle de sektörün ihtiyaçlarını karşılamaya yönelik etkili çözümler sunuyor. GES Mühendislik’in sunduğu çözümler arasında, askeri treyler de bulunuyor. Askeri operasyonlarda lojistik destek sağlamanın yanı sıra askeri araçların ve malzemelerin etkili bir şekilde taşınmasını sağlamak için tasarlanan bir yük taşıma sistemi olan askeri treyler; çeki oku, döner tabla ya da kanca gibi bir bağlantı arayüzü aracılığıyla çekildiği taşıta bağlanıyor. En az bir dingile sahip olan ve zorlu çevresel koşullarda görev yapmak üzere üretilen bu sistemler, askeri birlikler açısından sahada kritik ihtiyaçları karşılıyor....</p>", "advertorial", "dergi-icerigi", "images/msi/ges-muhendislik-askeri-treyler-uretimiyle-agir-yuklerin-al.webp", "MSI Dergisi Yayın Kurulu", "2025-10-01", "KURUMSAL SPONSOR", "ADVERTORIAL • TANITIM"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["ges-muhendislik-askeri-treyler-uretimiyle-agir-yuklerin-al"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "advertorial"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 282, ?9, 5, 0, 0.0, 1, ?10, ?11, 0, NULL, 18410, 15096)",
        params!["gurbag-savunma-ve-teknoloji", "Gürbağ Savunma ve Teknoloji", "GSPT DUAL Pan-Tilt Sistemi Entegrasyonu ile Farklı Faydalı Yükleri Bağımsız Olarak Yönlendirin!", "<p style=\"text-align: justify\"><em>Gürbağ Savunma ve Teknoloji, tasarlayıp ürettiği kontrol ve yönlendirme sistemleriyle savunma ve havacılık sektörünün özel ihtiyaçlarına cevap veriyor. Bunlardan GSPT DUAL Pan-Tilt Sistemi, farklı faydalı yüklerin bağımsız eksenlerde aynı anda yönlendirilmesine ihtiyaç duyulan hava savunma sistemi gibi uygulamalarda, sistem entegratörlerine tek elden çözüm sağlıyor.</em></p><p style=\"text-align: justify\">Gürbağ Savunma ve Teknoloji’nin özgün olarak geliştirdiği kontrol ve yönlendirme sistemlerinden GSPT DUAL Pan-Tilt Sistemi, birden fazla faydalı yükün aynı anda kullanılmasına olanak sağlıyor. GSPT DUAL’ın üzerine, ek bir pan-tilt sistemi daha entegre edilebiliyor. Bu ek sistemin de bağımsız bir şekilde hareket edebilmesi sayesinde, farklı faydalı yükler, aynı anda, farklı açılarda yönlendirilebiliyor. Örneğin, bir hava savunma sisteminin radarı, GSPT DUAL’ın üzerine ayrı bir pan-tilt sistemi ile entegre edildiğinde, GSPT DUAL’ın taşıdığı elektro-optik faydalı yüklerden bağımsız hareket edebiliyor. Böylece radar, tam kapsamda arama ve izleme görevine devam ederken elektro-optik sistemler, hedef takibi ve teşhisi işlevini yerine getirebiliyor.</p><p style=\"text-align: justify\"><strong>Tam Entegrasyon, Kontrol Arayüzünü de Kapsıyor</strong>...</p>", "advertorial", "dergi-icerigi", "images/msi/gurbag-savunma-ve-teknoloji.webp", "MSI Dergisi Yayın Kurulu", "2025-03-01", "KURUMSAL SPONSOR", "ADVERTORIAL • TANITIM"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["gurbag-savunma-ve-teknoloji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "advertorial"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 281, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 18655, 15297)",
        params!["igef-2025-kuresel-is-birliklerinin-kapilarini-araladi", "İGEF 2025, Küresel İş Birliklerinin Kapılarını Araladı", "İGEF 2025, Küresel İş Birliklerinin Kapılarını Araladı", "<p></p><p><em>İç Güvenlik Ekipmanları Fuarı 2025 (İGEF 2025), 18-20 Eylül tarihleri arasında, ATO Congresium Kongre ve Sergi Merkezinde gerçekleştirildi. Bu yıl 2’nci kez düzenlenen fuar, Türkiye’deki kurum ve kuruluşların temsilcilerinin yanı sıra 37 ülkeden karar verici heyetlere ve toplamda 60 ülkenin askerî ve emniyet ataşesine ev sahipliği yaptı. Fuar sonuç raporunda yer alan bilgilere göre; İGEF 2025’e, ulusal ve uluslararası 200’e yakın kurum ve kuruluş katıldı. Ayrıca 4 ülkeden içişleri bakanı, 17 ülkeden emniyet genel müdürü, 16 ülkeden jandarma genel komutanı ve 23 ülkeden de büyükelçi, askerî ve emniyet ataşesi düzeyinde katılım gerçekleşti.</em></p><p> ...</p>", "ozel-dosyalar", "dergi-icerigi", "images/msi/igef-2025-kuresel-is-birliklerinin-kapilarini-araladi.webp", "MSI Dergisi Yayın Kurulu", "2025-10-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["igef-2025-kuresel-is-birliklerinin-kapilarini-araladi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlik-sonrasi-ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 280, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 18900, 15497)",
        params!["expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan", "EXPOLINK Fuarcılık Yönetim Kurulu Başkanı Zeki ERDOĞAN:", "“2026’da daha büyük, daha güçlü ve uluslararası bilinirliği artmış bir İGEF katılımcıları bekliyor olacak.”", "<p></p><p><em>EXPOLINK Fuarcılık Yönetim Kurulu Başkanı Zeki Erdoğan ile yaptığımız söyleşide aldığımız bilgilere göre, İç Güvenlik Ekipmanları Fuarı (İGEF) 2025, 2024 yılında gerçekleştirilen ilk fuara göre yenilikler içeren ve istatistiklerini geliştiren bir etkinlik oldu. İGEF 2026 ise İGEF 2025’in istatistiklerini geçmeyi ve adına eklenen “uluslararası” ifadesi ile çıtayı yükseltmeyi hedefliyor. Erdoğan ile gerçekleştirdiğimiz söyleşide hem İGEF 2025’i değerlendirdik hem de İGEF 2026 ile ilgili planları ilk ağızdan dinledik.</em></p><p> ...</p>", "ozel-dosyalar", "dergi-icerigi", "images/msi/expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan.webp", "MSI Dergisi Yayın Kurulu", "2025-10-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlik-sonrasi-ozel-dosyalar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 279, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 19145, 15698)",
        params!["tskgv-genel-muduru-bilal-topcu", "TSKGV Genel Müdürü Bilal TOPÇU:", "“Hedefimiz, IDEF’i her dönemde içeriği derinleşen, etkisi artan ve küresel ölçekte daha belirgin bir rol üstlenen bir platforma dönüştürmek.”", "<p><em>Türk Silahlı Kuvvetlerini Güçlendirme Vakfı (TSKGV), sektörün, özellikle platform ihracatında çıtayı yükselttiği bir etkinlik olan IDEF 2025’in ardından, IDEF 2027 için çalışmalarına başladı. IDEF 2027, 3-9 Mayıs 2027 tarihleri arasında, İstanbul Fuar Merkezi’nde düzenlenecek. MSI Dergisi’nin sorularını yanıtlayan TSKGV Genel Müdürü Bilal Topçu; IDEF 2025’ten aldıkları notlar ışığında IDEF 2027’de hayata geçirmeyi planladıkları uygulamaları, IDEF 2027 üzerinde çalışan ekibin yeni organizasyonunu ve fuarla ilgili hedeflerini anlattı.</em></p><p> </p><p><strong>MSI Dergisi:</strong> IDEF 2025, önceki yıllardan farklı bir fuar alanında ve farklı bir organizasyon firması tarafından gerçekleştirildi. Fuar alanında yeni bir yerleşim düzeni uygulandı. Ayrıca IDEF Terminal, IDEF Port gibi farklı lokasyonlarda sergileme yapılması uygulaması da IDEF 2025’in yenilikleri arasındaydı. Bu yenilikler fuar katılımcıları tarafından nasıl karşılandı?...</p>", "soylesiler", "dergi-icerigi", "images/msi/tskgv-genel-muduru-bilal-topcu.webp", "MSI Dergisi Yayın Kurulu", "2026-08-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["tskgv-genel-muduru-bilal-topcu"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "soylesiler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "namlulu-silahlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 278, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 19390, 15899)",
        params!["saha-2026-deniz-sistemleri-ozel-sayisi-sarsilmaz", "SAHA 2026 Deniz Sistemleri Özel Sayısı / SARSILMAZ", "TR Mekatronik Genel Müdürü Dr. Fatih ÇEVİK: “30 mm ve 35 mm top sistemleri, deniz platformlarının hem uzak mesafedeki tehditleri baskı altına alması hem de yakın savunmada bir kuvvet çarpanı oluşturması açısından önemli bir konumda yer alıyor.”", "<p></p><p><em>Deniz platformlarına yönelik insansız hava aracı, dron ve kamikaze insansız deniz aracı gibi asimetrik tehditlerin arttığı günümüzde, orta kalibre toplar, bu tehditlere karşı etkin silahlar olarak öne çıkıyor. SARSILMAZ ile TUSAŞ’ın ortak girişimi TR Mekatronik, 30 mm ve 35 mm kalibrede topları özgün olarak geliştirme faaliyetlerinde son aşamaya geldi. Firmanın 20 mm ve 25 mm’lik kara toplarıyla 6 namlulu 20 mm topu da deniz platformlarında görev yapabilecek ürünleri arasında yer alıyor. TR Mekatronik Genel Müdürü Dr. Fatih Çevik, firmanın bu yoğun ürün geliştirme programının güncel durumunu, MSI Dergisi okuyucuları için anlattı.</em></p><p> ...</p>", "soylesiler", "dergi-icerigi", "images/msi/saha-2026-deniz-sistemleri-ozel-sayisi-sarsilmaz.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-deniz-sistemleri-ozel-sayisi-sarsilmaz"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "soylesiler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 19635, 16100)",
        params!["saha-2026-kara-sistemleri-ozel-sayisi-sarsilmaz", "SAHA 2026 Kara Sistemleri Özel Sayısı / SARSILMAZ", "SARSILMAZ Genel Müdür Yardımcısı ve BEST DEFENCE Yönetim Kurulu Üyesi Murat AKALIN: “Odağımız net: Sahada gerçek karşılığı olan, bugünün ihtiyaçlarını karşılayan ve yarının operasyonel gereksinimlerine uyum sağlayan çözümler geliştirmek.”", "<p></p><p><em>SARSILMAZ ve grup şirketleri, SAHA 2026’da, yenilikçi ürünleri ve geniş ürün yelpazeleriyle boy gösteriyor. Ziyaretçilerin, farklı ihtiyaçları karşılayan çok sayıda ürünü bir arada göreceği SARSILMAZ standı, aynı zamanda bu ürünleri bir araya getiren sistem çözümlerini de içeriyor. Böylece SARSILMAZ ve grup şirketlerinin, muharebe sahasındaki değişime ve gelişen tehditlere, sadece ürün bazlı değil entegre çözüm yaklaşımıyla cevap verdiğini gösteriyor. SARSILMAZ Genel Müdür Yardımcısı ve BEST DEFENCE Yönetim Kurulu Üyesi Murat Akalın, entegre çözüm yaklaşımlarını ve bu çözümlerin alt bileşenleri olan yenilikçi ürünlerini, MSI Dergisi okuyucuları için anlattı.</em></p><p> ...</p>", "soylesiler", "dergi-icerigi", "images/msi/saha-2026-kara-sistemleri-ozel-sayisi-sarsilmaz.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-kara-sistemleri-ozel-sayisi-sarsilmaz"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "soylesiler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 19880, 16301)",
        params!["saha-2026-hava-sistemleri-ozel-sayisi-sarsilmaz", "SAHA 2026 Hava Sistemleri Özel Sayısı / SARSILMAZ", "TR Mekatronik Genel Müdürü Dr. Fatih ÇEVİK: “Hava platformlarına yönelik geliştirdiğimiz 6 namlulu 20 mm topumuzun mühendislik prototipi uzun süredir faal durumda ve yaklaşık 1 yıldır aktif olarak atış gerçekleştiriyoruz.”", "<p><em>Türkiye’nin hava platformlarında hedeflediği yerlilik oranının gerçekleşmesi, bu platformların kritik alt sistemlerinin de özgün olarak geliştirilebilmesine bağlı. Taarruz helikopterleri için 20 mm burun topunu başarıyla ortaya koyan ve seri üretime geçiren TR Mekatronik, bu başarısının yanına 6 namlulu 20 mm topu ve silah podunu eklemeye hazırlanıyor. SARSILMAZ ile TUSAŞ’ın ortak girişimi olan firma, yürüttüğü bu projelerde, test ve kalifikasyon süreçlerini yakın zamanda tamamlayıp seri üretime geçmeyi hedefliyor. TR Mekatronik Genel Müdürü Dr. Fatih Çevik, firmanın bu yoğun ürün geliştirme programının güncel durumunu, MSI Dergisi okuyucuları için anlattı.</em></p><p> </p><p><strong>MSI Dergisi:</strong> Fatih Bey, TR Mekatronik, deniz ve hava platformları için kritik öneme sahip orta kalibre topların geliştirilmesi üzerinde çalışıyor. Sizden bu projelerle ilgili son durumu öğrenmek istiyoruz. 6 namlulu 20 mm top geliştirme ile ilgili faaliyetlerinizin son durumu hakkında bilgi alabilir miyiz?...</p>", "soylesiler", "dergi-icerigi", "images/msi/saha-2026-hava-sistemleri-ozel-sayisi-sarsilmaz.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-hava-sistemleri-ozel-sayisi-sarsilmaz"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "soylesiler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "namlulu-silahlar"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 20125, 16502)",
        params!["best-defence-genel-muduru-aziz-karas", "BEST DEFENCE Genel Müdürü Aziz KARAŞ:", "“Kullanıcı odaklı yaklaşımımızı, mühendislik gücümüz ve yenilikçi bakış açımızla birleştirerek sahada karşılığı olan, güvenilir ve etkin sistemler geliştiriyoruz.”", "<p></p><p><em>BEST DEFENCE, bir SARSILMAZ Grup şirketi olmasının getirdiği avantajla uzaktan komutalı silah sistemleri (UKSS) ve entegre savunma çözümlerini tek adresten sağlayabiliyor. UKSS’lerinde SARSILMAZ üretimi silahları kullanan firma, hareket halinde isabet oranı ve seri atış performansı gibi parametrelerde önemli ilerlemeler kaydetti. BEST DEFENCE, kuruluşunun ardından yoğun bir tempoda ilerleyerek ürün ailesine, deniz platformlarına yönelik çözümlerin yanı sıra anti-dron çözümlerini dahil etti. BEST DEFENCE Genel Müdürü Aziz Karaş, tüm bu gelişmeler ve firmanın 2026 hedefleriyle ilgili sorularımızı yanıtladı.</em></p><p> ...</p>", "soylesiler", "dergi-icerigi", "images/msi/best-defence-genel-muduru-aziz-karas.webp", "MSI Dergisi Yayın Kurulu", "2026-02-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["best-defence-genel-muduru-aziz-karas"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "soylesiler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 283, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 20370, 16703)",
        params!["elektro-optikin-oldugu-her-yerde-fotoniks-var", "Elektro-Optik’in Olduğu Her Yerde Fotoniks Var", "Elektro-Optik’in Olduğu Her Yerde Fotoniks Var", "<p style=\"text-align: center;\">Füzyon Görüş Sistemi, iki farklı elektro-optik algılayıcıdan alınan görüntüleri sayısal olarak birleştirerek hem derinlik algısını yükseltiyor hem de durumsal farkındalığı güçlendiriyor.</p><p></p><p><em>Fotoniks, IDEF 2025’te, sahip olduğu elektro-optik teknolojilerini uyguladığı ürünlerin yer aldığı geniş çözüm kümesini sergiledi. Firmanın standı, fuar boyunca; tek er için ürünler arayanlardan geniş alanda kullanılabilecek çözümler arayanlara, piyade silahı üreticilerinden kara aracı platform üreticilerine kadar geniş bir ziyaretçi kümesinin uğrak noktası oldu.</em>...</p>", "ozel-haberler", "dergi-icerigi", "images/msi/elektro-optikin-oldugu-her-yerde-fotoniks-var.webp", "MSI Dergisi Yayın Kurulu", "2025-09-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["elektro-optikin-oldugu-her-yerde-fotoniks-var"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "radarlar-ve-sensorler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 282, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 20615, 16904)",
        params!["uluslararasi-anadolu-kartali-2025-egitimi-ilklere-sahne-ol", "Uluslararası Anadolu Kartalı-2025 Eğitimi, İlklere Sahne Oldu", "Uluslararası Anadolu Kartalı-2025 Eğitimi, İlklere Sahne Oldu", "<p><em>Konya’daki 3. Ana Jet Üs Komutanlığında, 23 Haziran-4 Temmuz tarihleri arasında gerçekleştirilen Uluslararası Anadolu Kartalı-2025 Eğitimi, 3 yeniliğe sahne oldu. “Kriz Safhası Senaryosu” eğitimi ve “Seyir Füzesi Önleme” eğitiminin ilk kez gerçekleştirildiği Anadolu Kartalı’nda, insansız hava araçlarına (İHA) doğrudan bombardıman görevi de ilk defa verildi.</em></p><p> </p><p style=\"text-align: center;\">3. Ana Jet Üs Komutanı Hava Pilot Tümgeneral Mete Kuş...</p>", "ozel-haberler", "dergi-icerigi", "images/msi/uluslararasi-anadolu-kartali-2025-egitimi-ilklere-sahne-ol.webp", "MSI Dergisi Yayın Kurulu", "2025-08-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["uluslararasi-anadolu-kartali-2025-egitimi-ilklere-sahne-ol"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 281, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 20860, 17105)",
        params!["sarsilmaz-gelistirdigi-silahlarin-sahadaki-performansini-y", "SARSILMAZ, Geliştirdiği Silahların Sahadaki Performansını, Yeni ve Yenilikçi Ürünlerle Geleceğe Taşıyor", "SARSILMAZ, Geliştirdiği Silahların Sahadaki Performansını, Yeni ve Yenilikçi Ürünlerle Geleceğe Taşıyor", "<p><em>SARSILMAZ, mükemmeli hedefleyen ve güvenilir üretim vizyonuyla geliştirdiği ürünlerin performansını, AR-GE, ÜR-GE ve mühendislik gücünün yan sıra sahadan gelen bildirimlerle en üst seviyeye taşıyor Askeri birliklerin ve güvenlik güçlerinin sahada güvendiği ve etkin bir şekilde kullandığı ateşli silahlar ve silah sistemleri geliştiren SARSILMAZ, bu temellerin üzerine inşa ettiği yenilikçi ürünlerini IDEF 2025’te sergiliyor. SARSILMAZ ve grup şirketleri, tabancalardan piyade tüfeklerine; 20, 25 ve 30 mm otomatik toplardan uzaktan komutalı silah sistemlerine (UKSS) ve dört bacaklı, silahlı otonom kara robotlarına kadar geniş bir yelpazedeki ürünlerini, fuarın katılımcılarının ve ziyaretçilerinin ilgisine sunuyor.</em></p><p> </p><p>SARSILMAZ ve grup şirketleri, SARSILMAZ’ın yaklaşık 150 yıllık birikimiyle savunma ve havacılık sanayisine entegre, kapsamlı ve yüksek teknolojili çözümler sunuyor. Bu entegre yapı; hafif ve orta kalibreli silahlardan gelişmiş uzaktan komutalı silah sistemlerine, robotik platformlardan orta kalibre top sistemlerine ve mühimmata kadar; kara, hava ve deniz unsurlarının ihtiyaç duyduğu birçok bileşeni, tek kaynaktan sağlıyor....</p>", "ozel-haberler", "dergi-icerigi", "images/msi/sarsilmaz-gelistirdigi-silahlarin-sahadaki-performansini-y.webp", "MSI Dergisi Yayın Kurulu", "2025-07-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["sarsilmaz-gelistirdigi-silahlarin-sahadaki-performansini-y"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "namlulu-silahlar"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 280, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 21105, 17306)",
        params!["turk-savunma-ve-havacilik-sanayisi-kritik-bir-esigi-daha-a", "Türk Savunma ve Havacılık Sanayisi, Kritik Bir Eşiği Daha Aştı", "Endonezya, KAAN’ın İlk Uluslararası Kullanıcısı Olacak", "<p><em>Türkiye, Millî Muharip Uçak KAAN’ın envantere girmesi ile dünya genelinde, kendi jet savaş uçağını tasarlayıp üretebilen sayılı ülkeden biri olmaya hazırlanıyor. Bu kritik eşiğin bir sonraki aşaması olan, bu sınıftaki uçakları ihraç edebilme başarısını ise 11 Haziran’da imza altına aldı. Endonezya’nın başkenti Jakarta’da düzenlenen Indo Defence 2025 fuarı sırasında atılan imzalarla Endonezya, 48 adet KAAN’ı envanterine dahil etmek için ilk adımı attı.</em></p><p> </p><p>İmzalanan sözleşme uyarınca, ilk etapta 48 adet KAAN, 120 aylık bir takvim kapsamında üretilerek Endonezya’ya teslim edilecek. Uçaklarda, KAAN için geliştirilmekte olan yerli motor kullanılacak. Sözleşme kapsamında ayrıca Endonezya’ya havacılık alanında teknoloji transferi de gerçekleştirilecek. Ayrıca, KAAN’ın üretiminde, Endonezya’nın sahip olduğu yerel kabiliyetlerden yararlanılacak....</p>", "ozel-haberler", "dergi-icerigi", "images/msi/turk-savunma-ve-havacilik-sanayisi-kritik-bir-esigi-daha-a.webp", "MSI Dergisi Yayın Kurulu", "2025-07-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["turk-savunma-ve-havacilik-sanayisi-kritik-bir-esigi-daha-a"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 279, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 21350, 17507)",
        params!["turkiye-ve-kktc-dogu-akdenizdeki-arama-kurtarma-operasyonl", "Türkiye ve KKTC, Doğu Akdeniz’deki Arama-Kurtarma Operasyonları için Hazır", "Türkiye ve KKTC, Doğu Akdeniz’deki Arama-Kurtarma Operasyonları için Hazır", "<p><em>Türkiye ve Kuzey Kıbrıs Türk Cumhuriyeti’nin (KKTC) ortaklaşa düzenlediği Şehit Teğmen Caner Gönyeli Arama Kurtarma Tatbikatı’nda bu yıl; karaya düşen yolcu uçağına müdahale, düzensiz göçmenleri taşıyan bir geminin ticari bir gemiyle çarpışması ve ticari gemi ile akaryakıt tankerinin çarpışması senaryoları yer aldı. Tatbikata katılan unsurlar, Türkiye ve KKTC’nin, Doğu Akdeniz’de ortaya çıkabilecek arama-kurtarma ihtiyaçları için hazır olduklarını gösterdiler.</em></p><p> </p><p>Türkiye ve KKTC’nin arama-kurtarma görevini üstlenen kurumlarından unsurların katılımıyla gerçekleştirilen Şehit Teğmen Caner Gönyeli-2025 Arama Kurtarma Davet Tatbikatı, 23-27 Haziran’da, başarıyla icra edildi. Tatbikatın kara safhası KKTC topraklarında; deniz safhası ise Gazimağusa açıklarında, Türk Arama Kurtarma Bölgesi’nde ve KKTC kara sularında gerçekleştirildi....</p>", "ozel-haberler", "dergi-icerigi", "images/msi/turkiye-ve-kktc-dogu-akdenizdeki-arama-kurtarma-operasyonl.webp", "MSI Dergisi Yayın Kurulu", "2025-07-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["turkiye-ve-kktc-dogu-akdenizdeki-arama-kurtarma-operasyonl"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "ozel-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "etkinlikler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 278, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 21595, 17707)",
        params!["turk-savunma-ve-havacilik-sanayisinin-ihracati-daha-da-art", "Türk Savunma ve Havacılık Sanayisinin İhracatı Daha da Arttırılacak, Seri Üretim Altyapısı Daha da Güçlendirilecek", "Türk Savunma ve Havacılık Sanayisinin İhracatı Daha da Arttırılacak, Seri Üretim Altyapısı Daha da Güçlendirilecek", "<p style=\"text-align: justify\"><strong>Türk Savunma ve Havacılık Sanayisinin İhracatı Daha da Arttırılacak, Seri Üretim Altyapısı Daha da Güçlendirilecek</strong></p><p style=\"text-align: justify\">Türk savunma ve havacılık sanayisinin 2024 yılı karnesi ve 2025 yılındaki rotası, 14 Ocak’ta, Savunma Sanayii Başkanlığında (SSB) düzenlenen basın toplantısında ele alındı. Geçtiğimiz yılın değerlendirildiği ve 2025 yılına dair hedeflerin anlatıldığı toplantıda, Savunma Sanayii Başkanı Prof. Dr. Haluk Görgün, sektörle ilgili birçok konuya değindi. Prof. Dr. Görgün’ün değerlendirmelerinde öne çıkan konuların başında, sektörün ihracat karnesi yer aldı. Kritik teknolojilerde yerlilik oranı giderek artan sektörün, seri üretim altyapısının güçlendirilmesine yönelik faaliyetler ve NATO projelerinde daha fazla yer alınması da öne çıkan diğer konular arasındaydı.</p><p style=\"text-align: justify\">İhracat yapabilmek için teknolojik üstünlük, güven ve maliyet konularının önemine dikkat çeken Prof. Dr. Görgün, sektörün güncel durumuna ve ihracat performansına ilişkin şunları söyledi: <em>“Savunma ve havacılık sanayisinde faaliyet gösteren 3.500’den fazla firmamız, 1.100’ün üzerinde aktif projemiz, 3 milyar dolara yaklaşan AR-GE bütçemiz, yüzde 80’lerin üzerinde yerlilik oranımız ve 100 milyar doları aşan proje hacmimizle sektörümüzde büyük bir başarı elde ettik. Bir önceki seneye göre, yüzde 29’luk bir artışla 7 milyar 154 milyon dolarlık bir ihracat gerçekleştirdik.”</em>...</p>", "basyazi", "dergi-icerigi", "images/msi/turk-savunma-ve-havacilik-sanayisinin-ihracati-daha-da-art.webp", "MSI Dergisi Yayın Kurulu", "2025-02-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["turk-savunma-ve-havacilik-sanayisinin-ihracati-daha-da-art"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "basyazi"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 21840, 17908)",
        params!["saha-2026-ozel-sayilari-savunma-sanayii-baskanligi-mesaji", "SAHA 2026 ÖZEL SAYILARI / Savunma Sanayii Başkanlığı Mesajı", "Prof. Dr. Haluk GÖRGÜN: “Savunma Sanayii Başkanlığı olarak SAHA 2026’yı; ihracat hedeflerimize ivme kazandıran, ortak üretim ve ortak geliştirme fırsatlarını büyüten, teknoloji iş birliklerini derinleştiren ve ekosistemimizi küresel ölçekte daha rekabetçi kılan stratejik bir zemin olarak değerlendiriyoruz.”", "<p>Türkiye, savunma sanayiinde ortaya koyduğu büyük atılımı; güçlü mühendislik kabiliyeti, yüksek teknoloji odaklı AR-GE yaklaşımı ve sahaya yansıyan ürün başarısıyla her geçen gün daha ileriye taşımaktadır. Bu başarı; ana yüklenicilerimizden KOBİ’lerimize, üniversitelerimizden araştırma altyapılarımıza, girişimlerimizden tedarik zincirimizin her halkasına kadar geniş bir ekosistemin ortak emeğiyle büyümektedir. Savunma Sanayii Başkanlığı olarak bizler de bu ekosistemi daha rekabetçi, daha dayanıklı ve daha sürdürülebilir kılacak her adımı stratejik bir öncelik olarak görüyor; yerli ve millî üretim vizyonumuzu her geçen gün kararlılıkla güçlendiriyoruz.</p><p>Bu kapsamda kümelenmeler, yalnızca üretimi büyüten yapılar olmanın ötesinde; iş birliğini, birlikte geliştirmeyi, tedarik ağlarını ve uluslararası açılımı hızlandıran güçlü platformlar olarak öne çıkmaktadır. SAHA İstanbul Savunma, Havacılık ve Uzay Kümelenmesi’nin sektörümüze kazandırdığı dinamizm; paydaşlarımızın ortak hedefler etrafında buluşmasına, küresel ölçekte daha görünür ve daha etkili bir şekilde konumlanmasına önemli katkılar sunmaktadır.</p><p>İşte bu yaklaşımın en güçlü şekilde sahne aldığı buluşmalardan biri olan SAHA 2026 Uluslararası Savunma, Havacılık ve Uzay Sanayi Fuarı, 5–9 Mayıs 2026 tarihlerinde, İstanbul Fuar Merkezi ev sahipliğinde gerçekleştirilecek....</p>", "mesajlar", "dergi-icerigi", "images/msi/saha-2026-ozel-sayilari-savunma-sanayii-baskanligi-mesaji.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-savunma-sanayii-baskanligi-mesaji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "mesajlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 22085, 18109)",
        params!["saha-2026-ozel-sayilari-saha-istanbul-mesaji", "SAHA 2026 ÖZEL SAYILARI / SAHA İstanbul Mesajı", "SAHA İstanbul Yönetim Kurulu Başkanı Haluk BAYRAKTAR: “SAHA 2026, Türkiye’nin savunma ve havacılıkta ortaya koyduğu iradenin küresel vitrinidir.”", "<p></p><p></p><p>Küresel güvenlik mimarisi, artık yalnızca askerî güç dengeleriyle tanımlanmıyor. Bugün belirleyici olan hususlar; teknolojik egemenlik, üretim çevikliği, tedarik zinciri dayanıklılığı, veri üstünlüğü ve stratejik iş birliği kurabilme kapasitesidir. Savunma ve havacılık sanayisi, klasik platform yaklaşımının ötesine geçmiş; yapay zekâ, otonom sistemler, uzay teknolojileri, siber güvenlik ve çift kullanımlı çözümlerle yeniden şekillenmiştir. Türkiye, bu yeni dönemde yalnızca izleyen değil geliştiren, üreten, ihraç eden ve yön veren ülkeler arasında daha güçlü bir konuma yükselmektedir....</p>", "mesajlar", "dergi-icerigi", "images/msi/saha-2026-ozel-sayilari-saha-istanbul-mesaji.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-saha-istanbul-mesaji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "mesajlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "itki-tahrik-ve-guc-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 22330, 18310)",
        params!["saha-2026-ozel-sayilari-sasad-mesaji", "SAHA 2026 ÖZEL SAYILARI / SASAD Mesajı", "SASAD Yönetim Kurulu Başkanı Osman OKYAY: \"Türk Savunma ve Havacılık Sanayiinde Dönüşümün Yeni Evresi\"", "<p></p><p>Küresel güvenlik mimarisinin yeniden şekillendiği bir dönemde, savunma ve havacılık sanayii, artık yalnızca askeri kapasiteyle sınırlı bir alan değil. Ülkelerin üretim gücünü, teknoloji geliştirme kabiliyetini ve stratejik dayanıklılığını doğrudan yansıtan bir alana dönüşmüş durumda. Türkiye’nin son yıllarda izlediği yol da bu dönüşümün bir parçası.</p><p>2025 yılında, Türk savunma ve havacılık sanayii, ihracatta 10 milyar dolar eşiğini aştı. Bu bir büyüme göstergesi olduğu kadar, aynı zamanda sektörün küresel sistem içindeki yerinin değiştiğini gösteren net bir işaret. Bugün geldiğimiz noktada, daha entegre çalışan, daha hızlı ölçeklenebilen ve stratejik olarak daha uyumlu bir yapıdan bahsediyoruz....</p>", "mesajlar", "dergi-icerigi", "images/msi/saha-2026-ozel-sayilari-sasad-mesaji.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-sasad-mesaji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "mesajlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 283, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 22575, 18511)",
        params!["boran-vii-mesaj-jandarma-genel-komutani-orgeneral-ali-card", "BORAN VII Mesaj | Jandarma Genel Komutanı Orgeneral Ali ÇARDAKCI", "\"Bir keskin nişancı, yalnızca nişan almaz; aynı zamanda bulunduğu coğrafyada, milletinin vakarını ve devletinin kararlılığını temsil eder.\"", "<p>Kıymetli Okuyucular,</p><p>Günümüzün güvenlik ortamı, artık klasik çatışma tanımlarının çok ötesine geçmiştir.<strong> Hibrit savaş</strong> adı verilen yeni anlayış; <strong>konvansiyonel kuvvetlerle gayrinizami unsurların, siber saldırılarla psikolojik harekâtların, terör eylemleriyle organize suç faaliyetlerinin </strong>aynı anda yürütüldüğü çok katmanlı bir mücadele alanı oluşturmuştur. Bu tablo, tehditlerin yalnızca cephe hattında değil; <strong>şehirlerde, sınır bölgelerinde </strong>ve hatta <strong>sanal dünyada</strong> şekillendiğini göstermektedir.</p><p>Bu ortamda, değişen güvenlik kavramı, fiziki güçle üstünlük sağlamanın yanı sıra;<strong> hassasiyet, doğruluk, sürat </strong>ve <strong>ölçülü güç</strong> kullanımı ile kamu düzenini korumayı zorunlu kılmaktadır....</p>", "mesajlar", "dergi-icerigi", "images/msi/boran-vii-mesaj-jandarma-genel-komutani-orgeneral-ali-card.webp", "MSI Dergisi Yayın Kurulu", "2025-10-09", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["boran-vii-mesaj-jandarma-genel-komutani-orgeneral-ali-card"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "mesajlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 282, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 22820, 18712)",
        params!["boran-vii-mesaj-javdes-sirketler-toplulugu-genel-muduru-uf", "BORAN VII Mesaj |Javdes Şirketler Topluluğu Genel Müdürü Ufuk SOYLUOĞLU", "BORAN VII Mesaj |Javdes Şirketler Topluluğu Genel Müdürü Ufuk SOYLUOĞLU", "<p>Askerî literatürde keskin nişancılık, yalnızca yüksek isabet oranı değil; meskûn mahalde sivillerin emniyetini önceleyen; çevresel hasarı en aza indiren; disiplin, sükûnet ve akıl yürütme gerektiren bir kabiliyet olarak tanımlanır. BORAN Uluslararası Keskin Nişancı Yarışması’nı (BORAN), bu tanımın sahadaki en canlı karşılığı; taktik doktrin ile ileri teknolojinin aynı zeminde buluştuğu bir “kabiliyet ekosistemi” olarak kurguladık. 6-13 Ekim tarihlerinde, Foça Jandarma Komando Eğitim Komutanlığının ev sahipliğinde gerçekleştirilen BORAN VII; bu yaklaşımın olgunlaştığı, küresel ölçekte tanınırlığı hedefleyen yeni bir eşiği temsil ediyor.</p><p>BORAN’ı, “sadece bir yarışma” ya da “klasik bir fuar” kalıplarına sığdırmıyoruz. Çünkü burada, her kademeden son kullanıcı (nişancı, gözcü, takım ve birlik komutanı), sahada kullandığı tüfekten optiğe, kamuflajdan mühimmata kadar çözümlere doğrudan dokunuyor. Firmalar ise ürünlerini, gerçek görev profillerinde test ettirip anlık geri bildirim alıyor. Bu karşılaşma; karar vericileri, sahadaki profesyonelleri ve çözüm sağlayıcıları, aynı masada buluşturuyor. Bu doğrultuda BORAN’ın ayırt edici değeri, bir “deneyim ve öğrenme platformu” oluşturmasında yatıyor.</p><p>Bu yıl hedefimiz, çıtayı daha da yükseltmek: Türkiye dâhil 23 ülkeden yarışmaya katılan unsurları ağırlamaya hazırlanıyoruz. Adalet ve şeffaflık, operasyonel kültürümüzün temelini oluşturuyor. Çok açılı kamera sistemleriyle her atışı kayıt altına alıyor, itiraz süreçlerini anında işletiyoruz. Stres baskısı altında doğru karar verebilen, süre yönetimi ve görev disiplininde üstünlük sağlayan unsurlar, bu yarışmada öne çıkıyor. Böylece BORAN, yalnızca “en iyi atış”ı değil, gerçek görevde belirleyici olan “bütünsel performans”ı ölçüyor....</p>", "mesajlar", "dergi-icerigi", "images/msi/boran-vii-mesaj-javdes-sirketler-toplulugu-genel-muduru-uf.webp", "MSI Dergisi Yayın Kurulu", "2025-10-07", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["boran-vii-mesaj-javdes-sirketler-toplulugu-genel-muduru-uf"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "mesajlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "simulatorler-ve-egitim-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 281, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 23065, 18913)",
        params!["boran-vii-mesaj-foca-jandarma-komando-egitim-komutani-tumg", "BORAN VII Mesaj | Foça Jandarma Komando Eğitim Komutanı Tümgeneral Uğur ERTEKİN", "BORAN VII Mesaj | Foça Jandarma Komando Eğitim Komutanı Tümgeneral Uğur ERTEKİN", "<p>Teröristle mücadele harekâtının başlangıç noktası ve tüm jandarma komandoların kalpgahı olan şanlı yuva Foça Jandarma Komando Eğitim Komutanlığı olarak, geçmişi şan ve şerefle dolu Teşkilatımızın 186 yıllık köklü tecrübelerini geleceğe taşıyan BORAN Uluslararası Keskin Nişancı Yarışması'na bir kez daha ev sahipliği yapmanın gururunu yaşıyoruz.</p><p style=\"text-align: justify;\">BORAN, sadece bir yarışma değil, aynı zamanda dostluğun, tecrübe paylaşımının ve mesleki mükemmelliğin buluştuğu uluslararası bir etkinliktir. Farklı askeri ve kolluk kültürlerinden gelen keskin nişancı unsurlarının tecrübelerini paylaşmasına, birbirlerinden öğrenmesine ve gerek kurumsal aidiyet gerekse dostluk ve kardeşlik duygularını pekiştirmesine zemin oluşturmaktadır.</p><p style=\"text-align: justify;\">Başlangıçta sadece Jandarma Keskin Nişancı Unsurlarının iştirakiyle gerçekleştirilen ve bu sene yedincisini düzenleyeceğimiz BORAN Keskin Nişancı Yarışması, kısa zamanda ulusal bir yarışma olmanın ötesine geçmiştir. Dünyanın dört bir yanından çok sayıda dost ve müttefik ülkenin katılımıyla gerçekleştirilmeye başlanan yarışma, küresel bir boyut kazanmıştır....</p>", "mesajlar", "dergi-icerigi", "images/msi/boran-vii-mesaj-foca-jandarma-komando-egitim-komutani-tumg.webp", "MSI Dergisi Yayın Kurulu", "2025-10-07", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["boran-vii-mesaj-foca-jandarma-komando-egitim-komutani-tumg"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "mesajlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tskdan-haberler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "simulatorler-ve-egitim-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 280, ?9, 5, 0, 0.0, 0, ?10, ?11, 0, NULL, 23310, 19114)",
        params!["sarsilmazdan-safkan-anti-dron-cozumu", "SARSILMAZ’dan Safkan Anti-Dron Çözümü", "SAR Drone Killer ile Dronlara Geçit Yok", "<p><em>Hem operasyon sahasında hem de kritik tesislerin savunmasında giderek daha büyük bir tehdit haline gelen dronlara karşı kullanılan savunma yöntemleri arasında, özel mühimmat ateşleyen tüfekler de yer alıyor. SARSILMAZ’ın 150 yıla yakın üretim tecrübesi ve güvenlik birimlerinden alınan geri bildirimlerle geliştirdiği SAR Drone Killer da safkan bir anti-dron silahı olarak öne çıkıyor.</em> <em>Tüfek, SARSILMAZ Grup şirketlerinden BPS tarafından geliştirilen özel mühimmatla kullanıldığında etkinliğini arttırıyor.  Dron avlama görevinin gerektirdiği teknik özellikler ve kabiliyetlerle donatılan SAR Drone Killer; silahlı kuvvetlerin, kolluk kuvvetlerinin ve kritik altyapıların korunmasında görev yapan güvenlik personelinin tercihi olmayı hedefliyor.</em></p><p> </p><p>Anti-dron görevlerinde kullanılacak bir tüfeğin, görev etkinliğinin yanı sıra askerî standartlara uygun bir tasarıma sahip olması, hızlı reaksiyon gösterebilmesi ve güvenli, hassas kullanım imkânı sunması gerekir. Böyle bir silah, ancak en baştan bu gereksinimler düşünülerek tasarlandığında istenen performansı sunabilir....</p>", "tanitici-makaleler", "dergi-icerigi", "images/msi/sarsilmazdan-safkan-anti-dron-cozumu.webp", "MSI Dergisi Yayın Kurulu", "2026-06-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["sarsilmazdan-safkan-anti-dron-cozumu"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "muhimmatlar-ve-fuze-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "namlulu-silahlar"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 279, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 23555, 19315)",
        params!["nero-endustriden-modern-muharebe-platformlari-icin-entegre", "Nero Endüstri’den Modern Muharebe Platformları için “Entegre Alt Sistemler” Yaklaşımı", "Nero Endüstri’den Modern Muharebe Platformları için “Entegre Alt Sistemler” Yaklaşımı", "<p></p><p><em>Modern muharebe sahasında görev yapan platformların etkinliği; yalnızca ateş gücü, hareket kabiliyeti veya zırh korumasıyla değil, bu unsurları destekleyen alt sistemlerin koordineli ve kesintisiz çalışmasıyla da belirleniyor. Bu noktada devreye, tasarladığı ve ürettiği alt sistemlerle hem yurt içerisinde hem de yurt dışında pek çok platforma alt sistem sağlayan Nero Endüstri giriyor. Firma; personelin ve platformun emniyetle görev yapabilmesi için tehditlere karşı tedbir işlevini gören ve yürütülen görevi destekleyen çözümleri entegre bir mimari içerisinde sunarak platformların beka kabiliyetini, etkinliğini ve operasyonel sürekliliğini arttırmayı hedefliyor.</em></p><p> ...</p>", "tanitici-makaleler", "dergi-icerigi", "images/msi/nero-endustriden-modern-muharebe-platformlari-icin-entegre.webp", "MSI Dergisi Yayın Kurulu", "2026-06-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["nero-endustriden-modern-muharebe-platformlari-icin-entegre"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 278, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 23800, 19516)",
        params!["saha-2026-ozel-sayilari-saglamtek", "SAHA 2026 Özel Sayıları / SAĞLAMTEK", "SAĞLAMTEK’ten Akustik Dron Tespit Sistemi Çözümü: ADTS", "<p><strong>Göremiyorsam Dinlerim!</strong></p><p><em>Sektörde küresel konumlama sistemleri (KKS), antenler, veri bağları ve RF bileşenler alanlarında sunduğu yenilikçi ve özgün çözümlerle tanınan SAĞLAMTEK, ürün ailesini, akustik dron tespit sistemi ADTS ile genişletiyor. Özellikle meskûn mahallerde, diğer algılayıcıların yetersiz kaldığı durumlarda dronları etkili bir şekilde tespit edebilen ADTS, sahada karşılaşılabilecek büyük tehditleri henüz oluşmadan bertaraf ediyor.</em></p><p> ...</p>", "tanitici-makaleler", "dergi-icerigi", "images/msi/saha-2026-ozel-sayilari-saglamtek.webp", "MSI Dergisi Yayın Kurulu", "2026-05-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-saglamtek"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 286, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 24045, 19716)",
        params!["turk-hava-kuvvetleri-komutanliginda-pilot-egitimi", "Türk Hava Kuvvetleri Komutanlığında Pilot Eğitimi", "Türk Hava Kuvvetleri Komutanlığında Pilot Eğitimi", "<p>Tayfun YAŞAR / tayfun@removebeforephotos.com</p><p>Onur KURÇ / onur@removebeforephotos.com</p><p>...</p>", "makaleler-analizler", "dergi-icerigi", "images/msi/turk-hava-kuvvetleri-komutanliginda-pilot-egitimi.webp", "MSI Dergisi Yayın Kurulu", "2025-12-01", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["turk-hava-kuvvetleri-komutanliginda-pilot-egitimi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "makaleler-analizler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "haberlesme-ve-muharebe-destek-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "simulatorler-ve-egitim-sistemleri"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 285, ?9, 4, 0, 0.0, 0, ?10, ?11, 0, NULL, 24290, 19917)",
        params!["igef-2025-ozel-sayisi-analiz", "İGEF 2025 ÖZEL SAYISI / ANALİZ", "Türkiye'nin İç Güvenlik Alanındaki Yetenekleri: Harekât Taktik ve Tekniklerinin yanı sıra Teknolojide İnovasyon ve Türk Savunma ve Havacılık Sanayisinin Çözümleri", "<p></p><p>Terör ve organize suçtan yasa dışı göçe ve asimetrik savaşa kadar uzanan iç güvenlik tehditleri, dijital çağda hızla gelişmektedir. Bu ortamda Türkiye, saha deneyimi ve sürekli teknolojik inovasyon üzerine inşa edilmiş, dünyanın en kapsamlı ve uyarlanabilir iç güvenlik sistemlerinden birisini geliştirmiştir. Burada Türkiye’nin, iç güvenlik alanında, sadece teknolojik alanda değil harekât taktik ve teknikleri alanında da sürekli inovasyon yaptığına dikkat edilmelidir. Bu çerçevede Türkiye’nin iç güvenlik mimarisi, iki ana konuda özetlenebilir. Bunlar saha tecrübesi ışığında operasyonel olarak geliştirilen harekât taktikleri ve teknikleri ve bunlarla uyumlu olan geliştirilen iç güvenlik odaklı savunma sistemleri ve ekipmanıdır.</p><p style=\"text-align: justify;\"> Korg. (E) Dr. Yavuz TÜRKGENCİ...</p>", "makaleler-analizler", "dergi-icerigi", "images/msi/igef-2025-ozel-sayisi-analiz.webp", "MSI Dergisi Yayın Kurulu", "2025-09-19", "", ""],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["igef-2025-ozel-sayisi-analiz"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "makaleler-analizler"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "dergi-icerikleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "yazarlar"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "elektronik-harp-sistemleri"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hizmetler-kurumsal-iletisim"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi Editöryal Masası', 284, ?9, 5, 0, 0.0, 1, ?10, ?11, 0, NULL, 24535, 20118)",
        params!["gspt-u-tipi-pan-tilt-havada-karada-ve-denizde-gorev-yapmay", "GSPT U Tipi Pan-Tilt; Havada, Karada ve Denizde Görev Yapmaya Hazır", "Gürbağ Savunma ve Teknoloji, GSPT U Tipi Pan-Tilt sistemini; hava, kara ve deniz platformlarında, yüksek performansla ve güvenilir bir şekilde görev yapabilmesi için geliştirdi.", "<p style=\"text-align: justify\"><em>Gürbağ Savunma ve Teknoloji, GSPT U Tipi Pan-Tilt sistemini; hava, kara ve deniz platformlarında, yüksek performansla ve güvenilir bir şekilde görev yapabilmesi için geliştirdi. Firmanın geniş pan-tilt ürün ailesinin üyesi olan sistem, yüksek ağırlıklardaki</em> <em>faydalı yükleri kontrol edebiliyor ve yönlendirebiliyor. Zorlu çevresel şartlarda görev yapabilecek şekilde tasarlanan ve üretilen GSPT U Tipi Pan-Tilt sistemi, faydalı yükleri, yüksek hassasiyetle ve hızlı bir şekilde konumlandırabiliyor.</em></p><p style=\"text-align: justify\">Gürbağ Savunma ve Teknoloji, pan-tilt sistemleri alanındaki birikimi, teknolojisi ve mühendislik gücüyle geliştirdiği GSPT U Tipi Pan-Tilt’i, kontrol ve yönlendirme sistemlerinde fark yaratan bir çözüm olarak kullanıma sunuyor. Stabilizasyon kabiliyetine sahip olan sistem, yüksek ağırlıklardaki faydalı yükleri kontrol edebiliyor ve yönlendirebiliyor. Bu faydalı yükler arasında; elektro-optik sensörler ve lazer mesafe ölçerler gibi hassasiyetin önemli olduğu, karmaşık arayüzler üzerinden kontrol edilen sistemler de yer alıyor. GSPT U Tipi Pan-Tilt, tüm bu faydalı yüklerle tam uyumlu ve entegre şekilde çalışabiliyor.</p><p style=\"text-align: justify\"><strong>Her Koşulda, Her Zaman Görevde</strong>...</p>", "advertorial", "dergi-icerigi", "images/msi/gspt-u-tipi-pan-tilt-havada-karada-ve-denizde-gorev-yapmay.webp", "MSI Dergisi Yayın Kurulu", "2025-02-01", "KURUMSAL SPONSOR", "ADVERTORIAL • TANITIM"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["gspt-u-tipi-pan-tilt-havada-karada-ve-denizde-gorev-yapmay"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "advertorial"])?;
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "tanitici-makaleler"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "hava-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "deniz-araclari"])?;
        conn.execute("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)", params![cid, "kara-araclari"])?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO magazine_issues (
            issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
            year, cover_image, is_special_issue, event_name, page_count, dossier_count,
            summary_tr, summary_en, pdf_path, spreads_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, NULL, 120, 14, ?9, ?9, ?10, '[]')",
        params![286, "sayi-286-kasım-2026", "MSI Dergisi • Sayı 286 (Kasım 2026)", "MSI Magazine • Issue 286 (November 2026)", "Kasım", "November", 2026, "images/msi/dergi-286.webp", "MSI Dergisi'nin yeni sayısının içerikleri: KAPAK DOSYASI / ARES Tersanesi ...", "/api/v1/edergi/286/download-pdf"],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO magazine_issues (
            issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
            year, cover_image, is_special_issue, event_name, page_count, dossier_count,
            summary_tr, summary_en, pdf_path, spreads_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, NULL, 120, 14, ?9, ?9, ?10, '[]')",
        params![285, "sayi-285-ekim-2026", "MSI Dergisi • Sayı 285 (Ekim 2026)", "MSI Magazine • Issue 285 (October 2026)", "Ekim", "October", 2026, "images/msi/dergi-285.webp", "MSI Dergisi'nin yeni sayısının içerikleri: KAPAK DOSYASI / HAN Sistem Teknoloji Mühendislik HAN Yönetim Kurulu Başkanı Melih Han BİLGİN: “HAN’ın görev tanımını; yüksek teknolojiye sahip savunma sistemlerini çok hızlı ve maliyet etkin bir şekilde üreterek dünya", "/api/v1/edergi/285/download-pdf"],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO magazine_issues (
            issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
            year, cover_image, is_special_issue, event_name, page_count, dossier_count,
            summary_tr, summary_en, pdf_path, spreads_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, NULL, 120, 14, ?9, ?9, ?10, '[]')",
        params![284, "sayi-284-eylül-2026", "MSI Dergisi • Sayı 284 (Eylül 2026)", "MSI Magazine • Issue 284 (September 2026)", "Eylül", "September", 2026, "images/msi/dergi-284.webp", "MSI Dergisi'nin yeni sayısının içerikleri: KAPAK DOSYASI / DataBoss DataBoss Genel Müdürü Prof. Dr. S. Serdar Kozat: “Artık uzaktan kontrol edilebilen platformları, silah sistemlerini, cihazları ve sensörleri, insan operatörler yerine yapay zekâ yazılımlarının", "/api/v1/edergi/284/download-pdf"],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO magazine_issues (
            issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
            year, cover_image, is_special_issue, event_name, page_count, dossier_count,
            summary_tr, summary_en, pdf_path, spreads_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, NULL, 120, 14, ?9, ?9, ?10, '[]')",
        params![279, "sayi-279-temmuz-2026", "MSI Dergisi • Sayı 279 (Temmuz 2026)", "MSI Magazine • Issue 279 (July 2026)", "Temmuz", "July", 2026, "images/msi/dergi-279.webp", "MSI Dergisi’nin 279’uncu Sayısının Kapak Dosyası; Global bir teknoloji şirketine dönüşen SARSILMAZ’ın odağı net: “Sahada kendini kanıtlayan, bugünün ihtiyaçlarına çözüm üreten ve yarının operasyonel üstünlüğünü bugünden şekillendiren sistemler geliştirmek.” MS", "/api/v1/edergi/279/download-pdf"],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO magazine_issues (
            issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
            year, cover_image, is_special_issue, event_name, page_count, dossier_count,
            summary_tr, summary_en, pdf_path, spreads_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, NULL, 120, 14, ?9, ?9, ?10, '[]')",
        params![278, "sayi-278-haziran-2026", "MSI Dergisi • Sayı 278 (Haziran 2026)", "MSI Magazine • Issue 278 (June 2026)", "Haziran", "June", 2026, "images/msi/dergi-278.webp", "MSI Dergisi’nin 278. Sayısının Kapak Dosyası; ● Ender ARGE Genel Müdürü Mustafa Mutlu ÇOBAN: “Ender ARGE’yi, uzmanlık alanları doğrultusunda sistem ve alt sistemler geliştiren; büyük projelerde bu sistemleri ana yüklenicilerin altında sağlayan; bağımsız ürünle", "/api/v1/edergi/278/download-pdf"],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO magazine_issues (
            issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
            year, cover_image, is_special_issue, event_name, page_count, dossier_count,
            summary_tr, summary_en, pdf_path, spreads_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, NULL, 120, 14, ?9, ?9, ?10, '[]')",
        params![277, "sayi-277-mayıs-2026", "MSI Dergisi • Sayı 277 (Mayıs 2026)", "MSI Magazine • Issue 277 (May 2026)", "Mayıs", "May", 2026, "images/msi/dergi-277.webp", "MSI Dergisi’nin 277. Sayısının Kapak Dosyası; ● Vizyondan Gerçeğe: FMK Havacılık, Türkiye’nin Özgün Havacılık Platformlarının Kritik Tedarikçisi Oluyor MSI Dergisi’nin 277. Sayısının Başyazısı’nda; ...", "/api/v1/edergi/277/download-pdf"],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO magazine_issues (
            issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
            year, cover_image, is_special_issue, event_name, page_count, dossier_count,
            summary_tr, summary_en, pdf_path, spreads_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, NULL, 120, 14, ?9, ?9, ?10, '[]')",
        params![276, "sayi-276-nisan-2026", "MSI Dergisi • Sayı 276 (Nisan 2026)", "MSI Magazine • Issue 276 (April 2026)", "Nisan", "April", 2026, "images/msi/dergi-276.webp", "MSI Dergisi’nin 276. Sayısının Kapak Dosyası; ● Aeros Yönetim Kurulu Başkanı Serdar ÇORA: \"Kaydettiğimiz kurumsal gelişim ve başarıyla tamamladığımız sivil ve askerî projeler sayesinde, hava platformları için sistem entegratörü olma hedefimizin ilk aşamasını b", "/api/v1/edergi/276/download-pdf"],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO magazine_issues (
            issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
            year, cover_image, is_special_issue, event_name, page_count, dossier_count,
            summary_tr, summary_en, pdf_path, spreads_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, NULL, 120, 14, ?9, ?9, ?10, '[]')",
        params![275, "sayi-275-mart-2026", "MSI Dergisi • Sayı 275 (Mart 2026)", "MSI Magazine • Issue 275 (March 2026)", "Mart", "March", 2026, "images/msi/dergi-275.webp", "MSI Dergisi’nin 275. Sayısının Kapak Dosyası; ● Barok Savunma Kurucusu Senem Yıldız ÖZKILINÇ: “Barok Savunma, askeri araçların sensör, aydınlatma, haberleşme ve görev destek elektroniklerini uçtan uca tasarlayarak kara ve deniz platformlarında bütüncül sistem ", "/api/v1/edergi/275/download-pdf"],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 286, ?9, 5, 0, 0.0, 0, '', '', 0, NULL, 6000, 4920)",
        params!["kalder-ankara-29-kalite-cemberleri-paylasim-konferansinda-sektorleri-bulusturacak", "KalDer Ankara, 29. Kalite Çemberleri Paylaşım Konferansı'nda Sektörleri Buluşturacak", "MSI Dergisi savunma sanayii haberi.", "<p>KalDer Ankara, 29. Kalite Çemberleri Paylaşım Konferansı'nda Sektörleri Buluşturacak</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-0ebd1e68.webp", "Haber Merkezi", "2026-11-09"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["kalder-ankara-29-kalite-cemberleri-paylasim-konferansinda-sektorleri-bulusturacak"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 285, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 6245, 5120)",
        params!["ilk-hurkus-iiler-ciglide", "İlk HÜRKUŞ-II'ler Çiğli'de", "MSI Dergisi savunma sanayii haberi.", "<p>İlk HÜRKUŞ-II'ler Çiğli'de</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-8f38f602.webp", "Haber Merkezi", "2026-11-08"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["ilk-hurkus-iiler-ciglide"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 284, ?9, 5, 0, 0.0, 0, '', '', 0, NULL, 6490, 5321)",
        params!["mkenin-tolga-sistemi-deniz-hedeflerine-karsi-etkinligini-basarili-atislarla-gosterdi", "MKE'nin TOLGA Sistemi, Deniz Hedeflerine Karşı Etkinliğini Başarılı Atışlarla Gösterdi", "MSI Dergisi savunma sanayii haberi.", "<p>MKE'nin TOLGA Sistemi, Deniz Hedeflerine Karşı Etkinliğini Başarılı Atışlarla Gösterdi</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-36caedbb.webp", "Haber Merkezi", "2026-11-07"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["mkenin-tolga-sistemi-deniz-hedeflerine-karsi-etkinligini-basarili-atislarla-gosterdi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 283, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 6735, 5522)",
        params!["msi-dergisinin-286nci-sayisi-yayimlandi", "MSI Dergisi'nin 286'ncı Sayısı Yayımlandı!", "MSI Dergisi savunma sanayii haberi.", "<p>MSI Dergisi'nin 286'ncı Sayısı Yayımlandı!</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-8e169368.webp", "Haber Merkezi", "2026-10-24"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["msi-dergisinin-286nci-sayisi-yayimlandi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 282, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 6980, 5723)",
        params!["han-yonetim-kurulu-baskani-melih-han-bilgin", "HAN Yönetim Kurulu Başkanı Melih Han BİLGİN:", "MSI Dergisi savunma sanayii haberi.", "<p>HAN Yönetim Kurulu Başkanı Melih Han BİLGİN:</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-han-yonetim-kurulu-baskani-melih-han-bilgin.webp", "Haber Merkezi", "2026-10-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["han-yonetim-kurulu-baskani-melih-han-bilgin"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 281, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 7225, 5924)",
        params!["databoss-genel-muduru-prof-dr-s-serdar-kozat", "DataBoss Genel Müdürü Prof. Dr. S. Serdar KOZAT:", "MSI Dergisi savunma sanayii haberi.", "<p>DataBoss Genel Müdürü Prof. Dr. S. Serdar KOZAT:</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-databoss-genel-muduru-prof-dr-s-serdar-kozat.webp", "Haber Merkezi", "2026-09-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["databoss-genel-muduru-prof-dr-s-serdar-kozat"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 280, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 7470, 6125)",
        params!["ctech-genel-muduru-dr-cuneyd-firat-f18da8ea", "CTech Genel Müdürü Dr. Cüneyd Fırat:", "MSI Dergisi savunma sanayii haberi.", "<p>CTech Genel Müdürü Dr. Cüneyd Fırat:</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-ctech-genel-muduru-dr-cuneyd-firat-f18da8ea.webp", "Haber Merkezi", "2026-08-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["ctech-genel-muduru-dr-cuneyd-firat-f18da8ea"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 279, ?9, 4, 0, 0.0, 0, '', '', 0, NULL, 7715, 6326)",
        params!["global-bir-teknoloji-sirketine-donusen-sarsilmazin-odagi-net", "Global bir teknoloji şirketine dönüşen SARSILMAZ’ın odağı net:", "MSI Dergisi savunma sanayii haberi.", "<p>Global bir teknoloji şirketine dönüşen SARSILMAZ’ın odağı net:</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-global-bir-teknoloji-sirketine-donusen-sarsilmazin.webp", "Haber Merkezi", "2026-07-30"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["global-bir-teknoloji-sirketine-donusen-sarsilmazin-odagi-net"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 278, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 7960, 6527)",
        params!["ender-arge-genel-muduru-mustafa-mutlu-coban", "Ender ARGE Genel Müdürü Mustafa Mutlu ÇOBAN:", "MSI Dergisi savunma sanayii haberi.", "<p>Ender ARGE Genel Müdürü Mustafa Mutlu ÇOBAN:</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-f2545f43.webp", "Haber Merkezi", "2026-07-08"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["ender-arge-genel-muduru-mustafa-mutlu-coban"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 277, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 8205, 6728)",
        params!["ges-muhendislik-yonetim-kurulu-baskani-arzu-baytar-138fd5c4", "GES Mühendislik Yönetim Kurulu Başkanı Arzu BAYTAR:", "MSI Dergisi savunma sanayii haberi.", "<p>GES Mühendislik Yönetim Kurulu Başkanı Arzu BAYTAR:</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-ges-muhendislik-yonetim-kurulu-baskani-arzu-baytar.webp", "Haber Merkezi", "2026-10-02"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["ges-muhendislik-yonetim-kurulu-baskani-arzu-baytar-138fd5c4"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 8450, 6929)",
        params!["bps-yonetim-kurulu-uyesi-yalcin-kayali", "BPS Yönetim Kurulu Üyesi Yalçın KAYALI:", "MSI Dergisi savunma sanayii haberi.", "<p>BPS Yönetim Kurulu Üyesi Yalçın KAYALI:</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-bps-yonetim-kurulu-uyesi-yalcin-kayali.webp", "Haber Merkezi", "2026-10-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["bps-yonetim-kurulu-uyesi-yalcin-kayali"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 8695, 7129)",
        params!["expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan-917bfa71", "EXPOLINK Fuarcılık Yönetim Kurulu Başkanı Zeki ERDOĞAN:", "MSI Dergisi savunma sanayii haberi.", "<p>EXPOLINK Fuarcılık Yönetim Kurulu Başkanı Zeki ERDOĞAN:</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erd.webp", "Haber Merkezi", "2026-10-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan-917bfa71"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 5, 0, 0.0, 0, '', '', 0, NULL, 8940, 7330)",
        params!["aeros-tek-adresten-anahtar-teslim-cozumler-vizyonunu-kalite-sistemiyle-destekliyor", "Aeros, \"Tek Adresten Anahtar Teslim Çözümler\" Vizyonunu Kalite Sistemiyle Destekliyor", "MSI Dergisi savunma sanayii haberi.", "<p>Aeros, \"Tek Adresten Anahtar Teslim Çözümler\" Vizyonunu Kalite Sistemiyle Destekliyor</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-aeros-tek-adresten-anahtar-teslim-cozumler-vizyonu.webp", "Haber Merkezi", "2026-08-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["aeros-tek-adresten-anahtar-teslim-cozumler-vizyonunu-kalite-sistemiyle-destekliyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 5, 0, 0.0, 0, '', '', 0, NULL, 9185, 7531)",
        params!["musiad-uyeleri-msb-onayli-tedarikci-gunu25-etkinliginde-uzmanlarla-bir-araya-geldi", "MÜSİAD Üyeleri, MSB Onaylı Tedarikçi Günü’25 Etkinliğinde, Uzmanlarla Bir Araya Geldi", "MSI Dergisi savunma sanayii haberi.", "<p>MÜSİAD Üyeleri, MSB Onaylı Tedarikçi Günü’25 Etkinliğinde, Uzmanlarla Bir Araya Geldi</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-musiad-uyeleri-msb-onayli-tedarikci-gunu25-etkinli.webp", "Haber Merkezi", "2026-02-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["musiad-uyeleri-msb-onayli-tedarikci-gunu25-etkinliginde-uzmanlarla-bir-araya-geldi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 7, 0, 0.0, 0, '', '', 0, NULL, 9430, 7732)",
        params!["genelkurmay-baskanligina-orgeneral-selcuk-bayraktaroglu-kara-kuvvetleri-komutanligina-orgeneral-metin-tokel-atandi", "Genelkurmay Başkanlığına Orgeneral Selçuk Bayraktaroğlu; Kara Kuvvetleri Komutanlığına Orgeneral Metin Tokel Atandı", "MSI Dergisi savunma sanayii haberi.", "<p>Genelkurmay Başkanlığına Orgeneral Selçuk Bayraktaroğlu; Kara Kuvvetleri Komutanlığına Orgeneral Metin Tokel Atandı</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-genelkurmay-baskanligina-orgeneral-selcuk-bayrakta.webp", "Haber Merkezi", "2025-09-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["genelkurmay-baskanligina-orgeneral-selcuk-bayraktaroglu-kara-kuvvetleri-komutanligina-orgeneral-metin-tokel-atandi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 4, 0, 0.0, 0, '', '', 0, NULL, 9675, 7933)",
        params!["meteksan-savunmadan-uretim-kapasitesini-25-kat-arttiracak-hamle", "Meteksan Savunma’dan Üretim Kapasitesini 2,5 Kat Arttıracak Hamle", "MSI Dergisi savunma sanayii haberi.", "<p>Meteksan Savunma’dan Üretim Kapasitesini 2,5 Kat Arttıracak Hamle</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-meteksan-savunmadan-uretim-kapasitesini-25-kat-art.webp", "Haber Merkezi", "2025-09-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["meteksan-savunmadan-uretim-kapasitesini-25-kat-arttiracak-hamle"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 9920, 8134)",
        params!["teknofest-ruzgari-bu-kez-mavi-vatanda-esti", "TEKNOFEST Rüzgârı, Bu Kez Mavi Vatan’da Esti", "MSI Dergisi savunma sanayii haberi.", "<p>TEKNOFEST Rüzgârı, Bu Kez Mavi Vatan’da Esti</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-teknofest-ruzgari-bu-kez-mavi-vatanda-esti.webp", "Haber Merkezi", "2025-09-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["teknofest-ruzgari-bu-kez-mavi-vatanda-esti"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 7, 0, 0.0, 0, '', '', 0, NULL, 10165, 8335)",
        params!["farkli-malzemelerle-insa-ettigi-platformlarla-farkli-denizlerde-buyuyen-ares-tersanesi-iddiasini-gelecege-tasiyor", "Farklı Malzemelerle İnşa Ettiği Platformlarla Farklı Denizlerde Büyüyen ARES Tersanesi, İddiasını Geleceğe Taşıyor", "MSI Dergisi savunma sanayii haberi.", "<p>Farklı Malzemelerle İnşa Ettiği Platformlarla Farklı Denizlerde Büyüyen ARES Tersanesi, İddiasını Geleceğe Taşıyor</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-0ebd1e68.webp", "Haber Merkezi", "2026-11-09"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["farkli-malzemelerle-insa-ettigi-platformlarla-farkli-denizlerde-buyuyen-ares-tersanesi-iddiasini-gelecege-tasiyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 7, 0, 0.0, 0, '', '', 0, NULL, 10410, 8536)",
        params!["anti-dron-sistemleri-gelistirmeye-odaklanan-han-yenilikci-hizli-ve-maliyet-etkin-cozumler-parolasiyla-ilerliyor", "Anti Dron Sistemleri Geliştirmeye Odaklanan HAN; Yenilikçi, Hızlı ve Maliyet Etkin Çözümler Parolasıyla İlerliyor", "MSI Dergisi savunma sanayii haberi.", "<p>Anti Dron Sistemleri Geliştirmeye Odaklanan HAN; Yenilikçi, Hızlı ve Maliyet Etkin Çözümler Parolasıyla İlerliyor</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-anti-dron-sistemleri-gelistirmeye-odaklanan-han-ye.webp", "Haber Merkezi", "2026-10-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["anti-dron-sistemleri-gelistirmeye-odaklanan-han-yenilikci-hizli-ve-maliyet-etkin-cozumler-parolasiyla-ilerliyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 4, 0, 0.0, 0, '', '', 0, NULL, 10655, 8737)",
        params!["yapay-zeka-teknolojileri-databoss-ile-muharebe-sahasina-iniyor", "Yapay Zekâ Teknolojileri, DataBoss ile Muharebe Sahasına İniyor", "MSI Dergisi savunma sanayii haberi.", "<p>Yapay Zekâ Teknolojileri, DataBoss ile Muharebe Sahasına İniyor</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-b069d7c7.webp", "Haber Merkezi", "2026-09-22"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["yapay-zeka-teknolojileri-databoss-ile-muharebe-sahasina-iniyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 4, 0, 0.0, 0, '', '', 0, NULL, 10900, 8938)",
        params!["menatekte-30-yilin-ozeti-apartman-dairesinden-ihracat-sampiyonluguna", "MENATEK’te 30 Yılın Özeti: Apartman Dairesinden İhracat Şampiyonluğuna", "MSI Dergisi savunma sanayii haberi.", "<p>MENATEK’te 30 Yılın Özeti: Apartman Dairesinden İhracat Şampiyonluğuna</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-menatekte-30-yilin-ozeti-apartman-dairesinden-ihra.webp", "Haber Merkezi", "2025-08-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["menatekte-30-yilin-ozeti-apartman-dairesinden-ihracat-sampiyonluguna"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 5, 0, 0.0, 0, '', '', 0, NULL, 11145, 9138)",
        params!["teslimat-ve-siparis-adetlerini-arttiran-armsto-zirve-yuruyusunu-surduruyor", "Teslimat ve Sipariş Adetlerini Arttıran ARMSTO, Zirve Yürüyüşünü Sürdürüyor", "MSI Dergisi savunma sanayii haberi.", "<p>Teslimat ve Sipariş Adetlerini Arttıran ARMSTO, Zirve Yürüyüşünü Sürdürüyor</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-teslimat-ve-siparis-adetlerini-arttiran-armsto-zir.webp", "Haber Merkezi", "2025-07-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["teslimat-ve-siparis-adetlerini-arttiran-armsto-zirve-yuruyusunu-surduruyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 11390, 9339)",
        params!["saha-2026-ozel-sayilari-icisleri-bakanligi-mesaji", "SAHA 2026 Özel Sayıları / İçişleri Bakanlığı Mesajı", "MSI Dergisi savunma sanayii haberi.", "<p>SAHA 2026 Özel Sayıları / İçişleri Bakanlığı Mesajı</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-594ee6bf.webp", "Haber Merkezi", "2026-05-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-icisleri-bakanligi-mesaji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 11635, 9540)",
        params!["saha-2026-ozel-sayilari-milli-savunma-bakanligi-mesaji", "SAHA 2026 ÖZEL SAYILARI / Millî Savunma Bakanlığı Mesajı", "MSI Dergisi savunma sanayii haberi.", "<p>SAHA 2026 ÖZEL SAYILARI / Millî Savunma Bakanlığı Mesajı</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-594ee6bf.webp", "Haber Merkezi", "2026-05-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-milli-savunma-bakanligi-mesaji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 11880, 9741)",
        params!["saha-2026-ozel-sayilari-ticaret-bakanligi-mesaji", "SAHA 2026 ÖZEL SAYILARI / Ticaret Bakanlığı Mesajı", "MSI Dergisi savunma sanayii haberi.", "<p>SAHA 2026 ÖZEL SAYILARI / Ticaret Bakanlığı Mesajı</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-594ee6bf.webp", "Haber Merkezi", "2026-05-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["saha-2026-ozel-sayilari-ticaret-bakanligi-mesaji"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 5, 0, 0.0, 0, '', '', 0, NULL, 12125, 9942)",
        params!["sarsilmaz-sar-556-mt-ile-dayaniklilik-ve-performansin-yeni-standartlarini-belirliyor", "SARSILMAZ, SAR 556 MT ile Dayanıklılık ve Performansın Yeni Standartlarını Belirliyor", "MSI Dergisi savunma sanayii haberi.", "<p>SARSILMAZ, SAR 556 MT ile Dayanıklılık ve Performansın Yeni Standartlarını Belirliyor</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-sarsilmaz-sar-556-mt-ile-dayaniklilik-ve-performan.webp", "Haber Merkezi", "2026-09-10"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["sarsilmaz-sar-556-mt-ile-dayaniklilik-ve-performansin-yeni-standartlarini-belirliyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 5, 0, 0.0, 0, '', '', 0, NULL, 12370, 10143)",
        params!["teops-tasarim-uretim-ve-test-kabiliyetlerini-yeni-tesisiyle-guclendiriyor", "TEOPS; Tasarım, Üretim ve Test Kabiliyetlerini Yeni Tesisiyle Güçlendiriyor", "MSI Dergisi savunma sanayii haberi.", "<p>TEOPS; Tasarım, Üretim ve Test Kabiliyetlerini Yeni Tesisiyle Güçlendiriyor</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-94bda6dc.webp", "Haber Merkezi", "2026-09-08"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["teops-tasarim-uretim-ve-test-kabiliyetlerini-yeni-tesisiyle-guclendiriyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 12615, 10344)",
        params!["stest-sektoru-bilgilendiriyor-bolum-ii", "STEST Sektörü Bilgilendiriyor: Bölüm II", "MSI Dergisi savunma sanayii haberi.", "<p>STEST Sektörü Bilgilendiriyor: Bölüm II</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-stest-sektoru-bilgilendiriyor-bolum-ii.webp", "Haber Merkezi", "2026-09-07"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["stest-sektoru-bilgilendiriyor-bolum-ii"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 8, 0, 0.0, 0, '', '', 0, NULL, 12860, 10545)",
        params!["nero-endustri-muharebe-sahasindaki-gorunmeyen-tehditleri-twl-200-taktik-giyilebilir-lazer-uyari-sistemi-ile-tespit-edecek", "NERO Endüstri, Muharebe Sahasındaki Görünmeyen Tehditleri, TWL-200 Taktik Giyilebilir Lazer Uyarı Sistemi ile Tespit Edecek", "MSI Dergisi savunma sanayii haberi.", "<p>NERO Endüstri, Muharebe Sahasındaki Görünmeyen Tehditleri, TWL-200 Taktik Giyilebilir Lazer Uyarı Sistemi ile Tespit Edecek</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-nero-endustri-muharebe-sahasindaki-gorunmeyen-tehd.webp", "Haber Merkezi", "2026-09-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["nero-endustri-muharebe-sahasindaki-gorunmeyen-tehditleri-twl-200-taktik-giyilebilir-lazer-uyari-sistemi-ile-tespit-edecek"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 7, 0, 0.0, 0, '', '', 0, NULL, 13105, 10746)",
        params!["nero-grup-ekosistemi-bir-savunma-sisteminin-fikirden-sahaya-uzanan-omur-devrinin-her-asamasinda-goreve-hazir", "NERO Grup Ekosistemi, Bir Savunma Sisteminin Fikirden Sahaya Uzanan Ömür Devrinin Her Aşamasında Göreve Hazır", "MSI Dergisi savunma sanayii haberi.", "<p>NERO Grup Ekosistemi, Bir Savunma Sisteminin Fikirden Sahaya Uzanan Ömür Devrinin Her Aşamasında Göreve Hazır</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-51cb88d1.webp", "Haber Merkezi", "2026-08-06"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["nero-grup-ekosistemi-bir-savunma-sisteminin-fikirden-sahaya-uzanan-omur-devrinin-her-asamasinda-goreve-hazir"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 6, 0, 0.0, 0, '', '', 0, NULL, 13350, 10947)",
        params!["insansiz-hava-araci-veri-baglari-operasyonel-zorluklar-cozum-yontemleri-ve-akson-veri-bagi-sistemi-a2715649", "İnsansız Hava Aracı Veri Bağları: Operasyonel Zorluklar, Çözüm Yöntemleri ve AKSON Veri Bağı Sistemi", "MSI Dergisi savunma sanayii haberi.", "<p>İnsansız Hava Aracı Veri Bağları: Operasyonel Zorluklar, Çözüm Yöntemleri ve AKSON Veri Bağı Sistemi</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-insansiz-hava-araci-veri-baglari-operasyonel-zorlu.webp", "Haber Merkezi", "2026-06-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["insansiz-hava-araci-veri-baglari-operasyonel-zorluklar-cozum-yontemleri-ve-akson-veri-bagi-sistemi-a2715649"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 13595, 11147)",
        params!["milli-denizalti-milden-nasil-olmali", "Milli Denizaltı (MİLDEN) Nasıl Olmalı?", "MSI Dergisi savunma sanayii haberi.", "<p>Milli Denizaltı (MİLDEN) Nasıl Olmalı?</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-5e29955d.webp", "Haber Merkezi", "2026-01-02"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["milli-denizalti-milden-nasil-olmali"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 13840, 11348)",
        params!["kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti-e8ee34c2", "Küresel Savaş Gemisi Pazarında Son Bir Yıl Böyle Geçti:", "MSI Dergisi savunma sanayii haberi.", "<p>Küresel Savaş Gemisi Pazarında Son Bir Yıl Böyle Geçti:</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-5e29955d.webp", "Haber Merkezi", "2026-01-02"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti-e8ee34c2"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 14085, 11549)",
        params!["kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti", "Küresel Savaş Gemisi Pazarında Son Bir Yıl Böyle Geçti:", "MSI Dergisi savunma sanayii haberi.", "<p>Küresel Savaş Gemisi Pazarında Son Bir Yıl Böyle Geçti:</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-5e29955d.webp", "Haber Merkezi", "2026-01-02"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["kuresel-savas-gemisi-pazarinda-son-bir-yil-boyle-gecti"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 14330, 11750)",
        params!["denizden-guc-aktarimi-icin-ucak-gemisi", "Denizden Güç Aktarımı için Uçak Gemisi", "MSI Dergisi savunma sanayii haberi.", "<p>Denizden Güç Aktarımı için Uçak Gemisi</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-5e29955d.webp", "Haber Merkezi", "2026-01-02"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["denizden-guc-aktarimi-icin-ucak-gemisi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 6, 0, 0.0, 0, '', '', 0, NULL, 14575, 11951)",
        params!["kim-teknoloji-bor-kaynagini-yenilikci-cozumlere-donusturerek-kuresel-rekabette-one-cikiyor", "KİM Teknoloji, Bor Kaynağını Yenilikçi Çözümlere Dönüştürerek Küresel Rekabette Öne Çıkıyor", "MSI Dergisi savunma sanayii haberi.", "<p>KİM Teknoloji, Bor Kaynağını Yenilikçi Çözümlere Dönüştürerek Küresel Rekabette Öne Çıkıyor</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-7b60f69d.webp", "Haber Merkezi", "2026-01-14"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["kim-teknoloji-bor-kaynagini-yenilikci-cozumlere-donusturerek-kuresel-rekabette-one-cikiyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 4, 0, 0.0, 0, '', '', 0, NULL, 14820, 12152)",
        params!["gurbag-savunmadan-yuksek-performansli-ve-moduler-hareket-platformu", "Gürbağ Savunma’dan Yüksek Performanslı ve Modüler Hareket Platformu", "MSI Dergisi savunma sanayii haberi.", "<p>Gürbağ Savunma’dan Yüksek Performanslı ve Modüler Hareket Platformu</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-a8dc2f87.webp", "Haber Merkezi", "2026-01-13"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["gurbag-savunmadan-yuksek-performansli-ve-moduler-hareket-platformu"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 6, 0, 0.0, 0, '', '', 0, NULL, 15065, 12353)",
        params!["atesci-20-yili-askin-tecrubesiyle-savunma-sanayisinde-guven-teknoloji-ve-yeniligin-adresi", "Ateşçi: 20 Yılı Aşkın Tecrübesiyle Savunma Sanayisinde Güven, Teknoloji ve Yeniliğin Adresi", "MSI Dergisi savunma sanayii haberi.", "<p>Ateşçi: 20 Yılı Aşkın Tecrübesiyle Savunma Sanayisinde Güven, Teknoloji ve Yeniliğin Adresi</p>", "haberler", "gunluk-haber", "/images/msi-harvested/msi-savunma-sanayii-accccf5f.webp", "Haber Merkezi", "2025-12-17"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["atesci-20-yili-askin-tecrubesiyle-savunma-sanayisinde-guven-teknoloji-ve-yeniligin-adresi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 7, 0, 0.0, 0, '', '', 0, NULL, 15310, 12554)",
        params!["sarsilmaz-4-kitada-uluslararasi-organizasyonlarda-turkiyeyi-temsil-ederek-kuresel-rekabette-gucune-guc-katti", "SARSILMAZ, 4 Kıtada Uluslararası Organizasyonlarda Türkiye’yi Temsil Ederek Küresel Rekabette Gücüne Güç Kattı", "MSI Dergisi savunma sanayii haberi.", "<p>SARSILMAZ, 4 Kıtada Uluslararası Organizasyonlarda Türkiye’yi Temsil Ederek Küresel Rekabette Gücüne Güç Kattı</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-sarsilmaz-4-kitada-uluslararasi-organizasyonlarda-.webp", "Haber Merkezi", "2025-12-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["sarsilmaz-4-kitada-uluslararasi-organizasyonlarda-turkiyeyi-temsil-ederek-kuresel-rekabette-gucune-guc-katti"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 15555, 12755)",
        params!["stm-35inci-yila-guclu-adimlarla-giriyor", "STM, 35’inci Yıla Güçlü Adımlarla Giriyor", "MSI Dergisi savunma sanayii haberi.", "<p>STM, 35’inci Yıla Güçlü Adımlarla Giriyor</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-stm-35inci-yila-guclu-adimlarla-giriyor.webp", "Haber Merkezi", "2025-12-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["stm-35inci-yila-guclu-adimlarla-giriyor"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 15800, 12956)",
        params!["igef-2025-kuresel-is-birliklerinin-kapilarini-araladi", "İGEF 2025, Küresel İş Birliklerinin Kapılarını Araladı", "MSI Dergisi savunma sanayii haberi.", "<p>İGEF 2025, Küresel İş Birliklerinin Kapılarını Araladı</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-igef-2025-kuresel-is-birliklerinin-kapilarini-aral.webp", "Haber Merkezi", "2025-10-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["igef-2025-kuresel-is-birliklerinin-kapilarini-araladi"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO contents (
            slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
            content_type, source_group, cover_image, author_name, author_title,
            issue_number, published_at, read_time_min, visibility_level, single_price_try,
            is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
            view_count, true_read_count
        ) VALUES (?1, ?2, ?2, ?3, ?3, ?4, ?4, ?5, ?6, ?7, ?8, 'MSI Dergisi', 276, ?9, 3, 0, 0.0, 0, '', '', 0, NULL, 16045, 13156)",
        params!["expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan-3083a9eb", "EXPOLINK Fuarcılık Yönetim Kurulu Başkanı Zeki ERDOĞAN:", "MSI Dergisi savunma sanayii haberi.", "<p>EXPOLINK Fuarcılık Yönetim Kurulu Başkanı Zeki ERDOĞAN:</p>", "haberler", "gunluk-haber", "images/msi-harvested/msi-savunma-expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erd.webp", "Haber Merkezi", "2025-10-01"],
    )?;
    if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params!["expolink-fuarcilik-yonetim-kurulu-baskani-zeki-erdogan-3083a9eb"], |r| r.get::<_, i64>(0)) {
        conn.execute("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)", params![cid, "haberler"])?;
    }    Ok(())
}
