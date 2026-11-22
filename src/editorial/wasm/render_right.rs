//! Right Panel 4-Step Editorial Wizard renderer with 16 Rich Block Types, Live Blog, Gallery, Related Cards, Google Discover Guard & Rich Preview.

use crate::editorial::common::{
    build_category_tree, category_ancestor_chain, collect_descendant_category_ids,
    evaluate_seo_discover, sanitize_inline_rich_html, slugify_turkish, strip_all_html_tags,
    CheckLevel, ContentBlock, GalleryItem, RelatedArticleRef,
};

use crate::editorial::wasm::icons;
use crate::editorial::wasm::render::render_news_image;
use crate::editorial::wasm::rich_editor::rich_block_editable_inner_html;
use crate::editorial::wasm::state::{
    article_has_english, escape_html, is_valid_image_url, iso_to_datetime_local, js_now_hhmm,
    msi_content_type_label_tr, turkish_lowercase, AppState, MSI_CONTENT_TYPES,
};

pub fn render_editor_conflict_banner(s: &AppState) -> String {
    let Some(other) = s.effective_editing_conflict() else {
        return String::new();
    };
    format!(
        r#"<div class="hf-conflict-banner">
            <div class="flex items-center gap-2.5 min-w-0">
                <span class="hf-conflict-pulse"></span>
                {lock_ico}
                <div class="min-w-0">
                    <div class="hf-conflict-title text-xs font-bold leading-snug">Bu haber şu an {uname} tarafından düzenleniyor</div>
                    <div class="hf-conflict-desc text-[11px] font-medium">Üzerine yazma çakışmasını önlemek için kaydetmeden önce koordinasyon sağlayın.</div>
                </div>
            </div>
        </div>"#,
        lock_ico = icons::lock("w-4 h-4 text-amber-600 shrink-0"),
        uname = escape_html(&other.username),
    )
}

pub fn render_right_panel_inner(s: &AppState) -> String {
    let step = s.editor_step;

    let left_nav_html = if step == 1 {
        let exp_cls = if s.all_blocks_expanded {
            "text-brand-500 bg-brand-50 hover:bg-brand-100"
        } else {
            "text-zinc-300 hover:text-zinc-600 hover:bg-zinc-100"
        };
        let exp_title = if s.all_blocks_expanded {
            "Kompakt Görünüm"
        } else {
            "Geniş Görünüm"
        };
        let exp_ico = if s.all_blocks_expanded {
            icons::minimize2("w-3.5 h-3.5")
        } else {
            icons::maximize2("w-3.5 h-3.5")
        };
        format!(
            r#"<div class="flex items-center gap-1">
                <button type="button" data-action="mobile-back-middle" class="lg:hidden w-7 h-7 flex items-center justify-center rounded-full text-zinc-400 hover:text-zinc-900 hover:bg-zinc-100 transition-all" title="Habere Dön">{back_ico}</button>
                <button type="button" data-action="toggle-expand-blocks" title="{exp_title}" class="w-7 h-7 flex items-center justify-center rounded-full transition-all {exp_cls}">{exp_ico}</button>
            </div>"#,
            back_ico = icons::arrow_left("w-3.5 h-3.5"),
        )
    } else {
        format!(
            r#"<button type="button" data-action="wizard-prev" class="w-7 h-7 flex items-center justify-center rounded-full text-zinc-400 hover:text-zinc-900 hover:bg-zinc-100 transition-all" title="Geri Dön">{}</button>"#,
            icons::arrow_left("w-3.5 h-3.5")
        )
    };

    let mut steps_dots = String::new();
    for n in 1u8..=4u8 {
        let dot_cls = if step == n {
            "bg-brand-500 text-white"
        } else if step > n {
            "bg-zinc-900 text-white"
        } else {
            "bg-zinc-100 text-zinc-400"
        };
        let line_html = if n < 4 {
            let l_cls = if step > n { "bg-zinc-900" } else { "bg-zinc-100" };
            format!(r#"<div class="w-3 h-[1px] {l_cls}"></div>"#)
        } else {
            String::new()
        };
        let dot_title = match n {
            1 => "1. İçerik Taslağı",
            2 => "2. Görsel Optimizasyon",
            3 => "3. SEO Ayarları",
            _ => "4. Yayınla",
        };
        steps_dots.push_str(&format!(
            r#"<div class="flex items-center">
                <button type="button" data-action="set-wizard-step" data-step="{n}" title="{dot_title}" class="w-7 h-7 rounded-full flex items-center justify-center text-xs font-black cursor-pointer transition-all hover:scale-110 {dot_cls}">{n}</button>
                {line_html}
            </div>"#
        ));
    }

    let step_title = match step {
        1 => "1. İçerik Taslağı",
        2 => "2. Görsel Optimizasyon",
        3 => "3. SEO Ayarları",
        _ => "4. Yayınla",
    };

    let canon_header_title = s.canonical_editing_article().title;
    let rev_header_btn = if !s.editing_article.id.trim().is_empty() {
        format!(
            r#"<button type="button" data-action="open-revisions-modal" data-article-id="{aid}" data-article-title="{atitle}" title="Revizyon Geçmişi" class="min-h-[36px] px-3 flex items-center gap-1.5 rounded-[10px] bg-zinc-100 text-zinc-700 hover:bg-brand-50 hover:text-brand-600 text-xs font-semibold transition-all">
                {hist_ico}
                <span class="hidden sm:inline">Revizyonlar</span>
            </button>"#,
            aid = escape_html(&s.editing_article.id),
            atitle = escape_html(&canon_header_title),
            hist_ico = icons::history("w-3.5 h-3.5"),
        )
    } else {
        String::new()
    };

    let step_body = match step {
        1 => render_step_1(s),
        2 => render_step_2(s),
        3 => render_step_3(s),
        _ => render_step_4(s),
    };

    let footer_html = if step < 4 {
        let next_dis =
            if (step == 1 && canon_header_title.trim().is_empty() && s.editing_article.title.trim().is_empty()) || s.is_cropping {
                "disabled"
            } else {
                ""
            };
        let next_lbl = if step == 2 && s.is_cropping {
            "İşleniyor..."
        } else {
            "İlerle"
        };
        format!(
            r#"<div class="min-h-[60px] px-6 py-3 border-t border-zinc-100 flex items-center justify-end bg-zinc-50 shrink-0 mt-auto">
                <button id="btn-wizard-next" type="button" data-action="wizard-next" {next_dis} class="px-6 min-h-[40px] bg-brand-500 text-white rounded-[10px] text-[13.5px] font-semibold hover:bg-brand-600 transition-all shadow-sm disabled:opacity-50">{next_lbl}</button>
            </div>"#
        )
    } else {
        String::new()
    };

    let is_dr = s.editing_article.id.trim().starts_with("draft-")
        || s.editing_article.article_type.as_deref() == Some("draft");
    let close_or_clear_btn = if !s.editing_article.id.trim().is_empty() {
        let (title_tip, label) = if is_dr {
            ("Taslağı Editörden Kapat (Taslaklar listenizde durur)", "Kapat")
        } else {
            ("Editörden Kapat (Yayındaki haberiniz silinmez)", "Kapat")
        };
        let delete_draft_btn = if is_dr && s.caps().can_delete {
            format!(
                r#"<button type="button" data-action="delete-editing-draft" title="Taslağı Kalıcı Olarak Sil (Veritabanından kaldırır)" class="w-9 h-9 flex items-center justify-center text-zinc-500 hover:text-red-500 hover:bg-red-50 rounded-[10px] transition-all">{del_ico}</button>"#,
                del_ico = icons::trash2("w-3.5 h-3.5"),
            )
        } else {
            String::new()
        };
        format!(
            r#"{delete_draft_btn}<button type="button" data-action="clear-editor" title="{title_tip}" class="min-h-[36px] px-3 flex items-center gap-1.5 rounded-[10px] bg-zinc-100 text-zinc-700 hover:bg-zinc-200 hover:text-zinc-900 text-xs font-semibold transition-all">
                {x_ico}
                <span class="hidden sm:inline">{label}</span>
            </button>"#,
            x_ico = icons::x_mark("w-3.5 h-3.5"),
        )
    } else {
        format!(
            r#"<button type="button" data-action="clear-editor" title="Editörü Temizle" class="w-9 h-9 flex items-center justify-center text-zinc-400 hover:text-red-500 hover:bg-red-50 rounded-[10px] transition-all">{trash_ico}</button>"#,
            trash_ico = icons::trash2("w-3.5 h-3.5"),
        )
    };

    format!(
        r#"<div class="flex-none p-3 lg:p-4 pb-2 z-10 bg-white">
            <div class="flex items-center justify-between flex-wrap gap-2 pb-2 border-b border-zinc-100 min-h-[40px]">
                <div class="flex items-center gap-2 min-w-0">
                    {left_nav_html}
                    <div class="flex items-center gap-1.5">
                        {steps_dots}
                        <h2 class="text-xs font-semibold text-zinc-500 ml-2 hidden sm:block">{step_title}</h2>
                    </div>
                </div>
                <div class="flex items-center gap-1.5 flex-wrap">
                    <button type="button" data-action="open-preview-modal" title="Haberi Tam Ekran Önizle" class="min-h-[36px] px-3 flex items-center gap-1.5 rounded-[10px] bg-zinc-900 text-white hover:bg-brand-500 text-xs font-semibold shadow-sm transition-all">
                        {eye_ico}
                        <span class="hidden sm:inline">Önizle</span>
                    </button>
                    {rev_header_btn}
                    {close_or_clear_btn}
                </div>
            </div>
            <div id="hf-editor-lock-slot">{lock_banner}</div>
        </div>
        <div id="right-scroll" class="flex-1 overflow-y-auto p-4 lg:p-5 custom-scrollbar flex flex-col">
            <div class="flex-1 flex flex-col space-y-3 max-w-4xl mx-auto w-full">
                {step_body}
            </div>
        </div>
        {footer_html}"#,
        eye_ico = icons::eye("w-3.5 h-3.5"),
        lock_banner = render_editor_conflict_banner(s),
    )
}

pub fn render_blocks_count_badge_html(count: usize) -> String {
    if count > 0 {
        format!(r#"<span class="text-xs font-bold text-zinc-400">{count} Blok</span>"#)
    } else {
        String::new()
    }
}

pub fn render_attached_source_bar_html(s: &AppState) -> String {
    let Some(raw_src) = s.editing_article.effective_source_url() else {
        return String::new();
    };
    let Some(safe_src) = crate::editorial::wasm::render_research::safe_http_url(raw_src) else {
        return String::new();
    };
    let domain_lbl = crate::editorial::wasm::state::get_site_name(&safe_src);
    format!(
        r#"<div class="hf-attached-source-bar">
            <div class="hf-attached-source-left">
                {pin_ico}
                <span class="hf-attached-source-tag">{dom}</span>
                <a href="{u_esc}" target="_blank" rel="noopener noreferrer" class="hf-attached-source-link" title="{u_esc}">{u_esc}</a>
            </div>
            <button type="button" data-action="clear-research-source" class="hf-attached-source-clear" title="Kaynak Referansını Kaldır">×</button>
        </div>"#,
        pin_ico = icons::pin("w-3.5 h-3.5 text-brand-600 shrink-0"),
        dom = escape_html(&domain_lbl),
        u_esc = escape_html(&safe_src),
    )
}

pub fn render_step1_glossary_compliance_html(s: &AppState) -> String {
    if s.editor_lang != "en" {
        return String::new();
    }
    let (violations, has_any_text) = s.evaluate_active_en_glossary();
    if !violations.is_empty() {
        let mut items = String::new();
        for v in &violations {
            let occ_badge = if v.occurrences > 1 {
                format!(r#" <span class="opacity-70">(×{})</span>"#, v.occurrences)
            } else {
                String::new()
            };
            let tip = if v.note.trim().is_empty() {
                String::new()
            } else {
                format!(r#" title="{}""#, escape_html(&v.note))
            };
            items.push_str(&format!(
                r#"<div class="hf-glossary-violation-chip"{tip}>
                    <span class="hf-gv-wrong">"{wrong}"</span>
                    <span class="text-amber-600 font-black">→</span>
                    <span class="hf-gv-right">"{right}"</span>{occ_badge}
                </div>"#,
                wrong = escape_html(&v.matched_text),
                right = escape_html(&v.preferred_replacement),
            ));
        }
        format!(
            r#"<div class="hf-glossary-alert-card">
                <div class="flex items-center justify-between gap-2 flex-wrap">
                    <div class="flex items-center gap-1.5 text-xs font-black uppercase tracking-widest text-amber-900">
                        {alrt_ico}
                        <span>MSI TDR &amp; NATO Terminoloji Uyarısı ({cnt})</span>
                    </div>
                    <button type="button" data-action="apply-glossary-fixes" class="hf-glossary-fix-btn">
                        {zap_ico}
                        <span>Tüm Terminolojiyi Otomatik Düzelt</span>
                    </button>
                </div>
                <div class="flex items-center flex-wrap gap-1.5">
                    {items}
                </div>
            </div>"#,
            alrt_ico = icons::alert_circle("w-3.5 h-3.5 text-amber-600 shrink-0"),
            zap_ico = icons::sparkles("w-3 h-3"),
            cnt = violations.len(),
        )
    } else if has_any_text {
        format!(
            r#"<div class="hf-glossary-ok-badge">
                {chk_ico}
                <span>✓ MSI TDR &amp; NATO Terminoloji Uyumlu (British English)</span>
            </div>"#,
            chk_ico = icons::shield_check("w-3.5 h-3.5 text-emerald-600 shrink-0"),
        )
    } else {
        String::new()
    }
}

pub fn render_step1_glossary_drawer_list_html(s: &AppState) -> String {
    let q = turkish_lowercase(s.glossary_drawer_query.trim());
    let filtered: Vec<_> = s
        .glossary_rules
        .iter()
        .filter(|r| {
            if q.is_empty() {
                return true;
            }
            turkish_lowercase(&r.term_tr).contains(&q)
                || r.abbr_tr
                    .as_deref()
                    .map_or(false, |a| turkish_lowercase(a).contains(&q))
                || turkish_lowercase(&r.preferred_en).contains(&q)
                || r.abbr_en
                    .as_deref()
                    .map_or(false, |a| turkish_lowercase(a).contains(&q))
                || r.forbidden_variants
                    .iter()
                    .any(|f| turkish_lowercase(f).contains(&q))
        })
        .take(30)
        .collect();

    if filtered.is_empty() {
        return r#"<div class="text-xs font-semibold text-zinc-400 py-3 text-center">Eşleşen terminoloji kuralı bulunamadı.</div>"#.to_string();
    }

    let mut rows = String::new();
    for r in filtered {
        let tr_lbl = match r.abbr_tr.as_deref().filter(|a| !a.trim().is_empty()) {
            Some(ab) => format!("{} ({})", escape_html(&r.term_tr), escape_html(ab)),
            None => escape_html(&r.term_tr),
        };
        let forb_html = if r.forbidden_variants.is_empty() {
            String::new()
        } else {
            let joined = r
                .forbidden_variants
                .iter()
                .map(|v| escape_html(v.trim()))
                .collect::<Vec<_>>()
                .join(", ");
            format!(r#"<span class="hf-glossary-forb-hint">Yasaklı: {joined}</span>"#)
        };
        rows.push_str(&format!(
            r#"<div class="hf-glossary-drawer-row">
                <div class="min-w-0 flex-1 flex items-center flex-wrap gap-1.5 text-xs">
                    <span class="font-bold text-zinc-700">{tr_lbl}</span>
                    <span class="text-zinc-400 font-black">→</span>
                    <span class="font-extrabold text-emerald-700">{pref}</span>
                </div>
                {forb_html}
            </div>"#,
            pref = escape_html(&r.preferred_en),
        ));
    }
    rows
}

fn render_step_1(s: &AppState) -> String {
    let blocks = s.editing_article.blocks.as_deref().unwrap_or(&[]);
    let count_badge = render_blocks_count_badge_html(blocks.len());
    let spacing_cls = if s.all_blocks_expanded {
        "gap-2.5"
    } else {
        "gap-1"
    };
    let spot_val = s.editing_article.spot.as_deref().unwrap_or("");
    let is_live = s.editing_article.is_live_blog;
    let live_btn_cls = if is_live {
        "hf-live-toggle-btn hf-live-toggle-active"
    } else {
        "hf-live-toggle-btn hf-live-toggle-idle"
    };

    let is_en = s.editor_lang == "en";
    let canon = s.canonical_editing_article();
    let has_en = article_has_english(&canon);
    let en_status = canon.en_status.as_str();

    let tr_tab_cls = if !is_en {
        "hf-lang-pill hf-lang-pill-active"
    } else {
        "hf-lang-pill hf-lang-pill-idle"
    };
    let en_tab_cls = if is_en {
        "hf-lang-pill hf-lang-pill-active-en"
    } else {
        "hf-lang-pill hf-lang-pill-idle"
    };
    let en_dot_badge = if en_status == "approved" {
        r#"<span class="hf-en-mini-badge hf-en-mini-approved">✓</span>"#
    } else if en_status == "review" {
        r#"<span class="hf-en-mini-dot bg-amber-500" title="İncelemede"></span>"#
    } else if has_en || en_status == "draft" {
        r#"<span class="hf-en-mini-dot bg-sky-500" title="Çeviri Taslağı Mevcut"></span>"#
    } else {
        ""
    };

    let glossary_btn_cls = if s.show_glossary_drawer {
        "hf-glossary-ref-btn hf-glossary-ref-btn-active"
    } else {
        "hf-glossary-ref-btn"
    };

    let en_right_controls = if is_en {
        let sel_none = if en_status == "none" || en_status.is_empty() {
            "selected"
        } else {
            ""
        };
        let sel_draft = if en_status == "draft" { "selected" } else { "" };
        let sel_rev = if en_status == "review" { "selected" } else { "" };
        let sel_app = if en_status == "approved" { "selected" } else { "" };
        format!(
            r#"<select data-change="article-en-status" class="hf-en-status-select" title="İngilizce (MSI TDR) Çeviri Durumu">
                <option value="none" {sel_none}>Yok (none)</option>
                <option value="draft" {sel_draft}>Çeviri Taslağı (draft)</option>
                <option value="review" {sel_rev}>İncelemede (review)</option>
                <option value="approved" {sel_app}>Onaylı (approved)</option>
            </select>
            <button type="button" data-action="copy-tr-to-en-assisted" class="hf-tr-assist-btn" title="Türkçe başlık, spot ve blokları TSK/SSB/NATO sözlük karşılıklarıyla İngilizce editöre aktarır">
                {lang_ico}
                <span>TR'den Aktar (Sözlük Destekli)</span>
            </button>"#,
            lang_ico = icons::languages("w-3 h-3"),
        )
    } else {
        String::new()
    };

    let glossary_drawer_html = if s.show_glossary_drawer {
        format!(
            r#"<div class="hf-glossary-drawer">
                <div class="flex items-center justify-between gap-2 pb-2 border-b border-zinc-200/70">
                    <div class="flex items-center gap-1.5 text-xs font-black uppercase tracking-widest text-zinc-700">
                        {book_ico}
                        <span>MSI TDR &amp; NATO Resmi Terminoloji Rehberi ({cnt})</span>
                    </div>
                    <button type="button" data-action="open-settings-glossary" class="text-xs font-black uppercase tracking-wider text-brand-600 hover:underline">Sözlüğü Yönet</button>
                </div>
                <div class="relative">
                    {search_ico}
                    <input id="hf-step1-glossary-search" data-input="glossary-drawer-query" type="text" value="{gq}" placeholder="Kurum, kısaltma veya terim ara (Örn: SSB, MSB, Hv.K.K., KAAN)..." class="w-full h-8 bg-white border border-zinc-200 rounded-lg pl-8 pr-3 text-[11px] font-semibold text-zinc-800 focus:outline-none focus:border-brand-500" />
                </div>
                <div id="hf-step1-glossary-list-slot" class="hf-glossary-drawer-list custom-scrollbar">
                    {g_list}
                </div>
            </div>"#,
            book_ico = icons::book_open("w-3.5 h-3.5 text-brand-600"),
            cnt = s.glossary_rules.len(),
            search_ico = icons::search("absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-zinc-400"),
            gq = escape_html(&s.glossary_drawer_query),
            g_list = render_step1_glossary_drawer_list_html(s),
        )
    } else {
        String::new()
    };

    // Collapsible Author / Interview Attribution & Video Bar
    let author_name_val = s.editing_article.author_name.as_deref().unwrap_or("");
    let author_title_val = s.editing_article.author_title.as_deref().unwrap_or("");
    let video_url_val = s.editing_article.video_url.as_deref().unwrap_or("");
    let has_author_or_video = !author_name_val.trim().is_empty()
        || !author_title_val.trim().is_empty()
        || !video_url_val.trim().is_empty();
    let author_bar_open = s.show_author_meta_bar || has_author_or_video;
    let author_summary_pill = if !author_name_val.trim().is_empty() {
        format!(
            r#"<span class="hf-author-mini-pill">{}</span>"#,
            escape_html(author_name_val.trim())
        )
    } else if !video_url_val.trim().is_empty() {
        r#"<span class="hf-author-mini-pill">Video Eklendi ✓</span>"#.to_string()
    } else {
        String::new()
    };
    let author_meta_body = if author_bar_open {
        format!(
            r#"<div class="hf-author-meta-body">
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                    <div class="space-y-0.5">
                        <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-0.5">Yazar / Söyleşi Konuğu</label>
                        <input data-input="article-author-name" type="text" value="{aname}" placeholder="Örn: Dr. Kemal Aydın / TUSAŞ Gn. Md." class="hf-meta-input bg-white" />
                    </div>
                    <div class="space-y-0.5">
                        <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-0.5">Ünvan / Görev</label>
                        <input data-input="article-author-title" type="text" value="{atitle}" placeholder="Örn: Savunma Sanayii Analisti" class="hf-meta-input bg-white" />
                    </div>
                </div>
                <div class="space-y-0.5">
                    <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-0.5 flex items-center gap-1">{vid_ico} Video / Röportaj URL (Opsiyonel)</label>
                    <input data-input="article-video-url" type="url" value="{vurl}" placeholder="https://youtube.com/watch?v=..." class="hf-meta-input bg-white" />
                </div>
            </div>"#,
            aname = escape_html(author_name_val),
            atitle = escape_html(author_title_val),
            vurl = escape_html(video_url_val),
            vid_ico = icons::video("w-3 h-3 text-rose-500"),
        )
    } else {
        String::new()
    };

    let (title_lbl, title_ph, spot_lbl, spot_ph, body_lbl) = if is_en {
        (
            "English Headline (MSI TDR)",
            "Enter English Headline for MSI TDR...",
            "English Standfirst / Summary",
            "Write 1-2 sentence English standfirst / summary...",
            "English Body (MSI TDR)",
        )
    } else {
        (
            "Haber Başlığı",
            "Ana Haber Başlığını Buraya Girin...",
            "Haber Spotu (Özet Giriş)",
            "Haber spotunu (1-2 cümlelik özet giriş) buraya yazın...",
            "Haber İçeriği",
        )
    };

    let live_quick_box = if is_live {
        let q_time = if s.live_quick_time.trim().is_empty() {
            js_now_hhmm()
        } else {
            s.live_quick_time.clone()
        };
        format!(
            r#"<div class="hf-live-quick-card">
                <div class="flex items-center justify-between gap-2">
                    <div class="flex items-center gap-2 text-xs font-black uppercase tracking-widest text-red-700">
                        <span class="hf-live-dot"></span>
                        <span>Dakika Dakika Gelişme Ekle</span>
                    </div>
                    <span class="text-xs font-bold text-red-500/80">En üste anında eklenir</span>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-2">
                    <input id="hf-live-quick-time" data-input="live-quick-time" type="text" value="{qt}" placeholder="14:30" class="hf-live-time-input" />
                    <input id="hf-live-quick-title" data-input="live-quick-title" type="text" value="{qtitle}" placeholder="Kısa başlık (opsiyonel, örn: İlk Açıklama)" class="hf-live-title-input sm:col-span-2" />
                </div>
                <textarea id="hf-live-quick-body" data-input="live-quick-body" rows="2" placeholder="Canlı gelişme metnini buraya yazın..." class="hf-live-body-textarea">{qbody}</textarea>
                <div class="flex justify-end">
                    <button type="button" data-action="prepend-live-entry" class="hf-live-submit-btn">
                        {plus_ico}
                        <span>+ Canlı Gelişme Ekle</span>
                    </button>
                </div>
            </div>"#,
            qt = escape_html(&q_time),
            qtitle = escape_html(&s.live_quick_title),
            qbody = escape_html(&s.live_quick_body),
            plus_ico = icons::radio("w-3.5 h-3.5"),
        )
    } else {
        String::new()
    };

    format!(
        r#"<div class="flex flex-col gap-2.5">
            <div class="hf-bilingual-bar">
                <div class="hf-lang-segmented">
                    <button type="button" data-action="set-editor-lang" data-lang="tr" class="{tr_tab_cls}">
                        <span>🇹🇷 Türkçe (MSI Dergisi / Haber)</span>
                    </button>
                    <button type="button" data-action="set-editor-lang" data-lang="en" class="{en_tab_cls}">
                        <span>🇬🇧 English (MSI TDR)</span>
                        {en_dot_badge}
                    </button>
                </div>
                <div class="flex items-center gap-1.5 flex-wrap">
                    {en_right_controls}
                    <button type="button" data-action="toggle-glossary-drawer" class="{glossary_btn_cls}" title="TSK / SSB / NATO Terminoloji Rehberi">
                        {book_ico}
                        <span>Sözlük Rehberi</span>
                    </button>
                </div>
            </div>

            {glossary_drawer_html}

            <div id="hf-glossary-compliance-slot">{compliance_html}</div>

            <div class="space-y-0.5 relative group/title">
                <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-1">{title_lbl}</label>
                <textarea id="editor-title-textarea" data-input="editor-title" rows="1" placeholder="{title_ph}" class="w-full text-xl lg:text-2xl font-bold text-zinc-900 focus:outline-none bg-transparent placeholder-zinc-200 resize-none overflow-hidden leading-tight transition-all duration-200">{title}</textarea>
            </div>

            <div class="space-y-1">
                <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-1">{spot_lbl}</label>
                <textarea id="editor-spot-textarea" data-input="editor-spot" rows="2" placeholder="{spot_ph}" class="hf-spot-textarea">{spot}</textarea>
            </div>

            <div class="hf-author-meta-card">
                <button type="button" data-action="toggle-author-meta-bar" class="hf-author-meta-toggle">
                    <div class="flex items-center gap-2 min-w-0">
                        {user_ico}
                        <span class="text-xs font-black uppercase tracking-widest text-zinc-600">Yazar / Söyleşi Künyesi &amp; Video</span>
                        {author_summary_pill}
                    </div>
                    {chev_ico}
                </button>
                {author_meta_body}
            </div>

            <div id="hf-attached-source-slot">{attached_source_html}</div>

            <div class="flex-1 flex flex-col space-y-2 mt-1">
                <div class="flex items-center justify-between px-1 gap-2 flex-wrap">
                    <div class="flex items-center gap-2 flex-wrap">
                        <label class="text-xs font-black uppercase tracking-widest text-zinc-400">{body_lbl}</label>
                        <button type="button" data-action="toggle-live-blog" class="{live_btn_cls}" title="Canlı Anlatım (Dakika Dakika) Modunu Aç/Kapat">
                            <span class="hf-live-dot"></span>
                            <span>CANLI ANLATIM</span>
                        </button>
                        <span class="hidden sm:inline text-xs font-semibold text-zinc-300">• Metni seçerek biçimlendirme baloncuğunu açın</span>
                    </div>
                    <div id="blocks-count-badge">{count_badge}</div>
                </div>"#,
        book_ico = icons::book_open("w-3 h-3"),
        compliance_html = render_step1_glossary_compliance_html(s),
        title = escape_html(&s.editing_article.title),
        spot = escape_html(spot_val),
        user_ico = icons::user("w-3.5 h-3.5 text-brand-500 shrink-0"),
        chev_ico = icons::chevron_down(if author_bar_open {
            "w-3.5 h-3.5 text-zinc-500 rotate-180 transition-transform"
        } else {
            "w-3.5 h-3.5 text-zinc-400 transition-transform"
        }),
        attached_source_html = render_attached_source_bar_html(s),
    ) + &format!(
        r#"
                {live_quick_box}

                <div class="flex-1 flex flex-col bg-white">
                    <div id="blocks-list-container" class="flex flex-col {spacing_cls}">
                        {blocks_html}
                    </div>
                </div>

                <div class="hf-block-addition-bar mt-3 pt-3 border-t border-zinc-100 border-dashed space-y-2">
                    <div class="flex items-center flex-wrap gap-1.5 justify-center">
                        <button type="button" data-action="add-block" data-block-type="paragraph" class="hf-add-pill hf-add-pill-primary">
                            {plus_ico} Metin
                        </button>
                        <button type="button" data-action="add-block" data-block-type="heading1" class="hf-add-pill hf-add-pill-primary">
                            {plus_ico} H1
                        </button>
                        <button type="button" data-action="add-block" data-block-type="heading" class="hf-add-pill hf-add-pill-primary">
                            {plus_ico} H2
                        </button>
                        <button type="button" data-action="add-block" data-block-type="heading3" class="hf-add-pill hf-add-pill-primary">
                            {plus_ico} H3
                        </button>
                        <button type="button" data-action="add-block" data-block-type="image" class="hf-add-pill hf-add-pill-primary">
                            {plus_ico} Görsel
                        </button>
                        <button type="button" data-action="add-block" data-block-type="gallery" class="hf-add-pill hf-add-pill-primary">
                            {gal_ico} + Foto Galeri
                        </button>
                        <button type="button" data-action="add-block" data-block-type="related_article" class="hf-add-pill hf-add-pill-primary">
                            {rel_ico} + İlgili Haber
                        </button>
                        <button type="button" data-action="add-block" data-block-type="quote" class="hf-add-pill hf-add-pill-primary">
                            {quote_ico} Alıntı
                        </button>
                        <button type="button" data-action="add-block" data-block-type="bullet_list" class="hf-add-pill hf-add-pill-primary">
                            {list_ico} Liste
                        </button>
                    </div>
                    <div class="flex items-center flex-wrap gap-1.5 justify-center">
                        <button type="button" data-action="add-block" data-block-type="numbered_list" class="hf-add-pill hf-add-pill-secondary">
                            {num_ico} 1. Sıralı Liste
                        </button>
                        <button type="button" data-action="add-block" data-block-type="callout" class="hf-add-pill hf-add-pill-secondary">
                            {zap_ico} Bilgi / Son Dakika
                        </button>
                        <button type="button" data-action="add-block" data-block-type="summary" class="hf-add-pill hf-add-pill-secondary">
                            {star_ico} Öne Çıkanlar Özeti
                        </button>
                        <button type="button" data-action="add-block" data-block-type="embed" class="hf-add-pill hf-add-pill-secondary">
                            {play_ico} Video / Tweet Göm
                        </button>
                        <button type="button" data-action="add-block" data-block-type="divider" class="hf-add-pill hf-add-pill-secondary">
                            {minus_ico} Ayırıcı Çizgi
                        </button>
                        <button type="button" data-action="add-block" data-block-type="code" class="hf-add-pill hf-add-pill-secondary">
                            {code_ico} Kod Kutusu
                        </button>
                    </div>
                </div>
            </div>
        </div>"#,
        blocks_html = render_blocks_list_html(s),
        plus_ico = icons::plus("w-3 h-3"),
        gal_ico = icons::images("w-3 h-3 text-brand-500"),
        rel_ico = icons::newspaper("w-3 h-3 text-brand-500"),
        quote_ico = icons::quote("w-3 h-3"),
        list_ico = icons::list("w-3 h-3"),
        num_ico = icons::list_ordered("w-3 h-3"),
        zap_ico = icons::zap("w-3 h-3 text-amber-500"),
        star_ico = icons::sparkles("w-3 h-3 text-brand-500"),
        play_ico = icons::play_circle("w-3 h-3 text-rose-500"),
        minus_ico = icons::minus("w-3 h-3"),
        code_ico = icons::code2("w-3 h-3"),
    )
}

pub fn render_related_search_results_html(s: &AppState, block_id: &str) -> String {
    let q = s
        .related_search_queries
        .get(block_id)
        .map(|v| turkish_lowercase(v.trim()))
        .unwrap_or_default();
    let cur_id = s.editing_article.id.as_str();

    let matches: Vec<_> = s
        .published_articles
        .iter()
        .filter(|a| a.id != cur_id && !a.title.trim().is_empty())
        .filter(|a| q.is_empty() || turkish_lowercase(&a.title).contains(&q))
        .take(6)
        .collect();

    if matches.is_empty() {
        return r#"<div class="text-xs font-semibold text-zinc-400 py-2 px-1">Eşleşen yayınlanmış haber bulunamadı.</div>"#.to_string();
    }

    let mut rows = String::new();
    let bid_esc = escape_html(block_id);
    for art in matches {
        let thumb = art
            .thumb_url
            .as_deref()
            .or_else(|| art.effective_image_url());
        let thumb_html = render_news_image(thumb, &art.title, "w-full h-full object-cover");
        rows.push_str(&format!(
            r#"<button type="button" data-action="pick-related-article" data-block-id="{bid_esc}" data-article-id="{aid}" class="hf-related-pick-item">
                <div class="hf-related-pick-thumb">{thumb_html}</div>
                <div class="min-w-0 flex-1 text-left">
                    <div class="text-[11px] font-bold text-zinc-800 line-clamp-1">{title}</div>
                </div>
                <span class="text-xs font-black uppercase tracking-wider text-brand-600 shrink-0">Ekle</span>
            </button>"#,
            aid = escape_html(&art.id),
            title = escape_html(&art.title),
        ));
    }

    format!(r#"<div class="hf-related-pick-list">{rows}</div>"#)
}

pub fn render_blocks_list_html(s: &AppState) -> String {
    let blocks = s.editing_article.blocks.as_deref().unwrap_or(&[]);
    let drag_id = s.dragging_block_id.as_deref().unwrap_or("");
    let mut out = String::new();
    for block in blocks {
        out.push_str(&render_single_block(
            s,
            block,
            s.all_blocks_expanded,
            drag_id == block.id,
        ));
    }
    out
}

pub fn render_block_image_preview_inner(content: &str) -> String {
    if !content.trim().is_empty() {
        render_news_image(Some(content), "", "w-full h-full object-cover")
    } else {
        format!(
            r#"<div class="text-zinc-300 flex flex-col items-center gap-2">
                {}
                <span class="text-xs font-black uppercase tracking-widest">Görsel Seçilmedi</span>
            </div>"#,
            icons::plus("w-8 h-8 opacity-20")
        )
    }
}

/// Extracts raw URL from `<iframe src="...">` or `<blockquote ... href="...">` if the user pasted embed HTML.
pub fn normalize_raw_embed_input(input: &str) -> String {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("<iframe") {
        for pat in ["src=\"", "src='"] {
            if let Some(pos) = lower.find(pat) {
                let quote = pat.chars().last().unwrap_or('"');
                let rest = &trimmed[pos + pat.len()..];
                if let Some(end) = rest.find(quote) {
                    let u = rest[..end].trim();
                    if let Some(pr) = u.strip_prefix("//") {
                        return format!("https://{pr}");
                    }
                    return u.replace("&amp;", "&");
                }
            }
        }
    }
    if lower.contains("twitter-tweet") || lower.contains("x.com/") || lower.contains("twitter.com/")
    {
        if trimmed.contains('<') {
            // Extract `https://(x|twitter).com/.../status/DIGITS` from pasted blockquote HTML
            for quote in ['"', '\''] {
                let mut rest = trimmed;
                while let Some(pos) = rest.find("http") {
                    let cand_slice = &rest[pos..];
                    let end = cand_slice
                        .find([quote, ' ', '>', '<'])
                        .unwrap_or(cand_slice.len());
                    let cand = &cand_slice[..end];
                    if extract_tweet_id(cand).is_some() {
                        return cand.replace("&amp;", "&");
                    }
                    rest = &cand_slice[end..];
                }
            }
        }
    }
    if let Some(pr) = trimmed.strip_prefix("//") {
        return format!("https://{pr}");
    }
    trimmed.to_string()
}

/// Extracts a clean YouTube video ID (`watch?v=ID`, `youtu.be/ID`, `shorts/ID`, `live/ID`, `embed/ID`, `v/ID`).
pub fn extract_youtube_video_id(url: &str) -> Option<String> {
    let norm = normalize_raw_embed_input(url);
    let trimmed = norm.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("youtube.com")
        && !lower.contains("youtu.be")
        && !lower.contains("youtube-nocookie.com")
    {
        return None;
    }

    let raw_id = if let Some(pos) = lower.find("youtu.be/") {
        let after = &trimmed[pos + "youtu.be/".len()..];
        after.split(['?', '&', '/', '#']).next()
    } else if let Some(pos) = lower.find("watch?v=") {
        let after = &trimmed[pos + "watch?v=".len()..];
        after.split(['&', '#', '/']).next()
    } else if let Some(pos) = lower.find("&v=") {
        let after = &trimmed[pos + "&v=".len()..];
        after.split(['&', '#', '/']).next()
    } else if let Some(pos) = lower.find("/shorts/") {
        let after = &trimmed[pos + "/shorts/".len()..];
        after.split(['?', '&', '/', '#']).next()
    } else if let Some(pos) = lower.find("/live/") {
        let after = &trimmed[pos + "/live/".len()..];
        after.split(['?', '&', '/', '#']).next()
    } else if let Some(pos) = lower.find("/embed/") {
        let after = &trimmed[pos + "/embed/".len()..];
        after.split(['?', '&', '/', '#']).next()
    } else if let Some(pos) = lower.find("/v/") {
        let after = &trimmed[pos + "/v/".len()..];
        after.split(['?', '&', '/', '#']).next()
    } else {
        None
    }?;

    let id = raw_id.trim();
    if (6..=16).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Some(id.to_string())
    } else {
        None
    }
}

/// Extracts numeric X / Twitter post status ID (`x.com/.../status/12345`, `twitter.com/i/web/status/12345`, `Tweet.html?id=12345`).
pub fn extract_tweet_id(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("x.com/")
        && !lower.contains("twitter.com/")
        && !lower.contains("platform.twitter.com")
    {
        return None;
    }

    let raw_digits = if let Some(pos) = lower.find("/status/") {
        let after = &trimmed[pos + "/status/".len()..];
        after.split(['?', '&', '/', '#']).next()
    } else if let Some(pos) = lower.find("/statuses/") {
        let after = &trimmed[pos + "/statuses/".len()..];
        after.split(['?', '&', '/', '#']).next()
    } else if let Some(pos) = lower.find("id=") {
        let after = &trimmed[pos + "id=".len()..];
        after.split(['&', '#', '/', '"', '\'']).next()
    } else {
        None
    }?;

    let digits: String = raw_digits
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if (5..=25).contains(&digits.len()) {
        Some(digits)
    } else {
        None
    }
}

/// Extracts official Instagram iframe embed URL from `/p/{code}`, `/reel/{code}`, `/reels/{code}`, or `/tv/{code}`.
pub fn extract_instagram_embed_url(url: &str) -> Option<String> {
    let norm = normalize_raw_embed_input(url);
    let trimmed = norm.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("instagram.com/") {
        return None;
    }
    for (marker, canonical_kind) in [
        ("/reel/", "reel"),
        ("/reels/", "reel"),
        ("/p/", "p"),
        ("/tv/", "tv"),
    ] {
        if let Some(pos) = lower.find(marker) {
            let after = &trimmed[pos + marker.len()..];
            if let Some(code) = after.split(['/', '?', '&', '#']).next() {
                let c = code.trim();
                if (5..=32).contains(&c.len())
                    && c.chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                {
                    return Some(format!("https://www.instagram.com/{canonical_kind}/{c}/embed/"));
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtraVideoEmbed {
    DirectVideo(String),
    Vimeo(String),
    Dailymotion(String),
    TikTok(String),
}

pub fn extract_extra_video_embed(url: &str) -> Option<ExtraVideoEmbed> {
    let norm = normalize_raw_embed_input(url);
    let trimmed = norm.trim();
    let lower = trimmed.to_ascii_lowercase();

    // 1. Direct MP4 / WebM / OGG / MOV video file
    let path_only = lower.split(['?', '#']).next().unwrap_or("");
    if (lower.starts_with("https://")
        || lower.starts_with("http://")
        || trimmed.starts_with("/uploads/editorial/")
        || trimmed.starts_with("/uploads/"))
        && (path_only.ends_with(".mp4")
            || path_only.ends_with(".webm")
            || path_only.ends_with(".ogg")
            || path_only.ends_with(".mov"))
    {
        return Some(ExtraVideoEmbed::DirectVideo(trimmed.to_string()));
    }

    // 2. Vimeo (`vimeo.com/123456` or `player.vimeo.com/video/123456`)
    if lower.contains("vimeo.com/") {
        let after = if let Some(p) = lower.find("/video/") {
            &trimmed[p + "/video/".len()..]
        } else if let Some(p) = lower.find("vimeo.com/") {
            &trimmed[p + "vimeo.com/".len()..]
        } else {
            ""
        };
        let vid: String = after
            .split(['?', '&', '/', '#'])
            .next()
            .unwrap_or("")
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if (5..=15).contains(&vid.len()) {
            return Some(ExtraVideoEmbed::Vimeo(format!(
                "https://player.vimeo.com/video/{vid}"
            )));
        }
    }

    // 3. Dailymotion (`dailymotion.com/video/x8abcde` or `dai.ly/x8abcde` or `/embed/video/x8abcde`)
    if lower.contains("dailymotion.com/") || lower.contains("dai.ly/") {
        let after = if let Some(p) = lower.find("/video/") {
            &trimmed[p + "/video/".len()..]
        } else if let Some(p) = lower.find("dai.ly/") {
            &trimmed[p + "dai.ly/".len()..]
        } else {
            ""
        };
        let dm_id = after
            .split(['?', '&', '/', '#', '_'])
            .next()
            .unwrap_or("")
            .trim();
        if (4..=16).contains(&dm_id.len()) && dm_id.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Some(ExtraVideoEmbed::Dailymotion(format!(
                "https://www.dailymotion.com/embed/video/{dm_id}"
            )));
        }
    }

    // 4. TikTok (`tiktok.com/@user/video/123456` or `/embed/v2/123456`)
    if lower.contains("tiktok.com/") {
        let after = if let Some(p) = lower.find("/video/") {
            &trimmed[p + "/video/".len()..]
        } else if let Some(p) = lower.find("/v2/") {
            &trimmed[p + "/v2/".len()..]
        } else {
            ""
        };
        let tk_id: String = after
            .split(['?', '&', '/', '#'])
            .next()
            .unwrap_or("")
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if (8..=25).contains(&tk_id.len()) {
            return Some(ExtraVideoEmbed::TikTok(format!(
                "https://www.tiktok.com/embed/v2/{tk_id}"
            )));
        }
    }

    None
}

/// Checks if a plain standalone string (e.g. inside a `paragraph` block) is a single embeddable video/social URL.
pub fn detect_standalone_media_embed_url(text: &str) -> Option<String> {
    let t = text.trim();
    if t.is_empty() || t.contains(char::is_whitespace) {
        return None;
    }
    if extract_youtube_video_id(t).is_some()
        || extract_tweet_id(t).is_some()
        || extract_instagram_embed_url(t).is_some()
        || extract_extra_video_embed(t).is_some()
    {
        Some(t.to_string())
    } else {
        None
    }
}

/// Renders a live platform-aware embed player for Editor Step 1, Step 4 / Modal Preview, and Middle Reader.
pub fn render_embed_preview_card(url: &str, caption: Option<&str>) -> String {
    let normalized = normalize_raw_embed_input(url);
    let clean_url = normalized.trim();
    if clean_url.is_empty() {
        return r#"<div class="hf-embed-empty">Video veya sosyal medya bağlantısı girildiğinde önizleme burada görünür.</div>"#.to_string();
    }

    let lower = clean_url.to_ascii_lowercase();
    if !lower.starts_with("https://")
        && !lower.starts_with("http://")
        && !clean_url.starts_with("/uploads/editorial/")
        && !clean_url.starts_with("/uploads/")
    {
        return r#"<div class="hf-embed-empty">Geçerli bir https:// video veya sosyal medya bağlantısı girin.</div>"#
            .to_string();
    }

    let cap_html = caption
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(|c| format!(r#"<figcaption class="hf-figcaption">{}</figcaption>"#, escape_html(c)))
        .unwrap_or_default();

    let url_esc = escape_html(clean_url);

    // 1. YouTube (watch, shorts, live, youtu.be, embed)
    if let Some(yt_id) = extract_youtube_video_id(clean_url) {
        let yt_esc = escape_html(&yt_id);
        return format!(
            r#"<figure class="hf-embed-card">
                <div class="hf-embed-iframe-wrap">
                    <iframe src="https://www.youtube.com/embed/{yt_esc}?rel=0" title="YouTube video" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>
                </div>
                <div class="hf-embed-footer-link">
                    <a href="https://www.youtube.com/watch?v={yt_esc}" target="_blank" rel="noopener noreferrer">{play_ico} <span>YouTube'da İzle ↗</span></a>
                </div>
                {cap_html}
            </figure>"#,
            play_ico = icons::play_circle("w-3.5 h-3.5 text-rose-500"),
        );
    }

    // 2. X / Twitter Live Tweet Embed (`platform.twitter.com/embed/Tweet.html?id=...`)
    if let Some(tweet_id) = extract_tweet_id(clean_url) {
        let tid_esc = escape_html(&tweet_id);
        return format!(
            r#"<figure class="hf-embed-card hf-embed-tweet-card">
                <div class="hf-tweet-iframe-wrap">
                    <iframe data-tweet-id="{tid_esc}" src="https://platform.twitter.com/embed/Tweet.html?id={tid_esc}&lang=tr&dnt=true&theme=light" title="X (Twitter) Paylaşımı" allow="autoplay; fullscreen; clipboard-write; encrypted-media" referrerpolicy="no-referrer-when-downgrade" allowfullscreen></iframe>
                </div>
                <div class="hf-embed-footer-link">
                    <a href="{url_esc}" target="_blank" rel="noopener noreferrer">{tw_ico} <span>X (Twitter) Üzerinde Aç ↗</span></a>
                </div>
                {cap_html}
            </figure>"#,
            tw_ico = icons::twitter("w-3.5 h-3.5 text-zinc-800"),
        );
    }

    // 3. Instagram Live Post / Reel Embed
    if let Some(ig_embed_src) = extract_instagram_embed_url(clean_url) {
        let ig_esc = escape_html(&ig_embed_src);
        return format!(
            r#"<figure class="hf-embed-card hf-embed-ig-card">
                <div class="hf-ig-iframe-wrap">
                    <iframe src="{ig_esc}" title="Instagram Gönderisi" allow="autoplay; clipboard-write; encrypted-media; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>
                </div>
                <div class="hf-embed-footer-link">
                    <a href="{url_esc}" target="_blank" rel="noopener noreferrer">{ig_ico} <span>Instagram'da Aç ↗</span></a>
                </div>
                {cap_html}
            </figure>"#,
            ig_ico = icons::instagram("w-3.5 h-3.5 text-rose-500"),
        );
    }

    // 4. Direct Video (.mp4/.webm/.mov), Vimeo, Dailymotion, TikTok
    if let Some(extra) = extract_extra_video_embed(clean_url) {
        return match extra {
            ExtraVideoEmbed::DirectVideo(vsrc) => format!(
                r#"<figure class="hf-embed-card">
                    <div class="hf-embed-iframe-wrap">
                        <video controls preload="metadata" playsinline class="hf-direct-video-player" src="{v_esc}"></video>
                    </div>
                    {cap_html}
                </figure>"#,
                v_esc = escape_html(&vsrc),
            ),
            ExtraVideoEmbed::Vimeo(src) | ExtraVideoEmbed::Dailymotion(src) => format!(
                r#"<figure class="hf-embed-card">
                    <div class="hf-embed-iframe-wrap">
                        <iframe src="{s_esc}" title="Video Oynatıcı" allow="autoplay; fullscreen; picture-in-picture" allowfullscreen></iframe>
                    </div>
                    <div class="hf-embed-footer-link">
                        <a href="{url_esc}" target="_blank" rel="noopener noreferrer">{play_ico} <span>Kaynağında İzle ↗</span></a>
                    </div>
                    {cap_html}
                </figure>"#,
                s_esc = escape_html(&src),
                play_ico = icons::play_circle("w-3.5 h-3.5 text-brand-500"),
            ),
            ExtraVideoEmbed::TikTok(src) => format!(
                r#"<figure class="hf-embed-card hf-embed-tiktok-card">
                    <div class="hf-tiktok-iframe-wrap">
                        <iframe src="{s_esc}" title="TikTok Videosu" allow="autoplay; encrypted-media; fullscreen" allowfullscreen></iframe>
                    </div>
                    <div class="hf-embed-footer-link">
                        <a href="{url_esc}" target="_blank" rel="noopener noreferrer">{play_ico} <span>TikTok'ta Aç ↗</span></a>
                    </div>
                    {cap_html}
                </figure>"#,
                s_esc = escape_html(&src),
                play_ico = icons::play_circle("w-3.5 h-3.5 text-zinc-900"),
            ),
        };
    }

    // 5. Generic fallback card (e.g. profile URL without tweet status ID or unsupported external URL)
    let (platform_label, badge_cls, ico) =
        if lower.contains("x.com/") || lower.contains("twitter.com/") {
            (
                "X / TWITTER BAĞLANTISI",
                "hf-embed-badge-x",
                icons::twitter("w-4 h-4"),
            )
        } else if lower.contains("instagram.com/") {
            (
                "INSTAGRAM BAĞLANTISI",
                "hf-embed-badge-ig",
                icons::instagram("w-4 h-4"),
            )
        } else {
            (
                "HARİCİ MEDYA BAĞLANTISI",
                "hf-embed-badge-generic",
                icons::play_circle("w-4 h-4"),
            )
        };

    format!(
        r#"<figure class="hf-embed-card hf-embed-social-card">
            <div class="hf-embed-social-inner">
                <div class="hf-embed-badge {badge_cls}">
                    {ico}
                    <span>{platform_label}</span>
                </div>
                <a href="{url_esc}" target="_blank" rel="noopener noreferrer" class="hf-embed-link">{url_esc}</a>
            </div>
            {cap_html}
        </figure>"#
    )
}

fn parse_callout_meta(meta: Option<&str>) -> (&str, &str, &'static str) {
    let raw = meta.unwrap_or("info");
    let (variant_raw, custom_title) = raw.split_once('|').unwrap_or((raw, ""));
    let variant = match variant_raw.trim() {
        "breaking" => "breaking",
        "warning" => "warning",
        _ => "info",
    };
    let default_title = match variant {
        "breaking" => "SON DAKİKA",
        "warning" => "ÖNEMLİ NOT",
        _ => "BİLGİ KUTUSU",
    };
    (variant, custom_title.trim(), default_title)
}

fn render_block_type_select(id_esc: &str, current_type: &str) -> String {
    const OPTIONS: &[(&str, &str)] = &[
        ("paragraph", "Metin"),
        ("heading1", "H1 Başlık"),
        ("heading", "H2 Başlık"),
        ("heading3", "H3 Başlık"),
        ("image", "Görsel"),
        ("gallery", "Foto Galeri"),
        ("related_article", "İlgili Haber"),
        ("live_entry", "Canlı Gelişme"),
        ("quote", "Alıntı"),
        ("bullet_list", "Madde Listesi"),
        ("numbered_list", "Sıralı Liste"),
        ("callout", "Bilgi / Son Dakika"),
        ("summary", "Öne Çıkanlar"),
        ("embed", "Video / Medya"),
        ("divider", "Ayırıcı Çizgi"),
        ("code", "Kod Kutusu"),
    ];

    let normalized_cur = if OPTIONS.iter().any(|(k, _)| *k == current_type) {
        current_type
    } else {
        "paragraph"
    };

    let mut opts = String::new();
    for (val, lbl) in OPTIONS {
        let sel = if *val == normalized_cur {
            "selected"
        } else {
            ""
        };
        opts.push_str(&format!(r#"<option value="{val}" {sel}>{lbl}</option>"#));
    }

    format!(
        r#"<select data-change="change-block-type" data-block-id="{id_esc}" class="hf-block-type-select" title="Blok Tipini Değiştir">{opts}</select>"#
    )
}

fn render_single_block(
    s: &AppState,
    block: &ContentBlock,
    all_expanded: bool,
    is_dragging: bool,
) -> String {
    let drag_cls = if is_dragging { "hf-block-dragging" } else { "" };
    let compact_cls = if all_expanded { "" } else { "hf-block-compact" };
    let id_esc = escape_html(&block.id);
    let meta_val = block.meta.as_deref().unwrap_or("");
    let meta_esc = escape_html(meta_val);

    let field_html = match block.block_type.as_str() {
        "heading1" => {
            let inner = rich_block_editable_inner_html(&block.content);
            format!(
                r#"<div class="flex items-baseline gap-2 py-0.5">
                    <span class="hf-heading-badge hf-heading-badge-h1">H1</span>
                    <div id="blk-rich-{id_esc}" contenteditable="true" data-rich-block="true" data-input="rich-block" data-block-id="{id_esc}" data-block-kind="heading1" data-placeholder="Ana Başlık (H1)..." class="hf-rich-editable hf-rich-h1 flex-1 min-w-0">{inner}</div>
                </div>"#
            )
        }
        "heading" => {
            let inner = rich_block_editable_inner_html(&block.content);
            format!(
                r#"<div class="flex items-baseline gap-2 py-0.5">
                    <span class="hf-heading-badge">H2</span>
                    <div id="blk-rich-{id_esc}" contenteditable="true" data-rich-block="true" data-input="rich-block" data-block-id="{id_esc}" data-block-kind="heading" data-placeholder="Alt Başlık (H2)..." class="hf-rich-editable hf-rich-h2 flex-1 min-w-0">{inner}</div>
                </div>"#
            )
        }
        "heading3" => {
            let inner = rich_block_editable_inner_html(&block.content);
            format!(
                r#"<div class="flex items-baseline gap-2 py-0.5">
                    <span class="hf-heading-badge">H3</span>
                    <div id="blk-rich-{id_esc}" contenteditable="true" data-rich-block="true" data-input="rich-block" data-block-id="{id_esc}" data-block-kind="heading3" data-placeholder="Ara Başlık (H3)..." class="hf-rich-editable hf-rich-h3 flex-1 min-w-0">{inner}</div>
                </div>"#
            )
        }
        "image" => {
            let preview_inner = render_block_image_preview_inner(&block.content);
            format!(
                r#"<div class="space-y-2.5 py-1.5">
                    <div class="relative group/img aspect-video w-full bg-zinc-50 rounded-2xl overflow-hidden border border-zinc-100 flex items-center justify-center">
                        <div id="block-img-preview-{id_esc}" class="w-full h-full flex items-center justify-center">
                            {preview_inner}
                        </div>
                        <div class="absolute inset-0 bg-black/0 group-hover/img:bg-black/15 flex items-center justify-center gap-2 opacity-0 group-hover/img:opacity-100 transition-all">
                            <label class="bg-white px-4 py-2 rounded-full shadow-xl flex items-center gap-2 text-xs font-black uppercase tracking-widest text-zinc-800 cursor-pointer hover:bg-brand-50">
                                {plus_ico} Yükle
                                <input type="file" data-file="block-image" data-block-id="{id_esc}" class="hidden" accept="image/*" />
                            </label>
                            <button type="button" data-action="open-media-library" data-media-target="block:{id_esc}" class="bg-zinc-900 px-4 py-2 rounded-full shadow-xl flex items-center gap-2 text-xs font-black uppercase tracking-widest text-white hover:bg-brand-500 transition-all">
                                {arch_ico} Arşivden Seç
                            </button>
                        </div>
                    </div>
                    <div class="flex items-center gap-2 flex-wrap">
                        <input id="blk-in-{id_esc}" data-input="block-content" data-block-id="{id_esc}" data-block-kind="image" type="text" placeholder="Görsel URL (https://...)" value="{val}" class="hf-meta-input flex-1" />
                        <button type="button" data-action="open-media-library" data-media-target="block:{id_esc}" class="hf-inline-archive-btn">
                            {arch_ico} Arşivden Seç
                        </button>
                    </div>
                    <input data-input="block-meta" data-block-id="{id_esc}" data-block-kind="image" type="text" placeholder="Görsel Alt Yazısı / Kaynak (opsiyonel)" value="{meta_esc}" class="hf-meta-input" />
                </div>"#,
                plus_ico = icons::plus("w-3 h-3"),
                arch_ico = icons::images("w-3 h-3"),
                val = escape_html(&block.content),
            )
        }
        "gallery" => {
            let items: Vec<GalleryItem> =
                serde_json::from_str(block.content.trim()).unwrap_or_default();
            let mut items_html = String::new();
            if items.is_empty() {
                items_html.push_str(
                    r#"<div class="hf-gallery-empty">Henüz fotoğraf eklenmedi. Bilgisayardan çoklu fotoğraf yükleyin veya arşivden seçin.</div>"#,
                );
            } else {
                for (idx, it) in items.iter().enumerate() {
                    let thumb_src = it.thumb_url.as_deref().unwrap_or(&it.url);
                    let thumb_img =
                        render_news_image(Some(thumb_src), &it.caption, "w-full h-full object-cover");
                    items_html.push_str(&format!(
                        r#"<div class="hf-editor-gallery-row">
                            <span class="hf-gallery-num">{num}</span>
                            <div class="hf-editor-gallery-thumb">{thumb_img}</div>
                            <input data-input="gallery-caption" data-block-id="{id_esc}" data-item-idx="{idx}" type="text" value="{cap}" placeholder="Fotoğraf alt yazısı / açıklama..." class="hf-meta-input flex-1" />
                            <button type="button" data-action="remove-gallery-item" data-block-id="{id_esc}" data-item-idx="{idx}" class="w-7 h-7 flex items-center justify-center rounded-lg text-zinc-400 hover:text-red-500 hover:bg-red-50 transition-colors shrink-0" title="Fotoğrafı Kaldır">
                                {del_ico}
                            </button>
                        </div>"#,
                        num = idx + 1,
                        cap = escape_html(&it.caption),
                        del_ico = icons::trash2("w-3.5 h-3.5"),
                    ));
                }
            }

            format!(
                r#"<div class="hf-editor-gallery">
                    <div class="flex items-center justify-between gap-2 flex-wrap">
                        <span class="hf-list-badge">{gal_ico} FOTO GALERİ ({cnt})</span>
                        <div class="flex items-center gap-1.5 flex-wrap">
                            <label class="hf-gallery-upload-btn">
                                {plus_ico} Çoklu Fotoğraf Yükle
                                <input type="file" multiple accept="image/*" data-file="gallery-images" data-block-id="{id_esc}" class="hidden" />
                            </label>
                            <button type="button" data-action="open-media-library" data-media-target="gallery:{id_esc}" class="hf-gallery-archive-btn">
                                {gal_ico} Arşivden Görsel Ekle
                            </button>
                        </div>
                    </div>
                    <input data-input="block-meta" data-block-id="{id_esc}" data-block-kind="gallery" type="text" value="{meta_esc}" placeholder="Örn: Olay Yerinden Kareler" class="hf-meta-input" />
                    <div class="space-y-2">
                        {items_html}
                    </div>
                </div>"#,
                gal_ico = icons::images("w-3 h-3"),
                plus_ico = icons::plus("w-3 h-3"),
                cnt = items.len(),
            )
        }
        "related_article" => {
            let parsed: RelatedArticleRef =
                serde_json::from_str(block.content.trim()).unwrap_or_default();
            let is_searching = parsed.title.trim().is_empty()
                || s.editing_related_block_id.as_deref() == Some(block.id.as_str());
            let kicker_val = if meta_val.trim().is_empty() {
                "BUNU DA OKUYUN"
            } else {
                meta_val.trim()
            };

            if !is_searching {
                let thumb_html = parsed
                    .image_url
                    .as_deref()
                    .filter(|u| !u.trim().is_empty())
                    .map(|u| {
                        format!(
                            r#"<div class="hf-related-editor-thumb">{}</div>"#,
                            render_news_image(Some(u), &parsed.title, "w-full h-full object-cover")
                        )
                    })
                    .unwrap_or_default();
                format!(
                    r#"<div class="hf-editor-related">
                        <div class="flex items-center justify-between gap-2">
                            <input data-input="block-meta" data-block-id="{id_esc}" data-block-kind="related_article" type="text" value="{kicker}" placeholder="BUNU DA OKUYUN" class="hf-related-kicker-input" />
                            <button type="button" data-action="change-related-article" data-block-id="{id_esc}" class="text-xs font-black uppercase tracking-wider text-brand-600 hover:underline shrink-0">Değiştir</button>
                        </div>
                        <div class="flex items-center gap-3">
                            {thumb_html}
                            <div class="min-w-0 flex-1">
                                <div class="text-xs font-extrabold text-zinc-900 leading-snug">{rtitle}</div>
                                <div class="text-xs font-mono text-zinc-400 truncate">/icerik/{rslug}</div>
                            </div>
                        </div>
                    </div>"#,
                    kicker = escape_html(kicker_val),
                    rtitle = escape_html(&parsed.title),
                    rslug = escape_html(parsed.slug.as_deref().unwrap_or("")),
                )
            } else {
                let q_val = s
                    .related_search_queries
                    .get(&block.id)
                    .map(|v| v.as_str())
                    .unwrap_or("");
                let results_html = render_related_search_results_html(s, &block.id);
                format!(
                    r#"<div class="hf-editor-related">
                        <div class="flex items-center gap-2 text-xs font-black uppercase tracking-widest text-brand-600">
                            {news_ico}
                            <span>Site İçi İlgili Haber Kartı ("Bunu da Okuyun")</span>
                        </div>
                        <input id="hf-rel-search-{id_esc}" data-input="related-search" data-block-id="{id_esc}" type="text" value="{q}" placeholder="Yayınlanan haberlerde başlık ara..." class="hf-meta-input" />
                        <div id="hf-rel-results-{id_esc}">
                            {results_html}
                        </div>
                    </div>"#,
                    news_ico = icons::newspaper("w-3.5 h-3.5"),
                    q = escape_html(q_val),
                )
            }
        }
        "live_entry" => {
            let (time_raw, title_raw) = meta_val.split_once('|').unwrap_or((meta_val, ""));
            let inner = rich_block_editable_inner_html(&block.content);
            format!(
                r#"<div class="hf-editor-live-entry">
                    <div class="flex items-center gap-2">
                        <span class="hf-live-dot"></span>
                        <input data-input="live-entry-time" data-block-id="{id_esc}" type="text" value="{t_esc}" placeholder="14:35" class="hf-live-time-input w-20" />
                        <input data-input="live-entry-title" data-block-id="{id_esc}" type="text" value="{ttl_esc}" placeholder="Gelişme başlığı (opsiyonel)" class="hf-live-title-input flex-1" />
                    </div>
                    <div id="blk-rich-{id_esc}" contenteditable="true" data-rich-block="true" data-input="rich-block" data-block-id="{id_esc}" data-block-kind="live_entry" data-placeholder="Dakika dakika gelişme detayını yazın..." class="hf-rich-editable hf-rich-paragraph">{inner}</div>
                </div>"#,
                t_esc = escape_html(time_raw.trim()),
                ttl_esc = escape_html(title_raw.trim()),
            )
        }
        "quote" => {
            let inner = rich_block_editable_inner_html(&block.content);
            format!(
                r#"<div class="hf-editor-quote">
                    <div id="blk-rich-{id_esc}" contenteditable="true" data-rich-block="true" data-input="rich-block" data-block-id="{id_esc}" data-block-kind="quote" data-placeholder="Alıntı metnini buraya yazın..." class="hf-rich-editable hf-rich-quote">{inner}</div>
                    <input data-input="block-meta" data-block-id="{id_esc}" data-block-kind="quote" type="text" placeholder="— Alıntı Sahibi / Kaynak (opsiyonel)" value="{meta_esc}" class="hf-quote-cite-input" />
                </div>"#
            )
        }
        "bullet_list" => {
            let clean_lines = strip_all_html_tags(&block.content);
            format!(
                r#"<div class="hf-editor-list">
                    <div class="hf-list-header">
                        <span class="hf-list-badge">• MADDE LİSTESİ</span>
                        <span class="hf-list-hint">Her satır ayrı bir madde olarak yayınlanır</span>
                    </div>
                    <textarea id="blk-ta-{id_esc}" data-input="block-content" data-block-id="{id_esc}" data-block-kind="bullet_list" rows="3" placeholder="Birinci madde&#10;İkinci madde&#10;Üçüncü madde..." class="hf-list-textarea">{val}</textarea>
                </div>"#,
                val = escape_html(&clean_lines),
            )
        }
        "numbered_list" => {
            let clean_lines = strip_all_html_tags(&block.content);
            format!(
                r#"<div class="hf-editor-list">
                    <div class="hf-list-header">
                        <span class="hf-list-badge">1. SIRALI LİSTE</span>
                        <span class="hf-list-hint">Her satır numaralı bir adım olarak yayınlanır</span>
                    </div>
                    <textarea id="blk-ta-{id_esc}" data-input="block-content" data-block-id="{id_esc}" data-block-kind="numbered_list" rows="3" placeholder="Birinci adım&#10;İkinci adım&#10;Üçüncü adım..." class="hf-list-textarea">{val}</textarea>
                </div>"#,
                val = escape_html(&clean_lines),
            )
        }
        "callout" => {
            let (variant, custom_title, default_title) =
                parse_callout_meta(block.meta.as_deref());
            let inner = rich_block_editable_inner_html(&block.content);
            let p_info = if variant == "info" { "active" } else { "" };
            let p_brk = if variant == "breaking" { "active" } else { "" };
            let p_wrn = if variant == "warning" { "active" } else { "" };

            format!(
                r#"<div id="callout-box-{id_esc}" class="hf-editor-callout" data-variant="{variant}">
                    <div class="hf-callout-pills">
                        <button type="button" data-action="set-callout-variant" data-block-id="{id_esc}" data-variant="info" class="hf-callout-pill hf-pill-info {p_info}">Bilgi</button>
                        <button type="button" data-action="set-callout-variant" data-block-id="{id_esc}" data-variant="breaking" class="hf-callout-pill hf-pill-breaking {p_brk}">Son Dakika</button>
                        <button type="button" data-action="set-callout-variant" data-block-id="{id_esc}" data-variant="warning" class="hf-callout-pill hf-pill-warning {p_wrn}">Önemli Not</button>
                    </div>
                    <input data-input="callout-title" data-block-id="{id_esc}" data-callout-variant="{variant}" type="text" value="{ct_esc}" placeholder="{default_title}" class="hf-callout-title-input" />
                    <div id="blk-rich-{id_esc}" contenteditable="true" data-rich-block="true" data-input="rich-block" data-block-id="{id_esc}" data-block-kind="callout" data-placeholder="Kutu detay açıklamasını buraya yazın..." class="hf-rich-editable hf-rich-callout">{inner}</div>
                </div>"#,
                ct_esc = escape_html(custom_title),
            )
        }
        "summary" => {
            let heading_val = if meta_val.trim().is_empty() {
                "Öne Çıkanlar"
            } else {
                meta_val.trim()
            };
            let clean_lines = strip_all_html_tags(&block.content);
            format!(
                r#"<div class="hf-editor-summary">
                    <div class="hf-summary-header">
                        {star_ico}
                        <input data-input="block-meta" data-block-id="{id_esc}" data-block-kind="summary" type="text" value="{h_esc}" placeholder="Öne Çıkanlar" class="hf-summary-title-input" />
                    </div>
                    <textarea id="blk-ta-{id_esc}" data-input="block-content" data-block-id="{id_esc}" data-block-kind="summary" rows="3" placeholder="Haberin öne çıkan maddelerini alt alta yazın..." class="hf-list-textarea">{val}</textarea>
                    <span class="hf-list-hint">Her satır özet kartında ayrı bir madde olarak listelenir</span>
                </div>"#,
                star_ico = icons::sparkles("w-3.5 h-3.5 text-brand-500 shrink-0"),
                h_esc = escape_html(heading_val),
                val = escape_html(&clean_lines),
            )
        }
        "embed" => {
            let preview_card =
                render_embed_preview_card(&block.content, block.meta.as_deref());
            format!(
                r#"<div class="hf-editor-embed">
                    <div class="flex items-center gap-2">
                        {play_ico}
                        <input id="blk-in-{id_esc}" data-input="block-content" data-block-id="{id_esc}" data-block-kind="embed" type="url" placeholder="YouTube, X/Twitter veya Instagram bağlantısı yapıştırın..." value="{val}" class="hf-embed-url-input" />
                    </div>
                    <div id="block-embed-preview-{id_esc}">
                        {preview_card}
                    </div>
                    <input data-input="block-meta" data-block-id="{id_esc}" data-block-kind="embed" type="text" placeholder="Medya açıklaması / kaynak (opsiyonel)" value="{meta_esc}" class="hf-meta-input" />
                </div>"#,
                play_ico = icons::play_circle("w-4 h-4 text-brand-500 shrink-0"),
                val = escape_html(&block.content),
            )
        }
        "divider" => {
            r#"<div class="hf-editor-divider" aria-label="Ayırıcı Çizgi">
                <span class="hf-divider-line"></span>
                <span class="hf-divider-ornament">✦</span>
                <span class="hf-divider-line"></span>
            </div>"#
                .to_string()
        }
        "code" => {
            format!(
                r#"<div class="hf-editor-code">
                    <div class="hf-code-header">
                        {code_ico}
                        <span>KOD / VERİ KUTUSU</span>
                    </div>
                    <textarea id="blk-ta-{id_esc}" data-input="block-content" data-block-id="{id_esc}" data-block-kind="code" rows="3" spellcheck="false" placeholder="Kod parçacığı, tablo veya veri metni..." class="hf-code-textarea">{val}</textarea>
                </div>"#,
                code_ico = icons::code2("w-3.5 h-3.5 text-brand-400"),
                val = escape_html(&block.content),
            )
        }
        _ => {
            let inner = rich_block_editable_inner_html(&block.content);
            format!(
                r#"<div class="relative py-0.5">
                    <div id="blk-rich-{id_esc}" contenteditable="true" data-rich-block="true" data-input="rich-block" data-block-id="{id_esc}" data-block-kind="paragraph" data-placeholder="Metin ekleyin... (Biçimlendirmek için yazıyı seçin)" class="hf-rich-editable hf-rich-paragraph">{inner}</div>
                </div>"#
            )
        }
    };

    let type_select = render_block_type_select(&id_esc, &block.block_type);

    format!(
        r#"<div data-block-row="{id_esc}" class="hf-block-row relative group py-1.5 border-b border-zinc-100/80 last:border-0 flex items-start gap-2 hover:bg-zinc-50/40 px-2 -mx-2 rounded-xl transition-colors {compact_cls} {drag_cls}">
            <div class="flex flex-col items-center gap-1 mt-1 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity shrink-0">
                <div data-block-grip="{id_esc}" class="cursor-grab active:cursor-grabbing p-1 text-zinc-300 hover:text-zinc-600 hover:bg-zinc-100 rounded-md transition-all touch-none select-none" title="Sürükleyerek Sırala">
                    {grip_ico}
                </div>
                <button type="button" data-action="insert-block-below" data-block-id="{id_esc}" class="p-1 text-zinc-300 hover:text-brand-500 hover:bg-brand-50 rounded-md transition-all" title="Altına Metin Bloğu Ekle">
                    {plus_ico}
                </button>
            </div>
            <div class="flex-1 min-w-0">
                <div class="hf-block-toolbar">
                    {type_select}
                    <button type="button" data-action="duplicate-block" data-block-id="{id_esc}" class="hf-block-tool-btn" title="Bloğu Çoğalt">
                        {copy_ico}
                    </button>
                    <button type="button" data-action="delete-block" data-block-id="{id_esc}" class="hf-block-tool-btn hf-block-tool-danger" title="Bloğu Sil">
                        {trash_ico}
                    </button>
                </div>
                {field_html}
            </div>
        </div>"#,
        grip_ico = icons::grip_vertical("w-3.5 h-3.5"),
        plus_ico = icons::plus("w-3 h-3"),
        copy_ico = icons::copy("w-3 h-3"),
        trash_ico = icons::trash2("w-3 h-3"),
    )
}

fn render_step_2(s: &AppState) -> String {
    let current_img = s.editing_article.effective_image_url().unwrap_or("");
    let valid_img = is_valid_image_url(Some(current_img));
    let up_dis = if s.is_uploading { "disabled" } else { "" };

    let wm_toggle_cls = if s.upload_with_watermark {
        "hf-wm-toggle-active"
    } else {
        "hf-wm-toggle-idle"
    };

    let top_actions_html = if !current_img.is_empty() {
        format!(
            r#"<div class="flex items-center gap-2 flex-wrap">
                <button type="button" data-action="open-media-library" data-media-target="step2" class="hf-step2-archive-pill">
                    {arch_ico}
                    <span>Medya Arşivinden Seç</span>
                </button>
                <label class="text-xs font-black uppercase tracking-widest text-brand-500 cursor-pointer hover:underline flex items-center gap-1">
                    {ref_ico} Görseli Değiştir
                    <input type="file" data-file="step2-image" class="hidden" accept="image/*" />
                </label>
            </div>"#,
            arch_ico = icons::images("w-3 h-3"),
            ref_ico = icons::refresh_cw("w-3 h-3"),
        )
    } else {
        format!(
            r#"<button type="button" data-action="open-media-library" data-media-target="step2" class="hf-step2-archive-pill">
                {arch_ico}
                <span>Medya Arşivinden Seç</span>
            </button>"#,
            arch_ico = icons::images("w-3 h-3"),
        )
    };

    let stage_html = if !valid_img {
        let up_ico = if s.is_uploading {
            icons::loader2("w-3 h-3 animate-spin")
        } else {
            icons::send("w-3 h-3")
        };
        format!(
            r#"<div class="flex flex-col items-center gap-5 w-full h-full justify-center p-6">
                <div class="flex items-center gap-3 flex-wrap justify-center">
                    <label class="hf-step2-upload-card cursor-pointer group/upload">
                        <div class="w-12 h-12 bg-brand-50 rounded-2xl flex items-center justify-center text-brand-500 group-hover/upload:scale-105 transition-transform">
                            {plus_ico}
                        </div>
                        <span class="text-[11px] font-black uppercase tracking-wider text-zinc-900">Bilgisayardan Yükle</span>
                        <span class="text-xs font-semibold text-zinc-400">Otomatik WebP dönüşümü</span>
                        <input type="file" data-file="step2-image" class="hidden" accept="image/*" />
                    </label>
                    <button type="button" data-action="open-media-library" data-media-target="step2" class="hf-step2-upload-card group/arch">
                        <div class="w-12 h-12 bg-zinc-100 rounded-2xl flex items-center justify-center text-zinc-700 group-hover/arch:bg-brand-50 group-hover/arch:text-brand-500 transition-colors">
                            {arch_ico}
                        </div>
                        <span class="text-[11px] font-black uppercase tracking-wider text-zinc-900">Medya Arşivinden Seç</span>
                        <span class="text-xs font-semibold text-zinc-400">Hazır kapak görselleri</span>
                    </button>
                </div>

                <div class="w-full max-w-sm flex flex-col gap-1.5">
                    <div class="relative flex gap-2">
                        <div class="relative flex-1">
                            <input id="step2-remote-url-input" data-input="step2-remote-url" type="text" placeholder="Görsel URL'si yapıştırın..." value="{rem_val}" class="w-full h-10 bg-zinc-100 border border-transparent rounded-xl px-3.5 pr-10 text-xs font-bold focus:outline-none focus:bg-white focus:border-brand-500/20 transition-all" />
                            {globe_ico}
                        </div>
                        <button type="button" data-action="fetch-step2-url" {up_dis} class="h-10 px-4 bg-brand-500 text-white rounded-xl text-xs font-black uppercase tracking-widest hover:bg-brand-600 disabled:opacity-50 transition-all flex items-center gap-1.5">
                            {up_ico}
                            Çek
                        </button>
                    </div>
                </div>
            </div>"#,
            plus_ico = icons::plus("w-6 h-6"),
            arch_ico = icons::images("w-6 h-6"),
            globe_ico =
                icons::globe("absolute right-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-300"),
            rem_val = escape_html(&s.step2_remote_url),
        )
    } else {
        let cors_overlay = if s.editor_image_error {
            let up_ico = if s.is_uploading {
                icons::loader2("w-4 h-4 animate-spin")
            } else {
                icons::refresh_cw("w-4 h-4")
            };
            format!(
                r#"<div class="absolute inset-0 bg-black/40 backdrop-blur-sm flex flex-col items-center justify-center p-6 text-center z-20">
                    {alert_ico}
                    <h3 class="text-white font-black uppercase tracking-widest mb-2">Görsel Erişim Engellendi (CORS)</h3>
                    <p class="text-white/80 text-xs font-medium max-w-xs mb-6">
                        Dış kaynaklı bu görsel güvenlik politikaları nedeniyle doğrudan işlenemiyor. Sunucuya çekerek bu sorunu aşabilirsiniz.
                    </p>
                    <button type="button" data-action="fetch-cors-image" {up_dis} class="px-6 py-3 bg-brand-500 text-white rounded-2xl text-xs font-black uppercase tracking-widest hover:bg-brand-600 transition-all flex items-center gap-2 shadow-xl">
                        {up_ico}
                        Sunucuya Çek ve Düzenle
                    </button>
                </div>"#,
                alert_ico = icons::alert_circle("w-12 h-12 text-white mb-4 animate-bounce"),
            )
        } else {
            String::new()
        };

        format!(
            r#"<div class="hf-cropper-stage">
                <canvas id="hf-cropper-canvas" width="1200" height="675" class="hf-cropper-canvas"></canvas>
                <div class="hf-cropper-grid"></div>
                {cors_overlay}
                <div class="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-4 bg-black/60 backdrop-blur-md px-4 py-2 rounded-full z-10">
                    <input id="crop-zoom-slider" data-input="crop-zoom" type="range" value="{zoom:.2}" min="1" max="3" step="0.05" aria-label="Zoom" class="w-32 accent-brand-500" />
                    <button type="button" data-action="clear-step2-image" class="text-xs font-black uppercase text-white hover:text-red-400 transition-colors">Görseli Değiştir</button>
                </div>
            </div>"#,
            zoom = s.crop_zoom,
        )
    };

    let loaded_info_bar = if valid_img {
        let w = s.editing_article.cover_width;
        let h = s.editing_article.cover_height;
        let discover_badge = match (w, h) {
            (Some(cw), Some(ch)) if cw >= 1200 => format!(
                r#"<span class="hf-discover-res-badge hf-discover-res-ok">{chk} {cw}×{ch} px • WebP • Discover Uyumlu</span>"#,
                chk = icons::check_circle2("w-3.5 h-3.5"),
            ),
            (Some(cw), Some(ch)) => format!(
                r#"<span class="hf-discover-res-badge hf-discover-res-warn">{alrt} {cw}×{ch} px • İlerleyince 1200×675'e Yükseltilir</span>"#,
                alrt = icons::alert_circle("w-3.5 h-3.5"),
            ),
            _ => format!(
                r#"<span class="hf-discover-res-badge hf-discover-res-ok">{chk} 1200×675 px • WebP • Discover Uyumlu Kırpma</span>"#,
                chk = icons::check_circle2("w-3.5 h-3.5"),
            ),
        };
        let apply_wm_btn = if current_img.starts_with("/uploads/editorial/") || current_img.starts_with("/uploads/") {
            format!(
                r#"<button type="button" data-action="apply-watermark-current" {up_dis} class="hf-apply-wm-btn">
                    {shield_ico}
                    <span>Filigran / Logo Uygula</span>
                </button>"#,
                shield_ico = icons::shield_check("w-3.5 h-3.5"),
            )
        } else {
            String::new()
        };
        format!(
            r#"<div class="flex items-center justify-between gap-2 flex-wrap pt-1">
                {discover_badge}
                {apply_wm_btn}
            </div>"#
        )
    } else {
        String::new()
    };

    format!(
        r#"<div class="flex flex-col gap-4 py-4">
            <div class="space-y-2">
                <div class="flex items-center justify-between px-1 gap-2 flex-wrap">
                    <label class="text-xs font-black uppercase tracking-widest text-zinc-400">Öne Çıkan Görsel Seçimi & Kırpma (1200x675)</label>
                    {top_actions_html}
                </div>
                <div class="relative group aspect-[16/9] w-full bg-zinc-50 rounded-3xl overflow-hidden border border-zinc-100 flex items-center justify-center shadow-inner">
                    {stage_html}
                </div>
                {loaded_info_bar}
            </div>

            <div class="hf-watermark-option-bar">
                <div class="flex items-center gap-2.5 min-w-0">
                    <div class="w-8 h-8 rounded-xl bg-brand-50 text-brand-500 flex items-center justify-center shrink-0">
                        {shield_ico}
                    </div>
                    <div class="min-w-0">
                        <div class="text-xs font-extrabold text-zinc-800">Özel Haber / Logo Filigranı Bas</div>
                        <div class="text-xs font-semibold text-zinc-400">Aktifken yüklenen ve çekilen görsellere otomatik MSI Dergisi filigranı işlenir</div>
                    </div>
                </div>
                <button type="button" data-action="toggle-upload-watermark" class="hf-wm-toggle-btn {wm_toggle_cls}">
                    <span>{wm_lbl}</span>
                </button>
            </div>
        </div>"#,
        shield_ico = icons::shield_check("w-4 h-4"),
        wm_lbl = if s.upload_with_watermark { "AÇIK" } else { "KAPALI" },
    )
}

pub fn render_step3_tag_pills(s: &AppState) -> String {
    let tags = s.editing_article.tags.as_deref().unwrap_or(&[]);
    if tags.is_empty() {
        return r#"<span class="text-xs font-semibold text-zinc-400 px-1">Henüz etiket eklenmedi.</span>"#.to_string();
    }
    let mut out = String::new();
    for tag in tags {
        let t = tag.trim();
        if t.is_empty() {
            continue;
        }
        let t_esc = escape_html(t);
        out.push_str(&format!(
            r#"<span class="hf-tag-pill">
                <span>#{t_esc}</span>
                <button type="button" data-action="remove-tag" data-tag="{t_esc}" class="hf-tag-remove-btn" title="Etiketi Kaldır">&times;</button>
            </span>"#
        ));
    }
    out
}

pub fn render_step3_tag_suggestions(s: &AppState) -> String {
    let current = s.editing_article.tags.as_deref().unwrap_or(&[]);
    let available: Vec<_> = s
        .tag_suggestions
        .iter()
        .filter(|sug| {
            !sug.tag.trim().is_empty()
                && !current
                    .iter()
                    .any(|c| turkish_lowercase(c.trim()) == turkish_lowercase(sug.tag.trim()))
        })
        .take(8)
        .collect();

    if available.is_empty() {
        return String::new();
    }

    let mut chips = String::new();
    for sug in available {
        let t_esc = escape_html(sug.tag.trim());
        chips.push_str(&format!(
            r#"<button type="button" data-action="pick-tag-suggestion" data-tag="{t_esc}" class="hf-tag-suggestion-chip">
                <span>#{t_esc}</span>
                <span class="opacity-60">({cnt})</span>
            </button>"#,
            cnt = sug.count,
        ));
    }
    format!(
        r#"<div class="flex items-center flex-wrap gap-1.5 pt-1">
            <span class="text-xs font-black uppercase tracking-widest text-zinc-400">Önerilenler:</span>
            {chips}
        </div>"#
    )
}

pub fn render_step3_live_discover_and_serp(s: &AppState) -> String {
    let canon = s.canonical_editing_article();
    let art = &canon;
    let report = evaluate_seo_discover(art);
    let score = report.score;

    let (ring_cls, status_lbl) = if score >= 80 {
        ("hf-score-good", "Discover & News Hazır")
    } else if score >= 50 {
        ("hf-score-warn", "Geliştirilebilir")
    } else {
        ("hf-score-bad", "Kritik Eksikler Var")
    };

    let mut check_rows = String::new();
    for item in &report.checks {
        let (row_cls, ico) = match item.level {
            CheckLevel::Pass => (
                "hf-check-pass",
                icons::check_circle2("w-4 h-4 text-emerald-600 shrink-0"),
            ),
            CheckLevel::Warn => (
                "hf-check-warn",
                icons::alert_circle("w-4 h-4 text-amber-600 shrink-0"),
            ),
            CheckLevel::Fail => (
                "hf-check-fail",
                icons::alert_circle("w-4 h-4 text-red-500 shrink-0"),
            ),
        };
        check_rows.push_str(&format!(
            r#"<div class="hf-seo-check-row {row_cls}">
                {ico}
                <div class="min-w-0 flex-1">
                    <div class="text-[11px] font-extrabold text-zinc-900 leading-snug">{lbl}</div>
                    <div class="text-xs font-semibold text-zinc-500 leading-snug">{det}</div>
                </div>
            </div>"#,
            lbl = escape_html(&item.label),
            det = escape_html(&item.detail),
        ));
    }

    let seo = art.seo.as_ref();
    let eff_slug = art
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|sl| !sl.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let gen = slugify_turkish(&art.title);
            if gen.is_empty() {
                "ornek-haber-basligi".to_string()
            } else {
                gen
            }
        });
    let eff_title = seo
        .map(|x| x.title.trim())
        .filter(|t| !t.is_empty())
        .or_else(|| {
            let t = art.title.trim();
            (!t.is_empty()).then_some(t)
        })
        .unwrap_or("Haber Başlığı Google Arama ve Discover'da Burada Görünür");
    let eff_desc = seo
        .map(|x| x.description.trim())
        .filter(|d| !d.is_empty())
        .or_else(|| art.spot.as_deref().map(str::trim).filter(|sp| !sp.is_empty()))
        .unwrap_or("Haber spotu veya meta açıklama girildiğinde Google arama sonuçlarındaki iki satırlık özet önizlemesi burada yer alır.");
    let disc_img = render_news_image(
        art.effective_image_url(),
        eff_title,
        "w-full h-full object-cover",
    );

    format!(
        r#"<div class="space-y-4">
            <div class="hf-seo-guard-card">
                <div class="flex items-center justify-between gap-3 pb-3 border-b border-zinc-100">
                    <div class="flex items-center gap-3">
                        <div class="hf-score-meter {ring_cls}">
                            <span class="text-base font-black leading-none">{score}</span>
                            <span class="text-[11.5px] font-black uppercase opacity-75">PUAN</span>
                        </div>
                        <div>
                            <div class="text-xs font-black uppercase tracking-tight text-zinc-900">Google Discover & News Kontrolü</div>
                            <div class="text-xs font-bold text-zinc-500">{status_lbl}</div>
                        </div>
                    </div>
                </div>
                <div class="space-y-1.5 pt-1">
                    {check_rows}
                </div>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div class="hf-serp-preview-card">
                    <div class="text-xs font-black uppercase tracking-widest text-zinc-400 mb-2">Google Arama (SERP) Önizlemesi</div>
                    <div class="text-xs font-semibold text-zinc-500 truncate">msidergisi.com › icerik › {slug}</div>
                    <div class="hf-serp-title">{title}</div>
                    <div class="hf-serp-desc">{desc}</div>
                </div>
                <div class="hf-discover-preview-card">
                    <div class="text-xs font-black uppercase tracking-widest text-zinc-400 mb-2">Google Discover Kartı</div>
                    <div class="aspect-[16/9] rounded-xl overflow-hidden bg-zinc-100 border border-zinc-200/80 mb-2">
                        {disc_img}
                    </div>
                    <div class="text-xs font-extrabold text-zinc-900 line-clamp-2 leading-snug">{title}</div>
                    <div class="text-xs font-bold text-zinc-400 mt-1">MSI Dergisi • Az önce</div>
                </div>
            </div>
        </div>"#,
        slug = escape_html(&eff_slug),
        title = escape_html(eff_title),
        desc = escape_html(eff_desc),
    )
}

fn render_step_3(s: &AppState) -> String {
    let canon = s.canonical_editing_article();
    let seo = canon.seo.clone().unwrap_or_default();
    let slug_val = canon.slug.as_deref().unwrap_or("");
    let slug_ph = {
        let g = slugify_turkish(&canon.title);
        if g.is_empty() {
            "otomatik-kalici-baglanti".to_string()
        } else {
            g
        }
    };

    format!(
        r#"<div class="flex flex-col gap-4 py-4">
            <div class="space-y-5">
                <div class="flex items-center justify-between px-1 gap-2 flex-wrap">
                    <div class="flex items-center gap-2">
                        <div class="w-8 h-8 bg-zinc-900 text-white rounded-lg flex items-center justify-center">
                            {search_ico}
                        </div>
                        <div class="flex flex-col">
                            <span class="text-sm font-black text-zinc-800 uppercase tracking-tight">SEO Ayarlar</span>
                            <span class="text-xs font-bold text-zinc-400">Google Discover, News, Slug & #Etiket yönetimi.</span>
                        </div>
                    </div>
                    <button type="button" data-action="auto-fill-seo" class="h-8 px-4 bg-brand-50 text-brand-600 rounded-full text-xs font-black uppercase tracking-widest hover:bg-brand-100 transition-all flex items-center gap-2">
                        {auto_ico}
                        Otomatik Doldur
                    </button>
                </div>

                <div class="hf-slug-box">
                    <div class="flex items-center justify-between gap-2 flex-wrap">
                        <label class="text-xs font-black uppercase tracking-widest text-zinc-500">Kalıcı Bağlantı (Slug) & 301 Koruması</label>
                        <div class="flex items-center gap-2">
                            <button type="button" data-action="generate-slug-from-title" class="text-xs font-black uppercase tracking-wider text-brand-600 hover:underline">Başlıktan Üret</button>
                            <button type="button" data-action="open-redirects-modal" class="hf-redirects-open-btn">
                                {redir_ico}
                                <span>301 Yönlendirmeleri Gör</span>
                            </button>
                        </div>
                    </div>
                    <div class="hf-slug-input-wrap">
                        <span class="hf-slug-prefix">/icerik/</span>
                        <input id="hf-editor-slug" data-input="editor-slug" type="text" value="{slug_v}" placeholder="{slug_p}" class="hf-slug-input" />
                    </div>
                    <div class="text-xs font-semibold text-zinc-400 leading-snug">
                        Yayınlanmış haberin URL'sini değiştirirseniz eski adresten yeni adrese otomatik 301 yönlendirmesi tanımlanır.
                    </div>
                </div>

                <div class="hf-tags-box">
                    <label class="text-xs font-black uppercase tracking-widest text-zinc-500 flex items-center gap-1.5">
                        {hash_ico}
                        <span>Konu Etiketleri (#Tag)</span>
                    </label>
                    <div id="hf-tags-pills-slot" class="flex items-center flex-wrap gap-1.5">
                        {tag_pills}
                    </div>
                    <div class="flex items-center gap-2">
                        <input id="hf-tag-input" data-input="tag-query" type="text" value="{tq}" placeholder="Etiket yazıp Enter veya virgül (,) tuşuna basın..." class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-bold focus:outline-none focus:border-brand-500 transition-all" />
                        <button type="button" data-action="commit-tag-input" class="h-10 px-4 bg-zinc-900 text-white rounded-xl text-xs font-black uppercase tracking-widest hover:bg-brand-500 transition-colors shrink-0">Ekle</button>
                    </div>
                    <div id="hf-tag-suggestions-slot">
                        {tag_sugs}
                    </div>
                </div>

                <div class="space-y-3.5">
                    <div class="space-y-1.5">
                        <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-1">SEO Başlığı</label>
                        <input data-input="seo-title" type="text" value="{t}" class="w-full h-11 bg-zinc-100 rounded-2xl px-4 text-xs font-bold focus:bg-white focus:ring-2 focus:ring-brand-500/20 transition-all focus:outline-none" placeholder="Google'da görünecek başlık (35-70 karakter)" />
                    </div>

                    <div class="space-y-1.5">
                        <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-1">Meta Açıklama</label>
                        <textarea data-input="seo-description" class="w-full h-20 bg-zinc-100 rounded-2xl px-4 py-3 text-xs font-bold focus:bg-white focus:ring-2 focus:ring-brand-500/20 transition-all resize-none focus:outline-none" placeholder="Arama sonuçlarında görünecek kısa özet (80-160 karakter)">{d}</textarea>
                    </div>

                    <div class="space-y-1.5">
                        <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-1">Odak Anahtar Kelimeler</label>
                        <input data-input="seo-keywords" type="text" value="{k}" class="w-full h-11 bg-zinc-100 rounded-2xl px-4 text-xs font-bold focus:bg-white focus:ring-2 focus:ring-brand-500/20 transition-all focus:outline-none" placeholder="örn: teknoloji, ekonomi, gündem" />
                    </div>
                </div>

                <div id="hf-step3-live-seo-panel">
                    {live_seo_panel}
                </div>
            </div>
        </div>"#,
        search_ico = icons::search("w-4 h-4"),
        auto_ico = icons::zap("w-3 h-3"),
        redir_ico = icons::arrow_right_left("w-3 h-3"),
        hash_ico = icons::hash("w-3.5 h-3.5 text-brand-500"),
        slug_v = escape_html(slug_val),
        slug_p = escape_html(&slug_ph),
        tag_pills = render_step3_tag_pills(s),
        tq = escape_html(&s.tag_input_query),
        tag_sugs = render_step3_tag_suggestions(s),
        t = escape_html(&seo.title),
        d = escape_html(&seo.description),
        k = escape_html(&seo.keywords),
        live_seo_panel = render_step3_live_discover_and_serp(s),
    )
}

/// Renders all 13 rich block types into sanitized, publication-faithful HTML for Step 4 Preview.
pub fn render_preview_blocks_html(blocks: &[ContentBlock]) -> String {
    if blocks.is_empty() {
        return r#"<div class="py-8 text-center bg-zinc-100/50 rounded-xl border border-dashed border-zinc-200">
            <span class="text-xs font-black text-zinc-400 uppercase tracking-widest">İçerik Bulunmadı</span>
        </div>"#
            .to_string();
    }

    let mut out = String::new();
    for b in blocks {
        match b.block_type.as_str() {
            "heading1" => {
                let txt = escape_html(strip_all_html_tags(&b.content).trim());
                if !txt.is_empty() {
                    out.push_str(&format!(
                        r#"<h1 class="text-xl lg:text-2xl font-black text-zinc-900 tracking-tight leading-tight">{txt}</h1>"#
                    ));
                }
            }
            "heading" => {
                let txt = escape_html(strip_all_html_tags(&b.content).trim());
                if !txt.is_empty() {
                    out.push_str(&format!(
                        r#"<h2 class="text-base lg:text-lg font-black text-zinc-900 uppercase tracking-tight leading-snug">{txt}</h2>"#
                    ));
                }
            }
            "heading3" => {
                let txt = escape_html(strip_all_html_tags(&b.content).trim());
                if !txt.is_empty() {
                    out.push_str(&format!(
                        r#"<h3 class="text-sm lg:text-base font-extrabold text-zinc-800 leading-snug">{txt}</h3>"#
                    ));
                }
            }
            "image" => {
                let src = b.content.trim();
                if !src.is_empty() {
                    let cap = b
                        .meta
                        .as_deref()
                        .map(str::trim)
                        .filter(|c| !c.is_empty());
                    let cap_html = cap
                        .map(|c| {
                            format!(
                                r#"<figcaption class="hf-figcaption">{}</figcaption>"#,
                                escape_html(c)
                            )
                        })
                        .unwrap_or_default();
                    let img_html =
                        render_news_image(Some(src), cap.unwrap_or(""), "w-full h-auto");
                    out.push_str(&format!(
                        r#"<figure class="hf-preview-figure">{img_html}{cap_html}</figure>"#
                    ));
                }
            }
            "quote" => {
                let body = sanitize_inline_rich_html(b.content.trim()).replace('\n', "<br/>");
                if !strip_all_html_tags(&body).trim().is_empty() {
                    let cite_html = b
                        .meta
                        .as_deref()
                        .map(str::trim)
                        .filter(|c| !c.is_empty())
                        .map(|c| format!(r#"<cite>{}</cite>"#, escape_html(c)))
                        .unwrap_or_default();
                    out.push_str(&format!(
                        r#"<blockquote class="hf-quote-card"><p>{body}</p>{cite_html}</blockquote>"#
                    ));
                }
            }
            "bullet_list" => {
                let items: Vec<String> = b
                    .content
                    .lines()
                    .map(|l| {
                        sanitize_inline_rich_html(
                            l.trim().trim_start_matches(['•', '-', '*']).trim(),
                        )
                    })
                    .filter(|l| !strip_all_html_tags(l).trim().is_empty())
                    .map(|l| format!("<li>{l}</li>"))
                    .collect();
                if !items.is_empty() {
                    out.push_str(&format!(
                        r#"<ul class="hf-preview-ul">{}</ul>"#,
                        items.join("")
                    ));
                }
            }
            "numbered_list" => {
                let items: Vec<String> = b
                    .content
                    .lines()
                    .map(|l| sanitize_inline_rich_html(l.trim()))
                    .filter(|l| !strip_all_html_tags(l).trim().is_empty())
                    .map(|l| format!("<li>{l}</li>"))
                    .collect();
                if !items.is_empty() {
                    out.push_str(&format!(
                        r#"<ol class="hf-preview-ol">{}</ol>"#,
                        items.join("")
                    ));
                }
            }
            "callout" => {
                let body = sanitize_inline_rich_html(b.content.trim()).replace('\n', "<br/>");
                if !strip_all_html_tags(&body).trim().is_empty() {
                    let (variant, custom_title, default_title) =
                        parse_callout_meta(b.meta.as_deref());
                    let title = if custom_title.is_empty() {
                        default_title.to_string()
                    } else {
                        escape_html(custom_title)
                    };
                    out.push_str(&format!(
                        r#"<div class="hf-callout" data-variant="{variant}"><div class="hf-callout-title">{title}</div><div class="hf-callout-body">{body}</div></div>"#
                    ));
                }
            }
            "summary" => {
                let items: Vec<String> = b
                    .content
                    .lines()
                    .map(|l| {
                        sanitize_inline_rich_html(
                            l.trim().trim_start_matches(['•', '-', '*']).trim(),
                        )
                    })
                    .filter(|l| !strip_all_html_tags(l).trim().is_empty())
                    .map(|l| format!("<li>{l}</li>"))
                    .collect();
                if !items.is_empty() {
                    let heading = b
                        .meta
                        .as_deref()
                        .map(str::trim)
                        .filter(|m| !m.is_empty())
                        .map(escape_html)
                        .unwrap_or_else(|| "Öne Çıkanlar".to_string());
                    out.push_str(&format!(
                        r#"<div class="hf-summary"><h4>{heading}</h4><ul>{}</ul></div>"#,
                        items.join("")
                    ));
                }
            }
            "embed" => {
                if !b.content.trim().is_empty() {
                    out.push_str(&render_embed_preview_card(&b.content, b.meta.as_deref()));
                }
            }
            "divider" => {
                out.push_str(r#"<hr class="hf-divider" />"#);
            }
            "code" => {
                let code = escape_html(&b.content);
                if !code.trim().is_empty() {
                    out.push_str(&format!(
                        r#"<pre class="hf-code-block"><code>{code}</code></pre>"#
                    ));
                }
            }
            "gallery" => {
                let items: Vec<GalleryItem> =
                    serde_json::from_str(b.content.trim()).unwrap_or_default();
                let valid_items: Vec<_> = items
                    .iter()
                    .filter(|it| !it.url.trim().is_empty())
                    .collect();
                if !valid_items.is_empty() {
                    let gtitle = b
                        .meta
                        .as_deref()
                        .map(str::trim)
                        .filter(|m| !m.is_empty())
                        .map(|m| {
                            format!(
                                r#"<h4 class="hf-gallery-heading">{}</h4>"#,
                                escape_html(m)
                            )
                        })
                        .unwrap_or_default();
                    let mut figs = String::new();
                    for (idx, it) in valid_items.iter().enumerate() {
                        let img_html =
                            render_news_image(Some(it.url.trim()), &it.caption, "w-full h-auto");
                        let cap_html = if it.caption.trim().is_empty() {
                            String::new()
                        } else {
                            format!(
                                r#"<figcaption>{}</figcaption>"#,
                                escape_html(it.caption.trim())
                            )
                        };
                        figs.push_str(&format!(
                            r#"<figure class="hf-gallery-item" data-index="{num}">{img_html}{cap_html}</figure>"#,
                            num = idx + 1,
                        ));
                    }
                    out.push_str(&format!(
                        r#"<section class="hf-gallery">{gtitle}<div class="hf-gallery-grid">{figs}</div></section>"#
                    ));
                }
            }
            "related_article" => {
                let parsed: RelatedArticleRef =
                    serde_json::from_str(b.content.trim()).unwrap_or_default();
                if !parsed.title.trim().is_empty() {
                    let kicker = b
                        .meta
                        .as_deref()
                        .map(str::trim)
                        .filter(|m| !m.is_empty())
                        .map(escape_html)
                        .unwrap_or_else(|| "BUNU DA OKUYUN".to_string());
                    let slug = parsed.slug.as_deref().unwrap_or("").trim();
                    let href = if !slug.is_empty() {
                        format!("/icerik/{}", escape_html(slug))
                    } else {
                        "#".to_string()
                    };
                    let thumb_html = parsed
                        .image_url
                        .as_deref()
                        .filter(|u| !u.trim().is_empty())
                        .map(|u| {
                            format!(
                                r#"<img src="{}" alt="{}" class="hf-related-thumb" referrerpolicy="no-referrer" />"#,
                                escape_html(u.trim()),
                                escape_html(&parsed.title)
                            )
                        })
                        .unwrap_or_default();
                    out.push_str(&format!(
                        r#"<aside class="hf-related-card">{thumb_html}<div class="hf-related-meta"><span class="hf-related-kicker">{kicker}</span><a href="{href}" class="hf-related-link"><span class="hf-related-title">{t}</span></a></div></aside>"#,
                        t = escape_html(parsed.title.trim()),
                    ));
                }
            }
            "live_entry" => {
                let body = sanitize_inline_rich_html(b.content.trim()).replace('\n', "<br/>");
                if !strip_all_html_tags(&body).trim().is_empty() {
                    let raw_meta = b.meta.as_deref().unwrap_or("");
                    let (time_raw, title_raw) =
                        raw_meta.split_once('|').unwrap_or((raw_meta, ""));
                    let time_esc = escape_html(time_raw.trim());
                    let title_html = if title_raw.trim().is_empty() {
                        String::new()
                    } else {
                        format!(
                            r#"<strong class="hf-live-title">{}</strong>"#,
                            escape_html(title_raw.trim())
                        )
                    };
                    out.push_str(&format!(
                        r#"<div class="hf-live-entry" data-time="{time_esc}"><div class="hf-live-head"><span class="hf-live-time">{time_esc}</span>{title_html}</div><div class="hf-live-body">{body}</div></div>"#
                    ));
                }
            }
            _ => {
                let plain_cand = strip_all_html_tags(&b.content);
                if let Some(auto_embed_url) = detect_standalone_media_embed_url(plain_cand.trim()) {
                    out.push_str(&render_embed_preview_card(
                        &auto_embed_url,
                        b.meta.as_deref(),
                    ));
                } else {
                    let rich = sanitize_inline_rich_html(b.content.trim()).replace('\n', "<br/>");
                    if !strip_all_html_tags(&rich).trim().is_empty() {
                        out.push_str(&format!(r#"<p class="hf-preview-p">{rich}</p>"#));
                    }
                }
            }
        }
    }

    if out.is_empty() {
        r#"<div class="py-8 text-center bg-zinc-100/50 rounded-xl border border-dashed border-zinc-200">
            <span class="text-xs font-black text-zinc-400 uppercase tracking-widest">İçerik Bulunmadı</span>
        </div>"#
            .to_string()
    } else {
        out
    }
}

fn render_step_4(s: &AppState) -> String {
    let canon = s.canonical_editing_article();
    let art = &canon;
    let blocks = art.blocks.as_deref().unwrap_or(&[]);
    let kw = art
        .seo
        .as_ref()
        .map(|x| x.keywords.trim())
        .unwrap_or_default();

    let kw_badge = if !kw.is_empty() {
        format!(
            r#"<span class="px-2 py-1 bg-zinc-200 text-zinc-600 rounded text-xs font-black uppercase tracking-widest">{}</span>"#,
            escape_html(kw)
        )
    } else {
        String::new()
    };

    let word_cnt: usize = blocks
        .iter()
        .map(|b| strip_all_html_tags(&b.content).split_whitespace().count())
        .sum();
    let read_mins = art.read_time_min.unwrap_or(1).max(1);
    let seo_report = evaluate_seo_discover(art);
    let seo_badge_cls = if seo_report.score >= 80 {
        "bg-emerald-50 text-emerald-700 border border-emerald-200"
    } else if seo_report.score >= 50 {
        "bg-amber-50 text-amber-700 border border-amber-200"
    } else {
        "bg-red-50 text-red-600 border border-red-200"
    };

    let status = art.status.as_str();
    let sel_draft = if status.is_empty() || status == "draft" {
        "selected"
    } else {
        ""
    };
    let sel_rev = if status == "pending_review" {
        "selected"
    } else {
        ""
    };
    let sel_pub = if status == "publish" { "selected" } else { "" };
    let sel_fut = if status == "future" { "selected" } else { "" };
    let sel_arch = if status == "archived" { "selected" } else { "" };
    let pub_opts = if s.caps().can_publish {
        format!(
            r#"<option value="publish" {sel_pub}>Hemen Yayınla</option>
               <option value="future" {sel_fut}>İleri Tarihli Yayınla</option>
               <option value="archived" {sel_arch}>Arşive Kaldır</option>"#
        )
    } else {
        String::new()
    };

    let future_picker = if status == "future" {
        let dt_val = iso_to_datetime_local(art.effective_scheduled_at());
        format!(
            r#"<div class="mt-2">
                <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-1 mb-1 block">Yayın Tarihi &amp; Saati</label>
                <input data-input="scheduled-at" type="datetime-local" value="{dt_val}" class="w-full h-10 bg-white border border-zinc-200 rounded-lg px-3 text-xs font-bold text-zinc-700 focus:outline-none focus:border-brand-500 transition-colors" />
            </div>"#,
            dt_val = escape_html(&dt_val),
        )
    } else {
        String::new()
    };

    let can_pub = s.caps().can_publish;
    let post_dis = if s.is_posting { "disabled" } else { "" };
    let primary_action_btn = match status {
        "publish" => {
            if !can_pub {
                r#"<div class="p-3 rounded-xl bg-amber-50 border border-amber-200 text-[11px] font-bold text-amber-800">Doğrudan yayınlama yetkisi (<code>contents:publish</code>) yok — taslak veya başeditör onayına gönderebilirsiniz.</div>"#.to_string()
            } else {
                let ico = if s.is_posting {
                    icons::loader2("w-5 h-5 animate-spin")
                } else {
                    icons::send("w-4 h-4")
                };
                format!(
                    r#"<button type="button" data-action="publish-now" {post_dis} class="w-full min-h-[42px] px-5 bg-brand-500 text-white rounded-[10px] text-[14px] font-semibold shadow-sm hover:bg-brand-600 transition-all flex items-center justify-center gap-2.5 disabled:bg-zinc-400 disabled:shadow-none">
                        {ico}
                        Hemen Yayınla
                    </button>"#
                )
            }
        }
        "pending_review" => {
            let ico = if s.is_posting {
                icons::loader2("w-5 h-5 animate-spin")
            } else {
                icons::shield_check("w-4 h-4")
            };
            format!(
                r#"<button type="button" data-action="save-draft" {post_dis} class="w-full min-h-[42px] px-5 bg-amber-600 text-white rounded-[10px] text-[14px] font-semibold shadow-sm hover:bg-amber-700 transition-all flex items-center justify-center gap-2.5 disabled:bg-zinc-400 disabled:shadow-none">
                    {ico}
                    Başeditör Onayına Gönder
                </button>"#
            )
        }
        "future" => {
            if !can_pub {
                r#"<div class="p-3 rounded-xl bg-amber-50 border border-amber-200 text-[11px] font-bold text-amber-800">Zamanlı yayınlama yetkisi (<code>contents:publish</code>) gerekiyor.</div>"#.to_string()
            } else {
                let sched_dis = if s.is_posting || art.effective_scheduled_at().is_none() {
                    "disabled"
                } else {
                    ""
                };
                let ico = if s.is_posting {
                    icons::loader2("w-5 h-5 animate-spin")
                } else {
                    icons::clock("w-4 h-4")
                };
                format!(
                    r#"<button id="btn-schedule-post" type="button" data-action="save-draft" {sched_dis} class="w-full min-h-[42px] px-5 bg-brand-500 text-white rounded-[10px] text-[14px] font-semibold shadow-sm hover:bg-brand-600 transition-all flex items-center justify-center gap-2.5 disabled:bg-zinc-400 disabled:shadow-none">
                        {ico}
                        Yayını Zamanla
                    </button>"#
                )
            }
        }
        "archived" => {
            if !can_pub {
                String::new()
            } else {
                let ico = if s.is_posting {
                    icons::loader2("w-5 h-5 animate-spin")
                } else {
                    icons::save("w-4 h-4")
                };
                format!(
                    r#"<button type="button" data-action="save-draft" {post_dis} class="w-full min-h-[42px] px-5 bg-zinc-800 text-white rounded-[10px] text-[14px] font-semibold shadow-sm hover:bg-zinc-900 transition-all flex items-center justify-center gap-2.5 disabled:bg-zinc-400">
                        {ico}
                        Arşive Kaydet
                    </button>"#
                )
            }
        }
        _ => String::new(),
    };

    let secondary_draft_action = if status.is_empty() || status == "draft" {
        "save-draft"
    } else {
        "save-plain-draft"
    };
    let draft_btn_lbl = if status.is_empty() || status == "draft" {
        "Sisteme Taslak Kaydet"
    } else {
        "Taslak Olarak Kaydet"
    };

    // 1. MSI Publication Channel, Format & Magazine Issue Card
    let pub_ch = art.publication_channel.as_str();
    let ch_btn_cls = |target: &str| -> &'static str {
        if pub_ch == target || (target == "news" && (pub_ch.is_empty() || pub_ch == "web")) {
            "hf-seg-btn hf-seg-btn-active"
        } else {
            "hf-seg-btn"
        }
    };

    let mut format_opts = String::new();
    let cur_ct = if art.content_type.trim().is_empty() {
        "haberler"
    } else {
        art.content_type.trim()
    };
    for (code, label) in MSI_CONTENT_TYPES {
        let sel = if *code == cur_ct { "selected" } else { "" };
        format_opts.push_str(&format!(
            r#"<option value="{code}" {sel}>{label} ({code})</option>"#,
            code = escape_html(code),
            label = escape_html(label),
        ));
    }

    let cur_issue = art.issue_number;
    let issue_none_sel = if cur_issue.is_none() { "selected" } else { "" };
    let mut issue_opts = format!(
        r#"<option value="0" {issue_none_sel}>— Dergi Sayısı Seçin (Opsiyonel) —</option>"#
    );
    let mut matched_issue_in_list = false;
    for iss in &s.magazine_issues {
        let sel = if cur_issue == Some(iss.issue_number) && !matched_issue_in_list {
            matched_issue_in_list = true;
            "selected"
        } else {
            ""
        };
        let lang_tag = if iss.edition_lang.eq_ignore_ascii_case("en") {
            "MSI TDR"
        } else {
            "MSI Dergisi"
        };
        let main_title = if iss.edition_lang.eq_ignore_ascii_case("en")
            && !iss.title_en.trim().is_empty()
        {
            &iss.title_en
        } else if !iss.title_tr.trim().is_empty() {
            &iss.title_tr
        } else {
            lang_tag
        };
        let ev_suffix = match (iss.is_special_issue, iss.event_name.as_deref()) {
            (true, Some(ev)) if !ev.trim().is_empty() => format!(" • [ÖZEL: {}]", ev.trim()),
            (true, _) => " • [ÖZEL SAYI]".to_string(),
            _ => String::new(),
        };
        issue_opts.push_str(&format!(
            r#"<option value="{num}" {sel}>#{num} · {title} ({month} {year}){ev}</option>"#,
            num = iss.issue_number,
            title = escape_html(main_title),
            month = escape_html(&iss.publish_month),
            year = iss.year,
            ev = escape_html(&ev_suffix),
        ));
    }
    if let Some(ci) = cur_issue {
        if ci > 0 && !matched_issue_in_list {
            issue_opts.push_str(&format!(
                r#"<option value="{ci}" selected>#{ci} · Seçili Dergi Sayısı</option>"#
            ));
        }
    }

    let page_range_val = art.issue_page_range.as_deref().unwrap_or("");
    let feat_cls = if art.is_featured {
        "hf-switch-on"
    } else {
        "hf-switch-off"
    };

    // 2. SSB Defence Technology Taxonomy Card
    let selected_ssb: Vec<&str> = art
        .ssb_tags
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect();
    let ssb_count_badge = if selected_ssb.is_empty() {
        r#"<span class="hf-msi-count-pill">Etiket Seçilmedi</span>"#.to_string()
    } else {
        format!(
            r#"<span class="hf-msi-count-pill hf-msi-count-active">{} SSB Etiketi Seçili</span>"#,
            selected_ssb.len()
        )
    };

    let mut ordered_groups: Vec<String> = Vec::new();
    for preset in [
        "Platformlar",
        "Silah ve Mühimmat",
        "Elektronik ve Bilişim",
        "Alt Sistemler",
        "Yardımcı Hizmetler",
    ] {
        if s.ssb_tags_catalog
            .iter()
            .any(|t| t.category_group.eq_ignore_ascii_case(preset) || t.category_group == preset)
        {
            ordered_groups.push(preset.to_string());
        }
    }
    for t in &s.ssb_tags_catalog {
        let g = t.category_group.trim();
        if !g.is_empty() && !ordered_groups.iter().any(|existing| existing == g) {
            ordered_groups.push(g.to_string());
        }
    }

    let ssb_groups_html = if s.ssb_tags_catalog.is_empty() {
        r#"<div class="text-xs font-semibold text-zinc-400 py-2">SSB teknoloji kataloğu yükleniyor...</div>"#.to_string()
    } else {
        let mut g_out = String::new();
        for grp in &ordered_groups {
            let mut chips = String::new();
            for tag in s
                .ssb_tags_catalog
                .iter()
                .filter(|t| t.category_group.trim() == grp.as_str())
            {
                let is_on = selected_ssb.iter().any(|c| *c == tag.code.as_str());
                let chip_cls = if is_on {
                    "hf-ssb-chip hf-ssb-chip-active"
                } else {
                    "hf-ssb-chip"
                };
                chips.push_str(&format!(
                    r#"<button type="button" data-action="toggle-ssb-tag" data-code="{code}" title="{en}" class="{chip_cls}">{tr}</button>"#,
                    code = escape_html(&tag.code),
                    en = escape_html(&tag.name_en),
                    tr = escape_html(&tag.name_tr),
                ));
            }
            if !chips.is_empty() {
                g_out.push_str(&format!(
                    r#"<div class="space-y-1">
                        <div class="text-xs font-black uppercase tracking-widest text-zinc-400">{grp_esc}</div>
                        <div class="flex items-center flex-wrap gap-1.5">{chips}</div>
                    </div>"#,
                    grp_esc = escape_html(grp),
                ));
            }
        }
        format!(r#"<div class="space-y-2.5">{g_out}</div>"#)
    };

    // 3. Access Control (Paywall), Single-Article Price & Sponsorship / Advertorial Card
    let vis_lvl = art.visibility_level.clamp(0, 2);
    let vis_btn_cls = |lvl: i32| -> &'static str {
        if vis_lvl == lvl {
            "hf-seg-btn hf-seg-btn-active"
        } else {
            "hf-seg-btn"
        }
    };
    let single_price_box = if vis_lvl == 2 {
        let price_val = if art.single_price_try > 0.0 {
            format!("{:.2}", art.single_price_try)
        } else {
            String::new()
        };
        format!(
            r#"<div class="hf-paywall-price-box">
                <label class="text-xs font-black uppercase tracking-widest text-zinc-500">Tekil Makale Satış Fiyatı (₺)</label>
                <div class="relative w-36">
                    <span class="absolute left-3 top-1/2 -translate-y-1/2 text-xs font-black text-zinc-400">₺</span>
                    <input data-input="article-single-price" type="number" min="0" step="0.50" value="{price_val}" placeholder="45.00" class="w-full h-9 bg-white border border-zinc-200 rounded-xl pl-7 pr-3 text-xs font-extrabold text-zinc-800 focus:outline-none focus:border-brand-500" />
                </div>
            </div>"#
        )
    } else {
        String::new()
    };

    let spon_cls = if art.is_sponsored {
        "hf-switch-on"
    } else {
        "hf-switch-off"
    };
    let sponsor_fields_html = if art.is_sponsored {
        let brand_v = art.sponsor_brand.as_deref().unwrap_or("");
        let label_v = art
            .sponsor_legal_label
            .as_deref()
            .unwrap_or("ADVERTORIAL / KURUMSAL İŞ BİRLİĞİ");
        format!(
            r#"<div class="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-2 border-t border-amber-200/60">
                <div class="space-y-0.5">
                    <label class="text-xs font-black uppercase tracking-widest text-amber-800 px-0.5">Sponsor Kuruluş Adı</label>
                    <input data-input="article-sponsor-brand" type="text" value="{b}" placeholder="Örn: ASELSAN, ROKETSAN" class="w-full h-9 bg-white border border-amber-200 rounded-xl px-3 text-xs font-bold text-zinc-800 focus:outline-none focus:border-amber-500" />
                </div>
                <div class="space-y-0.5">
                    <label class="text-xs font-black uppercase tracking-widest text-amber-800 px-0.5">Yasal Etiket</label>
                    <input data-input="article-sponsor-label" type="text" value="{l}" placeholder="ADVERTORIAL / KURUMSAL İŞ BİRLİĞİ" class="w-full h-9 bg-white border border-amber-200 rounded-xl px-3 text-[11px] font-bold text-zinc-800 focus:outline-none focus:border-amber-500" />
                </div>
            </div>"#,
            b = escape_html(brand_v),
            l = escape_html(label_v),
        )
    } else {
        String::new()
    };

    // 4. English Edition (`en_status`) Selector in Step 4
    let en_st = art.en_status.as_str();
    let has_en = article_has_english(art);
    let en_sel_none = if en_st == "none" || en_st.is_empty() {
        "selected"
    } else {
        ""
    };
    let en_sel_dr = if en_st == "draft" || en_st == "auto_draft" {
        "selected"
    } else {
        ""
    };
    let en_sel_rv = if en_st == "review" { "selected" } else { "" };
    let en_sel_ap = if en_st == "approved" { "selected" } else { "" };
    let en_summary_hint = if has_en {
        format!(
            r#"<span class="px-2 py-0.5 rounded bg-emerald-50 text-emerald-700 text-xs font-black uppercase">EN Başlık Mevcut</span>"#
        )
    } else {
        r#"<span class="px-2 py-0.5 rounded bg-zinc-100 text-zinc-500 text-xs font-bold">1. Adımdan EN Eklenebilir</span>"#.to_string()
    };

    let selected_cat_ids = art.categories.as_deref().unwrap_or(&[]);
    let tree = build_category_tree(&s.categories);
    let active_chain = selected_cat_ids
        .iter()
        .rev()
        .find_map(|cid| {
            let ch = category_ancestor_chain(&s.categories, *cid);
            (!ch.is_empty()).then_some(ch)
        })
        .unwrap_or_default();
    let selected_root_id = active_chain.first().copied();
    let selected_sub_id = if active_chain.len() >= 2 {
        active_chain.last().copied()
    } else {
        None
    };

    let preview_cat_badges = if let Some(leaf_id) = active_chain.last().copied() {
        if let Some(item) = tree.iter().find(|t| t.category.id == leaf_id) {
            let color = item.category.color.as_deref().unwrap_or("#52c2ef");
            format!(
                r#"<span style="background-color: {color};" class="px-2 py-1 text-white rounded text-xs font-black uppercase tracking-widest">{path}</span>"#,
                color = escape_html(color),
                path = escape_html(&item.path_label),
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let manage_cats_btn = r#"<button type="button" data-action="toggle-settings-popover" class="text-xs font-black uppercase tracking-widest text-brand-600 hover:underline">Kategorileri Yönet</button>"#;

    let categories_selector_html = if tree.is_empty() {
        format!(
            r#"<div class="p-4 bg-zinc-50 border border-dashed border-zinc-200 rounded-2xl flex items-center justify-between gap-3">
                <div class="text-xs font-bold text-zinc-400">Henüz tanımlı kategori yok.</div>
                {manage_cats_btn}
            </div>"#
        )
    } else {
        let root_none_sel = if selected_root_id.is_none() {
            "selected"
        } else {
            ""
        };
        let mut root_opts =
            format!(r#"<option value="0" {root_none_sel}>— Ana Kategori Seçin —</option>"#);
        for item in tree.iter().filter(|t| t.depth == 0) {
            let sel = if selected_root_id == Some(item.category.id) {
                "selected"
            } else {
                ""
            };
            root_opts.push_str(&format!(
                r#"<option value="{id}" {sel}>{name}</option>"#,
                id = item.category.id,
                name = escape_html(&item.category.name),
            ));
        }

        let (sub_disabled, sub_opts) = if let Some(rid) = selected_root_id {
            let desc_ids: std::collections::HashSet<i32> =
                collect_descendant_category_ids(&s.categories, rid)
                    .into_iter()
                    .collect();
            let sub_items: Vec<_> = tree
                .iter()
                .filter(|t| desc_ids.contains(&t.category.id))
                .collect();

            if sub_items.is_empty() {
                (
                    "disabled",
                    r#"<option value="0">Alt kategori bulunmuyor</option>"#.to_string(),
                )
            } else {
                let sub_none_sel = if selected_sub_id.is_none() {
                    "selected"
                } else {
                    ""
                };
                let mut sopts = format!(
                    r#"<option value="0" {sub_none_sel}>— Alt Kategori Seçin (Opsiyonel) —</option>"#
                );
                let root_prefix = tree
                    .iter()
                    .find(|t| t.category.id == rid)
                    .map(|t| format!("{} › ", t.category.name))
                    .unwrap_or_default();
                for sub in sub_items {
                    let sel = if selected_sub_id == Some(sub.category.id) {
                        "selected"
                    } else {
                        ""
                    };
                    let rel_label = sub
                        .path_label
                        .strip_prefix(&root_prefix)
                        .unwrap_or(&sub.category.name);
                    sopts.push_str(&format!(
                        r#"<option value="{id}" {sel}>{lbl}</option>"#,
                        id = sub.category.id,
                        lbl = escape_html(rel_label),
                    ));
                }
                ("", sopts)
            }
        } else {
            let all_subs: Vec<_> = tree.iter().filter(|t| t.depth > 0).collect();
            if all_subs.is_empty() {
                (
                    "disabled",
                    r#"<option value="0">Önce ana kategori seçin</option>"#.to_string(),
                )
            } else {
                let mut sopts =
                    r#"<option value="0" selected>— Alt Kategori Seçin (Opsiyonel) —</option>"#
                        .to_string();
                for sub in all_subs {
                    sopts.push_str(&format!(
                        r#"<option value="{id}">{path}</option>"#,
                        id = sub.category.id,
                        path = escape_html(&sub.path_label),
                    ));
                }
                ("", sopts)
            }
        };

        format!(
            r#"<div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                <div class="space-y-1 min-w-0">
                    <span class="text-xs font-black uppercase tracking-widest text-zinc-400 px-1 block">Üst / Ana Kategori</span>
                    <div class="relative">
                        <select data-change="step4-parent-category" class="w-full h-12 bg-zinc-100 rounded-xl px-4 pr-10 text-xs font-bold text-zinc-800 focus:outline-none appearance-none">
                            {root_opts}
                        </select>
                        <div class="absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none">{chev_sub}</div>
                    </div>
                </div>
                <div class="space-y-1 min-w-0">
                    <span class="text-xs font-black uppercase tracking-widest text-zinc-400 px-1 block">Alt Kategori</span>
                    <div class="relative">
                        <select data-change="step4-sub-category" {sub_disabled} class="w-full h-12 bg-zinc-100 rounded-xl px-4 pr-10 text-xs font-bold text-zinc-800 focus:outline-none appearance-none disabled:opacity-50">
                            {sub_opts}
                        </select>
                        <div class="absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none">{chev_sub}</div>
                    </div>
                </div>
            </div>"#,
            chev_sub = icons::chevron_down("w-4 h-4 text-zinc-400"),
        )
    };

    let summary_spot_html = art
        .spot
        .as_deref()
        .map(str::trim)
        .filter(|sp| !sp.is_empty())
        .map(|sp| {
            format!(
                r#"<p class="text-[11px] font-medium text-zinc-500 leading-snug line-clamp-2">{}</p>"#,
                escape_html(sp)
            )
        })
        .unwrap_or_default();

    let summary_title = if art.title.trim().is_empty() {
        "Başlıksız Haber Taslağı".to_string()
    } else {
        escape_html(&art.title)
    };

    let middle_sync_lbl = if s.middle_preview_draft {
        "Orta Ekranda Açık ✓"
    } else {
        "Orta Ekranda Göster"
    };
    let middle_sync_cls = if s.middle_preview_draft {
        "bg-emerald-50 text-emerald-700 border-emerald-200"
    } else {
        "bg-white text-zinc-700 border-zinc-200 hover:border-brand-500 hover:text-brand-600"
    };

    let mut preview_extra_badges = String::new();
    match pub_ch {
        "magazine" => {
            if let Some(inum) = art.issue_number {
                preview_extra_badges.push_str(&format!(
                    r#"<span class="hf-msi-badge-crimson">MSI DERGİ #{inum}</span>"#
                ));
            } else {
                preview_extra_badges
                    .push_str(r#"<span class="hf-msi-badge-crimson">MSI DERGİSİ</span>"#);
            }
        }
        "both" => {
            if let Some(inum) = art.issue_number {
                preview_extra_badges.push_str(&format!(
                    r#"<span class="hf-msi-badge-crimson">DERGİ #{inum} + HABER</span>"#
                ));
            } else {
                preview_extra_badges
                    .push_str(r#"<span class="hf-msi-badge-crimson">DERGİ + HABER</span>"#);
            }
        }
        _ => {}
    }
    if cur_ct != "haberler" {
        preview_extra_badges.push_str(&format!(
            r#"<span class="hf-card-flag-badge">{}</span>"#,
            escape_html(msi_content_type_label_tr(cur_ct))
        ));
    }
    if art.is_live_blog {
        preview_extra_badges.push_str(
            r#"<span class="hf-card-live-badge"><span class="hf-live-dot"></span>CANLI ANLATIM</span>"#,
        );
    }
    if art.comments_locked {
        preview_extra_badges.push_str(r#"<span class="hf-card-flag-badge">YORUMLARA KAPALI</span>"#);
    }
    if art.ads_disabled {
        preview_extra_badges.push_str(r#"<span class="hf-card-flag-badge">REKLAMSIZ</span>"#);
    }

    let c_lock_cls = if art.comments_locked {
        "hf-switch-on"
    } else {
        "hf-switch-off"
    };
    let ads_off_cls = if art.ads_disabled {
        "hf-switch-on"
    } else {
        "hf-switch-off"
    };

    format!(
        r#"<div class="flex flex-col gap-5 py-3">
            <div class="space-y-4">
                <div class="flex items-center justify-between gap-2 px-1">
                    <div class="flex items-center gap-2">
                        <div class="w-8 h-8 bg-zinc-900 text-white rounded-lg flex items-center justify-center">{chk_ico}</div>
                        <div class="flex flex-col">
                            <span class="text-sm font-black text-zinc-800">Son Kontrol & Yayınlama</span>
                            <span class="text-xs font-bold text-zinc-400">MSI Dergisi / MSI TDR editoryal yayın, sayı, taksonomi ve erişim ayarları.</span>
                        </div>
                    </div>
                </div>

                <div class="hf-step4-summary-card">
                    <div class="hf-step4-summary-top flex items-start gap-3.5">
                        <div class="hf-step4-summary-thumb">
                            {hero_img}
                        </div>
                        <div class="flex-1 min-w-0 space-y-1.5">
                            <div class="flex items-center gap-1.5 flex-wrap">
                                <span class="px-2 py-0.5 bg-brand-100 text-brand-600 rounded text-xs font-black uppercase tracking-widest">{blk_cnt} Blok • {word_cnt} Kelime • {read_mins} dk</span>
                                <span class="px-2 py-0.5 rounded text-xs font-black uppercase tracking-widest {seo_badge_cls}">SEO {seo_score}/100</span>
                                {preview_cat_badges}
                                {preview_extra_badges}
                                {kw_badge}
                            </div>
                            <h3 class="text-sm font-black text-zinc-900 leading-snug line-clamp-2">{summary_title}</h3>
                            {summary_spot_html}
                        </div>
                    </div>
                    <div class="hf-step4-preview-actions">
                        <button type="button" data-action="open-preview-modal" class="hf-btn-full-preview whitespace-nowrap">
                            {eye_ico}
                            <span>Tam Ekran Önizle</span>
                        </button>
                        <button type="button" data-action="preview-draft-in-middle" class="hf-btn-middle-preview border {middle_sync_cls}">
                            {news_ico}
                            <span>{middle_sync_lbl}</span>
                        </button>
                    </div>
                </div>

                <div class="hf-msi-card">
                    <div class="flex items-center justify-between gap-2">
                        <div class="text-xs font-black uppercase tracking-widest text-zinc-700 flex items-center gap-1.5">
                            {book_ico}
                            <span>Yayın Kanalı, İçerik Formatı &amp; Dergi Sayısı</span>
                        </div>
                        <button type="button" data-action="open-settings-issues" class="text-xs font-black uppercase tracking-wider text-brand-600 hover:underline">Sayıları Yönet</button>
                    </div>

                    <div class="space-y-1">
                        <span class="text-xs font-black uppercase tracking-widest text-zinc-400 px-0.5">Yayın Kanalı</span>
                        <div class="hf-seg-group">
                            <button type="button" data-action="set-pub-channel" data-channel="news" class="{ch_news}">Savunma Haber</button>
                            <button type="button" data-action="set-pub-channel" data-channel="magazine" class="{ch_mag}">MSI Dergisi</button>
                            <button type="button" data-action="set-pub-channel" data-channel="both" class="{ch_both}">Her İkisi (Dergi + Haber)</button>
                        </div>
                    </div>

                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                        <div class="space-y-1 min-w-0">
                            <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-0.5">İçerik Formatı</label>
                            <div class="relative">
                                <select data-change="article-content-type" class="w-full h-10 bg-zinc-50 border border-zinc-200/80 rounded-xl px-3 pr-8 text-xs font-bold text-zinc-800 focus:outline-none focus:border-brand-500 appearance-none">
                                    {format_opts}
                                </select>
                                <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                            </div>
                        </div>
                        <div class="space-y-1 min-w-0">
                            <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-0.5">Sayfa Aralığı (Dergi)</label>
                            <input data-input="article-issue-pages" type="text" value="{page_range}" placeholder="Sayfa aralığı (Örn: S. 22-30)" class="w-full h-10 bg-zinc-50 border border-zinc-200/80 rounded-xl px-3 text-xs font-bold text-zinc-800 focus:outline-none focus:border-brand-500" />
                        </div>
                    </div>

                    <div class="space-y-1">
                        <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-0.5">Bağlı Dergi Sayısı (MSI Dergisi / MSI TDR)</label>
                        <div class="relative">
                            <select data-change="article-issue-number" class="w-full h-10 bg-zinc-50 border border-zinc-200/80 rounded-xl px-3 pr-8 text-xs font-bold text-zinc-800 focus:outline-none focus:border-brand-500 appearance-none">
                                {issue_opts}
                            </select>
                            <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                        </div>
                    </div>

                    <div class="hf-control-row pt-1">
                        <div class="flex items-center gap-2 min-w-0">
                            {star_ico}
                            <div class="min-w-0">
                                <div class="text-xs font-extrabold text-zinc-800">Manşet / Vitrin Öne Çıkan İçerik</div>
                                <div class="text-xs font-semibold text-zinc-400">Ana vitrinde / kapak dosyası sahnesinde öne çıkar</div>
                            </div>
                        </div>
                        <button type="button" data-action="toggle-is-featured" role="switch" aria-checked="{feat_aria}" class="hf-switch-btn {feat_cls}">
                            <span class="hf-switch-knob"></span>
                        </button>
                    </div>
                </div>

                <div class="space-y-2">
                    <div class="flex items-center justify-between px-1">
                        <label class="text-xs font-black uppercase tracking-widest text-zinc-400 flex items-center gap-1.5">
                            {tag_ico} Haber Kategorisi
                        </label>
                        {manage_cats_btn}
                    </div>
                    {categories_selector_html}
                </div>

                <div class="hf-msi-card">
                    <div class="flex items-center justify-between gap-2 flex-wrap">
                        <div class="text-xs font-black uppercase tracking-widest text-zinc-700 flex items-center gap-1.5">
                            {shield_ico}
                            <span>SSB Savunma Sanayii Teknoloji Taksonomisi</span>
                        </div>
                        {ssb_count_badge}
                    </div>
                    {ssb_groups_html}
                </div>

                <div class="hf-msi-card">
                    <div class="text-xs font-black uppercase tracking-widest text-zinc-700 flex items-center gap-1.5">
                        {lock_ico}
                        <span>Erişim Kademesi (Paywall) &amp; Kurumsal Sponsorluk</span>
                    </div>

                    <div class="space-y-1.5">
                        <span class="text-xs font-black uppercase tracking-widest text-zinc-400 px-0.5">Erişim Seviyesi</span>
                        <div class="hf-seg-group">
                            <button type="button" data-action="set-visibility-level" data-level="0" class="{vis_0}">Açık Erişim</button>
                            <button type="button" data-action="set-visibility-level" data-level="1" class="{vis_1}">Üye &amp; Kamu (.gov.tr/.tsk.tr)</button>
                            <button type="button" data-action="set-visibility-level" data-level="2" class="{vis_2}">Abone / Paywall</button>
                        </div>
                    </div>

                    {single_price_box}

                    <div class="hf-sponsor-wrap">
                        <div class="hf-control-row">
                            <div class="min-w-0">
                                <div class="text-xs font-extrabold text-zinc-800">Sponsorlu İçerik / Advertorial</div>
                                <div class="text-xs font-semibold text-zinc-400">Kurumsal iş birliği / Advertorial — abone duvarından muaftır</div>
                            </div>
                            <button type="button" data-action="toggle-is-sponsored" role="switch" aria-checked="{spon_aria}" class="hf-switch-btn {spon_cls}">
                                <span class="hf-switch-knob"></span>
                            </button>
                        </div>
                        {sponsor_fields_html}
                    </div>
                </div>

                <div class="hf-msi-card">
                    <div class="text-xs font-black uppercase tracking-widest text-zinc-700">Editoryal Onay &amp; Yayın Akışı</div>
                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                        <div class="space-y-1 min-w-0">
                            <label class="text-xs font-black uppercase tracking-widest text-zinc-400 px-0.5">Türkçe Yayın / Onay Durumu</label>
                            <div class="relative">
                                <select data-change="article-status" class="w-full h-11 bg-zinc-100 rounded-xl px-3.5 pr-9 text-xs font-bold text-zinc-800 focus:outline-none appearance-none">
                                    <option value="draft" {sel_draft}>Taslak (Hazırlanıyor)</option>
                                    <option value="pending_review" {sel_rev}>Başeditör Onayına Gönder (İnceleme Bekliyor)</option>
                                    {pub_opts}
                                </select>
                                <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                            </div>
                        </div>
                        <div class="space-y-1 min-w-0">
                            <div class="flex items-center justify-between gap-1 px-0.5">
                                <label class="text-xs font-black uppercase tracking-widest text-zinc-400">İngilizce (MSI TDR) Durumu</label>
                                {en_summary_hint}
                            </div>
                            <div class="relative">
                                <select data-change="article-en-status" class="w-full h-11 bg-zinc-100 rounded-xl px-3.5 pr-9 text-xs font-bold text-zinc-800 focus:outline-none appearance-none">
                                    <option value="none" {en_sel_none}>Yok (Sadece TR)</option>
                                    <option value="draft" {en_sel_dr}>Çeviri Taslağı</option>
                                    <option value="review" {en_sel_rv}>Çeviri İncelemede</option>
                                    <option value="approved" {en_sel_ap}>Onaylı / Yayına Hazır ✓</option>
                                </select>
                                <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                            </div>
                        </div>
                    </div>
                    {future_picker}
                </div>

                <div class="hf-article-controls-card">
                    <div class="text-xs font-black uppercase tracking-widest text-zinc-500">Hassas İçerik & Haber Kontrolleri</div>
                    <div class="hf-control-row">
                        <div class="flex items-center gap-2.5 min-w-0">
                            <div class="w-8 h-8 rounded-xl bg-zinc-100 text-zinc-600 flex items-center justify-center shrink-0">
                                {msg_off_ico}
                            </div>
                            <div class="min-w-0">
                                <div class="text-xs font-extrabold text-zinc-800">Yorumlara Kapat</div>
                                <div class="text-xs font-semibold text-zinc-400">Bu haberde okuyucu yorum alanını devre dışı bırak</div>
                            </div>
                        </div>
                        <button type="button" data-action="toggle-comments-locked" role="switch" aria-checked="{c_aria}" class="hf-switch-btn {c_lock_cls}">
                            <span class="hf-switch-knob"></span>
                        </button>
                    </div>
                    <div class="hf-control-row">
                        <div class="flex items-center gap-2.5 min-w-0">
                            <div class="w-8 h-8 rounded-xl bg-zinc-100 text-zinc-600 flex items-center justify-center shrink-0">
                                {ban_ico}
                            </div>
                            <div class="min-w-0">
                                <div class="text-xs font-extrabold text-zinc-800">Bu Haberde Reklam Gösterme</div>
                                <div class="text-xs font-semibold text-zinc-400">Adli veya hassas haberlerde reklam yerleşimlerini kapat</div>
                            </div>
                        </div>
                        <button type="button" data-action="toggle-ads-disabled" role="switch" aria-checked="{a_aria}" class="hf-switch-btn {ads_off_cls}">
                            <span class="hf-switch-knob"></span>
                        </button>
                    </div>
                </div>

                {primary_action_btn}

                <button type="button" data-action="{secondary_draft_action}" {post_dis} class="w-full min-h-[38px] px-4 bg-white text-zinc-700 border border-zinc-200 rounded-[10px] text-[13px] font-semibold hover:bg-zinc-50 hover:border-zinc-300 transition-all flex items-center justify-center gap-2 mt-1">
                    {save_ico}
                    {draft_btn_lbl}
                </button>
            </div>
        </div>"#,
        chk_ico = icons::check_circle2("w-4 h-4"),
        hero_img = render_news_image(
            art.effective_image_url(),
            &art.title,
            "w-full h-full object-cover"
        ),
        blk_cnt = blocks.len(),
        seo_score = seo_report.score,
        eye_ico = icons::eye("w-4 h-4"),
        news_ico = icons::newspaper("w-3.5 h-3.5"),
        book_ico = icons::book_open("w-3.5 h-3.5 text-rose-600"),
        ch_news = ch_btn_cls("news"),
        ch_mag = ch_btn_cls("magazine"),
        ch_both = ch_btn_cls("both"),
        page_range = escape_html(page_range_val),
        star_ico = icons::star("w-4 h-4 text-amber-500 shrink-0"),
        feat_aria = if art.is_featured { "true" } else { "false" },
        tag_ico = icons::tag("w-3.5 h-3.5 text-brand-500"),
        shield_ico = icons::shield_check("w-3.5 h-3.5 text-brand-600"),
        lock_ico = icons::lock("w-3.5 h-3.5 text-amber-600"),
        vis_0 = vis_btn_cls(0),
        vis_1 = vis_btn_cls(1),
        vis_2 = vis_btn_cls(2),
        spon_aria = if art.is_sponsored { "true" } else { "false" },
        msg_off_ico = icons::message_square_off("w-4 h-4"),
        ban_ico = icons::ban("w-4 h-4"),
        c_aria = if art.comments_locked { "true" } else { "false" },
        a_aria = if art.ads_disabled { "true" } else { "false" },
        chev_ico = icons::chevron_down("w-4 h-4 text-zinc-400"),
        save_ico = icons::save("w-4 h-4"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_youtube_ids_and_rejects_untrusted_strings() {
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=12s"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_youtube_video_id("https://youtu.be/abc_123-XYZ?si=test"),
            Some("abc_123-XYZ".to_string())
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/shorts/shortVid123"),
            Some("shortVid123".to_string())
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/live/liveStream99?si=abc"),
            Some("liveStream99".to_string())
        );
        assert_eq!(
            extract_tweet_id("https://x.com/msidergisi/status/1857492019283712001?s=20"),
            Some("1857492019283712001".to_string())
        );
        assert_eq!(
            extract_tweet_id("https://twitter.com/i/web/status/1857492019283712001"),
            Some("1857492019283712001".to_string())
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/watch?v=<script>"),
            None
        );
    }

    #[test]
    fn renders_all_rich_preview_blocks_faithfully_and_strips_xss() {
        let blocks = vec![
            ContentBlock {
                id: "1".into(),
                block_type: "heading1".into(),
                content: "Büyük Manşet".into(),
                meta: None,
            },
            ContentBlock {
                id: "2".into(),
                block_type: "heading3".into(),
                content: "Ara Başlık Detayı".into(),
                meta: None,
            },
            ContentBlock {
                id: "3".into(),
                block_type: "paragraph".into(),
                content: "Önemli <mark>vurgulu</mark> ve <em>italik</em> metin <script>alert(1)</script>".into(),
                meta: None,
            },
            ContentBlock {
                id: "4".into(),
                block_type: "quote".into(),
                content: "Ekonomide <strong>yeni dönem</strong> başlıyor.".into(),
                meta: Some("Prof. Dr. Ayşe Kaya".into()),
            },
            ContentBlock {
                id: "5".into(),
                block_type: "callout".into(),
                content: "Deprem bölgesinde son durum açıklandı.".into(),
                meta: Some("breaking|SON DAKİKA GELİŞMESİ".into()),
            },
            ContentBlock {
                id: "6".into(),
                block_type: "summary".into(),
                content: "Faiz sabit tutuldu\nEnflasyon tahmini güncellendi".into(),
                meta: Some("Öne Çıkan Başlıklar".into()),
            },
            ContentBlock {
                id: "7".into(),
                block_type: "embed".into(),
                content: "https://youtu.be/dQw4w9WgXcQ".into(),
                meta: Some("Canlı Yayın Kaydı".into()),
            },
            ContentBlock {
                id: "8".into(),
                block_type: "divider".into(),
                content: String::new(),
                meta: None,
            },
            ContentBlock {
                id: "9".into(),
                block_type: "code".into(),
                content: "BIST100: 10.450 (+2.1%)".into(),
                meta: None,
            },
        ];

        let html = render_preview_blocks_html(&blocks);
        assert!(html.contains("<h1"));
        assert!(html.contains("Büyük Manşet"));
        assert!(html.contains("<h3"));
        assert!(html.contains("Ara Başlık Detayı"));
        assert!(html.contains("<mark>vurgulu</mark>"));
        assert!(html.contains("<em>italik</em>"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("<blockquote"));
        assert!(html.contains("<cite>Prof. Dr. Ayşe Kaya</cite>"));
        assert!(html.contains("data-variant=\"breaking\""));
        assert!(html.contains("SON DAKİKA GELİŞMESİ"));
        assert!(html.contains("hf-summary"));
        assert!(html.contains("Öne Çıkan Başlıklar"));
        assert!(html.contains("https://www.youtube.com/embed/dQw4w9WgXcQ?rel=0"));
        assert!(html.contains("<figcaption class=\"hf-figcaption\">Canlı Yayın Kaydı</figcaption>"));
        assert!(html.contains("<hr class=\"hf-divider\" />"));
        assert!(html.contains("<pre class=\"hf-code-block\"><code>BIST100: 10.450 (+2.1%)</code></pre>"));

        // Standalone Tweet or Video URL inside a `paragraph` block also auto-upgrades to live embed!
        let tweet_para_html = render_preview_blocks_html(&[ContentBlock {
            id: "tw1".into(),
            block_type: "paragraph".into(),
            content: "https://x.com/ekonomi/status/1857492019283712001".into(),
            meta: None,
        }]);
        assert!(tweet_para_html.contains("https://platform.twitter.com/embed/Tweet.html?id=1857492019283712001"));
    }

    #[test]
    fn renders_gallery_related_article_and_live_entry_preview_blocks() {
        let gal_json = serde_json::to_string(&vec![GalleryItem {
            url: "/uploads/g1.webp".into(),
            thumb_url: Some("/uploads/g1-t.webp".into()),
            caption: "Saha görüntüsü".into(),
        }])
        .unwrap();
        let rel_json = serde_json::to_string(&RelatedArticleRef {
            article_id: "pub-1".into(),
            title: "Ekonomide Yeni Paketin Detayları".into(),
            slug: Some("ekonomide-yeni-paketin-detaylari".into()),
            image_url: Some("/uploads/r1.webp".into()),
        })
        .unwrap();

        let blocks = vec![
            ContentBlock {
                id: "10".into(),
                block_type: "live_entry".into(),
                content: "Basın toplantısı başladı.".into(),
                meta: Some("15:10|Canlı Yayındayız".into()),
            },
            ContentBlock {
                id: "11".into(),
                block_type: "gallery".into(),
                content: gal_json,
                meta: Some("Foto Galeri Başlığı".into()),
            },
            ContentBlock {
                id: "12".into(),
                block_type: "related_article".into(),
                content: rel_json,
                meta: Some("BUNU DA OKUYUN".into()),
            },
        ];

        let html = render_preview_blocks_html(&blocks);
        assert!(html.contains("hf-live-entry"));
        assert!(html.contains("15:10"));
        assert!(html.contains("Canlı Yayındayız"));
        assert!(html.contains("hf-gallery"));
        assert!(html.contains("Foto Galeri Başlığı"));
        assert!(html.contains("Saha görüntüsü"));
        assert!(html.contains("hf-related-card"));
        assert!(html.contains("BUNU DA OKUYUN"));
        assert!(html.contains("Ekonomide Yeni Paketin Detayları"));
    }
}
