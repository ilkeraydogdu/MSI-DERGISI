//! Analytics & Telemetry Views: `#tab-3` 3D B2B Defence Analytics,
//! `#tab-9` Readership Charts & Database Health, `#tab-15` Subscriber Lifecycle & Cohorts.

use super::tr_admin;
use super::ui::{
    action_btn, admin_card, icon, input_ctrl, kpi_card_toned, ops_table, tab_pane,
};
use crate::engine::design::html_dsl::HtmlBuf;
use crate::engine::translation_engine::TranslationCache;

pub fn render(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    render_tab_b2b_analytics(buf, tr, lang);
    render_tab_charts_health(buf, tr, lang);
    render_tab_lifecycle(buf, tr, lang);
}

fn render_tab_b2b_analytics(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let card_title = tr_admin(tr, lang, "admin.analytics.b2b_title", "3 Boyutlu Kurumsal Savunma Analitiği", "3-Dimensional Corporate Defence Analytics");
    let card_desc = tr_admin(tr, lang, "admin.analytics.b2b_desc", "İçerik, reklam kampanyası ve kurumsal okuyucu bazında doğrulanmış etkileşim ve kurum dağılımı.", "Verified engagement and institution breakdown across content, ad campaigns and corporate readers.");
    let search_ph = tr_admin(tr, lang, "admin.analytics.search_ph", "Başlık, sponsor, kurum veya okuyucu ara...", "Search title, sponsor, institution or reader...");
    let search_aria = tr_admin(tr, lang, "admin.analytics.search_aria", "Analitik tablosunda ara", "Search analytics table");
    let btn_csv = tr_admin(tr, lang, "admin.analytics.export_csv", "CSV İndir", "Export CSV");
    let btn_json = tr_admin(tr, lang, "admin.analytics.export_json", "Konsolide JSON", "Consolidated JSON");
    let dim_aria = tr_admin(tr, lang, "admin.analytics.dimension_aria", "Rapor boyutu", "Report dimension");
    let sv_content = tr_admin(tr, lang, "admin.analytics.subview_content", "İçerik Performansı", "Content Performance");
    let sv_ad = tr_admin(tr, lang, "admin.analytics.subview_ad", "Sponsor & Kampanya", "Sponsor & Campaign");
    let sv_reader = tr_admin(tr, lang, "admin.analytics.subview_reader", "Kurumsal Okuyucu", "Corporate Reader");
    let realtime_lbl = tr_admin(tr, lang, "admin.analytics.realtime_telemetry", "Gerçek zamanlı kurumsal telemetri", "Real-time corporate telemetry");

    let c_col_content = tr_admin(tr, lang, "admin.analytics.col_content", "İçerik & Kalıcı Bağlantı", "Content & Permalink");
    let c_col_views = tr_admin(tr, lang, "admin.analytics.col_views", "Görüntülenme", "Views");
    let c_col_reads = tr_admin(tr, lang, "admin.analytics.col_reads", "Doğrulanmış Okuma", "Verified Reads");
    let c_col_duration = tr_admin(tr, lang, "admin.analytics.col_duration", "Ort. Süre & Kaydırma", "Avg. Duration & Scroll");
    let c_col_inst = tr_admin(tr, lang, "admin.analytics.col_inst_dist", "Kurum Dağılımı", "Institution Breakdown");
    let c_loading = tr_admin(tr, lang, "admin.analytics.content_loading", "İçerik analitiği yükleniyor...", "Loading content analytics...");

    let a_col_sponsor = tr_admin(tr, lang, "admin.analytics.col_sponsor_slot", "Sponsor & Reklam Alanı", "Sponsor & Ad Slot");
    let a_col_ctr = tr_admin(tr, lang, "admin.analytics.col_impressions_ctr", "Gösterim / Tıklama / CTR", "Impressions / Clicks / CTR");
    let a_col_viewers = tr_admin(tr, lang, "admin.analytics.col_viewing_insts", "Görüntüleyen Kurumlar", "Viewing Institutions");
    let a_col_clickers = tr_admin(tr, lang, "admin.analytics.col_engaging_insts", "Etkileşim Veren Kurumlar", "Engaging Institutions");
    let a_loading = tr_admin(tr, lang, "admin.analytics.ad_loading", "Kampanya analitiği yükleniyor...", "Loading campaign analytics...");

    let r_col_reader = tr_admin(tr, lang, "admin.analytics.col_reader", "Okuyucu", "Reader");
    let r_col_inst = tr_admin(tr, lang, "admin.analytics.col_reader_inst", "Kurum & Kategori", "Institution & Category");
    let r_col_pkg = tr_admin(tr, lang, "admin.analytics.col_access_pkg", "Erişim Paketi", "Access Package");
    let r_col_pubs = tr_admin(tr, lang, "admin.analytics.col_viewed_pubs", "İncelenen Yayınlar", "Viewed Publications");
    let r_col_ssb = tr_admin(tr, lang, "admin.analytics.col_ssb_interests", "SSB İlgi Alanları", "SSB Interest Areas");
    let r_col_sponsor = tr_admin(tr, lang, "admin.analytics.col_sponsor_eng", "Sponsor Etkileşimi", "Sponsor Engagement");
    let r_loading = tr_admin(tr, lang, "admin.analytics.reader_loading", "Kurumsal okuyucu analitiği yükleniyor...", "Loading corporate reader analytics...");

    tab_pane(buf, "tab-3", "tab-btn-3", false, |tab| {
        admin_card(
            tab,
            "",
            None,
            "fa-pie-chart",
            &card_title,
            Some(&card_desc),
            |hdr| {
                hdr.el("div", &[("class", "flex gap-2 items-center flex-wrap")], |tb| {
                    input_ctrl(
                        tb,
                        "text",
                        "analytics-search",
                        "input-lg",
                        "",
                        &search_ph,
                        &[("aria-label", &search_aria)],
                    );
                    action_btn(tb, "btn-cmd btn-cmd-sm", "export-analytics-csv", Some("analytics:read"), Some("fa-file-excel-o"), &btn_csv);
                    action_btn(tb, "btn-cmd btn-cmd-sm btn-cmd-warning", "export-analytics-json", Some("analytics:read"), Some("fa-download"), &btn_json);
                });
            },
            |card| {
                card.el("div", &[("class", "panel-toolbar")], |bar| {
                    bar.el("div", &[("class", "segmented-pills"), ("role", "tablist"), ("aria-label", &dim_aria)], |seg| {
                        for (id, sub, fa, lbl, active) in [
                            ("btn-subview-content", "content", "fa-newspaper-o", sv_content.as_str(), true),
                            ("btn-subview-ad", "ad", "fa-bullhorn", sv_ad.as_str(), false),
                            ("btn-subview-reader", "reader", "fa-building-o", sv_reader.as_str(), false),
                        ] {
                            seg.el(
                                "button",
                                &[
                                    ("type", "button"),
                                    ("id", id),
                                    ("class", if active { "btn-cmd btn-cmd-sm btn-cmd-primary" } else { "btn-cmd btn-cmd-sm" }),
                                    ("data-admin-action", "switch-analytics-subview"),
                                    ("data-subview", sub),
                                ],
                                |b| {
                                    icon(b, fa);
                                    b.raw(" ");
                                    b.text(lbl);
                                },
                            );
                        }
                    });
                    bar.el("span", &[("class", "text-xs text-muted"), ("id", "analytics-count-label")], |s| {
                        s.text(&realtime_lbl);
                    });
                });

                card.el("div", &[("id", "subview-content"), ("class", "subview-pane active")], |sv| {
                    ops_table(
                        sv,
                        "",
                        None,
                        &[
                            (&c_col_content, ""),
                            (&c_col_views, "text-right"),
                            (&c_col_reads, "text-right"),
                            (&c_col_duration, ""),
                            (&c_col_inst, "col-wide"),
                        ],
                        "tbody-analytics-content",
                        Some(&c_loading),
                        Some("pagination-analytics-content"),
                    );
                });

                card.el("div", &[("id", "subview-ad"), ("class", "subview-pane")], |sv| {
                    ops_table(
                        sv,
                        "",
                        None,
                        &[
                            (&a_col_sponsor, ""),
                            (&a_col_ctr, ""),
                            (&a_col_viewers, "col-medium"),
                            (&a_col_clickers, "col-medium"),
                        ],
                        "tbody-analytics-ad",
                        Some(&a_loading),
                        Some("pagination-analytics-ad"),
                    );
                });

                card.el("div", &[("id", "subview-reader"), ("class", "subview-pane")], |sv| {
                    ops_table(
                        sv,
                        "",
                        None,
                        &[
                            (&r_col_reader, ""),
                            (&r_col_inst, ""),
                            (&r_col_pkg, ""),
                            (&r_col_pubs, ""),
                            (&r_col_ssb, ""),
                            (&r_col_sponsor, ""),
                        ],
                        "tbody-analytics-reader",
                        Some(&r_loading),
                        Some("pagination-analytics-reader"),
                    );
                });
            },
        );
    });
}

fn render_tab_charts_health(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let trend_title = tr_admin(tr, lang, "admin.analytics.trend_title", "Okuma Derinliği ve Erişim Eğilimi", "Reading Depth & Reach Trend");
    let trend_desc = tr_admin(tr, lang, "admin.analytics.trend_desc", "Son yayınlanan içeriklerde toplam görüntülenme ile doğrulanmış okuma karşılaştırması.", "Comparison of total views vs verified reads across recently published content.");
    let health_title = tr_admin(tr, lang, "admin.analytics.health_title", "Veritabanı & Motor Sağlığı", "Database & Engine Health");
    let health_desc = tr_admin(tr, lang, "admin.analytics.health_desc", "SQLite WAL katmanı ve tablo yoğunluğu.", "SQLite WAL layer and table density.");
    let refresh_btn = tr_admin(tr, lang, "admin.form.refresh", "Yenile", "Refresh");
    let health_loading = tr_admin(tr, lang, "admin.analytics.health_loading", "Sistem durumu sorgulanıyor...", "Querying system health...");
    let top_title = tr_admin(tr, lang, "admin.analytics.top_contents_title", "En Yüksek Erişim Alan Yayınlar (İlk 10)", "Top Reached Publications (Top 10)");
    let top_desc = tr_admin(tr, lang, "admin.analytics.top_contents_desc", "Tüm portal genelinde en fazla görüntülenen savunma ve havacılık dosyaları.", "Most viewed defence and aerospace dossiers across the entire portal.");

    tab_pane(buf, "tab-9", "tab-btn-9", false, |tab| {
        tab.el("div", &[("class", "workspace-grid media-seo-grid")], |grid| {
            admin_card(
                grid,
                "",
                Some("analytics:read"),
                "fa-area-chart",
                &trend_title,
                Some(&trend_desc),
                |_| {},
                |card| {
                    card.el("div", &[("class", "panel-body chart-container-md")], |bd| {
                        bd.el("canvas", &[("id", "chart-content-trend")], |_| {});
                    });
                },
            );

            admin_card(
                grid,
                "",
                Some("settings:read|analytics:read|system:read"),
                "fa-heartbeat",
                &health_title,
                Some(&health_desc),
                |hdr| action_btn(hdr, "btn-cmd btn-cmd-sm", "refresh-system-health", None, Some("fa-refresh"), &refresh_btn),
                |card| {
                    card.el("div", &[("id", "system-health-panel"), ("class", "panel-body")], |bd| {
                        bd.el("div", &[("class", "text-muted text-sm")], |t| {
                            t.text(&health_loading);
                        });
                    });
                },
            );
        });

        admin_card(
            tab,
            "",
            Some("analytics:read"),
            "fa-bar-chart",
            &top_title,
            Some(&top_desc),
            |_| {},
            |card| {
                card.el("div", &[("class", "panel-body chart-container-xl")], |bd| {
                    bd.el("canvas", &[("id", "chart-top-contents")], |_| {});
                });
            },
        );
    });
}

fn render_tab_lifecycle(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let kpi_churn = tr_admin(tr, lang, "admin.analytics.kpi_churn", "AYLIK KAYIP ORANI (CHURN)", "MONTHLY CHURN RATE");
    let kpi_churn_default = tr_admin(tr, lang, "admin.analytics.kpi_churn_default", "%0,0", "0.0%");
    let kpi_churn_sub = tr_admin(tr, lang, "admin.analytics.kpi_churn_sub", "Son 30 günlük iptal eğilimi", "Last 30 days cancellation trend");
    let kpi_lifetime = tr_admin(tr, lang, "admin.analytics.kpi_lifetime", "ORTALAMA ABONELİK ÖMRÜ", "AVG. SUBSCRIPTION LIFETIME");
    let kpi_lifetime_default = tr_admin(tr, lang, "admin.analytics.kpi_lifetime_default", "0 gün", "0 days");
    let kpi_lifetime_sub = tr_admin(tr, lang, "admin.analytics.kpi_lifetime_sub", "Aktif kurumsal & bireysel", "Active corporate & individual");
    let kpi_ltv = tr_admin(tr, lang, "admin.analytics.kpi_ltv", "YAŞAM BOYU DEĞER (LTV)", "LIFETIME VALUE (LTV)");
    let kpi_ltv_sub = tr_admin(tr, lang, "admin.analytics.kpi_ltv_sub", "Kullanıcı başı net katkı", "Net contribution per user");
    let kpi_paying = tr_admin(tr, lang, "admin.analytics.kpi_paying", "ÖDEYEN AKTİF ABONE", "PAYING ACTIVE SUBSCRIBERS");
    let kpi_paying_sub = tr_admin(tr, lang, "admin.analytics.kpi_paying_sub", "Doğrulanmış faturalı üye", "Verified invoiced member");

    let funnel_title = tr_admin(tr, lang, "admin.analytics.funnel_title", "Okuyucu Dönüşüm Hunisi", "Reader Conversion Funnel");
    let funnel_desc = tr_admin(tr, lang, "admin.analytics.funnel_desc", "Anonim ziyaretçiden faturalı kurumsal aboneye geçiş basamakları.", "Conversion steps from anonymous visitor to invoiced corporate subscriber.");
    let update_btn = tr_admin(tr, lang, "admin.form.update", "Güncelle", "Update");
    let rates_title = tr_admin(tr, lang, "admin.analytics.rates_title", "Kademe Geçiş Oranları", "Stage Transition Rates");
    let rates_desc = tr_admin(tr, lang, "admin.analytics.rates_desc", "Huni adımları arası dönüşüm verimliliği.", "Conversion efficiency between funnel steps.");
    let rates_loading = tr_admin(tr, lang, "admin.analytics.rates_loading", "Dönüşüm oranları hesaplanıyor...", "Calculating conversion rates...");

    let cohort_title = tr_admin(tr, lang, "admin.analytics.cohort_title", "Aylık Kohort Tutundurma Matrisi", "Monthly Cohort Retention Matrix");
    let cohort_desc = tr_admin(tr, lang, "admin.analytics.cohort_desc", "Kayıt olunan aya göre takip eden 6 aylık aktif okuma devamlılığı.", "6-month active reading retention following registration month.");
    let col_cohort_period = tr_admin(tr, lang, "admin.analytics.col_cohort_period", "Kayıt Dönemi", "Cohort Period");
    let col_new_members = tr_admin(tr, lang, "admin.analytics.col_new_members", "Yeni Üye", "New Users");
    let col_m1 = tr_admin(tr, lang, "admin.analytics.col_m1", "1. Ay", "M1");
    let col_m2 = tr_admin(tr, lang, "admin.analytics.col_m2", "2. Ay", "M2");
    let col_m3 = tr_admin(tr, lang, "admin.analytics.col_m3", "3. Ay", "M3");
    let col_m4 = tr_admin(tr, lang, "admin.analytics.col_m4", "4. Ay", "M4");
    let col_m5 = tr_admin(tr, lang, "admin.analytics.col_m5", "5. Ay", "M5");
    let col_m6 = tr_admin(tr, lang, "admin.analytics.col_m6", "6. Ay", "M6");
    let cohort_loading = tr_admin(tr, lang, "admin.analytics.cohort_loading", "Kohort analizi yükleniyor...", "Loading cohort analysis...");

    tab_pane(buf, "tab-15", "tab-btn-15", false, |tab| {
        tab.el("div", &[("class", "kpi-grid mb-4")], |kg| {
            kpi_card_toned(kg, "tone-accent", Some("fa-line-chart"), None, &kpi_churn, "kpi-churn-rate", &kpi_churn_default, "", None, Some(&kpi_churn_sub));
            kpi_card_toned(kg, "tone-info", Some("fa-clock-o"), None, &kpi_lifetime, "kpi-avg-lifetime", &kpi_lifetime_default, "", None, Some(&kpi_lifetime_sub));
            kpi_card_toned(kg, "tone-success", Some("fa-diamond"), None, &kpi_ltv, "kpi-avg-ltv", "₺0,00", "", None, Some(&kpi_ltv_sub));
            kpi_card_toned(kg, "tone-warning", Some("fa-credit-card"), None, &kpi_paying, "kpi-paying-users", "0", "", None, Some(&kpi_paying_sub));
        });

        tab.el("div", &[("class", "workspace-grid media-seo-grid")], |grid| {
            admin_card(
                grid,
                "",
                None,
                "fa-filter",
                &funnel_title,
                Some(&funnel_desc),
                |hdr| action_btn(hdr, "btn-cmd btn-cmd-sm", "load-user-analytics", None, Some("fa-refresh"), &update_btn),
                |card| {
                    card.el("div", &[("class", "panel-body chart-container-md")], |bd| {
                        bd.el("canvas", &[("id", "chart-conversion-funnel")], |_| {});
                    });
                },
            );

            admin_card(
                grid,
                "",
                None,
                "fa-random",
                &rates_title,
                Some(&rates_desc),
                |_| {},
                |card| {
                    card.el("div", &[("id", "funnel-rates"), ("class", "panel-body funnel-rates-stack")], |fr| {
                        fr.el("div", &[("class", "text-muted text-sm")], |t| {
                            t.text(&rates_loading);
                        });
                    });
                },
            );
        });

        admin_card(
            tab,
            "",
            None,
            "fa-th-large",
            &cohort_title,
            Some(&cohort_desc),
            |_| {},
            |card| {
                ops_table(
                    card,
                    "",
                    Some("table-cohort-analysis"),
                    &[
                        (&col_cohort_period, ""),
                        (&col_new_members, "text-right"),
                        (&col_m1, "text-right"),
                        (&col_m2, "text-right"),
                        (&col_m3, "text-right"),
                        (&col_m4, "text-right"),
                        (&col_m5, "text-right"),
                        (&col_m6, "text-right"),
                    ],
                    "tbody-cohort-analysis",
                    Some(&cohort_loading),
                    None,
                );
            },
        );
    });
}
