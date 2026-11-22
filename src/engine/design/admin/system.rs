//! System, Security & Infrastructure Views:
//! `#tab-8` Central Engines, i18n, Glossary & Locales, `#tab-11` RBAC & Audit Trail,
//! `#tab-17` Live Server & Domain Telemetry, `#tab-7` Renewals & 301 Redirects,
//! `#tab-13` Dynamic Custom Fields, Multi-Layer Cache & Database Backup.

use super::tr_admin;
use super::ui::{
    action_btn, admin_card, check_pill, details_box, form_group, icon, input_ctrl, kpi_card_toned,
    ops_table, select_ctrl, subtab_bar, subtab_pane, tab_pane,
};
use crate::engine::design::html_dsl::HtmlBuf;
use crate::engine::translation_engine::TranslationCache;

const RBAC_PERMISSION_GROUPS: &[(&str, &str, &str, &[&str])] = &[
    (
        "admin.rbac.group_editorial",
        "İçerik & Dergi",
        "Editorial & Magazine",
        &[
            "contents:read",
            "contents:write",
            "contents:publish",
            "contents:delete",
            "contents:manage",
            "approvals:read",
            "approvals:approve",
            "approvals:reject",
            "approvals:manage",
            "categories:read",
            "categories:write",
            "categories:manage",
            "issues:read",
            "issues:write",
            "issues:delete",
            "issues:manage",
            "media:read",
            "media:upload",
            "media:delete",
            "media:manage",
            "edergi:read",
            "edergi:download",
            "articles:read",
            "articles:premium",
        ],
    ),
    (
        "admin.rbac.group_commercial",
        "Ticari & Bülten",
        "Commercial & Newsletter",
        &[
            "ads:read",
            "ads:write",
            "ads:manage",
            "newsletters:read",
            "newsletters:write",
            "newsletters:manage",
            "email_groups:write",
            "email_groups:manage",
            "subscriptions:read",
            "subscriptions:write",
            "subscriptions:manage",
            "commerce:read",
            "commerce:write",
            "commerce:manage",
            "invoices:read",
            "invoices:write",
            "invoices:manage",
            "corporate_requests:read",
            "corporate_requests:decide",
            "corporate_requests:manage",
            "revenue:read",
        ],
    ),
    (
        "admin.rbac.group_system",
        "Sistem & Güvenlik",
        "System & Security",
        &[
            "users:read",
            "users:write",
            "users:verify",
            "users:manage",
            "roles:manage",
            "audit:read",
            "audit:write",
            "analytics:read",
            "analytics:export",
            "notifications:read",
            "notifications:write",
            "notifications:manage",
            "translations:read",
            "translations:write",
            "translations:manage",
            "glossary:write",
            "glossary:manage",
            "locales:manage",
            "redirects:write",
            "redirects:manage",
            "settings:read",
            "settings:write",
            "settings:manage",
            "system:read",
            "monitoring:read",
            "backup:manage",
        ],
    ),
];

pub fn render(buf: &mut HtmlBuf, seg: &str, tr: &TranslationCache, lang: &str) {
    render_tab_engines(buf, seg, tr, lang);
    render_tab_rbac(buf, tr, lang);
    render_tab_monitoring(buf, tr, lang);
    render_tab_automation(buf, tr, lang);
    render_tab_fields_cache(buf, tr, lang);
}

fn render_tab_engines(buf: &mut HtmlBuf, seg: &str, tr: &TranslationCache, lang: &str) {
    let studio_href = format!("/{}/tasarim", seg);

    let sub_settings = tr_admin(tr, lang, "admin.system.subtab_settings", "Site Ayarları & Bildirim Motoru", "Site Settings & Notifications");
    let sub_waf = tr_admin(tr, lang, "admin.system.subtab_waf", "WAF Güvenlik Kayıtları", "WAF Security Logs");
    let sub_i18n = tr_admin(tr, lang, "admin.system.subtab_i18n", "Çeviri Sözlüğü (TR / EN)", "Translation Dictionary (TR / EN)");
    let sub_glossary = tr_admin(tr, lang, "admin.system.subtab_glossary", "Terim Sözlüğü (Glossary)", "Terminology Glossary");
    let sub_locales = tr_admin(tr, lang, "admin.system.subtab_locales", "Dil Yönetimi (Locales)", "Locale Registry");

    let set_title = tr_admin(tr, lang, "admin.system.settings_title", "Merkezi Site Ayarları", "Central Site Settings");
    let set_desc = tr_admin(tr, lang, "admin.system.settings_desc", "Üst duyuru metni, aktif dergi sayısı ve iletişim parametrelerini L1 önbellekle anında güncelleyin.", "Update announcement banner, featured issue and contact parameters instantaneously via L1 cache.");
    let studio_btn = tr_admin(tr, lang, "admin.sidebar.studio", "Tasarım Stüdyosu ↗", "Design Studio ↗");
    let tfa_btn = tr_admin(tr, lang, "admin.system.configure_2fa", "2FA Yapılandır", "Configure 2FA");
    let lbl_set_key = tr_admin(tr, lang, "admin.system.set_key_lbl", "Ayar Anahtarı", "Setting Key");
    let opt_banner = tr_admin(tr, lang, "admin.system.opt_banner", "Üst Duyuru Bandı (announcement_banner_text)", "Top Announcement Banner (announcement_banner_text)");
    let opt_issue = tr_admin(tr, lang, "admin.system.opt_issue", "Öne Çıkan Sayı No (current_featured_issue)", "Featured Issue No (current_featured_issue)");
    let opt_phone = tr_admin(tr, lang, "admin.system.opt_phone", "Kurumsal Telefon (contact_phone)", "Corporate Phone (contact_phone)");
    let opt_at_provider = tr_admin(tr, lang, "admin.system.opt_at_provider", "Otomatik Çeviri Sağlayıcı (auto_translate_provider)", "Auto-Translate Provider (auto_translate_provider)");
    let opt_at_url = tr_admin(tr, lang, "admin.system.opt_at_url", "Otomatik Çeviri API Adresi (auto_translate_url)", "Auto-Translate API URL (auto_translate_url)");
    let opt_at_key = tr_admin(tr, lang, "admin.system.opt_at_key", "Otomatik Çeviri API Anahtarı (auto_translate_api_key)", "Auto-Translate API Key (auto_translate_api_key)");
    let lbl_set_group = tr_admin(tr, lang, "admin.system.set_group_lbl", "Ayar Grubu", "Setting Group");
    let lbl_val_tr = tr_admin(tr, lang, "admin.system.set_val_tr", "Türkçe Değer", "Turkish Value");
    let ph_val_tr = tr_admin(tr, lang, "admin.system.set_val_tr_ph", "Yeni Türkçe değer...", "New Turkish value...");
    let lbl_val_en = tr_admin(tr, lang, "admin.system.set_val_en", "İngilizce Karşılığı (EN)", "English Value (EN)");
    let ph_val_en = tr_admin(tr, lang, "admin.system.set_val_en_ph", "Corresponding English value...", "Corresponding English value...");
    let save_set_btn = tr_admin(tr, lang, "admin.system.save_setting", "Ayarı Kaydet ve Önbelleği Yenile", "Save Setting & Refresh Cache");

    let notif_title = tr_admin(tr, lang, "admin.system.notif_title", "Merkezi Bildirim & Üst Duyuru Motoru", "Central Notification & Banner Engine");
    let notif_desc = tr_admin(tr, lang, "admin.system.notif_desc", "Portal genelinde veya belirli savunma kurumlarına özel anlık duyuru yayınlayın.", "Publish portal-wide or institution-targeted announcements instantly.");
    let lbl_audience = tr_admin(tr, lang, "admin.system.notif_audience", "Hedef Kitle", "Target Audience");
    let aud_all = tr_admin(tr, lang, "admin.system.aud_all", "Tüm Ziyaretçiler", "All Visitors");
    let aud_msb = tr_admin(tr, lang, "admin.system.aud_msb", "MSB Personeli", "MoD (MSB) Personnel");
    let aud_ssb = tr_admin(tr, lang, "admin.system.aud_ssb", "SSB Personeli", "SSB Personnel");
    let aud_sector = tr_admin(tr, lang, "admin.system.aud_sector", "Sektör Kuruluşları", "Industry Organisations");
    let lbl_severity = tr_admin(tr, lang, "admin.system.notif_severity", "Önem Derecesi", "Severity");
    let sev_info = tr_admin(tr, lang, "admin.system.sev_info", "Bilgi", "Info");
    let sev_success = tr_admin(tr, lang, "admin.system.sev_success", "Başarı", "Success");
    let sev_warning = tr_admin(tr, lang, "admin.system.sev_warning", "Uyarı", "Warning");
    let sev_critical = tr_admin(tr, lang, "admin.system.sev_critical", "Kritik", "Critical");
    let lbl_banner = tr_admin(tr, lang, "admin.system.notif_banner", "Üst Duyuru Bandı", "Top Banner");
    let ban_yes = tr_admin(tr, lang, "admin.system.ban_yes", "Evet — Üst Bantta Göster", "Yes — Show in Top Bar");
    let ban_no = tr_admin(tr, lang, "admin.system.ban_no", "Hayır — Sadece Bildirim", "No — Notification Only");
    let lbl_notif_title = tr_admin(tr, lang, "admin.system.notif_title_tr", "Bildirim Başlığı (TR)", "Notification Title (TR)");
    let ph_notif_title = tr_admin(tr, lang, "admin.system.notif_title_ph", "Örn: MSI Dergisi 286. Sayısı Dijital Kiosk'ta Yayında", "e.g. MSI Magazine Issue 286 Now Live in Digital Kiosk");
    let lbl_notif_msg = tr_admin(tr, lang, "admin.system.notif_msg_tr", "Bildirim Detayı (TR)", "Notification Detail (TR)");
    let ph_notif_msg = tr_admin(tr, lang, "admin.system.notif_msg_ph", "Kamu ve TSK personeline ücretsiz dijital erişim açıktır.", "Free digital access is open for public sector and TAF personnel.");
    let publish_notif_btn = tr_admin(tr, lang, "admin.system.publish_notif", "Duyuruyu Yayınla", "Publish Announcement");

    let active_sets_title = tr_admin(tr, lang, "admin.system.active_settings_title", "Aktif Site Ayarları", "Active Site Settings");
    let col_key = tr_admin(tr, lang, "admin.system.col_key", "Anahtar", "Key");
    let col_tr_val = tr_admin(tr, lang, "admin.system.col_tr_val", "Türkçe Değer", "Turkish Value");
    let col_en_val = tr_admin(tr, lang, "admin.system.col_en_val", "İngilizce Değer", "English Value");
    let col_group = tr_admin(tr, lang, "admin.system.col_group", "Grup", "Group");
    let sets_loading = tr_admin(tr, lang, "admin.system.settings_loading", "Ayarlar yükleniyor...", "Loading settings...");

    let active_notifs_title = tr_admin(tr, lang, "admin.system.active_notifs_title", "Yayındaki Sistem Bildirimleri", "Published System Notifications");
    let col_target = tr_admin(tr, lang, "admin.system.col_target", "Hedef", "Target");
    let col_sev = tr_admin(tr, lang, "admin.system.col_severity", "Önem", "Severity");
    let col_title_msg = tr_admin(tr, lang, "admin.system.col_title_msg", "Başlık & Mesaj", "Title & Message");
    let col_loc = tr_admin(tr, lang, "admin.system.col_location", "Konum", "Placement");
    let notifs_loading = tr_admin(tr, lang, "admin.system.notifs_loading", "Bildirimler yükleniyor...", "Loading notifications...");

    let waf_title = tr_admin(tr, lang, "admin.system.waf_title", "WAF Uygulama Güvenlik Duvarı Kayıtları", "WAF Web Application Firewall Logs");
    let waf_desc = tr_admin(tr, lang, "admin.system.waf_desc", "SQL enjeksiyonu, XSS, dizin geçişi ve hız sınırı ihlallerinin gerçek zamanlı blokaj kayıtları.", "Real-time block logs for SQL injection, XSS, path traversal and rate-limit violations.");
    let waf_search_ph = tr_admin(tr, lang, "admin.system.waf_search_ph", "IP, tehdit türü veya yol filtrele...", "Filter IP, threat type or path...");
    let waf_search_aria = tr_admin(tr, lang, "admin.system.waf_search_aria", "WAF kayıtlarında ara", "Search WAF logs");
    let col_ts = tr_admin(tr, lang, "admin.system.col_timestamp", "Zaman Damgası", "Timestamp");
    let col_ip = tr_admin(tr, lang, "admin.system.col_source_ip", "Kaynak IP", "Source IP");
    let col_threat = tr_admin(tr, lang, "admin.system.col_threat_type", "Tehdit Türü", "Threat Type");
    let col_risk = tr_admin(tr, lang, "admin.system.col_risk_level", "Risk Seviyesi", "Risk Level");
    let col_req_detail = tr_admin(tr, lang, "admin.system.col_req_detail", "İstek Yolu & Ayrıntı", "Request Path & Detail");
    let waf_loading = tr_admin(tr, lang, "admin.system.waf_loading", "WAF güvenlik kayıtları yükleniyor...", "Loading WAF security logs...");

    let tr_upsert_title = tr_admin(tr, lang, "admin.system.tr_upsert_title", "Yeni Çeviri Anahtarı Ekle / Güncelle", "Add / Update Translation Key");
    let tr_key_lbl = tr_admin(tr, lang, "admin.system.tr_key_lbl", "Çeviri Anahtarı", "Translation Key");
    let tr_key_ph = tr_admin(tr, lang, "admin.system.tr_key_ph", "Örn: nav.subscribe", "e.g. nav.subscribe");
    let tr_grp_lbl = tr_admin(tr, lang, "admin.system.tr_group_lbl", "Bölüm Grubu", "Section Group");
    let tr_val_tr_lbl = tr_admin(tr, lang, "admin.system.tr_val_tr_lbl", "Türkçe Metin (TR)", "Turkish Text (TR)");
    let tr_val_tr_ph = tr_admin(tr, lang, "admin.system.tr_val_tr_ph", "Türkçe karşılığı...", "Turkish translation...");
    let tr_val_en_lbl = tr_admin(tr, lang, "admin.system.tr_val_en_lbl", "İngilizce Metin (EN)", "English Text (EN)");
    let tr_val_en_ph = tr_admin(tr, lang, "admin.system.tr_val_en_ph", "English translation...", "English translation...");
    let tr_save_btn = tr_admin(tr, lang, "admin.system.tr_save_btn", "Çeviriyi Kaydet", "Save Translation");
    let tr_auto_btn = tr_admin(tr, lang, "admin.system.tr_auto_btn", "Otomatik Çevir (TR → EN)", "Auto-Translate (TR → EN)");
    let tr_dict_title = tr_admin(tr, lang, "admin.system.tr_dict_title", "Merkezi Çok Dilli Çeviri Sözlüğü", "Central Multilingual Translation Dictionary");
    let tr_dict_desc = tr_admin(tr, lang, "admin.system.tr_dict_desc", "Arayüz metinlerini anahtar veya içerik bazında arayın ve anında düzenleyin.", "Search interface strings by key or content and edit immediately.");
    let tr_search_ph = tr_admin(tr, lang, "admin.system.tr_search_ph", "Anahtar veya metin ara...", "Search key or text...");
    let tr_search_aria = tr_admin(tr, lang, "admin.system.tr_search_aria", "Çeviri ara", "Search translation");
    let col_key_grp = tr_admin(tr, lang, "admin.system.col_key_group", "Anahtar & Grup", "Key & Group");
    let col_tr = tr_admin(tr, lang, "admin.system.col_tr", "Türkçe (TR)", "Turkish (TR)");
    let col_en = tr_admin(tr, lang, "admin.system.col_en", "İngilizce (EN)", "English (EN)");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let tr_loading = tr_admin(tr, lang, "admin.system.tr_loading", "Çeviri sözlüğü yükleniyor...", "Loading translation dictionary...");

    let tr_pending_title = tr_admin(tr, lang, "admin.system.tr_pending_title", "Onay Bekleyen Çeviriler", "Pending Translation Reviews");
    let tr_pending_desc = tr_admin(tr, lang, "admin.system.tr_pending_desc", "Taslak veya otomatik çevrilmiş arayüz metinlerini inceleyip onaylayın veya reddedin.", "Review, approve or reject draft and auto-translated interface strings.");
    let tr_st_all_pending = tr_admin(tr, lang, "admin.system.tr_st_all_pending", "Tüm Bekleyenler", "All Pending");
    let tr_st_draft = tr_admin(tr, lang, "admin.status.draft", "Taslak", "Draft");
    let tr_st_auto = tr_admin(tr, lang, "admin.system.tr_st_auto", "Otomatik Çeviri", "Auto-Translated");
    let tr_st_approved = tr_admin(tr, lang, "admin.status.approved", "Onaylı", "Approved");
    let tr_st_all = tr_admin(tr, lang, "admin.status.all", "Tümü", "All");
    let tr_pending_aria = tr_admin(tr, lang, "admin.system.tr_pending_filter_aria", "Çeviri onay durumu filtresi", "Translation review status filter");
    let refresh_btn = tr_admin(tr, lang, "admin.form.refresh", "Yenile", "Refresh");
    let col_status = tr_admin(tr, lang, "admin.workflow.col_status", "Durum", "Status");
    let tr_pending_loading = tr_admin(tr, lang, "admin.system.tr_pending_loading", "Bekleyen çeviriler yükleniyor...", "Loading pending translations...");

    let gl_title = tr_admin(tr, lang, "admin.system.gl_title", "Savunma & Editoryal Terim Sözlüğü (Glossary)", "Defence & Editorial Terminology Glossary");
    let gl_desc = tr_admin(tr, lang, "admin.system.gl_desc", "Otomatik çevirilerde zorunlu, korunacak veya yasaklı terim kurallarını yönetin.", "Manage fixed, preserved and forbidden terminology rules for automated translations.");
    let gl_dom_all = tr_admin(tr, lang, "admin.system.gl_dom_all", "Tüm Alanlar", "All Domains");
    let gl_dom_style = tr_admin(tr, lang, "admin.system.gl_dom_style", "Stil", "Style");
    let gl_dom_defence = tr_admin(tr, lang, "admin.system.gl_dom_defence", "Savunma", "Defence");
    let gl_dom_aero = tr_admin(tr, lang, "admin.system.gl_dom_aero", "Havacılık", "Aerospace");
    let gl_dom_naval = tr_admin(tr, lang, "admin.system.gl_dom_naval", "Deniz", "Naval");
    let gl_dom_edit = tr_admin(tr, lang, "admin.system.gl_dom_editorial", "Editoryal", "Editorial");
    let gl_dom_aria = tr_admin(tr, lang, "admin.system.gl_dom_aria", "Terim alanı filtresi", "Glossary domain filter");
    let gl_search_ph = tr_admin(tr, lang, "admin.system.gl_search_ph", "Terim ara...", "Search term...");
    let gl_search_aria = tr_admin(tr, lang, "admin.system.gl_search_aria", "Terim sözlüğünde ara", "Search glossary");
    let gl_active_only = tr_admin(tr, lang, "admin.system.gl_active_only", "Sadece Aktifler", "Active Only");
    let gl_new_btn = tr_admin(tr, lang, "admin.system.gl_new_btn", "Yeni Terim", "New Term");
    let gl_term_tr_lbl = tr_admin(tr, lang, "admin.system.gl_term_tr", "TR Terim", "TR Term");
    let gl_term_en_lbl = tr_admin(tr, lang, "admin.system.gl_term_en", "EN Terim", "EN Term");
    let gl_pref_en_lbl = tr_admin(tr, lang, "admin.system.gl_pref_en", "Tercih Edilen EN", "Preferred EN");
    let gl_abbr_tr_lbl = tr_admin(tr, lang, "admin.system.gl_abbr_tr", "TR Kısaltma", "TR Abbr.");
    let gl_abbr_en_lbl = tr_admin(tr, lang, "admin.system.gl_abbr_en", "EN Kısaltma", "EN Abbr.");
    let gl_domain_lbl = tr_admin(tr, lang, "admin.system.gl_domain", "Alan", "Domain");
    let gl_rule_lbl = tr_admin(tr, lang, "admin.system.gl_rule_type", "Kural Tipi", "Rule Type");
    let gl_rule_fixed = tr_admin(tr, lang, "admin.system.gl_rule_fixed", "Sabit (fixed)", "Fixed");
    let gl_rule_preserve = tr_admin(tr, lang, "admin.system.gl_rule_preserve", "Koru (preserve)", "Preserve");
    let gl_rule_forbidden = tr_admin(tr, lang, "admin.system.gl_rule_forbidden", "Yasaklı (forbidden)", "Forbidden");
    let gl_prio_lbl = tr_admin(tr, lang, "admin.system.gl_priority", "Öncelik", "Priority");
    let gl_src_lbl = tr_admin(tr, lang, "admin.system.gl_source_ref", "Kaynak Referans", "Source Reference");
    let gl_forbid_lbl = tr_admin(tr, lang, "admin.system.gl_forbidden_variants", "Yasaklı Varyantlar (Virgülle)", "Forbidden Variants (Comma-separated)");
    let gl_notes_lbl = tr_admin(tr, lang, "admin.system.gl_notes", "Notlar", "Notes");
    let gl_case_lbl = tr_admin(tr, lang, "admin.system.gl_case_sensitive", "Büyük/Küçük Harf Duyarlı", "Case Sensitive");
    let gl_is_active_lbl = tr_admin(tr, lang, "admin.system.gl_is_active", "Aktif", "Active");
    let gl_save_btn = tr_admin(tr, lang, "admin.system.gl_save", "Terimi Kaydet", "Save Term");
    let cancel_btn = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");
    let gl_col_dom = tr_admin(tr, lang, "admin.system.gl_col_dom_rule", "Alan & Kural", "Domain & Rule");
    let gl_col_forbid = tr_admin(tr, lang, "admin.system.gl_col_forbidden", "Yasaklı Varyantlar", "Forbidden Variants");
    let gl_loading = tr_admin(tr, lang, "admin.system.gl_loading", "Terim sözlüğü yükleniyor...", "Loading terminology glossary...");

    let loc_title = tr_admin(tr, lang, "admin.system.loc_title", "Dil & Yerelleştirme Kayıt Defteri (Locales)", "Language & Locale Registry");
    let loc_desc = tr_admin(tr, lang, "admin.system.loc_desc", "Portalda desteklenen dilleri, hreflang eşlemelerini ve yönelim (LTR/RTL) ayarlarını yönetin.", "Manage supported portal locales, hreflang mappings and text direction (LTR/RTL) settings.");
    let loc_new_btn = tr_admin(tr, lang, "admin.system.loc_new", "Yeni Dil Ekle", "Add Locale");
    let loc_code_lbl = tr_admin(tr, lang, "admin.system.loc_code", "Dil Kodu", "Locale Code");
    let loc_native_lbl = tr_admin(tr, lang, "admin.system.loc_native", "Yerel Ad", "Native Name");
    let loc_en_lbl = tr_admin(tr, lang, "admin.system.loc_en", "İngilizce Ad", "English Name");
    let loc_hreflang_lbl = tr_admin(tr, lang, "admin.system.loc_hreflang", "hreflang", "hreflang");
    let loc_og_lbl = tr_admin(tr, lang, "admin.system.loc_og", "og:locale", "og:locale");
    let loc_dir_lbl = tr_admin(tr, lang, "admin.system.loc_dir", "Yazı Yönü", "Text Direction");
    let loc_fb_lbl = tr_admin(tr, lang, "admin.system.loc_fallback", "Yedek Dil Kodu", "Fallback Code");
    let loc_sort_lbl = tr_admin(tr, lang, "admin.system.loc_sort", "Sıra", "Sort Order");
    let loc_enabled_lbl = tr_admin(tr, lang, "admin.system.loc_enabled", "Etkin", "Enabled");
    let loc_default_lbl = tr_admin(tr, lang, "admin.system.loc_default", "Varsayılan Dil", "Default Locale");
    let loc_save_btn = tr_admin(tr, lang, "admin.system.loc_save", "Dili Kaydet", "Save Locale");
    let loc_col_code = tr_admin(tr, lang, "admin.system.loc_col_code", "Kod", "Code");
    let loc_col_names = tr_admin(tr, lang, "admin.system.loc_col_names", "Yerel / EN Ad", "Native / EN Name");
    let loc_col_href = tr_admin(tr, lang, "admin.system.loc_col_href", "hreflang / og:locale", "hreflang / og:locale");
    let loc_col_dir = tr_admin(tr, lang, "admin.system.loc_col_dir", "Yön & Yedek", "Dir & Fallback");
    let loc_loading = tr_admin(tr, lang, "admin.system.loc_loading", "Dil kayıtları yükleniyor...", "Loading locale registry...");

    tab_pane(buf, "tab-8", "tab-btn-8", false, |tab| {
        subtab_bar(tab, "tab8", &[
            ("tab8-settings", &sub_settings, Some("settings:read|notifications:read")),
            ("tab8-waf", &sub_waf, Some("audit:read|settings:read")),
            ("tab8-i18n", &sub_i18n, Some("translations:read|settings:read")),
            ("tab8-glossary", &sub_glossary, Some("glossary:write|glossary:manage")),
            ("tab8-locales", &sub_locales, Some("locales:manage")),
        ]);

        subtab_pane(tab, "tab8-settings", true, Some("settings:read|notifications:read"), |pane| {
            pane.el("div", &[("class", "workspace-grid")], |grid| {
                grid.el("div", &[("class", "flex-col gap-4")], |col| {
                    admin_card(
                        col, "", Some("settings:read"), "fa-sliders", &set_title,
                        Some(&set_desc),
                        |hdr| {
                            hdr.el("div", &[("class", "flex gap-2 items-center")], |act| {
                                act.el("a", &[("href", &studio_href), ("class", "btn-cmd btn-cmd-sm")], |a| {
                                    icon(a, "fa-paint-brush");
                                    a.raw(" ");
                                    a.text(&studio_btn);
                                });
                                action_btn(act, "btn-cmd btn-cmd-sm btn-cmd-warning", "open-2fa-modal", None, Some("fa-key"), &tfa_btn);
                            });
                        },
                        |card| {
                            let ro_settings = tr_admin(
                                tr,
                                lang,
                                "admin.system.settings_readonly_notice",
                                "Salt okunur erişim: Ayarları güncellemek için settings:write yetkisi gerekir.",
                                "Read-only access: settings:write permission is required to update settings.",
                            );
                            card.el("div", &[("class", "hidden text-xs text-muted p-4"), ("data-readonly-notice", "settings:write")], |n| {
                                n.el("span", &[("class", "badge-tag mr-2")], |b| {
                                    b.text(if lang.eq_ignore_ascii_case("en") { "Read-Only" } else { "Salt Okunur" });
                                });
                                n.text(&ro_settings);
                            });
                            card.el_with("form", |a| { a.attr("id", "form-site-setting").attr("data-required-perm", "settings:write").bool_attr("novalidate", true); }, |f| {
                                f.el("div", &[("class", "form-row-2")], |r| {
                                    form_group(r, Some("set-key"), &lbl_set_key, |g| {
                                        select_ctrl(g, "set-key", "", &[
                                            ("announcement_banner_text", &opt_banner),
                                            ("current_featured_issue", &opt_issue),
                                            ("contact_phone", &opt_phone),
                                            ("auto_translate_provider", &opt_at_provider),
                                            ("auto_translate_url", &opt_at_url),
                                            ("auto_translate_api_key", &opt_at_key),
                                        ], &[]);
                                    });
                                    form_group(r, Some("set-group"), &lbl_set_group, |g| input_ctrl(g, "text", "set-group", "", "theme", "", &[]));
                                });
                                form_group(f, Some("set-val-tr"), &lbl_val_tr, |g| input_ctrl(g, "text", "set-val-tr", "", "", &ph_val_tr, &[("required", "required")]));
                                form_group(f, Some("set-val-en"), &lbl_val_en, |g| input_ctrl(g, "text", "set-val-en", "", "", &ph_val_en, &[]));
                                f.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-primary w-full flex-center"), ("data-required-perm", "settings:write")], |b| {
                                    icon(b, "fa-bolt");
                                    b.raw(" ");
                                    b.text(&save_set_btn);
                                });
                            });
                        },
                    );

                    admin_card(
                        col, "", Some("notifications:read|settings:read"), "fa-bell", &notif_title,
                        Some(&notif_desc),
                        |_| {},
                        |card| {
                            let ro_notif = tr_admin(
                                tr,
                                lang,
                                "admin.system.notif_readonly_notice",
                                "Salt okunur erişim: Duyuru yayınlamak için notifications:write yetkisi gerekir.",
                                "Read-only access: notifications:write permission is required to publish notifications.",
                            );
                            card.el("div", &[("class", "hidden text-xs text-muted p-4"), ("data-readonly-notice", "notifications:write|settings:write")], |n| {
                                n.el("span", &[("class", "badge-tag mr-2")], |b| {
                                    b.text(if lang.eq_ignore_ascii_case("en") { "Read-Only" } else { "Salt Okunur" });
                                });
                                n.text(&ro_notif);
                            });
                            card.el_with("form", |a| { a.attr("id", "form-sys-notif").attr("data-required-perm", "notifications:write|settings:write").bool_attr("novalidate", true); }, |f| {
                                f.el("div", &[("class", "form-row-3")], |r| {
                                    form_group(r, Some("notif-audience"), &lbl_audience, |g| {
                                        select_ctrl(g, "notif-audience", "", &[
                                            ("all", &aud_all),
                                            ("MSB", &aud_msb),
                                            ("SSB", &aud_ssb),
                                            ("Sektör Kuruluşları", &aud_sector),
                                        ], &[]);
                                    });
                                    form_group(r, Some("notif-severity"), &lbl_severity, |g| {
                                        select_ctrl(g, "notif-severity", "", &[("info", &sev_info), ("success", &sev_success), ("warning", &sev_warning), ("critical", &sev_critical)], &[]);
                                    });
                                    form_group(r, Some("notif-banner"), &lbl_banner, |g| {
                                        select_ctrl(g, "notif-banner", "", &[("true", &ban_yes), ("false", &ban_no)], &[]);
                                    });
                                });
                                form_group(f, Some("notif-title-tr"), &lbl_notif_title, |g| input_ctrl(g, "text", "notif-title-tr", "", "", &ph_notif_title, &[("required", "required")]));
                                form_group(f, Some("notif-msg-tr"), &lbl_notif_msg, |g| input_ctrl(g, "text", "notif-msg-tr", "", "", &ph_notif_msg, &[("required", "required")]));
                                f.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-primary w-full flex-center"), ("data-required-perm", "notifications:write|settings:write")], |b| {
                                    icon(b, "fa-paper-plane");
                                    b.raw(" ");
                                    b.text(&publish_notif_btn);
                                });
                            });
                        },
                    );
                });

                grid.el("div", &[("class", "flex-col gap-4")], |col| {
                    admin_card(col, "", Some("settings:read"), "fa-cogs", &active_sets_title, None, |_| {}, |card| {
                        ops_table(card, "scrollable-panel-lg", None, &[(&col_key, ""), (&col_tr_val, ""), (&col_en_val, ""), (&col_group, "")], "tbody-site-settings", Some(&sets_loading), None);
                    });
                    admin_card(col, "", Some("notifications:read|settings:read"), "fa-bullhorn", &active_notifs_title, None, |_| {}, |card| {
                        ops_table(card, "scrollable-panel-lg", None, &[(&col_target, ""), (&col_sev, ""), (&col_title_msg, ""), (&col_loc, "")], "tbody-sys-notifications", Some(&notifs_loading), None);
                    });
                });
            });
        });

        subtab_pane(tab, "tab8-waf", false, Some("audit:read|settings:read"), |pane| {
            admin_card(
                pane, "", None, "fa-shield", &waf_title,
                Some(&waf_desc),
                |hdr| {
                    input_ctrl(hdr, "text", "waf-search", "input-lg", "", &waf_search_ph, &[("aria-label", &waf_search_aria)]);
                },
                |card| {
                    ops_table(card, "", None, &[(&col_ts, ""), (&col_ip, ""), (&col_threat, ""), (&col_risk, ""), (&col_req_detail, "")], "tbody-security-logs", Some(&waf_loading), Some("pagination-security-logs"));
                },
            );
        });

        subtab_pane(tab, "tab8-i18n", false, Some("translations:read|settings:read"), |pane| {
            details_box(pane, "admin-card mb-4", false, Some("translations:write|settings:write"), &tr_upsert_title, |d| {
                d.el_with("form", |a| { a.attr("id", "form-translation-upsert").bool_attr("novalidate", true); }, |f| {
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("tr-key"), &tr_key_lbl, |g| input_ctrl(g, "text", "tr-key", "", "", &tr_key_ph, &[("required", "required")]));
                        form_group(r, Some("tr-group"), &tr_grp_lbl, |g| input_ctrl(g, "text", "tr-group", "", "general", "general", &[]));
                    });
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("tr-val-tr"), &tr_val_tr_lbl, |g| input_ctrl(g, "text", "tr-val-tr", "", "", &tr_val_tr_ph, &[("required", "required")]));
                        form_group(r, Some("tr-val-en"), &tr_val_en_lbl, |g| input_ctrl(g, "text", "tr-val-en", "", "", &tr_val_en_ph, &[]));
                    });
                    f.el("div", &[("class", "flex gap-2 flex-wrap")], |act| {
                        act.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-primary btn-cmd-sm")], |b| {
                            icon(b, "fa-floppy-o");
                            b.raw(" ");
                            b.text(&tr_save_btn);
                        });
                        act.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("id", "btn-tr-auto-translate"),
                                ("class", "btn-cmd btn-cmd-sm"),
                                ("data-admin-action", "auto-translate-text"),
                                ("data-required-perm", "translations:write|translations:read"),
                            ],
                            |b| {
                                icon(b, "fa-magic");
                                b.raw(" ");
                                b.text(&tr_auto_btn);
                            },
                        );
                    });
                });
                d.el("div", &[("id", "tr-glossary-warnings"), ("class", "hidden form-container-elevated mt-3"), ("role", "status")], |_| {});
            });

            admin_card(
                pane, "mb-4", None, "fa-language", &tr_dict_title,
                Some(&tr_dict_desc),
                |hdr| input_ctrl(hdr, "text", "tr-search", "input-lg", "", &tr_search_ph, &[("aria-label", &tr_search_aria)]),
                |card| {
                    ops_table(card, "", None, &[(&col_key_grp, ""), (&col_tr, ""), (&col_en, ""), (&col_action, "")], "tbody-translations", Some(&tr_loading), Some("pagination-translations"));
                },
            );

            admin_card(
                pane, "", Some("translations:read"), "fa-clock-o", &tr_pending_title,
                Some(&tr_pending_desc),
                |hdr| {
                    hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                        select_ctrl(
                            tb,
                            "tr-pending-status-filter",
                            "input-md",
                            &[
                                ("", &tr_st_all_pending),
                                ("draft", &tr_st_draft),
                                ("auto_translated", &tr_st_auto),
                                ("approved", &tr_st_approved),
                                ("all", &tr_st_all),
                            ],
                            &[("aria-label", &tr_pending_aria)],
                        );
                        action_btn(tb, "btn-cmd btn-cmd-sm", "refresh-pending-translations", None, Some("fa-refresh"), &refresh_btn);
                    });
                },
                |card| {
                    ops_table(
                        card,
                        "",
                        Some("table-pending-translations"),
                        &[(&col_key_grp, ""), (&col_tr, ""), (&col_en, ""), (&col_status, ""), (&col_action, "")],
                        "tbody-pending-translations",
                        Some(&tr_pending_loading),
                        Some("pagination-pending-translations"),
                    );
                },
            );
        });

        subtab_pane(tab, "tab8-glossary", false, Some("glossary:write|glossary:manage"), |pane| {
            admin_card(
                pane, "", None, "fa-book", &gl_title,
                Some(&gl_desc),
                |hdr| {
                    hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                        select_ctrl(
                            tb,
                            "glossary-domain-filter",
                            "input-md",
                            &[
                                ("", &gl_dom_all),
                                ("style", &gl_dom_style),
                                ("defence", &gl_dom_defence),
                                ("aerospace", &gl_dom_aero),
                                ("naval", &gl_dom_naval),
                                ("editorial", &gl_dom_edit),
                            ],
                            &[("aria-label", &gl_dom_aria)],
                        );
                        input_ctrl(tb, "text", "glossary-search", "input-md", "", &gl_search_ph, &[("aria-label", &gl_search_aria)]);
                        check_pill(tb, Some("glossary-active-only"), None, None, false, "", &gl_active_only);
                        action_btn(tb, "btn-cmd btn-cmd-primary btn-cmd-sm", "show-glossary-form", Some("glossary:write|glossary:manage"), Some("fa-plus"), &gl_new_btn);
                    });
                },
                |card| {
                    card.el("div", &[("id", "glossary-form-container"), ("class", "hidden form-container-elevated")], |gc| {
                        gc.el_with("form", |a| { a.attr("id", "form-glossary").bool_attr("novalidate", true); }, |f| {
                            f.void_el("input", &[("type", "hidden"), ("id", "gl-id"), ("value", "0")]);
                            f.el("div", &[("class", "form-row-3")], |r| {
                                form_group(r, Some("gl-term-tr"), &gl_term_tr_lbl, |g| input_ctrl(g, "text", "gl-term-tr", "", "", "", &[("required", "required")]));
                                form_group(r, Some("gl-term-en"), &gl_term_en_lbl, |g| input_ctrl(g, "text", "gl-term-en", "", "", "", &[]));
                                form_group(r, Some("gl-preferred-en"), &gl_pref_en_lbl, |g| input_ctrl(g, "text", "gl-preferred-en", "", "", "", &[("required", "required")]));
                            });
                            f.el("div", &[("class", "form-row-3")], |r| {
                                form_group(r, Some("gl-abbr-tr"), &gl_abbr_tr_lbl, |g| input_ctrl(g, "text", "gl-abbr-tr", "", "", "", &[]));
                                form_group(r, Some("gl-abbr-en"), &gl_abbr_en_lbl, |g| input_ctrl(g, "text", "gl-abbr-en", "", "", "", &[]));
                                form_group(r, Some("gl-domain"), &gl_domain_lbl, |g| {
                                    select_ctrl(g, "gl-domain", "", &[
                                        ("editorial", &gl_dom_edit),
                                        ("style", &gl_dom_style),
                                        ("defence", &gl_dom_defence),
                                        ("aerospace", &gl_dom_aero),
                                        ("naval", &gl_dom_naval),
                                    ], &[]);
                                });
                            });
                            f.el("div", &[("class", "form-row-3")], |r| {
                                form_group(r, Some("gl-rule-type"), &gl_rule_lbl, |g| {
                                    select_ctrl(g, "gl-rule-type", "", &[
                                        ("fixed", &gl_rule_fixed),
                                        ("preserve", &gl_rule_preserve),
                                        ("forbidden", &gl_rule_forbidden),
                                    ], &[]);
                                });
                                form_group(r, Some("gl-priority"), &gl_prio_lbl, |g| input_ctrl(g, "number", "gl-priority", "", "50", "50", &[]));
                                form_group(r, Some("gl-source-ref"), &gl_src_lbl, |g| input_ctrl(g, "text", "gl-source-ref", "", "", "", &[]));
                            });
                            f.el("div", &[("class", "form-row-2")], |r| {
                                form_group(r, Some("gl-forbidden-variants"), &gl_forbid_lbl, |g| input_ctrl(g, "text", "gl-forbidden-variants", "", "", "", &[]));
                                form_group(r, Some("gl-notes"), &gl_notes_lbl, |g| input_ctrl(g, "text", "gl-notes", "", "", "", &[]));
                            });
                            f.el("div", &[("class", "flex-between flex-wrap gap-2")], |fl| {
                                fl.el("div", &[("class", "flex gap-3 flex-wrap")], |chks| {
                                    check_pill(chks, Some("gl-case-sensitive"), None, None, false, "", &gl_case_lbl);
                                    check_pill(chks, Some("gl-is-active"), None, None, true, "", &gl_is_active_lbl);
                                });
                                fl.el("div", &[("class", "flex gap-2")], |btns| {
                                    btns.el(
                                        "button",
                                        &[("type", "submit"), ("id", "btn-glossary-save"), ("class", "btn-cmd btn-cmd-primary btn-cmd-sm"), ("data-required-perm", "glossary:write|glossary:manage")],
                                        |b| {
                                            icon(b, "fa-save");
                                            b.raw(" ");
                                            b.text(&gl_save_btn);
                                        },
                                    );
                                    action_btn(btns, "btn-cmd btn-cmd-sm", "hide-glossary-form", None, None, &cancel_btn);
                                });
                            });
                        });
                    });

                    ops_table(
                        card,
                        "",
                        Some("table-glossary"),
                        &[
                            (&gl_term_tr_lbl, ""),
                            (&gl_pref_en_lbl, ""),
                            (&gl_col_dom, ""),
                            (&gl_col_forbid, ""),
                            (&col_status, ""),
                            (&col_action, ""),
                        ],
                        "tbody-glossary",
                        Some(&gl_loading),
                        None,
                    );
                },
            );
        });

        subtab_pane(tab, "tab8-locales", false, Some("locales:manage"), |pane| {
            admin_card(
                pane, "", None, "fa-globe", &loc_title,
                Some(&loc_desc),
                |hdr| action_btn(hdr, "btn-cmd btn-cmd-primary btn-cmd-sm", "show-locale-form", Some("locales:manage"), Some("fa-plus"), &loc_new_btn),
                |card| {
                    card.el("div", &[("id", "locale-form-container"), ("class", "hidden form-container-elevated")], |lc| {
                        lc.el_with("form", |a| { a.attr("id", "form-locale").bool_attr("novalidate", true); }, |f| {
                            f.void_el("input", &[("type", "hidden"), ("id", "loc-edit-mode"), ("value", "create")]);
                            f.el("div", &[("class", "form-row-3")], |r| {
                                form_group(r, Some("loc-code"), &loc_code_lbl, |g| input_ctrl(g, "text", "loc-code", "", "", "de", &[("required", "required"), ("maxlength", "16")]));
                                form_group(r, Some("loc-name-native"), &loc_native_lbl, |g| input_ctrl(g, "text", "loc-name-native", "", "", "Deutsch", &[("required", "required")]));
                                form_group(r, Some("loc-name-en"), &loc_en_lbl, |g| input_ctrl(g, "text", "loc-name-en", "", "", "German", &[("required", "required")]));
                            });
                            f.el("div", &[("class", "form-row-3")], |r| {
                                form_group(r, Some("loc-hreflang"), &loc_hreflang_lbl, |g| input_ctrl(g, "text", "loc-hreflang", "", "", "de-DE", &[("required", "required")]));
                                form_group(r, Some("loc-og-locale"), &loc_og_lbl, |g| input_ctrl(g, "text", "loc-og-locale", "", "", "de_DE", &[("required", "required")]));
                                form_group(r, Some("loc-dir"), &loc_dir_lbl, |g| select_ctrl(g, "loc-dir", "", &[("ltr", "LTR"), ("rtl", "RTL")], &[]));
                            });
                            f.el("div", &[("class", "form-row-2")], |r| {
                                form_group(r, Some("loc-fallback-code"), &loc_fb_lbl, |g| input_ctrl(g, "text", "loc-fallback-code", "", "tr", "tr", &[]));
                                form_group(r, Some("loc-sort-order"), &loc_sort_lbl, |g| input_ctrl(g, "number", "loc-sort-order", "", "0", "0", &[]));
                            });
                            f.el("div", &[("class", "flex-between flex-wrap gap-2")], |fl| {
                                fl.el("div", &[("class", "flex gap-3 flex-wrap")], |chks| {
                                    check_pill(chks, Some("loc-is-enabled"), None, None, true, "", &loc_enabled_lbl);
                                    check_pill(chks, Some("loc-is-default"), None, None, false, "", &loc_default_lbl);
                                });
                                fl.el("div", &[("class", "flex gap-2")], |btns| {
                                    btns.el(
                                        "button",
                                        &[("type", "submit"), ("id", "btn-locale-save"), ("class", "btn-cmd btn-cmd-primary btn-cmd-sm"), ("data-required-perm", "locales:manage")],
                                        |b| {
                                            icon(b, "fa-save");
                                            b.raw(" ");
                                            b.text(&loc_save_btn);
                                        },
                                    );
                                    action_btn(btns, "btn-cmd btn-cmd-sm", "hide-locale-form", None, None, &cancel_btn);
                                });
                            });
                        });
                    });

                    ops_table(
                        card,
                        "",
                        Some("table-locales"),
                        &[
                            (&loc_col_code, ""),
                            (&loc_col_names, ""),
                            (&loc_col_href, ""),
                            (&loc_col_dir, ""),
                            (&col_status, ""),
                            (&col_action, ""),
                        ],
                        "tbody-locales",
                        Some(&loc_loading),
                        None,
                    );
                },
            );
        });
    });
}

fn render_tab_rbac(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let sub_users = tr_admin(tr, lang, "admin.system.subtab_users", "Kullanıcılar & Rol Atama", "Users & Role Assignment");
    let sub_roles = tr_admin(tr, lang, "admin.system.subtab_roles", "Roller & İzin Matrisi", "Roles & Permission Matrix");
    let sub_audit = tr_admin(tr, lang, "admin.system.subtab_audit", "Denetim İzi (Audit Trail)", "Audit Trail");

    let users_title = tr_admin(tr, lang, "admin.system.users_title", "Operatörler, Aboneler & Rol Atama", "Operators, Subscribers & Role Assignment");
    let users_desc = tr_admin(tr, lang, "admin.system.users_desc", "Kurumsal kullanıcıları arayın, yeni operatör tanımlayın ve en az ayrıcalık ilkesine göre RBAC rolü atayın.", "Search corporate users, define new operators and assign RBAC roles under least-privilege.");
    let users_search_ph = tr_admin(tr, lang, "admin.system.users_search_ph", "Ad, e-posta veya rol ara...", "Search name, email or role...");
    let users_search_aria = tr_admin(tr, lang, "admin.system.users_search_aria", "Kullanıcı ara", "Search users");
    let new_user_btn = tr_admin(tr, lang, "admin.system.new_user", "Yeni Kullanıcı", "New User");
    let lbl_email = tr_admin(tr, lang, "admin.system.u_email", "Kurumsal E-posta", "Corporate Email");
    let ph_email = tr_admin(tr, lang, "admin.system.u_email_ph", "ornek@msidergisi.com", "example@msidergisi.com");
    let lbl_full_name = tr_admin(tr, lang, "admin.system.u_full_name", "Ad Soyad", "Full Name");
    let lbl_password = tr_admin(tr, lang, "admin.system.u_password", "Parola (En az 8 karakter)", "Password (Min. 8 chars)");
    let lbl_role = tr_admin(tr, lang, "admin.system.u_role", "Sistem Rolü", "System Role");
    let lbl_org_cat = tr_admin(tr, lang, "admin.system.u_org_cat", "Kurum Kategorisi", "Organisation Category");
    let def_org_cat = tr_admin(tr, lang, "admin.tag.inst.sektor", "Sektör Kuruluşları", "Industry Organisations");
    let lbl_org_name = tr_admin(tr, lang, "admin.system.u_org_name", "Kurum Adı", "Organisation Name");
    let ph_org_name = tr_admin(tr, lang, "admin.system.u_org_name_ph", "Kurum veya birim adı", "Organisation or unit name");
    let lbl_job = tr_admin(tr, lang, "admin.system.u_job_title", "Ünvan / Görev", "Title / Role");
    let ph_job = tr_admin(tr, lang, "admin.system.u_job_title_ph", "Görev ünvanı", "Job title");
    let lbl_sub_tier = tr_admin(tr, lang, "admin.system.u_sub_tier", "Abonelik Paketi", "Subscription Package");
    let ph_sub_tier = tr_admin(tr, lang, "admin.system.u_sub_tier_ph", "kurumsal / dijital", "corporate / digital");
    let chk_gov = tr_admin(tr, lang, "admin.system.u_gov_verified", "Kamu Doğrulamalı (.gov.tr / .tsk.tr)", "Public Sector Verified (.gov.tr / .tsk.tr)");
    let save_user_btn = tr_admin(tr, lang, "admin.system.save_user", "Kullanıcıyı Kaydet", "Save User");
    let cancel_btn = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");

    let quick_assign_lbl = tr_admin(tr, lang, "admin.system.quick_role_assign", "HIZLI ROL ATAMA:", "QUICK ROLE ASSIGNMENT:");
    let ph_user_id = tr_admin(tr, lang, "admin.system.user_id_ph", "Kullanıcı ID", "User ID");
    let role_site_op = tr_admin(tr, lang, "admin.system.role_opt_site_op", "site_operator — Baş Operatör", "site_operator — Lead Operator");
    let role_super = tr_admin(tr, lang, "admin.system.role_opt_super", "super_admin — Süper Yönetici", "super_admin — Super Admin");
    let role_editor = tr_admin(tr, lang, "admin.system.role_opt_editor", "content_editor — İçerik Editörü", "content_editor — Content Editor");
    let role_pub = tr_admin(tr, lang, "admin.system.role_opt_pub", "content_publisher — Yayınlayıcı", "content_publisher — Publisher");
    let role_ads = tr_admin(tr, lang, "admin.system.role_opt_ads", "ads_manager — Reklam Yöneticisi", "ads_manager — Ads Manager");
    let role_fin = tr_admin(tr, lang, "admin.system.role_opt_fin", "finance_viewer — Finans Uzmanı", "finance_viewer — Finance Viewer");
    let role_sup = tr_admin(tr, lang, "admin.system.role_opt_sup", "support_agent — Destek Temsilcisi", "support_agent — Support Agent");
    let assign_role_btn = tr_admin(tr, lang, "admin.system.assign_role_btn", "Rol Ata", "Assign Role");
    let col_u_id = tr_admin(tr, lang, "admin.system.col_u_id_email", "ID / Kullanıcı & E-Posta", "ID / User & Email");
    let col_u_role = tr_admin(tr, lang, "admin.system.col_u_role_pkg", "Mevcut Rol & Paket", "Current Role & Package");
    let col_u_act = tr_admin(tr, lang, "admin.system.col_u_actions", "Yetki Atama & İşlemler", "Role Assignment & Actions");
    let users_loading = tr_admin(tr, lang, "admin.system.users_loading", "Kullanıcı listesi yükleniyor...", "Loading user directory...");

    let roles_title = tr_admin(tr, lang, "admin.system.roles_title", "Yetki Rolleri & İzin Matrisi", "Authority Roles & Permission Matrix");
    let roles_desc = tr_admin(tr, lang, "admin.system.roles_desc", "Sistem rollerini ve modül bazlı granüler erişim izinlerini yönetin.", "Manage system roles and granular module-level access permissions.");
    let new_role_btn = tr_admin(tr, lang, "admin.system.new_role_btn", "Yeni Rol Tanımla", "Define New Role");
    let lbl_role_code = tr_admin(tr, lang, "admin.system.role_code_lbl", "Rol Kodu", "Role Code");
    let ph_role_code = tr_admin(tr, lang, "admin.system.role_code_ph", "Örn: savunma_editoru", "e.g. defence_editor");
    let lbl_role_tr = tr_admin(tr, lang, "admin.system.role_name_tr_lbl", "Rol Adı (TR)", "Role Name (TR)");
    let ph_role_tr = tr_admin(tr, lang, "admin.system.role_name_tr_ph", "Savunma Editörü", "Defence Editor");
    let lbl_role_en = tr_admin(tr, lang, "admin.system.role_name_en_lbl", "Role Name (EN)", "Role Name (EN)");
    let lbl_mod_perms = tr_admin(tr, lang, "admin.system.module_perms_lbl", "Modül İzin Yetkileri", "Module Permissions");
    let save_role_btn = tr_admin(tr, lang, "admin.system.save_role_btn", "Rolü Kaydet", "Save Role");
    let col_r_code = tr_admin(tr, lang, "admin.system.col_role_code", "Rol Kodu", "Role Code");
    let col_r_tr = tr_admin(tr, lang, "admin.system.col_role_tr", "Rol Adı (TR)", "Role Name (TR)");
    let col_r_en = tr_admin(tr, lang, "admin.system.col_role_en", "Role Name (EN)", "Role Name (EN)");
    let col_r_perm = tr_admin(tr, lang, "admin.system.col_role_perm", "Tanımlı İzin", "Defined Permissions");
    let col_r_prot = tr_admin(tr, lang, "admin.system.col_role_prot", "Koruma", "Protection");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let roles_loading = tr_admin(tr, lang, "admin.system.roles_loading", "Roller yükleniyor...", "Loading roles...");

    let audit_title = tr_admin(tr, lang, "admin.system.audit_title", "Kurumsal Denetim İzi (Audit Trail)", "Corporate Audit Trail");
    let audit_desc = tr_admin(tr, lang, "admin.system.audit_desc", "Yönetim panelinde gerçekleştirilen tüm oluşturma, güncelleme, silme ve yetki işlemlerinin değiştirilemez kaydı.", "Immutable log of all create, update, delete and permission actions in the command center.");
    let aud_all = tr_admin(tr, lang, "admin.system.audit_all", "Tüm İşlemler", "All Actions");
    let aud_create = tr_admin(tr, lang, "admin.system.audit_create", "Oluşturma", "Create");
    let aud_update = tr_admin(tr, lang, "admin.system.audit_update", "Güncelleme", "Update");
    let aud_delete = tr_admin(tr, lang, "admin.system.audit_delete", "Silme", "Delete");
    let aud_login = tr_admin(tr, lang, "admin.system.audit_login", "Oturum Açma", "Login");
    let aud_flush = tr_admin(tr, lang, "admin.system.audit_flush", "Önbellek Temizleme", "Cache Flush");
    let aud_type_aria = tr_admin(tr, lang, "admin.system.audit_type_aria", "İşlem türü filtresi", "Action type filter");
    let aud_actor_ph = tr_admin(tr, lang, "admin.system.audit_actor_ph", "Operatör e-postası ara...", "Search operator email...");
    let aud_actor_aria = tr_admin(tr, lang, "admin.system.audit_actor_aria", "Operatör filtresi", "Operator filter");
    let col_dt = tr_admin(tr, lang, "admin.system.col_datetime", "Tarih & Saat", "Date & Time");
    let col_actor = tr_admin(tr, lang, "admin.system.col_operator", "Operatör", "Operator");
    let col_act = tr_admin(tr, lang, "admin.system.col_act", "Eylem", "Action");
    let col_entity = tr_admin(tr, lang, "admin.system.col_entity", "Hedef Varlık", "Target Entity");
    let col_summary = tr_admin(tr, lang, "admin.system.col_summary", "İşlem Özeti", "Summary");
    let col_ip = tr_admin(tr, lang, "admin.system.col_source_ip", "Kaynak IP", "Source IP");
    let audit_loading = tr_admin(tr, lang, "admin.system.audit_loading", "Denetim kayıtları yükleniyor...", "Loading audit logs...");

    tab_pane(buf, "tab-11", "tab-btn-11", false, |tab| {
        subtab_bar(tab, "tab11", &[
            ("tab11-users", &sub_users, Some("users:read|users:manage|roles:manage")),
            ("tab11-roles", &sub_roles, Some("users:read|roles:manage")),
            ("tab11-audit", &sub_audit, Some("audit:read|users:manage|settings:read")),
        ]);

        subtab_pane(tab, "tab11-users", true, Some("users:read|users:manage|roles:manage"), |pane| {
            admin_card(
                pane, "", None, "fa-users", &users_title,
                Some(&users_desc),
                |hdr| {
                    hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                        input_ctrl(tb, "text", "user-role-search", "input-lg", "", &users_search_ph, &[("aria-label", &users_search_aria)]);
                        action_btn(tb, "btn-cmd btn-cmd-primary btn-cmd-sm", "show-create-user", Some("users:manage"), Some("fa-user-plus"), &new_user_btn);
                    });
                },
                |card| {
                    card.el("div", &[("id", "user-crud-form-container"), ("class", "hidden form-container-elevated")], |uc| {
                        uc.void_el("input", &[("type", "hidden"), ("id", "u-edit-id"), ("value", "0")]);
                        uc.el("div", &[("class", "form-row-3")], |r| {
                            form_group(r, Some("u-email"), &lbl_email, |g| input_ctrl(g, "email", "u-email", "", "", &ph_email, &[]));
                            form_group(r, Some("u-full-name"), &lbl_full_name, |g| input_ctrl(g, "text", "u-full-name", "", "", &lbl_full_name, &[]));
                            form_group(r, Some("u-password"), &lbl_password, |g| input_ctrl(g, "password", "u-password", "", "", "••••••••", &[]));
                        });
                        uc.el("div", &[("class", "form-row-3")], |r| {
                            form_group(r, Some("u-role"), &lbl_role, |g| select_ctrl(g, "u-role", "", &[("registered", "registered")], &[]));
                            form_group(r, Some("u-org-cat"), &lbl_org_cat, |g| input_ctrl(g, "text", "u-org-cat", "", &def_org_cat, "", &[]));
                            form_group(r, Some("u-org-name"), &lbl_org_name, |g| input_ctrl(g, "text", "u-org-name", "", "", &ph_org_name, &[]));
                        });
                        uc.el("div", &[("class", "form-row-3")], |r| {
                            form_group(r, Some("u-job-title"), &lbl_job, |g| input_ctrl(g, "text", "u-job-title", "", "", &ph_job, &[]));
                            form_group(r, Some("u-sub-tier"), &lbl_sub_tier, |g| input_ctrl(g, "text", "u-sub-tier", "", "", &ph_sub_tier, &[]));
                            r.el("div", &[("class", "form-group form-group-end")], |g| check_pill(g, Some("u-gov-verified"), None, None, false, "", &chk_gov));
                        });
                        uc.el("div", &[("id", "user-custom-fields"), ("data-custom-field-values", "user"), ("class", "custom-fields-slot mb-3")], |_| {});
                        uc.el("div", &[("class", "flex gap-2")], |fl| {
                            action_btn(fl, "btn-cmd btn-cmd-primary btn-cmd-sm", "save-user-form", None, Some("fa-save"), &save_user_btn);
                            action_btn(fl, "btn-cmd btn-cmd-sm", "hide-user-form", None, None, &cancel_btn);
                        });
                    });

                    card.el("div", &[("class", "panel-toolbar"), ("data-required-perm", "roles:manage")], |bar| {
                        bar.el("span", &[("class", "text-xs text-muted font-semibold")], |s| { s.text(&quick_assign_lbl); });
                        bar.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |fl| {
                            input_ctrl(fl, "number", "manual-assign-user-id", "input-sm", "", &ph_user_id, &[("min", "1")]);
                            select_ctrl(fl, "manual-assign-role-code", "input-lg", &[
                                ("site_operator", &role_site_op),
                                ("super_admin", &role_super),
                                ("content_editor", &role_editor),
                                ("content_publisher", &role_pub),
                                ("ads_manager", &role_ads),
                                ("finance_viewer", &role_fin),
                                ("support_agent", &role_sup),
                            ], &[]);
                            action_btn(fl, "btn-cmd btn-cmd-primary btn-cmd-sm", "assign-manual-user-role", None, Some("fa-check"), &assign_role_btn);
                        });
                    });

                    ops_table(card, "", None, &[(&col_u_id, ""), (&col_u_role, ""), (&col_u_act, "")], "tbody-admin-users", Some(&users_loading), Some("pagination-admin-users"));
                },
            );
        });

        subtab_pane(tab, "tab11-roles", false, Some("users:read|roles:manage"), |pane| {
            admin_card(
                pane, "", None, "fa-shield", &roles_title,
                Some(&roles_desc),
                |hdr| action_btn(hdr, "btn-cmd btn-cmd-primary btn-cmd-sm", "show-role-form", Some("roles:manage"), Some("fa-plus"), &new_role_btn),
                |card| {
                    card.el("div", &[("id", "role-form-container"), ("class", "hidden form-container-elevated")], |rf| {
                        rf.void_el("input", &[("type", "hidden"), ("id", "role-edit-mode"), ("value", "create")]);
                        rf.el("div", &[("class", "form-row-3")], |r| {
                            form_group(r, Some("role-code"), &lbl_role_code, |g| input_ctrl(g, "text", "role-code", "", "", &ph_role_code, &[]));
                            form_group(r, Some("role-name-tr"), &lbl_role_tr, |g| input_ctrl(g, "text", "role-name-tr", "", "", &ph_role_tr, &[]));
                            form_group(r, Some("role-name-en"), &lbl_role_en, |g| input_ctrl(g, "text", "role-name-en", "", "", "Defence Editor", &[]));
                        });
                        rf.el("div", &[("class", "mb-3")], |pm| {
                            pm.el("div", &[("class", "form-label mb-2")], |l| { l.text(&lbl_mod_perms); });
                            pm.el("div", &[("id", "role-permissions-grid"), ("class", "perm-groups-wrap")], |grid| {
                                for (grp_key, fb_tr, fb_en, perms) in RBAC_PERMISSION_GROUPS {
                                    let grp_title = tr_admin(tr, lang, grp_key, fb_tr, fb_en);
                                    grid.el("div", &[("class", "perm-group-card")], |gc| {
                                        gc.el("div", &[("class", "perm-group-title")], |gt| { gt.text(&grp_title); });
                                        for p in *perms {
                                            check_pill(gc, None, None, Some(p), false, "", p);
                                        }
                                    });
                                }
                            });
                        });
                        rf.el("div", &[("class", "flex gap-2")], |fl| {
                            action_btn(fl, "btn-cmd btn-cmd-primary btn-cmd-sm", "save-role", None, Some("fa-save"), &save_role_btn);
                            action_btn(fl, "btn-cmd btn-cmd-sm", "hide-role-form", None, None, &cancel_btn);
                        });
                    });

                    ops_table(card, "", None, &[(&col_r_code, ""), (&col_r_tr, ""), (&col_r_en, ""), (&col_r_perm, ""), (&col_r_prot, ""), (&col_action, "")], "tbody-user-roles", Some(&roles_loading), Some("pagination-user-roles"));
                },
            );
        });

        subtab_pane(tab, "tab11-audit", false, Some("audit:read|users:manage|settings:read"), |pane| {
            admin_card(
                pane, "", None, "fa-file-text-o", &audit_title,
                Some(&audit_desc),
                |hdr| {
                    hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                        select_ctrl(tb, "audit-filter-action", "input-md", &[
                            ("", &aud_all), ("create", &aud_create), ("update", &aud_update),
                            ("delete", &aud_delete), ("login", &aud_login), ("flush", &aud_flush),
                        ], &[("aria-label", &aud_type_aria)]);
                        input_ctrl(tb, "text", "audit-filter-actor", "input-lg", "", &aud_actor_ph, &[("aria-label", &aud_actor_aria)]);
                    });
                },
                |card| {
                    ops_table(card, "", None, &[(&col_dt, ""), (&col_actor, ""), (&col_act, ""), (&col_entity, ""), (&col_summary, ""), (&col_ip, "")], "tbody-audit-logs", Some(&audit_loading), Some("pagination-audit-logs"));
                },
            );
        });
    });
}

fn render_tab_monitoring(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let kpi_cpu = tr_admin(tr, lang, "admin.monitoring.kpi_cpu", "İŞLEMCİ YÜKÜ (CPU)", "CPU LOAD");
    let kpi_cpu_sub = tr_admin(tr, lang, "admin.monitoring.kpi_cpu_sub", "Yük ortalaması ölçülüyor", "Measuring load average");
    let kpi_ram = tr_admin(tr, lang, "admin.monitoring.kpi_ram", "BELLEK KULLANIMI (RAM)", "MEMORY USAGE (RAM)");
    let kpi_ram_sub = tr_admin(tr, lang, "admin.monitoring.kpi_ram_sub", "Kullanılabilir / Toplam", "Available / Total");
    let kpi_disk = tr_admin(tr, lang, "admin.monitoring.kpi_disk", "DİSK DOLULUĞU", "DISK USAGE");
    let kpi_disk_sub = tr_admin(tr, lang, "admin.monitoring.kpi_disk_sub", "Kök dosya sistemi", "Root filesystem");
    let kpi_dom = tr_admin(tr, lang, "admin.monitoring.kpi_domains", "İZLENEN ALAN ADI", "MONITORED DOMAINS");
    let kpi_dom_sub = tr_admin(tr, lang, "admin.monitoring.kpi_domains_sub", "SSL & HTTP sağlık taraması", "SSL & HTTP health scan");
    let kpi_atk = tr_admin(tr, lang, "admin.monitoring.kpi_attacks", "ENGEL (SON 24S)", "BLOCKS (LAST 24H)");
    let kpi_atk_sub = tr_admin(tr, lang, "admin.monitoring.kpi_attacks_sub", "Fail2ban & WAF tespiti", "Fail2ban & WAF detection");
    let kpi_srv = tr_admin(tr, lang, "admin.monitoring.kpi_services", "ALTYAPI SERVİSLERİ", "INFRASTRUCTURE SERVICES");
    let kpi_srv_sub = tr_admin(tr, lang, "admin.monitoring.kpi_services_sub", "Aktif sistem portları", "Active system ports");

    let sub_live = tr_admin(tr, lang, "admin.monitoring.sub_live", "Canlı Kaynaklar & Servisler", "Live Resources & Services");
    let sub_dom = tr_admin(tr, lang, "admin.monitoring.sub_domains", "Alan Adı & SSL Sağlığı", "Domain & SSL Health");
    let sub_seo = tr_admin(tr, lang, "admin.monitoring.sub_seo", "Teknik SEO Denetimi", "Technical SEO Audit");

    let res_title = tr_admin(tr, lang, "admin.monitoring.res_title", "Canlı Sistem Kaynakları (CPU & Bellek)", "Live System Resources (CPU & Memory)");
    let res_desc = tr_admin(tr, lang, "admin.monitoring.res_desc", "15 saniyelik aralıklarla gerçek zamanlı sunucu telemetrisi.", "Real-time server telemetry sampled at 15-second intervals.");
    let refresh_now = tr_admin(tr, lang, "admin.monitoring.refresh_now", "Şimdi Yenile", "Refresh Now");
    let net_title = tr_admin(tr, lang, "admin.monitoring.net_title", "Ağ Veri Akışı (RX / TX)", "Network Traffic (RX / TX)");
    let net_desc = tr_admin(tr, lang, "admin.monitoring.net_desc", "Anlık ağ arayüzü bant genişliği.", "Live network interface bandwidth.");
    let net_loading = tr_admin(tr, lang, "admin.monitoring.net_loading", "Ağ telemetrisi ölçülüyor...", "Measuring network telemetry...");
    let srv_title = tr_admin(tr, lang, "admin.monitoring.srv_title", "Kritik Altyapı Servis Durumu", "Critical Infrastructure Service Status");
    let srv_desc = tr_admin(tr, lang, "admin.monitoring.srv_desc", "Web sunucusu, veritabanı, e-posta aktarım katmanı ve izleme servislerinin anlık erişilebilirlik yanıt süreleri.", "Real-time reachability and latency of web server, database, mail relay and monitoring services.");
    let srv_loading = tr_admin(tr, lang, "admin.monitoring.srv_loading", "Servis durumları kontrol ediliyor...", "Checking service statuses...");
    let traffic_title = tr_admin(tr, lang, "admin.monitoring.traffic_title", "Alan Adı Bazlı Trafik Dağılımı", "Traffic Distribution by Domain");
    let atk_title = tr_admin(tr, lang, "admin.monitoring.attacks_title", "Saldırı Tespit Zaman Çizelgesi", "Attack Detection Timeline");
    let atk_loading = tr_admin(tr, lang, "admin.monitoring.attacks_loading", "Güvenlik duvarı taranıyor...", "Scanning firewall logs...");

    let dom_title = tr_admin(tr, lang, "admin.monitoring.dom_title", "Alan Adı, HTTP & X.509 SSL Sertifika Sağlığı", "Domain, HTTP & X.509 SSL Certificate Health");
    let dom_desc = tr_admin(tr, lang, "admin.monitoring.dom_desc", "Sunucu üzerindeki sanal konakların yanıt süreleri ve sertifika geçerlilik takvimi.", "Response times and certificate validity schedule for server virtual hosts.");
    let dom_search_ph = tr_admin(tr, lang, "admin.monitoring.dom_search_ph", "Alan adı filtrele...", "Filter domain...");
    let dom_search_aria = tr_admin(tr, lang, "admin.monitoring.dom_search_aria", "Alan adı ara", "Search domain");
    let col_dom = tr_admin(tr, lang, "admin.monitoring.col_domain", "Alan Adı", "Domain");
    let col_http = tr_admin(tr, lang, "admin.monitoring.col_http", "HTTP Durumu", "HTTP Status");
    let col_lat = tr_admin(tr, lang, "admin.monitoring.col_latency", "Yanıt Süresi", "Latency");
    let col_ssl_exp = tr_admin(tr, lang, "admin.monitoring.col_ssl_exp", "SSL Bitiş", "SSL Expiry");
    let col_days_left = tr_admin(tr, lang, "admin.monitoring.col_days_left", "Kalan Gün", "Days Left");
    let col_last_chk = tr_admin(tr, lang, "admin.monitoring.col_last_check", "Son Kontrol", "Last Check");
    let dom_loading = tr_admin(tr, lang, "admin.monitoring.dom_loading", "Alan adı verileri yükleniyor...", "Loading domain data...");

    let seo_title = tr_admin(tr, lang, "admin.monitoring.seo_title", "Teknik SEO & Üstveri Uyumluluk Denetimi", "Technical SEO & Metadata Compliance Audit");
    let seo_desc = tr_admin(tr, lang, "admin.monitoring.seo_desc", "robots.txt, sitemap.xml, başlık, açıklama ve kanonik bağlantı doğrulaması.", "robots.txt, sitemap.xml, title, description and canonical link verification.");
    let seo_search_ph = tr_admin(tr, lang, "admin.monitoring.seo_search_ph", "Alan adı veya sorun ara...", "Search domain or issue...");
    let seo_search_aria = tr_admin(tr, lang, "admin.monitoring.seo_search_aria", "SEO tablosunda ara", "Search SEO table");
    let col_title = tr_admin(tr, lang, "admin.monitoring.col_title", "Başlık", "Title");
    let col_desc = tr_admin(tr, lang, "admin.monitoring.col_desc", "Açıklama", "Description");
    let col_findings = tr_admin(tr, lang, "admin.monitoring.col_findings", "Tespitler", "Findings");
    let seo_loading = tr_admin(tr, lang, "admin.monitoring.seo_loading", "SEO denetim sonuçları yükleniyor...", "Loading SEO audit results...");

    tab_pane(buf, "tab-17", "tab-btn-17", false, |tab| {
        tab.el(
            "div",
            &[
                ("id", "monitoring-status-banner"),
                ("class", "status-banner conflict-danger hidden"),
                ("role", "status"),
            ],
            |_| {},
        );
        tab.el("div", &[("class", "kpi-grid kpi-grid-6 mb-4")], |kg| {
            kpi_card_toned(kg, "tone-info", Some("fa-tachometer"), None, &kpi_cpu, "mon-cpu", "—", "", Some("mon-cpu-sub"), Some(&kpi_cpu_sub));
            kpi_card_toned(kg, "tone-accent", Some("fa-cubes"), None, &kpi_ram, "mon-ram", "—", "", Some("mon-ram-sub"), Some(&kpi_ram_sub));
            kpi_card_toned(kg, "tone-warning", Some("fa-hdd-o"), None, &kpi_disk, "mon-disk", "—", "", Some("mon-disk-sub"), Some(&kpi_disk_sub));
            kpi_card_toned(kg, "tone-success", Some("fa-globe"), None, &kpi_dom, "mon-domains", "—", "", Some("mon-domains-sub"), Some(&kpi_dom_sub));
            kpi_card_toned(kg, "tone-danger", Some("fa-shield"), None, &kpi_atk, "mon-attacks", "0", "text-danger", Some("mon-attacks-sub"), Some(&kpi_atk_sub));
            kpi_card_toned(kg, "tone-info", Some("fa-heartbeat"), None, &kpi_srv, "mon-services", "—", "", Some("mon-services-sub"), Some(&kpi_srv_sub));
        });

        subtab_bar(tab, "tab17", &[
            ("tab17-live", &sub_live, None),
            ("tab17-domains", &sub_dom, None),
            ("tab17-seo", &sub_seo, None),
        ]);

        subtab_pane(tab, "tab17-live", true, None, |pane| {
            pane.el("div", &[("class", "workspace-grid media-seo-grid")], |grid| {
                admin_card(
                    grid, "", None, "fa-area-chart", &res_title,
                    Some(&res_desc),
                    |hdr| action_btn(hdr, "btn-cmd btn-cmd-sm", "refresh-monitoring", None, Some("fa-refresh"), &refresh_now),
                    |card| {
                        card.el("div", &[("class", "panel-body chart-container-sm")], |bd| {
                            bd.el("canvas", &[("id", "chart-system-resources")], |_| {});
                        });
                    },
                );

                admin_card(
                    grid, "", None, "fa-exchange", &net_title,
                    Some(&net_desc),
                    |_| {},
                    |card| {
                        card.el("div", &[("id", "monitoring-network-panel"), ("class", "panel-body")], |bd| {
                            bd.el("div", &[("class", "text-muted text-center p-4")], |t| { t.text(&net_loading); });
                        });
                    },
                );
            });

            admin_card(
                pane, "", None, "fa-server", &srv_title,
                Some(&srv_desc),
                |_| {},
                |card| {
                    card.el("div", &[("id", "monitoring-services-panel"), ("class", "panel-body grid-services")], |bd| {
                        bd.el("div", &[("class", "text-muted text-sm")], |t| { t.text(&srv_loading); });
                    });
                },
            );

            pane.el("div", &[("class", "workspace-grid")], |grid| {
                admin_card(grid, "", None, "fa-pie-chart", &traffic_title, None, |_| {}, |card| {
                    card.el("div", &[("class", "panel-body chart-container-md")], |bd| {
                        bd.el("canvas", &[("id", "chart-traffic-dist")], |_| {});
                    });
                });

                admin_card(grid, "", None, "fa-shield", &atk_title, None, |_| {}, |card| {
                    card.el("div", &[("id", "monitoring-attacks-panel"), ("class", "scrollable-panel")], |bd| {
                        bd.el("div", &[("class", "text-muted text-center p-4")], |t| { t.text(&atk_loading); });
                    });
                });
            });
        });

        subtab_pane(tab, "tab17-domains", false, None, |pane| {
            admin_card(
                pane, "", None, "fa-globe", &dom_title,
                Some(&dom_desc),
                |hdr| input_ctrl(hdr, "text", "mon-domain-search", "input-lg", "", &dom_search_ph, &[("aria-label", &dom_search_aria)]),
                |card| {
                    ops_table(
                        card, "", Some("table-monitoring-domains"),
                        &[(&col_dom, ""), (&col_http, ""), (&col_lat, ""), (&col_ssl_exp, ""), (&col_days_left, ""), ("DNS", "text-center"), (&col_last_chk, "")],
                        "tbody-monitoring-domains", Some(&dom_loading), Some("pagination-mon-domains"),
                    );
                },
            );
        });

        subtab_pane(tab, "tab17-seo", false, None, |pane| {
            admin_card(
                pane, "", None, "fa-search-plus", &seo_title,
                Some(&seo_desc),
                |hdr| input_ctrl(hdr, "text", "mon-seo-search", "input-lg", "", &seo_search_ph, &[("aria-label", &seo_search_aria)]),
                |card| {
                    ops_table(
                        card, "", Some("table-monitoring-seo"),
                        &[(&col_dom, ""), ("robots.txt", "text-center"), ("sitemap.xml", "text-center"), (&col_title, "text-center"), (&col_desc, "text-center"), ("Canonical", "text-center"), (&col_findings, "")],
                        "tbody-monitoring-seo", Some(&seo_loading), Some("pagination-mon-seo"),
                    );
                },
            );
        });
    });
}

fn render_tab_automation(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let od_eyebrow = tr_admin(tr, lang, "admin.automation.opendata_eyebrow", "AÇIK VERİ MİMARİSİ · SIFIR BAĞIMLILIK", "OPEN DATA ARCHITECTURE · ZERO LOCK-IN");
    let od_desc = tr_admin(tr, lang, "admin.automation.opendata_desc", "Tüm içerik envanteri, dergi sayıları, taksonomi ve abonelik kayıtları şifresiz taşınabilir JSON arşiv formatında tek tıkla dışa aktarılabilir.", "All content inventory, magazine issues, taxonomy and subscription records can be exported in unencrypted portable JSON archive format in one click.");
    let od_btn = tr_admin(tr, lang, "admin.automation.opendata_btn", "Tam Açık Veri Yedeği İndir (JSON)", "Download Full Open Data Export (JSON)");

    let sub_redirects = tr_admin(tr, lang, "admin.automation.sub_redirects", "301 Kalıcı SEO Yönlendirmeleri", "301 Permanent SEO Redirects");
    let sub_reminders = tr_admin(tr, lang, "admin.automation.sub_reminders", "5 Kademeli Yenileme Otomasyonu", "5-Stage Renewal Automation");

    let red_title = tr_admin(tr, lang, "admin.automation.red_title", "301 Kalıcı Bağlantı Yönlendirme Yöneticisi", "301 Permanent Link Redirect Manager");
    let red_desc = tr_admin(tr, lang, "admin.automation.red_desc", "Eski haber veya kategori adreslerini SEO otorite kaybı olmadan yeni kalıcı bağlantılara yönlendirin.", "Redirect legacy article or category URLs to new permalinks without losing SEO authority.");
    let red_search_ph = tr_admin(tr, lang, "admin.automation.red_search_ph", "Eski veya yeni adres ara...", "Search old or new path...");
    let red_search_aria = tr_admin(tr, lang, "admin.automation.red_search_aria", "Yönlendirme ara", "Search redirects");
    let lbl_old_path = tr_admin(tr, lang, "admin.automation.old_path", "Eski Adres (Kaynak)", "Old Path (Source)");
    let ph_old_path = tr_admin(tr, lang, "admin.automation.old_path_ph", "/eski-haber-adresi.html", "/legacy-article-url.html");
    let lbl_new_path = tr_admin(tr, lang, "admin.automation.new_path", "Yeni Adres (Hedef)", "New Path (Target)");
    let ph_new_path = tr_admin(tr, lang, "admin.automation.new_path_ph", "/icerik/yeni-kalici-baglanti", "/en/article/new-permalink");
    let add_red_btn = tr_admin(tr, lang, "admin.automation.add_redirect", "301 Kuralı Ekle", "Add 301 Rule");
    let col_old = tr_admin(tr, lang, "admin.automation.col_old_path", "Eski Bağlantı Yolu", "Legacy Source Path");
    let col_new = tr_admin(tr, lang, "admin.automation.col_new_path", "Yeni Hedef Bağlantı", "New Target Path");
    let col_code = tr_admin(tr, lang, "admin.automation.col_status_code", "Durum Kodu", "Status Code");
    let col_hits = tr_admin(tr, lang, "admin.automation.col_hits", "Yönlendirme Sayısı", "Hit Count");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let red_loading = tr_admin(tr, lang, "admin.automation.red_loading", "301 yönlendirmeleri yükleniyor...", "Loading 301 redirects...");

    let rem_title = tr_admin(tr, lang, "admin.automation.rem_title", "5 Kademeli Abonelik Yenileme Otomasyonu", "5-Stage Subscription Renewal Automation");
    let rem_desc = tr_admin(tr, lang, "admin.automation.rem_desc", "Abonelik bitişine 60, 30, 14, 7 ve 1 gün kala otomatik hatırlatma; yenileyen abonelerde anında durdurma.", "Automated reminders at 60, 30, 14, 7 and 1 days before expiry; auto-halted upon renewal.");
    let rem_halt_btn = tr_admin(tr, lang, "admin.automation.halt_renewed", "Yenileyenlerde Hatırlatmayı Durdur", "Halt Reminders for Renewed");
    let col_sub = tr_admin(tr, lang, "admin.automation.col_subscriber", "Abone & Kurum", "Subscriber & Organisation");
    let col_stage = tr_admin(tr, lang, "admin.automation.col_stage", "Hatırlatma Kademesi", "Reminder Stage");
    let col_rem_time = tr_admin(tr, lang, "admin.automation.col_remaining", "Kalan Süre", "Time Remaining");
    let col_auto_st = tr_admin(tr, lang, "admin.automation.col_auto_status", "Otomasyon Durumu", "Automation Status");
    let col_sent_at = tr_admin(tr, lang, "admin.automation.col_sent_at", "Gönderim Zamanı", "Sent At");
    let rem_loading = tr_admin(tr, lang, "admin.automation.rem_loading", "Yenileme hatırlatmaları kontrol ediliyor...", "Checking renewal reminders...");

    tab_pane(buf, "tab-7", "tab-btn-7", false, |tab| {
        tab.el("div", &[("class", "open-data-hero mb-4"), ("data-glow-card", ""), ("data-required-perm", "analytics:export|settings:write")], |hero| {
            hero.el("div", &[], |left| {
                left.el("div", &[("class", "open-data-eyebrow")], |e| {
                    icon(e, "fa-unlock-alt");
                    e.raw(" ");
                    e.text(&od_eyebrow);
                });
                left.el("div", &[("class", "open-data-desc")], |d| {
                    d.text(&od_desc);
                });
            });
            hero.el(
                "a",
                &[("href", "/api/v1/admin/export/open-data"), ("target", "_blank"), ("rel", "noopener"), ("class", "btn-cmd btn-cmd-success"), ("data-required-perm", "analytics:export|settings:write")],
                |a| {
                    icon(a, "fa-download");
                    a.raw(" ");
                    a.text(&od_btn);
                },
            );
        });

        subtab_bar(tab, "tab7", &[
            ("tab7-redirects", &sub_redirects, Some("redirects:write|settings:read|contents:publish")),
            ("tab7-reminders", &sub_reminders, Some("subscriptions:read")),
        ]);

        subtab_pane(tab, "tab7-redirects", true, Some("redirects:write|settings:read|contents:publish"), |pane| {
            admin_card(
                pane, "", None, "fa-exchange", &red_title,
                Some(&red_desc),
                |hdr| input_ctrl(hdr, "text", "redirect-search", "input-lg", "", &red_search_ph, &[("aria-label", &red_search_aria)]),
                |card| {
                    card.el_with("form", |a| { a.attr("id", "form-seo-redirect").attr("class", "panel-toolbar-form").attr("data-required-perm", "settings:write|contents:publish|redirects:write|redirects:manage").bool_attr("novalidate", true); }, |f| {
                        f.el("div", &[("class", "form-row-3 items-end")], |r| {
                            form_group(r, Some("seo-old-path"), &lbl_old_path, |g| input_ctrl(g, "text", "seo-old-path", "", "", &ph_old_path, &[("required", "required")]));
                            form_group(r, Some("seo-new-path"), &lbl_new_path, |g| input_ctrl(g, "text", "seo-new-path", "", "", &ph_new_path, &[("required", "required")]));
                            r.el("div", &[("class", "form-group")], |g| {
                                g.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-primary w-full flex-center")], |b| {
                                    icon(b, "fa-plus");
                                    b.raw(" ");
                                    b.text(&add_red_btn);
                                });
                            });
                        });
                    });

                    ops_table(card, "", None, &[(&col_old, ""), (&col_new, ""), (&col_code, ""), (&col_hits, ""), (&col_action, "")], "tbody-redirects", Some(&red_loading), Some("pagination-redirects"));
                },
            );
        });

        subtab_pane(tab, "tab7-reminders", false, Some("subscriptions:read"), |pane| {
            admin_card(
                pane, "", None, "fa-bell-o", &rem_title,
                Some(&rem_desc),
                |hdr| action_btn(hdr, "btn-cmd btn-cmd-sm btn-cmd-warning", "halt-renewed-reminders", Some("subscriptions:write|subscriptions:manage"), Some("fa-ban"), &rem_halt_btn),
                |card| {
                    ops_table(card, "", None, &[(&col_sub, ""), (&col_stage, ""), (&col_rem_time, ""), (&col_auto_st, ""), (&col_sent_at, "")], "tbody-reminders", Some(&rem_loading), Some("pagination-reminders"));
                },
            );
        });
    });
}

fn render_tab_fields_cache(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let l1_title = tr_admin(tr, lang, "admin.cache.l1_title", "Bellek İçi Çeviri & Tasarım Önbelleği", "In-Memory Translation & Design Cache");
    let l1_desc = tr_admin(tr, lang, "admin.cache.l1_desc", "RwLock atomik okuma katmanı", "RwLock atomic read layer");
    let l2_title = tr_admin(tr, lang, "admin.cache.l2_title", "İçerik & Taksonomi Sorgu Önbelleği", "Content & Taxonomy Query Cache");
    let l2_desc = tr_admin(tr, lang, "admin.cache.l2_desc", "Yüksek hızlı SSR sayfa derleme", "High-speed SSR page assembly");
    let l3_title = tr_admin(tr, lang, "admin.cache.l3_title", "301 SEO Yönlendirme Tablosu", "301 SEO Redirect Table");
    let l3_desc = tr_admin(tr, lang, "admin.cache.l3_desc", "Sıfır gecikmeli kalıcı adres çözümleme", "Zero-latency permalink resolution");
    let badge_active = tr_admin(tr, lang, "admin.status.active", "Aktif", "Active");

    let sub_fields = tr_admin(tr, lang, "admin.system.subtab_fields", "Dinamik Özel Alanlar", "Dynamic Custom Fields");
    let sub_cache = tr_admin(tr, lang, "admin.system.subtab_cache", "Önbellek İşlem Geçmişi", "Cache Operation History");
    let sub_backup = tr_admin(tr, lang, "admin.system.subtab_backup", "Veritabanı Yedeği", "Database Backup");

    let cf_title = tr_admin(tr, lang, "admin.system.cf_title", "Dinamik Özel Alan Şemaları", "Dynamic Custom Field Schemas");
    let cf_desc = tr_admin(tr, lang, "admin.system.cf_desc", "İçerik, reklam veya kullanıcı kayıtlarına kod değişikliği gerektirmeden özel veri alanları ekleyin.", "Add custom data fields to content, ads or user records without code changes.");
    let cf_new = tr_admin(tr, lang, "admin.system.cf_new", "Yeni Özel Alan", "New Custom Field");
    let cf_key = tr_admin(tr, lang, "admin.system.cf_key", "Alan Anahtarı", "Field Key");
    let cf_key_ph = tr_admin(tr, lang, "admin.system.cf_key_ph", "Örn: platform_menzili", "e.g. platform_range");
    let cf_lbl_tr = tr_admin(tr, lang, "admin.system.cf_lbl_tr", "Alan Etiketi (TR)", "Field Label (TR)");
    let cf_lbl_tr_ph = tr_admin(tr, lang, "admin.system.cf_lbl_tr_ph", "Platform Menzili", "Platform Range");
    let cf_lbl_en = tr_admin(tr, lang, "admin.system.cf_lbl_en", "Field Label (EN)", "Field Label (EN)");
    let cf_type = tr_admin(tr, lang, "admin.system.cf_type", "Veri Tipi", "Data Type");
    let dt_text = tr_admin(tr, lang, "admin.system.dt_text", "Kısa Metin", "Short Text");
    let dt_textarea = tr_admin(tr, lang, "admin.system.dt_textarea", "Uzun Metin", "Long Text");
    let dt_select = tr_admin(tr, lang, "admin.system.dt_select", "Açılır Seçim Listesi", "Dropdown List");
    let dt_checkbox = tr_admin(tr, lang, "admin.system.dt_checkbox", "Onay Kutusu", "Checkbox");
    let dt_number = tr_admin(tr, lang, "admin.system.dt_number", "Sayısal Değer", "Numeric Value");
    let dt_date = tr_admin(tr, lang, "admin.system.dt_date", "Tarih", "Date");
    let dt_url = tr_admin(tr, lang, "admin.system.dt_url", "Bağlantı (URL)", "URL");
    let cf_applies = tr_admin(tr, lang, "admin.system.cf_applies", "Hedef Varlık", "Target Entity");
    let ent_content = tr_admin(tr, lang, "admin.system.ent_content", "İçerik", "Content");
    let ent_ad = tr_admin(tr, lang, "admin.system.ent_ad", "Reklam Kampanyası", "Ad Campaign");
    let ent_user = tr_admin(tr, lang, "admin.system.ent_user", "Kullanıcı", "User");
    let cf_sort = tr_admin(tr, lang, "admin.system.cf_sort", "Görüntüleme Sırası", "Display Order");
    let cf_opts = tr_admin(tr, lang, "admin.system.cf_options", "Liste Seçenekleri (Sadece açılır liste için JSON dizisi)", "List Options (JSON array for dropdown only)");
    let cf_opts_ph = tr_admin(tr, lang, "admin.system.cf_options_ph", r#"["Hava", "Kara", "Deniz", "Uzay"]"#, r#"["Air", "Land", "Naval", "Space"]"#);
    let cf_req = tr_admin(tr, lang, "admin.system.cf_required", "Zorunlu Alan", "Required Field");
    let cf_save = tr_admin(tr, lang, "admin.system.cf_save", "Alanı Kaydet", "Save Field");
    let cancel_btn = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");

    let col_key = tr_admin(tr, lang, "admin.system.col_key", "Anahtar", "Key");
    let col_tr = tr_admin(tr, lang, "admin.system.col_label_tr", "Etiket (TR)", "Label (TR)");
    let col_en = tr_admin(tr, lang, "admin.system.col_label_en", "Etiket (EN)", "Label (EN)");
    let col_target = tr_admin(tr, lang, "admin.system.col_target", "Hedef", "Target");
    let col_req = tr_admin(tr, lang, "admin.system.col_required", "Zorunlu", "Required");
    let col_sort = tr_admin(tr, lang, "admin.system.col_sort", "Sıra", "Order");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let cf_loading = tr_admin(tr, lang, "admin.system.cf_loading", "Özel alan tanımları yükleniyor...", "Loading custom field definitions...");

    let cache_title = tr_admin(tr, lang, "admin.system.cache_title", "Önbellek Temizleme & Senkronizasyon Geçmişi", "Cache Flush & Synchronisation History");
    let cache_desc = tr_admin(tr, lang, "admin.system.cache_desc", "L1, L2 ve L3 önbellek katmanlarında gerçekleştirilen temizleme operasyonları.", "Flush operations executed across L1, L2 and L3 cache layers.");
    let cache_flush_btn = tr_admin(tr, lang, "admin.system.cache_flush_btn", "Tüm Önbellek Katmanlarını Temizle", "Flush All Cache Layers");
    let col_ts = tr_admin(tr, lang, "admin.system.col_timestamp", "Zaman Damgası", "Timestamp");
    let col_layer = tr_admin(tr, lang, "admin.system.col_cache_layer", "Önbellek Katmanı", "Cache Layer");
    let col_op = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Operation");
    let col_actor = tr_admin(tr, lang, "admin.system.col_operator", "Operatör", "Operator");
    let col_detail = tr_admin(tr, lang, "admin.system.col_detail", "Ayrıntı", "Detail");
    let cache_loading = tr_admin(tr, lang, "admin.system.cache_loading", "Önbellek işlem geçmişi yükleniyor...", "Loading cache operation history...");

    let bk_title = tr_admin(tr, lang, "admin.system.backup_title", "Anlık Veritabanı Yedekleme", "On-Demand Database Backup");
    let bk_desc = tr_admin(tr, lang, "admin.system.backup_desc", "SQLite veritabanının ve doğrulama üstverisinin anlık kopyasını sunucu yedek dizinine alın.", "Create an immediate snapshot of the SQLite database and metadata into the server backups directory.");
    let bk_trigger_btn = tr_admin(tr, lang, "admin.system.backup_trigger_btn", "Şimdi Yedek Al", "Create Backup Now");
    let bk_ready_msg = tr_admin(tr, lang, "admin.system.backup_ready_msg", "Henüz bu oturumda yedek alınmadı. Yedek oluşturmak için yukarıdaki düğmeyi kullanın.", "No backup triggered in this session yet. Use the button above to create a snapshot.");

    tab_pane(buf, "tab-13", "tab-btn-13", false, |tab| {
        tab.el("div", &[("class", "cache-layers-strip mb-4")], |strip| {
            for (layer, title, desc, tone) in [
                ("L1", l1_title.as_str(), l1_desc.as_str(), "success"),
                ("L2", l2_title.as_str(), l2_desc.as_str(), "info"),
                ("L3", l3_title.as_str(), l3_desc.as_str(), "warning"),
            ] {
                strip.el("div", &[("class", &format!("cache-layer-card tone-{}", tone)), ("data-glow-card", "")], |c| {
                    c.el("div", &[("class", "flex-between mb-1")], |top| {
                        top.el("span", &[("class", "cache-layer-tag")], |t| { t.text(layer); });
                        top.el("span", &[("class", &format!("badge-pill {}", tone))], |b| { b.text(&badge_active); });
                    });
                    c.el("div", &[("class", "font-semibold text-sm")], |t| { t.text(title); });
                    c.el("div", &[("class", "text-xs text-muted mt-1")], |d| { d.text(desc); });
                });
            }
        });

        subtab_bar(tab, "tab13", &[
            ("tab13-fields", &sub_fields, Some("settings:read")),
            ("tab13-cache", &sub_cache, Some("settings:read")),
            ("tab13-backup", &sub_backup, Some("backup:manage")),
        ]);

        subtab_pane(tab, "tab13-fields", true, Some("settings:read"), |pane| {
            admin_card(
                pane, "", None, "fa-puzzle-piece", &cf_title,
                Some(&cf_desc),
                |hdr| action_btn(hdr, "btn-cmd btn-cmd-primary btn-cmd-sm", "show-custom-field-form", Some("settings:write"), Some("fa-plus"), &cf_new),
                |card| {
                    card.el("div", &[("id", "custom-field-form-container"), ("class", "hidden form-container-elevated")], |cf| {
                        cf.el("div", &[("class", "form-row-3")], |r| {
                            form_group(r, Some("cf-key"), &cf_key, |g| input_ctrl(g, "text", "cf-key", "", "", &cf_key_ph, &[]));
                            form_group(r, Some("cf-label-tr"), &cf_lbl_tr, |g| input_ctrl(g, "text", "cf-label-tr", "", "", &cf_lbl_tr_ph, &[]));
                            form_group(r, Some("cf-label-en"), &cf_lbl_en, |g| input_ctrl(g, "text", "cf-label-en", "", "", "Platform Range", &[]));
                        });
                        cf.el("div", &[("class", "form-row-3")], |r| {
                            form_group(r, Some("cf-type"), &cf_type, |g| {
                                select_ctrl(g, "cf-type", "", &[
                                    ("text", &dt_text), ("textarea", &dt_textarea), ("select", &dt_select),
                                    ("checkbox", &dt_checkbox), ("number", &dt_number), ("date", &dt_date), ("url", &dt_url),
                                ], &[]);
                            });
                            form_group(r, Some("cf-applies-to"), &cf_applies, |g| {
                                select_ctrl(g, "cf-applies-to", "", &[("content", &ent_content), ("ad", &ent_ad), ("user", &ent_user)], &[]);
                            });
                            form_group(r, Some("cf-sort-order"), &cf_sort, |g| input_ctrl(g, "number", "cf-sort-order", "", "0", "", &[]));
                        });
                        form_group(cf, Some("cf-options"), &cf_opts, |g| {
                            input_ctrl(g, "text", "cf-options", "", "", &cf_opts_ph, &[]);
                        });
                        cf.el("div", &[("class", "flex-between flex-wrap gap-2")], |fl| {
                            check_pill(fl, Some("cf-required"), None, None, false, "", &cf_req);
                            fl.el("div", &[("class", "flex gap-2")], |btns| {
                                action_btn(btns, "btn-cmd btn-cmd-primary btn-cmd-sm", "save-custom-field", None, Some("fa-save"), &cf_save);
                                action_btn(btns, "btn-cmd btn-cmd-sm", "hide-custom-field-form", None, None, &cancel_btn);
                            });
                        });
                    });

                    ops_table(card, "", None, &[(&col_key, ""), (&col_tr, ""), (&col_en, ""), (&cf_type, ""), (&col_target, ""), (&col_req, ""), (&col_sort, ""), (&col_action, "")], "tbody-custom-fields", Some(&cf_loading), Some("pagination-custom-fields"));
                },
            );
        });

        subtab_pane(tab, "tab13-cache", false, Some("settings:read"), |pane| {
            admin_card(
                pane, "", None, "fa-history", &cache_title,
                Some(&cache_desc),
                |hdr| action_btn(hdr, "btn-cmd btn-cmd-danger btn-cmd-sm", "flush-cache", Some("settings:write"), Some("fa-bolt"), &cache_flush_btn),
                |card| {
                    ops_table(card, "", None, &[(&col_ts, ""), (&col_layer, ""), (&col_op, ""), (&col_actor, ""), (&col_detail, "")], "tbody-cache-operations", Some(&cache_loading), Some("pagination-cache-operations"));
                },
            );
        });

        subtab_pane(tab, "tab13-backup", false, Some("backup:manage"), |pane| {
            admin_card(
                pane,
                "",
                None,
                "fa-database",
                &bk_title,
                Some(&bk_desc),
                |hdr| {
                    hdr.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("id", "btn-trigger-backup"),
                            ("class", "btn-cmd btn-cmd-primary btn-cmd-sm"),
                            ("data-admin-action", "trigger-backup"),
                            ("data-required-perm", "backup:manage"),
                        ],
                        |b| {
                            icon(b, "fa-download");
                            b.raw(" ");
                            b.text(&bk_trigger_btn);
                        },
                    );
                },
                |card| {
                    card.el("div", &[("id", "backup-status-box"), ("class", "panel-body")], |bd| {
                        bd.el("div", &[("id", "backup-last-result"), ("class", "text-sm text-muted")], |r| {
                            r.text(&bk_ready_msg);
                        });
                        bd.el("div", &[("class", "text-xs text-mono mt-2")], |m| {
                            m.el("span", &[("id", "backup-last-filename")], |_| {});
                            m.raw(" ");
                            m.el("span", &[("id", "backup-last-size")], |_| {});
                        });
                    });
                },
            );
        });
    });
}
