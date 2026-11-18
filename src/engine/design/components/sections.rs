use super::super::config::{Section, SectionKind};
use super::PageData;
use super::cards::{advertisement, article_card, issue_card};
use super::filters::enrich_issue_spreads;
use super::labels::localized;
use super::layout::heading;
use super::media::image_dimensions;
use super::primitives::{asset, icon, link};
use super::select::select_for_page;
use crate::engine::security_engine::escape_html as esc;
use crate::models::*;

fn render_ad_strip(d: &PageData<'_>, slots: &[&str], index: usize) -> Option<String> {
    let matched: Vec<&AdCampaign> = slots
        .iter()
        .filter_map(|code| d.ads.iter().find(|a| a.slot_code == *code && a.is_active))
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
                return format!(
                    "<section class=\"container section-space\">{}<p>Yeni sayı yakında burada.</p></section>",
                    heading(s)
                );
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
                            "<button type=\"button\" class=\"dossier-link\" data-open-issue=\"{}\">{open_lbl}</button>",
                            i.issue_number
                        )
                    });
                let display_badge = if d.lang == "en" {
                    match first_sp.category_badge.trim().to_ascii_uppercase().as_str() {
                        "KAPAK DOSYASI" => "COVER FILE".to_string(),
                        "SÖYLEŞİ" => "INTERVIEW".to_string(),
                        "ÖZEL DOSYA" => "SPECIAL DOSSIER".to_string(),
                        "ANALİZ" => "ANALYSIS".to_string(),
                        "DOSYA" => "DOSSIER".to_string(),
                        other => other.to_string(),
                    }
                } else {
                    first_sp.category_badge.clone()
                };
                let display_pages = if d.lang == "en" {
                    first_sp.page_no.replacen("S. ", "P. ", 1)
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
            let (metrics_aria, m1_val, m1_lbl, m2_val, m2_lbl, m3_lbl, f1, f2, live_pill, ref_lbl, cover_stats) =
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
                        format!("LIVE DIGITAL KIOSK · ISSUE {}", i.issue_number),
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
                        format!("CANLI DİJİTAL KİOSK · SAYI {}", i.issue_number),
                        "REFERANS SAYI",
                        format!("{} sayfa · {} özel dosya", i.page_count, i.dossier_count),
                    )
                };
            return format!(
                r#"<div class="hero-stage-band"><section class="magazine-hero container" data-hero-section><div class="hero-copy"><div class="eyebrow"><span class="accent-rule"></span>{eyebrow}</div><h1>{title}</h1><p class="hero-description">{description}</p><div class="hero-actions"><button class="button primary" data-open-issue="{number}">{open_cta} {arrow}</button><a class="text-link" href="{link}">{link_label} {arrow}</a></div><div class="hero-metrics" aria-label="{metrics_aria}"><div><strong>{m1_val}</strong><span>{m1_lbl}</span></div><div><strong>{m2_val}</strong><span>{m2_lbl}</span></div><div><strong>TR · EN</strong><span>{m3_lbl}</span></div></div><div class="hero-footnote"><span>{f1}</span><span>{f2}</span></div></div><div class="hero-cover-stage" data-hero-stage data-issue-number="{number}"><div class="stage-topbar"><span class="stage-live-pill"><span class="live-dot" aria-hidden="true"></span>{live_pill}</span>{spread_tabs}</div><span class="cover-index" aria-hidden="true">{number}<small>{ref_lbl}</small></span><div class="hero-mockup-3d" data-tilt-card><button class="hero-cover" data-open-issue="{number}" aria-label="{issue_title}"><img src="{cover}" alt="{issue_title}" width="{cw}" height="{ch}" fetchpriority="high"><span class="cover-sheen" aria-hidden="true"></span></button></div>{live_dossier}<div class="cover-caption"><span>{month} {year} · ISSN 1306-0309</span><span>{cover_stats}</span></div></div></section></div>"#,
                eyebrow = esc(&s.eyebrow),
                title = esc(&s.title).replace('\n', "<br>"),
                description = esc(&s.description),
                number = i.issue_number,
                open_cta = open_cta,
                arrow = icon("arrow"),
                link = link(&s.link),
                link_label = esc(&s.link_label),
                metrics_aria = metrics_aria,
                m1_val = m1_val,
                m1_lbl = m1_lbl,
                m2_val = m2_val,
                m2_lbl = m2_lbl,
                m3_lbl = m3_lbl,
                f1 = f1,
                f2 = f2,
                live_pill = live_pill,
                spread_tabs = spread_tabs,
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
                esc(&s.eyebrow),
                esc(&s.title),
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
            return d
                .ads
                .iter()
                .find(|a| a.slot_code == s.category && a.is_active)
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
            esc(&s.description)
                .split("\n\n")
                .map(|p| format!("<p>{}</p>", p.replace('\n', "<br>")))
                .collect::<String>()
        ),
        SectionKind::Cta => {
            return format!(
                "<section class=\"container section-space\"><div class=\"membership-banner\"><div><div class=\"eyebrow\">{}</div><h2>{}</h2><p>{}</p></div><a class=\"button primary\" href=\"{}\">{} {}</a></div></section>",
                esc(&s.eyebrow),
                esc(&s.title),
                esc(&s.description),
                link(&s.link),
                esc(&s.link_label),
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
        heading(&section)
    )
}

