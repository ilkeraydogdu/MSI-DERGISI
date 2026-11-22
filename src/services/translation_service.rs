//! Auto-Translate Service
//!
//! Provides machine translation via configurable external providers (LibreTranslate / DeepL)
//! with glossary-aware quality checking. Stateless: reads provider config from site_settings
//! on each call so admins can switch providers without restart.

use serde_json::Value;

/// Result of a glossary violation check, mirroring the JSON shape from
/// `Database::check_glossary_violations` but owned for service-layer use.
#[derive(Clone, Debug)]
pub struct GlossaryViolation {
    pub glossary_id: i64,
    pub domain: String,
    pub term_tr: String,
    pub forbidden: String,
    pub preferred_en: String,
    pub notes: String,
}

impl From<&Value> for GlossaryViolation {
    fn from(v: &Value) -> Self {
        Self {
            glossary_id: v["glossary_id"].as_i64().unwrap_or(0),
            domain: v["domain"].as_str().unwrap_or("").to_string(),
            term_tr: v["term_tr"].as_str().unwrap_or("").to_string(),
            forbidden: v["forbidden"].as_str().unwrap_or("").to_string(),
            preferred_en: v["preferred_en"].as_str().unwrap_or("").to_string(),
            notes: v["notes"].as_str().unwrap_or("").to_string(),
        }
    }
}

/// Stateless translation service. All configuration is read from the database
/// at call time so provider switches take effect immediately.
pub struct TranslationService;

impl TranslationService {
    /// Translate text using the configured external provider.
    ///
    /// Reads `auto_translate_provider`, `auto_translate_url`, and
    /// `auto_translate_api_key` from site_settings on every call.
    pub async fn auto_translate(
        db: &crate::db::Database,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String, String> {
        if text.trim().is_empty() {
            return Ok(String::new());
        }

        let settings = db.get_setting_map("en");
        let provider = settings
            .get("auto_translate_provider")
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();

        match provider.as_str() {
            "libretranslate" => {
                let url = settings
                    .get("auto_translate_url")
                    .filter(|u| !u.trim().is_empty())
                    .ok_or_else(|| {
                        "Auto-translate provider 'libretranslate' requires 'auto_translate_url' setting."
                            .to_string()
                    })?
                    .trim()
                    .to_string();
                Self::translate_libre(&url, text, source_lang, target_lang).await
            }
            "deepl" => {
                let api_key = settings
                    .get("auto_translate_api_key")
                    .filter(|k| !k.trim().is_empty())
                    .ok_or_else(|| {
                        "Auto-translate provider 'deepl' requires 'auto_translate_api_key' setting."
                            .to_string()
                    })?
                    .trim()
                    .to_string();
                Self::translate_deepl(&api_key, text, source_lang, target_lang).await
            }
            "none" | "" => Err("Auto-translate not configured. Set 'auto_translate_provider' in site settings to 'libretranslate' or 'deepl'.".to_string()),
            other => Err(format!(
                "Unknown auto-translate provider '{}'. Supported: libretranslate, deepl, none.",
                other
            )),
        }
    }

    /// Translate then run glossary violation checks on the result.
    /// Returns the translated text and any glossary violations found.
    pub async fn translate_with_glossary(
        db: &crate::db::Database,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<(String, Vec<GlossaryViolation>), String> {
        let translated = Self::auto_translate(db, text, source_lang, target_lang).await?;

        // Only check glossary when translating into English
        let violations = if target_lang.eq_ignore_ascii_case("en") {
            db.check_glossary_violations(&translated)
                .iter()
                .map(GlossaryViolation::from)
                .collect()
        } else {
            Vec::new()
        };

        Ok((translated, violations))
    }

    // ── Provider Implementations ───────────────────────────────────────────

    async fn translate_libre(
        url: &str,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let resp = client
            .post(url)
            .form(&[
                ("q", text),
                ("source", source_lang),
                ("target", target_lang),
                ("format", "text"),
            ])
            .send()
            .await
            .map_err(|e| format!("LibreTranslate request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "LibreTranslate returned HTTP {}: {}",
                status, body
            ));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| format!("LibreTranslate response parse error: {}", e))?;

        json["translatedText"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                format!(
                    "LibreTranslate response missing 'translatedText': {}",
                    json
                )
            })
    }

    async fn translate_deepl(
        api_key: &str,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        // DeepL API expects uppercase language codes
        let src = source_lang.to_ascii_uppercase();
        let tgt = target_lang.to_ascii_uppercase();

        // Determine endpoint: free key ends with ":fx"
        let endpoint = if api_key.ends_with(":fx") {
            "https://api-free.deepl.com/v2/translate"
        } else {
            "https://api.deepl.com/v2/translate"
        };

        let resp = client
            .post(endpoint)
            .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
            .form(&[
                ("text", text),
                ("source_lang", src.as_str()),
                ("target_lang", tgt.as_str()),
            ])
            .send()
            .await
            .map_err(|e| format!("DeepL request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("DeepL returned HTTP {}: {}", status, body));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| format!("DeepL response parse error: {}", e))?;

        json["translations"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|t| t["text"].as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                format!("DeepL response missing 'translations[0].text': {}", json)
            })
    }
}