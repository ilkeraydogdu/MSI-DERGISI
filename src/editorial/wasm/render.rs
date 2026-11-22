//! Root DOM renderer & targeted focus-safe / scroll-preserving slot updaters (`Login.tsx` + `App.tsx` visual parity).

use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement, HtmlInputElement, HtmlTextAreaElement};

use crate::editorial::wasm::cropper::sync_cropper_canvas;
use crate::editorial::wasm::icons;
use crate::editorial::wasm::render_left::{render_left_panel_inner, render_left_tab_body};
use crate::editorial::wasm::render_middle::render_middle_panel_inner;
use crate::editorial::wasm::render_modals::{
    render_editor_confirm_modal_body, render_media_modal_grid_body, render_modals_inner,
    render_preview_modal_body, render_redirects_modal_body, render_revisions_modal_body,
    render_settings_glossary_list_html, render_settings_popover_inner,
    render_settings_popover_tab_body, render_toasts_inner,
};
use crate::editorial::wasm::render_right::{
    render_attached_source_bar_html, render_block_image_preview_inner,
    render_blocks_count_badge_html, render_blocks_list_html, render_editor_conflict_banner,
    render_embed_preview_card, render_related_search_results_html, render_right_panel_inner,
    render_step1_glossary_compliance_html, render_step1_glossary_drawer_list_html,
    render_step3_live_discover_and_serp, render_step3_tag_pills, render_step3_tag_suggestions,
};
use crate::editorial::wasm::render_research::{
    hide_reader_quote_bubble, render_reader_quote_bubble_shell, render_research_filters_html,
    render_research_results_html,
};
use crate::editorial::wasm::rich_editor::{hide_selection_bubble, render_selection_bubble_shell};
use crate::editorial::wasm::state::{
    admin_path, escape_html, is_valid_image_url, read_msi_theme, with_state, AppState, LeftTab,
    MobileView, NewsViewMode, PanelWidths, PreviewDevice, SettingsTab, PLACEHOLDER_IMG,
};

fn doc() -> Option<Document> {
    web_sys::window()?.document()
}

pub fn render_news_image(src: Option<&str>, alt: &str, class_name: &str) -> String {
    let alt_safe = if alt.trim().is_empty() {
        "Haber görseli".to_string()
    } else {
        escape_html(alt)
    };
    let cls_safe = escape_html(class_name);

    if !is_valid_image_url(src) {
        return format!(
            r#"<div class="flex items-center justify-center bg-zinc-100 {cls_safe}">
                <img src="{PLACEHOLDER_IMG}" alt="{alt_safe}" class="w-2/3 h-auto max-w-[120px] object-contain opacity-80" />
            </div>"#
        );
    }

    let src_safe = escape_html(src.unwrap_or_default());
    format!(
        r#"<img src="{src_safe}" alt="{alt_safe}" class="{cls_safe}" referrerpolicy="no-referrer" data-news-image="1" data-fallback-class="{cls_safe}" />"#
    )
}

/// MSI session-aware gate notice (rendered only when session cookie is missing/expired before redirect).
fn render_login_screen(s: &AppState) -> String {
    let admin_href = escape_html(&admin_path());
    let err_box = if !s.login_error.is_empty() {
        format!(
            r#"<div class="mb-6 p-3 sm:p-4 bg-red-50 border border-red-100 rounded-2xl flex items-start gap-3 text-red-600">
                {}
                <p class="text-sm">{}</p>
            </div>"#,
            icons::alert_circle("w-5 h-5 shrink-0 mt-0.5"),
            escape_html(&s.login_error),
        )
    } else {
        String::new()
    };

    format!(
        r#"<div class="msi-studio-boot">
            <div class="msi-studio-boot-card">
                <img src="/images/logo.svg" alt="MSI Dergisi" width="48" height="48" class="mx-auto mb-3 object-contain" />
                <h1>MSI Dergisi · Editör Stüdyosu</h1>
                <p>Editör Stüdyosu doğrudan Komuta Merkezi oturumu ve rol yetkisi ile çalışır.</p>
                {err_box}
                <div class="msi-studio-boot-actions">
                    <a href="{admin_href}" class="msi-studio-btn-primary">Komuta Merkezi Girişine Git</a>
                </div>
            </div>
        </div>"#
    )
}

/// MSI Dergisi Editorial Studio Top Header.
fn render_header(s: &AppState) -> String {
    let filter_active = s.show_search_filters
        || !s.search_start_date.is_empty()
        || !s.search_end_date.is_empty();
    let filter_btn_cls = if filter_active {
        "bg-brand-500 text-white shadow-sm"
    } else {
        "bg-zinc-100 text-zinc-500 hover:bg-zinc-200"
    };

    let filter_dropdown = if s.show_search_filters {
        format!(
            r#"<div class="absolute top-full left-4 right-4 lg:left-12 lg:right-12 mt-2 bg-white border border-zinc-200 shadow-xl rounded-2xl p-4 z-50 hf-modal-enter">
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <div class="space-y-1.5">
                        <label class="text-[9px] font-black uppercase tracking-widest text-zinc-500 px-1">Başlangıç Tarihi</label>
                        <input data-input="search-start-date" type="date" value="{sd}" class="w-full h-10 bg-zinc-100 rounded-xl px-3 text-xs font-bold focus:outline-none" />
                    </div>
                    <div class="space-y-1.5">
                        <label class="text-[9px] font-black uppercase tracking-widest text-zinc-500 px-1">Bitiş Tarihi</label>
                        <input data-input="search-end-date" type="date" value="{ed}" class="w-full h-10 bg-zinc-100 rounded-xl px-3 text-xs font-bold focus:outline-none" />
                    </div>
                </div>
                <div class="flex justify-end gap-2 mt-4 pt-4 border-t border-zinc-100">
                    <button type="button" data-action="clear-search-filters" class="px-4 h-8 text-[10px] font-black uppercase tracking-widest text-zinc-500 hover:text-zinc-700">Temizle</button>
                    <button type="button" data-action="apply-search-filters" class="px-6 h-8 bg-brand-500 text-white rounded-lg text-[10px] font-black uppercase tracking-widest shadow-md hover:bg-brand-600 transition-colors">Uygula</button>
                </div>
            </div>"#,
            sd = escape_html(&s.search_start_date),
            ed = escape_html(&s.search_end_date),
        )
    } else {
        String::new()
    };

    let role_upper = s
        .user
        .as_ref()
        .map(|u| u.role.to_uppercase())
        .unwrap_or_default();
    let uname = s
        .user
        .as_ref()
        .map(|u| u.username.as_str())
        .unwrap_or("admin");
    let initial = uname
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "A".to_string());
    let chev_rot = if s.is_profile_dropdown_open {
        "rotate-180"
    } else {
        ""
    };

    let admin_href = escape_html(&admin_path());
    let cur_theme = read_msi_theme();
    let theme_lbl = if cur_theme == "dark" { "Açık Tema" } else { "Koyu Tema" };
    let design_studio_item = if s.user.as_ref().map_or(false, |u| {
        u.capabilities.can_access_admin
            && matches!(
                u.role.as_str(),
                "super_admin" | "admin" | "site_operator" | "editor_in_chief" | "managing_editor"
            )
    }) {
        format!(
            r#"<a href="{admin_href}/tasarim" class="w-full px-4 py-2 text-left text-sm font-bold hover:bg-zinc-100 transition-colors flex items-center gap-2 text-zinc-800">
                    {design_ico}
                    Tasarım Stüdyosu
                </a>"#,
            design_ico = icons::layout_grid("w-4 h-4 text-zinc-500"),
        )
    } else {
        String::new()
    };
    let profile_menu = if s.is_profile_dropdown_open {
        format!(
            r#"<div data-action="close-profile-menu" class="fixed inset-0 z-40 bg-transparent"></div>
            <div class="absolute right-0 top-full mt-2 w-56 bg-white rounded-xl shadow-xl border border-zinc-200 py-2 z-50 origin-top-right overflow-hidden hf-modal-enter">
                <div class="px-4 py-2 border-b border-zinc-100 mb-1">
                    <div class="text-[10px] font-black uppercase tracking-widest text-zinc-400">Rol: {role}</div>
                    <div class="text-sm font-bold">{uname}</div>
                </div>
                <a href="{admin_href}" class="w-full px-4 py-2 text-left text-sm font-bold hover:bg-zinc-100 transition-colors flex items-center gap-2 text-zinc-800">
                    {home_ico}
                    Komuta Merkezi
                </a>
                {design_studio_item}
                <a href="/hesabim" class="w-full px-4 py-2 text-left text-sm font-bold hover:bg-zinc-100 transition-colors flex items-center gap-2 text-zinc-800">
                    {lock_ico}
                    Hesap Portalı
                </a>
                <button type="button" id="menu-msi-theme-toggle" data-action="toggle-msi-theme" class="w-full px-4 py-2 text-left text-sm font-bold hover:bg-zinc-100 transition-colors flex items-center gap-2 text-zinc-800">
                    {theme_ico}
                    <span>{theme_lbl}</span>
                </button>
                <div class="h-px bg-zinc-100 my-1.5"></div>
                <button type="button" data-action="logout" class="w-full px-4 py-2 text-left text-sm font-bold hover:bg-red-50 text-red-600 transition-colors flex items-center gap-2">
                    {out_ico}
                    Çıkış Yap
                </button>
                <div class="h-px bg-zinc-100 my-1"></div>
                <div class="px-4 py-2 text-[9px] text-zinc-400 font-semibold bg-zinc-50/60">
                    <div class="font-bold text-zinc-600">MSI Dergisi · Editör Stüdyosu</div>
                    <div>© 2026 UMSA Prodüksiyon</div>
                </div>
            </div>"#,
            role = escape_html(&role_upper),
            uname = escape_html(uname),
            home_ico = icons::arrow_left("w-4 h-4 text-zinc-500"),
            lock_ico = icons::user("w-4 h-4 text-zinc-500"),
            theme_ico = icons::sparkles("w-4 h-4 text-zinc-500"),
            out_ico = icons::log_out("w-4 h-4"),
        )
    } else {
        String::new()
    };

    format!(
        r#"<header class="h-[58px] sticky top-0 flex items-center justify-between gap-3 px-4 lg:px-5 shrink-0 z-[100] transition-colors border-b bg-white/90 backdrop-blur-md border-zinc-200">
            <div class="flex items-center gap-2.5 shrink-0 min-w-0">
                <a href="{admin_href}" class="px-2.5 h-8 rounded-lg border border-zinc-200 bg-zinc-50 hover:bg-zinc-100 text-zinc-700 text-[11px] font-bold transition-colors inline-flex items-center gap-1.5 whitespace-nowrap" title="Komuta Merkezine Dön">
                    {back_ico}
                    <span class="hidden sm:inline">Komuta Merkezi</span>
                </a>
                <span class="hidden md:inline text-zinc-300 font-bold select-none">/</span>
                <a href="{admin_href}/editor" class="flex items-center gap-2 group min-w-0" title="Editör Stüdyosu">
                    <img src="/images/logo.svg" alt="MSI Dergisi" class="h-7 w-auto object-contain" style="max-width: 112px;" />
                    <div class="hidden xl:flex flex-col leading-tight">
                        <span class="text-xs font-black tracking-tight text-zinc-900">MSI Dergisi</span>
                        <span class="text-[9.5px] font-bold uppercase tracking-widest text-brand-600">Editör Stüdyosu</span>
                    </div>
                </a>
            </div>

            <div class="flex-1 max-w-xl px-2 lg:px-6 relative min-w-0">
                <div class="flex items-center gap-2">
                    <div class="relative flex-1 min-w-0">
                        {search_ico}
                        <input id="hf-search-input" data-input="search-query" type="text" placeholder="Başlık, spot veya anahtar kelime ara..." value="{sq}" class="w-full h-9 bg-zinc-100 border border-zinc-200/70 rounded-xl pl-10 pr-4 text-xs font-semibold focus:outline-none focus:ring-2 focus:ring-brand-500/20 focus:border-brand-500 transition-all" />
                    </div>
                    <button id="hf-filter-btn" type="button" data-action="toggle-search-filters" class="w-9 h-9 flex flex-shrink-0 items-center justify-center rounded-xl border border-zinc-200/70 transition-all {filter_btn_cls}" title="Gelişmiş Arama Filtreleri">
                        {filter_ico}
                    </button>
                </div>
                {filter_dropdown}
            </div>

            <div class="flex items-center gap-2 shrink-0">
                <button id="btn-msi-theme-toggle" type="button" data-action="toggle-msi-theme" title="Tema Değiştir (Açık / Koyu)" class="px-3 h-8 rounded-lg border border-zinc-200 bg-zinc-50 hover:bg-zinc-100 text-zinc-700 text-[11px] font-bold transition-colors hidden sm:inline-flex items-center gap-1.5 whitespace-nowrap">
                    <span>{theme_lbl}</span>
                </button>
                <div class="relative">
                    <button type="button" data-action="toggle-profile-menu" class="group flex items-center gap-2 bg-zinc-50 hover:bg-zinc-100 border border-zinc-200 p-1 pr-2.5 rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-brand-500/20" title="Profil Menüsü">
                        <div class="w-7 h-7 bg-brand-500 rounded-full flex items-center justify-center text-white text-xs font-black shadow-sm group-hover:bg-brand-600 transition-colors">
                            {initial}
                        </div>
                        <div class="text-left hidden md:block leading-tight">
                            <div class="text-[11px] font-bold text-zinc-900 max-w-[140px] truncate">{uname}</div>
                            <div class="text-[9px] font-bold uppercase tracking-wider text-zinc-400">{role}</div>
                        </div>
                        {chev}
                    </button>
                    {profile_menu}
                </div>
            </div>
        </header>"#,
        search_ico =
            icons::search("absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-400 pointer-events-none"),
        filter_ico = icons::filter("w-4 h-4"),
        sq = escape_html(&s.search_query),
        back_ico = icons::arrow_left("w-3.5 h-3.5"),
        role = escape_html(&role_upper),
        uname = escape_html(uname),
        initial = escape_html(&initial),
        chev = icons::chevron_down(&format!(
            "w-3.5 h-3.5 text-zinc-500 transition-transform duration-200 {chev_rot}"
        )),
    )
}

fn render_workspace(s: &AppState) -> String {
    let panels_html = if s.is_desktop {
        let w = s.panel_widths;
        format!(
            r#"<div id="left-panel" style="flex: 0 0 {lw:.3}%; width: {lw:.3}%;" class="bg-white flex flex-col overflow-hidden border-r border-zinc-200 h-full">
                {left}
            </div>
            <div data-resize-handle="left-middle" class="bg-zinc-100 hover:bg-brand-500 active:bg-brand-600 transition-colors flex items-center justify-center z-50 w-1.5 cursor-col-resize border-x border-zinc-200/50 shrink-0">
                <div class="w-0.5 h-8 bg-zinc-300 rounded-full"></div>
            </div>
            <div id="middle-panel" style="flex: 1 1 {mw:.3}%; width: {mw:.3}%;" class="bg-white flex flex-col relative overflow-hidden border-r border-zinc-200 h-full">
                {middle}
            </div>
            <div data-resize-handle="middle-right" class="bg-zinc-100 hover:bg-brand-500 active:bg-brand-600 transition-colors flex items-center justify-center z-50 w-1.5 cursor-col-resize border-x border-zinc-200/50 shrink-0">
                <div class="w-0.5 h-8 bg-zinc-300 rounded-full"></div>
            </div>
            <div id="right-panel" style="flex: 0 0 {rw:.3}%; width: {rw:.3}%;" class="bg-white flex flex-col relative overflow-hidden h-full">
                {right}
            </div>"#,
            lw = w.left,
            mw = w.middle,
            rw = w.right,
            left = render_left_panel_inner(s),
            middle = render_middle_panel_inner(s),
            right = render_right_panel_inner(s),
        )
    } else {
        match s.mobile_view {
            MobileView::Left => format!(
                r#"<div id="left-panel" class="bg-white flex flex-col overflow-hidden w-full h-full">{}</div>"#,
                render_left_panel_inner(s)
            ),
            MobileView::Middle => format!(
                r#"<div id="middle-panel" class="bg-white flex flex-col relative overflow-hidden w-full h-full">{}</div>"#,
                render_middle_panel_inner(s)
            ),
            MobileView::Right => format!(
                r#"<div id="right-panel" class="bg-white flex flex-col relative overflow-hidden w-full h-full">{}</div>"#,
                render_right_panel_inner(s)
            ),
        }
    };

    format!(
        r#"<div class="flex flex-col h-screen w-full bg-[#f8f9fa] overflow-hidden">
            <div id="hf-header-slot" class="contents">
                {header}
            </div>
            <div id="hf-panel-group" class="flex-1 flex w-full h-full overflow-hidden">
                {panels_html}
            </div>
            <div id="hf-modals-slot">
                {modals}
            </div>
            <div id="hf-toast-slot">
                {toasts}
            </div>
            {selection_bubble}
            {reader_quote_bubble}
        </div>"#,
        header = render_header(s),
        modals = render_modals_inner(s),
        toasts = render_toasts_inner(s),
        selection_bubble = render_selection_bubble_shell(),
        reader_quote_bubble = render_reader_quote_bubble_shell(),
    )
}

// ============================================================================
// Public Render & Targeted Refresh Entry Points
// ============================================================================

pub fn render_full() {
    hide_selection_bubble();
    hide_reader_quote_bubble();
    let Some(document) = doc() else { return };
    let Some(root) = document.get_element_by_id("root") else {
        return;
    };

    let html = with_state(|s| {
        if s.auth_loading {
            format!(
                r#"<div class="min-h-screen bg-zinc-950 flex items-center justify-center text-white">{}</div>"#,
                icons::loader2("w-8 h-8 animate-spin text-brand-500")
            )
        } else if s.token.is_none() || s.user.is_none() {
            render_login_screen(s)
        } else {
            render_workspace(s)
        }
    });

    root.set_inner_html(&html);
    sync_textarea_heights();
    sync_cropper_canvas();
}

pub fn refresh_header_banner() {
    let Some(document) = doc() else { return };
    let Some(slot) = document.get_element_by_id("hf-header-slot") else {
        return;
    };

    let saved_focus = document.active_element().and_then(|el| {
        let id = el.id();
        if id == "hf-search-input" {
            let inp = el.dyn_into::<HtmlInputElement>().ok()?;
            Some((
                id,
                inp.selection_start().ok().flatten(),
                inp.selection_end().ok().flatten(),
            ))
        } else {
            None
        }
    });

    let html = with_state(render_header);
    slot.set_inner_html(&html);

    if let Some((id, s_start, s_end)) = saved_focus {
        if let Some(inp) = document
            .get_element_by_id(&id)
            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        {
            let _ = inp.focus();
            let _ = inp.set_selection_range(s_start.unwrap_or(0), s_end.unwrap_or(0));
        }
    }
}

/// Updates Left Panel toolbar buttons & total counter in-place WITHOUT detaching DOM nodes!
pub fn sync_left_toolbar_state() {
    let Some(document) = doc() else { return };
    let (left_tab, show_settings, view_mode, is_refreshing, pub_total) = with_state(|s| {
        (
            s.left_tab,
            s.show_settings_popover,
            s.news_view_mode,
            s.is_refreshing_articles || (s.left_tab == LeftTab::Research && s.is_searching_research),
            s.published_total,
        )
    });

    let active_tab_cls =
        "w-10 h-10 flex items-center justify-center rounded-full transition-all bg-white text-brand-500 shadow-sm ring-1 ring-zinc-200/50";
    let idle_tab_cls =
        "w-10 h-10 flex items-center justify-center rounded-full transition-all text-zinc-500 hover:text-zinc-900 hover:bg-zinc-200/50";

    if let Some(btn_pub) = document.get_element_by_id("btn-left-tab-published") {
        btn_pub.set_class_name(if left_tab == LeftTab::Published {
            active_tab_cls
        } else {
            idle_tab_cls
        });
    }
    if let Some(btn_drf) = document.get_element_by_id("btn-left-tab-drafts") {
        btn_drf.set_class_name(if left_tab == LeftTab::Drafts {
            active_tab_cls
        } else {
            idle_tab_cls
        });
    }
    if let Some(btn_res) = document.get_element_by_id("btn-left-tab-research") {
        btn_res.set_class_name(if left_tab == LeftTab::Research {
            active_tab_cls
        } else {
            idle_tab_cls
        });
    }
    if let Some(btn_cfg) = document.get_element_by_id("btn-settings-trigger") {
        let _ = btn_cfg.set_attribute("aria-expanded", if show_settings { "true" } else { "false" });
        btn_cfg.set_class_name(if show_settings {
            "w-10 h-10 flex items-center justify-center rounded-full transition-all bg-brand-500 text-white shadow-sm"
        } else {
            idle_tab_cls
        });
    }

    let active_view_cls =
        "w-7 h-7 flex items-center justify-center rounded-md transition-all bg-white text-brand-600 shadow-sm";
    let idle_view_cls =
        "w-7 h-7 flex items-center justify-center rounded-md transition-all text-zinc-400 hover:text-zinc-600";
    if let Some(btn_grid) = document.get_element_by_id("btn-view-grid") {
        btn_grid.set_class_name(if view_mode == NewsViewMode::Grid {
            active_view_cls
        } else {
            idle_view_cls
        });
    }
    if let Some(btn_list) = document.get_element_by_id("btn-view-list") {
        btn_list.set_class_name(if view_mode == NewsViewMode::List {
            active_view_cls
        } else {
            idle_view_cls
        });
    }

    if let Some(btn_ref) = document.get_element_by_id("btn-refresh-articles") {
        if is_refreshing {
            let _ = btn_ref.set_attribute("disabled", "");
            btn_ref.set_inner_html(&icons::loader2("w-4 h-4 animate-spin"));
        } else {
            let _ = btn_ref.remove_attribute("disabled");
            btn_ref.set_inner_html(&icons::refresh_cw("w-4 h-4"));
        }
    }

    if let Some(lbl) = document.get_element_by_id("left-pub-total-label") {
        lbl.set_text_content(Some(&format!("TOPLAM: {pub_total} YAYIN")));
    }
}

pub fn refresh_left_panel() {
    let Some(document) = doc() else { return };
    if document.get_element_by_id("left-scroll").is_some()
        && document.get_element_by_id("btn-left-tab-published").is_some()
    {
        sync_left_toolbar_state();
        refresh_left_body();
        return;
    }
    let Some(panel) = document.get_element_by_id("left-panel") else {
        return;
    };
    let prev_scroll = document
        .get_element_by_id("left-scroll")
        .map(|e| e.scroll_top())
        .unwrap_or(0);

    let html = with_state(render_left_panel_inner);
    panel.set_inner_html(&html);

    if let Some(sc) = document.get_element_by_id("left-scroll") {
        sc.set_scroll_top(prev_scroll);
    }
}

pub fn refresh_left_body() {
    let Some(document) = doc() else { return };
    let Some(sc) = document.get_element_by_id("left-scroll") else {
        return;
    };
    let prev_scroll = sc.scroll_top();
    let html = with_state(render_left_tab_body);
    sc.set_inner_html(&html);
    sc.set_scroll_top(prev_scroll);
}

/// Focus-safe updater for Left Research Desk filters & results so `#hf-research-input` never loses focus.
pub fn refresh_research_slots() {
    sync_left_toolbar_state();
    let Some(document) = doc() else { return };
    let is_res_tab = with_state(|s| s.left_tab == LeftTab::Research);
    if !is_res_tab {
        return;
    }
    let (Some(filters_el), Some(results_el)) = (
        document.get_element_by_id("hf-research-filters-slot"),
        document.get_element_by_id("hf-research-results-slot"),
    ) else {
        refresh_left_body();
        return;
    };
    let (filters_html, results_html, info_html, is_busy, q_val, show_info) = with_state(|s| {
        (
            render_research_filters_html(s),
            render_research_results_html(s),
            crate::editorial::wasm::render_research::render_research_info_drawer_html(s),
            s.is_searching_research,
            s.research_query.clone(),
            s.show_research_info,
        )
    });
    filters_el.set_inner_html(&filters_html);
    results_el.set_inner_html(&results_html);

    if let Some(info_el) = document.get_element_by_id("hf-research-info-slot") {
        info_el.set_inner_html(&info_html);
    }
    if let Some(btn_info) = document.get_element_by_id("btn-research-info") {
        let _ = btn_info.set_attribute("aria-expanded", if show_info { "true" } else { "false" });
        if show_info {
            btn_info.set_class_name("hf-research-info-btn hf-research-info-btn-active");
        } else {
            btn_info.set_class_name("hf-research-info-btn hf-research-info-btn-idle");
        }
    }

    if let Some(inp) = document
        .get_element_by_id("hf-research-input")
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
    {
        if document.active_element().map(|a| a.id()).as_deref() != Some("hf-research-input")
            && inp.value() != q_val
        {
            inp.set_value(&q_val);
        }
    }
    if let Some(clear_btn) = document.get_element_by_id("btn-research-clear-query") {
        if q_val.trim().is_empty() {
            let _ = clear_btn.class_list().add_1("hidden");
        } else {
            let _ = clear_btn.class_list().remove_1("hidden");
        }
    }
    if let Ok(Some(btn)) = document.query_selector("button[data-action='run-research-search']") {
        if is_busy {
            let _ = btn.set_attribute("disabled", "");
        } else {
            let _ = btn.remove_attribute("disabled");
        }
    }
}

pub fn refresh_attached_source_slot() {
    let Some(document) = doc() else { return };
    if let Some(slot) = document.get_element_by_id("hf-attached-source-slot") {
        let html = with_state(render_attached_source_bar_html);
        slot.set_inner_html(&html);
    }
}

pub fn refresh_middle_panel() {
    hide_reader_quote_bubble();
    let Some(document) = doc() else { return };
    let Some(panel) = document.get_element_by_id("middle-panel") else {
        return;
    };
    let html = with_state(render_middle_panel_inner);
    panel.set_inner_html(&html);
}

pub fn refresh_right_panel() {
    hide_selection_bubble();
    let Some(document) = doc() else { return };
    let Some(panel) = document.get_element_by_id("right-panel") else {
        return;
    };
    let prev_scroll = document
        .get_element_by_id("right-scroll")
        .map(|e| e.scroll_top())
        .unwrap_or(0);

    let html = with_state(render_right_panel_inner);
    panel.set_inner_html(&html);

    if let Some(sc) = document.get_element_by_id("right-scroll") {
        sc.set_scroll_top(prev_scroll);
    }
    sync_textarea_heights();
    sync_cropper_canvas();
}

pub fn refresh_blocks_list_only() {
    hide_selection_bubble();
    let Some(document) = doc() else { return };
    let Some(container) = document.get_element_by_id("blocks-list-container") else {
        return;
    };
    let (html, cnt) = with_state(|s| {
        let len = s
            .editing_article
            .blocks
            .as_ref()
            .map(|b| b.len())
            .unwrap_or(0);
        (render_blocks_list_html(s), len)
    });
    container.set_inner_html(&html);
    if let Some(badge) = document.get_element_by_id("blocks-count-badge") {
        badge.set_inner_html(&render_blocks_count_badge_html(cnt));
    }
    sync_textarea_heights();
}

pub fn refresh_single_block_image_preview(block_id: &str, content: &str) {
    let Some(document) = doc() else { return };
    let target_id = format!("block-img-preview-{block_id}");
    if let Some(el) = document.get_element_by_id(&target_id) {
        el.set_inner_html(&render_block_image_preview_inner(content));
    }
}

pub fn refresh_single_block_embed_preview(block_id: &str, url: &str, caption: Option<&str>) {
    let Some(document) = doc() else { return };
    let target_id = format!("block-embed-preview-{block_id}");
    if let Some(el) = document.get_element_by_id(&target_id) {
        el.set_inner_html(&render_embed_preview_card(url, caption));
    }
}

pub fn refresh_editor_lock_banner() {
    let Some(document) = doc() else { return };
    if let Some(slot) = document.get_element_by_id("hf-editor-lock-slot") {
        let html = with_state(render_editor_conflict_banner);
        slot.set_inner_html(&html);
    }
}

pub fn refresh_related_search_results(block_id: &str) {
    let Some(document) = doc() else { return };
    let target_id = format!("hf-rel-results-{block_id}");
    if let Some(slot) = document.get_element_by_id(&target_id) {
        let html = with_state(|s| render_related_search_results_html(s, block_id));
        slot.set_inner_html(&html);
    }
}

pub fn refresh_step1_glossary_slots() {
    let Some(document) = doc() else { return };
    let (compliance_html, drawer_list_html) = with_state(|s| {
        (
            render_step1_glossary_compliance_html(s),
            if s.show_glossary_drawer {
                Some(render_step1_glossary_drawer_list_html(s))
            } else {
                None
            },
        )
    });
    if let Some(comp_slot) = document.get_element_by_id("hf-glossary-compliance-slot") {
        comp_slot.set_inner_html(&compliance_html);
    }
    if let Some(list_html) = drawer_list_html {
        if let Some(list_slot) = document.get_element_by_id("hf-step1-glossary-list-slot") {
            list_slot.set_inner_html(&list_html);
        }
    }
}

pub fn refresh_settings_glossary_list_slot() {
    let Some(document) = doc() else { return };
    if let Some(slot) = document.get_element_by_id("hf-settings-glossary-list-slot") {
        let html = with_state(render_settings_glossary_list_html);
        slot.set_inner_html(&html);
    }
}

pub fn refresh_step3_live_seo() {
    let Some(document) = doc() else { return };
    if let Some(slot) = document.get_element_by_id("hf-step3-live-seo-panel") {
        let html = with_state(render_step3_live_discover_and_serp);
        slot.set_inner_html(&html);
    }
}

pub fn refresh_step3_tags_ui() {
    let Some(document) = doc() else { return };
    let (pills_html, sugs_html, seo_html) = with_state(|s| {
        (
            render_step3_tag_pills(s),
            render_step3_tag_suggestions(s),
            render_step3_live_discover_and_serp(s),
        )
    });
    if let Some(pills_slot) = document.get_element_by_id("hf-tags-pills-slot") {
        pills_slot.set_inner_html(&pills_html);
    }
    if let Some(sugs_slot) = document.get_element_by_id("hf-tag-suggestions-slot") {
        sugs_slot.set_inner_html(&sugs_html);
    }
    if let Some(seo_slot) = document.get_element_by_id("hf-step3-live-seo-panel") {
        seo_slot.set_inner_html(&seo_html);
    }
}

pub fn sync_media_modal_state(refresh_grid: bool) {
    let Some(document) = doc() else { return };
    let (is_open, wm_on, count_num) = with_state(|s| {
        (
            s.show_media_modal,
            s.upload_with_watermark,
            s.media_total.max(s.media_assets.len()),
        )
    });

    let (Some(modal), Some(backdrop)) = (
        document.get_element_by_id("hf-media-modal"),
        document.get_element_by_id("hf-media-backdrop"),
    ) else {
        refresh_modals();
        return;
    };

    if is_open {
        let _ = backdrop.class_list().remove_1("hf-popover-closed");
        let _ = backdrop.class_list().add_1("hf-popover-open");
        let _ = modal.class_list().remove_1("hf-popover-closed");
        let _ = modal.class_list().add_1("hf-popover-open");
        let _ = modal.set_attribute("aria-hidden", "false");
    } else {
        let _ = backdrop.class_list().remove_1("hf-popover-open");
        let _ = backdrop.class_list().add_1("hf-popover-closed");
        let _ = modal.class_list().remove_1("hf-popover-open");
        let _ = modal.class_list().add_1("hf-popover-closed");
        let _ = modal.set_attribute("aria-hidden", "true");
    }

    if let Some(wm_btn) = document.get_element_by_id("hf-media-wm-btn") {
        wm_btn.set_class_name(if wm_on {
            "hf-wm-toggle-btn hf-wm-toggle-active"
        } else {
            "hf-wm-toggle-btn hf-wm-toggle-idle"
        });
    }
    if let Some(cnt_el) = document.get_element_by_id("hf-media-count-badge") {
        cnt_el.set_text_content(Some(&format!("{count_num} Görsel")));
    }

    if refresh_grid {
        if let Some(grid_slot) = document.get_element_by_id("hf-media-grid-slot") {
            let html = with_state(render_media_modal_grid_body);
            grid_slot.set_inner_html(&html);
        }
    }
}

pub fn sync_redirects_modal_state() {
    let Some(document) = doc() else { return };
    let is_open = with_state(|s| s.show_redirects_modal);
    let (Some(modal), Some(backdrop)) = (
        document.get_element_by_id("hf-redirects-modal"),
        document.get_element_by_id("hf-redirects-backdrop"),
    ) else {
        refresh_modals();
        return;
    };
    if is_open {
        let _ = backdrop.class_list().remove_1("hf-popover-closed");
        let _ = backdrop.class_list().add_1("hf-popover-open");
        let _ = modal.class_list().remove_1("hf-popover-closed");
        let _ = modal.class_list().add_1("hf-popover-open");
        let _ = modal.set_attribute("aria-hidden", "false");
    } else {
        let _ = backdrop.class_list().remove_1("hf-popover-open");
        let _ = backdrop.class_list().add_1("hf-popover-closed");
        let _ = modal.class_list().remove_1("hf-popover-open");
        let _ = modal.class_list().add_1("hf-popover-closed");
        let _ = modal.set_attribute("aria-hidden", "true");
    }
    if let Some(body_slot) = document.get_element_by_id("hf-redirects-body-slot") {
        let html = with_state(render_redirects_modal_body);
        body_slot.set_inner_html(&html);
    }
}

pub fn sync_revisions_modal_state() {
    let Some(document) = doc() else { return };
    let (is_open, sub_txt) = with_state(|s| {
        let sub = if s.revisions_article_title.trim().is_empty() {
            "Kaydedilen tüm haber sürümleri ve tek tıkla geri yükleme".to_string()
        } else {
            s.revisions_article_title.clone()
        };
        (s.show_revisions_modal, sub)
    });
    let (Some(modal), Some(backdrop)) = (
        document.get_element_by_id("hf-revisions-modal"),
        document.get_element_by_id("hf-revisions-backdrop"),
    ) else {
        refresh_modals();
        return;
    };
    if is_open {
        let _ = backdrop.class_list().remove_1("hf-popover-closed");
        let _ = backdrop.class_list().add_1("hf-popover-open");
        let _ = modal.class_list().remove_1("hf-popover-closed");
        let _ = modal.class_list().add_1("hf-popover-open");
        let _ = modal.set_attribute("aria-hidden", "false");
    } else {
        let _ = backdrop.class_list().remove_1("hf-popover-open");
        let _ = backdrop.class_list().add_1("hf-popover-closed");
        let _ = modal.class_list().remove_1("hf-popover-open");
        let _ = modal.class_list().add_1("hf-popover-closed");
        let _ = modal.set_attribute("aria-hidden", "true");
    }
    if let Some(sub_el) = document.get_element_by_id("hf-revisions-subtitle") {
        sub_el.set_text_content(Some(&sub_txt));
    }
    if let Some(body_slot) = document.get_element_by_id("hf-revisions-body-slot") {
        let html = with_state(render_revisions_modal_body);
        body_slot.set_inner_html(&html);
    }
}

pub fn sync_preview_modal_state() {
    let Some(document) = doc() else { return };
    let (is_open, dev, pub_dis) = with_state(|s| {
        (
            s.show_preview_modal,
            s.preview_device,
            s.is_posting || s.canonical_editing_article().title.trim().is_empty(),
        )
    });
    let (Some(modal), Some(backdrop)) = (
        document.get_element_by_id("hf-preview-modal"),
        document.get_element_by_id("hf-preview-backdrop"),
    ) else {
        refresh_modals();
        return;
    };

    if is_open {
        let _ = backdrop.class_list().remove_1("hf-popover-closed");
        let _ = backdrop.class_list().add_1("hf-popover-open");
        let _ = modal.class_list().remove_1("hf-popover-closed");
        let _ = modal.class_list().add_1("hf-popover-open");
        let _ = modal.set_attribute("aria-hidden", "false");
    } else {
        let _ = backdrop.class_list().remove_1("hf-popover-open");
        let _ = backdrop.class_list().add_1("hf-popover-closed");
        let _ = modal.class_list().remove_1("hf-popover-open");
        let _ = modal.class_list().add_1("hf-popover-closed");
        let _ = modal.set_attribute("aria-hidden", "true");
    }

    let active_dev_cls =
        "px-3 h-8 rounded-lg text-[10px] font-black uppercase tracking-wider flex items-center gap-1.5 transition-all bg-white text-zinc-900 shadow-sm";
    let idle_dev_cls =
        "px-3 h-8 rounded-lg text-[10px] font-black uppercase tracking-wider flex items-center gap-1.5 transition-all text-zinc-500 hover:text-zinc-900";

    if let Some(b_desk) = document.get_element_by_id("btn-preview-desk") {
        b_desk.set_class_name(if dev == PreviewDevice::Desktop {
            active_dev_cls
        } else {
            idle_dev_cls
        });
    }
    if let Some(b_mob) = document.get_element_by_id("btn-preview-mob") {
        b_mob.set_class_name(if dev == PreviewDevice::Mobile {
            active_dev_cls
        } else {
            idle_dev_cls
        });
    }
    if let Some(b_pub) = document.get_element_by_id("btn-preview-publish-now") {
        if pub_dis {
            let _ = b_pub.set_attribute("disabled", "");
        } else {
            let _ = b_pub.remove_attribute("disabled");
        }
    }

    if is_open {
        if let Some(body_slot) = document.get_element_by_id("hf-preview-body-slot") {
            let html = with_state(render_preview_modal_body);
            body_slot.set_inner_html(&html);
        }
    }
}

pub fn sync_editor_confirm_modal_state() {
    let Some(document) = doc() else { return };
    let is_open = with_state(|s| s.pending_editor_confirm.is_some());
    let (Some(modal), Some(backdrop)) = (
        document.get_element_by_id("hf-editor-confirm-modal"),
        document.get_element_by_id("hf-editor-confirm-backdrop"),
    ) else {
        refresh_modals();
        return;
    };

    if is_open {
        let _ = backdrop.class_list().remove_1("hf-popover-closed");
        let _ = backdrop.class_list().add_1("hf-popover-open");
        let _ = modal.class_list().remove_1("hf-popover-closed");
        let _ = modal.class_list().add_1("hf-popover-open");
        let _ = modal.set_attribute("aria-hidden", "false");
        if let Some(slot) = document.get_element_by_id("hf-editor-confirm-body-slot") {
            let html = with_state(render_editor_confirm_modal_body);
            slot.set_inner_html(&html);
        }
    } else {
        let _ = backdrop.class_list().remove_1("hf-popover-open");
        let _ = backdrop.class_list().add_1("hf-popover-closed");
        let _ = modal.class_list().remove_1("hf-popover-open");
        let _ = modal.class_list().add_1("hf-popover-closed");
        let _ = modal.set_attribute("aria-hidden", "true");
    }
}

pub fn refresh_modals() {
    let Some(document) = doc() else { return };
    let Some(slot) = document.get_element_by_id("hf-modals-slot") else {
        return;
    };
    let html = with_state(render_modals_inner);
    slot.set_inner_html(&html);
}

/// Synchronizes the centered Ayarlar modal popup visibility & sub-tab state in-place without detaching its shell or buttons!
pub fn sync_settings_popover_state(refresh_body: bool) {
    let Some(document) = doc() else { return };
    let (is_open, active_subtab) = with_state(|s| (s.show_settings_popover, s.settings_tab));

    let (Some(popover), Some(backdrop)) = (
        document.get_element_by_id("hf-settings-popover"),
        document.get_element_by_id("hf-settings-backdrop"),
    ) else {
        refresh_modals();
        return;
    };

    if is_open {
        let _ = backdrop.class_list().remove_1("hf-popover-closed");
        let _ = backdrop.class_list().add_1("hf-popover-open");
        let _ = popover.class_list().remove_1("hf-popover-closed");
        let _ = popover.class_list().add_1("hf-popover-open");
        let _ = popover.set_attribute("aria-hidden", "false");
    } else {
        let _ = backdrop.class_list().remove_1("hf-popover-open");
        let _ = backdrop.class_list().add_1("hf-popover-closed");
        let _ = popover.class_list().remove_1("hf-popover-open");
        let _ = popover.class_list().add_1("hf-popover-closed");
        let _ = popover.set_attribute("aria-hidden", "true");
    }

    let active_sub_cls =
        "flex-1 px-2 h-8 text-[9.5px] font-black uppercase tracking-wider rounded-lg transition-all whitespace-nowrap bg-white shadow-sm text-brand-600";
    let idle_sub_cls =
        "flex-1 px-2 h-8 text-[9.5px] font-black uppercase tracking-wider rounded-lg transition-all whitespace-nowrap text-zinc-500 hover:text-zinc-900";

    if let Some(btn_cats) = document.get_element_by_id("btn-settings-tab-categories") {
        btn_cats.set_class_name(if active_subtab == SettingsTab::Categories {
            active_sub_cls
        } else {
            idle_sub_cls
        });
    }
    if let Some(btn_sources) = document.get_element_by_id("btn-settings-tab-sources") {
        btn_sources.set_class_name(if active_subtab == SettingsTab::Sources {
            active_sub_cls
        } else {
            idle_sub_cls
        });
    }
    if let Some(btn_issues) = document.get_element_by_id("btn-settings-tab-issues") {
        btn_issues.set_class_name(if active_subtab == SettingsTab::Issues {
            active_sub_cls
        } else {
            idle_sub_cls
        });
    }
    if let Some(btn_glossary) = document.get_element_by_id("btn-settings-tab-glossary") {
        btn_glossary.set_class_name(if active_subtab == SettingsTab::Glossary {
            active_sub_cls
        } else {
            idle_sub_cls
        });
    }
    if let Some(btn_team) = document.get_element_by_id("btn-settings-tab-team") {
        btn_team.set_class_name(if active_subtab == SettingsTab::Team {
            active_sub_cls
        } else {
            idle_sub_cls
        });
    }

    if refresh_body {
        if let Some(sc) = document.get_element_by_id("hf-settings-bubble-scroll") {
            let prev_scroll = sc.scroll_top();
            let body_html = with_state(render_settings_popover_tab_body);
            sc.set_inner_html(&body_html);
            sc.set_scroll_top(prev_scroll);
        } else if let Some(inner_el) = document.get_element_by_id("hf-settings-bubble-inner") {
            let html = with_state(render_settings_popover_inner);
            inner_el.set_inner_html(&html);
        }
    }
}

pub fn refresh_settings_popover() {
    sync_settings_popover_state(true);
}

pub fn refresh_toasts() {
    let Some(document) = doc() else { return };
    let Some(slot) = document.get_element_by_id("hf-toast-slot") else {
        return;
    };
    let html = with_state(render_toasts_inner);
    slot.set_inner_html(&html);
}

pub fn apply_panel_widths_to_dom(w: PanelWidths) {
    let Some(document) = doc() else { return };
    if let Some(lp) = document
        .get_element_by_id("left-panel")
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = lp
            .style()
            .set_property("flex", &format!("0 0 {:.3}%", w.left));
        let _ = lp
            .style()
            .set_property("width", &format!("{:.3}%", w.left));
    }
    if let Some(mp) = document
        .get_element_by_id("middle-panel")
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = mp
            .style()
            .set_property("flex", &format!("1 1 {:.3}%", w.middle));
        let _ = mp
            .style()
            .set_property("width", &format!("{:.3}%", w.middle));
    }
    if let Some(rp) = document
        .get_element_by_id("right-panel")
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = rp
            .style()
            .set_property("flex", &format!("0 0 {:.3}%", w.right));
        let _ = rp
            .style()
            .set_property("width", &format!("{:.3}%", w.right));
    }
}

pub fn auto_fit_textarea(ta: &HtmlTextAreaElement) {
    let _ = ta.style().set_property("height", "auto");
    let sh = ta.scroll_height();
    if sh > 0 {
        let _ = ta.style().set_property("height", &format!("{sh}px"));
    }
}

pub fn sync_textarea_heights() {
    let step = with_state(|s| s.editor_step);
    if step != 1 {
        return;
    }
    let Some(document) = doc() else { return };

    if let Some(title_ta) = document
        .get_element_by_id("editor-title-textarea")
        .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
    {
        auto_fit_textarea(&title_ta);
    }

    if let Some(spot_ta) = document
        .get_element_by_id("editor-spot-textarea")
        .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
    {
        auto_fit_textarea(&spot_ta);
    }

    if let Ok(list) = document.query_selector_all("textarea[data-input='block-content']") {
        for idx in 0..list.length() {
            if let Some(node) = list.item(idx) {
                if let Ok(ta) = node.dyn_into::<HtmlTextAreaElement>() {
                    auto_fit_textarea(&ta);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editorial::wasm::state::{LeftTab, SettingsTab};
    use crate::editorial::common::{Article, EditorialCapabilities, User};

    #[test]
    fn renders_msi_session_gate_markup_without_external_links() {
        let mut s = AppState::new();
        s.auth_loading = false;
        s.login_error = "Oturum gerekli.".to_string();
        let html = render_login_screen(&s);
        assert!(html.contains("MSI Dergisi"));
        assert!(html.contains("Oturum gerekli."));
        assert!(html.contains("/images/logo.svg"));
        assert!(html.contains("/podmin"));
        assert!(!html.contains("https://"));
    }

    #[test]
    fn renders_publishing_only_three_column_workspace_and_floating_settings_popover() {
        let mut s = AppState::new();
        s.auth_loading = false;
        s.token = Some("__cookie_session__".to_string());
        s.user = Some(User {
            id: 1,
            username: "admin".to_string(),
            role: "admin".to_string(),
            capabilities: EditorialCapabilities::from_permissions(&["*".into()]),
            ..Default::default()
        });

        let ws1 = render_workspace(&s);
        assert!(ws1.contains("bg-white/90 backdrop-blur-md border-zinc-200"));
        assert!(!ws1.contains("Tasarruf Modu Devrede"));
        assert!(!ws1.contains("Yapay Zeka Tarama"));
        assert!(ws1.contains("Başlık, spot veya anahtar kelime ara..."));
        assert!(ws1.contains("Komuta Merkezi"));
        assert!(ws1.contains("ADMIN"));
        assert!(ws1.contains("Yayınlanan Haberler (0)"));
        assert!(ws1.contains("YAYIN SİSTEMİ AKTİF"));
        assert!(ws1.contains("TOPLAM: 0 YAYIN"));
        assert!(ws1.contains("Listeyi Yenile"));
        assert!(ws1.contains("Yeni Haber"));
        assert!(ws1.contains("id=\"btn-settings-trigger\""));
        assert!(ws1.contains("data-action=\"toggle-settings-popover\""));
        assert!(ws1.contains("id=\"hf-settings-popover\""));
        assert!(ws1.contains("aria-hidden=\"true\""));
        assert!(ws1.contains("Haber Seçilmedi"));
        assert!(ws1.contains("1. İçerik Taslağı"));
        assert!(ws1.contains("İLERLE"));
        assert!(ws1.contains("id=\"hf-selection-bubble\""));
        assert!(ws1.contains("id=\"hf-reader-quote-bubble\""));
        assert!(ws1.contains("id=\"btn-left-tab-research\""));
        assert!(ws1.contains("Video / Tweet Göm"));
        assert!(ws1.contains("Öne Çıkanlar Özeti"));
        assert!(ws1.contains("Bilgi / Son Dakika"));

        // Header filter dropdown has NO WordPress category selector
        s.show_search_filters = true;
        let hdr_html = render_header(&s);
        assert!(hdr_html.contains("Başlangıç Tarihi"));
        assert!(hdr_html.contains("Bitiş Tarihi"));
        assert!(!hdr_html.contains("Kategori (WP)"));
        assert!(!hdr_html.contains("search-category"));
        s.show_search_filters = false;

        // Reader with published article
        s.selected_article = Some(Article {
            id: "pub-1".to_string(),
            article_type: Some("published".to_string()),
            title: "Örnek Yayınlanan Haber Başlığı".to_string(),
            content: "<p>Haber detay içeriği burada yer alıyor.</p>".to_string(),
            status: "publish".to_string(),
            wp_post_url: Some("https://haber.example.com/ornek-haber".to_string()),
            ..Default::default()
        });
        let ws_reader = render_workspace(&s);
        assert!(ws_reader.contains("Örnek Yayınlanan Haber Başlığı"));
        assert!(ws_reader.contains("YAYINDA"));
        assert!(ws_reader.contains("Editöre Aktar"));
        assert!(ws_reader.contains("https://haber.example.com/ornek-haber"));
        assert!(!ws_reader.contains("Tekrar Yaz"));

        // Opening Settings opens a Centered Modal Popup with Categories & Team (no Social Media, no WordPress)!
        assert_eq!(s.left_tab, LeftTab::Published);
        s.show_settings_popover = true;
        let ws_settings = render_workspace(&s);
        assert_eq!(s.left_tab, LeftTab::Published);
        assert!(ws_settings.contains("Yayınlanan Haberler (0)"));
        assert!(ws_settings.contains("id=\"hf-settings-popover\""));
        assert!(ws_settings.contains("data-action=\"close-settings-popover\""));
        assert!(ws_settings.contains("Kategoriler"));
        assert!(ws_settings.contains("Yeni Ana / Alt Kategori Ekle"));
        assert!(ws_settings.contains("data-change=\"new-cat-parent\""));
        assert!(ws_settings.contains("Kaynaklar"));
        assert!(ws_settings.contains("Ekip"));
        assert!(!ws_settings.contains("X (Twitter) API"));
        assert!(!ws_settings.contains("Telegram Bağlantısını Test Et"));
        assert!(!ws_settings.contains("WordPress"));
        assert!(!ws_settings.contains("Ek WordPress Siteleri"));
        assert!(!ws_settings.contains("Yapay Zeka"));

        // Switching Settings sub-tab to Sources renders research source manager
        s.settings_tab = SettingsTab::Sources;
        let ws_src = render_workspace(&s);
        assert!(ws_src.contains("Yeni Araştırma Kaynağı Ekle"));
        assert!(ws_src.contains("Kayıtlı Kaynaklar"));

        // Switching Settings sub-tab to Team inside the popup keeps Drafts tab intact behind it
        s.left_tab = LeftTab::Drafts;
        s.settings_tab = SettingsTab::Team;
        let ws_team = render_workspace(&s);
        assert!(ws_team.contains("Taslaklar (0)"));
        assert!(ws_team.contains("MSI Ekip &amp; 17-Rol Yetki Yönetimi"));
        assert!(ws_team.contains("Komuta Merkezi Rol Yönetimine Git"));
        assert!(!ws_team.contains("WordPress"));
        s.show_settings_popover = false;

        // Wizard Step 2, 3, 4
        s.editor_step = 2;
        let ws2 = render_workspace(&s);
        assert!(ws2.contains("Öne Çıkan Görsel Seçimi & Kırpma (1200x675)"));

        s.editor_step = 3;
        let ws3 = render_workspace(&s);
        assert!(ws3.contains("SEO Ayarlar"));
        assert!(ws3.contains("Otomatik Doldur"));
        assert!(!ws3.contains("AI ile Oluştur"));

        s.editor_step = 4;
        s.categories = vec![
            crate::editorial::common::Category {
                id: 1,
                name: "Spor".to_string(),
                slug: "spor".to_string(),
                color: Some("#10b981".to_string()),
                parent_id: None,
                created_at: None,
            },
            crate::editorial::common::Category {
                id: 2,
                name: "Futbol".to_string(),
                slug: "futbol".to_string(),
                color: Some("#10b981".to_string()),
                parent_id: Some(1),
                created_at: None,
            },
        ];
        s.editing_article.categories = Some(vec![1, 2]);
        s.editing_article.status = "publish".to_string();
        let ws4 = render_workspace(&s);
        assert!(ws4.contains("Son Kontrol & Yayınlama"));
        assert!(ws4.contains("Tam Ekran Önizle"));
        assert!(ws4.contains("Orta Ekranda Göster"));
        assert!(ws4.contains("id=\"hf-preview-modal\""));
        assert!(ws4.contains("data-action=\"publish-now\""));
        assert!(ws4.contains("Hemen Yayınla"));
        assert!(ws4.contains("Taslak Olarak Kaydet"));
        assert!(ws4.contains("Haber Kategorisi"));
        assert!(ws4.contains("data-change=\"step4-parent-category\""));
        assert!(ws4.contains("data-change=\"step4-sub-category\""));
        assert!(ws4.contains("Spor › Futbol"));
        assert!(ws4.contains("Hassas İçerik & Haber Kontrolleri"));
        assert!(ws4.contains("data-action=\"toggle-comments-locked\""));
        assert!(ws4.contains("data-action=\"toggle-ads-disabled\""));
        assert!(!ws4.contains("toggle-article-category"));
        assert!(!ws4.contains("Gönderilecek Siteler"));
        assert!(!ws4.contains("post-wordpress"));

        // Middle column live draft preview & Full-screen Desktop/Mobile preview modal
        s.middle_preview_draft = true;
        s.show_preview_modal = true;
        s.preview_device = PreviewDevice::Mobile;
        s.editing_article.title = "Canlı Taslak Test Başlığı".to_string();
        let ws_prev = render_workspace(&s);
        assert!(ws_prev.contains("CANLI TASLAK ÖNİZLEMESİ"));
        assert!(ws_prev.contains("Orta Ekranda Açık ✓"));
        assert!(ws_prev.contains("hf-mobile-phone-frame"));
        assert!(ws_prev.contains("Canlı Taslak Test Başlığı"));
        s.middle_preview_draft = false;
        s.show_preview_modal = false;

        // Live conflict lock banner & card badge test
        s.editing_article.id = "pub-1".to_string();
        s.active_locks = vec![crate::editorial::common::ArticleLock {
            article_id: "pub-1".to_string(),
            user_id: 42,
            username: "mehmet_editor".to_string(),
            updated_at_ms: 1700000000000,
        }];
        let ws_locked = render_workspace(&s);
        assert!(ws_locked.contains("Bu haber şu an mehmet_editor tarafından düzenleniyor"));

        // List View displays article Spot under the title; Grid View hides it
        s.published_articles = vec![Article {
            id: "pub-spot-1".to_string(),
            title: "Enflasyon Verileri Açıklandı".to_string(),
            spot: Some("Kasım ayı tüketici fiyat endeksi beklentilerin altında gerçekleşti.".to_string()),
            status: "publish".to_string(),
            ..Default::default()
        }];
        s.left_tab = LeftTab::Published;
        s.news_view_mode = NewsViewMode::Grid;
        let ws_grid = render_workspace(&s);
        assert!(!ws_grid.contains("hf-list-card-spot"));

        s.news_view_mode = NewsViewMode::List;
        let ws_list = render_workspace(&s);
        assert!(ws_list.contains("hf-list-card-spot"));
        assert!(ws_list.contains("Kasım ayı tüketici fiyat endeksi beklentilerin altında gerçekleşti."));

        // Floating open research tabs pill appears when middle_showing_research is false and tabs exist
        s.research_tabs.push(crate::editorial::wasm::state::ResearchReaderTab {
            url: "https://www.ntv.com.tr/ekonomi/test-1".to_string(),
            title_hint: "Test Başlık".to_string(),
            source_hint: "NTV".to_string(),
            is_loading: false,
            error: None,
            extracted: None,
        });
        s.middle_showing_research = false;
        let ws_res_pill = render_workspace(&s);
        assert!(ws_res_pill.contains("🔍 Açık Araştırma (1)"));
        assert!(ws_res_pill.contains("data-action=\"enter-research-reader\""));
    }

    #[test]
    fn renders_msi_bilingual_editor_glossary_compliance_taxonomy_workflow_and_settings() {
        let mut s = AppState::new();
        s.auth_loading = false;
        s.token = Some("__cookie_session__".to_string());
        s.user = Some(User {
            id: 1,
            username: "msi_editor".to_string(),
            role: "admin".to_string(),
            capabilities: EditorialCapabilities::from_permissions(&["*".into()]),
            ..Default::default()
        });
        s.magazine_issues = vec![crate::editorial::common::MagazineIssueItem {
            id: 10,
            issue_number: 286,
            edition_lang: "tr".to_string(),
            title_tr: "MSI Dergisi Sayı 286".to_string(),
            title_en: "MSI TDR Issue 153".to_string(),
            publish_month: "Kasım".to_string(),
            year: 2026,
            is_special_issue: true,
            event_name: Some("SAHA EXPO".to_string()),
        }];
        s.ssb_tags_catalog = vec![crate::editorial::common::SsbTag {
            id: 1,
            code: "insansiz-sistemler".to_string(),
            category_group: "Platformlar".to_string(),
            name_tr: "İnsansız Hava/Kara/Deniz Sistemleri".to_string(),
            name_en: "Unmanned Aerial/Ground/Naval Systems".to_string(),
            sort_order: 1,
        }];
        s.glossary_rules = vec![crate::editorial::common::GlossaryRule {
            id: 1,
            term_tr: "Türkiye".to_string(),
            term_en: "Türkiye".to_string(),
            abbr_tr: None,
            abbr_en: None,
            category: "institutional".to_string(),
            rule_type: "fixed".to_string(),
            preferred_en: "Türkiye".to_string(),
            forbidden_variants: vec!["Turkey".to_string()],
            case_sensitive: false,
            note: Some("Resmi BM/NATO ülke yazımı".to_string()),
        }];

        // 1. Step 1 Bilingual Switcher + Live Glossary Compliance + Author/Video Bar
        s.editing_article.title = "KAAN Milli Muharip Uçak İlk Uçuşunu Tamamladı".to_string();
        s.editing_article.author_name = Some("Dr. Kemal Aydın".to_string());
        s.editing_article.author_title = Some("Savunma Sanayii Analisti".to_string());
        s.editing_article.video_url = Some("https://youtu.be/dQw4w9WgXcQ".to_string());
        s.switch_editor_lang("en");
        s.editing_article.title = "Turkey Defence Industry Milestone".to_string();
        s.show_glossary_drawer = true;

        let ws_step1 = render_workspace(&s);
        assert!(ws_step1.contains("data-action=\"set-editor-lang\""));
        assert!(ws_step1.contains("data-action=\"copy-tr-to-en-assisted\""));
        assert!(ws_step1.contains("TR'den Aktar (Sözlük Destekli)"));
        assert!(ws_step1.contains("MSI TDR &amp; NATO Terminoloji Uyarısı"));
        assert!(ws_step1.contains("data-action=\"apply-glossary-fixes\""));
        assert!(ws_step1.contains("English Headline (MSI TDR)"));
        assert!(ws_step1.contains("Dr. Kemal Aydın"));
        assert!(ws_step1.contains("data-input=\"article-video-url\""));
        assert!(ws_step1.contains("MSI TDR &amp; NATO Resmi Terminoloji Rehberi"));

        // Auto-fixing glossary violations turns alert into emerald compliant badge
        let fixed = s.apply_active_en_glossary_fixes();
        assert!(fixed >= 1);
        let ws_step1_ok = render_workspace(&s);
        assert!(ws_step1_ok.contains("MSI TDR &amp; NATO Terminoloji Uyumlu (British English)"));

        // 2. Step 4 MSI Publishing Cards, SSB Taxonomy, Paywall, Sponsorship & Multi-stage Workflow
        s.editor_step = 4;
        s.editing_article.publication_channel = "magazine".to_string();
        s.editing_article.content_type = "kapak-dosyasi".to_string();
        s.editing_article.issue_number = Some(286);
        s.editing_article.issue_page_range = Some("22-30".to_string());
        s.editing_article.is_featured = true;
        s.editing_article.ssb_tags = Some(vec!["insansiz-sistemler".to_string()]);
        s.editing_article.visibility_level = 2;
        s.editing_article.single_price_try = 45.0;
        s.editing_article.is_sponsored = true;
        s.editing_article.sponsor_brand = Some("TUSAŞ".to_string());
        s.editing_article.status = "pending_review".to_string();
        s.editing_article.en_status = "approved".to_string();
        s.middle_preview_draft = true;
        s.preview_lang = "en".to_string();

        let ws_step4 = render_workspace(&s);
        assert!(ws_step4.contains("data-action=\"set-pub-channel\""));
        assert!(ws_step4.contains("data-change=\"article-content-type\""));
        assert!(ws_step4.contains("Kapak Dosyası"));
        assert!(ws_step4.contains("MSI Dergisi Sayı 286"));
        assert!(ws_step4.contains("1 SSB Etiketi Seçili"));
        assert!(ws_step4.contains("data-action=\"toggle-ssb-tag\""));
        assert!(ws_step4.contains("data-input=\"article-single-price\""));
        assert!(ws_step4.contains("data-input=\"article-sponsor-brand\""));
        assert!(ws_step4.contains("Başeditör Onayına Gönder"));
        // Middle live preview renders English edition with MSI TDR ribbon, sponsor banner, video & SSB taxonomy
        assert!(ws_step4.contains("data-action=\"set-preview-lang\""));
        assert!(ws_step4.contains("MSI TDR · ISSUE 286 · PP. 22-30"));
        assert!(ws_step4.contains("Türkiye Defence Industry Milestone"));
        assert!(ws_step4.contains("TUSAŞ"));
        assert!(ws_step4.contains("Unmanned Aerial/Ground/Naval Systems"));
        assert!(ws_step4.contains("https://www.youtube.com/embed/dQw4w9WgXcQ?rel=0"));

        // 3. Left Drafts Tab Status Filters & Card Badges
        s.left_tab = LeftTab::Drafts;
        let canon = s.canonical_editing_article();
        s.draft_articles = vec![canon];
        s.draft_total = 1;
        let ws_left = render_workspace(&s);
        assert!(ws_left.contains("data-action=\"set-draft-status-filter\""));
        assert!(ws_left.contains("Onay Bekleyen"));
        assert!(ws_left.contains("DERGİ #286"));
        assert!(ws_left.contains("EN ✓"));
        assert!(ws_left.contains("ONAY BEKLİYOR"));
        assert!(ws_left.contains("PAYWALL"));
        assert!(ws_left.contains("SPONSORLU"));

        // 4. Settings Popover: Issues & Glossary Tabs
        s.show_settings_popover = true;
        s.settings_tab = SettingsTab::Issues;
        let ws_iss = render_workspace(&s);
        assert!(ws_iss.contains("Dergi Sayıları"));
        assert!(ws_iss.contains("data-action=\"create-magazine-issue\""));
        assert!(ws_iss.contains("SAHA EXPO"));

        s.settings_tab = SettingsTab::Glossary;
        let ws_glo = render_workspace(&s);
        assert!(ws_glo.contains("Terminoloji"));
        assert!(ws_glo.contains("data-action=\"create-glossary-rule\""));
        assert!(ws_glo.contains("Turkey"));
    }
}
