-- Migration 013: Admin Panel i18n Translation Keys
-- Seeds ~100 translation keys for the admin command center UI.
-- All keys use ON CONFLICT (key) DO NOTHING to never overwrite admin edits.

-- ═══════════════════════════════════════════════════════════════════
-- Navigation & Sidebar
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.nav.brand', 'UMSA KOMUTA', 'UMSA COMMAND', 'admin.nav'),
('admin.nav.brand_sub', 'Yayın & Operasyon', 'Publishing & Operations', 'admin.nav'),
('admin.nav.group_editorial', 'Yayın Yönetimi', 'Editorial Management', 'admin.nav'),
('admin.nav.group_commercial', 'Ticari & Kurumsal', 'Commercial & Corporate', 'admin.nav'),
('admin.nav.group_analytics', 'Analitik & Rapor', 'Analytics & Reports', 'admin.nav'),
('admin.nav.group_workflow', 'İş Akışı', 'Workflow', 'admin.nav'),
('admin.nav.group_system', 'Sistem & Güvenlik', 'System & Security', 'admin.nav'),
('admin.nav.contents', 'İçerik Yönetimi', 'Content Management', 'admin.nav'),
('admin.nav.issues', 'E-Dergi Sayıları', 'E-Magazine Issues', 'admin.nav'),
('admin.nav.media', 'Medya Kütüphanesi', 'Media Library', 'admin.nav'),
('admin.nav.taxonomy', 'Kategoriler & Etiketler', 'Categories & Tags', 'admin.nav'),
('admin.nav.ads', 'Reklam Kampanyaları', 'Ad Campaigns', 'admin.nav'),
('admin.nav.commerce', 'Gelir & Abonelik', 'Revenue & Subscriptions', 'admin.nav'),
('admin.nav.corporate', 'Kurumsal Başvurular', 'Corporate Requests', 'admin.nav'),
('admin.nav.analytics', 'Genel Analitik', 'General Analytics', 'admin.nav'),
('admin.nav.monitoring', 'Sistem İzleme', 'System Monitoring', 'admin.nav'),
('admin.nav.approvals', 'Onay Bekleyenler', 'Pending Approvals', 'admin.nav'),
('admin.nav.translations', 'Çeviri Yönetimi', 'Translation Management', 'admin.nav'),
('admin.nav.users', 'Kullanıcılar & Roller', 'Users & Roles', 'admin.nav'),
('admin.nav.settings', 'Site Ayarları', 'Site Settings', 'admin.nav'),
('admin.nav.audit', 'Denetim Kayıtları', 'Audit Logs', 'admin.nav'),
('admin.nav.studio', 'Tasarım Stüdyosu', 'Design Studio', 'admin.nav')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- Login Gateway
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.login.title', 'MSI Komuta Geçidi', 'MSI Command Gateway', 'admin.login'),
('admin.login.brand', 'Komuta Geçidi', 'Command Gateway', 'admin.login'),
('admin.login.subtitle', 'UMSA Prodüksiyon · MSI Dergisi & MSI TDR', 'UMSA Production · MSI Magazine & MSI TDR', 'admin.login'),
('admin.login.email_placeholder', 'operator@msidergisi.com', 'operator@msidergisi.com', 'admin.login'),
('admin.login.password_placeholder', 'Parolanızı girin', 'Enter your password', 'admin.login'),
('admin.login.submit', 'Giriş Yap', 'Sign In', 'admin.login'),
('admin.login.forgot_password', 'Parolamı unuttum', 'Forgot password', 'admin.login'),
('admin.login.totp_title', 'İki Faktörlü Doğrulama (2FA)', 'Two-Factor Authentication (2FA)', 'admin.login'),
('admin.login.totp_hint', 'Google Authenticator, Microsoft Authenticator veya 1Password ile eşleştirin.', 'Pair with Google Authenticator, Microsoft Authenticator or 1Password.', 'admin.login'),
('admin.login.totp_code_label', 'Doğrulama Kodu', 'Verification Code', 'admin.login'),
('admin.login.totp_code_placeholder', '6 haneli doğrulama kodu', '6-digit verification code', 'admin.login'),
('admin.login.totp_verify', 'Doğrula ve 2FA''yı Etkinleştir', 'Verify & Enable 2FA', 'admin.login'),
('admin.login.totp_generate', 'QR Kod Üret ve 2FA Kurulumunu Başlat', 'Generate QR Code & Start 2FA Setup', 'admin.login'),
('admin.login.totp_secret_label', 'Manuel Kurulum Anahtarı', 'Manual Setup Key', 'admin.login'),
('admin.login.totp_disable_title', '2FA''yı Devre Dışı Bırak', 'Disable 2FA', 'admin.login'),
('admin.login.totp_disable_confirm', 'Parolanızı doğrulayın', 'Confirm your password', 'admin.login'),
('admin.login.totp_disable_btn', 'Devre Dışı Bırak', 'Disable', 'admin.login'),
('admin.login.recovery_title', 'Parola Sıfırlama', 'Password Recovery', 'admin.login'),
('admin.login.recovery_hint', 'E-posta adresinize bir sıfırlama bağlantısı gönderilecek.', 'A reset link will be sent to your email address.', 'admin.login'),
('admin.login.recovery_submit', 'Sıfırlama Bağlantısı Gönder', 'Send Reset Link', 'admin.login'),
('admin.login.back_to_login', 'Girişe dön', 'Back to login', 'admin.login'),
('admin.login.footer', 'UMSA Prodüksiyon © 2026 · Yetkisiz erişim girişimleri kayıt altına alınır.', 'UMSA Production © 2026 · Unauthorized access attempts are logged.', 'admin.login'),
('admin.login.error_invalid', 'Geçersiz e-posta veya parola.', 'Invalid email or password.', 'admin.login'),
('admin.login.error_totp', 'Geçersiz doğrulama kodu.', 'Invalid verification code.', 'admin.login'),
('admin.login.error_network', 'Ağ bağlantı hatası.', 'Network connection error.', 'admin.login'),
('admin.login.success_recovery', 'Sıfırlama bağlantısı gönderildi.', 'Reset link sent.', 'admin.login')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- Editorial / Content Management
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.editorial.content_types.news', 'Haber', 'News', 'admin.editorial'),
('admin.editorial.content_types.analysis', 'Analiz', 'Analysis', 'admin.editorial'),
('admin.editorial.content_types.interview', 'Röportaj', 'Interview', 'admin.editorial'),
('admin.editorial.content_types.review', 'İnceleme', 'Review', 'admin.editorial'),
('admin.editorial.content_types.opinion', 'Köşe Yazısı', 'Opinion', 'admin.editorial'),
('admin.editorial.content_types.report', 'Rapor', 'Report', 'admin.editorial'),
('admin.editorial.content_types.photo_gallery', 'Foto Galeri', 'Photo Gallery', 'admin.editorial'),
('admin.editorial.content_types.video', 'Video', 'Video', 'admin.editorial'),
('admin.editorial.content_types.infographic', 'İnfografik', 'Infographic', 'admin.editorial'),
('admin.editorial.content_types.sponsored', 'Sponsorlu İçerik', 'Sponsored Content', 'admin.editorial'),
('admin.editorial.placeholder_title', 'Örn: KAAN Milli Muharip Uçak Yeni Faz Testleri', 'E.g.: KAAN National Combat Aircraft New Phase Tests', 'admin.editorial'),
('admin.editorial.placeholder_summary', 'Makale veya haberin yönetici özeti...', 'Article or news executive summary...', 'admin.editorial'),
('admin.editorial.placeholder_body', 'Detaylı teknik analiz ve haber metni...', 'Detailed technical analysis and news text...', 'admin.editorial'),
('admin.editorial.default_author', 'MSI Editoryal Masası', 'MSI Editorial Desk', 'admin.editorial'),
('admin.editorial.default_author_title', 'Savunma Sanayii Analisti', 'Defence Industry Analyst', 'admin.editorial'),
('admin.editorial.seo_preview_title', 'Başlık burada görünecek', 'Title will appear here', 'admin.editorial'),
('admin.editorial.seo_preview_desc', 'Açıklama metni burada görünecek...', 'Description text will appear here...', 'admin.editorial'),
('admin.editorial.media_loading', 'Medya kütüphanesi yükleniyor...', 'Media library loading...', 'admin.editorial')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- Content Status Labels
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.status.draft', 'Taslak', 'Draft', 'admin.status'),
('admin.status.pending', 'Onay Bekliyor', 'Pending Review', 'admin.status'),
('admin.status.published', 'Yayında', 'Published', 'admin.status'),
('admin.status.rejected', 'Reddedildi', 'Rejected', 'admin.status'),
('admin.status.archived', 'Arşivlendi', 'Archived', 'admin.status')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- Commercial / Advertising
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.commercial.oem_protection_banner', 'Sponsorlu İçerik Rakip OEM Koruması Aktif — Sponsorlu içerik sayfalarında rakip savunma üreticisi reklamları otomatik engellenir.', 'Sponsored Content Competitor OEM Protection Active — Competitor defence manufacturer ads are automatically blocked on sponsored content pages.', 'admin.commercial'),
('admin.commercial.corporate_subtitle', '.gov.tr ve .tsk.tr uzantılı kurumsal e-postalar otomatik doğrulanır.', '.gov.tr and .tsk.tr corporate emails are automatically verified.', 'admin.commercial'),
('admin.commercial.ad_slot.header_banner', 'Üst Banner', 'Header Banner', 'admin.commercial'),
('admin.commercial.ad_slot.sidebar_top', 'Sidebar Üst', 'Sidebar Top', 'admin.commercial'),
('admin.commercial.ad_slot.sidebar_bottom', 'Sidebar Alt', 'Sidebar Bottom', 'admin.commercial'),
('admin.commercial.ad_slot.in_content', 'İçerik İçi', 'In-Content', 'admin.commercial'),
('admin.commercial.ad_slot.after_content', 'İçerik Sonrası', 'After Content', 'admin.commercial'),
('admin.commercial.ad_slot.footer', 'Alt Banner', 'Footer Banner', 'admin.commercial'),
('admin.commercial.ad_slot.magazine_cover', 'Dergi Kapağı Yanı', 'Magazine Cover Side', 'admin.commercial'),
('admin.commercial.ad_slot.search_results', 'Arama Sonuçları', 'Search Results', 'admin.commercial'),
('admin.commercial.ad_slot.native_feed', 'Native Feed', 'Native Feed', 'admin.commercial'),
('admin.commercial.revenue_subscription', 'Abonelik', 'Subscription', 'admin.commercial'),
('admin.commercial.revenue_ads', 'Reklam', 'Advertising', 'admin.commercial'),
('admin.commercial.revenue_commerce', 'E-Ticaret', 'E-Commerce', 'admin.commercial'),
('admin.commercial.revenue_corporate', 'Kurumsal', 'Corporate', 'admin.commercial')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- KPI Labels
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.kpi.total_contents', 'YAYINDAKİ İÇERİK', 'PUBLISHED CONTENT', 'admin.kpi'),
('admin.kpi.total_contents_sub', 'Çoklu bölüm yayını', 'Multi-section publishing', 'admin.kpi'),
('admin.kpi.true_reads', 'DOĞRULANMIŞ OKUMA', 'VERIFIED READS', 'admin.kpi'),
('admin.kpi.true_reads_sub', 'Toplam görüntülenme', 'Total page views', 'admin.kpi'),
('admin.kpi.active_ads', 'AKTİF KAMPANYA', 'ACTIVE CAMPAIGNS', 'admin.kpi'),
('admin.kpi.active_ads_sub', 'Hedefli B2B gösterim', 'Targeted B2B impressions', 'admin.kpi'),
('admin.kpi.gov_requests', 'KURUMSAL BAŞVURU', 'CORPORATE REQUESTS', 'admin.kpi'),
('admin.kpi.gov_requests_sub', 'Kamu & kurum doğrulama', 'Government & institution verification', 'admin.kpi')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- Role Labels
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.role.super_admin', 'Süper Yönetici', 'Super Admin', 'admin.role'),
('admin.role.admin', 'Yönetici', 'Admin', 'admin.role'),
('admin.role.site_operator', 'Site Operatörü', 'Site Operator', 'admin.role'),
('admin.role.editor_in_chief', 'Genel Yayın Yönetmeni', 'Editor-in-Chief', 'admin.role'),
('admin.role.managing_editor', 'Yazı İşleri Müdürü', 'Managing Editor', 'admin.role'),
('admin.role.content_publisher', 'İçerik Yayıncı', 'Content Publisher', 'admin.role'),
('admin.role.content_editor', 'İçerik Editörü', 'Content Editor', 'admin.role'),
('admin.role.editor', 'Editör', 'Editor', 'admin.role'),
('admin.role.business_dev', 'İş Geliştirme', 'Business Development', 'admin.role'),
('admin.role.ads_manager', 'Reklam Yöneticisi', 'Ads Manager', 'admin.role'),
('admin.role.finance_viewer', 'Finans Görüntüleyici', 'Finance Viewer', 'admin.role'),
('admin.role.support_agent', 'Destek Temsilcisi', 'Support Agent', 'admin.role'),
('admin.role.subscriber_public', 'Genel Abone', 'Public Subscriber', 'admin.role'),
('admin.role.subscriber_corporate', 'Kurumsal Abone', 'Corporate Subscriber', 'admin.role'),
('admin.role.subscriber_print_digital', 'Basılı+Dijital Abone', 'Print+Digital Subscriber', 'admin.role'),
('admin.role.subscriber_digital', 'Dijital Abone', 'Digital Subscriber', 'admin.role'),
('admin.role.registered', 'Kayıtlı Kullanıcı', 'Registered User', 'admin.role')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- Toast & Error Messages
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.toast.success', 'İşlem tamamlandı.', 'Operation completed.', 'admin.toast'),
('admin.toast.save_success', 'Kaydedildi.', 'Saved.', 'admin.toast'),
('admin.toast.delete_success', 'Silindi.', 'Deleted.', 'admin.toast'),
('admin.toast.upload_success', 'Yüklendi.', 'Uploaded.', 'admin.toast'),
('admin.toast.publish_success', 'Yayınlandı.', 'Published.', 'admin.toast'),
('admin.toast.approve_success', 'Onaylandı.', 'Approved.', 'admin.toast'),
('admin.toast.reject_success', 'Reddedildi.', 'Rejected.', 'admin.toast'),
('admin.error.session_expired', 'Oturum süresi doldu (401).', 'Session expired (401).', 'admin.error'),
('admin.error.forbidden', 'Bu işlem için yetkiniz bulunmuyor (403).', 'You do not have permission for this action (403).', 'admin.error'),
('admin.error.network', 'Ağ bağlantı hatası.', 'Network connection error.', 'admin.error'),
('admin.error.server', 'Sunucu hatası oluştu.', 'A server error occurred.', 'admin.error'),
('admin.error.validation', 'Form verileri geçersiz.', 'Form data is invalid.', 'admin.error'),
('admin.error.not_found', 'Kayıt bulunamadı.', 'Record not found.', 'admin.error')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- Analytics Chart Labels
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.chart.funnel.visitor', 'Ziyaretçi', 'Visitor', 'admin.chart'),
('admin.chart.funnel.registered', 'Kayıtlı', 'Registered', 'admin.chart'),
('admin.chart.funnel.subscriber', 'Abone', 'Subscriber', 'admin.chart'),
('admin.chart.funnel.paying', 'Ödeyen', 'Paying', 'admin.chart'),
('admin.export.filename_contents', 'icerik_raporu', 'content_report', 'admin.export'),
('admin.export.filename_analytics', 'analitik_raporu', 'analytics_report', 'admin.export')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- System & RBAC Labels
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.system.reminder_2_months', '2 Ay Kala (60 Gün)', '2 Months Remaining (60 Days)', 'admin.system'),
('admin.system.reminder_1_month', '1 Ay Kala (30 Gün)', '1 Month Remaining (30 Days)', 'admin.system'),
('admin.system.reminder_1_week', '1 Hafta Kala (7 Gün)', '1 Week Remaining (7 Days)', 'admin.system'),
('admin.system.reminder_expired', 'Süresi Dolmuş', 'Expired', 'admin.system'),
('admin.operator.badge', 'Operatör', 'Operator', 'admin.system'),
('admin.operator.role_tag', 'Yetkili', 'Authorized', 'admin.system'),
('admin.site_link.tr', 'TR', 'TR', 'admin.system'),
('admin.site_link.en', 'EN', 'EN', 'admin.system')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- Common Form Labels
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.form.save', 'Kaydet', 'Save', 'admin.form'),
('admin.form.cancel', 'İptal', 'Cancel', 'admin.form'),
('admin.form.delete', 'Sil', 'Delete', 'admin.form'),
('admin.form.edit', 'Düzenle', 'Edit', 'admin.form'),
('admin.form.create', 'Oluştur', 'Create', 'admin.form'),
('admin.form.search', 'Ara', 'Search', 'admin.form'),
('admin.form.filter', 'Filtrele', 'Filter', 'admin.form'),
('admin.form.upload', 'Yükle', 'Upload', 'admin.form'),
('admin.form.preview', 'Önizleme', 'Preview', 'admin.form'),
('admin.form.publish', 'Yayınla', 'Publish', 'admin.form'),
('admin.form.approve', 'Onayla', 'Approve', 'admin.form'),
('admin.form.reject', 'Reddet', 'Reject', 'admin.form'),
('admin.form.close', 'Kapat', 'Close', 'admin.form'),
('admin.form.loading', 'Yükleniyor...', 'Loading...', 'admin.form'),
('admin.form.no_data', 'Veri bulunamadı.', 'No data found.', 'admin.form'),
('admin.form.confirm_delete', 'Bu kaydı silmek istediğinize emin misiniz?', 'Are you sure you want to delete this record?', 'admin.form')
ON CONFLICT (key) DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════
-- Page Title
-- ═══════════════════════════════════════════════════════════════════
INSERT INTO translations (key, value_tr, value_en, grp) VALUES
('admin.page_title', 'UMSA Komuta — MSI Dergisi & MSI TDR Yayın ve Operasyon Merkezi', 'UMSA Command — MSI Magazine & MSI TDR Publishing and Operations Center', 'admin.system')
ON CONFLICT (key) DO NOTHING;
