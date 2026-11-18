use crate::models::MagazinePageSpread;
use rusqlite::{params, Connection};

pub fn seed_magazine_issues(conn: &Connection) -> rusqlite::Result<()> {
    let spreads_280 = vec![
        MagazinePageSpread {
            page_no: "S. 01-04".to_string(),
            title_tr: "Kapak & İçindekiler Fihristi — Sayı 280".to_string(),
            title_en: "Cover & Table of Contents — Issue 280".to_string(),
            category_badge: "KAPAK & BAŞYAZI".to_string(),
            image_url: "images/msi/dergi-280.webp".to_string(),
            excerpt_tr: "22 Yıllık Yayıncılık Otoritesi: Başyazı, Savunma Gündemi ve Özel Dosyalar Fihristi.".to_string(),
            excerpt_en: "22 Years of Defense Publishing Authority: Editorial, Defense Briefs and Special Dossier Index.".to_string(),
            linked_content_slug: None,
        },
        MagazinePageSpread {
            page_no: "S. 12-18".to_string(),
            title_tr: "KAAN 5. Nesil Aviyonik, MURAD AESA Radar ve Seri Üretim Mimarisi".to_string(),
            title_en: "KAAN 5th-Gen Avionics, MURAD AESA Radar & Serial Production Architecture".to_string(),
            category_badge: "KAPAK DOSYASI".to_string(),
            image_url: "images/msi/kaan.webp".to_string(),
            excerpt_tr: "BÜRFİS, DAS ve yüksek hızlı fiber optik görev bilgisayarı mimarisinin teknik kesiti.".to_string(),
            excerpt_en: "Technical cutaway of BURFIS, DAS, and high-speed fiber-optic mission computing.".to_string(),
            linked_content_slug: None,
        },
        MagazinePageSpread {
            page_no: "S. 24-32".to_string(),
            title_tr: "CTech Sürü İDA Hibrit Haberleşme & Dr. M. Cüneyd Fırat Röportajı".to_string(),
            title_en: "CTech Swarm USV Hybrid Comms & Dr. M. Cuneyd Firat Executive Interview".to_string(),
            category_badge: "AYIN SÖYLEŞİSİ".to_string(),
            image_url: "images/msi/ctech-roportaj.webp".to_string(),
            excerpt_tr: "Mavi Vatan ve gökyüzünde elektronik harbe dayanıklı Ku/Ka-Bant SATCOM ve Taktik Mesh ağları.".to_string(),
            excerpt_en: "Jam-resistant Ku/Ka-Band SATCOM and Tactical Mesh networks across sea and air domains.".to_string(),
            linked_content_slug: None,
        },
        MagazinePageSpread {
            page_no: "S. 40-48".to_string(),
            title_tr: "AKINCI İHA-300 Atışı, Polonya MSPO ve SEDEC Özel Dosyaları".to_string(),
            title_en: "AKINCI IHA-300 Strike, MSPO Poland & SEDEC Post-Event Dossiers".to_string(),
            category_badge: "ÖZEL DOSYA".to_string(),
            image_url: "images/msi/akinci-iha300.webp".to_string(),
            excerpt_tr: "250 km süpersonik balistik taarruz testi ve uluslararası fuarlarda imzalanan stratejik sözleşmeler.".to_string(),
            excerpt_en: "250 km supersonic ballistic strike trial and strategic defense export contracts.".to_string(),
            linked_content_slug: None,
        },
    ];
    let spreads_json = serde_json::to_string(&spreads_280).unwrap_or_else(|_| "[]".to_string());

    let issues: &[(i32, &str, &str, &str, &str, &str, i32, &str, i32, Option<&str>, i32, i32, &str, &str, &str)] = &[
        (286, "msi-dergisi-sayi-286", "MSI Dergisi • Sayı 286 (Kasım 2026)", "MSI Magazine • Issue 286 (November 2026)", "Kasım", "November", 2026, "images/msi/msi-dergisi-sayi-286-kapak.webp", 0, None, 128, 14, "ARES Tersanesi 20. Yıl Kapak Dosyası, Denizhan-76 ve Savunma Ekosistemi Özel Dosyaları.", "ARES Shipyard 20th Anniversary Cover Story, Denizhan-76 and Defence Ecosystem Special Dossiers.", "/api/v1/edergi/286/download-pdf"),
        (280, "sayi-280-eylul-2026", "MSI Dergisi • Sayı 280 (Eylül 2026)", "MSI Magazine • Issue 280 (September 2026)", "Eylül", "September", 2026, "images/msi/dergi-280.webp", 0, None, 112, 12, "Milli Muharip Uçak KAAN 5. Nesil Aviyonik, CTech Sürü İDA Hibrit Haberleşme ve AKINCI İHA-300 Kapak Dosyası.", "National Combat Aircraft KAAN 5th-Gen Avionics, CTech Swarm USV Comms and AKINCI IHA-300 Cover Story.", "/api/v1/edergi/280/download-pdf"),
        (279, "sayi-279-agustos-2026", "MSI Dergisi • Sayı 279 (Ağustos 2026)", "MSI Magazine • Issue 279 (August 2026)", "Ağustos", "August", 2026, "images/msi/kaan.webp", 0, None, 108, 11, "Çelik Kubbe Katmanlı Hava Savunma Mimarisi, SİPER Ürün-2 ve MİLGEM İSTİF Sınıfı Fırkateynler.", "Steel Dome Multi-Layered Air Defence Architecture, SIPER Block-2 and MILGEM ISTIF-Class Frigates.", "/api/v1/edergi/279/download-pdf"),
        (278, "sayi-278-temmuz-2026", "MSI Dergisi • Sayı 278 (Temmuz 2026)", "MSI Magazine • Issue 278 (July 2026)", "Temmuz", "July", 2026, "images/msi/stm-fuar.webp", 0, None, 104, 10, "Mavi Vatan Deniz Sistemleri, STM500 Denizaltısı ve YALTES Savaş Yönetim Konsolları Dosyası.", "Blue Homeland Naval Systems, STM500 Submarine and YALTES Combat Management Consoles Dossier.", "/api/v1/edergi/278/download-pdf"),
        (277, "sayi-277-haziran-2026", "MSI Dergisi • Sayı 277 (Haziran 2026)", "MSI Magazine • Issue 277 (June 2026)", "Haziran", "June", 2026, "images/msi/mspo-turk.webp", 1, Some("Eurosatory & SEDEC"), 116, 14, "Avrupa Savunma Sanayii İş Birlikleri, ALTAY Ana Muharebe Tankı ve Yeni Nesil Zırhlı Muharebe Araçları.", "European Defence Cooperation, ALTAY Main Battle Tank and Next-Gen Armoured Combat Vehicles.", "/api/v1/edergi/277/download-pdf"),
        (276, "sayi-276-mayis-2026", "MSI Dergisi • Sayı 276 (Mayıs 2026)", "MSI Magazine • Issue 276 (May 2026)", "Mayıs", "May", 2026, "images/msi/skyarx.webp", 0, None, 104, 9, "Elektronik Harp, AESA Radar Teknolojileri ve Taktik Saha Haberleşme Sistemleri Özel İncelemesi.", "Electronic Warfare, AESA Radar Technologies and Tactical Field Communications Special Review.", "/api/v1/edergi/276/download-pdf"),
    ];

    for iss in issues {
        let pdf_file = format!("msi-dergisi-sayi-{}.pdf", iss.0);
        conn.execute(
            "INSERT OR IGNORE INTO magazine_issues (
                publication_type, issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
                year, cover_image, is_special_issue, event_name, page_count, dossier_count,
                summary_tr, summary_en, pdf_path, spreads_json, uploaded_pdf_file, en_status
            ) VALUES ('msi', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, 'none')",
            params![
                iss.0, iss.1, iss.2, iss.3, iss.4, iss.5,
                iss.6, iss.7, iss.8, iss.9, iss.10, iss.11,
                iss.12, iss.13, iss.14, &spreads_json, pdf_file
            ],
        )?;
    }

    Ok(())
}
