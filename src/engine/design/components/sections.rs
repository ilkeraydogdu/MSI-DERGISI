use super::super::config::{safe_url, Section, SectionKind};
use super::super::html_dsl::HtmlBuf;
use super::super::labels_helper::{
    is_en_content_ready, lang_href, localize_section_copy, tr_text,
};
use super::PageData;
use super::cards::{advertisement, article_card, dossier_toc_item, issue_card, DossierTocEntry};
use super::filters::{enrich_issue_spreads, is_published_content};
use super::labels::{category_display_label, localized, ssb_tag_display_label};
use super::layout::heading_localized;
use super::media::image_dimensions;
use super::primitives::{asset, icon, link};
use super::select::select_for_page;
use crate::engine::security_engine::escape_html as esc;
use crate::engine::translation_engine::TranslationCache;
use crate::models::*;

fn render_ad_strip(d: &PageData<'_>, slots: &[&str], index: usize) -> Option<String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let matched: Vec<&AdCampaign> = slots
        .iter()
        .filter_map(|code| {
            d.ads.iter().find(|a| {
                a.slot_code == *code
                    && a.is_active
                    && !a.is_floating_popup
                    && crate::services::ad_service::is_ad_within_schedule(a, &today)
            })
        })
        .collect();
    if matched.is_empty() {
        return None;
    }
    let cards: String = matched
        .iter()
        .enumerate()
        .map(|(sub_idx, ad)| advertisement(ad, d.lang, &format!("placement-{index}-{sub_idx}")))
        .collect();
    Some(format!(
        r#"<div class="container section-ad-strip" data-section="{index}"><div class="ad-banner-strip cols-{cols}">{cards}</div></div>"#,
        index = index,
        cols = matched.len().min(3),
        cards = cards
    ))
}

pub fn render_section(
    s: &Section,
    d: &PageData<'_>,
    index: usize,
    seen_ids: &mut std::collections::HashSet<i64>,
) -> String {
    let content = match s.kind {
        SectionKind::MagazineHero => {
            let issue = if d.lang == "en" {
                d.issues
                    .iter()
                    .filter(|i| i.publication_type == "msi_tdr")
                    .max_by_key(|i| i.issue_number)
                    .or_else(|| d.featured_issue.and_then(|n| d.issues.iter().find(|i| i.issue_number == n)))
            } else {
                d.featured_issue
                    .and_then(|n| d.issues.iter().find(|i| i.issue_number == n))
            }
            .or_else(|| d.issues.iter().max_by_key(|i| i.issue_number));
            let Some(i) = issue else {
                let empty_msg = if d.lang == "en" {
                    "New issue coming soon."
                } else {
                    "Yeni sayı yakında burada."
                };
                return format!(
                    "<section class=\"container section-space\">{}<p>{}</p></section>",
                    heading_localized(s, d.lang),
                    empty_msg
                );
            };
            let pub_type = if i.publication_type.trim().eq_ignore_ascii_case("msi_tdr") {
                "msi_tdr"
            } else {
                "msi"
            };
            let preview_spreads = enrich_issue_spreads(i, d.contents);
            let tabs_aria = if d.lang == "en" {
                "Special dossiers in this issue"
            } else {
                "Sayı içi özel dosyalar"
            };
            let spread_tabs = if preview_spreads.len() <= 1 {
                String::new()
            } else {
                format!(
                    "<div class=\"stage-tabs\" role=\"tablist\" aria-label=\"{tabs_aria}\">{}</div>",
                    preview_spreads
                        .iter()
                        .enumerate()
                        .map(|(idx, _)| format!(
                            "<button type=\"button\" class=\"stage-tab{active}\" role=\"tab\" aria-selected=\"{sel}\" data-hero-spread=\"{idx}\">{num:02}</button>",
                            active = if idx == 0 { " is-active" } else { "" },
                            sel = if idx == 0 { "true" } else { "false" },
                            idx = idx,
                            num = idx + 1
                        ))
                        .collect::<String>()
                )
            };
            let lang_q = if d.lang == "en" { "?lang=en" } else { "" };
            let live_dossier = if let Some(first_sp) = preview_spreads.first() {
                let raw_title = localized(d.lang, &first_sp.title_tr, &first_sp.title_en).trim();
                let raw_excerpt =
                    localized(d.lang, &first_sp.excerpt_tr, &first_sp.excerpt_en).trim();
                let sp_title = esc(raw_title);
                let show_excerpt =
                    !raw_excerpt.is_empty() && !raw_excerpt.eq_ignore_ascii_case(raw_title);
                let excerpt_html = if show_excerpt {
                    format!(
                        "<p class=\"dossier-excerpt\" data-dossier-excerpt>{}</p>",
                        esc(raw_excerpt)
                    )
                } else {
                    "<p class=\"dossier-excerpt\" data-dossier-excerpt hidden></p>".to_string()
                };
                let next_lbl = if d.lang == "en" {
                    "Next dossier ↻"
                } else {
                    "Sonraki dosya ↻"
                };
                let next_btn = if preview_spreads.len() > 1 {
                    format!("<button type=\"button\" class=\"dossier-next\" data-hero-next-spread>{next_lbl}</button>")
                } else {
                    String::new()
                };
                let read_lbl = if d.lang == "en" { "Read dossier →" } else { "Dosyayı oku →" };
                let open_lbl = if d.lang == "en" { "Open in issue →" } else { "Sayıda aç →" };
                let sp_link = first_sp
                    .linked_content_slug
                    .as_deref()
                    .filter(|slug| !slug.is_empty())
                    .map(|slug| {
                        format!(
                            "<a class=\"dossier-link\" data-dossier-link href=\"/icerik/{}{lang_q}\">{read_lbl}</a>",
                            esc(slug)
                        )
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "<button type=\"button\" class=\"dossier-link\" data-open-issue=\"{}\" data-publication-type=\"{}\">{open_lbl}</button>",
                            i.issue_number,
                            pub_type
                        )
                    });
                let display_badge = if d.lang == "en" {
                    match first_sp.category_badge.trim().to_ascii_uppercase().as_str() {
                        "KAPAK DOSYASI" | "KAPAK DOSYALARI" => "COVER FILE".to_string(),
                        "SÖYLEŞİ" | "SÖYLEŞİLER" => "INTERVIEW".to_string(),
                        "ÖZEL DOSYA" | "ÖZEL DOSYALAR" => "SPECIAL DOSSIER".to_string(),
                        "ÖZEL HABER" | "ÖZEL HABERLER" => "EXCLUSIVE REPORT".to_string(),
                        "BAŞYAZI" | "BAŞYAZILAR" => "EDITORIAL".to_string(),
                        "MESAJ" | "MESAJLAR" => "EXECUTIVE MESSAGE".to_string(),
                        "TANITICI MAKALE" | "TANITICI MAKALELER" => "SPECIAL COVERAGE".to_string(),
                        "ANALİZ" | "MAKALE / ANALİZ" => "ANALYSIS".to_string(),
                        "HABER" | "HABERLER" => "NEWS".to_string(),
                        "YENİ SAYI" => "NEW ISSUE".to_string(),
                        "DOSYA" => "DOSSIER".to_string(),
                        other => other.to_string(),
                    }
                } else {
                    first_sp.category_badge.clone()
                };
                let display_pages = if d.lang == "en" {
                    first_sp
                        .page_no
                        .replacen("S. ", "P. ", 1)
                        .replacen("DOSYA ", "DOSSIER ", 1)
                } else {
                    first_sp.page_no.clone()
                };
                format!(
                    r#"<div class="hero-live-dossier" data-live-dossier aria-live="polite"><span class="dossier-progress" data-dossier-progress aria-hidden="true"></span><div class="dossier-meta"><span class="dossier-badge" data-dossier-badge>{badge}</span><span class="dossier-pages" data-dossier-pages>{page_no}</span></div><strong class="dossier-title" data-dossier-title>{sp_title}</strong>{excerpt_html}<div class="dossier-actions">{sp_link}{next_btn}</div></div>"#,
                    badge = esc(&display_badge),
                    page_no = esc(&display_pages),
                    sp_title = sp_title,
                    excerpt_html = excerpt_html,
                    sp_link = sp_link,
                    next_btn = next_btn
                )
            } else {
                String::new()
            };
            let (cw, ch) = image_dimensions(&i.cover_image).unwrap_or((960, 1200));
            let open_cta = if d.lang == "en" {
                format!("Explore Issue {} live", i.issue_number)
            } else {
                format!("{}. sayıyı canlı incele", i.issue_number)
            };
            let (metrics_aria, m1_val, m1_lbl, m2_val, m2_lbl, m3_lbl, f1, f2, ref_lbl, cover_stats) =
                if d.lang == "en" {
                    (
                        "Publication metrics",
                        "22+ Years",
                        "CONTINUOUS AUTHORITY",
                        format!("{}+ Issues", i.issue_number),
                        "STRATEGIC ARCHIVE",
                        "GLOBAL REFERENCE",
                        "INDEPENDENT DEFENCE INTELLIGENCE",
                        "DEFENCE · AEROSPACE · SPACE · NAVAL",
                        "REFERENCE ISSUE",
                        format!("{} pages · {} special dossiers", i.page_count, i.dossier_count),
                    )
                } else {
                    (
                        "Yayın göstergeleri",
                        "22+ Yıl",
                        "KESİNTİSİZ OTORİTE",
                        format!("{}+ Sayı", i.issue_number),
                        "STRATEJİK HAFIZA",
                        "KÜRESEL REFERANS",
                        "BAĞIMSIZ SAVUNMA İSTİHBARATI",
                        "SAVUNMA · HAVACILIK · UZAY · DENİZCİLİK",
                        "REFERANS SAYI",
                        format!("{} sayfa · {} özel dosya", i.page_count, i.dossier_count),
                    )
                };
            let stage_topbar = if spread_tabs.is_empty() {
                String::new()
            } else {
                format!(r#"<div class="stage-topbar">{spread_tabs}</div>"#)
            };
            let hero_eyebrow = localize_section_copy(d.lang, &s.eyebrow);
            let hero_title = localize_section_copy(d.lang, &s.title);
            let hero_desc = localize_section_copy(d.lang, &s.description);
            let hero_link_label = localize_section_copy(d.lang, &s.link_label);
            let hero_link = lang_href(&s.link, d.lang);
            return format!(
                r#"<div class="hero-stage-band"><section class="magazine-hero container" data-hero-section><div class="hero-copy"><div class="eyebrow"><span class="accent-rule"></span>{eyebrow}</div><h1>{title}</h1><p class="hero-description">{description}</p><div class="hero-actions"><button class="button primary" data-open-issue="{number}" data-publication-type="{pub_type}">{open_cta} {arrow}</button><a class="text-link" href="{link}">{link_label} {arrow}</a></div><div class="hero-metrics" aria-label="{metrics_aria}"><div><strong>{m1_val}</strong><span>{m1_lbl}</span></div><div><strong>{m2_val}</strong><span>{m2_lbl}</span></div><div><strong>TR · EN</strong><span>{m3_lbl}</span></div></div><div class="hero-footnote"><span>{f1}</span><span>{f2}</span></div></div><div class="hero-cover-stage" data-hero-stage data-issue-number="{number}" data-publication-type="{pub_type}">{stage_topbar}<span class="cover-index" aria-hidden="true">{number}<small>{ref_lbl}</small></span><div class="hero-mockup-3d" data-tilt-card><button class="hero-cover" data-open-issue="{number}" data-publication-type="{pub_type}" aria-label="{issue_title}"><img src="{cover}" alt="{issue_title}" width="{cw}" height="{ch}" fetchpriority="high"><span class="cover-sheen" aria-hidden="true"></span></button></div>{live_dossier}<div class="cover-caption"><span>{month} {year} · ISSN 1306-0309</span><span>{cover_stats}</span></div></div></section></div>"#,
                eyebrow = esc(hero_eyebrow),
                title = esc(hero_title).replace('\n', "<br>"),
                description = esc(hero_desc),
                number = i.issue_number,
                pub_type = pub_type,
                open_cta = open_cta,
                arrow = icon("arrow"),
                link = link(&hero_link),
                link_label = esc(hero_link_label),
                metrics_aria = metrics_aria,
                m1_val = m1_val,
                m1_lbl = m1_lbl,
                m2_val = m2_val,
                m2_lbl = m2_lbl,
                m3_lbl = m3_lbl,
                f1 = f1,
                f2 = f2,
                stage_topbar = stage_topbar,
                ref_lbl = ref_lbl,
                issue_title = esc(localized(d.lang, &i.title_tr, &i.title_en)),
                cover = asset(&i.cover_image),
                cw = cw,
                ch = ch,
                live_dossier = live_dossier,
                month = esc(localized(d.lang, &i.publish_month_tr, &i.publish_month_en)),
                year = i.year,
                cover_stats = cover_stats
            );
        }
        SectionKind::NewsHero => {
            let items = select_for_page(s, d, seen_ids);
            let cards: String = items
                .iter()
                .enumerate()
                .map(|(i, c)| article_card(c, d.lang, i == 0))
                .collect();
            return format!(
                "<section class=\"container section-space news-lead\"><div class=\"eyebrow\">{}</div><h1>{}</h1><div class=\"news-hero-grid\">{}</div></section>",
                esc(localize_section_copy(d.lang, &s.eyebrow)),
                esc(localize_section_copy(d.lang, &s.title)),
                cards
            );
        }
        SectionKind::Articles => {
            let selected = select_for_page(s, d, seen_ids);
            if selected.is_empty() {
                if d.lang == "en" {
                    return String::new();
                }
                "<div class=\"empty-state\">Bu bölümde henüz içerik yok. <a href=\"/arama\">Arşivi keşfedin →</a></div>".into()
            } else {
                let cards: String = selected
                    .iter()
                    .map(|c| article_card(c, d.lang, false))
                    .collect();
                format!("<div class=\"stories layout-{}\">{cards}</div>", esc(&s.layout))
            }
        }
        SectionKind::Issues => {
            let mut issues: Vec<_> = d.issues.iter().collect();
            issues.sort_by_key(|i| std::cmp::Reverse(i.issue_number));
            format!(
                "<div class=\"issues-grid\">{}</div>",
                issues
                    .into_iter()
                    .skip(s.offset)
                    .take(s.limit)
                    .map(|i| issue_card(i, d.lang))
                    .collect::<String>()
            )
        }
        SectionKind::Topics => {
            let search_base = if d.lang == "en" { "/en/search" } else { "/arama" };
            format!(
                "<div class=\"topics-grid\">{}</div>",
                d.tags
                    .iter()
                    .take(s.limit)
                    .enumerate()
                    .map(|(i, t)| format!(
                        "<a href=\"{}?ssb_tag={}\"><span class=\"topic-index\">{:02}</span><span>{}</span>{}</a>",
                        search_base,
                        esc(&urlencoding::encode(&t.code)),
                        i + 1,
                        esc(localized(d.lang, &t.name_tr, &t.name_en)),
                        icon("arrow")
                    ))
                    .collect::<String>()
            )
        }
        SectionKind::Banner => {
            let strip = match s.category.as_str() {
                "ad-strip-top" => render_ad_strip(
                    d,
                    &["ad-live-home-top-1", "ad-live-home-top-2", "ad-live-home-top-3"],
                    index,
                ),
                "ad-strip-mid" => render_ad_strip(
                    d,
                    &["ad-live-home-mid-1", "ad-live-home-mid-2", "ad-live-home-mid-3"],
                    index,
                ),
                "ad-strip-bottom" => render_ad_strip(
                    d,
                    &[
                        "ad-live-home-bottom-1",
                        "ad-live-home-bottom-2",
                        "ad-live-home-footer-1",
                    ],
                    index,
                ),
                _ => None,
            };
            if let Some(html) = strip {
                return html;
            }
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            return d
                .ads
                .iter()
                .find(|a| {
                    a.slot_code == s.category
                        && a.is_active
                        && !a.is_floating_popup
                        && crate::services::ad_service::is_ad_within_schedule(a, &today)
                })
                .map(|a| {
                    format!(
                        "<div class=\"container\">{}</div>",
                        advertisement(a, d.lang, &format!("placement-{index}"))
                    )
                })
                .unwrap_or_default();
        }
        SectionKind::Text => format!(
            "<div class=\"editorial-prose\">{}</div>",
            esc(localize_section_copy(d.lang, &s.description))
                .split("\n\n")
                .map(|p| format!("<p>{}</p>", p.replace('\n', "<br>")))
                .collect::<String>()
        ),
        SectionKind::Cta => {
            let cta_href = lang_href(&s.link, d.lang);
            return format!(
                "<section class=\"container section-space\"><div class=\"membership-banner\"><div><div class=\"eyebrow\">{}</div><h2>{}</h2><p>{}</p></div><a class=\"button primary\" href=\"{}\">{} {}</a></div></section>",
                esc(localize_section_copy(d.lang, &s.eyebrow)),
                esc(localize_section_copy(d.lang, &s.title)),
                esc(localize_section_copy(d.lang, &s.description)),
                link(&cta_href),
                esc(localize_section_copy(d.lang, &s.link_label)),
                icon("arrow")
            )
        }
        SectionKind::Plans => format!(
            "<div class=\"plan-grid\">{}</div>",
            d.plans
                .iter()
                .take(s.limit)
                .map(|p| {
                    let card_cls = if p.is_featured {
                        "plan-card is-featured"
                    } else {
                        "plan-card"
                    };
                    let btn_cls = if p.is_featured {
                        "button primary"
                    } else {
                        "button secondary"
                    };
                    let eyebrow_lbl = if p.is_featured {
                        if d.lang == "en" {
                            "RECOMMENDED PLAN"
                        } else {
                            "ÖNERİLEN PLAN"
                        }
                    } else if d.lang == "en" {
                        "SUBSCRIPTION"
                    } else {
                        "ABONELİK"
                    };
                    let cta_lbl = if d.lang == "en" {
                        "Explore plan"
                    } else {
                        "Planı incele"
                    };
                    let contact_href = if d.lang == "en" {
                        "/kurumsal/iletisim?lang=en"
                    } else {
                        "/kurumsal/iletisim"
                    };
                    format!(
                        "<article class=\"{card_cls}\"><div class=\"eyebrow\">{eyebrow_lbl}</div><h3>{}</h3><div class=\"plan-price\">₺{:.0}</div><p>{}</p><ul>{}</ul><a class=\"{btn_cls}\" href=\"{contact_href}\" data-select-plan=\"{}\">{cta_lbl} {}</a></article>",
                        esc(localized(d.lang, &p.name_tr, &p.name_en)),
                        p.price_try,
                        esc(localized(d.lang, &p.period_label_tr, &p.period_label_en)),
                        (if d.lang == "en" { &p.features_en } else { &p.features_tr })
                            .iter()
                            .map(|f| format!("<li>{}</li>", esc(f)))
                            .collect::<String>(),
                        esc(&p.code),
                        icon("arrow")
                    )
                })
                .collect::<String>()
        ),
        SectionKind::Products => {
            let contact_href = if d.lang == "en" {
                "/kurumsal/iletisim?lang=en"
            } else {
                "/kurumsal/iletisim"
            };
            let prod_cta = if d.lang == "en" {
                "View publication"
            } else {
                "Yayını incele"
            };
            format!(
                "<div class=\"plan-grid\">{}</div>",
                d.products
                    .iter()
                    .filter(|p| p.is_available)
                    .take(s.limit)
                    .map(|p| format!(
                        "<article class=\"plan-card\"><img class=\"product-cover\" src=\"{}\" alt=\"{}\" loading=\"lazy\"><h3>{}</h3><p>{}</p><div class=\"plan-price\">₺{:.0}</div><a class=\"button secondary\" href=\"{}\" data-select-product=\"{}\">{}</a></article>",
                        asset(&p.cover_image),
                        esc(localized(d.lang, &p.title_tr, &p.title_en)),
                        esc(localized(d.lang, &p.title_tr, &p.title_en)),
                        esc(localized(d.lang, &p.description_tr, &p.description_en)),
                        p.price_try,
                        contact_href,
                        esc(&p.sku),
                        prod_cta
                    ))
                    .collect::<String>()
            )
        }
    };
    let kind_slug = match s.kind {
        SectionKind::Articles => "articles",
        SectionKind::Issues => "issues",
        SectionKind::Topics => "topics",
        SectionKind::Plans => "plans",
        SectionKind::Products => "products",
        SectionKind::Text => "text",
        SectionKind::MagazineHero => "hero",
        SectionKind::NewsHero => "news-hero",
        SectionKind::Banner => "banner",
        SectionKind::Cta => "cta",
    };
    let mut section = s.clone();
    if matches!(s.kind, SectionKind::Text) {
        section.description.clear();
    }
    format!(
        "<div class=\"section-band section-band-{kind_slug}\" data-band-index=\"{index}\"><section class=\"container section-space\" data-section=\"{index}\">{}{content}</section></div>",
        heading_localized(&section, d.lang)
    )
}

#[derive(Clone)]
pub struct RelatedStripOptions<'a> {
    pub lang: &'a str,
    pub is_current_sponsored: bool,
    pub exclude_ids: &'a [i64],
    pub limit: usize,
    pub tr: Option<&'a TranslationCache>,
    pub source_context: &'a str,
}

/// Renders the cross-page "İlginizi çekebilir" / "You may also like" strip (`B5`).
/// Strictly returns `""` when `opts.is_current_sponsored` is true or no eligible organic items exist.
pub fn render_related_strip(
    candidates: &[ContentItem],
    opts: &RelatedStripOptions<'_>,
) -> String {
    if opts.is_current_sponsored {
        return String::new();
    }
    let is_en = opts.lang.eq_ignore_ascii_case("en");
    let effective_limit = opts.limit.max(3);
    let mut seen = std::collections::HashSet::new();
    let mut selected: Vec<&ContentItem> = Vec::new();
    for c in candidates {
        if !is_published_content(c)
            || c.is_sponsored
            || c.content_type.eq_ignore_ascii_case("advertorial")
            || c.content_type.eq_ignore_ascii_case("advertorials")
            || c.content_type.eq_ignore_ascii_case("yeni-sayi-kapak")
            || opts.exclude_ids.contains(&c.id)
            || (is_en && !is_en_content_ready(c))
            || (!is_en && c.source_group.eq_ignore_ascii_case("msi-tdr"))
        {
            continue;
        }
        if seen.insert(c.id) {
            selected.push(c);
            if selected.len() >= effective_limit {
                break;
            }
        }
    }
    if selected.is_empty() {
        return String::new();
    }
    let eyebrow = tr_text(
        opts.tr,
        opts.lang,
        "reader.related_eyebrow",
        "OKUMAYA DEVAM EDİN",
        "CONTINUE READING",
    );
    let title = tr_text(
        opts.tr,
        opts.lang,
        "reader.related_title",
        "İlginizi çekebilir",
        "You may also like",
    );
    let explore_lbl = tr_text(
        opts.tr,
        opts.lang,
        "reader.explore_archive",
        "Tüm arşivi incele",
        "Explore full archive",
    );
    let search_href = lang_href("/arama", opts.lang);
    let safe_search_href = if safe_url(&search_href) {
        search_href.as_str()
    } else {
        "#"
    };
    let cards: String = selected
        .into_iter()
        .map(|c| article_card(c, opts.lang, false))
        .collect();

    let mut buf = HtmlBuf::with_capacity(2048);
    buf.el(
        "section",
        &[
            ("class", "container section-space related-strip"),
            ("id", "related-strip"),
            ("data-related-strip", ""),
            ("data-source-context", opts.source_context),
        ],
        |sec| {
            sec.el("div", &[("class", "section-heading")], |hd| {
                hd.el("div", &[], |left| {
                    left.el("div", &[("class", "eyebrow")], |eb| {
                        eb.text(&eyebrow);
                    });
                    left.el("h2", &[], |h2| {
                        h2.text(&title);
                    });
                });
                hd.el(
                    "a",
                    &[("class", "text-link"), ("href", safe_search_href)],
                    |a| {
                        a.text(&explore_lbl);
                        a.text(" ");
                        a.raw(&icon("arrow"));
                    },
                );
            });
            sec.el("div", &[("class", "stories related-strip-grid")], |grid| {
                grid.raw(&cards);
            });
        },
    );
    buf.into_string()
}

/// Renders the SSB-personalized feed section (`B4`) for signed-in readers.
/// Excludes sponsored items and exposes `[data-personalized-feed]` inside `#personalized-feed-band`
/// so JS hydration replaces only the cards container cleanly.
pub fn render_personalized_feed(
    items: &[ContentItem],
    user_interests: &[String],
    tags: &[SsbInterestTag],
    lang: &str,
    tr: Option<&TranslationCache>,
    limit: usize,
) -> String {
    let is_en = lang.eq_ignore_ascii_case("en");
    let effective_limit = limit.max(3);
    let interest_set: std::collections::HashSet<String> = user_interests
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let mut scored: Vec<(usize, &ContentItem)> = items
        .iter()
        .filter(|c| {
            is_published_content(c)
                && !c.is_sponsored
                && !c.content_type.eq_ignore_ascii_case("advertorial")
                && !c.content_type.eq_ignore_ascii_case("advertorials")
                && (!is_en || is_en_content_ready(c))
                && (is_en || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
        })
        .map(|c| {
            let matches = if interest_set.is_empty() {
                0
            } else {
                c.ssb_tags
                    .iter()
                    .filter(|t| interest_set.contains(&t.trim().to_ascii_lowercase()))
                    .count()
            };
            (matches, c)
        })
        .collect();

    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.published_at.cmp(&a.1.published_at))
            .then_with(|| b.1.id.cmp(&a.1.id))
    });

    let mut seen = std::collections::HashSet::new();
    let mut selected: Vec<&ContentItem> = Vec::new();
    for (_, c) in scored {
        if seen.insert(c.id) {
            selected.push(c);
            if selected.len() >= effective_limit {
                break;
            }
        }
    }

    let eyebrow = tr_text(
        tr,
        lang,
        "feed.personalized_eyebrow",
        "KİŞİSELLEŞTİRİLMİŞ AKIŞ",
        "PERSONALISED FEED",
    );
    let title = tr_text(
        tr,
        lang,
        "feed.personalized_title",
        "İlgi Alanlarınıza Özel",
        "Recommended for Your Interests",
    );
    let account_href = lang_href("/hesabim", lang);
    let manage_lbl = tr_text(
        tr,
        lang,
        "feed.manage_interests",
        "İlgi alanlarını düzenle",
        "Manage interests",
    );

    let chips_html: String = user_interests
        .iter()
        .filter(|code| !code.trim().is_empty())
        .map(|code| {
            let label = tags
                .iter()
                .find(|t| t.code.eq_ignore_ascii_case(code))
                .map(|t| localized(lang, &t.name_tr, &t.name_en).to_string())
                .unwrap_or_else(|| ssb_tag_display_label(code, lang));
            let search_url = lang_href(
                &format!("/arama?ssb_tag={}", urlencoding::encode(code)),
                lang,
            );
            format!(
                r#"<a class="topic-chip" href="{}">{}</a>"#,
                link(&search_url),
                esc(&label)
            )
        })
        .collect();

    let cards_html = if selected.is_empty() {
        let empty_msg = if user_interests.is_empty() {
            tr_text(
                tr,
                lang,
                "feed.empty_no_interests",
                "İlgi alanlarınıza özel öneriler için hesabınızdan SSB teknoloji etiketlerini seçin.",
                "Select SSB technology tags in your account to personalise your intelligence feed.",
            )
        } else {
            tr_text(
                tr,
                lang,
                "feed.empty_no_items",
                "Seçili ilgi alanlarınız için yakında yeni içerikler eklenecektir.",
                "New articles matching your selected interests will appear here soon.",
            )
        };
        format!(
            r#"<div class="empty-state">{} <a href="{}">{} →</a></div>"#,
            esc(&empty_msg),
            link(&account_href),
            esc(&manage_lbl)
        )
    } else {
        selected
            .into_iter()
            .map(|c| article_card(c, lang, false))
            .collect()
    };

    let limit_str = effective_limit.to_string();
    let mut buf = HtmlBuf::with_capacity(2048);
    buf.el(
        "section",
        &[
            ("id", "personalized-feed-band"),
            ("class", "container section-space personalized-band"),
            ("data-personalized-band", ""),
            ("data-endpoint", "/api/v1/public/personalized"),
        ],
        |sec| {
            sec.el("div", &[("class", "section-heading")], |hd| {
                hd.el("div", &[], |left| {
                    left.el("div", &[("class", "eyebrow")], |eb| {
                        eb.text(&eyebrow);
                    });
                    left.el("h2", &[], |h2| {
                        h2.text(&title);
                    });
                });
                hd.el(
                    "a",
                    &[
                        ("class", "text-link"),
                        (
                            "href",
                            if safe_url(&account_href) {
                                account_href.as_str()
                            } else {
                                "#"
                            },
                        ),
                    ],
                    |a| {
                        a.text(&manage_lbl);
                        a.text(" ");
                        a.raw(&icon("arrow"));
                    },
                );
            });
            if !chips_html.is_empty() {
                sec.el("div", &[("class", "personalized-tags-bar")], |bar| {
                    bar.el(
                        "div",
                        &[("class", "personalized-interest-chips topic-chips")],
                        |chips| {
                            chips.raw(&chips_html);
                        },
                    );
                });
            }
            sec.el(
                "div",
                &[
                    ("class", "stories personalized-grid"),
                    ("data-personalized-feed", ""),
                    ("data-limit", &limit_str),
                ],
                |grid| {
                    grid.raw(&cards_html);
                },
            );
        },
    );
    buf.into_string()
}

/// Converts a slice of `ContentItem` into structured `DossierTocEntry` rows (`B12`).
pub fn dossier_toc_from_contents<'a>(
    items: &'a [ContentItem],
    lang: &str,
) -> Vec<DossierTocEntry<'a>> {
    items
        .iter()
        .enumerate()
        .map(|(idx, c)| DossierTocEntry {
            index_label: format!("{:02}", idx + 1),
            badge: category_display_label(&c.content_type, lang).to_string(),
            title: localized(lang, &c.title_tr, &c.title_en),
            summary: localized(lang, &c.summary_tr, &c.summary_en),
            href: lang_href(&format!("/icerik/{}", c.slug), lang),
            read_time_min: Some(c.read_time_min.max(1)),
        })
        .collect()
}

/// Converts a slice of `MagazinePageSpread` into structured `DossierTocEntry` rows (`B12`).
pub fn dossier_toc_from_spreads<'a>(
    spreads: &'a [MagazinePageSpread],
    issue_number: i32,
    lang: &str,
) -> Vec<DossierTocEntry<'a>> {
    let is_en = lang.eq_ignore_ascii_case("en");
    spreads
        .iter()
        .enumerate()
        .map(|(idx, sp)| {
            let index_label = if sp.page_no.trim().is_empty() {
                format!("{:02}", idx + 1)
            } else if is_en {
                sp.page_no
                    .replacen("S. ", "P. ", 1)
                    .replacen("DOSYA ", "DOSSIER ", 1)
            } else {
                sp.page_no.clone()
            };
            let badge = if is_en {
                match sp.category_badge.trim().to_ascii_uppercase().as_str() {
                    "KAPAK DOSYASI" | "KAPAK DOSYALARI" => "COVER FILE".to_string(),
                    "SÖYLEŞİ" | "SÖYLEŞİLER" => "INTERVIEW".to_string(),
                    "ÖZEL DOSYA" | "ÖZEL DOSYALAR" => "SPECIAL DOSSIER".to_string(),
                    "ÖZEL HABER" | "ÖZEL HABERLER" => "EXCLUSIVE REPORT".to_string(),
                    "BAŞYAZI" | "BAŞYAZILAR" => "EDITORIAL".to_string(),
                    "MESAJ" | "MESAJLAR" => "EXECUTIVE MESSAGE".to_string(),
                    "TANITICI MAKALE" | "TANITICI MAKALELER" => "SPECIAL COVERAGE".to_string(),
                    "ANALİZ" | "MAKALE / ANALİZ" => "ANALYSIS".to_string(),
                    "HABER" | "HABERLER" => "NEWS".to_string(),
                    "YENİ SAYI" => "NEW ISSUE".to_string(),
                    "DOSYA" => "DOSSIER".to_string(),
                    other => other.to_string(),
                }
            } else {
                sp.category_badge.clone()
            };
            let href = sp
                .linked_content_slug
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|slug| lang_href(&format!("/icerik/{}", slug), lang))
                .unwrap_or_else(|| format!("#issue-{}-spread-{}", issue_number, idx + 1));
            DossierTocEntry {
                index_label,
                badge,
                title: localized(lang, &sp.title_tr, &sp.title_en),
                summary: localized(lang, &sp.excerpt_tr, &sp.excerpt_en),
                href,
                read_time_min: None,
            }
        })
        .collect()
}

/// Renders a structured Table of Contents block for special dossiers and issues (`B12`).
pub fn render_dossier_toc(
    heading_tr: &str,
    heading_en: &str,
    entries: &[DossierTocEntry<'_>],
    lang: &str,
    tr: Option<&TranslationCache>,
) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let is_en = lang.eq_ignore_ascii_case("en");
    let heading = if is_en && !heading_en.trim().is_empty() {
        heading_en
    } else {
        heading_tr
    };
    let aria_lbl = tr_text(
        tr,
        lang,
        "dossier.toc_aria",
        "Özel dosya içindekiler",
        "Special dossier table of contents",
    );
    let eyebrow = tr_text(
        tr,
        lang,
        "dossier.toc_eyebrow",
        "DOSYA İÇİNDEKİLER",
        "DOSSIER CONTENTS",
    );
    let sections_lbl = tr_text(tr, lang, "dossier.toc_sections", "bölüm", "sections");
    let rows: String = entries
        .iter()
        .map(|entry| dossier_toc_item(entry, lang))
        .collect();

    let mut buf = HtmlBuf::with_capacity(1536);
    buf.el(
        "nav",
        &[
            ("class", "dossier-toc"),
            ("id", "dossier-toc"),
            ("aria-label", &aria_lbl),
        ],
        |nav| {
            nav.el("div", &[("class", "dossier-toc-header")], |hd| {
                hd.el("div", &[], |left| {
                    left.el("span", &[("class", "eyebrow")], |eb| {
                        eb.text(&eyebrow);
                    });
                    left.el("h2", &[], |h2| {
                        h2.text(heading);
                    });
                });
                hd.el("span", &[("class", "dossier-toc-count")], |cnt| {
                    cnt.text(&format!("{} {}", entries.len(), sections_lbl));
                });
            });
            nav.el("ol", &[("class", "dossier-toc-list")], |list| {
                list.raw(&rows);
            });
        },
    );
    buf.into_string()
}

#[derive(Clone)]
pub struct EventsHubData<'a> {
    pub lang: &'a str,
    pub categories: &'a [CategoryNode],
    pub issues: &'a [MagazineIssue],
    pub contents: &'a [ContentItem],
    pub tr: Option<&'a TranslationCache>,
}

/// Renders the event track cards (`etkinlik-ozel-sayilari`, `etkinlik-sonrasi-ozel-dosyalar`)
/// for `/etkinlikler`.
pub fn render_event_tracks_grid(
    categories: &[CategoryNode],
    contents: &[ContentItem],
    lang: &str,
    tr: Option<&TranslationCache>,
) -> String {
    let is_en = lang.eq_ignore_ascii_case("en");
    let mut tracks: Vec<&CategoryNode> = categories
        .iter()
        .filter(|c| {
            c.parent_slug.as_deref() == Some("etkinlikler")
                || c.slug == "etkinlik-ozel-sayilari"
                || c.slug == "etkinlik-sonrasi-ozel-dosyalar"
        })
        .collect();
    tracks.sort_by_key(|c| c.sort_order);

    let filter_all_lbl = tr_text(tr, lang, "events.filter_all", "Tüm Etkinlikler", "All Events");
    let filter_special_lbl = tr_text(
        tr,
        lang,
        "events.filter_special",
        "Etkinlik Özel Sayıları",
        "Event Special Issues",
    );
    let filter_dossiers_lbl = tr_text(
        tr,
        lang,
        "events.filter_dossiers",
        "Etkinlik Sonrası Özel Dosyalar",
        "Post-Event Dossiers",
    );
    let track_status_lbl = tr_text(tr, lang, "events.track_badge", "ETKİNLİK DİZİNİ", "EVENT TRACK");
    let items_lbl = tr_text(tr, lang, "events.dossier_count", "dosya", "dossiers");
    let explore_lbl = tr_text(tr, lang, "events.explore_track", "Dosyaları incele", "Explore dossiers");

    let cards_html: String = tracks
        .iter()
        .map(|cat| {
            let count = contents
                .iter()
                .filter(|c| {
                    is_published_content(c)
                        && (!is_en || is_en_content_ready(c))
                        && (c.content_type == cat.slug
                            || c.placements.iter().any(|p| p == &cat.slug))
                })
                .count();
            let cat_href = lang_href(&format!("/kategori/{}", cat.slug), lang);
            let title = localized(lang, &cat.name_tr, &cat.name_en);
            let desc = localized(lang, &cat.description_tr, &cat.description_en);
            format!(
                r#"<a class="subdossier-card event-card" href="{href}"><div class="event-card-meta"><span class="event-status-pill">{badge}</span><span>{count} {items_lbl}</span></div><h3>{title}</h3><p>{desc}</p><span class="text-link">{explore_lbl} {arrow}</span></a>"#,
                href = link(&cat_href),
                badge = esc(&track_status_lbl),
                count = count,
                items_lbl = esc(&items_lbl),
                title = esc(title),
                desc = esc(desc),
                explore_lbl = esc(&explore_lbl),
                arrow = icon("arrow")
            )
        })
        .collect();

    if cards_html.is_empty() {
        return String::new();
    }

    let mut buf = HtmlBuf::with_capacity(1024);
    buf.el("div", &[("class", "events-hub-filters")], |bar| {
        bar.el(
            "a",
            &[("class", "topic-chip"), ("href", "#events-hub")],
            |a| {
                a.text(&filter_all_lbl);
            },
        );
        bar.el(
            "a",
            &[("class", "topic-chip"), ("href", "#events-special-issues")],
            |a| {
                a.text(&filter_special_lbl);
            },
        );
        bar.el(
            "a",
            &[("class", "topic-chip"), ("href", "#events-dossier-feed")],
            |a| {
                a.text(&filter_dossiers_lbl);
            },
        );
    });
    buf.el("div", &[("class", "subcategories-hub events-tracks")], |hub| {
        hub.el("div", &[("class", "events-hub-grid")], |grid| {
            grid.raw(&cards_html);
        });
    });
    buf.into_string()
}

/// Renders exhibition special issues (`is_special_issue || event_name.is_some()`) for `/etkinlikler`.
pub fn render_special_issues_band(
    issues: &[MagazineIssue],
    lang: &str,
    tr: Option<&TranslationCache>,
) -> String {
    let mut special: Vec<&MagazineIssue> = issues
        .iter()
        .filter(|i| i.is_special_issue || i.event_name.is_some())
        .collect();
    if special.is_empty() {
        return String::new();
    }
    special.sort_by_key(|i| std::cmp::Reverse(i.issue_number));

    let eyebrow = tr_text(
        tr,
        lang,
        "events.special_issues_eyebrow",
        "FUAR & ETKİNLİK ÖZEL SAYILARI",
        "EXHIBITION & EVENT SPECIAL ISSUES",
    );
    let title = tr_text(
        tr,
        lang,
        "events.special_issues_title",
        "Özel Sayılar ve Fuar Katalogları",
        "Special Issues & Exhibition Catalogues",
    );
    let cards: String = special
        .into_iter()
        .map(|i| issue_card(i, lang))
        .collect();

    let mut buf = HtmlBuf::with_capacity(1024);
    buf.el(
        "div",
        &[
            ("class", "events-special-issues"),
            ("id", "events-special-issues"),
        ],
        |sec| {
            sec.el("div", &[("class", "section-heading")], |hd| {
                hd.el("div", &[], |left| {
                    left.el("div", &[("class", "eyebrow")], |eb| {
                        eb.text(&eyebrow);
                    });
                    left.el("h2", &[], |h2| {
                        h2.text(&title);
                    });
                });
            });
            sec.el("div", &[("class", "issues-grid")], |grid| {
                grid.raw(&cards);
            });
        },
    );
    buf.into_string()
}

/// Renders the unified `/etkinlikler` Defence Events & Post-Event Dossiers Hub (`B12`).
pub fn render_events_hub(data: &EventsHubData<'_>) -> String {
    let is_en = data.lang.eq_ignore_ascii_case("en");
    let eyebrow = tr_text(
        data.tr,
        data.lang,
        "events.hub_eyebrow",
        "SAVUNMA FUARLARI VE SAHA DOSYALARI",
        "DEFENCE EXHIBITIONS & FIELD DOSSIERS",
    );
    let title = tr_text(
        data.tr,
        data.lang,
        "events.hub_title",
        "Etkinlikler ve Fuar Özel Dosyaları",
        "Defence Events & Post-Event Dossiers",
    );
    let desc = tr_text(
        data.tr,
        data.lang,
        "events.hub_desc",
        "Uluslararası savunma sanayii fuarlarına özel basılı sayılar, saha analizleri ve etkinlik sonrası derinlikli dosyalar.",
        "Special exhibition print issues, field briefings and comprehensive post-event defence dossiers.",
    );

    let tracks_html = render_event_tracks_grid(data.categories, data.contents, data.lang, data.tr);
    let special_issues_html = render_special_issues_band(data.issues, data.lang, data.tr);

    let mut dossier_items: Vec<&ContentItem> = data
        .contents
        .iter()
        .filter(|c| {
            let matches_track = matches!(
                c.content_type.as_str(),
                "etkinlikler" | "etkinlik-ozel-sayilari" | "etkinlik-sonrasi-ozel-dosyalar"
            ) || c.placements.iter().any(|p| {
                matches!(
                    p.as_str(),
                    "etkinlikler" | "etkinlik-ozel-sayilari" | "etkinlik-sonrasi-ozel-dosyalar"
                )
            });
            is_published_content(c)
                && !c.is_sponsored
                && !c.content_type.eq_ignore_ascii_case("advertorial")
                && !c.content_type.eq_ignore_ascii_case("advertorials")
                && matches_track
                && (!is_en || is_en_content_ready(c))
                && (is_en || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
        })
        .collect();
    dossier_items.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));

    let owned_dossiers: Vec<ContentItem> = dossier_items.into_iter().cloned().collect();
    let toc_entries = dossier_toc_from_contents(&owned_dossiers, data.lang);
    let toc_html = render_dossier_toc(
        "Etkinlik Dosyaları İçindekiler",
        "Event Dossiers Table of Contents",
        &toc_entries,
        data.lang,
        data.tr,
    );
    let feed_cards: String = owned_dossiers
        .iter()
        .map(|c| article_card(c, data.lang, false))
        .collect();

    let mut buf = HtmlBuf::with_capacity(4096);
    buf.el(
        "section",
        &[
            ("class", "container section-space events-hub"),
            ("id", "events-hub"),
        ],
        |sec| {
            sec.el("div", &[("class", "section-heading")], |hd| {
                hd.el("div", &[], |left| {
                    left.el("div", &[("class", "eyebrow")], |eb| {
                        eb.text(&eyebrow);
                    });
                    left.el("h1", &[], |h1| {
                        h1.text(&title);
                    });
                    left.el("p", &[], |p| {
                        p.text(&desc);
                    });
                });
            });
            if !tracks_html.is_empty() {
                sec.raw(&tracks_html);
            }
            if !special_issues_html.is_empty() {
                sec.raw(&special_issues_html);
            }
            if !owned_dossiers.is_empty() {
                sec.el(
                    "div",
                    &[("class", "events-dossier-feed"), ("id", "events-dossier-feed")],
                    |feed| {
                        feed.raw(&toc_html);
                        feed.el("div", &[("class", "stories layout-grid")], |grid| {
                            grid.raw(&feed_cards);
                        });
                    },
                );
            }
        },
    );
    buf.into_string()
}

/// Renders a lightweight public newsletter opt-in banner that opens `#newsletter-dialog` (`B7`).
pub fn render_newsletter_trigger_band(lang: &str, tr: Option<&TranslationCache>) -> String {
    let eyebrow = tr_text(
        tr,
        lang,
        "dialogs.newsletter.eyebrow",
        "MSI E-BÜLTEN",
        "MSI BRIEFINGS",
    );
    let title = tr_text(
        tr,
        lang,
        "dialogs.newsletter.band_title",
        "Savunma gündemini doğrudan e-postanızdan takip edin.",
        "Receive verified defence intelligence straight to your inbox.",
    );
    let desc = tr_text(
        tr,
        lang,
        "dialogs.newsletter.band_desc",
        "Yeni sayılar, özel savunma dosyaları ve uluslararası fuar raporları yayımlandığında ilk siz haberdar olun.",
        "Be the first to know when new magazine issues, special dossiers and international exhibition reports are published.",
    );
    let cta_lbl = tr_text(
        tr,
        lang,
        "dialogs.newsletter.band_cta",
        "Bültene abone ol",
        "Subscribe to briefings",
    );

    let mut buf = HtmlBuf::with_capacity(512);
    buf.el(
        "section",
        &[("class", "container section-space newsletter-cta-band")],
        |sec| {
            sec.el("div", &[("class", "membership-banner")], |banner| {
                banner.el("div", &[], |copy| {
                    copy.el("div", &[("class", "eyebrow")], |eb| {
                        eb.text(&eyebrow);
                    });
                    copy.el("h2", &[], |h2| {
                        h2.text(&title);
                    });
                    copy.el("p", &[], |p| {
                        p.text(&desc);
                    });
                });
                banner.el(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "button primary"),
                        ("data-open-dialog", "newsletter-dialog"),
                    ],
                    |btn| {
                        btn.text(&cta_lbl);
                        btn.text(" ");
                        btn.raw(&icon("arrow"));
                    },
                );
            });
        },
    );
    buf.into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item(id: i64, slug: &str, title_tr: &str, title_en: &str) -> ContentItem {
        ContentItem {
            id,
            slug: slug.into(),
            title_tr: title_tr.into(),
            title_en: title_en.into(),
            summary_tr: "Özet metin".into(),
            summary_en: "Summary text".into(),
            body_tr: "<p>Gövde</p>".into(),
            body_en: "<p>Body</p>".into(),
            content_type: "haberler".into(),
            source_group: "gunluk-haber".into(),
            cover_image: "images/example.webp".into(),
            published_at: "2026-11-10".into(),
            read_time_min: 4,
            status: "published".into(),
            en_status: "approved".into(),
            ..Default::default()
        }
    }

    #[test]
    fn related_strip_respects_sponsored_protection_and_minimum_items() {
        let mut sponsored = sample_item(5, "adv-1", "Sponsorlu", "Sponsored");
        sponsored.is_sponsored = true;
        sponsored.content_type = "advertorial".into();
        let mut plural_adv = sample_item(6, "adv-plural", "Reklam Dosyası", "Plural Advertorial");
        plural_adv.is_sponsored = false;
        plural_adv.content_type = "advertorials".into();
        // Place sponsored & plural_adv BEFORE org-1..3 so render_related_strip must actively
        // evaluate and exclude both before collecting the 3 organic items (effective_limit = 3).
        let items = vec![
            sample_item(1, "current-item", "Mevcut", "Current"),
            sponsored,
            plural_adv,
            sample_item(2, "org-1", "Organik 1", "Organic 1"),
            sample_item(3, "org-2", "Organik 2", "Organic 2"),
            sample_item(4, "org-3", "Organik 3", "Organic 3"),
        ];

        // Current article is sponsored -> must return empty string
        let empty_for_sponsored = render_related_strip(
            &items,
            &RelatedStripOptions {
                lang: "tr",
                is_current_sponsored: true,
                exclude_ids: &[1],
                limit: 3,
                tr: None,
                source_context: "article",
            },
        );
        assert!(empty_for_sponsored.is_empty());

        // Organic article -> excludes id 1 & sponsored id 5, renders 3 organic items
        let html = render_related_strip(
            &items,
            &RelatedStripOptions {
                lang: "en",
                is_current_sponsored: false,
                exclude_ids: &[1],
                limit: 2, // clamped to min 3
                tr: None,
                source_context: "search",
            },
        );
        assert!(html.contains("id=\"related-strip\""));
        assert!(html.contains("related-strip-grid"));
        assert!(html.contains("Organic 1"));
        assert!(html.contains("Organic 2"));
        assert!(html.contains("Organic 3"));
        assert!(!html.contains("Sponsored"));
        assert!(!html.contains("Plural Advertorial"));
        assert!(!html.contains("Current"));
    }

    #[test]
    fn personalized_feed_prioritizes_ssb_interests_and_exposes_hydration_hook() {
        let mut radar = sample_item(10, "radar-story", "Radar Haberi", "Radar Story");
        radar.ssb_tags = vec!["radarlar-ve-sensorler".into()];
        let mut air = sample_item(11, "air-story", "Hava Haberi", "Air Story");
        air.ssb_tags = vec!["hava-araclari".into()];
        let mut sponsored = sample_item(12, "sponsored-radar", "Reklam", "Ad");
        sponsored.ssb_tags = vec!["radarlar-ve-sensorler".into()];
        sponsored.is_sponsored = true;
        let mut plural_adv = sample_item(13, "adv-plural-feed", "Tanıtım", "Plural Adv Feed");
        plural_adv.ssb_tags = vec!["radarlar-ve-sensorler".into()];
        plural_adv.is_sponsored = false;
        plural_adv.content_type = "advertorials".into();

        let tags = vec![SsbInterestTag {
            code: "radarlar-ve-sensorler".into(),
            name_tr: "Radarlar ve Sensörler".into(),
            name_en: "AESA Radars & Sensors".into(),
            ..Default::default()
        }];
        let html = render_personalized_feed(
            &[air, radar, sponsored, plural_adv],
            &["radarlar-ve-sensorler".into()],
            &tags,
            "en",
            None,
            6,
        );
        assert!(html.contains("id=\"personalized-feed-band\""));
        assert!(html.contains("data-personalized-band"));
        assert!(html.contains("data-personalized-feed"));
        assert!(html.contains("data-limit=\"6\""));
        assert!(html.contains("AESA Radars &amp; Sensors"));
        assert!(!html.contains("sponsored-radar"));
        assert!(!html.contains("adv-plural-feed"));
        let radar_pos = html.find("Radar Story").unwrap();
        let air_pos = html.find("Air Story").unwrap();
        assert!(radar_pos < air_pos);
    }

    #[test]
    fn dossier_toc_and_events_hub_render_structured_sections() {
        let mut event_item = sample_item(21, "idef-report", "IDEF Analizi", "IDEF Analysis");
        event_item.content_type = "etkinlik-sonrasi-ozel-dosyalar".into();
        event_item.placements = vec!["etkinlikler".into()];
        let mut plural_adv_event =
            sample_item(22, "idef-adv", "IDEF Sponsorlu Dosya", "IDEF Plural Advertorial");
        plural_adv_event.is_sponsored = false;
        plural_adv_event.content_type = "advertorials".into();
        plural_adv_event.placements = vec!["etkinlikler".into()];

        let categories = vec![
            CategoryNode {
                slug: "etkinlik-ozel-sayilari".into(),
                parent_slug: Some("etkinlikler".into()),
                name_tr: "Etkinlik Özel Sayıları".into(),
                name_en: "Event Special Issues".into(),
                description_tr: "Fuar sayıları".into(),
                description_en: "Exhibition issues".into(),
                ..Default::default()
            },
            CategoryNode {
                slug: "etkinlik-sonrasi-ozel-dosyalar".into(),
                parent_slug: Some("etkinlikler".into()),
                name_tr: "Etkinlik Sonrası Özel Dosyalar".into(),
                name_en: "Post-Event Dossiers".into(),
                description_tr: "Saha dosyaları".into(),
                description_en: "Field dossiers".into(),
                ..Default::default()
            },
        ];
        let issues = vec![MagazineIssue {
            issue_number: 290,
            slug: "idef-ozel".into(),
            title_tr: "IDEF Özel Sayısı".into(),
            title_en: "IDEF Special Issue".into(),
            publish_month_tr: "Temmuz".into(),
            publish_month_en: "July".into(),
            year: 2026,
            cover_image: "images/example.webp".into(),
            is_special_issue: true,
            event_name: Some("IDEF".into()),
            ..Default::default()
        }];
        let contents = vec![plural_adv_event, event_item];
        let hub_html = render_events_hub(&EventsHubData {
            lang: "en",
            categories: &categories,
            issues: &issues,
            contents: &contents,
            tr: None,
        });
        assert!(hub_html.contains("id=\"events-hub\""));
        assert!(hub_html.contains("Event Special Issues"));
        assert!(hub_html.contains("Post-Event Dossiers"));
        assert!(hub_html.contains("IDEF Special Issue"));
        assert!(hub_html.contains("id=\"dossier-toc\""));
        assert!(hub_html.contains("IDEF Analysis"));
        assert!(!hub_html.contains("IDEF Plural Advertorial"));

        let spreads = vec![MagazinePageSpread {
            page_no: "S. 14".into(),
            title_tr: "Kapak".into(),
            title_en: "Cover Story".into(),
            category_badge: "KAPAK DOSYASI".into(),
            linked_content_slug: Some("cover-slug".into()),
            ..Default::default()
        }];
        let spread_toc = dossier_toc_from_spreads(&spreads, 290, "en");
        assert_eq!(spread_toc[0].index_label, "P. 14");
        assert_eq!(spread_toc[0].badge, "COVER FILE");
        assert_eq!(spread_toc[0].href, "/icerik/cover-slug?lang=en");

        let cta = render_newsletter_trigger_band("en", None);
        assert!(cta.contains("data-open-dialog=\"newsletter-dialog\""));
        assert!(!cta.contains("Defense"));
    }
}

