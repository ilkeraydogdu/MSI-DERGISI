//! Centralized Dynamic Multi-Language Translation Engine
//!
//! Architecture:
//! - All translations stored in `translations` DB table (key + 6 language columns)
//! - Loaded once at startup into `Arc<RwLock<HashMap<String, TranslationEntry>>>` cache
//! - Single entry point: `t(lang, "key")` with fallback chain: target -> TR -> EN -> key itself
//! - Admin API endpoints for CRUD operations on translations
//! - Supports: TR, EN, AR, DE, RU, FR

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Supported language codes
pub const SUPPORTED_LANGS: &[&str] = &["tr", "en", "ar", "de", "ru", "fr"];

/// A single translation entry with all language variants
#[derive(Clone, Debug)]
pub struct TranslationEntry {
    pub key: String,
    pub value_tr: String,
    pub value_en: String,
    pub value_ar: String,
    pub value_de: String,
    pub value_ru: String,
    pub value_fr: String,
    pub group: String,
}

/// Thread-safe in-memory translation cache
#[derive(Clone)]
pub struct TranslationCache {
    inner: Arc<RwLock<HashMap<String, TranslationEntry>>>,
}

impl TranslationCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load all translations from a pre-fetched list into the cache
    pub fn load(&self, entries: Vec<TranslationEntry>) {
        let mut map = HashMap::with_capacity(entries.len());
        for entry in entries {
            map.insert(entry.key.clone(), entry);
        }
        if let Ok(mut guard) = self.inner.write() {
            *guard = map;
        }
    }

    /// Reload the cache with fresh data (called after admin edits)
    pub fn reload(&self, entries: Vec<TranslationEntry>) {
        self.load(entries);
    }

    /// Core translation lookup with fallback chain: target -> TR -> EN -> key
    pub fn t(&self, lang: &str, key: &str) -> String {
        let guard = match self.inner.read() {
            Ok(g) => g,
            Err(_) => return key.to_string(),
        };

        let Some(entry) = guard.get(key) else {
            return key.to_string();
        };

        let val = match lang {
            "tr" => &entry.value_tr,
            "en" => &entry.value_en,
            "ar" => &entry.value_ar,
            "de" => &entry.value_de,
            "ru" => &entry.value_ru,
            "fr" => &entry.value_fr,
            _ => &entry.value_tr,
        };

        // Fallback chain: target lang -> TR -> EN -> key
        if !val.is_empty() {
            return val.clone();
        }
        if !entry.value_tr.is_empty() {
            return entry.value_tr.clone();
        }
        if !entry.value_en.is_empty() {
            return entry.value_en.clone();
        }
        key.to_string()
    }

    /// Get all entries (for admin listing)
    pub fn all_entries(&self) -> Vec<TranslationEntry> {
        match self.inner.read() {
            Ok(guard) => guard.values().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Get a single entry by key
    pub fn get_entry(&self, key: &str) -> Option<TranslationEntry> {
        match self.inner.read() {
            Ok(guard) => guard.get(key).cloned(),
            Err(_) => None,
        }
    }
}

impl Default for TranslationCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function: translate using a cache reference
pub fn t(cache: &TranslationCache, lang: &str, key: &str) -> String {
    cache.t(lang, key)
}

/// Seed default UI translations via centralized repo method.
/// Raw SQL removed — now delegates to Database::seed_default_translations().
pub fn seed_default_translations(db: &crate::db::Database) -> Result<(), String> {
    db.seed_default_translations().map_err(|e| format!("Default translation seed hatası: {}", e))
}