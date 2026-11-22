//! Global application state, pure formatting/sanitization/SEO helpers, and `localStorage` persistence.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::editorial::common::{
    apply_glossary_fixes_to_text, apply_tr_to_en_glossary_assist, blocks_to_wp_html,
    escape_html_basic, estimate_read_time_minutes, evaluate_glossary_compliance,
    parse_content_to_blocks, Article, ArticleLock, ArticleRevision, Category, ContentBlock,
    EditorialCapabilities, GalleryItem, GlossaryRule, GlossaryViolation, MagazineIssueItem,
    MediaAsset, SeoData, SsbTag, UrlRedirect, User, WordPressConfig,
};
use pulldown_cmark::{html as md_html, Options as MdOptions, Parser as MdParser};
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

pub const PLACEHOLDER_IMG: &str = "/images/logo.svg";

/// Reads `<body data-admin-path="...">`, falling back to `"/podmin"`.
pub fn admin_path() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(p) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
            .and_then(|b| b.get_attribute("data-admin-path"))
        {
            let trimmed = p.trim();
            if trimmed.starts_with('/') {
                return trimmed.to_string();
            }
        }
    }
    "/podmin".to_string()
}

/// Reads active MSI theme (`"dark"` or `"light"`).
pub fn read_msi_theme() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(t) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
            .and_then(|el| el.get_attribute("data-theme"))
        {
            if t == "dark" || t == "light" {
                return t;
            }
        }
        if let Some(ls) = storage_get("msi_admin_theme") {
            if ls == "dark" || ls == "light" {
                return ls;
            }
        }
    }
    "light".to_string()
}

/// Toggles `data-theme`, `localStorage.msi_admin_theme`, and `msi_admin_theme` cookie to match `/podmin`.
pub fn toggle_msi_theme() -> String {
    let next = if read_msi_theme() == "dark" {
        "light"
    } else {
        "dark"
    };
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(root) = doc.document_element() {
                let _ = root.set_attribute("data-theme", next);
            }
            if let Ok(html_doc) = wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlDocument>(doc) {
                let cookie_str =
                    format!("msi_admin_theme={next}; path=/; max-age=31536000; SameSite=Lax");
                let _ = html_doc.set_cookie(&cookie_str);
            }
        }
        storage_set("msi_admin_theme", next);
    }
    next.to_string()
}

thread_local! {
    pub static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new());
}

pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&AppState) -> R,
{
    APP_STATE.with(|cell| f(&cell.borrow()))
}

pub fn with_state_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut AppState) -> R,
{
    APP_STATE.with(|cell| f(&mut cell.borrow_mut()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileView {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftTab {
    Published,
    Drafts,
    Research,
}

#[derive(Debug, Clone)]
pub struct ResearchReaderTab {
    pub url: String,
    pub title_hint: String,
    pub source_hint: String,
    pub is_loading: bool,
    pub error: Option<String>,
    pub extracted: Option<crate::editorial::common::ExtractedReaderArticle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Categories,
    Sources,
    Issues,
    Glossary,
    Team,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StashedTrBuffer {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub spot: Option<String>,
    #[serde(default)]
    pub blocks: Vec<ContentBlock>,
}

pub const MSI_CONTENT_TYPES: &[(&str, &str)] = &[
    ("haberler", "Haberler"),
    ("kapak-dosyasi", "Kapak Dosyası"),
    ("soylesiler", "Söyleşi"),
    ("ozel-haberler", "Özel Haber"),
    ("basyazilar", "Başyazı"),
    ("mesajlar", "Mesajlar"),
    ("tanitici-makaleler", "Tanıtıcı Makale"),
    ("makaleler-analizler", "Makale / Analiz"),
    ("advertorials", "Advertorial"),
    ("ozel-dosya", "Özel Dosya"),
    ("tskdan-haberler", "TSK'dan Haberler"),
];

pub fn msi_content_type_label_tr(code: &str) -> &'static str {
    match code.trim() {
        "kapak-dosyasi" => "Kapak Dosyası",
        "soylesiler" => "Söyleşi",
        "ozel-haberler" => "Özel Haber",
        "basyazilar" => "Başyazı",
        "mesajlar" => "Mesajlar",
        "tanitici-makaleler" => "Tanıtıcı Makale",
        "makaleler-analizler" => "Makale / Analiz",
        "advertorials" => "Advertorial",
        "ozel-dosya" => "Özel Dosya",
        "tskdan-haberler" => "TSK'dan Haberler",
        _ => "Haberler",
    }
}

pub fn msi_content_type_label_en(code: &str) -> &'static str {
    match code.trim() {
        "kapak-dosyasi" => "Cover Story",
        "soylesiler" => "Interview",
        "ozel-haberler" => "Special Report",
        "basyazilar" => "Editorial",
        "mesajlar" => "Messages",
        "tanitici-makaleler" => "Promotional Feature",
        "makaleler-analizler" => "Article / Analysis",
        "advertorials" => "Advertorial",
        "ozel-dosya" => "Special Dossier",
        "tskdan-haberler" => "TAF News",
        _ => "Defence News",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewDevice {
    Desktop,
    Mobile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewsViewMode {
    Grid,
    List,
}

impl NewsViewMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            NewsViewMode::Grid => "grid",
            NewsViewMode::List => "list",
        }
    }

    pub fn from_str(s: &str) -> Self {
        if s == "list" {
            NewsViewMode::List
        } else {
            NewsViewMode::Grid
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PanelWidths {
    #[serde(rename = "left-panel")]
    pub left: f64,
    #[serde(rename = "middle-panel")]
    pub middle: f64,
    #[serde(rename = "right-panel")]
    pub right: f64,
}

impl Default for PanelWidths {
    fn default() -> Self {
        Self {
            left: 25.0,
            middle: 40.0,
            right: 35.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandleKind {
    LeftMiddle,
    MiddleRight,
}

#[derive(Debug, Clone, Copy)]
pub struct ResizeDrag {
    pub kind: ResizeHandleKind,
    pub start_x: f64,
    pub group_width: f64,
    pub start_left: f64,
    pub start_middle: f64,
    pub start_right: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TagSuggestion {
    pub tag: String,
    #[serde(default)]
    pub count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingEditorConfirm {
    NewArticle,
    ClearEditor,
    MoveSelectedToEditor,
}

pub struct AppState {
    // Auth
    pub auth_loading: bool,
    pub token: Option<String>,
    pub user: Option<User>,
    pub login_username: String,
    pub login_password: String,
    pub login_error: String,
    pub login_submitting: bool,

    // Viewport & Resizable Panels
    pub is_desktop: bool,
    pub mobile_view: MobileView,
    pub panel_widths: PanelWidths,
    pub active_resize: Option<ResizeDrag>,
    pub dragging_block_id: Option<String>,
    pub block_seq: u64,

    // Navigation, Floating Settings Popover & View Mode
    pub left_tab: LeftTab,
    pub show_settings_popover: bool,
    pub settings_tab: SettingsTab,
    pub news_view_mode: NewsViewMode,

    // Header Search & Dropdowns
    pub search_query: String,
    pub search_start_date: String,
    pub search_end_date: String,
    pub show_search_filters: bool,
    pub search_debounce_seq: u64,
    pub is_profile_dropdown_open: bool,

    // Published & Draft Articles + Active Reader Article
    pub published_articles: Vec<Article>,
    pub published_total: usize,
    pub draft_articles: Vec<Article>,
    pub draft_total: usize,
    pub is_loading_more: bool,
    pub is_refreshing_articles: bool,
    pub selected_article: Option<Article>,

    // Live Editing Conflict Locks
    pub active_locks: Vec<ArticleLock>,
    pub editing_conflict_lock: Option<ArticleLock>,
    pub lock_timer_seq: u64,

    // Categories & Team Settings
    pub categories: Vec<Category>,
    pub new_cat_name: String,
    pub new_cat_slug: String,
    pub new_cat_color: String,
    pub new_cat_parent_id: Option<i32>,
    pub editing_cat_id: Option<i32>,
    pub edit_cat_name: String,
    pub edit_cat_slug: String,
    pub edit_cat_color: String,
    pub edit_cat_parent_id: Option<i32>,
    pub is_saving_category: bool,
    pub config: WordPressConfig,
    pub is_testing: bool,
    pub users: Vec<User>,
    pub new_user_username: String,
    pub new_user_password: String,
    pub new_user_role: String,
    pub is_adding_user: bool,

    // Right Panel 4-Step Publishing Wizard
    pub editor_step: u8,
    pub editing_article: Article,
    pub editing_baseline_sig: String,
    pub all_blocks_expanded: bool,
    pub is_uploading: bool,
    pub is_posting: bool,
    pub step2_remote_url: String,
    pub editor_image_error: bool,
    pub is_cropping: bool,

    // Step 1 Live Blog Quick Entry & Related Article Search
    pub live_quick_time: String,
    pub live_quick_title: String,
    pub live_quick_body: String,
    pub related_search_queries: HashMap<String, String>,
    pub editing_related_block_id: Option<String>,

    // Step 2 Media Archive & Watermark Options
    pub upload_with_watermark: bool,
    pub cover_watermarked: bool,
    pub show_media_modal: bool,
    pub media_picker_target: String,
    pub media_assets: Vec<MediaAsset>,
    pub media_total: usize,
    pub media_search_query: String,
    pub media_search_seq: u64,
    pub is_loading_media: bool,

    // Step 3 Tags Autocomplete & 301 Redirects Modal
    pub tag_input_query: String,
    pub tag_suggestions: Vec<TagSuggestion>,
    pub tag_debounce_seq: u64,
    pub show_redirects_modal: bool,
    pub redirects: Vec<UrlRedirect>,
    pub is_loading_redirects: bool,

    // Revision History Modal & Full-Page Article Preview
    pub show_revisions_modal: bool,
    pub revisions_article_id: String,
    pub revisions_article_title: String,
    pub revisions: Vec<ArticleRevision>,
    pub is_loading_revisions: bool,
    pub is_restoring_revision: bool,
    pub middle_preview_draft: bool,
    pub show_preview_modal: bool,
    pub preview_device: PreviewDevice,
    pub pending_editor_confirm: Option<PendingEditorConfirm>,

    // Interactive 1200x675 Canvas Cropper
    pub crop_zoom: f64,
    pub crop_pan_x: f64,
    pub crop_pan_y: f64,
    pub crop_drag_origin: Option<(f64, f64, f64, f64)>, // (client_x, client_y, start_pan_x, start_pan_y)
    pub crop_image_el: Option<Rc<web_sys::HtmlImageElement>>,
    pub crop_loaded_src: String,

    // Change Password Modal
    pub show_change_password: bool,
    pub change_pw_current: String,
    pub change_pw_new: String,
    pub change_pw_confirm: String,
    pub is_changing_password: bool,

    // Notifications
    pub error: Option<String>,
    pub error_seq: u64,
    pub success: Option<String>,
    pub success_seq: u64,

    // Research Desk & Multi-Source Clean Reader View
    pub research_query: String,
    pub research_scope: String,
    pub research_time: String,
    pub research_results: Vec<crate::editorial::common::ResearchSearchItem>,
    pub is_searching_research: bool,
    pub research_loaded_once: bool,
    pub research_tabs: Vec<ResearchReaderTab>,
    pub active_research_url: Option<String>,
    pub middle_showing_research: bool,
    pub show_research_info: bool,
    pub research_sources: Vec<crate::editorial::common::ResearchSource>,
    pub selected_research_source_id: Option<i32>,
    pub new_source_name: String,
    pub new_source_url: String,
    pub new_source_category: String,
    pub new_source_scope: String,
    pub is_saving_source: bool,
    pub editing_source_id: Option<i32>,
    pub edit_source_name: String,
    pub edit_source_url: String,
    pub edit_source_category: String,
    pub edit_source_scope: String,
    pub edit_source_enabled: bool,
    pub research_category: String,
    pub research_panel_collapsed: bool,
    pub show_research_tabs_menu: bool,

    // MSI Dergisi & MSI TDR Editorial Metadata, Bilingual Active-Buffer & Workflow
    pub ssb_tags_catalog: Vec<SsbTag>,
    pub magazine_issues: Vec<MagazineIssueItem>,
    pub glossary_rules: Vec<GlossaryRule>,

    pub editor_lang: String, // "tr" | "en"
    pub stashed_tr_title: String,
    pub stashed_tr_spot: Option<String>,
    pub stashed_tr_blocks: Vec<ContentBlock>,
    pub preview_lang: String, // "tr" | "en"

    pub show_glossary_drawer: bool,
    pub glossary_drawer_query: String,
    pub show_author_meta_bar: bool,
    pub draft_status_filter: String, // "all" | "draft" | "pending_review" | "future" | "archived"

    // Settings: Magazine Issues CRUD
    pub new_issue_number: String,
    pub new_issue_lang: String,
    pub new_issue_title_tr: String,
    pub new_issue_title_en: String,
    pub new_issue_month: String,
    pub new_issue_year: String,
    pub new_issue_special: bool,
    pub new_issue_event: String,
    pub is_saving_issue: bool,

    // Settings: Glossary Compliance CRUD
    pub settings_glossary_query: String,
    pub new_glossary_term_tr: String,
    pub new_glossary_abbr_tr: String,
    pub new_glossary_preferred_en: String,
    pub new_glossary_abbr_en: String,
    pub new_glossary_forbidden: String,
    pub new_glossary_category: String,
    pub new_glossary_rule_type: String,
    pub new_glossary_case_sensitive: bool,
    pub new_glossary_note: String,
    pub is_saving_glossary: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let default_cfg = WordPressConfig::default();

        Self {
            auth_loading: true,
            token: None,
            user: None,
            login_username: String::new(),
            login_password: String::new(),
            login_error: String::new(),
            login_submitting: false,

            is_desktop: true,
            mobile_view: MobileView::Left,
            panel_widths: PanelWidths::default(),
            active_resize: None,
            dragging_block_id: None,
            block_seq: 1,

            left_tab: LeftTab::Published,
            show_settings_popover: false,
            settings_tab: SettingsTab::Categories,
            news_view_mode: NewsViewMode::Grid,

            search_query: String::new(),
            search_start_date: String::new(),
            search_end_date: String::new(),
            show_search_filters: false,
            search_debounce_seq: 0,
            is_profile_dropdown_open: false,

            published_articles: Vec::new(),
            published_total: 0,
            draft_articles: Vec::new(),
            draft_total: 0,
            is_loading_more: false,
            is_refreshing_articles: false,
            selected_article: None,

            active_locks: Vec::new(),
            editing_conflict_lock: None,
            lock_timer_seq: 0,

            categories: Vec::new(),
            new_cat_name: String::new(),
            new_cat_slug: String::new(),
            new_cat_color: "#52c2ef".to_string(),
            new_cat_parent_id: None,
            editing_cat_id: None,
            edit_cat_name: String::new(),
            edit_cat_slug: String::new(),
            edit_cat_color: "#52c2ef".to_string(),
            edit_cat_parent_id: None,
            is_saving_category: false,
            config: default_cfg,
            is_testing: false,
            users: Vec::new(),
            new_user_username: String::new(),
            new_user_password: String::new(),
            new_user_role: "editor".to_string(),
            is_adding_user: false,

            editor_step: 1,
            editing_article: empty_editing_article(),
            editing_baseline_sig: String::new(),
            all_blocks_expanded: true,
            is_uploading: false,
            is_posting: false,
            step2_remote_url: String::new(),
            editor_image_error: false,
            is_cropping: false,

            live_quick_time: js_now_hhmm(),
            live_quick_title: String::new(),
            live_quick_body: String::new(),
            related_search_queries: HashMap::new(),
            editing_related_block_id: None,

            upload_with_watermark: false,
            cover_watermarked: false,
            show_media_modal: false,
            media_picker_target: "step2".to_string(),
            media_assets: Vec::new(),
            media_total: 0,
            media_search_query: String::new(),
            media_search_seq: 0,
            is_loading_media: false,

            tag_input_query: String::new(),
            tag_suggestions: Vec::new(),
            tag_debounce_seq: 0,
            show_redirects_modal: false,
            redirects: Vec::new(),
            is_loading_redirects: false,

            show_revisions_modal: false,
            revisions_article_id: String::new(),
            revisions_article_title: String::new(),
            revisions: Vec::new(),
            is_loading_revisions: false,
            is_restoring_revision: false,
            middle_preview_draft: false,
            show_preview_modal: false,
            preview_device: PreviewDevice::Desktop,
            pending_editor_confirm: None,

            crop_zoom: 1.0,
            crop_pan_x: 0.0,
            crop_pan_y: 0.0,
            crop_drag_origin: None,
            crop_image_el: None,
            crop_loaded_src: String::new(),

            show_change_password: false,
            change_pw_current: String::new(),
            change_pw_new: String::new(),
            change_pw_confirm: String::new(),
            is_changing_password: false,

            error: None,
            error_seq: 0,
            success: None,
            success_seq: 0,

            research_query: String::new(),
            research_scope: "tr".to_string(),
            research_time: "all".to_string(),
            research_results: Vec::new(),
            is_searching_research: false,
            research_loaded_once: false,
            research_tabs: Vec::new(),
            active_research_url: None,
            middle_showing_research: false,
            show_research_info: false,
            research_sources: Vec::new(),
            selected_research_source_id: None,
            new_source_name: String::new(),
            new_source_url: String::new(),
            new_source_category: "Gündem".to_string(),
            new_source_scope: "tr".to_string(),
            is_saving_source: false,
            editing_source_id: None,
            edit_source_name: String::new(),
            edit_source_url: String::new(),
            edit_source_category: "Gündem".to_string(),
            edit_source_scope: "tr".to_string(),
            edit_source_enabled: true,
            research_category: String::new(),
            research_panel_collapsed: false,
            show_research_tabs_menu: false,

            ssb_tags_catalog: Vec::new(),
            magazine_issues: Vec::new(),
            glossary_rules: Vec::new(),

            editor_lang: "tr".to_string(),
            stashed_tr_title: String::new(),
            stashed_tr_spot: None,
            stashed_tr_blocks: Vec::new(),
            preview_lang: "tr".to_string(),

            show_glossary_drawer: false,
            glossary_drawer_query: String::new(),
            show_author_meta_bar: false,
            draft_status_filter: "all".to_string(),

            new_issue_number: String::new(),
            new_issue_lang: "tr".to_string(),
            new_issue_title_tr: String::new(),
            new_issue_title_en: String::new(),
            new_issue_month: String::new(),
            new_issue_year: "2026".to_string(),
            new_issue_special: false,
            new_issue_event: String::new(),
            is_saving_issue: false,

            settings_glossary_query: String::new(),
            new_glossary_term_tr: String::new(),
            new_glossary_abbr_tr: String::new(),
            new_glossary_preferred_en: String::new(),
            new_glossary_abbr_en: String::new(),
            new_glossary_forbidden: String::new(),
            new_glossary_category: "institutional".to_string(),
            new_glossary_rule_type: "fixed".to_string(),
            new_glossary_case_sensitive: false,
            new_glossary_note: String::new(),
            is_saving_glossary: false,
        }
    }

    pub fn caps(&self) -> EditorialCapabilities {
        self.user
            .as_ref()
            .map(|u| u.capabilities.clone())
            .unwrap_or_default()
    }

    pub fn active_research_tab_index(&self) -> Option<usize> {
        let active_url = self.active_research_url.as_deref()?;
        self.research_tabs.iter().position(|t| t.url == active_url)
    }

    pub fn prev_research_tab(&mut self) -> Option<String> {
        if self.research_tabs.is_empty() {
            return None;
        }
        let cur_idx = self.active_research_tab_index().unwrap_or(0);
        let prev_idx = if cur_idx == 0 {
            self.research_tabs.len().saturating_sub(1)
        } else {
            cur_idx - 1
        };
        let target_url = self.research_tabs[prev_idx].url.clone();
        self.active_research_url = Some(target_url.clone());
        Some(target_url)
    }

    pub fn next_research_tab(&mut self) -> Option<String> {
        if self.research_tabs.is_empty() {
            return None;
        }
        let cur_idx = self.active_research_tab_index().unwrap_or(0);
        let next_idx = (cur_idx + 1) % self.research_tabs.len();
        let target_url = self.research_tabs[next_idx].url.clone();
        self.active_research_url = Some(target_url.clone());
        Some(target_url)
    }

    pub fn next_block_id(&mut self) -> String {
        let now = js_now_ms();
        let id = format!("b-{}-{}", now, self.block_seq);
        self.block_seq += 1;
        id
    }

    /// Resets the bilingual active-buffer state cleanly to `"tr"` whenever an article is loaded or cleared.
    pub fn reset_editor_lang_buffers(&mut self) {
        self.editor_lang = "tr".to_string();
        self.stashed_tr_title.clear();
        self.stashed_tr_spot = None;
        self.stashed_tr_blocks.clear();
        self.preview_lang = "tr".to_string();
        self.show_glossary_drawer = false;
        self.show_author_meta_bar = self
            .editing_article
            .author_name
            .as_deref()
            .map_or(false, |s| !s.trim().is_empty())
            || self
                .editing_article
                .author_title
                .as_deref()
                .map_or(false, |s| !s.trim().is_empty())
            || self
                .editing_article
                .video_url
                .as_deref()
                .map_or(false, |s| !s.trim().is_empty());
        clear_stashed_tr_buffer();
    }

    /// Swaps the active editor buffer between `"tr"` (MSI Dergisi / Haber) and `"en"` (MSI TDR),
    /// keeping all 16 rich block types, floating bubble, paste parser, and drag-reorder 100% seamless.
    pub fn switch_editor_lang(&mut self, target_lang: &str) -> bool {
        let norm_target = if target_lang.eq_ignore_ascii_case("en") {
            "en"
        } else {
            "tr"
        };
        if self.editor_lang == norm_target {
            return false;
        }

        if norm_target == "en" {
            // Stash current TR fields
            self.stashed_tr_title = self.editing_article.title.clone();
            self.stashed_tr_spot = self.editing_article.spot.clone();
            self.stashed_tr_blocks = self
                .editing_article
                .blocks
                .clone()
                .unwrap_or_default();
            save_stashed_tr_buffer(
                "en",
                &StashedTrBuffer {
                    title: self.stashed_tr_title.clone(),
                    spot: self.stashed_tr_spot.clone(),
                    blocks: self.stashed_tr_blocks.clone(),
                },
            );

            // Move EN fields into active `editing_article.title / spot / blocks`
            let en_title = self.editing_article.title_en.clone().unwrap_or_default();
            let en_spot = self.editing_article.spot_en.clone();
            let init_id = self.next_block_id();
            let en_blocks = self
                .editing_article
                .blocks_en
                .clone()
                .filter(|b| !b.is_empty())
                .or_else(|| {
                    self.editing_article
                        .content_en
                        .as_deref()
                        .map(str::trim)
                        .filter(|c| !c.is_empty())
                        .map(parse_content_to_blocks)
                        .filter(|b| !b.is_empty())
                })
                .unwrap_or_else(|| {
                    vec![ContentBlock {
                        id: init_id,
                        block_type: "paragraph".to_string(),
                        content: String::new(),
                        meta: None,
                    }]
                });

            self.editing_article.title = en_title;
            self.editing_article.spot = en_spot;
            self.editing_article.blocks = Some(en_blocks);
            self.editor_lang = "en".to_string();
            if self.middle_preview_draft || self.show_preview_modal {
                self.preview_lang = "en".to_string();
            }
            save_editing_draft(&self.editing_article);
            true
        } else {
            // Save active EN buffer back into `title_en / spot_en / blocks_en / content_en`
            let en_title = self.editing_article.title.clone();
            let en_spot = self.editing_article.spot.clone();
            let en_blocks = self
                .editing_article
                .blocks
                .clone()
                .unwrap_or_default();
            let en_has_data = !en_title.trim().is_empty()
                || en_spot.as_deref().map_or(false, |s| !s.trim().is_empty())
                || has_meaningful_blocks(&en_blocks);

            if en_has_data {
                self.editing_article.title_en =
                    (!en_title.trim().is_empty()).then_some(en_title);
                self.editing_article.spot_en =
                    en_spot.filter(|s| !s.trim().is_empty());
                self.editing_article.content_en = Some(blocks_to_wp_html(&en_blocks));
                self.editing_article.blocks_en = Some(en_blocks);
                if self.editing_article.en_status.trim().is_empty()
                    || self.editing_article.en_status == "none"
                {
                    self.editing_article.en_status = "draft".to_string();
                }
            } else {
                self.editing_article.title_en = None;
                self.editing_article.spot_en = None;
                self.editing_article.content_en = None;
                self.editing_article.blocks_en = None;
            }

            // Restore stashed TR fields into active `editing_article.title / spot / blocks`
            let tr_title = std::mem::take(&mut self.stashed_tr_title);
            let tr_spot = self.stashed_tr_spot.take();
            let tr_blocks = std::mem::take(&mut self.stashed_tr_blocks);
            let init_id = self.next_block_id();
            self.editing_article.title = tr_title;
            self.editing_article.spot = tr_spot;
            self.editing_article.blocks = Some(if tr_blocks.is_empty() {
                vec![ContentBlock {
                    id: init_id,
                    block_type: "paragraph".to_string(),
                    content: String::new(),
                    meta: None,
                }]
            } else {
                tr_blocks
            });
            self.editor_lang = "tr".to_string();
            if self.middle_preview_draft || self.show_preview_modal {
                self.preview_lang = "tr".to_string();
            }
            clear_stashed_tr_buffer();
            save_editing_draft(&self.editing_article);
            true
        }
    }

    /// Returns a cloned `Article` guaranteed to have TR in `title / spot / blocks / content`
    /// and EN in `title_en / spot_en / blocks_en / content_en` regardless of `self.editor_lang`.
    pub fn canonical_editing_article(&self) -> Article {
        let mut art = self.editing_article.clone();
        if self.editor_lang == "en" {
            let en_title = art.title.clone();
            let en_spot = art.spot.clone();
            let en_blocks = art.blocks.clone().unwrap_or_default();
            let en_has_data = !en_title.trim().is_empty()
                || en_spot.as_deref().map_or(false, |s| !s.trim().is_empty())
                || has_meaningful_blocks(&en_blocks);

            if en_has_data {
                art.title_en = (!en_title.trim().is_empty()).then_some(en_title);
                art.spot_en = en_spot.filter(|s| !s.trim().is_empty());
                art.content_en = Some(blocks_to_wp_html(&en_blocks));
                art.blocks_en = Some(en_blocks);
                if art.en_status.trim().is_empty() || art.en_status == "none" {
                    art.en_status = "draft".to_string();
                }
            } else {
                art.title_en = None;
                art.spot_en = None;
                art.content_en = None;
                art.blocks_en = None;
            }

            art.title = self.stashed_tr_title.clone();
            art.spot = self.stashed_tr_spot.clone();
            let tr_blocks = if self.stashed_tr_blocks.is_empty() {
                vec![ContentBlock {
                    id: "b-init-1".to_string(),
                    block_type: "paragraph".to_string(),
                    content: String::new(),
                    meta: None,
                }]
            } else {
                self.stashed_tr_blocks.clone()
            };
            art.content = if has_meaningful_blocks(&tr_blocks) {
                blocks_to_wp_html(&tr_blocks)
            } else {
                String::new()
            };
            art.blocks = Some(tr_blocks);
        } else {
            if let Some(ref blks) = art.blocks {
                if has_meaningful_blocks(blks) {
                    art.content = blocks_to_wp_html(blks);
                }
            }
            if let Some(ref en_blks) = art.blocks_en {
                if has_meaningful_blocks(en_blks) {
                    art.content_en = Some(blocks_to_wp_html(en_blks));
                }
            }
        }

        let calc_blocks = art.blocks.as_deref().unwrap_or(&[]);
        art.read_time_min = Some(estimate_read_time_minutes(
            &art.title,
            art.spot.as_deref().unwrap_or(""),
            calc_blocks,
        ));
        art
    }

    /// Copies TR `title`, `spot`, and `blocks` into the active EN editor, running
    /// `apply_tr_to_en_glossary_assist` on textual fields while preserving images, embeds, galleries, and dividers.
    pub fn copy_tr_to_en_with_glossary_assist(&mut self) -> usize {
        if self.editor_lang != "en" {
            self.switch_editor_lang("en");
        }
        let rules = self.glossary_rules.clone();
        let tr_title = self.stashed_tr_title.clone();
        let tr_spot = self.stashed_tr_spot.clone();
        let tr_blocks = self.stashed_tr_blocks.clone();

        let assisted_title = apply_tr_to_en_glossary_assist(&tr_title, &rules);
        let assisted_spot = tr_spot
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| apply_tr_to_en_glossary_assist(s, &rules));

        let mut new_en_blocks = Vec::with_capacity(tr_blocks.len().max(1));
        for b in &tr_blocks {
            let nid = self.next_block_id();
            let (new_content, new_meta) = match b.block_type.as_str() {
                "divider" | "related_article" | "code" => (b.content.clone(), b.meta.clone()),
                "image" | "embed" => {
                    let cap = b
                        .meta
                        .as_deref()
                        .filter(|m| !m.trim().is_empty())
                        .map(|m| apply_tr_to_en_glossary_assist(m, &rules));
                    (b.content.clone(), cap)
                }
                "gallery" => {
                    let mut items: Vec<GalleryItem> =
                        serde_json::from_str(b.content.trim()).unwrap_or_default();
                    for it in &mut items {
                        if !it.caption.trim().is_empty() {
                            it.caption = apply_tr_to_en_glossary_assist(&it.caption, &rules);
                        }
                    }
                    let g_json =
                        serde_json::to_string(&items).unwrap_or_else(|_| b.content.clone());
                    let g_meta = b
                        .meta
                        .as_deref()
                        .filter(|m| !m.trim().is_empty())
                        .map(|m| apply_tr_to_en_glossary_assist(m, &rules));
                    (g_json, g_meta)
                }
                "callout" => {
                    let c = apply_tr_to_en_glossary_assist(&b.content, &rules);
                    let m = b.meta.as_deref().map(|raw| {
                        let (var, ttl) = raw.split_once('|').unwrap_or((raw, ""));
                        if ttl.trim().is_empty() {
                            raw.to_string()
                        } else {
                            format!("{var}|{}", apply_tr_to_en_glossary_assist(ttl, &rules))
                        }
                    });
                    (c, m)
                }
                "live_entry" => {
                    let c = apply_tr_to_en_glossary_assist(&b.content, &rules);
                    let m = b.meta.as_deref().map(|raw| {
                        let (tm, ttl) = raw.split_once('|').unwrap_or((raw, ""));
                        if ttl.trim().is_empty() {
                            raw.to_string()
                        } else {
                            format!("{tm}|{}", apply_tr_to_en_glossary_assist(ttl, &rules))
                        }
                    });
                    (c, m)
                }
                _ => {
                    let c = apply_tr_to_en_glossary_assist(&b.content, &rules);
                    let m = b
                        .meta
                        .as_deref()
                        .filter(|m| !m.trim().is_empty())
                        .map(|m| apply_tr_to_en_glossary_assist(m, &rules));
                    (c, m)
                }
            };
            new_en_blocks.push(ContentBlock {
                id: nid,
                block_type: b.block_type.clone(),
                content: new_content,
                meta: new_meta,
            });
        }

        if new_en_blocks.is_empty() {
            let nid = self.next_block_id();
            new_en_blocks.push(ContentBlock {
                id: nid,
                block_type: "paragraph".to_string(),
                content: String::new(),
                meta: None,
            });
        }

        let cnt = new_en_blocks.len();
        self.editing_article.title = assisted_title;
        self.editing_article.spot = assisted_spot;
        self.editing_article.blocks = Some(new_en_blocks);
        self.editing_article.en_status = "draft".to_string();
        save_editing_draft(&self.editing_article);
        cnt
    }

    /// Evaluates MSI TDR & British English glossary compliance across the active English editor buffer.
    pub fn evaluate_active_en_glossary(&self) -> (Vec<GlossaryViolation>, bool) {
        if self.editor_lang != "en" {
            return (Vec::new(), false);
        }
        let mut combined = String::new();
        if !self.editing_article.title.trim().is_empty() {
            combined.push_str(self.editing_article.title.trim());
            combined.push('\n');
        }
        if let Some(sp) = self
            .editing_article
            .spot
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            combined.push_str(sp.trim());
            combined.push('\n');
        }
        if let Some(blocks) = self.editing_article.blocks.as_ref() {
            for b in blocks {
                match b.block_type.as_str() {
                    "divider" | "related_article" | "code" => {}
                    "image" | "embed" => {
                        if let Some(cap) = b.meta.as_deref().filter(|m| !m.trim().is_empty()) {
                            combined.push_str(cap.trim());
                            combined.push('\n');
                        }
                    }
                    "gallery" => {
                        if let Some(gt) = b.meta.as_deref().filter(|m| !m.trim().is_empty()) {
                            combined.push_str(gt.trim());
                            combined.push('\n');
                        }
                        if let Ok(items) = serde_json::from_str::<Vec<GalleryItem>>(b.content.trim())
                        {
                            for it in items {
                                if !it.caption.trim().is_empty() {
                                    combined.push_str(it.caption.trim());
                                    combined.push('\n');
                                }
                            }
                        }
                    }
                    "callout" | "live_entry" => {
                        if let Some(raw_m) = b.meta.as_deref() {
                            if let Some((_, ttl)) = raw_m.split_once('|') {
                                if !ttl.trim().is_empty() {
                                    combined.push_str(ttl.trim());
                                    combined.push('\n');
                                }
                            }
                        }
                        if !b.content.trim().is_empty() {
                            combined.push_str(&b.content);
                            combined.push('\n');
                        }
                    }
                    _ => {
                        if let Some(m) = b.meta.as_deref().filter(|m| !m.trim().is_empty()) {
                            combined.push_str(m.trim());
                            combined.push('\n');
                        }
                        if !b.content.trim().is_empty() {
                            combined.push_str(&b.content);
                            combined.push('\n');
                        }
                    }
                }
            }
        }
        let has_any_text = !crate::editorial::common::strip_all_html_tags(&combined).trim().is_empty();
        if !has_any_text || self.glossary_rules.is_empty() {
            return (Vec::new(), has_any_text);
        }
        let violations = evaluate_glossary_compliance(&combined, &self.glossary_rules);
        (violations, true)
    }

    /// Applies 1-click glossary compliance fixes across the active English `title`, `spot`, and `blocks`.
    pub fn apply_active_en_glossary_fixes(&mut self) -> usize {
        if self.editor_lang != "en" || self.glossary_rules.is_empty() {
            return 0;
        }
        let rules = &self.glossary_rules;
        let mut total_fixes = 0usize;

        if !self.editing_article.title.is_empty() {
            let (fixed, n) = apply_glossary_fixes_to_text(&self.editing_article.title, rules);
            if n > 0 {
                self.editing_article.title = fixed;
                total_fixes += n;
            }
        }
        if let Some(ref sp) = self.editing_article.spot.clone() {
            let (fixed, n) = apply_glossary_fixes_to_text(sp, rules);
            if n > 0 {
                self.editing_article.spot = Some(fixed);
                total_fixes += n;
            }
        }
        if let Some(blocks) = &mut self.editing_article.blocks {
            for b in blocks.iter_mut() {
                match b.block_type.as_str() {
                    "divider" | "related_article" | "code" => {}
                    "image" | "embed" => {
                        if let Some(ref cap) = b.meta.clone() {
                            let (fixed, n) = apply_glossary_fixes_to_text(cap, rules);
                            if n > 0 {
                                b.meta = Some(fixed);
                                total_fixes += n;
                            }
                        }
                    }
                    "gallery" => {
                        if let Some(ref gt) = b.meta.clone() {
                            let (fixed, n) = apply_glossary_fixes_to_text(gt, rules);
                            if n > 0 {
                                b.meta = Some(fixed);
                                total_fixes += n;
                            }
                        }
                        if let Ok(mut items) =
                            serde_json::from_str::<Vec<GalleryItem>>(b.content.trim())
                        {
                            let mut changed_gal = false;
                            for it in &mut items {
                                let (fixed, n) = apply_glossary_fixes_to_text(&it.caption, rules);
                                if n > 0 {
                                    it.caption = fixed;
                                    total_fixes += n;
                                    changed_gal = true;
                                }
                            }
                            if changed_gal {
                                if let Ok(js) = serde_json::to_string(&items) {
                                    b.content = js;
                                }
                            }
                        }
                    }
                    "callout" | "live_entry" => {
                        if let Some(ref raw_m) = b.meta.clone() {
                            if let Some((prefix, ttl)) = raw_m.split_once('|') {
                                let (fixed_ttl, n) = apply_glossary_fixes_to_text(ttl, rules);
                                if n > 0 {
                                    b.meta = Some(format!("{prefix}|{fixed_ttl}"));
                                    total_fixes += n;
                                }
                            }
                        }
                        let (fixed_c, nc) = apply_glossary_fixes_to_text(&b.content, rules);
                        if nc > 0 {
                            b.content = fixed_c;
                            total_fixes += nc;
                        }
                    }
                    _ => {
                        if let Some(ref m) = b.meta.clone() {
                            let (fixed_m, nm) = apply_glossary_fixes_to_text(m, rules);
                            if nm > 0 {
                                b.meta = Some(fixed_m);
                                total_fixes += nm;
                            }
                        }
                        let (fixed_c, nc) = apply_glossary_fixes_to_text(&b.content, rules);
                        if nc > 0 {
                            b.content = fixed_c;
                            total_fixes += nc;
                        }
                    }
                }
            }
        }
        if total_fixes > 0 {
            save_editing_draft(&self.editing_article);
        }
        total_fixes
    }

    /// Appends a rich block to `editing_article.blocks` (replacing the sole initial empty paragraph `b-init-1` if present).
    pub fn append_or_replace_initial_block(
        &mut self,
        block_type: &str,
        content: String,
        meta: Option<String>,
    ) -> String {
        let bid = self.next_block_id();
        let new_blk = ContentBlock {
            id: bid.clone(),
            block_type: block_type.to_string(),
            content,
            meta,
        };
        let blocks = self.editing_article.blocks.get_or_insert_with(Vec::new);
        if blocks.len() == 1
            && blocks[0].block_type == "paragraph"
            && crate::editorial::common::strip_all_html_tags(&blocks[0].content)
                .trim()
                .is_empty()
        {
            blocks[0] = new_blk;
        } else {
            blocks.push(new_blk);
        }
        save_editing_draft(&self.editing_article);
        bid
    }

    pub fn filtered_published_articles(&self) -> Vec<&Article> {
        let q = turkish_lowercase(self.search_query.trim());
        if q.is_empty() {
            return self.published_articles.iter().collect();
        }
        self.published_articles
            .iter()
            .filter(|art| article_matches_query(art, &q))
            .collect()
    }

    pub fn draft_count_for_status(&self, status_filter: &str) -> usize {
        let q = turkish_lowercase(self.search_query.trim());
        self.draft_articles
            .iter()
            .filter(|art| q.is_empty() || article_matches_query(art, &q))
            .filter(|art| article_matches_draft_status(art, status_filter))
            .count()
    }

    pub fn filtered_draft_articles(&self) -> Vec<&Article> {
        let q = turkish_lowercase(self.search_query.trim());
        let sf = self.draft_status_filter.as_str();
        self.draft_articles
            .iter()
            .filter(|art| q.is_empty() || article_matches_query(art, &q))
            .filter(|art| article_matches_draft_status(art, sf))
            .collect()
    }

    pub fn is_other_user_lock(&self, lock: &ArticleLock) -> bool {
        if lock.username.trim().is_empty() {
            return false;
        }
        if let Some(me) = self.user.as_ref() {
            lock.user_id != me.id && !lock.username.eq_ignore_ascii_case(&me.username)
        } else {
            true
        }
    }

    pub fn lock_holder_for_article(&self, article_id: &str) -> Option<&ArticleLock> {
        let aid = article_id.trim();
        if aid.is_empty() {
            return None;
        }
        self.active_locks
            .iter()
            .find(|l| l.article_id == aid && self.is_other_user_lock(l))
    }

    pub fn effective_editing_conflict(&self) -> Option<&ArticleLock> {
        if let Some(ref conflict) = self.editing_conflict_lock {
            if conflict.article_id == self.editing_article.id && self.is_other_user_lock(conflict) {
                return Some(conflict);
            }
        }
        self.lock_holder_for_article(&self.editing_article.id)
    }
}

fn article_matches_query(art: &Article, q: &str) -> bool {
    turkish_lowercase(&art.title).contains(q)
        || art
            .spot
            .as_deref()
            .map_or(false, |sp| turkish_lowercase(sp).contains(q))
        || turkish_lowercase(&art.content).contains(q)
        || art
            .title_en
            .as_deref()
            .map_or(false, |te| turkish_lowercase(te).contains(q))
        || art
            .spot_en
            .as_deref()
            .map_or(false, |se| turkish_lowercase(se).contains(q))
        || art
            .author_name
            .as_deref()
            .map_or(false, |au| turkish_lowercase(au).contains(q))
        || art
            .ssb_tags
            .as_ref()
            .map_or(false, |tags| tags.iter().any(|t| turkish_lowercase(t).contains(q)))
}

fn article_matches_draft_status(art: &Article, filter_key: &str) -> bool {
    match filter_key.trim() {
        "" | "all" => true,
        "draft" => art.status.trim().is_empty() || art.status == "draft",
        "pending_review" => art.status == "pending_review",
        "future" => art.status == "future",
        "archived" => art.status == "archived",
        other => art.status == other,
    }
}

pub fn has_meaningful_blocks(blocks: &[ContentBlock]) -> bool {
    for b in blocks {
        match b.block_type.as_str() {
            "divider" => return true,
            "gallery" => {
                let c = b.content.trim();
                if !c.is_empty() && c != "[]" {
                    return true;
                }
            }
            "related_article" => {
                let c = b.content.trim();
                if !c.is_empty() && c != "{}" {
                    return true;
                }
            }
            "image" | "embed" => {
                if !b.content.trim().is_empty() {
                    return true;
                }
            }
            _ => {
                if !crate::editorial::common::strip_all_html_tags(&b.content).trim().is_empty() {
                    return true;
                }
            }
        }
    }
    false
}

pub fn article_has_english(art: &Article) -> bool {
    art.title_en
        .as_deref()
        .map_or(false, |t| !t.trim().is_empty())
        || art
            .spot_en
            .as_deref()
            .map_or(false, |s| !s.trim().is_empty())
        || art
            .content_en
            .as_deref()
            .map_or(false, |c| !crate::editorial::common::strip_all_html_tags(c).trim().is_empty())
        || art
            .blocks_en
            .as_ref()
            .map_or(false, |b| has_meaningful_blocks(b))
}

pub fn empty_editing_article() -> Article {
    Article {
        title: String::new(),
        content: String::new(),
        blocks: Some(vec![ContentBlock {
            id: "b-init-1".to_string(),
            block_type: "paragraph".to_string(),
            content: String::new(),
            meta: None,
        }]),
        status: "draft".to_string(),
        image_url: Some(String::new()),
        image_url_camel: Some(String::new()),
        seo: Some(SeoData::default()),
        auto_share: Some(true),
        ..Default::default()
    }
}

/// Returns `true` when the Right Editor holds any user-entered content (title, spot, cover image,
/// non-empty blocks, slug, tags, EN fields, or SEO fields) so destructive actions ask confirmation first.
pub fn is_editor_populated(art: &Article) -> bool {
    if !art.title.trim().is_empty() {
        return true;
    }
    if art.spot.as_deref().map_or(false, |sp| !sp.trim().is_empty()) {
        return true;
    }
    if article_has_english(art) {
        return true;
    }
    if is_valid_image_url(art.effective_image_url()) {
        return true;
    }
    if !art.content.trim().is_empty() {
        return true;
    }
    if let Some(blocks) = art.blocks.as_ref() {
        if has_meaningful_blocks(blocks) {
            return true;
        }
    }
    if art
        .slug
        .as_deref()
        .map_or(false, |sl| !sl.trim().is_empty())
    {
        return true;
    }
    if art
        .tags
        .as_ref()
        .map_or(false, |t| t.iter().any(|x| !x.trim().is_empty()))
    {
        return true;
    }
    if art
        .ssb_tags
        .as_ref()
        .map_or(false, |t| t.iter().any(|x| !x.trim().is_empty()))
    {
        return true;
    }
    if art
        .author_name
        .as_deref()
        .map_or(false, |a| !a.trim().is_empty())
    {
        return true;
    }
    if art
        .effective_source_url()
        .map_or(false, |su| !su.trim().is_empty())
    {
        return true;
    }
    if let Some(seo) = art.seo.as_ref() {
        if !seo.title.trim().is_empty()
            || !seo.description.trim().is_empty()
            || !seo.keywords.trim().is_empty()
        {
            return true;
        }
    }
    false
}

/// Deterministic content fingerprint of `editing_article` so untouched saved drafts/articles
/// never trigger false-positive "unsaved changes" prompts when closed or switched.
pub fn editor_article_signature(art: &Article) -> String {
    let mut sig = String::with_capacity(768);
    sig.push_str(art.id.trim());
    sig.push('\x1f');
    sig.push_str(art.title.trim());
    sig.push('\x1f');
    sig.push_str(art.spot.as_deref().unwrap_or("").trim());
    sig.push('\x1f');
    sig.push_str(art.effective_image_url().unwrap_or("").trim());
    sig.push('\x1f');
    sig.push_str(art.slug.as_deref().unwrap_or("").trim());
    sig.push('\x1f');
    sig.push_str(art.status.trim());
    sig.push('\x1f');
    sig.push_str(art.effective_scheduled_at().unwrap_or("").trim());
    sig.push('\x1f');
    sig.push_str(if art.is_live_blog { "1" } else { "0" });
    sig.push_str(if art.comments_locked { "1" } else { "0" });
    sig.push_str(if art.ads_disabled { "1" } else { "0" });
    sig.push('\x1f');

    // MSI Editorial & Bilingual fields
    sig.push_str(art.title_en.as_deref().unwrap_or("").trim());
    sig.push('\x1f');
    sig.push_str(art.spot_en.as_deref().unwrap_or("").trim());
    sig.push('\x1f');
    sig.push_str(art.en_status.trim());
    sig.push('\x1f');
    sig.push_str(art.publication_channel.trim());
    sig.push('|');
    sig.push_str(art.content_type.trim());
    sig.push('|');
    sig.push_str(&art.issue_number.unwrap_or(0).to_string());
    sig.push('|');
    sig.push_str(art.issue_page_range.as_deref().unwrap_or("").trim());
    sig.push('|');
    sig.push_str(if art.is_featured { "1" } else { "0" });
    sig.push('\x1f');
    sig.push_str(art.author_name.as_deref().unwrap_or("").trim());
    sig.push('|');
    sig.push_str(art.author_title.as_deref().unwrap_or("").trim());
    sig.push('|');
    sig.push_str(art.video_url.as_deref().unwrap_or("").trim());
    sig.push('\x1f');
    sig.push_str(&art.visibility_level.to_string());
    sig.push('|');
    sig.push_str(&format!("{:.2}", art.single_price_try));
    sig.push('|');
    sig.push_str(if art.is_sponsored { "1" } else { "0" });
    sig.push('|');
    sig.push_str(art.sponsor_brand.as_deref().unwrap_or("").trim());
    sig.push('|');
    sig.push_str(art.sponsor_legal_label.as_deref().unwrap_or("").trim());
    sig.push('\x1f');

    if let Some(ssb) = art.ssb_tags.as_ref() {
        for st in ssb {
            let clean = st.trim();
            if !clean.is_empty() {
                sig.push_str(clean);
                sig.push(',');
            }
        }
    }
    sig.push('\x1f');

    if let Some(cats) = art.categories.as_ref() {
        for cid in cats {
            sig.push_str(&cid.to_string());
            sig.push(',');
        }
    }
    sig.push('\x1f');

    if let Some(tags) = art.tags.as_ref() {
        for t in tags {
            let tt = t.trim();
            if !tt.is_empty() {
                sig.push_str(tt);
                sig.push(',');
            }
        }
    }
    sig.push('\x1f');
    sig.push_str(art.effective_source_url().unwrap_or("").trim());
    sig.push('\x1f');

    if let Some(seo) = art.seo.as_ref() {
        sig.push_str(seo.title.trim());
        sig.push('|');
        sig.push_str(seo.description.trim());
        sig.push('|');
        sig.push_str(seo.keywords.trim());
    }
    sig.push('\x1f');

    if let Some(blocks) = art.blocks.as_ref() {
        for b in blocks {
            let is_empty_para = b.block_type == "paragraph"
                && crate::editorial::common::strip_all_html_tags(&b.content).trim().is_empty();
            if is_empty_para {
                continue;
            }
            sig.push_str(&b.block_type);
            sig.push(':');
            sig.push_str(b.content.trim());
            sig.push(':');
            sig.push_str(b.meta.as_deref().unwrap_or("").trim());
            sig.push('\x1e');
        }
    } else if !art.content.trim().is_empty() {
        sig.push_str("raw:");
        sig.push_str(art.content.trim());
    }
    sig.push('\x1f');

    if let Some(blocks_en) = art.blocks_en.as_ref() {
        for b in blocks_en {
            let is_empty_para = b.block_type == "paragraph"
                && crate::editorial::common::strip_all_html_tags(&b.content).trim().is_empty();
            if is_empty_para {
                continue;
            }
            sig.push_str(&b.block_type);
            sig.push(':');
            sig.push_str(b.content.trim());
            sig.push(':');
            sig.push_str(b.meta.as_deref().unwrap_or("").trim());
            sig.push('\x1e');
        }
    } else if let Some(cen) = art.content_en.as_deref() {
        if !cen.trim().is_empty() {
            sig.push_str("raw_en:");
            sig.push_str(cen.trim());
        }
    }

    sig
}

/// Decides whether clearing/leaving the Right Editor requires user confirmation:
/// - Saved Draft (`draft-...`) or Published (`pub-...`): ONLY asks if the user actually modified it (`cur_sig != baseline_sig`)!
/// - Brand-new unsaved article (`id == ""`): asks if any field/block is populated.
pub fn should_confirm_editor_discard(s: &AppState) -> bool {
    let canon = s.canonical_editing_article();
    let cur_id = canon.id.trim();
    if !cur_id.is_empty() {
        let cur_sig = editor_article_signature(&canon);
        if !s.editing_baseline_sig.is_empty() {
            return cur_sig != s.editing_baseline_sig;
        }
        if let Some(saved) = s
            .draft_articles
            .iter()
            .find(|d| d.id == cur_id)
            .or_else(|| s.published_articles.iter().find(|p| p.id == cur_id))
        {
            let norm_saved = normalize_draft_for_editor(saved.clone());
            return cur_sig != editor_article_signature(&norm_saved);
        }
        return false;
    }
    is_editor_populated(&canon)
}

pub fn set_article_image(article: &mut Article, url: &str) {
    article.image_url = Some(url.to_string());
    article.image_url_camel = Some(url.to_string());
}

pub fn set_article_source_url(article: &mut Article, url: Option<&str>) {
    let clean = url.map(str::trim).filter(|u| !u.is_empty()).map(str::to_string);
    article.source_url = clean.clone();
    article.source_url_camel = clean.clone();
    article.original_url = clean.clone();
    article.original_url_camel = clean;
}

pub fn effective_wp_post_url(article: &Article) -> Option<&str> {
    article
        .wp_post_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            article
                .wp_post_url_camel
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
}

pub fn normalize_draft_for_editor(mut article: Article) -> Article {
    let has_blocks = article
        .blocks
        .as_ref()
        .map(|b| !b.is_empty())
        .unwrap_or(false);
    if !has_blocks && !article.content.trim().is_empty() {
        let parsed = parse_content_to_blocks(&article.content);
        if !parsed.is_empty() {
            article.blocks = Some(parsed);
        } else {
            article.blocks = Some(vec![ContentBlock {
                id: "blk-1".to_string(),
                block_type: "paragraph".to_string(),
                content: article.content.clone(),
                meta: None,
            }]);
        }
    } else if article.blocks.is_none() {
        article.blocks = Some(Vec::new());
    }

    let has_blocks_en = article
        .blocks_en
        .as_ref()
        .map(|b| !b.is_empty())
        .unwrap_or(false);
    if !has_blocks_en {
        if let Some(cen) = article
            .content_en
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let parsed_en = parse_content_to_blocks(cen);
            if !parsed_en.is_empty() {
                article.blocks_en = Some(parsed_en);
            }
        }
    }

    if article.auto_share.is_none() {
        article.auto_share = Some(true);
    }
    if article.status.trim().is_empty() {
        article.status = "draft".to_string();
    }
    if article.en_status.trim().is_empty() {
        article.en_status = "none".to_string();
    }
    if article.publication_channel.trim().is_empty() {
        article.publication_channel = "news".to_string();
    }
    if article.content_type.trim().is_empty() {
        article.content_type = "haberler".to_string();
    }
    if let Some(img) = article.effective_image_url().map(|s| s.to_string()) {
        set_article_image(&mut article, &img);
    }
    article
}

// ============================================================================
// Pure-Rust Local SEO Auto-Filler (Zero AI / Zero Network)
// ============================================================================

pub fn turkish_lowercase(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            'İ' => out.push('i'),
            'I' => out.push('ı'),
            'Ğ' => out.push('ğ'),
            'Ü' => out.push('ü'),
            'Ş' => out.push('ş'),
            'Ö' => out.push('ö'),
            'Ç' => out.push('ç'),
            _ => {
                for lc in ch.to_lowercase() {
                    out.push(lc);
                }
            }
        }
    }
    out
}

fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_html_and_md_plain(raw: &str) -> String {
    let mut no_tags = String::with_capacity(raw.len());
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => {
                in_tag = true;
                no_tags.push(' ');
            }
            '>' => {
                in_tag = false;
                no_tags.push(' ');
            }
            _ if !in_tag => no_tags.push(ch),
            _ => {}
        }
    }
    let cleaned = no_tags.replace("**", "").replace("##", "").replace('#', "");
    collapse_whitespace(&cleaned)
}

fn truncate_clean(input: &str, max_chars: usize, with_ellipsis: bool) -> String {
    let clean = collapse_whitespace(input);
    let chars: Vec<char> = clean.chars().collect();
    if chars.len() <= max_chars {
        return clean;
    }

    let limit = if with_ellipsis {
        max_chars.saturating_sub(3)
    } else {
        max_chars
    };
    let slice_str: String = chars[..limit].iter().collect();
    let trimmed = if let Some((prefix, _)) = slice_str.rsplit_once(' ') {
        if prefix.chars().count() >= (limit * 2) / 3 {
            prefix.trim_end_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        } else {
            slice_str.trim_end()
        }
    } else {
        slice_str.trim_end()
    };

    if with_ellipsis {
        format!("{trimmed}...")
    } else {
        trimmed.to_string()
    }
}

/// Pure-Rust local SEO builder (`Otomatik Doldur` in Step 3):
/// - `seo.title`: truncated 60-char article title
/// - `seo.description`: first ~150 chars of plain paragraph text
/// - `seo.keywords`: top meaningful words from title (supplemented by lead paragraph if needed)
pub fn build_auto_seo(article: &Article) -> SeoData {
    const STOP_WORDS: &[&str] = &[
        "ve", "ile", "bir", "bu", "şu", "için", "gibi", "ama", "fakat", "çok", "daha", "en",
        "mi", "mı", "mu", "mü", "da", "de", "ki", "olan", "olarak", "sonra", "önce", "üzere",
        "göre", "karşı", "kadar", "hem", "veya", "yeni", "son", "tüm", "her", "ne", "nasıl",
        "neden", "kim", "olan", "oldu", "olanlar",
    ];

    let raw_title = collapse_whitespace(&article.title);
    let seo_title = truncate_clean(&raw_title, 60, false);

    let mut para_parts: Vec<String> = Vec::new();
    let mut heading_parts: Vec<String> = Vec::new();
    if let Some(spot) = article.spot.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        para_parts.push(collapse_whitespace(spot));
    }
    if let Some(blocks) = article.blocks.as_ref() {
        for blk in blocks {
            match blk.block_type.as_str() {
                "paragraph" | "quote" | "callout" | "summary" | "bullet_list" | "numbered_list"
                | "live_entry" => {
                    let p = strip_html_and_md_plain(&blk.content);
                    if !p.is_empty() {
                        para_parts.push(p);
                    }
                }
                "heading1" | "heading" | "heading3" => {
                    let h = strip_html_and_md_plain(&blk.content);
                    if !h.is_empty() {
                        heading_parts.push(h);
                    }
                }
                _ => {}
            }
        }
    }

    let plain_body = if !para_parts.is_empty() {
        para_parts.join(" ")
    } else if !article.content.trim().is_empty() {
        strip_html_and_md_plain(&article.content)
    } else if !heading_parts.is_empty() {
        heading_parts.join(" ")
    } else {
        raw_title.clone()
    };

    let desc_source = if plain_body.is_empty() {
        &raw_title
    } else {
        &plain_body
    };
    let seo_description = truncate_clean(desc_source, 150, true);

    let mut keywords: Vec<String> = Vec::new();
    let kw_source = format!("{raw_title} {plain_body}");
    for token in turkish_lowercase(&kw_source).split(|c: char| !c.is_alphanumeric()) {
        let w = token.trim();
        let char_len = w.chars().count();
        if char_len < 3 || w.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if STOP_WORDS.contains(&w) {
            continue;
        }
        if !keywords.iter().any(|existing| existing == w) {
            keywords.push(w.to_string());
        }
        if keywords.len() >= 6 {
            break;
        }
    }

    let seo_keywords = keywords.join(", ");
    let img = article.effective_image_url().unwrap_or_default().to_string();

    SeoData {
        title: seo_title.clone(),
        description: seo_description.clone(),
        keywords: seo_keywords,
        og_title: seo_title.clone(),
        og_description: seo_description.clone(),
        twitter_title: seo_title,
        twitter_description: seo_description,
        image_url: img,
    }
}

/// Exact `isValidImageUrl` replica (`App.tsx` lines 132-146).
pub fn is_valid_image_url(url: Option<&str>) -> bool {
    let Some(raw) = url else { return false };
    let normalized = raw.trim().to_lowercase();
    if normalized.is_empty()
        || normalized.contains("placeholder.png")
        || normalized.contains("picsum.photos")
        || normalized == "null"
        || normalized == "undefined"
        || normalized.ends_with("/undefined")
        || normalized.ends_with("/null")
    {
        return false;
    }
    normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || normalized.starts_with("/uploads/")
        || normalized.starts_with("/images/")
}

/// Exact `getSiteName` domain badge helper.
pub fn get_site_name(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return "BİLİNMEYEN".to_string();
    }
    let after_scheme = trimmed
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or(trimmed);
    let host = after_scheme
        .split('/')
        .next()
        .unwrap_or(after_scheme)
        .split(':')
        .next()
        .unwrap_or(after_scheme)
        .trim_start_matches("www.");
    if host.contains('.') {
        let first = host.split('.').next().unwrap_or(host);
        if !first.is_empty() {
            return first.to_uppercase();
        }
    }
    trimmed.to_uppercase()
}

pub fn escape_html(s: &str) -> String {
    escape_html_basic(s)
}

/// Renders Markdown or HTML article content cleanly and safely into prose HTML.
pub fn render_safe_article_prose(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let looks_like_html = trimmed.contains("<p>")
        || trimmed.contains("<p ")
        || trimmed.contains("<h1")
        || trimmed.contains("<h2")
        || trimmed.contains("<h3")
        || trimmed.contains("<div")
        || trimmed.contains("<section")
        || trimmed.contains("<aside")
        || trimmed.contains("<br")
        || trimmed.contains("<figure")
        || trimmed.contains("<ul")
        || trimmed.contains("<ol")
        || trimmed.contains("<hr")
        || trimmed.contains("<pre")
        || trimmed.contains("<mark")
        || trimmed.contains("<blockquote")
        || trimmed.contains("hf-gallery")
        || trimmed.contains("hf-related-card")
        || trimmed.contains("hf-live-entry");

    let sanitized = if looks_like_html {
        sanitize_prose_html(trimmed)
    } else {
        let mut opts = MdOptions::empty();
        opts.insert(MdOptions::ENABLE_STRIKETHROUGH);
        let parser = MdParser::new_ext(trimmed, opts);
        let mut html_out = String::with_capacity(trimmed.len() + 128);
        md_html::push_html(&mut html_out, parser);
        sanitize_prose_html(&html_out)
    };

    let upgraded_figs = upgrade_wp_embed_figures(&sanitized);
    upgrade_standalone_embed_paragraphs(&upgraded_figs)
}

fn upgrade_standalone_embed_paragraphs(html: &str) -> String {
    if !html.contains("<p>") {
        return html.to_string();
    }
    let mut out = String::with_capacity(html.len() + 128);
    let mut rest = html;
    while let Some(start) = rest.find("<p>") {
        out.push_str(&rest[..start]);
        let after_p = &rest[start + 3..];
        let Some(end_p) = after_p.find("</p>") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let p_inner = &after_p[..end_p];
        let plain_cand = crate::editorial::common::strip_all_html_tags(p_inner);
        let unescaped = plain_cand.trim().replace("&amp;", "&");
        if let Some(embed_url) = crate::editorial::wasm::render_right::detect_standalone_media_embed_url(&unescaped)
        {
            out.push_str(&crate::editorial::wasm::render_right::render_embed_preview_card(
                &embed_url, None,
            ));
        } else {
            out.push_str("<p>");
            out.push_str(p_inner);
            out.push_str("</p>");
        }
        rest = &after_p[end_p + 4..];
    }
    out.push_str(rest);
    out
}

fn upgrade_wp_embed_figures(sanitized_html: &str) -> String {
    const OPEN_FIG: &str = "<figure class=\"wp-block-embed\">";
    const CLOSE_FIG: &str = "</figure>";

    if !sanitized_html.contains(OPEN_FIG) {
        return sanitized_html.to_string();
    }

    let mut out = String::with_capacity(sanitized_html.len() + 128);
    let mut rest = sanitized_html;
    while let Some(start) = rest.find(OPEN_FIG) {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + OPEN_FIG.len()..];
        let Some(end) = after_open.find(CLOSE_FIG) else {
            out.push_str(&rest[start..]);
            return out;
        };
        let fig_inner = &after_open[..end];
        let url = fig_inner
            .find("wp-block-embed__wrapper\">")
            .and_then(|p| {
                let r = &fig_inner[p + "wp-block-embed__wrapper\">".len()..];
                r.find("</div>").map(|e| r[..e].trim())
            })
            .unwrap_or_default();
        let cap = fig_inner.find("<figcaption>").and_then(|p| {
            let r = &fig_inner[p + "<figcaption>".len()..];
            r.find("</figcaption>").map(|e| r[..e].trim())
        });
        let unescaped_url = url.replace("&amp;", "&");
        out.push_str(&crate::editorial::wasm::render_right::render_embed_preview_card(
            &unescaped_url,
            cap,
        ));
        rest = &after_open[end + CLOSE_FIG.len()..];
    }
    out.push_str(rest);
    out
}

/// Whitelist HTML sanitizer allowing only safe editorial tags and safe `href`/`src` URLs.
pub fn sanitize_prose_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(rel_end) = input[i..].find('>') {
                let tag_raw = &input[i + 1..i + rel_end];
                let tag_trim = tag_raw.trim();

                // Strip dangerous block elements including their inner text: <script>, <style>, <iframe>, <object>
                let lower = tag_trim.to_ascii_lowercase();
                for forbidden in ["script", "style", "iframe", "object", "embed"] {
                    if lower == forbidden || lower.starts_with(&format!("{forbidden} ")) {
                        let close_pat = format!("</{forbidden}");
                        if let Some(close_pos) = input[i + rel_end + 1..]
                            .to_ascii_lowercase()
                            .find(&close_pat)
                        {
                            let after_close = i + rel_end + 1 + close_pos;
                            if let Some(gt) = input[after_close..].find('>') {
                                i = after_close + gt + 1;
                                continue;
                            }
                        }
                    }
                }
                if i >= bytes.len() {
                    break;
                }
                // Re-check if we jumped past forbidden block
                if bytes.get(i) != Some(&b'<')
                    || input[i..]
                        .find('>')
                        .map(|e| &input[i + 1..i + e] != tag_raw)
                        .unwrap_or(true)
                {
                    continue;
                }

                if let Some(safe_tag) = sanitize_single_tag(tag_trim) {
                    out.push_str(&safe_tag);
                }
                i += rel_end + 1;
                continue;
            }
        }

        // Safe UTF-8 slice up to next '<'
        let next_lt = input[i..].find('<').map(|p| i + p).unwrap_or(input.len());
        out.push_str(&input[i..next_lt]);
        i = next_lt;
    }

    out
}

fn sanitize_single_tag(tag_trim: &str) -> Option<String> {
    if tag_trim.starts_with('!') || tag_trim.starts_with('?') {
        return None;
    }
    let is_closing = tag_trim.starts_with('/');
    let content = if is_closing {
        tag_trim[1..].trim()
    } else {
        tag_trim.trim_end_matches('/').trim()
    };

    let (name_raw, attrs_raw) = match content.find(char::is_whitespace) {
        Some(idx) => (&content[..idx], content[idx..].trim()),
        None => (content, ""),
    };
    let name = name_raw.to_ascii_lowercase();

    const ALLOWED_SIMPLE: &[&str] = &[
        "p",
        "h1",
        "h2",
        "h3",
        "b",
        "em",
        "i",
        "u",
        "s",
        "strike",
        "del",
        "mark",
        "ul",
        "ol",
        "li",
        "blockquote",
        "cite",
        "figcaption",
        "pre",
        "code",
    ];

    if name == "br" || name == "hr" {
        return if is_closing {
            None
        } else {
            Some(format!("<{name} />"))
        };
    }

    if name == "h4" {
        if is_closing {
            return Some("</h4>".to_string());
        }
        let has_gal_heading = extract_attr_value(attrs_raw, "class")
            .map(|c| c.split_whitespace().any(|tok| tok == "hf-gallery-heading"))
            .unwrap_or(false);
        return if has_gal_heading {
            Some(r#"<h4 class="hf-gallery-heading">"#.to_string())
        } else {
            Some("<h4>".to_string())
        };
    }

    if name == "strong" {
        if is_closing {
            return Some("</strong>".to_string());
        }
        let has_live_title = extract_attr_value(attrs_raw, "class")
            .map(|c| c.split_whitespace().any(|tok| tok == "hf-live-title"))
            .unwrap_or(false);
        return if has_live_title {
            Some(r#"<strong class="hf-live-title">"#.to_string())
        } else {
            Some("<strong>".to_string())
        };
    }

    if name == "span" {
        if is_closing {
            return Some("</span>".to_string());
        }
        const SAFE_SPAN_CLASSES: &[&str] = &[
            "hf-live-time",
            "hf-related-kicker",
            "hf-related-title",
        ];
        let safe_cls: Vec<&str> = extract_attr_value(attrs_raw, "class")
            .map(|c| {
                c.split_whitespace()
                    .filter_map(|tok| SAFE_SPAN_CLASSES.iter().copied().find(|s| *s == tok))
                    .collect()
            })
            .unwrap_or_default();
        return if safe_cls.is_empty() {
            Some("<span>".to_string())
        } else {
            Some(format!(r#"<span class="{}">"#, safe_cls.join(" ")))
        };
    }

    if name == "section" {
        if is_closing {
            return Some("</section>".to_string());
        }
        let has_gal = extract_attr_value(attrs_raw, "class")
            .map(|c| c.split_whitespace().any(|tok| tok == "hf-gallery"))
            .unwrap_or(false);
        if has_gal {
            let gtitle_attr = extract_attr_value(attrs_raw, "data-gallery-title")
                .map(|t| format!(r#" data-gallery-title="{}""#, escape_html_basic(t.trim())))
                .unwrap_or_default();
            return Some(format!(r#"<section class="hf-gallery"{gtitle_attr}>"#));
        }
        return None;
    }

    if name == "aside" {
        if is_closing {
            return Some("</aside>".to_string());
        }
        let has_rel = extract_attr_value(attrs_raw, "class")
            .map(|c| c.split_whitespace().any(|tok| tok == "hf-related-card"))
            .unwrap_or(false);
        if has_rel {
            let mut attrs_out = String::from(r#" class="hf-related-card""#);
            if let Some(aid) = extract_attr_value(attrs_raw, "data-article-id") {
                attrs_out.push_str(&format!(
                    r#" data-article-id="{}""#,
                    escape_html_basic(aid.trim())
                ));
            }
            if let Some(slug) = extract_attr_value(attrs_raw, "data-slug") {
                attrs_out.push_str(&format!(
                    r#" data-slug="{}""#,
                    escape_html_basic(slug.trim())
                ));
            }
            return Some(format!("<aside{attrs_out}>"));
        }
        return None;
    }

    if name == "figure" {
        if is_closing {
            return Some("</figure>".to_string());
        }
        let cls_opt = extract_attr_value(attrs_raw, "class");
        let has_wp_embed = cls_opt
            .as_deref()
            .map(|c| c.split_whitespace().any(|tok| tok == "wp-block-embed"))
            .unwrap_or(false);
        if has_wp_embed {
            return Some(r#"<figure class="wp-block-embed">"#.to_string());
        }
        let has_gal_item = cls_opt
            .as_deref()
            .map(|c| c.split_whitespace().any(|tok| tok == "hf-gallery-item"))
            .unwrap_or(false);
        if has_gal_item {
            let idx_attr = extract_attr_value(attrs_raw, "data-index")
                .and_then(|v| v.trim().parse::<usize>().ok())
                .map(|n| format!(r#" data-index="{n}""#))
                .unwrap_or_default();
            return Some(format!(r#"<figure class="hf-gallery-item"{idx_attr}>"#));
        }
        return Some("<figure>".to_string());
    }

    if name == "div" {
        if is_closing {
            return Some("</div>".to_string());
        }
        const SAFE_DIV_CLASSES: &[&str] = &[
            "hf-callout",
            "hf-callout-title",
            "hf-callout-body",
            "hf-summary",
            "wp-block-embed__wrapper",
            "hf-gallery-grid",
            "hf-related-meta",
            "hf-live-entry",
            "hf-live-head",
            "hf-live-body",
        ];
        let safe_cls: Vec<&str> = extract_attr_value(attrs_raw, "class")
            .map(|c| {
                c.split_whitespace()
                    .filter_map(|tok| SAFE_DIV_CLASSES.iter().copied().find(|s| *s == tok))
                    .collect()
            })
            .unwrap_or_default();
        let safe_variant = extract_attr_value(attrs_raw, "data-variant").and_then(|v| {
            match v.trim() {
                "info" => Some("info"),
                "breaking" => Some("breaking"),
                "warning" => Some("warning"),
                _ => None,
            }
        });
        let mut attrs_out = String::new();
        if !safe_cls.is_empty() {
            attrs_out.push_str(&format!(r#" class="{}""#, safe_cls.join(" ")));
        }
        if let Some(var) = safe_variant {
            attrs_out.push_str(&format!(r#" data-variant="{var}""#));
        }
        if safe_cls.contains(&"hf-live-entry") {
            if let Some(t) = extract_attr_value(attrs_raw, "data-time") {
                attrs_out.push_str(&format!(r#" data-time="{}""#, escape_html_basic(t.trim())));
            }
        }
        return Some(format!("<div{attrs_out}>"));
    }

    if ALLOWED_SIMPLE.contains(&name.as_str()) {
        return if is_closing {
            Some(format!("</{name}>"))
        } else {
            Some(format!("<{name}>"))
        };
    }

    if name == "a" {
        if is_closing {
            return Some("</a>".to_string());
        }
        if let Some(href) = extract_attr_value(attrs_raw, "href") {
            let ht = href.trim();
            let hl = ht.to_ascii_lowercase();
            let is_rel_link = extract_attr_value(attrs_raw, "class")
                .map(|c| c.split_whitespace().any(|tok| tok == "hf-related-link"))
                .unwrap_or(false);
            let cls_attr = if is_rel_link {
                r#" class="hf-related-link""#
            } else {
                ""
            };
            if hl.starts_with("http://") || hl.starts_with("https://") {
                return Some(format!(
                    r#"<a href="{}"{cls_attr} target="_blank" rel="noopener noreferrer">"#,
                    escape_html_basic(ht)
                ));
            }
            if ht.starts_with("/icerik/") || ht.starts_with("/haber/") || ht == "#" {
                return Some(format!(
                    r#"<a href="{}"{cls_attr}>"#,
                    escape_html_basic(ht)
                ));
            }
        }
        return None;
    }

    if name == "img" && !is_closing {
        if let Some(src) = extract_attr_value(attrs_raw, "src") {
            let sl = src.trim().to_ascii_lowercase();
            if sl.starts_with("http://")
                || sl.starts_with("https://")
                || sl.starts_with("/uploads/")
                || sl.starts_with("/images/")
            {
                let alt = extract_attr_value(attrs_raw, "alt").unwrap_or_default();
                let is_rel_thumb = extract_attr_value(attrs_raw, "class")
                    .map(|c| c.split_whitespace().any(|tok| tok == "hf-related-thumb"))
                    .unwrap_or(false);
                let cls_attr = if is_rel_thumb {
                    r#" class="hf-related-thumb""#
                } else {
                    ""
                };
                let thumb_attr = extract_attr_value(attrs_raw, "data-thumb")
                    .filter(|t| {
                        let tl = t.trim().to_ascii_lowercase();
                        tl.starts_with("http://")
                            || tl.starts_with("https://")
                            || tl.starts_with("/uploads/")
                            || tl.starts_with("/images/")
                    })
                    .map(|t| format!(r#" data-thumb="{}""#, escape_html_basic(t.trim())))
                    .unwrap_or_default();
                return Some(format!(
                    r#"<img src="{}"{thumb_attr} alt="{}"{cls_attr} referrerpolicy="no-referrer" />"#,
                    escape_html_basic(src.trim()),
                    escape_html_basic(&alt)
                ));
            }
        }
        return None;
    }

    None
}

fn extract_attr_value(attrs: &str, target: &str) -> Option<String> {
    let lower = attrs.to_ascii_lowercase();
    let pat_dq = format!("{target}=\"");
    if let Some(pos) = lower.find(&pat_dq) {
        let rest = &attrs[pos + pat_dq.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    let pat_sq = format!("{target}='");
    if let Some(pos) = lower.find(&pat_sq) {
        let rest = &attrs[pos + pat_sq.len()..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

// ============================================================================
// Browser Date/Time & Storage Utilities (WASM + Host-Test Safe)
// ============================================================================

#[cfg(target_arch = "wasm32")]
pub fn js_now_ms() -> u64 {
    js_sys::Date::now() as u64
}

#[cfg(not(target_arch = "wasm32"))]
pub fn js_now_ms() -> u64 {
    1_700_000_000_000
}

#[cfg(target_arch = "wasm32")]
pub fn js_now_hhmm() -> String {
    let d = js_sys::Date::new_0();
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn js_now_hhmm() -> String {
    "12:00".to_string()
}

pub fn format_bytes_human(size_bytes: i64) -> String {
    if size_bytes <= 0 {
        return "0 KB".to_string();
    }
    if size_bytes < 1024 {
        return format!("{size_bytes} B");
    }
    let kb = (size_bytes as f64) / 1024.0;
    if kb < 1024.0 {
        return format!("{:.0} KB", kb.max(1.0));
    }
    let mb = kb / 1024.0;
    format!("{mb:.1} MB")
}

#[cfg(target_arch = "wasm32")]
pub fn js_now_iso() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn js_now_iso() -> String {
    "2026-11-16T12:00:00.000Z".to_string()
}

#[cfg(target_arch = "wasm32")]
fn parse_js_date(raw: &str) -> Option<js_sys::Date> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = if trimmed.len() >= 19 && trimmed.as_bytes().get(10) == Some(&b' ') {
        trimmed.replacen(' ', "T", 1)
    } else {
        trimmed.to_string()
    };
    let d = js_sys::Date::new(&JsValue::from_str(&normalized));
    if d.get_time().is_nan() {
        None
    } else {
        Some(d)
    }
}

/// Formats `dd.mm HH:MM` for left panel cards.
#[cfg(target_arch = "wasm32")]
pub fn format_card_datetime(raw: Option<&str>, fallback: &str) -> String {
    let Some(s) = raw.filter(|s| !s.trim().is_empty()) else {
        return fallback.to_string();
    };
    let Some(d) = parse_js_date(s) else {
        return fallback.to_string();
    };
    format!(
        "{:02}.{:02} {:02}:{:02}",
        d.get_date(),
        d.get_month() + 1,
        d.get_hours(),
        d.get_minutes()
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn format_card_datetime(raw: Option<&str>, fallback: &str) -> String {
    raw.filter(|s| !s.trim().is_empty())
        .map(|s| s.chars().take(16).collect())
        .unwrap_or_else(|| fallback.to_string())
}

/// Formats `dd Month YYYY HH:MM` in Turkish for Middle Reader.
#[cfg(target_arch = "wasm32")]
pub fn format_middle_turkish_date(raw: Option<&str>) -> Option<String> {
    const TR_MONTHS: [&str; 12] = [
        "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
        "Kasım", "Aralık",
    ];
    let s = raw.filter(|s| !s.trim().is_empty())?;
    let d = parse_js_date(s)?;
    let m_idx = (d.get_month() as usize).min(11);
    Some(format!(
        "{:02} {} {} {:02}:{:02}",
        d.get_date(),
        TR_MONTHS[m_idx],
        d.get_full_year(),
        d.get_hours(),
        d.get_minutes()
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn format_middle_turkish_date(raw: Option<&str>) -> Option<String> {
    raw.filter(|s| !s.trim().is_empty())
        .map(|_| "16 Kasım 2026 12:00".to_string())
}

/// Converts ISO string to `YYYY-MM-DDTHH:MM` for `<input type="datetime-local">`.
#[cfg(target_arch = "wasm32")]
pub fn iso_to_datetime_local(iso: Option<&str>) -> String {
    let Some(s) = iso.filter(|s| !s.trim().is_empty()) else {
        return String::new();
    };
    let Some(d) = parse_js_date(s) else {
        return String::new();
    };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date(),
        d.get_hours(),
        d.get_minutes()
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn iso_to_datetime_local(iso: Option<&str>) -> String {
    iso.unwrap_or_default().chars().take(16).collect()
}

#[cfg(target_arch = "wasm32")]
pub fn datetime_local_to_iso(val: &str) -> Option<String> {
    if val.trim().is_empty() {
        return None;
    }
    let d = js_sys::Date::new(&JsValue::from_str(val.trim()));
    if d.get_time().is_nan() {
        None
    } else {
        d.to_iso_string().as_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn datetime_local_to_iso(val: &str) -> Option<String> {
    if val.trim().is_empty() {
        None
    } else {
        Some(format!("{}:00.000Z", val.trim()))
    }
}

// ============================================================================
// localStorage Persistence
// ============================================================================

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[cfg(not(target_arch = "wasm32"))]
fn local_storage() -> Option<web_sys::Storage> {
    None
}

pub fn storage_get(key: &str) -> Option<String> {
    local_storage()?.get_item(key).ok()?
}

pub fn storage_set(key: &str, value: &str) {
    if let Some(ls) = local_storage() {
        let _ = ls.set_item(key, value);
    }
}

pub fn storage_remove(key: &str) {
    if let Some(ls) = local_storage() {
        let _ = ls.remove_item(key);
    }
}

pub fn load_panel_widths() -> PanelWidths {
    let raw = storage_get("msi-editorial-panels-v1");
    if let Some(s) = raw {
        if let Ok(parsed) = serde_json::from_str::<PanelWidths>(&s) {
            let left = parsed.left.clamp(15.0, 55.0);
            let right = parsed.right.clamp(25.0, 55.0);
            let middle = 100.0 - left - right;
            if middle >= 30.0 {
                return PanelWidths {
                    left,
                    middle,
                    right,
                };
            }
        }
    }
    PanelWidths::default()
}

pub fn save_panel_widths(widths: &PanelWidths) {
    if let Ok(json) = serde_json::to_string(widths) {
        storage_set("msi-editorial-panels-v1", &json);
    }
}

pub fn load_editing_draft() -> Article {
    if let Some(s) = storage_get("msi-editorial-editing-draft") {
        if let Ok(art) = serde_json::from_str::<Article>(&s) {
            return normalize_draft_for_editor(art);
        }
    }
    empty_editing_article()
}

pub fn save_editing_draft(article: &Article) {
    if let Ok(json) = serde_json::to_string(article) {
        storage_set("msi-editorial-editing-draft", &json);
    }
}

pub fn load_editing_baseline_sig() -> String {
    storage_get("msi-editorial-editing-baseline-sig").unwrap_or_default()
}

pub fn save_editing_baseline_sig(sig: &str) {
    if sig.is_empty() {
        storage_remove("msi-editorial-editing-baseline-sig");
    } else {
        storage_set("msi-editorial-editing-baseline-sig", sig);
    }
}

pub fn save_stashed_tr_buffer(lang: &str, buf: &StashedTrBuffer) {
    storage_set("msi-editorial-editor-lang", lang);
    if let Ok(json) = serde_json::to_string(buf) {
        storage_set("msi-editorial-stashed-tr", &json);
    }
}

pub fn load_stashed_tr_buffer() -> Option<(String, StashedTrBuffer)> {
    let lang = storage_get("msi-editorial-editor-lang")?;
    if lang != "en" {
        return None;
    }
    let raw = storage_get("msi-editorial-stashed-tr")?;
    let parsed = serde_json::from_str::<StashedTrBuffer>(&raw).ok()?;
    Some((lang, parsed))
}

pub fn clear_stashed_tr_buffer() {
    storage_remove("msi-editorial-editor-lang");
    storage_remove("msi-editorial-stashed-tr");
}

pub fn clear_editing_draft() {
    storage_remove("msi-editorial-editing-draft");
    storage_remove("msi-editorial-editing-baseline-sig");
    clear_stashed_tr_buffer();
}

pub fn load_news_view_mode() -> NewsViewMode {
    storage_get("news_view_mode")
        .map(|s| NewsViewMode::from_str(&s))
        .unwrap_or(NewsViewMode::Grid)
}

pub fn save_news_view_mode(mode: NewsViewMode) {
    storage_set("news_view_mode", mode.as_str());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bilingual_active_buffer_swaps_and_canonicalizes_cleanly() {
        let mut s = AppState::new();
        s.glossary_rules = vec![
            GlossaryRule {
                id: 1,
                term_tr: "Savunma Sanayii Başkanlığı".into(),
                term_en: "Presidency of Defence Industries (SSB)".into(),
                abbr_tr: Some("SSB".into()),
                abbr_en: Some("SSB".into()),
                category: "institutional".into(),
                rule_type: "fixed".into(),
                preferred_en: "Presidency of Defence Industries (SSB)".into(),
                forbidden_variants: vec!["SSM".into()],
                case_sensitive: false,
                note: None,
            },
            GlossaryRule {
                id: 2,
                term_tr: "Türkiye".into(),
                term_en: "Türkiye".into(),
                abbr_tr: None,
                abbr_en: None,
                category: "style".into(),
                rule_type: "fixed".into(),
                preferred_en: "Türkiye".into(),
                forbidden_variants: vec!["Turkey".into()],
                case_sensitive: true,
                note: None,
            },
        ];

        s.editing_article.id = "draft-286".into();
        s.editing_article.title = "Savunma Sanayii Başkanlığı Yeni Projeyi Açıkladı".into();
        s.editing_article.spot = Some("SSB koordinasyonunda yeni sistem teslim edildi.".into());
        s.editing_article.blocks = Some(vec![
            ContentBlock {
                id: "b1".into(),
                block_type: "paragraph".into(),
                content: "Türkiye ve SSB ortaklığı büyüyor.".into(),
                meta: None,
            },
            ContentBlock {
                id: "b2".into(),
                block_type: "image".into(),
                content: "/uploads/kaan.webp".into(),
                meta: Some("SSB teslimat töreni".into()),
            },
        ]);
        let baseline = editor_article_signature(&s.canonical_editing_article());
        s.editing_baseline_sig = baseline;

        // Switching to EN without editing keeps canonical article & dirty signature unchanged!
        assert!(s.switch_editor_lang("en"));
        assert_eq!(s.editor_lang, "en");
        assert!(!should_confirm_editor_discard(&s));

        // Assisted copy from TR -> EN
        let copied_blocks = s.copy_tr_to_en_with_glossary_assist();
        assert_eq!(copied_blocks, 2);
        assert_eq!(s.editing_article.en_status, "draft");
        assert!(s
            .editing_article
            .title
            .contains("Presidency of Defence Industries (SSB)"));
        assert_eq!(
            s.editing_article.blocks.as_ref().unwrap()[1].content,
            "/uploads/kaan.webp"
        );

        // Introduce a forbidden variant ("Turkey" and "SSM") and verify live compliance + 1-click fix
        s.editing_article.blocks.as_mut().unwrap()[0].content =
            "Turkey and SSM signed a contract.".into();
        let (violations, has_txt) = s.evaluate_active_en_glossary();
        assert!(has_txt);
        assert_eq!(violations.len(), 2);

        let fixes = s.apply_active_en_glossary_fixes();
        assert_eq!(fixes, 2);
        let (violations_after, _) = s.evaluate_active_en_glossary();
        assert!(violations_after.is_empty());

        // Canonical snapshot while STILL in EN mode preserves TR in `title` and EN in `title_en`!
        let canon = s.canonical_editing_article();
        assert_eq!(
            canon.title,
            "Savunma Sanayii Başkanlığı Yeni Projeyi Açıkladı"
        );
        assert!(canon
            .title_en
            .as_deref()
            .unwrap_or("")
            .contains("Presidency of Defence Industries (SSB)"));
        assert!(canon
            .content_en
            .as_deref()
            .unwrap_or("")
            .contains("Türkiye and Presidency of Defence Industries (SSB) signed a contract."));

        // Switch back to TR
        assert!(s.switch_editor_lang("tr"));
        assert_eq!(s.editor_lang, "tr");
        assert_eq!(
            s.editing_article.title,
            "Savunma Sanayii Başkanlığı Yeni Projeyi Açıkladı"
        );
        assert!(article_has_english(&s.editing_article));
    }

    #[test]
    fn defaults_to_published_left_tab_and_categories_settings_popover_closed() {
        let s = AppState::new();
        assert_eq!(s.left_tab, LeftTab::Published);
        assert!(!s.show_settings_popover);
        assert_eq!(s.settings_tab, SettingsTab::Categories);
        assert!(s.selected_article.is_none());
        assert!(!is_editor_populated(&s.editing_article));

        let mut filled = empty_editing_article();
        filled.blocks.as_mut().unwrap()[0].content = "İlk paragraf metni".to_string();
        assert!(is_editor_populated(&filled));

        // Saved draft opened in editor WITHOUT edits -> should_confirm_editor_discard is FALSE!
        let mut s_draft = AppState::new();
        let saved_draft = Article {
            id: "draft-99".to_string(),
            article_type: Some("draft".to_string()),
            title: "Kayıtlı Taslak Başlığı".to_string(),
            blocks: Some(vec![ContentBlock {
                id: "b1".to_string(),
                block_type: "paragraph".to_string(),
                content: "Taslak metni".to_string(),
                meta: None,
            }]),
            ..Default::default()
        };
        s_draft.editing_article = saved_draft.clone();
        s_draft.editing_baseline_sig = editor_article_signature(&saved_draft);
        assert!(!should_confirm_editor_discard(&s_draft));

        // Modifying title of that saved draft -> should_confirm_editor_discard becomes TRUE!
        s_draft.editing_article.title = "Değiştirilmiş Başlık".to_string();
        assert!(should_confirm_editor_discard(&s_draft));
    }

    #[test]
    fn builds_local_auto_seo_accurately_with_turkish_text() {
        let art = Article {
            title: "İstanbul Ekonomi Zirvesi'nde Dijital Dönüşüm ve Sürdürülebilir Enerji Yatırımları Konuşuldu".to_string(),
            blocks: Some(vec![ContentBlock {
                id: "b1".to_string(),
                block_type: "paragraph".to_string(),
                content: "Türkiye'nin önde gelen sanayi kuruluşları ve teknoloji liderleri İstanbul'da düzenlenen zirvede bir araya gelerek 2027 yatırım yol haritasını kamuoyuyla paylaştı ve önemli iş birliklerine imza attı.".to_string(),
                meta: None,
            }]),
            image_url: Some("/uploads/zirve.jpg".to_string()),
            ..Default::default()
        };

        let seo = build_auto_seo(&art);
        assert!(seo.title.chars().count() <= 60);
        assert!(seo.title.starts_with("İstanbul Ekonomi Zirvesi"));
        assert!(seo.description.chars().count() <= 150);
        assert!(seo.description.starts_with("Türkiye'nin önde gelen"));
        assert!(seo.keywords.contains("istanbul"));
        assert!(seo.keywords.contains("ekonomi"));
        assert_eq!(seo.image_url, "/uploads/zirve.jpg");
    }

    #[test]
    fn validates_image_urls_like_legacy_app() {
        assert!(!is_valid_image_url(None));
        assert!(!is_valid_image_url(Some("")));
        assert!(!is_valid_image_url(Some("null")));
        assert!(!is_valid_image_url(Some("undefined")));
        assert!(!is_valid_image_url(Some("https://picsum.photos/200")));
        assert!(!is_valid_image_url(Some("/images/placeholder.png")));
        assert!(is_valid_image_url(Some("https://cdn.example.com/news.jpg")));
        assert!(is_valid_image_url(Some("/uploads/cropped-123.jpg")));
        assert!(is_valid_image_url(Some("/images/msi/haber.webp")));
    }

    #[test]
    fn extracts_site_domain_badge_accurately() {
        assert_eq!(get_site_name(""), "BİLİNMEYEN");
        assert_eq!(get_site_name("https://www.ntv.com.tr/ekonomi/haber"), "NTV");
        assert_eq!(get_site_name("https://bbc.co.uk/news"), "BBC");
    }

    #[test]
    fn sanitizes_html_and_renders_markdown_prose() {
        let md = "## Önemli Başlık\n\nBu **kalın** bir metindir.";
        let rendered = render_safe_article_prose(md);
        assert!(rendered.contains("<h2>Önemli Başlık</h2>"));
        assert!(rendered.contains("<strong>kalın</strong>"));

        let dirty_html =
            r#"<p onclick="evil()">Temiz <mark>vurgulu</mark> ve <u>altı çizili</u> paragraf</p><script>alert(1)</script><h2>Alt Başlık</h2><div class="hf-callout" data-variant="breaking" onclick="bad()"><div class="hf-callout-title">SON DAKİKA</div><div class="hf-callout-body">Önemli gelişme</div></div><div class="hf-summary"><h4>Öne Çıkanlar</h4><ul><li>Madde 1</li></ul></div><figure class="wp-block-embed"><div class="wp-block-embed__wrapper">https://youtu.be/dQw4w9WgXcQ</div><figcaption>Video</figcaption></figure>"#;
        let clean = render_safe_article_prose(dirty_html);
        assert!(clean.contains("<p>Temiz <mark>vurgulu</mark> ve <u>altı çizili</u> paragraf</p>"));
        assert!(clean.contains("<h2>Alt Başlık</h2>"));
        assert!(clean.contains(r#"<div class="hf-callout" data-variant="breaking">"#));
        assert!(clean.contains(r#"<div class="hf-summary">"#));
        assert!(clean.contains("https://www.youtube.com/embed/dQw4w9WgXcQ?rel=0"));
        assert!(!clean.contains("script"));
        assert!(!clean.contains("onclick"));
    }

    #[test]
    fn preserves_gallery_related_card_and_live_entry_blocks_while_stripping_xss() {
        use crate::editorial::common::{blocks_to_wp_html, GalleryItem, RelatedArticleRef};

        let gal_json = serde_json::to_string(&vec![
            GalleryItem {
                url: "/uploads/gal-1.webp".into(),
                thumb_url: Some("/uploads/gal-1-thumb.webp".into()),
                caption: "İlk kare".into(),
            },
            GalleryItem {
                url: "/uploads/gal-2.webp".into(),
                thumb_url: None,
                caption: "İkinci kare".into(),
            },
        ])
        .unwrap();

        let rel_json = serde_json::to_string(&RelatedArticleRef {
            article_id: "pub-99".into(),
            title: "Merkez Bankası Enflasyon Raporu".into(),
            slug: Some("merkez-bankasi-enflasyon-raporu".into()),
            image_url: Some("/uploads/mb-thumb.webp".into()),
        })
        .unwrap();

        let wp_html = blocks_to_wp_html(&[
            ContentBlock {
                id: "l1".into(),
                block_type: "live_entry".into(),
                content: "Toplantı başladı <script>alert(1)</script> ve <strong>ilk karar</strong> açıklandı.".into(),
                meta: Some("14:35|Kritik Gelişme".into()),
            },
            ContentBlock {
                id: "g1".into(),
                block_type: "gallery".into(),
                content: gal_json,
                meta: Some("Olay Yerinden Kareler".into()),
            },
            ContentBlock {
                id: "r1".into(),
                block_type: "related_article".into(),
                content: rel_json,
                meta: Some("BUNU DA OKUYUN".into()),
            },
        ]);

        let malicious_injected = format!(
            r#"{wp_html}<script>evil_raw_payload()</script><aside class="hf-related-card" onclick="steal()"><a href="javascript:alert(1)" class="hf-related-link">Bad</a></aside>"#
        );
        let safe = render_safe_article_prose(&malicious_injected);

        // Live entry preserved
        assert!(safe.contains(r#"<div class="hf-live-entry" data-time="14:35">"#));
        assert!(safe.contains(r#"<span class="hf-live-time">14:35</span>"#));
        assert!(safe.contains(r#"<strong class="hf-live-title">Kritik Gelişme</strong>"#));
        assert!(safe.contains("<strong>ilk karar</strong>"));

        // Gallery preserved
        assert!(safe.contains(r#"<section class="hf-gallery" data-gallery-title="Olay Yerinden Kareler">"#));
        assert!(safe.contains(r#"<h4 class="hf-gallery-heading">Olay Yerinden Kareler</h4>"#));
        assert!(safe.contains(r#"<figure class="hf-gallery-item" data-index="1">"#));
        assert!(safe.contains("/uploads/gal-1.webp"));
        assert!(safe.contains("<figcaption>İlk kare</figcaption>"));

        // Related article card preserved
        assert!(safe.contains(r#"<aside class="hf-related-card" data-article-id="pub-99" data-slug="merkez-bankasi-enflasyon-raporu">"#));
        assert!(safe.contains(r#"class="hf-related-thumb""#));
        assert!(safe.contains(r#"<span class="hf-related-kicker">BUNU DA OKUYUN</span>"#));
        assert!(safe.contains(r#"<a href="/icerik/merkez-bankasi-enflasyon-raporu" class="hf-related-link">"#));
        assert!(safe.contains(r#"<span class="hf-related-title">Merkez Bankası Enflasyon Raporu</span>"#));

        // XSS stripped
        assert!(!safe.contains("<script"));
        assert!(!safe.contains("evil_raw_payload"));
        assert!(!safe.contains("onclick"));
        assert!(!safe.contains("javascript:"));
    }
}
