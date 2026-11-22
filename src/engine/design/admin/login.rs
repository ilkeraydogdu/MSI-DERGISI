//! Full-bleed interactive WebGL defence gallery & floating liquid-glass login gateway
//! (`/podmin` & `/{ADMIN_PATH_SECRET}`) with autonomous OS / local-clock theming.

use super::login_stage::{render_stage, STAGE_SLIDES};
use super::ui::{form_group, icon, icon_with_attrs};
use super::{clean_segment, tr_admin};
use crate::engine::design::html_dsl::HtmlBuf;
use crate::engine::design::theme_boot::AUTO_ONLY_THEME_PREHYDRATE_JS;
use crate::engine::translation_engine::TranslationCache;

pub fn render_login(nonce: Option<&str>, clean_admin_path: &str, tr: &TranslationCache, lang: &str) -> String {
    let seg = clean_segment(clean_admin_path);
    let admin_root = format!("/{}", seg);
    let clean_nonce = nonce.map(str::trim).filter(|n| !n.is_empty());
    let home_href = if lang.eq_ignore_ascii_case("en") { "/en" } else { "/" };

    let page_title = tr_admin(
        tr,
        lang,
        "admin.login.clean_title",
        "MSI Dergisi · Giriş",
        "MSI Review · Sign In",
    );
    let back_link = tr_admin(tr, lang, "admin.login.back_to_site", "Yayına Dön", "Back to Site");
    let heading = tr_admin(tr, lang, "admin.login.clean_heading", "Giriş Yap", "Sign In");
    let first_slide = &STAGE_SLIDES[0];
    let preload_srcset = format!("{} 1280w, {} 2400w", first_slide.src_sm, first_slide.src);

    let mut buf = HtmlBuf::with_capacity(12288);
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
            head.void_el(
                "link",
                &[
                    ("rel", "preload"),
                    ("as", "image"),
                    ("href", first_slide.src),
                    ("imagesrcset", &preload_srcset),
                    ("imagesizes", "100vw"),
                    ("type", "image/webp"),
                ],
            );
            head.link_css("/css/design-tokens.css")
                .link_css("/css/admin/tokens.css?v=40")
                .link_css("/css/admin/components.css?v=40")
                .link_css("/css/admin/login.css?v=40");
            head.script_inline(AUTO_ONLY_THEME_PREHYDRATE_JS, clean_nonce, None);
        });

        html.el(
            "body",
            &[("class", "login-body"), ("data-admin-path", &admin_root), ("data-admin-lang", lang)],
            |body| {
                body.el("main", &[("class", "login-root")], |root| {
                    render_stage(root, tr, lang, home_href, &back_link);

                    root.el("aside", &[("class", "login-dock")], |dock| {
                        dock.el("div", &[("class", "login-card"), ("id", "login-card"), ("data-glow-card", "")], |card| {
                            card.el("div", &[("class", "login-brand")], |br| {
                                br.void_el("img", &[
                                    ("src", "/images/logo-white.svg"),
                                    ("alt", "MSI Dergisi"),
                                    ("class", "login-brand-logo logo-for-dark"),
                                    ("width", "108"),
                                    ("height", "40"),
                                    ("decoding", "async"),
                                ]);
                                br.void_el("img", &[
                                    ("src", "/images/logo.svg"),
                                    ("alt", "MSI Dergisi"),
                                    ("class", "login-brand-logo logo-for-light"),
                                    ("width", "108"),
                                    ("height", "40"),
                                    ("decoding", "async"),
                                ]);
                                br.el("h1", &[("class", "login-title")], |h| {
                                    h.text(&heading);
                                });
                            });

                            card.el("div", &[("class", "login-error"), ("id", "login-error"), ("role", "alert")], |_| {});
                            card.el("div", &[("class", "login-success"), ("id", "login-success"), ("role", "status")], |_| {});

                            render_step_login(card, tr, lang);
                            render_step_totp(card, tr, lang);
                            render_step_forgot(card, tr, lang);

                            card.el("footer", &[("class", "login-card-foot")], |f| {
                                f.el("span", &[("class", "login-foot-copy")], |c| {
                                    c.text("© 2026 MSI Dergisi");
                                });
                                f.el(
                                    "a",
                                    &[
                                        ("href", "https://pofudukdijital.com/"),
                                        ("target", "_blank"),
                                        ("rel", "noopener noreferrer"),
                                        ("class", "login-agency-link"),
                                        ("aria-label", "Pofuduk Dijital"),
                                        ("title", "Pofuduk Dijital"),
                                    ],
                                    |a| {
                                        a.void_el("img", &[
                                            ("src", "/images/admin/pofuduk-logo-dark.svg"),
                                            ("alt", "Pofuduk Dijital"),
                                            ("class", "login-agency-logo logo-for-dark"),
                                            ("width", "70"),
                                            ("height", "24"),
                                            ("decoding", "async"),
                                        ]);
                                        a.void_el("img", &[
                                            ("src", "/images/admin/pofuduk-logo.svg"),
                                            ("alt", "Pofuduk Dijital"),
                                            ("class", "login-agency-logo logo-for-light"),
                                            ("width", "70"),
                                            ("height", "24"),
                                            ("decoding", "async"),
                                        ]);
                                    },
                                );
                            });
                        });
                    });
                });

                body.script_src("/js/vendor/gsap.min.js", clean_nonce, true);
                body.script_src("/js/admin/core.js?v=40", clean_nonce, true);
                body.script_src("/js/admin/login-stage.js?v=40", clean_nonce, true);
                body.script_src("/js/admin/login.js?v=40", clean_nonce, true);
            },
        );
    });
    buf.into_string()
}

fn render_step_login(card: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let lbl_email = tr_admin(tr, lang, "admin.login.gw_email", "E-posta", "Email");
    let ph_email = tr_admin(tr, lang, "admin.login.email_placeholder", "operator@msidergisi.com", "operator@msidergisi.com");
    let lbl_password = tr_admin(tr, lang, "admin.login.gw_password", "Parola", "Password");
    let forgot_link = tr_admin(tr, lang, "admin.login.gw_forgot", "Şifremi unuttum", "Forgot password");
    let pw_toggle_aria = tr_admin(tr, lang, "admin.login.pw_toggle_aria", "Parolayı göster/gizle", "Show/hide password");
    let submit_btn = tr_admin(tr, lang, "admin.login.clean_submit", "Giriş Yap", "Sign In");

    card.el_with(
        "form",
        |a| {
            a.attr("class", "login-form")
                .attr("id", "login-form")
                .attr("autocomplete", "on")
                .bool_attr("novalidate", true);
        },
        |f| {
            form_group(f, Some("login-email"), &lbl_email, |g| {
                g.void_with("input", |a| {
                    a.attr("type", "email")
                        .attr("id", "login-email")
                        .attr("class", "form-control login-field")
                        .attr("autocomplete", "username")
                        .attr("placeholder", &ph_email)
                        .bool_attr("required", true)
                        .bool_attr("autofocus", true);
                });
            });
            f.el("div", &[("class", "form-group")], |g| {
                g.el("div", &[("class", "form-label-row")], |row| {
                    row.el("label", &[("class", "form-label"), ("for", "login-password")], |l| {
                        l.text(&lbl_password);
                    });
                    row.el("button", &[("type", "button"), ("class", "text-link"), ("id", "btn-show-forgot")], |b| {
                        b.text(&forgot_link);
                    });
                });
                g.el("div", &[("class", "password-wrap")], |pw| {
                    pw.void_with("input", |a| {
                        a.attr("type", "password")
                            .attr("id", "login-password")
                            .attr("class", "form-control login-field")
                            .attr("autocomplete", "current-password")
                            .attr("placeholder", "••••••••")
                            .bool_attr("required", true);
                    });
                    pw.el(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "pw-toggle-btn"),
                            ("id", "btn-toggle-pw"),
                            ("aria-label", &pw_toggle_aria),
                            ("aria-pressed", "false"),
                        ],
                        |b| {
                            icon_with_attrs(b, "fa-eye", "", Some("pw-toggle-icon"));
                        },
                    );
                });
            });
            f.el("button", &[("type", "submit"), ("class", "btn-cmd login-submit login-cta"), ("id", "login-submit-btn")], |b| {
                b.el("span", &[("class", "cta-label")], |s| {
                    s.text(&submit_btn);
                });
                icon_with_attrs(b, "arrow-right", "cta-arrow", None);
            });
        },
    );
}

fn render_step_totp(card: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let step_hint = tr_admin(
        tr,
        lang,
        "admin.login.gw_totp_hint",
        "6 haneli doğrulama kodunu veya 8 karakterli yedek kodunuzu girin.",
        "Enter your 6-digit verification code or 8-character backup code.",
    );
    let lbl_code = tr_admin(tr, lang, "admin.login.gw_totp_label", "Doğrulama Kodu", "Verification Code");
    let verify_btn = tr_admin(tr, lang, "admin.login.gw_totp_submit", "Doğrula", "Verify");
    let back_btn = tr_admin(tr, lang, "admin.login.gw_back", "Geri Dön", "Back");

    card.el_with(
        "form",
        |a| {
            a.attr("class", "login-form hidden")
                .attr("id", "totp-login-form")
                .bool_attr("novalidate", true);
        },
        |f| {
            f.el("p", &[("class", "step-hint"), ("id", "totp-step-hint")], |p| {
                p.text(&step_hint);
            });
            form_group(f, Some("totp-code-input"), &lbl_code, |g| {
                g.void_with("input", |a| {
                    a.attr("type", "text")
                        .attr("id", "totp-code-input")
                        .attr("class", "form-control login-field text-mono login-otp-field")
                        .attr("minlength", "6")
                        .attr("maxlength", "20")
                        .attr("autocomplete", "one-time-code")
                        .attr("aria-describedby", "totp-step-hint")
                        .attr("placeholder", "000 000")
                        .bool_attr("required", true);
                });
            });
            f.el("button", &[("type", "submit"), ("class", "btn-cmd login-submit login-cta"), ("id", "totp-submit-btn")], |b| {
                b.el("span", &[("class", "cta-label")], |s| {
                    s.text(&verify_btn);
                });
                icon_with_attrs(b, "arrow-right", "cta-arrow", None);
            });
            f.el("button", &[("type", "button"), ("class", "btn-cmd login-secondary-btn"), ("id", "btn-back-from-totp")], |b| {
                icon(b, "fa-arrow-left");
                b.raw(" ");
                b.text(&back_btn);
            });
        },
    );
}

fn render_step_forgot(card: &mut HtmlBuf, tr: &TranslationCache, lang: &str) {
    let step_hint = tr_admin(
        tr,
        lang,
        "admin.login.gw_recovery_hint",
        "Şifre sıfırlama bağlantısı için e-posta adresinizi girin.",
        "Enter your email address to receive a password reset link.",
    );
    let lbl_email = tr_admin(tr, lang, "admin.login.gw_email", "E-posta", "Email");
    let ph_email = tr_admin(tr, lang, "admin.login.email_placeholder", "operator@msidergisi.com", "operator@msidergisi.com");
    let submit_btn = tr_admin(tr, lang, "admin.login.gw_recovery_submit", "Bağlantı Gönder", "Send Reset Link");
    let back_btn = tr_admin(tr, lang, "admin.login.gw_back", "Geri Dön", "Back");

    card.el_with(
        "form",
        |a| {
            a.attr("class", "login-form hidden")
                .attr("id", "forgot-password-form")
                .bool_attr("novalidate", true);
        },
        |f| {
            f.el("p", &[("class", "step-hint")], |p| {
                p.text(&step_hint);
            });
            form_group(f, Some("forgot-email"), &lbl_email, |g| {
                g.void_with("input", |a| {
                    a.attr("type", "email")
                        .attr("id", "forgot-email")
                        .attr("class", "form-control login-field")
                        .attr("autocomplete", "email")
                        .attr("placeholder", &ph_email)
                        .bool_attr("required", true);
                });
            });
            f.el("button", &[("type", "submit"), ("class", "btn-cmd login-submit login-cta"), ("id", "forgot-submit-btn")], |b| {
                b.el("span", &[("class", "cta-label")], |s| {
                    s.text(&submit_btn);
                });
                icon_with_attrs(b, "arrow-right", "cta-arrow", None);
            });
            f.el("button", &[("type", "button"), ("class", "btn-cmd login-secondary-btn"), ("id", "btn-back-from-forgot")], |b| {
                icon(b, "fa-arrow-left");
                b.raw(" ");
                b.text(&back_btn);
            });
        },
    );
}
