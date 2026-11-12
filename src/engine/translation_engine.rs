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

/// Seed default UI translations into the database if the table is empty.
/// These cover all previously-hardcoded strings found during the audit.
pub fn seed_default_translations(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM translations", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }

    // (key, value_tr, value_en, group)
    let defaults: Vec<(&str, &str, &str, &str)> = vec![
        // Navigation & Shell
        ("nav.home", "Ana Sayfa", "Home", "navigation"),
        ("nav.search", "Arama", "Search", "navigation"),
        ("nav.account", "Hesabım", "My Account", "navigation"),
        ("nav.login", "Giriş yap", "Log in", "navigation"),
        ("nav.register", "Kayıt ol", "Register", "navigation"),
        ("nav.logout", "Çıkış", "Log out", "navigation"),
        ("nav.subscriptions", "Abonelik ve Satış", "Subscriptions & Store", "navigation"),
        ("nav.edergi", "E-Dergi", "E-Magazine", "navigation"),
        ("nav.corporate", "Kurumsal", "Corporate", "navigation"),

        // Search page
        ("search.eyebrow", "MSI ARŞİVİNİ KEŞFEDİN", "EXPLORE THE MSI ARCHIVE", "search"),
        ("search.heading", "Bir konunun peşinden gidin.", "Follow a topic through.", "search"),
        ("search.description", "Haberler, analizler, söyleşiler ve dergi arşivi. Hepsi aynı yerde.", "News, analyses, interviews and magazine archive. All in one place.", "search"),
        ("search.query_label", "Arama kelimesi", "Search term", "search"),
        ("search.placeholder", "Konu, kurum veya teknoloji…", "Topic, institution or technology…", "search"),
        ("search.source_label", "Yayın", "Source", "search"),
        ("search.sort_label", "Sıralama", "Sort", "search"),
        ("search.submit", "Ara →", "Search →", "search"),
        ("search.results_summary", "{count} sonuç · {showing} içerik gösteriliyor", "{count} results · showing {showing} items", "search"),
        ("search.clear_filters", "Filtreleri temizle", "Clear filters", "search"),
        ("search.filter_categories", "Kategoriler", "Categories", "search"),
        ("search.filter_tags", "İlgi alanları", "Interest areas", "search"),
        ("search.filter_issues", "Dergi sayıları", "Magazine issues", "search"),
        ("search.all_categories", "Tüm kategoriler", "All categories", "search"),
        ("search.all_topics", "Tüm konular", "All topics", "search"),
        ("search.all_issues", "Tüm sayılar", "All issues", "search"),
        ("search.no_results_title", "Sonuç bulunamadı.", "No results found.", "search"),
        ("search.no_results_desc", "Farklı bir kelime deneyin veya seçtiğiniz filtreleri temizleyin.", "Try a different keyword or clear your filters.", "search"),
        ("search.sort.latest", "En yeni", "Latest", "search"),
        ("search.sort.popular", "En çok okunan", "Most read", "search"),
        ("search.sort.true_read", "Okuma ilgisine göre", "By reading engagement", "search"),
        ("search.source.all", "Tüm yayınlar", "All sources", "search"),
        ("search.source.dergi", "Dergi içerikleri", "Magazine content", "search"),
        ("search.source.gunluk", "Günlük haberler", "Daily news", "search"),
        ("search.source.online", "Online içerikler", "Online content", "search"),
        ("search.issue_prefix", "Sayı", "Issue", "search"),

        // Category page
        ("category.empty", "Bu kategoride henüz içerik bulunmuyor.", "No content in this category yet.", "category"),
        ("category.explore_all", "Tüm içerikleri keşfedin →", "Explore all content →", "category"),
        ("category.prev_page", "← Önceki", "← Previous", "category"),
        ("category.next_page", "Sonraki →", "Next →", "category"),
        ("category.pagination_label", "Kategori sayfaları", "Category pages", "category"),
        ("category.topic_filter_label", "İlgi alanı", "Interest area", "category"),
        ("category.all_topics", "Tüm konular", "All topics", "category"),
        ("category.filter_button", "Filtrele", "Filter", "category"),
        ("category.reset_filter", "Sıfırla", "Reset", "category"),
        ("category.content_count", "{count} içerik", "{count} items", "category"),
        ("category.articles_suffix", "içerik", "articles", "category"),
        ("category.sub_hub_label", "FUAR, KONFERANS VE ETKİNLİK ÖZEL DOSYALARI", "EXHIBITION & CONFERENCE SPECIAL DOSSIERS", "category"),

        // 404 page
        ("error.404_badge", "SAYFA BULUNAMADI", "PAGE NOT FOUND", "errors"),
        ("error.404_title", "Aradığınız Sayfa Bulunamadı", "Page Not Found", "errors"),
        ("error.404_desc", "İstediğiniz içerik taşınmış olabilir veya adres hatalı girilmiş olabilir.", "The content you requested may have been moved or the address may be incorrect.", "errors"),
        ("error.404_home", "Ana Sayfaya Dön", "Return to Home", "errors"),
        ("error.404_search", "İçerikleri ara", "Search content", "errors"),
        ("error.404_page_title", "404 Sayfa Bulunamadı", "404 Page Not Found", "errors"),

        // Account page
        ("account.eyebrow", "OKUYUCU MERKEZİ", "READER CENTER", "account"),
        ("account.heading_line1", "Okuma yolculuğunuz", "Your reading journey", "account"),
        ("account.heading_line2", "burada devam ediyor.", "continues here.", "account"),
        ("account.description", "Aboneliğinizi, yayınlarınızı ve bildirimlerinizi tek yerden takip edin.", "Manage your subscription, publications and notifications in one place.", "account"),
        ("account.login_cta", "Giriş yap / Hesap oluştur →", "Log in / Create account →", "account"),
        ("account.subscription_link", "Abonelik seçenekleri", "Subscription options", "account"),
        ("account.page_title_guest", "Hesabım", "My Account", "account"),
        ("account.page_title_logged_in", "Okuyucu & Abone Kontrol Merkezi", "Reader & Subscriber Control Center", "account"),

        // Paywall messages
        ("paywall.subscriber_only_en", "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.", "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.", "paywall"),
        ("paywall.subscriber_only_tr", "Abonelere Özel İçerik: Bu analizin tamamı sadece MSI Dergisi abonelerine açıktır. Okumaya devam etmek için abone olabilir veya bu makaleyi tekil olarak ({price} TL) satın alabilirsiniz. (.gov.tr ve .tsk.tr kurumsal e-postalarına ücretsizdir.)", "Abonelere Özel İçerik: Bu analizin tamamı sadece MSI Dergisi abonelerine açıktır. Okumaya devam etmek için abone olabilir veya bu makaleyi tekil olarak ({price} TL) satın alabilirsiniz. (.gov.tr ve .tsk.tr kurumsal e-postalarına ücretsizdir.)", "paywall"),

        // Auth error messages
        ("auth.admin_required", "Bu işlem için yönetici hesabınızla giriş yapın.", "Please log in with your administrator account for this operation.", "auth"),

        // SEO / Meta
        ("seo.default_description", "MSI Dergisi & MSI TDR — Savunma, Havacılık ve Güvenlik Teknolojileri Referans Platformu (ISSN 1306-0309)", "MSI Magazine & MSI TDR — Defense, Aerospace and Security Technologies Reference Platform (ISSN 1306-0309)", "seo"),
        ("seo.search_page_title", "Arama ve keşif", "Search and discovery", "seo"),

        // AI Plugin
        ("plugin.description_human", "Türkiye'nin savunma ve havacılık sanayii dergisi MSI Dergisi'nin güncel haberleri, analizleri ve dergi içerikleri.", "Latest news, analyses and magazine content from MSI Dergisi, Turkey's defense and aerospace industry magazine.", "plugin"),

        // DRM / PDF labels
        ("drm.public_license", "KAMU / ABONE E-DERGI LISANSI", "PUBLIC / SUBSCRIBER E-MAGAZINE LICENSE", "drm"),
        ("drm.dossier_header", "BU SAYIDA YER ALAN DOSYALAR", "DOSSIERS IN THIS ISSUE", "drm"),
        ("drm.page_prefix", "Sayfa", "Page", "drm"),
        ("drm.crypto_signature", "Kriptografik Imza", "Cryptographic Signature", "drm"),
        ("drm.selected_pages", "Secili Sayfalar", "Selected Pages", "drm"),
        ("drm.full_archive", "Tam Sayi Arsiv Paketi", "Full Issue Archive Package", "drm"),
        ("drm.pages_word", "Sayfa", "Pages", "drm"),

        // Ad conflict messages
        ("ads.conflict_warning", "UYARI: '{slot}' slotunda veya [{tags}] hedef etiketlerinde maksimum kampanya kotası ({current}/{limit}) aşıldı{suffix}. Çakışan aktif kampanyalar: {conflicts}. Yine de mevcut kampanyaların yanına eklemek istiyor musunuz?", "WARNING: Maximum campaign quota ({current}/{limit}) exceeded in slot '{slot}' or target tags [{tags}]{suffix}. Conflicting active campaigns: {conflicts}. Do you still want to add alongside existing campaigns?", "ads"),
        ("ads.no_conflict", "Çakışma tespit edilmedi{suffix}. Reklam alanı ve SSB anahtar kelime kotası uygun.", "No conflict detected{suffix}. Ad slot and SSB keyword quota available.", "ads"),
        ("ads.date_range", " [Tarih Aralığı: {start} → {end}, Mod: {mode}]", " [Date Range: {start} → {end}, Mode: {mode}]", "ads"),
        ("ads.fallback_dimensions", "970 × 90 STANDART VARSAYILAN VİTRİN", "970 × 90 STANDARD DEFAULT SHOWCASE", "ads"),
        ("ads.fallback_title_tr", "Hava-Hava, Kara ve Deniz Platformlarında Telifli Havacılık & Savunma Fotoğrafçılığı", "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms", "ads"),
        ("ads.fallback_title_en", "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms", "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms", "ads"),
        ("ads.fallback_desc_tr", "Onur Kurç & Tayfun Yaşar yönetiminde Remove Before Photos yüksek çözünürlüklü telifli savunma görsel arşivi ve MSI Dergisi kurumsal abonelik çözümleri.", "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.", "ads"),
        ("ads.fallback_desc_en", "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.", "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.", "ads"),

        // Subscription tier labels
        ("tier.public_subscriber", "Kamu E-Dergi Abonesi (Ücretsiz)", "Public E-Magazine Subscriber (Free)", "subscriptions"),
        ("tier.academic", "Akademik / Öğrenci Okur", "Academic / Student Reader", "subscriptions"),
        ("tier.standard", "Standart Kayıtlı Okur", "Standard Registered Reader", "subscriptions"),

        // Analytics
        ("analytics.campaign_prefix", "Kampanya #", "Campaign #", "analytics"),
    ];

    for (key, tr, en, group) in &defaults {
        conn.execute(
            "INSERT OR IGNORE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp)
             VALUES (?1, ?2, ?3, '', '', '', '', ?4)",
            rusqlite::params![key, tr, en, group],
        )?;
    }

    Ok(())
}