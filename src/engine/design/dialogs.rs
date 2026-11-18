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
    render_cookie_consent_bar(buf, ctx);
    render_cookie_dialog(buf, ctx);
    render_gallery_lightbox(buf, ctx);
}

fn append_antispam_fields(f: &mut HtmlBuf) {
    f.void_with("input", |inp| {
        inp.attr("type", "text")
            .attr("name", "website_url")
            .attr("tabindex", "-1")
            .attr("autocomplete", "off")
            .attr(
                "style",
                "position:absolute;left:-9999px;opacity:0;height:0;width:0;",
            )
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
    let tab_pdf = t(ctx, "dialogs.issue.tab_pdf", "PDF Okuyucu", "PDF Reader");
    let prev_aria = t(ctx, "dialogs.issue.prev_spread", "Önceki bölüm", "Previous section");
    let next_aria = t(ctx, "dialogs.issue.next_spread", "Sonraki bölüm", "Next section");
    let open_pdf = t(ctx, "dialogs.issue.open_pdf", "PDF'yi aç ↗", "Open PDF ↗");
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
    let pdf_src = format!("/api/v1/edergi/{}/download-pdf#toolbar=0", active_issue);

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
                r.el("div", &[], |col| {
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
                            panel.el("p", &[("class", "reader-hint")], |p| {
                                p.text(&hint);
                            });
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
                            lbl.text(&corp_email_lbl);
                            lbl.void_with("input", |inp| {
                                inp.attr("name", "email")
                                    .attr("type", "email")
                                    .attr("autocomplete", "email")
                                    .bool_attr("required", true);
                            });
                        });
                    });
                    f.el_with(
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
                    f.el("div", &[("class", "form-grid-2")], |row| {
                        row.el("label", &[], |lbl| {
                            lbl.text(&reg_pw_lbl);
                            lbl.void_with("input", |inp| {
                                inp.attr("name", "password")
                                    .attr("type", "password")
                                    .attr("autocomplete", "new-password")
                                    .attr("minlength", "8")
                                    .bool_attr("required", true);
                            });
                        });
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
                    });
                    f.el("div", &[("class", "form-grid-2")], |row| {
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
                    f.el("fieldset", &[("class", "ssb-fieldset")], |fs| {
                        fs.el("legend", &[], |lg| {
                            lg.text(&ssb_legend);
                        });
                        fs.el("div", &[("id", "register-ssb-tags"), ("class", "ssb-tag-picks")], |_| {});
                    });
                    f.el("label", &[], |lbl| {
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
                    f.el("label", &[("class", "check-label")], |lbl| {
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
                                span.text(" and ");
                                span.el(
                                    "a",
                                    &[("href", &terms_href), ("target", "_blank"), ("rel", "noopener")],
                                    |a| {
                                        a.text("Terms of Use");
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
                                span.text("'ni ve ");
                                span.el(
                                    "a",
                                    &[("href", &terms_href), ("target", "_blank"), ("rel", "noopener")],
                                    |a| {
                                        a.text("Üyelik / Kullanım Koşulları");
                                    },
                                );
                                span.text("'nı okudum; kişisel verilerimin işlenmesine ilişkin aydınlatıldım.");
                            }
                        });
                    });
                    f.el("label", &[("class", "check-label")], |lbl| {
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
        assert!(guest_html.contains("id=\"cookie-consent-bar\""));
        assert!(guest_html.contains("id=\"cookie-dialog\""));
        assert!(guest_html.contains("id=\"gallery-lightbox\""));
        assert!(guest_html.contains("Welcome"));
        assert!(guest_html.contains("Create account"));
        assert!(guest_html.contains("MoND (Ministry of National Defence)"));
        assert!(!guest_html.contains("Hoş geldiniz"));
        assert!(!guest_html.contains("Zorunlu Oturum"));
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
        assert!(sub_html.contains("/api/v1/edergi/286/download-pdf#toolbar=0"));
    }
}
