//! Modals, Centered Settings Popup (`Kategoriler`, `Kaynaklar`, `Dergi Sayıları`, `Terminoloji`) & Toasts renderer for MSI Dergisi Editorial Studio.

use crate::editorial::common::{
    build_category_tree, collect_descendant_category_ids, slugify_turkish, Category,
};

use crate::editorial::wasm::icons;
use crate::editorial::wasm::render::render_news_image;
use crate::editorial::wasm::render_middle::render_draft_article_sheet_html;
use crate::editorial::wasm::state::{
    admin_path, escape_html, format_bytes_human, format_middle_turkish_date, AppState,
    PendingEditorConfirm, PreviewDevice, SettingsTab, ToastItem, ToastKind,
};

pub const CATEGORY_COLOR_PALETTE: &[(&str, &str)] = &[
    ("#3b66c4", "Mavi"),
    ("#10b981", "Yeşil"),
    ("#f59e0b", "Turuncu"),
    ("#ef4444", "Kırmızı"),
    ("#8b5cf6", "Mor"),
    ("#18181b", "Siyah"),
];

pub fn render_modals_inner(s: &AppState) -> String {
    let mut out = String::new();

    if s.user.is_some() || s.show_settings_popover {
        out.push_str(&render_settings_popover_shell(s));
    }

    out.push_str(&render_media_modal_shell(s));
    out.push_str(&render_redirects_modal_shell(s));
    out.push_str(&render_revisions_modal_shell(s));
    out.push_str(&render_preview_modal_shell(s));
    out.push_str(&render_editor_confirm_modal_shell(s));

    out
}

fn render_settings_popover_shell(s: &AppState) -> String {
    let state_cls = if s.show_settings_popover {
        "hf-popover-open"
    } else {
        "hf-popover-closed"
    };
    format!(
        r#"<div id="hf-settings-backdrop" data-action="close-settings-popover" class="hf-settings-backdrop {state_cls}"></div>
        <div id="hf-settings-popover" role="dialog" aria-modal="true" aria-label="Ayarlar" aria-hidden="{aria_hid}" class="hf-settings-popover {state_cls}">
            <div id="hf-settings-bubble-inner" class="hf-settings-bubble-card">
                {inner}
            </div>
        </div>"#,
        aria_hid = if s.show_settings_popover { "false" } else { "true" },
        inner = render_settings_popover_inner(s),
    )
}

pub fn render_settings_popover_tab_body(s: &AppState) -> String {
    match s.settings_tab {
        SettingsTab::Categories => render_settings_categories(s),
        SettingsTab::Sources => render_settings_sources(s),
        SettingsTab::Issues => render_settings_issues(s),
        SettingsTab::Glossary => render_settings_glossary(s),
        SettingsTab::Team => render_settings_team(s),
    }
}

pub fn render_settings_popover_inner(s: &AppState) -> String {
    let can_team = s.caps().can_access_admin;

    let sub_btn = |tab: SettingsTab, btn_id: &str, key: &str, label: &str| -> String {
        let cls = if s.settings_tab == tab {
            "bg-white shadow-sm text-brand-600"
        } else {
            "text-zinc-500 hover:text-zinc-900"
        };
        format!(
            r#"<button id="{btn_id}" type="button" data-action="set-settings-tab" data-subtab="{key}" class="flex-1 px-3 min-h-[36px] text-xs font-semibold rounded-lg transition-all whitespace-nowrap {cls}">{label}</button>"#
        )
    };

    let b_team = if can_team || s.settings_tab == SettingsTab::Team {
        sub_btn(SettingsTab::Team, "btn-settings-tab-team", "team", "Ekip & Roller")
    } else {
        String::new()
    };

    let sub_content = render_settings_popover_tab_body(s);

    format!(
        r#"<div class="hf-modal-header px-6 py-4 border-b border-zinc-100 flex items-center justify-between gap-3 shrink-0">
            <div class="flex items-center gap-3">
                <div class="w-9 h-9 rounded-xl bg-brand-50 text-brand-500 flex items-center justify-center shrink-0">
                    {gear_ico}
                </div>
                <div>
                    <h3 class="text-sm font-bold text-zinc-900">Sistem &amp; MSI Yayın Ayarları</h3>
                    <p class="text-xs font-medium text-zinc-500">Kategori, araştırma kaynağı, dergi sayıları, MSI TDR terminolojisi ve ekip yönetimi</p>
                </div>
            </div>
            <div class="flex items-center gap-2">
                <button type="button" data-action="close-settings-popover" aria-label="Ayarları kapat" title="Kapat" class="w-9 h-9 flex items-center justify-center rounded-lg text-zinc-500 hover:text-zinc-900 hover:bg-zinc-100 transition-colors">
                    <span class="text-lg leading-none">&times;</span>
                </button>
            </div>
        </div>
        <div class="hf-modal-subheader px-6 py-3 border-b border-zinc-100 shrink-0 overflow-x-auto">
            <div class="flex bg-zinc-100 p-1 rounded-xl border border-zinc-200/60 gap-1 min-w-max">
                {b_cats}
                {b_sources}
                {b_issues}
                {b_glossary}
                {b_team}
            </div>
        </div>
        <div id="hf-settings-bubble-scroll" class="overflow-y-auto p-6 custom-scrollbar">
            {sub_content}
        </div>
        <div class="hf-modal-footer px-6 py-3.5 border-t border-zinc-100 shrink-0 flex items-center justify-between text-xs text-zinc-500 font-medium select-none">
            <div class="flex items-center gap-1.5">
                <span class="font-black text-zinc-700">MSI Dergisi · Editör Stüdyosu</span>
                <span class="bg-zinc-200/70 text-zinc-600 px-1.5 py-0.5 rounded text-xs font-bold">v2.6</span>
                <span>•</span>
                <span>© 2026</span>
            </div>
            <a href="{admin_href}" class="text-zinc-600 hover:text-brand-600 font-bold transition-colors flex items-center gap-1">
                <span>Komuta Merkezi</span>
            </a>
        </div>"#,
        admin_href = escape_html(&admin_path()),
        gear_ico = icons::settings("w-4 h-4"),
        b_cats = sub_btn(
            SettingsTab::Categories,
            "btn-settings-tab-categories",
            "categories",
            "Kategoriler"
        ),
        b_sources = sub_btn(
            SettingsTab::Sources,
            "btn-settings-tab-sources",
            "sources",
            "Kaynaklar"
        ),
        b_issues = sub_btn(
            SettingsTab::Issues,
            "btn-settings-tab-issues",
            "issues",
            "Dergi Sayıları"
        ),
        b_glossary = sub_btn(
            SettingsTab::Glossary,
            "btn-settings-tab-glossary",
            "glossary",
            "Terminoloji"
        ),
    )
}

fn render_color_swatches(selected_hex: &str, action_name: &str) -> String {
    let mut out = String::new();
    let norm = if selected_hex.trim().is_empty() {
        "#3b66c4"
    } else {
        selected_hex.trim()
    };
    for (hex, label) in CATEGORY_COLOR_PALETTE {
        let is_sel = norm.eq_ignore_ascii_case(hex);
        let ring = if is_sel {
            "ring-2 ring-offset-2 ring-zinc-900 scale-110"
        } else {
            "opacity-75 hover:opacity-100"
        };
        let chk = if is_sel {
            icons::check("w-3 h-3 text-white")
        } else {
            String::new()
        };
        out.push_str(&format!(
            r#"<button type="button" data-action="{action_name}" data-color="{hex}" title="{label}" style="background-color: {hex};" class="w-6 h-6 rounded-full flex items-center justify-center transition-all {ring}">{chk}</button>"#
        ));
    }
    out
}

fn render_parent_select_options(
    categories: &[Category],
    selected_parent_id: Option<i32>,
    exclude_self_id: Option<i32>,
) -> String {
    let excluded: std::collections::HashSet<i32> = match exclude_self_id {
        Some(eid) => {
            let mut set: std::collections::HashSet<i32> =
                collect_descendant_category_ids(categories, eid)
                    .into_iter()
                    .collect();
            set.insert(eid);
            set
        }
        None => std::collections::HashSet::new(),
    };

    let root_sel = if selected_parent_id.is_none() {
        "selected"
    } else {
        ""
    };
    let mut opts = format!(
        r#"<option value="0" {root_sel}>— Ana Kategori (Üst Yok) —</option>"#
    );

    for item in build_category_tree(categories) {
        if excluded.contains(&item.category.id) {
            continue;
        }
        let sel = if selected_parent_id == Some(item.category.id) {
            "selected"
        } else {
            ""
        };
        let indent = if item.depth == 0 {
            String::new()
        } else {
            format!("{}↳ ", "\u{00A0}\u{00A0}".repeat(item.depth))
        };
        opts.push_str(&format!(
            r#"<option value="{id}" {sel}>{indent}{name}</option>"#,
            id = item.category.id,
            indent = indent,
            name = escape_html(&item.category.name),
        ));
    }

    opts
}

fn render_settings_categories(s: &AppState) -> String {
    let auto_slug_hint = if s.new_cat_name.trim().is_empty() {
        "otomatik-kisa-ad (opsiyonel)".to_string()
    } else {
        slugify_turkish(&s.new_cat_name)
    };
    let save_dis = if s.is_saving_category { "disabled" } else { "" };
    let tree = build_category_tree(&s.categories);
    let new_parent_opts = render_parent_select_options(&s.categories, s.new_cat_parent_id, None);

    let selected_parent_hint = s
        .new_cat_parent_id
        .and_then(|pid| tree.iter().find(|t| t.category.id == pid))
        .map(|p| {
            format!(
                r#"<div class="flex items-center justify-between px-3 py-1.5 rounded-xl bg-brand-50 text-brand-600 text-xs font-bold">
                    <span>Alt kategori eklenecek üst dal: <strong>{path}</strong></span>
                    <button type="button" data-action="clear-new-cat-parent" class="text-xs font-black uppercase tracking-widest hover:underline">Ana Kategoriye Dön</button>
                </div>"#,
                path = escape_html(&p.path_label),
            )
        })
        .unwrap_or_default();

    let mut list_html = String::new();
    if tree.is_empty() {
        list_html.push_str(&format!(
            r#"<div class="py-8 text-center bg-zinc-50/80 border border-dashed border-zinc-200 rounded-2xl space-y-2">
                <div class="w-10 h-10 rounded-full bg-white border border-zinc-200 flex items-center justify-center mx-auto text-zinc-400">
                    {tag_ico}
                </div>
                <div class="text-[11px] font-black uppercase tracking-widest text-zinc-500">Henüz Kategori Eklenmedi</div>
                <p class="text-xs font-medium text-zinc-400">Önce ana kategorilerinizi (Örn: Spor, Ekonomi), ardından alt kategorilerinizi (Örn: Futbol, Borsa) ekleyebilirsiniz.</p>
            </div>"#,
            tag_ico = icons::tag("w-4 h-4"),
        ));
    } else {
        list_html.push_str(r#"<div class="space-y-2">"#);
        for item in &tree {
            let cat = &item.category;
            let color = cat.color.as_deref().unwrap_or("#3b66c4");
            if s.editing_cat_id == Some(cat.id) {
                let edit_parent_opts = render_parent_select_options(
                    &s.categories,
                    s.edit_cat_parent_id,
                    Some(cat.id),
                );
                list_html.push_str(&format!(
                    r#"<div class="p-4 bg-zinc-50 border-2 border-brand-500/40 rounded-2xl space-y-3">
                        <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                            <input id="hf-edit-cat-name" data-input="edit-cat-name" type="text" placeholder="Kategori Adı" value="{ename}" class="w-full h-9 bg-white border border-zinc-200 rounded-xl px-3.5 text-xs font-bold focus:outline-none focus:border-brand-500" />
                            <div class="relative">
                                <select data-change="edit-cat-parent" class="w-full h-9 bg-white border border-zinc-200 rounded-xl px-3 pr-8 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500 appearance-none">
                                    {edit_parent_opts}
                                </select>
                                <div class="absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                            </div>
                            <input data-input="edit-cat-slug" type="text" placeholder="kisa-ad-slug" value="{eslug}" class="w-full h-9 bg-white border border-zinc-200 rounded-xl px-3.5 text-[11px] font-semibold text-zinc-600 focus:outline-none focus:border-brand-500" />
                        </div>
                        <div class="flex items-center justify-between gap-3 flex-wrap">
                            <div class="flex items-center gap-2">
                                <span class="text-xs font-black uppercase tracking-widest text-zinc-400">Renk:</span>
                                {edit_swatches}
                            </div>
                            <div class="flex items-center gap-2">
                                <button type="button" data-action="cancel-edit-category" class="px-4 min-h-[36px] bg-white border border-zinc-200 text-zinc-700 rounded-[10px] text-xs font-semibold hover:bg-zinc-100 transition-all">İptal</button>
                                <button type="button" data-action="confirm-edit-category" {save_dis} class="px-4 min-h-[36px] bg-brand-500 text-white rounded-[10px] text-xs font-semibold hover:bg-brand-600 transition-all disabled:opacity-50">Güncelle</button>
                            </div>
                        </div>
                    </div>"#,
                    ename = escape_html(&s.edit_cat_name),
                    eslug = escape_html(&s.edit_cat_slug),
                    chev_ico = icons::chevron_down("w-3.5 h-3.5 text-zinc-400"),
                    edit_swatches = render_color_swatches(&s.edit_cat_color, "pick-edit-cat-color"),
                ));
            } else {
                let indent_rem = (item.depth as f64) * 1.35;
                let row_bg = if item.depth == 0 {
                    "bg-white border-zinc-200"
                } else {
                    "bg-zinc-50/80 border-zinc-200/80"
                };
                let hierarchy_badge = if item.depth == 0 {
                    r#"<span class="px-2 py-0.5 rounded-md bg-zinc-100 text-zinc-500 text-[11.5px] font-black uppercase tracking-widest">ANA KATEGORİ</span>"#.to_string()
                } else {
                    format!(
                        r#"<span class="px-2 py-0.5 rounded-md bg-brand-50 text-brand-600 text-[11.5px] font-black uppercase tracking-wider">{path}</span>"#,
                        path = escape_html(&item.path_label),
                    )
                };
                let branch_prefix = if item.depth > 0 {
                    r#"<span class="text-zinc-300 font-bold text-xs leading-none select-none">↳</span>"#
                } else {
                    ""
                };

                list_html.push_str(&format!(
                    r#"<div style="margin-left: {indent_rem:.2}rem;" class="flex items-center justify-between gap-2 p-2.5 px-3.5 border rounded-xl hover:border-zinc-300 transition-all {row_bg}">
                        <div class="flex items-center gap-2.5 min-w-0">
                            {branch_prefix}
                            <span style="background-color: {color};" class="w-3 h-3 rounded-full shrink-0 shadow-sm"></span>
                            <div class="min-w-0">
                                <div class="flex items-center gap-2 flex-wrap">
                                    <span class="text-xs font-bold text-zinc-900 truncate">{name}</span>
                                    {hierarchy_badge}
                                </div>
                                <div class="text-xs font-semibold text-zinc-400 truncate">/{slug}</div>
                            </div>
                        </div>
                        <div class="flex items-center gap-1 shrink-0">
                            {sub_btns}
                        </div>
                    </div>"#,
                    color = escape_html(color),
                    name = escape_html(&cat.name),
                    slug = escape_html(&cat.slug),
                    sub_btns = {
                        let edit_part = if s.caps().can_manage_categories {
                            format!(
                                r#"<button type="button" data-action="quick-add-subcategory" data-id="{}" class="px-2 h-7 flex items-center gap-1 rounded-lg text-xs font-black uppercase tracking-wider text-zinc-500 hover:text-brand-600 hover:bg-brand-50 transition-colors" title="Bu kategorinin altına alt kategori ekle">{}<span>Alt</span></button><button type="button" data-action="start-edit-category" data-id="{}" class="w-7 h-7 flex items-center justify-center rounded-lg text-zinc-400 hover:text-brand-600 hover:bg-brand-50 transition-colors" title="Kategoriyi Düzenle">{}</button>"#,
                                cat.id,
                                icons::plus("w-3 h-3"),
                                cat.id,
                                icons::pen_square("w-3.5 h-3.5"),
                            )
                        } else {
                            String::new()
                        };
                        let del_part = if s.caps().can_delete_categories {
                            format!(
                                r#"<button type="button" data-action="delete-category" data-id="{}" class="w-7 h-7 flex items-center justify-center rounded-lg text-zinc-400 hover:text-red-500 hover:bg-red-50 transition-colors" title="Kategoriyi Sil">{}</button>"#,
                                cat.id,
                                icons::trash2("w-3.5 h-3.5"),
                            )
                        } else {
                            String::new()
                        };
                        format!("{edit_part}{del_part}")
                    },
                ));
            }
        }
        list_html.push_str("</div>");
    }

    let root_count = tree.iter().filter(|t| t.depth == 0).count();
    let sub_count = tree.len().saturating_sub(root_count);

    let create_cat_box = if s.caps().can_manage_categories {
        format!(
            r#"<div class="p-4 rounded-2xl bg-zinc-50/80 border border-zinc-200/70 space-y-3">
                <div class="flex items-center justify-between">
                    <h2 class="text-xs font-black uppercase tracking-widest text-zinc-500 flex items-center gap-1.5">
                        {tag_ico} Yeni Ana / Alt Kategori Ekle
                    </h2>
                    <span class="text-xs font-bold uppercase tracking-widest text-zinc-400">Anında Kaydedilir</span>
                </div>
                {selected_parent_hint}
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                    <input id="hf-new-cat-name" data-input="new-cat-name" type="text" autocomplete="off" placeholder="Kategori Adı (Örn: Spor, Futbol)" value="{nc_name}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-bold focus:outline-none focus:border-brand-500 transition-all" />
                    <div class="relative">
                        <select data-change="new-cat-parent" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 pr-8 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500 appearance-none">
                            {new_parent_opts}
                        </select>
                        <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                    </div>
                    <input id="hf-new-cat-slug" data-input="new-cat-slug" type="text" autocomplete="off" placeholder="{slug_ph}" value="{nc_slug}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-[11px] font-semibold text-zinc-600 focus:outline-none focus:border-brand-500 transition-all" />
                </div>
                <div class="flex items-center justify-between gap-3 flex-wrap pt-1">
                    <div class="flex items-center gap-2">
                        <span class="text-xs font-black uppercase tracking-widest text-zinc-400">Renk Etiketi:</span>
                        {swatches}
                    </div>
                    <button type="button" data-action="create-category" {save_dis} class="px-5 min-h-[38px] bg-brand-500 hover:bg-brand-600 text-white rounded-[10px] text-[13px] font-semibold flex items-center gap-2 shadow-sm transition-all disabled:opacity-50">
                        {plus_ico}
                        <span>Kategori Ekle</span>
                    </button>
                </div>
            </div>"#,
            tag_ico = icons::tag("w-3.5 h-3.5 text-brand-500"),
            chev_ico = icons::chevron_down("w-3.5 h-3.5 text-zinc-400"),
            plus_ico = icons::plus("w-3.5 h-3.5"),
            nc_name = escape_html(&s.new_cat_name),
            nc_slug = escape_html(&s.new_cat_slug),
            slug_ph = escape_html(&auto_slug_hint),
            swatches = render_color_swatches(&s.new_cat_color, "pick-new-cat-color"),
        )
    } else {
        r#"<div class="p-3 rounded-xl bg-zinc-50 border border-zinc-200 text-[11px] font-semibold text-zinc-600">Kategori ekleme/düzenleme yetkisi (<code>categories:write</code>) sadece yetkili rollere açıktır.</div>"#.to_string()
    };

    format!(
        r#"<div class="space-y-5">
            {create_cat_box}
            <div class="space-y-2.5">
                <div class="flex items-center justify-between px-1">
                    <h3 class="text-xs font-black uppercase tracking-widest text-zinc-400">Kategori Ağacı ({root_count} Ana, {sub_count} Alt)</h3>
                </div>
                {list_html}
            </div>
        </div>"#,
    )
}

fn render_source_category_options(selected: &str) -> String {
    const CATS: &[&str] = &[
        "Gündem",
        "Savunma ve Askeri",
        "Ajans",
        "Ekonomi",
        "Dünya",
        "Teknoloji",
        "Spor",
        "Yerel",
    ];
    let cur = if CATS.iter().any(|c| c.eq_ignore_ascii_case(selected.trim())) {
        selected.trim()
    } else {
        "Gündem"
    };
    let mut opts = String::new();
    for cat in CATS {
        let sel = if cat.eq_ignore_ascii_case(cur) {
            "selected"
        } else {
            ""
        };
        opts.push_str(&format!(
            r#"<option value="{c}" {sel}>{c}</option>"#,
            c = escape_html(cat),
        ));
    }
    opts
}

fn render_source_scope_options(selected: &str) -> String {
    let is_global = selected.trim().eq_ignore_ascii_case("global");
    let tr_sel = if !is_global { "selected" } else { "" };
    let gl_sel = if is_global { "selected" } else { "" };
    format!(
        r#"<option value="tr" {tr_sel}>🇹🇷 Türkçe Basın</option>
        <option value="global" {gl_sel}>🌍 Dünya Basını</option>"#
    )
}

fn render_settings_sources(s: &AppState) -> String {
    let save_dis = if s.is_saving_source { "disabled" } else { "" };
    let save_btn_inner = if s.is_saving_source {
        format!(
            "{} <span>Keşfediliyor...</span>",
            icons::loader2("w-3.5 h-3.5 animate-spin")
        )
    } else {
        format!(
            "{} <span>+ Kaynağı Kaydet</span>",
            icons::rss("w-3.5 h-3.5")
        )
    };

    let new_cat_opts = render_source_category_options(&s.new_source_category);
    let new_scope_opts = render_source_scope_options(&s.new_source_scope);

    let mut list_html = String::new();
    if s.research_sources.is_empty() {
        list_html.push_str(&format!(
            r#"<div class="py-8 text-center bg-zinc-50/80 border border-dashed border-zinc-200 rounded-2xl space-y-2">
                <div class="w-10 h-10 rounded-full bg-white border border-zinc-200 flex items-center justify-center mx-auto text-zinc-400">
                    {rss_ico}
                </div>
                <div class="text-[11px] font-black uppercase tracking-widest text-zinc-500">Henüz Kayıtlı Kaynak Yok</div>
                <p class="text-xs font-medium text-zinc-400">Yukarıdaki formdan ajans, ulusal veya yerel haber sitesi / RSS adresini ekleyebilirsiniz.</p>
            </div>"#,
            rss_ico = icons::rss("w-4 h-4"),
        ));
    } else {
        list_html.push_str(r#"<div class="space-y-2">"#);
        for src in &s.research_sources {
            if s.editing_source_id == Some(src.id) {
                let edit_cat_opts = render_source_category_options(&s.edit_source_category);
                let edit_scope_opts = render_source_scope_options(&s.edit_source_scope);
                list_html.push_str(&format!(
                    r#"<div class="p-4 bg-zinc-50 border-2 border-brand-500/40 rounded-2xl space-y-3">
                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                            <input id="hf-edit-source-url" data-input="edit-source-url" type="text" placeholder="RSS veya Site Adresi" value="{eurl}" class="w-full h-9 bg-white border border-zinc-200 rounded-xl px-3.5 text-[11px] font-semibold text-zinc-700 focus:outline-none focus:border-brand-500" />
                            <input id="hf-edit-source-name" data-input="edit-source-name" type="text" placeholder="Kaynak Adı" value="{ename}" class="w-full h-9 bg-white border border-zinc-200 rounded-xl px-3.5 text-xs font-bold focus:outline-none focus:border-brand-500" />
                        </div>
                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                            <div class="relative">
                                <select data-change="edit-source-category" class="w-full h-9 bg-white border border-zinc-200 rounded-xl px-3 pr-8 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500 appearance-none">
                                    {edit_cat_opts}
                                </select>
                                <div class="absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                            </div>
                            <div class="relative">
                                <select data-change="edit-source-scope" class="w-full h-9 bg-white border border-zinc-200 rounded-xl px-3 pr-8 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500 appearance-none">
                                    {edit_scope_opts}
                                </select>
                                <div class="absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                            </div>
                        </div>
                        <div class="flex items-center justify-end gap-2">
                            <button type="button" data-action="cancel-edit-source" class="px-4 min-h-[36px] bg-white border border-zinc-200 text-zinc-700 rounded-[10px] text-xs font-semibold hover:bg-zinc-100 transition-all">İptal</button>
                            <button type="button" data-action="confirm-edit-source" {save_dis} class="px-4 min-h-[36px] bg-brand-500 text-white rounded-[10px] text-xs font-semibold hover:bg-brand-600 transition-all disabled:opacity-50">Güncelle</button>
                        </div>
                    </div>"#,
                    eurl = escape_html(&s.edit_source_url),
                    ename = escape_html(&s.edit_source_name),
                    chev_ico = icons::chevron_down("w-3.5 h-3.5 text-zinc-400"),
                ));
            } else {
                let status_dot = if src.enabled {
                    r#"<span class="hf-source-status-dot hf-source-status-on" title="Aktif"></span>"#
                } else {
                    r#"<span class="hf-source-status-dot hf-source-status-off" title="Pasif"></span>"#
                };
                let row_opacity = if src.enabled {
                    "bg-white border-zinc-200"
                } else {
                    "bg-zinc-50/80 border-zinc-200/70 opacity-75"
                };
                let kind_badge = if src.kind == "rss" {
                    r#"<span class="hf-source-badge hf-source-badge-rss">RSS AKIŞI</span>"#
                } else {
                    r#"<span class="hf-source-badge hf-source-badge-site">SİTE TARAMA</span>"#
                };
                let cat_label = if src.category.trim().is_empty() {
                    "Gündem"
                } else {
                    src.category.trim()
                };
                let scope_label = if src.scope.trim().eq_ignore_ascii_case("global") {
                    "GLOBAL"
                } else {
                    "TR"
                };
                let (toggle_cls, toggle_lbl) = if src.enabled {
                    ("hf-source-toggle-pill hf-source-toggle-on", "Aktif")
                } else {
                    ("hf-source-toggle-pill hf-source-toggle-off", "Pasif")
                };
                let clean_href = crate::editorial::wasm::render_research::safe_http_url(&src.url);
                let url_sub = if let Some(ref href) = clean_href {
                    format!(
                        r#"<a href="{h}" target="_blank" rel="noopener noreferrer" class="hf-source-url-sub">{u}</a>"#,
                        h = escape_html(href),
                        u = escape_html(&src.url),
                    )
                } else {
                    format!(
                        r#"<span class="hf-source-url-sub">{}</span>"#,
                        escape_html(&src.url)
                    )
                };

                list_html.push_str(&format!(
                    r#"<div class="flex items-center justify-between gap-2.5 p-3 border rounded-xl hover:border-zinc-300 transition-all {row_opacity}">
                        <div class="flex items-start gap-2.5 min-w-0 flex-1">
                            {status_dot}
                            <div class="min-w-0 flex-1 space-y-1">
                                <div class="flex items-center gap-1.5 flex-wrap">
                                    <span class="text-xs font-bold text-zinc-900 truncate">{name}</span>
                                    {kind_badge}
                                    <span class="hf-source-badge hf-source-badge-cat">{cat}</span>
                                    <span class="hf-source-badge hf-source-badge-scope">{scope}</span>
                                </div>
                                <div class="truncate">{url_sub}</div>
                            </div>
                        </div>
                        <div class="flex items-center gap-1 shrink-0">
                            <button type="button" data-action="toggle-research-source" data-id="{id}" class="{toggle_cls}" title="Kaynağı Aktif / Pasif Yap">{toggle_lbl}</button>
                            <button type="button" data-action="start-edit-source" data-id="{id}" class="w-7 h-7 flex items-center justify-center rounded-lg text-zinc-400 hover:text-brand-600 hover:bg-brand-50 transition-colors" title="Kaynağı Düzenle">{edit_ico}</button>
                            <button type="button" data-action="delete-research-source" data-id="{id}" class="w-7 h-7 flex items-center justify-center rounded-lg text-zinc-400 hover:text-red-500 hover:bg-red-50 transition-colors" title="Kaynağı Sil">{del_ico}</button>
                        </div>
                    </div>"#,
                    id = src.id,
                    name = escape_html(&src.name),
                    cat = escape_html(cat_label),
                    scope = scope_label,
                    edit_ico = icons::pen_square("w-3.5 h-3.5"),
                    del_ico = icons::trash2("w-3.5 h-3.5"),
                ));
            }
        }
        list_html.push_str("</div>");
    }

    format!(
        r#"<div class="space-y-5">
            <div class="hf-sources-info-banner">
                {info_ico}
                <p>Doğrudan RSS adresi (<code>.../rss.xml</code>) veya haber sitesi adresi (<code>https://ekonomim.com</code>) girebilirsiniz. Sistem RSS akışını otomatik keşfeder; RSS yoksa site içi canlı tarama modunda çalıştırır.</p>
            </div>

            <div class="p-4 rounded-2xl bg-zinc-50/80 border border-zinc-200/70 space-y-3">
                <div class="flex items-center justify-between">
                    <h2 class="text-xs font-black uppercase tracking-widest text-zinc-500 flex items-center gap-1.5">
                        {rss_ico} Yeni Araştırma Kaynağı Ekle
                    </h2>
                    <span class="text-xs font-bold uppercase tracking-widest text-zinc-400">Otomatik RSS Keşfi</span>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                    <input id="hf-new-source-url" data-input="new-source-url" type="text" autocomplete="off" placeholder="RSS veya Site Adresi (Örn: https://www.aa.com.tr veya .../rss.xml)" value="{ns_url}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-[11px] font-semibold text-zinc-800 focus:outline-none focus:border-brand-500 transition-all" />
                    <input id="hf-new-source-name" data-input="new-source-name" type="text" autocomplete="off" placeholder="Kaynak Adı (Boş bırakılırsa otomatik bulunur)" value="{ns_name}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-bold focus:outline-none focus:border-brand-500 transition-all" />
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                    <div class="relative">
                        <select data-change="new-source-category" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 pr-8 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500 appearance-none">
                            {new_cat_opts}
                        </select>
                        <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                    </div>
                    <div class="relative">
                        <select data-change="new-source-scope" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 pr-8 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500 appearance-none">
                            {new_scope_opts}
                        </select>
                        <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                    </div>
                </div>
                <div class="flex justify-end pt-1">
                    <button type="button" data-action="create-research-source" {save_dis} class="px-5 min-h-[38px] bg-brand-500 hover:bg-brand-600 text-white rounded-[10px] text-[13px] font-semibold flex items-center gap-2 shadow-sm transition-all disabled:opacity-50">
                        {save_btn_inner}
                    </button>
                </div>
            </div>

            <div class="space-y-2.5">
                <div class="flex items-center justify-between px-1">
                    <h3 class="text-xs font-black uppercase tracking-widest text-zinc-400">Kayıtlı Kaynaklar ({count})</h3>
                </div>
                {list_html}
            </div>
        </div>"#,
        info_ico = icons::info("w-4 h-4 text-brand-500 shrink-0 mt-0.5"),
        rss_ico = icons::rss("w-3.5 h-3.5 text-brand-500"),
        chev_ico = icons::chevron_down("w-3.5 h-3.5 text-zinc-400"),
        ns_url = escape_html(&s.new_source_url),
        ns_name = escape_html(&s.new_source_name),
        count = s.research_sources.len(),
    )
}

fn render_settings_issues(s: &AppState) -> String {
    let _save_dis = if s.is_saving_issue { "disabled" } else { "" };
    let is_en_lang = s.new_issue_lang.trim().eq_ignore_ascii_case("en");
    let tr_sel = if !is_en_lang { "selected" } else { "" };
    let en_sel = if is_en_lang { "selected" } else { "" };

    let spec_cls = if s.new_issue_special {
        "bg-brand-50 border-brand-500 text-brand-700"
    } else {
        "bg-white border-zinc-200 text-zinc-600 hover:bg-zinc-50"
    };
    let spec_chk = if s.new_issue_special {
        icons::check("w-3 h-3 text-brand-600")
    } else {
        String::new()
    };

    let mut list_html = String::new();
    if s.magazine_issues.is_empty() {
        list_html.push_str(&format!(
            r#"<div class="py-8 text-center bg-zinc-50/80 border border-dashed border-zinc-200 rounded-2xl space-y-2">
                <div class="w-10 h-10 rounded-full bg-white border border-zinc-200 flex items-center justify-center mx-auto text-zinc-400">
                    {book_ico}
                </div>
                <div class="text-[11px] font-black uppercase tracking-widest text-zinc-500">Henüz Dergi Sayısı Eklenmedi</div>
                <p class="text-xs font-medium text-zinc-400">Yukarıdaki formdan MSI Dergisi (TR) veya MSI TDR (EN) sayılarını tanımlayabilirsiniz.</p>
            </div>"#,
            book_ico = icons::book_open("w-4 h-4"),
        ));
    } else {
        list_html.push_str(r#"<div class="space-y-2">"#);
        for iss in &s.magazine_issues {
            let is_en = iss.edition_lang.trim().eq_ignore_ascii_case("en");
            let lang_badge = if is_en {
                r#"<span class="px-2 py-0.5 rounded bg-sky-50 text-sky-700 border border-sky-200 text-[11.5px] font-black uppercase tracking-wider">🇬🇧 MSI TDR</span>"#
            } else {
                r#"<span class="px-2 py-0.5 rounded bg-rose-50 text-rose-700 border border-rose-200 text-[11.5px] font-black uppercase tracking-wider">🇹🇷 MSI DERGİSİ</span>"#
            };
            let title_main = if is_en && !iss.title_en.trim().is_empty() {
                iss.title_en.as_str()
            } else {
                iss.title_tr.as_str()
            };
            let sub_title = if !is_en && !iss.title_en.trim().is_empty() {
                format!(
                    r#"<span class="text-xs font-semibold text-zinc-400">• {}</span>"#,
                    escape_html(iss.title_en.trim())
                )
            } else if is_en && !iss.title_tr.trim().is_empty() && iss.title_tr.trim() != title_main.trim() {
                format!(
                    r#"<span class="text-xs font-semibold text-zinc-400">• {}</span>"#,
                    escape_html(iss.title_tr.trim())
                )
            } else {
                String::new()
            };
            let date_lbl = if !iss.publish_month.trim().is_empty() {
                format!("{} {}", escape_html(iss.publish_month.trim()), iss.year)
            } else {
                format!("{}", iss.year)
            };
            let event_badge = if iss.is_special_issue {
                let ev_txt = iss
                    .event_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|e| !e.is_empty())
                    .unwrap_or("ÖZEL SAYI");
                format!(
                    r#"<span class="px-2 py-0.5 rounded bg-amber-50 text-amber-700 border border-amber-200 text-[11.5px] font-black uppercase tracking-wider">★ {}</span>"#,
                    escape_html(ev_txt)
                )
            } else {
                String::new()
            };

            list_html.push_str(&format!(
                r#"<div class="flex items-center justify-between gap-3 p-3 bg-white border border-zinc-200 rounded-xl hover:border-zinc-300 transition-all">
                    <div class="min-w-0 space-y-1">
                        <div class="flex items-center gap-2 flex-wrap">
                            <span class="px-2 py-0.5 rounded bg-zinc-900 text-white text-xs font-black">#{num}</span>
                            {lang_badge}
                            <span class="text-xs font-bold text-zinc-900">{t_main}</span>
                            {sub_title}
                            {event_badge}
                        </div>
                        <div class="text-xs font-semibold text-zinc-500">Yayın Dönemi: {date_lbl}</div>
                    </div>
                    {del_issue_btn}
                </div>"#,
                del_issue_btn = if s.caps().can_delete_issues {
                    format!(
                        r#"<button type="button" data-action="delete-magazine-issue" data-id="{}" class="w-8 h-8 flex items-center justify-center rounded-lg text-zinc-400 hover:text-red-500 hover:bg-red-50 transition-colors shrink-0" title="Dergi Sayısını Sil">{}</button>"#,
                        iss.id,
                        icons::trash2("w-3.5 h-3.5"),
                    )
                } else {
                    String::new()
                },
                num = iss.issue_number,
                t_main = escape_html(title_main),
            ));
        }
        list_html.push_str("</div>");
    }

    let save_dis = if s.is_saving_issue || !s.caps().can_manage_issues { "disabled" } else { "" };
    let issue_write_dis = if s.caps().can_manage_issues { "" } else { "disabled title=\"Yetki gerekli: issues:write\"" };
    let event_input_html = if s.new_issue_special {
        format!(
            r#"<input data-input="new-issue-event" type="text" autocomplete="off" placeholder="Fuar / Özel Etkinlik Adı (Örn: IDEF 2027 Özel Sayısı, SAHA EXPO)" value="{ev}" class="w-full h-9 bg-white border border-amber-300 rounded-xl px-3.5 text-xs font-bold text-zinc-800 focus:outline-none focus:border-brand-500" />"#,
            ev = escape_html(&s.new_issue_event),
        )
    } else {
        String::new()
    };

    format!(
        r#"<div class="space-y-5">
            <div class="p-4 rounded-2xl bg-zinc-50/80 border border-zinc-200/70 space-y-3">
                <div class="flex items-center justify-between">
                    <h2 class="text-xs font-black uppercase tracking-widest text-zinc-600 flex items-center gap-1.5">
                        {book_ico} Yeni MSI Dergisi / MSI TDR Sayısı Tanımla
                    </h2>
                    <span class="text-xs font-bold uppercase tracking-widest text-zinc-400">4. Adımda Seçilebilir</span>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-4 gap-2.5">
                    <input data-input="new-issue-number" type="number" min="1" placeholder="Sayı No (Örn: 286)" value="{num}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-bold text-zinc-800 focus:outline-none focus:border-brand-500" />
                    <div class="relative">
                        <select data-change="new-issue-lang" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3 pr-8 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500 appearance-none">
                            <option value="tr" {tr_sel}>🇹🇷 MSI Dergisi (TR)</option>
                            <option value="en" {en_sel}>🇬🇧 MSI TDR (EN)</option>
                        </select>
                        <div class="absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                    </div>
                    <input data-input="new-issue-month" type="text" autocomplete="off" placeholder="Ay (Örn: Kasım / Nov)" value="{mon}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-semibold text-zinc-700 focus:outline-none focus:border-brand-500" />
                    <input data-input="new-issue-year" type="number" min="2000" max="2100" placeholder="Yıl (2026)" value="{yr}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-bold text-zinc-800 focus:outline-none focus:border-brand-500" />
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                    <input data-input="new-issue-title-tr" type="text" autocomplete="off" placeholder="Türkçe Başlık (Örn: MSI Dergisi Sayı 286)" value="{t_tr}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-bold text-zinc-800 focus:outline-none focus:border-brand-500" />
                    <input data-input="new-issue-title-en" type="text" autocomplete="off" placeholder="İngilizce Başlık (Opsiyonel — Örn: MSI TDR Issue 153)" value="{t_en}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-semibold text-zinc-700 focus:outline-none focus:border-brand-500" />
                </div>
                {event_input_html}
                <div class="flex items-center justify-between gap-3 flex-wrap pt-1">
                    <button type="button" data-action="toggle-new-issue-special" class="px-3.5 min-h-[36px] rounded-[10px] border text-xs font-semibold flex items-center gap-2 transition-all {spec_cls}">
                        {spec_chk}
                        <span>Özel Sayı / Fuar Edisyonu (IDEF, SAHA EXPO vb.)</span>
                    </button>
                    <button type="button" data-action="create-magazine-issue" {save_dis} class="px-5 min-h-[38px] bg-brand-500 hover:bg-brand-600 text-white rounded-[10px] text-[13px] font-semibold flex items-center gap-2 shadow-sm transition-all disabled:opacity-50" {issue_write_dis}>
                        {plus_ico}
                        <span>Dergi Sayısı Ekle</span>
                    </button>
                </div>
            </div>

            <div class="space-y-2.5">
                <div class="flex items-center justify-between px-1">
                    <h3 class="text-xs font-black uppercase tracking-widest text-zinc-400">Tanımlı Dergi Sayıları ({cnt})</h3>
                </div>
                {list_html}
            </div>
        </div>"#,
        book_ico = icons::book_open("w-3.5 h-3.5 text-brand-500"),
        chev_ico = icons::chevron_down("w-3.5 h-3.5 text-zinc-400"),
        plus_ico = icons::plus("w-3.5 h-3.5"),
        num = escape_html(&s.new_issue_number),
        mon = escape_html(&s.new_issue_month),
        yr = escape_html(&s.new_issue_year),
        t_tr = escape_html(&s.new_issue_title_tr),
        t_en = escape_html(&s.new_issue_title_en),
        cnt = s.magazine_issues.len(),
    )
}

pub fn render_settings_glossary_list_html(s: &AppState) -> String {
    if s.glossary_rules.is_empty() {
        return format!(
            r#"<div class="py-8 text-center bg-zinc-50/80 border border-dashed border-zinc-200 rounded-2xl space-y-2">
                <div class="w-10 h-10 rounded-full bg-white border border-zinc-200 flex items-center justify-center mx-auto text-zinc-400">
                    {lang_ico}
                </div>
                <div class="text-[11px] font-black uppercase tracking-widest text-zinc-500">Henüz Terminoloji Kuralı Yok</div>
                <p class="text-xs font-medium text-zinc-400">Yukarıdan TSK, SSB, NATO ve İngiliz İngilizcesi (British English) imla kurallarını ekleyebilirsiniz.</p>
            </div>"#,
            lang_ico = icons::languages("w-4 h-4"),
        );
    }

    let q_low = s.settings_glossary_query.trim().to_lowercase();
    let filtered: Vec<&crate::editorial::common::GlossaryRule> = s
        .glossary_rules
        .iter()
        .filter(|r| {
            if q_low.is_empty() {
                return true;
            }
            r.term_tr.to_lowercase().contains(&q_low)
                || r.preferred_en.to_lowercase().contains(&q_low)
                || r.abbr_tr
                    .as_deref()
                    .map(|a| a.to_lowercase().contains(&q_low))
                    .unwrap_or(false)
                || r.abbr_en
                    .as_deref()
                    .map(|a| a.to_lowercase().contains(&q_low))
                    .unwrap_or(false)
                || r.forbidden_variants
                    .iter()
                    .any(|f| f.to_lowercase().contains(&q_low))
        })
        .collect();

    if filtered.is_empty() {
        return r#"<div class="py-6 text-center bg-zinc-50 border border-dashed border-zinc-200 rounded-xl text-[11px] font-bold text-zinc-400">Aramanızla eşleşen terminoloji kuralı bulunamadı.</div>"#.to_string();
    }

    let mut out = String::from(r#"<div class="space-y-2">"#);
    for rule in filtered {
        let tr_abbr = rule
            .abbr_tr
            .as_deref()
            .filter(|a| !a.trim().is_empty())
            .map(|a| format!(r#" <span class="text-zinc-400 font-semibold">({})</span>"#, escape_html(a)))
            .unwrap_or_default();
        let en_abbr = rule
            .abbr_en
            .as_deref()
            .filter(|a| !a.trim().is_empty())
            .map(|a| format!(r#" <span class="text-emerald-600/80 font-semibold">({})</span>"#, escape_html(a)))
            .unwrap_or_default();
        let forbidden_html = if rule.forbidden_variants.is_empty() {
            String::new()
        } else {
            let chips: Vec<String> = rule
                .forbidden_variants
                .iter()
                .map(|fv| {
                    format!(
                        r#"<span class="px-1.5 py-0.5 rounded bg-rose-50 text-rose-700 border border-rose-200 text-xs font-bold line-through">{}</span>"#,
                        escape_html(fv)
                    )
                })
                .collect();
            format!(
                r#"<div class="flex items-center flex-wrap gap-1 pt-0.5"><span class="text-xs font-black uppercase tracking-wider text-rose-500">Yasaklı:</span>{}</div>"#,
                chips.join("")
            )
        };
        let note_html = rule
            .note
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|n| format!(r#"<div class="text-xs font-medium text-zinc-500">{}</div>"#, escape_html(n)))
            .unwrap_or_default();

        out.push_str(&format!(
            r#"<div class="flex items-start justify-between gap-2.5 p-3 bg-white border border-zinc-200 rounded-xl hover:border-zinc-300 transition-all">
                <div class="min-w-0 flex-1 space-y-1">
                    <div class="flex items-center gap-1.5 flex-wrap text-xs">
                        <span class="font-bold text-zinc-800">{t_tr}{tr_abbr}</span>
                        <span class="text-brand-500 font-black">→</span>
                        <span class="font-extrabold text-emerald-700">{p_en}{en_abbr}</span>
                        <span class="px-1.5 py-0.5 rounded bg-zinc-100 text-zinc-500 text-[11.5px] font-black uppercase tracking-wider">{cat}</span>
                    </div>
                    {forbidden_html}
                    {note_html}
                </div>
                {del_gloss_btn}
            </div>"#,
            del_gloss_btn = if s.caps().can_manage_glossary {
                format!(
                    r#"<button type="button" data-action="delete-glossary-rule" data-id="{}" class="w-7 h-7 flex items-center justify-center rounded-lg text-zinc-400 hover:text-red-500 hover:bg-red-50 transition-colors shrink-0" title="Kuralı Sil">{}</button>"#,
                    rule.id,
                    icons::trash2("w-3.5 h-3.5"),
                )
            } else {
                String::new()
            },
            t_tr = escape_html(&rule.term_tr),
            p_en = escape_html(&rule.preferred_en),
            cat = escape_html(&rule.category),
        ));
    }
    out.push_str("</div>");
    out
}

fn render_settings_glossary(s: &AppState) -> String {
    let save_dis = if s.is_saving_glossary || !s.caps().can_manage_glossary {
        "disabled"
    } else {
        ""
    };
    let cat = s.new_glossary_category.trim();
    let c_inst = if cat == "institutional" || cat.is_empty() { "selected" } else { "" };
    let c_styl = if cat == "style" { "selected" } else { "" };
    let c_tech = if cat == "technical" { "selected" } else { "" };

    let rt = s.new_glossary_rule_type.trim();
    let r_fixed = if rt == "fixed" || rt.is_empty() { "selected" } else { "" };
    let r_forb = if rt == "forbidden" { "selected" } else { "" };

    let case_cls = if s.new_glossary_case_sensitive {
        "bg-brand-50 border-brand-500 text-brand-700"
    } else {
        "bg-white border-zinc-200 text-zinc-600 hover:bg-zinc-50"
    };
    let case_chk = if s.new_glossary_case_sensitive {
        icons::check("w-3 h-3 text-brand-600")
    } else {
        String::new()
    };

    format!(
        r#"<div class="space-y-5">
            <div class="p-4 rounded-2xl bg-zinc-50/80 border border-zinc-200/70 space-y-3">
                <div class="flex items-center justify-between">
                    <h2 class="text-xs font-black uppercase tracking-widest text-zinc-600 flex items-center gap-1.5">
                        {lang_ico} Yeni MSI TDR / NATO Terminoloji & İmla Kuralı
                    </h2>
                    <span class="text-xs font-bold uppercase tracking-widest text-zinc-400">Canlı Denetçi</span>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                    <input data-input="new-glossary-term-tr" type="text" autocomplete="off" placeholder="Türkçe Terim (Örn: Milli Savunma Bakanlığı)" value="{t_tr}" class="sm:col-span-2 w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-bold text-zinc-800 focus:outline-none focus:border-brand-500" />
                    <input data-input="new-glossary-abbr-tr" type="text" autocomplete="off" placeholder="TR Kısaltma (Örn: MSB)" value="{a_tr}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-semibold text-zinc-700 focus:outline-none focus:border-brand-500" />
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                    <input data-input="new-glossary-preferred-en" type="text" autocomplete="off" placeholder="Onaylı İngilizce (Örn: Ministry of National Defence)" value="{p_en}" class="sm:col-span-2 w-full h-10 bg-white border border-emerald-300 rounded-xl px-3.5 text-xs font-bold text-zinc-900 focus:outline-none focus:border-emerald-500" />
                    <input data-input="new-glossary-abbr-en" type="text" autocomplete="off" placeholder="EN Kısaltma (Örn: MoND)" value="{a_en}" class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl px-3.5 text-xs font-semibold text-zinc-700 focus:outline-none focus:border-brand-500" />
                </div>
                <input data-input="new-glossary-forbidden" type="text" autocomplete="off" placeholder="Yasaklı / Hatalı Yazımlar — virgülle ayırın (Örn: MoD, Ministry of Defense, Turkey)" value="{forb}" class="w-full h-10 bg-white border border-rose-200 rounded-xl px-3.5 text-xs font-semibold text-zinc-800 focus:outline-none focus:border-rose-400" />
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                    <div class="relative">
                        <select data-change="new-glossary-category" class="w-full h-9 bg-white border border-zinc-200/80 rounded-xl px-3 pr-8 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500 appearance-none">
                            <option value="institutional" {c_inst}>Kurumsal (TSK / SSB / Bakanlık)</option>
                            <option value="style" {c_styl}>İngiliz İngilizcesi &amp; İmla (UK Style)</option>
                            <option value="technical" {c_tech}>Teknik &amp; Savunma Terimi</option>
                        </select>
                        <div class="absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                    </div>
                    <div class="relative">
                        <select data-change="new-glossary-rule-type" class="w-full h-9 bg-white border border-zinc-200/80 rounded-xl px-3 pr-8 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500 appearance-none">
                            <option value="fixed" {r_fixed}>Resmi Karşılık (Fixed Term)</option>
                            <option value="forbidden" {r_forb}>Yasaklı Yazım / İmla (Forbidden)</option>
                        </select>
                        <div class="absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none">{chev_ico}</div>
                    </div>
                    <input data-input="new-glossary-note" type="text" autocomplete="off" placeholder="Editör Notu (Opsiyonel)" value="{note}" class="w-full h-9 bg-white border border-zinc-200/80 rounded-xl px-3 text-[11px] font-semibold text-zinc-700 focus:outline-none focus:border-brand-500" />
                </div>
                <div class="flex items-center justify-between gap-3 flex-wrap pt-1">
                    <button type="button" data-action="toggle-new-glossary-case" class="px-3.5 min-h-[36px] rounded-[10px] border text-xs font-semibold flex items-center gap-2 transition-all {case_cls}">
                        {case_chk}
                        <span>Büyük/Küçük Harf Duyarlı Eşleşme</span>
                    </button>
                    <button type="button" data-action="create-glossary-rule" {save_dis} class="px-5 min-h-[38px] bg-brand-500 hover:bg-brand-600 text-white rounded-[10px] text-[13px] font-semibold flex items-center gap-2 shadow-sm transition-all disabled:opacity-50">
                        {plus_ico}
                        <span>Kural Ekle</span>
                    </button>
                </div>
            </div>

            <div class="space-y-2.5">
                <div class="flex items-center justify-between gap-3 flex-wrap">
                    <h3 class="text-xs font-black uppercase tracking-widest text-zinc-400">Kayıtlı Terminoloji Sözlüğü ({cnt})</h3>
                    <div class="relative w-full sm:w-64">
                        {search_ico}
                        <input id="hf-settings-glossary-search" data-input="settings-glossary-query" type="text" autocomplete="off" placeholder="Terim, kısaltma veya yasaklı ara..." value="{q}" class="w-full h-8 bg-white border border-zinc-200 rounded-lg pl-8 pr-3 text-[11px] font-bold text-zinc-700 focus:outline-none focus:border-brand-500" />
                    </div>
                </div>
                <div id="hf-settings-glossary-list-slot">
                    {list_html}
                </div>
            </div>
        </div>"#,
        lang_ico = icons::languages("w-3.5 h-3.5 text-brand-500"),
        chev_ico = icons::chevron_down("w-3.5 h-3.5 text-zinc-400"),
        plus_ico = icons::plus("w-3.5 h-3.5"),
        search_ico = icons::search("absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-zinc-400"),
        t_tr = escape_html(&s.new_glossary_term_tr),
        a_tr = escape_html(&s.new_glossary_abbr_tr),
        p_en = escape_html(&s.new_glossary_preferred_en),
        a_en = escape_html(&s.new_glossary_abbr_en),
        forb = escape_html(&s.new_glossary_forbidden),
        note = escape_html(&s.new_glossary_note),
        cnt = s.glossary_rules.len(),
        q = escape_html(&s.settings_glossary_query),
        list_html = render_settings_glossary_list_html(s),
    )
}

fn render_settings_team(s: &AppState) -> String {
    let admin_href = escape_html(&admin_path());
    let role = s
        .user
        .as_ref()
        .map(|u| u.role.as_str())
        .unwrap_or("editor");
    let uname = s
        .user
        .as_ref()
        .map(|u| u.username.as_str())
        .unwrap_or("Operatör");
    format!(
        r#"<div class="space-y-4">
            <h2 class="text-xs font-black uppercase tracking-widest text-zinc-400">MSI Ekip &amp; 17-Rol Yetki Yönetimi</h2>
            <div class="p-4 rounded-2xl bg-zinc-50 border border-zinc-200 space-y-2.5">
                <div class="text-xs font-bold text-zinc-900">Aktif Editör: {uname} <span class="px-2 py-0.5 rounded bg-brand-50 text-brand-700 text-xs uppercase">{role}</span></div>
                <p class="text-[11px] text-zinc-600 leading-relaxed">
                    Kullanıcı hesapları, 17 kanonik MSI Dergisi rolü ve ayrıntılı RBAC yetki matrisi doğrudan MSI Komuta Merkezi üzerinden merkezi olarak yönetilir.
                </p>
                <div class="pt-1">
                    <a href="{admin_href}#tab-11" class="inline-flex items-center gap-2 px-4 min-h-[38px] bg-brand-500 text-white rounded-[10px] text-[13px] font-semibold hover:bg-brand-600 transition-colors">
                        Komuta Merkezi Rol Yönetimine Git
                    </a>
                </div>
            </div>
        </div>"#,
        uname = escape_html(uname),
        role = escape_html(role),
    )
}

// ============================================================================
// Centered Media Library Modal (#hf-media-modal)
// ============================================================================

fn render_media_modal_shell(s: &AppState) -> String {
    let state_cls = if s.show_media_modal {
        "hf-popover-open"
    } else {
        "hf-popover-closed"
    };
    let wm_cls = if s.upload_with_watermark {
        "hf-wm-toggle-active"
    } else {
        "hf-wm-toggle-idle"
    };
    let count_num = s.media_total.max(s.media_assets.len());

    format!(
        r#"<div id="hf-media-backdrop" data-action="close-media-library" class="hf-modal-backdrop {state_cls}"></div>
        <div id="hf-media-modal" role="dialog" aria-modal="true" aria-label="Medya Kütüphanesi" aria-hidden="{aria_hid}" class="hf-centered-modal-wrap {state_cls}">
            <div class="hf-media-modal-card">
                <div class="hf-modal-header px-6 py-4 border-b border-zinc-100 flex items-center justify-between gap-3 shrink-0 flex-wrap">
                    <div class="flex items-center gap-3">
                        <div class="w-9 h-9 rounded-xl bg-brand-50 text-brand-500 flex items-center justify-center shrink-0">
                            {img_ico}
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <h3 class="text-sm font-bold text-zinc-900">Medya Arşivi</h3>
                                <span id="hf-media-count-badge" class="px-2.5 py-0.5 rounded-full bg-zinc-100 text-zinc-600 text-[11px] font-semibold">{count_num} Görsel</span>
                            </div>
                            <p class="text-xs font-medium text-zinc-500">Otomatik WebP optimizasyonlu merkezi görsel arşivi</p>
                        </div>
                    </div>
                    <div class="flex items-center gap-2 flex-wrap">
                        {upload_controls}
                        <button type="button" data-action="close-media-library" aria-label="Medya arşivini kapat" title="Kapat" class="w-9 h-9 flex items-center justify-center rounded-lg text-zinc-500 hover:text-zinc-900 hover:bg-zinc-100 transition-colors">
                            <span class="text-lg leading-none">&times;</span>
                        </button>
                    </div>
                </div>
                <div class="hf-modal-subheader px-6 py-3 border-b border-zinc-100 shrink-0">
                    <div class="relative">
                        {search_ico}
                        <input id="hf-media-search-input" data-input="media-search-query" type="text" value="{q}" placeholder="Dosya adı veya alt metin ile arşivde ara..." class="w-full h-10 bg-white border border-zinc-200/80 rounded-xl pl-10 pr-4 text-xs font-semibold text-zinc-800 focus:outline-none focus:border-brand-500 transition-all" />
                    </div>
                </div>
                <div id="hf-media-grid-slot" class="flex-1 overflow-y-auto p-6 custom-scrollbar">
                    {grid}
                </div>
            </div>
        </div>"#,
        aria_hid = if s.show_media_modal { "false" } else { "true" },
        img_ico = icons::images("w-4 h-4"),
        upload_controls = if s.caps().can_upload_media {
            format!(
                r#"<button id="hf-media-wm-btn" type="button" data-action="toggle-upload-watermark" class="hf-wm-toggle-btn {wm_cls}" title="Yüklenen görsellere otomatik logo filigranı bas">{shield_ico}<span>Filigran</span></button><label class="hf-media-upload-btn">{plus_ico}<span>Görsel Yükle</span><input type="file" multiple accept="image/*" data-file="media-library-upload" class="hidden" /></label>"#,
                shield_ico = icons::shield_check("w-3.5 h-3.5"),
                plus_ico = icons::plus("w-3.5 h-3.5"),
            )
        } else {
            String::new()
        },
        search_ico = icons::search("absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-400"),
        q = escape_html(&s.media_search_query),
        grid = render_media_modal_grid_body(s),
    )
}

pub fn render_media_modal_grid_body(s: &AppState) -> String {
    if s.is_loading_media && s.media_assets.is_empty() {
        return format!(
            r#"<div class="py-16 flex flex-col items-center justify-center gap-3 text-zinc-400">
                {}
                <span class="text-xs font-black uppercase tracking-widest">Medya Arşivi Yükleniyor...</span>
            </div>"#,
            icons::loader2("w-6 h-6 animate-spin text-brand-500")
        );
    }

    if s.media_assets.is_empty() {
        return format!(
            r#"<div class="py-14 text-center bg-zinc-50/80 border border-dashed border-zinc-200 rounded-2xl space-y-2">
                <div class="w-11 h-11 rounded-full bg-white border border-zinc-200 flex items-center justify-center mx-auto text-zinc-400">
                    {img_ico}
                </div>
                <div class="text-[11px] font-black uppercase tracking-widest text-zinc-500">Arşivde Görsel Bulunamadı</div>
                <p class="text-xs font-medium text-zinc-400">Üstteki "Görsel Yükle" butonuyla arşive yeni fotoğraflar ekleyebilirsiniz.</p>
            </div>"#,
            img_ico = icons::images("w-5 h-5"),
        );
    }

    let action_hint = if s.media_picker_target.starts_with("gallery:") {
        "Galeriye Ekle"
    } else {
        "Bu Görseli Seç"
    };

    let mut cards = String::new();
    for asset in &s.media_assets {
        let thumb_src = asset.thumb_url.as_deref().unwrap_or(&asset.url);
        let thumb_html = render_news_image(
            Some(thumb_src),
            &asset.original_name,
            "w-full h-full object-cover transition-transform duration-300 group-hover:scale-105",
        );
        let wm_badge = if asset.watermarked {
            r#"<span class="hf-media-wm-badge">FILIGRAN</span>"#
        } else {
            ""
        };
        let dim_label = if asset.width > 0 && asset.height > 0 {
            format!("{}×{}", asset.width, asset.height)
        } else {
            "WebP".to_string()
        };
        let size_label = format_bytes_human(asset.size_bytes);
        let disp_name = if asset.original_name.trim().is_empty() {
            &asset.filename
        } else {
            &asset.original_name
        };

        cards.push_str(&format!(
            r#"<div class="hf-media-asset-card group">
                <button type="button" data-action="select-media-asset" data-asset-id="{id}" class="hf-media-select-area" title="{action_hint}">
                    <div class="hf-media-thumb-wrap">
                        {thumb_html}
                        {wm_badge}
                        <div class="hf-media-hover-cta">
                            <span>{action_hint}</span>
                        </div>
                    </div>
                    <div class="hf-media-meta">
                        <div class="hf-media-name">{name}</div>
                        <div class="hf-media-specs">{dim_label} • {size_label}</div>
                    </div>
                </button>
                {del_media_btn}
            </div>"#,
            id = asset.id,
            name = escape_html(disp_name),
            del_media_btn = if s.caps().can_delete_media {
                format!(
                    r#"<button type="button" data-action="delete-media-asset" data-asset-id="{}" class="hf-media-del-btn" title="Arşivden Sil">{}</button>"#,
                    asset.id,
                    icons::trash2("w-3.5 h-3.5"),
                )
            } else {
                String::new()
            },
        ));
    }

    format!(r#"<div class="hf-media-grid">{cards}</div>"#)
}

// ============================================================================
// Centered 301 URL Redirects Modal (#hf-redirects-modal)
// ============================================================================

fn render_redirects_modal_shell(s: &AppState) -> String {
    let state_cls = if s.show_redirects_modal {
        "hf-popover-open"
    } else {
        "hf-popover-closed"
    };
    format!(
        r#"<div id="hf-redirects-backdrop" data-action="close-redirects-modal" class="hf-modal-backdrop {state_cls}"></div>
        <div id="hf-redirects-modal" role="dialog" aria-modal="true" aria-label="301 Yönlendirmeleri" aria-hidden="{aria_hid}" class="hf-centered-modal-wrap {state_cls}">
            <div class="hf-compact-modal-card">
                <div class="hf-modal-header px-6 py-4 border-b border-zinc-100 flex items-center justify-between gap-3 shrink-0">
                    <div class="flex items-center gap-3">
                        <div class="w-9 h-9 rounded-xl bg-brand-50 text-brand-500 flex items-center justify-center shrink-0">
                            {redir_ico}
                        </div>
                        <div>
                            <h3 class="text-sm font-bold text-zinc-900">Aktif 301 URL Yönlendirmeleri</h3>
                            <p class="text-xs font-medium text-zinc-500">Eski haber bağlantılarından yeni kalıcı adreslere otomatik SEO koruması</p>
                        </div>
                    </div>
                    <button type="button" data-action="close-redirects-modal" aria-label="Yönlendirmeleri kapat" title="Kapat" class="w-9 h-9 flex items-center justify-center rounded-lg text-zinc-500 hover:text-zinc-900 hover:bg-zinc-100 transition-colors">
                        <span class="text-lg leading-none">&times;</span>
                    </button>
                </div>
                <div id="hf-redirects-body-slot" class="flex-1 overflow-y-auto p-6 custom-scrollbar">
                    {body}
                </div>
            </div>
        </div>"#,
        aria_hid = if s.show_redirects_modal { "false" } else { "true" },
        redir_ico = icons::arrow_right_left("w-4 h-4"),
        body = render_redirects_modal_body(s),
    )
}

pub fn render_redirects_modal_body(s: &AppState) -> String {
    if s.is_loading_redirects && s.redirects.is_empty() {
        return format!(
            r#"<div class="py-12 flex flex-col items-center justify-center gap-2 text-zinc-400">
                {}
                <span class="text-xs font-black uppercase tracking-widest">Yönlendirmeler Yükleniyor...</span>
            </div>"#,
            icons::loader2("w-5 h-5 animate-spin text-brand-500")
        );
    }

    if s.redirects.is_empty() {
        return format!(
            r#"<div class="py-10 text-center bg-zinc-50/80 border border-dashed border-zinc-200 rounded-2xl space-y-2">
                <div class="w-10 h-10 rounded-full bg-white border border-zinc-200 flex items-center justify-center mx-auto text-zinc-400">
                    {ico}
                </div>
                <div class="text-[11px] font-black uppercase tracking-widest text-zinc-500">Tanımlı 301 Yönlendirmesi Yok</div>
                <p class="text-xs font-medium text-zinc-400">Yayınlanmış bir haberin kısa adını (slug) değiştirdiğinizde otomatik 301 kuralı burada listelenir.</p>
            </div>"#,
            ico = icons::arrow_right_left("w-4 h-4"),
        );
    }

    let mut rows = String::new();
    for r in &s.redirects {
        rows.push_str(&format!(
            r#"<div class="hf-redirect-row">
                <div class="min-w-0 flex-1 space-y-1">
                    <div class="flex items-center gap-2 flex-wrap text-xs font-bold">
                        <span class="text-zinc-500 line-through font-mono truncate">/icerik/{old_s}</span>
                        <span class="text-brand-500 font-black">→</span>
                        <span class="text-emerald-700 font-mono truncate">/icerik/{new_s}</span>
                    </div>
                    <div class="flex items-center gap-2 text-xs font-bold text-zinc-400">
                        <span class="px-1.5 py-0.5 rounded bg-emerald-50 text-emerald-700 font-black">{code}</span>
                        <span>{hits} yönlendirme</span>
                    </div>
                </div>
                {del_redir_btn}
            </div>"#,
            old_s = escape_html(&r.old_slug),
            new_s = escape_html(&r.new_slug),
            code = if r.status_code > 0 { r.status_code } else { 301 },
            hits = r.hits,
            del_redir_btn = if s.caps().can_manage_redirects {
                format!(
                    r#"<button type="button" data-action="delete-redirect" data-redirect-id="{}" class="w-8 h-8 flex items-center justify-center rounded-lg text-zinc-400 hover:text-red-500 hover:bg-red-50 transition-colors shrink-0" title="Kuralı Sil">{}</button>"#,
                    r.id,
                    icons::trash2("w-3.5 h-3.5"),
                )
            } else {
                String::new()
            },
        ));
    }

    format!(r#"<div class="space-y-2">{rows}</div>"#)
}

// ============================================================================
// Centered Revision History Modal (#hf-revisions-modal)
// ============================================================================

fn render_revisions_modal_shell(s: &AppState) -> String {
    let state_cls = if s.show_revisions_modal {
        "hf-popover-open"
    } else {
        "hf-popover-closed"
    };
    let subtitle = if s.revisions_article_title.trim().is_empty() {
        "Kaydedilen tüm haber sürümleri ve tek tıkla geri yükleme".to_string()
    } else {
        s.revisions_article_title.clone()
    };

    format!(
        r#"<div id="hf-revisions-backdrop" data-action="close-revisions-modal" class="hf-modal-backdrop {state_cls}"></div>
        <div id="hf-revisions-modal" role="dialog" aria-modal="true" aria-label="Revizyon Geçmişi" aria-hidden="{aria_hid}" class="hf-centered-modal-wrap {state_cls}">
            <div class="hf-compact-modal-card">
                <div class="hf-modal-header px-6 py-4 border-b border-zinc-100 flex items-center justify-between gap-3 shrink-0">
                    <div class="flex items-center gap-3 min-w-0">
                        <div class="w-9 h-9 rounded-xl bg-brand-50 text-brand-500 flex items-center justify-center shrink-0">
                            {hist_ico}
                        </div>
                        <div class="min-w-0">
                            <h3 class="text-sm font-bold text-zinc-900">Revizyon Geçmişi</h3>
                            <p id="hf-revisions-subtitle" class="text-xs font-medium text-zinc-500 truncate">{sub}</p>
                        </div>
                    </div>
                    <button type="button" data-action="close-revisions-modal" aria-label="Revizyon geçmişini kapat" title="Kapat" class="w-9 h-9 flex items-center justify-center rounded-lg text-zinc-500 hover:text-zinc-900 hover:bg-zinc-100 transition-colors shrink-0">
                        <span class="text-lg leading-none">&times;</span>
                    </button>
                </div>
                <div id="hf-revisions-body-slot" class="flex-1 overflow-y-auto p-6 custom-scrollbar">
                    {body}
                </div>
            </div>
        </div>"#,
        aria_hid = if s.show_revisions_modal { "false" } else { "true" },
        hist_ico = icons::history("w-4 h-4"),
        sub = escape_html(&subtitle),
        body = render_revisions_modal_body(s),
    )
}

pub fn render_revisions_modal_body(s: &AppState) -> String {
    if s.is_loading_revisions && s.revisions.is_empty() {
        return format!(
            r#"<div class="py-12 flex flex-col items-center justify-center gap-2 text-zinc-400">
                {}
                <span class="text-xs font-black uppercase tracking-widest">Sürüm Geçmişi Yükleniyor...</span>
            </div>"#,
            icons::loader2("w-5 h-5 animate-spin text-brand-500")
        );
    }

    if s.revisions.is_empty() {
        return format!(
            r#"<div class="py-10 text-center bg-zinc-50/80 border border-dashed border-zinc-200 rounded-2xl space-y-2">
                <div class="w-10 h-10 rounded-full bg-white border border-zinc-200 flex items-center justify-center mx-auto text-zinc-400">
                    {ico}
                </div>
                <div class="text-[11px] font-black uppercase tracking-widest text-zinc-500">Henüz Kayıtlı Revizyon Yok</div>
                <p class="text-xs font-medium text-zinc-400">Bu haber her güncellendiğinde veya yayınlandığında otomatik sürüm yedeği oluşturulur.</p>
            </div>"#,
            ico = icons::history("w-4 h-4"),
        );
    }

    let restore_dis = if s.is_restoring_revision {
        "disabled"
    } else {
        ""
    };
    let mut rows = String::new();
    for rev in &s.revisions {
        let dt_str = format_middle_turkish_date(rev.created_at.as_deref())
            .or_else(|| rev.created_at.clone())
            .unwrap_or_else(|| "Kayıtlı Sürüm".to_string());
        let note_badge = rev
            .note
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|n| {
                format!(
                    r#"<span class="px-2 py-0.5 rounded-md bg-brand-50 text-brand-600 text-xs font-black uppercase tracking-wider">{}</span>"#,
                    escape_html(n)
                )
            })
            .unwrap_or_default();
        let slug_html = rev
            .slug
            .as_deref()
            .map(str::trim)
            .filter(|sl| !sl.is_empty())
            .map(|sl| {
                format!(
                    r#"<div class="text-xs font-mono font-semibold text-zinc-400 truncate">/icerik/{}</div>"#,
                    escape_html(sl)
                )
            })
            .unwrap_or_default();

        rows.push_str(&format!(
            r#"<div class="hf-revision-row">
                <div class="min-w-0 flex-1 space-y-1">
                    <div class="flex items-center gap-2 flex-wrap">
                        <span class="px-1.5 py-0.5 rounded bg-zinc-900 text-white text-xs font-black">#{rid}</span>
                        <span class="text-xs font-extrabold text-zinc-700 uppercase tracking-wider">{editor}</span>
                        <span class="text-xs font-semibold text-zinc-400">• {dt}</span>
                        {note_badge}
                    </div>
                    <div class="text-xs font-bold text-zinc-900 leading-snug">{title}</div>
                    {slug_html}
                </div>
                <button type="button" data-action="restore-revision" data-article-id="{aid}" data-rev-id="{rid}" {restore_dis} class="hf-restore-rev-btn">
                    {rev_ico}
                    <span>Bu Sürüme Geri Dön</span>
                </button>
            </div>"#,
            rid = rev.id,
            aid = escape_html(&rev.article_id),
            editor = escape_html(&rev.editor_name),
            dt = escape_html(&dt_str),
            title = escape_html(&rev.title),
            rev_ico = icons::refresh_ccw("w-3 h-3"),
        ));
    }

    format!(r#"<div class="space-y-2.5">{rows}</div>"#)
}

fn render_preview_modal_shell(s: &AppState) -> String {
    let state_cls = if s.show_preview_modal {
        "hf-popover-open"
    } else {
        "hf-popover-closed"
    };
    let desk_cls = if s.preview_device == PreviewDevice::Desktop {
        "bg-white text-zinc-900 shadow-sm"
    } else {
        "text-zinc-500 hover:text-zinc-900"
    };
    let mob_cls = if s.preview_device == PreviewDevice::Mobile {
        "bg-white text-zinc-900 shadow-sm"
    } else {
        "text-zinc-500 hover:text-zinc-900"
    };
    let canon = s.canonical_editing_article();
    let pub_dis = if s.is_posting || canon.title.trim().is_empty() {
        "disabled"
    } else {
        ""
    };

    format!(
        r#"<div id="hf-preview-backdrop" data-action="close-preview-modal" class="hf-preview-backdrop {state_cls}"></div>
        <div id="hf-preview-modal" role="dialog" aria-modal="true" aria-label="Haber Önizlemesi" aria-hidden="{aria_hid}" class="hf-preview-modal {state_cls}">
            <div class="hf-preview-modal-card">
                <div class="hf-modal-header px-6 py-4 border-b border-zinc-200/80 flex items-center justify-between gap-3 shrink-0 flex-wrap">
                    <div class="flex items-center gap-3">
                        <div class="w-9 h-9 rounded-xl bg-brand-50 text-brand-500 flex items-center justify-center shrink-0">
                            {eye_ico}
                        </div>
                        <div>
                            <h3 class="text-sm font-bold text-zinc-900">Yayın Öncesi Tam Sayfa Önizleme</h3>
                            <p class="text-xs font-medium text-zinc-500">Haberin sitede okuyucuya görüneceği gerçek sayfa düzeni</p>
                        </div>
                    </div>

                    <div class="flex items-center gap-1 bg-zinc-100 p-1 rounded-xl border border-zinc-200/70">
                        <button id="btn-preview-desk" type="button" data-action="set-preview-device" data-device="desktop" class="px-3.5 min-h-[36px] rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-all {desk_cls}">
                            {desk_ico}
                            <span>Masaüstü</span>
                        </button>
                        <button id="btn-preview-mob" type="button" data-action="set-preview-device" data-device="mobile" class="px-3.5 min-h-[36px] rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-all {mob_cls}">
                            {mob_ico}
                            <span>Mobil (Telefon)</span>
                        </button>
                    </div>

                    <div class="flex items-center gap-2">
                        <button id="btn-preview-publish-now" type="button" data-action="publish-from-preview" {pub_dis} class="px-4 min-h-[38px] bg-brand-500 hover:bg-brand-600 text-white rounded-[10px] text-[13px] font-semibold flex items-center gap-2 shadow-sm transition-all disabled:opacity-50">
                            {send_ico}
                            <span>Hemen Yayınla</span>
                        </button>
                        <button type="button" data-action="close-preview-modal" title="Önizlemeyi Kapat" class="px-3.5 min-h-[38px] flex items-center gap-1.5 rounded-[10px] border border-zinc-200 text-zinc-700 hover:text-zinc-900 hover:bg-zinc-100 text-[13px] font-semibold transition-colors">
                            <span>Kapat</span>
                            <span class="text-base leading-none">&times;</span>
                        </button>
                    </div>
                </div>

                <div id="hf-preview-body-slot" class="flex-1 overflow-y-auto custom-scrollbar hf-preview-stage">
                    {body}
                </div>
            </div>
        </div>"#,
        aria_hid = if s.show_preview_modal { "false" } else { "true" },
        eye_ico = icons::eye("w-4 h-4"),
        desk_ico = icons::monitor("w-3.5 h-3.5"),
        mob_ico = icons::smartphone("w-3.5 h-3.5"),
        send_ico = icons::send("w-3.5 h-3.5"),
        body = render_preview_modal_body(s),
    )
}

pub fn render_preview_modal_body(s: &AppState) -> String {
    let is_mob = s.preview_device == PreviewDevice::Mobile;
    let sheet_html = render_draft_article_sheet_html(s, is_mob);
    if is_mob {
        format!(
            r#"<div class="py-6 px-3 flex justify-center">
                <div class="hf-mobile-phone-frame">
                    <div class="hf-mobile-phone-notch"></div>
                    <div class="bg-white rounded-[1.8rem] overflow-hidden">
                        {sheet_html}
                    </div>
                </div>
            </div>"#
        )
    } else {
        format!(
            r#"<div class="py-6 sm:py-8 px-3 sm:px-6">
                <div class="hf-desktop-article-frame">
                    {sheet_html}
                </div>
            </div>"#
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmTone {
    Danger,
    Warning,
    Info,
}

fn render_editor_confirm_modal_shell(s: &AppState) -> String {
    let is_open = s.pending_editor_confirm.is_some();
    let state_cls = if is_open {
        "hf-popover-open"
    } else {
        "hf-popover-closed"
    };
    format!(
        r#"<div id="hf-editor-confirm-backdrop" data-action="cancel-editor-confirm" class="hf-confirm-backdrop {state_cls}"></div>
        <div id="hf-editor-confirm-modal" role="dialog" aria-modal="true" aria-labelledby="hf-confirm-modal-title" aria-hidden="{aria_hid}" class="hf-confirm-modal-wrap {state_cls}">
            <div id="hf-editor-confirm-body-slot" class="hf-confirm-modal-card">
                {body}
            </div>
        </div>"#,
        aria_hid = if is_open { "false" } else { "true" },
        body = render_editor_confirm_modal_body(s),
    )
}

pub fn render_editor_confirm_modal_body(s: &AppState) -> String {
    let kind = s
        .pending_editor_confirm
        .clone()
        .unwrap_or(PendingEditorConfirm::ClearEditor);

    let (tone, badge_cls, badge_lbl, title, desc, summary_pill, save_row_html, confirm_lbl, confirm_cls) =
        match kind {
            PendingEditorConfirm::NewArticle
            | PendingEditorConfirm::ClearEditor
            | PendingEditorConfirm::MoveSelectedToEditor => {
                let cur_id = s.editing_article.id.trim();
                let is_saved_draft = cur_id.starts_with("draft-")
                    || s.editing_article.article_type.as_deref() == Some("draft");
                let is_published_art = !is_saved_draft
                    && (cur_id.starts_with("pub-")
                        || s.editing_article.article_type.as_deref() == Some("published"));

                let action_suffix = match kind {
                    PendingEditorConfirm::NewArticle => "ve Yeni Aç",
                    PendingEditorConfirm::ClearEditor => "ve Kapat",
                    PendingEditorConfirm::MoveSelectedToEditor => "ve Aktar",
                    _ => "",
                };

                let (tone, b_cls, b_lbl, ttl, dsc, save_lbl_opt, disc_lbl, disc_btn_cls) =
                    if is_saved_draft {
                        (
                            ConfirmTone::Warning,
                            "hf-confirm-mode-badge hf-confirm-mode-draft",
                            "KAYITLI TASLAK",
                            "Taslakta Kaydedilmemiş Değişiklikler Var".to_string(),
                            "Taslağın kendisi silinmez, Taslaklar listenizde durmaya devam eder. Yaptığınız son düzenlemeler için ne yapılsın?".to_string(),
                            Some(format!("Değişiklikleri Taslağa Kaydet {action_suffix}")),
                            "Değişiklikleri Kaydetme".to_string(),
                            "hf-confirm-btn hf-confirm-btn-discard-soft",
                        )
                    } else if is_published_art {
                        (
                            ConfirmTone::Warning,
                            "hf-confirm-mode-badge hf-confirm-mode-pub",
                            "YAYINDAKİ HABER",
                            "Yayındaki Haberde Kaydedilmemiş Değişiklikler Var".to_string(),
                            "Yayındaki haberiniz silinmez; yalnızca sağ editörde yaptığınız kaydedilmemiş düzenlemeler iptal edilir.".to_string(),
                            None,
                            "Değişiklikleri İptal Et".to_string(),
                            "hf-confirm-btn hf-confirm-btn-discard-soft",
                        )
                    } else {
                        let (new_title, new_discard) = match kind {
                            PendingEditorConfirm::NewArticle => {
                                ("Yeni Habere Geçilsin mi?", "Kaydetmeden Yeni Aç")
                            }
                            PendingEditorConfirm::ClearEditor => {
                                ("Editör İçeriği Temizlensin mi?", "Kaydetmeden Temizle")
                            }
                            _ => ("Editör İçeriği Değiştirilsin mi?", "Kaydetmeden Aktar"),
                        };
                        (
                            ConfirmTone::Danger,
                            "hf-confirm-mode-badge hf-confirm-mode-new",
                            "YENİ İÇERİK",
                            new_title.to_string(),
                            "Bu haber henüz Taslaklar'a kaydedilmedi. Şimdi çıkarsanız yazdığınız içerik sıfırlanacaktır.".to_string(),
                            Some(format!("Taslak Olarak Kaydet {action_suffix}")),
                            new_discard.to_string(),
                            "hf-confirm-btn hf-confirm-btn-danger",
                        )
                    };

                let canon = s.canonical_editing_article();
                let cur_title = canon.title.trim();
                let filled_blocks = canon
                    .blocks
                    .as_ref()
                    .map(|blks| {
                        blks.iter()
                            .filter(|b| {
                                !crate::editorial::common::strip_all_html_tags(&b.content)
                                    .trim()
                                    .is_empty()
                                    || ((b.block_type == "image" || b.block_type == "embed")
                                        && !b.content.trim().is_empty())
                            })
                            .count()
                    })
                    .unwrap_or(0);

                let summary = if !cur_title.is_empty() {
                    format!(
                        r#"<div class="hf-confirm-summary-pill">Başlık: <strong>{}</strong> • {filled_blocks} dolu blok</div>"#,
                        escape_html(cur_title)
                    )
                } else {
                    format!(
                        r#"<div class="hf-confirm-summary-pill">Editörde <strong>{filled_blocks} dolu içerik bloğu</strong> bulunuyor.</div>"#
                    )
                };

                let save_row = match save_lbl_opt {
                    Some(lbl) => format!(
                        r#"<button type="button" data-action="save-draft-and-confirm-editor" class="hf-confirm-btn hf-confirm-btn-save w-full">
                            {save_ico}
                            <span>{lbl}</span>
                        </button>"#,
                        save_ico = icons::save("w-4 h-4 shrink-0"),
                        lbl = escape_html(&lbl),
                    ),
                    None => String::new(),
                };

                (tone, b_cls, b_lbl, ttl, dsc, summary, save_row, disc_lbl, disc_btn_cls)
            }
            PendingEditorConfirm::DeleteArticle { title, is_draft, .. } => {
                let ttl = if is_draft {
                    "Taslağı Kalıcı Olarak Sil?"
                } else {
                    "Haberi Kalıcı Olarak Sil?"
                };
                let pill = format!(
                    r#"<div class="hf-confirm-summary-pill">İçerik: <strong>{}</strong></div>"#,
                    escape_html(&title)
                );
                (
                    ConfirmTone::Danger,
                    "hf-confirm-mode-badge hf-confirm-mode-new",
                    "KALICI SİLME",
                    ttl.to_string(),
                    "Bu işlem geri alınamaz. Seçili içerik veritabanından ve yayından kalıcı olarak silinecektir.".to_string(),
                    pill,
                    String::new(),
                    "Evet, Kalıcı Sil".to_string(),
                    "hf-confirm-btn hf-confirm-btn-danger",
                )
            }
            PendingEditorConfirm::DeleteCategory { name, .. } => (
                ConfirmTone::Danger,
                "hf-confirm-mode-badge hf-confirm-mode-new",
                "KATEGORİ SİLME",
                "Kategoriyi Sil?".to_string(),
                "Bu kategori kalıcı olarak silinecektir. Alt kategoriler varsa bir üst kategoriye aktarılır.".to_string(),
                format!(
                    r#"<div class="hf-confirm-summary-pill">Kategori: <strong>{}</strong></div>"#,
                    escape_html(&name)
                ),
                String::new(),
                "Kategoriyi Sil".to_string(),
                "hf-confirm-btn hf-confirm-btn-danger",
            ),
            PendingEditorConfirm::DeleteResearchSource { name, .. } => (
                ConfirmTone::Danger,
                "hf-confirm-mode-badge hf-confirm-mode-new",
                "KAYNAK SİLME",
                "Haber Kaynağını Sil?".to_string(),
                "Bu haber kaynağı araştırma masasından kalıcı olarak kaldırılacaktır.".to_string(),
                format!(
                    r#"<div class="hf-confirm-summary-pill">Kaynak: <strong>{}</strong></div>"#,
                    escape_html(&name)
                ),
                String::new(),
                "Kaynağı Sil".to_string(),
                "hf-confirm-btn hf-confirm-btn-danger",
            ),
            PendingEditorConfirm::DeleteMediaAsset { label, .. } => (
                ConfirmTone::Danger,
                "hf-confirm-mode-badge hf-confirm-mode-new",
                "MEDYA SİLME",
                "Görseli Arşivden Sil?".to_string(),
                "Bu görsel medya arşivinden kalıcı olarak silinecektir. Bu işlem geri alınamaz.".to_string(),
                format!(
                    r#"<div class="hf-confirm-summary-pill">Görsel: <strong>{}</strong></div>"#,
                    escape_html(&label)
                ),
                String::new(),
                "Görseli Kalıcı Sil".to_string(),
                "hf-confirm-btn hf-confirm-btn-danger",
            ),
            PendingEditorConfirm::RestoreArticleRevision {
                rev_id, rev_title, ..
            } => (
                ConfirmTone::Warning,
                "hf-confirm-mode-badge hf-confirm-mode-draft",
                "REVİZYON GERİ YÜKLEME",
                "Seçilen Sürüme Geri Dön?".to_string(),
                "Mevcut haber içeriği ve başlığı seçilen revizyon sürümüyle değiştirilecektir.".to_string(),
                format!(
                    r#"<div class="hf-confirm-summary-pill">Sürüm #{rev_id}: <strong>{}</strong></div>"#,
                    escape_html(&rev_title)
                ),
                String::new(),
                "Bu Sürüme Geri Dön".to_string(),
                "hf-confirm-btn hf-confirm-btn-warning",
            ),
            PendingEditorConfirm::DeleteMagazineIssue { label, .. } => (
                ConfirmTone::Danger,
                "hf-confirm-mode-badge hf-confirm-mode-new",
                "DERGİ SAYISI SİLME",
                "Dergi Sayısını Sil?".to_string(),
                "Bu dergi sayısı katalog listesinden kalıcı olarak silinecektir.".to_string(),
                format!(
                    r#"<div class="hf-confirm-summary-pill">Sayı: <strong>{}</strong></div>"#,
                    escape_html(&label)
                ),
                String::new(),
                "Dergi Sayısını Sil".to_string(),
                "hf-confirm-btn hf-confirm-btn-danger",
            ),
            PendingEditorConfirm::DeleteGlossaryRule { label, .. } => (
                ConfirmTone::Danger,
                "hf-confirm-mode-badge hf-confirm-mode-new",
                "TERMİNOLOJİ SİLME",
                "Terminoloji Kuralını Sil?".to_string(),
                "Bu TSK/SSB/NATO terminoloji kuralı sözlükten kalıcı olarak kaldırılacaktır.".to_string(),
                format!(
                    r#"<div class="hf-confirm-summary-pill">Kural: <strong>{}</strong></div>"#,
                    escape_html(&label)
                ),
                String::new(),
                "Kuralı Sil".to_string(),
                "hf-confirm-btn hf-confirm-btn-danger",
            ),
        };

    let (icon_box_cls, icon_html) = match tone {
        ConfirmTone::Danger => ("hf-confirm-icon-box hf-confirm-tone-danger", icons::alert_circle("w-5 h-5")),
        ConfirmTone::Warning => ("hf-confirm-icon-box hf-confirm-tone-warning", icons::alert_circle("w-5 h-5")),
        ConfirmTone::Info => ("hf-confirm-icon-box hf-confirm-tone-info", icons::info("w-5 h-5")),
    };

    format!(
        r#"<div class="hf-confirm-body">
            <div class="hf-confirm-head">
                <div class="{icon_box_cls}">
                    {icon_html}
                </div>
                <div class="space-y-1 min-w-0 flex-1">
                    <span class="{badge_cls}">{badge_lbl}</span>
                    <h3 id="hf-confirm-modal-title" class="hf-confirm-title">{title}</h3>
                    <p class="hf-confirm-desc">{desc}</p>
                </div>
            </div>
            {summary_pill}
            <div class="hf-confirm-actions-stack">
                {save_row_html}
                <div class="hf-confirm-secondary-grid">
                    <button type="button" data-action="cancel-editor-confirm" class="hf-confirm-btn hf-confirm-btn-cancel">
                        Vazgeç
                    </button>
                    <button type="button" data-action="confirm-editor-discard" class="{confirm_cls}">
                        <span>{confirm_lbl}</span>
                    </button>
                </div>
            </div>
        </div>"#,
        title = escape_html(&title),
        desc = escape_html(&desc),
        confirm_lbl = escape_html(&confirm_lbl),
    )
}

pub fn render_single_toast_html(item: &ToastItem, animate_enter: bool) -> String {
    let (tone_cls, role_attr, title_txt, icon_html) = match item.kind {
        ToastKind::Success => (
            "hf-toast-success",
            "status",
            "Başarılı",
            icons::check_circle2("w-4 h-4"),
        ),
        ToastKind::Error => (
            "hf-toast-error",
            "alert",
            "Hata",
            icons::alert_circle("w-4 h-4"),
        ),
        ToastKind::Warning => (
            "hf-toast-warning",
            "status",
            "Uyarı",
            icons::alert_circle("w-4 h-4"),
        ),
        ToastKind::Info => (
            "hf-toast-info",
            "status",
            "Bilgi",
            icons::info("w-4 h-4"),
        ),
    };
    let enter_cls = if animate_enter { " hf-toast-enter" } else { "" };
    format!(
        r#"<div class="hf-toast {tone_cls}{enter_cls}" data-toast-id="{id}" role="{role_attr}">
            <div class="hf-toast-icon">{icon_html}</div>
            <div class="hf-toast-body">
                <div class="hf-toast-title">{title_txt}</div>
                <p class="hf-toast-msg">{msg}</p>
            </div>
            <button type="button" data-action="dismiss-toast" data-toast-id="{id}" class="hf-toast-close" aria-label="Bildirimi kapat" title="Bildirimi kapat">
                {x_ico}
            </button>
        </div>"#,
        id = item.id,
        msg = escape_html(&item.message),
        x_ico = icons::x_mark("w-3.5 h-3.5"),
    )
}

pub fn render_toasts_inner(s: &AppState) -> String {
    if s.toasts.is_empty() {
        return String::new();
    }
    let mut items_html = String::new();
    for item in &s.toasts {
        items_html.push_str(&render_single_toast_html(item, item.id == s.toast_seq));
    }
    format!(
        r#"<div id="hf-toast-region" class="hf-toast-region" aria-live="polite" aria-relevant="additions removals">{items_html}</div>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_non_blocking_confirm_modal_for_all_seven_destructive_and_restore_variants() {
        let mut s = AppState::new();
        let cases = vec![
            (
                PendingEditorConfirm::DeleteArticle {
                    article_id: "pub-7".into(),
                    title: "KAAN Test Uçuşu".into(),
                    is_draft: false,
                },
                "Haberi Kalıcı Olarak Sil?",
                "KAAN Test Uçuşu",
                "hf-confirm-tone-danger",
            ),
            (
                PendingEditorConfirm::DeleteCategory {
                    id: 4,
                    name: "Deniz Sistemleri".into(),
                },
                "Kategoriyi Sil?",
                "Deniz Sistemleri",
                "hf-confirm-tone-danger",
            ),
            (
                PendingEditorConfirm::DeleteResearchSource {
                    id: 9,
                    name: "TRT Haber".into(),
                },
                "Haber Kaynağını Sil?",
                "TRT Haber",
                "hf-confirm-tone-danger",
            ),
            (
                PendingEditorConfirm::DeleteMediaAsset {
                    asset_id: 22,
                    label: "ankamini.webp".into(),
                },
                "Görseli Arşivden Sil?",
                "ankamini.webp",
                "hf-confirm-tone-danger",
            ),
            (
                PendingEditorConfirm::RestoreArticleRevision {
                    article_id: "pub-7".into(),
                    rev_id: 3,
                    rev_title: "Eski Başlık".into(),
                },
                "Seçilen Sürüme Geri Dön?",
                "Sürüm #3",
                "hf-confirm-tone-warning",
            ),
            (
                PendingEditorConfirm::DeleteMagazineIssue {
                    id: 14,
                    label: "#286 MSI Dergisi".into(),
                },
                "Dergi Sayısını Sil?",
                "#286 MSI Dergisi",
                "hf-confirm-tone-danger",
            ),
            (
                PendingEditorConfirm::DeleteGlossaryRule {
                    id: 19,
                    label: "MSB → MoND".into(),
                },
                "Terminoloji Kuralını Sil?",
                "MSB → MoND",
                "hf-confirm-tone-danger",
            ),
        ];

        for (variant, expected_title, expected_summary, expected_tone_cls) in cases {
            s.pending_editor_confirm = Some(variant);
            let shell = render_editor_confirm_modal_shell(&s);
            assert!(shell.contains("hf-confirm-backdrop hf-popover-open"));
            assert!(shell.contains("hf-confirm-modal-wrap hf-popover-open"));
            assert!(shell.contains("aria-labelledby=\"hf-confirm-modal-title\""));
            assert!(shell.contains(expected_title));
            assert!(shell.contains(expected_summary));
            assert!(shell.contains(expected_tone_cls));
            assert!(shell.contains("data-action=\"cancel-editor-confirm\""));
            assert!(shell.contains("Vazgeç"));
            assert!(shell.contains("data-action=\"confirm-editor-discard\""));
        }
    }

    #[test]
    fn renders_queued_bottom_right_toasts_with_accessibility_roles_and_close_buttons() {
        let mut s = AppState::new();
        assert!(render_toasts_inner(&s).is_empty());

        let id_ok = s.push_toast(ToastKind::Success, "Haber taslak olarak kaydedildi.");
        let id_err = s.push_toast(ToastKind::Error, "Sunucu bağlantısı kesildi.");
        let id_warn = s.push_toast(ToastKind::Warning, "Terminoloji kontrolü önerilir.");
        let id_info = s.push_toast(ToastKind::Info, "Taslak otomatik yedeklendi.");

        let html = render_toasts_inner(&s);
        assert!(html.contains("class=\"hf-toast-region\""));
        assert!(html.contains("hf-toast-success"));
        assert!(html.contains("hf-toast-error"));
        assert!(html.contains("hf-toast-warning"));
        assert!(html.contains("hf-toast-info"));
        assert!(html.contains("role=\"alert\""));
        assert!(html.contains("role=\"status\""));
        assert!(html.contains(&format!("data-toast-id=\"{id_ok}\"")));
        assert!(html.contains(&format!("data-toast-id=\"{id_err}\"")));
        assert!(html.contains(&format!("data-toast-id=\"{id_warn}\"")));
        assert!(html.contains(&format!("data-toast-id=\"{id_info}\"")));
        assert!(html.contains("data-action=\"dismiss-toast\""));
        assert!(html.contains("aria-label=\"Bildirimi kapat\""));
        // Only the newest toast gets hf-toast-enter on full render
        assert_eq!(html.matches("hf-toast-enter").count(), 1);
    }

    #[test]
    fn dead_end_change_password_modal_is_removed() {
        let s = AppState::new();
        let html = render_modals_inner(&s);
        assert!(!html.contains("submit-change-pw"));
        assert!(!html.contains("pw-current"));
        assert!(!html.contains("Şifre Değiştir"));
    }
}
