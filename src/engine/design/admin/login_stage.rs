//! Interactive full-bleed defence photography stage for `/podmin`.
//! Renders SSR slide metadata, crisp initial fallback frame, editorial serif caption,
//! clickable 6-slide progress rail, and accessible prev/pause/next controls.

use std::fmt::Write as _;

use super::tr_admin;
use super::ui::icon;
use crate::engine::design::html_dsl::HtmlBuf;
use crate::engine::translation_engine::TranslationCache;

/// Single curated defence photography entry on the login stage.
#[derive(Clone, Copy)]
pub struct StageSlide {
    pub id: &'static str,
    pub src: &'static str,
    pub src_sm: &'static str,
    pub width: u32,
    pub height: u32,
    pub focus_x: f32,
    pub focus_y: f32,
    pub domain_key: &'static str,
    pub domain_tr: &'static str,
    pub domain_en: &'static str,
    pub title: &'static str,
    pub alt_tr: &'static str,
    pub alt_en: &'static str,
}

/// Ordered stage gallery (slide 01 F-4E for ultra-fast 73 KB first paint).
pub const STAGE_SLIDES: &[StageSlide] = &[
    StageSlide {
        id: "01-f4e",
        src: "/images/admin/stage/01-f4e.webp",
        src_sm: "/images/admin/stage/01-f4e-1280.webp",
        width: 2400,
        height: 1600,
        focus_x: 0.50,
        focus_y: 0.45,
        domain_key: "admin.login.stage_domain_air",
        domain_tr: "HAVA",
        domain_en: "AIR",
        title: "F-4E 2020 Terminator",
        alt_tr: "Türk Hava Kuvvetleri F-4E 2020 Terminator savaş uçağı",
        alt_en: "Turkish Air Force F-4E 2020 Terminator fighter jet",
    },
    StageSlide {
        id: "02-kizilelma",
        src: "/images/admin/stage/02-kizilelma.webp",
        src_sm: "/images/admin/stage/02-kizilelma-1280.webp",
        width: 1920,
        height: 1280,
        focus_x: 0.46,
        focus_y: 0.58,
        domain_key: "admin.login.stage_domain_air",
        domain_tr: "HAVA",
        domain_en: "AIR",
        title: "Bayraktar KIZILELMA",
        alt_tr: "Bayraktar KIZILELMA insansız savaş uçağı uçuş testi",
        alt_en: "Bayraktar KIZILELMA unmanned combat aircraft flight test",
    },
    StageSlide {
        id: "03-istif",
        src: "/images/admin/stage/03-istif.webp",
        src_sm: "/images/admin/stage/03-istif-1280.webp",
        width: 2400,
        height: 1600,
        focus_x: 0.42,
        focus_y: 0.56,
        domain_key: "admin.login.stage_domain_sea",
        domain_tr: "DENİZ",
        domain_en: "SEA",
        title: "TCG İSTANBUL (F-515)",
        alt_tr: "İstif sınıfı TCG İSTANBUL fırkateyni açık deniz seyrinde",
        alt_en: "Istif-class frigate TCG ISTANBUL underway at sea",
    },
    StageSlide {
        id: "04-tb2t-ai",
        src: "/images/admin/stage/04-tb2t-ai.webp",
        src_sm: "/images/admin/stage/04-tb2t-ai-1280.webp",
        width: 2400,
        height: 1600,
        focus_x: 0.50,
        focus_y: 0.62,
        domain_key: "admin.login.stage_domain_air",
        domain_tr: "HAVA",
        domain_en: "AIR",
        title: "Bayraktar TB2T-AI",
        alt_tr: "Bayraktar TB2T-AI insansız hava aracı uçuşta",
        alt_en: "Bayraktar TB2T-AI unmanned aerial vehicle in flight",
    },
    StageSlide {
        id: "05-mavi-vatan",
        src: "/images/admin/stage/05-mavi-vatan.webp",
        src_sm: "/images/admin/stage/05-mavi-vatan-1280.webp",
        width: 2400,
        height: 1600,
        focus_x: 0.55,
        focus_y: 0.50,
        domain_key: "admin.login.stage_domain_sea",
        domain_tr: "DENİZ",
        domain_en: "SEA",
        title: "Mavi Vatan Filo Seyri",
        alt_tr: "Türk Deniz Kuvvetleri unsurları deniz seyrinde",
        alt_en: "Turkish Naval Forces task group underway at sea",
    },
    StageSlide {
        id: "06-fnss",
        src: "/images/admin/stage/06-fnss.webp",
        src_sm: "/images/admin/stage/06-fnss-1280.webp",
        width: 2400,
        height: 1600,
        focus_x: 0.45,
        focus_y: 0.56,
        domain_key: "admin.login.stage_domain_land",
        domain_tr: "KARA",
        domain_en: "LAND",
        title: "FNSS Kara Sistemleri",
        alt_tr: "FNSS zırhlı muharebe aracı arazi testinde",
        alt_en: "FNSS armoured combat vehicle during field trials",
    },
];

fn push_json_escaped(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str(r#"\""#),
            '\\' => out.push_str(r#"\\"#),
            '\n' => out.push_str(r#"\n"#),
            '\r' => out.push_str(r#"\r"#),
            '\t' => out.push_str(r#"\t"#),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn serialize_slides_json(tr: &TranslationCache, lang: &str) -> String {
    let mut out = String::with_capacity(2048);
    out.push('[');
    for (idx, slide) in STAGE_SLIDES.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        let domain = tr_admin(tr, lang, slide.domain_key, slide.domain_tr, slide.domain_en);
        let alt = if lang == "en" { slide.alt_en } else { slide.alt_tr };

        out.push_str(r#"{"id":"#);
        push_json_escaped(&mut out, slide.id);
        out.push_str(r#","src":"#);
        push_json_escaped(&mut out, slide.src);
        out.push_str(r#","srcSm":"#);
        push_json_escaped(&mut out, slide.src_sm);
        let _ = write!(
            out,
            r#","w":{},"h":{},"fx":{:.2},"fy":{:.2},"domain":"#,
            slide.width, slide.height, slide.focus_x, slide.focus_y
        );
        push_json_escaped(&mut out, &domain);
        out.push_str(r#","title":"#);
        push_json_escaped(&mut out, slide.title);
        out.push_str(r#","alt":"#);
        push_json_escaped(&mut out, alt);
        out.push('}');
    }
    out.push(']');
    out
}

/// Renders the interactive full-bleed stage backdrop, captions, and controls.
pub fn render_stage(
    buf: &mut HtmlBuf,
    tr: &TranslationCache,
    lang: &str,
    home_href: &str,
    back_link: &str,
) {
    let first = &STAGE_SLIDES[0];
    let total = STAGE_SLIDES.len();
    let total_str = format!("{total:02}");
    let count_str = total.to_string();
    let first_domain = tr_admin(tr, lang, first.domain_key, first.domain_tr, first.domain_en);
    let first_alt = if lang == "en" { first.alt_en } else { first.alt_tr };
    let first_srcset = format!("{} 1280w, {} 2400w", first.src_sm, first.src);

    let stage_aria = tr_admin(tr, lang, "admin.login.stage_aria", "Savunma fotoğraf galerisi", "Defence photography stage");
    let controls_aria = tr_admin(tr, lang, "admin.login.stage_controls", "Galeri kontrolleri", "Gallery controls");
    let rail_aria = tr_admin(tr, lang, "admin.login.stage_rail", "Görsel seçimi", "Slide selector");
    let prev_aria = tr_admin(tr, lang, "admin.login.stage_prev", "Önceki görsel", "Previous slide");
    let toggle_aria = tr_admin(tr, lang, "admin.login.stage_toggle", "Oynat / Duraklat", "Play / Pause");
    let next_aria = tr_admin(tr, lang, "admin.login.stage_next", "Sonraki görsel", "Next slide");
    let slides_json = serialize_slides_json(tr, lang);

    buf.el(
        "section",
        &[
            ("class", "stage"),
            ("id", "login-stage"),
            ("data-active-index", "0"),
            ("data-count", &count_str),
            ("aria-label", &stage_aria),
        ],
        |st| {
            st.void_el(
                "img",
                &[
                    ("class", "stage-fallback"),
                    ("id", "stage-fallback-img"),
                    ("src", first.src),
                    ("srcset", &first_srcset),
                    ("sizes", "100vw"),
                    ("width", "2400"),
                    ("height", "1600"),
                    ("alt", first_alt),
                    ("decoding", "async"),
                    ("fetchpriority", "high"),
                ],
            );
            st.el("canvas", &[("class", "stage-canvas"), ("id", "stage-canvas"), ("aria-hidden", "true")], |_| {});
            st.el("div", &[("class", "stage-scrim"), ("aria-hidden", "true")], |_| {});
            st.el("div", &[("class", "stage-grain"), ("aria-hidden", "true")], |_| {});
            st.el("div", &[("class", "stage-veil"), ("id", "stage-veil"), ("aria-hidden", "true")], |_| {});
            st.el("div", &[("class", "stage-cursor"), ("id", "stage-cursor"), ("aria-hidden", "true")], |_| {});
            st.el(
                "div",
                &[
                    ("id", "stage-slides-data"),
                    ("class", "stage-slides-data"),
                    ("data-slides", &slides_json),
                    ("aria-hidden", "true"),
                ],
                |_| {},
            );

            st.el("div", &[("class", "stage-top")], |top| {
                top.el("a", &[("href", home_href), ("class", "stage-back"), ("id", "stage-back-link")], |a| {
                    icon(a, "arrow-left");
                    a.el("span", &[], |s| {
                        s.text(back_link);
                    });
                });
            });

            st.el("div", &[("class", "stage-hud")], |hud| {
                hud.el("div", &[("class", "stage-caption"), ("id", "stage-caption"), ("aria-live", "polite")], |cap| {
                    cap.el("div", &[("class", "cap-meta")], |meta| {
                        meta.el("span", &[("class", "cap-index"), ("id", "cap-index")], |s| { s.text("01"); });
                        meta.el("span", &[("class", "cap-sep"), ("aria-hidden", "true")], |s| { s.text("/"); });
                        meta.el("span", &[("class", "cap-total"), ("id", "cap-total")], |s| { s.text(&total_str); });
                        meta.el("span", &[("class", "cap-domain"), ("id", "cap-domain")], |s| { s.text(&first_domain); });
                    });
                    cap.el("h2", &[("class", "cap-title"), ("id", "cap-title")], |h| { h.text(first.title); });
                });

                hud.el("div", &[("class", "stage-controls"), ("role", "region"), ("aria-label", &controls_aria)], |ctrls| {
                    ctrls.el("div", &[("class", "stage-rail"), ("id", "stage-rail"), ("role", "tablist"), ("aria-label", &rail_aria)], |rail| {
                        for (idx, slide) in STAGE_SLIDES.iter().enumerate() {
                            let idx_str = idx.to_string();
                            let num_str = format!("{:02}", idx + 1);
                            let is_active = idx == 0;
                            let item_cls = if is_active { "rail-item is-active" } else { "rail-item" };
                            let selected = if is_active { "true" } else { "false" };
                            let item_aria = format!("{num_str} — {}", slide.title);

                            rail.el(
                                "button",
                                &[
                                    ("type", "button"),
                                    ("class", item_cls),
                                    ("data-stage-index", &idx_str),
                                    ("role", "tab"),
                                    ("aria-selected", selected),
                                    ("aria-label", &item_aria),
                                ],
                                |btn| {
                                    btn.el("span", &[("class", "rail-num")], |n| { n.text(&num_str); });
                                    btn.el("span", &[("class", "rail-track")], |tr_el| {
                                        tr_el.el("span", &[("class", "rail-fill")], |_| {});
                                    });
                                },
                            );
                        }
                    });

                    ctrls.el("div", &[("class", "stage-nav")], |nav| {
                        nav.el("button", &[("type", "button"), ("class", "stage-nav-btn"), ("id", "stage-prev"), ("aria-label", &prev_aria)], |b| {
                            icon(b, "chevron-left");
                        });
                        nav.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("class", "stage-nav-btn"),
                                ("id", "stage-toggle"),
                                ("aria-label", &toggle_aria),
                                ("aria-pressed", "false"),
                            ],
                            |b| {
                                b.el("span", &[("class", "icon-pause")], |s| icon(s, "pause"));
                                b.el("span", &[("class", "icon-play")], |s| icon(s, "play"));
                            },
                        );
                        nav.el("button", &[("type", "button"), ("class", "stage-nav-btn"), ("id", "stage-next"), ("aria-label", &next_aria)], |b| {
                            icon(b, "chevron-right");
                        });
                    });
                });
            });
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn stage_slides_exist_on_disk_and_render_contract() {
        for slide in STAGE_SLIDES {
            let full_rel = slide.src.trim_start_matches('/');
            let sm_rel = slide.src_sm.trim_start_matches('/');
            let full_path = Path::new("public").join(full_rel.strip_prefix("images/").map_or(full_rel, |_| full_rel));
            let sm_path = Path::new("public").join(sm_rel.strip_prefix("images/").map_or(sm_rel, |_| sm_rel));
            assert!(
                full_path.exists(),
                "Stage slide full image missing: {}",
                full_path.display()
            );
            assert!(
                sm_path.exists(),
                "Stage slide 1280px image missing: {}",
                sm_path.display()
            );
        }

        let tr = TranslationCache::new();
        let mut buf = HtmlBuf::with_capacity(4096);
        render_stage(&mut buf, &tr, "tr", "/", "Yayına Dön");
        let html = buf.into_string();

        assert!(html.contains(r#"id="login-stage""#));
        assert_eq!(html.matches("data-stage-index=").count(), STAGE_SLIDES.len());
        assert!(!html.contains("style="), "SSR stage markup must contain zero inline style attributes");
        assert!(html.contains("/images/admin/stage/01-f4e.webp"));
        assert!(html.contains("F-4E 2020 Terminator"));
        assert!(!html.contains("<h1"), "Stage must use h2 so /podmin keeps a single h1");
    }
}
