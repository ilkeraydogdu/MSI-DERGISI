//! Editorial Views: `#tab-1` Content & Drawer, `#tab-18` E-Dergi & Drawer,
//! `#tab-10` Media Library & Live SEO Preview, `#tab-5` Sub-tabbed Taxonomy & SSB.

use super::ui::{
    action_btn, admin_card, admin_drawer, check_matrix, check_matrix_with_attrs, check_pill,
    details_box, form_group, icon, input_ctrl, lang_tabs, ops_table, select_ctrl, subtab_bar,
    subtab_pane, tab_pane, textarea_ctrl, upload_field,
};
use super::{institution_tags, placement_tags, ssb_tags, tr_admin};
use crate::engine::design::html_dsl::HtmlBuf;
use crate::engine::translation_engine::TranslationCache;

/// Content type keys for i18n lookup.
const CONTENT_TYPE_KEYS: &[(&str, &str, &str)] = &[
    ("haberler", "Haberler", "News"),
    ("ozel-haberler", "Özel Haberler", "Exclusive Reports"),
    ("soylesiler", "Söyleşiler", "Interviews"),
    ("mesajlar", "Mesajlar", "Messages"),
    ("tanitici-makaleler", "Tanıtıcı Makaleler", "Special Coverages"),
    ("makaleler-analizler", "Makaleler & Analizler", "Articles & Analysis"),
    ("kapak-dosyasi", "Kapak Dosyası", "Cover Dossier"),
    ("basyazilar", "Başyazılar", "Editorials"),
    ("advertorials", "Sponsorlu Dosya", "Sponsored Dossier"),
    ("video", "Video & Multimedya", "Video & Multimedia"),
];

fn content_types_resolved(tr: &TranslationCache, lang: &str) -> Vec<(String, String)> {
    CONTENT_TYPE_KEYS
        .iter()
        .map(|(code, fb_tr, fb_en)| {
            let key = format!("admin.editorial.type_{}", code.replace('-', "_"));
            let label = tr_admin(tr, lang, &key, fb_tr, fb_en);
            (code.to_string(), label)
        })
        .collect()
}

pub fn render(buf: &mut HtmlBuf, seg: &str, tr: &TranslationCache, lang: &str) {
    render_tab_contents(buf, seg, tr, lang);
    render_tab_issues(buf, tr, lang);
    render_tab_media_seo(buf, tr, lang);
    render_tab_taxonomy(buf, tr, lang);
}

fn render_tab_contents(buf: &mut HtmlBuf, seg: &str, tr: &TranslationCache, lang: &str) {
    let ct_resolved = content_types_resolved(tr, lang);
    let filter_types: Vec<(String, String)> = {
        let mut v = vec![(String::new(), tr_admin(tr, lang, "admin.form.all_content_types", "Tüm İçerik Türleri", "All Content Types"))];
        for (code, label) in &ct_resolved {
            v.push((code.clone(), label.clone()));
        }
        v
    };
    let filter_types_ref: Vec<(&str, &str)> = filter_types.iter().map(|(c, l)| (c.as_str(), l.as_str())).collect();

    let card_title = tr_admin(tr, lang, "admin.editorial.contents_title", "Yayın & İçerik Envanteri", "Publishing & Content Inventory");
    let card_desc = tr_admin(tr, lang, "admin.editorial.contents_desc", "Tekil kaynaklı içerikler, çoklu bölüm yerleşimleri, 14 SSB alanı eşleşmeleri ve doğrulanmış okuma metrikleri.", "Single-source content, multi-section placements, 14 SSB field mappings and verified reading metrics.");
    let search_ph = tr_admin(tr, lang, "admin.editorial.search_ph", "Başlık, sayı no veya etiket ara...", "Search title, issue no or tag...");
    let search_aria = tr_admin(tr, lang, "admin.editorial.search_aria", "İçerik ara", "Search content");
    let type_filter_aria = tr_admin(tr, lang, "admin.editorial.type_filter_aria", "İçerik türü filtresi", "Content type filter");
    let status_all = tr_admin(tr, lang, "admin.status.all", "Tüm Durumlar", "All Statuses");
    let status_published = tr_admin(tr, lang, "admin.status.published", "Yayında", "Published");
    let status_draft = tr_admin(tr, lang, "admin.status.draft", "Taslak", "Draft");
    let status_pending = tr_admin(tr, lang, "admin.status.pending", "Onay Bekliyor", "Pending Review");
    let status_filter_aria = tr_admin(tr, lang, "admin.editorial.status_filter_aria", "Yayın durumu filtresi", "Publication status filter");
    let new_content = tr_admin(tr, lang, "admin.editorial.new_content", "Yeni İçerik", "New Content");
    let studio_href = format!("/{}/editor", seg);
    let open_studio = tr_admin(tr, lang, "admin.editorial.open_studio", "Editör Stüdyosunda Yaz ↗", "Open Editorial Studio ↗");
    let col_record = tr_admin(tr, lang, "admin.editorial.col_record", "Kayıt / Sayı", "Record / Issue");
    let col_title_status = tr_admin(tr, lang, "admin.editorial.col_title_status", "Başlık & Yayın Durumu", "Title & Status");
    let col_placement = tr_admin(tr, lang, "admin.editorial.col_placement", "Yerleşim & SSB Alanları", "Placement & SSB Fields");
    let col_access = tr_admin(tr, lang, "admin.editorial.col_access", "Erişim Seviyesi", "Access Level");
    let col_engagement = tr_admin(tr, lang, "admin.editorial.col_engagement", "Etkileşim", "Engagement");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let loading = tr_admin(tr, lang, "admin.editorial.contents_loading", "İçerik envanteri yükleniyor...", "Loading content inventory...");

    tab_pane(buf, "tab-1", "tab-btn-1", true, |tab| {
        admin_card(
            tab,
            "",
            None,
            "fa-newspaper-o",
            &card_title,
            Some(&card_desc),
            |hdr| {
                hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                    input_ctrl(tb, "text", "filter-content-search", "input-lg", "", &search_ph, &[("aria-label", &search_aria)]);
                    select_ctrl(tb, "filter-content-type", "input-md", &filter_types_ref, &[("aria-label", &type_filter_aria)]);
                    select_ctrl(
                        tb,
                        "filter-content-status",
                        "input-md",
                        &[
                            ("", &status_all),
                            ("published", &status_published),
                            ("draft", &status_draft),
                            ("pending_review", &status_pending),
                        ],
                        &[("aria-label", &status_filter_aria)],
                    );
                    action_btn(tb, "btn-cmd btn-cmd-primary", "open-content-drawer", Some("contents:write"), Some("fa-plus"), &new_content);
                    tb.el(
                        "a",
                        &[
                            ("href", &studio_href),
                            ("class", "btn-cmd"),
                            ("data-required-perm", "contents:write"),
                        ],
                        |a| {
                            icon(a, "fa-pencil-square-o");
                            a.raw(" ");
                            a.text(&open_studio);
                        },
                    );
                });
            },
            |card| {
                ops_table(
                    card,
                    "",
                    None,
                    &[
                        (&col_record, ""),
                        (&col_title_status, ""),
                        (&col_placement, ""),
                        (&col_access, ""),
                        (&col_engagement, "text-right"),
                        (&col_action, ""),
                    ],
                    "tbody-contents",
                    Some(&loading),
                    Some("pagination-contents"),
                );
            },
        );
        render_content_drawer(tab, tr, lang, &ct_resolved);
    });
}

fn render_content_drawer(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str, ct_resolved: &[(String, String)]) {
    let drawer_title = tr_admin(tr, lang, "admin.editorial.new_content_title", "Yeni İçerik Oluştur", "Create New Content");
    let clear_btn = tr_admin(tr, lang, "admin.form.clear", "Temizle", "Clear");
    let tab_tr = tr_admin(tr, lang, "admin.form.tab_tr", "Türkçe — MSI Dergisi", "Turkish — MSI Review");
    let tab_en = tr_admin(tr, lang, "admin.form.tab_en", "English — MSI TDR", "English — MSI TDR");
    let lbl_title_tr = tr_admin(tr, lang, "admin.form.title_tr", "Başlık (TR)", "Title (TR)");
    let ph_title_tr = tr_admin(tr, lang, "admin.form.title_tr_ph", "Örn: KAAN Milli Muharip Uçak Yeni Faz Testleri", "e.g. KAAN National Combat Aircraft New Phase Trials");
    let lbl_summary_tr = tr_admin(tr, lang, "admin.form.summary_tr", "Yönetici Özeti / Spot (TR)", "Executive Summary / Lead (TR)");
    let ph_summary_tr = tr_admin(tr, lang, "admin.form.summary_tr_ph", "Makale veya haberin yönetici özeti...", "Article or news executive summary...");
    let lbl_body_tr = tr_admin(tr, lang, "admin.form.body_tr", "İçerik Metni (TR — HTML Destekli)", "Body Text (TR — HTML Supported)");
    let lbl_title_en = tr_admin(tr, lang, "admin.form.title_en", "Title (EN)", "Title (EN)");
    let ph_title_en = tr_admin(tr, lang, "admin.form.title_en_ph", "e.g. KAAN National Combat Aircraft New Phase Trials", "e.g. KAAN National Combat Aircraft New Phase Trials");
    let lbl_summary_en = tr_admin(tr, lang, "admin.form.summary_en", "Executive Summary (EN)", "Executive Summary (EN)");
    let ph_summary_en = tr_admin(tr, lang, "admin.form.summary_en_ph", "English summary for MSI TDR readers...", "English summary for MSI TDR readers...");
    let lbl_body_en = tr_admin(tr, lang, "admin.form.body_en", "Article Body (EN — HTML)", "Article Body (EN — HTML)");
    let lbl_content_type = tr_admin(tr, lang, "admin.form.content_type", "İçerik Türü", "Content Type");
    let lbl_source = tr_admin(tr, lang, "admin.form.source", "Yayın Kaynağı", "Publication Source");
    let src_daily = tr_admin(tr, lang, "admin.form.source_daily", "Günlük Haber", "Daily News");
    let src_magazine = tr_admin(tr, lang, "admin.form.source_magazine", "Dergi İçeriği", "Magazine Content");
    let src_online = tr_admin(tr, lang, "admin.form.source_online", "Sadece Dijital", "Digital Only");
    let lbl_status = tr_admin(tr, lang, "admin.form.status", "Yayın Durumu", "Publication Status");
    let st_published = tr_admin(tr, lang, "admin.status.published", "Yayında", "Published");
    let st_draft = tr_admin(tr, lang, "admin.status.draft", "Taslak", "Draft");
    let st_pending = tr_admin(tr, lang, "admin.status.pending", "Onay Bekliyor", "Pending Review");
    let lbl_pub_settings = tr_admin(tr, lang, "admin.form.pub_settings", "Yayın, Yazar & Erişim Duvarı Ayarları", "Publishing, Author & Paywall Settings");
    let lbl_access = tr_admin(tr, lang, "admin.form.access_level", "Erişim Seviyesi", "Access Level");
    let acc_open = tr_admin(tr, lang, "admin.form.access_open", "Açık Erişim", "Open Access");
    let acc_registered = tr_admin(tr, lang, "admin.form.access_registered", "Kayıtlı Okuyucu & Aboneler", "Registered Readers & Subscribers");
    let acc_subscribers = tr_admin(tr, lang, "admin.form.access_subscribers", "Sadece Aboneler / Tekil Satış", "Subscribers Only / Single Sale");
    let lbl_price_issue = tr_admin(tr, lang, "admin.form.price_issue", "Tekil Fiyat (₺) & Sayı No", "Unit Price (₺) & Issue No");
    let title_price = tr_admin(tr, lang, "admin.form.price_title", "Tekil Satış Fiyatı (₺)", "Unit Sale Price (₺)");
    let title_issue = tr_admin(tr, lang, "admin.form.issue_no_title", "Dergi Sayı No", "Magazine Issue No");
    let lbl_readtime_author = tr_admin(tr, lang, "admin.form.readtime_author", "Okuma (Dk) & Yazar", "Read Time (Min) & Author");
    let title_readtime = tr_admin(tr, lang, "admin.form.readtime_title", "Okuma Süresi (Dk)", "Reading Duration (Min)");
    let ph_author = tr_admin(tr, lang, "admin.form.author_ph", "Yazar Adı", "Author Name");
    let default_author = tr_admin(tr, lang, "admin.form.author_default", "MSI Editöryal Masası", "MSI Editorial Desk");
    let lbl_pub_date = tr_admin(tr, lang, "admin.form.pub_date", "Yayın Tarihi ve Saati", "Publication Date & Time");
    let ph_pub_date = tr_admin(tr, lang, "admin.form.pub_date_ph", "YYYY-AA-GG SS:DD (boş bırakılırsa anlık)", "YYYY-MM-DD HH:MM (leave empty for instant)");
    let lbl_author_title = tr_admin(tr, lang, "admin.form.author_title", "Yazar Ünvanı", "Author Title");
    let ph_author_title = tr_admin(tr, lang, "admin.form.author_title_ph", "Savunma Sanayii Analisti", "Defence Industry Analyst");
    let lbl_cover = tr_admin(tr, lang, "admin.form.cover_image", "Kapak Görseli", "Cover Image");
    let lbl_video = tr_admin(tr, lang, "admin.form.video_url", "Video Bağlantısı", "Video URL");
    let lbl_slug = tr_admin(tr, lang, "admin.form.slug", "Kalıcı Bağlantı Kodu", "Permalink Slug");
    let ph_slug = tr_admin(tr, lang, "admin.form.slug_ph", "Otomatik üretilir", "Auto-generated");
    let chk_featured = tr_admin(tr, lang, "admin.form.featured", "Manşet / Öne Çıkan Yayın", "Featured / Highlighted Publication");
    let chk_sponsored = tr_admin(tr, lang, "admin.form.sponsored", "Sponsorlu İçerik (Rakip Korumalı)", "Sponsored Content (Competitor Protected)");
    let lbl_sponsor_brand = tr_admin(tr, lang, "admin.form.sponsor_brand", "Sponsor Marka", "Sponsor Brand");
    let ph_sponsor_brand = tr_admin(tr, lang, "admin.form.sponsor_brand_ph", "Örn: ASELSAN, TUSAŞ, ROKETSAN", "e.g. ASELSAN, TUSAŞ, ROKETSAN");
    let lbl_legal_label = tr_admin(tr, lang, "admin.form.legal_label", "Yasal Etiket", "Legal Label");
    let lbl_placements = tr_admin(tr, lang, "admin.form.placements", "Çoklu Gösterim Bölümleri", "Multi-Section Placements");
    let lbl_ssb = tr_admin(tr, lang, "admin.form.ssb_fields", "14 SSB Savunma Sanayii İlgi Alanı", "14 SSB Defence Industry Interest Areas");
    let lbl_target_insts = tr_admin(tr, lang, "admin.form.target_institutions", "Hedef Kurumlar (Boş bırakılırsa tüm okuyuculara açık)", "Target Institutions (Leave empty for all readers)");
    let cancel_btn = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");
    let close_lbl = tr_admin(tr, lang, "admin.form.close", "Kapat", "Close");
    let choose_lbl = tr_admin(tr, lang, "admin.form.choose", "Seç", "Choose");
    let submit_review = tr_admin(tr, lang, "admin.form.submit_review", "Taslak Kaydet & Onaya Gönder", "Save Draft & Submit for Review");
    let save_publish = tr_admin(tr, lang, "admin.form.save_publish", "Kaydet ve Yayınla", "Save and Publish");

    let ct_refs: Vec<(&str, &str)> = ct_resolved.iter().map(|(c, l)| (c.as_str(), l.as_str())).collect();
    let placements_resolved = placement_tags(tr, lang);
    let placements_refs: Vec<(&str, &str)> = placements_resolved.iter().map(|(c, l)| (*c, l.as_str())).collect();
    let ssb_resolved = ssb_tags(tr, lang);
    let ssb_refs: Vec<(&str, &str)> = ssb_resolved.iter().map(|(c, l)| (*c, l.as_str())).collect();
    let inst_resolved = institution_tags(tr, lang);
    let inst_refs: Vec<(&str, &str)> = inst_resolved.iter().map(|(c, l)| (*c, l.as_str())).collect();

    admin_drawer(
        buf,
        "drawer-content",
        "content-form-heading",
        &drawer_title,
        &close_lbl,
        |act| action_btn(act, "btn-cmd btn-cmd-sm", "reset-content-form", None, Some("fa-eraser"), &clear_btn),
        |body| {
            body.el_with("form", |a| { a.attr("id", "form-content-ssot").bool_attr("novalidate", true); }, |f| {
                f.void_el("input", &[("type", "hidden"), ("id", "c-id"), ("value", "0")]);
                lang_tabs(
                    f,
                    &tab_tr,
                    &tab_en,
                    |tr_tab| {
                        form_group(tr_tab, Some("c-title-tr"), &lbl_title_tr, |g| input_ctrl(g, "text", "c-title-tr", "", "", &ph_title_tr, &[]));
                        form_group(tr_tab, Some("c-summary-tr"), &lbl_summary_tr, |g| textarea_ctrl(g, "c-summary-tr", "textarea-sm", &ph_summary_tr, &[]));
                        form_group(tr_tab, Some("c-body-tr"), &lbl_body_tr, |g| textarea_ctrl(g, "c-body-tr", "", "<p>Detaylı teknik analiz ve haber metni...</p>", &[("rows", "6")]));
                    },
                    |en_tab| {
                        form_group(en_tab, Some("c-title-en"), &lbl_title_en, |g| input_ctrl(g, "text", "c-title-en", "", "", &ph_title_en, &[]));
                        form_group(en_tab, Some("c-summary-en"), &lbl_summary_en, |g| textarea_ctrl(g, "c-summary-en", "textarea-sm", &ph_summary_en, &[]));
                        form_group(en_tab, Some("c-body-en"), &lbl_body_en, |g| textarea_ctrl(g, "c-body-en", "", "<p>Detailed defence analysis in English...</p>", &[("rows", "6")]));
                    },
                );
                f.el("div", &[("class", "form-row-3")], |r| {
                    form_group(r, Some("c-content-type"), &lbl_content_type, |g| select_ctrl(g, "c-content-type", "", &ct_refs, &[]));
                    form_group(r, Some("c-source-group"), &lbl_source, |g| {
                        select_ctrl(g, "c-source-group", "", &[("gunluk-haber", &src_daily), ("dergi-icerigi", &src_magazine), ("sadece-online", &src_online), ("msi-tdr", "MSI TDR")], &[]);
                    });
                    form_group(r, Some("c-status"), &lbl_status, |g| {
                        select_ctrl(g, "c-status", "", &[("published", &st_published), ("draft", &st_draft), ("pending_review", &st_pending)], &[]);
                    });
                });
                details_box(f, "", true, None, &lbl_pub_settings, |d| {
                    d.el("div", &[("class", "form-row-3")], |r| {
                        form_group(r, Some("c-visibility-level"), &lbl_access, |g| {
                            select_ctrl(g, "c-visibility-level", "", &[("0", &acc_open), ("1", &acc_registered), ("2", &acc_subscribers)], &[]);
                        });
                        form_group(r, Some("c-single-price"), &lbl_price_issue, |g| {
                            g.el("div", &[("class", "flex gap-2")], |fl| {
                                input_ctrl(fl, "number", "c-single-price", "", "0", "125.0", &[("step", "0.5"), ("title", &title_price)]);
                                input_ctrl(fl, "number", "c-issue-number", "input-sm", "286", "286", &[("list", "c-issue-datalist"), ("title", &title_issue)]);
                                fl.el("datalist", &[("id", "c-issue-datalist")], |_| {});
                            });
                        });
                        form_group(r, Some("c-author-name"), &lbl_readtime_author, |g| {
                            g.el("div", &[("class", "flex gap-2")], |fl| {
                                input_ctrl(fl, "number", "c-read-time", "input-xs", "5", "", &[("title", &title_readtime)]);
                                input_ctrl(fl, "text", "c-author-name", "", &default_author, &ph_author, &[]);
                            });
                        });
                    });
                    d.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("c-published-at"), &lbl_pub_date, |g| input_ctrl(g, "text", "c-published-at", "", "", &ph_pub_date, &[]));
                        form_group(r, Some("c-author-title"), &lbl_author_title, |g| input_ctrl(g, "text", "c-author-title", "", "", &ph_author_title, &[]));
                    });
                    d.el("div", &[("class", "form-row-3")], |r| {
                        form_group(r, Some("c-cover-image"), &lbl_cover, |g| upload_field(g, "c-cover-image", "images/msi/dergi-286.webp", "", "c-cover-file", ".webp,.png,.jpg,.jpeg,.gif,.pdf,.mp4,.webm", "trigger-cover-upload", "fa-upload", &choose_lbl));
                        form_group(r, Some("c-video-url"), &lbl_video, |g| input_ctrl(g, "text", "c-video-url", "", "", "https://player.vimeo.com/...", &[]));
                        form_group(r, Some("c-slug"), &lbl_slug, |g| input_ctrl(g, "text", "c-slug", "", "", &ph_slug, &[]));
                    });
                    d.el("div", &[("class", "flex items-center justify-between gap-3 mb-2 flex-wrap")], |fl| {
                        check_pill(fl, Some("c-is-featured"), None, None, true, "font-semibold", &chk_featured);
                        check_pill(fl, Some("c-is-sponsored"), None, None, false, "font-semibold", &chk_sponsored);
                    });
                    d.el("div", &[("id", "c-sponsor-extra-row"), ("class", "form-row-2 hidden")], |sr| {
                        form_group(sr, Some("c-sponsor-brand"), &lbl_sponsor_brand, |g| input_ctrl(g, "text", "c-sponsor-brand", "", "", &ph_sponsor_brand, &[]));
                        form_group(sr, Some("c-sponsor-legal-label"), &lbl_legal_label, |g| select_ctrl(g, "c-sponsor-legal-label", "", &[("SPONSORLU İÇERİK", "SPONSORLU İÇERİK"), ("İLANDIR", "İLANDIR"), ("ADVERTORIAL", "ADVERTORIAL")], &[]));
                    });
                });
                details_box(f, "", true, None, &lbl_placements, |d| check_matrix(d, "c-placements-matrix", "c_placement", &placements_refs, &["kapak-dosyalari", "dergi-icerikleri"]));
                details_box(f, "", false, None, &lbl_ssb, |d| check_matrix(d, "c-ssb-matrix", "c_ssb", &ssb_refs, &["hava-araclari", "radarlar-ve-sensorler"]));
                details_box(f, "", false, None, &lbl_target_insts, |d| {
                    check_matrix_with_attrs(d, "c-target-institutions-matrix", "c_target_inst", &inst_refs, &[], &[("data-target-institutions", "1")]);
                });
                f.el("div", &[("id", "content-custom-fields"), ("data-custom-field-values", "content"), ("class", "custom-fields-slot mt-3")], |_| {});
            });
        },
        |ft| {
            action_btn(ft, "btn-cmd", "close-drawer", None, None, &cancel_btn);
            action_btn(ft, "btn-cmd btn-cmd-warning", "submit-form-for-review", Some("contents:write"), None, &submit_review);
            ft.el(
                "button",
                &[("type", "submit"), ("form", "form-content-ssot"), ("class", "btn-cmd btn-cmd-primary"), ("id", "btn-content-save"), ("data-required-perm", "contents:write")],
                |b| {
                    b.text(&save_publish);
                },
            );
        },
    );
}

fn render_tab_issues(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let card_title = tr_admin(tr, lang, "admin.editorial.issues_title", "Dijital Dergi Arşivi & Sayı Yönetimi", "Digital Magazine Archive & Issue Management");
    let card_desc = tr_admin(tr, lang, "admin.editorial.issues_desc", "MSI Dergisi ve MSI TDR basılı/dijital sayılarını, yüksek çözünürlüklü kapaklarını ve DRM filigranlı PDF dosyalarını yönetin.", "Manage MSI Magazine and MSI TDR print/digital issues, high-resolution covers and DRM-watermarked PDF files.");
    let search_ph = tr_admin(tr, lang, "admin.editorial.issue_search_ph", "Sayı no veya başlık ara...", "Search issue no or title...");
    let search_aria = tr_admin(tr, lang, "admin.editorial.issue_search_aria", "Dergi sayısı ara", "Search magazine issue");
    let pub_all = tr_admin(tr, lang, "admin.editorial.pub_all", "Tüm Yayınlar", "All Publications");
    let pub_msi = tr_admin(tr, lang, "admin.editorial.pub_msi", "MSI Dergisi (TR)", "MSI Magazine (TR)");
    let pub_tdr = tr_admin(tr, lang, "admin.editorial.pub_tdr", "MSI TDR (EN)", "MSI TDR (EN)");
    let pub_filter_aria = tr_admin(tr, lang, "admin.editorial.pub_filter_aria", "Yayın filtresi", "Publication filter");
    let new_issue = tr_admin(tr, lang, "admin.editorial.new_issue", "Yeni Dergi Sayısı", "New Magazine Issue");
    let col_issue = tr_admin(tr, lang, "admin.editorial.col_issue", "Sayı", "Issue");
    let col_pub_period = tr_admin(tr, lang, "admin.editorial.col_pub_period", "Yayın & Dönem", "Publication & Period");
    let col_cover_title = tr_admin(tr, lang, "admin.editorial.col_cover_title", "Kapak & Başlık", "Cover & Title");
    let col_scope = tr_admin(tr, lang, "admin.editorial.col_scope", "Kapsam", "Scope");
    let col_epdf = tr_admin(tr, lang, "admin.editorial.col_epdf", "E-Dergi PDF", "E-Magazine PDF");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let loading = tr_admin(tr, lang, "admin.editorial.issues_loading", "Dergi arşivi yükleniyor...", "Loading magazine archive...");
    let drawer_title = tr_admin(tr, lang, "admin.editorial.issue_drawer_title", "Yeni Dergi Sayısı Tanımla", "Define New Magazine Issue");
    let clear_btn = tr_admin(tr, lang, "admin.form.clear", "Temizle", "Clear");
    let lbl_number = tr_admin(tr, lang, "admin.editorial.issue_number", "Sayı Numarası", "Issue Number");
    let lbl_pub_type = tr_admin(tr, lang, "admin.editorial.issue_pub_type", "Yayın", "Publication");
    let lbl_year = tr_admin(tr, lang, "admin.editorial.issue_year", "Yayın Yılı", "Publication Year");
    let tab_tr = tr_admin(tr, lang, "admin.form.tab_tr", "Türkçe — MSI Dergisi", "Turkish — MSI Review");
    let tab_en = tr_admin(tr, lang, "admin.form.tab_en", "English — MSI TDR", "English — MSI TDR");
    let lbl_title_tr = tr_admin(tr, lang, "admin.editorial.issue_title_tr", "Sayı Başlığı (TR)", "Issue Title (TR)");
    let ph_title_tr = tr_admin(tr, lang, "admin.editorial.issue_title_tr_ph", "MSI Dergisi Sayı 287", "MSI Magazine Issue 287");
    let lbl_month_tr = tr_admin(tr, lang, "admin.editorial.issue_month_tr", "Yayın Ayı (TR)", "Publication Month (TR)");
    let ph_month_tr = tr_admin(tr, lang, "admin.editorial.issue_month_tr_ph", "Kasım", "November");
    let lbl_summary_tr = tr_admin(tr, lang, "admin.editorial.issue_summary_tr", "Kapak Dosyası Özeti (TR)", "Cover Dossier Summary (TR)");
    let lbl_title_en = tr_admin(tr, lang, "admin.editorial.issue_title_en", "Issue Title (EN)", "Issue Title (EN)");
    let ph_title_en = tr_admin(tr, lang, "admin.editorial.issue_title_en_ph", "MSI TDR Issue 287", "MSI TDR Issue 287");
    let lbl_month_en = tr_admin(tr, lang, "admin.editorial.issue_month_en", "Month (EN)", "Month (EN)");
    let ph_month_en = tr_admin(tr, lang, "admin.editorial.issue_month_en_ph", "November", "November");
    let lbl_summary_en = tr_admin(tr, lang, "admin.editorial.issue_summary_en", "Issue Summary (EN)", "Issue Summary (EN)");
    let lbl_cover = tr_admin(tr, lang, "admin.form.cover_image", "Kapak Görseli", "Cover Image");
    let lbl_pdf = tr_admin(tr, lang, "admin.editorial.issue_pdf", "E-Dergi PDF Dosyası", "E-Magazine PDF File");
    let lbl_pages = tr_admin(tr, lang, "admin.editorial.issue_pages", "Sayfa Sayısı", "Page Count");
    let lbl_dossiers = tr_admin(tr, lang, "admin.editorial.issue_dossiers", "Özel Dosya Sayısı", "Special Dossier Count");
    let lbl_slug = tr_admin(tr, lang, "admin.form.slug", "Kalıcı Bağlantı Kodu", "Permalink Slug");
    let ph_slug = tr_admin(tr, lang, "admin.form.slug_ph", "Otomatik üretilir", "Auto-generated");
    let lbl_event = tr_admin(tr, lang, "admin.editorial.issue_event", "İlişkili Fuar / Etkinlik", "Related Fair / Event");
    let chk_special = tr_admin(tr, lang, "admin.editorial.issue_special", "Fuar / Etkinlik Özel Sayısı", "Fair / Event Special Issue");
    let iss_auto_convert_lbl = tr_admin(tr, lang, "admin.issues.auto_convert", "PDF yüklenince / kaydedilince otomatik Web E-Dergi'ye dönüştür", "Auto-convert to Web E-Dergi on save");
    let iss_convert_now_lbl = tr_admin(tr, lang, "admin.issues.convert_now", "PDF → Web E-Dergi Dönüştür", "Convert PDF → Web E-Dergi");
    let iss_convert_hint = tr_admin(tr, lang, "admin.issues.convert_hint_new", "PDF yüklendikten sonra tek tıkla sayfa önbelleği ve içindekiler üretilir.", "After uploading a PDF, generate page cache and TOC in one click.");
    let cancel_btn = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");
    let close_lbl = tr_admin(tr, lang, "admin.form.close", "Kapat", "Close");
    let choose_lbl = tr_admin(tr, lang, "admin.form.choose", "Seç", "Choose");
    let save_btn = tr_admin(tr, lang, "admin.editorial.issue_save", "Dergi Sayısını Kaydet", "Save Magazine Issue");

    tab_pane(buf, "tab-18", "tab-btn-18", false, |tab| {
        admin_card(
            tab,
            "",
            None,
            "fa-book",
            &card_title,
            Some(&card_desc),
            |hdr| {
                hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                    input_ctrl(tb, "text", "iss-search", "input-md", "", &search_ph, &[("aria-label", &search_aria)]);
                    select_ctrl(
                        tb,
                        "iss-pub-filter",
                        "input-md",
                        &[("all", &pub_all), ("msi", &pub_msi), ("msi_tdr", &pub_tdr)],
                        &[("aria-label", &pub_filter_aria)],
                    );
                    action_btn(tb, "btn-cmd btn-cmd-primary", "open-issue-drawer", Some("issues:write|issues:manage"), Some("fa-plus"), &new_issue);
                });
            },
            |card| {
                ops_table(
                    card,
                    "",
                    None,
                    &[
                        (&col_issue, ""),
                        (&col_pub_period, ""),
                        (&col_cover_title, ""),
                        (&col_scope, ""),
                        (&col_epdf, ""),
                        (&col_action, ""),
                    ],
                    "tbody-admin-issues",
                    Some(&loading),
                    Some("pagination-issues"),
                );
            },
        );

        admin_drawer(
            tab,
            "drawer-issue",
            "issue-form-heading",
            &drawer_title,
            &close_lbl,
            |act| action_btn(act, "btn-cmd btn-cmd-sm", "reset-issue-form", None, Some("fa-eraser"), &clear_btn),
            |body| {
                body.el_with("form", |a| { a.attr("id", "form-issue").bool_attr("novalidate", true); }, |f| {
                    f.el("div", &[("class", "form-row-3")], |r| {
                        form_group(r, Some("iss-number"), &lbl_number, |g| input_ctrl(g, "number", "iss-number", "", "", "287", &[("min", "1")]));
                        form_group(r, Some("iss-pub-type"), &lbl_pub_type, |g| select_ctrl(g, "iss-pub-type", "", &[("msi", &pub_msi), ("msi_tdr", &pub_tdr)], &[]));
                        form_group(r, Some("iss-year"), &lbl_year, |g| input_ctrl(g, "number", "iss-year", "", "2026", "", &[("min", "2004")]));
                    });
                    f.el("div", &[("id", "iss-key-lock-hint"), ("class", "hidden text-xs text-muted mb-2")], |_| {});
                    lang_tabs(
                        f,
                        &tab_tr,
                        &tab_en,
                        |tr_tab| {
                            tr_tab.el("div", &[("class", "form-row-2")], |r| {
                                form_group(r, Some("iss-title-tr"), &lbl_title_tr, |g| input_ctrl(g, "text", "iss-title-tr", "", "", &ph_title_tr, &[]));
                                form_group(r, Some("iss-month-tr"), &lbl_month_tr, |g| input_ctrl(g, "text", "iss-month-tr", "", "", &ph_month_tr, &[]));
                            });
                            form_group(tr_tab, Some("iss-summary-tr"), &lbl_summary_tr, |g| textarea_ctrl(g, "iss-summary-tr", "textarea-sm", "", &[]));
                        },
                        |en_tab| {
                            en_tab.el("div", &[("class", "form-row-2")], |r| {
                                form_group(r, Some("iss-title-en"), &lbl_title_en, |g| input_ctrl(g, "text", "iss-title-en", "", "", &ph_title_en, &[]));
                                form_group(r, Some("iss-month-en"), &lbl_month_en, |g| input_ctrl(g, "text", "iss-month-en", "", "", &ph_month_en, &[]));
                            });
                            form_group(en_tab, Some("iss-summary-en"), &lbl_summary_en, |g| textarea_ctrl(g, "iss-summary-en", "textarea-sm", "", &[]));
                        },
                    );
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("iss-cover-image"), &lbl_cover, |g| upload_field(g, "iss-cover-image", "", "images/msi/dergi-286.webp", "iss-cover-file", ".webp,.png,.jpg,.jpeg", "trigger-iss-cover", "fa-image", &choose_lbl));
                        form_group(r, Some("iss-pdf-file"), &lbl_pdf, |g| upload_field(g, "iss-pdf-file", "", "msi-dergisi-sayi-287.pdf", "iss-pdf-upload", ".pdf", "trigger-iss-pdf", "fa-file-pdf-o", &choose_lbl));
                    });
                    f.el("div", &[("class", "form-row-3")], |r| {
                        form_group(r, Some("iss-pages"), &lbl_pages, |g| input_ctrl(g, "number", "iss-pages", "", "112", "", &[("min", "1")]));
                        form_group(r, Some("iss-dossiers"), &lbl_dossiers, |g| input_ctrl(g, "number", "iss-dossiers", "", "6", "", &[("min", "0")]));
                        form_group(r, Some("iss-slug"), &lbl_slug, |g| input_ctrl(g, "text", "iss-slug", "", "", &ph_slug, &[]));
                    });
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("iss-event"), &lbl_event, |g| input_ctrl(g, "text", "iss-event", "", "", "IDEF / SAHA EXPO", &[]));
                        r.el("div", &[("class", "form-group form-group-end")], |g| check_pill(g, Some("iss-special"), None, None, false, "", &chk_special));
                    });
                    f.el("div", &[("id", "iss-converter-box"), ("class", "form-group mt-2")], |bx| {
                        bx.el("div", &[("class", "flex flex-wrap items-center justify-between gap-2 p-3 border rounded-md")], |row| {
                            check_pill(row, Some("iss-auto-convert"), None, None, true, "", &iss_auto_convert_lbl);
                            row.el(
                                "button",
                                &[
                                    ("type", "button"),
                                    ("id", "btn-iss-convert-now"),
                                    ("class", "btn-cmd btn-cmd-sm"),
                                    ("data-admin-action", "convert-drawer-pdf"),
                                    ("data-convert-pdf", "1"),
                                ],
                                |b| {
                                    icon(b, "fa-magic");
                                    b.raw(" ");
                                    b.text(&iss_convert_now_lbl);
                                },
                            );
                        });
                        bx.el("div", &[("id", "iss-convert-status"), ("class", "text-xs text-muted mt-1")], |s| {
                            s.text(&iss_convert_hint);
                        });
                    });
                });
            },
            |ft| {
                action_btn(ft, "btn-cmd", "close-drawer", None, None, &cancel_btn);
                ft.el(
                    "button",
                    &[("type", "submit"), ("form", "form-issue"), ("class", "btn-cmd btn-cmd-primary"), ("data-required-perm", "issues:write|issues:manage")],
                    |b| {
                        icon(b, "fa-save");
                        b.raw(" ");
                        b.text(&save_btn);
                    },
                );
            },
        );
    });
}

fn render_tab_media_seo(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let media_title = tr_admin(tr, lang, "admin.editorial.media_title", "Görsel & Medya Kütüphanesi", "Image & Media Library");
    let media_desc = tr_admin(tr, lang, "admin.editorial.media_desc", "Kart üzerine tıklayarak dosya bağlantısını panoya kopyalayın; kalem simgesiyle SEO alt metnini düzenleyin.", "Click a card to copy the file URL to clipboard; use the pencil icon to edit SEO alt text.");
    let media_search_ph = tr_admin(tr, lang, "admin.editorial.media_search_ph", "Dosya adı veya etiket ara...", "Search filename or tag...");
    let media_search_aria = tr_admin(tr, lang, "admin.editorial.media_search_aria", "Medya ara", "Search media");
    let media_upload = tr_admin(tr, lang, "admin.form.upload", "Medya Yükle", "Upload Media");
    let lbl_alt_tr = tr_admin(tr, lang, "admin.editorial.media_alt_tr", "Alternatif Metin (TR)", "Alt Text (TR)");
    let ph_alt_tr = tr_admin(tr, lang, "admin.editorial.media_alt_tr_ph", "Görsel açıklaması (TR)", "Image description (TR)");
    let lbl_alt_en = tr_admin(tr, lang, "admin.editorial.media_alt_en", "Alt Text (EN)", "Alt Text (EN)");
    let ph_alt_en = tr_admin(tr, lang, "admin.editorial.media_alt_en_ph", "Image alt description (EN)", "Image alt description (EN)");
    let lbl_tags = tr_admin(tr, lang, "admin.editorial.media_tags", "Etiketler", "Tags");
    let save_meta = tr_admin(tr, lang, "admin.editorial.media_save_meta", "Bilgileri Kaydet", "Save Metadata");
    let cancel_meta = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");
    let media_loading = tr_admin(tr, lang, "admin.editorial.media_loading", "Medya kütüphanesi yükleniyor...", "Loading media library...");
    let seo_title = tr_admin(tr, lang, "admin.editorial.seo_title", "Canlı Arama & Sosyal Medya Önizlemesi", "Live Search & Social Media Preview");
    let seo_desc = tr_admin(tr, lang, "admin.editorial.seo_desc", "Seçilen yayının Google SERP, Open Graph ve Schema.org çıktısı.", "Google SERP, Open Graph and Schema.org output for the selected publication.");
    let seo_select_label = tr_admin(tr, lang, "admin.editorial.seo_select_label", "Önizlenecek Yayın", "Publication to Preview");
    let seo_select_ph = tr_admin(tr, lang, "admin.editorial.seo_select_ph", "-- İçerik seçin --", "-- Select content --");
    let serp_eyebrow = tr_admin(tr, lang, "admin.editorial.serp_eyebrow", "GOOGLE ARAMA SONUCU (SERP)", "GOOGLE SEARCH RESULT (SERP)");
    let serp_title_default = tr_admin(tr, lang, "admin.editorial.seo_preview_title", "İçerik seçildiğinde başlık burada görüntülenir", "Title will appear here when content is selected");
    let serp_url_default = tr_admin(tr, lang, "admin.editorial.serp_url_default", "msidergisi.com/icerik/ornek-analiz", "msidergisi.com/en/content/sample-analysis");
    let serp_desc_default = tr_admin(tr, lang, "admin.editorial.seo_preview_desc", "Yönetici özeti arama motoru açıklama satırında burada yer alır.", "Executive summary appears here in the search engine description line.");
    let og_eyebrow = tr_admin(tr, lang, "admin.editorial.og_eyebrow", "SOSYAL MEDYA KARTI (OPEN GRAPH)", "SOCIAL MEDIA CARD (OPEN GRAPH)");
    let og_img_ph = tr_admin(tr, lang, "admin.editorial.og_img_ph", "Kapak görseli önizlemesi", "Cover image preview");
    let og_title_default = tr_admin(tr, lang, "admin.editorial.og_title_default", "Sosyal Medya Başlığı", "Social Media Title");
    let og_desc_default = tr_admin(tr, lang, "admin.editorial.og_desc_default", "Paylaşım açıklama özeti", "Share description summary");
    let jsonld_eyebrow = tr_admin(tr, lang, "admin.editorial.jsonld_eyebrow", "YAPISAL VERİ ŞEMASI (JSON-LD)", "STRUCTURED DATA SCHEMA (JSON-LD)");

    tab_pane(buf, "tab-10", "tab-btn-10", false, |tab| {
        tab.el("div", &[("class", "workspace-grid media-seo-grid")], |grid| {
            admin_card(
                grid,
                "",
                None,
                "fa-picture-o",
                &media_title,
                Some(&media_desc),
                |hdr| {
                    hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                        input_ctrl(tb, "text", "media-search", "input-lg", "", &media_search_ph, &[("aria-label", &media_search_aria)]);
                        tb.el(
                            "label",
                            &[
                                ("for", "media-bulk-upload"),
                                ("class", "btn-cmd btn-cmd-primary btn-cmd-sm"),
                                ("data-required-perm", "media:upload|contents:write"),
                                ("role", "button"),
                                ("tabindex", "0"),
                                ("aria-label", &media_upload),
                            ],
                            |lbl| {
                                icon(lbl, "fa-upload");
                                lbl.raw(" ");
                                lbl.text(&media_upload);
                                lbl.void_with("input", |a| {
                                    a.attr("type", "file").attr("id", "media-bulk-upload").attr("class", "hidden").bool_attr("multiple", true);
                                });
                            },
                        );
                    });
                },
                |card| {
                    card.el("div", &[("id", "media-edit-panel"), ("class", "hidden form-container-elevated")], |ep| {
                        ep.void_el("input", &[("type", "hidden"), ("id", "media-edit-id"), ("value", "0")]);
                        ep.el("div", &[("class", "form-row-3 mb-2")], |r| {
                            form_group(r, Some("media-edit-alt-tr"), &lbl_alt_tr, |g| input_ctrl(g, "text", "media-edit-alt-tr", "", "", &ph_alt_tr, &[]));
                            form_group(r, Some("media-edit-alt-en"), &lbl_alt_en, |g| input_ctrl(g, "text", "media-edit-alt-en", "", "", &ph_alt_en, &[]));
                            form_group(r, Some("media-edit-tags"), &lbl_tags, |g| input_ctrl(g, "text", "media-edit-tags", "", "", "kaan, tusas, hava", &[]));
                        });
                        ep.el("div", &[("class", "flex gap-2")], |fl| {
                            action_btn(fl, "btn-cmd btn-cmd-primary btn-cmd-sm", "save-media-meta", None, Some("fa-floppy-o"), &save_meta);
                            action_btn(fl, "btn-cmd btn-cmd-sm", "cancel-media-meta", None, None, &cancel_meta);
                        });
                    });
                    card.el("div", &[("id", "media-library-grid"), ("class", "grid-media")], |mg| {
                        mg.el("div", &[("class", "text-muted text-center p-4 col-span-full")], |t| {
                            t.text(&media_loading);
                        });
                    });
                    card.el("div", &[("id", "pagination-media-library")], |_| {});
                },
            );

            admin_card(
                grid,
                "",
                None,
                "fa-search",
                &seo_title,
                Some(&seo_desc),
                |_| {},
                |card| {
                    card.el("div", &[("class", "panel-body")], |bd| {
                        form_group(bd, Some("seo-preview-select"), &seo_select_label, |g| {
                            select_ctrl(g, "seo-preview-select", "", &[("", &seo_select_ph)], &[]);
                        });
                        bd.el("div", &[("id", "seo-preview-panel"), ("class", "text-sm")], |sp| {
                            sp.el("div", &[("class", "seo-preview-box")], |bx| {
                                bx.el("div", &[("class", "seo-box-eyebrow text-info")], |t| {
                                    t.text(&serp_eyebrow);
                                });
                                bx.el("div", &[("id", "seo-google-title"), ("class", "text-search-link")], |t| {
                                    t.text(&serp_title_default);
                                });
                                bx.el("div", &[("id", "seo-google-url"), ("class", "text-muted text-xs mb-1 text-mono")], |t| {
                                    t.text(&serp_url_default);
                                });
                                bx.el("div", &[("id", "seo-google-desc"), ("class", "text-secondary text-og-desc")], |t| {
                                    t.text(&serp_desc_default);
                                });
                            });
                            sp.el("div", &[("class", "seo-preview-box")], |bx| {
                                bx.el("div", &[("class", "seo-box-eyebrow text-success")], |t| {
                                    t.text(&og_eyebrow);
                                });
                                bx.el("div", &[("id", "seo-og-preview"), ("class", "border rounded-md overflow-hidden")], |og| {
                                    og.el("div", &[("id", "seo-og-img-wrap"), ("class", "img-placeholder text-muted text-xs")], |p| {
                                        p.text(&og_img_ph);
                                    });
                                    og.el("div", &[("class", "p-3")], |cp| {
                                        cp.el("div", &[("id", "seo-og-title"), ("class", "font-bold text-sm mb-sm")], |t| {
                                            t.text(&og_title_default);
                                        });
                                        cp.el("div", &[("id", "seo-og-desc"), ("class", "text-muted text-xs")], |t| {
                                            t.text(&og_desc_default);
                                        });
                                    });
                                });
                            });
                            sp.el("div", &[("class", "seo-preview-box-inner")], |bx| {
                                bx.el("div", &[("class", "seo-box-eyebrow text-warning")], |t| {
                                    t.text(&jsonld_eyebrow);
                                });
                                bx.el("pre", &[("id", "seo-jsonld"), ("class", "text-mono text-xs pre-scrollable")], |p| {
                                    p.text("{}");
                                });
                            });
                        });
                    });
                },
            );
        });

        let guide_title = tr_admin(tr, lang, "admin.editorial.media_guide_title", "Medya Teknik Özellikleri ve Boyut Rehberi", "Media Technical Specification Guide");
        let guide_desc = tr_admin(tr, lang, "admin.editorial.media_guide_desc", "Platform genelinde yüksek performans, hızlı yükleme ve reklam alanı uyumluluğu için standart medya boyutları ve format sınırları.", "Standard media dimensions, aspect ratios, formats and file size limits across editorial and advertising placements.");
        let col_slot = tr_admin(tr, lang, "admin.editorial.media_guide_col_slot", "Yerleşim / Kullanım Alanı", "Placement / Slot");
        let col_dim = tr_admin(tr, lang, "admin.editorial.media_guide_col_dim", "Önerilen Boyut (px)", "Recommended Size (px)");
        let col_fmt = tr_admin(tr, lang, "admin.editorial.media_guide_col_fmt", "Format", "Format");
        let col_max = tr_admin(tr, lang, "admin.editorial.media_guide_col_max", "Azami Boyut", "Max File Size");
        let col_note = tr_admin(tr, lang, "admin.editorial.media_guide_col_note", "Teknik Not", "Technical Note");
        let rows: Vec<Vec<String>> = vec![
            vec![
                tr_admin(tr, lang, "admin.editorial.mg_hero", "Ana Manşet / Hero Kapak", "Hero / Featured Cover"),
                "1600 × 900 px (16:9)".into(),
                "WebP / JPEG".into(),
                "250 KB".into(),
                tr_admin(tr, lang, "admin.editorial.mg_hero_n", "Yüksek çözünürlüklü 16:9 yatay vitrin görseli.", "High-resolution 16:9 widescreen hero asset."),
            ],
            vec![
                tr_admin(tr, lang, "admin.editorial.mg_article", "Haber & Makale Kartı", "Article & News Card"),
                "1200 × 675 px (16:9)".into(),
                "WebP / JPEG".into(),
                "250 KB".into(),
                tr_admin(tr, lang, "admin.editorial.mg_article_n", "Kategori ve liste kartları için standart 16:9.", "Standard 16:9 aspect ratio for feeds."),
            ],
            vec![
                tr_admin(tr, lang, "admin.editorial.mg_issue", "E-Dergi Sayı Kapağı", "E-Magazine Issue Cover"),
                "960 × 1200 px (4:5)".into(),
                "WebP / PNG".into(),
                "300 KB".into(),
                tr_admin(tr, lang, "admin.editorial.mg_issue_n", "Dikey dergi kapak oranı (4:5).", "Portrait magazine cover aspect ratio (4:5)."),
            ],
            vec![
                tr_admin(tr, lang, "admin.editorial.mg_leader", "Üst & Alt Yatay Banner (Leaderboard)", "Top & Footer Leaderboard Banner"),
                "970 × 90 px / 728 × 90 px".into(),
                "WebP / PNG / SVG".into(),
                "150 KB".into(),
                tr_admin(tr, lang, "admin.editorial.mg_leader_n", "Masaüstü geniş banner reklam yerleşimi.", "Desktop full-width banner ad placement."),
            ],
            vec![
                tr_admin(tr, lang, "admin.editorial.mg_tower", "Yan Dikey Kule Banner (Tower)", "Sidebar Tower Banner"),
                "300 × 900 px / 300 × 600 px".into(),
                "WebP / PNG".into(),
                "180 KB".into(),
                tr_admin(tr, lang, "admin.editorial.mg_tower_n", "Sağ/sol kenar çubuğu dikey sponsor alanı.", "Sidebar vertical skyscraper sponsor slot."),
            ],
            vec![
                tr_admin(tr, lang, "admin.editorial.mg_popup", "Kayan Açılır Duyuru (Floating Popup)", "Floating Popup Banner"),
                "600 × 400 px (3:2)".into(),
                "WebP / PNG".into(),
                "200 KB".into(),
                tr_admin(tr, lang, "admin.editorial.mg_popup_n", "Oturum bazlı kapatılabilir sponsor kutusu.", "Dismissible session-capped sponsor popup."),
            ],
            vec![
                tr_admin(tr, lang, "admin.editorial.mg_pdf", "E-Dergi Baskı & Dijital Sayı", "E-Magazine Full Issue PDF"),
                "A4 (210 × 297 mm) / PDF/A".into(),
                "PDF".into(),
                "25 MB".into(),
                tr_admin(tr, lang, "admin.editorial.mg_pdf_n", "Dinamik DRM filigranı ve sayfa dilimleme ile uyumlu.", "Compatible with dynamic DRM watermarking."),
            ],
            vec![
                tr_admin(tr, lang, "admin.editorial.mg_video", "Video & Savunma Klibi", "Video & Defence Briefing Clip"),
                "1920 × 1080 px (16:9)".into(),
                "MP4 (H.264)".into(),
                "25 MB".into(),
                tr_admin(tr, lang, "admin.editorial.mg_video_n", "İzlenme yüzdesi telemetrisi (video_watch_pct) destekli.", "Tracked with granular video_watch_pct telemetry."),
            ],
        ];
        admin_card(
            tab,
            "mt-4",
            Some("media-spec-guide-card"),
            "fa-sliders",
            &guide_title,
            Some(&guide_desc),
            |_| {},
            |card| {
                let headers = [col_slot.as_str(), col_dim.as_str(), col_fmt.as_str(), col_max.as_str(), col_note.as_str()];
                let row_refs: Vec<Vec<&str>> = rows.iter().map(|r| r.iter().map(|s| s.as_str()).collect()).collect();
                let mut html = String::from("<div class=\"table-responsive\"><table class=\"table ops-table\"><thead><tr>");
                for h in &headers {
                    html.push_str(&format!("<th>{}</th>", crate::engine::security_engine::escape_html(h)));
                }
                html.push_str("</tr></thead><tbody>");
                for r in &row_refs {
                    html.push_str("<tr>");
                    for cell in r {
                        html.push_str(&format!("<td>{}</td>", crate::engine::security_engine::escape_html(cell)));
                    }
                    html.push_str("</tr>");
                }
                html.push_str("</tbody></table></div>");
                card.raw(&html);
            },
        );
    });
}

fn render_tab_taxonomy(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let subtab_cats = tr_admin(tr, lang, "admin.editorial.tax_subtab_cats", "Yayın Kategorileri", "Publication Categories");
    let subtab_ssb = tr_admin(tr, lang, "admin.editorial.tax_subtab_ssb", "14 SSB Savunma İlgi Alanı", "14 SSB Defence Interest Areas");
    let subtab_recs = tr_admin(tr, lang, "admin.editorial.tax_subtab_recs", "Öneri Kuralları", "Recommendation Rules");
    let rec_form_title = tr_admin(tr, lang, "admin.editorial.rec_form_title", "Yeni Öneri Kuralı Ekle / Güncelle", "Add / Update Recommendation Rule");
    let rec_name_lbl = tr_admin(tr, lang, "admin.editorial.rec_rule_name", "Kural Adı", "Rule Name");
    let rec_name_ph = tr_admin(tr, lang, "admin.editorial.rec_rule_name_ph", "Örn: Hava Araçları İçin İlgili Analiz", "e.g. Related Analysis for Aerial Systems");
    let rec_src_lbl = tr_admin(tr, lang, "admin.editorial.rec_source", "Kaynak Etiket / Kategori", "Source Tag / Category");
    let rec_src_ph = tr_admin(tr, lang, "admin.editorial.rec_source_ph", "hava-araclari veya *", "hava-araclari or *");
    let rec_inst_lbl = tr_admin(tr, lang, "admin.editorial.rec_target_inst", "Hedef Kurum", "Target Institution");
    let rec_all_inst = tr_admin(tr, lang, "admin.editorial.rec_all_inst", "Tüm Kurumlar (*)", "All Institutions (*)");
    let rec_slug_lbl = tr_admin(tr, lang, "admin.editorial.rec_slug", "Önerilen İçerik Slug", "Recommended Content Slug");
    let rec_slug_ph = tr_admin(tr, lang, "admin.editorial.rec_slug_ph", "kaan-milli-muharip-ucak-analiz", "kaan-fighter-analysis");
    let rec_prio_lbl = tr_admin(tr, lang, "admin.editorial.rec_priority", "Öncelik", "Priority");
    let rec_active_lbl = tr_admin(tr, lang, "admin.editorial.rec_is_active", "Aktif Kural", "Active Rule");
    let rec_save_btn = tr_admin(tr, lang, "admin.editorial.rec_save", "Öneri Kuralını Kaydet", "Save Recommendation Rule");
    let rec_clear_btn = tr_admin(tr, lang, "admin.form.clear", "Temizle", "Clear");
    let rec_card_title = tr_admin(tr, lang, "admin.editorial.rec_card_title", "Akıllı İçerik Öneri Kuralları", "Smart Content Recommendation Rules");
    let rec_card_desc = tr_admin(tr, lang, "admin.editorial.rec_card_desc", "Makale içi ve makale sonu ilgili içerik bloklarında kurum ve SSB etiketine göre öncelikli öneri eşleşmelerini yönetin.", "Manage priority recommendation mappings by institution and SSB tag for inline and bottom article blocks.");
    let rec_col_name = tr_admin(tr, lang, "admin.editorial.rec_col_name", "Kural Adı", "Rule Name");
    let rec_col_src = tr_admin(tr, lang, "admin.editorial.rec_col_source", "Kaynak Etiket", "Source Tag");
    let rec_col_inst = tr_admin(tr, lang, "admin.editorial.rec_col_inst", "Hedef Kurum", "Target Institution");
    let rec_col_slug = tr_admin(tr, lang, "admin.editorial.rec_col_slug", "Önerilen İçerik", "Recommended Slug");
    let rec_col_status = tr_admin(tr, lang, "admin.workflow.col_status", "Durum", "Status");
    let rec_loading = tr_admin(tr, lang, "admin.editorial.rec_loading", "Öneri kuralları yükleniyor...", "Loading recommendation rules...");
    let inst_resolved = institution_tags(tr, lang);
    let mut rec_inst_opts: Vec<(&str, &str)> = vec![("*", &rec_all_inst)];
    for (code, lbl) in &inst_resolved {
        rec_inst_opts.push((*code, lbl.as_str()));
    }
    let cat_form_title = tr_admin(tr, lang, "admin.editorial.cat_form_title", "Yeni Kategori Tanımla / Düzenle", "Define / Edit Category");
    let lbl_cat_name_tr = tr_admin(tr, lang, "admin.editorial.cat_name_tr", "Kategori Adı (TR)", "Category Name (TR)");
    let ph_cat_name_tr = tr_admin(tr, lang, "admin.editorial.cat_name_tr_ph", "Örn: İnsansız Deniz Sistemleri", "e.g. Unmanned Naval Systems");
    let lbl_cat_name_en = tr_admin(tr, lang, "admin.editorial.cat_name_en", "Category Name (EN)", "Category Name (EN)");
    let ph_cat_name_en = tr_admin(tr, lang, "admin.editorial.cat_name_en_ph", "e.g. Unmanned Naval Systems", "e.g. Unmanned Naval Systems");
    let lbl_cat_slug = tr_admin(tr, lang, "admin.form.slug", "Kalıcı Bağlantı Kodu", "Permalink Slug");
    let ph_cat_slug = tr_admin(tr, lang, "admin.editorial.cat_slug_ph", "insansiz-deniz-sistemleri", "unmanned-naval-systems");
    let lbl_cat_group = tr_admin(tr, lang, "admin.editorial.cat_group", "Bölüm Grubu", "Section Group");
    let grp_contents = tr_admin(tr, lang, "admin.editorial.cat_grp_contents", "İçerikler", "Content");
    let grp_grouped = tr_admin(tr, lang, "admin.editorial.cat_grp_grouped", "Gruplandırılmış", "Grouped");
    let grp_subscription = tr_admin(tr, lang, "admin.editorial.cat_grp_subscription", "Abonelik", "Subscription");
    let grp_corporate = tr_admin(tr, lang, "admin.editorial.cat_grp_corporate", "Kurumsal", "Corporate");
    let lbl_cat_sort = tr_admin(tr, lang, "admin.editorial.cat_sort", "Sıra", "Sort Order");
    let lbl_cat_desc = tr_admin(tr, lang, "admin.editorial.cat_desc_tr", "Kategori Açıklaması (TR)", "Category Description (TR)");
    let ph_cat_desc = tr_admin(tr, lang, "admin.editorial.cat_desc_ph", "Bölüm açıklaması...", "Section description...");
    let cat_save_btn = tr_admin(tr, lang, "admin.editorial.cat_save", "Kategoriyi Kaydet", "Save Category");
    let cat_card_title = tr_admin(tr, lang, "admin.editorial.cat_card_title", "Mevcut Yayın Kategorileri", "Existing Publication Categories");
    let cat_card_desc = tr_admin(tr, lang, "admin.editorial.cat_card_desc", "Çekirdek ve dinamik olarak tanımlanmış tüm portal bölümleri.", "All core and dynamically defined portal sections.");
    let col_permalink = tr_admin(tr, lang, "admin.editorial.col_permalink", "Kalıcı Bağlantı", "Permalink");
    let col_cat_name = tr_admin(tr, lang, "admin.editorial.col_cat_name", "Kategori Adı (TR / EN)", "Category Name (TR / EN)");
    let col_section_group = tr_admin(tr, lang, "admin.editorial.col_section_group", "Bölüm Grubu", "Section Group");
    let col_structure = tr_admin(tr, lang, "admin.editorial.col_structure", "Yapı", "Structure");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let cats_loading = tr_admin(tr, lang, "admin.editorial.cats_loading", "Kategoriler yükleniyor...", "Loading categories...");
    let ssb_form_title = tr_admin(tr, lang, "admin.editorial.ssb_form_title", "SSB Katalog İlgi Alanı Ekle / Düzenle", "Add / Edit SSB Catalog Interest Area");
    let lbl_ssb_name_tr = tr_admin(tr, lang, "admin.editorial.ssb_name_tr", "SSB İlgi Alanı (TR)", "SSB Interest Area (TR)");
    let ph_ssb_name_tr = tr_admin(tr, lang, "admin.editorial.ssb_name_tr_ph", "Örn: Kuantum ve Fotonik", "e.g. Quantum & Photonic");
    let lbl_ssb_name_en = tr_admin(tr, lang, "admin.editorial.ssb_name_en", "SSB Interest Area (EN)", "SSB Interest Area (EN)");
    let ph_ssb_name_en = tr_admin(tr, lang, "admin.editorial.ssb_name_en_ph", "e.g. Quantum & Photonic", "e.g. Quantum & Photonic");
    let lbl_ssb_code = tr_admin(tr, lang, "admin.editorial.ssb_code", "Katalog Kodu", "Catalog Code");
    let ph_ssb_code = tr_admin(tr, lang, "admin.editorial.ssb_code_ph", "kuantum-ve-fotonik", "quantum-and-photonic");
    let lbl_ssb_group = tr_admin(tr, lang, "admin.editorial.ssb_tech_group", "Teknoloji Grubu", "Technology Group");
    let ph_ssb_group = tr_admin(tr, lang, "admin.editorial.ssb_tech_group_ph", "İleri Teknolojiler", "Advanced Technologies");
    let lbl_ssb_sort = tr_admin(tr, lang, "admin.editorial.ssb_sort", "Sıra", "Sort Order");
    let ssb_save_btn = tr_admin(tr, lang, "admin.editorial.ssb_save", "SSB İlgi Alanını Kaydet", "Save SSB Interest Area");
    let ssb_card_title = tr_admin(tr, lang, "admin.editorial.ssb_card_title", "14 SSB Savunma Sanayii İlgi Alanı Kataloğu", "14 SSB Defence Industry Interest Area Catalog");
    let ssb_card_desc = tr_admin(tr, lang, "admin.editorial.ssb_card_desc", "İçerik etiketleme, kurumsal bülten ve reklam hedeflemede kullanılan resmi taksonomi.", "Official taxonomy used for content tagging, corporate newsletters and ad targeting.");
    let col_catalog_code = tr_admin(tr, lang, "admin.editorial.col_catalog_code", "Katalog Kodu", "Catalog Code");
    let col_interest = tr_admin(tr, lang, "admin.editorial.col_interest", "İlgi Alanı (TR / EN)", "Interest Area (TR / EN)");
    let col_parent_group = tr_admin(tr, lang, "admin.editorial.col_parent_group", "Üst Grup", "Parent Group");
    let col_sort = tr_admin(tr, lang, "admin.editorial.col_sort", "Sıra", "Sort");
    let ssb_loading = tr_admin(tr, lang, "admin.editorial.ssb_loading", "SSB kataloğu yükleniyor...", "Loading SSB catalog...");

    tab_pane(buf, "tab-5", "tab-btn-5", false, |tab| {
        subtab_bar(tab, "tab5", &[
            ("tab5-cats", &subtab_cats, None),
            ("tab5-ssb", &subtab_ssb, None),
            ("tab5-recommendations", &subtab_recs, Some("contents:read|settings:read|ads:read")),
        ]);

        subtab_pane(tab, "tab5-cats", true, None, |pane| {
            details_box(pane, "admin-card mb-4", false, Some("categories:write|settings:write"), &cat_form_title, |d| {
                d.el_with("form", |a| { a.attr("id", "form-new-category").attr("data-required-perm", "categories:write|settings:write").bool_attr("novalidate", true); }, |f| {
                    f.void_el("input", &[("type", "hidden"), ("id", "cat-edit-id"), ("value", "0")]);
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("cat-name-tr"), &lbl_cat_name_tr, |g| input_ctrl(g, "text", "cat-name-tr", "", "", &ph_cat_name_tr, &[]));
                        form_group(r, Some("cat-name-en"), &lbl_cat_name_en, |g| input_ctrl(g, "text", "cat-name-en", "", "", &ph_cat_name_en, &[]));
                    });
                    f.el("div", &[("class", "form-row-3")], |r| {
                        form_group(r, Some("cat-slug"), &lbl_cat_slug, |g| input_ctrl(g, "text", "cat-slug", "", "", &ph_cat_slug, &[]));
                        form_group(r, Some("cat-group-type"), &lbl_cat_group, |g| {
                            select_ctrl(g, "cat-group-type", "", &[("icerikler", &grp_contents), ("gruplandirilmis", &grp_grouped), ("abonelik", &grp_subscription), ("kurumsal", &grp_corporate)], &[]);
                        });
                        form_group(r, Some("cat-sort-order"), &lbl_cat_sort, |g| input_ctrl(g, "number", "cat-sort-order", "", "25", "", &[]));
                    });
                    form_group(f, Some("cat-desc-tr"), &lbl_cat_desc, |g| input_ctrl(g, "text", "cat-desc-tr", "", "", &ph_cat_desc, &[]));
                    f.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-primary")], |b| {
                        icon(b, "fa-plus");
                        b.raw(" ");
                        b.text(&cat_save_btn);
                    });
                });
            });
            admin_card(pane, "", None, "fa-sitemap", &cat_card_title, Some(&cat_card_desc), |_| {}, |card| {
                ops_table(card, "", None, &[(&col_permalink, ""), (&col_cat_name, ""), (&col_section_group, ""), (&col_structure, ""), (&col_action, "")], "tbody-categories", Some(&cats_loading), None);
            });
        });

        subtab_pane(tab, "tab5-ssb", false, None, |pane| {
            details_box(pane, "admin-card mb-4", false, Some("categories:write|settings:write"), &ssb_form_title, |d| {
                d.el_with("form", |a| { a.attr("id", "form-new-ssb").attr("data-required-perm", "categories:write|settings:write").bool_attr("novalidate", true); }, |f| {
                    f.void_el("input", &[("type", "hidden"), ("id", "ssb-edit-old-code"), ("value", "")]);
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("ssb-name-tr"), &lbl_ssb_name_tr, |g| input_ctrl(g, "text", "ssb-name-tr", "", "", &ph_ssb_name_tr, &[]));
                        form_group(r, Some("ssb-name-en"), &lbl_ssb_name_en, |g| input_ctrl(g, "text", "ssb-name-en", "", "", &ph_ssb_name_en, &[]));
                    });
                    f.el("div", &[("class", "form-row-3")], |r| {
                        form_group(r, Some("ssb-code"), &lbl_ssb_code, |g| input_ctrl(g, "text", "ssb-code", "", "", &ph_ssb_code, &[]));
                        form_group(r, Some("ssb-group"), &lbl_ssb_group, |g| input_ctrl(g, "text", "ssb-group", "", "", &ph_ssb_group, &[]));
                        form_group(r, Some("ssb-sort"), &lbl_ssb_sort, |g| input_ctrl(g, "number", "ssb-sort", "", "15", "", &[]));
                    });
                    f.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-success")], |b| {
                        icon(b, "fa-tags");
                        b.raw(" ");
                        b.text(&ssb_save_btn);
                    });
                });
            });
            admin_card(pane, "", None, "fa-tasks", &ssb_card_title, Some(&ssb_card_desc), |_| {}, |card| {
                ops_table(card, "", None, &[(&col_catalog_code, ""), (&col_interest, ""), (&col_parent_group, ""), (&col_sort, ""), (&col_action, "")], "tbody-ssb-tags", Some(&ssb_loading), None);
            });
        });

        subtab_pane(tab, "tab5-recommendations", false, Some("contents:read|settings:read|ads:read"), |pane| {
            pane.el("div", &[("data-recommendation-rules", "1")], |wrap| {
                details_box(wrap, "admin-card mb-4", true, Some("contents:write|contents:manage|settings:write"), &rec_form_title, |d| {
                    d.el_with("form", |a| { a.attr("id", "form-recommendation-rule").attr("data-required-perm", "contents:write|contents:manage|settings:write").bool_attr("novalidate", true); }, |f| {
                        f.void_el("input", &[("type", "hidden"), ("id", "rec-id"), ("value", "0")]);
                        f.el("div", &[("id", "rec-form-mode-badge"), ("class", "hidden text-xs font-semibold text-accent mb-2"), ("role", "status")], |_| {});
                        f.el("div", &[("class", "form-row-3")], |r| {
                            form_group(r, Some("rec-rule-name"), &rec_name_lbl, |g| input_ctrl(g, "text", "rec-rule-name", "", "", &rec_name_ph, &[("name", "rule_name"), ("required", "required")]));
                            form_group(r, Some("rec-source-tag"), &rec_src_lbl, |g| input_ctrl(g, "text", "rec-source-tag", "", "*", &rec_src_ph, &[("name", "source_tag_or_cat"), ("list", "rec-source-tag-list"), ("required", "required")]));
                            form_group(r, Some("rec-target-inst"), &rec_inst_lbl, |g| select_ctrl(g, "rec-target-inst", "", &rec_inst_opts, &[("name", "target_institution")]));
                        });
                        f.el("div", &[("class", "form-row-3 items-end")], |r| {
                            form_group(r, Some("rec-content-slug"), &rec_slug_lbl, |g| input_ctrl(g, "text", "rec-content-slug", "", "", &rec_slug_ph, &[("name", "recommended_content_slug"), ("list", "rec-content-slug-list"), ("required", "required")]));
                            form_group(r, Some("rec-priority"), &rec_prio_lbl, |g| input_ctrl(g, "number", "rec-priority", "", "10", "10", &[("name", "priority")]));
                            r.el("div", &[("class", "form-group form-group-end")], |g| check_pill(g, Some("rec-is-active"), Some("is_active"), None, true, "", &rec_active_lbl));
                        });
                        f.el("datalist", &[("id", "rec-source-tag-list")], |_| {});
                        f.el("datalist", &[("id", "rec-content-slug-list")], |_| {});
                        f.el("div", &[("class", "flex gap-2 flex-wrap")], |btns| {
                            btns.el("button", &[("type", "submit"), ("id", "btn-rec-save"), ("class", "btn-cmd btn-cmd-primary btn-cmd-sm")], |b| {
                                icon(b, "fa-save");
                                b.raw(" ");
                                b.text(&rec_save_btn);
                            });
                            action_btn(btns, "btn-cmd btn-cmd-sm", "reset-rec-rule-form", None, Some("fa-eraser"), &rec_clear_btn);
                        });
                    });
                });
                admin_card(wrap, "", None, "fa-random", &rec_card_title, Some(&rec_card_desc), |_| {}, |card| {
                    ops_table(
                        card,
                        "",
                        Some("table-recommendation-rules"),
                        &[
                            (&rec_col_name, ""),
                            (&rec_col_src, ""),
                            (&rec_col_inst, ""),
                            (&rec_col_slug, ""),
                            (&rec_prio_lbl, "text-right"),
                            (&rec_col_status, ""),
                            (&col_action, ""),
                        ],
                        "tbody-recommendation-rules",
                        Some(&rec_loading),
                        None,
                    );
                });
            });
        });
    });
}
