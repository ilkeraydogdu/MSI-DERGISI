use super::{
    components::{empty_state, icon, link, localized, notification_item},
    config,
};
use crate::engine::{security_engine::escape_html as esc, seo_router_engine};
use crate::models::*;
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn render(
    lang: &str,
    mode: &str,
    settings: &HashMap<String, String>,
    seo: &SeoMetaBundle,
    categories: &[CategoryNode],
    tags: &[SsbInterestTag],
    issues: &[MagazineIssue],
    contents: &[ContentItem],
    notifications: &[SystemNotification],
    body: &str,
    _exec_us: u128,
) -> String {
    let design = config::load(settings);
    let nav: String = design
        .navigation
        .iter()
        .map(|n| format!("<a href=\"{}\">{}</a>", link(&n.href), esc(&n.label)))
        .collect();
    let category_links: String = categories
        .iter()
        .filter(|c| {
            c.group_type == "icerikler"
                || c.group_type == "gruplandirilmis"
                || c.group_type == "etkinlik-dosyasi"
        })
        .map(|c| {
            format!(
                "<a href=\"/kategori/{}\">{}</a>",
                esc(&c.slug),
                esc(localized(lang, &c.name_tr, &c.name_en))
            )
        })
        .collect();
    let public_notifications: Vec<_> = notifications
        .iter()
        .filter(|n| {
            n.target_email.as_deref().unwrap_or("").is_empty()
                && ["all", "public", "global", ""].contains(&n.target_audience.as_str())
        })
        .collect();
    let notification_items: String = public_notifications
        .iter()
        .take(20)
        .map(|n| notification_item(n, lang))
        .collect();
    let announcement = settings
        .get("announcement_banner_text")
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .or_else(|| {
            public_notifications
                .iter()
                .find(|n| n.is_active_banner)
                .map(|n| localized(lang, &n.title_tr, &n.title_en).into())
        });
    let announcement_html = announcement
        .map(|t| {
            format!(
                "<aside class=\"announcement\" id=\"site-announcement\"><div class=\"container announcement-inner\"><span><b>MSI'DAN</b> {}</span><button class=\"icon-button\" data-dismiss-announcement aria-label=\"Duyuruyu kapat\">{}</button></div></aside>",
                esc(&t),
                icon("close")
            )
        })
        .unwrap_or_default();

    let mut ticker_pool: Vec<&ContentItem> = contents
        .iter()
        .filter(|c| c.content_type != "yeni-sayi-kapak")
        .collect();
    ticker_pool.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));
    let ticker_html = if ticker_pool.is_empty() {
        String::new()
    } else {
        let primary_items: String = ticker_pool
            .iter()
            .take(10)
            .enumerate()
            .map(|(idx, c)| {
                format!(
                    r#"<a class="ticker-item{active}" href="/icerik/{slug}" data-ticker-item="{idx}"><span class="ticker-sep" aria-hidden="true">◆</span>{title}</a>"#,
                    active = if idx == 0 { " is-active" } else { "" },
                    slug = esc(&c.slug),
                    idx = idx,
                    title = esc(localized(lang, &c.title_tr, &c.title_en))
                )
            })
            .collect();
        let clone_items: String = ticker_pool
            .iter()
            .take(10)
            .map(|c| {
                format!(
                    r#"<a class="ticker-item is-clone" href="/icerik/{slug}" tabindex="-1" aria-hidden="true"><span class="ticker-sep" aria-hidden="true">◆</span>{title}</a>"#,
                    slug = esc(&c.slug),
                    title = esc(localized(lang, &c.title_tr, &c.title_en))
                )
            })
            .collect();
        let badge_lbl = if lang == "en" { "LIVE NEWS" } else { "HABERLER" };
        let prev_lbl = if lang == "en" { "Previous headline" } else { "Önceki haber" };
        let next_lbl = if lang == "en" { "Next headline" } else { "Sonraki haber" };
        format!(
            r#"<div class="ticker-wrap"><div class="container"><div class="breaking-ticker is-marquee" data-news-ticker aria-label="Son dakika haber kuşağı"><a class="ticker-badge" href="/kategori/haberler"><span class="ticker-pulse" aria-hidden="true"></span>{badge_lbl}</a><div class="ticker-controls"><button type="button" class="ticker-nav" data-ticker-prev aria-label="{prev_lbl}">‹</button><button type="button" class="ticker-nav" data-ticker-next aria-label="{next_lbl}">›</button></div><div class="ticker-viewport"><div class="ticker-track" data-ticker-track>{primary_items}{clone_items}</div></div></div></div></div>"#
        )
    };

    let brand = settings
        .get("brand_title")
        .map(String::as_str)
        .unwrap_or("MSI Dergisi");
    let email = esc(settings
        .get("contact_email")
        .map(String::as_str)
        .unwrap_or("info@savunmahaber.com"));
    let phone = esc(settings
        .get("contact_phone")
        .map(String::as_str)
        .unwrap_or("+90 312 225 41 73"));
    let fax = esc(settings
        .get("contact_fax")
        .map(String::as_str)
        .unwrap_or("+90 312 225 41 74"));
    let address = esc(settings
        .get("contact_address")
        .map(String::as_str)
        .unwrap_or("Yaşamkent Mah. 3147 Sok. No:10 Çankaya / Ankara TÜRKİYE"));
    let bootstrap = config::json_for_script(
        &serde_json::json!({
            "lang": lang,
            "mode": mode,
            "theme": design.theme.default_mode,
            "issues": issues,
            "tags": tags,
            "featuredIssue": settings.get("current_featured_issue").and_then(|v| v.parse::<i32>().ok())
        }),
    );
    let dialogs = include_str!("dialogs.html");
    format!(
        include_str!("shell.html"),
        announcement_html = announcement_html,
        ticker_html = ticker_html,
        nav = nav,
        category_links = category_links,
        body = body,
        email = email,
        phone = phone,
        fax = fax,
        address = address,
        dialogs = dialogs,
        bootstrap = bootstrap,
        article_layout = design.templates.article_layout,
        category_layout = design.templates.category_layout,
        search_layout = design.templates.search_layout,
        reading_font = design.templates.reading_font,
        lang = esc(lang),
        initial_theme = if design.theme.default_mode == "dark" {
            "dark"
        } else {
            "light"
        },
        title = esc(&seo.title),
        seo = seo_router_engine::render_seo_head_tags(seo, None, &crate::engine::seo_router_engine::SeoBrandConfig::from_settings(settings)),
        background = esc(&design.theme.light.background),
        tokens = design.css(),
        body_class = if mode == "news" { "show-news" } else { "" },
        default_theme = design.theme.default_mode,
        brand = esc(brand),
        search = icon("search"),
        bell = icon("bell"),
        moon = icon("moon"),
        user = icon("user"),
        arrow = icon("arrow"),
        menu = icon("menu"),
        book = icon("book"),
        news_icon = icon("book"),
        close = icon("close"),
        notification_count = public_notifications.len(),
        notification_content = if notification_items.is_empty() {
            empty_state("Şu anda yeni bildirim yok.")
        } else {
            notification_items
        },
        other_lang = if lang == "en" { "tr" } else { "en" },
        other_lang_upper = if lang == "en" { "TR" } else { "EN" },
        magazine_active = if mode == "news" { "" } else { "active" },
        news_active = if mode == "news" { "active" } else { "" },
        year = chrono::Utc::now().format("%Y")
    )
}
