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

/// Canonical 13 official MSI institutional categories used when DB rows are not yet passed.
pub fn default_institution_choices() -> Vec<InstitutionChoice> {
    [
        ("Sektör Kuruluşları", "Sektör Kuruluşları", "Defence Industry Companies"),
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
    ]
    .into_iter()
    .map(|(code, tr, en)| InstitutionChoice {
        code: code.to_string(),
        label_tr: tr.to_string(),
        label_en: en.to_string(),
    })
    .collect()
}

/// Gate preventing unapproved or TR-copied articles from appearing on `/en` public listings.
/// In Wave 2B/merge this upgrades to `c.en_status == "approved" || c.source_group == "msi-tdr"`.
#[inline]
pub fn is_en_content_ready(c: &ContentItem) -> bool {
    c.source_group.eq_ignore_ascii_case("msi-tdr")
        || (!c.title_en.trim().is_empty()
            && !c.body_en.trim().is_empty()
            && c.title_en.trim() != c.title_tr.trim())
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

    fn item(source: &str, title_tr: &str, title_en: &str, body_en: &str) -> ContentItem {
        ContentItem {
            id: 1,
            slug: "test-slug".into(),
            title_tr: title_tr.into(),
            title_en: title_en.into(),
            summary_tr: String::new(),
            summary_en: String::new(),
            body_tr: "<p>TR</p>".into(),
            body_en: body_en.into(),
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
    fn en_content_readiness_blocks_tr_copied_and_empty_body_articles() {
        assert!(!is_en_content_ready(&item("gunluk-haber", "Aynı Başlık", "Aynı Başlık", "<p>Body</p>")));
        assert!(!is_en_content_ready(&item("gunluk-haber", "TR", "EN Title", "   ")));
        assert!(!is_en_content_ready(&item("gunluk-haber", "TR", "", "<p>Body</p>")));
        assert!(is_en_content_ready(&item("gunluk-haber", "TR", "EN Title", "<p>Body</p>")));
        assert!(is_en_content_ready(&item("msi-tdr", "TDR", "TDR", "")));
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
}
