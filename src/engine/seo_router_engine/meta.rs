use super::{CentralSeoEngine, SeoBrandConfig};
use crate::engine::security_engine::escape_html;
use crate::models::{CategoryNode, ContentItem, MagazineIssue, SeoMetaBundle};
use serde_json::{json, Value};

impl CentralSeoEngine {
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
        let clean_lower = clean_path.to_ascii_lowercase();
        let robots = if clean_lower.starts_with("/hesabim")
            || clean_lower.starts_with("/podmin")
            || clean_lower.starts_with("/admin")
        {
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
            .take(crate::db::constants::HOMEPAGE_GRID_LIMIT)
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
            .take(crate::db::constants::HOMEPAGE_FEATURED_LIMIT)
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

        let is_noindex = safe_robots.to_ascii_lowercase().contains("noindex");
        let canonical_and_alternates_html = if is_noindex || safe_canon.is_empty() {
            String::new()
        } else {
            let mut tags = format!("    <link rel=\"canonical\" href=\"{}\" />\n", safe_canon);
            if !safe_tr.is_empty() {
                tags.push_str(&format!("    <link rel=\"alternate\" hreflang=\"tr\" href=\"{}\" />\n", safe_tr));
            }
            if !safe_en.is_empty() {
                tags.push_str(&format!("    <link rel=\"alternate\" hreflang=\"en\" href=\"{}\" />\n", safe_en));
            }
            tags.push_str(&format!("    <link rel=\"alternate\" hreflang=\"x-default\" href=\"{}\" />\n", safe_canon));
            tags
        };
        let og_url_html = if is_noindex || safe_canon.is_empty() {
            String::new()
        } else {
            format!("    <meta property=\"og:url\" content=\"{}\" />\n", safe_canon)
        };
        let json_ld_html = if meta.json_ld.trim().is_empty() {
            String::new()
        } else {
            let safe_ld = meta
                .json_ld
                .replace('<', "\\u003c")
                .replace('>', "\\u003e")
                .replace('&', "\\u0026");
            format!("    <script type=\"application/ld+json\">{}</script>", safe_ld)
        };

        format!(
            r#"<meta name="description" content="{desc}" />
{kw_tag}    <meta name="robots" content="{robots}" />
    <meta name="googlebot" content="{robots}" />
    <meta name="yandex" content="{robots}" />
    <meta name="bingbot" content="{robots}" />
    <meta name="format-detection" content="telephone=no" />
{canonical_and_alternates_html}    <link rel="alternate" type="application/rss+xml" title="{rss_tr}" href="/rss.xml?lang=tr" />
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
{og_url_html}    <meta property="og:type" content="{og_type}" />
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
{extra_tags}{json_ld_html}"#,
            title = safe_title,
            desc = safe_desc,
            kw_tag = kw_tag,
            robots = safe_robots,
            canonical_and_alternates_html = canonical_and_alternates_html,
            og_locale = og_locale,
            og_alt_locale = og_alt_locale,
            og_url_html = og_url_html,
            og_type = safe_og_type,
            og_img = safe_og_img,
            og_alt = safe_og_alt,
            extra_tags = extra_tags,
            json_ld_html = json_ld_html,
            tw = twitter_handle,
            site_name = site_name,
            rss_tr = rss_title_tr,
            rss_en = rss_title_en,
            short_name = escape_html(&brand.opensearch_short_name),
            search_title = search_title,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn private_routes_emit_noindex_nofollow_case_insensitively() {
        let brand = SeoBrandConfig::from_settings(&HashMap::new());
        for path in [
            "/podmin",
            "/Podmin",
            "/PODMIN/tasarim",
            "/hesabim",
            "/Hesabim",
            "/admin",
            "/Admin/login",
        ] {
            let meta = CentralSeoEngine::build_seo_meta(
                "https://msidergisi.com",
                path,
                "Panel",
                "Panel",
                "",
                "website",
                "tr",
                None,
                &brand,
                "msidergisi.com",
                "savunmahaber.com",
            );
            assert_eq!(
                meta.robots, "noindex, nofollow",
                "path {} must emit noindex, nofollow",
                path
            );
        }

        let public_meta = CentralSeoEngine::build_seo_meta(
            "https://msidergisi.com",
            "/icerik/savunma-raporu",
            "Savunma Raporu",
            "Analiz",
            "",
            "article",
            "tr",
            None,
            &brand,
            "msidergisi.com",
            "savunmahaber.com",
        );
        assert!(
            public_meta.robots.starts_with("index, follow"),
            "public path must remain indexable, got {}",
            public_meta.robots
        );
    }

    #[test]
    fn noindex_and_empty_canonical_omit_canonical_hreflang_and_og_url() {
        let brand = SeoBrandConfig::from_settings(&HashMap::new());
        let noindex_meta = SeoMetaBundle {
            title: "404 Sayfa Bulunamadı".to_string(),
            description: "Sayfa bulunamadı".to_string(),
            canonical_url: String::new(),
            hreflang_tr: String::new(),
            hreflang_en: String::new(),
            og_type: "website".to_string(),
            og_image: "/images/logo.svg".to_string(),
            robots: "noindex, nofollow".to_string(),
            json_ld: String::new(),
            ..Default::default()
        };
        let head = CentralSeoEngine::render_seo_head_tags(&noindex_meta, None, &brand);
        assert!(head.contains("<meta name=\"robots\" content=\"noindex, nofollow\" />"));
        assert!(!head.contains("rel=\"canonical\""));
        assert!(!head.contains("hreflang="));
        assert!(!head.contains("og:url"));
        assert!(!head.contains("application/ld+json"));

        // Even if a caller leaves canonical_url populated on a noindex page (e.g. /hesabim),
        // canonical/hreflang/og:url must be omitted.
        let auth_noindex = CentralSeoEngine::build_seo_meta(
            "https://msidergisi.com",
            "/hesabim",
            "Hesabım",
            "Kullanıcı Paneli",
            "",
            "website",
            "tr",
            None,
            &brand,
            "msidergisi.com",
            "savunmahaber.com",
        );
        let auth_head = CentralSeoEngine::render_seo_head_tags(&auth_noindex, None, &brand);
        assert!(auth_head.contains("<meta name=\"robots\" content=\"noindex, nofollow\" />"));
        assert!(!auth_head.contains("rel=\"canonical\""));
        assert!(!auth_head.contains("hreflang="));
        assert!(!auth_head.contains("og:url"));
        assert!(auth_head.contains("application/ld+json"));

        // Indexable public pages preserve canonical, hreflang, og:url, and JSON-LD.
        let public_meta = CentralSeoEngine::build_seo_meta(
            "https://msidergisi.com",
            "/",
            "MSI Dergisi",
            "Savunma ve Havacılık",
            "",
            "website",
            "tr",
            None,
            &brand,
            "msidergisi.com",
            "savunmahaber.com",
        );
        let public_head = CentralSeoEngine::render_seo_head_tags(&public_meta, None, &brand);
        assert!(public_head.contains("<link rel=\"canonical\" href=\"https://msidergisi.com/\" />"));
        assert!(public_head.contains("hreflang=\"tr\""));
        assert!(public_head.contains("hreflang=\"en\""));
        assert!(public_head.contains("hreflang=\"x-default\""));
        assert!(public_head.contains("property=\"og:url\""));
        assert!(public_head.contains("application/ld+json"));
    }
}
