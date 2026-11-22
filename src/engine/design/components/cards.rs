use super::super::config::safe_url;
use super::super::html_dsl::HtmlBuf;
use super::super::labels_helper::{lang_href, tr_text};
use super::labels::{category_display_label, localized};
use super::media::image_dimensions;
use super::primitives::{asset, icon, link};
use crate::engine::security_engine::escape_html as esc;
use crate::engine::translation_engine::TranslationCache;
use crate::models::*;

/// Hardens an image path for CSS `url('...')` inside an HTML `style` attribute by
/// stripping ASCII control characters and percent-encoding `'`, `"`, `(`, `)`, `\`, and spaces
/// before HTML-escaping, preventing CSS breakout even after HTML entity decoding.
fn css_url_value(raw_path: &str) -> String {
    let normalized = if raw_path.starts_with("https://") {
        raw_path.to_string()
    } else {
        format!("/{}", raw_path.trim_start_matches('/'))
    };
    let mut sanitized = String::with_capacity(normalized.len());
    for ch in normalized.chars() {
        if ch.is_ascii_control() {
            continue;
        }
        match ch {
            '\'' => sanitized.push_str("%27"),
            '"' => sanitized.push_str("%22"),
            '(' => sanitized.push_str("%28"),
            ')' => sanitized.push_str("%29"),
            '\\' => sanitized.push_str("%5C"),
            ' ' => sanitized.push_str("%20"),
            _ => sanitized.push(ch),
        }
    }
    esc(&sanitized)
}

fn primary_ssb_tag(tags: &[String]) -> Option<&str> {
    tags.iter()
        .map(String::as_str)
        .map(str::trim)
        .find(|t| !t.is_empty() && *t != "*")
}

fn ssb_tag_attr(tags: &[String]) -> String {
    match primary_ssb_tag(tags) {
        Some(tag) => format!(r#" data-ssb-tag="{}""#, esc(tag)),
        None => String::new(),
    }
}

pub fn article_card(c: &ContentItem, lang: &str, eager: bool) -> String {
    let title = esc(localized(lang, &c.title_tr, &c.title_en));
    let summary = esc(localized(lang, &c.summary_tr, &c.summary_en));
    let badge = if c.is_sponsored {
        if lang == "en" { "SPONSORED" } else { "SPONSORLU İÇERİK" }
    } else if c.visibility_level >= 2 {
        if lang == "en" { "SUBSCRIBER EXCLUSIVE" } else { "ABONEYE ÖZEL" }
    } else if c.visibility_level == 1 {
        if lang == "en" { "MEMBER ACCESS" } else { "ÜYELERE ÖZEL" }
    } else {
        category_display_label(&c.content_type, lang)
    };
    let date = c.published_at.get(..10).unwrap_or(&c.published_at);
    let image = asset(&c.cover_image);
    let bg_image = css_url_value(&c.cover_image);
    let (w, h) = image_dimensions(&c.cover_image).unwrap_or((800, 500));
    let is_portrait = h >= w;
    let article_href = if lang == "en" {
        format!("/icerik/{}?lang=en", esc(&c.slug))
    } else {
        format!("/icerik/{}", esc(&c.slug))
    };
    let media_html = if is_portrait {
        format!(
            r#"<a class="story-image is-portrait" href="{href}" tabindex="-1" aria-hidden="true"><span class="story-image-backdrop" style="background-image:url('{bg_image}')" aria-hidden="true"></span><img src="{image}" alt="{title}" loading="{loading}" decoding="async" width="{w}" height="{h}"></a>"#,
            href = article_href,
            bg_image = bg_image,
            image = image,
            title = title,
            loading = if eager { "eager" } else { "lazy" },
            w = w,
            h = h
        )
    } else {
        format!(
            r#"<a class="story-image" href="{href}" tabindex="-1" aria-hidden="true"><img src="{image}" alt="{title}" loading="{loading}" decoding="async" width="{w}" height="{h}"></a>"#,
            href = article_href,
            image = image,
            title = title,
            loading = if eager { "eager" } else { "lazy" },
            w = w,
            h = h
        )
    };
    let ssb_attr = ssb_tag_attr(&c.ssb_tags);
    format!(
        r#"<article class="story-card" data-content-id="{content_id}" data-category-slug="{category_slug}" data-ssb-tags="{tags}"{ssb_attr}>{media_html}<div class="story-copy"><div class="eyebrow">{badge}</div><h3><a href="{href}">{title}</a></h3><p>{summary}</p><div class="story-meta"><time datetime="{date}">{date}</time><span>{minutes} {min_label}</span></div></div></article>"#,
        content_id = c.id,
        category_slug = esc(&c.content_type),
        tags = esc(&c.ssb_tags.join(",")),
        ssb_attr = ssb_attr,
        media_html = media_html,
        href = article_href,
        badge = esc(&badge.replace('-', " ")),
        title = title,
        summary = summary,
        date = esc(date),
        minutes = c.read_time_min.max(1),
        min_label = if lang == "en" { "min read" } else { "dk okuma" }
    )
}

pub fn issue_card(i: &MagazineIssue, lang: &str) -> String {
    let (w, h) = image_dimensions(&i.cover_image).unwrap_or((480, 600));
    let eyebrow_lbl = if lang == "en" { "ISSUE" } else { "SAYI" };
    let aria_lbl = if lang == "en" {
        format!("Explore issue {}", i.issue_number)
    } else {
        format!("{}. sayıyı incele", i.issue_number)
    };
    let pub_type = if i.publication_type.trim().eq_ignore_ascii_case("msi_tdr") {
        "msi_tdr"
    } else {
        "msi"
    };
    format!(
        r##"<article class="issue-card" data-publication-type="{pub_type}"><button class="issue-cover" data-open-issue="{number}" data-publication-type="{pub_type}" aria-label="{aria_lbl}"><img src="{image}" alt="{title}" loading="lazy" decoding="async" width="{w}" height="{h}"></button><div class="eyebrow">{eyebrow_lbl} {number} · {month} {year}</div><h3>{title}</h3><button class="text-link" data-open-issue="{number}" data-publication-type="{pub_type}">{cta} {arrow}</button></article>"##,
        pub_type = pub_type,
        number = i.issue_number,
        aria_lbl = esc(&aria_lbl),
        eyebrow_lbl = eyebrow_lbl,
        image = asset(&i.cover_image),
        title = esc(localized(lang, &i.title_tr, &i.title_en)),
        month = esc(localized(lang, &i.publish_month_tr, &i.publish_month_en)),
        year = i.year,
        w = w,
        h = h,
        cta = if lang == "en" { "Explore issue" } else { "Sayıyı incele" },
        arrow = icon("arrow")
    )
}

pub fn advertisement(ad: &AdCampaign, lang: &str, id: &str) -> String {
    let corner_tag = if lang == "en" { "AD" } else { "REKLAM" };
    let aria_ad = if lang == "en" { "Advertisement" } else { "Reklam" };
    let ad_ssb_attr = ssb_tag_attr(&ad.target_ssb_tags);
    let lang_q = if lang == "en" { "?lang=en" } else { "" };
    let resolved_target = if lang == "en" && ad.target_url.starts_with('/') && !ad.target_url.contains("lang=") {
        if ad.target_url.contains('?') {
            format!("{}&lang=en", ad.target_url)
        } else {
            format!("{}?lang=en", ad.target_url)
        }
    } else {
        ad.target_url.clone()
    };
    if let Some(img_path) = ad.image_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let (w, h) = image_dimensions(img_path).unwrap_or((900, 300));
        let is_graphic_banner = img_path.contains(crate::db::constants::AD_GRAPHIC_BANNER_SUBSTRING);
        if is_graphic_banner {
            return format!(
                r#"<aside class="advertisement ad-visual-banner" id="{id}" data-ad-id="{ad_id}"{ad_ssb_attr} aria-label="{aria_ad}"><a class="ad-banner-frame" href="{href}" data-ad-click-id="{ad_id}"><span class="ad-corner-tag">{corner_tag}</span><img src="{img}" alt="{sponsor} — {title}" loading="lazy" decoding="async" width="{w}" height="{h}"></a></aside>"#,
                id = esc(id),
                ad_id = ad.id,
                ad_ssb_attr = ad_ssb_attr,
                aria_ad = aria_ad,
                corner_tag = corner_tag,
                href = link(&resolved_target),
                img = asset(img_path),
                sponsor = esc(&ad.sponsor_name),
                title = esc(localized(lang, &ad.title_tr, &ad.title_en)),
                w = w,
                h = h
            );
        }
        let cta_lbl = if lang == "en" { "Explore" } else { "İncele" };
        let collab_lbl = if lang == "en" {
            "Advertising & sponsorship"
        } else {
            "Reklam ve iş birliği"
        };
        let disc_lbl = if lang == "en" { "SPONSORED" } else { "REKLAM" };
        return format!(
            r#"<aside class="advertisement ad-hybrid" id="{id}" data-ad-id="{ad_id}"{ad_ssb_attr} aria-label="{aria_ad}"><div class="ad-disclosure"><span>{disc_lbl} · <strong>{sponsor}</strong></span><a href="/kurumsal/reklam-ve-sponsorluk{lang_q}">{collab_lbl}</a></div><a class="ad-content ad-content-media" href="{href}" data-ad-click-id="{ad_id}"><span class="ad-thumb"><img src="{img}" alt="{title}" loading="lazy" decoding="async" width="{w}" height="{h}"></span><div class="ad-copy"><span class="eyebrow">{badge}</span><h3>{title}</h3><p>{desc}</p></div><span class="ad-cta">{cta_lbl} {arrow}</span></a></aside>"#,
            id = esc(id),
            ad_id = ad.id,
            ad_ssb_attr = ad_ssb_attr,
            aria_ad = aria_ad,
            disc_lbl = disc_lbl,
            sponsor = esc(&ad.sponsor_name),
            lang_q = lang_q,
            collab_lbl = collab_lbl,
            href = link(&resolved_target),
            img = asset(img_path),
            title = esc(localized(lang, &ad.title_tr, &ad.title_en)),
            w = w,
            h = h,
            badge = esc(&ad.badge_label),
            desc = esc(localized(lang, &ad.desc_tr, &ad.desc_en)),
            cta_lbl = cta_lbl,
            arrow = icon("arrow")
        );
    }
    let collab_lbl = if lang == "en" {
        "Advertising & sponsorship"
    } else {
        "Reklam ve iş birliği"
    };
    let disc_lbl = if lang == "en" { "SPONSORED" } else { "REKLAM" };
    format!(
        r#"<aside class="advertisement" id="{}" data-ad-id="{}"{} aria-label="{}"><div class="ad-disclosure">{} · {}<a href="/kurumsal/reklam-ve-sponsorluk{}">{}</a></div><a class="ad-content" href="{}" data-ad-click-id="{}"><div><span class="eyebrow">{}</span><h3>{}</h3><p>{}</p></div><span class="ad-arrow">{}</span></a></aside>"#,
        esc(id),
        ad.id,
        ad_ssb_attr,
        aria_ad,
        disc_lbl,
        esc(&ad.sponsor_name),
        lang_q,
        collab_lbl,
        link(&resolved_target),
        ad.id,
        esc(&ad.badge_label),
        esc(localized(lang, &ad.title_tr, &ad.title_en)),
        esc(localized(lang, &ad.desc_tr, &ad.desc_en)),
        icon("arrow")
    )
}

/// Renders the authentic MSI right-hand editorial directory + related thumbnail cards + vertical 300x900 ad rail.
pub fn editorial_sidebar(
    active_category: &str,
    related: &[ContentItem],
    ads: &[AdCampaign],
    lang: &str,
) -> String {
    let dir_items: &[(&str, &str, &str)] = &[
        ("haberler", "HABERLER", "NEWS"),
        ("yeni-sayi-kapak", "YENİ SAYI KAPAK", "NEW ISSUE"),
        ("kapak-dosyasi", "KAPAK DOSYALARI", "COVER FILES"),
        ("soylesiler", "SÖYLEŞİLER", "INTERVIEWS"),
        ("ozel-haberler", "ÖZEL HABERLER", "EXCLUSIVE NEWS"),
        ("basyazilar", "BAŞYAZILAR", "EDITORIALS"),
        ("mesajlar", "MESAJLAR", "MESSAGES"),
        ("tanitici-makaleler", "TANITICI MAKALELER", "SPECIAL COVERAGES"),
        ("makaleler-analizler", "MAKALELER / ANALİZLER", "ARTICLES & ANALYSIS"),
        ("advertorials", "ADVERTORIALS", "ADVERTORIALS"),
        ("ozel-dosya", "ÖZEL DOSYA", "SPECIAL DOSSIERS"),
    ];

    let lang_q = if lang == "en" { "?lang=en" } else { "" };
    let nav_html: String = dir_items
        .iter()
        .map(|(slug, tr_lbl, en_lbl)| {
            let is_active = active_category.eq_ignore_ascii_case(slug)
                || (*slug == "kapak-dosyasi" && active_category == "kapak-dosyalari")
                || (*slug == "basyazilar" && active_category == "basyazi")
                || (*slug == "advertorials" && active_category == "advertorial")
                || (*slug == "ozel-dosya" && active_category == "ozel-dosyalar");
            format!(
                r#"<a class="sidebar-cat-btn{}" href="/kategori/{}{}">{}</a>"#,
                if is_active { " is-active" } else { "" },
                slug,
                lang_q,
                if lang == "en" { *en_lbl } else { *tr_lbl }
            )
        })
        .collect();

    let related_html = if related.is_empty() {
        String::new()
    } else {
        let heading_lbl = if lang == "en" {
            "RECOMMENDED FOR YOU"
        } else {
            "İLGİNİZİ ÇEKEBİLİR"
        };
        let cards: String = related
            .iter()
            .take(crate::db::constants::COMPONENTS_HIGHLIGHT_LIMIT)
            .map(|c| {
                let t = esc(localized(lang, &c.title_tr, &c.title_en));
                let img = asset(&c.cover_image);
                let bg_img = css_url_value(&c.cover_image);
                let (w, h) = image_dimensions(&c.cover_image).unwrap_or((480, 300));
                let date = esc(c.published_at.get(..10).unwrap_or(&c.published_at));
                let portrait_cls = if h >= w { " is-portrait" } else { "" };
                let backdrop = if h >= w {
                    format!(
                        r#"<span class="story-image-backdrop" style="background-image:url('{bg_img}')" aria-hidden="true"></span>"#
                    )
                } else {
                    String::new()
                };
                let ssb_attr = ssb_tag_attr(&c.ssb_tags);
                format!(
                    r#"<article class="sidebar-rel-card" data-content-id="{content_id}" data-category-slug="{category_slug}" data-ssb-tags="{tags}"{ssb_attr}><a class="sidebar-rel-thumb{portrait_cls}" href="/icerik/{slug}{lang_q}">{backdrop}<img src="{img}" alt="{t}" loading="lazy" decoding="async" width="{w}" height="{h}"></a><h4><a href="/icerik/{slug}{lang_q}">{t}</a></h4><time>{date}</time></article>"#,
                    content_id = c.id,
                    category_slug = esc(&c.content_type),
                    tags = esc(&c.ssb_tags.join(",")),
                    ssb_attr = ssb_attr,
                    portrait_cls = portrait_cls,
                    slug = esc(&c.slug),
                    lang_q = lang_q,
                    backdrop = backdrop,
                    img = img,
                    t = t,
                    w = w,
                    h = h,
                    date = date
                )
            })
            .collect();
        format!(
            r#"<div class="sidebar-widget"><div class="msi-section-ribbon is-compact">{heading_lbl}</div><div class="sidebar-rel-list">{cards}</div></div>"#
        )
    };

    let tower_pool: Vec<&AdCampaign> = ads
        .iter()
        .filter(|a| a.is_active && a.slot_code.starts_with(crate::db::constants::AD_SLOT_HOME_RIGHT_PREFIX))
        .collect();
    let seed_key = format!(
        "{}:{}",
        active_category,
        related.first().map(|c| c.slug.as_str()).unwrap_or("")
    );
    let seed_idx: usize = seed_key
        .bytes()
        .fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
    let selected_tower = if !tower_pool.is_empty() {
        Some(tower_pool[seed_idx % tower_pool.len()])
    } else {
        ads.iter()
            .find(|a| a.is_active && !a.is_floating_popup && a.image_url.is_some())
    };
    let tower_tag = if lang == "en" { "AD" } else { "REKLAM" };
    let tower_ad = selected_tower
        .map(|a| {
            let img = a
                .image_url
                .as_deref()
                .unwrap_or(crate::services::media_service::DEFAULT_AD_SIDEBAR);
            let (w, h) = image_dimensions(img).unwrap_or((533, 1600));
            let resolved_href = lang_href(&a.target_url, lang);
            let ad_ssb_attr = ssb_tag_attr(&a.target_ssb_tags);
            format!(
                r#"<div class="sidebar-widget sidebar-tower-ad" data-ad-id="{id}"{ad_ssb_attr}><a class="ad-banner-frame is-vertical" href="{href}" data-ad-click-id="{id}"><span class="ad-corner-tag">{tower_tag}</span><img src="{src}" alt="{sponsor} — {title}" loading="lazy" decoding="async" width="{w}" height="{h}"></a></div>"#,
                id = a.id,
                ad_ssb_attr = ad_ssb_attr,
                href = link(&resolved_href),
                tower_tag = tower_tag,
                src = asset(img),
                sponsor = esc(&a.sponsor_name),
                title = esc(localized(lang, &a.title_tr, &a.title_en)),
                w = w,
                h = h
            )
        })
        .unwrap_or_default();

    let dir_aria = if lang == "en" { "Editorial directory" } else { "Yayın dizini" };
    let sec_aria = if lang == "en" { "Sections" } else { "Bölümler" };
    format!(
        r#"<aside class="editorial-sidebar" aria-label="{dir_aria}"><nav class="sidebar-cat-nav" aria-label="{sec_aria}">{nav_html}</nav>{related_html}{tower_ad}</aside>"#
    )
}

/// Renders the non-blocking floating popup ad (`ad-floating-popup`) when active and flagged.
pub fn floating_popup_ad(
    ad: &AdCampaign,
    lang: &str,
    tr: Option<&TranslationCache>,
) -> String {
    if !ad.is_active || !ad.is_floating_popup {
        return String::new();
    }
    let ad_id_str = ad.id.to_string();
    let tag_lbl = tr_text(tr, lang, "ads.sponsored_tag", "SPONSORLU", "SPONSORED");
    let close_lbl = tr_text(tr, lang, "shell.close", "Kapat", "Close");
    let cta_lbl = tr_text(tr, lang, "ads.explore_cta", "İncele →", "Explore →");
    let resolved_href = lang_href(&ad.target_url, lang);
    let safe_href = if safe_url(&resolved_href) {
        resolved_href
    } else {
        "#".to_string()
    };
    let title = localized(lang, &ad.title_tr, &ad.title_en);
    let desc = localized(lang, &ad.desc_tr, &ad.desc_en);
    let aria_lbl = format!("{} — {}", ad.sponsor_name, title);

    let mut aside_attrs: Vec<(&str, &str)> = vec![
        ("id", "ad-floating-popup"),
        ("class", "ad-floating-popup"),
        ("data-ad-id", &ad_id_str),
        ("data-ad-floating", "1"),
        ("role", "complementary"),
        ("aria-label", &aria_lbl),
    ];
    if let Some(tag) = primary_ssb_tag(&ad.target_ssb_tags) {
        aside_attrs.push(("data-ssb-tag", tag));
    }

    let mut buf = HtmlBuf::with_capacity(1024);
    buf.el(
        "aside",
        &aside_attrs,
        |b| {
            b.el("div", &[("class", "ad-floating-head")], |h| {
                h.el("span", &[("class", "ad-floating-sponsor")], |sp| {
                    sp.el("strong", &[], |s| {
                        s.text(&tag_lbl);
                    });
                    sp.text(" · ");
                    sp.text(&ad.sponsor_name);
                });
                h.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "icon-button ad-floating-close"),
                        ("data-dismiss-floating-ad", &ad_id_str),
                        ("aria-label", &close_lbl),
                    ],
                    |btn| {
                        btn.raw(&icon("close"));
                    },
                );
            });
            b.el(
                "a",
                &[
                    ("class", "ad-floating-body"),
                    ("href", &safe_href),
                    ("data-ad-click-id", &ad_id_str),
                ],
                |body| {
                    if let Some(img_path) =
                        ad.image_url.as_deref().map(str::trim).filter(|s| !s.is_empty())
                    {
                        let img_src = if img_path.starts_with("https://") {
                            img_path.to_string()
                        } else {
                            format!("/{}", img_path.trim_start_matches('/'))
                        };
                        let (w, h) = image_dimensions(img_path).unwrap_or((600, 400));
                        let w_str = w.to_string();
                        let h_str = h.to_string();
                        body.void_el(
                            "img",
                            &[
                                ("class", "ad-floating-img"),
                                ("src", &img_src),
                                ("alt", title),
                                ("loading", "lazy"),
                                ("decoding", "async"),
                                ("width", &w_str),
                                ("height", &h_str),
                            ],
                        );
                    }
                    body.el("div", &[("class", "ad-floating-copy")], |cp| {
                        if !ad.badge_label.trim().is_empty() {
                            cp.el("span", &[("class", "eyebrow")], |eb| {
                                eb.text(&ad.badge_label);
                            });
                        }
                        cp.el("h3", &[], |h3| {
                            h3.text(title);
                        });
                        if !desc.trim().is_empty() {
                            cp.el("p", &[], |p| {
                                p.text(desc);
                            });
                        }
                        cp.el("span", &[("class", "ad-floating-cta")], |cta| {
                            cta.text(&cta_lbl);
                        });
                    });
                },
            );
        },
    );
    buf.into_string()
}

/// Centralized notification item — used by both shell.rs (public header) and account.rs (user dashboard).
pub fn notification_item(n: &SystemNotification, lang: &str) -> String {
    let view_lbl = if lang == "en" { "View →" } else { "İncele →" };
    let action = n
        .action_url
        .as_ref()
        .filter(|u| safe_url(u))
        .map(|u| format!("<a class=\"text-link\" href=\"{}\">{view_lbl}</a>", link(u)))
        .unwrap_or_default();
    format!(
        "<article class=\"notification-item\" data-notification-id=\"{}\"><time>{}</time><h3>{}</h3><p>{}</p>{}</article>",
        n.id,
        esc(n.created_at.get(..10).unwrap_or(&n.created_at)),
        esc(localized(lang, &n.title_tr, &n.title_en)),
        esc(localized(lang, &n.message_tr, &n.message_en)),
        action
    )
}

#[derive(Clone, Debug)]
pub struct DossierTocEntry<'a> {
    pub index_label: String,
    pub badge: String,
    pub title: &'a str,
    pub summary: &'a str,
    pub href: String,
    pub read_time_min: Option<i32>,
}

/// Renders a single issue-style TOC row inside `.dossier-toc-list`.
pub fn dossier_toc_item(entry: &DossierTocEntry<'_>, lang: &str) -> String {
    let safe_href = if safe_url(&entry.href) {
        entry.href.as_str()
    } else {
        "#"
    };
    let min_label = if lang.eq_ignore_ascii_case("en") {
        "min read"
    } else {
        "dk okuma"
    };
    let mut buf = HtmlBuf::with_capacity(512);
    buf.el("li", &[("class", "dossier-toc-item")], |li| {
        li.el("span", &[("class", "dossier-toc-index")], |idx| {
            idx.text(&entry.index_label);
        });
        li.el("div", &[("class", "dossier-toc-body")], |body| {
            let has_badge = !entry.badge.trim().is_empty();
            let has_time = entry.read_time_min.map(|m| m > 0).unwrap_or(false);
            if has_badge || has_time {
                body.el("div", &[("class", "dossier-toc-meta")], |meta| {
                    if has_badge {
                        meta.el("span", &[("class", "eyebrow")], |eb| {
                            eb.text(&entry.badge);
                        });
                    }
                    if let Some(mins) = entry.read_time_min.filter(|m| *m > 0) {
                        meta.el("span", &[("class", "dossier-toc-time")], |tm| {
                            tm.text(&format!("{} {}", mins, min_label));
                        });
                    }
                });
            }
            body.el("h3", &[("class", "dossier-toc-title")], |h3| {
                h3.el(
                    "a",
                    &[("class", "dossier-toc-link"), ("href", safe_href)],
                    |a| {
                        a.text(entry.title);
                    },
                );
            });
            if !entry.summary.trim().is_empty() {
                body.el("p", &[("class", "dossier-toc-summary")], |p| {
                    p.text(entry.summary);
                });
            }
        });
        li.el(
            "a",
            &[
                ("class", "dossier-toc-arrow"),
                ("href", safe_href),
                ("aria-label", entry.title),
            ],
            |a| {
                a.raw(&icon("arrow"));
            },
        );
    });
    buf.into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dossier_toc_item_escapes_html_and_sanitizes_urls() {
        let entry = DossierTocEntry {
            index_label: "01".into(),
            badge: "KAPAK DOSYASI".into(),
            title: "<script>alert(1)</script> KAAN".into(),
            summary: "Detaylı analiz & değerlendirme".into(),
            href: "javascript:alert(1)".into(),
            read_time_min: Some(6),
        };
        let tr_html = dossier_toc_item(&entry, "tr");
        assert!(tr_html.contains("class=\"dossier-toc-item\""));
        assert!(tr_html.contains("class=\"dossier-toc-link\""));
        assert!(tr_html.contains("href=\"#\""));
        assert!(!tr_html.contains("<script>"));
        assert!(tr_html.contains("&lt;script&gt;alert(1)&lt;/script&gt; KAAN"));
        assert!(tr_html.contains("6 dk okuma"));

        let en_entry = DossierTocEntry {
            index_label: "P. 14".into(),
            badge: "COVER STORY".into(),
            title: "Naval Air Defence",
            summary: "",
            href: "/icerik/naval-air-defence?lang=en".into(),
            read_time_min: Some(4),
        };
        let en_html = dossier_toc_item(&en_entry, "en");
        assert!(en_html.contains("href=\"/icerik/naval-air-defence?lang=en\""));
        assert!(en_html.contains("4 min read"));
        assert!(!en_html.contains("dossier-toc-summary"));
    }

    #[test]
    fn css_url_value_prevents_style_url_breakout() {
        let escaped = css_url_value("images/foo');background:red;//\n\t \"\\");
        assert!(!escaped.contains('\''));
        assert!(!escaped.contains('"'));
        assert!(!escaped.contains('('));
        assert!(!escaped.contains(')'));
        assert!(!escaped.contains('\\'));
        assert!(!escaped.contains(' '));
        assert!(!escaped.contains('\n'));
        assert!(!escaped.contains('\t'));
        assert_eq!(escaped, "/images/foo%27%29;background:red;//%20%22%5C");
    }
}
