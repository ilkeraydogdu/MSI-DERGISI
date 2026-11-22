//! Left Panel renderer for MSI Dergisi Editorial Studio (`Yayınlananlar`, `Taslaklar` + `Ayarlar`).

use crate::editorial::common::Article;

use crate::editorial::wasm::icons;
use crate::editorial::wasm::render::render_news_image;
use crate::editorial::wasm::state::{
    admin_path, article_has_english, escape_html, format_card_datetime, AppState, LeftTab,
    NewsViewMode,
};

fn render_card_status_badges(s: &AppState, art: &Article) -> String {
    let mut out = String::new();
    if let Some(holder) = s.lock_holder_for_article(&art.id) {
        out.push_str(&format!(
            r#"<span class="hf-card-lock-badge">🔒 {uname} düzenliyor</span>"#,
            uname = escape_html(&holder.username)
        ));
    }

    match art.publication_channel.as_str() {
        "magazine" => {
            if let Some(iss) = art.issue_number {
                out.push_str(&format!(
                    r#"<span class="hf-card-msi-badge">DERGİ #{iss}</span>"#
                ));
            } else {
                out.push_str(r#"<span class="hf-card-msi-badge">DERGİ</span>"#);
            }
        }
        "both" => {
            if let Some(iss) = art.issue_number {
                out.push_str(&format!(
                    r#"<span class="hf-card-msi-badge">DERGİ #{iss}+HABER</span>"#
                ));
            } else {
                out.push_str(r#"<span class="hf-card-msi-badge">DERGİ+HABER</span>"#);
            }
        }
        _ => {
            if let Some(iss) = art.issue_number {
                out.push_str(&format!(
                    r#"<span class="hf-card-news-badge">HABER #{iss}</span>"#
                ));
            } else {
                out.push_str(r#"<span class="hf-card-news-badge">HABER</span>"#);
            }
        }
    }

    if art.en_status == "approved" {
        out.push_str(r#"<span class="hf-card-en-badge hf-card-en-ok">EN ✓</span>"#);
    } else if art.en_status == "review" {
        out.push_str(r#"<span class="hf-card-en-badge hf-card-en-rev">EN İNCELEME</span>"#);
    } else if art.en_status == "draft"
        || art.en_status == "auto_draft"
        || article_has_english(art)
    {
        out.push_str(r#"<span class="hf-card-en-badge">EN TASLAK</span>"#);
    }

    if art.status == "pending_review" {
        out.push_str(r#"<span class="hf-card-review-badge">ONAY BEKLİYOR</span>"#);
    }

    if art.is_featured {
        out.push_str(r#"<span class="hf-card-feat-badge">★ VİTRİN</span>"#);
    }

    if art.is_sponsored {
        out.push_str(r#"<span class="hf-card-spon-badge">SPONSORLU</span>"#);
    }

    if art.visibility_level == 2 {
        out.push_str(r#"<span class="hf-card-paywall-badge">PAYWALL</span>"#);
    } else if art.visibility_level == 1 {
        out.push_str(r#"<span class="hf-card-gov-badge">KAMU</span>"#);
    }

    if art.is_live_blog {
        out.push_str(
            r#"<span class="hf-card-live-badge"><span class="hf-live-dot"></span>CANLI</span>"#,
        );
    }
    if art.comments_locked {
        out.push_str(r#"<span class="hf-card-flag-badge">YORUMLARA KAPALI</span>"#);
    }
    if art.ads_disabled {
        out.push_str(r#"<span class="hf-card-flag-badge">REKLAMSIZ</span>"#);
    }
    if out.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="flex items-center flex-wrap gap-1">{out}</div>"#)
    }
}

pub fn render_left_panel_inner(s: &AppState) -> String {
    let tab_btn = |tab: LeftTab, btn_id: &str, title: &str, icon_svg: String| -> String {
        let active_cls = if s.left_tab == tab {
            "bg-white text-brand-500 shadow-sm ring-1 ring-zinc-200/50"
        } else {
            "text-zinc-500 hover:text-zinc-900 hover:bg-zinc-200/50"
        };
        let tab_key = match tab {
            LeftTab::Published => "published",
            LeftTab::Drafts => "drafts",
            LeftTab::Research => "research",
        };
        format!(
            r#"<button id="{btn_id}" type="button" data-action="set-left-tab" data-tab="{tab_key}" title="{title}" class="w-10 h-10 flex items-center justify-center rounded-full transition-all {active_cls}">{icon_svg}</button>"#
        )
    };

    let admin_settings_btn = {
        let btn_cls = if s.show_settings_popover {
            "bg-brand-500 text-white shadow-sm"
        } else {
            "text-zinc-500 hover:text-zinc-900 hover:bg-zinc-200/50"
        };
        format!(
            r#"<button id="btn-settings-trigger" type="button" data-action="toggle-settings-popover" aria-expanded="{expanded}" title="Ayarlar" class="w-10 h-10 flex items-center justify-center rounded-full transition-all {btn_cls}">{ico}</button>"#,
            expanded = if s.show_settings_popover { "true" } else { "false" },
            ico = icons::settings("w-4 h-4"),
        )
    };

    let grid_cls = if s.news_view_mode == NewsViewMode::Grid {
        "bg-white text-brand-600 shadow-sm"
    } else {
        "text-zinc-400 hover:text-zinc-600"
    };
    let list_cls = if s.news_view_mode == NewsViewMode::List {
        "bg-white text-brand-600 shadow-sm"
    } else {
        "text-zinc-400 hover:text-zinc-600"
    };
    let layout_toggle_html = format!(
        r#"<div class="flex bg-zinc-100 p-1 rounded-lg border border-zinc-200/30">
            <button id="btn-view-grid" type="button" data-action="set-view-mode" data-mode="grid" title="Izgara Görünümü" class="w-7 h-7 flex items-center justify-center rounded-md transition-all {grid_cls}">{}</button>
            <button id="btn-view-list" type="button" data-action="set-view-mode" data-mode="list" title="Liste Görünümü" class="w-7 h-7 flex items-center justify-center rounded-md transition-all {list_cls}">{}</button>
        </div>"#,
        icons::layout_grid("w-3.5 h-3.5"),
        icons::list("w-3.5 h-3.5")
    );

    let ref_dis = if s.is_refreshing_articles {
        "disabled"
    } else {
        ""
    };
    let ref_ico = if s.is_refreshing_articles {
        icons::loader2("w-4 h-4 animate-spin")
    } else {
        icons::refresh_cw("w-4 h-4")
    };
    let right_action_btns = format!(
        r#"<button id="btn-refresh-articles" type="button" data-action="refresh-articles" {ref_dis} title="Listeyi Yenile" class="w-9 h-9 flex items-center justify-center bg-zinc-100 text-zinc-600 hover:bg-zinc-200 rounded-full transition-all disabled:opacity-50">{ref_ico}</button>
        <button id="btn-new-article" type="button" data-action="new-article" title="Yeni Haber" class="w-10 h-10 flex items-center justify-center bg-brand-500 text-white hover:bg-brand-600 rounded-full shadow-sm shadow-brand-500/20 transition-all">{new_ico}</button>"#,
        new_ico = icons::pen_square("w-4 h-4"),
    );

    format!(
        r#"<div class="px-4 pt-4 pb-2 shrink-0 flex items-center justify-between gap-2 border-b border-zinc-100 mb-2">
            <div class="flex bg-zinc-100 p-1.5 rounded-full border border-zinc-200/50 shadow-inner overflow-hidden">
                {t_pub}
                {t_drafts}
                {t_research}
                {admin_settings_btn}
            </div>
            <div class="flex items-center gap-1.5 shrink-0">
                {layout_toggle_html}
                {right_action_btns}
            </div>
        </div>
        <div id="left-scroll" class="flex-1 overflow-y-auto p-4 custom-scrollbar">
            {body}
        </div>
        <div class="px-3.5 py-2.5 border-t border-zinc-200 bg-zinc-100/90 shrink-0 space-y-1.5 select-none">
            <div class="flex items-center justify-between text-[9px] font-black uppercase tracking-widest text-zinc-500">
                <div class="flex items-center gap-2">
                    <div class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></div>
                    <span>YAYIN SİSTEMİ AKTİF</span>
                </div>
                <span id="left-pub-total-label">TOPLAM: {pub_total} YAYIN</span>
            </div>
            <div class="flex items-center justify-between text-[8.5px] font-semibold text-zinc-400 border-t border-zinc-200/60 pt-1.5">
                <span>© 2026 MSI Dergisi • Editör Stüdyosu</span>
                <a href="{admin_href}" class="text-zinc-500 hover:text-brand-600 font-bold transition-colors">
                    Komuta Merkezi
                </a>
            </div>
        </div>"#,
        admin_href = escape_html(&admin_path()),
        t_pub = tab_btn(
            LeftTab::Published,
            "btn-left-tab-published",
            "Yayınlananlar",
            icons::file_text("w-4 h-4")
        ),
        t_drafts = tab_btn(
            LeftTab::Drafts,
            "btn-left-tab-drafts",
            "Taslaklar",
            icons::save("w-4 h-4")
        ),
        t_research = tab_btn(
            LeftTab::Research,
            "btn-left-tab-research",
            "Araştırma Masası (Canlı Haber & Kaynak Arama)",
            icons::compass("w-4 h-4")
        ),
        body = render_left_tab_body(s),
        pub_total = s.published_total,
    )
}

pub fn render_left_tab_body(s: &AppState) -> String {
    match s.left_tab {
        LeftTab::Published => render_published_tab(s),
        LeftTab::Drafts => render_drafts_tab(s),
        LeftTab::Research => crate::editorial::wasm::render_research::render_research_left_tab(s),
    }
}

fn render_published_tab(s: &AppState) -> String {
    let filtered = s.filtered_published_articles();
    let is_grid = s.news_view_mode == NewsViewMode::Grid;
    let grid_cls = if is_grid {
        "grid-cols-[repeat(auto-fill,minmax(160px,1fr))]"
    } else {
        "grid-cols-1"
    };
    let sel_id = s
        .selected_article
        .as_ref()
        .map(|a| a.id.as_str())
        .unwrap_or("");

    let can_del = s.caps().can_delete;
    let mut cards = String::new();
    for art in &filtered {
        let outer_dir = if is_grid {
            "flex-col hover:-translate-y-1"
        } else {
            "flex-row hf-list-card-row"
        };
        let inner_dir = if is_grid { "flex-col" } else { "flex-row" };
        let sel_cls = if sel_id == art.id && !art.id.is_empty() {
            "ring-2 ring-emerald-500 bg-emerald-50/10"
        } else {
            "opacity-95 hover:opacity-100"
        };
        let thumb_wrap = if is_grid {
            "aspect-[16/10] w-full border-b"
        } else {
            "hf-list-card-thumb border-r"
        };
        let badge_pos = if is_grid { "top-2" } else { "top-1.5" };
        let body_pad = if is_grid {
            "p-3 gap-2"
        } else {
            "p-2.5 gap-1.5 min-w-0"
        };
        let title_clamp = if is_grid {
            "text-[11px] line-clamp-3"
        } else {
            "text-[10.5px] line-clamp-2"
        };
        let list_spot_html = if !is_grid {
            art.spot
                .as_deref()
                .map(str::trim)
                .filter(|sp| !sp.is_empty())
                .map(|sp| format!(r#"<p class="hf-list-card-spot">{}</p>"#, escape_html(sp)))
                .unwrap_or_default()
        } else {
            String::new()
        };

        let pub_date = format_card_datetime(
            art.effective_publish_date()
                .or(art.created_at.as_deref())
                .or(art.created_at_camel.as_deref()),
            "YAYINLANDI",
        );
        let thumb_html = render_news_image(
            art.effective_image_url(),
            &art.title,
            "w-full h-full object-cover transition-transform duration-700 group-hover:scale-105",
        );

        let publisher_html = match art.publisher_name.as_deref().filter(|p| !p.is_empty()) {
            Some(pub_name) => format!(
                r#"<div class="text-[9px] font-bold text-zinc-500 uppercase tracking-widest flex items-center gap-1">{} {}</div>"#,
                icons::user("w-3 h-3"),
                escape_html(pub_name)
            ),
            None => String::new(),
        };
        let extra_flags_html = render_card_status_badges(s, art);
        let share_btn = if s.caps().can_publish {
            format!(
                r#"<div class="absolute right-2 top-2 z-10"><button type="button" data-action="manual-share" data-article-id="{id}" class="w-7 h-7 bg-black/60 text-white rounded-full flex items-center justify-center shadow-lg hover:bg-brand-500 backdrop-blur-sm transition-all" title="Sosyal Medyada Paylaş">{share_ico}</button></div>"#,
                id = escape_html(&art.id),
                share_ico = icons::share2("w-3.5 h-3.5"),
            )
        } else {
            String::new()
        };
        let del_btn = if can_del {
            format!(
                r#"<button type="button" data-action="delete-article" data-article-id="{id}" class="opacity-0 group-hover:opacity-100 text-zinc-300 hover:text-red-500 transition-all" title="Haberi Sil">{trash_ico}</button>"#,
                id = escape_html(&art.id),
                trash_ico = icons::trash2("w-3 h-3"),
            )
        } else {
            String::new()
        };

        cards.push_str(&format!(
            r#"<div class="group relative bg-white border border-zinc-200 rounded-xl transition-all duration-300 hover:shadow-lg hover:shadow-zinc-200/50 flex overflow-hidden {outer_dir} {sel_cls}">
                <div role="button" tabindex="0" data-action="select-article" data-article-source="published" data-article-id="{id}" class="text-left flex flex-1 overflow-hidden cursor-pointer {inner_dir}">
                    <div class="bg-zinc-100 overflow-hidden relative border-zinc-100 shrink-0 {thumb_wrap}">
                        {thumb_html}
                        <div class="absolute left-2 flex items-center gap-2 {badge_pos}">
                            <span class="text-[7px] font-black uppercase tracking-widest text-emerald-700 bg-white/90 backdrop-blur px-1.5 py-0.5 rounded shadow-sm">YAYINLANDI</span>
                        </div>
                        {share_btn}
                    </div>
                    <div class="flex-1 flex flex-col justify-between {body_pad}">
                        <div class="space-y-1">
                            {extra_flags_html}
                            <h3 class="font-bold leading-snug tracking-tight text-zinc-900 group-hover:text-brand-600 transition-colors {title_clamp}">{title}</h3>
                            {list_spot_html}
                        </div>
                        <div class="flex flex-col gap-1.5">
                            <div class="flex items-center justify-between">
                                <div class="text-[8.5px] font-bold text-zinc-400 flex items-center gap-1.5">
                                    <span class="w-1 h-1 rounded-full bg-emerald-400"></span>
                                    {pub_date}
                                </div>
                                <div class="flex items-center gap-1.5 text-zinc-400">
                                    <span class="flex items-center gap-0.5 text-[9px]" title="Görüntülenme">{eye_ico} {views}</span>
                                    <span class="flex items-center gap-0.5 text-[9px]" title="Yorumlar">{msg_ico} {comments}</span>
                                    <button type="button" data-action="refresh-stats" data-article-id="{id}" class="hover:text-brand-500 transition-colors" title="İstatistikleri Güncelle">{ref_ico}</button>
                                    {del_btn}
                                </div>
                            </div>
                            {publisher_html}
                        </div>
                    </div>
                </div>
            </div>"#,
            id = escape_html(&art.id),
            title = escape_html(&art.title),
            pub_date = escape_html(&pub_date),
            views = art.views,
            comments = art.comments,
            eye_ico = icons::eye("w-3 h-3"),
            msg_ico = icons::message_circle("w-3 h-3"),
            ref_ico = icons::refresh_ccw("w-3 h-3"),
        ));
    }

    let load_more_html =
        if s.published_articles.len() < s.published_total && s.search_query.trim().is_empty() {
            let dis = if s.is_loading_more { "disabled" } else { "" };
            let label = if s.is_loading_more {
                "Yükleniyor...".to_string()
            } else {
                format!(
                    "Daha Fazla Göster ({})",
                    s.published_total.saturating_sub(s.published_articles.len())
                )
            };
            format!(
                r#"<button type="button" data-action="load-more" data-kind="published" {dis} class="col-span-full w-full py-3 mt-4 bg-zinc-100 hover:bg-zinc-200 text-zinc-600 rounded-xl font-bold text-xs uppercase tracking-wider transition-colors disabled:opacity-50">{label}</button>"#
            )
        } else {
            String::new()
        };

    let empty_html = if filtered.is_empty() {
        format!(
            r#"<div class="col-span-full py-12 text-center space-y-4">
                <div class="w-12 h-12 bg-zinc-100 rounded-full flex items-center justify-center mx-auto">{}</div>
                <div class="text-[10px] font-black uppercase tracking-widest text-zinc-400">Yayınlanan Haber Yok</div>
                <button type="button" data-action="new-article" class="text-[10px] font-black uppercase tracking-widest text-brand-600 hover:underline">İlk Haberi Oluştur</button>
            </div>"#,
            icons::check_circle2("w-6 h-6 text-zinc-400")
        )
    } else {
        String::new()
    };

    let header_count = if s.search_query.trim().is_empty() {
        s.published_total.max(filtered.len())
    } else {
        filtered.len()
    };

    format!(
        r#"<div class="space-y-4">
            <div class="flex items-center justify-between px-1">
                <h2 class="text-[10px] font-black uppercase tracking-widest text-zinc-400">Yayınlanan Haberler ({count})</h2>
            </div>
            <div class="grid gap-3 {grid_cls}">
                {cards}
                {load_more_html}
                {empty_html}
            </div>
        </div>"#,
        count = header_count
    )
}

fn render_drafts_tab(s: &AppState) -> String {
    let filtered = s.filtered_draft_articles();
    let is_grid = s.news_view_mode == NewsViewMode::Grid;
    let grid_cls = if is_grid {
        "grid-cols-[repeat(auto-fill,minmax(160px,1fr))]"
    } else {
        "grid-cols-1"
    };
    let sel_id = s
        .selected_article
        .as_ref()
        .map(|a| a.id.as_str())
        .unwrap_or("");
    let editing_id = s.editing_article.id.as_str();

    let can_del = s.caps().can_delete;
    let mut cards = String::new();
    for art in &filtered {
        let outer_dir = if is_grid {
            "flex-col hover:-translate-y-1"
        } else {
            "flex-row hf-list-card-row"
        };
        let inner_dir = if is_grid { "flex-col" } else { "flex-row" };
        let is_selected = !art.id.is_empty() && sel_id == art.id.as_str();
        let is_in_editor = !art.id.is_empty() && editing_id == art.id.as_str();
        let in_editor_pill = if is_in_editor {
            r#"<span class="text-[7px] font-black uppercase tracking-widest text-white bg-brand-500 px-1.5 py-0.5 rounded shadow-sm">EDİTÖRDE</span>"#
        } else {
            ""
        };
        let sel_cls = if is_selected {
            "ring-2 ring-brand-500 bg-brand-50/20"
        } else {
            "opacity-95 hover:opacity-100"
        };
        let thumb_wrap = if is_grid {
            "aspect-[16/10] w-full border-b"
        } else {
            "hf-list-card-thumb border-r"
        };
        let badge_pos = if is_grid { "top-2" } else { "top-1.5" };
        let body_pad = if is_grid {
            "p-3 gap-2"
        } else {
            "p-2.5 gap-1.5 min-w-0"
        };
        let title_clamp = if is_grid {
            "text-[11px] line-clamp-3"
        } else {
            "text-[10.5px] line-clamp-2"
        };
        let list_spot_html = if !is_grid {
            art.spot
                .as_deref()
                .map(str::trim)
                .filter(|sp| !sp.is_empty())
                .map(|sp| format!(r#"<p class="hf-list-card-spot">{}</p>"#, escape_html(sp)))
                .unwrap_or_default()
        } else {
            String::new()
        };

        let (badge_html, dot_cls, date_str) = match art.status.as_str() {
            "pending_review" => (
                r#"<span class="text-[7px] font-black uppercase tracking-widest text-amber-800 bg-amber-100/95 backdrop-blur px-1.5 py-0.5 rounded shadow-sm">ONAY BEKLİYOR</span>"#,
                "bg-amber-500",
                format_card_datetime(
                    art.created_at
                        .as_deref()
                        .or(art.created_at_camel.as_deref()),
                    "İNCELEMEDE",
                ),
            ),
            "future" => (
                r#"<span class="text-[7px] font-black uppercase tracking-widest text-amber-700 bg-amber-50/95 backdrop-blur px-1.5 py-0.5 rounded shadow-sm">ZAMANLANDI</span>"#,
                "bg-amber-400",
                format_card_datetime(art.effective_scheduled_at(), "ZAMANLANDI"),
            ),
            "archived" => (
                r#"<span class="text-[7px] font-black uppercase tracking-widest text-zinc-700 bg-zinc-200/95 backdrop-blur px-1.5 py-0.5 rounded shadow-sm">ARŞİV</span>"#,
                "bg-zinc-400",
                format_card_datetime(
                    art.created_at
                        .as_deref()
                        .or(art.created_at_camel.as_deref()),
                    "ARŞİV",
                ),
            ),
            _ => (
                r#"<span class="text-[7px] font-black uppercase tracking-widest text-zinc-600 bg-white/90 backdrop-blur px-1.5 py-0.5 rounded shadow-sm">TASLAK</span>"#,
                "bg-zinc-300",
                format_card_datetime(
                    art.created_at
                        .as_deref()
                        .or(art.created_at_camel.as_deref()),
                    "TASLAK",
                ),
            ),
        };
        let display_title = if art.title.trim().is_empty() {
            "BAŞLIKSIZ TASLAK"
        } else {
            art.title.as_str()
        };

        let thumb_html = render_news_image(
            art.effective_image_url(),
            display_title,
            "w-full h-full object-cover transition-transform duration-700 group-hover:scale-105",
        );
        let extra_flags_html = render_card_status_badges(s, art);
        let del_draft_btn = if can_del {
            format!(
                r#"<button type="button" data-action="delete-article" data-article-id="{id}" class="opacity-0 group-hover:opacity-100 text-zinc-300 hover:text-red-500 transition-all" title="Taslağı Sil">{trash_ico}</button>"#,
                id = escape_html(&art.id),
                trash_ico = icons::trash2("w-3.5 h-3.5"),
            )
        } else {
            String::new()
        };

        cards.push_str(&format!(
            r#"<div class="group relative bg-white border border-zinc-200 rounded-xl transition-all duration-300 hover:shadow-lg hover:shadow-zinc-200/50 flex overflow-hidden {outer_dir} {sel_cls}">
                <div role="button" tabindex="0" data-action="select-draft" data-article-id="{id}" class="text-left flex flex-1 overflow-hidden cursor-pointer {inner_dir}">
                    <div class="bg-zinc-100 overflow-hidden relative border-zinc-100 shrink-0 {thumb_wrap}">
                        {thumb_html}
                        <div class="absolute left-2 flex items-center gap-1.5 {badge_pos}">
                            {badge_html}
                            {in_editor_pill}
                        </div>
                    </div>
                    <div class="flex-1 flex flex-col justify-between {body_pad}">
                        <div class="space-y-1">
                            {extra_flags_html}
                            <h3 class="font-bold leading-snug tracking-tight text-zinc-900 group-hover:text-brand-600 transition-colors {title_clamp}">{title}</h3>
                            {list_spot_html}
                        </div>
                        <div class="flex items-center justify-between">
                            <div class="text-[8.5px] font-bold text-zinc-400 flex items-center gap-1.5">
                                <span class="w-1 h-1 rounded-full {dot_cls}"></span>
                                {date_str}
                            </div>
                            {del_draft_btn}
                        </div>
                    </div>
                </div>
            </div>"#,
            id = escape_html(&art.id),
            title = escape_html(display_title),
            date_str = escape_html(&date_str),
        ));
    }

    let load_more_html =
        if s.draft_articles.len() < s.draft_total && s.search_query.trim().is_empty() {
            let dis = if s.is_loading_more { "disabled" } else { "" };
            let label = if s.is_loading_more {
                "Yükleniyor...".to_string()
            } else {
                format!(
                    "Daha Fazla Göster ({})",
                    s.draft_total.saturating_sub(s.draft_articles.len())
                )
            };
            format!(
                r#"<button type="button" data-action="load-more" data-kind="draft" {dis} class="col-span-full w-full py-3 mt-4 bg-zinc-100 hover:bg-zinc-200 text-zinc-600 rounded-xl font-bold text-xs uppercase tracking-wider transition-colors disabled:opacity-50">{label}</button>"#
            )
        } else {
            String::new()
        };

    let empty_html = if filtered.is_empty() {
        format!(
            r#"<div class="col-span-full py-12 text-center space-y-4">
                <div class="w-12 h-12 bg-zinc-100 rounded-full flex items-center justify-center mx-auto">{}</div>
                <div class="text-[10px] font-black uppercase tracking-widest text-zinc-400">Taslak Yok</div>
            </div>"#,
            icons::save("w-6 h-6 text-zinc-400")
        )
    } else {
        String::new()
    };

    let header_count = if s.search_query.trim().is_empty() && s.draft_status_filter == "all" {
        s.draft_total.max(filtered.len())
    } else {
        filtered.len()
    };

    let sf = s.draft_status_filter.as_str();
    let pill = |key: &str, lbl: &str, cnt: usize| -> String {
        let cls = if sf == key || (key == "all" && sf.is_empty()) {
            "hf-draft-filter-pill hf-draft-filter-active"
        } else {
            "hf-draft-filter-pill"
        };
        format!(
            r#"<button type="button" data-action="set-draft-status-filter" data-filter="{key}" class="{cls}">{lbl} <span class="opacity-70">({cnt})</span></button>"#
        )
    };
    let status_filters_bar = format!(
        r#"<div class="flex items-center flex-wrap gap-1 px-0.5">
            {p_all}
            {p_drf}
            {p_rev}
            {p_fut}
            {p_arc}
        </div>"#,
        p_all = pill("all", "Tümü", s.draft_count_for_status("all")),
        p_drf = pill("draft", "Taslak", s.draft_count_for_status("draft")),
        p_rev = pill(
            "pending_review",
            "Onay Bekleyen",
            s.draft_count_for_status("pending_review")
        ),
        p_fut = pill("future", "Zamanlanmış", s.draft_count_for_status("future")),
        p_arc = pill("archived", "Arşiv", s.draft_count_for_status("archived")),
    );

    format!(
        r#"<div class="space-y-3">
            <div class="flex items-center justify-between px-1">
                <h2 class="text-[10px] font-black uppercase tracking-widest text-zinc-400 flex items-center gap-2">
                    {save_ico}
                    Taslaklar ({count})
                </h2>
            </div>
            {status_filters_bar}
            <div class="grid gap-3 {grid_cls}">
                {cards}
                {load_more_html}
                {empty_html}
            </div>
        </div>"#,
        save_ico = icons::save("w-3.5 h-3.5"),
        count = header_count
    )
}
