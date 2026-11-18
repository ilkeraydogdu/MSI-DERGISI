#[allow(unused_imports)]
use super::*;

impl Database {
    // ── Translation Engine DB Methods ──────────────────────────────────

    /// Load all translations from the `translations` table.
    pub fn load_all_translations(&self) -> Vec<crate::engine::translation_engine::TranslationEntry> {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare(
            "SELECT key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp FROM translations ORDER BY key ASC"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map([], |row| {
            Ok(crate::engine::translation_engine::TranslationEntry {
                key: row.get(0)?,
                value_tr: row.get(1)?,
                value_en: row.get(2)?,
                value_ar: row.get(3)?,
                value_de: row.get(4)?,
                value_ru: row.get(5)?,
                value_fr: row.get(6)?,
                group: row.get(7)?,
            })
        }) {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        };
        rows
    }

    /// Upsert a single translation entry.
    pub fn upsert_translation(&self, entry: &crate::engine::translation_engine::TranslationEntry) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.key,
                entry.value_tr,
                entry.value_en,
                entry.value_ar,
                entry.value_de,
                entry.value_ru,
                entry.value_fr,
                entry.group,
            ],
        )?;
        drop(conn);
        self.refresh_l1_cache();
        Ok(())
    }

    /// Delete a translation by key.
    pub fn delete_translation(&self, key: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute("DELETE FROM translations WHERE key = ?1", rusqlite::params![key])?;
        drop(conn);
        self.refresh_l1_cache();
        Ok(())
    }

    // =========================================================================
    // ADVANCED ADMIN: Content Version History

    // =========================================================================

    /// Returns the total number of translation records in the database.
    pub fn count_translations(&self) -> Result<i64, rusqlite::Error> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.query_row("SELECT COUNT(*) FROM translations", [], |r| r.get(0))
    }

    /// Seeds default UI translations into the database if the table is empty.
    /// Centralized replacement for translation_engine.rs raw SQL.
    pub fn seed_default_translations(&self) -> Result<(), rusqlite::Error> {
        let count = self.count_translations()?;
        if count > 0 {
            return Ok(());
        }

        let defaults: Vec<(&str, &str, &str, &str)> = vec![
            ("nav.home", "Ana Sayfa", "Home", "navigation"),
            ("nav.search", "Arama", "Search", "navigation"),
            ("nav.account", "Hesabım", "My Account", "navigation"),
            ("nav.login", "Giriş yap", "Log in", "navigation"),
            ("nav.register", "Kayıt ol", "Register", "navigation"),
            ("nav.logout", "Çıkış", "Log out", "navigation"),
            ("nav.subscriptions", "Abonelik ve Satış", "Subscriptions & Store", "navigation"),
            ("nav.edergi", "E-Dergi", "E-Magazine", "navigation"),
            ("nav.corporate", "Kurumsal", "Corporate", "navigation"),
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
            ("error.404_badge", "SAYFA BULUNAMADI", "PAGE NOT FOUND", "errors"),
            ("error.404_title", "Aradığınız Sayfa Bulunamadı", "Page Not Found", "errors"),
            ("error.404_desc", "İstediğiniz içerik taşınmış olabilir veya adres hatalı girilmiş olabilir.", "The content you requested may have been moved or the address may be incorrect.", "errors"),
            ("error.404_home", "Ana Sayfaya Dön", "Return to Home", "errors"),
            ("error.404_search", "İçerikleri ara", "Search content", "errors"),
            ("error.404_page_title", "404 Sayfa Bulunamadı", "404 Page Not Found", "errors"),
            ("account.eyebrow", "OKUYUCU MERKEZİ", "READER CENTER", "account"),
            ("account.heading_line1", "Okuma yolculuğunuz", "Your reading journey", "account"),
            ("account.heading_line2", "burada devam ediyor.", "continues here.", "account"),
            ("account.description", "Aboneliğinizi, yayınlarınızı ve bildirimlerinizi tek yerden takip edin.", "Manage your subscription, publications and notifications in one place.", "account"),
            ("account.login_cta", "Giriş yap / Hesap oluştur →", "Log in / Create account →", "account"),
            ("account.subscription_link", "Abonelik seçenekleri", "Subscription options", "account"),
            ("account.page_title_guest", "Hesabım", "My Account", "account"),
            ("account.page_title_logged_in", "Okuyucu & Abone Kontrol Merkezi", "Reader & Subscriber Control Center", "account"),
            ("paywall.subscriber_only_en", "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.", "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.", "paywall"),
            ("paywall.subscriber_only_tr", "Abonelere Özel İçerik: Bu analizin tamamı sadece MSI Dergisi abonelerine açıktır. Okumaya devam etmek için abone olabilir veya bu makaleyi tekil olarak ({price} TL) satın alabilirsiniz. (.gov.tr ve .tsk.tr kurumsal e-postalarına ücretsizdir.)", "Abonelere Özel İçerik: Bu analizin tamamı sadece MSI Dergisi abonelerine açıktır. Okumaya devam etmek için abone olabilir veya bu makaleyi tekil olarak ({price} TL) satın alabilirsiniz. (.gov.tr ve .tsk.tr kurumsal e-postalarına ücretsizdir.)", "paywall"),
            ("auth.admin_required", "Bu işlem için yönetici hesabınızla giriş yapın.", "Please log in with your administrator account for this operation.", "auth"),
            ("seo.default_description", "MSI Dergisi & MSI TDR — Savunma, Havacılık ve Güvenlik Teknolojileri Referans Platformu (ISSN 1306-0309)", "MSI Magazine & MSI TDR — Defense, Aerospace and Security Technologies Reference Platform (ISSN 1306-0309)", "seo"),
            ("seo.search_page_title", "Arama ve keşif", "Search and discovery", "seo"),
            ("plugin.description_human", "Türkiye'nin savunma ve havacılık sanayii dergisi MSI Dergisi'nin güncel haberleri, analizleri ve dergi içerikleri.", "Latest news, analyses and magazine content from MSI Dergisi, Turkey's defense and aerospace industry magazine.", "plugin"),
            ("drm.public_license", "KAMU / ABONE E-DERGI LISANSI", "PUBLIC / SUBSCRIBER E-MAGAZINE LICENSE", "drm"),
            ("drm.dossier_header", "BU SAYIDA YER ALAN DOSYALAR", "DOSSIERS IN THIS ISSUE", "drm"),
            ("drm.page_prefix", "Sayfa", "Page", "drm"),
            ("drm.crypto_signature", "Kriptografik Imza", "Cryptographic Signature", "drm"),
            ("drm.selected_pages", "Secili Sayfalar", "Selected Pages", "drm"),
            ("drm.full_archive", "Tam Sayi Arsiv Paketi", "Full Issue Archive Package", "drm"),
            ("drm.pages_word", "Sayfa", "Pages", "drm"),
            ("ads.conflict_warning", "UYARI: '{slot}' slotunda veya [{tags}] hedef etiketlerinde maksimum kampanya kotası ({current}/{limit}) aşıldı{suffix}. Çakışan aktif kampanyalar: {conflicts}. Yine de mevcut kampanyaların yanına eklemek istiyor musunuz?", "WARNING: Maximum campaign quota ({current}/{limit}) exceeded in slot '{slot}' or target tags [{tags}]{suffix}. Conflicting active campaigns: {conflicts}. Do you still want to add alongside existing campaigns?", "ads"),
            ("ads.no_conflict", "Çakışma tespit edilmedi{suffix}. Reklam alanı ve SSB anahtar kelime kotası uygun.", "No conflict detected{suffix}. Ad slot and SSB keyword quota available.", "ads"),
            ("ads.date_range", " [Tarih Aralığı: {start} → {end}, Mod: {mode}]", " [Date Range: {start} → {end}, Mode: {mode}]", "ads"),
            ("ads.fallback_dimensions", "970 × 90 STANDART VARSAYILAN VİTRİN", "970 × 90 STANDARD DEFAULT SHOWCASE", "ads"),
            ("ads.fallback_title_tr", "Hava-Hava, Kara ve Deniz Platformlarında Telifli Havacılık & Savunma Fotoğrafçılığı", "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms", "ads"),
            ("ads.fallback_title_en", "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms", "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms", "ads"),
            ("ads.fallback_desc_tr", "Onur Kurç & Tayfun Yaşar yönetiminde Remove Before Photos yüksek çözünürlüklü telifli savunma görsel arşivi ve MSI Dergisi kurumsal abonelik çözümleri.", "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.", "ads"),
            ("ads.fallback_desc_en", "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.", "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.", "ads"),
            ("tier.public_subscriber", "Kamu E-Dergi Abonesi (Ücretsiz)", "Public E-Magazine Subscriber (Free)", "subscriptions"),
            ("tier.academic", "Akademik / Öğrenci Okur", "Academic / Student Reader", "subscriptions"),
            ("tier.standard", "Standart Kayıtlı Okur", "Standard Registered Reader", "subscriptions"),
            ("analytics.campaign_prefix", "Kampanya #", "Campaign #", "analytics"),
        ];

        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        for (key, tr, en, group) in &defaults {
            conn.execute(
                "INSERT OR IGNORE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp) VALUES (?1, ?2, ?3, '', '', '', '', ?4)",
                rusqlite::params![key, tr, en, group],
            )?;
        }
        Ok(())
    }

    /// Seeds API-specific translation keys into the database.
    /// Centralized replacement for api_response.rs raw SQL.
    pub fn seed_api_translations(&self) -> Result<(), rusqlite::Error> {
        let entries: Vec<(&str, &str, &str)> = vec![
            ("api.error.content_not_found", "İçerik bulunamadı", "Content not found"),
            ("api.error.invalid_email", "Geçerli bir kurumsal e-posta adresi giriniz.", "Please enter a valid corporate email address."),
            ("api.error.email_already_exists", "Bu e-posta ile bir hesap zaten var. Lütfen giriş yapın.", "An account with this email already exists. Please log in."),
            ("api.error.registration_fields_required", "Ad soyad, en az 8 karakterli şifre ve KVKK onayı gerekli.", "Full name, minimum 8-character password and KVKK consent are required."),
            ("api.error.kvkk_consent_required", "Ad soyad, en az 8 karakterli şifre ve KVKK aydınlatma beyanı gerekli.", "Full name, minimum 8-character password and KVKK disclosure statement are required."),
            ("api.error.login_failed", "E-posta veya şifre hatalı.", "Invalid email or password."),
            ("api.error.auth_required_pdf", "E-Dergi PDF indirmek için abone girişi yapınız.", "Subscriber login required to download E-Magazine PDF."),
            ("api.error.subscriber_access_required", "PDF indirme yetkisi aktif MSI Dergisi abonelerine ve onaylı kamu personeline özeldir.", "PDF download access is exclusive to active MSI Magazine subscribers and verified government personnel."),
            ("api.error.issue_not_found", "İstenen dergi sayısı bulunamadı.", "Requested magazine issue not found."),
            ("api.error.checkout_fields_required", "Geçerli ad soyad ve e-posta adresi zorunludur.", "Valid full name and email address are required."),
            ("api.error.distance_sales_consent_required", "6502 sayılı Kanun gereğince Ön Bilgilendirme Formu ve Mesafeli Satış Sözleşmesi onayı zorunludur.", "Preliminary Information Form and Distance Sales Agreement consent required per Law No. 6502."),
            ("api.error.plan_not_found", "Seçilen abonelik planı veya yayın katalogda bulunamadı.", "Selected subscription plan or publication not found in catalog."),
            ("api.error.invalid_file_type", "Yalnızca .webp, .png, .jpg, .svg, .pdf ve .mp4 dosyaları yüklenebilir.", "Only .webp, .png, .jpg, .svg, .pdf and .mp4 files can be uploaded."),
            ("api.error.file_too_large", "Dosya boş veya 25 MB sınırını aşıyor.", "File is empty or exceeds the 25 MB limit."),
            ("api.error.upload_no_field", "Yüklenecek dosya alanı bulunamadı.", "No file field found for upload."),
            ("api.error.server_error_generic", "Bir sunucu hatası oluştu. Lütfen tekrar deneyin.", "A server error occurred. Please try again."),
            ("api.error.server_error_save", "Kaydedilirken bir sunucu hatası oluştu.", "A server error occurred while saving."),
            ("api.error.server_error_upload", "Dosya yükleme sırasında bir sunucu hatası oluştu.", "A server error occurred during file upload."),
            ("api.error.server_error_file_write", "Dosya kaydedilirken bir sunucu hatası oluştu.", "A server error occurred while saving the file."),
            ("api.error.server_error_order", "Sipariş kaydı oluşturulurken bir sunucu hatası oluştu. Lütfen tekrar deneyin.", "A server error occurred while creating the order record. Please try again."),
            ("api.error.server_error_ad_save", "Reklam kaydedilirken bir sunucu hatası oluştu.", "A server error occurred while saving the advertisement."),
            ("api.error.bad_request", "Geçersiz istek.", "Bad request."),
            ("api.error.validation_error", "Doğrulama hatası. Lütfen girdilerinizi kontrol edin.", "Validation error. Please check your input."),
            ("api.error.auth_required", "Kimlik doğrulama gerekli.", "Authentication required."),
            ("api.error.payment_error", "Ödeme işlemi sırasında bir hata oluştu. Lütfen tekrar deneyin.", "Payment processing error. Please try again."),
        ];

        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        for (key, tr, en) in &entries {
            conn.execute(
                "INSERT OR IGNORE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp) VALUES (?1, ?2, ?3, '', '', '', '', 'api')",
                rusqlite::params![key, tr, en],
            )?;
        }
        Ok(())
    }

    // =========================================================================
    // PHASE 4: EDITORIAL APPROVAL WORKFLOW SYSTEM
}
