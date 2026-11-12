use super::components::{
    advertisement, article_card, asset, breadcrumb, category_display_label, editorial_sidebar,
    image_dimensions, localized,
};
use crate::{
    engine::security_engine::escape_html as esc,
    models::{AdCampaign, ContentItem},
};

pub fn article(
    item: &ContentItem,
    body: &str,
    locked: bool,
    reason: &str,
    related: &[ContentItem],
    ads: &[AdCampaign],
    lang: &str,
) -> String {
    let title = esc(localized(lang, &item.title_tr, &item.title_en));
    let sponsor = if item.is_sponsored {
        format!(
            "<div class=\"sponsor-notice\"><strong>{}</strong> · {}</div>",
            esc(item
                .sponsor_legal_label
                .as_deref()
                .unwrap_or("SPONSORLU İÇERİK")),
            esc(item.sponsor_brand.as_deref().unwrap_or(""))
        )
    } else {
        String::new()
    };
    let paywall = if locked {
        format!(
            r##"<aside class="paywall-panel"><div class="eyebrow">MSI OKUYUCU KULÜBÜ</div><h2>Hikâyenin tamamını keşfedin.</h2><p>{}</p><div class="hero-actions"><a href="#modal-login" class="button primary">Giriş yap / Hesap oluştur →</a><a href="/abonelik-ve-satis" class="button secondary">Abonelikleri incele</a></div></aside>"##,
            esc(reason)
        )
    } else {
        String::new()
    };
    let recommendations = if !item.is_sponsored && !related.is_empty() {
        format!(
            "<section class=\"container section-space\"><div class=\"section-heading\"><div><div class=\"eyebrow\">OKUMAYA DEVAM EDİN</div><h2>İlginizi çekebilir</h2></div></div><div class=\"stories\">{}</div></section>",
            related
                .iter()
                .take(crate::db::constants::READER_RELATED_LIMIT)
                .map(|c| article_card(c, lang, false))
                .collect::<String>()
        )
    } else {
        String::new()
    };
    let tags_html = if item.ssb_tags.is_empty() {
        String::new()
    } else {
        let chips: String = item
            .ssb_tags
            .iter()
            .map(|t| {
                format!(
                    "<a class=\"article-tag-pill\" href=\"/arama?ssb_tag={}\">#{}</a>",
                    esc(&urlencoding::encode(t)),
                    esc(&t.replace('-', " "))
                )
            })
            .collect();
        format!(
            r#"<div class="article-tags-bar" aria-label="Teknoloji etiketleri"><span class="eyebrow">TEKNOLOJİ ETİKETLERİ</span><div class="article-tag-pills">{chips}</div></div>"#
        )
    };
    let inline_pool: Vec<&AdCampaign> = ads
        .iter()
        .filter(|a| {
            a.is_active
                && !a.is_floating_popup
                && !a.slot_code.starts_with("ad-live-home-right")
                && !a.slot_code.starts_with("ad-live-home-")
        })
        .collect();
    let matched_inline: Vec<&AdCampaign> = inline_pool
        .iter()
        .copied()
        .filter(|a| {
            a.target_categories
                .iter()
                .any(|c| c == "*" || c == &item.content_type)
                || a.target_ssb_tags
                    .iter()
                    .any(|t| t == "*" || item.ssb_tags.contains(t))
        })
        .collect();
    let chosen_ad = if !matched_inline.is_empty() {
        Some(matched_inline[(item.id.unsigned_abs() as usize) % matched_inline.len()])
    } else if !inline_pool.is_empty() {
        Some(inline_pool[(item.id.unsigned_abs() as usize) % inline_pool.len()])
    } else {
        ads.iter()
            .find(|a| a.is_active && !a.slot_code.starts_with("ad-live-home-right"))
    };
    let inline_ad = chosen_ad
        .map(|a| advertisement(a, lang, "article-sponsor"))
        .unwrap_or_default();
    let safe_body = crate::engine::security_engine::sanitize_rich_html(body);
    let video_block = item
        .video_url
        .as_deref()
        .map(str::trim)
        .filter(|u| {
            !u.is_empty()
                && (u.starts_with("images/")
                    || u.starts_with("/images/")
                    || u.starts_with("https://"))
        })
        .map(|u| {
            if u.ends_with(".mp4") || u.ends_with(".webm") {
                format!(
                    r#"<figure class="article-video"><video controls preload="metadata" playsinline poster="{poster}" data-msi-video="{slug}"><source src="{src}" type="video/mp4"></video></figure>"#,
                    poster = asset(&item.cover_image),
                    slug = esc(&item.slug),
                    src = asset(u)
                )
            } else {
                String::new()
            }
        })
        .unwrap_or_default();

    let cover_src = asset(&item.cover_image);
    let (cw, ch) = image_dimensions(&item.cover_image).unwrap_or((900, 560));
    let is_portrait_or_compact = ch >= cw || cw <= 820;
    let cover_figure = if is_portrait_or_compact {
        format!(
            r#"<figure class="article-cover is-natural" style="--natural-w:{cw}px"><span class="article-cover-backdrop" style="background-image:url('{cover_src}')" aria-hidden="true"></span><img src="{cover_src}" alt="{title}" width="{cw}" height="{ch}" fetchpriority="high" decoding="async"></figure>"#,
            cw = cw,
            ch = ch,
            cover_src = cover_src,
            title = title
        )
    } else {
        format!(
            r#"<figure class="article-cover"><img src="{cover_src}" alt="{title}" width="{cw}" height="{ch}" fetchpriority="high" decoding="async"></figure>"#,
            cw = cw,
            ch = ch,
            cover_src = cover_src,
            title = title
        )
    };

    let cat_label = esc(&category_display_label(&item.content_type, lang).replace('-', " "));
    let sidebar = editorial_sidebar(&item.content_type, related, ads, lang);
    let breadcrumb_html = breadcrumb(
        if lang == "en" { "Home" } else { "Ana sayfa" },
        &item.content_type,
        &cat_label,
    );

    let author_slug = crate::controllers::web_controller::author_slug_from_name(&item.author_name);
    let author_html = if author_slug.is_empty() {
        format!("<span>{}</span>", esc(&item.author_name))
    } else {
        let lang_q = if lang == "en" { "?lang=en" } else { "" };
        format!(
            r#"<a href="/yazar/{slug}{lang_q}" class="author-byline-link">{name}</a>"#,
            slug = esc(&author_slug),
            lang_q = lang_q,
            name = esc(&item.author_name)
        )
    };
    let author_role_html = if item.author_title.trim().is_empty() {
        String::new()
    } else {
        format!(
            r#"<span class="author-byline-role">{}</span>"#,
            esc(item.author_title.trim())
        )
    };

    format!(
        r#"<div class="article-page" data-msi-content-slug="{slug}"><div class="container">{breadcrumb_html}<div class="article-master-grid"><div class="article-main-col"><div class="msi-section-ribbon"><a href="/kategori/{category}">{cat_label}</a></div><header class="article-header">{sponsor}<h1>{title}</h1><p class="article-summary">{summary}</p><div class="article-byline">{author_html}{author_role_html}<time datetime="{date}">{date}</time><span>{minutes} {min_lbl}</span><button class="text-link" data-share-article>{share_lbl} ↗</button></div></header>{cover_figure}{video_block}<article class="msi-article-body">{body}</article>{paywall}{tags_html}<div class="article-end"><span>MSI DERGİSİ · ISSN 1306-0309</span><a href="/e-dergi">{archive_lbl} →</a></div>{inline_ad}</div>{sidebar}</div></div>{recommendations}</div>"#,
        slug = esc(&item.slug),
        breadcrumb_html = breadcrumb_html,
        category = esc(&item.content_type),
        cat_label = cat_label,
        sponsor = sponsor,
        title = title,
        summary = esc(localized(lang, &item.summary_tr, &item.summary_en)),
        author_html = author_html,
        author_role_html = author_role_html,
        date = esc(item.published_at.get(..10).unwrap_or(&item.published_at)),
        minutes = item.read_time_min.max(1),
        min_lbl = if lang == "en" { "min read" } else { "dk okuma" },
        share_lbl = if lang == "en" { "Share link" } else { "Bağlantıyı paylaş" },
        cover_figure = cover_figure,
        video_block = video_block,
        body = safe_body,
        paywall = paywall,
        tags_html = tags_html,
        archive_lbl = if lang == "en" { "Digital E-Magazine Archive" } else { "Dijital E-Dergi Arşivi" },
        inline_ad = inline_ad,
        sidebar = sidebar,
        recommendations = recommendations
    )
}
