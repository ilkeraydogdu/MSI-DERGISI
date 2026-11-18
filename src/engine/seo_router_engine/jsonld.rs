use super::{CentralSeoEngine, SeoBrandConfig};
use serde_json::{json, Value};

impl CentralSeoEngine {
    /// Authoritative `NewsMediaOrganization` entity node for Google News, Knowledge Graph, Yandex & AI/LLM engines.
    /// All values sourced from `SeoBrandConfig` (backed by `site_settings` DB table).
    pub(super) fn publisher_entity(clean_base: &str, brand: &SeoBrandConfig, primary_domain: &str, news_domain: &str) -> Value {
        let logo_url = format!("{}{}", clean_base, brand.logo_url);
        json!({
            "@type": "NewsMediaOrganization",
            "@id": format!("{}/#organization", clean_base),
            "name": brand.publisher_name,
            "legalName": brand.publisher_legal_name,
            "alternateName": [
                "MSI Dergisi",
                "MSI TDR",
                "MSI Turkish Defence Review",
                news_domain,
                primary_domain
            ],
            "url": clean_base,
            "foundingDate": brand.founding_date,
            "issn": brand.issn,
            "logo": {
                "@type": "ImageObject",
                "@id": format!("{}/#logo", clean_base),
                "url": logo_url,
                "contentUrl": logo_url,
                "width": 448,
                "height": 166,
                "caption": format!("{} Resmi Logosu", brand.opensearch_short_name)
            },
            "address": {
                "@type": "PostalAddress",
                "streetAddress": brand.address_street,
                "addressLocality": brand.address_locality,
                "addressRegion": brand.address_region,
                "postalCode": brand.postal_code,
                "addressCountry": brand.country
            },
            "contactPoint": [
                {
                    "@type": "ContactPoint",
                    "telephone": brand.contact_phone,
                    "email": brand.contact_email,
                    "contactType": "editorial and customer support",
                    "areaServed": ["TR", "Worldwide"],
                    "availableLanguage": ["Turkish", "English"]
                }
            ],
            "masthead": format!("{}/kurumsal/kunye", clean_base),
            "publishingPrinciples": format!("{}/kurumsal/telif-ve-etik", clean_base),
            "ethicsPolicy": format!("{}/kurumsal/telif-ve-etik", clean_base),
            "correctionsPolicy": format!("{}/kurumsal/duzeltme-ve-cevap-hakki", clean_base),
            "actionableFeedbackPolicy": format!("{}/kurumsal/iletisim", clean_base),
            "ownershipFundingInfo": format!("{}/kurumsal/hakkimizda", clean_base),
            "sameAs": brand.same_as_urls(primary_domain, news_domain)
        })
    }

    pub(super) fn website_entity(clean_base: &str, in_language: &str, brand: &SeoBrandConfig) -> Value {
        json!({
            "@type": "WebSite",
            "@id": format!("{}/#website", clean_base),
            "url": clean_base,
            "name": brand.brand_title_tr,
            "alternateName": brand.brand_title_en,
            "inLanguage": in_language,
            "publisher": {
                "@id": format!("{}/#organization", clean_base)
            },
            "potentialAction": {
                "@type": "SearchAction",
                "target": {
                    "@type": "EntryPoint",
                    "urlTemplate": format!("{}/arama?q={{search_term_string}}", clean_base)
                },
                "query-input": "required name=search_term_string"
            }
        })
    }
}
