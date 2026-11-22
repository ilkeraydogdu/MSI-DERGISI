//! Modern 2026 Editorial Split-Screen Login Gateway (`/podmin` & `/{ADMIN_PATH_SECRET}`).
//! Left half: sculpted inset full-height cinema window with vertical fighter jet photography,
//! photographic ambient backlight bleed, optical glass rim, and fluid GSAP atmospheric motion.
//! Right half: clean liquid-glass sign-in surface with autonomous OS / local-clock theming.

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
    let hero_aria = tr_admin(tr, lang, "admin.login.clean_visual_aria", "Savaş uçağı görseli", "Fighter jet visual");
    let hero_alt = tr_admin(
        tr,
        lang,
        "admin.login.clean_visual_alt",
        "Bulutların üzerinde tırmanan çift motorlu savaş uçağı",
        "Twin-engine stealth fighter jet climbing above twilight clouds",
    );

    let mut buf = HtmlBuf::with_capacity(8192);
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
                    ("href", "/images/admin/podmin-fighter-vertical.webp"),
                    ("type", "image/webp"),
                ],
            );
            head.link_css("/css/admin/tokens.css?v=35")
                .link_css("/css/admin/components.css?v=35")
                .link_css("/css/admin/login.css?v=35");
            head.script_inline(AUTO_ONLY_THEME_PREHYDRATE_JS, clean_nonce, None);
        });

        html.el("body", &[("data-admin-path", &admin_root), ("data-admin-lang", lang)], |body| {
            body.el("div", &[("class", "login-canvas-bg"), ("aria-hidden", "true")], |bg| {
                bg.el("div", &[("class", "canvas-dot-mesh")], |_| {});
                bg.el("div", &[("class", "canvas-keylight")], |_| {});
            });

            body.el("main", &[("class", "login-split")], |split| {
                // Left half: Sculpted full-height inset cinema window with real ambient color bleed
                split.el("aside", &[("class", "login-showcase"), ("id", "jet-stage"), ("aria-label", &hero_aria)], |sc| {
                    sc.el("div", &[("class", "jet-ambient-bleed"), ("aria-hidden", "true")], |bleed| {
                        bleed.void_el(
                            "img",
                            &[
                                ("src", "/images/admin/podmin-fighter-vertical.webp"),
                                ("alt", ""),
                                ("class", "jet-bleed-img"),
                                ("width", "896"),
                                ("height", "1200"),
                                ("decoding", "async"),
                            ],
                        );
                    });

                    sc.el("div", &[("class", "jet-cinema-frame"), ("id", "jet-cinema-frame")], |frame| {
                        frame.el("div", &[("class", "jet-media-track"), ("id", "jet-media-track")], |tr_el| {
                            tr_el.void_el(
                                "img",
                                &[
                                    ("src", "/images/admin/podmin-fighter-vertical.webp"),
                                    ("alt", &hero_alt),
                                    ("class", "jet-visual-img"),
                                    ("id", "jet-visual-img"),
                                    ("width", "896"),
                                    ("height", "1200"),
                                    ("decoding", "async"),
                                ],
                            );
                        });
                        frame.el("div", &[("class", "jet-atmosphere-sheen"), ("aria-hidden", "true")], |_| {});
                        frame.el("div", &[("class", "jet-glass-glare"), ("id", "jet-glass-glare"), ("aria-hidden", "true")], |_| {});
                        frame.el("div", &[("class", "jet-cinema-rim"), ("aria-hidden", "true")], |_| {});
                        frame.el("div", &[("class", "jet-reveal-veil"), ("id", "jet-reveal-veil"), ("aria-hidden", "true")], |_| {});
                    });
                });

                // Right half: Spacious modern studio canvas + sculpted liquid-glass sign-in surface
                split.el("section", &[("class", "login-panel")], |panel| {
                    panel.el("header", &[("class", "login-topbar")], |top| {
                        top.el("a", &[("href", home_href), ("class", "login-back-link")], |a| {
                            icon(a, "fa-arrow-left");
                            a.raw(" ");
                            a.text(&back_link);
                        });
                    });

                    panel.el("div", &[("class", "login-center")], |center| {
                        center.el("div", &[("class", "login-card"), ("id", "login-card"), ("data-glow-card", "")], |card| {
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
                        });
                    });

                    panel.el("footer", &[("class", "login-bottom")], |f| {
                        f.text("© 2026 MSI Dergisi");
                    });
                });
            });

            body.script_src("/js/vendor/gsap.min.js", clean_nonce, true);
            body.script_src("/js/admin/core.js?v=35", clean_nonce, true);
            body.script_src("/js/admin/login.js?v=35", clean_nonce, true);
        });
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
