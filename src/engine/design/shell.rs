//! Pure-Rust SSR master shell renderer using `html_dsl::HtmlBuf`.
//! Zero `include_str!("shell.html")`, zero Bootstrap CSS, zero string-replace CSP patches.

use super::{
    components::{
        empty_state, enrich_issue_spreads, floating_popup_ad, icon, link, localized,
        notification_item,
    },
    config,
    dialogs::{self, DialogContext},
    html_dsl::HtmlBuf,
    labels_helper::{
        i18n_bundle_json, is_en_content_ready, lang_href, localize_section_copy, tr_text,
        InstitutionChoice,
    },
};
use crate::engine::{
    security_engine::escape_html as esc,
    seo_router_engine::{self, SeoBrandConfig},
    translation_engine::TranslationCache,
};
use crate::models::*;
use std::collections::{HashMap, HashSet};

/// Optional SSR context for centralized translations, dynamic institution options,
/// floating popup ads, and subscriber PDF reader access.
#[derive(Default)]
pub struct ShellOptions<'a> {
    pub tr: Option<&'a TranslationCache>,
    pub institutions: &'a [InstitutionChoice],
    pub floating_popup: Option<&'a AdCampaign>,
    pub can_read_pdf: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    lang: &str,
    mode: &str,
    settings: &HashMap<String, String>,
    seo: &SeoMetaBundle,
    categories: &[CategoryNode],
    tags: &[SsbInterestTag],
    issues: &[MagazineIssue],
    contents: &[ContentItem],
    notifications: &[SystemNotification],
    body: &str,
    _exec_us: u128,
) -> String {
    render_with_nonce(
        lang, mode, settings, seo, categories, tags, issues, contents, notifications, body,
        _exec_us, None,
    )
}

/// Nonce-aware shell renderer keeping backward-compatible signature for existing callers (e.g. `editor.rs`).
#[allow(clippy::too_many_arguments)]
pub fn render_with_nonce(
    lang: &str,
    mode: &str,
    settings: &HashMap<String, String>,
    seo: &SeoMetaBundle,
    categories: &[CategoryNode],
    tags: &[SsbInterestTag],
    issues: &[MagazineIssue],
    contents: &[ContentItem],
    notifications: &[SystemNotification],
    body: &str,
    exec_us: u128,
    nonce: Option<&str>,
) -> String {
    render_full(
        lang,
        mode,
        settings,
        seo,
        categories,
        tags,
        issues,
        contents,
        notifications,
        body,
        exec_us,
        nonce,
        &ShellOptions::default(),
    )
}

/// Full pure-Rust SSR master shell renderer with `TranslationCache`, dynamic institution
/// list, floating popup ad support, and subscriber PDF viewer gating.
#[allow(clippy::too_many_arguments)]
pub fn render_full(
    lang: &str,
    mode: &str,
    settings: &HashMap<String, String>,
    seo: &SeoMetaBundle,
    categories: &[CategoryNode],
    tags: &[SsbInterestTag],
    issues: &[MagazineIssue],
    contents: &[ContentItem],
    notifications: &[SystemNotification],
    body: &str,
    _exec_us: u128,
    nonce: Option<&str>,
    opts: &ShellOptions<'_>,
) -> String {
    let design = config::load(settings);
    let tr = opts.tr;
    let is_en = lang.eq_ignore_ascii_case("en");
    let featured_issue = settings
        .get("current_featured_issue")
        .and_then(|v| v.parse::<i32>().ok());

    let brand = settings
        .get("brand_title")
        .map(String::as_str)
        .unwrap_or("MSI Dergisi");
    let email = settings
        .get("contact_email")
        .map(String::as_str)
        .unwrap_or("info@savunmahaber.com");
    let phone = settings
        .get("contact_phone")
        .map(String::as_str)
        .unwrap_or("+90 312 225 41 73");
    let fax = settings
        .get("contact_fax")
        .map(String::as_str)
        .unwrap_or("+90 312 225 41 74");
    let address = settings
        .get("contact_address")
        .map(String::as_str)
        .unwrap_or("Yaşamkent Mah. 3147 Sok. No:10 Çankaya / Ankara TÜRKİYE");

    let initial_theme = if design.theme.default_mode == "dark" {
        "dark"
    } else {
        "light"
    };
    let body_class = format!(
        "{}reading-{} category-{} search-{} reading-font-{}",
        if mode == "news" { "show-news " } else { "" },
        design.templates.article_layout,
        design.templates.category_layout,
        design.templates.search_layout,
        design.templates.reading_font
    );

    let public_notifications: Vec<&SystemNotification> = notifications
        .iter()
        .filter(|n| {
            n.target_email.as_deref().unwrap_or("").is_empty()
                && ["all", "public", "global", ""].contains(&n.target_audience.as_str())
        })
        .collect();
    let announcement = settings
        .get("announcement_banner_text")
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .or_else(|| {
            public_notifications
                .iter()
                .find(|n| n.is_active_banner)
                .map(|n| localized(lang, &n.title_tr, &n.title_en).into())
        });

    let enriched_issues: Vec<MagazineIssue> = issues
        .iter()
        .map(|iss| {
            let mut cloned = iss.clone();
            cloned.spreads = enrich_issue_spreads(iss, contents);
            cloned
        })
        .collect();

    let bootstrap_val = serde_json::json!({
        "lang": lang,
        "mode": mode,
        "theme": design.theme.default_mode,
        "issues": enriched_issues,
        "tags": tags,
        "featuredIssue": featured_issue,
    });
    let i18n_val = i18n_bundle_json(tr, lang);
    let seo_head_html = seo_router_engine::render_seo_head_tags(
        seo,
        None,
        &SeoBrandConfig::from_settings(settings),
    );
    let seo_title = localize_section_copy(lang, &seo.title);

    let mut buf = HtmlBuf::with_capacity(24_576);
    buf.raw("<!doctype html>");
    buf.el(
        "html",
        &[
            ("lang", lang),
            ("data-theme", initial_theme),
            ("data-theme-preference", &design.theme.default_mode),
        ],
        |html| {
            html.el("head", &[], |head| {
                head.void_el("meta", &[("charset", "utf-8")]);
                head.void_el(
                    "meta",
                    &[("name", "viewport"), ("content", "width=device-width, initial-scale=1")],
                );
                head.el("title", &[], |t| {
                    t.text(seo_title);
                });
                head.raw(&seo_head_html);
                head.void_el(
                    "meta",
                    &[("name", "theme-color"), ("content", &design.theme.light.background)],
                );
                head.void_el("link", &[("rel", "icon"), ("type", "image/svg+xml"), ("href", "/favicon.svg")]);
                head.void_el("link", &[("rel", "icon"), ("type", "image/x-icon"), ("href", "/favicon.ico")]);
                head.void_el("link", &[("rel", "icon"), ("type", "image/png"), ("sizes", "32x32"), ("href", "/images/favicon-32x32.png")]);
                head.void_el("link", &[("rel", "icon"), ("type", "image/png"), ("sizes", "64x64"), ("href", "/images/favicon.png")]);
                head.void_el("link", &[("rel", "apple-touch-icon"), ("sizes", "180x180"), ("href", "/apple-touch-icon.png")]);
                for font_href in [
                    "/fonts/editorial/ibm-plex-sans-latin-400-normal.woff2",
                    "/fonts/editorial/ibm-plex-sans-latin-ext-400-normal.woff2",
                    "/fonts/editorial/source-serif-4-latin-opsz-normal.woff2",
                    "/fonts/editorial/source-serif-4-latin-ext-opsz-normal.woff2",
                ] {
                    head.void_el(
                        "link",
                        &[
                            ("rel", "preload"),
                            ("href", font_href),
                            ("as", "font"),
                            ("type", "font/woff2"),
                            ("crossorigin", ""),
                        ],
                    );
                }
                // Bootstrap CSS intentionally eliminated; pure MSI editorial design system only
                head.link_css("/css/msi-design-system.css?v=35");
                head.link_css("/css/design-tokens.css");
                head.script_inline(
                    crate::engine::design::theme_boot::UNIFIED_THEME_PREHYDRATE_JS,
                    nonce,
                    None,
                );
            });

            html.el(
                "body",
                &[
                    ("class", &body_class),
                    ("data-default-theme", &design.theme.default_mode),
                ],
                |bd| {
                    bd.el(
                        "div",
                        &[("class", "scroll-telemetry"), ("id", "scroll-telemetry"), ("aria-hidden", "true")],
                        |_| {},
                    );
                    bd.el("div", &[("class", "ambient-stage"), ("aria-hidden", "true")], |amb| {
                        amb.el("span", &[("class", "ambient-orb orb-a")], |_| {});
                        amb.el("span", &[("class", "ambient-orb orb-b")], |_| {});
                        amb.el("span", &[("class", "ambient-orb orb-c")], |_| {});
                    });
                    bd.el("div", &[("class", "grain-veil"), ("aria-hidden", "true")], |_| {});

                    let skip_lbl = tr_text(tr, lang, "shell.skip_to_content", "İçeriğe geç", "Skip to content");
                    bd.el("a", &[("href", "#main-content"), ("class", "skip-link")], |a| {
                        a.text(&skip_lbl);
                    });

                    if let Some(ann_text) = announcement {
                        let from_lbl = tr_text(tr, lang, "shell.from_msi", "MSI'DAN", "FROM MSI");
                        let dismiss_lbl = tr_text(tr, lang, "shell.dismiss_announcement", "Duyuruyu kapat", "Dismiss announcement");
                        bd.el("aside", &[("class", "announcement"), ("id", "site-announcement")], |as_el| {
                            as_el.el("div", &[("class", "container announcement-inner")], |inner| {
                                inner.el("span", &[], |sp| {
                                    sp.el("b", &[], |b| {
                                        b.text(&from_lbl);
                                    });
                                    sp.text(" ");
                                    sp.text(&ann_text);
                                });
                                inner.el(
                                    "button",
                                    &[
                                        ("type", "button"),
                                        ("class", "icon-button"),
                                        ("data-dismiss-announcement", ""),
                                        ("aria-label", &dismiss_lbl),
                                    ],
                                    |btn| {
                                        btn.raw(&icon("close"));
                                    },
                                );
                            });
                        });
                    }

                    write_header(bd, lang, tr, brand, public_notifications.len());
                    write_nav_bar(bd, lang, mode, &design, categories, tr);
                    write_ticker(bd, lang, contents, tr);

                    bd.el("main", &[("id", "main-content"), ("tabindex", "-1")], |main_el| {
                        main_el.raw(body);
                    });

                    write_footer(bd, lang, brand, email, phone, fax, address, tr);
                    write_notifications_dialog(bd, lang, &public_notifications, tr);

                    dialogs::render_into(
                        bd,
                        &DialogContext {
                            lang,
                            tr,
                            institutions: opts.institutions,
                            can_read_pdf: opts.can_read_pdf,
                            featured_issue,
                        },
                    );

                    if let Some(popup_ad) = opts.floating_popup {
                        let popup_html = floating_popup_ad(popup_ad, lang, tr);
                        if !popup_html.is_empty() {
                            bd.raw(&popup_html);
                        }
                    }

                    bd.el(
                        "div",
                        &[("class", "toast-region"), ("aria-live", "polite"), ("aria-atomic", "true")],
                        |_| {},
                    );

                    bd.json_script("msi-bootstrap", &bootstrap_val, nonce);
                    bd.json_script("msi-i18n", &i18n_val, nonce);
                    bd.script_src("/js/vendor/lenis.min.js", nonce, true);
                    bd.script_src("/js/vendor/gsap.min.js", nonce, true);
                    bd.script_src("/js/vendor/ScrollTrigger.min.js", nonce, true);
                    bd.script_src("/js/msi-ui.js?v=35", nonce, true);
                    bd.script_inline(SPECULATION_RULES_JSON, nonce, Some("speculationrules"));
                },
            );
        },
    );
    let _ = is_en;
    buf.into_string()
}

fn write_header(
    bd: &mut HtmlBuf,
    lang: &str,
    tr: Option<&TranslationCache>,
    brand: &str,
    notification_count: usize,
) {
    let home_href = lang_href("/", lang);
    let search_href = lang_href("/arama", lang);
    let account_href = lang_href("/hesabim", lang);
    let search_aria = tr_text(tr, lang, "nav.search", "Ara", "Search");
    let notif_aria = tr_text(tr, lang, "shell.notifications", "Bildirimler", "Notifications");
    let theme_aria = tr_text(tr, lang, "shell.toggle_theme", "Tema değiştir", "Toggle theme");
    let account_aria = tr_text(tr, lang, "nav.account", "Hesabım", "My Account");
    let menu_aria = tr_text(tr, lang, "shell.menu", "Menü", "Menu");
    let notif_count_str = notification_count.to_string();

    bd.el("header", &[("class", "site-header")], |hdr| {
        hdr.el("div", &[("class", "masthead-band")], |band| {
            band.el("div", &[("class", "container masthead")], |mh| {
                mh.el(
                    "a",
                    &[("class", "wordmark"), ("href", &home_href), ("aria-label", brand)],
                    |a| {
                        a.el("span", &[("class", "wordmark-brand")], |wb| {
                            wb.void_el(
                                "img",
                                &[
                                    ("class", "brand-logo"),
                                    ("src", "/images/logo-white.svg"),
                                    ("alt", brand),
                                    ("width", "178"),
                                    ("height", "66"),
                                    ("decoding", "async"),
                                ],
                            );
                        });
                    },
                );
                mh.el("div", &[("class", "header-actions")], |acts| {
                    acts.el(
                        "a",
                        &[("class", "icon-button"), ("href", &search_href), ("aria-label", &search_aria)],
                        |a| {
                            a.raw(&icon("search"));
                        },
                    );
                    acts.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "icon-button"),
                            ("data-open-dialog", "notifications-dialog"),
                            ("aria-label", &notif_aria),
                        ],
                        |btn| {
                            btn.raw(&icon("bell"));
                            btn.el("span", &[("class", "notification-count")], |sp| {
                                sp.text(&notif_count_str);
                            });
                        },
                    );
                    acts.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "icon-button"),
                            ("id", "btnToggleThemeMode"),
                            ("aria-label", &theme_aria),
                        ],
                        |btn| {
                            btn.raw(&icon("moon"));
                        },
                    );
                    acts.el(
                        "a",
                        &[
                            ("class", "icon-button account-link"),
                            ("href", &account_href),
                            ("aria-label", &account_aria),
                            ("title", &account_aria),
                        ],
                        |a| {
                            a.raw(&icon("user"));
                        },
                    );
                    acts.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "icon-button mobile-toggle"),
                            ("aria-label", &menu_aria),
                            ("aria-controls", "site-navigation"),
                            ("aria-expanded", "false"),
                        ],
                        |btn| {
                            btn.raw(&icon("menu"));
                        },
                    );
                });
            });
        });
    });
}

fn nav_label(href: &str, tr_default: &str, lang: &str) -> String {
    if !lang.eq_ignore_ascii_case("en") {
        return tr_default.to_string();
    }
    let slug = href
        .trim_start_matches("/kategori/")
        .split('?')
        .next()
        .unwrap_or("");
    match slug {
        "haberler" => "News".to_string(),
        "kapak-dosyasi" | "kapak-dosyalari" => "Cover Files".to_string(),
        "soylesiler" => "Interviews".to_string(),
        "ozel-haberler" => "Exclusive Reports".to_string(),
        "basyazilar" | "basyazi" => "Editorials".to_string(),
        "mesajlar" => "Messages".to_string(),
        "tanitici-makaleler" => "Special Coverages".to_string(),
        "makaleler-analizler" => "Analysis".to_string(),
        "advertorials" | "advertorial" => "Advertorials".to_string(),
        "ozel-dosya" | "ozel-dosyalar" => "Special Dossiers".to_string(),
        _ => tr_default.to_string(),
    }
}

fn write_nav_bar(
    bd: &mut HtmlBuf,
    lang: &str,
    mode: &str,
    design: &config::DesignConfig,
    categories: &[CategoryNode],
    tr: Option<&TranslationCache>,
) {
    let active_mode = if mode == "news" { "news" } else { "magazine" };
    let mode_aria = tr_text(tr, lang, "shell.mode_aria", "Yayın modu", "Publication mode");
    let nav_aria = tr_text(tr, lang, "shell.nav_aria", "Ana menü", "Primary navigation");
    let mag_lbl = tr_text(tr, lang, "shell.mode_magazine", "DERGİ", "MAGAZINE");
    let news_lbl = tr_text(tr, lang, "shell.mode_news", "HABER", "NEWS");
    let edergi_lbl = tr_text(tr, lang, "nav.edergi", "E-Dergi", "E-Magazine");
    let more_topics_lbl = tr_text(tr, lang, "shell.all_topics", "Tüm konular", "All topics");

    let mag_href = if lang.eq_ignore_ascii_case("en") {
        "/en?mode=magazine"
    } else {
        "/?mode=magazine"
    };
    let news_href = if lang.eq_ignore_ascii_case("en") {
        "/en?mode=news"
    } else {
        "/?mode=news"
    };
    let edergi_href = lang_href("/e-dergi", lang);
    let other_lang = if lang.eq_ignore_ascii_case("en") { "tr" } else { "en" };
    let other_lang_upper = if lang.eq_ignore_ascii_case("en") { "TR" } else { "EN" };
    let switch_href = format!("/switch/{}", other_lang);

    bd.el("div", &[("class", "navigation-border")], |nb| {
        nb.el("div", &[("class", "container nav-command-row")], |row| {
            row.el(
                "div",
                &[
                    ("class", "mode-switch"),
                    ("role", "group"),
                    ("aria-label", &mode_aria),
                    ("data-mode-switch", ""),
                    ("data-active-mode", active_mode),
                ],
                |sw| {
                    sw.el("span", &[("class", "mode-switch-thumb"), ("aria-hidden", "true")], |_| {});
                    sw.el(
                        "a",
                        &[
                            ("href", mag_href),
                            ("id", "btnSwitchDergi"),
                            ("data-portal", "magazine"),
                            ("class", if mode == "news" { "" } else { "active" }),
                        ],
                        |a| {
                            a.text(&mag_lbl);
                        },
                    );
                    sw.el(
                        "a",
                        &[
                            ("href", news_href),
                            ("id", "btnSwitchHaber"),
                            ("data-portal", "news"),
                            ("class", if mode == "news" { "active" } else { "" }),
                        ],
                        |a| {
                            a.text(&news_lbl);
                        },
                    );
                },
            );
            row.el("a", &[("class", "mobile-edergi-pill"), ("href", &edergi_href)], |a| {
                a.text(&edergi_lbl);
                a.text(" ");
                a.raw(&icon("arrow"));
            });
            row.el(
                "nav",
                &[("class", "navigation"), ("id", "site-navigation"), ("aria-label", &nav_aria)],
                |nav_el| {
                    for n in &design.navigation {
                        let href_resolved = lang_href(&n.href, lang);
                        let safe = link(&href_resolved);
                        let label = nav_label(&n.href, &n.label, lang);
                        nav_el.el("a", &[("href", &safe)], |a| {
                            a.text(&label);
                        });
                    }
                    nav_el.el("details", &[("class", "nav-more")], |det| {
                        det.el("summary", &[], |sum| {
                            sum.text(&more_topics_lbl);
                            sum.text(" ");
                            sum.el("span", &[], |sp| {
                                sp.text("⌄");
                            });
                        });
                        det.el("div", &[("class", "mega-menu")], |mega| {
                            write_mega_menu(mega, lang, categories, tr);
                        });
                    });
                    nav_el.el("a", &[("class", "nav-edergi-link"), ("href", &edergi_href)], |a| {
                        a.text(&edergi_lbl);
                    });
                    nav_el.el(
                        "a",
                        &[
                            ("class", "language-link"),
                            ("href", &switch_href),
                            ("data-language", other_lang),
                        ],
                        |a| {
                            a.text(other_lang_upper);
                        },
                    );
                    let mob_account_href = lang_href("/hesabim", lang);
                    let mob_sub_href = lang_href("/abonelik-ve-satis", lang);
                    let mob_account_lbl = tr_text(tr, lang, "nav.account", "Hesabım", "My Account");
                    let mob_sub_lbl = tr_text(tr, lang, "nav.subscriptions", "Abonelik", "Subscribe");
                    nav_el.el("div", &[("class", "mobile-nav-actions")], |ma| {
                        ma.el("a", &[("class", "button primary"), ("href", &mob_account_href)], |a| {
                            a.text(&mob_account_lbl);
                        });
                        ma.el("a", &[("class", "button secondary"), ("href", &mob_sub_href)], |a| {
                            a.text(&mob_sub_lbl);
                        });
                    });
                },
            );
        });
    });
}

fn write_mega_menu(
    mega: &mut HtmlBuf,
    lang: &str,
    categories: &[CategoryNode],
    tr: Option<&TranslationCache>,
) {
    let canonical_editorial_skip = [
        "basyazi",
        "kapak-dosyalari",
        "ozel-dosyalar",
        "advertorial",
        "dergi-icerikleri",
        "etkinlikler",
        "etkinlik-ozel-sayilari",
        "etkinlik-sonrasi-ozel-dosyalar",
        "highlights",
        "feature",
    ];
    let mut seen_labels = HashSet::new();
    let mut editorial = Vec::new();
    let mut dossiers = Vec::new();

    for c in categories {
        if canonical_editorial_skip.contains(&c.slug.as_str())
            || (lang == "en" && c.slug == "makaleler-analizler")
        {
            continue;
        }
        let raw_label = localized(lang, &c.name_tr, &c.name_en);
        let display_label = if c.slug == "ni" && raw_label.trim().eq_ignore_ascii_case("NI") {
            if lang == "en" {
                "NI Test & Measurement Dossier"
            } else {
                "NI Test ve Ölçüm Dosyası"
            }
        } else {
            raw_label
        };
        if !seen_labels.insert(display_label.trim().to_lowercase()) {
            continue;
        }
        let href = lang_href(&format!("/kategori/{}", c.slug), lang);
        if c.group_type == "etkinlik-dosyasi" {
            dossiers.push((href, display_label.to_string()));
        } else if c.group_type == "icerikler" || c.group_type == "gruplandirilmis" {
            editorial.push((href, display_label.to_string()));
        }
    }

    let sec_lbl = tr_text(tr, lang, "shell.mega_editorial", "YAYIN BÖLÜMLERİ", "EDITORIAL SECTIONS");
    let dos_lbl = tr_text(
        tr,
        lang,
        "shell.mega_dossiers",
        "FUAR, KONFERANS VE ÖZEL DOSYALAR",
        "EXHIBITION & SPECIAL DOSSIERS",
    );

    for (heading, items) in [(sec_lbl, editorial), (dos_lbl, dossiers)] {
        mega.el("div", &[("class", "mega-section")], |sec| {
            sec.el("div", &[("class", "mega-section-title")], |t| {
                t.text(&heading);
            });
            sec.el("div", &[("class", "mega-links-grid")], |grid| {
                for (href, label) in items {
                    grid.el("a", &[("href", &href)], |a| {
                        a.text(&label);
                    });
                }
            });
        });
    }
}

fn write_ticker(
    bd: &mut HtmlBuf,
    lang: &str,
    contents: &[ContentItem],
    tr: Option<&TranslationCache>,
) {
    let mut ticker_pool: Vec<&ContentItem> = contents
        .iter()
        .filter(|c| {
            c.content_type != "yeni-sayi-kapak"
                && c.status == "published"
                && (lang != "tr" || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
                && (lang != "en" || is_en_content_ready(c))
        })
        .collect();
    ticker_pool.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));
    if ticker_pool.is_empty() {
        return;
    }

    let badge_lbl = tr_text(tr, lang, "shell.ticker_badge", "HABERLER", "LIVE NEWS");
    let prev_lbl = tr_text(tr, lang, "shell.ticker_prev", "Önceki haber", "Previous headline");
    let next_lbl = tr_text(tr, lang, "shell.ticker_next", "Sonraki haber", "Next headline");
    let ticker_aria = tr_text(
        tr,
        lang,
        "shell.ticker_aria",
        "Son dakika haber kuşağı",
        "Breaking news ticker",
    );
    let ticker_href = lang_href("/kategori/haberler", lang);

    bd.el("div", &[("class", "ticker-wrap")], |wrap| {
        wrap.el("div", &[("class", "container")], |cont| {
            cont.el(
                "div",
                &[
                    ("class", "breaking-ticker is-marquee"),
                    ("data-news-ticker", ""),
                    ("aria-label", &ticker_aria),
                ],
                |ticker| {
                    ticker.el("a", &[("class", "ticker-badge"), ("href", &ticker_href)], |a| {
                        a.el("span", &[("class", "ticker-pulse"), ("aria-hidden", "true")], |_| {});
                        a.text(&badge_lbl);
                    });
                    ticker.el("div", &[("class", "ticker-controls")], |ctrls| {
                        ctrls.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("class", "ticker-nav"),
                                ("data-ticker-prev", ""),
                                ("aria-label", &prev_lbl),
                            ],
                            |b| {
                                b.text("‹");
                            },
                        );
                        ctrls.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("class", "ticker-nav"),
                                ("data-ticker-next", ""),
                                ("aria-label", &next_lbl),
                            ],
                            |b| {
                                b.text("›");
                            },
                        );
                    });
                    ticker.el("div", &[("class", "ticker-viewport")], |vp| {
                        vp.el("div", &[("class", "ticker-track"), ("data-ticker-track", "")], |track| {
                            for (idx, c) in ticker_pool
                                .iter()
                                .take(crate::db::constants::SHELL_TRENDING_LIMIT)
                                .enumerate()
                            {
                                let cls = if idx == 0 {
                                    "ticker-item is-active"
                                } else {
                                    "ticker-item"
                                };
                                let href = lang_href(&format!("/icerik/{}", c.slug), lang);
                                let idx_str = idx.to_string();
                                let title = localized(lang, &c.title_tr, &c.title_en);
                                track.el(
                                    "a",
                                    &[
                                        ("class", cls),
                                        ("href", &href),
                                        ("data-ticker-item", &idx_str),
                                    ],
                                    |a| {
                                        a.el("span", &[("class", "ticker-sep"), ("aria-hidden", "true")], |sp| {
                                            sp.text("◆");
                                        });
                                        a.text(title);
                                    },
                                );
                            }
                            for c in ticker_pool
                                .iter()
                                .take(crate::db::constants::SHELL_LATEST_ISSUES_LIMIT)
                            {
                                let href = lang_href(&format!("/icerik/{}", c.slug), lang);
                                let title = localized(lang, &c.title_tr, &c.title_en);
                                track.el(
                                    "a",
                                    &[
                                        ("class", "ticker-item is-clone"),
                                        ("href", &href),
                                        ("tabindex", "-1"),
                                        ("aria-hidden", "true"),
                                    ],
                                    |a| {
                                        a.el("span", &[("class", "ticker-sep"), ("aria-hidden", "true")], |sp| {
                                            sp.text("◆");
                                        });
                                        a.text(title);
                                    },
                                );
                            }
                        });
                    });
                },
            );
        });
    });
}

fn write_notifications_dialog(
    bd: &mut HtmlBuf,
    lang: &str,
    public_notifications: &[&SystemNotification],
    tr: Option<&TranslationCache>,
) {
    let reader_lbl = tr_text(tr, lang, "shell.reader_center", "OKUYUCU MERKEZİ", "READER CENTER");
    let notif_lbl = tr_text(tr, lang, "shell.notifications", "Bildirimler", "Notifications");
    let mark_all_lbl = tr_text(
        tr,
        lang,
        "shell.mark_all_read",
        "Tümünü okundu işaretle",
        "Mark all as read",
    );
    let close_lbl = tr_text(tr, lang, "shell.close", "Kapat", "Close");
    let no_notif_lbl = tr_text(
        tr,
        lang,
        "shell.no_notifications",
        "Şu anda yeni bildirim yok.",
        "No new notifications right now.",
    );

    bd.el(
        "dialog",
        &[
            ("id", "notifications-dialog"),
            ("class", "ui-dialog notification-dialog"),
            ("aria-labelledby", "notification-title"),
        ],
        |dlg| {
            dlg.el("div", &[("class", "dialog-heading")], |dh| {
                dh.el("div", &[], |left| {
                    left.el("div", &[("class", "eyebrow")], |eb| {
                        eb.text(&reader_lbl);
                    });
                    left.el("h2", &[("id", "notification-title")], |h2| {
                        h2.text(&notif_lbl);
                    });
                });
                dh.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "icon-button"),
                        ("data-close-dialog", ""),
                        ("aria-label", &close_lbl),
                    ],
                    |btn| {
                        btn.raw(&icon("close"));
                    },
                );
            });
            dlg.el("div", &[("class", "notification-actions")], |acts| {
                acts.el("button", &[("type", "button"), ("class", "text-link"), ("data-read-notifications", "")], |btn| {
                    btn.text(&mark_all_lbl);
                });
            });
            dlg.el("div", &[("class", "notification-list")], |list| {
                if public_notifications.is_empty() {
                    list.raw(&empty_state(&esc(&no_notif_lbl)));
                } else {
                    for n in public_notifications
                        .iter()
                        .take(crate::db::constants::SHELL_NAV_CONTENT_LIMIT)
                    {
                        list.raw(&notification_item(n, lang));
                    }
                }
            });
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn write_footer(
    bd: &mut HtmlBuf,
    lang: &str,
    brand: &str,
    email: &str,
    phone: &str,
    fax: &str,
    address: &str,
    tr: Option<&TranslationCache>,
) {
    let is_en = lang.eq_ignore_ascii_case("en");
    let home = lang_href("/", lang);
    let tagline = tr_text(
        tr,
        lang,
        "footer.tagline",
        "Aylık, Türkçe ve İngilizce Yayımlanan Savunma, Havacılık ve Güvenlik Teknolojileri Dergisi · ISSN 1306-0309",
        "Monthly Turkish & English Defence, Aerospace and Security Technologies Review · ISSN 1306-0309",
    );
    let h_contact = tr_text(tr, lang, "footer.contact_heading", "İletişim & Merkez Ofis", "Contact & Headquarters");
    let lbl_addr = tr_text(tr, lang, "footer.address_label", "Adres:", "Address:");
    let social_txt = tr_text(
        tr,
        lang,
        "footer.social_prompt",
        "Bizi Sosyal Medya Hesaplarımızdan Takip Edebilirsiniz",
        "Follow Us on Official Social Media Channels",
    );
    let social_aria = tr_text(tr, lang, "footer.social_aria", "Resmî sosyal medya hesapları", "Official social media");
    let h_cats = tr_text(tr, lang, "footer.sections_heading", "Yayın Kategorileri", "Publication Sections");
    let h_about = tr_text(tr, lang, "footer.about_heading", "Hakkımızda & Künye", "About & Masthead");
    let h_reader = tr_text(tr, lang, "footer.reader_heading", "Okuyucu & Veri Koruma", "Reader & Privacy");
    let legal_aria = tr_text(tr, lang, "footer.legal_aria", "Yasal bildirimler", "Legal notices");
    let cookie_pref_lbl = tr_text(tr, lang, "footer.cookie_prefs", "Çerez tercihleri", "Cookie preferences");
    let year = chrono::Utc::now().format("%Y").to_string();
    let copyright_txt = if is_en {
        format!("© {year} UMSA Production Publishing Ltd. · All rights reserved (Law No. 5846).")
    } else {
        format!("© {year} UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. · Tüm hakları saklıdır (5846 s. FSEK).")
    };

    let cat_links: &[(&str, &str, &str)] = &[
        ("/kategori/haberler", "Haberler", "News"),
        ("/kategori/kapak-dosyasi", "Kapak Dosyaları", "Cover Files"),
        ("/kategori/soylesiler", "Söyleşiler", "Interviews"),
        ("/kategori/ozel-haberler", "Özel Haberler", "Special Reports"),
        ("/kategori/basyazilar", "Başyazılar", "Editorial Comments"),
        ("/kategori/tanitici-makaleler", "Tanıtıcı Makaleler", "Special Coverages"),
        ("/kategori/ozel-dosya", "Özel Dosya", "Special Dossiers"),
        ("/e-dergi", "E-Dergi & Arşiv", "E-Magazine & Archive"),
    ];
    let about_links: &[(&str, &str, &str)] = &[
        ("/kurumsal/kunye", "Künye", "Masthead"),
        ("/kurumsal/hakkimizda", "Biz Kimiz? (Hakkımızda)", "Who We Are (About)"),
        ("/kurumsal/yolculugumuz", "Yolculuğumuz", "Our Journey"),
        ("/kurumsal/cozum-ortaklarimiz", "Çözüm Ortaklarımız", "Solution Partners"),
        ("/kurumsal/amac-magazine", "AMAC Magazine", "AMAC Magazine"),
        ("/kurumsal/kasabaworks", "KasabaWorks", "KasabaWorks"),
        ("/kurumsal/iletisim", "İletişim ve Reklam", "Contact & Advertising"),
    ];
    let reader_links: &[(&str, &str, &str)] = &[
        ("/hesabim", "Hesabım", "My Account"),
        ("/abonelik-ve-satis", "Abonelik ve Yayınlar", "Subscriptions & Store"),
        ("/kurumsal/kvkk", "KVKK Aydınlatma Metni", "KVKK Disclosure"),
        ("/kurumsal/kvkk-basvuru-formu", "KVKK Başvuru Formu", "Data Request Form"),
        ("/kurumsal/gizlilik-politikasi", "Gizlilik Politikası", "Privacy Policy"),
        ("/kurumsal/cerez-politikasi", "Çerez Politikası", "Cookie Policy"),
        ("/rss.xml", "RSS Haber Akışı", "RSS News Feed"),
    ];
    let legal_links: &[(&str, &str, &str)] = &[
        ("/kurumsal/kullanim-kosullari", "Kullanım Koşulları", "Terms of Use"),
        ("/kurumsal/mesafeli-satis-sozlesmesi", "Mesafeli Satış Sözleşmesi", "Distance Sales Agreement"),
        ("/kurumsal/iptal-ve-iade-kosullari", "İptal ve İade Koşulları", "Cancellation & Refund"),
        ("/kurumsal/ticari-ileti-aydinlatma", "Ticari Elektronik İleti (ETK/İYS)", "Commercial Messaging"),
        ("/kurumsal/telif-ve-etik", "Telif ve Yayın Etiği", "Copyright & Ethics"),
        ("/kurumsal/duzeltme-ve-cevap-hakki", "Düzeltme ve Cevap Hakkı", "Correction & Reply"),
        ("/kurumsal/reklam-ve-sponsorluk", "Reklam ve Sponsorluk İlkeleri", "Advertising Principles"),
    ];
    let mailto = format!("mailto:{}", email);
    let tel_href = format!("tel:{}", phone.replace([' ', '(', ')', '-'], ""));

    bd.el("footer", &[("class", "site-footer")], |ft| {
        ft.el("div", &[("class", "footer-brand-strip")], |strip| {
            strip.el("div", &[("class", "container footer-brand-inner")], |inner| {
                inner.el("a", &[("href", &home), ("class", "wordmark"), ("aria-label", brand)], |a| {
                    a.el("span", &[("class", "wordmark-brand")], |wb| {
                        wb.void_el(
                            "img",
                            &[
                                ("class", "brand-logo"),
                                ("src", "/images/logo-white.svg"),
                                ("alt", brand),
                                ("width", "128"),
                                ("height", "47"),
                                ("decoding", "async"),
                            ],
                        );
                    });
                });
                inner.el("span", &[], |sp| {
                    sp.text(&tagline);
                });
            });
        });

        ft.el("div", &[("class", "container footer-grid")], |grid| {
            grid.el("div", &[], |col| {
                col.el("h2", &[], |h2| {
                    h2.text(&h_contact);
                });
                col.el("p", &[("class", "footer-contact-list")], |p| {
                    p.el("strong", &[], |s| { s.text(&lbl_addr); });
                    p.text(" ");
                    p.text(address);
                    p.raw("<br>");
                    p.el("strong", &[], |s| { s.text("P:"); });
                    p.text(" ");
                    p.el("a", &[("href", &tel_href)], |a| { a.text(phone); });
                    p.raw("<br>");
                    p.el("strong", &[], |s| { s.text("F:"); });
                    p.text(" ");
                    p.text(fax);
                    p.raw("<br>");
                    p.el("strong", &[], |s| { s.text("E:"); });
                    p.text(" ");
                    p.el("a", &[("href", &mailto)], |a| { a.text(email); });
                    p.raw("<br>");
                    p.el(
                        "a",
                        &[
                            ("href", "https://www.savunmahaber.com"),
                            ("rel", "noopener noreferrer"),
                            ("target", "_blank"),
                        ],
                        |a| {
                            a.text("www.savunmahaber.com");
                        },
                    );
                });
                col.el("div", &[("class", "footer-social-block")], |sb| {
                    sb.el("span", &[], |sp| { sp.text(&social_txt); });
                    sb.el("div", &[("class", "footer-social-links"), ("aria-label", &social_aria)], |links| {
                        links.raw(SOCIAL_LINKS_SVG_HTML);
                    });
                });
            });

            for (heading, list) in [
                (h_cats, cat_links),
                (h_about, about_links),
                (h_reader, reader_links),
            ] {
                grid.el("div", &[], |col| {
                    col.el("h2", &[], |h2| { h2.text(&heading); });
                    for (path, lbl_tr, lbl_en) in list {
                        let href = if *path == "/rss.xml" {
                            (*path).to_string()
                        } else {
                            lang_href(path, lang)
                        };
                        col.el("a", &[("href", &href)], |a| {
                            a.text(if is_en { lbl_en } else { lbl_tr });
                        });
                    }
                });
            }
        });

        ft.el("nav", &[("class", "container footer-legal-bar"), ("aria-label", &legal_aria)], |bar| {
            for (path, lbl_tr, lbl_en) in legal_links {
                let href = lang_href(path, lang);
                bar.el("a", &[("href", &href)], |a| {
                    a.text(if is_en { lbl_en } else { lbl_tr });
                });
            }
            bar.el(
                "button",
                &[
                    ("type", "button"),
                    ("class", "footer-cookie-btn"),
                    ("data-open-dialog", "cookie-dialog"),
                ],
                |btn| {
                    btn.text(&cookie_pref_lbl);
                },
            );
        });

        ft.el("div", &[("class", "container footer-bottom")], |btm| {
            btm.el("span", &[("class", "footer-bottom-left")], |sp| {
                sp.text(&copyright_txt);
            });
            btm.el("span", &[("class", "footer-bottom-center")], |sp| {
                sp.text("MSI Dergisi · MSI Turkish Defence Review (ISSN 1306-0309)");
            });
            btm.el("span", &[("class", "footer-bottom-right")], |sp| {
                sp.text(if is_en { "Software & Design: " } else { "Yazılım & Tasarım: " });
                sp.el(
                    "a",
                    &[
                        ("href", "https://pofudukdijital.com"),
                        ("target", "_blank"),
                        ("rel", "noopener"),
                        ("class", "footer-credit-link"),
                    ],
                    |a| {
                        a.text("Pofuduk Dijital");
                    },
                );
            });
        });
    });
}

const SPECULATION_RULES_JSON: &str = r#"{"prefetch":[{"source":"document","where":{"and":[{"href_matches":["/icerik/*","/kategori/*","/e-dergi","/abonelik-ve-satis","/kurumsal/*"]},{"not":{"href_matches":["/podmin*","/admin*","/api/*","*logout*"]}}]},"eagerness":"moderate"}]}"#;

const SOCIAL_LINKS_SVG_HTML: &str = r#"<a href="https://www.facebook.com/MSI.Dergisi/" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi Facebook"><svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M13.5 21v-8h2.7l.4-3h-3.1V8.1c0-.9.3-1.5 1.6-1.5h1.7V3.9c-.3 0-1.3-.1-2.5-.1-2.5 0-4.2 1.5-4.2 4.3V10H7.3v3h2.8v8h3.4Z"/></svg></a><a href="https://www.linkedin.com/company/msi-dergisi" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi LinkedIn"><svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M6.9 8.5a2 2 0 1 1 0-4 2 2 0 0 1 0 4ZM5.2 10h3.4v10.2H5.2V10Zm5.5 0h3.2v1.4h.1c.5-.8 1.6-1.7 3.3-1.7 3.5 0 4.1 2.3 4.1 5.3v5.2H18v-4.6c0-1.1 0-2.5-1.5-2.5s-1.8 1.2-1.8 2.4v4.7h-3.4V10Z"/></svg></a><a href="https://www.instagram.com/msidergisi/" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi Instagram"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="5"/><circle cx="12" cy="12" r="4"/><circle cx="17.5" cy="6.5" r="1" fill="currentColor" stroke="none"/></svg></a><a href="https://nsosyal.com/msidergisi" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi NSosyal"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="2"/><path d="M16.2 7.8a6 6 0 0 1 0 8.4m-8.4 0a6 6 0 0 1 0-8.4m11.2-2.8a10 10 0 0 1 0 14m-14 0a10 10 0 0 1 0-14"/></svg></a><a href="https://twitter.com/MSI_Dergisi" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi X (Twitter)"><svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M18.2 3h2.9l-6.4 7.3L22.2 21h-5.9l-4.6-6-5.3 6H3.5l6.8-7.8L3.1 3h6l4.2 5.5L18.2 3Zm-1 16.2h1.6L8.3 4.7H6.6l10.6 14.5Z"/></svg></a><a href="https://www.youtube.com/c/msidergisi" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi YouTube"><svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M21.6 7.2a2.7 2.7 0 0 0-1.9-1.9C18 4.8 12 4.8 12 4.8s-6 0-7.7.5A2.7 2.7 0 0 0 2.4 7.2C2 8.9 2 12 2 12s0 3.1.4 4.8a2.7 2.7 0 0 0 1.9 1.9c1.7.5 7.7.5 7.7.5s6 0 7.7-.5a2.7 2.7 0 0 0 1.9-1.9c.4-1.7.4-4.8.4-4.8s0-3.1-.4-4.8ZM10 15.2V8.8l5.5 3.2-5.5 3.2Z"/></svg></a>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_omits_bootstrap_embeds_i18n_and_renders_floating_popup_safely() {
        let tr = TranslationCache::new();
        let seo = SeoMetaBundle {
            title: "Test".into(),
            description: "Desc".into(),
            canonical_url: "https://test.msidergisi.com/en".into(),
            hreflang_tr: "https://test.msidergisi.com/".into(),
            hreflang_en: "https://test.msidergisi.com/en".into(),
            og_image: "/images/logo.svg".into(),
            og_type: "website".into(),
            robots: "index, follow".into(),
            json_ld: "{}".into(),
            ..Default::default()
        };
        let popup = AdCampaign {
            id: 8,
            slot_code: "ad-floating-popup".into(),
            badge_label: "SHOWCASE".into(),
            dimensions: "600x400".into(),
            sponsor_name: "KasabaWorks".into(),
            sponsor_type: "in_house".into(),
            pill_style: "gold".into(),
            title_tr: "TR Başlık".into(),
            title_en: "EN <script>alert(1)</script> Title".into(),
            desc_tr: "TR Açıklama".into(),
            desc_en: "EN Description".into(),
            target_url: "/kurumsal/cozum-ortaklarimiz".into(),
            image_url: None,
            budget_score: 85,
            is_default_fallback: true,
            is_floating_popup: true,
            is_newsletter_ad: false,
            target_categories: vec![],
            target_ssb_tags: vec![],
            target_institutions: vec![],
            impressions_count: 0,
            clicks_count: 0,
            is_active: true,
            start_date: None,
            end_date: None,
            rotation_mode: "weighted".into(),
        };
        let html = render_full(
            "en",
            "magazine",
            &HashMap::new(),
            &seo,
            &[],
            &[],
            &[],
            &[],
            &[],
            "<div>Hello</div>",
            10,
            Some("test-nonce"),
            &ShellOptions {
                tr: Some(&tr),
                institutions: &[],
                floating_popup: Some(&popup),
                can_read_pdf: true,
            },
        );
        assert!(!html.contains("bootstrap.min.css"));
        assert!(!html.contains("msi-ssr-enhancements"));
        assert!(!html.contains("window.__MSI_I18N__="));
        assert!(html.contains("/css/msi-design-system.css?v=35"));
        assert!(html.contains("/js/msi-ui.js?v=35"));
        assert!(html.contains("/images/logo-white.svg"));
        assert!(html.contains("<script id=\"msi-i18n\" type=\"application/json\" nonce=\"test-nonce\">"));
        assert!(html.contains("id=\"ad-floating-popup\""));
        assert!(html.contains("EN &lt;script&gt;alert(1)&lt;/script&gt; Title"));
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("/api/v1/edergi/286/download-pdf?publication_type=msi#toolbar=0"));
        assert!(!html.contains("Hoş geldiniz"));
        let body_part = html.split("</head>").nth(1).unwrap_or("");
        assert!(!body_part.contains("Defense"));
    }
}
