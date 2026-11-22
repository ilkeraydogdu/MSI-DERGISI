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

    pub fn load_all_translations_paginated(
        &self,
        limit: u32,
        offset: u32,
    ) -> (Vec<crate::engine::translation_engine::TranslationEntry>, u64) {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return (vec![], 0),
        };
        let total: u64 = conn
            .query_row("SELECT COUNT(*) FROM translations", [], |r| r.get(0))
            .unwrap_or(0);
        let mut stmt = match conn.prepare(
            "SELECT key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp FROM translations ORDER BY key ASC LIMIT ?1 OFFSET ?2",
        ) {
            Ok(s) => s,
            Err(_) => return (vec![], total),
        };
        let rows = match stmt.query_map(rusqlite::params![limit, offset], |row| {
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
            Ok(r) => r,
            Err(_) => return (vec![], total),
        };
        (rows.filter_map(|r| r.ok()).collect(), total)
    }

    // =========================================================================
    // ADVANCED ADMIN: Content Version History

    // =========================================================================

    /// Returns the total number of translation records in the database.
    pub fn count_translations(&self) -> Result<i64, rusqlite::Error> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.query_row("SELECT COUNT(*) FROM translations", [], |r| r.get(0))
    }

    /// Seeds default UI translations idempotently into the database via `INSERT OR IGNORE`.
    /// Never skips when existing rows exist so newly introduced keys are always populated.
    pub fn seed_default_translations(&self) -> Result<(), rusqlite::Error> {
        let defaults: Vec<(&str, &str, &str, &str)> = vec![
            // ── Navigation ──
            ("nav.home", "Ana Sayfa", "Home", "navigation"),
            ("nav.search", "Arama", "Search", "navigation"),
            ("nav.account", "Hesabım", "My Account", "navigation"),
            ("nav.login", "Giriş yap", "Log in", "navigation"),
            ("nav.register", "Kayıt ol", "Register", "navigation"),
            ("nav.logout", "Çıkış", "Log out", "navigation"),
            ("nav.subscriptions", "Abonelik ve Satış", "Subscriptions & Store", "navigation"),
            ("nav.edergi", "E-Dergi", "E-Magazine", "navigation"),
            ("nav.corporate", "Kurumsal", "Corporate", "navigation"),
            ("nav.news", "Haberler", "Defence News", "navigation"),
            ("nav.magazine", "Dergi", "Magazine", "navigation"),
            ("nav.events", "Etkinlikler", "Defence Events", "navigation"),
            ("nav.special_dossiers", "Özel Dosyalar", "Special Dossiers", "navigation"),
            ("nav.interviews", "Söyleşiler", "Interviews", "navigation"),
            ("nav.editorials", "Başyazılar", "Editorials", "navigation"),
            ("nav.archive", "Arşiv", "Archive", "navigation"),
            ("nav.admin_panel", "Yönetim Masası", "Command Suite", "navigation"),
            ("nav.menu", "Menü", "Menu", "navigation"),
            ("nav.theme_toggle", "Görünüm", "Theme", "navigation"),
            ("nav.lang_tr", "Türkçe", "Turkish", "navigation"),
            ("nav.lang_en", "English", "English", "navigation"),
            // ── Home ──
            ("home.latest_news", "Son Savunma Haberleri", "Latest Defence News", "home"),
            ("home.cover_stories", "Kapak Dosyaları", "Cover Stories", "home"),
            ("home.special_dossiers", "Özel Dosyalar", "Special Dossiers", "home"),
            ("home.interviews", "Söyleşiler", "Executive Interviews", "home"),
            ("home.editorial_board", "Başyazı", "Editorial", "home"),
            ("home.read_more", "Devamını oku →", "Read more →", "home"),
            ("home.view_all", "Tümünü gör →", "View all →", "home"),
            ("home.featured_issue", "Güncel Sayı", "Current Issue", "home"),
            ("home.live_ticker", "SAVUNMA GÜNDEMİ", "DEFENCE BRIEF", "home"),
            ("home.defence_intelligence", "Stratejik Savunma & Havacılık İstihbaratı", "Strategic Defence & Aerospace Intelligence", "home"),
            ("home.newsletter_cta", "Haftalık Savunma Bültenine Katılın", "Subscribe to Weekly Defence Brief", "home"),
            ("home.subscribe_now", "Abone Ol", "Subscribe Now", "home"),
            // ── Article ──
            ("article.read_time", "{min} dk okuma", "{min} min read", "article"),
            ("article.published_at", "Yayımlandı", "Published", "article"),
            ("article.author", "Yazar", "Author", "article"),
            ("article.issue_ref", "Sayı {issue}", "Issue {issue}", "article"),
            ("article.related", "İlginizi Çekebilir", "Related Intelligence", "article"),
            ("article.share", "Paylaş", "Share", "article"),
            ("article.sponsored_badge", "SPONSORLU İÇERİK", "SPONSORED CONTENT", "article"),
            ("article.subscriber_badge", "ABONE ÖZEL", "SUBSCRIBER ONLY", "article"),
            ("article.unlock_single", "Tekil Makale Satın Al", "Unlock Single Article", "article"),
            ("article.unlock_subscribe", "E-Dergi Abonesi Ol", "Become an E-Magazine Subscriber", "article"),
            ("article.watch_video", "Videoyu İzle", "Watch Video", "article"),
            ("article.back_to_category", "Kategoriye Dön", "Back to Category", "article"),
            ("article.en_notice_pending", "Bu içeriğin onaylı İngilizce çevirisi hazırlanmaktadır.", "An approved English translation for this article is currently being prepared.", "article"),
            // ── E-Dergi ──
            ("edergi.title", "MSI E-Dergi Kütüphanesi", "MSI E-Magazine Library", "edergi"),
            ("edergi.subtitle", "Filigran korumalı tam sayı dijital baskı ve sayfa aralığı indirme merkezi.", "Dynamically watermarked digital magazine editions and selective page downloads.", "edergi"),
            ("edergi.read_issue", "Sayıyı İncele", "Inspect Issue", "edergi"),
            ("edergi.download_pdf", "Filigranlı PDF İndir", "Download Watermarked PDF", "edergi"),
            ("edergi.download_range", "Sayfa Aralığı İndir", "Download Page Range", "edergi"),
            ("edergi.page_range_label", "Sayfa Aralığı (örn. 1-16)", "Page Range (e.g. 1-16)", "edergi"),
            ("edergi.pages_unit", "sayfa", "pages", "edergi"),
            ("edergi.dossiers_unit", "dosya", "dossiers", "edergi"),
            ("edergi.special_edition", "Özel Sayı", "Special Edition", "edergi"),
            ("edergi.msi_edition", "MSI Dergisi (TR)", "MSI Magazine (TR)", "edergi"),
            ("edergi.tdr_edition", "MSI TDR (EN)", "MSI TDR (EN)", "edergi"),
            ("edergi.watermark_notice", "Tüm PDF indirmeleri kullanıcı adı, kurum ve tarih bilgisiyle kişiselleştirilmiş filigran taşır.", "All PDF downloads carry a personalised watermark with subscriber name, organisation and UTC timestamp.", "edergi"),
            ("edergi.subscriber_only", "Aktif abonelik veya doğrulanmış kamu e-postası gerektirir.", "Requires an active subscription or verified public-sector email.", "edergi"),
            ("edergi.pdf_preview_tab", "PDF Okuyucu", "PDF Reader", "edergi"),
            ("edergi.toc_tab", "İçindekiler", "Table of Contents", "edergi"),
            // ── Dialogs ──
            ("dialogs.close", "Kapat", "Close", "dialogs"),
            ("dialogs.cancel", "İptal", "Cancel", "dialogs"),
            ("dialogs.confirm", "Onayla", "Confirm", "dialogs"),
            ("dialogs.save", "Kaydet", "Save", "dialogs"),
            ("dialogs.loading", "Yükleniyor…", "Loading…", "dialogs"),
            ("dialogs.issue_title", "Dergi Sayısı İnceleme", "Magazine Issue Preview", "dialogs"),
            ("dialogs.auth_title", "Okuyucu & Abone Oturumu", "Reader & Subscriber Sign-In", "dialogs"),
            ("dialogs.offer_title", "Abonelik & Satın Alma", "Subscription & Checkout", "dialogs"),
            ("dialogs.cookie_title", "Çerez ve KVKK Tercihleri", "Cookie & Privacy Preferences", "dialogs"),
            ("dialogs.cookie_accept_all", "Tümünü Kabul Et", "Accept All", "dialogs"),
            ("dialogs.cookie_necessary_only", "Yalnızca Zorunlu", "Essential Only", "dialogs"),
            ("dialogs.cookie_save_prefs", "Tercihleri Kaydet", "Save Preferences", "dialogs"),
            ("dialogs.cookie_settings", "Çerez Ayarları", "Cookie Settings", "dialogs"),
            ("dialogs.cookie_analytics", "Analitik Çerezler", "Analytics Cookies", "dialogs"),
            ("dialogs.cookie_marketing", "Pazarlama ve Sponsorluk", "Marketing & Sponsorship", "dialogs"),
            ("dialogs.lightbox_prev", "Önceki Görsel", "Previous Image", "dialogs"),
            ("dialogs.lightbox_next", "Sonraki Görsel", "Next Image", "dialogs"),
            // ── Auth ──
            ("auth.admin_required", "Bu işlem için yönetici hesabınızla giriş yapın.", "Please log in with your administrator account for this operation.", "auth"),
            ("auth.login_tab", "Giriş Yap", "Log In", "auth"),
            ("auth.register_tab", "Kayıt Ol", "Register", "auth"),
            ("auth.email", "Kurumsal E-Posta", "Corporate Email", "auth"),
            ("auth.password", "Parola", "Password", "auth"),
            ("auth.full_name", "Ad Soyad", "Full Name", "auth"),
            ("auth.organization_category", "Kurum Sınıfı", "Organisation Category", "auth"),
            ("auth.organization_name", "Kurum / Firma Adı", "Organisation / Company Name", "auth"),
            ("auth.job_title", "Görev / Unvan", "Role / Job Title", "auth"),
            ("auth.kvkk_consent", "KVKK Aydınlatma Metni ve Üyelik Sözleşmesini okudum, onaylıyorum.", "I have read and accept the Privacy Notice and Membership Terms.", "auth"),
            ("auth.email_consent", "Savunma bültenleri ve yeni sayı duyurularını e-posta ile almak istiyorum.", "I agree to receive defence intelligence newsletters and new issue alerts.", "auth"),
            ("auth.ssb_interests", "Teknoloji İlgi Alanları (SSB Taksonomisi)", "Technology Interests (SSB Taxonomy)", "auth"),
            ("auth.forgot_password", "Parolamı Unuttum", "Forgot Password", "auth"),
            ("auth.reset_password", "Parolayı Sıfırla", "Reset Password", "auth"),
            ("auth.login_submit", "Giriş Yap", "Sign In", "auth"),
            ("auth.register_submit", "Hesap Oluştur", "Create Account", "auth"),
            ("auth.gov_free_hint", ".gov.tr / .tsk.tr kamu personeli e-posta doğrulaması sonrası ücretsiz dijital aboneliğe hak kazanır.", "Public-sector (.gov.tr / .tsk.tr) personnel receive complimentary digital subscription upon email verification.", "auth"),
            ("auth.totp_code", "2FA Doğrulama Kodu", "2FA Verification Code", "auth"),
            ("auth.verify_email_notice", "Lütfen gelen kutunuzdaki doğrulama bağlantısını onaylayın.", "Please confirm the verification link sent to your inbox.", "auth"),
            // ── Search ──
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
            // ── Category ──
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
            // ── Errors ──
            ("error.404_badge", "SAYFA BULUNAMADI", "PAGE NOT FOUND", "errors"),
            ("error.404_title", "Aradığınız Sayfa Bulunamadı", "Page Not Found", "errors"),
            ("error.404_desc", "İstediğiniz içerik taşınmış olabilir veya adres hatalı girilmiş olabilir.", "The content you requested may have been moved or the address may be incorrect.", "errors"),
            ("error.404_home", "Ana Sayfaya Dön", "Return to Home", "errors"),
            ("error.404_search", "İçerikleri ara", "Search content", "errors"),
            ("error.404_page_title", "404 Sayfa Bulunamadı", "404 Page Not Found", "errors"),
            // ── Account ──
            ("account.eyebrow", "OKUYUCU MERKEZİ", "READER CENTER", "account"),
            ("account.heading_line1", "Okuma yolculuğunuz", "Your reading journey", "account"),
            ("account.heading_line2", "burada devam ediyor.", "continues here.", "account"),
            ("account.description", "Aboneliğinizi, yayınlarınızı ve bildirimlerinizi tek yerden takip edin.", "Manage your subscription, publications and notifications in one place.", "account"),
            ("account.login_cta", "Giriş yap / Hesap oluştur →", "Log in / Create account →", "account"),
            ("account.subscription_link", "Abonelik seçenekleri", "Subscription options", "account"),
            ("account.page_title_guest", "Hesabım", "My Account", "account"),
            ("account.page_title_logged_in", "Okuyucu & Abone Kontrol Merkezi", "Reader & Subscriber Control Centre", "account"),
            ("account.profile_tab", "Profil & Tercihler", "Profile & Preferences", "account"),
            ("account.subscriptions_tab", "Abonelik Durumu", "Subscription Status", "account"),
            ("account.purchases_tab", "Satın Alınan Yayınlar", "Purchased Publications", "account"),
            ("account.invoices_tab", "Faturalar", "Invoices", "account"),
            ("account.security_tab", "Güvenlik & 2FA", "Security & 2FA", "account"),
            ("account.export_data", "Verilerimi İndir (KVKK JSON)", "Export My Data (GDPR JSON)", "account"),
            ("account.delete_account", "Hesabımı Sil", "Delete Account", "account"),
            ("account.resend_verification", "Doğrulama E-Postasını Tekrar Gönder", "Resend Verification Email", "account"),
            ("account.status_verified", "Doğrulanmış", "Verified", "account"),
            ("account.status_unverified", "Doğrulama Bekliyor", "Pending Verification", "account"),
            // ── Store & Commerce ──
            ("store.title", "Abonelik Paketleri & Yayın Mağazası", "Subscriptions & Publication Store", "store"),
            ("store.plans_heading", "Kurumsal ve Bireysel Abonelikler", "Corporate & Individual Subscriptions", "store"),
            ("store.products_heading", "Prestij Kitapları, Raporlar ve Telifli Fotoğraf Lisansları", "Prestige Books, Research Reports & Licensed Photography", "store"),
            ("store.buy_now", "Satın Al", "Buy Now", "store"),
            ("store.subscribe", "Abone Ol", "Subscribe", "store"),
            ("store.price_try", "{price} TL", "{price} TRY", "store"),
            ("store.checkout_title", "Güvenli Ödeme ve Fatura Bilgileri", "Secure Checkout & Billing Details", "store"),
            ("store.distance_sales_agree", "Ön Bilgilendirme Formu ve Mesafeli Satış Sözleşmesini onaylıyorum.", "I agree to the Preliminary Information Form and Distance Sales Agreement.", "store"),
            ("store.preliminary_info", "Ön Bilgilendirme", "Preliminary Information", "store"),
            ("store.pay_with_paytr", "PayTR ile Güvenli Öde", "Pay Securely with PayTR", "store"),
            ("store.download_asset", "Dijital Yayını İndir", "Download Digital Asset", "store"),
            ("store.in_stock", "Anında Teslim", "Instant Delivery", "store"),
            ("store.corporate_seats", "10 Kurumsal Kullanıcı + 5 Basılı Sayı", "10 Corporate Seats + 5 Print Issues", "store"),
            // ── Footer ──
            ("footer.copyright", "© UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. Tüm hakları saklıdır.", "© UMSA Production Printing and Publishing Ltd. Co. All rights reserved.", "footer"),
            ("footer.issn", "ISSN 1306-0309", "ISSN 1306-0309", "footer"),
            ("footer.address_label", "Adres", "Address", "footer"),
            ("footer.phone_label", "Telefon", "Telephone", "footer"),
            ("footer.email_label", "E-Posta", "Email", "footer"),
            ("footer.kvkk", "KVKK Aydınlatma", "Privacy Notice", "footer"),
            ("footer.privacy", "Gizlilik Politikası", "Privacy Policy", "footer"),
            ("footer.cookies", "Çerez Politikası", "Cookie Policy", "footer"),
            ("footer.terms", "Kullanım Koşulları", "Terms of Use", "footer"),
            ("footer.about", "Hakkımızda", "About Us", "footer"),
            ("footer.contact", "İletişim", "Contact", "footer"),
            ("footer.masthead", "Künye", "Masthead", "footer"),
            ("footer.follow_us", "Bizi Takip Edin", "Follow Us", "footer"),
            // ── Paywall / SEO / Plugin / DRM / Ads / Subscriptions / Analytics ──
            ("paywall.subscriber_only_en", "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.", "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.", "paywall"),
            ("paywall.subscriber_only_tr", "Abonelere Özel İçerik: Bu analizin tamamı sadece MSI Dergisi abonelerine açıktır. Okumaya devam etmek için abone olabilir veya bu makaleyi tekil olarak ({price} TL) satın alabilirsiniz. (.gov.tr ve .tsk.tr kurumsal e-postalarına ücretsizdir.)", "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.", "paywall"),
            ("seo.default_description", "MSI Dergisi & MSI TDR — Savunma, Havacılık ve Güvenlik Teknolojileri Referans Platformu (ISSN 1306-0309)", "MSI Magazine & MSI TDR — Defence, Aerospace and Security Technologies Reference Platform (ISSN 1306-0309)", "seo"),
            ("seo.search_page_title", "Arama ve keşif", "Search and discovery", "seo"),
            ("plugin.description_human", "Türkiye'nin savunma ve havacılık sanayii dergisi MSI Dergisi'nin güncel haberleri, analizleri ve dergi içerikleri.", "Latest news, analyses and magazine content from MSI Dergisi, Türkiye's defence and aerospace industry magazine.", "plugin"),
            ("drm.public_license", "KAMU / ABONE E-DERGI LISANSI", "PUBLIC / SUBSCRIBER E-MAGAZINE LICENCE", "drm"),
            ("drm.dossier_header", "BU SAYIDA YER ALAN DOSYALAR", "DOSSIERS IN THIS ISSUE", "drm"),
            ("drm.page_prefix", "Sayfa", "Page", "drm"),
            ("drm.crypto_signature", "Kriptografik İmza", "Cryptographic Signature", "drm"),
            ("drm.selected_pages", "Seçili Sayfalar", "Selected Pages", "drm"),
            ("drm.full_archive", "Tam Sayı Arşiv Paketi", "Full Issue Archive Package", "drm"),
            ("drm.pages_word", "Sayfa", "Pages", "drm"),
            ("ads.conflict_warning", "UYARI: '{slot}' slotunda veya [{tags}] hedef etiketlerinde maksimum kampanya kotası ({current}/{limit}) aşıldı{suffix}. Çakışan aktif kampanyalar: {conflicts}. Yine de mevcut kampanyaların yanına eklemek istiyor musunuz?", "WARNING: Maximum campaign quota ({current}/{limit}) exceeded in slot '{slot}' or target tags [{tags}]{suffix}. Conflicting active campaigns: {conflicts}. Do you still want to add alongside existing campaigns?", "ads"),
            ("ads.no_conflict", "Çakışma tespit edilmedi{suffix}. Reklam alanı ve SSB anahtar kelime kotası uygun.", "No conflict detected{suffix}. Ad slot and SSB keyword quota available.", "ads"),
            ("ads.date_range", " [Tarih Aralığı: {start} → {end}, Mod: {mode}]", " [Date Range: {start} → {end}, Mode: {mode}]", "ads"),
            ("ads.fallback_dimensions", "970 × 90 STANDART VARSAYILAN VİTRİN", "970 × 90 STANDARD DEFAULT SHOWCASE", "ads"),
            ("ads.fallback_title_tr", "Hava-Hava, Kara ve Deniz Platformlarında Telifli Havacılık & Savunma Fotoğrafçılığı", "Copyrighted Aviation & Defence Photography Across Air-to-Air, Land and Naval Platforms", "ads"),
            ("ads.fallback_title_en", "Reklam Alanı Varsayılan Başlığı (EN)", "Copyrighted Aviation & Defence Photography Across Air-to-Air, Land and Naval Platforms", "ads"),
            ("ads.fallback_desc_tr", "Onur Kurç & Tayfun Yaşar yönetiminde Remove Before Photos yüksek çözünürlüklü telifli savunma görsel arşivi ve MSI Dergisi kurumsal abonelik çözümleri.", "Remove Before Photos high-resolution defence visual archive and MSI Dergisi corporate subscription solutions.", "ads"),
            ("ads.fallback_desc_en", "Reklam Alanı Varsayılan Açıklaması (EN)", "Remove Before Photos high-resolution defence visual archive and MSI Dergisi corporate subscription solutions.", "ads"),
            ("tier.public_subscriber", "Kamu E-Dergi Abonesi (Ücretsiz)", "Public E-Magazine Subscriber (Free)", "subscriptions"),
            ("tier.academic", "Akademik / Öğrenci Okur", "Academic / Student Reader", "subscriptions"),
            ("tier.standard", "Standart Kayıtlı Okur", "Standard Registered Reader", "subscriptions"),
            ("analytics.campaign_prefix", "Kampanya #", "Campaign #", "analytics"),
            // ── Admin Suite ──
            ("admin.workspace_publish", "Yayın Masası", "Publishing Desk", "admin"),
            ("admin.workspace_magazine", "Dergi & Medya", "Magazine & Media", "admin"),
            ("admin.workspace_readers", "Okuyucu & Kurum", "Readers & Organisations", "admin"),
            ("admin.workspace_commercial", "Ticari", "Commercial", "admin"),
            ("admin.workspace_analytics", "Analitik", "Analytics", "admin"),
            ("admin.workspace_system", "Sistem & Tasarım", "System & Design", "admin"),
            ("admin.status_draft", "Taslak", "Draft", "admin"),
            ("admin.status_pending", "Onay Bekliyor", "Pending Review", "admin"),
            ("admin.status_published", "Yayında", "Published", "admin"),
            ("admin.status_rejected", "Reddedildi", "Rejected", "admin"),
            ("admin.status_archived", "Arşivde", "Archived", "admin"),
            ("admin.en_status_none", "EN Yok", "No EN", "admin"),
            ("admin.en_status_auto_draft", "Otomatik Taslak", "Auto Draft", "admin"),
            ("admin.en_status_approved", "EN Onaylı", "EN Approved", "admin"),
            ("admin.en_status_stale", "Güncel Değil", "Out of Sync", "admin"),
            ("admin.auto_translate", "Otonom Çevir", "Auto-Translate", "admin"),
            ("admin.approve_en", "EN Onayla", "Approve EN", "admin"),
            ("admin.save_draft", "Taslak Kaydet", "Save Draft", "admin"),
            ("admin.submit_review", "Onaya Gönder", "Submit for Review", "admin"),
            ("admin.publish", "Yayınla", "Publish", "admin"),
            ("admin.search_placeholder", "Başlık, slug veya yazar ara…", "Search title, slug or author…", "admin"),
            ("admin.new_item", "+ Yeni", "+ New", "admin"),
            ("admin.delete_confirm", "Bu kaydı kalıcı olarak silmek istediğinize emin misiniz?", "Are you sure you want to permanently delete this record?", "admin"),
            ("admin.glossary_title", "Savunma Terminoloji Sözlüğü (Glossary)", "Defence Terminology Glossary", "admin"),
            ("admin.locales_title", "Dil ve Yerelleştirme Yönetimi", "Locales & Language Management", "admin"),
            ("admin.email_groups_title", "Bülten E-Posta Grupları", "Newsletter Email Groups", "admin"),
        ];

        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR IGNORE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp)
                 VALUES (?1, ?2, ?3, '', '', '', '', ?4)",
            )?;
            for (key, tr, en, group) in &defaults {
                stmt.execute(rusqlite::params![key, tr, en, group])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Seeds ALL 26 API-specific translation keys into the database idempotently.
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
            ("api.error.too_many_requests", "Çok fazla istek gönderildi. Lütfen kısa bir süre sonra tekrar deneyin.", "Too many requests. Please try again shortly."),
        ];

        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR IGNORE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp)
                 VALUES (?1, ?2, ?3, '', '', '', '', 'api')",
            )?;
            for (key, tr, en) in &entries {
                stmt.execute(rusqlite::params![key, tr, en])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    // =========================================================================
    // TRANSLATION APPROVAL WORKFLOW (Migration 014)
    // =========================================================================

    /// Upsert a translation entry with full approval-workflow metadata.
    pub fn upsert_translation_with_status(
        &self,
        entry: &crate::engine::translation_engine::TranslationEntry,
        status: &str,
        approved_by: Option<i64>,
        auto_translated_at: Option<&str>,
        source_lang: Option<&str>,
    ) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now_str = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let approved_at_val = if status == "approved" && approved_by.is_some() {
            Some(now_str.clone())
        } else {
            None
        };
        conn.execute(
            "INSERT OR REPLACE INTO translations (key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp, status, approved_by, approved_at, auto_translated_at, source_lang)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                entry.key,
                entry.value_tr,
                entry.value_en,
                entry.value_ar,
                entry.value_de,
                entry.value_ru,
                entry.value_fr,
                entry.group,
                status,
                approved_by,
                approved_at_val,
                auto_translated_at,
                source_lang.unwrap_or("tr"),
            ],
        )?;
        drop(conn);
        self.refresh_l1_cache();
        Ok(())
    }

    /// Approve a translation: sets status='approved', records who approved and when.
    /// Returns `Ok(false)` when no row matches `key`.
    pub fn approve_translation(&self, key: &str, user_id: i64) -> rusqlite::Result<bool> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now_str = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let affected = conn.execute(
            "UPDATE translations SET status = 'approved', approved_by = ?1, approved_at = ?2 WHERE key = ?3",
            rusqlite::params![user_id, now_str, key],
        )?;
        drop(conn);
        if affected > 0 {
            self.refresh_l1_cache();
        }
        Ok(affected > 0)
    }

    /// Reject a translation back to draft status.
    /// Returns `Ok(false)` when no row matches `key`.
    pub fn reject_translation(&self, key: &str) -> rusqlite::Result<bool> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let affected = conn.execute(
            "UPDATE translations SET status = 'draft', approved_by = NULL, approved_at = NULL WHERE key = ?1",
            rusqlite::params![key],
        )?;
        drop(conn);
        if affected > 0 {
            self.refresh_l1_cache();
        }
        Ok(affected > 0)
    }

    /// List translations filtered by status with pagination.
    /// Supports `""`/`"all"` (all rows), `"pending"` (`draft` + `auto_translated`), or exact status.
    /// Returns (entries, total_count).
    pub fn list_translations_by_status(
        &self,
        status_filter: &str,
        page: i64,
        per_page: i64,
    ) -> (Vec<serde_json::Value>, u64) {
        let conn = match self.open_conn() {
            Ok(c) => c,
            Err(_) => return (vec![], 0),
        };
        let safe_page = page.max(1);
        let safe_per_page = per_page.clamp(1, 200);
        let offset = (safe_page - 1) * safe_per_page;
        let norm_status = status_filter.trim();

        let (count_sql, list_sql, bind_status): (&str, &str, Option<&str>) =
            if norm_status.is_empty() || norm_status == "all" {
                (
                    "SELECT COUNT(*) FROM translations",
                    "SELECT key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp,
                            COALESCE(status,'approved'), COALESCE(approved_by,0), COALESCE(approved_at,''),
                            COALESCE(auto_translated_at,''), COALESCE(source_lang,'tr')
                     FROM translations ORDER BY key ASC LIMIT ?1 OFFSET ?2",
                    None,
                )
            } else if norm_status == "pending" {
                (
                    "SELECT COUNT(*) FROM translations WHERE status IN ('draft', 'auto_translated')",
                    "SELECT key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp,
                            COALESCE(status,'approved'), COALESCE(approved_by,0), COALESCE(approved_at,''),
                            COALESCE(auto_translated_at,''), COALESCE(source_lang,'tr')
                     FROM translations WHERE status IN ('draft', 'auto_translated')
                     ORDER BY key ASC LIMIT ?1 OFFSET ?2",
                    None,
                )
            } else {
                (
                    "SELECT COUNT(*) FROM translations WHERE status = ?1",
                    "SELECT key, value_tr, value_en, value_ar, value_de, value_ru, value_fr, grp,
                            COALESCE(status,'approved'), COALESCE(approved_by,0), COALESCE(approved_at,''),
                            COALESCE(auto_translated_at,''), COALESCE(source_lang,'tr')
                     FROM translations WHERE status = ?3 ORDER BY key ASC LIMIT ?1 OFFSET ?2",
                    Some(norm_status),
                )
            };

        let total: u64 = match bind_status {
            Some(st) => conn.query_row(count_sql, rusqlite::params![st], |r| r.get(0)),
            None => conn.query_row(count_sql, [], |r| r.get(0)),
        }
        .unwrap_or(0);

        fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<serde_json::Value> {
            Ok(serde_json::json!({
                "key": row.get::<_, String>(0)?,
                "value_tr": row.get::<_, String>(1)?,
                "value_en": row.get::<_, String>(2)?,
                "value_ar": row.get::<_, String>(3)?,
                "value_de": row.get::<_, String>(4)?,
                "value_ru": row.get::<_, String>(5)?,
                "value_fr": row.get::<_, String>(6)?,
                "group": row.get::<_, String>(7)?,
                "status": row.get::<_, String>(8)?,
                "approved_by": row.get::<_, i64>(9)?,
                "approved_at": row.get::<_, String>(10)?,
                "auto_translated_at": row.get::<_, String>(11)?,
                "source_lang": row.get::<_, String>(12)?,
            }))
        }

        let mut stmt = match conn.prepare(list_sql) {
            Ok(s) => s,
            Err(_) => return (vec![], total),
        };
        let items: Vec<serde_json::Value> = match bind_status {
            Some(st) => stmt
                .query_map(rusqlite::params![safe_per_page, offset, st], map_row)
                .map(|iter| iter.filter_map(Result::ok).collect())
                .unwrap_or_default(),
            None => stmt
                .query_map(rusqlite::params![safe_per_page, offset], map_row)
                .map(|iter| iter.filter_map(Result::ok).collect())
                .unwrap_or_default(),
        };

        (items, total)
    }

    // =========================================================================
    // PHASE 4: EDITORIAL APPROVAL WORKFLOW SYSTEM
}
