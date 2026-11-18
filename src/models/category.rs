use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_dir() -> String {
    "ltr".to_string()
}

fn default_fallback_tr() -> String {
    "tr".to_string()
}

fn default_rule_type() -> String {
    "fixed".to_string()
}

fn default_match_mode() -> String {
    "suffix".to_string()
}

fn default_domain_kind() -> String {
    "public_sector".to_string()
}

fn default_grants_role() -> String {
    "registered".to_string()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CategoryNode {
    #[serde(default)]
    pub id: i64,
    pub slug: String,
    #[serde(default)]
    pub parent_slug: Option<String>,
    pub name_tr: String,
    #[serde(default)]
    pub name_en: String,
    /// Group: `icerikler`, `gruplandirilmis`, `abonelik`, `kurumsal`, `gelecek-faz`
    #[serde(default)]
    pub group_type: String,
    #[serde(default)]
    pub description_tr: String,
    #[serde(default)]
    pub description_en: String,
    #[serde(default)]
    pub icon_class: String,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default = "default_true")]
    pub is_dynamic: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SsbInterestTag {
    #[serde(default)]
    pub id: i64,
    pub code: String,
    pub name_tr: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub category_group: String,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LocaleConfig {
    pub code: String,
    pub name_native: String,
    pub name_en: String,
    pub hreflang: String,
    pub og_locale: String,
    #[serde(default = "default_dir")]
    pub dir: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub is_enabled: bool,
    #[serde(default = "default_fallback_tr")]
    pub fallback_code: String,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TranslationGlossaryTerm {
    #[serde(default)]
    pub id: i64,
    pub term_tr: String,
    pub term_en: String,
    #[serde(default)]
    pub abbreviation_tr: String,
    #[serde(default)]
    pub abbreviation_en: String,
    pub domain: String,
    #[serde(default = "default_rule_type")]
    pub rule_type: String,
    pub preferred_en: String,
    #[serde(default)]
    pub forbidden_variants: Vec<String>,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub source_ref: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub created_by: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InstitutionCategory {
    pub code: String,
    pub legacy_value: String,
    pub label_tr: String,
    pub label_en: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub is_public_sector: bool,
    #[serde(default = "default_true")]
    pub requires_verification: bool,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GovEmailDomain {
    #[serde(default)]
    pub id: i64,
    pub domain: String,
    #[serde(default = "default_match_mode")]
    pub match_mode: String,
    #[serde(default = "default_domain_kind")]
    pub domain_kind: String,
    #[serde(default)]
    pub institution_code: Option<String>,
    #[serde(default = "default_grants_role")]
    pub grants_role: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EmailGroup {
    #[serde(default)]
    pub id: i64,
    pub code: String,
    pub name_tr: String,
    pub name_en: String,
    #[serde(default)]
    pub description_tr: String,
    #[serde(default)]
    pub description_en: String,
    #[serde(default)]
    pub filter_json: String,
    #[serde(default)]
    pub static_emails_json: String,
    #[serde(default = "default_true")]
    pub requires_marketing_consent: bool,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub created_by: String,
    #[serde(default)]
    pub updated_at: String,
}