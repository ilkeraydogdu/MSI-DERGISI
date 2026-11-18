pub fn localized<'a>(lang: &str, tr: &'a str, en: &'a str) -> &'a str {
    if lang == "en" && !en.is_empty() {
        en
    } else {
        tr
    }
}

pub fn category_display_label<'a>(slug: &'a str, lang: &str) -> &'a str {
    if lang == "en" {
        match slug {
            "haberler" => "NEWS",
            "kapak-dosyasi" | "kapak-dosyalari" => "COVER STORY",
            "soylesiler" => "INTERVIEW",
            "ozel-haberler" => "EXCLUSIVE REPORT",
            "basyazilar" | "basyazi" => "EDITORIAL",
            "mesajlar" => "EXECUTIVE MESSAGE",
            "tanitici-makaleler" => "SPECIAL COVERAGE",
            "makaleler-analizler" => "ANALYSIS",
            "advertorials" | "advertorial" => "ADVERTORIAL",
            "ozel-dosya" | "ozel-dosyalar" => "SPECIAL DOSSIER",
            "yeni-sayi-kapak" => "NEW ISSUE",
            _ => slug,
        }
    } else {
        match slug {
            "haberler" => "HABERLER",
            "kapak-dosyasi" | "kapak-dosyalari" => "KAPAK DOSYASI",
            "soylesiler" => "SÖYLEŞİ",
            "ozel-haberler" => "ÖZEL HABER",
            "basyazilar" | "basyazi" => "BAŞYAZI",
            "mesajlar" => "MESAJ",
            "tanitici-makaleler" => "TANITICI MAKALE",
            "makaleler-analizler" => "MAKALE / ANALİZ",
            "advertorials" | "advertorial" => "ADVERTORIAL",
            "ozel-dosya" | "ozel-dosyalar" => "ÖZEL DOSYA",
            "yeni-sayi-kapak" => "YENİ SAYI",
            "tskdan-haberler" => "TSK'DAN HABERLER",
            _ => slug,
        }
    }
}

pub const CANONICAL_SSB_TAG_CODES: &[&str] = &[
    "kara-araclari",
    "deniz-araclari",
    "hava-araclari",
    "muhimmatlar-ve-fuze-sistemleri",
    "namlulu-silahlar",
    "haberlesme-ve-muharebe-destek-sistemleri",
    "radarlar-ve-sensorler",
    "simulatorler-ve-egitim-sistemleri",
    "elektronik-harp-sistemleri",
    "itki-tahrik-ve-guc-sistemleri",
    "hizmetler-danismanlik",
    "hizmetler-egitim",
    "hizmetler-kurumsal-iletisim",
    "hizmetler-kurumsal-catering",
];

pub fn ssb_tag_display_label(code: &str, lang: &str) -> String {
    let mapped = if lang == "en" {
        match code {
            "kara-araclari" => Some("Land Vehicles & Armored Platforms"),
            "deniz-araclari" => Some("Naval Vessels & Submarines"),
            "hava-araclari" => Some("Combat & Unmanned Aerial Vehicles"),
            "muhimmatlar-ve-fuze-sistemleri" => Some("Munitions & Missile Systems"),
            "namlulu-silahlar" => Some("Barrels, Small Arms & Cannons"),
            "haberlesme-ve-muharebe-destek-sistemleri" => Some("C4ISR & Tactical Communications"),
            "radarlar-ve-sensorler" => Some("AESA Radars & Electro-Optical Sensors"),
            "simulatorler-ve-egitim-sistemleri" => Some("Simulators & Synthetic Training"),
            "elektronik-harp-sistemleri" => Some("Electronic Warfare & Countermeasures"),
            "itki-tahrik-ve-guc-sistemleri" => Some("Propulsion, Turboshaft & Powerpacks"),
            "hizmetler-danismanlik" => Some("Services - Defence Consultancy"),
            "hizmetler-egitim" => Some("Services - Technical & Operational Training"),
            "hizmetler-kurumsal-iletisim" => Some("Services - Defence PR & Exhibition Architecture"),
            "hizmetler-kurumsal-catering" => Some("Services - Field & Summit Catering Logistics"),
            _ => None,
        }
    } else {
        match code {
            "kara-araclari" => Some("Kara Araçları"),
            "deniz-araclari" => Some("Deniz Araçları"),
            "hava-araclari" => Some("Hava Araçları"),
            "muhimmatlar-ve-fuze-sistemleri" => Some("Mühimmatlar ve Füze Sistemleri"),
            "namlulu-silahlar" => Some("Namlulu Silahlar"),
            "haberlesme-ve-muharebe-destek-sistemleri" => {
                Some("Haberleşme ve Muharebe Destek Sistemleri")
            }
            "radarlar-ve-sensorler" => Some("Radarlar ve Sensörler"),
            "simulatorler-ve-egitim-sistemleri" => Some("Simülatörler ve Eğitim Sistemleri"),
            "elektronik-harp-sistemleri" => Some("Elektronik Harp Sistemleri"),
            "itki-tahrik-ve-guc-sistemleri" => Some("İtki / Tahrik ve Güç Sistemleri"),
            "hizmetler-danismanlik" => Some("Hizmetler - Danışmanlık"),
            "hizmetler-egitim" => Some("Hizmetler - Eğitim"),
            "hizmetler-kurumsal-iletisim" => Some("Hizmetler - Kurumsal İletişim"),
            "hizmetler-kurumsal-catering" => Some("Hizmetler - Kurumsal Catering"),
            _ => None,
        }
    };
    mapped
        .map(str::to_string)
        .unwrap_or_else(|| code.replace('-', " "))
}
