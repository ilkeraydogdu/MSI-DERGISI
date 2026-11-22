//! MSI Dergisi Editorial Studio HTML shell renderer (100% Pure Rust WASM host page).

use super::html_dsl::HtmlBuf;
use crate::engine::security_engine::escape_html;

const EDITORIAL_THEME_PREHYDRATE_JS: &str =
    crate::engine::design::theme_boot::UNIFIED_THEME_PREHYDRATE_JS;

/// Renders the `/podmin/editor` page loading the MSI Editorial Studio WASM application.
/// Uses the request CSP nonce, syncs with `msi_admin_theme`, and avoids any external CDN assets.
pub fn render_editorial_studio_shell(nonce: Option<&str>, admin_path: &str) -> String {
    let clean_nonce = nonce.filter(|s| !s.trim().is_empty());
    let clean_admin = if admin_path.trim().is_empty() {
        "/podmin"
    } else {
        admin_path.trim()
    };

    let mut d = HtmlBuf::with_capacity(4096);
    d.raw("<!DOCTYPE html>");
    d.el("html", &[("lang", "tr"), ("class", "dark-ready")], |h| {
        h.el("head", &[], |head| {
            head.void_el("meta", &[("charset", "utf-8")]);
            head.void_el(
                "meta",
                &[("name", "viewport"), ("content", "width=device-width, initial-scale=1.0")],
            );
            head.void_el(
                "meta",
                &[("name", "robots"), ("content", "noindex, nofollow, noarchive")],
            );
            head.el("title", &[], |t| {
                t.text("MSI Dergisi · Editör Stüdyosu");
            });
            head.void_el("link", &[("rel", "icon"), ("href", "/favicon.ico")]);
            head.void_el(
                "link",
                &[("rel", "icon"), ("type", "image/svg+xml"), ("href", "/favicon.svg")],
            );
            head.void_el(
                "link",
                &[
                    ("rel", "icon"),
                    ("type", "image/png"),
                    ("sizes", "32x32"),
                    ("href", "/images/favicon-32x32.png"),
                ],
            );
            head.script_inline(EDITORIAL_THEME_PREHYDRATE_JS, clean_nonce, None);
            head.link_css("/css/admin/tokens.css?v=35");
            head.link_css("/css/admin/editorial-studio.css?v=35");
        });
        h.el(
            "body",
            &[
                ("class", "msi-editorial-host"),
                ("data-admin-path", clean_admin),
            ],
            |b| {
                b.el("div", &[("id", "root")], |root| {
                    root.el("div", &[("class", "msi-studio-boot")], |boot| {
                        boot.el("div", &[("class", "msi-studio-boot-card")], |card| {
                            card.void_el(
                                "img",
                                &[
                                    ("src", "/images/logo.svg"),
                                    ("alt", "MSI Dergisi"),
                                    ("width", "44"),
                                    ("height", "44"),
                                ],
                            );
                            card.el("h1", &[], |h1| {
                                h1.text("MSI Dergisi · Editör Stüdyosu");
                            });
                            card.el("p", &[], |p| {
                                p.text("Saf Rust WebAssembly editör motoru yükleniyor...");
                            });
                        });
                    });
                });
                let boot_js = format!(
                    "import init from '{admin}/editor/studio.js?v=35'; init({{ module_or_path: '{admin}/editor/studio.wasm?v=35' }}).catch(function(e){{ var r=document.getElementById('root'); if(r){{ r.innerHTML='<div class=\"msi-studio-boot\"><div class=\"msi-studio-boot-card\"><h1>Editör Stüdyosu Başlatılamadı</h1><p>Lütfen sayfayı yenileyin veya Komuta Merkezi üzerinden tekrar açın.</p><div class=\"msi-studio-boot-actions\"><a href=\"{admin}\" class=\"msi-studio-btn-primary\">Komuta Merkezine Dön</a></div></div></div>'; }} console.error('WASM init error:', e); }});",
                    admin = escape_html(clean_admin)
                );
                b.script_inline(&boot_js, clean_nonce, Some("module"));
            },
        );
    });
    d.into_string()
}

/// Renders an MSI-styled 403 Forbidden page for signed-in operators lacking `contents:write`.
pub fn render_editorial_studio_forbidden(
    nonce: Option<&str>,
    admin_path: &str,
    user_email: &str,
    user_role: &str,
) -> String {
    let clean_nonce = nonce.filter(|s| !s.trim().is_empty());
    let clean_admin = if admin_path.trim().is_empty() {
        "/podmin"
    } else {
        admin_path.trim()
    };
    let email_safe = escape_html(user_email);
    let role_safe = escape_html(user_role);

    let mut d = HtmlBuf::with_capacity(2048);
    d.raw("<!DOCTYPE html>");
    d.el("html", &[("lang", "tr"), ("class", "dark-ready")], |h| {
        h.el("head", &[], |head| {
            head.void_el("meta", &[("charset", "utf-8")]);
            head.void_el(
                "meta",
                &[("name", "viewport"), ("content", "width=device-width, initial-scale=1.0")],
            );
            head.void_el(
                "meta",
                &[("name", "robots"), ("content", "noindex, nofollow, noarchive")],
            );
            head.el("title", &[], |t| {
                t.text("403 · Editör Stüdyosu Erişim Yetkisi Gerekli");
            });
            head.script_inline(EDITORIAL_THEME_PREHYDRATE_JS, clean_nonce, None);
            head.link_css("/css/admin/tokens.css?v=35");
            head.link_css("/css/admin/editorial-studio.css?v=35");
        });
        h.el("body", &[("class", "msi-editorial-host")], |b| {
            b.el("main", &[("class", "msi-studio-boot")], |m| {
                m.el(
                    "section",
                    &[("class", "msi-studio-boot-card msi-studio-forbidden")],
                    |card| {
                        card.el("span", &[("class", "msi-studio-badge-err")], |sp| {
                            sp.text("403 · YETKİSİZ ERİŞİM");
                        });
                        card.el("h1", &[], |h1| {
                            h1.text("Editör Stüdyosu Erişimi Kısıtlı");
                        });
                        card.el("p", &[], |p| {
                            p.raw(&format!(
                                "Oturumunuz (<strong>{}</strong> · rol: <code>{}</code>) <code>contents:write</code> editör yetkisine sahip olmadığı için Editör Stüdyosu açılamadı.",
                                email_safe, role_safe
                            ));
                        });
                        card.el("div", &[("class", "msi-studio-boot-actions")], |act| {
                            act.el(
                                "a",
                                &[("href", clean_admin), ("class", "msi-studio-btn-primary")],
                                |a| {
                                    a.text("Komuta Merkezine Dön");
                                },
                            );
                        });
                    },
                );
            });
        });
    });
    d.into_string()
}
