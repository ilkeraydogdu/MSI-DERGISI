//! Admin Command Center Shell: `<head>`, `#admin-sidebar`, `.top-header`,
//! `#command-hero-strip`, `#global-kpi-strip`, tab containers, `#totp-security-modal`,
//! `.ops-toast-region`, `#msi-i18n`, and scripts.

use super::ui::{action_btn, action_btn_extra, form_group, icon, input_ctrl, kpi_card_toned};
use super::{analytics, clean_segment, commercial, editorial, system, tr_admin, workflow};
use crate::engine::design::html_dsl::HtmlBuf;
use crate::engine::translation_engine::TranslationCache;
use serde_json::{Map, Value};

/// Shared CSP-nonced pre-hydration script (explicit preference -> OS color-scheme -> local clock).
pub(super) const THEME_PREHYDRATE_JS: &str =
    crate::engine::design::theme_boot::UNIFIED_THEME_PREHYDRATE_JS;

/// Collects non-empty `admin.*` translation entries as a flat `{ key: value }` JSON map.
pub(super) fn admin_i18n_json(tr: &TranslationCache, lang: &str) -> Value {
    let norm_lang = if lang.eq_ignore_ascii_case("en") { "en" } else { "tr" };
    let mut map = Map::new();
    for entry in tr.all_entries() {
        if entry.key.starts_with("admin.") {
            let val = tr.t(norm_lang, &entry.key);
            if val != entry.key && !val.trim().is_empty() {
                map.insert(entry.key, Value::String(val));
            }
        }
    }
    Value::Object(map)
}

/// Sidebar navigation structure: (group_key, &[(tab_num, icon_name, label_key, perm, has_badge)])
const SIDEBAR_GROUPS: &[(&str, &[(&str, &str, &str, &str, bool)])] = &[
    ("admin.nav.group_editorial", &[
        ("1", "fa-newspaper-o", "admin.nav.contents", "contents:read", false),
        ("18", "fa-book", "admin.nav.issues", "issues:read|issues:delete|contents:read", false),
        ("16", "fa-check-square-o", "admin.nav.approvals", "approvals:read|approvals:manage|approvals:approve|contents:publish", true),
        ("12", "fa-sliders", "admin.nav.bulk_ops", "contents:write", false),
        ("10", "fa-picture-o", "admin.nav.media_seo", "media:read|contents:read", false),
        ("5", "fa-sitemap", "admin.nav.taxonomy", "categories:read", false),
    ]),
    ("admin.nav.group_commercial", &[
        ("2", "fa-bullhorn", "admin.nav.ads", "ads:read", false),
        ("14", "fa-try", "admin.nav.revenue", "revenue:read|commerce:read|subscriptions:read|invoices:read", false),
        ("4", "fa-building-o", "admin.nav.corporate", "corporate_requests:read", false),
    ]),
    ("admin.nav.group_analytics", &[
        ("3", "fa-pie-chart", "admin.nav.analytics", "analytics:read", false),
        ("9", "fa-line-chart", "admin.nav.charts_health", "analytics:read|settings:read|system:read", false),
        ("15", "fa-users", "admin.nav.lifecycle", "analytics:read|revenue:read", false),
    ]),
    ("admin.nav.group_comms", &[
        ("6", "fa-envelope-o", "admin.nav.newsletter", "newsletters:read|email_groups:write|email_groups:manage", false),
    ]),
    ("admin.nav.group_system", &[
        ("8", "fa-cog", "admin.nav.settings", "settings:read|notifications:read|translations:read|glossary:write|glossary:manage|locales:manage", false),
        ("11", "fa-shield", "admin.nav.rbac", "users:read|roles:manage|audit:read", false),
        ("17", "fa-server", "admin.nav.monitoring", "monitoring:read|system:read", false),
        ("7", "fa-exchange", "admin.nav.automation", "subscriptions:read|settings:read|redirects:write|contents:publish", false),
        ("13", "fa-puzzle-piece", "admin.nav.custom_fields", "settings:read|backup:manage", false),
    ]),
];

/// Fallback labels for sidebar groups when translation cache is empty.
const SIDEBAR_GROUP_FALLBACKS: &[(&str, &str, &str)] = &[
    ("admin.nav.group_editorial", "Editoryal & E-Dergi", "Editorial & E-Magazine"),
    ("admin.nav.group_commercial", "Ticari & Reklam", "Commercial & Ads"),
    ("admin.nav.group_analytics", "Birleşik Analitik Merkezi", "Unified Analytics Center"),
    ("admin.nav.group_comms", "İletişim & Bülten", "Communications & Newsletter"),
    ("admin.nav.group_system", "Sistem & Güvenlik", "System & Security"),
];

/// Fallback labels for sidebar items when translation cache is empty.
const SIDEBAR_ITEM_FALLBACKS: &[(&str, &str, &str)] = &[
    ("admin.nav.contents", "İçerik Yönetimi", "Content Management"),
    ("admin.nav.issues", "E-Dergi & Sayılar", "E-Magazine & Issues"),
    ("admin.nav.media_seo", "Medya & SEO", "Media & SEO"),
    ("admin.nav.taxonomy", "Taksonomi & SSB", "Taxonomy & SSB"),
    ("admin.nav.ads", "Reklam Yönetimi", "Ad Management"),
    ("admin.nav.revenue", "Gelir & Mağaza", "Revenue & Store"),
    ("admin.nav.corporate", "Kurumsal Onaylar", "Corporate Approvals"),
    ("admin.nav.analytics", "Kurumsal Analitik", "Corporate Analytics"),
    ("admin.nav.charts_health", "Grafikler & Sağlık", "Charts & Health"),
    ("admin.nav.lifecycle", "Yaşam Döngüsü", "Lifecycle"),
    ("admin.nav.approvals", "Onay Bekleyenler", "Pending Approvals"),
    ("admin.nav.bulk_ops", "Toplu İşlemler", "Bulk Operations"),
    ("admin.nav.newsletter", "Bülten & E-Posta", "Newsletter & Email"),
    ("admin.nav.settings", "Motorlar & Ayarlar", "Engines & Settings"),
    ("admin.nav.rbac", "RBAC & Denetim", "RBAC & Audit"),
    ("admin.nav.monitoring", "Sunucu İzleme", "Server Monitoring"),
    ("admin.nav.automation", "Otomasyon & 301", "Automation & 301"),
    ("admin.nav.custom_fields", "Özel Alan & Önbellek", "Custom Fields & Cache"),
];

const ADMIN_SCRIPTS: &[&str] = &[
    "/js/vendor/gsap.min.js",
    "/js/vendor/chart.umd.min.js",
    "/js/admin/core.js?v=36",
    "/js/admin/editorial.js?v=36",
    "/js/admin/taxonomy-media.js?v=36",
    "/js/admin/issues.js?v=36",
    "/js/admin/commercial.js?v=36",
    "/js/admin/analytics.js?v=36",
    "/js/admin/system-rbac.js?v=36",
    "/js/admin/monitoring.js?v=36",
];

pub fn render_dashboard(
    nonce: Option<&str>,
    clean_admin_path: &str,
    operator_email: &str,
    operator_role: &str,
    tr: &TranslationCache,
    lang: &str,
) -> String {
    let seg = clean_segment(clean_admin_path);
    let admin_root = format!("/{}", seg);
    let clean_nonce = nonce.map(str::trim).filter(|n| !n.is_empty());

    let page_title = tr_admin(tr, lang, "admin.shell.title", "UMSA Komuta — MSI Dergisi & MSI TDR Yayın ve Operasyon Merkezi", "UMSA Command — MSI Review & MSI TDR Publishing & Operations Center");

    let mut buf = HtmlBuf::with_capacity(114688);
    buf.raw("<!DOCTYPE html>");
    buf.el("html", &[("lang", lang), ("data-theme", "light")], |html| {
        html.el("head", &[], |head| {
            head.void_el("meta", &[("charset", "UTF-8")]);
            head.void_el("meta", &[("name", "viewport"), ("content", "width=device-width, initial-scale=1.0")]);
            head.void_el("meta", &[("name", "robots"), ("content", "noindex,nofollow,noarchive")]);
            head.void_el("meta", &[("name", "referrer"), ("content", "same-origin")]);
            head.el("title", &[], |t| {
                t.text(&page_title);
            });
            head.void_el("link", &[("rel", "icon"), ("href", "/favicon.ico"), ("sizes", "any")]);
            head.void_el("link", &[("rel", "icon"), ("type", "image/svg+xml"), ("href", "/favicon.svg")]);
            head.void_el("link", &[("rel", "icon"), ("type", "image/png"), ("sizes", "32x32"), ("href", "/images/favicon-32x32.png")]);
            head.link_css("/css/design-tokens.css")
                .link_css("/css/admin/tokens.css?v=36")
                .link_css("/css/admin/layout.css?v=36")
                .link_css("/css/admin/components.css?v=36");
            head.script_inline(THEME_PREHYDRATE_JS, clean_nonce, None);
        });

        html.el(
            "body",
            &[
                ("data-admin-path", &admin_root),
                ("data-operator-email", operator_email.trim()),
                ("data-operator-role", operator_role.trim()),
                ("data-admin-lang", lang),
            ],
            |body| {
                body.el("div", &[("class", "app-layout")], |app| {
                    app.el("div", &[("class", "sidebar-overlay"), ("id", "sidebar-overlay")], |_| {});
                    render_sidebar(app, seg, operator_email, operator_role, tr, lang);
                    app.el("div", &[("class", "main-content")], |main| {
                        render_top_header(main, tr, lang);
                        main.el("main", &[("class", "page-content")], |page| {
                            render_hero_strip(page, seg, operator_email, operator_role, tr, lang);
                            render_kpi_strip(page, tr, lang);
                            page.el("div", &[("class", "tabs-content")], |tabs| {
                                editorial::render(tabs, seg, tr, lang);
                                commercial::render(tabs, tr, lang);
                                analytics::render(tabs, tr, lang);
                                workflow::render(tabs, tr, lang);
                                system::render(tabs, seg, tr, lang);
                            });
                        });
                    });
                });

                render_totp_modal(body, tr, lang);
                render_confirm_modal(body, tr, lang);
                body.el("div", &[("id", "ops-toast-region"), ("class", "ops-toast-region"), ("aria-live", "polite"), ("aria-relevant", "additions")], |_| {});
                body.json_script("msi-i18n", &admin_i18n_json(tr, lang), clean_nonce);
                for src in ADMIN_SCRIPTS {
                    body.script_src(src, clean_nonce, true);
                }
            },
        );
    });
    buf.into_string()
}

fn render_sidebar(
    buf: &mut HtmlBuf,
    seg: &str,
    operator_email: &str,
    operator_role: &str,
    tr: &TranslationCache,
    lang: &str,
) {
    let editor_href = format!("/{}/editor", seg);
    let studio_href = format!("/{}/tasarim", seg);
    let brand_text = tr_admin(tr, lang, "admin.shell.brand", "UMSA KOMUTA", "UMSA COMMAND");
    let brand_sub = tr_admin(tr, lang, "admin.shell.brand_sub", "Yayın & Operasyon Merkezi", "Publishing & Operations Center");
    let search_ph = tr_admin(tr, lang, "admin.sidebar.search_placeholder", "Modül filtrele...", "Filter modules...");
    let search_aria = tr_admin(tr, lang, "admin.sidebar.search_aria", "Yönetim modüllerini filtrele", "Filter admin modules");
    let nav_aria = tr_admin(tr, lang, "admin.sidebar.nav_aria", "Yönetim bölümleri", "Admin sections");
    let editor_label = tr_admin(tr, lang, "admin.sidebar.editorial_studio", "Editör Stüdyosu ↗", "Editorial Studio ↗");
    let studio_label = tr_admin(tr, lang, "admin.sidebar.studio", "Tasarım Stüdyosu ↗", "Design Studio ↗");
    let close_menu_aria = tr_admin(tr, lang, "admin.header.close_menu", "Menüyü Kapat", "Close menu");
    let operator_label = tr_admin(tr, lang, "admin.header.operator", "Operatör", "Operator");
    let role_label = tr_admin(tr, lang, "admin.header.role", "Yetkili", "Authorized");
    let logout_title = tr_admin(tr, lang, "admin.header.logout", "Güvenli Çıkış", "Secure Logout");
    let logout_text = tr_admin(tr, lang, "admin.header.logout_label", "Güvenli Çıkış", "Secure Logout");
    let init_email = if operator_email.trim().is_empty() {
        operator_label.as_str()
    } else {
        operator_email.trim()
    };
    let init_role = if operator_role.trim().is_empty() {
        role_label.as_str()
    } else {
        operator_role.trim()
    };

    buf.el("aside", &[("class", "sidebar"), ("id", "admin-sidebar")], |sb| {
        sb.el("div", &[("class", "sidebar-brand")], |br| {
            br.el("div", &[("class", "sidebar-brand-main")], |bm| {
                bm.el("div", &[("class", "sidebar-brand-mark")], |mk| {
                    mk.void_el("img", &[
                        ("src", "/images/logo.svg"),
                        ("alt", "MSI Dergisi"),
                        ("class", "sidebar-brand-logo"),
                        ("width", "86"),
                        ("height", "32"),
                        ("decoding", "async"),
                    ]);
                });
                bm.el("div", &[("class", "sidebar-brand-copy")], |cp| {
                    cp.el("div", &[("class", "sidebar-brand-text")], |t| {
                        t.text(&brand_text);
                    });
                    cp.el("div", &[("class", "sidebar-brand-sub")], |s| {
                        s.text(&brand_sub);
                    });
                });
            });
            br.el(
                "button",
                &[
                    ("type", "button"),
                    ("id", "sidebar-close-btn"),
                    ("class", "btn-icon sidebar-mobile-close"),
                    ("aria-label", &close_menu_aria),
                ],
                |b| icon(b, "fa-times"),
            );
        });

        sb.el("div", &[("class", "sidebar-search-wrap")], |sw| {
            icon(sw, "fa-search");
            sw.void_el("input", &[
                ("type", "search"),
                ("id", "sidebar-quick-filter"),
                ("class", "sidebar-search-input"),
                ("placeholder", &search_ph),
                ("aria-label", &search_aria),
            ]);
        });

        sb.el("nav", &[("class", "sidebar-nav"), ("role", "tablist"), ("aria-label", &nav_aria)], |nav| {
            for (group_key, items) in SIDEBAR_GROUPS {
                let group_label = lookup_fallback(SIDEBAR_GROUP_FALLBACKS, group_key, tr, lang);
                nav.el("div", &[("class", "sidebar-group"), ("data-group-name", group_key), ("role", "presentation")], |grp| {
                    grp.el("div", &[("class", "sidebar-group-label")], |lbl| {
                        lbl.text(&group_label);
                    });
                    for (num, fa, label_key, perm, has_badge) in *items {
                        let item_label = lookup_fallback(SIDEBAR_ITEM_FALLBACKS, label_key, tr, lang);
                        let active = *num == "1";
                        let tab_id = format!("tab-{}", num);
                        let btn_id = format!("tab-btn-{}", num);
                        grp.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("id", &btn_id),
                                ("class", if active { "sidebar-link active" } else { "sidebar-link" }),
                                ("data-tab", &tab_id),
                                ("data-group", group_key),
                                ("data-required-perm", perm),
                                ("title", &item_label),
                                ("role", "tab"),
                                ("aria-selected", if active { "true" } else { "false" }),
                                ("aria-controls", &tab_id),
                            ],
                            |b| {
                                icon(b, fa);
                                b.el("span", &[("class", "sidebar-link-label")], |s| {
                                    s.text(&item_label);
                                });
                                if *has_badge {
                                    b.el("span", &[("id", "approval-badge"), ("class", "badge-pill hidden")], |bg| {
                                        bg.text("0");
                                    });
                                }
                            },
                        );
                        if *num == "1" {
                            grp.el(
                                "a",
                                &[
                                    ("href", &editor_href),
                                    ("data-group", group_key),
                                    ("data-required-perm", "contents:write"),
                                    ("class", "sidebar-link"),
                                    ("title", &editor_label),
                                ],
                                |a| {
                                    icon(a, "fa-pencil-square-o");
                                    a.el("span", &[("class", "sidebar-link-label")], |s| {
                                        s.text(&editor_label);
                                    });
                                },
                            );
                        }
                    }
                });
            }
        });

        sb.el("div", &[("class", "sidebar-footer")], |ft| {
            ft.el("div", &[("class", "sidebar-studio-wrap"), ("data-required-perm", "settings:read")], |sw| {
                sw.el(
                    "a",
                    &[("href", &studio_href), ("data-required-perm", "settings:read"), ("class", "sidebar-link sidebar-studio-link"), ("title", &studio_label)],
                    |a| {
                        icon(a, "fa-paint-brush");
                        a.el("span", &[("class", "sidebar-link-label")], |s| {
                            s.text(&studio_label);
                        });
                    },
                );
            });
            ft.el("div", &[("class", "sidebar-user-card"), ("id", "operator-identity-badge")], |card| {
                card.el("div", &[("class", "sidebar-user-meta")], |meta| {
                    meta.el("span", &[("class", "operator-dot"), ("aria-hidden", "true")], |_| {});
                    meta.el("div", &[("class", "sidebar-user-copy")], |cp| {
                        cp.el("div", &[("id", "operator-email-display"), ("class", "sidebar-user-email")], |e| {
                            e.text(init_email);
                        });
                        cp.el("span", &[("class", "operator-role-tag"), ("id", "operator-role-display")], |r| {
                            r.text(init_role);
                        });
                    });
                });
                card.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("id", "btn-logout"),
                        ("class", "sidebar-logout-btn"),
                        ("data-admin-action", "logout-operator"),
                        ("title", &logout_title),
                        ("aria-label", &logout_title),
                    ],
                    |b| {
                        icon(b, "fa-sign-out");
                        b.el("span", &[("class", "logout-label")], |s| {
                            s.text(&logout_text);
                        });
                    },
                );
            });
        });
    });
}

fn lookup_fallback(fallbacks: &[(&str, &str, &str)], key: &str, tr: &TranslationCache, lang: &str) -> String {
    for (k, fb_tr, fb_en) in fallbacks {
        if *k == key {
            return tr_admin(tr, lang, key, fb_tr, fb_en);
        }
    }
    tr.t(lang, key)
}

fn render_top_header(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let toggle_aria = tr_admin(tr, lang, "admin.header.toggle_menu", "Menüyü Aç/Kapat", "Toggle menu");
    let default_group = tr_admin(tr, lang, "admin.nav.group_editorial", "Editoryal & E-Dergi", "Editorial & E-Magazine");
    let default_page = tr_admin(tr, lang, "admin.nav.contents", "İçerik Yönetimi", "Content Management");
    let refresh_title = tr_admin(tr, lang, "admin.header.refresh", "Verileri Yenile", "Refresh data");
    let theme_title = tr_admin(tr, lang, "admin.header.theme", "Açık / Koyu Tema", "Light / Dark Theme");
    let theme_aria = tr_admin(tr, lang, "admin.header.theme_aria", "Tema Değiştir", "Toggle theme");
    let tfa_title = tr_admin(tr, lang, "admin.header.2fa_title", "İki Faktörlü Doğrulama (2FA)", "Two-Factor Authentication (2FA)");
    let tfa_aria = tr_admin(tr, lang, "admin.header.2fa_aria", "2FA Ayarları", "2FA Settings");
    let site_tr = tr_admin(tr, lang, "admin.header.site_tr", "MSI TR ↗", "MSI TR ↗");
    let site_en = tr_admin(tr, lang, "admin.header.site_en", "TDR EN ↗", "TDR EN ↗");

    buf.el("header", &[("class", "top-header")], |hdr| {
        hdr.el("div", &[("class", "header-left")], |left| {
            left.el(
                "button",
                &[("type", "button"), ("id", "sidebar-toggle"), ("aria-label", &toggle_aria), ("class", "btn-icon sidebar-mobile-btn")],
                |b| icon(b, "fa-bars"),
            );
            left.el("div", &[("class", "header-breadcrumbs")], |bc| {
                bc.el("span", &[("class", "header-crumb"), ("id", "page-group-label")], |c| {
                    c.text(&default_group);
                });
                bc.el("span", &[("class", "header-sep")], |s| {
                    s.text("/");
                });
                bc.el("h1", &[("class", "header-title"), ("id", "page-title")], |h| {
                    h.text(&default_page);
                });
            });
        });

        hdr.el("div", &[("class", "header-right")], |right| {
            for (btn_id, action, title, aria, fa) in [
                ("topbar-refresh-btn", "refresh-all", refresh_title.as_str(), refresh_title.as_str(), "fa-refresh"),
                ("topbar-theme-toggle", "toggle-theme", theme_title.as_str(), theme_aria.as_str(), "fa-adjust"),
                ("topbar-2fa-btn", "open-2fa-modal", tfa_title.as_str(), tfa_aria.as_str(), "fa-key"),
            ] {
                right.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("id", btn_id),
                        ("class", "btn-icon"),
                        ("data-admin-action", action),
                        ("title", title),
                        ("aria-label", aria),
                    ],
                    |b| icon(b, fa),
                );
            }
            right.el("div", &[("class", "site-links-pill")], |pill| {
                pill.el("a", &[("href", "/"), ("target", "_blank"), ("rel", "noopener")], |a| {
                    a.text(&site_tr);
                });
                pill.el("span", &[("class", "pill-divider")], |d| {
                    d.text("·");
                });
                pill.el("a", &[("href", "/en"), ("target", "_blank"), ("rel", "noopener")], |a| {
                    a.text(&site_en);
                });
            });
        });
    });
}

fn render_hero_strip(
    buf: &mut HtmlBuf,
    seg: &str,
    operator_email: &str,
    operator_role: &str,
    tr: &TranslationCache,
    lang: &str,
) {
    let editor_href = format!("/{}/editor", seg);
    let studio_href = format!("/{}/tasarim", seg);
    let eyebrow = tr_admin(tr, lang, "admin.hero.eyebrow", "MSI KOMUTA MERKEZİ", "MSI COMMAND CENTER");
    let welcome = tr_admin(tr, lang, "admin.hero.welcome", "Hoş Geldiniz", "Welcome");
    let subtitle = tr_admin(
        tr,
        lang,
        "admin.hero.subtitle",
        "Yayın akışı, B2B savunma analitiği ve yetkilendirilmiş modülleriniz tek merkezden yönetilir.",
        "Manage editorial workflows, B2B defence analytics and your authorised modules from one hub.",
    );
    let cta_new_content = tr_admin(tr, lang, "admin.hero.cta_new_content", "Yeni İçerik", "New Content");
    let cta_editor = tr_admin(tr, lang, "admin.hero.cta_editorial_studio", "Editör Stüdyosu ↗", "Editorial Studio ↗");
    let cta_approvals = tr_admin(tr, lang, "admin.hero.cta_approvals", "Onay Kuyruğu", "Approval Queue");
    let cta_issues = tr_admin(tr, lang, "admin.hero.cta_issues", "E-Dergi Arşivi", "E-Magazine Archive");
    let cta_new_ad = tr_admin(tr, lang, "admin.hero.cta_new_ad", "Yeni Kampanya", "New Campaign");
    let cta_revenue = tr_admin(tr, lang, "admin.hero.cta_revenue", "Gelir & Faturalar", "Revenue & Invoices");
    let cta_corporate = tr_admin(tr, lang, "admin.hero.cta_corporate", "Kurumsal Onaylar", "Corporate Approvals");
    let cta_users = tr_admin(tr, lang, "admin.hero.cta_users", "Kullanıcı & RBAC", "Users & RBAC");
    let cta_analytics = tr_admin(tr, lang, "admin.hero.cta_analytics", "B2B Analitik", "B2B Analytics");
    let cta_health = tr_admin(tr, lang, "admin.hero.cta_health", "Sistem Sağlığı", "System Health");
    let cta_studio = tr_admin(tr, lang, "admin.hero.cta_studio", "Tasarım Stüdyosu ↗", "Design Studio ↗");
    let cta_all_actions = tr_admin(tr, lang, "admin.hero.all_actions", "Tüm İşlemler", "All Actions");
    let status_title = tr_admin(tr, lang, "admin.hero.status_title", "Oturum & Yetki Durumu", "Session & Access Status");
    let status_ok = tr_admin(tr, lang, "admin.hero.status_active", "Aktif Oturum · RBAC Korumalı", "Active Session · RBAC Protected");
    let modules_lbl = tr_admin(tr, lang, "admin.hero.modules_label", "Erişilebilir Modül", "Accessible Modules");
    let display_name = if operator_email.trim().is_empty() {
        tr_admin(tr, lang, "admin.header.operator", "Operatör", "Operator")
    } else {
        operator_email.trim().to_string()
    };
    let display_role = if operator_role.trim().is_empty() {
        tr_admin(tr, lang, "admin.header.role", "Yetkili", "Authorized")
    } else {
        operator_role.trim().to_string()
    };

    buf.el("section", &[("id", "command-hero-strip"), ("class", "command-hero-grid"), ("aria-label", &eyebrow)], |hero| {
        hero.el("div", &[("class", "hero-welcome-card"), ("data-glow-card", "")], |wc| {
            wc.el("div", &[("class", "hero-copy")], |cp| {
                cp.el("div", &[("class", "hero-eyebrow-row")], |er| {
                    er.el("span", &[("class", "hero-eyebrow")], |e| { e.text(&eyebrow); });
                    er.el("span", &[("id", "hero-role-brief"), ("class", "hero-role-brief")], |rb| {
                        rb.text(&display_role);
                    });
                });
                cp.el("h2", &[("class", "hero-heading")], |h| {
                    h.text(&welcome);
                    h.raw(", ");
                    h.el("span", &[("id", "hero-operator-name")], |n| { n.text(&display_name); });
                });
                cp.el("p", &[("id", "hero-role-subtitle"), ("class", "hero-subtitle")], |p| { p.text(&subtitle); });
            });
            wc.el("div", &[("class", "hero-actions"), ("id", "hero-quick-actions")], |act| {
                action_btn_extra(act, "btn-hero btn-hero-primary", "hero-open-content-drawer", Some("contents:write"), Some("fa-plus"), &cta_new_content, &[("data-hero-shortcut", "1"), ("data-hero-order", "0")]);
                act.el(
                    "a",
                    &[
                        ("href", &editor_href),
                        ("class", "btn-hero btn-hero-ghost"),
                        ("data-required-perm", "contents:write"),
                        ("data-hero-shortcut", "1"),
                        ("data-hero-order", "1"),
                    ],
                    |a| { icon(a, "fa-pencil-square-o"); a.raw(" "); a.text(&cta_editor); },
                );
                action_btn_extra(act, "btn-hero btn-hero-primary", "open-ad-drawer", Some("ads:write"), Some("fa-bullhorn"), &cta_new_ad, &[("data-hero-shortcut", "1"), ("data-hero-order", "2")]);
                act.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "btn-hero btn-hero-ghost"),
                        ("data-tab", "tab-16"),
                        ("data-required-perm", "approvals:read|approvals:manage|approvals:approve|contents:publish"),
                        ("data-hero-shortcut", "1"),
                        ("data-hero-order", "3"),
                    ],
                    |b| { icon(b, "fa-check-square-o"); b.raw(" "); b.text(&cta_approvals); },
                );
                act.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "btn-hero btn-hero-ghost"),
                        ("data-tab", "tab-18"),
                        ("data-required-perm", "issues:read|issues:delete|contents:read"),
                        ("data-hero-shortcut", "1"),
                        ("data-hero-order", "4"),
                    ],
                    |b| { icon(b, "fa-book"); b.raw(" "); b.text(&cta_issues); },
                );
                act.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "btn-hero btn-hero-ghost"),
                        ("data-tab", "tab-14"),
                        ("data-required-perm", "revenue:read|commerce:read|invoices:read"),
                        ("data-hero-shortcut", "1"),
                        ("data-hero-order", "5"),
                    ],
                    |b| { icon(b, "fa-try"); b.raw(" "); b.text(&cta_revenue); },
                );
                act.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "btn-hero btn-hero-ghost"),
                        ("data-tab", "tab-4"),
                        ("data-required-perm", "corporate_requests:read"),
                        ("data-hero-shortcut", "1"),
                        ("data-hero-order", "6"),
                    ],
                    |b| { icon(b, "fa-building-o"); b.raw(" "); b.text(&cta_corporate); },
                );
                act.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "btn-hero btn-hero-ghost"),
                        ("data-tab", "tab-11"),
                        ("data-required-perm", "users:read|roles:manage"),
                        ("data-hero-shortcut", "1"),
                        ("data-hero-order", "7"),
                    ],
                    |b| { icon(b, "fa-users"); b.raw(" "); b.text(&cta_users); },
                );
                act.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "btn-hero btn-hero-ghost"),
                        ("data-tab", "tab-3"),
                        ("data-required-perm", "analytics:read"),
                        ("data-hero-shortcut", "1"),
                        ("data-hero-order", "8"),
                    ],
                    |b| { icon(b, "fa-pie-chart"); b.raw(" "); b.text(&cta_analytics); },
                );
                act.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "btn-hero btn-hero-ghost"),
                        ("data-tab", "tab-9"),
                        ("data-required-perm", "analytics:read|settings:read|system:read"),
                        ("data-hero-shortcut", "1"),
                        ("data-hero-order", "9"),
                    ],
                    |b| { icon(b, "fa-heartbeat"); b.raw(" "); b.text(&cta_health); },
                );
                act.el(
                    "a",
                    &[
                        ("href", &studio_href),
                        ("class", "btn-hero btn-hero-ghost"),
                        ("data-required-perm", "settings:read"),
                        ("data-hero-shortcut", "1"),
                        ("data-hero-order", "10"),
                    ],
                    |a| { icon(a, "fa-paint-brush"); a.raw(" "); a.text(&cta_studio); },
                );
                act.el("div", &[("id", "hero-overflow-wrap"), ("class", "hero-overflow-wrap hidden")], |ov| {
                    ov.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("id", "hero-overflow-toggle"),
                            ("class", "btn-hero btn-hero-ghost hero-overflow-btn"),
                            ("aria-haspopup", "menu"),
                            ("aria-expanded", "false"),
                            ("aria-controls", "hero-overflow-menu"),
                        ],
                        |b| {
                            b.text(&cta_all_actions);
                            b.raw(" ");
                            b.el("span", &[("id", "hero-overflow-count"), ("class", "hero-overflow-badge")], |c| {
                                c.text("0");
                            });
                            b.raw(" ");
                            icon(b, "chevron-down");
                        },
                    );
                    ov.el(
                        "div",
                        &[
                            ("id", "hero-overflow-menu"),
                            ("class", "hero-overflow-menu hidden"),
                            ("role", "menu"),
                            ("aria-labelledby", "hero-overflow-toggle"),
                        ],
                        |_| {},
                    );
                });
            });
        });

        hero.el("div", &[("class", "hero-status-card"), ("data-glow-card", "")], |sc| {
            sc.el("div", &[("class", "hero-status-top")], |top| {
                top.el("span", &[("class", "hero-status-title")], |t| {
                    icon(t, "fa-shield");
                    t.raw(" ");
                    t.text(&status_title);
                });
                top.el("span", &[("id", "hero-role-badge"), ("class", "operator-role-tag")], |r| {
                    r.text(&display_role);
                });
            });
            sc.el("div", &[("class", "hero-status-metric")], |m| {
                m.el("span", &[("id", "hero-module-count"), ("class", "hero-metric-num tabular-nums")], |n| { n.text("—"); });
                m.el("span", &[("class", "hero-metric-label")], |l| { l.text(&modules_lbl); });
            });
            sc.el("div", &[("class", "hero-status-foot")], |ft| {
                ft.el("span", &[("class", "operator-dot"), ("aria-hidden", "true")], |_| {});
                ft.el("span", &[], |s| { s.text(&status_ok); });
            });
        });
    });
}

fn render_kpi_strip(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let aria = tr_admin(tr, lang, "admin.kpi.aria", "Yönetici özet metrikleri", "Admin summary metrics");
    let kpi_contents = tr_admin(tr, lang, "admin.kpi.contents", "YAYINDAKİ İÇERİK", "PUBLISHED CONTENT");
    let kpi_contents_sub = tr_admin(tr, lang, "admin.kpi.contents_sub", "Çoklu bölüm yayını", "Multi-section publishing");
    let kpi_reads = tr_admin(tr, lang, "admin.kpi.reads", "DOĞRULANMIŞ OKUMA", "VERIFIED READS");
    let kpi_reads_sub = tr_admin(tr, lang, "admin.kpi.reads_sub", "Toplam görüntülenme", "Total impressions");
    let kpi_ads = tr_admin(tr, lang, "admin.kpi.ads", "AKTİF KAMPANYA", "ACTIVE CAMPAIGNS");
    let kpi_ads_sub = tr_admin(tr, lang, "admin.kpi.ads_sub", "Hedefli kurumsal gösterim", "Targeted corporate impressions");
    let kpi_corp = tr_admin(tr, lang, "admin.kpi.corporate", "KURUMSAL BAŞVURU", "CORPORATE REQUESTS");
    let kpi_corp_sub = tr_admin(tr, lang, "admin.kpi.corporate_sub", "Kamu & kurum doğrulama", "Public & institutional verification");

    buf.el(
        "section",
        &[
            ("id", "global-kpi-strip"),
            ("class", "kpi-strip"),
            ("data-hide-tabs", "tab-14,tab-15,tab-17"),
            ("aria-label", &aria),
        ],
        |s| {
            kpi_card_toned(s, "tone-accent", Some("fa-newspaper-o"), Some("contents:read"), &kpi_contents, "kpi-total-contents", "—", "", Some("kpi-placements-sub"), Some(&kpi_contents_sub));
            kpi_card_toned(s, "tone-info", Some("fa-eye"), Some("analytics:read|contents:read"), &kpi_reads, "kpi-true-reads", "—", "", Some("kpi-views-sub"), Some(&kpi_reads_sub));
            kpi_card_toned(s, "tone-warning", Some("fa-bullhorn"), Some("ads:read"), &kpi_ads, "kpi-active-ads", "—", "", Some("kpi-ads-sub"), Some(&kpi_ads_sub));
            kpi_card_toned(s, "tone-success", Some("fa-building-o"), Some("corporate_requests:read"), &kpi_corp, "kpi-gov-requests", "—", "", Some("kpi-gov-sub"), Some(&kpi_corp_sub));
        },
    );
}

fn render_totp_modal(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let title = tr_admin(tr, lang, "admin.totp.title", "İki Faktörlü Kimlik Doğrulama (2FA)", "Two-Factor Authentication (2FA)");
    let subtitle = tr_admin(tr, lang, "admin.totp.subtitle", "Google Authenticator, Microsoft Authenticator veya 1Password ile eşleştirin.", "Pair with Google Authenticator, Microsoft Authenticator or 1Password.");
    let close_aria = tr_admin(tr, lang, "admin.totp.close_aria", "2FA penceresini kapat", "Close 2FA window");
    let init_desc = tr_admin(tr, lang, "admin.totp.init_desc", "Operatör hesabınız için yeni bir kriptografik TOTP anahtarı ve QR kodu üretin.", "Generate a new cryptographic TOTP key and QR code for your operator account.");
    let init_btn = tr_admin(tr, lang, "admin.totp.start_setup", "QR Kod Üret ve 2FA Kurulumunu Başlat", "Generate QR Code and Start 2FA Setup");
    let qr_alt = tr_admin(tr, lang, "admin.totp.qr_alt", "2FA QR Kodu", "2FA QR Code");
    let secret_label = tr_admin(tr, lang, "admin.totp.secret_label", "Manuel Kurulum Anahtarı", "Manual Setup Key");
    let backup_title = tr_admin(tr, lang, "admin.totp.backup_codes", "YEDEK KURTARMA KODLARI:", "BACKUP RECOVERY CODES:");
    let verify_label = tr_admin(tr, lang, "admin.totp.verify_label", "Doğrulama Kodu", "Verification Code");
    let verify_ph = tr_admin(tr, lang, "admin.totp.verify_placeholder", "6 haneli doğrulama kodu", "6-digit verification code");
    let verify_btn = tr_admin(tr, lang, "admin.totp.verify_btn", "Doğrula ve 2FA'yı Etkinleştir", "Verify and Enable 2FA");
    let disable_title = tr_admin(tr, lang, "admin.totp.disable_title", "MEVCUT 2FA KORUMASINI DEVRE DIŞI BIRAK", "DISABLE CURRENT 2FA PROTECTION");
    let disable_ph = tr_admin(tr, lang, "admin.totp.disable_password_ph", "Mevcut parolanızı doğrulayın", "Confirm your current password");
    let disable_btn = tr_admin(tr, lang, "admin.totp.disable_btn", "Devre Dışı Bırak", "Disable");

    buf.el(
        "div",
        &[
            ("id", "totp-security-modal"),
            ("class", "modal-overlay hidden"),
            ("role", "dialog"),
            ("aria-modal", "true"),
            ("aria-hidden", "true"),
            ("aria-labelledby", "totp-modal-title"),
        ],
        |ov| {
            ov.el("div", &[("class", "modal-content admin-card")], |mc| {
                mc.el("div", &[("class", "panel-header")], |hdr| {
                    hdr.el("div", &[], |l| {
                        l.el("div", &[("class", "panel-title"), ("id", "totp-modal-title")], |t| {
                            icon(t, "fa-key");
                            t.raw(" ");
                            t.text(&title);
                        });
                        l.el("div", &[("class", "panel-subtitle")], |s| {
                            s.text(&subtitle);
                        });
                    });
                    hdr.el(
                        "button",
                        &[("type", "button"), ("class", "btn-icon"), ("id", "btn-close-2fa-modal"), ("data-admin-action", "close-2fa-modal"), ("aria-label", &close_aria)],
                        |b| icon(b, "fa-times"),
                    );
                });
                mc.el("div", &[("id", "totp-setup-stage-init"), ("class", "panel-body")], |st| {
                    st.el("p", &[("class", "text-sm text-secondary mb-3")], |p| {
                        p.text(&init_desc);
                    });
                    action_btn(st, "btn-cmd btn-cmd-primary w-full flex-center", "start-2fa-setup", None, Some("fa-qrcode"), &init_btn);
                });
                mc.el("div", &[("id", "totp-setup-stage-verify"), ("class", "hidden panel-body")], |st| {
                    st.el("div", &[("class", "totp-qr-box")], |qr| {
                        qr.void_el("img", &[("id", "totp-qr-img"), ("alt", &qr_alt), ("src", "")]);
                    });
                    form_group(st, Some("totp-secret-display"), &secret_label, |g| {
                        g.void_with("input", |a| {
                            a.attr("type", "text")
                                .attr("id", "totp-secret-display")
                                .attr("class", "form-control text-mono text-sm")
                                .bool_attr("readonly", true);
                        });
                    });
                    st.el("div", &[("id", "totp-backup-codes-wrap"), ("class", "hidden seo-preview-box")], |bw| {
                        bw.el("div", &[("class", "text-xs font-bold text-warning mb-1")], |d| {
                            d.text(&backup_title);
                        });
                        bw.el("pre", &[("id", "totp-backup-codes-list"), ("class", "text-mono text-xs pre-wrap")], |_| {});
                    });
                    st.el_with("form", |a| { a.attr("id", "form-totp-verify-setup").bool_attr("novalidate", true); }, |f| {
                        form_group(f, Some("totp-setup-code"), &verify_label, |g| {
                            input_ctrl(g, "text", "totp-setup-code", "text-mono", "", &verify_ph, &[("minlength", "6"), ("maxlength", "20"), ("autocomplete", "one-time-code")]);
                        });
                        f.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-success w-full flex-center")], |b| {
                            icon(b, "fa-check");
                            b.raw(" ");
                            b.text(&verify_btn);
                        });
                    });
                });
                mc.el("div", &[("class", "panel-body border-top")], |dis| {
                    dis.el("div", &[("class", "text-xs font-semibold text-muted mb-2")], |d| {
                        d.text(&disable_title);
                    });
                    dis.el_with("form", |a| { a.attr("id", "form-totp-disable").attr("class", "flex gap-2 flex-wrap").bool_attr("novalidate", true); }, |f| {
                        input_ctrl(f, "password", "totp-disable-password", "", "", &disable_ph, &[("autocomplete", "current-password")]);
                        f.el("button", &[("type", "submit"), ("class", "btn-cmd btn-cmd-danger")], |b| {
                            b.text(&disable_btn);
                        });
                    });
                });
            });
        },
    );
}

fn render_confirm_modal(buf: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let default_title = tr_admin(tr, lang, "admin.confirm.title", "İşlem Onayı", "Confirm Action");
    let cancel_btn = tr_admin(tr, lang, "admin.form.cancel", "Vazgeç", "Cancel");
    let confirm_btn = tr_admin(tr, lang, "admin.confirm.accept", "Onayla", "Confirm");
    let close_aria = tr_admin(tr, lang, "admin.form.close", "Kapat", "Close");

    buf.el(
        "div",
        &[
            ("id", "ops-confirm-modal"),
            ("class", "modal-overlay hidden"),
            ("role", "dialog"),
            ("aria-modal", "true"),
            ("aria-hidden", "true"),
            ("aria-labelledby", "ops-confirm-title"),
            ("aria-describedby", "ops-confirm-message"),
        ],
        |ov| {
            ov.el("div", &[("class", "modal-content admin-card modal-confirm-card")], |mc| {
                mc.el("div", &[("class", "panel-header")], |hdr| {
                    hdr.el("div", &[("class", "panel-title"), ("id", "ops-confirm-title")], |t| {
                        icon(t, "fa-exclamation-triangle");
                        t.raw(" ");
                        t.el("span", &[("id", "ops-confirm-title-text")], |tx| {
                            tx.text(&default_title);
                        });
                    });
                    hdr.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "btn-icon"),
                            ("id", "ops-confirm-close"),
                            ("aria-label", &close_aria),
                        ],
                        |b| icon(b, "fa-times"),
                    );
                });
                mc.el("div", &[("class", "panel-body")], |bd| {
                    bd.el("p", &[("id", "ops-confirm-message"), ("class", "text-sm text-secondary")], |_| {});
                });
                mc.el("div", &[("class", "modal-footer")], |ft| {
                    ft.el("button", &[("type", "button"), ("class", "btn-cmd"), ("id", "ops-confirm-cancel")], |b| {
                        b.text(&cancel_btn);
                    });
                    ft.el("button", &[("type", "button"), ("class", "btn-cmd btn-cmd-danger"), ("id", "ops-confirm-accept")], |b| {
                        b.text(&confirm_btn);
                    });
                });
            });
        },
    );
}
