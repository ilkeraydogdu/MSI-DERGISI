use super::{
    components::{empty_state, enrich_issue_spreads, icon, link, localized, notification_item},
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
    render_with_nonce(
        lang, mode, settings, seo, categories, tags, issues, contents, notifications, body, _exec_us, None,
    )
}

/// Nonce-aware shell renderer. When `nonce` is provided, injects `nonce="..."` on all inline
/// `<script>` tags in the rendered HTML output for CSP script-src enforcement.
#[allow(clippy::too_many_arguments)]
pub fn render_with_nonce(
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
    nonce: Option<&str>,
) -> String {
    let design = config::load(settings);
    let nav: String = design
        .navigation
        .iter()
        .map(|n| format!("<a href=\"{}\">{}</a>", link(&n.href), esc(&n.label)))
        .collect();
    let canonical_editorial_skip = [
        "basyazi",
        "kapak-dosyalari",
        "ozel-dosyalar",
        "advertorial",
        "dergi-icerikleri",
        "etkinlikler",
        "etkinlik-ozel-sayilari",
        "etkinlik-sonrasi-ozel-dosyalar",
        "highlights",
        "feature",
    ];
    let mut seen_mega_labels = std::collections::HashSet::new();
    let mut editorial_mega_links = String::new();
    let mut dossier_mega_links = String::new();
    for c in categories {
        if canonical_editorial_skip.contains(&c.slug.as_str())
            || (lang == "en" && c.slug == "makaleler-analizler")
        {
            continue;
        }
        let raw_label = localized(lang, &c.name_tr, &c.name_en);
        let display_label = if c.slug == "ni" && raw_label.trim().eq_ignore_ascii_case("NI") {
            if lang == "en" {
                "NI Test & Measurement Dossier"
            } else {
                "NI Test ve Ölçüm Dosyası"
            }
        } else {
            raw_label
        };
        let norm_key = display_label.trim().to_lowercase();
        if !seen_mega_labels.insert(norm_key) {
            continue;
        }
        let lang_q = if lang == "en" { "?lang=en" } else { "" };
        let item_html = format!(
            "<a href=\"/kategori/{}{}\">{}</a>",
            esc(&c.slug),
            lang_q,
            esc(display_label)
        );
        if c.group_type == "etkinlik-dosyasi" {
            dossier_mega_links.push_str(&item_html);
        } else if c.group_type == "icerikler" || c.group_type == "gruplandirilmis" {
            editorial_mega_links.push_str(&item_html);
        }
    }
    let sec_lbl = if lang == "en" {
        "EDITORIAL SECTIONS"
    } else {
        "YAYIN BÖLÜMLERİ"
    };
    let dos_lbl = if lang == "en" {
        "EXHIBITION & SPECIAL DOSSIERS"
    } else {
        "FUAR, KONFERANS VE ÖZEL DOSYALAR"
    };
    let category_links = format!(
        "<div class=\"mega-section\"><div class=\"mega-section-title\">{sec_lbl}</div><div class=\"mega-links-grid\">{editorial_mega_links}</div></div><div class=\"mega-section\"><div class=\"mega-section-title\">{dos_lbl}</div><div class=\"mega-links-grid\">{dossier_mega_links}</div></div>"
    );
    let public_notifications: Vec<_> = notifications
        .iter()
        .filter(|n| {
            n.target_email.as_deref().unwrap_or("").is_empty()
                && ["all", "public", "global", ""].contains(&n.target_audience.as_str())
        })
        .collect();
    let notification_items: String = public_notifications
        .iter()
        .take(crate::db::constants::SHELL_NAV_CONTENT_LIMIT)
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
    let from_msi_lbl = if lang == "en" { "FROM MSI" } else { "MSI'DAN" };
    let dismiss_lbl = if lang == "en" {
        "Dismiss announcement"
    } else {
        "Duyuruyu kapat"
    };
    let announcement_html = announcement
        .map(|t| {
            format!(
                "<aside class=\"announcement\" id=\"site-announcement\"><div class=\"container announcement-inner\"><span><b>{from_msi_lbl}</b> {text}</span><button class=\"icon-button\" data-dismiss-announcement aria-label=\"{dismiss_lbl}\">{close}</button></div></aside>",
                from_msi_lbl = from_msi_lbl,
                text = esc(&t),
                dismiss_lbl = dismiss_lbl,
                close = icon("close")
            )
        })
        .unwrap_or_default();

    let lang_q = if lang == "en" { "?lang=en" } else { "" };
    let mut ticker_pool: Vec<&ContentItem> = contents
        .iter()
        .filter(|c| {
            c.content_type != "yeni-sayi-kapak"
                && c.status == "published"
                && (lang != "tr" || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
        })
        .collect();
    if lang == "en" {
        let en_ticker: Vec<&ContentItem> = ticker_pool
            .iter()
            .copied()
            .filter(|c| {
                c.source_group.eq_ignore_ascii_case("msi-tdr")
                    || (!c.title_en.trim().is_empty() && c.title_en != c.title_tr)
            })
            .collect();
        if !en_ticker.is_empty() {
            ticker_pool = en_ticker;
        }
    }
    ticker_pool.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));
    let ticker_html = if ticker_pool.is_empty() {
        String::new()
    } else {
        let primary_items: String = ticker_pool
            .iter()
            .take(crate::db::constants::SHELL_TRENDING_LIMIT)
            .enumerate()
            .map(|(idx, c)| {
                format!(
                    r#"<a class="ticker-item{active}" href="/icerik/{slug}{lang_q}" data-ticker-item="{idx}"><span class="ticker-sep" aria-hidden="true">◆</span>{title}</a>"#,
                    active = if idx == 0 { " is-active" } else { "" },
                    slug = esc(&c.slug),
                    lang_q = lang_q,
                    idx = idx,
                    title = esc(localized(lang, &c.title_tr, &c.title_en))
                )
            })
            .collect();
        let clone_items: String = ticker_pool
            .iter()
            .take(crate::db::constants::SHELL_LATEST_ISSUES_LIMIT)
            .map(|c| {
                format!(
                    r#"<a class="ticker-item is-clone" href="/icerik/{slug}{lang_q}" tabindex="-1" aria-hidden="true"><span class="ticker-sep" aria-hidden="true">◆</span>{title}</a>"#,
                    slug = esc(&c.slug),
                    lang_q = lang_q,
                    title = esc(localized(lang, &c.title_tr, &c.title_en))
                )
            })
            .collect();
        let badge_lbl = if lang == "en" { "LIVE NEWS" } else { "HABERLER" };
        let prev_lbl = if lang == "en" { "Previous headline" } else { "Önceki haber" };
        let next_lbl = if lang == "en" { "Next headline" } else { "Sonraki haber" };
        let ticker_aria = if lang == "en" { "Breaking news ticker" } else { "Son dakika haber kuşağı" };
        let ticker_href = format!("/kategori/haberler{}", lang_q);
        format!(
            r#"<div class="ticker-wrap"><div class="container"><div class="breaking-ticker is-marquee" data-news-ticker aria-label="{ticker_aria}"><a class="ticker-badge" href="{ticker_href}"><span class="ticker-pulse" aria-hidden="true"></span>{badge_lbl}</a><div class="ticker-controls"><button type="button" class="ticker-nav" data-ticker-prev aria-label="{prev_lbl}">‹</button><button type="button" class="ticker-nav" data-ticker-next aria-label="{next_lbl}">›</button></div><div class="ticker-viewport"><div class="ticker-track" data-ticker-track>{primary_items}{clone_items}</div></div></div></div></div>"#
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
    let enriched_issues: Vec<MagazineIssue> = issues
        .iter()
        .map(|iss| {
            let mut cloned = iss.clone();
            cloned.spreads = enrich_issue_spreads(iss, contents);
            cloned
        })
        .collect();
    let bootstrap = config::json_for_script(
        &serde_json::json!({
            "lang": lang,
            "mode": mode,
            "theme": design.theme.default_mode,
            "issues": enriched_issues,
            "tags": tags,
            "featuredIssue": settings.get("current_featured_issue").and_then(|v| v.parse::<i32>().ok())
        }),
    );
    let dialogs = localize_dialogs(include_str!("dialogs.html"), lang);
    let year_str = chrono::Utc::now().format("%Y").to_string();
    let home_href = if lang == "en" { "/en" } else { "/" };
    let search_href = if lang == "en" { "/en/search" } else { "/arama" };
    let mode_magazine_href = if lang == "en" { "/en?mode=magazine" } else { "/?mode=magazine" };
    let mode_news_href = if lang == "en" { "/en?mode=news" } else { "/?mode=news" };
    let edergi_href = if lang == "en" { "/e-dergi?lang=en" } else { "/e-dergi" };
    let footer_html = render_footer(lang, brand, &email, &phone, &fax, &address, &year_str);
    let html = format!(
        include_str!("shell.html"),
        announcement_html = announcement_html,
        ticker_html = ticker_html,
        nav = nav,
        category_links = category_links,
        body = body,
        footer_html = footer_html,
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
        home_href = home_href,
        search_href = search_href,
        search_aria = if lang == "en" { "Search" } else { "Ara" },
        theme_aria = if lang == "en" { "Toggle theme" } else { "Tema değiştir" },
        skip_link_label = if lang == "en" { "Skip to content" } else { "İçeriğe geç" },
        mode_magazine_href = mode_magazine_href,
        mode_news_href = mode_news_href,
        edergi_href = edergi_href,
        search = icon("search"),
        bell = icon("bell"),
        moon = icon("moon"),
        arrow = icon("arrow"),
        menu = icon("menu"),
        close = icon("close"),
        notification_count = public_notifications.len(),
        reader_center_label = if lang == "en" { "READER CENTER" } else { "OKUYUCU MERKEZİ" },
        notifications_label = if lang == "en" { "Notifications" } else { "Bildirimler" },
        mark_all_read_label = if lang == "en" { "Mark all as read" } else { "Tümünü okundu işaretle" },
        notification_content = if notification_items.is_empty() {
            empty_state(if lang == "en" {
                "No new notifications right now."
            } else {
                "Şu anda yeni bildirim yok."
            })
        } else {
            notification_items
        },
        other_lang = if lang == "en" { "tr" } else { "en" },
        other_lang_upper = if lang == "en" { "TR" } else { "EN" },
        active_mode = if mode == "news" { "news" } else { "magazine" },
        magazine_active = if mode == "news" { "" } else { "active" },
        news_active = if mode == "news" { "active" } else { "" },
        switch_magazine_label = if lang == "en" { "MAGAZINE" } else { "DERGİ" },
        switch_news_label = if lang == "en" { "NEWS" } else { "HABER" },
        edergi_label = if lang == "en" { "E-Magazine" } else { "E-Dergi" },
        more_topics_label = if lang == "en" { "All topics" } else { "Tüm konular" },
    );

    // Inject nonce attribute on all inline <script> tags when CSP nonce is provided.
    // External scripts (with src=) are unaffected; only inline scripts need the nonce.
    if let Some(n) = nonce {
        let nonce_attr = format!(" nonce=\"{}\"", n);
        html.replace("<script id=\"msi-bootstrap\"", &format!("<script{} id=\"msi-bootstrap\"", nonce_attr))
            .replace("<script type=\"speculationrules\">", &format!("<script{} type=\"speculationrules\">", nonce_attr))
                        .replace("<script src=\"/js/vendor/lenis.min.js\"", &format!("<script{} src=\"/js/vendor/lenis.min.js\"", nonce_attr))
            .replace("<script src=\"/js/vendor/gsap.min.js\"", &format!("<script{} src=\"/js/vendor/gsap.min.js\"", nonce_attr))
            .replace("<script src=\"/js/vendor/ScrollTrigger.min.js\"", &format!("<script{} src=\"/js/vendor/ScrollTrigger.min.js\"", nonce_attr))
            .replace("<script src=\"/js/msi-ui.js", &format!("<script{} src=\"/js/msi-ui.js", nonce_attr))
    } else {
        html
    }
}

fn render_footer(
    lang: &str,
    brand: &str,
    email: &str,
    phone: &str,
    fax: &str,
    address: &str,
    year: &str,
) -> String {
    let is_en = lang == "en";
    let home = if is_en { "/en" } else { "/" };
    let lq = if is_en { "?lang=en" } else { "" };
    let badge = if is_en { "REVIEW" } else { "DERGİSİ" };
    let tagline = if is_en {
        "Monthly Turkish & English Defence, Aerospace and Security Technologies Review · ISSN 1306-0309"
    } else {
        "Aylık, Türkçe ve İngilizce Yayımlanan Savunma, Havacılık ve Güvenlik Teknolojileri Dergisi · ISSN 1306-0309"
    };
    let h_contact = if is_en { "Contact &amp; Headquarters" } else { "İletişim &amp; Merkez Ofis" };
    let lbl_addr = if is_en { "Address:" } else { "Adres:" };
    let social_txt = if is_en {
        "Follow Us on Official Social Media Channels"
    } else {
        "Bizi Sosyal Medya Hesaplarımızdan Takip Edebilirsiniz"
    };
    let h_cats = if is_en { "Publication Sections" } else { "Yayın Kategorileri" };
    let h_about = if is_en { "About &amp; Masthead" } else { "Hakkımızda &amp; Künye" };
    let h_reader = if is_en { "Reader &amp; Privacy" } else { "Okuyucu &amp; Veri Koruma" };
    let cat_links = if is_en {
        format!(
            r#"<a href="/kategori/haberler{lq}">News</a><a href="/kategori/kapak-dosyasi{lq}">Cover Files</a><a href="/kategori/soylesiler{lq}">Interviews</a><a href="/kategori/ozel-haberler{lq}">Special Reports</a><a href="/kategori/basyazilar{lq}">Editorial Comments</a><a href="/kategori/tanitici-makaleler{lq}">Special Coverages</a><a href="/kategori/ozel-dosya{lq}">Special Dossiers</a><a href="/e-dergi{lq}">E-Magazine &amp; Archive</a>"#,
            lq = lq
        )
    } else {
        r#"<a href="/kategori/haberler">Haberler</a><a href="/kategori/kapak-dosyasi">Kapak Dosyaları</a><a href="/kategori/soylesiler">Söyleşiler</a><a href="/kategori/ozel-haberler">Özel Haberler</a><a href="/kategori/basyazilar">Başyazılar</a><a href="/kategori/tanitici-makaleler">Tanıtıcı Makaleler</a><a href="/kategori/ozel-dosya">Özel Dosya</a><a href="/e-dergi">E-Dergi &amp; Arşiv</a>"#.to_string()
    };
    let about_links = if is_en {
        format!(
            r#"<a href="/kurumsal/kunye{lq}">Masthead</a><a href="/kurumsal/hakkimizda{lq}">Who We Are (About)</a><a href="/kurumsal/yolculugumuz{lq}">Our Journey</a><a href="/kurumsal/cozum-ortaklarimiz{lq}">Solution Partners</a><a href="/kurumsal/amac-magazine{lq}">AMAC Magazine</a><a href="/kurumsal/kasabaworks{lq}">KasabaWorks</a><a href="/kurumsal/iletisim{lq}">Contact &amp; Advertising</a>"#,
            lq = lq
        )
    } else {
        r#"<a href="/kurumsal/kunye">Künye</a><a href="/kurumsal/hakkimizda">Biz Kimiz? (Hakkımızda)</a><a href="/kurumsal/yolculugumuz">Yolculuğumuz</a><a href="/kurumsal/cozum-ortaklarimiz">Çözüm Ortaklarımız</a><a href="/kurumsal/amac-magazine">AMAC Magazine</a><a href="/kurumsal/kasabaworks">KasabaWorks</a><a href="/kurumsal/iletisim">İletişim ve Reklam</a>"#.to_string()
    };
    let reader_links = if is_en {
        format!(
            r#"<a href="/hesabim{lq}">My Account</a><a href="/abonelik-ve-satis{lq}">Subscriptions &amp; Store</a><a href="/kurumsal/kvkk{lq}">KVKK Disclosure</a><a href="/kurumsal/kvkk-basvuru-formu{lq}">Data Request Form</a><a href="/kurumsal/gizlilik-politikasi{lq}">Privacy Policy</a><a href="/kurumsal/cerez-politikasi{lq}">Cookie Policy</a><a href="/rss.xml">RSS News Feed</a>"#,
            lq = lq
        )
    } else {
        r#"<a href="/hesabim">Hesabım</a><a href="/abonelik-ve-satis">Abonelik ve Yayınlar</a><a href="/kurumsal/kvkk">KVKK Aydınlatma Metni</a><a href="/kurumsal/kvkk-basvuru-formu">KVKK Başvuru Formu</a><a href="/kurumsal/gizlilik-politikasi">Gizlilik Politikası</a><a href="/kurumsal/cerez-politikasi">Çerez Politikası</a><a href="/rss.xml">RSS Haber Akışı</a>"#.to_string()
    };
    let legal_links = if is_en {
        format!(
            r#"<a href="/kurumsal/kullanim-kosullari{lq}">Terms of Use</a><a href="/kurumsal/mesafeli-satis-sozlesmesi{lq}">Distance Sales Agreement</a><a href="/kurumsal/iptal-ve-iade-kosullari{lq}">Cancellation &amp; Refund</a><a href="/kurumsal/ticari-ileti-aydinlatma{lq}">Commercial Messaging</a><a href="/kurumsal/telif-ve-etik{lq}">Copyright &amp; Ethics</a><a href="/kurumsal/duzeltme-ve-cevap-hakki{lq}">Correction &amp; Reply</a><a href="/kurumsal/reklam-ve-sponsorluk{lq}">Advertising Principles</a><button type="button" class="footer-cookie-btn" data-open-dialog="cookie-dialog">Cookie preferences</button>"#,
            lq = lq
        )
    } else {
        r#"<a href="/kurumsal/kullanim-kosullari">Kullanım Koşulları</a><a href="/kurumsal/mesafeli-satis-sozlesmesi">Mesafeli Satış Sözleşmesi</a><a href="/kurumsal/iptal-ve-iade-kosullari">İptal ve İade Koşulları</a><a href="/kurumsal/ticari-ileti-aydinlatma">Ticari Elektronik İleti (ETK/İYS)</a><a href="/kurumsal/telif-ve-etik">Telif ve Yayın Etiği</a><a href="/kurumsal/duzeltme-ve-cevap-hakki">Düzeltme ve Cevap Hakkı</a><a href="/kurumsal/reklam-ve-sponsorluk">Reklam ve Sponsorluk İlkeleri</a><button type="button" class="footer-cookie-btn" data-open-dialog="cookie-dialog">Çerez tercihleri</button>"#.to_string()
    };
    let copyright_txt = if is_en {
        format!("&copy; {year} UMSA Production Publishing Ltd. · All rights reserved (Law No. 5846).")
    } else {
        format!("&copy; {year} UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti. · Tüm hakları saklıdır (5846 s. FSEK).")
    };
    format!(
        r#"<footer class="site-footer">
      <div class="footer-brand-strip">
        <div class="container footer-brand-inner">
          <a href="{home}" class="wordmark" aria-label="{brand}">
            <span class="wordmark-brand">
              <img class="brand-logo" src="/images/logo-white.svg" alt="{brand}" width="108" height="40" loading="lazy" decoding="async" />
              <span class="wordmark-badge">{badge}</span>
            </span>
          </a>
          <span>{tagline}</span>
        </div>
      </div>
      <div class="container footer-grid">
        <div>
          <h2>{h_contact}</h2>
          <p class="footer-contact-list">
            <strong>{lbl_addr}</strong> {address}<br />
            <strong>P:</strong> <a href="tel:+903122254173">{phone}</a><br />
            <strong>F:</strong> {fax}<br />
            <strong>E:</strong> <a href="mailto:{email}">{email}</a><br />
            <a href="https://www.savunmahaber.com" rel="noopener noreferrer" target="_blank">www.savunmahaber.com</a>
          </p>
          <div class="footer-social-block">
            <span>{social_txt}</span>
            <div class="footer-social-links" aria-label="Official social media">
              <a href="https://www.facebook.com/MSI.Dergisi/" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi Facebook"><svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M13.5 21v-8h2.7l.4-3h-3.1V8.1c0-.9.3-1.5 1.6-1.5h1.7V3.9c-.3 0-1.3-.1-2.5-.1-2.5 0-4.2 1.5-4.2 4.3V10H7.3v3h2.8v8h3.4Z"/></svg></a>
              <a href="https://www.linkedin.com/company/msi-dergisi" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi LinkedIn"><svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M6.9 8.5a2 2 0 1 1 0-4 2 2 0 0 1 0 4ZM5.2 10h3.4v10.2H5.2V10Zm5.5 0h3.2v1.4h.1c.5-.8 1.6-1.7 3.3-1.7 3.5 0 4.1 2.3 4.1 5.3v5.2H18v-4.6c0-1.1 0-2.5-1.5-2.5s-1.8 1.2-1.8 2.4v4.7h-3.4V10Z"/></svg></a>
              <a href="https://www.instagram.com/msidergisi/" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi Instagram"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="5"/><circle cx="12" cy="12" r="4"/><circle cx="17.5" cy="6.5" r="1" fill="currentColor" stroke="none"/></svg></a>
              <a href="https://nsosyal.com/msidergisi" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi NSosyal"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="2"/><path d="M16.2 7.8a6 6 0 0 1 0 8.4m-8.4 0a6 6 0 0 1 0-8.4m11.2-2.8a10 10 0 0 1 0 14m-14 0a10 10 0 0 1 0-14"/></svg></a>
              <a href="https://twitter.com/MSI_Dergisi" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi X (Twitter)"><svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M18.2 3h2.9l-6.4 7.3L22.2 21h-5.9l-4.6-6-5.3 6H3.5l6.8-7.8L3.1 3h6l4.2 5.5L18.2 3Zm-1 16.2h1.6L8.3 4.7H6.6l10.6 14.5Z"/></svg></a>
              <a href="https://www.youtube.com/c/msidergisi" target="_blank" rel="noopener noreferrer" aria-label="MSI Dergisi YouTube"><svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M21.6 7.2a2.7 2.7 0 0 0-1.9-1.9C18 4.8 12 4.8 12 4.8s-6 0-7.7.5A2.7 2.7 0 0 0 2.4 7.2C2 8.9 2 12 2 12s0 3.1.4 4.8a2.7 2.7 0 0 0 1.9 1.9c1.7.5 7.7.5 7.7.5s6 0 7.7-.5a2.7 2.7 0 0 0 1.9-1.9c.4-1.7.4-4.8.4-4.8s0-3.1-.4-4.8ZM10 15.2V8.8l5.5 3.2-5.5 3.2Z"/></svg></a>
            </div>
          </div>
        </div>
        <div><h2>{h_cats}</h2>{cat_links}</div>
        <div><h2>{h_about}</h2>{about_links}</div>
        <div><h2>{h_reader}</h2>{reader_links}</div>
      </div>
      <nav class="container footer-legal-bar" aria-label="Legal notices">{legal_links}</nav>
      <div class="container footer-bottom">
        <span>{copyright_txt}</span>
        <span>MSI Dergisi · MSI Turkish Defence Review (ISSN 1306-0309)</span>
      </div>
    </footer>"#,
        home = home,
        brand = esc(brand),
        badge = badge,
        tagline = tagline,
        h_contact = h_contact,
        lbl_addr = lbl_addr,
        address = address,
        phone = phone,
        fax = fax,
        email = email,
        social_txt = social_txt,
        h_cats = h_cats,
        cat_links = cat_links,
        h_about = h_about,
        about_links = about_links,
        h_reader = h_reader,
        reader_links = reader_links,
        legal_links = legal_links,
        copyright_txt = copyright_txt,
    )
}

fn localize_dialogs(raw: &str, lang: &str) -> String {
    if lang != "en" {
        return raw.to_string();
    }
    raw.replace(
        "aria-label=\"Çerez tercih bildirimi\"",
        "aria-label=\"Cookie consent notice\"",
    )
    .replace("6698 SAYILI KVKK · ÇEREZ TERCİHLERİ", "PRIVACY · COOKIE PREFERENCES")
    .replace(
        "Zorunlu oturum ve güvenlik çerezleri dışındaki birinci taraf okuma analitiği",
        "First-party reading analytics and sponsorship measurement outside strictly necessary",
    )
    .replace(
        "ve sponsorluk ölçümlemesi yalnızca açık rızanızla etkinleştirilir. Detaylar:",
        "session and security cookies are enabled only with your explicit consent. Details:",
    )
    .replace(
        "<a href=\"/kurumsal/cerez-politikasi\">Çerez Politikası</a>.",
        "<a href=\"/kurumsal/cerez-politikasi?lang=en\">Cookie Policy</a>.",
    )
    .replace("Tümünü kabul et", "Accept all")
    .replace("Yalnızca zorunlu", "Necessary only")
    .replace("Tercihleri yönet", "Manage preferences")
    .replace(
        "6698 SAYILI KVKK &amp; ÇEREZ REHBERİ UYUMU",
        "PRIVACY &amp; COOKIE PREFERENCES",
    )
    .replace("Çerez ve Gizlilik Tercihleri", "Cookie &amp; Privacy Preferences")
    .replace(
        "MSI Dergisi platformunda yurt dışı merkezli üçüncü taraf reklam veya",
        "MSI Dergisi / MSI TDR does not use external third-party advertising or",
    )
    .replace(
        "profilleme takipçileri (Google Analytics, Meta Pixel vb.) kullanılmaz.",
        "behavioral profiling trackers.",
    )
    .replace(
        "Zorunlu olmayan birinci taraf ölçüm kategorilerini aşağıdan dilediğiniz an",
        "You can enable or disable optional first-party measurement categories below at any time (",
    )
    .replace("açıp kapatabilirsiniz (<a href=\"/kurumsal/cerez-politikasi\"", "<a href=\"/kurumsal/cerez-politikasi?lang=en\"")
    .replace(">Çerez Politikası</a\n    >).", ">Cookie Policy</a>).")
    .replace(
        "Zorunlu Oturum, Tema ve Güvenlik Çerezleri",
        "Strictly Necessary Session, Theme &amp; Security Cookies",
    )
    .replace(
        "HttpOnly güvenli oturum (<code>msi_session</code>), dil tercihi\n            (<code>msi_lang</code>), tema (<code>msi_theme</code>) ve çerez rıza\n            kaydı. KVKK m.5/2-(c) ve (f) uyarınca her zaman aktiftir.",
        "HttpOnly session cookie (<code>msi_session</code>), language preference (<code>msi_lang</code>), theme (<code>msi_theme</code>), and consent state. Always active.",
    )
    .replace(
        "Birinci Taraf Okuma Derinliği Analitiği (İsteğe Bağlı)",
        "First-Party Reading Analytics (Optional)",
    )
    .replace(
        "Makale okuma süresi ve sayfa kaydırma oranının anonimleştirilmiş\n            olarak ölçülmesini sağlar (KVKK m.5/1 açık rıza).",
        "Measures anonymized article reading depth and duration with your explicit consent.",
    )
    .replace(
        "Kurumsal Sponsorluk Gösterim Ölçümü (İsteğe Bağlı)",
        "Corporate Sponsorship Impression Measurement (Optional)",
    )
    .replace(
        "Savunma sanayii kurumsal tanıtım alanlarının gösterim ve tıklama\n            sayımını etkinleştirir; davranışsal profilleme yapılmaz (KVKK m.5/1\n            açık rıza).",
        "Enables impression and click counting for defence corporate sponsorships without behavioral profiling.",
    )
    .replace("aria-label=\"Kapat\"", "aria-label=\"Close\"")
    .replace("Seçimlerimi kaydet", "Save preferences")
}
