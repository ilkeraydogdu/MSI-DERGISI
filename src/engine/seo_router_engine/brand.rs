use std::collections::HashMap;

/// Centralized brand/publisher configuration extracted from `site_settings` DB table.
/// All SEO metadata flows through this struct — zero hardcoded brand constants in logic.
#[derive(Clone, Debug)]
pub struct SeoBrandConfig {
    pub publisher_name: String,
    pub publisher_legal_name: String,
    pub founding_date: String,
    pub issn: String,
    pub default_og_image: String,
    pub default_edergi_cover: String,
    pub logo_url: String,
    pub address_street: String,
    pub address_locality: String,
    pub address_region: String,
    pub postal_code: String,
    pub country: String,
    pub contact_phone: String,
    pub contact_email: String,
    pub twitter_handle: String,
    pub default_keywords_tr: String,
    pub default_keywords_en: String,
    pub rss_title_tr: String,
    pub rss_title_en: String,
    pub rss_desc_tr: String,
    pub rss_desc_en: String,
    pub copyright_notice: String,
    pub opensearch_short_name: String,
    pub opensearch_description: String,
    pub manifest_short_name: String,
    pub manifest_description: String,
    pub llms_header_text: String,
    pub category_fallback_desc: String,
    pub json_feed_description: String,
    pub social_facebook: String,
    pub social_linkedin: String,
    pub social_instagram: String,
    pub social_twitter: String,
    pub social_youtube: String,
    pub brand_title_tr: String,
    pub brand_title_en: String,
}

impl SeoBrandConfig {
    /// Build from a `settings_map` (as returned by `DbPool::get_setting_map`).
    /// Falls back to safe defaults only for truly optional fields; required brand
    /// identity fields use the canonical MSI Dergisi values so the site never
    /// renders broken metadata even if seed data is missing.
    pub fn from_settings(map: &HashMap<String, String>) -> Self {
        let g = |key: &str, fallback: &str| -> String {
            map.get(key)
                .filter(|v| !v.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| fallback.to_string())
        };

        Self {
            publisher_name: g("brand_title", "MSI Dergisi & MSI Turkish Defence Review"),
            publisher_legal_name: g("publisher_legal_name", "UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti."),
            founding_date: g("publisher_founding_date", "2004"),
            issn: g("publisher_issn", "1306-0309"),
            default_og_image: g("default_og_image", crate::services::media_service::DEFAULT_OG_IMAGE),
            default_edergi_cover: g("default_edergi_cover", crate::services::media_service::DEFAULT_ISSUE_COVER),
            logo_url: g("publisher_logo_url", "/images/logo.png"),
            address_street: g("publisher_address_street", "Yaşamkent Mah. 3147 Sok. No:10"),
            address_locality: g("publisher_address_locality", "Çankaya"),
            address_region: g("publisher_address_region", "Ankara"),
            postal_code: g("publisher_postal_code", "06810"),
            country: g("publisher_country", "TR"),
            contact_phone: g("contact_phone", "+90-312-225-4173"),
            contact_email: g("contact_email", "info@savunmahaber.com"),
            twitter_handle: g("twitter_handle", "@MSI_Dergisi"),
            default_keywords_tr: g("default_keywords_tr", "MSI Dergisi, savunmahaber, Türk savunma sanayii, havacılık, uzay, deniz platformları, İHA, SİHA, KAAN, MİLGEM, füze sistemleri, elektronik harp"),
            default_keywords_en: g("default_keywords_en", "MSI TDR, Turkish Defence Review, Turkish defense industry, aerospace, naval platforms, UAV, missiles, radar, electronic warfare"),
            rss_title_tr: g("rss_title_tr", "MSI Dergisi — Savunmahaber.com | Savunma, Havacılık ve Güvenlik Teknolojileri"),
            rss_title_en: g("rss_title_en", "MSI TDR — Turkish Defence Review | Defense & Aerospace Intelligence"),
            rss_desc_tr: g("rss_description_tr", "MSI Dergisi (ISSN 1306-0309) ve savunmahaber.com tekil içerik havuzundan güncel savunma sanayii haberleri, özel dosyalar, söyleşiler ve analizler."),
            rss_desc_en: g("rss_description_en", "Authoritative analysis, Turkish defense industry news, special dossiers, and executive interviews from MSI Turkish Defence Review (ISSN 1306-0309)."),
            copyright_notice: g("copyright_notice", "UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti."),
            opensearch_short_name: g("opensearch_short_name", "MSI Dergisi"),
            opensearch_description: g("opensearch_description", "MSI Dergisi ve MSI TDR savunma, havacılık ve güvenlik teknolojileri arşivinde arama yapın."),
            manifest_short_name: g("manifest_short_name", "MSI Dergisi"),
            manifest_description: g("manifest_description", "Savunma, havacılık ve güvenlik teknolojilerinde 2004'ten bugüne stratejik referans yayını (ISSN 1306-0309)."),
            llms_header_text: g("llms_header_text", "Türkiye'nin 2004 yılından bu yana kesintisiz yayımlanan lider Savunma, Havacılık, Uzay ve Denizcilik Teknolojileri referans yayını (ISSN: 1306-0309). Yayıncı: UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. (Ankara, Türkiye)."),
            category_fallback_desc: g("category_fallback_desc", "MSI Dergisi savunma sanayii, havacılık ve güvenlik teknolojileri içerik arşivi."),
            json_feed_description: g("json_feed_description", "2004'ten bugüne Türk savunma ve havacılık sanayisinin stratejik referans yayını (ISSN 1306-0309)."),
            social_facebook: g("social_facebook", "https://www.facebook.com/MSI.Dergisi"),
            social_linkedin: g("social_linkedin", "https://www.linkedin.com/company/msi-dergisi"),
            social_instagram: g("social_instagram", "https://www.instagram.com/msidergisi"),
            social_twitter: g("social_twitter", "https://twitter.com/MSI_Dergisi"),
            social_youtube: g("social_youtube", "https://www.youtube.com/c/msidergisi"),
            brand_title_tr: g("brand_title", "MSI Dergisi — Savunma Teknolojileri ve Havacılık Platformu"),
            brand_title_en: g("brand_title_en", "MSI TDR — Turkish Defence Review"),
        }
    }

    /// Returns the `sameAs` array for Schema.org publisher entity, built from DB social settings.
    pub fn same_as_urls(&self, primary_domain: &str, news_domain: &str) -> Vec<String> {
        let mut urls = vec![
            format!("https://{}", primary_domain),
            format!("https://{}", news_domain),
        ];
        if !self.social_linkedin.is_empty() { urls.push(self.social_linkedin.clone()); }
        if !self.social_twitter.is_empty() { urls.push(self.social_twitter.clone()); }
        // Also include x.com mirror if twitter handle exists
        if !self.social_twitter.is_empty() {
            let x_url = self.social_twitter.replace("twitter.com", "x.com");
            if x_url != self.social_twitter {
                urls.push(x_url);
            }
        }
        if !self.social_instagram.is_empty() { urls.push(self.social_instagram.clone()); }
        if !self.social_facebook.is_empty() { urls.push(self.social_facebook.clone()); }
        if !self.social_youtube.is_empty() { urls.push(self.social_youtube.clone()); }
        urls
    }
}

