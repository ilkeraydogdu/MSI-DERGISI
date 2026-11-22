//! Editorial Workflow Views: `#tab-16` Pending Approvals & Review Modal,
//! `#tab-12` Bulk Operations & Version History, `#tab-6` Targeted B2B Newsletters & Email Groups.

use super::ui::{
    action_btn, admin_card, admin_drawer, check_pill, form_group, icon, input_ctrl, lang_tabs,
    ops_table, select_ctrl, subtab_bar, subtab_pane, tab_pane, textarea_ctrl,
};
use super::{institution_tags, ssb_tags, tr_admin};
use crate::engine::design::html_dsl::HtmlBuf;
use crate::engine::translation_engine::TranslationCache;

pub fn render(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    render_tab_approvals(buf, tr, lang);
    render_tab_bulk_versions(buf, tr, lang);
    render_tab_newsletters(buf, tr, lang);
}

fn render_tab_approvals(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let card_title = tr_admin(tr, lang, "admin.workflow.approvals_title", "Editöryal Onay Kuyruğu", "Editorial Approval Queue");
    let card_desc = tr_admin(tr, lang, "admin.workflow.approvals_desc", "Yayın öncesi son kontrol bekleyen haber, söyleşi ve analiz dosyalarını inceleyip karara bağlayın.", "Review and decide on news, interviews and analysis dossiers awaiting pre-publication sign-off.");
    let refresh_btn = tr_admin(tr, lang, "admin.workflow.refresh_list", "Listeyi Yenile", "Refresh List");
    let col_title = tr_admin(tr, lang, "admin.workflow.col_content_title", "İçerik Başlığı & Bağlantı", "Content Title & Link");
    let col_author = tr_admin(tr, lang, "admin.workflow.col_author", "Yazar", "Author");
    let col_type = tr_admin(tr, lang, "admin.workflow.col_content_type", "İçerik Türü", "Content Type");
    let col_submitted = tr_admin(tr, lang, "admin.workflow.col_submitted_at", "Gönderim Zamanı", "Submitted At");
    let col_status = tr_admin(tr, lang, "admin.workflow.col_status", "Durum", "Status");
    let col_decision = tr_admin(tr, lang, "admin.workflow.col_decision", "Yayın Kararı", "Publication Decision");
    let loading = tr_admin(tr, lang, "admin.workflow.approvals_loading", "Onay bekleyen içerikler kontrol ediliyor...", "Checking pending approvals...");

    let modal_title = tr_admin(tr, lang, "admin.workflow.approval_modal_title", "Editöryal Onay Kararı", "Editorial Approval Decision");
    let close_lbl = tr_admin(tr, lang, "admin.form.close", "Kapat", "Close");
    let note_lbl = tr_admin(tr, lang, "admin.workflow.approval_note_lbl", "Editöryal Karar Notu (İsteğe Bağlı)", "Editorial Decision Note (Optional)");
    let note_ph = tr_admin(tr, lang, "admin.workflow.approval_note_ph", "Yazar ekibine iletilecek kısa değerlendirme notu...", "Brief evaluation note for the editorial team...");
    let cancel_btn = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");
    let confirm_btn = tr_admin(tr, lang, "admin.workflow.confirm_publish", "Onayla ve Yayınla", "Approve and Publish");

    tab_pane(buf, "tab-16", "tab-btn-16", false, |tab| {
        admin_card(
            tab,
            "",
            None,
            "fa-check-square-o",
            &card_title,
            Some(&card_desc),
            |hdr| action_btn(hdr, "btn-cmd btn-cmd-sm", "load-pending-approvals", None, Some("fa-refresh"), &refresh_btn),
            |card| {
                ops_table(
                    card,
                    "",
                    Some("table-pending-approvals"),
                    &[
                        (&col_title, ""),
                        (&col_author, ""),
                        (&col_type, ""),
                        (&col_submitted, ""),
                        (&col_status, ""),
                        (&col_decision, ""),
                    ],
                    "tbody-pending-approvals",
                    Some(&loading),
                    None,
                );
            },
        );

        tab.el(
            "div",
            &[
                ("id", "approval-modal"),
                ("class", "modal-overlay hidden"),
                ("role", "dialog"),
                ("aria-modal", "true"),
                ("aria-labelledby", "approval-modal-title"),
            ],
            |ov| {
                ov.el("div", &[("class", "modal-content admin-card")], |mc| {
                    mc.el("div", &[("class", "panel-header")], |hdr| {
                        hdr.el("div", &[("class", "panel-title"), ("id", "approval-modal-title")], |t| {
                            t.text(&modal_title);
                        });
                        hdr.el(
                            "button",
                            &[("type", "button"), ("class", "btn-icon"), ("data-admin-action", "close-approval-modal"), ("aria-label", &close_lbl)],
                            |b| icon(b, "fa-times"),
                        );
                    });
                    mc.el("div", &[("class", "panel-body")], |bd| {
                        form_group(bd, Some("approval-note"), &note_lbl, |g| {
                            textarea_ctrl(g, "approval-note", "textarea-sm", &note_ph, &[("rows", "3")]);
                        });
                    });
                    mc.el("div", &[("class", "modal-footer")], |ft| {
                        action_btn(ft, "btn-cmd", "close-approval-modal", None, None, &cancel_btn);
                        ft.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("class", "btn-cmd btn-cmd-primary"),
                                ("id", "approval-confirm-btn"),
                                ("data-admin-action", "confirm-approval"),
                            ],
                            |b| {
                                b.text(&confirm_btn);
                            },
                        );
                    });
                });
            },
        );
    });
}

fn render_tab_bulk_versions(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let bulk_title = tr_admin(tr, lang, "admin.workflow.bulk_title", "Toplu İçerik Operasyonları", "Bulk Content Operations");
    let bulk_desc = tr_admin(tr, lang, "admin.workflow.bulk_desc", "Birden fazla yayının erişim seviyesini, manşet durumunu veya yayın statüsünü tek adımda güncelleyin.", "Update visibility level, featured flag or publication status of multiple items in a single step.");
    let search_ph = tr_admin(tr, lang, "admin.workflow.bulk_search_ph", "İçerik ara...", "Search content...");
    let search_aria = tr_admin(tr, lang, "admin.workflow.bulk_search_aria", "Toplu işlem listesinde ara", "Search bulk operations list");
    let select_all_btn = tr_admin(tr, lang, "admin.workflow.select_all", "Tümünü Seç", "Select All");
    let clear_btn = tr_admin(tr, lang, "admin.form.clear", "Temizle", "Clear");
    let selected_lbl = tr_admin(tr, lang, "admin.workflow.selected_count", "Seçili kayıt: ", "Selected records: ");
    let act_placeholder = tr_admin(tr, lang, "admin.workflow.bulk_act_ph", "Uygulanacak işlemi seçin...", "Select operation to apply...");
    let act_publish = tr_admin(tr, lang, "admin.workflow.act_publish", "Yayına Al", "Publish");
    let act_draft = tr_admin(tr, lang, "admin.workflow.act_draft", "Taslağa Çek", "Move to Draft");
    let act_archive = tr_admin(tr, lang, "admin.workflow.act_archive", "Arşive Kaldır", "Archive");
    let act_feat = tr_admin(tr, lang, "admin.workflow.act_set_featured", "Manşete Taşı", "Set Featured");
    let act_unfeat = tr_admin(tr, lang, "admin.workflow.act_unset_featured", "Manşetten Çıkar", "Unset Featured");
    let act_vis0 = tr_admin(tr, lang, "admin.workflow.act_vis_0", "Açık Erişim Yap", "Set Open Access");
    let act_vis1 = tr_admin(tr, lang, "admin.workflow.act_vis_1", "Kayıtlı Okuyucuya Aç", "Open to Registered Readers");
    let act_vis2 = tr_admin(tr, lang, "admin.workflow.act_vis_2", "Sadece Aboneler Yap", "Subscribers Only");
    let act_delete = tr_admin(tr, lang, "admin.workflow.act_delete", "Kalıcı Olarak Sil", "Delete Permanently");
    let act_aria = tr_admin(tr, lang, "admin.workflow.bulk_act_aria", "Toplu işlem türü", "Bulk action type");
    let execute_btn = tr_admin(tr, lang, "admin.workflow.execute_bulk", "Seçililere Uygula", "Apply to Selected");
    let select_all_aria = tr_admin(tr, lang, "admin.workflow.select_all_rows", "Tüm satırları seç", "Select all rows");
    let th_id = tr_admin(tr, lang, "admin.workflow.col_id", "ID", "ID");
    let th_title = tr_admin(tr, lang, "admin.workflow.col_title", "İçerik Başlığı", "Content Title");
    let th_type = tr_admin(tr, lang, "admin.workflow.col_type", "Tür", "Type");
    let th_access = tr_admin(tr, lang, "admin.workflow.col_access", "Erişim", "Access");
    let th_views = tr_admin(tr, lang, "admin.workflow.col_views", "Görüntülenme", "Views");

    let ver_title = tr_admin(tr, lang, "admin.workflow.versions_title", "Sürüm Geçmişi & Geri Yükleme", "Version History & Restore");
    let ver_desc = tr_admin(tr, lang, "admin.workflow.versions_desc", "İçerik revizyonlarını karşılaştırın ve önceki sürüme dönün.", "Compare content revisions and restore a previous version.");
    let ver_select_lbl = tr_admin(tr, lang, "admin.workflow.versions_select_lbl", "İncelenecek İçerik", "Content to Inspect");
    let ver_select_ph = tr_admin(tr, lang, "admin.workflow.versions_select_ph", "-- İçerik seçin --", "-- Select content --");
    let ver_empty = tr_admin(tr, lang, "admin.workflow.versions_empty", "Sürüm geçmişini görüntülemek için listeden bir içerik seçin.", "Select a content item from the list to view version history.");

    tab_pane(buf, "tab-12", "tab-btn-12", false, |tab| {
        tab.el("div", &[("class", "workspace-grid media-seo-grid")], |grid| {
            admin_card(
                grid,
                "",
                None,
                "fa-sliders",
                &bulk_title,
                Some(&bulk_desc),
                |hdr| {
                    hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                        input_ctrl(tb, "text", "bulk-search-input", "input-md", "", &search_ph, &[("aria-label", &search_aria)]);
                        action_btn(tb, "btn-cmd btn-cmd-sm", "bulk-select-all", None, Some("fa-check-square-o"), &select_all_btn);
                        action_btn(tb, "btn-cmd btn-cmd-sm", "bulk-deselect-all", None, Some("fa-times"), &clear_btn);
                    });
                },
                |card| {
                    card.el("div", &[("class", "panel-toolbar")], |bar| {
                        bar.el("span", &[("class", "text-sm text-secondary")], |s| {
                            s.text(&selected_lbl);
                            s.el("strong", &[("id", "bulk-selected-count"), ("class", "text-accent font-bold")], |st| {
                                st.text("0");
                            });
                        });
                        bar.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |act| {
                            select_ctrl(
                                act,
                                "bulk-action-select",
                                "input-xl",
                                &[
                                    ("", &act_placeholder),
                                    ("publish", &act_publish),
                                    ("draft", &act_draft),
                                    ("archive", &act_archive),
                                    ("set_featured", &act_feat),
                                    ("unset_featured", &act_unfeat),
                                    ("set_visibility_0", &act_vis0),
                                    ("set_visibility_1", &act_vis1),
                                    ("set_visibility_2", &act_vis2),
                                    ("delete", &act_delete),
                                ],
                                &[("aria-label", &act_aria)],
                            );
                            action_btn(act, "btn-cmd btn-cmd-primary btn-cmd-sm", "execute-bulk", Some("contents:write"), Some("fa-play"), &execute_btn);
                        });
                    });

                    card.el("div", &[("class", "table-responsive scrollable-panel-lg")], |wrap| {
                        wrap.el("table", &[("class", "ops-table")], |tbl| {
                            tbl.el("thead", &[], |th_w| {
                                th_w.el("tr", &[], |tr_row| {
                                    tr_row.el("th", &[("class", "w-checkbox")], |th| {
                                        th.void_el("input", &[("type", "checkbox"), ("id", "bulk-select-all-cb"), ("aria-label", &select_all_aria)]);
                                    });
                                    for lbl in [th_id.as_str(), th_title.as_str(), th_type.as_str(), th_access.as_str(), th_views.as_str()] {
                                        tr_row.el("th", &[], |th| {
                                            th.text(lbl);
                                        });
                                    }
                                });
                            });
                            tbl.el("tbody", &[("id", "tbody-bulk-contents")], |_| {});
                        });
                    });
                },
            );

            admin_card(
                grid,
                "",
                None,
                "fa-code-fork",
                &ver_title,
                Some(&ver_desc),
                |_| {},
                |card| {
                    card.el("div", &[("class", "panel-body")], |bd| {
                        form_group(bd, Some("version-history-select"), &ver_select_lbl, |g| {
                            select_ctrl(g, "version-history-select", "", &[("", &ver_select_ph)], &[]);
                        });
                        bd.el("div", &[("id", "version-history-list"), ("class", "scrollable-panel-lg border rounded-md")], |lst| {
                            lst.el("div", &[("class", "text-muted text-center p-4 text-sm")], |t| {
                                t.text(&ver_empty);
                            });
                        });
                        bd.el("div", &[("id", "pagination-version-history"), ("class", "mt-2")], |_| {});
                    });
                },
            );
        });
    });
}

fn render_tab_newsletters(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let all_insts_lbl = tr_admin(tr, lang, "admin.workflow.nl_all_institutions", "Tüm Kurumlar", "All Institutions");
    let univ_lbl = tr_admin(tr, lang, "admin.workflow.nl_university", "Üniversite / Akademik", "University / Academic");
    let all_ssb_lbl = tr_admin(tr, lang, "admin.workflow.nl_all_ssb", "Tüm SSB İlgi Alanları", "All SSB Interest Areas");

    let inst_resolved = institution_tags(tr, lang);
    let mut inst_opts: Vec<(&str, &str)> = vec![("ALL", &all_insts_lbl)];
    for (code, lbl) in &inst_resolved {
        inst_opts.push((*code, lbl.as_str()));
    }
    inst_opts.push(("Üniversite / Öğrenci", &univ_lbl));

    let ssb_resolved = ssb_tags(tr, lang);
    let mut ssb_opts: Vec<(&str, &str)> = vec![("ALL", &all_ssb_lbl)];
    for (code, lbl) in &ssb_resolved {
        ssb_opts.push((*code, lbl.as_str()));
    }

    let subtab_campaigns = tr_admin(tr, lang, "admin.workflow.subtab_campaigns", "Bülten Kampanyaları", "Newsletter Campaigns");
    let subtab_email_groups = tr_admin(tr, lang, "admin.workflow.subtab_email_groups", "E-Posta Grupları", "Email Groups");

    let card_title = tr_admin(tr, lang, "admin.workflow.nl_title", "Kurumsal E-Bülten Kampanyaları", "Corporate Newsletter Campaigns");
    let card_desc = tr_admin(tr, lang, "admin.workflow.nl_desc", "KVKK onaylı savunma sanayii profesyonellerine, kamu kurumlarına ve abonelere gönderilen hedefli bülten arşivleri.", "Targeted newsletter archives sent to KVKK-consented defence professionals, public institutions and subscribers.");
    let new_nl_btn = tr_admin(tr, lang, "admin.workflow.nl_new", "Yeni Bülten Gönder", "Send New Newsletter");
    let col_campaign = tr_admin(tr, lang, "admin.workflow.nl_col_campaign", "Kampanya & Tarih", "Campaign & Date");
    let col_subject = tr_admin(tr, lang, "admin.workflow.nl_col_subject", "Konu Başlığı (TR / EN)", "Subject (TR / EN)");
    let col_audience = tr_admin(tr, lang, "admin.workflow.nl_col_audience", "Hedef Kitle & SSB Filtresi", "Target Audience & SSB Filter");
    let col_sponsor = tr_admin(tr, lang, "admin.workflow.nl_col_sponsor", "Bülten Sponsoru", "Newsletter Sponsor");
    let col_delivery = tr_admin(tr, lang, "admin.workflow.nl_col_delivery", "Teslimat", "Delivery");
    let col_action = tr_admin(tr, lang, "admin.form.col_action", "İşlem", "Action");
    let nl_loading = tr_admin(tr, lang, "admin.workflow.nl_loading", "Gönderilen bülten kampanyaları yükleniyor...", "Loading sent newsletter campaigns...");

    let drawer_title = tr_admin(tr, lang, "admin.workflow.nl_drawer_title", "Hedefli Kurumsal E-Bülten Gönderimi", "Targeted Corporate Newsletter Dispatch");
    let close_lbl = tr_admin(tr, lang, "admin.form.close", "Kapat", "Close");
    let tab_tr = tr_admin(tr, lang, "admin.workflow.nl_tab_tr", "Türkçe Konu", "Turkish Subject");
    let tab_en = tr_admin(tr, lang, "admin.workflow.nl_tab_en", "English Subject", "English Subject");
    let subj_tr_lbl = tr_admin(tr, lang, "admin.workflow.nl_subj_tr_lbl", "Bülten Konu Başlığı (TR)", "Newsletter Subject (TR)");
    let subj_tr_ph = tr_admin(tr, lang, "admin.workflow.nl_subj_tr_ph", "Örn: [MSI Özel Bülten] Hava Savunma ve Radar Sistemleri Dosyası", "e.g. [MSI Special Briefing] Air Defence & Radar Dossier");
    let subj_en_lbl = tr_admin(tr, lang, "admin.workflow.nl_subj_en_lbl", "Newsletter Subject (EN)", "Newsletter Subject (EN)");
    let subj_en_ph = tr_admin(tr, lang, "admin.workflow.nl_subj_en_ph", "e.g. [MSI TDR Briefing] Air Defence & Radar Dossier", "e.g. [MSI TDR Briefing] Air Defence & Radar Dossier");
    let lbl_target_inst = tr_admin(tr, lang, "admin.workflow.nl_target_inst", "Hedef Kurum", "Target Institution");
    let lbl_ssb = tr_admin(tr, lang, "admin.workflow.nl_ssb_tag", "SSB İlgi Alanı", "SSB Interest Area");
    let lbl_segment = tr_admin(tr, lang, "admin.workflow.nl_segment", "Alıcı Segmenti", "Recipient Segment");
    let seg_all = tr_admin(tr, lang, "admin.workflow.nl_seg_all", "KVKK İleti Onaylı Tüm Okuyucular", "All KVKK-Consented Readers");
    let seg_gov = tr_admin(tr, lang, "admin.workflow.nl_seg_gov", "Doğrulanmış Kamu (.gov.tr / .tsk.tr)", "Verified Public Sector (.gov.tr / .tsk.tr)");
    let seg_sub = tr_admin(tr, lang, "admin.workflow.nl_seg_sub", "Aktif Kurumsal & Bireysel Aboneler", "Active Corporate & Individual Subscribers");
    let lbl_sponsor_name = tr_admin(tr, lang, "admin.workflow.nl_sponsor_name", "Bülten Sponsoru (İsteğe Bağlı)", "Newsletter Sponsor (Optional)");
    let sponsor_name_ph = tr_admin(tr, lang, "admin.workflow.nl_sponsor_ph", "Boş bırakılırsa aktif sponsor otomatik atanır", "Auto-assigned from active newsletter ad if empty");
    let lbl_sponsor_ad = tr_admin(tr, lang, "admin.workflow.nl_sponsor_ad_id", "Sponsor Reklam Kampanyası", "Sponsor Ad Campaign");
    let auto_sponsor_opt = tr_admin(tr, lang, "admin.workflow.nl_sponsor_auto", "Otomatik Seç (Aktif E-Bülten Sponsoru)", "Auto-Inject (Active Newsletter Sponsor)");
    let lbl_email_group = tr_admin(tr, lang, "admin.workflow.nl_email_group", "E-Posta Grubu (İsteğe Bağlı)", "Email Group (Optional)");
    let email_group_none = tr_admin(tr, lang, "admin.workflow.nl_email_group_none", "— Alıcı Segment Filtresi Kullan —", "— Use Segment Filter —");
    let lbl_scheduled_at = tr_admin(tr, lang, "admin.workflow.nl_scheduled_at", "Zamanlama Tarihi (UTC/Yerel)", "Schedule Date & Time");
    let lbl_nl_status = tr_admin(tr, lang, "admin.workflow.nl_status", "Gönderim Durumu", "Dispatch Status");
    let st_sent = tr_admin(tr, lang, "admin.workflow.nl_st_sent", "Hemen Gönder", "Send Immediately");
    let st_scheduled = tr_admin(tr, lang, "admin.workflow.nl_st_scheduled", "Zamanla", "Scheduled");
    let st_draft = tr_admin(tr, lang, "admin.workflow.nl_st_draft", "Taslak", "Draft");
    let lbl_body = tr_admin(tr, lang, "admin.workflow.nl_body_html", "Bülten İçerik Metni (HTML)", "Newsletter Body (HTML)");
    let default_body = tr_admin(tr, lang, "admin.workflow.nl_default_body", "<p>Değerli savunma ve havacılık ekosistemi paydaşlarımız...</p>", "<p>Dear defence and aerospace ecosystem stakeholders...</p>");
    let cancel_btn = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");
    let send_btn = tr_admin(tr, lang, "admin.workflow.nl_send_btn", "Bülteni Gönder ve Kayıt Altına Al", "Send and Log Newsletter");

    let eg_card_title = tr_admin(tr, lang, "admin.workflow.eg_title", "Segmentli E-Posta Grupları", "Segmented Email Groups");
    let eg_card_desc = tr_admin(tr, lang, "admin.workflow.eg_desc", "Dinamik rol/kurum filtreleri ve statik kurumsal alıcı listeleriyle bülten grupları tanımlayın.", "Define newsletter groups with dynamic role/institution filters and static institutional recipient lists.");
    let eg_new_btn = tr_admin(tr, lang, "admin.workflow.eg_new", "Yeni E-Posta Grubu", "New Email Group");
    let eg_code_lbl = tr_admin(tr, lang, "admin.workflow.eg_code", "Grup Kodu", "Group Code");
    let eg_code_ph = tr_admin(tr, lang, "admin.workflow.eg_code_ph", "savunma_yoneticileri", "defence_executives");
    let eg_name_tr_lbl = tr_admin(tr, lang, "admin.workflow.eg_name_tr", "Grup Adı (TR)", "Group Name (TR)");
    let eg_name_tr_ph = tr_admin(tr, lang, "admin.workflow.eg_name_tr_ph", "Savunma Yöneticileri", "Defence Executives");
    let eg_name_en_lbl = tr_admin(tr, lang, "admin.workflow.eg_name_en", "Grup Adı (EN)", "Group Name (EN)");
    let eg_name_en_ph = tr_admin(tr, lang, "admin.workflow.eg_name_en_ph", "Defence Executives", "Defence Executives");
    let eg_desc_tr_lbl = tr_admin(tr, lang, "admin.workflow.eg_desc_tr", "Açıklama (TR)", "Description (TR)");
    let eg_desc_en_lbl = tr_admin(tr, lang, "admin.workflow.eg_desc_en", "Description (EN)", "Description (EN)");
    let eg_sort_lbl = tr_admin(tr, lang, "admin.workflow.eg_sort", "Sıralama", "Sort Order");
    let eg_filter_lbl = tr_admin(tr, lang, "admin.workflow.eg_filter_json", "Dinamik Filtre (JSON)", "Dynamic Filter (JSON)");
    let eg_static_lbl = tr_admin(tr, lang, "admin.workflow.eg_static_emails_json", "Statik E-Posta Listesi (JSON)", "Static Emails (JSON)");
    let eg_consent_lbl = tr_admin(tr, lang, "admin.workflow.eg_requires_consent", "KVKK İleti İzni Gerekli", "Requires KVKK Marketing Consent");
    let eg_active_lbl = tr_admin(tr, lang, "admin.workflow.eg_is_active", "Aktif", "Active");
    let eg_save_btn = tr_admin(tr, lang, "admin.workflow.eg_save", "Grubu Kaydet", "Save Group");
    let eg_col_code = tr_admin(tr, lang, "admin.workflow.eg_col_code", "Kod", "Code");
    let eg_col_name = tr_admin(tr, lang, "admin.workflow.eg_col_name", "Grup Adı (TR / EN)", "Group Name (TR / EN)");
    let eg_col_filter = tr_admin(tr, lang, "admin.workflow.eg_col_filter", "Filtre / Statik", "Filter / Static");
    let eg_col_status = tr_admin(tr, lang, "admin.workflow.eg_col_status", "KVKK & Durum", "KVKK & Status");
    let eg_loading = tr_admin(tr, lang, "admin.workflow.eg_loading", "E-posta grupları yükleniyor...", "Loading email groups...");
    let eg_static_ph = tr_admin(tr, lang, "admin.workflow.eg_static_emails_ph", r#"["ornek@kurum.gov.tr"]"#, r#"["example@agency.gov"]"#);

    tab_pane(buf, "tab-6", "tab-btn-6", false, |tab| {
        subtab_bar(tab, "tab6", &[
            ("tab6-campaigns", &subtab_campaigns, Some("newsletters:read")),
            ("tab6-email-groups", &subtab_email_groups, Some("email_groups:write|email_groups:manage")),
        ]);

        subtab_pane(tab, "tab6-campaigns", true, Some("newsletters:read"), |pane| {
            admin_card(
                pane,
                "",
                None,
                "fa-envelope-o",
                &card_title,
                Some(&card_desc),
                |hdr| action_btn(hdr, "btn-cmd btn-cmd-primary", "open-newsletter-drawer", Some("newsletters:write"), Some("fa-paper-plane"), &new_nl_btn),
                |card| {
                    ops_table(
                        card,
                        "",
                        None,
                        &[
                            (&col_campaign, ""),
                            (&col_subject, ""),
                            (&col_audience, ""),
                            (&col_sponsor, ""),
                            (&col_delivery, ""),
                            (&col_action, ""),
                        ],
                        "tbody-newsletters",
                        Some(&nl_loading),
                        Some("pagination-newsletters"),
                    );
                },
            );

            admin_drawer(
                pane,
                "drawer-newsletter",
                "newsletter-drawer-heading",
                &drawer_title,
                &close_lbl,
                |_| {},
                |body| {
                    body.el_with("form", |a| { a.attr("id", "form-newsletter").bool_attr("novalidate", true); }, |f| {
                        lang_tabs(
                            f,
                            &tab_tr,
                            &tab_en,
                            |tr_tab| {
                                form_group(tr_tab, Some("nl-subject-tr"), &subj_tr_lbl, |g| {
                                    input_ctrl(g, "text", "nl-subject-tr", "", "", &subj_tr_ph, &[("required", "required")]);
                                });
                            },
                            |en_tab| {
                                form_group(en_tab, Some("nl-subject-en"), &subj_en_lbl, |g| {
                                    input_ctrl(g, "text", "nl-subject-en", "", "", &subj_en_ph, &[]);
                                });
                            },
                        );
                        f.el("div", &[("class", "form-row-3")], |r| {
                            form_group(r, Some("nl-institution"), &lbl_target_inst, |g| select_ctrl(g, "nl-institution", "", &inst_opts, &[]));
                            form_group(r, Some("nl-ssb-tag"), &lbl_ssb, |g| select_ctrl(g, "nl-ssb-tag", "", &ssb_opts, &[]));
                            form_group(r, Some("nl-group-filter"), &lbl_segment, |g| {
                                select_ctrl(g, "nl-group-filter", "", &[
                                    ("all_consented", &seg_all),
                                    ("gov_verified", &seg_gov),
                                    ("active_subscribers", &seg_sub),
                                ], &[]);
                            });
                        });
                        f.el("div", &[("class", "form-row-3")], |r| {
                            form_group(r, Some("nl-email-group"), &lbl_email_group, |g| {
                                select_ctrl(g, "nl-email-group", "", &[("", &email_group_none)], &[("name", "email_group_code")]);
                            });
                            form_group(r, Some("nl-scheduled-at"), &lbl_scheduled_at, |g| {
                                input_ctrl(g, "datetime-local", "nl-scheduled-at", "", "", "", &[("name", "scheduled_at")]);
                            });
                            form_group(r, Some("nl-status"), &lbl_nl_status, |g| {
                                select_ctrl(g, "nl-status", "", &[("sent", &st_sent), ("scheduled", &st_scheduled), ("draft", &st_draft)], &[("name", "status")]);
                            });
                        });
                        f.el("div", &[("class", "form-row-2")], |r| {
                            form_group(r, Some("nl-sponsor-name"), &lbl_sponsor_name, |g| input_ctrl(g, "text", "nl-sponsor-name", "", "", &sponsor_name_ph, &[]));
                            form_group(r, Some("nl-sponsor-ad-id"), &lbl_sponsor_ad, |g| {
                                select_ctrl(g, "nl-sponsor-ad-id", "", &[("", &auto_sponsor_opt)], &[("name", "sponsor_ad_id")]);
                            });
                        });
                        form_group(f, Some("nl-body-html"), &lbl_body, |g| {
                            textarea_ctrl(g, "nl-body-html", "", &default_body, &[("rows", "6")]);
                        });
                    });
                },
                |ft| {
                    action_btn(ft, "btn-cmd", "close-drawer", None, None, &cancel_btn);
                    ft.el(
                        "button",
                        &[("type", "submit"), ("form", "form-newsletter"), ("class", "btn-cmd btn-cmd-primary"), ("data-required-perm", "newsletters:write")],
                        |b| {
                            icon(b, "fa-paper-plane");
                            b.raw(" ");
                            b.text(&send_btn);
                        },
                    );
                },
            );
        });

        subtab_pane(tab, "tab6-email-groups", false, Some("email_groups:write|email_groups:manage"), |pane| {
            admin_card(
                pane,
                "",
                None,
                "fa-users",
                &eg_card_title,
                Some(&eg_card_desc),
                |hdr| {
                    action_btn(hdr, "btn-cmd btn-cmd-primary btn-cmd-sm", "show-email-group-form", Some("email_groups:write|email_groups:manage"), Some("fa-plus"), &eg_new_btn);
                },
                |card| {
                    card.el("div", &[("id", "email-group-form-container"), ("class", "hidden form-container-elevated")], |fc| {
                        fc.el_with("form", |a| { a.attr("id", "form-email-group").bool_attr("novalidate", true); }, |f| {
                            f.void_el("input", &[("type", "hidden"), ("id", "eg-id"), ("value", "0")]);
                            f.el("div", &[("class", "form-row-3")], |r| {
                                form_group(r, Some("eg-code"), &eg_code_lbl, |g| input_ctrl(g, "text", "eg-code", "", "", &eg_code_ph, &[("required", "required")]));
                                form_group(r, Some("eg-name-tr"), &eg_name_tr_lbl, |g| input_ctrl(g, "text", "eg-name-tr", "", "", &eg_name_tr_ph, &[("required", "required")]));
                                form_group(r, Some("eg-name-en"), &eg_name_en_lbl, |g| input_ctrl(g, "text", "eg-name-en", "", "", &eg_name_en_ph, &[]));
                            });
                            f.el("div", &[("class", "form-row-3")], |r| {
                                form_group(r, Some("eg-desc-tr"), &eg_desc_tr_lbl, |g| input_ctrl(g, "text", "eg-desc-tr", "", "", "", &[]));
                                form_group(r, Some("eg-desc-en"), &eg_desc_en_lbl, |g| input_ctrl(g, "text", "eg-desc-en", "", "", "", &[]));
                                form_group(r, Some("eg-sort-order"), &eg_sort_lbl, |g| input_ctrl(g, "number", "eg-sort-order", "", "0", "0", &[]));
                            });
                            f.el("div", &[("class", "form-row-2")], |r| {
                                form_group(r, Some("eg-filter-json"), &eg_filter_lbl, |g| textarea_ctrl(g, "eg-filter-json", "textarea-sm text-mono", r#"{"roles":["registered"]}"#, &[("rows", "2")]));
                                form_group(r, Some("eg-static-emails-json"), &eg_static_lbl, |g| textarea_ctrl(g, "eg-static-emails-json", "textarea-sm text-mono", &eg_static_ph, &[("rows", "2")]));
                            });
                            f.el("div", &[("class", "flex-between flex-wrap gap-2")], |fl| {
                                fl.el("div", &[("class", "flex gap-3 flex-wrap")], |chks| {
                                    check_pill(chks, Some("eg-requires-consent"), None, None, true, "", &eg_consent_lbl);
                                    check_pill(chks, Some("eg-is-active"), None, None, true, "", &eg_active_lbl);
                                });
                                fl.el("div", &[("class", "flex gap-2")], |btns| {
                                    btns.el(
                                        "button",
                                        &[("type", "submit"), ("id", "btn-email-group-save"), ("class", "btn-cmd btn-cmd-primary btn-cmd-sm"), ("data-required-perm", "email_groups:write|email_groups:manage")],
                                        |b| {
                                            icon(b, "fa-save");
                                            b.raw(" ");
                                            b.text(&eg_save_btn);
                                        },
                                    );
                                    action_btn(btns, "btn-cmd btn-cmd-sm", "hide-email-group-form", None, None, &cancel_btn);
                                });
                            });
                        });
                    });

                    ops_table(
                        card,
                        "",
                        Some("table-email-groups"),
                        &[
                            (&eg_col_code, ""),
                            (&eg_col_name, ""),
                            (&eg_col_filter, ""),
                            (&eg_col_status, ""),
                            (&col_action, ""),
                        ],
                        "tbody-email-groups",
                        Some(&eg_loading),
                        None,
                    );
                },
            );
        });
    });
}
