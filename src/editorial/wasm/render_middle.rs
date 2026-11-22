//! Middle Reading Panel & Real Website Article Sheet renderer for MSI Dergisi Editorial Studio.

use crate::editorial::common::{
    blocks_to_wp_html, build_category_tree, category_ancestor_chain, estimate_read_time_minutes,
    parse_content_to_blocks, slugify_turkish, Article,
};

use crate::editorial::wasm::icons;
use crate::editorial::wasm::render::render_news_image;
use crate::editorial::wasm::render_right::{render_embed_preview_card, render_preview_blocks_html};
use crate::editorial::wasm::state::{
    article_has_english, effective_wp_post_url, escape_html, format_middle_turkish_date,
    get_site_name, msi_content_type_label_en, msi_content_type_label_tr,
    render_safe_article_prose, AppState,
};

fn render_reader_lang_switcher(art: &Article, active_lang: &str, force_show: bool) -> String {
    if !force_show && !article_has_english(art) {
        return String::new();
    }
    let is_en = active_lang == "en";
    let tr_cls = if !is_en {
        "hf-reader-lang-btn hf-reader-lang-active"
    } else {
        "hf-reader-lang-btn"
    };
    let en_cls = if is_en {
        "hf-reader-lang-btn hf-reader-lang-active-en"
    } else {
        "hf-reader-lang-btn"
    };
    let en_status_pill = match art.en_status.as_str() {
        "approved" => r#"<span class="hf-reader-en-pill hf-reader-en-ok">ONAYLI ✓</span>"#,
        "review" => r#"<span class="hf-reader-en-pill hf-reader-en-rev">İNCELEMEDE</span>"#,
        "draft" => r#"<span class="hf-reader-en-pill">TASLAK</span>"#,
        _ => "",
    };
    format!(
        r#"<div class="hf-reader-lang-bar">
            <div class="hf-reader-lang-seg">
                <button type="button" data-action="set-preview-lang" data-lang="tr" class="{tr_cls}">🇹🇷 Türkçe (MSI Dergisi)</button>
                <button type="button" data-action="set-preview-lang" data-lang="en" class="{en_cls}">🇬🇧 English (MSI TDR)</button>
            </div>
            {en_status_pill}
        </div>"#
    )
}

fn render_msi_meta_header_blocks(s: &AppState, art: &Article, is_en: bool) -> (String, String, String) {
    let mut ribbons = Vec::new();

    let page_part = art
        .issue_page_range
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            if is_en {
                format!(" · PP. {}", escape_html(p))
            } else if p.to_uppercase().starts_with("S.") {
                format!(" · {}", escape_html(p))
            } else {
                format!(" · S. {}", escape_html(p))
            }
        })
        .unwrap_or_default();

    let chan_label = match art.publication_channel.as_str() {
        "magazine" => {
            if let Some(inum) = art.issue_number {
                if is_en {
                    format!("MSI TDR · ISSUE {inum}{page_part}")
                } else {
                    format!("MSI DERGİSİ · SAYI {inum}{page_part}")
                }
            } else if is_en {
                "MSI TDR EDITION".to_string()
            } else {
                "MSI DERGİSİ".to_string()
            }
        }
        "both" => {
            if let Some(inum) = art.issue_number {
                if is_en {
                    format!("MSI TDR · ISSUE {inum}{page_part} + DEFENCE NEWS")
                } else {
                    format!("MSI DERGİSİ · SAYI {inum}{page_part} + HABER")
                }
            } else if is_en {
                "MSI TDR + DEFENCE NEWS".to_string()
            } else {
                "MSI DERGİSİ + SAVUNMA HABER".to_string()
            }
        }
        _ => {
            if let Some(inum) = art.issue_number {
                if is_en {
                    format!("DEFENCE NEWS · ISSUE #{inum}{page_part}")
                } else {
                    format!("SAVUNMA HABER · SAYI #{inum}{page_part}")
                }
            } else if is_en {
                "DEFENCE NEWS".to_string()
            } else {
                "SAVUNMA HABER".to_string()
            }
        }
    };

    let chan_cls = if art.publication_channel == "magazine" || art.publication_channel == "both" {
        "hf-reader-msi-ribbon hf-reader-msi-crimson"
    } else {
        "hf-reader-msi-ribbon"
    };
    ribbons.push(format!(r#"<span class="{chan_cls}">{chan_label}</span>"#));

    let ct_lbl = if is_en {
        msi_content_type_label_en(&art.content_type)
    } else {
        msi_content_type_label_tr(&art.content_type)
    };
    ribbons.push(format!(
        r#"<span class="hf-reader-format-pill">{}</span>"#,
        escape_html(ct_lbl)
    ));

    if art.is_featured {
        let fl = if is_en { "★ FEATURED" } else { "★ VİTRİN / MANŞET" };
        ribbons.push(format!(r#"<span class="hf-reader-feat-pill">{fl}</span>"#));
    }

    match art.visibility_level {
        1 => {
            let lbl = if is_en {
                "🔒 GOV / MILITARY ACCESS (.gov.tr / .tsk.tr)"
            } else {
                "🔒 ÜYE & KAMU (.gov.tr / .tsk.tr)"
            };
            ribbons.push(format!(r#"<span class="hf-reader-gov-pill">{lbl}</span>"#));
        }
        2 => {
            let price_str = if art.single_price_try > 0.0 {
                format!(" • ₺{:.2}", art.single_price_try)
            } else {
                String::new()
            };
            let lbl = if is_en {
                format!("🔐 SUBSCRIBER PAYWALL{price_str}")
            } else {
                format!("🔐 ABONE / E-DERGİ PAYWALL{price_str}")
            };
            ribbons.push(format!(
                r#"<span class="hf-reader-paywall-pill">{lbl}</span>"#
            ));
        }
        _ => {}
    }

    let sponsor_banner = if art.is_sponsored {
        let legal = art
            .sponsor_legal_label
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("ADVERTORIAL / KURUMSAL İŞ BİRLİĞİ");
        let brand_part = art
            .sponsor_brand
            .as_deref()
            .map(str::trim)
            .filter(|b| !b.is_empty())
            .map(|b| format!(" · <strong>{}</strong>", escape_html(b)))
            .unwrap_or_default();
        format!(
            r#"<div class="hf-reader-sponsor-banner">{legal_esc}{brand_part}</div>"#,
            legal_esc = escape_html(legal),
        )
    } else {
        String::new()
    };

    let video_section = art
        .video_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| {
            let cap = if is_en {
                Some("Video / Interview Broadcast")
            } else {
                Some("Video / Röportaj Kaydı")
            };
            render_embed_preview_card(v, cap)
        })
        .unwrap_or_default();

    let ssb_footer = art
        .ssb_tags
        .as_ref()
        .filter(|t| !t.is_empty())
        .map(|codes| {
            let mut pills = Vec::new();
            for code in codes {
                let c = code.trim();
                if c.is_empty() {
                    continue;
                }
                let label = s
                    .ssb_tags_catalog
                    .iter()
                    .find(|item| item.code == c)
                    .map(|item| {
                        if is_en && !item.name_en.trim().is_empty() {
                            item.name_en.as_str()
                        } else {
                            item.name_tr.as_str()
                        }
                    })
                    .unwrap_or(c);
                pills.push(format!(
                    r#"<span class="hf-reader-ssb-pill">{}</span>"#,
                    escape_html(label)
                ));
            }
            if pills.is_empty() {
                String::new()
            } else {
                let heading = if is_en {
                    "SSB Defence Technology Taxonomy:"
                } else {
                    "SSB Teknoloji Taksonomisi:"
                };
                format!(
                    r#"<div class="pt-4 border-t border-zinc-100 flex items-center flex-wrap gap-1.5">
                        <span class="text-[9.5px] font-black uppercase tracking-widest text-zinc-400 mr-1">{heading}</span>
                        {}
                    </div>"#,
                    pills.join("")
                )
            }
        })
        .unwrap_or_default();

    let top_header = format!(
        r#"<div class="space-y-2">
            <div class="flex items-center flex-wrap gap-1.5">{ribbons}</div>
            {sponsor_banner}
        </div>"#,
        ribbons = ribbons.join(""),
    );

    (top_header, video_section, ssb_footer)
}

fn resolve_read_time_minutes(art: &Article) -> i32 {
    if let Some(m) = art.read_time_min.filter(|x| *x > 0) {
        return m;
    }
    let blocks = art
        .blocks
        .clone()
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| parse_content_to_blocks(&art.content));
    estimate_read_time_minutes(&art.title, art.spot.as_deref().unwrap_or(""), &blocks)
}

/// Renders the editing draft (`s.canonical_editing_article()`) as a clean, distraction-free real news portal article sheet.
/// Shared by both the Middle Column Live Draft Preview and the Full-Screen Desktop/Mobile Preview Modal.
pub fn render_draft_article_sheet_html(s: &AppState, is_mobile_frame: bool) -> String {
    let canon = s.canonical_editing_article();
    let art = &canon;
    let has_en = article_has_english(art);
    let show_en = s.preview_lang == "en" && (has_en || s.editor_lang == "en");

    let lang_switcher_html =
        render_reader_lang_switcher(art, if show_en { "en" } else { "tr" }, s.editor_lang == "en");
    let (msi_top_html, video_embed_html, ssb_footer_html) =
        render_msi_meta_header_blocks(s, art, show_en);

    let blocks = if show_en {
        art.blocks_en.as_deref().unwrap_or(&[])
    } else {
        art.blocks.as_deref().unwrap_or(&[])
    };

    let tree = build_category_tree(&s.categories);
    let selected_cat_ids = art.categories.as_deref().unwrap_or(&[]);
    let active_chain = selected_cat_ids
        .iter()
        .rev()
        .find_map(|cid| {
            let ch = category_ancestor_chain(&s.categories, *cid);
            (!ch.is_empty()).then_some(ch)
        })
        .unwrap_or_default();

    let cat_breadcrumb_html = if let Some(leaf_id) = active_chain.last().copied() {
        if let Some(item) = tree.iter().find(|t| t.category.id == leaf_id) {
            let color = item.category.color.as_deref().unwrap_or("#52c2ef");
            format!(
                r#"<span style="background-color: {color};" class="px-2.5 py-1 rounded-md text-white text-[10px] font-black uppercase tracking-widest shadow-sm">{path}</span>"#,
                color = escape_html(color),
                path = escape_html(&item.path_label),
            )
        } else {
            r#"<span class="px-2.5 py-1 rounded-md bg-zinc-100 text-zinc-500 text-[10px] font-black uppercase tracking-widest">GENEL HABER</span>"#.to_string()
        }
    } else {
        r#"<span class="px-2.5 py-1 rounded-md bg-zinc-100 text-zinc-500 text-[10px] font-black uppercase tracking-widest">KATEGORİ SEÇİLMEDİ</span>"#.to_string()
    };

    let live_badge_html = if art.is_live_blog {
        format!(
            r#"<span class="hf-reader-live-badge"><span class="hf-live-dot"></span>{} CANLI ANLATIM</span>"#,
            icons::radio("w-3 h-3")
        )
    } else {
        String::new()
    };

    let custom_author = art
        .author_name
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty());
    let author_name = custom_author
        .or_else(|| art.publisher_name.as_deref().filter(|p| !p.trim().is_empty()))
        .or_else(|| s.user.as_ref().map(|u| u.username.as_str()))
        .unwrap_or("Editör");
    let author_title_part = art
        .author_title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| format!(r#" <span class="text-zinc-400 font-semibold">· {}</span>"#, escape_html(t)))
        .unwrap_or_default();
    let read_mins = resolve_read_time_minutes(art);
    let read_time_lbl = if show_en {
        format!("{read_mins} min read")
    } else {
        format!("{read_mins} dk okuma")
    };

    let effective_slug = art
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|sl| !sl.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| slugify_turkish(&art.title));

    let slug_bar = if !effective_slug.is_empty() {
        format!(
            r#"<span class="text-[10px] font-mono font-semibold text-zinc-400 truncate">/icerik/{}</span>"#,
            escape_html(&effective_slug)
        )
    } else {
        String::new()
    };

    let active_title = if show_en {
        art.title_en.as_deref().unwrap_or("")
    } else {
        art.title.as_str()
    };
    let display_title = if active_title.trim().is_empty() {
        if show_en {
            "English Headline Not Yet Entered".to_string()
        } else {
            "Haber Başlığı Henüz Girilmedi".to_string()
        }
    } else {
        escape_html(active_title)
    };

    let title_cls = if is_mobile_frame {
        "text-xl font-black text-zinc-900 leading-[1.2] tracking-tight"
    } else {
        "text-2xl lg:text-3xl font-black text-zinc-900 leading-[1.18] tracking-tight"
    };

    let active_spot = if show_en {
        art.spot_en.as_deref()
    } else {
        art.spot.as_deref()
    };
    let spot_html = active_spot
        .map(str::trim)
        .filter(|sp| !sp.is_empty())
        .map(|sp| format!(r#"<p class="hf-preview-sheet-spot">{}</p>"#, escape_html(sp)))
        .unwrap_or_default();

    let hero_img = render_news_image(
        art.effective_image_url(),
        active_title,
        "w-full h-full object-cover",
    );

    let active_raw_content = if show_en {
        art.content_en.as_deref().unwrap_or("")
    } else {
        art.content.as_str()
    };
    let body_html = if !blocks.is_empty() {
        render_preview_blocks_html(blocks)
    } else if !active_raw_content.trim().is_empty() {
        render_safe_article_prose(active_raw_content)
    } else if show_en {
        r#"<div class="py-10 text-center bg-zinc-50 rounded-2xl border border-dashed border-zinc-200">
            <span class="text-[11px] font-bold text-zinc-400">English body has not been entered yet. Switch to EN in Step 1 to write or copy from Turkish.</span>
        </div>"#
            .to_string()
    } else {
        r#"<div class="py-10 text-center bg-zinc-50 rounded-2xl border border-dashed border-zinc-200">
            <span class="text-[11px] font-bold text-zinc-400">Haber metni henüz eklenmedi. 1. adımdan içerik bloklarını yazabilirsiniz.</span>
        </div>"#
            .to_string()
    };

    let tags_html = art
        .tags
        .as_ref()
        .filter(|t| !t.is_empty())
        .map(|tags| {
            let pills: Vec<String> = tags
                .iter()
                .filter(|t| !t.trim().is_empty())
                .map(|t| format!(r#"<span class="hf-tag-pill">#{}</span>"#, escape_html(t.trim())))
                .collect();
            if pills.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<div class="pt-6 border-t border-zinc-100 flex items-center flex-wrap gap-2">
                        <span class="text-[10px] font-black uppercase tracking-widest text-zinc-400 mr-1">Etiketler:</span>
                        {}
                    </div>"#,
                    pills.join("")
                )
            }
        })
        .unwrap_or_default();

    let mut policy_notices = Vec::new();
    if art.comments_locked {
        policy_notices.push(format!(
            r#"<div class="flex items-center gap-2 text-xs font-bold text-zinc-600">{ico} <span>Bu haber yorumlara kapatılmıştır.</span></div>"#,
            ico = icons::message_square_off("w-4 h-4 text-amber-600")
        ));
    }
    if art.ads_disabled {
        policy_notices.push(format!(
            r#"<div class="flex items-center gap-2 text-xs font-bold text-zinc-600">{ico} <span>Hassas içerik koruması: Bu haber sayfasında reklam gösterilmez.</span></div>"#,
            ico = icons::ban("w-4 h-4 text-rose-500")
        ));
    }
    let policy_footer = if policy_notices.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="p-4 rounded-2xl bg-zinc-50 border border-zinc-200/80 space-y-2">{}</div>"#,
            policy_notices.join("")
        )
    };

    let pad_cls = if is_mobile_frame {
        "p-5 space-y-5"
    } else {
        "p-6 sm:p-10 lg:p-12 space-y-7"
    };

    format!(
        r#"<article class="hf-preview-article-sheet {pad_cls}">
            {lang_switcher_html}
            <div class="space-y-3.5">
                {msi_top_html}
                <div class="flex items-center justify-between gap-2 flex-wrap">
                    <div class="flex items-center gap-2 flex-wrap">
                        {cat_breadcrumb_html}
                        {live_badge_html}
                    </div>
                    {slug_bar}
                </div>

                <h1 class="{title_cls}">{display_title}</h1>

                {spot_html}

                <div class="flex items-center flex-wrap gap-2.5 pt-1 text-[11px] font-bold text-zinc-500">
                    <span class="flex items-center gap-1.5 text-zinc-800">{user_ico} {author}{author_title_part}</span>
                    <span class="text-zinc-300">•</span>
                    <span class="flex items-center gap-1 text-zinc-600">{clock_ico} {read_time_lbl}</span>
                    <span class="text-zinc-300">•</span>
                    <span>Yayın Önizlemesi</span>
                </div>
            </div>

            <div class="aspect-video bg-zinc-100 rounded-2xl overflow-hidden border border-zinc-200 shadow-sm">
                {hero_img}
            </div>

            {video_embed_html}

            <div class="prose hf-rich-prose max-w-none text-zinc-700 leading-[1.85] space-y-5">
                {body_html}
            </div>

            {ssb_footer_html}
            {tags_html}
            {policy_footer}
        </article>"#,
        user_ico = icons::user("w-3.5 h-3.5 text-brand-500"),
        clock_ico = icons::clock("w-3.5 h-3.5 text-zinc-400"),
        author = escape_html(author_name),
    )
}

pub fn render_middle_panel_inner(s: &AppState) -> String {
    if s.middle_preview_draft {
        let research_pill = crate::editorial::wasm::render_research::render_open_research_floating_pill(s);
        let back_to_selected_btn = if s.selected_article.is_some() {
            r#"<button type="button" data-action="exit-middle-draft-preview" class="h-8 px-3 rounded-full bg-white border border-zinc-200 text-zinc-600 hover:text-zinc-900 hover:bg-zinc-50 text-[10px] font-black uppercase tracking-wider transition-all">Seçili Habere Dön</button>"#
        } else {
            ""
        };
        let sheet = render_draft_article_sheet_html(s, false);
        return format!(
            r#"<div class="hf-middle-preview-bar">
                <div class="flex items-center gap-2 min-w-0">
                    <span class="hf-live-preview-pill"><span class="hf-live-preview-dot"></span>CANLI TASLAK ÖNİZLEMESİ</span>
                    <span class="hidden xl:inline text-[10px] font-semibold text-zinc-400 truncate">Sağ editördeki değişiklikler burada anlık görünür</span>
                </div>
                <div class="flex items-center gap-2 shrink-0">
                    {research_pill}
                    {back_to_selected_btn}
                    <button type="button" data-action="open-preview-modal" class="h-8 px-3.5 rounded-full bg-zinc-900 text-white hover:bg-brand-500 text-[10px] font-black uppercase tracking-wider flex items-center gap-1.5 shadow-sm transition-all">
                        {eye_ico}
                        <span>Tam Ekran</span>
                    </button>
                </div>
            </div>
            <div id="middle-scroll" class="flex-1 overflow-y-auto custom-scrollbar relative w-full bg-white">
                <div class="w-full max-w-4xl mx-auto pb-20">
                    {sheet}
                </div>
            </div>"#,
            eye_ico = icons::eye("w-3.5 h-3.5"),
        );
    }

    if s.middle_showing_research {
        return crate::editorial::wasm::render_research::render_research_middle_reader(s);
    }

    let research_floating_pill = crate::editorial::wasm::render_research::render_open_research_floating_pill(s);

    let Some(ref art) = s.selected_article else {
        let floating_top = if research_floating_pill.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div class="absolute right-6 top-6 z-50">{research_floating_pill}</div>"#
            )
        };
        return format!(
            r#"{floating_top}<div id="middle-scroll" class="flex-1 overflow-y-auto custom-scrollbar relative w-full">
            <div class="h-full flex flex-col items-center justify-center opacity-20 uppercase font-black text-xs tracking-widest whitespace-pre">Haber Seçilmedi</div>
        </div>"#
        );
    };

    let has_en = article_has_english(art);
    let show_en = s.preview_lang == "en" && has_en;
    let lang_switcher_html =
        render_reader_lang_switcher(art, if show_en { "en" } else { "tr" }, false);
    let (msi_top_html, video_embed_html, ssb_footer_html) =
        render_msi_meta_header_blocks(s, art, show_en);

    let active_title = if show_en {
        art.title_en
            .as_deref()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or(art.title.as_str())
    } else {
        art.title.as_str()
    };

    let hero_img = render_news_image(
        art.effective_image_url(),
        active_title,
        "w-full h-full object-cover",
    );

    let raw_date = art
        .effective_publish_date()
        .or(art.created_at.as_deref())
        .or(art.created_at_camel.as_deref())
        .or_else(|| art.effective_scheduled_at())
        .or_else(|| art.effective_source_time());

    let date_span = match format_middle_turkish_date(raw_date) {
        Some(dt) => format!(
            r#"<span class="bg-zinc-100 px-2 py-1 rounded text-zinc-600">{}</span>"#,
            escape_html(&dt)
        ),
        None => String::new(),
    };

    let is_published = art.article_type.as_deref() == Some("published") || art.status == "publish";
    let status_span = if is_published {
        r#"<span class="bg-emerald-50 px-2 py-1 rounded text-emerald-600">YAYINDA</span>"#
    } else if art.status == "pending_review" {
        r#"<span class="bg-amber-100 px-2 py-1 rounded text-amber-800">ONAY BEKLİYOR</span>"#
    } else if art.status == "future" {
        r#"<span class="bg-amber-50 px-2 py-1 rounded text-amber-600">ZAMANLANDI</span>"#
    } else if art.status == "archived" {
        r#"<span class="bg-zinc-200 px-2 py-1 rounded text-zinc-700">ARŞİVDE</span>"#
    } else {
        r#"<span class="bg-zinc-100 px-2 py-1 rounded text-zinc-500">TASLAK</span>"#
    };

    let wp_url_opt = effective_wp_post_url(art).filter(|u| {
        let low = u.to_ascii_lowercase();
        low.starts_with("http://") || low.starts_with("https://")
    });

    let globe_btn_html = match wp_url_opt {
        Some(url) => format!(
            r#"<a href="{href}" target="_blank" rel="noopener noreferrer" class="w-8 h-8 bg-white text-zinc-700 border border-zinc-200 rounded-lg flex items-center justify-center shadow-sm hover:text-brand-500 hover:border-brand-500/40 transition-all shrink-0" title="Yayındaki Haberi Aç">
                {globe_ico}
            </a>"#,
            href = escape_html(url),
            globe_ico = icons::globe("w-4 h-4"),
        ),
        None => String::new(),
    };

    let revisions_btn_html = if !art.id.trim().is_empty() {
        format!(
            r#"<button type="button" data-action="open-revisions-modal" data-article-id="{aid}" data-article-title="{atitle}" class="h-8 px-2.5 bg-white text-zinc-700 border border-zinc-200 rounded-lg flex items-center justify-center gap-1.5 shadow-sm hover:text-brand-500 hover:border-brand-500/40 transition-all text-[10px] font-bold tracking-wide shrink-0 whitespace-nowrap" title="Revizyon Geçmişi">
                {hist_ico}
                <span class="hidden sm:inline">Revizyon Geçmişi</span>
            </button>"#,
            aid = escape_html(&art.id),
            atitle = escape_html(&art.title),
            hist_ico = icons::history("w-3.5 h-3.5"),
        )
    } else {
        String::new()
    };

    let mut control_badges = String::new();
    if art.is_live_blog {
        control_badges.push_str(&format!(
            r#"<span class="hf-reader-live-badge"><span class="hf-live-dot"></span>{} CANLI ANLATIM</span>"#,
            icons::radio("w-3 h-3")
        ));
    }
    if art.comments_locked {
        control_badges.push_str(&format!(
            r#"<span class="hf-reader-flag-badge">{} YORUMLARA KAPALI</span>"#,
            icons::message_square_off("w-3 h-3")
        ));
    }
    if art.ads_disabled {
        control_badges.push_str(&format!(
            r#"<span class="hf-reader-flag-badge">{} REKLAMSIZ</span>"#,
            icons::ban("w-3 h-3")
        ));
    }
    if let Some(slug) = art.slug.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        control_badges.push_str(&format!(
            r#"<span class="bg-zinc-100 px-2 py-1 rounded text-zinc-500 lowercase font-mono">/icerik/{}</span>"#,
            escape_html(slug)
        ));
    }

    let active_spot = if show_en {
        art.spot_en.as_deref()
    } else {
        art.spot.as_deref()
    };
    let spot_html = active_spot
        .map(str::trim)
        .filter(|sp| !sp.is_empty())
        .map(|sp| format!(r#"<p class="hf-reader-spot">{}</p>"#, escape_html(sp)))
        .unwrap_or_default();

    let custom_author = art
        .author_name
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty());
    let author_display = custom_author
        .or_else(|| art.publisher_name.as_deref().filter(|p| !p.trim().is_empty()));
    let author_title_part = art
        .author_title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| format!(r#" <span class="text-zinc-400 font-semibold">· {}</span>"#, escape_html(t)))
        .unwrap_or_default();
    let read_mins = resolve_read_time_minutes(art);
    let read_time_lbl = if show_en {
        format!("{read_mins} min read")
    } else {
        format!("{read_mins} dk okuma")
    };
    let byline_html = match author_display {
        Some(a) => format!(
            r#"<div class="flex items-center flex-wrap gap-2.5 pt-1 text-[11px] font-bold text-zinc-500">
                <span class="flex items-center gap-1.5 text-zinc-800">{user_ico} {author}{author_title_part}</span>
                <span class="text-zinc-300">•</span>
                <span class="flex items-center gap-1 text-zinc-600">{clock_ico} {read_time_lbl}</span>
            </div>"#,
            user_ico = icons::user("w-3.5 h-3.5 text-brand-500"),
            clock_ico = icons::clock("w-3.5 h-3.5 text-zinc-400"),
            author = escape_html(a),
        ),
        None => format!(
            r#"<div class="flex items-center flex-wrap gap-2 pt-1 text-[11px] font-bold text-zinc-500">
                <span class="flex items-center gap-1 text-zinc-600">{clock_ico} {read_time_lbl}</span>
            </div>"#,
            clock_ico = icons::clock("w-3.5 h-3.5 text-zinc-400"),
        ),
    };

    let tags_html = art
        .tags
        .as_ref()
        .filter(|t| !t.is_empty())
        .map(|tags| {
            let pills: Vec<String> = tags
                .iter()
                .filter(|t| !t.trim().is_empty())
                .map(|t| format!(r#"<span class="hf-tag-pill">#{}</span>"#, escape_html(t.trim())))
                .collect();
            if pills.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<div class="pt-6 border-t border-zinc-100 flex items-center flex-wrap gap-2">{}</div>"#,
                    pills.join("")
                )
            }
        })
        .unwrap_or_default();

    let meta_extra_html = if let Some(url) = wp_url_opt {
        format!(
            r#"<span class="truncate max-w-[150px] lg:max-w-[200px]">{}</span>"#,
            escape_html(&get_site_name(url))
        )
    } else if let Some(pub_name) = art.publisher_name.as_deref().filter(|p| !p.trim().is_empty()) {
        format!(
            r#"<span class="truncate max-w-[150px] lg:max-w-[200px]">{}</span>"#,
            escape_html(pub_name)
        )
    } else {
        String::new()
    };

    let raw_prose = if show_en {
        if let Some(c_en) = art.content_en.as_deref().filter(|c| !c.trim().is_empty()) {
            c_en.to_string()
        } else if let Some(b_en) = art.blocks_en.as_ref().filter(|b| !b.is_empty()) {
            blocks_to_wp_html(b_en)
        } else {
            String::new()
        }
    } else if !art.content.trim().is_empty() {
        art.content.clone()
    } else if let Some(blocks) = art.blocks.as_ref().filter(|b| !b.is_empty()) {
        blocks_to_wp_html(blocks)
    } else {
        String::new()
    };
    let prose_html = render_safe_article_prose(&raw_prose);

    let mode_toggle_html = if !s.research_tabs.is_empty() {
        format!(
            r#"<div class="hf-reader-mode-toggle">
                <div class="hf-reader-mode-btn hf-reader-mode-active" title="Haber Düzenleme Masasındasınız">
                    {edit_ico}
                    <span>Haberim</span>
                </div>
                <button type="button" data-action="enter-research-reader" class="hf-reader-mode-btn" title="Açık Araştırma Sekmelerine Geç">
                    {book_ico}
                    <span>Araştırma ({n}) ↗</span>
                </button>
            </div>"#,
            n = s.research_tabs.len(),
            edit_ico = icons::pen_square("w-3.5 h-3.5 text-brand-500"),
            book_ico = icons::compass("w-3.5 h-3.5"),
        )
    } else {
        String::from(r#"<span class="text-[11px] font-bold text-zinc-400 hidden sm:inline">Haber Önizleme</span>"#)
    };

    format!(
        r#"<div class="hf-middle-action-bar flex items-center justify-between gap-2 px-4 lg:px-6 py-2.5 border-b border-zinc-200/80 bg-white/95 backdrop-blur-sm shrink-0 z-30">
            <div class="flex items-center gap-2 min-w-0">
                <button type="button" data-action="mobile-back-left" class="lg:hidden w-8 h-8 bg-zinc-50 rounded-lg flex items-center justify-center text-zinc-600 border border-zinc-200 shrink-0" title="Haber Listesine Dön">
                    {back_ico}
                </button>
                {mode_toggle_html}
            </div>
            <div class="flex items-center gap-2 flex-wrap justify-end">
                {research_floating_pill}
                {revisions_btn_html}
                {globe_btn_html}
                <button type="button" data-action="move-to-editor" class="h-8 px-3.5 bg-brand-500 hover:bg-brand-600 text-white rounded-lg flex items-center justify-center gap-1.5 shadow-sm transition-all text-[10px] font-extrabold uppercase tracking-wider shrink-0 whitespace-nowrap" title="Editöre Aktar">
                    {plus_ico}
                    <span class="hidden sm:inline">Editöre Aktar</span>
                </button>
            </div>
        </div>
        <div id="middle-scroll" class="flex-1 overflow-y-auto custom-scrollbar relative w-full">
            <div class="w-full max-w-4xl mx-auto p-6 lg:p-10 pb-24 space-y-6">
                {lang_switcher_html}

                <div class="relative group">
                    <div class="aspect-video bg-zinc-100 rounded-2xl lg:rounded-3xl overflow-hidden border border-zinc-200">
                        {hero_img}
                    </div>
                </div>

                <div class="space-y-3">
                    {msi_top_html}
                    <div class="flex items-center flex-wrap gap-2 text-[9px] font-black uppercase tracking-widest text-zinc-500">
                        {date_span}
                        {status_span}
                        {control_badges}
                        {meta_extra_html}
                    </div>
                    <h1 class="text-lg lg:text-xl font-black leading-[1.2] tracking-tight text-zinc-900">{title}</h1>
                    {spot_html}
                    {byline_html}
                </div>

                {video_embed_html}

                <div class="h-px bg-zinc-100 w-full"></div>

                <div class="prose hf-rich-prose prose-sm lg:prose-base max-w-none text-zinc-600 font-medium leading-[1.8] prose-headings:text-zinc-900 prose-headings:font-black prose-headings:tracking-tight prose-p:mb-6 prose-strong:text-zinc-900 prose-a:text-brand-600">
                    <div>{prose_html}</div>
                </div>

                {ssb_footer_html}
                {tags_html}
            </div>
        </div>"#,
        plus_ico = icons::plus("w-4 h-4"),
        back_ico = icons::arrow_left("w-4 h-4"),
        title = escape_html(active_title),
    )
}
