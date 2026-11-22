//! Shared i18n, institution taxonomy, and EN content-readiness helpers for public SSR.

use crate::engine::translation_engine::TranslationCache;
use crate::models::ContentItem;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Institutional category choice rendered in registration & reader profile `<select>` menus.
/// Backed by `institution_categories` once Migration 012 lands, with clean built-in defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstitutionChoice {
    pub code: String,
    pub label_tr: String,
    pub label_en: String,
}

/// Canonical 14 official MSI institutional categories used when DB rows are not yet passed.
pub fn default_institution_choices() -> Vec<InstitutionChoice> {
    [
        ("Sektör Kuruluşları", "Sektör Kuruluşları", "Defence Industry Organisations"),
        ("MSB", "MSB (Millî Savunma Bakanlığı)", "MoND (Ministry of National Defence)"),
        ("SSB", "SSB (Savunma Sanayii Başkanlığı)", "SSB (Presidency of Defence Industries)"),
        ("K.K.K.lığı", "K.K.K.lığı (Kara Kuvvetleri)", "Turkish Land Forces Command"),
        ("Dz.K.K.lığı", "Dz.K.K.lığı (Deniz Kuvvetleri)", "Turkish Naval Forces Command"),
        ("Hv.K.K.lığı", "Hv.K.K.lığı (Hava Kuvvetleri)", "Turkish Air Force Command"),
        ("J.G.K.lığı", "J.G.K.lığı (Jandarma Genel K.)", "Gendarmerie General Command"),
        ("EGM", "EGM (Emniyet Genel Müdürlüğü)", "Turkish National Police"),
        ("S.G.K.lığı", "S.G.K.lığı (Sahil Güvenlik K.)", "Turkish Coast Guard Command"),
        ("MİT", "MİT (Millî İstihbarat Teşkilatı)", "National Intelligence Organization"),
        ("Diğer Kamu Kurumları", "Diğer Kamu Kurumları", "Other Public Institutions"),
        ("Üniversite / Öğrenci", "Üniversite / Öğrenci", "University / Student"),
        ("Yurt Dışı Paydaş", "Yurt Dışı Paydaş", "International Stakeholder"),
        ("savunma_medya", "Savunma Medya / Basın", "Defence Media / Press"),
    ]
    .into_iter()
    .map(|(code, tr, en)| InstitutionChoice {
        code: code.to_string(),
        label_tr: tr.to_string(),
        label_en: en.to_string(),
    })
    .collect()
}

/// Loads active `institution_categories` from SQLite (Migration 012), mapping `legacy_value`
/// (when non-empty, else `code`) as the `<option value>` so existing users, B2B analytics,
/// and `auth_service` stay 100% aligned, falling back to `default_institution_choices` only if empty.
pub fn institution_choices_from_db(db: &crate::db::Database) -> Vec<InstitutionChoice> {
    let rows = db.list_institution_categories(true);
    if rows.is_empty() {
        return default_institution_choices();
    }
    rows.into_iter()
        .map(|cat| InstitutionChoice {
            code: if cat.legacy_value.trim().is_empty() {
                cat.code
            } else {
                cat.legacy_value
            },
            label_tr: cat.label_tr,
            label_en: cat.label_en,
        })
        .collect()
}

/// Gate preventing unapproved or TR-copied articles from appearing on `/en` public listings.
#[inline]
pub fn is_en_content_ready(c: &ContentItem) -> bool {
    c.en_status == "approved" || c.source_group == "msi-tdr"
}

/// Looks up `key` in `TranslationCache` for `lang`, checking that the requested language
/// column is actually non-empty before accepting it; otherwise falls back to `fallback_tr` / `fallback_en`.
pub fn tr_text(
    tr: Option<&TranslationCache>,
    lang: &str,
    key: &str,
    fallback_tr: &str,
    fallback_en: &str,
) -> String {
    let is_en = lang.eq_ignore_ascii_case("en");
    let default_val = if is_en { fallback_en } else { fallback_tr };
    if let Some(cache) = tr {
        if let Some(entry) = cache.get_entry(key) {
            let candidate = match lang {
                "tr" => &entry.value_tr,
                "en" => &entry.value_en,
                "ar" => &entry.value_ar,
                "de" => &entry.value_de,
                "ru" => &entry.value_ru,
                "fr" => &entry.value_fr,
                _ => &entry.value_tr,
            };
            if !candidate.trim().is_empty() {
                return candidate.clone();
            }
            if !is_en && !entry.value_tr.trim().is_empty() {
                return entry.value_tr.clone();
            }
            if is_en && !entry.value_en.trim().is_empty() {
                return entry.value_en.clone();
            }
        }
    }
    default_val.to_string()
}

/// Appends `?lang=en` (or maps `/` -> `/en`, `/arama` -> `/en/search`) when `lang == "en"`.
pub fn lang_href(path: &str, lang: &str) -> String {
    if !lang.eq_ignore_ascii_case("en") {
        return path.to_string();
    }
    if path == "/" {
        return "/en".to_string();
    }
    if path == "/arama" {
        return "/en/search".to_string();
    }
    if let Some(rest) = path.strip_prefix("/arama?") {
        return format!("/en/search?{}", rest);
    }
    if path.is_empty()
        || path.starts_with('#')
        || path.starts_with("http://")
        || path.starts_with("https://")
        || path.starts_with("mailto:")
        || path.contains("lang=")
        || !path.starts_with('/')
    {
        return path.to_string();
    }
    let (base, frag) = match path.split_once('#') {
        Some((b, f)) => (b, Some(f)),
        None => (path, None),
    };
    let sep = if base.contains('?') { '&' } else { '?' };
    match frag {
        Some(f) => format!("{}{}lang=en#{}", base, sep, f),
        None => format!("{}{}lang=en", base, sep),
    }
}

/// Translates built-in Turkish `defaults.json` section copy to British English when rendering `/en`,
/// falling back verbatim for any custom operator-authored page text.
pub fn localize_section_copy<'a>(lang: &str, raw: &'a str) -> &'a str {
    if !lang.eq_ignore_ascii_case("en") || raw.trim().is_empty() {
        return raw;
    }
    match raw.trim() {
        "Savunmanın aklı.\nGeleceğin doktrini." => "The Mind of Defence.\nThe Doctrine of Tomorrow.",
        "Savunmanın aklı. Geleceğin doktrini." => "The Mind of Defence. The Doctrine of Tomorrow.",
        "Savunma, havacılık ve uzay ekosisteminde stratejik istihbarat, mühendislik derinliği ve referans analizler." => "Strategic intelligence, engineering depth and reference analyses across the defence, aerospace and space ecosystem.",
        "MSI DERGİSİ · STRATEJİK REFERANS YAYINI" => "MSI REVIEW · STRATEGIC REFERENCE PUBLICATION",
        "Muharebe sahasını değiştiren teknolojileri, karar vericilerin vizyonunu ve mühendislik mimarisini birinci kaynaktan okuyun. Sıfır gürültü, mutlak derinlik." => "Read battlefield-shaping technologies, decision-maker perspectives and engineering architecture from primary sources. Zero noise, absolute depth.",
        "Dijital kütüphaneyi keşfet" => "Explore digital library",
        "Sayının kalbinden stratejik kapak dosyaları" => "Strategic cover dossiers from the heart of the issue",
        "KAPAK DOSYALARI" => "COVER FILES",
        "Tüm kapak dosyaları" => "All cover files",
        "Karar vericiler ve sektör liderleriyle birinci elden vizyon" => "First-hand vision from decision-makers and industry leaders",
        "SÖYLEŞİLER" => "INTERVIEWS",
        "Tüm söyleşiler" => "All interviews",
        "Saha analizleri, uluslararası fuarlar ve özel savunma dosyaları" => "Field analyses, international exhibitions and special defence dossiers",
        "ÖZEL HABERLER" => "EXCLUSIVE REPORTS",
        "Tüm özel haberler" => "All exclusive reports",
        "Editoryal perspektif ve stratejik muhasebe" => "Editorial perspective and strategic assessment",
        "BAŞYAZILAR" => "EDITORIALS",
        "Tüm başyazılar" => "All editorials",
        "Mühendislik kabiliyetleri, alt sistemler ve küresel çözümler" => "Engineering capabilities, subsystems and global solutions",
        "ÖZEL SAYILAR · TANITICI MAKALELER" => "SPECIAL ISSUES · SPECIAL COVERAGES",
        "Tüm tanıtıcı makaleler" => "All special coverages",
        "Bakanlık, tedarik makamı ve komuta kademesi mesajları" => "Messages from ministries, procurement authorities and command leadership",
        "MESAJLAR" => "EXECUTIVE MESSAGES",
        "Tüm mesajlar" => "All messages",
        "Doktrin, harp tarihi ve geleceğin kuvvet yapısı" => "Doctrine, military history and future force structure",
        "MAKALELER / ANALİZLER" => "ARTICLES & ANALYSIS",
        "Tüm makale ve analizler" => "All articles & analysis",
        "Savunma sanayii kurumsal vitrini ve teknoloji lansmanları" => "Defence industry corporate showcase and technology launches",
        "ADVERTORIALS" => "ADVERTORIALS",
        "Tüm kurumsal tanıtımlar" => "All corporate features",
        "2004'ten bugüne savunma ve havacılık hafızası" => "Defence and aerospace memory since 2004",
        "MSI DİJİTAL KÜTÜPHANESİ" => "MSI DIGITAL LIBRARY",
        "Tüm arşivi incele" => "Explore full archive",
        "Uzmanlık alanınıza göre istihbarat akışı" => "Intelligence feed tailored to your domain",
        "SSB TEKNOLOJİ TAKSONOMİSİ" => "SSB TECHNOLOGY TAXONOMY",
        "Stratejik bilgiye tam yetkiyle hükmedin." => "Command strategic intelligence with full authority.",
        "Özel kapak dosyaları, yüksek çözünürlüklü E-Dergi arşivi ve kurumsal analizlerle savunma ekosisteminin her zaman bir adım önünde olun." => "Stay ahead of the defence ecosystem with exclusive cover dossiers, high-resolution E-Magazine archives and institutional analyses.",
        "Abonelik yetkilerini incele" => "Explore subscription plans",
        "Savunma gündeminin komuta merkezi." => "The command centre of the defence agenda.",
        "Saha gelişmeleri, tedarik programları, ihracat sözleşmeleri ve yeni nesil sistemler." => "Field developments, procurement programmes, export contracts and next-generation systems.",
        "Kritik gelişmeler ve saha gündemi" => "Critical developments and field agenda",
        "SAVUNMA HABER · CANLI AKIŞ" => "DEFENCE NEWS · LIVE FEED",
        "Programlar, sözleşmeler ve teknolojik eşikler" => "Programmes, contracts and technological milestones",
        "GÜNCEL İSTİHBARAT" => "CURRENT INTELLIGENCE",
        "Tüm haber akışı" => "All news feed",
        "Derinlikli saha ve etkinlik dosyaları" => "In-depth field and exhibition dossiers",
        "Gündemi teknoloji dikeyine göre filtreleyin" => "Filter the news agenda by technology vertical",
        "STRATEJİK ALANLAR" => "STRATEGIC DOMAINS",
        "Stratejik radarda öne çıkanlar" => "Highlights on the strategic radar",
        "GÜNDEM ARŞİVİ" => "NEWS ARCHIVE",
        "Haber arşivine git" => "Go to news archive",
        "Manşetin ardındaki doktrini okuyun." => "Read the doctrine behind the headlines.",
        "Günlük haber akışının ötesine geçin; MSI Dergisi'nin derinlikli mühendislik ve strateji dosyalarını inceleyin." => "Go beyond daily headlines; explore MSI Review's in-depth engineering and strategy dossiers.",
        "Dergi moduna geç" => "Switch to magazine mode",
        "E-Dergi Arşivi" => "E-Magazine Archive",
        "Abonelik ve Yayınlar" => "Subscriptions & Store",

        // --- e-dergi & abonelik-ve-satis ---
        "MSI Dijital Kütüphanesi" => "MSI Digital Library",
        "2004'ten bugüne Türk ve dünya savunma sanayisinin teknik hafızası. Sayıları canlı inceleyin, arşive hükmedin." => "The technical memory of the Turkish and global defence industry since 2004. Browse issues live and command the archive.",
        "DİJİTAL KİOSK & ARŞİV" => "DIGITAL KIOSK & ARCHIVE",
        "Güncel ve geçmiş referans sayılar" => "Current and archival reference issues",
        "Kamu (.gov.tr / .tsk.tr) personeline ücretsiz E-Dergi erişimi ve kurumsal kütüphane planlarını keşfedin." => "Explore complimentary E-Magazine access for public-sector (.gov.tr / .tsk.tr) personnel and institutional library plans.",
        "Stratejik bilgiye tam erişim sağlayın." => "Gain full access to strategic intelligence.",
        "Bireysel uzmanlık planları, kurumsal kütüphane yetkileri ve prestij referans yayınlarıyla karar alma süreçlerinizi güçlendirin." => "Empower your decision-making with individual specialist plans, institutional library privileges and prestige reference publications.",
        "MSI OKUYUCU KULÜBÜ" => "MSI READER CLUB",
        "Abonelik ve erişim planları" => "Subscription and access plans",
        "KURUMSAL KÜTÜPHANE SEÇKİSİ" => "INSTITUTIONAL LIBRARY SELECTION",
        "Referans kitaplar, almanaklar ve özel yayınlar" => "Reference books, almanacs and special editions",
        "Çok kullanıcılı kurumsal lisans mı arıyorsunuz?" => "Looking for a multi-user institutional licence?",
        "Bakanlık, kuvvet komutanlığı, üniversite ve savunma sanayii kuruluşlarına özel toplu erişim modelleri için ekibimizle görüşün." => "Speak with our team for tailored bulk-access models for ministries, service commands, universities and defence industry organisations.",
        "Kurumsal teklif alın" => "Request an institutional quote",

        // --- Corporate Pages (/kurumsal/*) — 19 Pages in British English ---

        // 1. kurumsal-kunye
        "Resmî Künye ve İnternet Haber Sitesi İletişim Beyanı" => "Official Masthead and Online News Portal Contact Statement",
        "5187 sayılı Basın Kanunu (7418 s. Kanun değişikliği RG: 18.10.2022/31987) ve 5651 sayılı Kanun m.3 uyarınca UMSA Prodüksiyon • MSI Dergisi (ISSN 1306-0309) & MSI TDR Resmî Künyesi." => "Official Masthead of UMSA Production • MSI Review (ISSN 1306-0309) & MSI TDR pursuant to Turkish Press Law No. 5187 (as amended by Law No. 7418, OG: 18.10.2022/31987) and Law No. 5651 Art. 3.",
        "5187 & 7418 SAYILI KANUN ZORUNLU KÜNYE BİLGİLERİ" => "MANDATORY MASTHEAD DISCLOSURES UNDER LAWS NO. 5187 & 7418",
        "İmtiyaz Sahibi, Tüzel Kişilik ve Yayın Yönetimi" => "Publisher, Legal Entity and Editorial Management",
        "Yayın Adı: MSI Dergisi — Savunma, Havacılık ve Güvenlik Teknolojileri Dergisi & MSI Turkish Defence Review (MSI TDR) / savunmahaber.com\nISSN: 1306-0309\nYayın Türü: Yaygın Süreli Yayın (Aylık Basılı Dergi ve Sürekli Güncellenen İnternet Haber Sitesi)\n\nİmtiyaz Sahibi Tüzel Kişilik Unvanı: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\nİmtiyaz Sahibi Tüzel Kişilik Temsilcisi ve Genel Yayın Yönetmeni: Ümit BAYRAKTAR — ubayraktar@savunmahaber.com\nHaber Editörü ve Genel Koordinatör: Naile BAYRAKTAR — n.bayraktar@savunmahaber.com\nSorumlu Yazı İşleri Müdürü: Dr. K. Burak CODUR — b.codur@savunmahaber.com\n\nİçerik Ekibi ve Haber Merkezi:\n• Haber Merkezi Ortak E-Posta: haber.ekip@savunmahaber.com • news@savunmahaber.com\n• Emre ÜRE — e.ure@savunmahaber.com\n• Sena ŞAHAN — s.sahan@savunmahaber.com\n• Ahmet Sezai KARASAZOĞLU — a.karasazoglu@savunmahaber.com\n\nİş Geliştirme ve Kurumsal Projeler:\n• Proje ve İş Geliştirme Koordinatörü: Özay ÖYMEZ — o.oymez@savunmahaber.com\n• Kurumsal İlişkiler ve Abonelik: Refika TURGUT — r.turgut@savunmahaber.com" => "Publication Title: MSI Review (MSI Dergisi) — Defence, Aerospace and Security Technologies Magazine & MSI Turkish Defence Review (MSI TDR) / savunmahaber.com\nISSN: 1306-0309\nPublication Type: National Periodical (Monthly Print Magazine and Continuously Updated Online News Portal)\n\nPublisher Legal Entity: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\nLegal Entity Representative & Editor-in-Chief: Ümit BAYRAKTAR — ubayraktar@savunmahaber.com\nNews Editor & General Coordinator: Naile BAYRAKTAR — n.bayraktar@savunmahaber.com\nManaging Editor: Dr. K. Burak CODUR — b.codur@savunmahaber.com\n\nEditorial Team & News Desk:\n• Shared News Desk Email: haber.ekip@savunmahaber.com • news@savunmahaber.com\n• Emre ÜRE — e.ure@savunmahaber.com\n• Sena ŞAHAN — s.sahan@savunmahaber.com\n• Ahmet Sezai KARASAZOĞLU — a.karasazoglu@savunmahaber.com\n\nBusiness Development & Corporate Projects:\n• Project & Business Development Coordinator: Özay ÖYMEZ — o.oymez@savunmahaber.com\n• Corporate Relations & Subscriptions: Refika TURGUT — r.turgut@savunmahaber.com",
        "AKADEMİK VE STRATEJİK DANIŞMA KURULU" => "ACADEMIC AND STRATEGIC ADVISORY BOARD",
        "Yayın Kurulu, Danışmanlar ve Çözüm Ortakları" => "Editorial Board, Advisors and Solution Partners",
        "Yayın Kurulu:\nProf. Dr. Süleyman TOLUN, Prof. Dr. Adil YÜKSELEN, Prof. Dr. Okan ADDEMİR, Prof. Dr. Zahit MECİTOĞLU, Prof. Dr. Mikdat KADIOĞLU, Prof. Dr. Gonca COŞKUN, Prof. Dr. Yıldırım SALDIRANER, Doç. Dr. Burak ÇINAR, Prof. Dr. Ş. Hakan ATAPEK\n\nDanışmanlar:\n• Askerî Danışman: Tuğgeneral (E) Hüsamettin ESEN\n• Uluslararası İlişkiler Danışmanı: Dr. Savaş BİÇER\n\nGrafik Tasarım ve Kurumsal İletişim Ajansı:\n• KasabaWorks — info@kasabaworks.com\n\nHavacılık ve Savunma Fotoğraf Ajansı:\n• Remove Before Photos (RBF) — Onur KURÇ & Tayfun YAŞAR" => "Editorial Board:\nProf. Dr. Süleyman TOLUN, Prof. Dr. Adil YÜKSELEN, Prof. Dr. Okan ADDEMİR, Prof. Dr. Zahit MECİTOĞLU, Prof. Dr. Mikdat KADIOĞLU, Prof. Dr. Gonca COŞKUN, Prof. Dr. Yıldırım SALDIRANER, Assoc. Prof. Dr. Burak ÇINAR, Prof. Dr. Ş. Hakan ATAPEK\n\nAdvisors:\n• Military Advisor: Brig. Gen. (Ret.) Hüsamettin ESEN\n• International Relations Advisor: Dr. Savaş BİÇER\n\nGraphic Design and Corporate Communications Agency:\n• KasabaWorks — info@kasabaworks.com\n\nAerospace and Defence Photography Agency:\n• Remove Before Photos (RBF) — Onur KURÇ & Tayfun YAŞAR",
        "5651 SAYILI KANUN M.3 & BASIN KANUNU M.7 BİLDİRİMİ" => "LAW NO. 5651 ART. 3 & PRESS LAW ART. 7 NOTICE",
        "Yönetim Yeri, Baskı Tesisi ve Yer Sağlayıcı Bilgileri" => "Headquarters, Printing Facility and Hosting Provider Details",
        "Yönetim Yeri ve Çalışma Adresi:\nUMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\nYaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — TÜRKİYE\nTelefon: +90 312 225 41 73 • Faks: +90 312 225 41 74\nTicari İletişim E-Posta Adresi: info@savunmahaber.com • info@msidergisi.com\nDüzeltme, Cevap ve Hukuki Bildirim E-Posta: info@savunmahaber.com\n\nBasılı Yayın Matbaa / Teknik Hizmetler:\nEverest Basım Reklam ve Matbaa Hiz. San. Tic. Ltd. Şti.\nMaltepe Mah. Litros Yolu Sok. No: 2-4, 2. Matbaacılar Sitesi 5. Kat F Blok No: 3 NF 7, Zeytinburnu / İstanbul • Tel: +90 212 434 51 34\n\n5651 Sayılı Kanun Uyarınca Yer Sağlayıcı Bilgileri:\nUnvan: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. Dijital Yayın Altyapısı (Avrupa / Türkiye Bulut Veri Merkezi Altyapısı)\nİletişim: Yaşamkent Mah. 3147 Sok. No:10 Çankaya / Ankara • Tel: +90 312 225 41 73 • E-Posta: info@savunmahaber.com\n\nYasal Uyarı: Bu internet haber sitesinde yayımlanan haber, makale, söyleşi, fotoğraf ve E-Dergi içeriklerinin tamamı 5846 sayılı Fikir ve Sanat Eserleri Kanunu kapsamında korunmaktadır. İzin alınmaksızın ve kaynak gösterilmeksizin iktibas edilemez." => "Headquarters and Operating Address:\nUMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\nYaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — TÜRKİYE\nTelephone: +90 312 225 41 73 • Fax: +90 312 225 41 74\nCommercial Contact Email: info@savunmahaber.com • info@msidergisi.com\nCorrection, Reply & Legal Notices Email: info@savunmahaber.com\n\nPrint Production / Technical Services:\nEverest Basım Reklam ve Matbaa Hiz. San. Tic. Ltd. Şti.\nMaltepe Mah. Litros Yolu Sok. No: 2-4, 2. Matbaacılar Sitesi 5. Kat F Blok No: 3 NF 7, Zeytinburnu / İstanbul • Tel: +90 212 434 51 34\n\nHosting Provider Information under Law No. 5651:\nEntity: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. Digital Publishing Infrastructure (European / Turkish Cloud Data Centre Infrastructure)\nContact: Yaşamkent Mah. 3147 Sok. No:10 Çankaya / Ankara • Tel: +90 312 225 41 73 • Email: info@savunmahaber.com\n\nLegal Notice: All news, articles, interviews, photographs and E-Magazine content published on this online news portal are protected under Turkish Law No. 5846 on Intellectual and Artistic Works. Reproduction without prior authorisation and proper attribution is prohibited.",

        // 2. kurumsal-hakkimizda
        "Hakkımızda — Biz Kimiz?" => "About Us — Who We Are",
        "2004'ten bugüne aylık Türkçe ve İngilizce yayımlanan Savunma, Havacılık ve Güvenlik Teknolojileri referans yayın grubu." => "The reference publishing group for Defence, Aerospace and Security Technologies, published monthly in Turkish and English since 2004.",
        "KURUMSAL KİMLİK · 2004'TEN BUGÜNE" => "CORPORATE IDENTITY · SINCE 2004",
        "Türk savunma ve havacılık sanayisinin küresel referans sesi" => "The global reference voice of the Turkish defence and aerospace industry",
        "Türk savunma ve havacılık sektörünü, yurt içinde ve yurt dışında etkin bir şekilde tanıtacak referans yayınları okuyucularımızla buluşturmak hedefiyle 2004 yılında attığımız ilk adımla başlayan yolculuğumuz, yeni yayınlar ve yeni yapılanmalarla gelişerek ve güçlenerek bugünlere ulaştı.\n\nDolu dolu geçen 22 yılın ve okuyucusu ile buluşan 400'ü aşkın dergi sayısının ardından, yolumuza emin ve güçlü adımlarla devam ediyoruz.\n\n• 2004 — MSI Dergisi (ISSN 1306-0309) yayın hayatına başladı.\n• 2014 — İngilizce uluslararası edisyonumuz MSI Turkish Defence Review (MSI TDR) yayına başladı.\n• 2015 — Türkiye'de görev yapan yabancı ülke askerî ataşelerinin resmî yayını AMAC Magazine yayın ailesine katıldı.\n• 2019 — Savunma ve havacılık iletişim ajansımız KasabaWorks kuruldu.\n\nSektördeki gelişmelere dair birinci kaynaklardan elde ettiğimiz bilgilerle oluşturduğumuz içerikleri; basılı dergilerimiz, internet haber sitemiz (msidergisi.com / savunmahaber.com), dijital E-Dergi kütüphanemiz ve sosyal medya kanallarımız aracılığıyla karar vericilere, tedarik makamlarına, silahlı kuvvetler personeline, akademisyenlere ve sanayi paydaşlarına ulaştırıyoruz." => "Our journey began in 2004 with the aim of bringing authoritative reference publications to readers that effectively present the Turkish defence and aerospace sector both domestically and internationally, and it has grown and strengthened through new publications and specialised ventures.\n\nFollowing 22 eventful years and more than 400 magazine issues delivered to readers, we continue forward with confident and steady strides.\n\n• 2004 — MSI Review (MSI Dergisi, ISSN 1306-0309) launched publication.\n• 2014 — Our international English edition, MSI Turkish Defence Review (MSI TDR), began publication.\n• 2015 — AMAC Magazine, the official publication of foreign military attachés accredited in Türkiye, joined our publishing family.\n• 2019 — Our defence and aerospace communications agency, KasabaWorks, was established.\n\nBuilt on verified intelligence gathered directly from primary sources, our editorial output reaches decision-makers, procurement authorities, armed forces personnel, academics and industry stakeholders across our print editions, online news portal (msidergisi.com / savunmahaber.com), digital E-Magazine library and social media channels.",

        // 3. kurumsal-biz-kimiz
        "Biz Kimiz? — Bağımsız Savunma Gazeteciliği İlkelerimiz" => "Who We Are — Our Principles of Independent Defence Journalism",
        "MSI Dergisi ve MSI Turkish Defence Review yayın grubunun editoryal bağımsızlık, birinci kaynak doğrulama ve sektörel hafıza misyonu." => "The mission of editorial independence, primary-source verification and institutional memory across MSI Review and MSI Turkish Defence Review.",
        "EDİTORYAL KİMLİK · BAĞIMSIZ SAVUNMA YAYINCILIĞI" => "EDITORIAL IDENTITY · INDEPENDENT DEFENCE PUBLISHING",
        "Savunma ve havacılık ekosisteminin güvenilir bilgi kaynağı" => "The trusted source of intelligence for the defence and aerospace ecosystem",
        "MSI Dergisi (UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.), savunma, havacılık, uzay ve güvenlik teknolojileri alanında 2004 yılında kurulan ve sektörü mühendislik derinliğiyle belgeleyen bağımsız ihtisas yayın organıdır.\n\nYayın İlkelerimiz ve Farkımız:\n• Birinci Kaynaktan Doğrulama: Haber, söyleşi ve teknik dosyalarımızı Millî Savunma Bakanlığı (MSB), Savunma Sanayii Başkanlığı (SSB), Türk Silahlı Kuvvetleri kuvvet komutanlıkları, ana yükleniciler ve KOBİ ölçeğindeki alt sistem geliştiricileri nezdinde yürütülen doğrudan saha çalışmalarıyla hazırlarız.\n• Editoryal Bağımsızlık: Yazı işleri kadromuz reklam, sponsorluk ve ticari ilişkilerden mutlak biçimde ayrıştırılmıştır; ticari iş birliği içerikleri okuyucuya her zaman açık yasal etiketle sunulur.\n• Okuyucu Profilimiz: Basılı sayılarımız, dijital E-Dergi kütüphanemiz ve uluslararası İngilizce edisyonumuz (MSI TDR) aracılığıyla karar vericilere, tedarik makamlarına, askerî ataşeliklere, mühendislere, akademisyenlere ve sektör profesyonellerine ulaşırız.\n• Kalıcı Sektörel Hafıza: Günlük haber akışının ötesine geçerek, Türk savunma ve havacılık sanayisinin teknoloji kazanımlarını, uluslararası fuar performansını ve ihracat atılımlarını kalıcı bir kurumsal arşive dönüştürürüz." => "MSI Review (UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.) is an independent specialist publication founded in 2004 in the field of defence, aerospace, space and security technologies, documenting the sector with genuine engineering depth.\n\nOur Editorial Principles and Distinction:\n• Primary-Source Verification: We produce our news reports, interviews and technical dossiers through direct field reporting with the Ministry of National Defence (MoND), the Presidency of Defence Industries (SSB), Turkish Armed Forces service commands, prime contractors and SME subsystem developers.\n• Editorial Independence: Our editorial staff operate with strict separation from advertising, sponsorship and commercial agreements; commercial partner content is always clearly distinguished with explicit legal disclosures.\n• Our Readership Profile: Through our print magazines, digital E-Magazine library and international English edition (MSI TDR), we reach decision-makers, procurement authorities, military attachés, engineers, academics and industry professionals.\n• Enduring Institutional Memory: Going beyond daily news cycles, we preserve the technological milestones, international exhibition presence and export achievements of the Turkish defence and aerospace industry in an enduring reference archive.",

        // 4. kurumsal-yolculugumuz
        "Yolculuğumuz (2004'ten Bugüne Tarihsel Gelişim)" => "Our Journey (Historical Milestones Since 2004)",
        "Türk savunma sanayisinin öncü referans yayını olarak 2004'ten bugüne uzanan kilometre taşlarımız." => "Our milestones since 2004 as the pioneering reference publication of the Turkish defence industry.",
        "KİLOMETRE TAŞLARI (2004 – 2024)" => "MILESTONES (2004 – 2024)",
        "Tarihsel Gelişim" => "Historical Progression",
        "Türk savunma sanayisinin öncü kuruluşu olarak, yılların deneyimi ve teknolojik yenilikleriyle geleceği şekillendiriyoruz.\n\n• 2004 — MSI Dergisi'nin ilk sayısının yayınlanması\n• 2006 — İlk İngilizce Sayı: DSA Malezya Fuarı\n• 2013 — MSI Dergisi'nin 100'üncü sayısının yayınlanması\n• 2014 — MSI TDR'nin ilk sayısının yayınlanması\n• 2015 — AMAC Magazine'in ilk sayısının yayınlanması\n• 2020 — MSI Dergisi'nin 200'üncü sayısının yayınlanması\n• 2021 — MSI TDR'nin 100'üncü sayısının yayınlanması\n• 2022 — MSI Dergisi'nin 230'uncu sayısının yayınlanması\n• 2023 — MSI Dergisi'nin 245'inci sayısının yayınlanması\n• 2024 — MSI Dergisi'nin 250'inci sayısının yayınlanması\n\nMSI Dergisi: Sektöre ilişkin güncel gelişmeleri her ay ve Türkçe olarak okurlarına sunan MSI Dergisi; son kullanıcı, tedarik makamı, üniversiteler, araştırma merkezleri, kümelenmeler, ana yükleniciler ve alt yükleniciler arasında stratejik bilgi köprüsü kurmaktadır.\n\nMSI TDR (Turkish Defence Review): 2014 yılından bu yana sektörün ihracat faaliyetlerini destekleyecek şekilde uluslararası karar vericilere İngilizce olarak ulaşan tek aylık Türk savunma ve havacılık dergisidir." => "As a pioneering institution in the Turkish defence industry, we help shape the future through decades of experience and technological innovation.\n\n• 2004 — Publication of the inaugural issue of MSI Review (MSI Dergisi)\n• 2006 — First English Special Issue: DSA Malaysia Exhibition\n• 2013 — Publication of the 100th issue of MSI Review\n• 2014 — Publication of the inaugural issue of MSI TDR\n• 2015 — Publication of the inaugural issue of AMAC Magazine\n• 2020 — Publication of the 200th issue of MSI Review\n• 2021 — Publication of the 100th issue of MSI TDR\n• 2022 — Publication of the 230th issue of MSI Review\n• 2023 — Publication of the 245th issue of MSI Review\n• 2024 — Publication of the 250th issue of MSI Review\n\nMSI Review (MSI Dergisi): Delivering monthly sector coverage in Turkish, MSI Review builds a strategic knowledge bridge connecting end users, procurement authorities, universities, research centres, industry clusters, prime contractors and subcontractors.\n\nMSI TDR (Turkish Defence Review): Since 2014, the sole monthly Turkish defence and aerospace magazine reaching international decision-makers in English to support the industry's export programmes.",

        // 5. kurumsal-cozum-ortaklarimiz
        "Çözüm Ortaklarımız: AMAC Magazine, KasabaWorks & Remove Before Photos" => "Our Solution Partners: AMAC Magazine, KasabaWorks & Remove Before Photos",
        "UMSA Prodüksiyon stratejik iletişim, kurumsal tasarım, havacılık fotoğrafçılığı ve savunma diplomasisi ekosistemi." => "UMSA Production's ecosystem of strategic communications, corporate design, aerospace photography and defence diplomacy.",
        "STRATEJİK İLETİŞİM VE DİPLOMASİ EKOSİSTEMİ" => "STRATEGIC COMMUNICATIONS AND DIPLOMACY ECOSYSTEM",
        "AMAC Magazine, KasabaWorks ve Remove Before Photos" => "AMAC Magazine, KasabaWorks and Remove Before Photos",
        "AMAC Magazine (Savunma Diplomasisi • 2015'ten Beri):\nAnkara Askerî Ataşeler Birliği (Ankara Military Attaché Corps — AMAC) üyesi askerî ataşeler ve temsil ettikleri ülkelerin diplomatik misyonları ile Türk savunma ve havacılık sanayisi arasında köprü vazifesi gören AMAC Magazine, İngilizce olarak yılda 6 sayı yayımlanmaktadır.\n\nKasabaWorks (Reklam ve Kurumsal İletişim Ajansı • 2019'dan Beri):\nTürk savunma ve havacılık sektörünün tanıtım alanındaki özel ihtiyaçlarına cevap verebilmek amacıyla kurulan KasabaWorks, 'Dış Kaynak Kurumsal İletişim Birimi' iş modeliyle kurumsal kimlik, uluslararası fuar iletişimi ve prestij yayın tasarımları sunmaktadır. İletişim: info@kasabaworks.com\n\nRemove Before Photos — RBF (Havacılık ve Savunma Fotoğraf Ajansı):\nOnur KURÇ ve Tayfun YAŞAR yönetimindeki Remove Before Photos ekibi; hava-hava muharebe fotoğrafçılığı, saha tatbikat çekimleri ve yüksek çözünürlüklü telifli savunma fotoğraf arşivi alanında resmî çözüm ortağımızdır." => "AMAC Magazine (Defence Diplomacy • Since 2015):\nServing as a bridge between the military attachés of the Ankara Military Attaché Corps (AMAC), their respective diplomatic missions, and the Turkish defence and aerospace industry, AMAC Magazine is published six times a year in English.\n\nKasabaWorks (Advertising and Corporate Communications Agency • Since 2019):\nFounded to address the specialised promotional requirements of the Turkish defence and aerospace sector, KasabaWorks operates as an 'Outsourced Corporate Communications Unit', delivering corporate identity, international exhibition communications and prestige publication design. Contact: info@kasabaworks.com\n\nRemove Before Photos — RBF (Aerospace and Defence Photography Agency):\nLed by Onur KURÇ and Tayfun YAŞAR, the Remove Before Photos team is our official partner for air-to-air combat photography, field exercise coverage and licensed high-resolution defence photography archives.",

        // 6. kurumsal-amac-magazine
        "AMAC Magazine — Savunma Diplomasisi Prestij Yayını" => "AMAC Magazine — Prestige Publication of Defence Diplomacy",
        "Ankara Askerî Ataşeler Birliği (AMAC) ile Türk savunma ve havacılık sanayisi arasındaki uluslararası referans yayını." => "The international reference publication connecting the Ankara Military Attaché Corps (AMAC) and the Turkish defence and aerospace industry.",
        "SAVUNMA DİPLOMASİSİ · YILDA 6 SAYI (İNGİLİZCE)" => "DEFENCE DIPLOMACY · 6 ISSUES PER YEAR (ENGLISH)",
        "AMAC Magazine" => "AMAC Magazine",
        "Ankara Askerî Ataşeler Birliği (Ankara Military Attaché Corps — AMAC) üyesi yabancı ülke askerî ataşeleri, büyükelçilik savunma misyonları ve uluslararası tedarik heyetleri ile Türk savunma ve havacılık sanayisi arasında doğrudan iletişim köprüsü kuran AMAC Magazine, UMSA Prodüksiyon tarafından İngilizce olarak yılda 6 sayı yayımlanmaktadır.\n\n44'ü aşkın sayısıyla Türk savunma sanayisinin ihracat ve uluslararası iş birliği vizyonunu diplomatik misyonlara birinci elden aktaran yayın hakkında kurumsal iş birliği talepleriniz için info@savunmahaber.com adresinden ekibimize ulaşabilirsiniz." => "Published six times annually in English by UMSA Production, AMAC Magazine establishes a direct communications bridge between foreign military attachés belonging to the Ankara Military Attaché Corps (AMAC), embassy defence missions, international procurement delegations and the Turkish defence and aerospace industry.\n\nWith more than 44 issues published, the magazine conveys the export and international cooperation vision of the Turkish defence sector directly to diplomatic missions. For institutional collaboration enquiries, please contact our team at info@savunmahaber.com.",

        // 7. kurumsal-kasabaworks
        "KasabaWorks — Savunma ve Havacılık İletişim Ajansı" => "KasabaWorks — Defence and Aerospace Communications Agency",
        "Savunma, havacılık ve ileri teknoloji kuruluşları için dış kaynak kurumsal iletişim, tasarım ve fuar yönetimi ajansı." => "Outsourced corporate communications, design and exhibition management agency for defence, aerospace and advanced technology organisations.",
        "KURUMSAL İLETİŞİM & TASARIM · 2019'DAN BERİ" => "CORPORATE COMMUNICATIONS & DESIGN · SINCE 2019",
        "KasabaWorks Reklam ve İletişim Ajansı" => "KasabaWorks Advertising and Communications Agency",
        "Türk savunma ve havacılık sektörünün uzmanlık gerektiren tanıtım ihtiyaçlarına yanıt vermek amacıyla 2019 yılında kurulan KasabaWorks, MSI Dergisi ekibinin 20+ yıllık sektörel terminoloji ve mühendislik birikimi üzerine inşa edilmiştir.\n\n'Dış Kaynak Kurumsal İletişim Birimi' modeliyle savunma sanayii ana ve alt yüklenicilerine; kurumsal kimlik, teknik ürün katalogları, prestij kitapları, uluslararası savunma fuarları (IDEF, SAHA EXPO, Eurosatory, MSPO, DIMDEX, DSA) iletişim yönetimi ve dijital medya prodüksiyonu hizmetleri sunmaktadır.\n\nİletişim: info@kasabaworks.com • Tel: +90 312 225 41 73" => "Established in 2019 to meet the highly specialised communications requirements of the Turkish defence and aerospace sector, KasabaWorks builds on more than 20 years of sector terminology and engineering expertise from the MSI Review team.\n\nThrough its 'Outsourced Corporate Communications Unit' model, KasabaWorks provides defence prime contractors and subcontractors with corporate identity, technical product catalogues, prestige monographs, international defence exhibition communications management (IDEF, SAHA EXPO, Eurosatory, MSPO, DIMDEX, DSA) and digital media production.\n\nContact: info@kasabaworks.com • Tel: +90 312 225 41 73",

        // 8. kurumsal-iletisim
        "İletişim, Merkez Ofis ve Hukuki Bildirim Kanalları" => "Contact, Headquarters and Legal Notification Channels",
        "UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. • Yaşamkent Mah. 3147 Sok. No:10 Çankaya / Ankara • +90 312 225 41 73" => "UMSA Production • Yaşamkent Mah. 3147 Sok. No:10 Çankaya / Ankara, Türkiye • +90 312 225 41 73",
        "MERKEZ OFİS & EDİTORYAL İLETİŞİM" => "HEADQUARTERS & EDITORIAL CONTACT",
        "Bize Ulaşın" => "Contact Us",
        "Tüzel Kişilik Unvanı: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\nYönetim ve Çalışma Adresi: Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — TÜRKİYE\nTelefon: +90 312 225 41 73\nFaks: +90 312 225 41 74\n\nBirim E-Posta Adresleri:\n• Kurumsal İletişim ve Okuyucu İlişkileri: info@savunmahaber.com • info@msidergisi.com\n• Haber Merkezi ve Basın Bültenleri: news@savunmahaber.com • haber.ekip@savunmahaber.com\n• Abonelik, Kurumsal Lisans ve Satış: r.turgut@savunmahaber.com\n• Reklam, Sponsorluk ve İş Geliştirme: o.oymez@savunmahaber.com\n• KVKK İlgili Kişi Başvuruları ve Hukuki Bildirimler: info@savunmahaber.com\n\nResmî Sosyal Medya Hesaplarımız:\n• LinkedIn: linkedin.com/company/msi-dergisi\n• X (Twitter): x.com/MSI_Dergisi\n• Instagram: instagram.com/msidergisi\n• Facebook: facebook.com/MSI.Dergisi\n• YouTube: youtube.com/c/msidergisi" => "Legal Entity: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\nHeadquarters & Operating Address: Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — TÜRKİYE\nTelephone: +90 312 225 41 73\nFax: +90 312 225 41 74\n\nDepartment Email Addresses:\n• Corporate Communications & Reader Relations: info@savunmahaber.com • info@msidergisi.com\n• News Desk & Press Releases: news@savunmahaber.com • haber.ekip@savunmahaber.com\n• Subscriptions, Corporate Licences & Sales: r.turgut@savunmahaber.com\n• Advertising, Sponsorships & Business Development: o.oymez@savunmahaber.com\n• KVKK Data Subject Requests & Legal Notices: info@savunmahaber.com\n\nOfficial Social Media Channels:\n• LinkedIn: linkedin.com/company/msi-dergisi\n• X (Twitter): x.com/MSI_Dergisi\n• Instagram: instagram.com/msidergisi\n• Facebook: facebook.com/MSI.Dergisi\n• YouTube: youtube.com/c/msidergisi",

        // 9. kurumsal-kvkk
        "6698 Sayılı KVKK Kapsamında Kişisel Verilerin İşlenmesine İlişkin Aydınlatma Metni" => "Personal Data Processing Disclosure Notice under Law No. 6698 (KVKK)",
        "6698 sayılı Kişisel Verilerin Korunması Kanunu m.10, 7499 sayılı Kanun (RG: 12.03.2024/32487) m.6-m.9 değişiklikleri ve RG: 30356 Aydınlatma Tebliği uyarınca resmî aydınlatma metni." => "Official disclosure notice pursuant to Article 10 of Turkish Personal Data Protection Law No. 6698 (KVKK), amendments under Law No. 7499 (OG: 12.03.2024/32487) Arts. 6–9, and the Communiqué on Disclosure Obligations (OG: 30356).",
        "6698 SAYILI KANUN M.10 & RG: 30356 TEBLİĞİ" => "LAW NO. 6698 ART. 10 & COMMUNIQUÉ OG: 30356",
        "1. Veri Sorumlusunun Kimliği ve Kapsam" => "1. Identity of the Data Controller and Scope",
        "İşbu Aydınlatma Metni, 6698 sayılı Kişisel Verilerin Korunması Kanunu ('KVKK') m.10, 10 Mart 2018 tarihli ve 30356 sayılı Resmî Gazete'de yayımlanan Aydınlatma Yükümlülüğünün Yerine Getirilmesinde Uyulacak Usul ve Esaslar Hakkında Tebliğ ile 12 Mart 2024 tarihli ve 32487 sayılı Resmî Gazete'de yayımlanan 7499 sayılı Kanun değişiklikleri uyarınca, Veri Sorumlusu sıfatıyla MSI Dergisi (UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.) tarafından hazırlanmıştır.\n\n• Veri Sorumlusu: MSI Dergisi — UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\n• Adres: Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — TÜRKİYE\n• Telefon: +90 312 225 41 73 • Faks: +90 312 225 41 74\n• E-Posta: info@savunmahaber.com\n• Web Platformları: https://www.msidergisi.com ve savunmahaber.com" => "This Disclosure Notice has been prepared by MSI Review (UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.) acting as Data Controller pursuant to Article 10 of Law No. 6698 on the Protection of Personal Data ('KVKK'), the Communiqué on Procedures and Principles for Fulfilling the Obligation to Inform (Official Gazette No. 30356, 10 March 2018), and the amendments introduced by Law No. 7499 (Official Gazette No. 32487, 12 March 2024).\n\n• Data Controller: MSI Review — UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\n• Address: Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — TÜRKİYE\n• Telephone: +90 312 225 41 73 • Fax: +90 312 225 41 74\n• Email: info@savunmahaber.com\n• Web Platforms: https://www.msidergisi.com and savunmahaber.com",
        "KİŞİSEL VERİ KATEGORİLERİ, AMAÇLAR VE HUKUKİ SEBEPLER" => "PERSONAL DATA CATEGORIES, PURPOSES AND LEGAL GROUNDS",
        "2. İşlenen Kişisel Veriler, İşleme Amaçları ve KVKK m.5 Hukuki Sebepleri" => "2. Categories of Personal Data, Processing Purposes and Legal Grounds under KVKK Art. 5",
        "Platformumuz üzerinden toplanan kişisel verileriniz, Aydınlatma Tebliği m.5/1-f hükmüne uygun olarak açık rıza şartından bağımsız biçimde aşağıdaki amaç ve hukuki sebeplerle işlenmektedir:\n\nA) Kimlik ve Kurumsal Üyelik Verileri (Ad-soyad, görev/ünvan, kurum adı, resmi kurum kategorisi):\n• Amaç: Okuyucu hesabının oluşturulması, kamu (.gov.tr / .tsk.tr / .edu.tr) e-posta uzantılı kullanıcılara ücretsiz dijital E-Dergi yetkisinin tanımlanması ve kurumsal doğrulama süreçlerinin yürütülmesi.\n• Hukuki Sebep: KVKK m.5/2-(c) 'Bir sözleşmenin kurulması veya ifasıyla doğrudan doğruya ilgili olması'.\n\nB) İletişim ve Finans/Abonelik Verileri (Kurumsal e-posta, telefon, teslimat/fatura adresi, sipariş ve proforma/fatura kayıtları):\n• Amaç: Basılı dergi, dijital E-Dergi ve tekil yayın satış/abonelik işlemlerinin ifası, 213 sayılı Vergi Usul Kanunu ve 6502 sayılı TKHK kapsamındaki faturalandırma ve abonelik yenileme bildirimlerinin yapılması.\n• Hukuki Sebep: KVKK m.5/2-(c) sözleşmenin ifası ve m.5/2-(ç) 'Veri sorumlusunun hukuki yükümlülüğünü yerine getirebilmesi için zorunlu olması'.\n\nC) İşlem Güvenliği ve Siber Güvenlik Kayıtları (IP adresi, erişim zaman damgası, oturum doğrulama özeti, KVKK aydınlatma/onay zaman damgası, E-Dergi PDF DRM filigran kodu, WAF güvenlik denetim logları):\n• Amaç: 5651 sayılı Kanun kapsamındaki trafik kayıt yükümlülüğünün yerine getirilmesi, yetkisiz erişim/SQLi/XSS saldırılarının engellenmesi ve 5846 sayılı FSEK kapsamında dijital yayın telif güvenliğinin sağlanması.\n• Hukuki Sebep: KVKK m.5/2-(a) 'Kanunlarda açıkça öngörülmesi' ve m.5/2-(f) 'İlgili kişinin temel hak ve özgürlüklerine zarar vermemek kaydıyla veri sorumlusunun meşru menfaati'.\n\nD) İsteğe Bağlı Ticari Elektronik İleti ve Analitik/Pazarlama Verileri (E-bülten tercihi, SSB teknoloji ilgi alanı etiketleri, çerez onay tercihleri):\n• Amaç: Yeni sayı duyuruları, özel savunma dosyaları bülten gönderimi ve okuma deneyimi analitiği.\n• Hukuki Sebep: KVKK m.5/1 kapsamında özgür iradeyle verilen ve her zaman geri alınabilen 'Açık Rıza' (ve 6563 sayılı ETK kapsamında ticari elektronik ileti onayı)." => "Personal data collected through our platform are processed separately from explicit consent requirements, in accordance with Article 5/1-f of the Disclosure Communiqué, for the following purposes and legal grounds:\n\nA) Identity and Institutional Membership Data (Full name, role/title, organisation name, official institution category):\n• Purpose: Creating reader accounts, granting complimentary digital E-Magazine privileges to verified public-sector (.gov.tr / .tsk.tr / .edu.tr) email holders and executing institutional verification workflows.\n• Legal Ground: KVKK Art. 5/2-(c) 'Directly related to the establishment or performance of a contract'.\n\nB) Contact, Billing and Subscription Data (Work email, telephone, delivery/billing address, orders and proforma/invoice records):\n• Purpose: Fulfilment of print magazine, digital E-Magazine and single-item purchases/subscriptions, statutory invoicing under Tax Procedure Law No. 213 and Consumer Protection Law No. 6502, and subscription renewal notifications.\n• Legal Ground: KVKK Art. 5/2-(c) performance of a contract and Art. 5/2-(ç) 'Mandatory for the data controller to fulfil its legal obligation'.\n\nC) Transaction Security and Cybersecurity Logs (IP address, access timestamp, session verification digest, KVKK disclosure/consent timestamp, E-Magazine PDF DRM watermark code, WAF security audit logs):\n• Purpose: Compliance with traffic logging duties under Law No. 5651, prevention of unauthorised access and SQLi/XSS attacks, and digital copyright protection under Law No. 5846.\n• Legal Ground: KVKK Art. 5/2-(a) 'Expressly provided for by law' and Art. 5/2-(f) 'Legitimate interests of the data controller without harming the fundamental rights and freedoms of the data subject'.\n\nD) Optional Commercial Electronic Messaging and Analytics/Marketing Data (Newsletter preference, SSB technology interest tags, cookie consent choices):\n• Purpose: New issue announcements, special defence dossier briefings and anonymised reading experience analytics.\n• Legal Ground: Freely given and revocable 'Explicit Consent' under KVKK Art. 5/1 (and commercial messaging consent under Law No. 6563).",
        "7499 SAYILI KANUN (RG: 12.03.2024/32487) YENİ AKTARIM REJİMİ" => "AMENDED TRANSFER REGIME UNDER LAW NO. 7499 (OG: 12.03.2024/32487)",
        "3. Kişisel Verilerin Yurt İçi ve Yurt Dışı Aktarımı (KVKK m.8 ve Değişik m.9)" => "3. Domestic and Cross-Border Transfers of Personal Data (KVKK Art. 8 & Amended Art. 9)",
        "3.1. Yurt İçine Aktarım (KVKK m.8):\nKişisel verileriniz; yalnızca yukarıda belirtilen amaçların gerçekleştirilmesiyle sınırlı olarak ödeme kuruluşu iş ortağımıza (PayTR Ödeme ve Elektronik Para Kuruluşu A.Ş. — BDDK lisanslı ve PCI-DSS uyumlu), basılı yayın teslimatlarında anlaşmalı kargo/posta kuruluşlarına, mali müşavirlik birimlerine ve kanunen yetkili kamu kurumları ile adli mercilere KVKK m.8 çerçevesinde aktarılabilmektedir.\n\n3.2. Yurt Dışına Aktarım (7499 Sayılı Kanun ile Değişik KVKK m.9 ve RG: 10.07.2024 / 32598 Yönetmeliği):\n1 Haziran 2024 tarihinde yürürlüğe giren değişik KVKK m.9 ve Kişisel Verilerin Yurt Dışına Aktarılmasına İlişkin Usul ve Esaslar Hakkında Yönetmelik uyarınca kişisel verileriniz yurt dışına ancak;\n(i) Kişisel Verileri Koruma Kurulu ('Kurul') tarafından hakkında yeterlilik kararı verilen ülke veya uluslararası kuruluşlara,\n(ii) Yeterlilik kararı bulunmaması hâlinde KVKK m.5 ve m.6'daki şartlardan birinin varlığı ve ilgili kişinin aktarımın yapılacağı ülkede de haklarını kullanma ve etkili kanun yollarına başvurma imkânının bulunması kaydıyla Kurul tarafından ilan edilen Standart Sözleşme'nin imzalanması ve 5 iş günü içinde Kurum'a bildirilmesi dâhil uygun güvencelerden birinin sağlanması durumunda veya\n(iii) KVKK m.9/6 kapsamındaki arızi (istisnai) hâllerden birinin varlığı hâlinde aktarılabilir." => "3.1. Domestic Transfers (KVKK Art. 8):\nSolely to the extent necessary for the purposes stated above, your personal data may be transferred domestically under KVKK Art. 8 to our payment service partner (PayTR Ödeme ve Elektronik Para Kuruluşu A.Ş. — licensed by BRSA and PCI-DSS compliant), contracted courier/postal carriers for print deliveries, certified accounting advisors, and legally authorised public authorities and judicial bodies.\n\n3.2. Cross-Border Transfers (KVKK Art. 9 as Amended by Law No. 7499 and Regulation OG: 10.07.2024 / 32598):\nPursuant to amended KVKK Art. 9 (in force since 1 June 2024) and the Regulation on Procedures and Principles for Cross-Border Transfers of Personal Data, your personal data may be transferred abroad only where:\n(i) An adequacy decision has been issued by the Personal Data Protection Board ('Board') for the destination country or international organisation;\n(ii) In the absence of an adequacy decision, one of the conditions under KVKK Arts. 5–6 applies, the data subject enjoys enforceable rights and effective legal remedies in the destination country, and appropriate safeguards are provided—such as signing the Standard Contractual Clauses published by the Board and notifying the Authority within 5 business days; or\n(iii) One of the exceptional (derogatory) situations specified in KVKK Art. 9/6 applies.",
        "KVKK MADDE 11 & VERİ SORUMLUSUNA BAŞVURU USULÜ" => "KVKK ARTICLE 11 & DATA SUBJECT APPLICATION PROCEDURE",
        "4. Toplanma Yöntemi ve İlgili Kişinin Hakları (KVKK m.11)" => "4. Collection Method and Rights of the Data Subject (KVKK Art. 11)",
        "Kişisel verileriniz; msidergisi.com ve savunmahaber.com üzerindeki üyelik, kurumsal doğrulama, abonelik/sipariş formları ve teknik güvenlik katmanları aracılığıyla elektronik ortamda otomatik veya kısmen otomatik yöntemlerle toplanmaktadır.\n\nKVKK m.11 uyarınca Veri Sorumlusuna başvurarak;\n• Kişisel verilerinizin işlenip işlenmediğini öğrenme ve işlenmişse bilgi talep etme,\n• İşlenme amacını ve bunların amacına uygun kullanılıp kullanılmadığını öğrenme,\n• Yurt içinde veya yurt dışında kişisel verilerin aktarıldığı üçüncü kişileri bilme,\n• Eksik veya yanlış işlenmiş olması hâlinde düzeltilmesini isteme,\n• KVKK m.7 şartları çerçevesinde silinmesini veya yok edilmesini isteme ve bu işlemlerin aktarıldığı üçüncü kişilere bildirilmesini talep etme,\n• Münhasıran otomatik sistemler vasıtasıyla analiz edilmesi suretiyle aleyhinize bir sonucun ortaya çıkmasına itiraz etme,\n• Kanuna aykırı olarak işlenmesi sebebiyle zarara uğramanız hâlinde zararın giderilmesini talep etme haklarına sahipsiniz.\n\nHaklarınıza ilişkin taleplerinizi '/kurumsal/kvkk-basvuru-formu' sayfasında yer alan usullere uygun olarak yazılı şekilde 'Yaşamkent Mah. 3147 Sok. No:10 Çankaya / Ankara' adresine veya sistemimizde kayıtlı e-posta adresiniz üzerinden 'info@savunmahaber.com' adresine iletebilirsiniz. Başvurularınız niteliğine göre en geç 30 (otuz) gün içinde ücretsiz olarak sonuçlandırılır." => "Your personal data are collected electronically through automated or partially automated means via registration, institutional verification, subscription/order forms and technical security layers on msidergisi.com and savunmahaber.com.\n\nUnder KVKK Art. 11, you are entitled to apply to the Data Controller to:\n• Learn whether your personal data are processed and request information if so,\n• Learn the purpose of processing and whether data are used in accordance with that purpose,\n• Know the third parties to whom personal data are transferred domestically or abroad,\n• Request rectification if data are incomplete or inaccurate,\n• Request erasure or destruction under KVKK Art. 7 and notification of such actions to third parties to whom data were transferred,\n• Object to any adverse outcome arising exclusively from automated analysis,\n• Claim compensation for damages incurred due to unlawful processing.\n\nYou may submit your requests in accordance with the procedures detailed on '/kurumsal/kvkk-basvuru-formu?lang=en' in writing to 'Yaşamkent Mah. 3147 Sok. No:10 Çankaya / Ankara' or from your registered email address to 'info@savunmahaber.com'. Requests are concluded free of charge within 30 (thirty) days at the latest.",

        // 10. kurumsal-kvkk-basvuru-formu
        "KVKK İlgili Kişi Başvuru Usul ve Esasları Rehberi" => "KVKK Data Subject Application Procedures and Principles Guide",
        "10.03.2018 tarihli ve 30356 sayılı Resmî Gazete'de yayımlanan Veri Sorumlusuna Başvuru Usul ve Esasları Hakkında Tebliğ m.5 uyarınca resmi başvuru kılavuzu." => "Official application guide pursuant to Article 5 of the Communiqué on Procedures and Principles for Applications to Data Controllers (OG: 10.03.2018 / 30356).",
        "RG: 10.03.2018 / 30356 TEBLİĞ M.4 VE M.5" => "COMMUNIQUÉ OG: 10.03.2018 / 30356 ARTS. 4 & 5",
        "Başvuru Kanalları ve Zorunlu Başvuru Unsurları" => "Application Channels and Mandatory Application Details",
        "6698 sayılı KVKK m.13 ve Veri Sorumlusuna Başvuru Usul ve Esasları Hakkında Tebliğ m.5 uyarınca ilgili kişiler, KVKK m.11 kapsamındaki taleplerini Türkçe olarak aşağıdaki kanallardan biriyle UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.'ye iletebilir:\n\n1. Yazılı ve Islak İmzalı Başvuru: 'Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara' adresine kimlik teyitli elden teslim veya noter kanalıyla ('KVKK Bilgi Talebi' notuyla).\n2. Kayıtlı Elektronik Posta (KEP) / Güvenli Elektronik İmza / Mobil İmza: Güvenli elektronik imza veya mobil imza ile imzalanmış dilekçenin kurumsal e-posta kanallarımıza iletilmesi.\n3. Sistemde Kayıtlı E-Posta Adresi Üzerinden Başvuru: Okuyucu hesabınızda daha önce doğrulanmış ve sistemimizde kayıtlı bulunan e-posta adresinizden 'info@savunmahaber.com' adresine gönderilecek e-posta ile.\n\nTebliğ m.5/2 Uyarınca Başvuruda Bulunması Zorunlu Bilgiler:\n• Ad, soyad ve başvuru yazılı ise imza,\n• Türkiye Cumhuriyeti vatandaşları için T.C. kimlik numarası; yabancılar için uyruğu, pasaport numarası veya varsa kimlik numarası,\n• Tebligata esas yerleşim yeri veya iş yeri adresi,\n• Varsa bildirime esas elektronik posta adresi, telefon ve faks numarası,\n• Talep konusu ve konuya ilişkin bilgi/belgeler.\n\nYanıt Süresi ve Şikâyet Hakkı (KVKK m.13–14):\nUsulüne uygun başvurular en geç 30 gün içinde ücretsiz sonuçlandırılır; işlemin ayrıca bir maliyet gerektirmesi hâlinde Kişisel Verileri Koruma Kurulunca belirlenen tarifedeki ücret alınabilir. Başvurunun reddedilmesi, verilen cevabın yetersiz bulunması veya süresinde başvuruya cevap verilmemesi hâllerinde; cevabı öğrendiğiniz tarihten itibaren 30 ve her hâlde başvuru tarihinden itibaren 60 gün içinde Kişisel Verileri Koruma Kuruluna şikâyette bulunma hakkınız saklıdır." => "Pursuant to KVKK Art. 13 and Art. 5 of the Communiqué on Procedures and Principles for Applications to Data Controllers, data subjects may submit requests under KVKK Art. 11 in Turkish to UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. via one of the following channels:\n\n1. Written Application with Wet Signature: Delivered in person with identity verification or via notary public to 'Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara' (marked 'KVKK Information Request').\n2. Registered Electronic Mail (KEP) / Qualified Electronic Signature / Mobile Signature: Petition signed with a qualified electronic or mobile signature sent to our corporate email channels.\n3. Application via Registered Email Address: Email sent to 'info@savunmahaber.com' from the email address previously verified and recorded in your reader account.\n\nMandatory Information under Communiqué Art. 5/2:\n• Full name and signature (if submitted in writing),\n• Turkish Republic ID number for Turkish citizens; nationality and passport or ID number for foreign nationals,\n• Residential or business address for formal notification,\n• Email address, telephone and fax number for notifications, if available,\n• Subject matter of the request and supporting information/documents.\n\nResponse Time and Right to Lodge a Complaint (KVKK Arts. 13–14):\nProperly submitted applications are concluded free of charge within 30 days at the latest; if the request entails an additional cost, the tariff determined by the Personal Data Protection Board may apply. If the request is rejected, the response is deemed insufficient, or no response is provided within the statutory period, you may lodge a complaint with the Personal Data Protection Board within 30 days of learning the response and in any event within 60 days of the application date.",

        // 11. kurumsal-gizlilik-politikasi
        "Gizlilik, Veri Güvenliği ve Kişisel Veri Saklama-İmha Politikası" => "Privacy, Data Security and Personal Data Retention & Destruction Policy",
        "6698 sayılı KVKK m.12, RG: 30224 Silme-Yok Etme-Anonim Hale Getirme Yönetmeliği ve 5651 sayılı Kanun kapsamında veri güvenliği ve saklama süreleri." => "Data security measures and statutory retention periods under KVKK Art. 12, Regulation OG: 30224 on Erasure, Destruction and Anonymisation, and Law No. 5651.",
        "KVKK MADDE 12 TEKNİK VE İDARİ TEDBİRLER" => "KVKK ARTICLE 12 TECHNICAL AND ORGANISATIONAL MEASURES",
        "1. Siber Güvenlik ve Veri Koruma Mimarisi" => "1. Cybersecurity and Data Protection Architecture",
        "UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. ('MSI Dergisi'), KVKK m.12 uyarınca kişisel verilerin hukuka aykırı olarak işlenmesini ve erişilmesini önlemek, muhafazasını sağlamak amacıyla aşağıdaki gerçek teknik ve idari güvenlik tedbirlerini uygulamaktadır:\n\n• Kriptografik Parola Güvenliği: Kullanıcı parolaları açık metin olarak asla saklanmaz; endüstri standardı bellek-sert Argon2id algoritması ve kriptografik tuzlama (salting) ile özetlenerek muhafaza edilir.\n• Güvenli Oturum Yönetimi: Oturum doğrulaması tarayıcı JavaScript kodunun erişimine kapalı (HttpOnly), SameSite=Lax ve HTTPS üzerinde Secure bayraklı, HMAC-SHA256 imzalı oturum çerezi ('msi_session') ile yürütülür; oturum anahtarları localStorage üzerinde tutulmaz.\n• Aktarım ve Ağ Güvenliği: Tüm veri trafiği TLS 1.2/1.3 şifrelemesi, katı Content-Security-Policy (CSP), X-Frame-Options: SAMEORIGIN ve merkezi Rust Web Uygulama Güvenlik Duvarı (WAF — SQL Injection, XSS, Path Traversal ve hız sınırlama denetimi) ile korunmaktadır.\n• Ödeme Güvenliği: Kredi/banka kartı verileri MSI Dergisi sunucularında işlenmez ve saklanmaz; doğrudan BDDK lisanslı ve PCI-DSS Seviye 1 sertifikalı ödeme kuruluşu PayTR güvenli iFrame altyapısı üzerinden gerçekleştirilir." => "Pursuant to KVKK Art. 12, UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. ('MSI Review') implements the following technical and organisational safeguards to prevent unlawful processing or unauthorised access and to ensure the integrity of personal data:\n\n• Cryptographic Password Security: User passwords are never stored in plaintext; they are hashed using the industry-standard memory-hard Argon2id algorithm with cryptographic salting.\n• Secure Session Management: Session authentication relies on an HMAC-SHA256 signed cookie ('msi_session') configured with HttpOnly (inaccessible to browser JavaScript), SameSite=Lax and Secure over HTTPS; session keys are never stored in localStorage.\n• Transport and Network Security: All data traffic is protected by TLS 1.2/1.3 encryption, strict Content-Security-Policy (CSP), X-Frame-Options: SAMEORIGIN and a central Rust Web Application Firewall (WAF enforcing SQL Injection, XSS, Path Traversal and rate-limit controls).\n• Payment Security: Credit and debit card details are never processed or stored on MSI Review servers; transactions take place directly via the secure iFrame infrastructure of PayTR, a BRSA-licensed and PCI-DSS Level 1 certified payment institution.",
        "RG: 28.10.2017 / 30224 YÖNETMELİĞİ UYUMU" => "COMPLIANCE WITH REGULATION OG: 28.10.2017 / 30224",
        "2. Yasal Veri Saklama Süreleri ve Periyodik İmha Esasları" => "2. Statutory Data Retention Periods and Periodic Destruction Rules",
        "İlgili mevzuatta öngörülen veya işlendikleri amaç için gerekli olan süre kadar muhafaza edilen kişisel veriler, sürenin bitimini takip eden ilk periyodik imha döneminde (6 aylık periyotlarla) silinir, yok edilir veya anonim hâle getirilir:\n\n• Abonelik, Sipariş, Proforma ve Fatura Kayıtları: 213 sayılı Vergi Usul Kanunu m.253 uyarınca 5 yıl; 6102 sayılı Türk Ticaret Kanunu m.82 uyarınca 10 yıl.\n• Mesafeli Satış ve Tüketici İşlem Kayıtları: 6502 sayılı TKHK ve Mesafeli Sözleşmeler Yönetmeliği m.20 uyarınca 3 yıl.\n• Ticari Elektronik İleti Onay ve Ret Kayıtları: 6563 sayılı ETK ve ilgili Yönetmelik uyarınca onayın geri alındığı/sona erdiği tarihten itibaren 3 yıl.\n• İnternet Trafik ve Siber Güvenlik Logları: 5651 sayılı Kanun uyarınca 2 yıl.\n• Üyelik ve Kurumsal Doğrulama Kayıtları: Üyelik ilişkisi devam ettiği sürece ve sona ermesinden itibaren genel zamanaşımı (6098 sayılı TBK m.146 uyarınca 10 yıl) boyunca hukuki uyuşmazlıkların ispatı amacıyla sınırlı erişimle saklanır." => "Personal data retained for the duration prescribed by applicable legislation or required by the processing purpose are erased, destroyed or anonymised in the first periodic destruction cycle (conducted every 6 months) following expiry:\n\n• Subscription, Order, Proforma and Invoice Records: 5 years under Tax Procedure Law No. 213 Art. 253; 10 years under Turkish Commercial Code No. 6102 Art. 82.\n• Distance Sales and Consumer Transaction Records: 3 years under Consumer Protection Law No. 6502 and Distance Contracts Regulation Art. 20.\n• Commercial Electronic Messaging Consent and Opt-Out Records: 3 years from withdrawal or expiry of consent under Law No. 6563 and related regulations.\n• Internet Traffic and Cybersecurity Logs: 2 years pursuant to Law No. 5651.\n• Membership and Institutional Verification Records: Retained for the duration of membership and for the general statutory limitation period thereafter (10 years under Turkish Code of Obligations No. 6098 Art. 146) under restricted access solely for evidentiary purposes in legal disputes.",

        // 12. kurumsal-cerez-politikasi
        "Çerez (Cookie) ve Yerel Depolama Aydınlatma Politikası" => "Cookie and Local Storage Disclosure Policy",
        "Kişisel Verileri Koruma Kurumu 'Çerez Uygulamaları Hakkında Rehber' ilkelerine tam uyumlu gerçek çerez ve tarayıcı depolama envanteri." => "Transparent cookie and browser storage inventory fully aligned with the Turkish Personal Data Protection Authority's 'Guidelines on Cookie Practices'.",
        "KVKK KURUMU ÇEREZ UYGULAMALARI REHBERİ UYUMU" => "COMPLIANCE WITH KVKK AUTHORITY COOKIE GUIDELINES",
        "1. Çerez Türleri, Hukuki Sebepler ve Gerçek Çerez Envanteri" => "1. Cookie Categories, Legal Grounds and Actual Cookie Inventory",
        "msidergisi.com ve savunmahaber.com dijital platformlarında, Kişisel Verileri Koruma Kurumu tarafından yayımlanan 'Çerez Uygulamaları Hakkında Rehber' uyarınca 'kesinlikle gerekli (zorunlu) çerezler' dışındaki analitik ve sponsorluk ölçümleme teknolojileri varsayılan olarak kapalı tutulmakta (opt-in) ve yalnızca açık rızanız hâlinde etkinleştirilmektedir.\n\n2.1. Kesinlikle Gerekli (Zorunlu) Çerezler — KVKK m.5/2-(c) ve (f) (Açık Rıza Gerektirmez):\n• msi_session (Birinci Taraf HTTP Çerezi — HttpOnly, SameSite=Lax, Secure): Kullanıcı oturumunun doğrulanması, abone/kamu yetkilerinin korunması ve güvenli yönetim erişimi için kullanılır. Saklama süresi: 24 saat (veya oturum kapatılana kadar).\n\n2.2. İşlevsel ve Tercih Kayıtları — Kullanıcı Talebiyle KVKK m.5/2-(f):\n• msi_lang (Birinci Taraf Çerez): Kullanıcının açıkça seçtiği yayın dilini (tr / en) hatırlar. Süre: 1 yıl.\n• msi_theme (Tarayıcı localStorage): Açık / koyu okuma teması tercihini cihazda saklar.\n• msi_cookie_consent (Tarayıcı localStorage): Çerez yönetim panelindeki onay/ret tercihlerinizi ve zaman damgasını saklar. Süre: 12 ay.\n• msi_read_notifications & msi_dismissed_announcement (Tarayıcı localStorage): Bildirim ve duyuru çubuğu okundu bilgisini yalnızca cihazınızda tutar.\n\n2.3. Performans ve Okuma Analitiği — KVKK m.5/1 Açık Rıza (İsteğe Bağlı / Opt-In):\n• msi_session_id & Birinci Taraf Okuma Telemetrisi: Üçüncü taraf reklam ağlarına veri aktarmaksızın, makale okuma süresi ve kaydırma derinliği istatistiklerini birinci taraf sunucumuzda anonim oturum koduyla ölçer. Yalnızca 'Analitik' izni verildiğinde çalışır.\n\n2.4. Kurumsal Sponsorluk ve Reklam Ölçümleme — KVKK m.5/1 Açık Rıza (İsteğe Bağlı / Opt-In):\n• Birinci Taraf Sponsorluk Gösterim/Tıklama Sayacı: Sayfa içi savunma sanayii kurumsal vitrin alanlarının gösterim ve tıklama performansını ölçer. Yalnızca 'Sponsorluk Ölçümü' izni verildiğinde çalışır." => "On msidergisi.com and savunmahaber.com, in accordance with the Personal Data Protection Authority's 'Guidelines on Cookie Practices', all analytics and sponsorship measurement technologies other than strictly necessary cookies are disabled by default (opt-in) and activated solely upon your explicit consent.\n\n2.1. Strictly Necessary Cookies — KVKK Art. 5/2-(c) & (f) (No Explicit Consent Required):\n• msi_session (First-Party HTTP Cookie — HttpOnly, SameSite=Lax, Secure): Used to authenticate user sessions, protect subscriber/public-sector entitlements and secure administrative access. Retention: 24 hours (or until logout).\n\n2.2. Functional and Preference Storage — User-Requested under KVKK Art. 5/2-(f):\n• msi_lang (First-Party Cookie): Remembers your explicitly selected publication language (tr / en). Retention: 1 year.\n• msi_theme (Browser localStorage): Stores your light/dark reading theme preference on your device.\n• msi_cookie_consent (Browser localStorage): Records your consent/rejection preferences and timestamp from the cookie manager. Retention: 12 months.\n• msi_read_notifications & msi_dismissed_announcement (Browser localStorage): Keeps notification read status and announcement bar dismissal locally on your device.\n\n2.3. Performance and Reading Analytics — KVKK Art. 5/1 Explicit Consent (Optional / Opt-In):\n• msi_session_id & First-Party Reading Telemetry: Measures reading duration, scroll depth and video watch progress on our first-party server under an anonymised session identifier without transferring data to third-party advertising networks. Active only when 'Analytics' consent is granted.\n\n2.4. Corporate Sponsorship and Ad Measurement — KVKK Art. 5/1 Explicit Consent (Optional / Opt-In):\n• First-Party Sponsorship Impression/Click Counter: Measures impression and click performance for on-page defence industry showcase placements. Active only when 'Sponsorship Measurement' consent is granted.",
        "TERCİH YÖNETİMİ VE TARAYICI AYARLARI" => "PREFERENCE MANAGEMENT AND BROWSER CONTROLS",
        "2. Çerez Tercihlerini Değiştirme ve Geri Alma Hakkı" => "2. Right to Modify and Withdraw Cookie Consent",
        "Sayfanın alt bilgi (footer) alanında yer alan 'Çerez Tercihleri' bağlantısına dilediğiniz anda tıklayarak zorunlu olmayan çerezlere verdiğiniz açık rızayı geri alabilir veya tercihlerinizi güncelleyebilirsiniz.\n\nAyrıca tarayıcınızın ayarları üzerinden çerezleri her zaman silebilir veya engelleyebilirsiniz:\n• Google Chrome: Ayarlar > Gizlilik ve Güvenlik > Üçüncü Taraf Çerezleri / Site Verileri\n• Mozilla Firefox: Ayarlar > Gizlilik ve Güvenlik > Çerezler ve Site Verileri\n• Apple Safari: Ayarlar / Tercihler > Gizlilik\n• Microsoft Edge: Ayarlar > Çerezler ve Site İzinleri" => "You may withdraw your explicit consent for non-essential cookies or update your preferences at any time by clicking the 'Cookie Preferences' link in the website footer.\n\nYou can also delete or block cookies at any time through your browser settings:\n• Google Chrome: Settings > Privacy and Security > Third-Party Cookies / Site Data\n• Mozilla Firefox: Settings > Privacy & Security > Cookies and Site Data\n• Apple Safari: Settings / Preferences > Privacy\n• Microsoft Edge: Settings > Cookies and Site Permissions",

        // 13. kurumsal-mesafeli-satis-sozlesmesi
        "Ön Bilgilendirme Formu ve Mesafeli Satış / Abonelik Sözleşmesi Koşulları" => "Pre-Contractual Information Form and Distance Sales / Subscription Agreement Terms",
        "6502 sayılı Tüketicinin Korunması Hakkında Kanun (7529 s. Kanun değişiklikleri), RG: 29188 Mesafeli Sözleşmeler Yönetmeliği ve RG: 29246 Abonelik Sözleşmeleri Yönetmeliği hükümleri." => "Provisions under Consumer Protection Law No. 6502 (as amended by Law No. 7529), Distance Contracts Regulation (OG: 29188) and Subscription Contracts Regulation (OG: 29246).",
        "6502 SAYILI TKHK M.48 & M.52 ÖN BİLGİLENDİRME" => "CONSUMER PROTECTION LAW NO. 6502 ARTS. 48 & 52 DISCLOSURE",
        "1. Satıcı / Sağlayıcı Bilgileri ve Sözleşmenin Konusu" => "1. Seller / Provider Details and Subject Matter of the Contract",
        "1.1. Satıcı / Sağlayıcı Bilgileri:\n• Unvan: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\n• Adres: Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — TÜRKİYE\n• Telefon: +90 312 225 41 73 • Faks: +90 312 225 41 74\n• E-Posta (Müşteri Hizmetleri ve Şikâyet): info@savunmahaber.com • r.turgut@savunmahaber.com\n\n1.2. Konu ve Kapsam:\nİşbu Ön Bilgilendirme ve Mesafeli Satış / Abonelik Koşulları; 6502 sayılı Tüketicinin Korunması Hakkında Kanun ('TKHK'), 27.11.2014 tarihli ve 29188 sayılı Resmî Gazete'de yayımlanan Mesafeli Sözleşmeler Yönetmeliği (ve güncel değişiklikleri) ile 24.01.2015 tarihli ve 29246 sayılı Resmî Gazete'de yayımlanan Abonelik Sözleşmeleri Yönetmeliği uyarınca, Alıcı/Abone'nin msidergisi.com üzerinden elektronik ortamda sipariş verdiği basılı süreli yayın aboneliği, dijital E-Dergi aboneliği, tekil dijital makale erişimi veya basılı referans kitap/rapor satışına ilişkin hak ve yükümlülükleri düzenler." => "1.1. Seller / Provider Details:\n• Legal Name: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.\n• Address: Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — TÜRKİYE\n• Telephone: +90 312 225 41 73 • Fax: +90 312 225 41 74\n• Email (Customer Service & Complaints): info@savunmahaber.com • r.turgut@savunmahaber.com\n\n1.2. Subject and Scope:\nThese Pre-Contractual Information and Distance Sales / Subscription Terms govern the rights and obligations of the Buyer/Subscriber and the Seller regarding print periodical subscriptions, digital E-Magazine subscriptions, single locked-article access or printed reference books/reports ordered electronically via msidergisi.com, pursuant to Consumer Protection Law No. 6502, the Distance Contracts Regulation (OG: 27.11.2014 / 29188, as amended) and the Subscription Contracts Regulation (OG: 24.01.2015 / 29246).",
        "FİYATLANDIRMA, ÖDEME VE İFA ESASLARI" => "PRICING, PAYMENT AND FULFILMENT PRINCIPLES",
        "2. Bedel, Ödeme Güvenliği ve Teslimat Süreçleri" => "2. Price, Payment Security and Delivery Procedures",
        "• Tüm yayın ve abonelik planlarının Türk Lirası (TL) cinsinden vergiler dâhil toplam satış bedeli '/abonelik-ve-satis' sayfasında ve sipariş onay ekranında açıkça gösterilir; sipariş ekranında belirtilmeyen hiçbir ek masraf talep edilmez.\n• Ödemeler BDDK lisanslı PayTR sanal POS altyapısı üzerinden kredi/banka kartıyla veya kurumsal fatura/proforma kapsamında banka havalesi/EFT yoluyla gerçekleştirilir.\n• Dijital İçerik İfası: Dijital E-Dergi abonelikleri ve tekil kilitli makale erişimleri, ödemenin onaylanmasını takiben elektronik ortamda derhal aktive edilir.\n• Fiziksel Yayın Teslimatı: Basılı kitap, rapor ve basılı dergi sayıları; stokta bulunması hâlinde en geç 30 (otuz) günlük yasal süreyi aşmamak kaydıyla Alıcı'nın bildirdiği teslimat adresine anlaşmalı kargo/posta aracılığıyla gönderilir." => "• The total tax-inclusive price in Turkish Lira (TRY) for all publications and subscription plans is displayed transparently on '/abonelik-ve-satis?lang=en' and on the order confirmation screen; no undisclosed fees are charged.\n• Payments are processed by credit/debit card through the BRSA-licensed PayTR virtual POS infrastructure or via corporate bank transfer/EFT against a proforma invoice.\n• Digital Content Fulfilment: Digital E-Magazine subscriptions and single-article access entitlements are activated electronically immediately upon payment confirmation.\n• Physical Publication Delivery: Printed books, reports and print magazine issues in stock are dispatched via contracted courier/postal service to the delivery address provided by the Buyer within the statutory maximum of 30 (thirty) days.",
        "RG: 29188 M.9 & M.15 CAYMA HAKKI VE YASAL İSTİSNALAR" => "REGULATION OG: 29188 ARTS. 9 & 15 RIGHT OF WITHDRAWAL & EXEMPTIONS",
        "3. Cayma Hakkı, Süreli Yayın / Dijital İçerik İstisnaları ve Abonelik Feshi" => "3. Right of Withdrawal, Periodical / Digital Exemptions and Subscription Termination",
        "3.1. Genel Cayma Hakkı (14 Gün):\nTüketici sıfatını haiz Alıcı; fiziksel kitap ve rapor alımlarında malın kendisine veya gösterdiği üçüncü kişiye teslim edildiği tarihten itibaren, abonelik/hizmet sözleşmelerinde ise sözleşmenin kurulduğu tarihten itibaren 14 (ondört) gün içinde herhangi bir gerekçe göstermeksizin ve cezai şart ödemeksizin cayma hakkını kullanabilir. Cayma bildiriminin 'info@savunmahaber.com' adresine yazılı/kalıcı veri saklayıcısı ile yöneltilmesi yeterlidir; bedel iadesi bildirimden itibaren 14 gün içinde ödeme aracına uygun şekilde tek seferde yapılır.\n\n3.2. Mesafeli Sözleşmeler Yönetmeliği m.15/1 Uyarınca Cayma Hakkının Kullanılamayacağı İstisnalar:\n• m.15/1-(f): Abonelik sözleşmesi kapsamında sağlananlar dışında, gazete ve dergi gibi süreli yayınların tekil teslimine ilişkin sözleşmelerde cayma hakkı kullanılamaz.\n• m.15/1-(ğ): Elektronik ortamda anında ifa edilen hizmetler veya tüketiciye anında teslim edilen gayrimaddi mallara (ödeme anında erişime açılan dijital E-Dergi PDF indirmeleri ve tekil dijital makaleler) ilişkin sözleşmelerde cayma hakkı kullanılamaz.\n\n3.3. Abonelik Sözleşmesinin Feshi (Abonelik Sözleşmeleri Yönetmeliği m.22–25):\nTüketici, belirsiz süreli veya süresi 1 yıl ve daha uzun olan abonelik sözleşmesini herhangi bir gerekçe göstermeksizin ve cezai şart ödemeksizin istediği zaman yazılı veya kalıcı veri saklayıcısı ile ('info@savunmahaber.com') bildirimde bulunarak feshedebilir. Fesih bildiriminin ulaşmasından itibaren en geç 7 gün içinde fesih işlemi yerine getirilir ve aboneliğin kalan süresine/ifa edilmeyen sayılarına tekabül eden tutar 14 gün içinde tüketiciye iade edilir." => "3.1. General Right of Withdrawal (14 Days):\nA Buyer acting as a consumer may exercise the right of withdrawal without giving any reason and without penalty within 14 (fourteen) days from the date of physical delivery (for books and reports) or from contract formation (for subscription/service contracts). Written notice sent via a durable medium to 'info@savunmahaber.com' suffices; a full refund is issued in a single payment via the original payment method within 14 days of receipt of the withdrawal notice.\n\n3.2. Statutory Exemptions under Distance Contracts Regulation Art. 15/1:\n• Art. 15/1-(f): The right of withdrawal does not apply to contracts for the single delivery of newspapers, magazines and other periodicals, save for subscription contracts.\n• Art. 15/1-(ğ): The right of withdrawal does not apply to services performed instantaneously in an electronic environment or intangible goods delivered immediately to the consumer (digital E-Magazine PDF downloads and single digital articles unlocked upon payment).\n\n3.3. Subscription Termination (Subscription Contracts Regulation Arts. 22–25):\nThe consumer may terminate an indefinite-term subscription or a subscription of 1 year or longer at any time without justification or penalty by notifying 'info@savunmahaber.com' in writing or via a durable medium. Termination is effected within 7 days of receipt of the notice, and any amount corresponding to the remaining period or undelivered issues is refunded within 14 days.",
        "UYUŞMAZLIK ÇÖZÜMÜ VE YETKİLİ MERCİLER" => "DISPUTE RESOLUTION AND COMPETENT AUTHORITIES",
        "4. Tüketici Hakem Heyetleri ve Yetkili Mahkemeler" => "4. Consumer Arbitration Boards and Competent Courts",
        "6502 sayılı TKHK m.68 kapsamında tüketici işlemlerinden doğabilecek uyuşmazlıklarda, Ticaret Bakanlığı tarafından her yıl Resmî Gazete'de yayımlanan Tebliğ ile belirlenen güncel parasal sınırlar dâhilinde Alıcı'nın yerleşim yerinin bulunduğu veya tüketici işleminin yapıldığı yerdeki İl veya İlçe Tüketici Hakem Heyetleri; söz konusu parasal sınırların üzerindeki uyuşmazlıklarda ise Tüketici Mahkemeleri yetkilidir.\n\n6102 sayılı Türk Ticaret Kanunu kapsamında tacir veya kamu tüzel kişisi sıfatıyla gerçekleştirilen kurumsal abonelik, toplu lisans ve reklam/sponsorluk sözleşmelerinden doğan uyuşmazlıklarda Ankara Mahkemeleri ve İcra Müdürlükleri yetkilidir." => "For disputes arising from consumer transactions under Law No. 6502 Art. 68, Provincial or District Consumer Arbitration Boards at the Buyer's place of residence or the place of transaction have jurisdiction up to the monetary thresholds published annually by the Ministry of Trade in the Official Gazette; for disputes exceeding those thresholds, Consumer Courts have jurisdiction.\n\nFor disputes arising from corporate subscriptions, bulk licences and advertising/sponsorship contracts entered into by merchants or public legal entities under Turkish Commercial Code No. 6102, the Courts and Enforcement Offices of Ankara have exclusive jurisdiction.",

        // 14. kurumsal-iptal-ve-iade-kosullari
        "Teslimat, İptal, Fesih ve İade Koşulları" => "Delivery, Cancellation, Termination and Refund Policy",
        "Basılı yayınlar, kurumsal/bireysel abonelikler ve dijital E-Dergi içeriklerine ilişkin teslimat, iptal ve bedel iadesi süreçleri." => "Delivery, cancellation and refund procedures for print publications, corporate/individual subscriptions and digital E-Magazine content.",
        "TESLİMAT, İPTAL VE İADE REHBERİ" => "DELIVERY, CANCELLATION AND REFUND GUIDE",
        "Abonelik ve Yayın Siparişlerinde İptal / İade Esasları" => "Cancellation and Refund Principles for Subscription and Publication Orders",
        "1. Sipariş İptali:\nHenüz kargoya verilmemiş basılı kitap/rapor siparişlerinizi veya henüz dijital erişimi/PDF üretimi başlatılmamış proforma siparişlerinizi 'info@savunmahaber.com' adresine sipariş numaranızı ileterek kesintisiz iptal edebilirsiniz.\n\n2. Basılı Yayınlarda İade ve Ayıplı Mal Hükümleri (6502 s. TKHK m.8–11):\nKargo teslimatı sırasında hasar görmüş veya basım hatası bulunan yayınlar için teslim tarihinden itibaren yasal süreler içinde ücretsiz değişim veya tam bedel iadesi hakkınız mevcuttur. 14 günlük cayma hakkı kapsamındaki fiziksel kitap iadelerinde ürünün yeniden satılabilir özelliğini kaybetmemiş olması gerekir.\n\n3. Dijital E-Dergi ve Tekil Makale Erişimleri:\nMesafeli Sözleşmeler Yönetmeliği m.15/1-(ğ) hükmü uyarınca elektronik ortamda anında ifa edilen dijital içeriklerde (kişiye özel DRM filigranlı PDF indirme işlemi gerçekleşmiş E-Dergi sayıları ve kilidi açılmış tekil makaleler) ifa başladıktan sonra cayma hakkı kapsamında iade yapılamaz.\n\n4. Yıllık Abonelik Feshi ve Kıstelyevm İade:\nAbonelik Sözleşmeleri Yönetmeliği m.23 ve m.25 uyarınca yıllık abonelik fesih talepleriniz en geç 7 gün içinde işleme alınır; halihazırda teslim edilmiş/ifa edilmiş sayılar düşüldükten sonra kalan abonelik bedeli 14 gün içinde ödeme yönteminizle aynı kanaldan iade edilir." => "1. Order Cancellation:\nYou may cancel print book/report orders that have not yet been dispatched, or proforma orders whose digital access/PDF generation has not yet commenced, free of charge by quoting your order number to 'info@savunmahaber.com'.\n\n2. Print Publication Returns & Defective Goods (Law No. 6502 Arts. 8–11):\nFor publications damaged in transit or containing printing defects, you are entitled to free replacement or a full refund within statutory deadlines from delivery. Physical books returned under the 14-day right of withdrawal must remain in resalable condition.\n\n3. Digital E-Magazine and Single-Article Access:\nPursuant to Distance Contracts Regulation Art. 15/1-(ğ), digital content performed instantaneously in an electronic environment (personalised DRM-watermarked E-Magazine PDF downloads and unlocked single articles) is non-refundable once fulfilment has begun.\n\n4. Annual Subscription Termination & Pro-Rata Refund:\nUnder Subscription Contracts Regulation Arts. 23 and 25, annual subscription termination requests are processed within 7 days; after deducting issues already delivered or fulfilled, the remaining subscription balance is refunded within 14 days through the original payment channel.",

        // 15. kurumsal-ticari-ileti-aydinlatma
        "6563 Sayılı ETK Kapsamında Ticari Elektronik İleti Bilgilendirme ve Onay Metni" => "Commercial Electronic Messaging Disclosure and Consent Notice under Law No. 6563",
        "6563 sayılı Elektronik Ticaretin Düzenlenmesi Hakkında Kanun, 7416 sayılı Kanun ve Ticari İletişim ve Ticari Elektronik İletiler Hakkında Yönetmelik (İYS) metni." => "Disclosure notice pursuant to Electronic Commerce Law No. 6563, Law No. 7416 and the Regulation on Commercial Communications and Commercial Electronic Messages (İYS).",
        "6563 SAYILI KANUN & İLETİ YÖNETİM SİSTEMİ (İYS)" => "LAW NO. 6563 & MESSAGE MANAGEMENT SYSTEM (İYS)",
        "Ticari Elektronik İleti Onayı ve Ret Hakkı" => "Commercial Electronic Messaging Consent and Right to Opt Out",
        "İşbu metin, 6563 sayılı Elektronik Ticaretin Düzenlenmesi Hakkında Kanun ('ETK'), Ticari İletişim ve Ticari Elektronik İletiler Hakkında Yönetmelik (RG: 15.07.2015 / 29417 ve 04.01.2020 / 30998 değişiklikleri) uyarınca UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. ('MSI Dergisi') tarafından bilgilendirme amacıyla hazırlanmıştır.\n\n1. Onayın Kapsamı ve İsteğe Bağlılık:\nKayıt formunda yer alan 'Ticari elektronik ileti ve bülten' kutucuğunu işaretlemeniz tamamen özgür iradenize bağlıdır; bu onayı vermemeniz MSI Dergisi okuyucu üyeliğinize veya abonelik hizmetlerinden yararlanmanıza hiçbir şekilde engel teşkil etmez.\n\n2. Gönderim Amaçları:\nOnay vermeniz hâlinde paylaşmış olduğunuz e-posta adresinize; MSI Dergisi ve MSI TDR yeni sayı duyuruları, özel savunma sanayii dosyaları, sektörel fuar/konferans bültenleri ve abonelik kampanya bilgilendirmeleri gönderilebilecektir.\n\n3. Her Zaman Ücretsiz Ret (Çıkış) Hakkı:\n6563 sayılı Kanun m.8 uyarınca dilediğiniz zaman hiçbir gerekçe göstermeksizin ticari elektronik ileti almayı reddedebilirsiniz. Gönderilen her e-bültendeki 'Bülten aboneliğinden ayrıl' bağlantısını kullanarak, İleti Yönetim Sistemi (iys.org.tr) üzerinden veya 'info@savunmahaber.com' adresine bildirimde bulunarak ret hakkınızı ücretsiz kullanabilirsiniz; talebiniz ulaştığı tarihten itibaren 3 (üç) iş günü içinde ticari elektronik ileti gönderimi durdurulur.\n\nNot: Mevcut aboneliğinizin yenilenmesi, fatura/sipariş durumu ve hesap güvenliğine ilişkin zorunlu bilgilendirme iletileri Yönetmelik m.6 kapsamında önceden onay şartına tabi değildir." => "This notice has been prepared for informational purposes by UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. ('MSI Review') pursuant to Law No. 6563 on the Regulation of Electronic Commerce ('ETK') and the Regulation on Commercial Communications and Commercial Electronic Messages (OG: 15.07.2015 / 29417, as amended OG: 04.01.2020 / 30998).\n\n1. Scope and Optional Nature of Consent:\nTicking the 'Commercial electronic messages and briefings' box on the registration form is entirely voluntary; declining this consent does not affect your MSI Review reader membership or access to subscription services.\n\n2. Messaging Purposes:\nWhere consent is granted, your email address may receive new issue announcements for MSI Review and MSI TDR, special defence industry dossiers, sector exhibition/conference briefings and subscription campaign updates.\n\n3. Unconditional Free Right to Opt Out:\nUnder Law No. 6563 Art. 8, you may opt out of receiving commercial electronic messages at any time without giving any reason. You may exercise this right free of charge via the 'Unsubscribe' link in every briefing email, through the Message Management System (iys.org.tr), or by emailing 'info@savunmahaber.com'; commercial messaging ceases within 3 (three) business days of receipt of your request.\n\nNote: Essential transactional notifications regarding renewal of your active subscription, billing/order status and account security are exempt from prior consent under Regulation Art. 6.",

        // 16. kurumsal-kullanim-kosullari
        "Platform Kullanım Koşulları ve Okuyucu Üyelik Şartları" => "Platform Terms of Use and Reader Membership Conditions",
        "msidergisi.com ve savunmahaber.com dijital yayın platformunun kullanım şartları, üyelik kuralları ve yasal sorumluluk sınırları." => "Terms of use, membership rules and legal liability boundaries for the msidergisi.com and savunmahaber.com digital publishing platform.",
        "DİJİTAL PLATFORM KULLANIM ŞARTLARI" => "DIGITAL PLATFORM TERMS OF USE",
        "1. Hizmet Tanımı, Üyelik ve Kabul Edilebilir Kullanım" => "1. Service Description, Membership and Acceptable Use",
        "msidergisi.com ve savunmahaber.com alan adları üzerinden sunulan dijital yayıncılık platformunu ziyaret eden veya üye olan tüm kullanıcılar aşağıdaki şartları okumuş ve kabul etmiş sayılır:\n\n• Doğru Beyan Yükümlülüğü: Üyelik ve kamu/kurumsal abonelik doğrulama formlarında beyan edilen ad-soyad, kurum ve görev bilgilerinin gerçeğe uygun olması zorunludur. Yanıltıcı kurum beyanı tespit edilen hesapların kurumsal yetkileri askıya alınır.\n• Bireysel Hesap Güvenliği: Her abonelik hesabı (çok kullanıcılı kurumsal kütüphane lisansları hariç) tekil kullanıcıya özeldir; şifre ve oturum bilgilerinin üçüncü kişilerle paylaşılması yasaktır.\n• Altyapı Güvenliği: Platforma yönelik otomatik veri kazıma (scraping), bot saldırısı, güvenlik zafiyeti taraması veya DRM/paywall koruma mekanizmalarını aşmaya yönelik her türlü girişim merkezi güvenlik duvarı (WAF) tarafından kaydedilir ve 5237 sayılı Türk Ceza Kanunu m.243–244 bilişim suçları kapsamında adli mercilere bildirilir.\n• Açık Kaynak Bilgilendirme Sınırı: Platformda yer alan savunma, havacılık ve güvenlik analizleri kamuya açık kaynaklar, resmî kurum açıklamaları ve uzman değerlendirmelerine dayanmaktadır; yatırım, ihale veya resmî teknik şartname taahhüdü niteliği taşımaz.\n• Uygulanacak Hukuk: İşbu koşulların yorumlanmasında Türkiye Cumhuriyeti kanunları uygulanır." => "All visitors and registered members accessing the digital publishing platform at msidergisi.com and savunmahaber.com are deemed to have read and accepted the following terms:\n\n• Duty of Accurate Representation: Full name, organisation and role information declared in registration and public/corporate verification forms must be truthful. Accounts found to contain misleading institutional declarations will have their institutional privileges suspended.\n• Individual Account Security: Except for multi-seat institutional library licences, each subscription account is personal to a single user; sharing passwords or session credentials with third parties is prohibited.\n• Infrastructure Security: Automated data scraping, bot attacks, vulnerability scanning or any attempt to circumvent DRM/paywall protections are logged by our central Web Application Firewall (WAF) and reported to judicial authorities under cybercrime provisions of Turkish Penal Code No. 5237 Arts. 243–244.\n• Open-Source Informational Scope: Defence, aerospace and security analyses published on the platform rely on open sources, official institutional statements and expert assessments; they do not constitute investment advice, tender commitments or official technical specifications.\n• Governing Law: These terms are governed by and construed in accordance with the laws of the Republic of Türkiye.",

        // 17. kurumsal-telif-ve-etik
        "Telif Hakları (5846 s. FSEK), Yapay Zekâ Hak Saklılığı ve Yayın Etiği İlkeleri" => "Copyright (Law No. 5846), AI Text & Data Mining Reservation and Editorial Ethics",
        "5846 sayılı Fikir ve Sanat Eserleri Kanunu, dijital DRM filigran koruması, yapay zekâ eğitim kısıtlaması ve basın yayın etiği ilkelerimiz." => "Intellectual property protection under Law No. 5846, digital DRM watermark security, AI training restrictions and editorial ethics principles.",
        "5846 SAYILI FİKİR VE SANAT ESERLERİ KANUNU" => "LAW NO. 5846 ON INTELLECTUAL AND ARTISTIC WORKS",
        "1. Fikrî Mülkiyet, İktibas Kuralları ve Yapay Zekâ Hak Saklılığı" => "1. Intellectual Property, Quotation Rules and AI Rights Reservation",
        "1.1. Eser Sahipliği:\nmsidergisi.com, savunmahaber.com ve MSI Dergisi / MSI TDR basılı ve dijital sayılarında yer alan tüm özgün haber, makale, söyleşi, çeviri, grafik, mizanpaj, logo ve fotoğrafların mali ve manevi hakları 5846 sayılı Fikir ve Sanat Eserleri Kanunu ('FSEK') kapsamında UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.'ye veya ilgili lisans veren eser sahiplerine (Remove Before Photos dâhil) aittir.\n\n1.2. İktibas ve Kaynak Gösterme (FSEK m.35–36):\nBasın özetleri ve akademik atıf sınırlarını aşacak şekilde içeriklerin tamamının veya önemli bir bölümünün yazılı izin alınmaksızın kopyalanması, çoğaltılması, yeniden yayımlanması veya ticari amaçla dağıtılması yasaktır. Hukuka uygun kısa alıntılarda 'Kaynak: MSI Dergisi (msidergisi.com)' ibaresi ve aktif bağlantı verilmesi zorunludur.\n\n1.3. Dijital Hak Yönetimi (DRM) ve Filigran:\nAbone kütüphanesinden indirilen E-Dergi PDF nüshaları; abonenin adı, kurumu, zaman damgası ve kriptografik doğrulama özetiyle kişiye özel filigranlanmaktadır. Yetkisiz paylaşım veya çoğaltma FSEK m.71 ve devamı uyarınca hukuki ve cezai sorumluluk doğurur.\n\n1.4. Yapay Zekâ Eğitimi ve Metin/Veri Madenciliği Beyanı:\nArama motoru ve yapay zekâ yanıt sistemlerinin (arama dizinleme ve kaynaklı özet atfı) '/llms.txt' ve '/robots.txt' kurallarına uygun olarak kaynak ve bağlantı göstermesi serbesttir; buna karşılık arşivimizin ticari büyük dil modeli (LLM) ham eğitim veri seti olarak toplu kopyalanması UMSA Prodüksiyon'un yazılı iznine tabidir." => "1.1. Copyright Ownership:\nAll economic and moral rights in original news reports, articles, interviews, translations, graphics, layouts, logos and photographs published on msidergisi.com, savunmahaber.com and in print/digital issues of MSI Review / MSI TDR belong to UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. or respective licensors (including Remove Before Photos) under Turkish Law No. 5846 ('FSEK').\n\n1.2. Quotation and Attribution (FSEK Arts. 35–36):\nCopying, reproducing, republishing or commercially distributing full works or substantial portions beyond press-summary and academic citation limits without prior written permission is prohibited. Lawful brief excerpts must include the attribution 'Source: MSI Review (msidergisi.com)' and an active link.\n\n1.3. Digital Rights Management (DRM) and Watermarking:\nE-Magazine PDF editions downloaded from the subscriber library are individually watermarked with the subscriber's name, organisation, timestamp and cryptographic verification hash. Unauthorised sharing or redistribution entails civil and criminal liability under FSEK Art. 71 et seq.\n\n1.4. AI Training and Text & Data Mining Reservation:\nSearch engines and AI answer systems may index and cite attributed summaries in accordance with '/llms.txt' and '/robots.txt'; however, bulk copying of our archive as raw training datasets for commercial large language models (LLMs) requires prior written authorisation from UMSA Production.",
        "BASIN AHLAK ESASLARI & EDİTORYAL STANDARTLAR" => "PRESS ETHICS PRINCIPLES & EDITORIAL STANDARDS",
        "2. MSI Dergisi Yayın Etiği İlkeleri" => "2. MSI Review Editorial Ethics Principles",
        "• Birinci Kaynaktan Doğrulama: Savunma ve havacılık haberlerimizde resmî tedarik makamları (MSB, SSB, TSK kuvvet komutanlıkları), yüklenici kuruluşların resmî açıklamaları ve sahadaki uzman muhabirlerimizin teyitli bulguları esas alınır.\n• Millî Güvenlik ve Gizlilik Hassasiyeti: Kamuya açıklanmamış veya gizlilik dereceli hiçbir askerî/teknik veri yayımlanmaz; yalnızca açık kaynak istihbaratı ve resmî izinli saha verileri paylaşılır.\n• Editoryal Bağımsızlık ve Şeffaflık: Haber ve analiz içerikleri reklam veya sponsorluk ilişkilerinden tamamen bağımsızdır. Ticari iş birliği kapsamındaki tanıtım metinleri 'SPONSORLU İÇERİK / ADVERTORIAL' etiketiyle açıkça ayrıştırılır.\n• Hızlı Düzeltme Taahhüdü: Maddi hata tespit edilen içerikler 5187 sayılı Basın Kanunu ve gazetecilik etiği gereğince gecikmeksizin düzeltilir ve şeffaf biçimde güncellenir." => "• Primary-Source Verification: Our defence and aerospace reporting relies on official statements from procurement authorities (MoND, SSB, Turkish Armed Forces service commands), contractor disclosures and corroborated findings by our specialist field correspondents.\n• National Security and Classification Sensitivity: No classified or non-public military/technical data are ever published; only open-source intelligence and officially cleared field reporting are shared.\n• Editorial Independence and Transparency: News and analytical reporting are strictly independent of advertising and sponsorship relationships. Commercial partner features are clearly separated with 'SPONSORED CONTENT / ADVERTORIAL' labels.\n• Prompt Correction Commitment: Any factual inaccuracy identified is corrected without delay and transparently updated in accordance with Press Law No. 5187 and journalistic ethics.",

        // 18. kurumsal-duzeltme-ve-cevap-hakki
        "Düzeltme ve Cevap Hakkı (5187 s. Kanun m.14) ile İçerik Bildirim Usulü" => "Right of Correction and Reply (Law No. 5187 Art. 14) and Content Notice Procedure",
        "5187 sayılı Basın Kanunu m.14 (7418 sayılı Kanun internet haber sitesi hükümleri) ve 5651 sayılı Kanun m.9 uyarınca düzeltme, cevap ve bildirim prosedürü." => "Correction, reply and notice-and-takedown procedure pursuant to Press Law No. 5187 Art. 14 (online news portal provisions under Law No. 7418) and Law No. 5651 Art. 9.",
        "5187 SAYILI BASIN KANUNU M.14 & 7418 SAYILI KANUN" => "PRESS LAW NO. 5187 ART. 14 & LAW NO. 7418",
        "İnternet Haber Sitesinde Düzeltme, Cevap ve Hukuki Bildirim Prosedürü" => "Online News Portal Correction, Reply and Legal Notice Procedure",
        "1. Düzeltme ve Cevap Hakkı (5187 sayılı Basın Kanunu m.14 — 7418 sayılı Kanun Değişikliği):\nİnternet haber sitemizde yayımlanan süreli yayın içeriklerinde şeref ve haysiyetini ihlal edici veya kişilerle ilgili gerçeğe aykırı yayım yapılması hâlinde, zarar gören kişinin yayım tarihinden itibaren 2 (iki) ay içinde göndereceği, suç unsuru içermeyen ve üçüncü kişilerin hukuken korunan menfaatlerine aykırı olmayan düzeltme ve cevap yazısı; Sorumlu Yazı İşleri Müdürü tarafından hiçbir düzeltme ve ekleme yapılmaksızın, yazının alındığı tarihten itibaren en geç 1 (bir) gün içinde, ilgili yayının yer aldığı sayfa ve sütunlarda, URL bağlantısı sağlanmak suretiyle, aynı puntolarla ve aynı şekilde 1 (bir) hafta süreyle yayımlanır.\n\n2. 5651 Sayılı Kanun Kapsamında İçerik Bildirimi (Uyar-Kaldır):\n5651 sayılı Kanun kapsamında kişilik haklarının veya 5846 sayılı FSEK kapsamında fikrî mülkiyet haklarının ihlal edildiği iddiasına dayalı bildirimler; ilgili URL adresi, ihlal gerekçesi ve kimlik/yetki belgeleriyle birlikte Sorumlu Yazı İşleri Müdürlüğümüze iletilebilir.\n\n3. Başvuru Kanalları:\n• Sorumlu Yazı İşleri Müdürü: Dr. K. Burak CODUR — b.codur@savunmahaber.com\n• Kurumsal Hukuk / İletişim E-Posta: info@savunmahaber.com\n• Yazılı Tebligat Adresi: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti., Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — Tel: +90 312 225 41 73" => "1. Right of Correction and Reply (Press Law No. 5187 Art. 14 — as Amended by Law No. 7418):\nWhere a periodical publication on our online news portal infringes a person's honour and dignity or contains factually inaccurate statements about an individual, a correction and reply text submitted by the aggrieved party within 2 (two) months of publication—provided it contains no criminal elements and does not violate legally protected interests of third parties—shall be published by the Managing Editor without any alteration or addition within 1 (one) day of receipt, in the same page and column layout with a direct URL link, in identical typeface, for a period of 1 (one) week.\n\n2. Content Notice Procedure under Law No. 5651 (Notice-and-Takedown):\nNotifications alleging infringement of personality rights under Law No. 5651 or intellectual property rights under Law No. 5846 may be submitted to our Managing Editor's Office together with the exact URL, legal grounds and identity/authorisation documents.\n\n3. Submission Channels:\n• Managing Editor: Dr. K. Burak CODUR — b.codur@savunmahaber.com\n• Corporate Legal / Contact Email: info@savunmahaber.com\n• Written Service Address: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti., Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — Tel: +90 312 225 41 73",

        // 19. kurumsal-reklam-ve-sponsorluk
        "Reklam, Sponsorluk ve Ticari İletişim Politikası" => "Advertising, Sponsorship and Commercial Communications Policy",
        "6502 sayılı Kanun m.61 ve Ticari Reklam ve Haksız Ticari Uygulamalar Yönetmeliği (RG: 29232) örtülü reklam yasağına tam uyumlu reklam politikamız." => "Our advertising policy fully compliant with Law No. 6502 Art. 61 and the Regulation on Commercial Advertising and Unfair Commercial Practices (OG: 29232) prohibiting surreptitious advertising.",
        "6502 SAYILI KANUN M.61 & RG: 29232 YÖNETMELİĞİ" => "LAW NO. 6502 ART. 61 & REGULATION OG: 29232",
        "Editoryal Bağımsızlık, Örtülü Reklam Yasağı ve Sponsorluk Kuralları" => "Editorial Independence, Prohibition of Surreptitious Advertising and Sponsorship Rules",
        "MSI Dergisi ve MSI TDR platformlarında yürütülen tüm reklam, banner ve sponsorlu içerik faaliyetleri; 6502 sayılı Tüketicinin Korunması Hakkında Kanun m.61, Ticari Reklam ve Haksız Ticari Uygulamalar Yönetmeliği (RG: 10.01.2015 / 29232 ve güncel değişiklikler) ile Reklam Kurulu ilke kararlarına tam uyumlu olarak yürütülür:\n\n1. Örtülü Reklam Yasağı ve Açık Etiketleme:\n6502 sayılı Kanun m.61/4 uyarınca örtülü reklam kesinlikle yasaktır. Bedel veya benzeri bir karşılıkla yayımlanan tüm tanıtım yazıları ve kurumsal dosyalar; okuyucunun ilk bakışta fark edebileceği şekilde 'SPONSORLU İÇERİK', 'ADVERTORIAL' veya 'REKLAM · KURUMSAL İŞ BİRLİĞİ' ibaresiyle açıkça etiketlenir.\n\n2. Sektörel Uygunluk ve Kabul Kriterleri:\nPlatformumuzda yalnızca savunma, havacılık, uzay, denizcilik, siber güvenlik, ileri mühendislik ve kurumsal yan sanayi/hizmet ekosistemine yönelik kurumsal tanıtımlar kabul edilir. Bireysel silah satışı ilanları, siyasi propaganda, kumar/bahis, tütün/alkol ve yanıltıcı/doğrulanamaz iddialar içeren reklamlar kesinlikle kabul edilmez.\n\n3. Rakip Koruması ve Çakışma Denetimi:\nSponsorlu içerik sayfalarında rakip marka reklamlarının gösterilmemesi için merkezi reklam motorumuzda kategori ve marka çakışma kontrolü uygulanmaktadır.\n\n4. Reklam ve Kurumsal İş Birliği İletişim:\n• Proje ve İş Geliştirme Koordinatörü: Özay ÖYMEZ — o.oymez@savunmahaber.com\n• Kurumsal İletişim: info@savunmahaber.com • Tel: +90 312 225 41 73" => "All advertising, banner placements and sponsored content operations across MSI Review and MSI TDR platforms strictly comply with Consumer Protection Law No. 6502 Art. 61, the Regulation on Commercial Advertising and Unfair Commercial Practices (OG: 10.01.2015 / 29232, as amended) and decisions of the Turkish Advertising Board:\n\n1. Prohibition of Surreptitious Advertising and Clear Labelling:\nSurreptitious advertising is strictly prohibited under Law No. 6502 Art. 61/4. All promotional features and corporate dossiers published for consideration are clearly labelled at first glance as 'SPONSORED CONTENT', 'ADVERTORIAL' or 'ADVERTISEMENT · CORPORATE PARTNERSHIP'.\n\n2. Sectoral Suitability and Acceptance Criteria:\nOur platforms accept only institutional promotions relevant to defence, aerospace, space, naval, cybersecurity, advanced engineering and supporting industrial/service ecosystems. Personal firearms listings, political propaganda, gambling/betting, tobacco/alcohol and misleading or unsubstantiated claims are strictly declined.\n\n3. Competitor Protection and Conflict Checks:\nOur central advertising engine enforces category and brand conflict checks so that competing brand advertisements are never served on sponsored content pages.\n\n4. Advertising and Corporate Partnership Contact:\n• Project & Business Development Coordinator: Özay ÖYMEZ — o.oymez@savunmahaber.com\n• Corporate Communications: info@savunmahaber.com • Tel: +90 312 225 41 73",

        _ => raw,
    }
}

/// Builds the centralized `{ "<lang>": { key: val } }` dictionary embedded inside
/// `<script id="msi-i18n" type="application/json">` for frontend `msi-ui.js` `t()`.
pub fn i18n_bundle_json(tr: Option<&TranslationCache>, lang: &str) -> Value {
    let norm_lang = if lang.trim().is_empty() { "tr" } else { lang.trim() };
    let is_en = norm_lang.eq_ignore_ascii_case("en");
    let mut strings: BTreeMap<String, String> = BTreeMap::new();
    for (k, v_tr, v_en) in PUBLIC_UI_DEFAULTS {
        strings.insert(
            (*k).to_string(),
            if is_en { (*v_en).to_string() } else { (*v_tr).to_string() },
        );
    }

    if let Some(cache) = tr {
        for entry in cache.all_entries() {
            let val = match norm_lang {
                "tr" => &entry.value_tr,
                "en" => &entry.value_en,
                "ar" => &entry.value_ar,
                "de" => &entry.value_de,
                "ru" => &entry.value_ru,
                "fr" => &entry.value_fr,
                _ => &entry.value_tr,
            };
            if !val.trim().is_empty() {
                strings.insert(entry.key.clone(), val.clone());
            } else if is_en && !entry.value_en.trim().is_empty() {
                strings.insert(entry.key.clone(), entry.value_en.clone());
            } else if !is_en && !entry.value_tr.trim().is_empty() {
                strings.insert(entry.key.clone(), entry.value_tr.clone());
            }
        }
    }

    let mut wrapper = BTreeMap::new();
    wrapper.insert(norm_lang.to_string(), strings);
    serde_json::to_value(wrapper).unwrap_or_else(|_| serde_json::json!({}))
}

const PUBLIC_UI_DEFAULTS: &[(&str, &str, &str)] = &[
    ("shell.skip_to_content", "İçeriğe geç", "Skip to content"),
    ("shell.mode_magazine", "DERGİ", "MAGAZINE"),
    ("shell.mode_news", "HABER", "NEWS"),
    ("shell.edergi", "E-Dergi", "E-Magazine"),
    ("shell.all_topics", "Tüm konular", "All topics"),
    ("shell.reader_center", "OKUYUCU MERKEZİ", "READER CENTER"),
    ("shell.notifications", "Bildirimler", "Notifications"),
    ("shell.mark_all_read", "Tümünü okundu işaretle", "Mark all as read"),
    ("shell.no_notifications", "Şu anda yeni bildirim yok.", "No new notifications right now."),
    ("shell.close", "Kapat", "Close"),
    ("dialogs.issue.title", "E-Dergi", "E-Magazine"),
    ("dialogs.issue.tab_toc", "İçindekiler", "Table of Contents"),
    ("dialogs.issue.tab_pdf", "PDF Okuyucu", "PDF Reader"),
    ("dialogs.issue.open_pdf", "PDF'yi aç ↗", "Open PDF ↗"),
    ("dialogs.issue.hint", "Sayıdaki bölümleri oklarla inceleyebilirsiniz. PDF erişimi abonelik koşullarına bağlıdır.", "Use the arrows to browse issue dossiers. PDF access depends on your subscription tier."),
    ("dialogs.issue.pdf_locked", "Tam sayı PDF okuyucusu aktif aboneler ve doğrulanmış kamu personeli içindir.", "The full-issue PDF reader is reserved for active subscribers and verified public-sector personnel."),
    ("dialogs.issue.prev_spread", "Önceki bölüm", "Previous section"),
    ("dialogs.issue.next_spread", "Sonraki bölüm", "Next section"),
    ("dialogs.auth.eyebrow", "MSI OKUYUCU MERKEZİ", "MSI READER CENTER"),
    ("dialogs.auth.welcome", "Hoş geldiniz", "Welcome"),
    ("dialogs.auth.login", "Giriş yap", "Log in"),
    ("dialogs.auth.register", "Hesap oluştur", "Create account"),
    ("dialogs.auth.email", "E-posta", "Email"),
    ("dialogs.auth.password", "Şifre", "Password"),
    ("dialogs.auth.forgot", "Şifremi unuttum", "Forgot password"),
    ("dialogs.offer.eyebrow", "ABONELİK VE YAYINLAR", "SUBSCRIPTIONS & PUBLICATIONS"),
    ("dialogs.cookie.accept_all", "Tümünü kabul et", "Accept all"),
    ("dialogs.cookie.necessary_only", "Yalnızca zorunlu", "Necessary only"),
    ("dialogs.cookie.manage", "Tercihleri yönet", "Manage preferences"),
    ("dialogs.cookie.save", "Seçimlerimi kaydet", "Save preferences"),
    ("dialogs.lightbox.eyebrow", "GÖRSEL GALERİ", "IMAGE GALLERY"),
    ("dialogs.lightbox.open_raw", "Tam boy aç ↗", "Open full size ↗"),
    ("reader.paywall_title", "Hikâyenin tamamını keşfedin.", "Discover the full story."),
    ("reader.related_eyebrow", "OKUMAYA DEVAM EDİN", "CONTINUE READING"),
    ("reader.related_title", "İlginizi çekebilir", "You may also like"),
    ("reader.tags_eyebrow", "TEKNOLOJİ ETİKETLERİ", "TECHNOLOGY TAGS"),
    ("reader.share", "Bağlantıyı paylaş", "Share link"),
    ("reader.archive", "Dijital E-Dergi Arşivi", "Digital E-Magazine Archive"),
    ("error.404_badge", "SAYFA BULUNAMADI", "PAGE NOT FOUND"),
    ("error.404_title", "Aradığınız Sayfa Bulunamadı", "Page Not Found"),
    ("error.404_desc", "İstediğiniz içerik taşınmış olabilir veya adres hatalı girilmiş olabilir.", "The content you requested may have been moved or the address may be incorrect."),
    ("error.404_home", "Ana Sayfaya Dön", "Return to Home"),
    ("error.404_search", "İçerikleri ara", "Search content"),
    ("error.404_page_title", "404 Sayfa Bulunamadı", "404 Page Not Found"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::translation_engine::TranslationEntry;

    fn item(source: &str, en_status: &str, title_tr: &str, title_en: &str, body_en: &str) -> ContentItem {
        ContentItem {
            id: 1,
            slug: "test-slug".into(),
            title_tr: title_tr.into(),
            title_en: title_en.into(),
            body_tr: "<p>TR</p>".into(),
            body_en: body_en.into(),
            content_type: "haberler".into(),
            source_group: source.into(),
            cover_image: "images/example.webp".into(),
            published_at: "2026-11-08".into(),
            read_time_min: 3,
            status: "published".into(),
            en_status: en_status.into(),
            ..Default::default()
        }
    }

    #[test]
    fn en_content_readiness_blocks_unapproved_and_allows_approved_or_tdr() {
        assert!(!is_en_content_ready(&item("gunluk-haber", "none", "TR", "EN Title", "<p>Body</p>")));
        assert!(!is_en_content_ready(&item("gunluk-haber", "auto_draft", "TR", "EN Title", "<p>Body</p>")));
        assert!(is_en_content_ready(&item("gunluk-haber", "approved", "TR", "EN Title", "<p>Body</p>")));
        assert!(is_en_content_ready(&item("msi-tdr", "none", "TDR", "TDR", "")));
    }

    #[test]
    fn i18n_bundle_merges_translation_cache_entries_over_defaults() {
        let tr = TranslationCache::new();
        tr.load(vec![TranslationEntry {
            key: "dialogs.auth.welcome".into(),
            value_tr: "Özel Hoş Geldiniz".into(),
            value_en: "Custom Welcome".into(),
            value_ar: String::new(),
            value_de: String::new(),
            value_ru: String::new(),
            value_fr: String::new(),
            group: "dialogs".into(),
        }]);
        let bundle_tr = i18n_bundle_json(Some(&tr), "tr");
        let bundle_en = i18n_bundle_json(Some(&tr), "en");
        assert_eq!(bundle_tr["tr"]["dialogs.auth.welcome"], "Özel Hoş Geldiniz");
        assert_eq!(bundle_en["en"]["dialogs.auth.welcome"], "Custom Welcome");
        assert_eq!(bundle_en["en"]["dialogs.issue.tab_pdf"], "PDF Reader");
    }

    #[test]
    fn kurumsal_pages_translate_to_british_english_cleanly() {
        let cfg = crate::engine::design::config::defaults();
        let kurumsal_pages: Vec<_> = cfg
            .pages
            .iter()
            .filter(|p| p.slug.starts_with("kurumsal-"))
            .collect();
        assert_eq!(kurumsal_pages.len(), 19, "expected 19 kurumsal-* pages including kurumsal-biz-kimiz");
        assert!(kurumsal_pages.iter().any(|p| p.slug == "kurumsal-biz-kimiz"));

        for page in kurumsal_pages {
            let title_en = localize_section_copy("en", &page.title);
            assert!(!title_en.trim().is_empty(), "empty EN title for {}", page.slug);
            assert_ne!(title_en.trim(), page.title.trim(), "untranslated page.title for {}", page.slug);
            assert!(!title_en.contains("Defense"), "US spelling in page.title for {}", page.slug);

            let desc_en = localize_section_copy("en", &page.description);
            assert!(!desc_en.trim().is_empty(), "empty EN description for {}", page.slug);
            assert_ne!(desc_en.trim(), page.description.trim(), "untranslated page.description for {}", page.slug);
            assert!(!desc_en.contains("Defense"), "US spelling in page.description for {}", page.slug);

            for (idx, sec) in page.sections.iter().enumerate() {
                if !sec.eyebrow.trim().is_empty() {
                    let eb_en = localize_section_copy("en", &sec.eyebrow);
                    assert_ne!(eb_en.trim(), sec.eyebrow.trim(), "untranslated section.eyebrow on {}[{}]", page.slug, idx);
                    assert!(!eb_en.contains("Defense"), "US spelling in eyebrow on {}[{}]", page.slug, idx);
                }
                if !sec.title.trim().is_empty() {
                    let st_en = localize_section_copy("en", &sec.title);
                    if sec.title.trim() != "AMAC Magazine" {
                        assert_ne!(st_en.trim(), sec.title.trim(), "untranslated section.title on {}[{}]", page.slug, idx);
                    }
                    assert!(!st_en.contains("Defense"), "US spelling in section.title on {}[{}]", page.slug, idx);
                }
                if !sec.description.trim().is_empty() {
                    let sd_en = localize_section_copy("en", &sec.description);
                    assert_ne!(sd_en.trim(), sec.description.trim(), "untranslated section.description on {}[{}]", page.slug, idx);
                    assert!(!sd_en.contains("Defense"), "US spelling in section.description on {}[{}]", page.slug, idx);
                }
            }
        }
    }

    #[test]
    fn all_defaults_json_pages_translate_to_british_english_without_turkish_leaks() {
        let raw_json: Value = serde_json::from_str(include_str!("defaults.json"))
            .expect("defaults.json must parse cleanly");
        let pages = raw_json["pages"]
            .as_array()
            .expect("defaults.json pages must be an array");
        assert!(pages.len() >= 23);

        let turkish_diacritics = ['ç', 'ğ', 'ı', 'ö', 'ş', 'ü', 'Ç', 'Ğ', 'İ', 'Ö', 'Ş', 'Ü'];

        for page in pages {
            let slug = page["slug"].as_str().unwrap_or("");
            for field in ["title", "description"] {
                let raw = page[field].as_str().unwrap_or("").trim();
                if raw.is_empty() {
                    continue;
                }
                let en = localize_section_copy("en", raw);
                assert_ne!(
                    en.trim(),
                    raw,
                    "untranslated {slug}.{field}: {raw}"
                );
                assert!(
                    !en.contains("Defense"),
                    "US spelling in {slug}.{field}: {en}"
                );
                if !slug.starts_with("kurumsal-") {
                    assert!(
                        !en.chars().any(|c| turkish_diacritics.contains(&c)),
                        "Turkish character leak in {slug}.{field}: {en}"
                    );
                }
            }

            if let Some(sections) = page["sections"].as_array() {
                for (idx, sec) in sections.iter().enumerate() {
                    for field in ["eyebrow", "title", "description", "link_label"] {
                        let raw = sec[field].as_str().unwrap_or("").trim();
                        if raw.is_empty() {
                            continue;
                        }
                        let en = localize_section_copy("en", raw);
                        if raw != "ADVERTORIALS" && raw != "AMAC Magazine" {
                            assert_ne!(
                                en.trim(),
                                raw,
                                "untranslated {slug}.sections[{idx}].{field}: {raw}"
                            );
                        }
                        assert!(
                            !en.contains("Defense"),
                            "US spelling in {slug}.sections[{idx}].{field}: {en}"
                        );
                        if !slug.starts_with("kurumsal-") {
                            assert!(
                                !en.chars().any(|c| turkish_diacritics.contains(&c)),
                                "Turkish character leak in {slug}.sections[{idx}].{field}: {en}"
                            );
                        }
                    }
                }
            }
        }
        assert_eq!(localize_section_copy("en", "ADVERTORIALS"), "ADVERTORIALS");
    }
}
