use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const SETTING_KEY: &str = "design_config";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub background: String,
    pub surface: String,
    pub subtle: String,
    pub text: String,
    pub muted: String,
    pub border: String,
    pub accent: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub light: Palette,
    pub dark: Palette,
    pub width: u16,
    pub radius: u8,
    pub font_size: u8,
    pub default_mode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationItem {
    pub label: String,
    pub href: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    MagazineHero,
    NewsHero,
    Articles,
    Issues,
    Topics,
    Banner,
    Text,
    Cta,
    Plans,
    Products,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Section {
    pub kind: SectionKind,
    pub title: String,
    pub eyebrow: String,
    pub description: String,
    pub source: String,
    pub category: String,
    pub layout: String,
    pub limit: usize,
    pub offset: usize,
    pub enabled: bool,
    pub link: String,
    pub link_label: String,
}
impl Default for Section {
    fn default() -> Self {
        Self {
            kind: SectionKind::Articles,
            title: String::new(),
            eyebrow: String::new(),
            description: String::new(),
            source: "all".into(),
            category: String::new(),
            layout: "grid".into(),
            limit: 6,
            offset: 0,
            enabled: true,
            link: String::new(),
            link_label: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignConfig {
    pub version: u8,
    pub theme: Theme,
    #[serde(default)]
    pub templates: Templates,
    pub navigation: Vec<NavigationItem>,
    pub pages: Vec<Page>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Templates {
    pub article_layout: String,
    pub category_layout: String,
    pub search_layout: String,
    pub reading_font: String,
}
impl Default for Templates {
    fn default() -> Self {
        Self {
            article_layout: "sidebar".into(),
            category_layout: "grid".into(),
            search_layout: "grid".into(),
            reading_font: "serif".into(),
        }
    }
}

pub fn defaults() -> DesignConfig {
    serde_json::from_str(include_str!("defaults.json")).expect("Built-in design must be valid")
}

pub fn load(settings: &HashMap<String, String>) -> DesignConfig {
    if let Some(raw) = settings.get(SETTING_KEY) {
        if let Ok(config) = serde_json::from_str::<DesignConfig>(raw) {
            if config.validate().is_ok() {
                return config;
            }
        }
    }
    let mut config = defaults();
    if let Some(accent) = settings
        .get("primary_accent_color")
        .filter(|s| valid_color(s))
    {
        config.theme.light.accent = accent.clone();
    }
    config
}

pub fn valid_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn safe_url(value: &str) -> bool {
    (value.starts_with('/') && !value.starts_with("//")
        || value.starts_with("https://")
        || value.starts_with("http://")
        || value.starts_with('#'))
        && !value.chars().any(|c| c.is_control() || c == '\\')
}

pub fn json_for_script<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "null".into())
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

impl DesignConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Desteklenmeyen tasarım sürümü.".into());
        }
        if !["sidebar", "centered"].contains(&self.templates.article_layout.as_str())
            || !["grid", "list"].contains(&self.templates.category_layout.as_str())
            || !["grid", "list"].contains(&self.templates.search_layout.as_str())
            || !["serif", "sans"].contains(&self.templates.reading_font.as_str())
        {
            return Err("Geçersiz sayfa şablonu.".into());
        }
        if !(1000..=1600).contains(&self.theme.width)
            || self.theme.radius > 16
            || !(15..=20).contains(&self.theme.font_size)
        {
            return Err("Genişlik 1000–1600, köşe 0–16, yazı 15–20 aralığında olmalı.".into());
        }
        if !["light", "dark", "system"].contains(&self.theme.default_mode.as_str()) {
            return Err("Geçersiz tema modu.".into());
        }
        for p in [&self.theme.light, &self.theme.dark] {
            if [
                &p.background,
                &p.surface,
                &p.subtle,
                &p.text,
                &p.muted,
                &p.border,
                &p.accent,
            ]
            .iter()
            .any(|c| !valid_color(c))
            {
                return Err("Renkler #RRGGBB biçiminde olmalı.".into());
            }
            if contrast(&p.text, &p.background) < 4.5
                || contrast(&p.text, &p.surface) < 4.5
                || contrast(&p.muted, &p.surface) < 4.5
                || contrast(&p.muted, &p.background) < 4.5
                || contrast(&p.text, &p.subtle) < 4.5
                || contrast(&p.accent, "#ffffff") < 4.5
            {
                return Err("Okunabilirlik için metin, ikincil metin ve buton kontrastı en az 4.5:1 olmalı.".into());
            }
        }
        if self.navigation.len() > 10
            || self
                .navigation
                .iter()
                .any(|n| n.label.trim().is_empty() || n.label.len() > 80 || !safe_url(&n.href))
        {
            return Err("Menüde en fazla 10 geçerli bağlantı olabilir.".into());
        }
        if self.pages.len() > 50 {
            return Err("En fazla 50 sayfa tanımlanabilir.".into());
        }
        let mut slugs = HashSet::new();
        for p in &self.pages {
            if p.slug.is_empty()
                || p.slug.len() > 80
                || !p
                    .slug
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
                || !slugs.insert(&p.slug)
            {
                return Err(
                    "Sayfa adresleri benzersiz olmalı; küçük harf, rakam ve tire kullanın.".into(),
                );
            }
            if p.title.trim().is_empty()
                || p.title.len() > 200
                || p.description.len() > 2000
                || !["magazine", "news"].contains(&p.mode.as_str())
            {
                return Err("Sayfa başlığı veya yayın modu geçersiz.".into());
            }
            if p.sections.is_empty() || p.sections.len() > 24 {
                return Err("Sayfalarda 1–24 bölüm olmalı.".into());
            }
            for s in &p.sections {
                if s.limit == 0
                    || s.limit > 24
                    || s.offset > 1000
                    || !["grid", "list", "feature"].contains(&s.layout.as_str())
                    || !["all", "magazine", "news"].contains(&s.source.as_str())
                    || s.title.len() > 200
                    || s.eyebrow.len() > 100
                    || s.description.len() > 10000
                    || s.category.len() > 100
                    || s.link_label.len() > 100
                    || (!s.link.is_empty() && !safe_url(&s.link))
                {
                    return Err("Bölüm ayarlarını kontrol edin: sınır 1–24, düzen grid/list/feature, kaynak all/magazine/news.".into());
                }
            }
        }
        if !slugs.contains(&"magazine".to_string()) || !slugs.contains(&"news".to_string()) {
            return Err("Dergi ve haber ana sayfaları korunmalı.".into());
        }
        Ok(())
    }
    pub fn page(&self, slug: &str) -> Option<&Page> {
        self.pages.iter().find(|p| p.slug == slug)
    }
    pub fn css(&self) -> String {
        fn palette(p: &Palette) -> String {
            format!("--msi-bg-body:{};--msi-bg-surface:{};--msi-bg-subtle:{};--msi-text-primary:{};--msi-text-muted:{};--msi-border:{};--msi-red:{};", p.background, p.surface, p.subtle, p.text, p.muted, p.border, p.accent)
        }
        format!(":root{{{}--site-width:{}px;--radius:{}px;--base-size:{}px;}}html[data-theme=dark]{{{}color-scheme:dark;}}", palette(&self.theme.light), self.theme.width, self.theme.radius, self.theme.font_size, palette(&self.theme.dark))
    }
}

fn contrast(a: &str, b: &str) -> f64 {
    fn luminance(c: &str) -> f64 {
        [0.2126, 0.7152, 0.0722]
            .iter()
            .enumerate()
            .map(|(i, weight)| {
                let v =
                    u8::from_str_radix(&c[1 + i * 2..3 + i * 2], 16).unwrap_or(0) as f64 / 255.0;
                weight
                    * if v <= 0.04045 {
                        v / 12.92
                    } else {
                        ((v + 0.055) / 1.055).powf(2.4)
                    }
            })
            .sum()
    }
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_accessible() {
        defaults().validate().unwrap();
    }
    #[test]
    fn rejects_css_and_link_injection() {
        let mut config = defaults();
        config.theme.light.accent = "red;}</style><script>".into();
        assert!(config.validate().is_err());
        for value in [
            "javascript:alert(1)",
            "//evil.test",
            "/\\evil.test",
            "/\nevil",
        ] {
            assert!(!safe_url(value));
        }
        assert!(!json_for_script(&"</script>").contains('<'));
    }
    #[test]
    fn rejects_invalid_pages_and_unreadable_colors() {
        let mut c = defaults();
        c.pages.push(c.pages[0].clone());
        assert!(c.validate().is_err());
        let mut c = defaults();
        c.theme.dark.text = c.theme.dark.surface.clone();
        assert!(c.validate().is_err());
        let mut c = defaults();
        c.pages[0].sections[0].limit = 1000;
        assert!(c.validate().is_err());
    }
    #[test]
    fn invalid_saved_config_falls_back() {
        let mut s = HashMap::new();
        s.insert(SETTING_KEY.into(), "{}".into());
        assert_eq!(load(&s).version, 1);
    }
}
