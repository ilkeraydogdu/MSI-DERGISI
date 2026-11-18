use crate::models::SubscriptionPlan;
use rusqlite::{params, Connection};

pub fn seed_subscription_plans_and_commerce(conn: &Connection) -> rusqlite::Result<()> {
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
