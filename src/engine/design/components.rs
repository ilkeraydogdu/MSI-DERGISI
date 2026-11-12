use super::config::{safe_url, Page, Section, SectionKind};
use crate::engine::security_engine::escape_html as esc;
use crate::models::*;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

pub struct PageData<'a> {
    pub lang: &'a str,
    pub contents: &'a [ContentItem],
    pub issues: &'a [MagazineIssue],
    pub tags: &'a [SsbInterestTag],
    pub ads: &'a [AdCampaign],
    pub plans: &'a [SubscriptionPlan],
    pub products: &'a [CommerceProduct],
    pub featured_issue: Option<i32>,
}

static IMAGE_DIM_CACHE: OnceLock<RwLock<HashMap<String, Option<(u32, u32)>>>> = OnceLock::new();

/// Fast memoized WebP header dimension reader (`VP8 `, `VP8L`, `VP8X`) for zero-CLS smart framing.
pub fn image_dimensions(rel_path: &str) -> Option<(u32, u32)> {
    let clean = rel_path.trim().trim_start_matches('/');
    if clean.is_empty() || clean.starts_with("http://") || clean.starts_with("https://") {
        return None;
    }
    let cache = IMAGE_DIM_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(guard) = cache.read() {
        if let Some(&cached) = guard.get(clean) {
            return cached;
        }
    }
    let parsed = read_webp_header(Path::new(clean));
    if let Ok(mut guard) = cache.write() {
        guard.insert(clean.to_string(), parsed);
    }
    parsed
}

fn read_webp_header(path: &Path) -> Option<(u32, u32)> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; 32];
    let n = file.read(&mut buf).ok()?;
    if n < 30 || &buf[0..4] != b"RIFF" || &buf[8..12] != b"WEBP" {
        return None;
    }
    match &buf[12..16] {
        b"VP8 " => {
            let w = (u16::from_le_bytes([buf[26], buf[27]]) & 0x3fff) as u32;
            let h = (u16::from_le_bytes([buf[28], buf[29]]) & 0x3fff) as u32;
            if w > 0 && h > 0 { Some((w, h)) } else { None }
        }
        b"VP8L" => {
            let bits = u32::from_le_bytes([buf[21], buf[22], buf[23], buf[24]]);
            let w = (bits & 0x3fff) + 1;
            let h = ((bits >> 14) & 0x3fff) + 1;
            if w > 0 && h > 0 { Some((w, h)) } else { None }
        }
        b"VP8X" => {
            let w = (buf[24] as u32 | ((buf[25] as u32) << 8) | ((buf[26] as u32) << 16)) + 1;
            let h = (buf[27] as u32 | ((buf[28] as u32) << 8) | ((buf[29] as u32) << 16)) + 1;
            if w > 0 && h > 0 { Some((w, h)) } else { None }
        }
        _ => None,
    }
}

pub fn localized<'a>(lang: &str, tr: &'a str, en: &'a str) -> &'a str {
    if lang == "en" && !en.is_empty() {
        en
    } else {
        tr
    }
}

pub fn category_display_label<'a>(slug: &'a str, lang: &str) -> &'a str {
    if lang == "en" {
        match slug {
            "haberler" => "NEWS",
            "kapak-dosyasi" | "kapak-dosyalari" => "COVER STORY",
            "soylesiler" => "INTERVIEW",
            "ozel-haberler" => "EXCLUSIVE REPORT",
            "basyazilar" | "basyazi" => "EDITORIAL",
            "mesajlar" => "EXECUTIVE MESSAGE",
            "tanitici-makaleler" => "SPECIAL COVERAGE",
            "makaleler-analizler" => "ANALYSIS",
            "advertorials" | "advertorial" => "ADVERTORIAL",
            "ozel-dosya" | "ozel-dosyalar" => "SPECIAL DOSSIER",
            "yeni-sayi-kapak" => "NEW ISSUE",
            _ => slug,
        }
    } else {
        match slug {
            "haberler" => "HABERLER",
            "kapak-dosyasi" | "kapak-dosyalari" => "KAPAK DOSYASI",
            "soylesiler" => "SÖYLEŞİ",
            "ozel-haberler" => "ÖZEL HABER",
            "basyazilar" | "basyazi" => "BAŞYAZI",
            "mesajlar" => "MESAJ",
            "tanitici-makaleler" => "TANITICI MAKALE",
            "makaleler-analizler" => "MAKALE / ANALİZ",
            "advertorials" | "advertorial" => "ADVERTORIAL",
            "ozel-dosya" | "ozel-dosyalar" => "ÖZEL DOSYA",
            "yeni-sayi-kapak" => "YENİ SAYI",
            "tskdan-haberler" => "TSK'DAN HABERLER",
            _ => slug,
        }
    }
}

pub fn asset(path: &str) -> String {
    if path.starts_with("https://") {
        esc(path)
    } else {
        format!("/{}", esc(path.trim_start_matches('/')))
    }
}
pub fn link(url: &str) -> String {
    if safe_url(url) {
        esc(url)
    } else {
        "#".into()
    }
}
pub fn icon(name: &str) -> String {
    let path = match name {
        "search" => "<circle cx='10.5' cy='10.5' r='6.5'/><path d='m16 16 5 5'/>",
        "bell" => "<path d='M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4'/>",
        "moon" => "<path d='M21 13a9 9 0 0 1-10-10 9 9 0 1 0 10 10Z'/>",
        "menu" => "<path d='M3 6h18M3 12h18M3 18h18'/>",
        "user" => "<circle cx='12' cy='8' r='4'/><path d='M4 21v-2a8 8 0 0 1 16 0v2'/>",
        "arrow" => "<path d='M4 12h16m-6-6 6 6-6 6'/>",
        "close" => "<path d='m6 6 12 12M6 18 18 6'/>",
        _ => "<path d='M3 3h7l2 2 2-2h7v17h-7l-2 2-2-2H3Zm9 2v17'/>",
    };
    format!("<svg class=\"ui-icon\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.6\" aria-hidden=\"true\">{path}</svg>")
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
    let (w, h) = image_dimensions(&c.cover_image).unwrap_or((800, 500));
    let is_portrait = h >= w;
    let media_html = if is_portrait {
        format!(
            r#"<a class="story-image is-portrait" href="/icerik/{slug}" tabindex="-1" aria-hidden="true"><span class="story-image-backdrop" style="background-image:url('{image}')" aria-hidden="true"></span><img src="{image}" alt="{title}" loading="{loading}" decoding="async" width="{w}" height="{h}"></a>"#,
            slug = esc(&c.slug),
            image = image,
            title = title,
            loading = if eager { "eager" } else { "lazy" },
            w = w,
            h = h
        )
    } else {
        format!(
            r#"<a class="story-image" href="/icerik/{slug}" tabindex="-1" aria-hidden="true"><img src="{image}" alt="{title}" loading="{loading}" decoding="async" width="{w}" height="{h}"></a>"#,
            slug = esc(&c.slug),
            image = image,
            title = title,
            loading = if eager { "eager" } else { "lazy" },
            w = w,
            h = h
        )
    };
    format!(
        r#"<article class="story-card" data-ssb-tags="{tags}">{media_html}<div class="story-copy"><div class="eyebrow">{badge}</div><h3><a href="/icerik/{slug}">{title}</a></h3><p>{summary}</p><div class="story-meta"><time datetime="{date}">{date}</time><span>{minutes} {min_label}</span></div></div></article>"#,
        tags = esc(&c.ssb_tags.join(",")),
        media_html = media_html,
        slug = esc(&c.slug),
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
    format!(
        r##"<article class="issue-card"><button class="issue-cover" data-open-issue="{number}" aria-label="{number}. sayıyı incele"><img src="{image}" alt="{title}" loading="lazy" decoding="async" width="{w}" height="{h}"></button><div class="eyebrow">SAYI {number} · {month} {year}</div><h3>{title}</h3><button class="text-link" data-open-issue="{number}">{cta} {arrow}</button></article>"##,
        number = i.issue_number,
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
    if let Some(img_path) = ad.image_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let (w, h) = image_dimensions(img_path).unwrap_or((900, 300));
        let is_graphic_banner = img_path.contains("msi-dergisi-reklam-");
        if is_graphic_banner {
            return format!(
                r#"<aside class="advertisement ad-visual-banner" id="{id}" data-ad-id="{ad_id}" aria-label="Reklam"><a class="ad-banner-frame" href="{href}" data-ad-click-id="{ad_id}"><span class="ad-corner-tag">REKLAM</span><img src="{img}" alt="{sponsor} — {title}" loading="lazy" decoding="async" width="{w}" height="{h}"></a></aside>"#,
                id = esc(id),
                ad_id = ad.id,
                href = link(&ad.target_url),
                img = asset(img_path),
                sponsor = esc(&ad.sponsor_name),
                title = esc(localized(lang, &ad.title_tr, &ad.title_en)),
                w = w,
                h = h
            );
        }
        return format!(
            r#"<aside class="advertisement ad-hybrid" id="{id}" data-ad-id="{ad_id}" aria-label="Reklam"><div class="ad-disclosure">REKLAM · {sponsor}<a href="/kurumsal/reklam-ve-sponsorluk">Reklam ve iş birliği</a></div><a class="ad-content ad-content-media" href="{href}" data-ad-click-id="{ad_id}"><span class="ad-thumb"><img src="{img}" alt="{title}" loading="lazy" decoding="async" width="{w}" height="{h}"></span><div class="ad-copy"><span class="eyebrow">{badge}</span><h3>{title}</h3><p>{desc}</p></div><span class="ad-arrow">{arrow}</span></a></aside>"#,
            id = esc(id),
            ad_id = ad.id,
            sponsor = esc(&ad.sponsor_name),
            href = link(&ad.target_url),
            img = asset(img_path),
            title = esc(localized(lang, &ad.title_tr, &ad.title_en)),
            w = w,
            h = h,
            badge = esc(&ad.badge_label),
            desc = esc(localized(lang, &ad.desc_tr, &ad.desc_en)),
            arrow = icon("arrow")
        );
    }
    format!(
        r#"<aside class="advertisement" id="{}" data-ad-id="{}" aria-label="Reklam"><div class="ad-disclosure">REKLAM · {}<a href="/kurumsal/reklam-ve-sponsorluk">Reklam ve iş birliği</a></div><a class="ad-content" href="{}" data-ad-click-id="{}"><div><span class="eyebrow">{}</span><h3>{}</h3><p>{}</p></div><span class="ad-arrow">{}</span></a></aside>"#,
        esc(id),
        ad.id,
        esc(&ad.sponsor_name),
        link(&ad.target_url),
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

    let nav_html: String = dir_items
        .iter()
        .map(|(slug, tr_lbl, en_lbl)| {
            let is_active = active_category.eq_ignore_ascii_case(slug)
                || (*slug == "kapak-dosyasi" && active_category == "kapak-dosyalari")
                || (*slug == "basyazilar" && active_category == "basyazi")
                || (*slug == "advertorials" && active_category == "advertorial")
                || (*slug == "ozel-dosya" && active_category == "ozel-dosyalar");
            format!(
                r#"<a class="sidebar-cat-btn{}" href="/kategori/{}">{}</a>"#,
                if is_active { " is-active" } else { "" },
                slug,
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
            .take(5)
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
                    r#"<article class="sidebar-rel-card"><a class="sidebar-rel-thumb{portrait_cls}" href="/icerik/{slug}">{backdrop}<img src="{img}" alt="{t}" loading="lazy" decoding="async" width="{w}" height="{h}"></a><h4><a href="/icerik/{slug}">{t}</a></h4><time>{date}</time></article>"#,
                    portrait_cls = portrait_cls,
                    slug = esc(&c.slug),
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

    let tower_ad = ads
        .iter()
        .find(|a| a.is_active && a.slot_code.starts_with("ad-live-home-right"))
        .or_else(|| ads.iter().find(|a| a.is_active && a.image_url.is_some()))
        .map(|a| {
            let img = a
                .image_url
                .as_deref()
                .unwrap_or(crate::services::media_service::DEFAULT_AD_SIDEBAR);
            let (w, h) = image_dimensions(img).unwrap_or((533, 1600));
            format!(
                r#"<div class="sidebar-widget sidebar-tower-ad" data-ad-id="{id}"><a class="ad-banner-frame is-vertical" href="{href}" data-ad-click-id="{id}"><span class="ad-corner-tag">REKLAM</span><img src="{src}" alt="{sponsor} — {title}" loading="lazy" decoding="async" width="{w}" height="{h}"></a></div>"#,
                id = a.id,
                href = link(&a.target_url),
                src = asset(img),
                sponsor = esc(&a.sponsor_name),
                title = esc(localized(lang, &a.title_tr, &a.title_en)),
                w = w,
                h = h
            )
        })
        .unwrap_or_default();

    format!(
        r#"<aside class="editorial-sidebar" aria-label="Yayın kategorileri ve öneriler"><nav class="sidebar-cat-nav" aria-label="Bölüm dizini">{nav_html}</nav>{related_html}{tower_ad}</aside>"#
    )
}

/// Centralized notification item — used by both shell.rs (public header) and account.rs (user dashboard).
pub fn notification_item(n: &SystemNotification, lang: &str) -> String {
    let action = n
        .action_url
        .as_ref()
        .filter(|u| safe_url(u))
        .map(|u| format!("<a class=\"text-link\" href=\"{}\">İncele →</a>", link(u)))
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

/// Centralized empty-state placeholder.
pub fn empty_state(message: &str) -> String {
    format!("<div class=\"empty-state\">{}</div>", message)
}

/// Centralized page heading with eyebrow + title + optional description.
pub fn page_heading(eyebrow: &str, title: &str, description: &str) -> String {
    let desc = if description.is_empty() {
        String::new()
    } else {
        format!("<p>{}</p>", esc(description))
    };
    format!(
        "<div class=\"page-heading\"><div class=\"eyebrow\">{}</div><h1>{}</h1>{}</div>",
        esc(eyebrow),
        esc(title),
        desc
    )
}

/// Centralized breadcrumb navigation for article/reader pages.
pub fn breadcrumb(home_label: &str, category_slug: &str, category_label: &str) -> String {
    format!(
        "<nav class=\"breadcrumb-line\" aria-label=\"İçerik yolu\"><a href=\"/\">{}</a><span>/</span><a href=\"/kategori/{}\">{}</a></nav>",
        esc(home_label),
        esc(category_slug),
        esc(category_label)
    )
}

fn heading(s: &Section) -> String {
    let cta = if !s.link.is_empty() {
        format!(
            "<a class=\"text-link\" href=\"{}\">{} {}</a>",
            link(&s.link),
            esc(&s.link_label),
            icon("arrow")
        )
    } else {
        String::new()
    };
    format!(
        "<div class=\"section-heading\"><div><div class=\"eyebrow\">{}</div><h2>{}</h2>{}</div>{}</div>",
        esc(&s.eyebrow),
        esc(&s.title),
        if s.description.is_empty() {
            String::new()
        } else {
            format!("<p>{}</p>", esc(&s.description))
        },
        cta
    )
}

#[cfg(test)]
fn select<'a>(s: &Section, data: &'a PageData<'_>) -> Vec<&'a ContentItem> {
    select_for_page(s, data, &mut std::collections::HashSet::new())
}

fn select_for_page<'a>(
    s: &Section,
    data: &'a PageData<'_>,
    seen_ids: &mut std::collections::HashSet<i64>,
) -> Vec<&'a ContentItem> {
    let mut items: Vec<_> = data
        .contents
        .iter()
        .filter(|c| {
            (s.category == "yeni-sayi-kapak" || c.content_type != "yeni-sayi-kapak")
                && (s.source == "all"
                    || s.source == "magazine" && c.source_group == "dergi-icerigi"
                    || s.source == "news" && c.source_group != "dergi-icerigi")
                && (s.category.is_empty()
                    || c.content_type == s.category
                    || c.placements.contains(&s.category)
                    || c.ssb_tags.contains(&s.category))
        })
        .collect();
    items.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));
    let chosen: Vec<&'a ContentItem> = items
        .into_iter()
        .skip(s.offset)
        .filter(|c| !seen_ids.contains(&c.id))
        .take(s.limit)
        .collect();
    for item in &chosen {
        seen_ids.insert(item.id);
    }
    chosen
}

pub fn render_page(page: &Page, data: &PageData<'_>) -> String {
    let mut seen_ids = std::collections::HashSet::new();
    let sections: String = page
        .sections
        .iter()
        .filter(|s| s.enabled)
        .enumerate()
        .map(|(i, s)| render_section(s, data, i, &mut seen_ids))
        .collect();
    if page
        .sections
        .iter()
        .filter(|s| s.enabled)
        .any(|s| matches!(s.kind, SectionKind::MagazineHero | SectionKind::NewsHero))
    {
        sections
    } else {
        format!(
            "<header class=\"container page-heading section-space\"><div class=\"eyebrow\">MSI DERGİSİ</div><h1>{}</h1><p>{}</p></header>{}",
            esc(&page.title),
            esc(&page.description),
            sections
        )
    }
}

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

fn render_section(
    s: &Section,
    d: &PageData<'_>,
    index: usize,
    seen_ids: &mut std::collections::HashSet<i64>,
) -> String {
    let content = match s.kind {
        SectionKind::MagazineHero => {
            let issue = d
                .featured_issue
                .and_then(|n| d.issues.iter().find(|i| i.issue_number == n))
                .or_else(|| d.issues.iter().max_by_key(|i| i.issue_number));
            let Some(i) = issue else {
                return format!(
                    "<section class=\"container section-space\">{}<p>Yeni sayı yakında burada.</p></section>",
                    heading(s)
                );
            };
            let preview_spreads: Vec<_> = i.spreads.iter().take(4).collect();
            let spread_tabs = if preview_spreads.is_empty() {
                String::new()
            } else {
                format!(
                    "<div class=\"stage-tabs\" role=\"tablist\" aria-label=\"Sayı içi özel dosyalar\">{}</div>",
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
            let live_dossier = if let Some(first_sp) = preview_spreads.first() {
                let sp_title = esc(localized(d.lang, &first_sp.title_tr, &first_sp.title_en));
                let sp_excerpt = esc(localized(d.lang, &first_sp.excerpt_tr, &first_sp.excerpt_en));
                let sp_link = first_sp
                    .linked_content_slug
                    .as_deref()
                    .filter(|slug| !slug.is_empty())
                    .map(|slug| {
                        format!(
                            "<a class=\"dossier-link\" data-dossier-link href=\"/icerik/{}\">Dosyayı oku →</a>",
                            esc(slug)
                        )
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "<button type=\"button\" class=\"dossier-link\" data-open-issue=\"{}\">Sayıda aç →</button>",
                            i.issue_number
                        )
                    });
                format!(
                    r#"<div class="hero-live-dossier" data-live-dossier aria-live="polite"><span class="dossier-progress" data-dossier-progress aria-hidden="true"></span><div class="dossier-meta"><span class="dossier-badge" data-dossier-badge>{badge}</span><span class="dossier-pages" data-dossier-pages>{page_no}</span></div><strong class="dossier-title" data-dossier-title>{sp_title}</strong><p class="dossier-excerpt" data-dossier-excerpt>{sp_excerpt}</p><div class="dossier-actions">{sp_link}<button type="button" class="dossier-next" data-hero-next-spread>Sonraki dosya ↻</button></div></div>"#,
                    badge = esc(&first_sp.category_badge),
                    page_no = esc(&first_sp.page_no),
                    sp_title = sp_title,
                    sp_excerpt = sp_excerpt,
                    sp_link = sp_link
                )
            } else {
                String::new()
            };
            let (cw, ch) = image_dimensions(&i.cover_image).unwrap_or((960, 1200));
            return format!(
                r#"<section class="magazine-hero container" data-hero-section><div class="hero-copy"><div class="eyebrow"><span class="accent-rule"></span>{eyebrow}</div><h1>{title}</h1><p class="hero-description">{description}</p><div class="hero-actions"><button class="button primary" data-open-issue="{number}">{number}. sayıyı canlı incele {arrow}</button><a class="text-link" href="{link}">{link_label} {arrow}</a></div><div class="hero-metrics" aria-label="Yayın göstergeleri"><div><strong>22+ Yıl</strong><span>KESİNTİSİZ OTORİTE</span></div><div><strong>{number}+ Sayı</strong><span>STRATEJİK HAFIZA</span></div><div><strong>TR · EN</strong><span>KÜRESEL REFERANS</span></div></div><div class="hero-footnote"><span>BAĞIMSIZ SAVUNMA İSTİHBARATI</span><span>SAVUNMA · HAVACILIK · UZAY · DENİZCİLİK</span></div></div><div class="hero-cover-stage" data-hero-stage data-issue-number="{number}"><div class="stage-topbar"><span class="stage-live-pill"><span class="live-dot" aria-hidden="true"></span>CANLI DİJİTAL KİOSK · SAYI {number}</span>{spread_tabs}</div><span class="cover-index" aria-hidden="true">{number}<small>REFERANS SAYI</small></span><div class="hero-mockup-3d" data-tilt-card><button class="hero-cover" data-open-issue="{number}" aria-label="{issue_title}"><img src="{cover}" alt="{issue_title}" width="{cw}" height="{ch}" fetchpriority="high"><span class="cover-sheen" aria-hidden="true"></span></button></div>{live_dossier}<div class="cover-caption"><span>{month} {year} · ISSN 1306-0309</span><span>{pages} sayfa · {dossiers} özel dosya</span></div></div></section>"#,
                eyebrow = esc(&s.eyebrow),
                title = esc(&s.title).replace('\n', "<br>"),
                description = esc(&s.description),
                number = i.issue_number,
                arrow = icon("arrow"),
                link = link(&s.link),
                link_label = esc(&s.link_label),
                spread_tabs = spread_tabs,
                issue_title = esc(localized(d.lang, &i.title_tr, &i.title_en)),
                cover = asset(&i.cover_image),
                cw = cw,
                ch = ch,
                live_dossier = live_dossier,
                month = esc(localized(d.lang, &i.publish_month_tr, &i.publish_month_en)),
                year = i.year,
                pages = i.page_count,
                dossiers = i.dossier_count
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
            let cards: String = select_for_page(s, d, seen_ids)
                .iter()
                .map(|c| article_card(c, d.lang, false))
                .collect();
            if cards.is_empty() {
                "<div class=\"empty-state\">Bu bölümde henüz içerik yok. <a href=\"/arama\">Arşivi keşfedin →</a></div>".into()
            } else {
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
        SectionKind::Topics => format!(
            "<div class=\"topics-grid\">{}</div>",
            d.tags
                .iter()
                .take(s.limit)
                .enumerate()
                .map(|(i, t)| format!(
                    "<a href=\"/arama?ssb_tag={}\"><span class=\"topic-index\">{:02}</span><span>{}</span>{}</a>",
                    esc(&urlencoding::encode(&t.code)),
                    i + 1,
                    esc(localized(d.lang, &t.name_tr, &t.name_en)),
                    icon("arrow")
                ))
                .collect::<String>()
        ),
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
                .map(|p| format!(
                    "<article class=\"plan-card\"><div class=\"eyebrow\">{}</div><h3>{}</h3><div class=\"plan-price\">₺{:.0}</div><p>{}</p><ul>{}</ul><a class=\"button primary\" href=\"/kurumsal/iletisim\" data-select-plan=\"{}\">Planı incele {}</a></article>",
                    if p.is_featured { "ÖNERİLEN PLAN" } else { "ABONELİK" },
                    esc(localized(d.lang, &p.name_tr, &p.name_en)),
                    p.price_try,
                    esc(localized(d.lang, &p.period_label_tr, &p.period_label_en)),
                    (if d.lang == "en" { &p.features_en } else { &p.features_tr })
                        .iter()
                        .map(|f| format!("<li>{}</li>", esc(f)))
                        .collect::<String>(),
                    esc(&p.code),
                    icon("arrow")
                ))
                .collect::<String>()
        ),
        SectionKind::Products => format!(
            "<div class=\"plan-grid\">{}</div>",
            d.products
                .iter()
                .filter(|p| p.is_available)
                .take(s.limit)
                .map(|p| format!(
                    "<article class=\"plan-card\"><img class=\"product-cover\" src=\"{}\" alt=\"{}\" loading=\"lazy\"><h3>{}</h3><p>{}</p><div class=\"plan-price\">₺{:.0}</div><a class=\"button secondary\" href=\"/kurumsal/iletisim\" data-select-product=\"{}\">Yayını incele</a></article>",
                    asset(&p.cover_image),
                    esc(localized(d.lang, &p.title_tr, &p.title_en)),
                    esc(localized(d.lang, &p.title_tr, &p.title_en)),
                    esc(localized(d.lang, &p.description_tr, &p.description_en)),
                    p.price_try,
                    esc(&p.sku)
                ))
                .collect::<String>()
        ),
    };
    let mut section = s.clone();
    if matches!(s.kind, SectionKind::Text) {
        section.description.clear();
    }
    format!(
        "<section class=\"container section-space\" data-section=\"{index}\">{}{content}</section>",
        heading(&section)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn content(source: &str, slug: &str) -> ContentItem {
        ContentItem {
            id: 1,
            slug: slug.into(),
            title_tr: "<script>alert(1)</script>".into(),
            title_en: String::new(),
            summary_tr: "A & B".into(),
            summary_en: String::new(),
            body_tr: String::new(),
            body_en: String::new(),
            content_type: "haberler".into(),
            source_group: source.into(),
            cover_image: "images/example.webp".into(),
            author_name: String::new(),
            author_title: String::new(),
            issue_number: None,
            published_at: "2026-11-08".into(),
            read_time_min: 3,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: false,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: false,
            video_url: None,
            view_count: 0,
            true_read_count: 0,
            placements: vec![],
            ssb_tags: vec![],
        }
    }
    #[test]
    fn cards_escape_editorial_strings() {
        let html = article_card(&content("gunluk-haber", "safe-slug"), "tr", false);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("A &amp; B"));
    }
    #[test]
    fn page_sections_filter_and_order_without_unrelated_fallback() {
        let contents = vec![
            content("gunluk-haber", "news"),
            content("dergi-icerigi", "magazine"),
        ];
        let data = PageData {
            lang: "tr",
            contents: &contents,
            issues: &[],
            tags: &[],
            ads: &[],
            plans: &[],
            products: &[],
            featured_issue: None,
        };
        let mut section = Section {
            source: "magazine".into(),
            ..Section::default()
        };
        assert_eq!(select(&section, &data)[0].slug, "magazine");
        section.source = "news".into();
        assert_eq!(select(&section, &data)[0].slug, "news");
        section.category = "nonexistent".into();
        assert!(select(&section, &data).is_empty());
        section.category.clear();
        section.offset = 1;
        assert!(select(&section, &data).is_empty());
        section.offset = 0;
        section.enabled = false;
        let page = Page {
            slug: "test".into(),
            title: "Test".into(),
            description: String::new(),
            mode: "news".into(),
            sections: vec![section],
        };
        assert!(!render_page(&page, &data).contains("story-card"));
    }
    #[test]
    fn custom_text_is_safe_and_rendered_once() {
        let data = PageData {
            lang: "tr",
            contents: &[],
            issues: &[],
            tags: &[],
            ads: &[],
            plans: &[],
            products: &[],
            featured_issue: None,
        };
        let section = Section {
            kind: SectionKind::Text,
            title: "Text".into(),
            description: "Unique <img onerror=alert(1)>".into(),
            ..Section::default()
        };
        let page = Page {
            slug: "test".into(),
            title: "Title".into(),
            description: String::new(),
            mode: "magazine".into(),
            sections: vec![section],
        };
        let html = render_page(&page, &data);
        assert_eq!(html.matches("Unique").count(), 1);
        assert!(!html.contains("<img onerror"));
        assert_eq!(html.matches("<h1>").count(), 1);
    }
    #[test]
    fn parses_real_webp_dimensions_and_frames_portrait_cards() {
        let dims = image_dimensions("images/msi/msi-dergisi-sayi-286-kapak.webp");
        assert_eq!(dims, Some((960, 1200)));
        let mut portrait = content("dergi-icerigi", "portrait-card");
        portrait.cover_image = "images/msi/msi-dergisi-sayi-286-kapak.webp".into();
        let card_html = article_card(&portrait, "tr", true);
        assert!(card_html.contains("story-image is-portrait"));
        assert!(card_html.contains("story-image-backdrop"));
        assert!(card_html.contains("width=\"960\" height=\"1200\""));
    }
    #[test]
    fn editorial_sidebar_highlights_active_category() {
        let html = editorial_sidebar("ozel-dosya", &[content("gunluk-haber", "rel-1")], &[], "tr");
        assert!(html.contains("sidebar-cat-btn is-active\" href=\"/kategori/ozel-dosya\""));
        assert!(html.contains("İLGİNİZİ ÇEKEBİLİR"));
    }
    #[test]
    fn cross_section_deduplication_prevents_repeated_cards_on_same_page() {
        let mut first = content("dergi-icerigi", "shared-slug");
        first.id = 10;
        first.content_type = "kapak-dosyasi".into();
        first.placements = vec!["soylesiler".into()];
        let mut second = content("dergi-icerigi", "interview-only");
        second.id = 11;
        second.content_type = "soylesiler".into();
        let contents = vec![first, second];
        let data = PageData {
            lang: "tr",
            contents: &contents,
            issues: &[],
            tags: &[],
            ads: &[],
            plans: &[],
            products: &[],
            featured_issue: None,
        };
        let page = Page {
            slug: "mag".into(),
            title: "Mag".into(),
            description: String::new(),
            mode: "magazine".into(),
            sections: vec![
                Section {
                    kind: SectionKind::Articles,
                    source: "all".into(),
                    category: "kapak-dosyasi".into(),
                    limit: 3,
                    enabled: true,
                    ..Section::default()
                },
                Section {
                    kind: SectionKind::Articles,
                    source: "all".into(),
                    category: "soylesiler".into(),
                    limit: 3,
                    enabled: true,
                    ..Section::default()
                },
            ],
        };
        let html = render_page(&page, &data);
        assert_eq!(html.matches("/icerik/shared-slug\"").count(), 2); // 1 image link + 1 title link = 1 card
        assert_eq!(html.matches("/icerik/interview-only\"").count(), 2);
    }
}
