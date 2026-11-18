use super::{CentralSeoEngine, SeoBrandConfig};
use serde_json::json;

impl CentralSeoEngine {
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
