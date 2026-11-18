use super::{CentralSeoEngine, SeoBrandConfig};
use crate::models::ContentItem;
use serde_json::{json, Value};

impl CentralSeoEngine {
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
                .take(crate::db::constants::SITEMAP_PREVIEW_LIMIT)
                .collect();
            if en_items.is_empty() {
                contents.iter().take(crate::db::constants::SITEMAP_PREVIEW_LIMIT).collect()
            } else {
                en_items
            }
        } else {
            contents.iter().take(crate::db::constants::SITEMAP_PREVIEW_LIMIT).collect()
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
                "      <description><![CDATA[{}]]></description>\n",
                summary
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
            .take(crate::db::constants::SITEMAP_NEWSLETTER_LIMIT)
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
                {
                    let body_html = if is_en && !c.body_en.is_empty() {
                        &c.body_en
                    } else {
                        &c.body_tr
                    };
                    json!({
                        "id": url,
                        "url": url,
                        "title": title,
                        "summary": summary,
                        "content_html": body_html,
                        "image": Self::resolve_asset_url(&clean_base, &c.cover_image, default_img),
                        "date_published": Self::to_iso8601_tr(&c.published_at),
                        "authors": [{ "name": c.author_name }],
                        "tags": c.ssb_tags,
                        "language": if is_en { "en-US" } else { "tr-TR" }
                    })
                }
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
}
