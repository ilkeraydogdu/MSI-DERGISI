//! Pure-Rust SSR Modular Admin Views (`src/engine/design/admin/`).
//!
//! Renders the MSI Command Gateway (`/podmin` login/2FA/recovery) and the
//! 18-tab Editorial, Commercial, Analytics, Workflow & System Command Center
//! using `HtmlBuf` with zero inline styles, zero inline event handlers, and
//! human-centric Turkish labels.

use crate::engine::translation_engine::TranslationCache;

pub mod analytics;
pub mod commercial;
pub mod editorial;
pub mod login;
pub mod shell;
pub mod system;
pub mod ui;
pub mod workflow;

pub use login::render_login;
pub use shell::render_dashboard;

/// Centralized i18n lookup helper for all Admin SSR modules.
pub(crate) fn tr_admin(
    tr: &TranslationCache,
    lang: &str,
    key: &str,
    fallback_tr: &str,
    fallback_en: &str,
) -> String {
    let val = tr.t(lang, key);
    if val != key {
        return val;
    }
    if lang.eq_ignore_ascii_case("en") {
        fallback_en.to_string()
    } else {
        fallback_tr.to_string()
    }
}

/// Normalizes a secret admin path segment (strips leading/trailing whitespace and
/// slashes, rejects multi-segment paths, and normalizes `podmin` / retired slugs).
pub(crate) fn clean_segment(admin_path: &str) -> &str {
    let trimmed = admin_path.trim_matches(|c: char| c.is_whitespace() || c == '/');
    if trimmed.is_empty()
        || trimmed.contains('/')
        || crate::config::is_retired_admin_segment(trimmed)
        || trimmed.eq_ignore_ascii_case("podmin")
    {
        "podmin"
    } else {
        trimmed
    }
}

/// Canonical 7 multi-section editorial placement options (code, i18n_key, tr, en).
const PLACEMENT_TAG_DEFS: &[(&str, &str, &str, &str)] = &[
    ("mesajlar", "admin.tag.placement.mesajlar", "Mesajlar", "Messages"),
    ("ozel-dosyalar", "admin.tag.placement.ozel_dosyalar", "Özel Dosyalar", "Special Dossiers"),
    ("kapak-dosyalari", "admin.tag.placement.kapak_dosyalari", "Kapak Dosyaları", "Cover Dossiers"),
    ("tskdan-haberler", "admin.tag.placement.tskdan_haberler", "TSK'dan Haberler", "News from TAF"),
    ("dergi-icerikleri", "admin.tag.placement.dergi_icerikleri", "Dergi İçerikleri", "Magazine Content"),
    ("etkinlik-ozel-sayilari", "admin.tag.placement.etkinlik_ozel_sayilari", "Etkinlik Özel Sayıları", "Event Special Issues"),
    ("etkinlik-sonrasi-ozel-dosyalar", "admin.tag.placement.etkinlik_sonrasi", "Etkinlik Sonrası Özel Dosyalar", "Post-Event Special Dossiers"),
];

/// Canonical 14 SSB defense industry catalog interest tags (code, i18n_key, tr, en).
const SSB_TAG_DEFS: &[(&str, &str, &str, &str)] = &[
    ("kara-araclari", "admin.tag.ssb.kara_araclari", "Kara Araçları", "Land Vehicles"),
    ("deniz-araclari", "admin.tag.ssb.deniz_araclari", "Deniz Araçları", "Naval Platforms"),
    ("hava-araclari", "admin.tag.ssb.hava_araclari", "Hava Araçları", "Aerial Systems"),
    ("muhimmatlar-ve-fuze-sistemleri", "admin.tag.ssb.muhimmatlar", "Mühimmatlar ve Füze Sistemleri", "Munitions & Missile Systems"),
    ("namlulu-silahlar", "admin.tag.ssb.namlulu_silahlar", "Namlulu Silahlar", "Barrelled Weapons"),
    ("haberlesme-ve-muharebe-destek-sistemleri", "admin.tag.ssb.haberlesme", "Haberleşme ve Muharebe Destek", "Communications & Combat Support"),
    ("radarlar-ve-sensorler", "admin.tag.ssb.radarlar", "Radarlar ve Sensörler", "Radars & Sensors"),
    ("simulatorler-ve-egitim-sistemleri", "admin.tag.ssb.simulatorler", "Simülatörler ve Eğitim", "Simulators & Training"),
    ("elektronik-harp-sistemleri", "admin.tag.ssb.elektronik_harp", "Elektronik Harp Sistemleri", "Electronic Warfare Systems"),
    ("itki-tahrik-ve-guc-sistemleri", "admin.tag.ssb.itki_tahrik", "İtki / Tahrik ve Güç Sistemleri", "Propulsion & Power Systems"),
    ("hizmetler-danismanlik", "admin.tag.ssb.danismanlik", "Hizmetler - Danışmanlık", "Services - Consultancy"),
    ("hizmetler-egitim", "admin.tag.ssb.egitim", "Hizmetler - Eğitim", "Services - Training"),
    ("hizmetler-kurumsal-iletisim", "admin.tag.ssb.kurumsal_iletisim", "Hizmetler - Kurumsal İletişim", "Services - Corporate Comms"),
    ("hizmetler-kurumsal-catering", "admin.tag.ssb.kurumsal_catering", "Hizmetler - Kurumsal Catering", "Services - Corporate Catering"),
];

/// Canonical 14 target defense & public institutions for B2B campaigns & content targeting (code, i18n_key, tr, en).
const INSTITUTION_TAG_DEFS: &[(&str, &str, &str, &str)] = &[
    ("Sektör Kuruluşları", "admin.tag.inst.sektor", "Sektör Kuruluşları", "Industry Organisations"),
    ("MSB", "admin.tag.inst.msb", "MSB (Millî Savunma B.)", "MoD (MSB)"),
    ("SSB", "admin.tag.inst.ssb", "SSB (Savunma Sanayii B.)", "SSB (Defence Industries)"),
    ("K.K.K.lığı", "admin.tag.inst.kkk", "K.K.K.lığı", "Turkish Land Forces"),
    ("Dz.K.K.lığı", "admin.tag.inst.dzkk", "Dz.K.K.lığı", "Turkish Naval Forces"),
    ("Hv.K.K.lığı", "admin.tag.inst.hvkk", "Hv.K.K.lığı", "Turkish Air Force"),
    ("J.G.K.lığı", "admin.tag.inst.jgk", "J.G.K.lığı", "Gendarmerie Command"),
    ("EGM", "admin.tag.inst.egm", "EGM", "Turkish National Police"),
    ("S.G.K.lığı", "admin.tag.inst.sgk", "S.G.K.lığı", "Turkish Coast Guard"),
    ("MİT", "admin.tag.inst.mit", "MİT", "National Intelligence Org."),
    ("Diğer Kamu Kurumları", "admin.tag.inst.diger_kamu", "Diğer Kamu Kurumları", "Other Public Institutions"),
    ("Üniversite / Öğrenci", "admin.tag.inst.universite", "Üniversite / Öğrenci", "University / Student"),
    ("Yurt Dışı Paydaş", "admin.tag.inst.yurt_disi", "Yurt Dışı Paydaş", "International Stakeholder"),
    ("savunma_medya", "admin.tag.inst.medya", "Savunma Medya / Basın", "Defence Media / Press"),
];

fn resolve_tag_list(
    defs: &[(&'static str, &str, &str, &str)],
    tr: &TranslationCache,
    lang: &str,
) -> Vec<(&'static str, String)> {
    defs.iter()
        .map(|(code, key, fb_tr, fb_en)| (*code, tr_admin(tr, lang, key, fb_tr, fb_en)))
        .collect()
}

pub(crate) fn placement_tags(tr: &TranslationCache, lang: &str) -> Vec<(&'static str, String)> {
    resolve_tag_list(PLACEMENT_TAG_DEFS, tr, lang)
}

pub(crate) fn ssb_tags(tr: &TranslationCache, lang: &str) -> Vec<(&'static str, String)> {
    resolve_tag_list(SSB_TAG_DEFS, tr, lang)
}

pub(crate) fn institution_tags(tr: &TranslationCache, lang: &str) -> Vec<(&'static str, String)> {
    resolve_tag_list(INSTITUTION_TAG_DEFS, tr, lang)
}

#[cfg(test)]
mod tests {
    use super::clean_segment;

    #[test]
    fn clean_segment_matches_config_normalization_edge_cases() {
        for to_podmin in [
            "",
            "   ",
            "///",
            "podmin",
            "PODMIN",
            "/Podmin/",
            "podmin/tasarim",
            "msi-komuta",
            "/MSI-KOMUTA/",
            "msi%2dkomuta",
            "msi%2Dkomuta",
            "msi komuta",
            "msi-komuta/tasarim",
            "custom/nested",
        ] {
            assert_eq!(
                clean_segment(to_podmin),
                "podmin",
                "Expected clean_segment({:?}) == \"podmin\"",
                to_podmin
            );
        }

        assert_eq!(clean_segment("e2e-admin-abc123"), "e2e-admin-abc123");
        assert_eq!(clean_segment("  /e2e-admin-abc123/  "), "e2e-admin-abc123");
    }
}
