use crate::engine::security_engine::escape_html as esc;
use crate::models::*;
use super::super::config::safe_url;
use super::labels::{category_display_label, localized};
use super::media::image_dimensions;
use super::primitives::{asset, icon, link};

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
    let (w, h) = image_dimensions(&c.cover_image).unwrap_or((800, 500));
    let is_portrait = h >= w;
    let article_href = if lang == "en" {
        format!("/icerik/{}?lang=en", esc(&c.slug))
    } else {
        format!("/icerik/{}", esc(&c.slug))
    };
    let media_html = if is_portrait {
        format!(
            r#"<a class="story-image is-portrait" href="{href}" tabindex="-1" aria-hidden="true"><span class="story-image-backdrop" style="background-image:url('{image}')" aria-hidden="true"></span><img src="{image}" alt="{title}" loading="{loading}" decoding="async" width="{w}" height="{h}"></a>"#,
            href = article_href,
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
    format!(
        r#"<article class="story-card" data-ssb-tags="{tags}">{media_html}<div class="story-copy"><div class="eyebrow">{badge}</div><h3><a href="{href}">{title}</a></h3><p>{summary}</p><div class="story-meta"><time datetime="{date}">{date}</time><span>{minutes} {min_label}</span></div></div></article>"#,
        tags = esc(&c.ssb_tags.join(",")),
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
    format!(
        r##"<article class="issue-card"><button class="issue-cover" data-open-issue="{number}" aria-label="{aria_lbl}"><img src="{image}" alt="{title}" loading="lazy" decoding="async" width="{w}" height="{h}"></button><div class="eyebrow">{eyebrow_lbl} {number} · {month} {year}</div><h3>{title}</h3><button class="text-link" data-open-issue="{number}">{cta} {arrow}</button></article>"##,
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
                r#"<aside class="advertisement ad-visual-banner" id="{id}" data-ad-id="{ad_id}" aria-label="{aria_ad}"><a class="ad-banner-frame" href="{href}" data-ad-click-id="{ad_id}"><span class="ad-corner-tag">{corner_tag}</span><img src="{img}" alt="{sponsor} — {title}" loading="lazy" decoding="async" width="{w}" height="{h}"></a></aside>"#,
                id = esc(id),
                ad_id = ad.id,
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
            r#"<aside class="advertisement ad-hybrid" id="{id}" data-ad-id="{ad_id}" aria-label="{aria_ad}"><div class="ad-disclosure"><span>{disc_lbl} · <strong>{sponsor}</strong></span><a href="/kurumsal/reklam-ve-sponsorluk{lang_q}">{collab_lbl}</a></div><a class="ad-content ad-content-media" href="{href}" data-ad-click-id="{ad_id}"><span class="ad-thumb"><img src="{img}" alt="{title}" loading="lazy" decoding="async" width="{w}" height="{h}"></span><div class="ad-copy"><span class="eyebrow">{badge}</span><h3>{title}</h3><p>{desc}</p></div><span class="ad-cta">{cta_lbl} {arrow}</span></a></aside>"#,
            id = esc(id),
            ad_id = ad.id,
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
        r#"<aside class="advertisement" id="{}" data-ad-id="{}" aria-label="{}"><div class="ad-disclosure">{} · {}<a href="/kurumsal/reklam-ve-sponsorluk{}">{}</a></div><a class="ad-content" href="{}" data-ad-click-id="{}"><div><span class="eyebrow">{}</span><h3>{}</h3><p>{}</p></div><span class="ad-arrow">{}</span></a></aside>"#,
        esc(id),
        ad.id,
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
                let (w, h) = image_dimensions(&c.cover_image).unwrap_or((480, 300));
                let date = esc(c.published_at.get(..10).unwrap_or(&c.published_at));
                let portrait_cls = if h >= w { " is-portrait" } else { "" };
                let backdrop = if h >= w {
                    format!(
                        r#"<span class="story-image-backdrop" style="background-image:url('{img}')" aria-hidden="true"></span>"#
                    )
                } else {
                    String::new()
                };
                format!(
                    r#"<article class="sidebar-rel-card"><a class="sidebar-rel-thumb{portrait_cls}" href="/icerik/{slug}{lang_q}">{backdrop}<img src="{img}" alt="{t}" loading="lazy" decoding="async" width="{w}" height="{h}"></a><h4><a href="/icerik/{slug}{lang_q}">{t}</a></h4><time>{date}</time></article>"#,
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
        ads.iter().find(|a| a.is_active && a.image_url.is_some())
    };
    let tower_tag = if lang == "en" { "AD" } else { "REKLAM" };
    let tower_ad = selected_tower
        .map(|a| {
            let img = a
                .image_url
                .as_deref()
                .unwrap_or(crate::services::media_service::DEFAULT_AD_SIDEBAR);
            let (w, h) = image_dimensions(img).unwrap_or((533, 1600));
            format!(
                r#"<div class="sidebar-widget sidebar-tower-ad" data-ad-id="{id}"><a class="ad-banner-frame is-vertical" href="{href}" data-ad-click-id="{id}"><span class="ad-corner-tag">{tower_tag}</span><img src="{src}" alt="{sponsor} — {title}" loading="lazy" decoding="async" width="{w}" height="{h}"></a></div>"#,
                id = a.id,
                href = link(&a.target_url),
                tower_tag = tower_tag,
                src = asset(img),
                sponsor = esc(&a.sponsor_name),
                title = esc(localized(lang, &a.title_tr, &a.title_en)),
                w = w,
                h = h
            )
        })
        .unwrap_or_default();

    format!(
        r#"<aside class="editorial-sidebar" aria-label="Editorial directory"><nav class="sidebar-cat-nav" aria-label="Sections">{nav_html}</nav>{related_html}{tower_ad}</aside>"#
    )
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
