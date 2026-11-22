//! Pure-Rust localized SSR renderer for public UI dialogs and consent banners.
//! Replaces legacy `dialogs.html` + string-replace patches with `HtmlBuf` and `TranslationCache`.

use super::html_dsl::HtmlBuf;
use super::labels_helper::{
    default_institution_choices, lang_href, tr_text, InstitutionChoice,
};
use crate::engine::translation_engine::TranslationCache;

/// Context for rendering localized public dialogs.
pub struct DialogContext<'a> {
    pub lang: &'a str,
    pub tr: Option<&'a TranslationCache>,
    pub institutions: &'a [InstitutionChoice],
    pub can_read_pdf: bool,
    pub featured_issue: Option<i32>,
}

/// Renders all public dialogs (`#issue-dialog`, `#auth-dialog`, `#offer-dialog`,
/// `#cookie-consent-bar`, `#cookie-dialog`, `#gallery-lightbox`) cleanly in pure Rust.
pub fn render(ctx: &DialogContext<'_>) -> String {
    let mut buf = HtmlBuf::with_capacity(8192);
    render_into(&mut buf, ctx);
    buf.into_string()
}

pub fn render_into(buf: &mut HtmlBuf, ctx: &DialogContext<'_>) {
    render_issue_dialog(buf, ctx);
    render_auth_dialog(buf, ctx);
    render_offer_dialog(buf, ctx);
    render_newsletter_dialog(buf, ctx);
    render_cart_drawer(buf, ctx);
    render_cookie_consent_bar(buf, ctx);
    render_cookie_dialog(buf, ctx);
    render_gallery_lightbox(buf, ctx);
}

fn append_antispam_fields(f: &mut HtmlBuf) {
    f.void_with("input", |inp| {
        inp.attr("type", "text")
            .attr("name", "website_url")
            .attr("class", "hp-antispam-field")
            .attr("tabindex", "-1")
            .attr("autocomplete", "off")
            .attr("aria-hidden", "true");
    });
    f.void_with("input", |inp| {
        inp.attr("type", "hidden")
            .attr("name", "_form_ts")
            .attr("value", "")
            .bool_attr("data-form-timing", true);
    });
}

fn t(ctx: &DialogContext<'_>, key: &str, tr: &str, en: &str) -> String {
    tr_text(ctx.tr, ctx.lang, key, tr, en)
}

fn close_label(ctx: &DialogContext<'_>) -> String {
    t(ctx, "shell.close", "Kapat", "Close")
}

fn write_dialog_heading(
    buf: &mut HtmlBuf,
    eyebrow_id: Option<&str>,
    eyebrow_text: &str,
    title_id: &str,
    title_text: &str,
    close_aria: &str,
) {
    buf.el("div", &[("class", "dialog-heading")], |b| {
        b.el("div", &[], |inner| {
            inner.el_with(
                "div",
                |attrs| {
                    attrs.attr("class", "eyebrow");
                    if let Some(eid) = eyebrow_id {
                        attrs.attr("id", eid);
                    }
                },
                |eb| {
                    eb.text(eyebrow_text);
                },
            );
            inner.el("h2", &[("id", title_id)], |h| {
                h.text(title_text);
            });
        });
        b.el(
            "button",
            &[
                ("type", "button"),
                ("class", "icon-button"),
                ("data-close-dialog", ""),
                ("aria-label", close_aria),
            ],
            |btn| {
                btn.raw("&times;");
            },
        );
    });
}

fn render_issue_dialog(buf: &mut HtmlBuf, ctx: &DialogContext<'_>) {
    let close_aria = close_label(ctx);
    let title = t(ctx, "dialogs.issue.title", "E-Dergi", "E-Magazine");
    let tab_toc = t(ctx, "dialogs.issue.tab_toc", "İçindekiler", "Table of Contents");
    let tab_flipbook = t(ctx, "dialogs.issue.tab_flipbook", "Sayfa Çevirici", "Flipbook Reader");
    let tab_pdf = t(ctx, "dialogs.issue.tab_pdf", "PDF Okuyucu", "PDF Reader");
    let prev_aria = t(ctx, "dialogs.issue.prev_spread", "Önceki bölüm", "Previous section");
    let next_aria = t(ctx, "dialogs.issue.next_spread", "Sonraki bölüm", "Next section");
    let open_fb_cta = t(ctx, "dialogs.issue.open_flipbook", "Sayfa Çeviricide Aç →", "Open in Flipbook →");
    let open_pdf = t(ctx, "dialogs.issue.open_pdf", "Filigranlı PDF İndir ↗", "Download Watermarked PDF ↗");
    let export_title = t(ctx, "dialogs.issue.export_title", "Filigranlı PDF Dışa Aktarım", "Watermarked PDF Export");
    let export_all = t(ctx, "dialogs.issue.export_all", "Tam Sayı", "Full Issue");
    let export_current = t(ctx, "dialogs.issue.export_current", "Bu Sayfa / Çift Sayfa", "Current Page / Spread");
    let export_custom = t(ctx, "dialogs.issue.export_custom", "Belirli Sayfalar", "Custom Pages");
    let export_ph = t(ctx, "dialogs.issue.export_placeholder", "Örn: 1-5, 8", "e.g. 1-5, 8");
    let export_input_aria = t(ctx, "dialogs.issue.export_input_aria", "İndirilecek sayfa aralığı", "Page range to download");
    let watermark_notice = t(
        ctx,
        "dialogs.issue.watermark_notice",
        "Kişisel ve kurumsal DRM güvenlik filigranı indirilen tüm sayfalara dinamik olarak işlenir.",
        "Personal and institutional DRM security watermark is dynamically embedded into every exported page.",
    );
    let fb_prev_aria = t(ctx, "dialogs.issue.fb_prev", "Önceki sayfa", "Previous page");
    let fb_next_aria = t(ctx, "dialogs.issue.fb_next", "Sonraki sayfa", "Next page");
    let fb_page_aria = t(ctx, "dialogs.issue.fb_page_aria", "Sayfa numarasına git", "Jump to page number");
    let fb_toc_aria = t(ctx, "dialogs.issue.fb_toc_aria", "Bölüme atla", "Jump to section");
    let fb_zoom_out_aria = t(ctx, "dialogs.issue.fb_zoom_out", "Uzaklaştır", "Zoom out");
    let fb_zoom_in_aria = t(ctx, "dialogs.issue.fb_zoom_in", "Yakınlaştır", "Zoom in");
    let fb_zoom_reset_aria = t(ctx, "dialogs.issue.fb_zoom_reset", "Yakınlaştırmayı sıfırla", "Reset zoom");
    let fb_fullscreen_lbl = t(ctx, "dialogs.issue.fb_fullscreen", "Tam Ekran", "Fullscreen");
    let hint = t(
        ctx,
        "dialogs.issue.hint",
        "Sayıdaki bölümleri oklarla inceleyebilirsiniz. PDF erişimi abonelik koşullarına bağlıdır.",
        "Use the arrows to browse issue dossiers. PDF access is subject to subscription entitlements.",
    );
    let view_tabs_aria = t(
        ctx,
        "dialogs.issue.view_tabs_aria",
        "E-Dergi okuma görünümü",
        "E-Magazine reading view",
    );
    let active_issue = ctx.featured_issue.unwrap_or(286);
    let pdf_src = format!(
        "/api/v1/edergi/{}/download-pdf?publication_type=msi#toolbar=0",
        active_issue
    );

    buf.el(
        "dialog",
        &[
            ("id", "issue-dialog"),
            ("class", "ui-dialog issue-dialog"),
            ("aria-labelledby", "issue-dialog-title"),
        ],
        |b| {
            write_dialog_heading(
                b,
                Some("issue-dialog-meta"),
                "",
                "issue-dialog-title",
                &title,
                &close_aria,
            );
            b.el("div", &[("class", "issue-reader")], |r| {
                r.void_el("img", &[("id", "issue-dialog-cover"), ("alt", "")]);
                r.el("div", &[("class", "edergi-dialog-main")], |col| {
                    col.el("p", &[("id", "issue-dialog-summary")], |_| {});
                    col.el(
                        "div",
                        &[
                            ("class", "issue-view-tabs"),
                            ("role", "tablist"),
                            ("aria-label", &view_tabs_aria),
                        ],
                        |tabs| {
                            tabs.el(
                                "button",
                                &[
                                    ("type", "button"),
                                    ("id", "issue-tab-toc"),
                                    ("class", "issue-view-tab is-active"),
                                    ("role", "tab"),
                                    ("aria-selected", "true"),
                                    ("aria-controls", "issue-toc-panel"),
                                    ("data-issue-view", "toc"),
                                ],
                                |btn| {
                                    btn.text(&tab_toc);
                                },
                            );
                            tabs.el(
                                "button",
                                &[
                                    ("type", "button"),
                                    ("id", "issue-tab-flipbook"),
                                    ("class", "issue-view-tab"),
                                    ("role", "tab"),
                                    ("aria-selected", "false"),
                                    ("aria-controls", "issue-flipbook-panel"),
                                    ("data-issue-view", "flipbook"),
                                ],
                                |btn| {
                                    btn.text(&tab_flipbook);
                                },
                            );
                            tabs.el(
                                "button",
                                &[
                                    ("type", "button"),
                                    ("id", "issue-tab-pdf"),
                                    ("class", "issue-view-tab"),
                                    ("role", "tab"),
                                    ("aria-selected", "false"),
                                    ("aria-controls", "issue-pdf-panel"),
                                    ("data-issue-view", "pdf"),
                                ],
                                |btn| {
                                    btn.text(&tab_pdf);
                                },
                            );
                        },
                    );
                    col.el(
                        "div",
                        &[
                            ("id", "issue-toc-panel"),
                            ("role", "tabpanel"),
                            ("aria-labelledby", "issue-tab-toc"),
                        ],
                        |panel| {
                            panel.el("div", &[("class", "reader-toolbar")], |tb| {
                                tb.el(
                                    "button",
                                    &[
                                        ("type", "button"),
                                        ("class", "button secondary"),
                                        ("data-reader-prev", ""),
                                        ("aria-label", &prev_aria),
                                    ],
                                    |btn| {
                                        btn.text("←");
                                    },
                                );
                                tb.el(
                                    "span",
                                    &[("id", "reader-position"), ("aria-live", "polite")],
                                    |_| {},
                                );
                                tb.el(
                                    "button",
                                    &[
                                        ("type", "button"),
                                        ("class", "button secondary"),
                                        ("data-reader-next", ""),
                                        ("aria-label", &next_aria),
                                    ],
                                    |btn| {
                                        btn.text("→");
                                    },
                                );
                            });
                            panel.el("article", &[("id", "reader-spread")], |_| {});
                            panel.el(
                                "div",
                                &[("class", "edergi-toc-cta-row")],
                                |row| {
                                    row.el(
                                        "button",
                                        &[
                                            ("type", "button"),
                                            ("class", "button secondary edergi-open-fb-btn"),
                                            ("data-issue-view", "flipbook"),
                                        ],
                                        |btn| {
                                            btn.text(&open_fb_cta);
                                        },
                                    );
                                },
                            );
                            panel.el("div", &[("id", "edergi-toc-list"), ("class", "edergi-toc-list")], |_| {});
                            panel.el("div", &[("class", "edergi-export-box")], |exp| {
                                exp.el("div", &[("class", "edergi-export-head")], |hd| {
                                    hd.text(&export_title);
                                });
                                exp.el(
                                    "div",
                                    &[
                                        ("class", "edergi-export-modes"),
                                        ("role", "radiogroup"),
                                        ("aria-label", &export_title),
                                    ],
                                    |modes| {
                                        modes.el("label", &[("class", "edergi-export-option")], |lbl| {
                                            lbl.void_with("input", |inp| {
                                                inp.attr("type", "radio")
                                                    .attr("name", "edergi_export_mode")
                                                    .attr("value", "all")
                                                    .bool_attr("checked", true);
                                            });
                                            lbl.el("span", &[], |sp| {
                                                sp.text(&export_all);
                                            });
                                        });
                                        modes.el("label", &[("class", "edergi-export-option")], |lbl| {
                                            lbl.void_with("input", |inp| {
                                                inp.attr("type", "radio")
                                                    .attr("name", "edergi_export_mode")
                                                    .attr("value", "current");
                                            });
                                            lbl.el("span", &[("id", "edergi-export-current-label")], |sp| {
                                                sp.text(&export_current);
                                            });
                                        });
                                        modes.el("label", &[("class", "edergi-export-option")], |lbl| {
                                            lbl.void_with("input", |inp| {
                                                inp.attr("type", "radio")
                                                    .attr("name", "edergi_export_mode")
                                                    .attr("value", "custom");
                                            });
                                            lbl.el("span", &[], |sp| {
                                                sp.text(&export_custom);
                                            });
                                        });
                                    },
                                );
                                exp.el("div", &[("class", "edergi-export-custom-row")], |row| {
                                    row.void_with("input", |inp| {
                                        inp.attr("id", "edergi-pages-input")
                                            .attr("type", "text")
                                            .attr("class", "edergi-pages-input")
                                            .attr("placeholder", &export_ph)
                                            .attr("aria-label", &export_input_aria)
                                            .bool_attr("disabled", true);
                                    });
                                    row.el(
                                        "a",
                                        &[
                                            ("id", "issue-download"),
                                            ("class", "button primary"),
                                            ("target", "_blank"),
                                            ("rel", "noopener"),
                                        ],
                                        |a| {
                                            a.text(&open_pdf);
                                        },
                                    );
                                });
                                exp.el(
                                    "p",
                                    &[
                                        ("id", "edergi-export-feedback"),
                                        ("class", "form-feedback"),
                                        ("role", "status"),
                                        ("aria-live", "polite"),
                                    ],
                                    |_| {},
                                );
                                exp.el("p", &[("class", "edergi-watermark-notice")], |p| {
                                    p.text(&watermark_notice);
                                });
                            });
                            panel.el("p", &[("class", "reader-hint")], |p| {
                                p.text(&hint);
                            });
                        },
                    );
                    col.el_with(
                        "div",
                        |attrs| {
                            attrs
                                .attr("id", "issue-flipbook-panel")
                                .attr("class", "edergi-flipbook-panel")
                                .attr("role", "tabpanel")
                                .attr("aria-labelledby", "issue-tab-flipbook")
                                .bool_attr("hidden", true);
                        },
                        |panel| {
                            panel.el("div", &[("class", "edergi-fb-toolbar")], |tb| {
                                tb.el("div", &[("class", "edergi-fb-nav-group")], |nav| {
                                    nav.el(
                                        "button",
                                        &[
                                            ("type", "button"),
                                            ("class", "button secondary edergi-fb-btn"),
                                            ("data-fb-prev", ""),
                                            ("aria-label", &fb_prev_aria),
                                        ],
                                        |btn| {
                                            btn.text("←");
                                        },
                                    );
                                    nav.el("label", &[("class", "edergi-fb-jump-lbl")], |lbl| {
                                        lbl.void_el(
                                            "input",
                                            &[
                                                ("id", "edergi-page-input"),
                                                ("type", "number"),
                                                ("min", "1"),
                                                ("value", "1"),
                                                ("aria-label", &fb_page_aria),
                                            ],
                                        );
                                        lbl.el("span", &[("class", "edergi-fb-total")], |sp| {
                                            sp.raw("/ ");
                                            sp.el("strong", &[("id", "edergi-page-total")], |st| {
                                                st.text("1");
                                            });
                                        });
                                    });
                                    nav.el(
                                        "button",
                                        &[
                                            ("type", "button"),
                                            ("class", "button secondary edergi-fb-btn"),
                                            ("data-fb-next", ""),
                                            ("aria-label", &fb_next_aria),
                                        ],
                                        |btn| {
                                            btn.text("→");
                                        },
                                    );
                                });
                                tb.el(
                                    "select",
                                    &[
                                        ("id", "edergi-toc-jump"),
                                        ("class", "edergi-toc-jump"),
                                        ("aria-label", &fb_toc_aria),
                                    ],
                                    |_| {},
                                );
                                tb.el("div", &[("class", "edergi-fb-zoom-group")], |zg| {
                                    zg.el(
                                        "button",
                                        &[
                                            ("type", "button"),
                                            ("class", "button secondary edergi-fb-btn"),
                                            ("data-fb-zoom", "out"),
                                            ("aria-label", &fb_zoom_out_aria),
                                        ],
                                        |btn| {
                                            btn.text("−");
                                        },
                                    );
                                    zg.el("span", &[("id", "edergi-zoom-level"), ("class", "edergi-zoom-level")], |sp| {
                                        sp.text("100%");
                                    });
                                    zg.el(
                                        "button",
                                        &[
                                            ("type", "button"),
                                            ("class", "button secondary edergi-fb-btn"),
                                            ("data-fb-zoom", "in"),
                                            ("aria-label", &fb_zoom_in_aria),
                                        ],
                                        |btn| {
                                            btn.text("+");
                                        },
                                    );
                                    zg.el(
                                        "button",
                                        &[
                                            ("type", "button"),
                                            ("class", "button secondary edergi-fb-btn"),
                                            ("data-fb-zoom", "reset"),
                                            ("aria-label", &fb_zoom_reset_aria),
                                        ],
                                        |btn| {
                                            btn.text("1:1");
                                        },
                                    );
                                    zg.el(
                                        "button",
                                        &[
                                            ("type", "button"),
                                            ("id", "edergi-fullscreen-btn"),
                                            ("class", "button secondary edergi-fb-btn"),
                                            ("data-fb-fullscreen", ""),
                                        ],
                                        |btn| {
                                            btn.text(&fb_fullscreen_lbl);
                                        },
                                    );
                                });
                            });
                            panel.el(
                                "div",
                                &[
                                    ("class", "edergi-fb-stage"),
                                    ("tabindex", "0"),
                                    ("data-fb-stage", ""),
                                ],
                                |stage| {
                                    stage.el("div", &[("class", "edergi-fb-viewport")], |vp| {
                                        vp.el("div", &[("id", "edergi-book"), ("class", "edergi-book")], |book| {
                                            book.el("div", &[("id", "edergi-slot-left"), ("class", "edergi-page-slot is-left")], |slot| {
                                                slot.el("span", &[("id", "edergi-badge-left"), ("class", "edergi-page-badge")], |_| {});
                                                slot.void_el("img", &[("id", "edergi-page-left"), ("alt", ""), ("decoding", "async")]);
                                                slot.el("div", &[("class", "edergi-page-skeleton"), ("aria-hidden", "true")], |_| {});
                                            });
                                            book.el("div", &[("class", "edergi-spine"), ("aria-hidden", "true")], |_| {});
                                            book.el("div", &[("id", "edergi-slot-right"), ("class", "edergi-page-slot is-right")], |slot| {
                                                slot.el("span", &[("id", "edergi-badge-right"), ("class", "edergi-page-badge")], |_| {});
                                                slot.void_el("img", &[("id", "edergi-page-right"), ("alt", ""), ("decoding", "async")]);
                                                slot.el("div", &[("class", "edergi-page-skeleton"), ("aria-hidden", "true")], |_| {});
                                            });
                                        });
                                    });
                                    stage.el_with(
                                        "div",
                                        |attrs| {
                                            attrs
                                                .attr("id", "edergi-fb-gate")
                                                .attr("class", "edergi-fb-gate")
                                                .bool_attr("hidden", true);
                                        },
                                        |gate| {
                                            gate.el("p", &[("id", "edergi-fb-gate-msg")], |_| {});
                                            gate.el("div", &[("id", "edergi-fb-gate-actions"), ("class", "hero-actions")], |_| {});
                                        },
                                    );
                                },
                            );
                        },
                    );
                    col.el_with(
                        "div",
                        |attrs| {
                            attrs
                                .attr("id", "issue-pdf-panel")
                                .attr("class", "issue-pdf-panel")
                                .attr("role", "tabpanel")
                                .attr("aria-labelledby", "issue-tab-pdf")
                                .bool_attr("hidden", true);
                        },
                        |panel| {
                            if ctx.can_read_pdf {
                                let iframe_title = t(
                                    ctx,
                                    "dialogs.issue.pdf_frame_title",
                                    "Filigranlı E-Dergi PDF Okuyucu",
                                    "Watermarked E-Magazine PDF Reader",
                                );
                                panel.el(
                                    "iframe",
                                    &[
                                        ("id", "issue-pdf-frame"),
                                        ("class", "issue-pdf-frame"),
                                        ("title", &iframe_title),
                                        ("src", &pdf_src),
                                        ("data-subscriber", "1"),
                                        ("loading", "lazy"),
                                    ],
                                    |_| {},
                                );
                            } else {
                                let locked_msg = t(
                                    ctx,
                                    "dialogs.issue.pdf_locked",
                                    "Tam sayı PDF okuyucusu aktif aboneler ve doğrulanmış kamu personeli içindir.",
                                    "The full-issue PDF reader is available to active subscribers and verified public-sector personnel.",
                                );
                                let login_cta = t(
                                    ctx,
                                    "account.login_cta",
                                    "Giriş yap / Hesap oluştur →",
                                    "Sign in / Create account →",
                                );
                                let sub_cta = t(
                                    ctx,
                                    "reader.view_subscriptions",
                                    "Abonelikleri incele",
                                    "Explore subscriptions",
                                );
                                let sub_href = lang_href("/abonelik-ve-satis", ctx.lang);
                                panel.el("div", &[("class", "issue-pdf-locked")], |box_el| {
                                    box_el.el("p", &[], |p| {
                                        p.text(&locked_msg);
                                    });
                                    box_el.el("div", &[("class", "hero-actions")], |acts| {
                                        acts.el(
                                            "a",
                                            &[("href", "#modal-login"), ("class", "button primary")],
                                            |a| {
                                                a.text(&login_cta);
                                            },
                                        );
                                        acts.el(
                                            "a",
                                            &[("href", &sub_href), ("class", "button secondary")],
                                            |a| {
                                                a.text(&sub_cta);
                                            },
                                        );
                                    });
                                });
                            }
                        },
                    );
                });
            });
        },
    );
}

fn render_auth_dialog(buf: &mut HtmlBuf, ctx: &DialogContext<'_>) {
    let close_aria = close_label(ctx);
    let eyebrow = t(ctx, "dialogs.auth.eyebrow", "MSI OKUYUCU MERKEZİ", "MSI READER CENTER");
    let welcome = t(ctx, "dialogs.auth.welcome", "Hoş geldiniz", "Welcome");
    let tabs_aria = t(ctx, "dialogs.auth.tabs_aria", "Hesap işlemleri", "Account actions");
    let login_lbl = t(ctx, "dialogs.auth.login", "Giriş yap", "Sign in");
    let register_lbl = t(ctx, "dialogs.auth.register", "Hesap oluştur", "Create account");
    let email_lbl = t(ctx, "dialogs.auth.email", "E-posta", "Email");
    let pw_lbl = t(ctx, "dialogs.auth.password", "Şifre", "Password");
    let forgot_lbl = t(ctx, "dialogs.auth.forgot", "Şifremi unuttum", "Forgot password");
    let login_btn = t(ctx, "dialogs.auth.login_submit", "Giriş yap →", "Sign in →");

    let forgot_title = t(ctx, "dialogs.auth.forgot_title", "Şifre Sıfırlama Talebi", "Password Reset Request");
    let forgot_desc = t(
        ctx,
        "dialogs.auth.forgot_desc",
        "Kayıtlı kurumsal veya bireysel e-posta adresinizi girin. Güvenli şifre sıfırlama bağlantısı adresinize iletilecektir.",
        "Enter your registered corporate or personal email address. A secure password reset link will be sent to your inbox.",
    );
    let forgot_email_lbl = t(ctx, "dialogs.auth.forgot_email", "Kayıtlı e-posta adresi", "Registered email address");
    let forgot_placeholder = t(
        ctx,
        "dialogs.auth.forgot_placeholder",
        "ad.soyad@kurum.gov.tr",
        "name.surname@institution.gov.tr",
    );
    let forgot_submit = t(
        ctx,
        "dialogs.auth.forgot_submit",
        "Sıfırlama bağlantısı gönder →",
        "Send reset link →",
    );
    let back_login = t(
        ctx,
        "dialogs.auth.back_to_login",
        "← Giriş ekranına dön",
        "← Back to sign in",
    );

    let totp_title = t(
        ctx,
        "dialogs.auth.totp_title",
        "İki Aşamalı Doğrulama (2FA)",
        "Two-Factor Authentication (2FA)",
    );
    let totp_desc = t(
        ctx,
        "dialogs.auth.totp_desc",
        "Authenticator uygulamanızdaki doğrulama kodunu veya tek kullanımlık yedek kurtarma kodunuzu girin.",
        "Enter the verification code from your authenticator app or a single-use backup recovery code.",
    );
    let totp_code_lbl = t(
        ctx,
        "dialogs.auth.totp_code",
        "Doğrulama kodu / Yedek kod",
        "Verification code / Backup code",
    );
    let totp_submit = t(
        ctx,
        "dialogs.auth.totp_submit",
        "Doğrula ve giriş yap →",
        "Verify and sign in →",
    );
    let totp_back = t(
        ctx,
        "dialogs.auth.totp_back",
        "← Farklı hesapla giriş yap",
        "← Sign in with another account",
    );

    let full_name_lbl = t(ctx, "dialogs.auth.full_name", "Ad soyad", "Full name");
    let corp_email_lbl = t(ctx, "dialogs.auth.corp_email", "Kurumsal e-posta", "Work email");
    let gov_hint = t(
        ctx,
        "dialogs.auth.gov_hint",
        "✓ Resmi kamu / akademik e-posta uzantısı algılandı — ücretsiz dijital E-Dergi yetkisi otomatik tanımlanacaktır.",
        "✓ Official public / academic email domain detected — complimentary digital E-Magazine access will be assigned automatically.",
    );
    let reg_pw_lbl = t(
        ctx,
        "dialogs.auth.reg_password",
        "Şifre (en az 8 karakter)",
        "Password (min. 8 characters)",
    );
    let job_lbl = t(ctx, "dialogs.auth.job_title", "Görev / Ünvan", "Role / Title");
    let job_ph = t(
        ctx,
        "dialogs.auth.job_placeholder",
        "Örn: Sistem Mühendisi, Proje Subayı",
        "e.g. Systems Engineer, Programme Officer",
    );
    let org_name_lbl = t(
        ctx,
        "dialogs.auth.org_name",
        "Kurum adı",
        "Institution / Company name",
    );
    let org_cat_lbl = t(
        ctx,
        "dialogs.auth.org_cat",
        "Kurum kategorisi (Resmi Taksonomi)",
        "Institution category (Official Taxonomy)",
    );
    let ssb_legend = t(
        ctx,
        "dialogs.auth.ssb_legend",
        "SSB Teknoloji İlgi Alanları (Kişiselleştirilmiş Akış)",
        "SSB Technology Interest Areas (Personalised Feed)",
    );
    let req_msg_lbl = t(
        ctx,
        "dialogs.auth.req_message",
        "Kurumsal doğrulama notu (Opsiyonel)",
        "Corporate verification note (Optional)",
    );
    let req_msg_ph = t(
        ctx,
        "dialogs.auth.req_message_ph",
        "Kamu e-postası dışındaki başvurular için birim / sicil referansı",
        "Department or badge reference for non-public email domains",
    );
    let reg_submit = t(ctx, "dialogs.auth.register_submit", "Hesap oluştur →", "Create account →");
    let reg_step1_title = t(
        ctx,
        "dialogs.auth.reg_step1_title",
        "Kimlik ve Hesap Bilgileri",
        "Identity & Credentials",
    );
    let reg_step2_title = t(
        ctx,
        "dialogs.auth.reg_step2_title",
        "Kurum ve Görev Doğrulama",
        "Organisation & Role Verification",
    );
    let reg_gov_note = t(
        ctx,
        "dialogs.auth.reg_gov_note",
        ".gov.tr / .tsk.tr kurumsal e-posta adresleri anında dijital E-Dergi erişimi kazanır.",
        "Official .gov.tr / .tsk.tr domains unlock instant digital E-Magazine access.",
    );
    let reg_step3_title = t(
        ctx,
        "dialogs.auth.reg_step3_title",
        "SSB Teknoloji İlgi Alanları",
        "SSB Technology Interests",
    );
    let reg_step4_title = t(
        ctx,
        "dialogs.auth.reg_step4_title",
        "Yasal Onaylar (KVKK & ETK)",
        "Legal & Communication Consents",
    );

    let kvkk_href = lang_href("/kurumsal/kvkk", ctx.lang);
    let terms_href = lang_href("/kurumsal/kullanim-kosullari", ctx.lang);
    let etk_href = lang_href("/kurumsal/ticari-ileti-aydinlatma", ctx.lang);

    let fallback_institutions;
    let choices: &[InstitutionChoice] = if ctx.institutions.is_empty() {
        fallback_institutions = default_institution_choices();
        &fallback_institutions
    } else {
        ctx.institutions
    };

    buf.el(
        "dialog",
        &[
            ("id", "auth-dialog"),
            ("class", "ui-dialog auth-dialog"),
            ("aria-labelledby", "auth-title"),
        ],
        |b| {
            write_dialog_heading(b, None, &eyebrow, "auth-title", &welcome, &close_aria);
            b.el(
                "div",
                &[("class", "auth-tabs"), ("role", "tablist"), ("aria-label", &tabs_aria)],
                |tabs| {
                    tabs.el(
                        "button",
                        &[
                            ("id", "login-tab"),
                            ("type", "button"),
                            ("role", "tab"),
                            ("aria-selected", "true"),
                            ("aria-controls", "login-form"),
                            ("data-auth-tab", "login"),
                        ],
                        |btn| {
                            btn.text(&login_lbl);
                        },
                    );
                    tabs.el(
                        "button",
                        &[
                            ("id", "register-tab"),
                            ("type", "button"),
                            ("role", "tab"),
                            ("aria-selected", "false"),
                            ("aria-controls", "register-form"),
                            ("tabindex", "-1"),
                            ("data-auth-tab", "register"),
                        ],
                        |btn| {
                            btn.text(&register_lbl);
                        },
                    );
                },
            );

            // Login form
            b.el(
                "form",
                &[
                    ("id", "login-form"),
                    ("role", "tabpanel"),
                    ("aria-labelledby", "login-tab"),
                    ("class", "ui-form"),
                ],
                |f| {
                    append_antispam_fields(f);
                    f.el("label", &[], |lbl| {
                        lbl.text(&email_lbl);
                        lbl.void_with("input", |inp| {
                            inp.attr("name", "email")
                                .attr("type", "email")
                                .attr("autocomplete", "email")
                                .bool_attr("required", true);
                        });
                    });
                    f.el("label", &[], |lbl| {
                        lbl.el("span", &[("class", "auth-label-row")], |row| {
                            row.el("span", &[], |s| {
                                s.text(&pw_lbl);
                            });
                            row.el(
                                "button",
                                &[
                                    ("type", "button"),
                                    ("class", "text-link auth-forgot-trigger"),
                                    ("data-auth-view", "forgot"),
                                ],
                                |btn| {
                                    btn.text(&forgot_lbl);
                                },
                            );
                        });
                        lbl.void_with("input", |inp| {
                            inp.attr("name", "password")
                                .attr("type", "password")
                                .attr("autocomplete", "current-password")
                                .bool_attr("required", true);
                        });
                    });
                    f.el("p", &[("class", "form-feedback"), ("role", "status")], |_| {});
                    f.el("button", &[("class", "button primary"), ("type", "submit")], |btn| {
                        btn.text(&login_btn);
                    });
                },
            );

            // Forgot password form
            b.el_with(
                "form",
                |attrs| {
                    attrs
                        .attr("id", "forgot-password-form")
                        .attr("class", "ui-form")
                        .bool_attr("hidden", true);
                },
                |f| {
                    append_antispam_fields(f);
                    f.el("div", &[("class", "auth-step-banner")], |banner| {
                        banner.el("strong", &[], |s| {
                            s.text(&forgot_title);
                        });
                        banner.el("p", &[], |p| {
                            p.text(&forgot_desc);
                        });
                    });
                    f.el("label", &[], |lbl| {
                        lbl.text(&forgot_email_lbl);
                        lbl.void_with("input", |inp| {
                            inp.attr("name", "email")
                                .attr("type", "email")
                                .attr("autocomplete", "email")
                                .attr("placeholder", &forgot_placeholder)
                                .bool_attr("required", true);
                        });
                    });
                    f.el("p", &[("class", "form-feedback"), ("role", "status")], |_| {});
                    f.el("button", &[("class", "button primary"), ("type", "submit")], |btn| {
                        btn.text(&forgot_submit);
                    });
                    f.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "text-link auth-back-btn"),
                            ("data-auth-view", "login"),
                        ],
                        |btn| {
                            btn.text(&back_login);
                        },
                    );
                },
            );

            // TOTP verification form
            b.el_with(
                "form",
                |attrs| {
                    attrs
                        .attr("id", "totp-login-form")
                        .attr("class", "ui-form")
                        .bool_attr("hidden", true);
                },
                |f| {
                    append_antispam_fields(f);
                    f.el("div", &[("class", "auth-step-banner is-security")], |banner| {
                        banner.el("strong", &[], |s| {
                            s.text(&totp_title);
                        });
                        banner.el("p", &[], |p| {
                            p.text(&totp_desc);
                        });
                    });
                    f.void_el("input", &[("type", "hidden"), ("name", "temp_token"), ("value", "")]);
                    f.el("label", &[], |lbl| {
                        lbl.text(&totp_code_lbl);
                        lbl.void_with("input", |inp| {
                            inp.attr("name", "code")
                                .attr("type", "text")
                                .attr("class", "totp-code-input")
                                .attr("autocomplete", "one-time-code")
                                .attr("minlength", "6")
                                .attr("maxlength", "20")
                                .attr("placeholder", "000000")
                                .bool_attr("required", true);
                        });
                    });
                    f.el("p", &[("class", "form-feedback"), ("role", "status")], |_| {});
                    f.el("button", &[("class", "button primary"), ("type", "submit")], |btn| {
                        btn.text(&totp_submit);
                    });
                    f.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "text-link auth-back-btn"),
                            ("data-auth-view", "login"),
                        ],
                        |btn| {
                            btn.text(&totp_back);
                        },
                    );
                },
            );

            // Register form
            b.el_with(
                "form",
                |attrs| {
                    attrs
                        .attr("id", "register-form")
                        .attr("role", "tabpanel")
                        .attr("aria-labelledby", "register-tab")
                        .attr("class", "ui-form")
                        .bool_attr("hidden", true);
                },
                |f| {
                    append_antispam_fields(f);
                    // Step 1: Identity & Credentials
                    f.el("section", &[("class", "auth-reg-section"), ("data-reg-step", "1")], |sec| {
                        sec.el("div", &[("class", "auth-reg-section-head")], |hd| {
                            hd.el("span", &[("class", "auth-reg-step-num")], |n| {
                                n.text("01");
                            });
                            hd.el("span", &[("class", "auth-reg-section-title")], |tl| {
                                tl.text(&reg_step1_title);
                            });
                        });
                        sec.el("div", &[("class", "form-grid-2")], |row| {
                            row.el("label", &[], |lbl| {
                                lbl.text(&full_name_lbl);
                                lbl.void_with("input", |inp| {
                                    inp.attr("name", "full_name")
                                        .attr("autocomplete", "name")
                                        .attr("maxlength", "100")
                                        .bool_attr("required", true);
                                });
                            });
                            row.el("label", &[], |lbl| {
                                lbl.text(&corp_email_lbl);
                                lbl.void_with("input", |inp| {
                                    inp.attr("name", "email")
                                        .attr("type", "email")
                                        .attr("autocomplete", "email")
                                        .bool_attr("required", true);
                                });
                            });
                        });
                        sec.el_with(
                            "div",
                            |attrs| {
                                attrs
                                    .attr("id", "gov-domain-hint")
                                    .attr("class", "gov-domain-hint")
                                    .bool_attr("hidden", true);
                            },
                            |hint| {
                                hint.text(&gov_hint);
                            },
                        );
                        sec.el("label", &[], |lbl| {
                            lbl.text(&reg_pw_lbl);
                            lbl.void_with("input", |inp| {
                                inp.attr("name", "password")
                                    .attr("type", "password")
                                    .attr("autocomplete", "new-password")
                                    .attr("minlength", "8")
                                    .bool_attr("required", true);
                            });
                        });
                    });

                    // Step 2: Organisation & Role Verification
                    f.el("section", &[("class", "auth-reg-section"), ("data-reg-step", "2")], |sec| {
                        sec.el("div", &[("class", "auth-reg-section-head")], |hd| {
                            hd.el("span", &[("class", "auth-reg-step-num")], |n| {
                                n.text("02");
                            });
                            hd.el("span", &[("class", "auth-reg-section-title")], |tl| {
                                tl.text(&reg_step2_title);
                            });
                        });
                        sec.el("div", &[("class", "auth-gov-note")], |note| {
                            note.text(&reg_gov_note);
                        });
                        sec.el("div", &[("class", "form-grid-2")], |row| {
                            row.el("label", &[], |lbl| {
                                lbl.text(&org_name_lbl);
                                lbl.void_with("input", |inp| {
                                    inp.attr("name", "organization_name")
                                        .attr("autocomplete", "organization")
                                        .bool_attr("required", true);
                                });
                            });
                            row.el("label", &[], |lbl| {
                                lbl.text(&org_cat_lbl);
                                lbl.el("select", &[("name", "organization_category")], |sel| {
                                    for inst in choices {
                                        let lbl_txt = if ctx.lang.eq_ignore_ascii_case("en")
                                            && !inst.label_en.trim().is_empty()
                                        {
                                            &inst.label_en
                                        } else {
                                            &inst.label_tr
                                        };
                                        sel.el("option", &[("value", &inst.code)], |opt| {
                                            opt.text(lbl_txt);
                                        });
                                    }
                                });
                            });
                        });
                        sec.el("div", &[("class", "form-grid-2")], |row| {
                            row.el("label", &[], |lbl| {
                                lbl.text(&job_lbl);
                                lbl.void_el(
                                    "input",
                                    &[
                                        ("name", "job_title"),
                                        ("autocomplete", "organization-title"),
                                        ("placeholder", &job_ph),
                                        ("maxlength", "100"),
                                    ],
                                );
                            });
                            row.el("label", &[], |lbl| {
                                lbl.text(&req_msg_lbl);
                                lbl.void_el(
                                    "input",
                                    &[
                                        ("name", "request_message"),
                                        ("maxlength", "240"),
                                        ("placeholder", &req_msg_ph),
                                    ],
                                );
                            });
                        });
                    });

                    // Step 3: SSB Technology Interests
                    f.el("section", &[("class", "auth-reg-section"), ("data-reg-step", "3")], |sec| {
                        sec.el("div", &[("class", "auth-reg-section-head")], |hd| {
                            hd.el("span", &[("class", "auth-reg-step-num")], |n| {
                                n.text("03");
                            });
                            hd.el("span", &[("class", "auth-reg-section-title")], |tl| {
                                tl.text(&reg_step3_title);
                            });
                        });
                        sec.el("fieldset", &[("class", "ssb-fieldset")], |fs| {
                            fs.el("legend", &[], |lg| {
                                lg.text(&ssb_legend);
                            });
                            fs.el("div", &[("id", "register-ssb-tags"), ("class", "ssb-tag-picks")], |_| {});
                        });
                    });

                    // Step 4: Legal & Communication Consents
                    f.el("section", &[("class", "auth-reg-section"), ("data-reg-step", "4")], |sec| {
                        sec.el("div", &[("class", "auth-reg-section-head")], |hd| {
                            hd.el("span", &[("class", "auth-reg-step-num")], |n| {
                                n.text("04");
                            });
                            hd.el("span", &[("class", "auth-reg-section-title")], |tl| {
                                tl.text(&reg_step4_title);
                            });
                        });
                        sec.void_el(
                            "input",
                            &[
                                ("type", "hidden"),
                                ("name", "tos_version"),
                                ("value", "2026-01"),
                            ],
                        );
                        sec.el("label", &[("class", "check-label")], |lbl| {
                            lbl.void_with("input", |inp| {
                                inp.attr("type", "checkbox")
                                    .attr("name", "kvkk_consent")
                                    .bool_attr("required", true);
                            });
                            lbl.el("span", &[], |span| {
                                if ctx.lang.eq_ignore_ascii_case("en") {
                                    span.text("I have read the ");
                                    span.el(
                                        "a",
                                        &[("href", &kvkk_href), ("target", "_blank"), ("rel", "noopener")],
                                        |a| {
                                            a.text("KVKK Personal Data Disclosure Notice");
                                        },
                                    );
                                    span.text(" and acknowledge the processing of my personal data.");
                                } else {
                                    span.el(
                                        "a",
                                        &[("href", &kvkk_href), ("target", "_blank"), ("rel", "noopener")],
                                        |a| {
                                            a.text("6698 sayılı KVKK Aydınlatma Metni");
                                        },
                                    );
                                    span.text("'ni okudum; kişisel verilerimin işlenmesine ilişkin aydınlatıldım.");
                                }
                            });
                        });
                        sec.el("label", &[("class", "check-label check-label-tos")], |lbl| {
                            lbl.void_with("input", |inp| {
                                inp.attr("type", "checkbox")
                                    .attr("name", "tos_accepted")
                                    .attr("id", "register-tos-accepted")
                                    .attr("value", "true")
                                    .bool_attr("required", true);
                            });
                            lbl.el("span", &[], |span| {
                                if ctx.lang.eq_ignore_ascii_case("en") {
                                    span.text("I have read and accept the ");
                                    span.el(
                                        "a",
                                        &[("href", &terms_href), ("target", "_blank"), ("rel", "noopener")],
                                        |a| {
                                            a.text("Membership & Terms of Use");
                                        },
                                    );
                                    span.text(" (version 2026-01).");
                                } else {
                                    span.text("2026-01 sürüm tarihli ");
                                    span.el(
                                        "a",
                                        &[("href", &terms_href), ("target", "_blank"), ("rel", "noopener")],
                                        |a| {
                                            a.text("Üyelik ve Kullanım Koşulları");
                                        },
                                    );
                                    span.text("'nı okudum, kabul ediyorum.");
                                }
                            });
                        });
                        sec.el("label", &[("class", "check-label")], |lbl| {
                            lbl.void_el("input", &[("type", "checkbox"), ("name", "email_consent")]);
                            lbl.el("span", &[], |span| {
                                if ctx.lang.eq_ignore_ascii_case("en") {
                                    span.text("I would like to receive email updates on new issues, special dossiers, and defence briefings under the ");
                                    span.el(
                                        "a",
                                        &[("href", &etk_href), ("target", "_blank"), ("rel", "noopener")],
                                        |a| {
                                            a.text("Commercial Electronic Messaging Notice");
                                        },
                                    );
                                    span.text(" (Optional).");
                                } else {
                                    span.el(
                                        "a",
                                        &[("href", &etk_href), ("target", "_blank"), ("rel", "noopener")],
                                        |a| {
                                            a.text("6563 sayılı ETK Ticari Elektronik İleti Metni");
                                        },
                                    );
                                    span.text(" kapsamında yeni sayılar, özel dosyalar ve savunma bültenleri hakkında e-posta ile bilgilendirilmek istiyorum (İsteğe bağlı).");
                                }
                            });
                        });
                    });

                    f.el("p", &[("class", "form-feedback"), ("role", "status")], |_| {});
                    f.el("button", &[("class", "button primary"), ("type", "submit")], |btn| {
                        btn.text(&reg_submit);
                    });
                },
            );
        },
    );
}

fn render_offer_dialog(buf: &mut HtmlBuf, ctx: &DialogContext<'_>) {
    let close_aria = close_label(ctx);
    let eyebrow = t(
        ctx,
        "dialogs.offer.eyebrow",
        "ABONELİK VE YAYINLAR",
        "SUBSCRIPTIONS & PUBLICATIONS",
    );
    let full_name_lbl = t(ctx, "dialogs.auth.full_name", "Ad soyad", "Full name");
    let email_lbl = t(ctx, "dialogs.auth.corp_email", "Kurumsal e-posta", "Work email");
    let org_lbl = t(ctx, "dialogs.offer.org_name", "Kurum / Şirket adı", "Institution / Company name");
    let submit_lbl = t(
        ctx,
        "dialogs.offer.submit",
        "Online sipariş & proforma oluştur →",
        "Create online order & proforma →",
    );
    let contact_email_lbl = t(
        ctx,
        "dialogs.offer.contact_email",
        "E-posta ile bilgi al ↗",
        "Request info via email ↗",
    );
    let other_contact_lbl = t(
        ctx,
        "dialogs.offer.other_contact",
        "Diğer iletişim seçenekleri →",
        "Other contact options →",
    );
    let dist_href = lang_href("/kurumsal/mesafeli-satis-sozlesmesi", ctx.lang);
    let refund_href = lang_href("/kurumsal/iptal-ve-iade-kosullari", ctx.lang);
    let contact_href = lang_href("/kurumsal/iletisim", ctx.lang);

    buf.el(
        "dialog",
        &[
            ("id", "offer-dialog"),
            ("class", "ui-dialog offer-dialog"),
            ("aria-labelledby", "offer-title"),
        ],
        |b| {
            write_dialog_heading(b, None, &eyebrow, "offer-title", "", &close_aria);
            b.el("p", &[("id", "offer-description")], |_| {});
            b.el("div", &[("class", "plan-price"), ("id", "offer-price")], |_| {});
            b.el("ul", &[("id", "offer-features"), ("class", "offer-features")], |_| {});
            b.el("form", &[("id", "offer-checkout-form"), ("class", "ui-form offer-checkout-form")], |f| {
                f.void_el("input", &[("type", "hidden"), ("name", "item_type"), ("value", "subscription")]);
                f.void_el("input", &[("type", "hidden"), ("name", "item_code"), ("value", "")]);
                f.el("div", &[("class", "form-grid-2")], |row| {
                    row.el("label", &[], |lbl| {
                        lbl.text(&full_name_lbl);
                        lbl.void_with("input", |inp| {
                            inp.attr("name", "full_name")
                                .attr("autocomplete", "name")
                                .attr("maxlength", "100")
                                .bool_attr("required", true);
                        });
                    });
                    row.el("label", &[], |lbl| {
                        lbl.text(&email_lbl);
                        lbl.void_with("input", |inp| {
                            inp.attr("name", "email")
                                .attr("type", "email")
                                .attr("autocomplete", "email")
                                .bool_attr("required", true);
                        });
                    });
                });
                f.el("label", &[], |lbl| {
                    lbl.text(&org_lbl);
                    lbl.void_with("input", |inp| {
                        inp.attr("name", "organization_name")
                            .attr("autocomplete", "organization")
                            .attr("maxlength", "120")
                            .bool_attr("required", true);
                    });
                });
                f.el("label", &[("class", "check-label")], |lbl| {
                    lbl.void_with("input", |inp| {
                        inp.attr("type", "checkbox")
                            .attr("name", "distance_sales_consent")
                            .bool_attr("required", true);
                    });
                    lbl.el("span", &[], |span| {
                        if ctx.lang.eq_ignore_ascii_case("en") {
                            span.text("I have read and accept the ");
                            span.el(
                                "a",
                                &[("href", &dist_href), ("target", "_blank"), ("rel", "noopener")],
                                |a| {
                                    a.text("Preliminary Information & Distance Sales Agreement");
                                },
                            );
                            span.text(" and ");
                            span.el(
                                "a",
                                &[("href", &refund_href), ("target", "_blank"), ("rel", "noopener")],
                                |a| {
                                    a.text("Cancellation & Refund Policy");
                                },
                            );
                            span.text(".");
                        } else {
                            span.el(
                                "a",
                                &[("href", &dist_href), ("target", "_blank"), ("rel", "noopener")],
                                |a| {
                                    a.text("Ön Bilgilendirme Formu ve Mesafeli Satış Sözleşmesi");
                                },
                            );
                            span.text(" ile ");
                            span.el(
                                "a",
                                &[("href", &refund_href), ("target", "_blank"), ("rel", "noopener")],
                                |a| {
                                    a.text("İptal ve İade Koşulları");
                                },
                            );
                            span.text("'nı okudum, kabul ediyorum.");
                        }
                    });
                });
                f.el("p", &[("class", "form-feedback"), ("id", "offer-feedback"), ("role", "status")], |_| {});
                f.el_with(
                    "div",
                    |attrs| {
                        attrs
                            .attr("id", "paytr-iframe-wrap")
                            .attr("class", "paytr-iframe-wrap")
                            .bool_attr("hidden", true);
                    },
                    |_| {},
                );
                f.el("button", &[("class", "button primary"), ("type", "submit")], |btn| {
                    btn.text(&submit_lbl);
                });
            });
            b.el("div", &[("class", "offer-secondary-actions")], |acts| {
                acts.el("a", &[("id", "offer-contact"), ("class", "button secondary")], |a| {
                    a.text(&contact_email_lbl);
                });
                acts.el("a", &[("href", &contact_href), ("class", "text-link")], |a| {
                    a.text(&other_contact_lbl);
                });
            });
        },
    );
}

fn render_cookie_consent_bar(buf: &mut HtmlBuf, ctx: &DialogContext<'_>) {
    let aria_lbl = t(
        ctx,
        "dialogs.cookie.bar_aria",
        "Çerez tercih bildirimi",
        "Cookie consent notice",
    );
    let eyebrow = t(
        ctx,
        "dialogs.cookie.bar_eyebrow",
        "6698 SAYILI KVKK · ÇEREZ TERCİHLERİ",
        "PRIVACY · COOKIE PREFERENCES",
    );
    let copy_prefix = t(
        ctx,
        "dialogs.cookie.bar_copy",
        "Zorunlu oturum ve güvenlik çerezleri dışındaki birinci taraf okuma analitiği ve sponsorluk ölçümlemesi yalnızca açık rızanızla etkinleştirilir. Detaylar: ",
        "First-party reading analytics and sponsorship measurement outside strictly necessary session and security cookies are enabled only with your explicit consent. Details: ",
    );
    let policy_lbl = t(
        ctx,
        "dialogs.cookie.policy_link",
        "Çerez Politikası",
        "Cookie Policy",
    );
    let accept_all = t(ctx, "dialogs.cookie.accept_all", "Tümünü kabul et", "Accept all");
    let necessary = t(ctx, "dialogs.cookie.necessary_only", "Yalnızca zorunlu", "Necessary only");
    let manage = t(ctx, "dialogs.cookie.manage", "Tercihleri yönet", "Manage preferences");
    let cookie_href = lang_href("/kurumsal/cerez-politikasi", ctx.lang);

    buf.el_with(
        "aside",
        |attrs| {
            attrs
                .attr("id", "cookie-consent-bar")
                .attr("class", "cookie-consent-bar")
                .attr("aria-label", &aria_lbl)
                .bool_attr("hidden", true);
        },
        |b| {
            b.el("div", &[("class", "cookie-bar-eyebrow")], |eb| {
                eb.text(&eyebrow);
            });
            b.el("p", &[("class", "cookie-bar-copy")], |p| {
                p.text(&copy_prefix);
                p.el("a", &[("href", &cookie_href)], |a| {
                    a.text(&policy_lbl);
                });
                p.text(".");
            });
            b.el("div", &[("class", "cookie-bar-actions")], |acts| {
                acts.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "button primary cookie-btn"),
                        ("data-cookie-accept", "all"),
                    ],
                    |btn| {
                        btn.text(&accept_all);
                    },
                );
                acts.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "button secondary cookie-btn"),
                        ("data-cookie-accept", "necessary"),
                    ],
                    |btn| {
                        btn.text(&necessary);
                    },
                );
                acts.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "text-link cookie-manage-btn"),
                        ("data-open-dialog", "cookie-dialog"),
                    ],
                    |btn| {
                        btn.text(&manage);
                    },
                );
            });
        },
    );
}

fn render_cookie_dialog(buf: &mut HtmlBuf, ctx: &DialogContext<'_>) {
    let close_aria = close_label(ctx);
    let eyebrow = t(
        ctx,
        "dialogs.cookie.dialog_eyebrow",
        "6698 SAYILI KVKK & ÇEREZ REHBERİ UYUMU",
        "PRIVACY & COOKIE PREFERENCES",
    );
    let title = t(
        ctx,
        "dialogs.cookie.dialog_title",
        "Çerez ve Gizlilik Tercihleri",
        "Cookie & Privacy Preferences",
    );
    let policy_lbl = t(ctx, "dialogs.cookie.policy_link", "Çerez Politikası", "Cookie Policy");
    let cookie_href = lang_href("/kurumsal/cerez-politikasi", ctx.lang);

    let necessary_title = t(
        ctx,
        "dialogs.cookie.necessary_title",
        "Zorunlu Oturum, Tema ve Güvenlik Çerezleri",
        "Strictly Necessary Session, Theme & Security Cookies",
    );
    let analytics_title = t(
        ctx,
        "dialogs.cookie.analytics_title",
        "Birinci Taraf Okuma Derinliği Analitiği (İsteğe Bağlı)",
        "First-Party Reading Analytics (Optional)",
    );
    let analytics_desc = t(
        ctx,
        "dialogs.cookie.analytics_desc",
        "Makale okuma süresi ve sayfa kaydırma oranının anonimleştirilmiş olarak ölçülmesini sağlar (KVKK m.5/1 açık rıza).",
        "Measures anonymised article reading depth and duration with your explicit consent.",
    );
    let marketing_title = t(
        ctx,
        "dialogs.cookie.marketing_title",
        "Kurumsal Sponsorluk Gösterim Ölçümü (İsteğe Bağlı)",
        "Corporate Sponsorship Impression Measurement (Optional)",
    );
    let marketing_desc = t(
        ctx,
        "dialogs.cookie.marketing_desc",
        "Savunma sanayii kurumsal tanıtım alanlarının gösterim ve tıklama sayımını etkinleştirir; davranışsal profilleme yapılmaz (KVKK m.5/1 açık rıza).",
        "Enables impression and click counting for defence corporate sponsorships without behavioural profiling.",
    );
    let save_lbl = t(ctx, "dialogs.cookie.save", "Seçimlerimi kaydet", "Save preferences");
    let accept_all = t(ctx, "dialogs.cookie.accept_all", "Tümünü kabul et", "Accept all");
    let necessary = t(ctx, "dialogs.cookie.necessary_only", "Yalnızca zorunlu", "Necessary only");

    buf.el(
        "dialog",
        &[
            ("id", "cookie-dialog"),
            ("class", "ui-dialog cookie-dialog"),
            ("aria-labelledby", "cookie-dialog-title"),
        ],
        |b| {
            write_dialog_heading(b, None, &eyebrow, "cookie-dialog-title", &title, &close_aria);
            b.el("p", &[("class", "cookie-dialog-intro")], |p| {
                if ctx.lang.eq_ignore_ascii_case("en") {
                    p.text("MSI Dergisi / MSI TDR does not use external third-party advertising or behavioural profiling trackers. You can enable or disable optional first-party measurement categories below at any time (");
                } else {
                    p.text("MSI Dergisi platformunda yurt dışı merkezli üçüncü taraf reklam veya profilleme takipçileri (Google Analytics, Meta Pixel vb.) kullanılmaz. Zorunlu olmayan birinci taraf ölçüm kategorilerini aşağıdan dilediğiniz an açıp kapatabilirsiniz (");
                }
                p.el("a", &[("href", &cookie_href)], |a| {
                    a.text(&policy_lbl);
                });
                p.text(").");
            });
            b.el(
                "form",
                &[
                    ("id", "cookie-preferences-form"),
                    ("class", "ui-form cookie-preferences-form"),
                ],
                |f| {
                    f.el("div", &[("class", "cookie-category-item")], |item| {
                        item.el("label", &[("class", "check-label")], |lbl| {
                            lbl.void_with("input", |inp| {
                                inp.attr("type", "checkbox")
                                    .bool_attr("checked", true)
                                    .bool_attr("disabled", true);
                            });
                            lbl.el("span", &[], |span| {
                                span.el("strong", &[], |s| {
                                    s.text(&necessary_title);
                                });
                                span.el("small", &[], |sm| {
                                    if ctx.lang.eq_ignore_ascii_case("en") {
                                        sm.text("HttpOnly session cookie (");
                                        sm.el("code", &[], |c| { c.text("msi_session"); });
                                        sm.text("), language preference (");
                                        sm.el("code", &[], |c| { c.text("msi_lang"); });
                                        sm.text("), theme (");
                                        sm.el("code", &[], |c| { c.text("msi_theme"); });
                                        sm.text("), and consent state. Always active.");
                                    } else {
                                        sm.text("HttpOnly güvenli oturum (");
                                        sm.el("code", &[], |c| { c.text("msi_session"); });
                                        sm.text("), dil tercihi (");
                                        sm.el("code", &[], |c| { c.text("msi_lang"); });
                                        sm.text("), tema (");
                                        sm.el("code", &[], |c| { c.text("msi_theme"); });
                                        sm.text(") ve çerez rıza kaydı. KVKK m.5/2-(c) ve (f) uyarınca her zaman aktiftir.");
                                    }
                                });
                            });
                        });
                    });
                    for (name, ctitle, cdesc) in [
                        ("analytics", analytics_title, analytics_desc),
                        ("marketing", marketing_title, marketing_desc),
                    ] {
                        f.el("div", &[("class", "cookie-category-item")], |item| {
                            item.el("label", &[("class", "check-label")], |lbl| {
                                lbl.void_el("input", &[("type", "checkbox"), ("name", name)]);
                                lbl.el("span", &[], |span| {
                                    span.el("strong", &[], |s| {
                                        s.text(&ctitle);
                                    });
                                    span.el("small", &[], |sm| {
                                        sm.text(&cdesc);
                                    });
                                });
                            });
                        });
                    }
                    f.el("div", &[("class", "cookie-dialog-actions")], |acts| {
                        acts.el("button", &[("type", "submit"), ("class", "button primary")], |btn| {
                            btn.text(&save_lbl);
                        });
                        acts.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("class", "button secondary"),
                                ("data-cookie-accept", "all"),
                            ],
                            |btn| {
                                btn.text(&accept_all);
                            },
                        );
                        acts.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("class", "button secondary"),
                                ("data-cookie-accept", "necessary"),
                            ],
                            |btn| {
                                btn.text(&necessary);
                            },
                        );
                    });
                },
            );
        },
    );
}

fn render_gallery_lightbox(buf: &mut HtmlBuf, ctx: &DialogContext<'_>) {
    let close_aria = close_label(ctx);
    let counter_lbl = t(ctx, "dialogs.lightbox.eyebrow", "GÖRSEL GALERİ", "IMAGE GALLERY");
    let open_raw = t(ctx, "dialogs.lightbox.open_raw", "Tam boy aç ↗", "Open full size ↗");
    let prev_aria = t(ctx, "dialogs.lightbox.prev", "Önceki görsel", "Previous image");
    let next_aria = t(ctx, "dialogs.lightbox.next", "Sonraki görsel", "Next image");

    buf.el(
        "dialog",
        &[
            ("id", "gallery-lightbox"),
            ("class", "ui-dialog lightbox-dialog"),
            ("aria-labelledby", "lightbox-caption"),
        ],
        |b| {
            b.el("div", &[("class", "lightbox-topbar")], |top| {
                top.el("span", &[("class", "eyebrow"), ("id", "lightbox-counter")], |sp| {
                    sp.text(&counter_lbl);
                });
                top.el("div", &[("class", "lightbox-actions")], |acts| {
                    acts.el(
                        "a",
                        &[
                            ("id", "lightbox-raw-link"),
                            ("class", "text-link"),
                            ("href", "#"),
                            ("target", "_blank"),
                            ("rel", "noopener"),
                        ],
                        |a| {
                            a.text(&open_raw);
                        },
                    );
                    acts.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "icon-button"),
                            ("data-close-dialog", ""),
                            ("aria-label", &close_aria),
                        ],
                        |btn| {
                            btn.raw("&times;");
                        },
                    );
                });
            });
            b.el("div", &[("class", "lightbox-stage")], |stage| {
                stage.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "button secondary lightbox-nav"),
                        ("data-lightbox-prev", ""),
                        ("aria-label", &prev_aria),
                    ],
                    |btn| {
                        btn.text("←");
                    },
                );
                stage.el("figure", &[("class", "lightbox-figure")], |fig| {
                    fig.void_el("img", &[("id", "lightbox-img"), ("alt", "")]);
                    fig.el("figcaption", &[("id", "lightbox-caption")], |_| {});
                });
                stage.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "button secondary lightbox-nav"),
                        ("data-lightbox-next", ""),
                        ("aria-label", &next_aria),
                    ],
                    |btn| {
                        btn.text("→");
                    },
                );
            });
        },
    );
}

fn render_newsletter_dialog(buf: &mut HtmlBuf, ctx: &DialogContext<'_>) {
    let close_aria = close_label(ctx);
    let eyebrow = t(
        ctx,
        "dialogs.newsletter.eyebrow",
        "MSI E-BÜLTEN",
        "MSI BRIEFINGS",
    );
    let title = t(
        ctx,
        "dialogs.newsletter.title",
        "Haftalık Savunma Bültenine Katılın",
        "Subscribe to Defence Briefings",
    );
    let desc = t(
        ctx,
        "dialogs.newsletter.desc",
        "Seçtiğiniz yayın grubuna göre yeni sayı duyuruları, özel savunma dosyaları ve fuar raporları doğrudan e-postanıza gelsin.",
        "Receive new issue announcements, special defence dossiers and exhibition reports tailored to your selected briefing list.",
    );
    let email_lbl = t(ctx, "dialogs.auth.email", "E-posta", "Email");
    let email_ph = if ctx.lang.eq_ignore_ascii_case("en") {
        "name.surname@organisation.gov.tr"
    } else {
        "ad.soyad@kurum.gov.tr"
    };
    let full_name_lbl = t(
        ctx,
        "dialogs.newsletter.full_name",
        "Ad soyad (İsteğe bağlı)",
        "Full name (Optional)",
    );
    let group_lbl = t(
        ctx,
        "dialogs.newsletter.group",
        "Bülten yayın grubu",
        "Briefing edition",
    );
    let opt_general = t(
        ctx,
        "dialogs.newsletter.opt_general",
        "Genel Savunma Bülteni",
        "General Defence Briefing",
    );
    let opt_issues = t(
        ctx,
        "dialogs.newsletter.opt_issues",
        "Yeni Sayı & Özel Dosyalar",
        "New Issues & Special Dossiers",
    );
    let opt_events = t(
        ctx,
        "dialogs.newsletter.opt_events",
        "Fuar & Etkinlik Duyuruları",
        "Exhibitions & Event Alerts",
    );
    let submit_lbl = t(
        ctx,
        "dialogs.newsletter.submit",
        "Bültene katıl",
        "Subscribe now",
    );
    let kvkk_href = lang_href("/kurumsal/kvkk", ctx.lang);
    let etk_href = lang_href("/kurumsal/ticari-ileti-aydinlatma", ctx.lang);

    buf.el(
        "dialog",
        &[
            ("id", "newsletter-dialog"),
            ("class", "ui-dialog newsletter-dialog"),
            ("aria-labelledby", "newsletter-dialog-title"),
        ],
        |b| {
            write_dialog_heading(
                b,
                None,
                &eyebrow,
                "newsletter-dialog-title",
                &title,
                &close_aria,
            );
            b.el("p", &[("class", "newsletter-dialog-desc")], |p| {
                p.text(&desc);
            });
            b.el(
                "form",
                &[
                    ("id", "public-newsletter-form"),
                    ("class", "ui-form"),
                    ("action", "/api/v1/public/newsletter/subscribe"),
                    ("data-endpoint", "/api/v1/public/newsletter/subscribe"),
                ],
                |f| {
                    append_antispam_fields(f);
                    f.el("label", &[], |lbl| {
                        lbl.text(&email_lbl);
                        lbl.void_with("input", |inp| {
                            inp.attr("name", "email")
                                .attr("type", "email")
                                .attr("autocomplete", "email")
                                .attr("placeholder", email_ph)
                                .bool_attr("required", true);
                        });
                    });
                    f.el("div", &[("class", "form-grid-2")], |row| {
                        row.el("label", &[], |lbl| {
                            lbl.text(&full_name_lbl);
                            lbl.void_el(
                                "input",
                                &[
                                    ("name", "full_name"),
                                    ("type", "text"),
                                    ("autocomplete", "name"),
                                    ("maxlength", "100"),
                                ],
                            );
                        });
                        row.el("label", &[], |lbl| {
                            lbl.text(&group_lbl);
                            lbl.el("select", &[("name", "email_group_code")], |sel| {
                                sel.el("option", &[("value", "general")], |opt| {
                                    opt.text(&opt_general);
                                });
                                sel.el("option", &[("value", "edergi-releases")], |opt| {
                                    opt.text(&opt_issues);
                                });
                                sel.el("option", &[("value", "exhibition-alerts")], |opt| {
                                    opt.text(&opt_events);
                                });
                            });
                        });
                    });
                    f.el("label", &[("class", "check-label")], |lbl| {
                        lbl.void_with("input", |inp| {
                            inp.attr("type", "checkbox")
                                .attr("name", "kvkk_consent")
                                .attr("value", "true")
                                .bool_attr("required", true);
                        });
                        lbl.el("span", &[], |span| {
                            if ctx.lang.eq_ignore_ascii_case("en") {
                                span.text("I have read the ");
                                span.el(
                                    "a",
                                    &[
                                        ("href", &kvkk_href),
                                        ("target", "_blank"),
                                        ("rel", "noopener"),
                                    ],
                                    |a| {
                                        a.text("KVKK Disclosure Notice");
                                    },
                                );
                                span.text(" and consent to receiving defence briefings under the ");
                                span.el(
                                    "a",
                                    &[
                                        ("href", &etk_href),
                                        ("target", "_blank"),
                                        ("rel", "noopener"),
                                    ],
                                    |a| {
                                        a.text("Commercial Messaging Notice");
                                    },
                                );
                                span.text(".");
                            } else {
                                span.el(
                                    "a",
                                    &[
                                        ("href", &kvkk_href),
                                        ("target", "_blank"),
                                        ("rel", "noopener"),
                                    ],
                                    |a| {
                                        a.text("6698 sayılı KVKK Aydınlatma Metni");
                                    },
                                );
                                span.text("'ni okudum ve ");
                                span.el(
                                    "a",
                                    &[
                                        ("href", &etk_href),
                                        ("target", "_blank"),
                                        ("rel", "noopener"),
                                    ],
                                    |a| {
                                        a.text("6563 sayılı ETK Bilgilendirme Metni");
                                    },
                                );
                                span.text(" kapsamında savunma bülteni e-postaları almayı onaylıyorum.");
                            }
                        });
                    });
                    f.el(
                        "p",
                        &[
                            ("class", "form-feedback"),
                            ("id", "public-newsletter-feedback"),
                            ("role", "status"),
                        ],
                        |_| {},
                    );
                    f.el("button", &[("type", "submit"), ("class", "button primary")], |btn| {
                        btn.text(&submit_lbl);
                    });
                },
            );
        },
    );
}

fn render_cart_drawer(buf: &mut HtmlBuf, ctx: &DialogContext<'_>) {
    let close_aria = close_label(ctx);
    let title = t(ctx, "dialogs.cart.title", "Yayın Sepetim", "Publication Cart");
    let empty_lbl = t(
        ctx,
        "dialogs.cart.empty",
        "Sepetinizde henüz yayın veya abonelik bulunmuyor.",
        "Your cart currently contains no publications or subscriptions.",
    );
    let full_name_lbl = t(ctx, "dialogs.auth.full_name", "Ad soyad", "Full name");
    let email_lbl = t(ctx, "dialogs.auth.corp_email", "Kurumsal e-posta", "Work email");
    let org_lbl = t(
        ctx,
        "dialogs.offer.org_name",
        "Kurum / Şirket adı",
        "Institution / Company name",
    );
    let submit_lbl = t(
        ctx,
        "dialogs.cart.submit",
        "Sepeti öde",
        "Proceed to checkout",
    );
    let subtotal_lbl = t(ctx, "dialogs.cart.subtotal", "Ara toplam", "Subtotal");
    let distance_href = lang_href("/kurumsal/mesafeli-satis-sozlesmesi", ctx.lang);

    buf.el_with(
        "div",
        |attrs| {
            attrs
                .attr("id", "cart-backdrop")
                .attr("class", "cart-backdrop")
                .bool_attr("hidden", true);
        },
        |_| {},
    );
    buf.el_with(
        "aside",
        |attrs| {
            attrs
                .attr("id", "cart-drawer")
                .attr("class", "cart-drawer")
                .attr("role", "dialog")
                .attr("aria-modal", "true")
                .attr("aria-hidden", "true")
                .attr("aria-labelledby", "cart-drawer-title")
                .bool_attr("hidden", true);
        },
        |aside| {
            aside.el("div", &[("class", "cart-drawer-head")], |hd| {
                hd.el("h2", &[("id", "cart-drawer-title")], |h2| {
                    h2.text(&title);
                });
                hd.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "icon-button"),
                        ("data-cart-close", ""),
                        ("aria-label", &close_aria),
                    ],
                    |btn| {
                        btn.raw("&times;");
                    },
                );
            });
            aside.el("div", &[("class", "cart-drawer-body")], |body| {
                body.el(
                    "p",
                    &[("id", "cart-empty-state"), ("class", "empty-state")],
                    |p| {
                        p.text(&empty_lbl);
                    },
                );
                body.el(
                    "div",
                    &[("id", "cart-items-list"), ("class", "cart-items-list")],
                    |_| {},
                );
                body.el_with(
                    "form",
                    |attrs| {
                        attrs
                            .attr("id", "cart-checkout-form")
                            .attr("class", "ui-form")
                            .bool_attr("hidden", true);
                    },
                    |f| {
                        f.el("label", &[], |lbl| {
                            lbl.text(&full_name_lbl);
                            lbl.void_with("input", |inp| {
                                inp.attr("name", "full_name")
                                    .attr("autocomplete", "name")
                                    .bool_attr("required", true);
                            });
                        });
                        f.el("label", &[], |lbl| {
                            lbl.text(&email_lbl);
                            lbl.void_with("input", |inp| {
                                inp.attr("name", "email")
                                    .attr("type", "email")
                                    .attr("autocomplete", "email")
                                    .bool_attr("required", true);
                            });
                        });
                        f.el("label", &[], |lbl| {
                            lbl.text(&org_lbl);
                            lbl.void_el(
                                "input",
                                &[
                                    ("name", "organization_name"),
                                    ("autocomplete", "organization"),
                                ],
                            );
                        });
                        f.el("label", &[("class", "check-label")], |lbl| {
                            lbl.void_with("input", |inp| {
                                inp.attr("type", "checkbox")
                                    .attr("name", "distance_sales_consent")
                                    .attr("value", "true")
                                    .bool_attr("required", true);
                            });
                            lbl.el("span", &[], |span| {
                                if ctx.lang.eq_ignore_ascii_case("en") {
                                    span.text("I accept the ");
                                    span.el(
                                        "a",
                                        &[
                                            ("href", &distance_href),
                                            ("target", "_blank"),
                                            ("rel", "noopener"),
                                        ],
                                        |a| {
                                            a.text("Pre-Contractual Information & Distance Sales Terms");
                                        },
                                    );
                                    span.text(".");
                                } else {
                                    span.el(
                                        "a",
                                        &[
                                            ("href", &distance_href),
                                            ("target", "_blank"),
                                            ("rel", "noopener"),
                                        ],
                                        |a| {
                                            a.text("Ön Bilgilendirme Formu ve Mesafeli Satış Sözleşmesi");
                                        },
                                    );
                                    span.text("'ni okudum, kabul ediyorum.");
                                }
                            });
                        });
                        f.el("p", &[("class", "form-feedback"), ("role", "status")], |_| {});
                        f.el(
                            "button",
                            &[("type", "submit"), ("class", "button primary")],
                            |btn| {
                                btn.text(&submit_lbl);
                            },
                        );
                    },
                );
                body.el(
                    "p",
                    &[
                        ("class", "form-feedback"),
                        ("id", "cart-feedback"),
                        ("role", "status"),
                    ],
                    |_| {},
                );
                body.el_with(
                    "div",
                    |attrs| {
                        attrs
                            .attr("id", "cart-paytr-wrap")
                            .attr("class", "paytr-iframe-wrap")
                            .bool_attr("hidden", true);
                    },
                    |_| {},
                );
            });
            aside.el("div", &[("class", "cart-drawer-footer")], |ft| {
                ft.el("div", &[("class", "cart-subtotal-row")], |row| {
                    row.el("span", &[], |sp| {
                        sp.text(&subtotal_lbl);
                    });
                    row.el("strong", &[("id", "cart-subtotal")], |st| {
                        st.text("₺0");
                    });
                });
            });
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_localized_en_dialogs_without_tr_leaks_and_gates_pdf_iframe() {
        let tr = TranslationCache::new();
        let guest_html = render(&DialogContext {
            lang: "en",
            tr: Some(&tr),
            institutions: &[],
            can_read_pdf: false,
            featured_issue: Some(286),
        });
        assert!(guest_html.contains("id=\"issue-dialog\""));
        assert!(guest_html.contains("id=\"auth-dialog\""));
        assert!(guest_html.contains("id=\"offer-dialog\""));
        assert!(guest_html.contains("id=\"newsletter-dialog\""));
        assert!(guest_html.contains("id=\"public-newsletter-form\""));
        assert!(guest_html.contains("action=\"/api/v1/public/newsletter/subscribe\""));
        assert!(guest_html.contains("name=\"email_group_code\""));
        assert!(guest_html.contains("id=\"cart-backdrop\""));
        assert!(guest_html.contains("id=\"cart-drawer\""));
        assert!(guest_html.contains("id=\"cart-items-list\""));
        assert!(guest_html.contains("id=\"cart-empty-state\""));
        assert!(guest_html.contains("id=\"cart-subtotal\""));
        assert!(guest_html.contains("id=\"cart-checkout-form\""));
        assert!(guest_html.contains("id=\"cart-feedback\""));
        assert!(guest_html.contains("id=\"cart-paytr-wrap\""));
        assert!(guest_html.contains("id=\"cart-paytr-wrap\" class=\"paytr-iframe-wrap\""));
        assert!(!guest_html.contains("paytr-frame-wrap"));
        assert!(guest_html.contains("name=\"kvkk_consent\""));
        assert!(guest_html.contains("name=\"tos_accepted\""));
        assert!(guest_html.contains("id=\"register-tos-accepted\""));
        assert!(guest_html.contains("name=\"tos_version\" value=\"2026-01\""));
        assert!(guest_html.contains("id=\"cookie-consent-bar\""));
        assert!(guest_html.contains("id=\"cookie-dialog\""));
        assert!(guest_html.contains("id=\"gallery-lightbox\""));
        assert!(guest_html.contains("Welcome"));
        assert!(guest_html.contains("Create account"));
        assert!(guest_html.contains("MoND (Ministry of National Defence)"));
        assert!(!guest_html.contains("Hoş geldiniz"));
        assert!(!guest_html.contains("Zorunlu Oturum"));
        assert!(!guest_html.contains("Yayın Sepetim"));
        assert!(!guest_html.contains("Defense"));
        assert!(!guest_html.contains("<iframe id=\"issue-pdf-frame\""));

        let sub_html = render(&DialogContext {
            lang: "en",
            tr: Some(&tr),
            institutions: &[],
            can_read_pdf: true,
            featured_issue: Some(286),
        });
        assert!(sub_html.contains("<iframe id=\"issue-pdf-frame\""));
        assert!(sub_html.contains("/api/v1/edergi/286/download-pdf?publication_type=msi#toolbar=0"));
    }
}
