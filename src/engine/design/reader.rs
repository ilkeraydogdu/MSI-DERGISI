use std::collections::HashSet;

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

fn is_void_html_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn is_container_html_tag(tag: &str) -> bool {
    matches!(
        tag,
        "blockquote"
            | "ul"
            | "ol"
            | "li"
            | "table"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "td"
            | "th"
            | "figure"
            | "figcaption"
            | "aside"
            | "nav"
            | "section"
            | "article"
            | "details"
            | "summary"
            | "pre"
            | "header"
            | "footer"
            | "svg"
    )
}

fn extract_tag_name(raw: &str) -> String {
    raw.trim_start()
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b':')
        .map(|b| (b as char).to_ascii_lowercase())
        .collect()
}

fn find_unquoted_tag_end(html: &str, start_after_lt: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut quote: Option<u8> = None;
    let mut j = start_after_lt;
    while j < bytes.len() {
        let b = bytes[j];
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            quote = Some(b);
        } else if b == b'>' {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn find_raw_text_end(html: &str, from: usize, closing_tag: &str) -> Option<usize> {
    let lower_tail = html[from..].to_ascii_lowercase();
    let needle = format!("</{}", closing_tag);
    let mut search_pos = 0usize;
    while let Some(rel) = lower_tail[search_pos..].find(&needle) {
        let after_name = search_pos + rel + needle.len();
        let rest = &lower_tail[after_name..];
        let trimmed = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
        if trimmed.starts_with('>') {
            let gt_offset = rest.len() - trimmed.len() + 1;
            return Some(from + after_name + gt_offset);
        }
        search_pos += rel + needle.len();
    }
    None
}

fn is_meaningful_paragraph(inner: &str) -> bool {
    let bytes = inner.as_bytes();
    let mut k = 0usize;
    while k < bytes.len() {
        if bytes[k] == b'<' {
            let Some(gt) = find_unquoted_tag_end(inner, k + 1) else {
                break;
            };
            let raw_tag = inner[k + 1..gt].trim_start();
            if !raw_tag.starts_with('/') && !raw_tag.starts_with('!') && !raw_tag.starts_with('?') {
                let name = extract_tag_name(raw_tag);
                if matches!(
                    name.as_str(),
                    "img" | "video" | "svg" | "picture" | "audio" | "iframe" | "canvas"
                ) {
                    return true;
                }
            }
            k = gt + 1;
        } else if bytes[k] == b'&' {
            let rest = &inner[k..];
            if let Some(semi) = rest.find(';') {
                if semi <= 10 {
                    let ent = &rest[..=semi];
                    if ent.eq_ignore_ascii_case("&nbsp;")
                        || ent.eq_ignore_ascii_case("&#160;")
                        || ent.eq_ignore_ascii_case("&#0160;")
                        || ent.eq_ignore_ascii_case("&#xa0;")
                        || ent.eq_ignore_ascii_case("&#x00a0;")
                    {
                        k += semi + 1;
                        continue;
                    }
                }
            }
            return true;
        } else {
            let Some(ch) = inner[k..].chars().next() else {
                break;
            };
            if !ch.is_whitespace() && ch != '\u{00A0}' && ch != '\u{200B}' && ch != '\u{FEFF}' {
                return true;
            }
            k += ch.len_utf8();
        }
    }
    false
}

/// Finds a safe UTF-8 byte index immediately after paragraph 2 (when >= 3 meaningful top-level
/// paragraphs exist) or after paragraph 1 (when 2 exist). Returns `None` for 0-1 paragraphs.
pub fn find_paragraph_split_index(html: &str) -> Option<usize> {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    let mut tag_stack: Vec<String> = Vec::new();
    let mut active_top_p: Option<usize> = None;
    let mut boundaries: Vec<usize> = Vec::new();

    while i < len {
        let Some(rel_lt) = html[i..].find('<') else {
            break;
        };
        i += rel_lt;

        if html[i..].starts_with("<!--") {
            let Some(end_rel) = html[i + 4..].find("-->") else {
                break;
            };
            i = i + 4 + end_rel + 3;
            continue;
        }

        if html[i..].starts_with("<!") || html[i..].starts_with("<?") {
            let Some(gt) = find_unquoted_tag_end(html, i + 2) else {
                break;
            };
            i = gt + 1;
            continue;
        }

        let Some(gt) = find_unquoted_tag_end(html, i + 1) else {
            break;
        };
        let tag_end = gt + 1;
        let raw_inner = &html[i + 1..gt];
        let trimmed = raw_inner.trim_start();
        if trimmed.is_empty() {
            i = tag_end;
            continue;
        }

        let is_closing = trimmed.starts_with('/');
        let name = if is_closing {
            extract_tag_name(&trimmed[1..])
        } else {
            extract_tag_name(trimmed)
        };
        if name.is_empty() {
            i = tag_end;
            continue;
        }

        if !is_closing && (name == "script" || name == "style") {
            let Some(raw_end) = find_raw_text_end(html, tag_end, &name) else {
                break;
            };
            i = raw_end;
            continue;
        }

        if is_closing {
            if name == "p" {
                if let Some(pos) = tag_stack.iter().rposition(|t| t == "p") {
                    tag_stack.truncate(pos);
                    let in_nested_or_container = tag_stack
                        .iter()
                        .any(|t| t == "p" || is_container_html_tag(t));
                    if !in_nested_or_container {
                        if let Some(inner_start) = active_top_p.take() {
                            if is_meaningful_paragraph(&html[inner_start..i]) {
                                boundaries.push(tag_end);
                            }
                        }
                    }
                }
            } else if let Some(pos) = tag_stack.iter().rposition(|t| t == &name) {
                tag_stack.truncate(pos);
                if active_top_p.is_some() && !tag_stack.iter().any(|t| t == "p") {
                    active_top_p = None;
                }
            }
        } else {
            let is_self_closing = raw_inner.trim_end().ends_with('/') || is_void_html_tag(&name);
            if !is_self_closing {
                if name == "p" {
                    let suppressed = tag_stack
                        .iter()
                        .any(|t| t == "p" || is_container_html_tag(t));
                    if !suppressed {
                        active_top_p = Some(tag_end);
                    }
                }
                tag_stack.push(name);
            }
        }

        i = tag_end;
    }

    if boundaries.len() >= 3 {
        Some(boundaries[1])
    } else if boundaries.len() == 2 {
        Some(boundaries[0])
    } else {
        None
    }
}

/// Splices `inline_html` at the safe paragraph boundary inside sanitized article HTML.
pub fn inject_inline_related_html(safe_body: &str, inline_html: &str) -> Option<String> {
    if inline_html.trim().is_empty() {
        return None;
    }
    let pos = find_paragraph_split_index(safe_body)?;
    let (head, tail) = safe_body.split_at(pos);
    Some(format!("{}{}{}", head, inline_html, tail))
}

fn render_inline_related_block(
    c: &ContentItem,
    lang: &str,
    tr: Option<&TranslationCache>,
) -> String {
    let is_en = lang.eq_ignore_ascii_case("en");
    let heading_lbl = tr_text(
        tr,
        lang,
        "reader.inline_related_label",
        "İLGİNİZİ ÇEKEBİLİR",
        "RELATED",
    );
    let aria_lbl = tr_text(
        tr,
        lang,
        "reader.inline_related_aria",
        "İlgili içerik önerisi",
        "Related article recommendation",
    );
    let min_lbl = if is_en { "min read" } else { "dk okuma" };
    let href = if is_en {
        format!("/icerik/{}?lang=en", esc(&c.slug))
    } else {
        format!("/icerik/{}", esc(&c.slug))
    };
    let title = esc(localized(lang, &c.title_tr, &c.title_en));
    let summary_raw = localized(lang, &c.summary_tr, &c.summary_en).trim();
    let summary_html = if summary_raw.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="msi-inline-related-summary">{}</p>"#, esc(summary_raw))
    };
    let cat_lbl = esc(&category_display_label(&c.content_type, lang).replace('-', " "));
    let img_src = asset(&c.cover_image);
    let (w, h) = image_dimensions(&c.cover_image).unwrap_or((640, 400));
    let portrait_cls = if h >= w { " is-portrait" } else { "" };
    let date = esc(c.published_at.get(..10).unwrap_or(&c.published_at));
    let author_html = if c.author_name.trim().is_empty() {
        String::new()
    } else {
        format!(
            r#"<span class="msi-inline-related-author">{}</span>"#,
            esc(c.author_name.trim())
        )
    };
    let minutes = c.read_time_min.max(1);

    format!(
        r#"<aside class="msi-inline-related" data-related-inline aria-label="{aria_lbl}"><div class="msi-inline-related-head"><span class="eyebrow">{heading_lbl}</span><span class="msi-inline-related-cat">{cat_lbl}</span></div><div class="msi-inline-related-card"><a class="msi-inline-related-thumb{portrait_cls}" href="{href}" tabindex="-1" aria-hidden="true"><img src="{img_src}" alt="{title}" loading="lazy" decoding="async" width="{w}" height="{h}"></a><div class="msi-inline-related-copy"><h3 class="msi-inline-related-title"><a href="{href}">{title}</a></h3>{summary_html}<div class="msi-inline-related-meta">{author_html}<time datetime="{date}">{date}</time><span>{minutes} {min_lbl}</span></div></div></div></aside>"#,
        aria_lbl = esc(&aria_lbl),
        heading_lbl = esc(&heading_lbl),
        cat_lbl = cat_lbl,
        portrait_cls = portrait_cls,
        href = href,
        img_src = img_src,
        title = title,
        w = w,
        h = h,
        summary_html = summary_html,
        author_html = author_html,
        date = date,
        minutes = minutes,
        min_lbl = esc(min_lbl),
    )
}

pub fn article(
    item: &ContentItem,
    body: &str,
    locked: bool,
    reason: &str,
    related: &[ContentItem],
    ads: &[AdCampaign],
    lang: &str,
) -> String {
    article_with_toc_and_translations(item, body, locked, reason, related, ads, lang, None, None)
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
    article_with_toc_and_translations(item, body, locked, reason, related, ads, lang, None, tr)
}

#[allow(clippy::too_many_arguments)]
pub fn article_with_toc_and_translations(
    item: &ContentItem,
    body: &str,
    locked: bool,
    reason: &str,
    related: &[ContentItem],
    ads: &[AdCampaign],
    lang: &str,
    toc_html: Option<&str>,
    tr: Option<&TranslationCache>,
) -> String {
    let is_en = lang.eq_ignore_ascii_case("en");
    let is_sponsored_or_adv = item.is_sponsored
        || item.content_type.eq_ignore_ascii_case("advertorial")
        || item.content_type.eq_ignore_ascii_case("advertorials");
    let title = esc(localized(lang, &item.title_tr, &item.title_en));
    let sponsor_default = tr_text(
        tr,
        lang,
        "reader.sponsored_default",
        "SPONSORLU İÇERİK",
        "SPONSORED CONTENT",
    );
    let sponsor = if is_sponsored_or_adv {
        let legal_label = item
            .sponsor_legal_label
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&sponsor_default);
        match item
            .sponsor_brand
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(brand) => format!(
                "<div class=\"sponsor-notice\"><strong>{}</strong> · {}</div>",
                esc(legal_label),
                esc(brand)
            ),
            None => format!(
                "<div class=\"sponsor-notice\"><strong>{}</strong></div>",
                esc(legal_label)
            ),
        }
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

    let eligible_related: Vec<&ContentItem> = if is_sponsored_or_adv {
        Vec::new()
    } else {
        let mut seen_ids: HashSet<i64> = HashSet::new();
        let mut seen_slugs: HashSet<&str> = HashSet::new();
        related
            .iter()
            .filter(|c| {
                if c.id == item.id || c.slug == item.slug {
                    return false;
                }
                if c.is_sponsored
                    || c.content_type.eq_ignore_ascii_case("advertorial")
                    || c.content_type.eq_ignore_ascii_case("advertorials")
                {
                    return false;
                }
                if is_en && !is_en_content_ready(c) {
                    return false;
                }
                if !is_en && c.source_group.eq_ignore_ascii_case("msi-tdr") {
                    return false;
                }
                seen_ids.insert(c.id) && seen_slugs.insert(c.slug.as_str())
            })
            .collect()
    };

    let safe_body = crate::engine::security_engine::sanitize_rich_html(body);
    let (rendered_body, inline_used_id) = if !is_sponsored_or_adv && !locked && !eligible_related.is_empty() {
        let inline_item = eligible_related[0];
        let inline_html = render_inline_related_block(inline_item, lang, tr);
        if let Some(spliced) = inject_inline_related_html(&safe_body, &inline_html) {
            (spliced, Some(inline_item.id))
        } else {
            (safe_body, None)
        }
    } else {
        (safe_body, None)
    };

    let bottom_related: Vec<&ContentItem> = if is_sponsored_or_adv {
        Vec::new()
    } else {
        eligible_related
            .iter()
            .copied()
            .filter(|c| Some(c.id) != inline_used_id)
            .take(crate::db::constants::READER_RELATED_LIMIT)
            .collect()
    };
    let recommendations = if !is_sponsored_or_adv && !bottom_related.is_empty() {
        let rel_eyebrow = tr_text(tr, lang, "reader.related_eyebrow", "OKUMAYA DEVAM EDİN", "CONTINUE READING");
        let rel_title = tr_text(tr, lang, "reader.related_title", "İlginizi çekebilir", "You may also like");
        format!(
            "<section class=\"container section-space related-strip msi-related-strip\" id=\"related-strip\" data-related-strip><div class=\"section-heading\"><div><div class=\"eyebrow\">{}</div><h2>{}</h2></div></div><div class=\"stories related-strip-grid\">{}</div></section>",
            esc(&rel_eyebrow),
            esc(&rel_title),
            bottom_related
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
    let sidebar_related: Vec<ContentItem> = if is_sponsored_or_adv {
        Vec::new()
    } else {
        eligible_related.iter().map(|&c| c.clone()).collect()
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
    let toc_block = toc_html.unwrap_or("");

    let primary_ssb = item
        .ssb_tags
        .iter()
        .map(|s| s.trim())
        .find(|s| !s.is_empty() && *s != "*")
        .unwrap_or("");
    let ssb_tags_csv = item.ssb_tags.join(",");
    let ssb_tag_attr = if primary_ssb.is_empty() {
        String::new()
    } else {
        format!(r#" data-ssb-tag="{}""#, esc(primary_ssb))
    };

    format!(
        r#"<div class="article-page" data-msi-content-slug="{slug}" data-content-id="{content_id}" data-category-slug="{category_slug}" data-ssb-tags="{ssb_tags_csv}"{ssb_tag_attr}><div class="container">{breadcrumb_html}<div class="article-master-grid"><div class="article-main-col"><div class="msi-section-ribbon"><a href="{cat_href}">{cat_label}</a></div><header class="article-header">{sponsor}<h1>{title}</h1><p class="article-summary">{summary}</p><div class="article-byline">{author_html}{author_role_html}<time datetime="{date}">{date}</time><span>{minutes} {min_lbl}</span><button type="button" class="text-link" data-share-article>{share_lbl} ↗</button></div></header>{cover_figure}{video_block}{en_pending_notice}{toc_block}<article class="msi-article-body" data-msi-content-slug="{slug}" data-content-id="{content_id}" data-category-slug="{category_slug}" data-ssb-tags="{ssb_tags_csv}"{ssb_tag_attr}>{body}</article>{paywall}{tags_html}<div class="article-end"><span>{masthead_end}</span><a href="{edergi_href}">{archive_lbl} →</a></div>{inline_ad}</div>{sidebar}</div></div>{recommendations}</div>"#,
        slug = esc(&item.slug),
        content_id = item.id,
        category_slug = esc(&item.content_type),
        ssb_tags_csv = esc(&ssb_tags_csv),
        ssb_tag_attr = ssb_tag_attr,
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
        toc_block = toc_block,
        body = rendered_body,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item(id: i64, slug: &str) -> ContentItem {
        ContentItem {
            id,
            slug: slug.to_string(),
            title_tr: format!("Başlık {}", id),
            title_en: format!("Title {}", id),
            summary_tr: format!("Özet {}", id),
            summary_en: format!("Summary {}", id),
            body_tr: "<p>Birinci paragraf.</p><p>İkinci paragraf.</p><p>Üçüncü paragraf.</p>".into(),
            body_en: "<p>First paragraph.</p><p>Second paragraph.</p><p>Third paragraph.</p>".into(),
            content_type: "haberler".into(),
            source_group: "gunluk-haber".into(),
            cover_image: "images/example.webp".into(),
            author_name: "Editör Ekibi".into(),
            author_title: String::new(),
            issue_number: None,
            published_at: "2026-11-10 10:00:00".into(),
            read_time_min: 4,
            visibility_level: 0,
            single_price_try: 0.0,
            is_sponsored: false,
            sponsor_brand: None,
            sponsor_legal_label: None,
            is_featured: false,
            video_url: None,
            view_count: 100,
            true_read_count: 50,
            placements: vec!["haberler".into()],
            ssb_tags: vec!["radarlar-ve-sensorler".into(), "hava-araclari".into()],
            status: "published".into(),
            en_status: "approved".into(),
            ..Default::default()
        }
    }

    #[test]
    fn splitter_injects_after_second_paragraph_for_three_or_more() {
        let html = "<p>P1</p><p>P2</p><p>P3</p><p>P4</p>";
        let out = inject_inline_related_html(html, "<aside data-related-inline>REL</aside>")
            .expect("should split");
        assert_eq!(
            out,
            "<p>P1</p><p>P2</p><aside data-related-inline>REL</aside><p>P3</p><p>P4</p>"
        );
    }

    #[test]
    fn splitter_ignores_leading_empty_and_whitespace_paragraphs() {
        let html = "<p></p><p style=\"text-align: justify\"> </p><p>&nbsp;</p><p><br></p><p>Gerçek 1</p><p>Gerçek 2</p><p>Gerçek 3</p>";
        let out = inject_inline_related_html(html, "<aside>REL</aside>").expect("should split");
        assert_eq!(
            out,
            "<p></p><p style=\"text-align: justify\"> </p><p>&nbsp;</p><p><br></p><p>Gerçek 1</p><p>Gerçek 2</p><aside>REL</aside><p>Gerçek 3</p>"
        );
    }

    #[test]
    fn splitter_handles_two_paragraphs_and_skips_one_or_zero() {
        let two = "<p>First</p><p>Second</p>";
        let out = inject_inline_related_html(two, "<aside>REL</aside>").expect("should split two");
        assert_eq!(out, "<p>First</p><aside>REL</aside><p>Second</p>");

        assert!(inject_inline_related_html("<p>Only one</p>", "<aside>REL</aside>").is_none());
        assert!(inject_inline_related_html("<ul><li>No p tags</li></ul>", "<aside>REL</aside>").is_none());
    }

    #[test]
    fn splitter_ignores_nested_paragraphs_in_containers() {
        let html = "<blockquote><p>Q1</p><p>Q2</p></blockquote><ol><li><p>TOC</p></li></ol><figure><p>Cap</p></figure><p>Body 1</p><p>Body 2</p><p>Body 3</p>";
        let out = inject_inline_related_html(html, "<aside>REL</aside>").expect("should split");
        assert_eq!(
            out,
            "<blockquote><p>Q1</p><p>Q2</p></blockquote><ol><li><p>TOC</p></li></ol><figure><p>Cap</p></figure><p>Body 1</p><p>Body 2</p><aside>REL</aside><p>Body 3</p>"
        );
    }

    #[test]
    fn splitter_handles_comments_quoted_gt_entities_turkish_utf8_and_unclosed_eof() {
        let html = "<!-- </p></p> --><p data-note=\"a > b </p> c\">Türkçe ğüşöçıİ &amp; &uuml;</p><p>İkinci paragraf ğüşöçıİ</p><p>Üçüncü paragraf</p>";
        let out = inject_inline_related_html(html, "<aside>REL</aside>").expect("should split");
        assert!(out.contains("<p>İkinci paragraf ğüşöçıİ</p><aside>REL</aside><p>Üçüncü paragraf</p>"));

        let unclosed_second_p = "<p>P1</p><p class=\"broken";
        assert!(inject_inline_related_html(unclosed_second_p, "<aside>REL</aside>").is_none());

        let trailing_unclosed = "<p>P1</p><p>P2</p><span class=\"broken";
        assert_eq!(
            inject_inline_related_html(trailing_unclosed, "<aside>REL</aside>").as_deref(),
            Some("<p>P1</p><aside>REL</aside><p>P2</p><span class=\"broken")
        );
    }

    #[test]
    fn reader_deduplicates_inline_and_bottom_and_suppresses_sponsored() {
        let current = sample_item(1, "current-article");
        let r1 = sample_item(2, "rel-1");
        let r2 = sample_item(3, "rel-2");
        let r3 = sample_item(4, "rel-3");
        let r4 = sample_item(5, "rel-4");
        let mut r_sponsored = sample_item(6, "rel-spon");
        r_sponsored.is_sponsored = true;
        let body = "<p>P1</p><p>P2</p><p>P3</p><p>P4</p>";
        let toc = "<nav class=\"dossier-toc\" id=\"dossier-toc\"><ol><li><p class=\"dossier-toc-summary\">TOC</p><svg></svg></li></ol></nav>";

        let html = article_with_toc_and_translations(
            &current,
            body,
            false,
            "",
            &[r1.clone(), r1.clone(), r_sponsored, r2, r3, r4],
            &[],
            "tr",
            Some(toc),
            None,
        );

        assert!(html.contains(r#"class="msi-inline-related" data-related-inline"#));
        assert!(html.contains(r#"id="related-strip" data-related-strip"#));
        assert!(html.contains(r#"id="dossier-toc""#));
        assert!(html.contains("<svg></svg>"));
        assert!(html.contains(r#"data-ssb-tag="radarlar-ve-sensorler""#));
        assert!(html.contains(r#"data-content-id="1""#));
        assert!(!html.contains("/icerik/rel-spon"));
        // Inline has rel-1, bottom strip has rel-2, rel-3, rel-4 (never rel-1 inside #related-strip)
        let strip_part = html.split(r#"id="related-strip""#).nth(1).unwrap();
        assert!(!strip_part.contains("/icerik/rel-1"));
        assert!(strip_part.contains("/icerik/rel-2"));
        assert!(strip_part.contains("/icerik/rel-3"));
        assert!(strip_part.contains("/icerik/rel-4"));

        // Sponsored article suppresses inline, bottom, and sidebar
        let mut spon_cur = sample_item(10, "spon-cur");
        spon_cur.is_sponsored = true;
        let spon_html = article_with_toc_and_translations(
            &spon_cur,
            body,
            false,
            "",
            std::slice::from_ref(&r1),
            &[],
            "tr",
            None,
            None,
        );
        assert!(!spon_html.contains("data-related-inline"));
        assert!(!spon_html.contains("data-related-strip"));
        assert!(!spon_html.contains("sidebar-rel-card"));

        // Plural advertorials also suppresses recommendations
        let mut adv_cur = sample_item(11, "adv-cur");
        adv_cur.content_type = "advertorials".into();
        let adv_html = article_with_toc_and_translations(
            &adv_cur,
            body,
            false,
            "",
            &[r1],
            &[],
            "tr",
            None,
            None,
        );
        assert!(!adv_html.contains("data-related-inline"));
        assert!(!adv_html.contains("data-related-strip"));
        assert!(!adv_html.contains("sidebar-rel-card"));
        assert!(adv_html.contains(r#"<div class="sponsor-notice"><strong>SPONSORLU İÇERİK</strong></div>"#));

        // Blank legal label falls back to default, and brand is appended with separator only when present
        let mut adv_brand = sample_item(12, "adv-brand");
        adv_brand.content_type = "advertorial".into();
        adv_brand.sponsor_legal_label = Some("   ".into());
        adv_brand.sponsor_brand = Some(" Roketsan ".into());
        let adv_brand_html = article_with_toc_and_translations(
            &adv_brand,
            body,
            false,
            "",
            &[],
            &[],
            "en",
            None,
            None,
        );
        assert!(adv_brand_html.contains(r#"<div class="sponsor-notice"><strong>SPONSORED CONTENT</strong> · Roketsan</div>"#));
    }
}
