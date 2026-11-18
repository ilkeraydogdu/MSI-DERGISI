use super::{CentralSeoEngine, SeoBrandConfig};
use crate::models::{CategoryNode, ContentItem, MagazineIssue};

impl CentralSeoEngine {
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

        for item in contents.iter().take(crate::db::constants::SEARCH_MAX_RESULTS) {
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
}
