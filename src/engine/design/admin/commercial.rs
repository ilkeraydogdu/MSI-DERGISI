//! Commercial & Corporate Views: `#tab-2` Ad Campaigns & Drawer,
//! `#tab-14` 4-Subtab Revenue/Invoices/Plans/Products, `#tab-4` Corporate Approvals.

use super::ui::{
    action_btn, action_btn_extra, admin_card, admin_drawer, check_matrix, check_pill, details_box,
    form_group, icon, input_ctrl, kpi_card_toned, lang_tabs, ops_table, select_ctrl, subtab_bar,
    subtab_pane, tab_pane, textarea_ctrl, upload_field,
};
use super::{institution_tags, ssb_tags, tr_admin};
use crate::engine::design::html_dsl::HtmlBuf;
use crate::engine::translation_engine::TranslationCache;

/// Ad slot keys for i18n lookup.
const AD_SLOT_KEYS: &[(&str, &str, &str)] = &[
    ("ad-mag-top", "Dergi Üst Kuşak (970x90)", "Magazine Top Banner (970x90)"),
    ("ad-mag-mid", "Dergi Orta Kuşak (728x90)", "Magazine Mid Banner (728x90)"),
    ("ad-mag-pricing", "Abonelik Yan Alanı (300x250)", "Subscription Side Area (300x250)"),
    ("ad-news-top", "Haber Üst Kuşak (970x90)", "News Top Banner (970x90)"),
    ("ad-news-infeed", "Haber Akış İçi (728x90)", "News In-Feed (728x90)"),
    ("ad-news-mid", "Haber Orta Kutu (300x250)", "News Mid Box (300x250)"),
    ("ad-news-billboard", "Prestij Panosu (970x250)", "Prestige Billboard (970x250)"),
    ("ad-floating-popup", "Giriş Tanıtım Penceresi (600x400)", "Entry Promo Popup (600x400)"),
    ("ad-newsletter-top", "E-Bülten Üst Sponsor (600x120)", "Newsletter Top Sponsor (600x120)"),
];

fn ad_slots_resolved(tr: &TranslationCache, lang: &str) -> Vec<(String, String)> {
    AD_SLOT_KEYS
        .iter()
        .map(|(code, fb_tr, fb_en)| {
            let key = format!("admin.commercial.ad_slot.{}", code.replace('-', "_"));
            let label = tr_admin(tr, lang, &key, fb_tr, fb_en);
            (code.to_string(), label)
        })
        .collect()
}

pub fn render(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    render_tab_ads(buf, tr, lang);
    render_tab_commerce(buf, tr, lang);
    render_tab_corporate(buf, tr, lang);
}

fn render_tab_ads(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let banner_strong = tr_admin(tr, lang, "admin.commercial.oem_banner_strong", "Sponsorlu İçerik Rakip OEM Koruması Devrede:", "Sponsored Content Competitor OEM Protection Active:");
    let banner_text = tr_admin(tr, lang, "admin.commercial.oem_protection_banner", " Sponsorlu içerik sayfalarında rakip savunma üreticisi reklamları otomatik filtrelenir.", " Competitor defence manufacturer ads are automatically filtered on sponsored content pages.");
    let banner_badge = tr_admin(tr, lang, "admin.commercial.oem_active_badge", "Aktif Koruma", "Active Protection");
    let card_title = tr_admin(tr, lang, "admin.commercial.ads_title", "Kurumsal Reklam & Sponsorluk Envanteri", "Corporate Advertising & Sponsorship Inventory");
    let card_desc = tr_admin(tr, lang, "admin.commercial.ads_desc", "Tüm reklam kuşakları, hedef kurum kırılımları, öncelik puanları ve tıklama dönüşüm oranları.", "All ad slots, target institution breakdowns, priority scores and click conversion rates.");
    let search_ph = tr_admin(tr, lang, "admin.commercial.ad_search_ph", "Sponsor, başlık veya kuşak ara...", "Search sponsor, title or slot...");
    let search_aria = tr_admin(tr, lang, "admin.commercial.ad_search_aria", "Kampanya ara", "Search campaign");
    let filter_all = tr_admin(tr, lang, "admin.status.all", "Tüm Kampanyalar", "All Campaigns");
    let filter_active = tr_admin(tr, lang, "admin.status.active", "Aktif", "Active");
    let filter_inactive = tr_admin(tr, lang, "admin.status.inactive", "Pasif", "Inactive");
    let filter_oem = tr_admin(tr, lang, "admin.commercial.filter_defense_oem", "Savunma OEM", "Defence OEM");
    let filter_aria = tr_admin(tr, lang, "admin.commercial.ad_filter_aria", "Kampanya filtresi", "Campaign filter");
    let new_campaign = tr_admin(tr, lang, "admin.commercial.new_campaign", "Yeni Kampanya", "New Campaign");
    let col_slot = tr_admin(tr, lang, "admin.commercial.col_ad_slot", "Reklam Kuşağı & Boyut", "Ad Slot & Size");
    let col_sponsor = tr_admin(tr, lang, "admin.commercial.col_sponsor", "Sponsor & Kategori", "Sponsor & Category");
    let col_campaign = tr_admin(tr, lang, "admin.commercial.col_campaign", "Kampanya Başlığı & Öncelik", "Campaign Title & Priority");
    let col_targeting = tr_admin(tr, lang, "admin.commercial.col_targeting", "Hedef Kurum & SSB", "Target Institution & SSB");
    let col_impressions = tr_admin(tr, lang, "admin.commercial.col_impressions", "Gösterim / Tıklama", "Impressions / Clicks");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let loading = tr_admin(tr, lang, "admin.commercial.ads_loading", "Reklam kampanyaları yükleniyor...", "Loading ad campaigns...");

    tab_pane(buf, "tab-2", "tab-btn-2", false, |tab| {
        tab.el("div", &[("class", "status-banner shield-banner")], |sb| {
            let trimmed_banner = banner_text.trim();
            let clean_banner = trimmed_banner
                .strip_prefix("Sponsorlu İçerik Rakip OEM Koruması Aktif — ")
                .or_else(|| {
                    trimmed_banner
                        .strip_prefix("Sponsored Content Competitor OEM Protection Active — ")
                })
                .unwrap_or(trimmed_banner)
                .trim();
            sb.el("div", &[("class", "flex items-center gap-2")], |left| {
                icon(left, "fa-shield");
                left.el("span", &[], |s| {
                    s.el("strong", &[], |st| { st.text(&banner_strong); });
                    s.text(" ");
                    s.text(clean_banner);
                });
            });
            sb.el("span", &[("class", "badge-pill success")], |b| {
                b.text(&banner_badge);
            });
        });

        admin_card(
            tab,
            "",
            None,
            "fa-bullhorn",
            &card_title,
            Some(&card_desc),
            |hdr| {
                hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                    input_ctrl(tb, "text", "filter-ad-search", "input-lg", "", &search_ph, &[("aria-label", &search_aria)]);
                    select_ctrl(
                        tb,
                        "filter-ad-status",
                        "input-md",
                        &[("", &filter_all), ("active", &filter_active), ("inactive", &filter_inactive), ("defense_oem", &filter_oem)],
                        &[("aria-label", &filter_aria)],
                    );
                    action_btn_extra(tb, "btn-cmd btn-cmd-primary", "open-ad-drawer", Some("ads:write"), Some("fa-plus"), &new_campaign, &[("data-open-drawer", "drawer-ad")]);
                });
            },
            |card| {
                ops_table(
                    card,
                    "",
                    None,
                    &[
                        (&col_slot, ""),
                        (&col_sponsor, ""),
                        (&col_campaign, ""),
                        (&col_targeting, ""),
                        (&col_impressions, "text-right"),
                        (&col_action, "col-action"),
                    ],
                    "tbody-ads",
                    Some(&loading),
                    Some("pagination-ads"),
                );
            },
        );

        render_ad_drawer(tab, tr, lang);
        render_ad_conflict_modal(tab, tr, lang);
    });
}

fn render_ad_drawer(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let ad_slots = ad_slots_resolved(tr, lang);
    let ad_slots_ref: Vec<(&str, &str)> = ad_slots.iter().map(|(c, l)| (c.as_str(), l.as_str())).collect();

    let drawer_title = tr_admin(tr, lang, "admin.commercial.ad_drawer_title", "Yeni Reklam Kampanyası", "New Ad Campaign");
    let clear_btn = tr_admin(tr, lang, "admin.form.clear", "Temizle", "Clear");
    let quota_check = tr_admin(tr, lang, "admin.commercial.quota_check", "Kota Kontrolü", "Quota Check");
    let quota_ready = tr_admin(tr, lang, "admin.commercial.quota_ready", "Kota kontrolü hazır.", "Quota check ready.");
    let lbl_slot = tr_admin(tr, lang, "admin.commercial.lbl_ad_slot", "Reklam Kuşağı", "Ad Slot");
    let lbl_sponsor_type = tr_admin(tr, lang, "admin.commercial.lbl_sponsor_type", "Sponsor Kategorisi", "Sponsor Category");
    let sp_defense = tr_admin(tr, lang, "admin.commercial.sp_defense_oem", "Savunma Üreticisi (Rakip Korumalı)", "Defence Manufacturer (Competitor Protected)");
    let sp_auxiliary = tr_admin(tr, lang, "admin.commercial.sp_auxiliary", "Yan Hizmet Sağlayıcı (Muaf)", "Auxiliary Service Provider (Exempt)");
    let sp_inhouse = tr_admin(tr, lang, "admin.commercial.sp_inhouse", "MSI Öz Tanıtım / Yedek", "MSI Self-Promo / Fallback");
    let lbl_sponsor_name = tr_admin(tr, lang, "admin.commercial.lbl_sponsor_name", "Sponsor Marka", "Sponsor Brand");
    let ph_sponsor_name = tr_admin(tr, lang, "admin.commercial.ph_sponsor_name", "Örn: ASELSAN, ROKETSAN, STM", "e.g. ASELSAN, ROKETSAN, STM");
    let lbl_dimensions = tr_admin(tr, lang, "admin.commercial.lbl_dimensions", "Görsel Boyutu", "Image Dimensions");
    let lbl_budget = tr_admin(tr, lang, "admin.commercial.lbl_budget_score", "Öncelik Puanı (1-100)", "Priority Score (1-100)");
    let tab_tr = tr_admin(tr, lang, "admin.form.tab_tr", "Türkçe — MSI Dergisi", "Turkish — MSI Review");
    let tab_en = tr_admin(tr, lang, "admin.form.tab_en", "English — MSI TDR", "English — MSI TDR");
    let lbl_title_tr = tr_admin(tr, lang, "admin.commercial.lbl_title_tr", "Kampanya Başlığı (TR)", "Campaign Title (TR)");
    let ph_title_tr = tr_admin(tr, lang, "admin.commercial.ph_title_tr", "Örn: AESA Radar Teknolojilerinde Küresel Liderlik", "e.g. Global Leadership in AESA Radar Technologies");
    let lbl_desc_tr = tr_admin(tr, lang, "admin.commercial.lbl_desc_tr", "Alt Metin (TR)", "Subtitle (TR)");
    let ph_desc_tr = tr_admin(tr, lang, "admin.commercial.ph_desc_tr", "Yeni nesil hava ve deniz platformları için entegre çözümler.", "Integrated solutions for next-gen air and naval platforms.");
    let lbl_title_en = tr_admin(tr, lang, "admin.commercial.lbl_title_en", "Campaign Title (EN)", "Campaign Title (EN)");
    let ph_title_en = tr_admin(tr, lang, "admin.commercial.ph_title_en", "Global Leadership in AESA Radar Systems", "Global Leadership in AESA Radar Systems");
    let lbl_desc_en = tr_admin(tr, lang, "admin.commercial.lbl_desc_en", "Subtitle (EN)", "Subtitle (EN)");
    let ph_desc_en = tr_admin(tr, lang, "admin.commercial.ph_desc_en", "Integrated solutions for next-gen platforms.", "Integrated solutions for next-gen platforms.");
    let lbl_target_url = tr_admin(tr, lang, "admin.commercial.lbl_target_url", "Hedef Bağlantı Adresi", "Target URL");
    let lbl_image = tr_admin(tr, lang, "admin.commercial.lbl_image", "Görsel Dosyası", "Image File");
    let lbl_badge = tr_admin(tr, lang, "admin.commercial.lbl_badge_label", "Yasal Etiket", "Legal Label");
    let lbl_pill_style = tr_admin(tr, lang, "admin.commercial.lbl_pill_style", "Vurgu Rengi", "Accent Color");
    let pill_blue = tr_admin(tr, lang, "admin.commercial.pill_blue", "Kurumsal Mavi", "Corporate Blue");
    let pill_gold = tr_admin(tr, lang, "admin.commercial.pill_gold", "Lansman Altını", "Launch Gold");
    let pill_emerald = tr_admin(tr, lang, "admin.commercial.pill_emerald", "Teknoloji Zümrütü", "Technology Emerald");
    let lbl_target_cats = tr_admin(tr, lang, "admin.commercial.lbl_target_categories", "Hedef Kategoriler", "Target Categories");
    let lbl_institutions = tr_admin(tr, lang, "admin.commercial.lbl_institutions", "Hedef Kamu & Savunma Kurumları", "Target Government & Defence Institutions");
    let lbl_ssb_fields = tr_admin(tr, lang, "admin.commercial.lbl_ssb_fields", "Hedef SSB İlgi Alanları", "Target SSB Interest Areas");
    let chk_fallback = tr_admin(tr, lang, "admin.commercial.chk_fallback", "Varsayılan Yedek Reklam", "Default Fallback Ad");
    let chk_popup = tr_admin(tr, lang, "admin.commercial.chk_popup", "Giriş Tanıtım Penceresi", "Entry Promo Popup");
    let chk_newsletter = tr_admin(tr, lang, "admin.commercial.chk_newsletter", "E-Bülten Sponsorluğu", "Newsletter Sponsorship");
    let chk_active = tr_admin(tr, lang, "admin.commercial.chk_active", "Yayında / Aktif", "Live / Active");
    let lbl_start_date = tr_admin(tr, lang, "admin.commercial.lbl_start_date", "Başlangıç Tarihi", "Start Date");
    let lbl_end_date = tr_admin(tr, lang, "admin.commercial.lbl_end_date", "Bitiş Tarihi", "End Date");
    let lbl_rotation_mode = tr_admin(tr, lang, "admin.commercial.lbl_rotation_mode", "Rotasyon Kipi", "Rotation Mode");
    let rot_weighted = tr_admin(tr, lang, "admin.commercial.rot_weighted", "Ağırlıklı (Öncelik Puanı)", "Weighted (Priority Score)");
    let rot_even = tr_admin(tr, lang, "admin.commercial.rot_even", "Eşit Dağılım (Even)", "Even Rotation");
    let cancel_btn = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");
    let close_lbl = tr_admin(tr, lang, "admin.form.close", "Kapat", "Close");
    let choose_lbl = tr_admin(tr, lang, "admin.form.choose", "Seç", "Choose");
    let save_btn = tr_admin(tr, lang, "admin.commercial.save_campaign", "Kampanyayı Kaydet", "Save Campaign");
    let inst_resolved = institution_tags(tr, lang);
    let inst_refs: Vec<(&str, &str)> = inst_resolved.iter().map(|(c, l)| (*c, l.as_str())).collect();
    let ssb_resolved = ssb_tags(tr, lang);
    let ssb_refs: Vec<(&str, &str)> = ssb_resolved.iter().map(|(c, l)| (*c, l.as_str())).collect();

    admin_drawer(
        buf,
        "drawer-ad",
        "ad-form-heading",
        &drawer_title,
        &close_lbl,
        |act| {
            action_btn(act, "btn-cmd btn-cmd-sm", "reset-ad-form", None, Some("fa-eraser"), &clear_btn);
            action_btn(act, "btn-cmd btn-cmd-sm btn-cmd-warning", "check-ad-conflict", None, Some("fa-exclamation-triangle"), &quota_check);
        },
        |body| {
            body.el("div", &[("id", "ad-conflict-banner"), ("class", "status-banner hidden")], |cb| {
                icon(cb, "fa-check-circle");
                cb.el("span", &[("id", "ad-conflict-text")], |t| {
                    t.text(&quota_ready);
                });
            });
            body.el_with("form", |a| { a.attr("id", "form-ad-campaign").bool_attr("novalidate", true); }, |f| {
                f.void_el("input", &[("type", "hidden"), ("id", "ad-id"), ("value", "0")]);
                f.el("div", &[("class", "form-row-2")], |r| {
                    form_group(r, Some("ad-slot-code"), &lbl_slot, |g| select_ctrl(g, "ad-slot-code", "", &ad_slots_ref, &[]));
                    form_group(r, Some("ad-sponsor-type"), &lbl_sponsor_type, |g| {
                        select_ctrl(
                            g,
                            "ad-sponsor-type",
                            "",
                            &[
                                ("defense_oem", &sp_defense),
                                ("auxiliary_service", &sp_auxiliary),
                                ("in_house", &sp_inhouse),
                            ],
                            &[],
                        );
                    });
                });
                f.el("div", &[("class", "form-row-3")], |r| {
                    form_group(r, Some("ad-sponsor-name"), &lbl_sponsor_name, |g| input_ctrl(g, "text", "ad-sponsor-name", "", "", &ph_sponsor_name, &[]));
                    form_group(r, Some("ad-dimensions"), &lbl_dimensions, |g| input_ctrl(g, "text", "ad-dimensions", "", "970x90", "", &[]));
                    form_group(r, Some("ad-budget-score"), &lbl_budget, |g| input_ctrl(g, "number", "ad-budget-score", "", "85", "", &[("min", "1"), ("max", "100")]));
                });
                lang_tabs(
                    f,
                    &tab_tr,
                    &tab_en,
                    |tr_tab| {
                        form_group(tr_tab, Some("ad-title-tr"), &lbl_title_tr, |g| input_ctrl(g, "text", "ad-title-tr", "", "", &ph_title_tr, &[]));
                        form_group(tr_tab, Some("ad-desc-tr"), &lbl_desc_tr, |g| input_ctrl(g, "text", "ad-desc-tr", "", "", &ph_desc_tr, &[]));
                    },
                    |en_tab| {
                        form_group(en_tab, Some("ad-title-en"), &lbl_title_en, |g| input_ctrl(g, "text", "ad-title-en", "", "", &ph_title_en, &[]));
                        form_group(en_tab, Some("ad-desc-en"), &lbl_desc_en, |g| input_ctrl(g, "text", "ad-desc-en", "", "", &ph_desc_en, &[]));
                    },
                );
                f.el("div", &[("class", "form-row-2")], |r| {
                    form_group(r, Some("ad-target-url"), &lbl_target_url, |g| input_ctrl(g, "url", "ad-target-url", "", "https://www.msidergisi.com", "", &[]));
                    form_group(r, Some("ad-image-url"), &lbl_image, |g| upload_field(g, "ad-image-url", "", "images/uploads/banner.webp", "ad-image-file", ".webp,.png,.jpg,.jpeg,.gif", "trigger-ad-upload", "fa-upload", &choose_lbl));
                });
                f.el("div", &[("class", "form-row-3")], |r| {
                    form_group(r, Some("ad-badge-label"), &lbl_badge, |g| input_ctrl(g, "text", "ad-badge-label", "", "SPONSORLU REKLAM ALANI", "", &[]));
                    form_group(r, Some("ad-pill-style"), &lbl_pill_style, |g| select_ctrl(g, "ad-pill-style", "", &[("blue", &pill_blue), ("gold", &pill_gold), ("emerald", &pill_emerald)], &[]));
                    form_group(r, Some("ad-target-categories"), &lbl_target_cats, |g| input_ctrl(g, "text", "ad-target-categories", "", "ozel-haberler, kapak-dosyalari, soylesiler", "", &[]));
                });
                f.el("div", &[("class", "form-row-3")], |r| {
                    form_group(r, Some("ad-start-date"), &lbl_start_date, |g| input_ctrl(g, "date", "ad-start-date", "", "", "", &[("name", "start_date")]));
                    form_group(r, Some("ad-end-date"), &lbl_end_date, |g| input_ctrl(g, "date", "ad-end-date", "", "", "", &[("name", "end_date")]));
                    form_group(r, Some("ad-rotation-mode"), &lbl_rotation_mode, |g| {
                        select_ctrl(g, "ad-rotation-mode", "", &[("weighted", &rot_weighted), ("even", &rot_even)], &[("name", "rotation_mode")]);
                    });
                });
                form_group(f, None, &lbl_institutions, |g| check_matrix(g, "ad-institutions-matrix", "ad_inst", &inst_refs, &["MSB", "SSB", "K.K.K.lığı", "Hv.K.K.lığı", "Sektör Kuruluşları"]));
                form_group(f, None, &lbl_ssb_fields, |g| check_matrix(g, "ad-ssb-matrix", "ad_ssb", &ssb_refs, &["hava-araclari", "radarlar-ve-sensorler"]));
                f.el("div", &[("class", "flex gap-3 flex-wrap")], |fl| {
                    check_pill(fl, Some("ad-is-fallback"), None, None, false, "", &chk_fallback);
                    check_pill(fl, Some("ad-is-popup"), None, None, false, "", &chk_popup);
                    check_pill(fl, Some("ad-is-newsletter"), None, None, false, "", &chk_newsletter);
                    check_pill(fl, Some("ad-is-active"), None, None, true, "", &chk_active);
                });
                f.el("div", &[("id", "ad-custom-fields"), ("data-custom-field-values", "ad"), ("class", "custom-fields-slot mt-3")], |_| {});
            });
        },
        |ft| {
            action_btn(ft, "btn-cmd", "close-drawer", None, None, &cancel_btn);
            ft.el(
                "button",
                &[("type", "submit"), ("form", "form-ad-campaign"), ("class", "btn-cmd btn-cmd-primary"), ("data-required-perm", "ads:write")],
                |b| {
                    icon(b, "fa-save");
                    b.raw(" ");
                    b.text(&save_btn);
                },
            );
        },
    );
}

fn render_ad_conflict_modal(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let title = tr_admin(tr, lang, "admin.commercial.conflict_modal_title", "Reklam Kuşağı Kota Çakışması", "Ad Slot Quota Conflict");
    let close_lbl = tr_admin(tr, lang, "admin.form.close", "Kapat", "Close");
    let cancel_lbl = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");
    let override_lbl = tr_admin(tr, lang, "admin.commercial.conflict_override_btn", "Kotayı Aşarak Kaydet (Override)", "Save with Override");

    buf.el(
        "div",
        &[
            ("id", "ad-conflict-modal"),
            ("class", "modal-overlay hidden"),
            ("role", "dialog"),
            ("aria-modal", "true"),
            ("aria-hidden", "true"),
            ("aria-labelledby", "ad-conflict-modal-title"),
        ],
        |ov| {
            ov.el("div", &[("class", "modal-content admin-card modal-confirm-card")], |mc| {
                mc.el("div", &[("class", "panel-header")], |hdr| {
                    hdr.el("div", &[("class", "panel-title"), ("id", "ad-conflict-modal-title")], |t| {
                        icon(t, "fa-exclamation-triangle");
                        t.raw(" ");
                        t.text(&title);
                    });
                    hdr.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "btn-icon"),
                            ("id", "ad-conflict-close"),
                            ("data-admin-action", "close-ad-conflict-modal"),
                            ("aria-label", &close_lbl),
                        ],
                        |b| icon(b, "fa-times"),
                    );
                });
                mc.el("div", &[("class", "panel-body")], |bd| {
                    bd.el("p", &[("id", "ad-conflict-modal-msg"), ("class", "text-sm text-secondary mb-3")], |_| {});
                    bd.el("ul", &[("id", "ad-conflict-modal-list"), ("class", "conflict-campaign-list text-xs text-mono")], |_| {});
                });
                mc.el("div", &[("class", "modal-footer")], |ft| {
                    action_btn(ft, "btn-cmd", "close-ad-conflict-modal", None, None, &cancel_lbl);
                    ft.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("id", "ad-conflict-override-btn"),
                            ("class", "btn-cmd btn-cmd-warning"),
                            ("data-admin-action", "confirm-ad-override"),
                        ],
                        |b| {
                            icon(b, "fa-check");
                            b.raw(" ");
                            b.text(&override_lbl);
                        },
                    );
                });
            });
        },
    );
}

fn render_tab_commerce(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let subtab_revenue = tr_admin(tr, lang, "admin.commercial.subtab_revenue", "Gelir Özeti & Trend", "Revenue Summary & Trend");
    let subtab_plans = tr_admin(tr, lang, "admin.commercial.subtab_plans", "Abonelik Planları", "Subscription Plans");
    let subtab_products = tr_admin(tr, lang, "admin.commercial.subtab_products", "Mağaza Koleksiyonu", "Store Collection");
    let subtab_invoices = tr_admin(tr, lang, "admin.commercial.subtab_invoices", "Siparişler & Faturalar", "Orders & Invoices");
    let kpi_total = tr_admin(tr, lang, "admin.commercial.kpi_total_collection", "TOPLAM TAHSİLAT", "TOTAL COLLECTION");
    let kpi_total_sub = tr_admin(tr, lang, "admin.commercial.kpi_total_collection_sub", "Seçili dönem net gelir", "Selected period net revenue");
    let kpi_invoices = tr_admin(tr, lang, "admin.commercial.kpi_approved_invoices", "ONAYLI FATURA", "APPROVED INVOICES");
    let kpi_invoices_sub = tr_admin(tr, lang, "admin.commercial.kpi_approved_invoices_sub", "Ödemesi tamamlanan işlem", "Completed payment transactions");
    let kpi_subscription = tr_admin(tr, lang, "admin.commercial.revenue_subscription", "ABONELİK GELİRİ", "SUBSCRIPTION REVENUE");
    let kpi_subscription_sub = tr_admin(tr, lang, "admin.commercial.kpi_subscription_sub", "Bireysel & kurumsal paketler", "Individual & corporate packages");
    let kpi_store = tr_admin(tr, lang, "admin.commercial.kpi_store_sales", "MAĞAZA & TEKİL SATIŞ", "STORE & SINGLE SALES");
    let kpi_store_sub = tr_admin(tr, lang, "admin.commercial.kpi_store_sales_sub", "Kitap, rapor ve fotoğraf lisansı", "Books, reports and photo licenses");
    let chart_title = tr_admin(tr, lang, "admin.commercial.chart_daily_revenue", "Günlük Tahsilat Eğilimi", "Daily Revenue Trend");
    let chart_desc = tr_admin(tr, lang, "admin.commercial.chart_daily_revenue_desc", "PayTR ve kurumsal havale onaylı günlük gelir akışı.", "Daily revenue flow confirmed via PayTR and corporate wire transfers.");
    let date_from_aria = tr_admin(tr, lang, "admin.commercial.date_from_aria", "Başlangıç tarihi", "Start date");
    let date_to_aria = tr_admin(tr, lang, "admin.commercial.date_to_aria", "Bitiş tarihi", "End date");
    let refresh_btn = tr_admin(tr, lang, "admin.commercial.refresh", "Güncelle", "Refresh");
    let empty_state = tr_admin(tr, lang, "admin.commercial.revenue_empty", "Seçilen tarih aralığında tahsil edilmiş ödeme kaydı bulunmuyor.", "No collected payment records found in the selected date range.");
    let breakdown_title = tr_admin(tr, lang, "admin.commercial.breakdown_title", "Gelir Kalemi Dağılımı", "Revenue Item Breakdown");
    let breakdown_desc = tr_admin(tr, lang, "admin.commercial.breakdown_desc", "Ürün ve abonelik türlerine göre paylar.", "Shares by product and subscription type.");
    let col_revenue_item = tr_admin(tr, lang, "admin.commercial.col_revenue_item", "Gelir Kalemi", "Revenue Item");
    let col_amount = tr_admin(tr, lang, "admin.commercial.col_amount", "Tutar", "Amount");
    let col_share = tr_admin(tr, lang, "admin.commercial.col_share", "Pay", "Share");
    let data_loading = tr_admin(tr, lang, "admin.form.loading", "Veri yükleniyor...", "Loading data...");
    let plan_form_title = tr_admin(tr, lang, "admin.commercial.plan_form_title", "Abonelik Planı Ekle / Güncelle", "Add / Update Subscription Plan");
    let lbl_plan_code = tr_admin(tr, lang, "admin.commercial.lbl_plan_code", "Plan Kodu", "Plan Code");
    let lbl_plan_price = tr_admin(tr, lang, "admin.commercial.lbl_plan_price", "Yıllık Fiyat (₺)", "Annual Price (₺)");
    let lbl_plan_sort = tr_admin(tr, lang, "admin.commercial.lbl_plan_sort", "Görüntüleme Sırası", "Display Order");
    let lbl_plan_name_tr = tr_admin(tr, lang, "admin.commercial.lbl_plan_name_tr", "Plan Adı (TR)", "Plan Name (TR)");
    let ph_plan_name_tr = tr_admin(tr, lang, "admin.commercial.ph_plan_name_tr", "Basılı + Dijital Tam Erişim", "Print + Digital Full Access");
    let lbl_plan_name_en = tr_admin(tr, lang, "admin.commercial.lbl_plan_name_en", "Plan Name (EN)", "Plan Name (EN)");
    let ph_plan_name_en = tr_admin(tr, lang, "admin.commercial.ph_plan_name_en", "Print + Digital Full Access", "Print + Digital Full Access");
    let lbl_period_tr = tr_admin(tr, lang, "admin.commercial.lbl_period_tr", "Dönem Etiketi (TR)", "Period Label (TR)");
    let ph_period_tr = tr_admin(tr, lang, "admin.commercial.ph_period_tr", "Yıllık", "Annual");
    let lbl_period_en = tr_admin(tr, lang, "admin.commercial.lbl_period_en", "Period Label (EN)", "Period Label (EN)");
    let ph_period_en = tr_admin(tr, lang, "admin.commercial.ph_period_en", "Annual", "Annual");
    let lbl_features_tr = tr_admin(tr, lang, "admin.commercial.lbl_features_tr", "Kapsam Maddeleri (TR — Satır başı)", "Feature Items (TR — One per line)");
    let lbl_features_en = tr_admin(tr, lang, "admin.commercial.lbl_features_en", "Features (EN — One per line)", "Features (EN — One per line)");
    let chk_featured_plan = tr_admin(tr, lang, "admin.commercial.chk_featured_plan", "Önerilen / Öne Çıkan Plan", "Recommended / Featured Plan");
    let save_plan_btn = tr_admin(tr, lang, "admin.commercial.save_plan", "Planı Kaydet", "Save Plan");
    let clear_btn = tr_admin(tr, lang, "admin.form.clear", "Temizle", "Clear");
    let plans_card_title = tr_admin(tr, lang, "admin.commercial.plans_card_title", "Aktif Abonelik Paketleri", "Active Subscription Packages");
    let plans_card_desc = tr_admin(tr, lang, "admin.commercial.plans_card_desc", "Okuyucu abonelik sayfasında sunulan bireysel ve kurumsal katmanlar.", "Individual and corporate tiers offered on the reader subscription page.");
    let col_plan_code = tr_admin(tr, lang, "admin.commercial.col_plan_code", "Plan Kodu", "Plan Code");
    let col_plan_name = tr_admin(tr, lang, "admin.commercial.col_plan_name", "Paket Adı (TR / EN)", "Package Name (TR / EN)");
    let col_price = tr_admin(tr, lang, "admin.commercial.col_price", "Fiyat", "Price");
    let col_period_scope = tr_admin(tr, lang, "admin.commercial.col_period_scope", "Dönem & Kapsam", "Period & Scope");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let plans_loading = tr_admin(tr, lang, "admin.commercial.plans_loading", "Planlar yükleniyor...", "Loading plans...");
    let product_form_title = tr_admin(tr, lang, "admin.commercial.product_form_title", "Mağaza Ürünü Ekle / Güncelle", "Add / Update Store Product");
    let lbl_sku = tr_admin(tr, lang, "admin.commercial.lbl_sku", "Stok Kodu (SKU)", "Stock Code (SKU)");
    let ph_sku = tr_admin(tr, lang, "admin.commercial.ph_sku", "KITAP-MSI-2026", "BOOK-MSI-2026");
    let lbl_product_type = tr_admin(tr, lang, "admin.commercial.lbl_product_type", "Ürün Türü", "Product Type");
    let pt_book = tr_admin(tr, lang, "admin.commercial.pt_book", "Prestij Kitabı", "Prestige Book");
    let pt_report = tr_admin(tr, lang, "admin.commercial.pt_report", "Stratejik Rapor", "Strategic Report");
    let pt_photo = tr_admin(tr, lang, "admin.commercial.pt_photo_license", "Fotoğraf Lisansı", "Photo License");
    let pt_single = tr_admin(tr, lang, "admin.commercial.pt_single_article", "Tekil Dosya", "Single Dossier");
    let lbl_product_price = tr_admin(tr, lang, "admin.commercial.lbl_product_price", "Satış Fiyatı (₺)", "Sale Price (₺)");
    let lbl_product_title_tr = tr_admin(tr, lang, "admin.commercial.lbl_product_title_tr", "Ürün Başlığı (TR)", "Product Title (TR)");
    let ph_product_title_tr = tr_admin(tr, lang, "admin.commercial.ph_product_title_tr", "Ürün başlığı...", "Product title...");
    let lbl_product_title_en = tr_admin(tr, lang, "admin.commercial.lbl_product_title_en", "Product Title (EN)", "Product Title (EN)");
    let ph_product_title_en = tr_admin(tr, lang, "admin.commercial.ph_product_title_en", "Product title...", "Product title...");
    let lbl_author = tr_admin(tr, lang, "admin.commercial.lbl_author", "Yazar / Kaynak", "Author / Source");
    let ph_author = tr_admin(tr, lang, "admin.commercial.ph_product_author", "UMSA Prodüksiyon", "UMSA Production");
    let lbl_resolution = tr_admin(tr, lang, "admin.commercial.lbl_resolution", "Sayfa / Çözünürlük", "Pages / Resolution");
    let ph_res = tr_admin(tr, lang, "admin.commercial.ph_product_res", "320 Sayfa / 4K TIFF", "320 Pages / 4K TIFF");
    let lbl_cover = tr_admin(tr, lang, "admin.form.cover_image", "Kapak Görseli", "Cover Image");
    let lbl_desc_tr = tr_admin(tr, lang, "admin.commercial.lbl_desc_tr", "Açıklama (TR)", "Description (TR)");
    let ph_desc_tr = tr_admin(tr, lang, "admin.commercial.ph_product_desc_tr", "Kısa ürün tanıtımı...", "Short product description...");
    let lbl_desc_en = tr_admin(tr, lang, "admin.commercial.lbl_desc_en", "Description (EN)", "Description (EN)");
    let ph_desc_en = tr_admin(tr, lang, "admin.commercial.ph_product_desc_en", "Short description...", "Short description...");
    let chk_available = tr_admin(tr, lang, "admin.commercial.chk_available", "Satışa Açık", "Available for Sale");
    let save_product_btn = tr_admin(tr, lang, "admin.commercial.save_product", "Ürünü Kaydet", "Save Product");
    let products_card_title = tr_admin(tr, lang, "admin.commercial.products_card_title", "Mağaza Koleksiyonu & Lisanslı Eserler", "Store Collection & Licensed Works");
    let products_card_desc = tr_admin(tr, lang, "admin.commercial.products_card_desc", "Prestij kitapları, düşünce kuruluşu raporları ve yüksek çözünürlüklü havacılık fotoğraf lisansları.", "Prestige books, think tank reports and high-resolution aviation photo licenses.");
    let col_sku = tr_admin(tr, lang, "admin.commercial.col_sku", "Stok Kodu", "Stock Code");
    let col_category = tr_admin(tr, lang, "admin.commercial.col_category", "Kategori", "Category");
    let col_work_source = tr_admin(tr, lang, "admin.commercial.col_work_source", "Eser & Kaynak", "Work & Source");
    let col_unit_price = tr_admin(tr, lang, "admin.commercial.col_unit_price", "Birim Fiyat", "Unit Price");
    let products_loading = tr_admin(tr, lang, "admin.commercial.products_loading", "Ürünler yükleniyor...", "Loading products...");
    let invoices_card_title = tr_admin(tr, lang, "admin.commercial.invoices_card_title", "Siparişler & Faturalar", "Orders & Invoices");
    let invoices_card_desc = tr_admin(tr, lang, "admin.commercial.invoices_card_desc", "Tüm abonelik ve mağaza siparişlerinin ödeme durumları.", "Payment status of all subscription and store orders.");
    let col_invoice_date = tr_admin(tr, lang, "admin.commercial.col_invoice_date", "Fatura No & Tarih", "Invoice No & Date");
    let col_customer = tr_admin(tr, lang, "admin.commercial.col_customer", "Müşteri", "Customer");
    let col_item = tr_admin(tr, lang, "admin.commercial.col_item", "Satın Alınan Kalem", "Purchased Item");
    let col_payment_status = tr_admin(tr, lang, "admin.commercial.col_payment_status", "Ödeme Durumu", "Payment Status");
    let invoices_loading = tr_admin(tr, lang, "admin.commercial.invoices_loading", "Faturalar yükleniyor...", "Loading invoices...");

    tab_pane(buf, "tab-14", "tab-btn-14", false, |tab| {
        subtab_bar(tab, "tab14", &[
            ("tab14-revenue", &subtab_revenue, Some("revenue:read")),
            ("tab14-plans", &subtab_plans, Some("subscriptions:read|commerce:read")),
            ("tab14-products", &subtab_products, Some("commerce:read")),
            ("tab14-invoices", &subtab_invoices, Some("invoices:read|commerce:read")),
        ]);

        subtab_pane(tab, "tab14-revenue", true, Some("revenue:read"), |pane| {
            pane.el("div", &[("class", "kpi-grid mb-4")], |kg| {
                kpi_card_toned(kg, "tone-success", Some("fa-try"), None, &kpi_total, "kpi-revenue-total", "₺0,00", "", None, Some(&kpi_total_sub));
                kpi_card_toned(kg, "tone-info", Some("fa-file-text-o"), None, &kpi_invoices, "kpi-revenue-count", "0", "", None, Some(&kpi_invoices_sub));
                kpi_card_toned(kg, "tone-warning", Some("fa-star"), None, &kpi_subscription, "kpi-revenue-subscription", "₺0,00", "", None, Some(&kpi_subscription_sub));
                kpi_card_toned(kg, "tone-accent", Some("fa-cube"), None, &kpi_store, "kpi-revenue-product", "₺0,00", "", None, Some(&kpi_store_sub));
            });

            pane.el("div", &[("class", "workspace-grid media-seo-grid")], |grid| {
                admin_card(
                    grid,
                    "",
                    None,
                    "fa-area-chart",
                    &chart_title,
                    Some(&chart_desc),
                    |hdr| {
                        hdr.el("div", &[("class", "date-range-bar")], |tb| {
                            input_ctrl(tb, "date", "revenue-date-from", "input-date", "", "", &[("aria-label", &date_from_aria)]);
                            tb.el("span", &[("class", "text-muted")], |s| { s.text("→"); });
                            input_ctrl(tb, "date", "revenue-date-to", "input-date", "", "", &[("aria-label", &date_to_aria)]);
                            action_btn(tb, "btn-cmd btn-cmd-sm", "load-revenue-summary", None, Some("fa-refresh"), &refresh_btn);
                        });
                    },
                    |card| {
                        card.el("div", &[("class", "panel-body chart-container-md"), ("id", "revenue-chart-wrap")], |bd| {
                            bd.el("canvas", &[("id", "chart-revenue-daily")], |_| {});
                            bd.el("div", &[("id", "revenue-chart-empty"), ("class", "empty-state hidden")], |em| {
                                icon(em, "fa-line-chart");
                                em.el("p", &[], |p| { p.text(&empty_state); });
                            });
                        });
                    },
                );

                admin_card(grid, "", None, "fa-list", &breakdown_title, Some(&breakdown_desc), |_| {}, |card| {
                    ops_table(card, "", None, &[(&col_revenue_item, ""), (&col_amount, "text-right"), (&col_share, "text-right")], "tbody-revenue-breakdown", Some(&data_loading), None);
                });
            });
        });

        subtab_pane(tab, "tab14-plans", false, Some("subscriptions:read|commerce:read"), |pane| {
            details_box(pane, "admin-card mb-4", false, Some("subscriptions:write|commerce:manage"), &plan_form_title, |d| {
                d.el_with("form", |a| { a.attr("id", "form-sub-plan").attr("data-required-perm", "subscriptions:write|commerce:manage").bool_attr("novalidate", true); }, |f| {
                    f.el("div", &[("class", "form-row-3")], |r| {
                        form_group(r, Some("sp-code"), &lbl_plan_code, |g| input_ctrl(g, "text", "sp-code", "", "", "kurumsal", &[]));
                        form_group(r, Some("sp-price"), &lbl_plan_price, |g| input_ctrl(g, "number", "sp-price", "", "", "2400", &[("step", "0.5")]));
                        form_group(r, Some("sp-sort"), &lbl_plan_sort, |g| input_ctrl(g, "number", "sp-sort", "", "10", "", &[]));
                    });
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("sp-name-tr"), &lbl_plan_name_tr, |g| input_ctrl(g, "text", "sp-name-tr", "", "", &ph_plan_name_tr, &[]));
                        form_group(r, Some("sp-name-en"), &lbl_plan_name_en, |g| input_ctrl(g, "text", "sp-name-en", "", "", &ph_plan_name_en, &[]));
                    });
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("sp-period-tr"), &lbl_period_tr, |g| input_ctrl(g, "text", "sp-period-tr", "", &ph_period_tr, "", &[]));
                        form_group(r, Some("sp-period-en"), &lbl_period_en, |g| input_ctrl(g, "text", "sp-period-en", "", &ph_period_en, "", &[]));
                    });
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("sp-features-tr"), &lbl_features_tr, |g| textarea_ctrl(g, "sp-features-tr", "textarea-sm", "12 Basılı Sayı\nTam Dijital Arşiv", &[]));
                        form_group(r, Some("sp-features-en"), &lbl_features_en, |g| textarea_ctrl(g, "sp-features-en", "textarea-sm", "12 Print Issues\nFull Digital Archive", &[]));
                    });
                    f.el("div", &[("class", "flex justify-between items-center flex-wrap gap-2")], |fl| {
                        check_pill(fl, Some("sp-featured"), None, None, false, "", &chk_featured_plan);
                        fl.el("div", &[("class", "flex gap-2 flex-wrap")], |btns| {
                            btns.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-primary btn-cmd-sm")], |b| {
                                icon(b, "fa-floppy-o");
                                b.raw(" ");
                                b.text(&save_plan_btn);
                            });
                            action_btn(btns, "btn-cmd btn-cmd-sm", "reset-sub-plan-form", None, Some("fa-eraser"), &clear_btn);
                        });
                    });
                });
            });
            admin_card(pane, "", None, "fa-star", &plans_card_title, Some(&plans_card_desc), |_| {}, |card| {
                ops_table(card, "", None, &[(&col_plan_code, ""), (&col_plan_name, ""), (&col_price, "text-right"), (&col_period_scope, ""), (&col_action, "col-action")], "tbody-sub-plans", Some(&plans_loading), None);
            });
        });

        subtab_pane(tab, "tab14-products", false, Some("commerce:read"), |pane| {
            details_box(pane, "admin-card mb-4", false, Some("commerce:write|commerce:manage"), &product_form_title, |d| {
                d.el_with("form", |a| { a.attr("id", "form-commerce-product").attr("data-required-perm", "commerce:write|commerce:manage").bool_attr("novalidate", true); }, |f| {
                    f.void_el("input", &[("type", "hidden"), ("id", "cp-id"), ("value", "0")]);
                    f.el("div", &[("class", "form-row-3")], |r| {
                        form_group(r, Some("cp-sku"), &lbl_sku, |g| input_ctrl(g, "text", "cp-sku", "", "", &ph_sku, &[]));
                        form_group(r, Some("cp-type"), &lbl_product_type, |g| {
                            select_ctrl(g, "cp-type", "", &[("book", &pt_book), ("report", &pt_report), ("photo_license", &pt_photo), ("single_article", &pt_single)], &[]);
                        });
                        form_group(r, Some("cp-price"), &lbl_product_price, |g| input_ctrl(g, "number", "cp-price", "", "", "850", &[("step", "0.5")]));
                    });
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("cp-title-tr"), &lbl_product_title_tr, |g| input_ctrl(g, "text", "cp-title-tr", "", "", &ph_product_title_tr, &[]));
                        form_group(r, Some("cp-title-en"), &lbl_product_title_en, |g| input_ctrl(g, "text", "cp-title-en", "", "", &ph_product_title_en, &[]));
                    });
                    f.el("div", &[("class", "form-row-3")], |r| {
                        form_group(r, Some("cp-author"), &lbl_author, |g| input_ctrl(g, "text", "cp-author", "", "", &ph_author, &[]));
                        form_group(r, Some("cp-res"), &lbl_resolution, |g| input_ctrl(g, "text", "cp-res", "", "", &ph_res, &[]));
                        form_group(r, Some("cp-cover"), &lbl_cover, |g| input_ctrl(g, "text", "cp-cover", "", "", "images/msi/dergi-286.webp", &[]));
                    });
                    f.el("div", &[("class", "form-row-2")], |r| {
                        form_group(r, Some("cp-desc-tr"), &lbl_desc_tr, |g| input_ctrl(g, "text", "cp-desc-tr", "", "", &ph_desc_tr, &[]));
                        form_group(r, Some("cp-desc-en"), &lbl_desc_en, |g| input_ctrl(g, "text", "cp-desc-en", "", "", &ph_desc_en, &[]));
                    });
                    f.el("div", &[("class", "flex justify-between items-center flex-wrap gap-2")], |fl| {
                        check_pill(fl, Some("cp-available"), None, None, true, "", &chk_available);
                        fl.el("div", &[("class", "flex gap-2 flex-wrap")], |btns| {
                            btns.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-primary btn-cmd-sm")], |b| {
                                icon(b, "fa-floppy-o");
                                b.raw(" ");
                                b.text(&save_product_btn);
                            });
                            action_btn(btns, "btn-cmd btn-cmd-sm", "reset-commerce-product-form", None, Some("fa-eraser"), &clear_btn);
                        });
                    });
                });
            });
            admin_card(pane, "", None, "fa-cube", &products_card_title, Some(&products_card_desc), |_| {}, |card| {
                ops_table(card, "", None, &[(&col_sku, ""), (&col_category, ""), (&col_work_source, ""), (&col_unit_price, "text-right"), (&col_action, "col-action")], "tbody-commerce-products", Some(&products_loading), None);
            });
        });

        subtab_pane(tab, "tab14-invoices", false, Some("invoices:read|commerce:read"), |pane| {
            admin_card(pane, "", None, "fa-file-text-o", &invoices_card_title, Some(&invoices_card_desc), |_| {}, |card| {
                ops_table(card, "", None, &[(&col_invoice_date, ""), (&col_customer, ""), (&col_item, ""), (&col_amount, "text-right"), (&col_payment_status, ""), (&col_action, "col-action")], "tbody-invoices", Some(&invoices_loading), Some("pagination-invoices"));
            });
        });
    });
}

fn render_tab_corporate(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let card_title = tr_admin(tr, lang, "admin.commercial.corporate_title", "Kamu & Kurumsal Erişim Başvuruları", "Government & Corporate Access Requests");
    let card_desc = tr_admin(tr, lang, "admin.commercial.corporate_subtitle", ".gov.tr ve .tsk.tr uzantılı kurumsal e-postalar otomatik doğrulanır; savunma sanayii firmalarının ve akademik kurumların başvurularını buradan onaylayın.", ".gov.tr and .tsk.tr corporate emails are automatically verified; approve defence industry and academic institution requests here.");
    let filter_all = tr_admin(tr, lang, "admin.status.all_requests", "Tüm Başvurular", "All Requests");
    let filter_pending = tr_admin(tr, lang, "admin.status.pending", "Bekleyenler", "Pending");
    let filter_approved = tr_admin(tr, lang, "admin.status.approved", "Onaylananlar", "Approved");
    let filter_rejected = tr_admin(tr, lang, "admin.status.rejected", "Reddedilenler", "Rejected");
    let filter_aria = tr_admin(tr, lang, "admin.commercial.corp_filter_aria", "Başvuru durumu filtresi", "Request status filter");
    let col_request_date = tr_admin(tr, lang, "admin.commercial.col_request_date", "Başvuru & Tarih", "Request & Date");
    let col_applicant = tr_admin(tr, lang, "admin.commercial.col_applicant", "Başvuran & E-Posta", "Applicant & Email");
    let col_institution = tr_admin(tr, lang, "admin.commercial.col_institution", "Kurum & Kategori", "Institution & Category");
    let col_note = tr_admin(tr, lang, "admin.commercial.col_note", "Başvuru Notu", "Request Note");
    let col_status = tr_admin(tr, lang, "admin.commercial.col_status", "Durum", "Status");
    let col_decision = tr_admin(tr, lang, "admin.commercial.col_decision", "Karar", "Decision");
    let loading = tr_admin(tr, lang, "admin.commercial.corp_loading", "Kurumsal başvurular yükleniyor...", "Loading corporate requests...");

    tab_pane(buf, "tab-4", "tab-btn-4", false, |tab| {
        admin_card(
            tab,
            "",
            None,
            "fa-building-o",
            &card_title,
            Some(&card_desc),
            |hdr| {
                select_ctrl(
                    hdr,
                    "filter-corp-status",
                    "input-md",
                    &[("", &filter_all), ("pending", &filter_pending), ("approved", &filter_approved), ("rejected", &filter_rejected)],
                    &[("aria-label", &filter_aria)],
                );
            },
            |card| {
                ops_table(
                    card,
                    "",
                    None,
                    &[
                        (&col_request_date, ""),
                        (&col_applicant, ""),
                        (&col_institution, ""),
                        (&col_note, ""),
                        (&col_status, ""),
                        (&col_decision, "col-action"),
                    ],
                    "tbody-corporate-requests",
                    Some(&loading),
                    None,
                );
            },
        );
    });
}
