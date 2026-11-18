use super::components::{
    advertisement, article_card, asset, breadcrumb_localized, category_display_label,
    editorial_sidebar, image_dimensions, localized, ssb_tag_display_label,
};
use super::labels_helper::{is_en_content_ready, lang_href, tr_text};
use crate::{
    engine::{
        security_engine::escape_html as esc,
        translation_engine::TranslationCache,
    },
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
    article_with_translations(item, body, locked, reason, related, ads, lang, None)
}

#[allow(clippy::too_many_arguments)]
pub fn article_with_translations(
    item: &ContentItem,
    body: &str,
    locked: bool,
    reason: &str,
    related: &[ContentItem],
    ads: &[AdCampaign],
    lang: &str,
    tr: Option<&TranslationCache>,
) -> String {
    let is_en = lang.eq_ignore_ascii_case("en");
    let title = esc(localized(lang, &item.title_tr, &item.title_en));
    let sponsor_default = tr_text(
        tr,
        lang,
        "reader.sponsored_default",
        "SPONSORLU İÇERİK",
        "SPONSORED CONTENT",
    );
    let sponsor = if item.is_sponsored {
        format!(
            "<div class=\"sponsor-notice\"><strong>{}</strong> · {}</div>",
            esc(item
                .sponsor_legal_label
                .as_deref()
                .unwrap_or(&sponsor_default)),
            esc(item.sponsor_brand.as_deref().unwrap_or(""))
        )
    } else {
        String::new()
    };

    let paywall = if locked {
        let single_buy_btn = if item.single_price_try > 0.0 {
            let buy_lbl = if is_en {
                format!("Buy This Article (₺{:.0}) →", item.single_price_try)
            } else {
                format!("Bu Makaleyi Satın Al (₺{:.0}) →", item.single_price_try)
            };
            format!(
                r#"<button type="button" class="button secondary paywall-buy-btn" data-buy-article="{slug}" data-article-title="{title}" data-article-price="{price:.2}">{buy_lbl}</button>"#,
                slug = esc(&item.slug),
                title = title,
                price = item.single_price_try,
                buy_lbl = esc(&buy_lbl)
            )
        } else {
            String::new()
        };
        let club_eyebrow = tr_text(tr, lang, "reader.club_eyebrow", "MSI OKUYUCU KULÜBÜ", "MSI READER CLUB");
        let paywall_heading = tr_text(
            tr,
            lang,
            "reader.paywall_title",
            "Hikâyenin tamamını keşfedin.",
            "Discover the full story.",
        );
        let login_cta = tr_text(
            tr,
            lang,
            "account.login_cta",
            "Giriş yap / Hesap oluştur →",
            "Sign in / Create account →",
        );
        let subs_cta = tr_text(
            tr,
            lang,
            "reader.view_subscriptions",
            "Abonelikleri incele",
            "Explore subscriptions",
        );
        let subs_href = lang_href("/abonelik-ve-satis", lang);
        format!(
            r##"<aside class="paywall-panel"><div class="eyebrow">{club_eyebrow}</div><h2>{paywall_heading}</h2><p>{reason}</p><div class="hero-actions"><a href="#modal-login" class="button primary">{login_cta}</a>{single_buy_btn}<a href="{subs_href}" class="button secondary">{subs_cta}</a></div></aside>"##,
            club_eyebrow = esc(&club_eyebrow),
            paywall_heading = esc(&paywall_heading),
            reason = esc(reason),
            login_cta = esc(&login_cta),
            single_buy_btn = single_buy_btn,
            subs_href = esc(&subs_href),
            subs_cta = esc(&subs_cta),
        )
    } else {
        String::new()
    };

    let filtered_related: Vec<&ContentItem> = related
        .iter()
        .filter(|c| !is_en || is_en_content_ready(c))
        .take(crate::db::constants::READER_RELATED_LIMIT)
        .collect();
    let recommendations = if !item.is_sponsored && !filtered_related.is_empty() {
        let rel_eyebrow = tr_text(tr, lang, "reader.related_eyebrow", "OKUMAYA DEVAM EDİN", "CONTINUE READING");
        let rel_title = tr_text(tr, lang, "reader.related_title", "İlginizi çekebilir", "You may also like");
        format!(
            "<section class=\"container section-space\"><div class=\"section-heading\"><div><div class=\"eyebrow\">{}</div><h2>{}</h2></div></div><div class=\"stories\">{}</div></section>",
            esc(&rel_eyebrow),
            esc(&rel_title),
            filtered_related
                .iter()
                .map(|c| article_card(c, lang, false))
                .collect::<String>()
        )
    } else {
        String::new()
    };

    let search_base = if is_en { "/en/search" } else { "/arama" };
    let tags_html = if item.ssb_tags.is_empty() {
        String::new()
    } else {
        let tags_aria = tr_text(tr, lang, "reader.tags_aria", "Teknoloji etiketleri", "Technology tags");
        let tags_eyebrow = tr_text(tr, lang, "reader.tags_eyebrow", "TEKNOLOJİ ETİKETLERİ", "TECHNOLOGY TAGS");
        let chips: String = item
            .ssb_tags
            .iter()
            .map(|t| {
                format!(
                    "<a class=\"article-tag-pill\" href=\"{}?ssb_tag={}\">#{}</a>",
                    search_base,
                    esc(&urlencoding::encode(t)),
                    esc(&ssb_tag_display_label(t, lang))
                )
            })
            .collect();
        format!(
            r#"<div class="article-tags-bar" aria-label="{tags_aria}"><span class="eyebrow">{tags_eyebrow}</span><div class="article-tag-pills">{chips}</div></div>"#,
            tags_aria = esc(&tags_aria),
            tags_eyebrow = esc(&tags_eyebrow),
            chips = chips,
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
            .find(|a| a.is_active && !a.is_floating_popup && !a.slot_code.starts_with("ad-live-home-right"))
    };
    let inline_ad = chosen_ad
        .map(|a| advertisement(a, lang, "article-sponsor"))
        .unwrap_or_default();

    let safe_body = crate::engine::security_engine::sanitize_rich_html(body);
    let en_pending_notice = if is_en && !is_en_content_ready(item) {
        let msg = tr_text(
            tr,
            lang,
            "reader.en_pending_notice",
            "Bu içeriğin İngilizce editoryal onayı henüz tamamlanmamıştır; Türkçe özgün metin gösterilmektedir.",
            "The English edition of this article is pending editorial approval; displaying the original Turkish edition.",
        );
        format!("<div class=\"sponsor-notice\">{}</div>", esc(&msg))
    } else {
        String::new()
    };

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

    let cat_label_raw = category_display_label(&item.content_type, lang).replace('-', " ");
    let cat_label = esc(&cat_label_raw);
    let sidebar_related: Vec<ContentItem> = if is_en {
        related
            .iter()
            .filter(|c| is_en_content_ready(c))
            .cloned()
            .collect()
    } else {
        related.to_vec()
    };
    let sidebar = editorial_sidebar(&item.content_type, &sidebar_related, ads, lang);
    let home_lbl = tr_text(tr, lang, "nav.home", "Ana Sayfa", "Home");
    let breadcrumb_html = breadcrumb_localized(
        lang,
        &home_lbl,
        &item.content_type,
        &cat_label_raw,
        tr,
    );

    let author_slug = crate::controllers::web_controller::author_slug_from_name(&item.author_name);
    let author_html = if author_slug.is_empty() {
        format!("<span>{}</span>", esc(&item.author_name))
    } else {
        let author_href = lang_href(&format!("/yazar/{}", author_slug), lang);
        format!(
            r#"<a href="{href}" class="author-byline-link">{name}</a>"#,
            href = esc(&author_href),
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

    let min_lbl = tr_text(tr, lang, "reader.min_read", "dk okuma", "min read");
    let share_lbl = tr_text(tr, lang, "reader.share", "Bağlantıyı paylaş", "Share link");
    let archive_lbl = tr_text(
        tr,
        lang,
        "reader.archive",
        "Dijital E-Dergi Arşivi",
        "Digital E-Magazine Archive",
    );
    let masthead_end = if is_en {
        "MSI REVIEW · ISSN 1306-0309"
    } else {
        "MSI DERGİSİ · ISSN 1306-0309"
    };
    let cat_href = lang_href(&format!("/kategori/{}", item.content_type), lang);
    let edergi_href = lang_href("/e-dergi", lang);

    format!(
        r#"<div class="article-page" data-msi-content-slug="{slug}"><div class="container">{breadcrumb_html}<div class="article-master-grid"><div class="article-main-col"><div class="msi-section-ribbon"><a href="{cat_href}">{cat_label}</a></div><header class="article-header">{sponsor}<h1>{title}</h1><p class="article-summary">{summary}</p><div class="article-byline">{author_html}{author_role_html}<time datetime="{date}">{date}</time><span>{minutes} {min_lbl}</span><button type="button" class="text-link" data-share-article>{share_lbl} ↗</button></div></header>{cover_figure}{video_block}{en_pending_notice}<article class="msi-article-body">{body}</article>{paywall}{tags_html}<div class="article-end"><span>{masthead_end}</span><a href="{edergi_href}">{archive_lbl} →</a></div>{inline_ad}</div>{sidebar}</div></div>{recommendations}</div>"#,
        slug = esc(&item.slug),
        breadcrumb_html = breadcrumb_html,
        cat_href = esc(&cat_href),
        cat_label = cat_label,
        sponsor = sponsor,
        title = title,
        summary = esc(localized(lang, &item.summary_tr, &item.summary_en)),
        author_html = author_html,
        author_role_html = author_role_html,
        date = esc(item.published_at.get(..10).unwrap_or(&item.published_at)),
        minutes = item.read_time_min.max(1),
        min_lbl = esc(&min_lbl),
        share_lbl = esc(&share_lbl),
        cover_figure = cover_figure,
        video_block = video_block,
        en_pending_notice = en_pending_notice,
        body = safe_body,
        paywall = paywall,
        tags_html = tags_html,
        masthead_end = masthead_end,
        edergi_href = esc(&edergi_href),
        archive_lbl = esc(&archive_lbl),
        inline_ad = inline_ad,
        sidebar = sidebar,
        recommendations = recommendations
    )
}
