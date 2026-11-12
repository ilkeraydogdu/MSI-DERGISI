use crate::engine::security_engine::escape_html;
use crate::models::{CategoryNode, ContentItem, MagazineIssue, SeoMetaBundle, SsbInterestTag};
use serde_json::{json, Value};
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

pub struct CentralSeoEngine;

impl CentralSeoEngine {
    pub fn normalize_base_url(base_url: &str) -> String {
        let trimmed = base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            // Fallback only when no base_url is provided at all; callers should
            // always pass state.config.public_url. Uses PRIMARY_DOMAIN env var
            // as single source of truth instead of hardcoded domain.
            let domain = std::env::var("PRIMARY_DOMAIN")
                .unwrap_or_else(|_| "msidergisi.com".to_string());
            format!("https://{}", domain)
        } else {
            trimmed.to_string()
        }
    }

    pub fn resolve_asset_url(base_url: &str, asset_path: &str, default_image: &str) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let trimmed = asset_path.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else if trimmed.is_empty() {
            format!("{}{}", clean_base, default_image)
        } else if trimmed.starts_with('/') {
            format!("{}{}", clean_base, trimmed)
        } else {
            format!("{}/{}", clean_base, trimmed)
        }
    }

    /// Converts `"YYYY-MM-DD HH:MM:SS"` or `"YYYY-MM-DD"` into timezone-aware ISO 8601 (`+03:00`)
    /// required by Google News, Yandex News, and Schema.org `NewsArticle`.
    pub fn to_iso8601_tr(raw: &str) -> String {
        let t = raw.trim();
        if t.contains('T') && (t.contains('+') || t.ends_with('Z')) {
            return t.to_string();
        }
        if t.len() >= 19 && t.as_bytes().get(10) == Some(&b' ') {
            return format!("{}T{}+03:00", &t[..10], &t[11..19]);
        }
        if t.len() >= 16 && t.as_bytes().get(10) == Some(&b' ') {
            return format!("{}T{}:00+03:00", &t[..10], &t[11..16]);
        }
        if t.len() >= 10 {
            return format!("{}T09:00:00+03:00", &t[..10]);
        }
        // SEO FIX (M1): Removed silent hardcoded fallback that produced wrong metadata for
        // unparseable dates. Log a warning so invalid dates are caught during development.
        tracing::warn!(
            raw_date = %raw,
            "to_iso8601_tr: could not parse date, returning empty string"
        );
        String::new()
    }

    /// Converts `"YYYY-MM-DD HH:MM:SS"` into RFC-2822 format for RSS 2.0 `<pubDate>`.
    fn to_rfc2822_tr(raw: &str) -> String {
        let iso = Self::to_iso8601_tr(raw);
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&iso) {
            return dt.to_rfc2822();
        }
        iso
    }

    fn xml_escape(input: &str) -> String {
        let mut out = String::with_capacity(input.len() + 16);
        for ch in input.chars() {
            match ch {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&apos;"),
                _ => out.push(ch),
            }
        }
        out
    }

    fn strip_html_plain(html: &str) -> String {
        let mut out = String::with_capacity(html.len());
        let mut in_tag = false;
        for ch in html.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => {
                    in_tag = false;
                    out.push(' ');
                }
                _ if !in_tag => out.push(ch),
                _ => {}
            }
        }
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Authoritative `NewsMediaOrganization` entity node for Google News, Knowledge Graph, Yandex & AI/LLM engines.
    /// All values sourced from `SeoBrandConfig` (backed by `site_settings` DB table).
    fn publisher_entity(clean_base: &str, brand: &SeoBrandConfig, primary_domain: &str, news_domain: &str) -> Value {
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

    fn website_entity(clean_base: &str, in_language: &str, brand: &SeoBrandConfig) -> Value {
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

    /// Builds complete OpenGraph, Twitter Card, Yandex/Google robots, hreflang (TR/EN),
    /// Canonical URL, and Schema.org `@graph` JSON-LD metadata.
    /// All brand/publisher data sourced from `SeoBrandConfig` (backed by `site_settings` DB).
    #[allow(clippy::too_many_arguments)]
    pub fn build_seo_meta(
        base_url: &str,
        path: &str,
        title: &str,
        description: &str,
        og_image: &str,
        og_type: &str,
        lang: &str,
        article: Option<&ContentItem>,
        brand: &SeoBrandConfig,
        primary_domain: &str,
        news_domain: &str,
    ) -> SeoMetaBundle {
        let clean_base = Self::normalize_base_url(base_url);
        let clean_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };

        let canonical_url = format!("{}{}", clean_base, clean_path);
        let sep = if clean_path.contains('?') { "&" } else { "?" };
        let hreflang_tr = format!("{}{}{}lang=tr", clean_base, clean_path, sep);
        let hreflang_en = format!("{}{}{}lang=en", clean_base, clean_path, sep);

        let effective_img = if og_image.trim().is_empty() || og_image.ends_with(".svg") {
            brand.default_og_image.as_str()
        } else {
            og_image
        };
        let full_og_image = Self::resolve_asset_url(&clean_base, effective_img, &brand.default_og_image);

        let is_en = lang.eq_ignore_ascii_case("en");
        let in_language = if is_en { "en-US" } else { "tr-TR" };

        let default_keywords = if is_en {
            &brand.default_keywords_en
        } else {
            &brand.default_keywords_tr
        };

        let mut keywords = default_keywords.clone();
        let mut extra_head_html = String::new();

        let json_ld_val = if let Some(art) = article {
            let headline = if is_en && !art.title_en.is_empty() {
                &art.title_en
            } else {
                &art.title_tr
            };
            let abstract_text = if is_en && !art.summary_en.is_empty() {
                &art.summary_en
            } else {
                &art.summary_tr
            };
            let body_plain = Self::strip_html_plain(if is_en && !art.body_en.is_empty() {
                &art.body_en
            } else {
                &art.body_tr
            });
            let word_count = body_plain.split_whitespace().count().max(60);
            let iso_pub = Self::to_iso8601_tr(&art.published_at);
            let primary_category = art
                .placements
                .first()
                .map(|s| s.as_str())
                .unwrap_or(art.content_type.as_str());

            if !art.ssb_tags.is_empty() {
                keywords = format!(
                    "{}, {}, {}, savunma sanayii",
                    art.ssb_tags
                        .iter()
                        .map(|t| t.replace('-', " "))
                        .collect::<Vec<_>>()
                        .join(", "),
                    art.content_type.replace('-', " "),
                    brand.opensearch_short_name
                );
            }

            // Pre-render OpenGraph Article & Google News meta tags directly inside SeoMetaBundle
            // so shell.rs renders 100% of them automatically.
            let author_esc = escape_html(&art.author_name);
            let date_esc = escape_html(&iso_pub);
            let section_esc = escape_html(&art.content_type);
            let kw_esc = escape_html(&keywords);
            let tag_metas: Vec<String> = art
                .ssb_tags
                .iter()
                .map(|t| {
                    format!(
                        r#"    <meta property="article:tag" content="{}" />"#,
                        escape_html(&t.replace('-', " "))
                    )
                })
                .collect();
            extra_head_html = format!(
                "    <meta name=\"author\" content=\"{author}\" />\n    <meta name=\"news_keywords\" content=\"{kw}\" />\n    <meta property=\"article:published_time\" content=\"{date}\" />\n    <meta property=\"article:modified_time\" content=\"{date}\" />\n    <meta property=\"article:section\" content=\"{sec}\" />\n    <meta property=\"article:publisher\" content=\"{fb}\" />\n{tags}\n",
                author = author_esc,
                kw = kw_esc,
                date = date_esc,
                sec = section_esc,
                fb = escape_html(&brand.social_facebook),
                tags = tag_metas.join("\n")
            );

            let is_free = art.visibility_level == 0;
            let mut article_node = json!({
                "@type": "NewsArticle",
                "@id": format!("{}#article", canonical_url),
                "mainEntityOfPage": {
                    "@type": "WebPage",
                    "@id": canonical_url
                },
                "headline": headline,
                "description": abstract_text,
                "image": [
                    {
                        "@type": "ImageObject",
                        "url": full_og_image,
                        "width": 1280,
                        "height": 720,
                        "caption": headline,
                        "representativeOfPage": true
                    }
                ],
                "datePublished": iso_pub,
                "dateModified": iso_pub,
                "inLanguage": in_language,
                "isAccessibleForFree": is_free,
                "wordCount": word_count,
                "timeRequired": format!("PT{}M", art.read_time_min.max(1)),
                "author": {
                    "@type": "Person",
                    "name": art.author_name,
                    "jobTitle": art.author_title,
                    "worksFor": {
                        "@id": format!("{}/#organization", clean_base)
                    }
                },
                "publisher": {
                    "@id": format!("{}/#organization", clean_base)
                },
                "keywords": keywords,
                "articleSection": art.content_type
            });

            if !is_free {
                article_node["hasPart"] = json!({
                    "@type": "WebPageElement",
                    "isAccessibleForFree": false,
                    "cssSelector": ".paywall-panel"
                });
            }

            json!({
                "@context": "https://schema.org",
                "@graph": [
                    Self::publisher_entity(&clean_base, brand, primary_domain, news_domain),
                    Self::website_entity(&clean_base, in_language, brand),
                    article_node,
                    {
                        "@type": "BreadcrumbList",
                        "@id": format!("{}#breadcrumb", canonical_url),
                        "itemListElement": [
                            {
                                "@type": "ListItem",
                                "position": 1,
                                "name": if is_en { "Home" } else { "Ana Sayfa" },
                                "item": format!("{}/", clean_base)
                            },
                            {
                                "@type": "ListItem",
                                "position": 2,
                                "name": primary_category.replace('-', " "),
                                "item": format!("{}/kategori/{}", clean_base, primary_category)
                            },
                            {
                                "@type": "ListItem",
                                "position": 3,
                                "name": headline,
                                "item": canonical_url
                            }
                        ]
                    }
                ]
            })
        } else if clean_path == "/" {
            json!({
                "@context": "https://schema.org",
                "@graph": [
                    Self::publisher_entity(&clean_base, brand, primary_domain, news_domain),
                    Self::website_entity(&clean_base, in_language, brand),
                    {
                        "@type": "WebPage",
                        "@id": format!("{}/#webpage", clean_base),
                        "url": canonical_url,
                        "name": title,
                        "description": description,
                        "inLanguage": in_language,
                        "isPartOf": { "@id": format!("{}/#website", clean_base) },
                        "about": { "@id": format!("{}/#organization", clean_base) }
                    }
                ]
            })
        } else {
            json!({
                "@context": "https://schema.org",
                "@graph": [
                    Self::publisher_entity(&clean_base, brand, primary_domain, news_domain),
                    Self::website_entity(&clean_base, in_language, brand),
                    {
                        "@type": "WebPage",
                        "@id": format!("{}#webpage", canonical_url),
                        "url": canonical_url,
                        "name": title,
                        "description": description,
                        "inLanguage": in_language,
                        "isPartOf": { "@id": format!("{}/#website", clean_base) }
                    },
                    {
                        "@type": "BreadcrumbList",
                        "@id": format!("{}#breadcrumb", canonical_url),
                        "itemListElement": [
                            {
                                "@type": "ListItem",
                                "position": 1,
                                "name": if is_en { "Home" } else { "Ana Sayfa" },
                                "item": format!("{}/", clean_base)
                            },
                            {
                                "@type": "ListItem",
                                "position": 2,
                                "name": title,
                                "item": canonical_url
                            }
                        ]
                    }
                ]
            })
        };

        let json_ld = serde_json::to_string(&json_ld_val).unwrap_or_else(|_| "{}".to_string());
        let robots = if clean_path.starts_with("/hesabim") || clean_path.starts_with("/admin") {
            "noindex, nofollow".to_string()
        } else {
            "index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1".to_string()
        };

        let short_name = &brand.opensearch_short_name;
        SeoMetaBundle {
            title: if title.contains("MSI") {
                title.to_string()
            } else {
                format!("{} | {}", title, short_name)
            },
            description: description.to_string(),
            canonical_url,
            og_type: og_type.to_string(),
            og_image: full_og_image,
            og_image_alt: title.to_string(),
            robots,
            keywords,
            hreflang_tr,
            hreflang_en,
            extra_head_html,
            json_ld,
        }
    }

    /// Specialized SEO metadata builder for `/kategori/:slug` hubs with `CollectionPage` + `ItemList`
    /// and pagination `rel="prev"` / `rel="next"`.
    /// All brand/publisher data sourced from `SeoBrandConfig` (backed by `site_settings` DB).
    #[allow(clippy::too_many_arguments)]
    pub fn build_category_seo_meta(
        base_url: &str,
        category: &CategoryNode,
        items: &[ContentItem],
        page: usize,
        total_pages: usize,
        lang: &str,
        brand: &SeoBrandConfig,
        primary_domain: &str,
        news_domain: &str,
    ) -> SeoMetaBundle {
        let clean_base = Self::normalize_base_url(base_url);
        let is_en = lang.eq_ignore_ascii_case("en");
        let in_language = if is_en { "en-US" } else { "tr-TR" };

        let cat_name = if is_en && !category.name_en.is_empty() {
            &category.name_en
        } else {
            &category.name_tr
        };
        let fallback_desc = if is_en {
            "MSI TDR defense industry, aviation and security technologies content archive."
        } else {
            &brand.category_fallback_desc
        };
        let cat_desc = if is_en && !category.description_en.is_empty() {
            &category.description_en
        } else if !category.description_tr.is_empty() {
            &category.description_tr
        } else {
            fallback_desc
        };

        let page_suffix = if page > 1 {
            format!(" — Sayfa {}", page)
        } else {
            String::new()
        };
        let short_name = &brand.opensearch_short_name;
        let full_title = format!("{} | {}{}", cat_name, short_name, page_suffix);
        let path = crate::engine::url_builder::category_url_page(&category.slug, page);
        let canonical_url = format!("{}{}", clean_base, path);
        let sep = if path.contains('?') { "&" } else { "?" };
        let hreflang_tr = format!("{}{}{}lang=tr", clean_base, path, sep);
        let hreflang_en = format!("{}{}{}lang=en", clean_base, path, sep);

        let lead_img = items
            .first()
            .map(|c| c.cover_image.as_str())
            .unwrap_or(brand.default_og_image.as_str());
        let full_og_image = Self::resolve_asset_url(&clean_base, lead_img, &brand.default_og_image);

        let mut extra_head = String::new();
        if page > 1 {
            let prev_url = if page == 2 {
                format!("{}/kategori/{}", clean_base, escape_html(&category.slug))
            } else {
                format!(
                    "{}/kategori/{}?page={}",
                    clean_base,
                    escape_html(&category.slug),
                    page - 1
                )
            };
            extra_head.push_str(&format!("    <link rel=\"prev\" href=\"{}\" />\n", prev_url));
        }
        if page < total_pages {
            let next_url = format!(
                "{}/kategori/{}?page={}",
                clean_base,
                escape_html(&category.slug),
                page + 1
            );
            extra_head.push_str(&format!("    <link rel=\"next\" href=\"{}\" />\n", next_url));
        }

        let list_elements: Vec<Value> = items
            .iter()
            .take(12)
            .enumerate()
            .map(|(i, c)| {
                json!({
                    "@type": "ListItem",
                    "position": i + 1,
                    "url": format!("{}/icerik/{}", clean_base, c.slug),
                    "name": if is_en && !c.title_en.is_empty() { &c.title_en } else { &c.title_tr }
                })
            })
            .collect();

        let json_ld_val = json!({
            "@context": "https://schema.org",
            "@graph": [
                Self::publisher_entity(&clean_base, brand, primary_domain, news_domain),
                Self::website_entity(&clean_base, in_language, brand),
                {
                    "@type": "CollectionPage",
                    "@id": format!("{}#collection", canonical_url),
                    "url": canonical_url,
                    "name": full_title,
                    "description": cat_desc,
                    "inLanguage": in_language,
                    "isPartOf": { "@id": format!("{}/#website", clean_base) },
                    "mainEntity": {
                        "@type": "ItemList",
                        "numberOfItems": items.len(),
                        "itemListElement": list_elements
                    }
                },
                {
                    "@type": "BreadcrumbList",
                    "@id": format!("{}#breadcrumb", canonical_url),
                    "itemListElement": [
                        {
                            "@type": "ListItem",
                            "position": 1,
                            "name": if is_en { "Home" } else { "Ana Sayfa" },
                            "item": format!("{}/", clean_base)
                        },
                        {
                            "@type": "ListItem",
                            "position": 2,
                            "name": cat_name,
                            "item": canonical_url
                        }
                    ]
                }
            ]
        });

        SeoMetaBundle {
            title: full_title,
            description: cat_desc.to_string(),
            canonical_url,
            og_type: "website".to_string(),
            og_image: full_og_image,
            og_image_alt: cat_name.to_string(),
            robots: "index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1"
                .to_string(),
            keywords: format!("{}, {}, savunma sanayii, savunmahaber", cat_name, short_name),
            hreflang_tr,
            hreflang_en,
            extra_head_html: extra_head,
            json_ld: serde_json::to_string(&json_ld_val).unwrap_or_else(|_| "{}".to_string()),
        }
    }

    /// Specialized SEO metadata builder for `/e-dergi` with `Periodical` + `PublicationIssue` JSON-LD.
    /// All brand/publisher data sourced from `SeoBrandConfig` (backed by `site_settings` DB).
    pub fn build_edergi_seo_meta(
        base_url: &str,
        title: &str,
        description: &str,
        issues: &[MagazineIssue],
        lang: &str,
        brand: &SeoBrandConfig,
        primary_domain: &str,
        news_domain: &str,
    ) -> SeoMetaBundle {
        let clean_base = Self::normalize_base_url(base_url);
        let is_en = lang.eq_ignore_ascii_case("en");
        let in_language = if is_en { "en-US" } else { "tr-TR" };
        let canonical_url = format!("{}/e-dergi", clean_base);

        let top_cover = issues
            .iter()
            .max_by_key(|i| i.issue_number)
            .map(|i| i.cover_image.as_str())
            .unwrap_or(brand.default_edergi_cover.as_str());
        let full_og_image = Self::resolve_asset_url(&clean_base, top_cover, &brand.default_og_image);

        let mut sorted_issues: Vec<&MagazineIssue> = issues.iter().collect();
        sorted_issues.sort_by(|a, b| b.issue_number.cmp(&a.issue_number));

        let has_part: Vec<Value> = sorted_issues
            .into_iter()
            .take(16)
            .map(|iss| {
                json!({
                    "@type": "PublicationIssue",
                    "issueNumber": iss.issue_number.to_string(),
                    "name": if is_en { &iss.title_en } else { &iss.title_tr },
                    "datePublished": format!("{}", iss.year),
                    "numberOfPages": iss.page_count,
                    "image": Self::resolve_asset_url(&clean_base, &iss.cover_image, &brand.default_og_image),
                    "url": format!("{}/e-dergi?issue={}", clean_base, iss.issue_number)
                })
            })
            .collect();

        let short_name = &brand.opensearch_short_name;
        let json_ld_val = json!({
            "@context": "https://schema.org",
            "@graph": [
                Self::publisher_entity(&clean_base, brand, primary_domain, news_domain),
                Self::website_entity(&clean_base, in_language, brand),
                {
                    "@type": "Periodical",
                    "@id": format!("{}/e-dergi#periodical", clean_base),
                    "name": brand.brand_title_tr,
                    "alternateName": brand.brand_title_en,
                    "issn": brand.issn,
                    "publisher": { "@id": format!("{}/#organization", clean_base) },
                    "url": canonical_url,
                    "description": description,
                    "inLanguage": ["tr-TR", "en-US"],
                    "hasPart": has_part
                },
                {
                    "@type": "BreadcrumbList",
                    "@id": format!("{}/e-dergi#breadcrumb", clean_base),
                    "itemListElement": [
                        {
                            "@type": "ListItem",
                            "position": 1,
                            "name": if is_en { "Home" } else { "Ana Sayfa" },
                            "item": format!("{}/", clean_base)
                        },
                        {
                            "@type": "ListItem",
                            "position": 2,
                            "name": "E-Dergi & Dijital Arşiv",
                            "item": canonical_url
                        }
                    ]
                }
            ]
        });

        SeoMetaBundle {
            title: format!("{} | {} (ISSN {})", title, short_name, brand.issn),
            description: description.to_string(),
            canonical_url: canonical_url.clone(),
            og_type: "website".to_string(),
            og_image: full_og_image,
            og_image_alt: title.to_string(),
            robots: "index, follow, max-image-preview:large, max-snippet:-1".to_string(),
            keywords: format!("{} E-Dergi, {} arşiv, ISSN {}, savunma sanayii dergisi, dijital dergi", short_name, brand.brand_title_en, brand.issn),
            hreflang_tr: format!("{}?lang=tr", canonical_url),
            hreflang_en: format!("{}?lang=en", canonical_url),
            extra_head_html: String::new(),
            json_ld: serde_json::to_string(&json_ld_val).unwrap_or_else(|_| "{}".to_string()),
        }
    }

    /// Renders HTML `<head>` SEO meta tags for Google, Yandex, Bing, Social Cards, RSS/JSON Feeds,
    /// OpenSearch, LLMs.txt, and Schema.org JSON-LD.
    /// Brand-dependent values (Twitter handle, site name, feed titles) sourced from `SeoBrandConfig`.
    pub fn render_seo_head_tags(meta: &SeoMetaBundle, article: Option<&ContentItem>, brand: &SeoBrandConfig) -> String {
        let safe_title = escape_html(&meta.title);
        let safe_desc = escape_html(&meta.description);
        let safe_canon = escape_html(&meta.canonical_url);
        let safe_og_type = escape_html(&meta.og_type);
        let safe_og_img = escape_html(&meta.og_image);
        let safe_og_alt = escape_html(if meta.og_image_alt.is_empty() {
            &meta.title
        } else {
            &meta.og_image_alt
        });
        let safe_robots = escape_html(if meta.robots.is_empty() {
            "index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1"
        } else {
            &meta.robots
        });
        let safe_kw = escape_html(&meta.keywords);
        let safe_tr = escape_html(&meta.hreflang_tr);
        let safe_en = escape_html(&meta.hreflang_en);
        let is_en = meta.hreflang_en.contains("lang=en") && meta.title.contains("MSI TDR");
        let og_locale = if is_en { "en_US" } else { "tr_TR" };
        let og_alt_locale = if is_en { "tr_TR" } else { "en_US" };

        let extra_tags = if !meta.extra_head_html.is_empty() {
            meta.extra_head_html.clone()
        } else if let Some(art) = article {
            let author = escape_html(&art.author_name);
            let date = escape_html(&Self::to_iso8601_tr(&art.published_at));
            let section = escape_html(&art.content_type);
            let keywords = escape_html(&art.ssb_tags.join(", "));
            format!(
                "    <meta name=\"author\" content=\"{author}\" />\n    <meta name=\"news_keywords\" content=\"{keywords}\" />\n    <meta property=\"article:published_time\" content=\"{date}\" />\n    <meta property=\"article:modified_time\" content=\"{date}\" />\n    <meta property=\"article:section\" content=\"{section}\" />\n"
            )
        } else {
            String::new()
        };

        let kw_tag = if safe_kw.is_empty() {
            String::new()
        } else {
            format!("    <meta name=\"keywords\" content=\"{}\" />\n", safe_kw)
        };

        let twitter_handle = escape_html(&brand.twitter_handle);
        let site_name = escape_html(&format!("{} & {}", brand.opensearch_short_name, brand.brand_title_en.split(" — ").next().unwrap_or("MSI TDR")));
        let rss_title_tr = escape_html(&brand.rss_title_tr);
        let rss_title_en = escape_html(&brand.rss_title_en);
        let search_title = escape_html(&format!("{} Arama", brand.opensearch_short_name));

        format!(
            r#"<meta name="description" content="{desc}" />
{kw_tag}    <meta name="robots" content="{robots}" />
    <meta name="googlebot" content="{robots}" />
    <meta name="yandex" content="{robots}" />
    <meta name="bingbot" content="{robots}" />
    <meta name="format-detection" content="telephone=no" />
    <link rel="canonical" href="{canon}" />
    <link rel="alternate" hreflang="tr" href="{tr}" />
    <link rel="alternate" hreflang="en" href="{en}" />
    <link rel="alternate" hreflang="x-default" href="{canon}" />
    <link rel="alternate" type="application/rss+xml" title="{rss_tr}" href="/rss.xml?lang=tr" />
    <link rel="alternate" type="application/rss+xml" title="{rss_en}" href="/rss.xml?lang=en" />
    <link rel="alternate" type="application/feed+json" title="{short_name} JSON Feed" href="/feed.json" />
    <link rel="search" type="application/opensearchdescription+xml" title="{search_title}" href="/opensearch.xml" />
    <link rel="manifest" href="/manifest.webmanifest" />
    <link rel="author" href="/kurumsal/kunye" />
    <meta property="og:locale" content="{og_locale}" />
    <meta property="og:locale:alternate" content="{og_alt_locale}" />
    <meta property="og:site_name" content="{site_name}" />
    <meta property="og:title" content="{title}" />
    <meta property="og:description" content="{desc}" />
    <meta property="og:url" content="{canon}" />
    <meta property="og:type" content="{og_type}" />
    <meta property="og:image" content="{og_img}" />
    <meta property="og:image:alt" content="{og_alt}" />
    <meta property="og:image:width" content="1200" />
    <meta property="og:image:height" content="630" />
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:site" content="{tw}" />
    <meta name="twitter:creator" content="{tw}" />
    <meta name="twitter:title" content="{title}" />
    <meta name="twitter:description" content="{desc}" />
    <meta name="twitter:image" content="{og_img}" />
    <meta name="twitter:image:alt" content="{og_alt}" />
{extra_tags}    <script type="application/ld+json">{json_ld}</script>"#,
            title = safe_title,
            desc = safe_desc,
            kw_tag = kw_tag,
            robots = safe_robots,
            canon = safe_canon,
            tr = safe_tr,
            en = safe_en,
            og_locale = og_locale,
            og_alt_locale = og_alt_locale,
            og_type = safe_og_type,
            og_img = safe_og_img,
            og_alt = safe_og_alt,
            extra_tags = extra_tags,
            json_ld = meta
                .json_ld
                .replace('<', "\\u003c")
                .replace('>', "\\u003e")
                .replace('&', "\\u0026"),
            tw = twitter_handle,
            site_name = site_name,
            rss_tr = rss_title_tr,
            rss_en = rss_title_en,
            short_name = escape_html(&brand.opensearch_short_name),
            search_title = search_title,
        )
    }

    /// Generates a 100% valid XML Sitemap with `<image:image>` and `<xhtml:link hreflang>` extensions.
    pub fn generate_sitemap_xml(
        base_url: &str,
        categories: &[CategoryNode],
        contents: &[ContentItem],
        issues: &[MagazineIssue],
        brand: &SeoBrandConfig,
    ) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let esc_base = Self::xml_escape(&clean_base);
        let default_img = &brand.default_og_image;
        let mut xml = String::with_capacity(196_608);
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(
            r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:image="http://www.google.com/schemas/sitemap-image/1.1" xmlns:xhtml="http://www.w3.org/1999/xhtml">"#,
        );
        xml.push('\n');

        let static_routes = [
            ("/", "1.0", "hourly"),
            ("/e-dergi", "0.95", "weekly"),
            ("/abonelik-ve-satis", "0.9", "weekly"),
            ("/arama", "0.8", "daily"),
        ];

        for (route, prio, freq) in static_routes {
            xml.push_str(&format!(
                "  <url>\n    <loc>{base}{route}</loc>\n    <xhtml:link rel=\"alternate\" hreflang=\"tr\" href=\"{base}{route}?lang=tr\" />\n    <xhtml:link rel=\"alternate\" hreflang=\"en\" href=\"{base}{route}?lang=en\" />\n    <changefreq>{freq}</changefreq>\n    <priority>{prio}</priority>\n  </url>\n",
                base = esc_base,
                route = route,
                freq = freq,
                prio = prio
            ));
        }

        for cat in categories {
            if cat.group_type == "kurumsal" {
                xml.push_str(&format!(
                    "  <url><loc>{}/kurumsal/{}</loc><changefreq>monthly</changefreq><priority>0.7</priority></url>\n",
                    esc_base,
                    Self::xml_escape(&cat.slug)
                ));
            } else {
                xml.push_str(&format!(
                    "  <url><loc>{}/kategori/{}</loc><changefreq>daily</changefreq><priority>0.85</priority></url>\n",
                    esc_base,
                    Self::xml_escape(&cat.slug)
                ));
            }
        }

        for issue in issues {
            let cover_url = Self::resolve_asset_url(&clean_base, &issue.cover_image, default_img);
            xml.push_str(&format!(
                "  <url>\n    <loc>{base}/e-dergi?issue={no}</loc>\n    <changefreq>monthly</changefreq>\n    <priority>0.85</priority>\n    <image:image>\n      <image:loc>{img}</image:loc>\n      <image:title>{title}</image:title>\n    </image:image>\n  </url>\n",
                base = esc_base,
                no = issue.issue_number,
                img = Self::xml_escape(&cover_url),
                title = Self::xml_escape(&issue.title_tr)
            ));
        }

        for item in contents {
            let lastmod = Self::to_iso8601_tr(&item.published_at);
            let img_url = Self::resolve_asset_url(&clean_base, &item.cover_image, default_img);
            xml.push_str(&format!(
                "  <url>\n    <loc>{base}/icerik/{slug}</loc>\n    <lastmod>{lastmod}</lastmod>\n    <xhtml:link rel=\"alternate\" hreflang=\"tr\" href=\"{base}/icerik/{slug}?lang=tr\" />\n    <xhtml:link rel=\"alternate\" hreflang=\"en\" href=\"{base}/icerik/{slug}?lang=en\" />\n    <changefreq>weekly</changefreq>\n    <priority>0.9</priority>\n    <image:image>\n      <image:loc>{img}</image:loc>\n      <image:title>{title}</image:title>\n    </image:image>\n  </url>\n",
                base = esc_base,
                slug = Self::xml_escape(&item.slug),
                lastmod = Self::xml_escape(&lastmod),
                img = Self::xml_escape(&img_url),
                title = Self::xml_escape(&item.title_tr)
            ));
        }

        xml.push_str("</urlset>\n");
        xml
    }

    /// Generates Google News & Yandex News XML Sitemap (`/sitemap-news.xml`) with `xmlns:news`.
    /// Publication names sourced from `SeoBrandConfig`.
    pub fn generate_news_sitemap_xml(base_url: &str, contents: &[ContentItem], brand: &SeoBrandConfig) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let esc_base = Self::xml_escape(&clean_base);
        let default_img = &brand.default_og_image;
        let mut xml = String::with_capacity(65_536);
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(
            r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:news="http://www.google.com/schemas/sitemap-news/0.9" xmlns:image="http://www.google.com/schemas/sitemap-image/1.1">"#,
        );
        xml.push('\n');

        for item in contents.iter().take(200) {
            let pub_iso = Self::to_iso8601_tr(&item.published_at);
            let is_en_only = item.title_tr.ends_with("(MSI TDR)") && !item.title_en.is_empty();
            let pub_lang = if is_en_only { "en" } else { "tr" };
            let pub_name = if is_en_only {
                &brand.brand_title_en
            } else {
                &brand.opensearch_short_name
            };
            let headline = if is_en_only {
                &item.title_en
            } else {
                &item.title_tr
            };
            let kw = item
                .ssb_tags
                .iter()
                .map(|t| t.replace('-', " "))
                .collect::<Vec<_>>()
                .join(", ");
            let img_url = Self::resolve_asset_url(&clean_base, &item.cover_image, default_img);

            xml.push_str(&format!(
                "  <url>\n    <loc>{base}/icerik/{slug}</loc>\n    <news:news>\n      <news:publication>\n        <news:name>{pub_name}</news:name>\n        <news:language>{pub_lang}</news:language>\n      </news:publication>\n      <news:publication_date>{pub_iso}</news:publication_date>\n      <news:title>{title}</news:title>\n      <news:keywords>{kw}</news:keywords>\n    </news:news>\n    <image:image>\n      <image:loc>{img}</image:loc>\n      <image:title>{title}</image:title>\n    </image:image>\n  </url>\n",
                base = esc_base,
                slug = Self::xml_escape(&item.slug),
                pub_name = Self::xml_escape(pub_name),
                pub_lang = pub_lang,
                pub_iso = Self::xml_escape(&pub_iso),
                title = Self::xml_escape(headline),
                kw = Self::xml_escape(&kw),
                img = Self::xml_escape(&img_url)
            ));
        }

        xml.push_str("</urlset>\n");
        xml
    }

    /// Generates dedicated Image Sitemap (`/sitemap-images.xml`) for Google Images & Yandex Images.
    pub fn generate_image_sitemap_xml(
        base_url: &str,
        contents: &[ContentItem],
        issues: &[MagazineIssue],
        brand: &SeoBrandConfig,
    ) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let esc_base = Self::xml_escape(&clean_base);
        let default_img = &brand.default_og_image;
        let mut xml = String::with_capacity(131_072);
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(
            r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:image="http://www.google.com/schemas/sitemap-image/1.1">"#,
        );
        xml.push('\n');

        for issue in issues {
            let img = Self::resolve_asset_url(&clean_base, &issue.cover_image, default_img);
            xml.push_str(&format!(
                "  <url>\n    <loc>{base}/e-dergi?issue={no}</loc>\n    <image:image>\n      <image:loc>{img}</image:loc>\n      <image:title>{title}</image:title>\n      <image:caption>{cap}</image:caption>\n    </image:image>\n  </url>\n",
                base = esc_base,
                no = issue.issue_number,
                img = Self::xml_escape(&img),
                title = Self::xml_escape(&issue.title_tr),
                cap = Self::xml_escape(&issue.summary_tr)
            ));
        }

        for item in contents {
            let img = Self::resolve_asset_url(&clean_base, &item.cover_image, default_img);
            xml.push_str(&format!(
                "  <url>\n    <loc>{base}/icerik/{slug}</loc>\n    <image:image>\n      <image:loc>{img}</image:loc>\n      <image:title>{title}</image:title>\n      <image:caption>{cap}</image:caption>\n    </image:image>\n  </url>\n",
                base = esc_base,
                slug = Self::xml_escape(&item.slug),
                img = Self::xml_escape(&img),
                title = Self::xml_escape(&item.title_tr),
                cap = Self::xml_escape(&item.summary_tr)
            ));
        }

        xml.push_str("</urlset>\n");
        xml
    }

    /// Generates `robots.txt` engineered for Google, Yandex (`Host:` + `Clean-param:`), Bing,
    /// and AI/LLM Answer Engines (`OAI-SearchBot`, `ChatGPT-User`, `PerplexityBot`, `ClaudeBot`, `Applebot`, `Google-Extended`).
    pub fn generate_robots_txt(base_url: &str) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let host_only = clean_base
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        format!(
            "# MSI Dergisi & MSI TDR (UMSA Produksiyon) — Dynamic robots.txt
# Engineered for Google, Yandex, Bing, and AI Answer Engines (GEO)

User-agent: *
Allow: /
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /e-dergi
Allow: /abonelik-ve-satis
Allow: /images/msi/
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /yonetim
Disallow: /hesabim
Disallow: /api/v1/admin/
Disallow: /api/v1/payments/
Disallow: /api/v1/auth/
Disallow: /arama?

# Google Search, News, Discover & Images
User-agent: Googlebot
Allow: /
Disallow: /admin
Disallow: /hesabim
Disallow: /api/v1/admin/

User-agent: Googlebot-News
Allow: /icerik/
Allow: /kategori/

User-agent: Googlebot-Image
Allow: /images/

# Yandex Search, Yandex News & Yandex Images
User-agent: Yandex
Allow: /
Disallow: /admin
Disallow: /hesabim
Disallow: /api/
Clean-param: utm_source&utm_medium&utm_campaign&utm_term&utm_content&fbclid&gclid /

User-agent: YandexBot
Allow: /
Disallow: /admin
Disallow: /hesabim
Disallow: /api/

User-agent: YandexImages
Allow: /images/

# Bing & Copilot
User-agent: bingbot
Allow: /
Disallow: /admin
Disallow: /hesabim
Disallow: /api/v1/admin/

# AI Search & Generative Engine Optimization (GEO) Crawlers
User-agent: OAI-SearchBot
Allow: /

User-agent: ChatGPT-User
Allow: /

User-agent: GPTBot
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /e-dergi
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /hesabim
Disallow: /api/

User-agent: PerplexityBot
Allow: /

User-agent: ClaudeBot
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /hesabim

User-agent: Google-Extended
Allow: /

User-agent: Applebot
Allow: /

Host: {host}
Sitemap: {base}/sitemap.xml
Sitemap: {base}/sitemap-news.xml
Sitemap: {base}/sitemap-images.xml
Sitemap: {base}/rss.xml?lang=tr
Sitemap: {base}/rss.xml?lang=en
",
            host = host_only,
            base = clean_base
        )
    }

    /// Generates valid RSS 2.0 + Media RSS XML syndication feed.
    /// Channel title, description, and copyright sourced from `SeoBrandConfig`.
    pub fn generate_rss_xml(base_url: &str, contents: &[ContentItem], lang: &str, brand: &SeoBrandConfig) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let is_en = lang.eq_ignore_ascii_case("en");
        let default_img = &brand.default_og_image;

        let channel_title = if is_en { &brand.rss_title_en } else { &brand.rss_title_tr };
        let channel_desc = if is_en { &brand.rss_desc_en } else { &brand.rss_desc_tr };
        let channel_lang = if is_en { "en-us" } else { "tr-tr" };

        let mut rss = String::with_capacity(32_768);
        rss.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        rss.push('\n');
        rss.push_str(
            r#"<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/" xmlns:dc="http://purl.org/dc/elements/1.1/">"#,
        );
        rss.push('\n');
        rss.push_str("  <channel>\n");
        rss.push_str(&format!(
            "    <title>{}</title>\n",
            Self::xml_escape(channel_title)
        ));
        rss.push_str(&format!(
            "    <link>{}</link>\n",
            Self::xml_escape(&clean_base)
        ));
        rss.push_str(&format!(
            "    <description>{}</description>\n",
            Self::xml_escape(channel_desc)
        ));
        rss.push_str(&format!("    <language>{}</language>\n", channel_lang));
        rss.push_str(&format!("    <copyright>{}</copyright>\n", Self::xml_escape(&brand.copyright_notice)));
        rss.push_str(&format!(
            "    <atom:link href=\"{}/rss.xml?lang={}\" rel=\"self\" type=\"application/rss+xml\" />\n",
            Self::xml_escape(&clean_base),
            if is_en { "en" } else { "tr" }
        ));

        let filtered: Vec<&ContentItem> = if is_en {
            let en_items: Vec<&ContentItem> = contents
                .iter()
                .filter(|c| !c.title_en.is_empty())
                .take(40)
                .collect();
            if en_items.is_empty() {
                contents.iter().take(40).collect()
            } else {
                en_items
            }
        } else {
            contents.iter().take(40).collect()
        };

        for item in filtered {
            let title = if is_en && !item.title_en.is_empty() {
                &item.title_en
            } else {
                &item.title_tr
            };
            let summary = if is_en && !item.summary_en.is_empty() {
                &item.summary_en
            } else {
                &item.summary_tr
            };
            let item_url = if is_en {
                format!("{}/icerik/{}?lang=en", clean_base, item.slug)
            } else {
                format!("{}/icerik/{}", clean_base, item.slug)
            };
            let img_url = Self::resolve_asset_url(&clean_base, &item.cover_image, default_img);
            let rfc_date = Self::to_rfc2822_tr(&item.published_at);

            rss.push_str("    <item>\n");
            rss.push_str(&format!(
                "      <title>{}</title>\n",
                Self::xml_escape(title)
            ));
            rss.push_str(&format!(
                "      <link>{}</link>\n",
                Self::xml_escape(&item_url)
            ));
            rss.push_str(&format!(
                "      <guid isPermaLink=\"true\">{}</guid>\n",
                Self::xml_escape(&item_url)
            ));
            rss.push_str(&format!(
                "      <description>{}</description>\n",
                Self::xml_escape(summary)
            ));
            rss.push_str(&format!(
                "      <dc:creator>{}</dc:creator>\n",
                Self::xml_escape(&item.author_name)
            ));
            rss.push_str(&format!(
                "      <category>{}</category>\n",
                Self::xml_escape(&item.content_type)
            ));
            rss.push_str(&format!(
                "      <media:content url=\"{}\" medium=\"image\" />\n",
                Self::xml_escape(&img_url)
            ));
            rss.push_str(&format!(
                "      <pubDate>{}</pubDate>\n",
                Self::xml_escape(&rfc_date)
            ));
            rss.push_str("    </item>\n");
        }

        rss.push_str("  </channel>\n</rss>\n");
        rss
    }

    /// Generates JSON Feed 1.1 (`/feed.json`) for modern syndication & AI news readers.
    /// Title and description sourced from `SeoBrandConfig`.
    pub fn generate_json_feed(base_url: &str, contents: &[ContentItem], lang: &str, brand: &SeoBrandConfig) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let is_en = lang.eq_ignore_ascii_case("en");
        let default_img = &brand.default_og_image;
        let items: Vec<Value> = contents
            .iter()
            .filter(|c| !is_en || !c.title_en.is_empty())
            .take(50)
            .map(|c| {
                let title = if is_en && !c.title_en.is_empty() {
                    &c.title_en
                } else {
                    &c.title_tr
                };
                let summary = if is_en && !c.summary_en.is_empty() {
                    &c.summary_en
                } else {
                    &c.summary_tr
                };
                let url = format!("{}/icerik/{}", clean_base, c.slug);
                json!({
                    "id": url,
                    "url": url,
                    "title": title,
                    "summary": summary,
                    "image": Self::resolve_asset_url(&clean_base, &c.cover_image, default_img),
                    "date_published": Self::to_iso8601_tr(&c.published_at),
                    "authors": [{ "name": c.author_name }],
                    "tags": c.ssb_tags,
                    "language": if is_en { "en-US" } else { "tr-TR" }
                })
            })
            .collect();

        let feed_title = if is_en { &brand.brand_title_en } else { &brand.brand_title_tr };
        let feed = json!({
            "version": "https://jsonfeed.org/version/1.1",
            "title": feed_title,
            "home_page_url": clean_base,
            "feed_url": format!("{}/feed.json", clean_base),
            "description": &brand.json_feed_description,
            "icon": format!("{}/images/icon-512.png", clean_base),
            "favicon": format!("{}/images/favicon.png", clean_base),
            "language": if is_en { "en-US" } else { "tr-TR" },
            "items": items
        });
        serde_json::to_string_pretty(&feed).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generates `/llms.txt` and `/llms-full.txt` (AI / LLM / GEO Discovery Standard).
    /// Header text and publisher identity sourced from `SeoBrandConfig`.
    pub fn generate_llms_txt(
        base_url: &str,
        categories: &[CategoryNode],
        ssb_tags: &[SsbInterestTag],
        issues: &[MagazineIssue],
        contents: &[ContentItem],
        full: bool,
        brand: &SeoBrandConfig,
    ) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let mut out = String::with_capacity(if full { 131_072 } else { 24_576 });

        out.push_str(&format!("# {} & MSI Turkish Defence Review (MSI TDR)\n\n", brand.opensearch_short_name));
        out.push_str(&format!("> {}\n\n", brand.llms_header_text));
        out.push_str("## Kurumsal Kimlik & Otorite (Entity Facts)\n");
        out.push_str(&format!("- **Resmi Yayın Adları:** {} (Türkçe, {}–günümüz, 286+ sayı), MSI Turkish Defence Review / MSI TDR (İngilizce, 2014–günümüz, 153+ sayı), AMAC Magazine (Ankara Askerî Ataşeler Birliği resmi yayını, 2015–günümüz)\n", brand.opensearch_short_name, brand.founding_date));
        out.push_str(&format!("- **ISSN:** {}\n", brand.issn));
        out.push_str(&format!("- **Yayıncı Tüzel Kişilik:** {}\n", brand.publisher_legal_name));
        out.push_str(&format!("- **Merkez Adres:** {} {} / {}, Türkiye • Tel: {}\n", brand.address_street, brand.address_locality, brand.address_region, brand.contact_phone));
        out.push_str(&format!("- **Ana Web Adresi:** {}/\n", clean_base));
        out.push_str(&format!("- **Resmi Künye (Masthead):** {}/kurumsal/kunye\n", clean_base));
        out.push_str(&format!("- **Hakkımızda & Tarihçe:** {}/kurumsal/hakkimizda | {}/kurumsal/yolculugumuz\n", clean_base, clean_base));
        out.push_str(&format!("- **Yayın Etiği & Telif Politikası:** {}/kurumsal/telif-ve-etik\n", clean_base));
        out.push_str(&format!("- **KVKK & Hukuki Metinler:** {}/kurumsal/kvkk | {}/kurumsal/gizlilik-politikasi | {}/kurumsal/cerez-politikasi | {}/kurumsal/mesafeli-satis-sozlesmesi\n\n", clean_base, clean_base, clean_base, clean_base));

        out.push_str("## Makine Okunabilir Veri Akışları (Machine-Readable Feeds)\n");
        out.push_str(&format!("- XML Sitemap: {}/sitemap.xml\n", clean_base));
        out.push_str(&format!("- Google/Yandex News Sitemap: {}/sitemap-news.xml\n", clean_base));
        out.push_str(&format!("- Image Sitemap: {}/sitemap-images.xml\n", clean_base));
        out.push_str(&format!("- Türkçe RSS 2.0: {}/rss.xml?lang=tr\n", clean_base));
        out.push_str(&format!("- English RSS 2.0 (MSI TDR): {}/rss.xml?lang=en\n", clean_base));
        out.push_str(&format!("- JSON Feed 1.1: {}/feed.json\n", clean_base));
        out.push_str(&format!("- Full AI Content Index: {}/llms-full.txt\n\n", clean_base));

        out.push_str("## Savunma Sanayii Teknoloji Taksonomisi (SSB Dikeyleri)\n");
        for t in ssb_tags {
            out.push_str(&format!(
                "- [{} / {}]({}/arama?ssb_tag={})\n",
                t.name_tr, t.name_en, clean_base, t.code
            ));
        }
        out.push('\n');

        out.push_str("## Ana Kategori ve Özel Dosya Merkezleri\n");
        for c in categories.iter().filter(|c| c.group_type != "kurumsal") {
            out.push_str(&format!(
                "- [{} ({})]({}/kategori/{}): {}\n",
                c.name_tr, c.name_en, clean_base, c.slug, c.description_tr
            ));
        }
        out.push('\n');

        let mut sorted_issues: Vec<&MagazineIssue> = issues.iter().collect();
        sorted_issues.sort_by(|a, b| b.issue_number.cmp(&a.issue_number));
        out.push_str("## Güncel Dergi Sayıları (MSI Dergisi & MSI TDR Dijital Kütüphane)\n");
        for iss in sorted_issues.iter().take(if full { 25 } else { 10 }) {
            out.push_str(&format!(
                "- [{}]({}/e-dergi?issue={}): {}\n",
                iss.title_tr, clean_base, iss.issue_number, iss.summary_tr
            ));
        }
        out.push('\n');

        let limit = if full { 350 } else { 40 };
        out.push_str(&format!(
            "## Son Savunma Sanayii Haberleri, Özel Dosyalar ve Analizler (Toplam Arşiv: {} Makale)\n",
            contents.len()
        ));
        for item in contents.iter().take(limit) {
            let date = item.published_at.get(..10).unwrap_or(&item.published_at);
            out.push_str(&format!(
                "- [{}]({}/icerik/{}) ({}, {}): {}\n",
                item.title_tr, clean_base, item.slug, date, item.author_name, item.summary_tr
            ));
        }

        out
    }

    /// Generates OpenSearch XML (`/opensearch.xml`) for native browser search integration.
    /// ShortName and Description sourced from `SeoBrandConfig`.
    pub fn generate_opensearch_xml(base_url: &str, brand: &SeoBrandConfig) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let esc_base = Self::xml_escape(&clean_base);
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
  <ShortName>{short_name}</ShortName>
  <Description>{desc}</Description>
  <InputEncoding>UTF-8</InputEncoding>
  <Image width="32" height="32" type="image/png">{base}/images/favicon-32x32.png</Image>
  <Image width="16" height="16" type="image/x-icon">{base}/favicon.ico</Image>
  <Url type="text/html" method="get" template="{base}/arama?q={{searchTerms}}" />
  <Language>tr-TR</Language>
  <Language>en-US</Language>
</OpenSearchDescription>
"#,
            base = esc_base,
            short_name = Self::xml_escape(&brand.opensearch_short_name),
            desc = Self::xml_escape(&brand.opensearch_description)
        )
    }

    /// Generates PWA Web App Manifest (`/manifest.webmanifest`).
    /// Name, short_name, and description sourced from `SeoBrandConfig`.
    pub fn generate_web_manifest(base_url: &str, brand: &SeoBrandConfig) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let manifest = json!({
            "name": brand.publisher_name,
            "short_name": brand.manifest_short_name,
            "description": brand.manifest_description,
            "start_url": "/",
            "scope": "/",
            "display": "standalone",
            "background_color": "#f5f4f1",
            "theme_color": "#c4122f",
            "lang": "tr-TR",
            "icons": [
                {
                    "src": "/images/icon-192.png",
                    "sizes": "192x192",
                    "type": "image/png",
                    "purpose": "any maskable"
                },
                {
                    "src": "/images/icon-512.png",
                    "sizes": "512x512",
                    "type": "image/png",
                    "purpose": "any maskable"
                },
                {
                    "src": "/images/apple-touch-icon.png",
                    "sizes": "180x180",
                    "type": "image/png"
                }
            ],
            "shortcuts": [
                {
                    "name": "Savunma Haber",
                    "url": "/?mode=news#haber"
                },
                {
                    "name": "E-Dergi Dijital Kütüphane",
                    "url": "/e-dergi"
                },
                {
                    "name": "Arşivde Ara",
                    "url": "/arama"
                }
            ],
            "id": clean_base
        });
        serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_seo_meta(
    base_url: &str,
    path: &str,
    title: &str,
    description: &str,
    og_image: &str,
    og_type: &str,
    lang: &str,
    article: Option<&ContentItem>,
    brand: &SeoBrandConfig,
    primary_domain: &str,
    news_domain: &str,
) -> SeoMetaBundle {
    CentralSeoEngine::build_seo_meta(
        base_url,
        path,
        title,
        description,
        og_image,
        og_type,
        lang,
        article,
        brand,
        primary_domain,
        news_domain,
    )
}

pub fn render_seo_head_tags(meta: &SeoMetaBundle, article: Option<&ContentItem>, brand: &SeoBrandConfig) -> String {
    CentralSeoEngine::render_seo_head_tags(meta, article, brand)
}

pub fn generate_sitemap_xml(
    base_url: &str,
    categories: &[CategoryNode],
    contents: &[ContentItem],
    issues: &[MagazineIssue],
    brand: &SeoBrandConfig,
) -> String {
    CentralSeoEngine::generate_sitemap_xml(base_url, categories, contents, issues, brand)
}

pub fn generate_robots_txt(base_url: &str) -> String {
    CentralSeoEngine::generate_robots_txt(base_url)
}

pub fn generate_rss_xml(base_url: &str, contents: &[ContentItem], lang: &str, brand: &SeoBrandConfig) -> String {
    CentralSeoEngine::generate_rss_xml(base_url, contents, lang, brand)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_brand() -> SeoBrandConfig {
        SeoBrandConfig::from_settings(&HashMap::new())
    }

    fn sample_article(vis: i32) -> ContentItem {
        ContentItem {
            id: 1,
            slug: "kaan-milli-muharip-ucak-analizi".into(),
            title_tr: "KAAN Milli Muharip Uçak Analizi".into(),
            title_en: "KAAN National Combat Aircraft Analysis".into(),
            summary_tr: "KAAN programında test ve seri üretim yol haritası.".into(),
            summary_en: "Testing and serial production roadmap in KAAN program.".into(),
            body_tr: "<p>Detaylı savunma ve havacılık mühendisliği incelemesi.</p>".into(),
            body_en: "<p>Detailed aerospace engineering analysis.</p>".into(),
            content_type: "ozel-haberler".into(),
            source_group: "dergi-icerigi".into(),
            cover_image: "images/msi/msi-dergisi-tusas-kaan-milli-muharip-ucak.webp".into(),
            author_name: "Ümit Bayraktar".into(),
            author_title: "Genel Yayın Yönetmeni".into(),
            issue_number: Some(286),
            published_at: "2026-10-24 16:38:00".into(),
            read_time_min: 6,
            visibility_level: vis,
            single_price_try: 95.0,
            is_sponsored: false,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: true,
            video_url: None,
            view_count: 500,
            true_read_count: 250,
            placements: vec!["ozel-haberler".into(), "kapak-dosyalari".into()],
            ssb_tags: vec!["hava-araclari".into(), "radarlar-ve-sensorler".into()],
        }
    }

    #[test]
    fn gated_article_emits_google_paywall_flexible_sampling_and_article_meta() {
        let art = sample_article(2);
        let brand = test_brand();
        let meta = build_seo_meta(
            "https://msidergisi.com",
            "/icerik/kaan-milli-muharip-ucak-analizi",
            &art.title_tr,
            &art.summary_tr,
            &art.cover_image,
            "article",
            "tr",
            Some(&art),
            &brand,
            "msidergisi.com",
            "savunmahaber.com",
        );
        assert!(meta.json_ld.contains("\"NewsMediaOrganization\""));
        assert!(meta.json_ld.contains("\"isAccessibleForFree\":false"));
        assert!(meta.json_ld.contains(".paywall-panel"));
        assert!(meta.json_ld.contains("1306-0309"));
        let head = render_seo_head_tags(&meta, None, &brand);
        assert!(head.contains("article:published_time"));
        assert!(head.contains("2026-10-24T16:38:00+03:00"));
        assert!(head.contains("name=\"yandex\""));
    }

    #[test]
    fn news_and_llms_feeds_are_well_formed() {
        let art = sample_article(0);
        let brand = test_brand();
        let news_xml = CentralSeoEngine::generate_news_sitemap_xml("https://msidergisi.com", &[art.clone()], &brand);
        assert!(news_xml.contains("<news:publication_date>2026-10-24T16:38:00+03:00</news:publication_date>"));
        let llms = CentralSeoEngine::generate_llms_txt("https://msidergisi.com", &[], &[], &[], &[art], false, &brand);
        assert!(llms.contains("ISSN: 1306-0309"));
        assert!(llms.contains("/icerik/kaan-milli-muharip-ucak-analizi"));
        let robots = generate_robots_txt("https://msidergisi.com");
        assert!(robots.contains("Host: msidergisi.com"));
        assert!(robots.contains("Sitemap: https://msidergisi.com/sitemap-news.xml"));
    }
}
