//! Pure-Rust async HTTP client (`fetch`), file/crop uploads, MSI Editorial Studio API & Cookie+CSRF Auth.

use crate::editorial::common::{
    blocks_to_wp_html, parse_content_to_blocks, slugify_turkish, Article, ArticleLock,
    ArticleRevision, Category, ContentBlock, GalleryItem, GlossaryRule, MagazineIssueItem,
    MediaAsset, SsbTag, UrlRedirect, User, WordPressConfig,
};
use serde_json::{json, Value};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    Blob, BlobPropertyBag, File, FormData, Headers, HtmlDocument, HtmlInputElement, Request,
    RequestCredentials, RequestInit, Response,
};

use crate::editorial::wasm::cropper::{decode_base64, export_cropped_jpeg_blob};
use crate::editorial::wasm::state::{
    admin_path, build_auto_seo, clear_editing_draft, editor_article_signature,
    empty_editing_article, is_valid_image_url, js_now_iso, js_now_ms, normalize_draft_for_editor,
    save_editing_baseline_sig, save_editing_draft, set_article_image,
    should_confirm_editor_discard, storage_remove, with_state, with_state_mut, LeftTab,
    MobileView, PendingEditorConfirm, TagSuggestion,
};

pub fn set_timeout_ms<F: FnOnce() + 'static>(ms: i32, f: F) {
    if let Some(win) = web_sys::window() {
        let cb = Closure::once(f);
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            ms,
        );
        cb.forget();
    }
}

pub fn confirm_dialog(msg: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.confirm_with_message(msg).ok())
        .unwrap_or(true)
}

pub fn show_error(msg: impl Into<String>) {
    let text = msg.into();
    let seq = with_state_mut(|s| {
        s.error_seq += 1;
        s.error = Some(text);
        s.error_seq
    });
    crate::editorial::wasm::render::refresh_toasts();
    set_timeout_ms(2600, move || {
        let changed = with_state_mut(|s| {
            if s.error_seq == seq {
                s.error = None;
                true
            } else {
                false
            }
        });
        if changed {
            crate::editorial::wasm::render::refresh_toasts();
        }
    });
}

pub fn show_success(msg: impl Into<String>) {
    let text = msg.into();
    let seq = with_state_mut(|s| {
        s.success_seq += 1;
        s.success = Some(text);
        s.success_seq
    });
    crate::editorial::wasm::render::refresh_toasts();
    set_timeout_ms(2600, move || {
        let changed = with_state_mut(|s| {
            if s.success_seq == seq {
                s.success = None;
                true
            } else {
                false
            }
        });
        if changed {
            crate::editorial::wasm::render::refresh_toasts();
        }
    });
}

fn read_cookie(name: &str) -> Option<String> {
    let doc = web_sys::window()?.document()?.dyn_into::<HtmlDocument>().ok()?;
    let raw = doc.cookie().ok()?;
    let prefix = format!("{name}=");
    for part in raw.split(';') {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix(&prefix) {
            let clean = js_sys::decode_uri_component(val)
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| val.to_string());
            if !clean.is_empty() {
                return Some(clean);
            }
        }
    }
    None
}

fn is_mutating_method(method: &str) -> bool {
    !matches!(
        method.trim().to_ascii_uppercase().as_str(),
        "GET" | "HEAD" | "OPTIONS"
    )
}

fn parse_content_id_from_query() -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    let query = search.strip_prefix('?').unwrap_or(&search);
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == "content_id" || k == "id" {
                let decoded = js_sys::decode_uri_component(v)
                    .ok()
                    .and_then(|s| s.as_string())
                    .unwrap_or_else(|| v.to_string());
                let trimmed = decoded.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
        }
    }
    None
}

async fn api_json(
    method: &str,
    url: &str,
    _token: Option<&str>,
    body: Option<&Value>,
) -> Result<(bool, u16, Value), String> {
    let win = web_sys::window().ok_or_else(|| "Tarayıcı penceresi bulunamadı.".to_string())?;
    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_credentials(RequestCredentials::SameOrigin);

    let headers = Headers::new().map_err(|_| "HTTP başlıkları oluşturulamadı.".to_string())?;
    if is_mutating_method(method) {
        if let Some(csrf) = read_cookie("csrf_token") {
            let _ = headers.set("X-CSRF-Token", &csrf);
        }
    }
    if let Some(b) = body {
        let _ = headers.set("Content-Type", "application/json");
        let serialized = serde_json::to_string(b).unwrap_or_else(|_| "{}".to_string());
        opts.set_body(&JsValue::from_str(&serialized));
    }
    opts.set_headers(&headers);

    let req = Request::new_with_str_and_init(url, &opts)
        .map_err(|_| "İstek oluşturulamadı.".to_string())?;
    let resp_val = JsFuture::from(win.fetch_with_request(&req))
        .await
        .map_err(|_| "Sunucuya bağlanılamadı. Lütfen tekrar deneyin.".to_string())?;
    let resp: Response = resp_val
        .dyn_into()
        .map_err(|_| "Geçersiz sunucu yanıtı.".to_string())?;

    let ok = resp.ok();
    let status = resp.status();
    if status == 401 {
        let _ = win.location().set_href(&admin_path());
    }
    let text_promise = resp
        .text()
        .map_err(|_| "Yanıt gövdesi okunamadı.".to_string())?;
    let text_val = JsFuture::from(text_promise)
        .await
        .map_err(|_| "Yanıt metni alınamadı.".to_string())?;
    let text = text_val.as_string().unwrap_or_default();
    let mut parsed = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({}));
    if !ok && parsed.get("error").is_none() {
        if let Some(msg) = parsed.get("message").and_then(|v| v.as_str()).map(str::to_string) {
            if let Some(obj) = parsed.as_object_mut() {
                obj.insert("error".to_string(), Value::String(msg));
            }
        }
    }

    Ok((ok, status, parsed))
}

fn encode_param(val: &str) -> String {
    js_sys::encode_uri_component(val)
        .as_string()
        .unwrap_or_default()
}

// ============================================================================
// Cookie Session Boot & Deep-Link / Logout
// ============================================================================

pub fn init_auth_session() {
    storage_remove("token");

    spawn_local(async move {
        match api_json("GET", "/api/v1/admin/editorial/session", None, None).await {
            Ok((ok, _, data)) if ok && data.get("authenticated").and_then(|v| v.as_bool()) == Some(true) => {
                let maybe_user = data
                    .get("user")
                    .and_then(|u| serde_json::from_value::<User>(u.clone()).ok());
                if let Some(user) = maybe_user {
                    with_state_mut(|s| {
                        s.token = Some("__cookie_session__".to_string());
                        s.user = Some(user);
                        s.auth_loading = false;
                    });
                    crate::editorial::wasm::render::render_full();
                    bootstrap_authenticated_data();
                    if let Some(deep_id) = parse_content_id_from_query() {
                        load_article_into_editor_by_id(deep_id);
                    }
                    return;
                }
            }
            _ => {}
        }

        with_state_mut(|s| {
            s.token = None;
            s.user = None;
            s.auth_loading = false;
            s.login_error = "Oturumunuz sona erdi veya bu ekran için yetkiniz bulunmuyor.".to_string();
        });
        if let Some(win) = web_sys::window() {
            let _ = win.location().set_href(&admin_path());
        }
        crate::editorial::wasm::render::render_full();
    });
}

fn load_article_into_editor_by_id(content_id: String) {
    let clean = content_id.trim().to_string();
    if clean.is_empty() {
        return;
    }
    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/articles/{}", encode_param(&clean));
        if let Ok((true, _, data)) = api_json("GET", &url, None, None).await {
            if let Some(art) = data
                .get("article")
                .and_then(|v| serde_json::from_value::<Article>(v.clone()).ok())
            {
                with_state_mut(|s| {
                    s.selected_article = Some(art.clone());
                    s.middle_showing_research = false;
                    s.middle_preview_draft = false;
                    s.editing_article = normalize_draft_for_editor(art.clone());
                    s.reset_editor_lang_buffers();
                    let baseline = editor_article_signature(&s.canonical_editing_article());
                    save_editing_baseline_sig(&baseline);
                    s.editing_baseline_sig = baseline;
                    s.editor_step = 1;
                });
                save_editing_draft(&art);
                acquire_article_lock(&art.id);
                crate::editorial::wasm::render::render_full();
            }
        }
    });
}

pub fn submit_login() {
    if let Some(win) = web_sys::window() {
        let _ = win.location().set_href(&admin_path());
    }
}

pub fn perform_logout() {
    let old_id = with_state(|s| s.editing_article.id.trim().to_string());
    spawn_local(async move {
        if !old_id.is_empty() {
            let lock_url = format!("/api/v1/admin/editorial/articles/{}/lock", encode_param(&old_id));
            let _ = api_json("DELETE", &lock_url, None, None).await;
        }
        let _ = api_json("POST", "/api/v1/auth/logout", None, Some(&json!({}))).await;
        if let Some(win) = web_sys::window() {
            let _ = win.location().set_href(&admin_path());
        }
    });

    storage_remove("token");
    with_state_mut(|s| {
        s.lock_timer_seq += 1;
        s.token = None;
        s.user = None;
        s.is_profile_dropdown_open = false;
        s.show_settings_popover = false;
        s.show_media_modal = false;
        s.show_redirects_modal = false;
        s.show_revisions_modal = false;
        s.is_adding_user = false;
        s.show_change_password = false;
        s.active_locks.clear();
        s.editing_conflict_lock = None;
        s.published_articles.clear();
        s.published_total = 0;
        s.draft_articles.clear();
        s.draft_total = 0;
        s.selected_article = None;
        s.research_results.clear();
        s.research_tabs.clear();
        s.active_research_url = None;
        s.middle_showing_research = false;
        s.research_loaded_once = false;
        s.is_searching_research = false;
        s.ssb_tags_catalog.clear();
        s.magazine_issues.clear();
        s.glossary_rules.clear();
        s.preview_lang = "tr".to_string();
    });
    crate::editorial::wasm::render::render_full();
}

pub fn submit_change_password() {
    let (token, cur, new_pw, confirm_pw, busy) = with_state(|s| {
        (
            s.token.clone(),
            s.change_pw_current.clone(),
            s.change_pw_new.clone(),
            s.change_pw_confirm.clone(),
            s.is_changing_password,
        )
    });
    if busy {
        return;
    }
    let Some(tok) = token else { return };

    if new_pw.len() < 6 {
        show_error("Yeni şifre en az 6 karakter olmalı.");
        return;
    }
    if new_pw != confirm_pw {
        show_error("Yeni şifreler eşleşmiyor.");
        return;
    }

    with_state_mut(|s| s.is_changing_password = true);
    crate::editorial::wasm::render::refresh_modals();

    spawn_local(async move {
        let _ = (cur, new_pw, tok);
        with_state_mut(|s| s.is_changing_password = false);
        crate::editorial::wasm::render::refresh_modals();
        show_error("Şifre değiştirme işlemi için Hesap Portalı (/hesabim) kullanılır.");
    });
}

// ============================================================================
// Initial Data Load & Published / Draft Articles Fetching
// ============================================================================

pub fn bootstrap_authenticated_data() {
    let Some(_tok) = with_state(|s| s.token.clone()) else {
        return;
    };

    fetch_articles("all", false);
    start_lock_heartbeat_timer();
    let existing_edit_id = with_state(|s| s.editing_article.id.trim().to_string());
    if !existing_edit_id.is_empty() {
        acquire_article_lock(&existing_edit_id);
    }
    fetch_tag_suggestions("");
    fetch_research_sources();
    fetch_editorial_meta();

    spawn_local(async move {
        // 1. Categories
        if let Ok((true, _, data)) = api_json("GET", "/api/v1/admin/editorial/categories", None, None).await {
            if let Some(cats_val) = data.get("categories") {
                if let Ok(cats) = serde_json::from_value::<Vec<Category>>(cats_val.clone()) {
                    with_state_mut(|s| s.categories = cats);
                }
            }
        }

        // 2. Settings
        if let Ok((true, _, data)) = api_json("GET", "/api/v1/admin/editorial/settings", None, None).await {
            if let Some(settings) = data.get("settings") {
                if let Some(wp_cfg_str) = settings.get("wp_config").and_then(|v| v.as_str()) {
                    if let Ok(parsed) = serde_json::from_str::<WordPressConfig>(wp_cfg_str) {
                        with_state_mut(|s| s.config = parsed);
                    }
                }
            }
        }

        // 3. Research Sources
        if let Ok((true, _, data)) = api_json("GET", "/api/v1/admin/editorial/research/sources", None, None).await {
            if let Some(srcs_val) = data.get("sources") {
                if let Ok(srcs) = serde_json::from_value::<Vec<crate::editorial::common::ResearchSource>>(srcs_val.clone()) {
                    with_state_mut(|s| s.research_sources = srcs);
                }
            }
        }

        crate::editorial::wasm::render::sync_left_toolbar_state();
        crate::editorial::wasm::render::refresh_settings_popover();
        crate::editorial::wasm::render::refresh_research_slots();
        if with_state(|s| s.editor_step == 4) {
            crate::editorial::wasm::render::refresh_right_panel();
        }
    });
}

pub fn trigger_debounced_search() {
    let seq = with_state_mut(|s| {
        s.search_debounce_seq += 1;
        s.search_debounce_seq
    });
    crate::editorial::wasm::render::refresh_left_body();

    set_timeout_ms(400, move || {
        let should_run = with_state(|s| s.search_debounce_seq == seq && s.token.is_some());
        if should_run {
            fetch_articles("all", false);
        }
    });
}

pub fn fetch_articles(kind: &'static str, load_more: bool) {
    fetch_articles_internal(kind, load_more, false);
}

pub fn fetch_articles_silent() {
    fetch_articles_internal("all", false, true);
}

fn articles_snapshot_signature(articles: &[Article], total: usize) -> String {
    let mut sig = format!("{total}:");
    for a in articles {
        sig.push_str(&a.id);
        sig.push('|');
        sig.push_str(&a.title);
        sig.push('|');
        sig.push_str(&a.status);
        sig.push('|');
        sig.push_str(&a.en_status);
        sig.push('|');
        sig.push_str(&a.publication_channel);
        sig.push('|');
        sig.push_str(&a.issue_number.unwrap_or(0).to_string());
        sig.push('|');
        sig.push_str(&a.visibility_level.to_string());
        sig.push('|');
        sig.push_str(&a.views.to_string());
        sig.push('|');
        sig.push_str(&a.comments.to_string());
        sig.push(';');
    }
    sig
}

fn fetch_articles_internal(kind: &'static str, load_more: bool, silent: bool) {
    let (token, q, start_date, end_date, offsets) = with_state(|s| {
        (
            s.token.clone(),
            s.search_query.trim().to_string(),
            s.search_start_date.clone(),
            s.search_end_date.clone(),
            (s.published_articles.len(), s.draft_articles.len()),
        )
    });
    let Some(tok) = token else { return };

    if !silent {
        if load_more {
            with_state_mut(|s| s.is_loading_more = true);
            crate::editorial::wasm::render::refresh_left_body();
        } else {
            with_state_mut(|s| s.is_refreshing_articles = true);
            crate::editorial::wasm::render::sync_left_toolbar_state();
        }
    }

    let mut extra_params = String::new();
    if !q.is_empty() {
        extra_params.push_str(&format!("&q={}", encode_param(&q)));
    }
    if !start_date.is_empty() {
        extra_params.push_str(&format!("&startDate={}", encode_param(&start_date)));
    }
    if !end_date.is_empty() {
        extra_params.push_str(&format!("&endDate={}", encode_param(&end_date)));
    }

    let types: Vec<&'static str> = if kind == "all" {
        vec!["published", "draft"]
    } else {
        vec![kind]
    };

    spawn_local(async move {
        let (prev_pub_sig, prev_draft_sig) = with_state(|s| {
            (
                articles_snapshot_signature(&s.published_articles, s.published_total),
                articles_snapshot_signature(&s.draft_articles, s.draft_total),
            )
        });

        for t in types {
            let offset = if load_more {
                match t {
                    "published" => offsets.0,
                    "draft" => offsets.1,
                    _ => 0,
                }
            } else {
                0
            };

            let url = format!("/api/v1/admin/editorial/articles?type={t}&offset={offset}{extra_params}");
            match api_json("GET", &url, Some(&tok), None).await {
                Ok((true, _, data)) => {
                    let articles = data
                        .get("articles")
                        .and_then(|v| serde_json::from_value::<Vec<Article>>(v.clone()).ok())
                        .unwrap_or_default();
                    let total = data
                        .get("totalCount")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(articles.len() as u64) as usize;

                    with_state_mut(|s| match t {
                        "published" => {
                            if load_more {
                                s.published_articles.extend(articles);
                            } else {
                                s.published_articles = articles;
                                s.published_total = total.max(s.published_articles.len());
                            }
                        }
                        "draft" => {
                            if load_more {
                                s.draft_articles.extend(articles);
                            } else {
                                s.draft_articles = articles;
                                s.draft_total = total.max(s.draft_articles.len());
                            }
                        }
                        _ => {}
                    });
                }
                Ok((false, status, data)) => {
                    if !silent {
                        let msg = data
                            .get("error")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{t} haberleri yüklenemedi ({status})"));
                        show_error(msg);
                    }
                }
                Err(_) => {}
            }
        }

        let (left_changed, middle_changed) = with_state_mut(|s| {
            s.is_loading_more = false;
            s.is_refreshing_articles = false;

            let new_pub_sig = articles_snapshot_signature(&s.published_articles, s.published_total);
            let new_draft_sig = articles_snapshot_signature(&s.draft_articles, s.draft_total);
            let list_diff = new_pub_sig != prev_pub_sig || new_draft_sig != prev_draft_sig;

            if !load_more && s.selected_article.is_none() && !s.published_articles.is_empty() {
                s.selected_article = s.published_articles.first().cloned();
                return (true, true);
            }

            if let Some(cur) = s.selected_article.clone() {
                if let Some(updated) = s
                    .published_articles
                    .iter()
                    .find(|a| a.id == cur.id)
                    .or_else(|| s.draft_articles.iter().find(|a| a.id == cur.id))
                    .cloned()
                {
                    let cur_json = serde_json::to_string(&cur).unwrap_or_default();
                    let upd_json = serde_json::to_string(&updated).unwrap_or_default();
                    if cur_json != upd_json {
                        s.selected_article = Some(updated);
                        return (list_diff, true);
                    }
                }
            }
            (list_diff, false)
        });

        crate::editorial::wasm::render::sync_left_toolbar_state();
        if !silent || left_changed {
            crate::editorial::wasm::render::refresh_left_body();
        }
        if middle_changed {
            crate::editorial::wasm::render::refresh_middle_panel();
        }
    });
}

pub fn delete_article(article_id: String) {
    if article_id.trim().is_empty() {
        return;
    }
    if !confirm_dialog("Bu haberi kalıcı olarak silmek istediğinize emin misiniz?") {
        return;
    }
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };

    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/articles/{}", encode_param(&article_id));
        match api_json("DELETE", &url, Some(&tok), None).await {
            Ok((true, _, _)) => {
                let mut cleared_editor = false;
                with_state_mut(|s| {
                    let before_pub = s.published_articles.len();
                    s.published_articles.retain(|a| a.id != article_id);
                    if s.published_articles.len() < before_pub {
                        s.published_total = s.published_total.saturating_sub(1);
                    }

                    let before_draft = s.draft_articles.len();
                    s.draft_articles.retain(|a| a.id != article_id);
                    if s.draft_articles.len() < before_draft {
                        s.draft_total = s.draft_total.saturating_sub(1);
                    }

                    if s.selected_article
                        .as_ref()
                        .map(|a| a.id == article_id)
                        .unwrap_or(false)
                    {
                        s.selected_article = s
                            .published_articles
                            .first()
                            .cloned()
                            .or_else(|| s.draft_articles.first().cloned());
                    }

                    if s.editing_article.id == article_id {
                        clear_editing_draft();
                        s.editing_article = empty_editing_article();
                        s.reset_editor_lang_buffers();
                        s.editor_step = 1;
                        s.editor_image_error = false;
                        s.crop_loaded_src.clear();
                        s.crop_image_el = None;
                        cleared_editor = true;
                    }
                });

                crate::editorial::wasm::render::refresh_left_panel();
                crate::editorial::wasm::render::refresh_middle_panel();
                if cleared_editor {
                    crate::editorial::wasm::render::refresh_right_panel();
                }
                show_success("Haber silindi.");
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Haber silinemedi.");
                show_error(err);
            }
            Err(_) => show_error("Bağlantı hatası."),
        }
    });
}
// ============================================================================
// Move to Editor, Upload, Crop & Local SEO Auto-Fill
// ============================================================================

pub fn move_selected_to_editor() {
    let should_confirm = with_state(|s| {
        let Some(ref sel) = s.selected_article else {
            return false;
        };
        let same_id = !s.editing_article.id.trim().is_empty() && s.editing_article.id == sel.id;
        !same_id && should_confirm_editor_discard(s)
    });
    if should_confirm {
        with_state_mut(|s| {
            s.pending_editor_confirm = Some(PendingEditorConfirm::MoveSelectedToEditor);
        });
        crate::editorial::wasm::render::sync_editor_confirm_modal_state();
        return;
    }
    move_selected_to_editor_force();
}

pub fn move_selected_to_editor_force() {
    let (selected, old_editing_id) =
        with_state(|s| (s.selected_article.clone(), s.editing_article.id.clone()));
    let Some(sel) = selected else { return };

    let content_str = sel.content.clone();
    let parsed_blocks = if let Some(existing) = sel.blocks.clone().filter(|b| !b.is_empty()) {
        existing
    } else {
        let p = parse_content_to_blocks(&content_str);
        if !p.is_empty() {
            p
        } else if !content_str.trim().is_empty() {
            vec![ContentBlock {
                id: format!("blk-{}", js_now_ms()),
                block_type: "paragraph".to_string(),
                content: content_str.clone(),
                meta: None,
            }]
        } else {
            vec![ContentBlock {
                id: "b-init-1".to_string(),
                block_type: "paragraph".to_string(),
                content: String::new(),
                meta: None,
            }]
        }
    };

    let parsed_blocks_en = sel
        .blocks_en
        .clone()
        .filter(|b| !b.is_empty())
        .or_else(|| {
            sel.content_en
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(parse_content_to_blocks)
                .filter(|b| !b.is_empty())
        });

    let final_image = sel.effective_image_url().unwrap_or_default().to_string();
    let is_draft = sel.article_type.as_deref() == Some("draft") || sel.id.starts_with("draft-");
    let is_pub = sel.article_type.as_deref() == Some("published") || sel.id.starts_with("pub-");

    let mut new_editing = sel.clone();
    new_editing.id = if is_draft || is_pub {
        sel.id.clone()
    } else {
        String::new()
    };
    new_editing.article_type = if is_draft {
        Some("draft".to_string())
    } else if is_pub {
        Some("published".to_string())
    } else {
        None
    };
    new_editing.content = content_str;
    new_editing.blocks = Some(parsed_blocks);
    new_editing.blocks_en = parsed_blocks_en;
    if new_editing.status.trim().is_empty() {
        new_editing.status = "draft".to_string();
    }
    if new_editing.en_status.trim().is_empty() {
        new_editing.en_status = "none".to_string();
    }
    if new_editing.publication_channel.trim().is_empty() {
        new_editing.publication_channel = "news".to_string();
    }
    if new_editing.content_type.trim().is_empty() {
        new_editing.content_type = "haberler".to_string();
    }
    new_editing.image_url = Some(final_image.clone());
    new_editing.image_url_camel = Some(final_image.clone());
    new_editing.scheduled_at = sel.effective_scheduled_at().map(|s| s.to_string());
    new_editing.scheduled_at_camel = sel.effective_scheduled_at().map(|s| s.to_string());
    new_editing.wp_post_id = sel.wp_post_id.or(sel.wp_post_id_camel);
    new_editing.wp_post_id_camel = sel.wp_post_id_camel.or(sel.wp_post_id);
    new_editing.wp_post_url = sel
        .wp_post_url
        .clone()
        .or_else(|| sel.wp_post_url_camel.clone());
    new_editing.wp_post_url_camel = sel
        .wp_post_url_camel
        .clone()
        .or_else(|| sel.wp_post_url.clone());
    new_editing.auto_share = Some(sel.auto_share.unwrap_or(true));

    if !old_editing_id.trim().is_empty() && old_editing_id != new_editing.id {
        release_article_lock(&old_editing_id);
    }
    let next_lock_id = new_editing.id.clone();

    with_state_mut(|s| {
        s.mobile_view = MobileView::Right;
        s.editor_step = 1;
        s.editor_image_error = false;
        s.editing_conflict_lock = None;
        s.editing_related_block_id = None;
        s.middle_preview_draft = false;
        s.cover_watermarked = false;
        s.crop_loaded_src.clear();
        s.crop_image_el = None;
        s.editing_article = new_editing.clone();
        s.reset_editor_lang_buffers();
        let baseline = editor_article_signature(&s.canonical_editing_article());
        save_editing_baseline_sig(&baseline);
        s.editing_baseline_sig = baseline;
    });
    save_editing_draft(&new_editing);

    if !next_lock_id.trim().is_empty() {
        acquire_article_lock(&next_lock_id);
    }

    if with_state(|s| s.is_desktop) {
        crate::editorial::wasm::render::refresh_left_body();
        crate::editorial::wasm::render::refresh_right_panel();
    } else {
        crate::editorial::wasm::render::render_full();
    }
    show_success("Editöre aktarıldı.");

    let hostname = web_sys::window()
        .and_then(|w| w.location().hostname().ok())
        .unwrap_or_default();
    if is_valid_image_url(Some(&final_image))
        && final_image.starts_with("http")
        && (hostname.is_empty() || !final_image.contains(&hostname))
    {
        set_timeout_ms(100, move || {
            fetch_remote_image_to_editor(final_image);
        });
    }
}

pub fn auto_fill_seo_local() {
    let canon = with_state(|s| s.canonical_editing_article());
    let has_data = {
        let has_title = !canon.title.trim().is_empty();
        let has_blocks = canon
            .blocks
            .as_ref()
            .map(|b| b.iter().any(|x| !x.content.trim().is_empty()))
            .unwrap_or(false);
        let has_content = !canon.content.trim().is_empty();
        has_title || has_blocks || has_content
    };

    if !has_data {
        show_error("Otomatik doldurmak için önce Türkçe haber başlığı veya içeriği girin.");
        return;
    }

    with_state_mut(|s| {
        let seo = build_auto_seo(&canon);
        if s.editing_article
            .slug
            .as_deref()
            .map(|sl| sl.trim().is_empty())
            .unwrap_or(true)
        {
            let gen_slug = slugify_turkish(&canon.title);
            if !gen_slug.is_empty() {
                s.editing_article.slug = Some(gen_slug);
            }
        }
        s.editing_article.seo = Some(seo);
        save_editing_draft(&s.editing_article);
    });
    crate::editorial::wasm::render::refresh_right_panel();
    show_success("SEO alanları otomatik dolduruldu.");
}

#[derive(Debug, Clone)]
pub struct UploadedMediaInfo {
    pub url: String,
    pub thumb_url: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub watermarked: bool,
    pub asset: Option<MediaAsset>,
}

fn parse_uploaded_media_info(data: &Value) -> Option<UploadedMediaInfo> {
    let url = data.get("url")?.as_str()?.trim().to_string();
    if url.is_empty() {
        return None;
    }
    let asset = data
        .get("asset")
        .and_then(|v| serde_json::from_value::<MediaAsset>(v.clone()).ok());
    let thumb_url = data
        .get("thumbUrl")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| asset.as_ref().and_then(|a| a.thumb_url.clone()));
    let width = data
        .get("width")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .filter(|w| *w > 0)
        .or_else(|| asset.as_ref().map(|a| a.width).filter(|w| *w > 0));
    let height = data
        .get("height")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .filter(|h| *h > 0)
        .or_else(|| asset.as_ref().map(|a| a.height).filter(|h| *h > 0));
    let watermarked = data
        .get("watermarked")
        .and_then(|v| v.as_bool())
        .or_else(|| asset.as_ref().map(|a| a.watermarked))
        .unwrap_or(false);

    Some(UploadedMediaInfo {
        url,
        thumb_url,
        width,
        height,
        watermarked,
        asset,
    })
}

fn upsert_media_asset_in_state(asset: MediaAsset) {
    with_state_mut(|s| {
        let existed = s.media_assets.iter().any(|a| a.id == asset.id);
        s.media_assets.retain(|a| a.id != asset.id);
        s.media_assets.insert(0, asset);
        if !existed {
            s.media_total += 1;
        }
    });
}

pub async fn upload_media_blob(
    blob: &Blob,
    filename: &str,
    watermark: bool,
) -> Option<UploadedMediaInfo> {
    let _token = with_state(|s| s.token.clone())?;
    with_state_mut(|s| s.is_uploading = true);

    let Ok(form) = FormData::new() else {
        with_state_mut(|s| s.is_uploading = false);
        show_error("Form verisi oluşturulamadı.");
        return None;
    };
    if form
        .append_with_blob_and_filename("image", blob, filename)
        .is_err()
    {
        with_state_mut(|s| s.is_uploading = false);
        show_error("Görsel form verisine eklenemedi.");
        return None;
    }

    let Some(win) = web_sys::window() else {
        with_state_mut(|s| s.is_uploading = false);
        return None;
    };
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_credentials(RequestCredentials::SameOrigin);
    if let Ok(headers) = Headers::new() {
        if let Some(csrf) = read_cookie("csrf_token") {
            let _ = headers.set("X-CSRF-Token", &csrf);
        }
        opts.set_headers(&headers);
    }
    opts.set_body(form.as_ref());

    let endpoint = if watermark {
        "/api/v1/admin/editorial/upload?watermark=1"
    } else {
        "/api/v1/admin/editorial/upload?watermark=0"
    };

    let result = match Request::new_with_str_and_init(endpoint, &opts) {
        Ok(req) => match JsFuture::from(win.fetch_with_request(&req)).await {
            Ok(resp_val) => {
                if let Ok(resp) = resp_val.dyn_into::<Response>() {
                    if let Ok(tp) = resp.text() {
                        if let Ok(tv) = JsFuture::from(tp).await {
                            let txt = tv.as_string().unwrap_or_default();
                            serde_json::from_str::<Value>(&txt).ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        },
        Err(_) => None,
    };

    with_state_mut(|s| s.is_uploading = false);

    match result {
        Some(data) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
            let info = parse_uploaded_media_info(&data)?;
            if let Some(ref asset) = info.asset {
                upsert_media_asset_in_state(asset.clone());
            }
            Some(info)
        }
        Some(data) => {
            let err = data
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Yükleme başarısız.");
            show_error(err);
            None
        }
        None => {
            show_error("Görsel yüklenirken bir hata oluştu.");
            None
        }
    }
}

pub async fn upload_blob_with_filename(blob: &Blob, filename: &str) -> Option<String> {
    let wm = with_state(|s| s.upload_with_watermark);
    upload_media_blob(blob, filename, wm)
        .await
        .map(|info| info.url)
}

pub fn upload_file_for_step2(file: File) {
    let wm = with_state(|s| s.upload_with_watermark);
    spawn_local(async move {
        let filename = file.name();
        let blob: &Blob = file.as_ref();
        if let Some(info) = upload_media_blob(blob, &filename, wm).await {
            with_state_mut(|s| {
                set_article_image(&mut s.editing_article, &info.url);
                s.editing_article.thumb_url = info.thumb_url.or_else(|| Some(info.url.clone()));
                if let Some(w) = info.width {
                    s.editing_article.cover_width = Some(w);
                }
                if let Some(h) = info.height {
                    s.editing_article.cover_height = Some(h);
                }
                s.cover_watermarked = info.watermarked;
                s.editor_image_error = false;
                s.crop_loaded_src.clear();
                s.crop_image_el = None;
                save_editing_draft(&s.editing_article);
            });
            crate::editorial::wasm::render::refresh_right_panel();
        }
    });
}

pub fn upload_file_for_block(block_id: String, file: File) {
    let wm = with_state(|s| s.upload_with_watermark);
    spawn_local(async move {
        let filename = file.name();
        let blob: &Blob = file.as_ref();
        if let Some(info) = upload_media_blob(blob, &filename, wm).await {
            with_state_mut(|s| {
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == block_id {
                            b.content = info.url.clone();
                        }
                    }
                }
                save_editing_draft(&s.editing_article);
            });
            crate::editorial::wasm::render::refresh_right_panel();
        }
    });
}

pub fn upload_files_for_gallery(block_id: String, files: Vec<File>) {
    if files.is_empty() {
        return;
    }
    let wm = with_state(|s| s.upload_with_watermark);
    spawn_local(async move {
        let mut uploaded_items: Vec<GalleryItem> = Vec::new();
        for f in files {
            let fname = f.name();
            let blob: &Blob = f.as_ref();
            if let Some(info) = upload_media_blob(blob, &fname, wm).await {
                uploaded_items.push(GalleryItem {
                    url: info.url,
                    thumb_url: info.thumb_url,
                    caption: String::new(),
                });
            }
        }
        if !uploaded_items.is_empty() {
            let count = uploaded_items.len();
            with_state_mut(|s| {
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == block_id && b.block_type == "gallery" {
                            let mut existing: Vec<GalleryItem> =
                                serde_json::from_str(b.content.trim()).unwrap_or_default();
                            existing.extend(uploaded_items.clone());
                            b.content = serde_json::to_string(&existing)
                                .unwrap_or_else(|_| "[]".to_string());
                        }
                    }
                }
                save_editing_draft(&s.editing_article);
            });
            crate::editorial::wasm::render::refresh_blocks_list_only();
            show_success(format!("{count} fotoğraf galeriye eklendi."));
        }
    });
}

pub fn upload_files_to_media_library(files: Vec<File>) {
    if files.is_empty() {
        return;
    }
    let wm = with_state(|s| s.upload_with_watermark);
    spawn_local(async move {
        let mut ok_count = 0usize;
        for f in files {
            let fname = f.name();
            let blob: &Blob = f.as_ref();
            if let Some(info) = upload_media_blob(blob, &fname, wm).await {
                ok_count += 1;
                if info.asset.is_none() {
                    // Refresh archive list from server if asset payload wasn't inlined
                    fetch_media_assets();
                }
            }
        }
        if ok_count > 0 {
            crate::editorial::wasm::render::sync_media_modal_state(true);
            show_success(format!("{ok_count} görsel medya arşivine yüklendi."));
        }
    });
}

pub fn fetch_remote_image_to_editor(url: String) {
    let trimmed = url.trim().to_string();
    if !trimmed.starts_with("http") {
        return;
    }
    let (tok_opt, wm) = with_state(|s| (s.token.clone(), s.upload_with_watermark));
    let Some(tok) = tok_opt else {
        return;
    };

    with_state_mut(|s| s.is_uploading = true);
    if with_state(|s| s.editor_step == 2) {
        crate::editorial::wasm::render::refresh_right_panel();
    }

    spawn_local(async move {
        let body = json!({ "url": trimmed, "watermark": wm });
        match api_json("POST", "/api/v1/admin/editorial/fetch-remote-image", Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                if let Some(info) = parse_uploaded_media_info(&data) {
                    if let Some(ref asset) = info.asset {
                        upsert_media_asset_in_state(asset.clone());
                    }
                    with_state_mut(|s| {
                        let was_clean = !s.editing_baseline_sig.is_empty()
                            && editor_article_signature(&s.canonical_editing_article())
                                == s.editing_baseline_sig;
                        s.is_uploading = false;
                        set_article_image(&mut s.editing_article, &info.url);
                        s.editing_article.thumb_url =
                            info.thumb_url.or_else(|| Some(info.url.clone()));
                        if let Some(w) = info.width {
                            s.editing_article.cover_width = Some(w);
                        }
                        if let Some(h) = info.height {
                            s.editing_article.cover_height = Some(h);
                        }
                        s.cover_watermarked = info.watermarked;
                        s.step2_remote_url.clear();
                        s.editor_image_error = false;
                        s.crop_loaded_src.clear();
                        s.crop_image_el = None;
                        if was_clean {
                            let new_sig = editor_article_signature(&s.canonical_editing_article());
                            save_editing_baseline_sig(&new_sig);
                            s.editing_baseline_sig = new_sig;
                        }
                        save_editing_draft(&s.editing_article);
                    });
                    if with_state(|s| s.editor_step == 2) {
                        crate::editorial::wasm::render::refresh_right_panel();
                    }
                    show_success("Görsel başarıyla çekildi.");
                    return;
                }
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Görsel çekilemedi.");
                with_state_mut(|s| {
                    s.is_uploading = false;
                    s.editor_image_error = true;
                });
                if with_state(|s| s.editor_step == 2) {
                    crate::editorial::wasm::render::refresh_right_panel();
                }
                show_error(err);
                return;
            }
            Err(_) => {
                with_state_mut(|s| s.is_uploading = false);
                if with_state(|s| s.editor_step == 2) {
                    crate::editorial::wasm::render::refresh_right_panel();
                }
                show_error("Görsel çekilirken bir hata oluştu.");
                return;
            }
        }
        with_state_mut(|s| s.is_uploading = false);
    });
}

pub fn apply_watermark_to_current_cover() {
    let (tok_opt, cur_img, busy) = with_state(|s| {
        (
            s.token.clone(),
            s.editing_article
                .effective_image_url()
                .unwrap_or_default()
                .trim()
                .to_string(),
            s.is_uploading,
        )
    });
    if busy
        || (!cur_img.starts_with("/uploads/editorial/") && !cur_img.starts_with("/uploads/"))
    {
        return;
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| s.is_uploading = true);
    crate::editorial::wasm::render::refresh_right_panel();

    spawn_local(async move {
        let body = json!({ "url": cur_img });
        match api_json("POST", "/api/v1/admin/editorial/media/watermark-url", Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                if let Some(info) = parse_uploaded_media_info(&data) {
                    if let Some(ref asset) = info.asset {
                        upsert_media_asset_in_state(asset.clone());
                    }
                    with_state_mut(|s| {
                        s.is_uploading = false;
                        set_article_image(&mut s.editing_article, &info.url);
                        s.editing_article.thumb_url =
                            info.thumb_url.or_else(|| Some(info.url.clone()));
                        if let Some(w) = info.width {
                            s.editing_article.cover_width = Some(w);
                        }
                        if let Some(h) = info.height {
                            s.editing_article.cover_height = Some(h);
                        }
                        s.cover_watermarked = true;
                        s.editor_image_error = false;
                        s.crop_loaded_src.clear();
                        s.crop_image_el = None;
                        save_editing_draft(&s.editing_article);
                    });
                    crate::editorial::wasm::render::refresh_right_panel();
                    show_success("Filigran / logo başarıyla uygulandı.");
                    return;
                }
                with_state_mut(|s| s.is_uploading = false);
                crate::editorial::wasm::render::refresh_right_panel();
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_uploading = false);
                crate::editorial::wasm::render::refresh_right_panel();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Filigran uygulanamadı.");
                show_error(err);
            }
            Err(_) => {
                with_state_mut(|s| s.is_uploading = false);
                crate::editorial::wasm::render::refresh_right_panel();
                show_error("Bağlantı hatası.");
            }
        }
    });
}

pub fn handle_crop_and_continue() {
    let (has_img, is_err, busy, wm_needed, need_tags) = with_state(|s| {
        (
            is_valid_image_url(s.editing_article.effective_image_url()),
            s.editor_image_error,
            s.is_cropping,
            s.upload_with_watermark && !s.cover_watermarked,
            s.tag_suggestions.is_empty(),
        )
    });
    if busy {
        return;
    }
    if need_tags {
        fetch_tag_suggestions("");
    }
    if !has_img || is_err {
        with_state_mut(|s| s.editor_step = 3);
        crate::editorial::wasm::render::refresh_right_panel();
        return;
    }

    let blob_opt = match export_cropped_jpeg_blob() {
        Ok(b) => b,
        Err(err_msg) => {
            show_error(err_msg);
            return;
        }
    };

    let Some(blob) = blob_opt else {
        with_state_mut(|s| s.editor_step = 3);
        crate::editorial::wasm::render::refresh_right_panel();
        return;
    };

    with_state_mut(|s| s.is_cropping = true);
    crate::editorial::wasm::render::refresh_right_panel();

    spawn_local(async move {
        if let Some(info) = upload_media_blob(&blob, "cropped-header.jpg", wm_needed).await {
            with_state_mut(|s| {
                s.is_cropping = false;
                set_article_image(&mut s.editing_article, &info.url);
                s.editing_article.thumb_url = info.thumb_url.or_else(|| Some(info.url.clone()));
                s.editing_article.cover_width = info.width.or(Some(1200));
                s.editing_article.cover_height = info.height.or(Some(675));
                s.cover_watermarked = s.cover_watermarked || info.watermarked;
                s.crop_loaded_src.clear();
                s.crop_image_el = None;
                s.editor_step = 3;
                save_editing_draft(&s.editing_article);
            });
            crate::editorial::wasm::render::refresh_right_panel();
        } else {
            with_state_mut(|s| s.is_cropping = false);
            crate::editorial::wasm::render::refresh_right_panel();
            show_error(
                "Görsel kırpılırken bir sorun oluştu. Lütfen görseli değiştirip tekrar deneyin.",
            );
        }
    });
}

// ============================================================================
// Direct CMS Publishing, Draft Save, Stats Refresh & Social Share
// ============================================================================

fn configured_social_platforms(cfg: &WordPressConfig) -> Vec<&'static str> {
    let mut platforms = Vec::new();
    if !cfg.telegram.bot_token.trim().is_empty() && !cfg.telegram.chat_id.trim().is_empty() {
        platforms.push("telegram");
    }
    if !cfg.twitter.api_key.trim().is_empty()
        && !cfg.twitter.api_secret.trim().is_empty()
        && !cfg.twitter.access_token.trim().is_empty()
        && !cfg.twitter.access_secret.trim().is_empty()
    {
        platforms.push("twitter");
    }
    platforms
}

pub fn publish_article_now() {
    let (art, cfg, tok_opt, publisher_name, busy) = with_state(|s| {
        (
            s.canonical_editing_article(),
            s.config.clone(),
            s.token.clone(),
            s.user.as_ref().map(|u| u.username.clone()),
            s.is_posting,
        )
    });
    if busy {
        return;
    }
    if art.title.trim().is_empty() {
        show_error("Lütfen önce bir haber başlığı girin.");
        return;
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| {
        s.is_posting = true;
        s.error = None;
        s.success = None;
    });
    crate::editorial::wasm::render::refresh_right_panel();
    crate::editorial::wasm::render::refresh_toasts();

    let final_content = match art.blocks.as_ref().filter(|b| !b.is_empty()) {
        Some(blocks) => blocks_to_wp_html(blocks),
        None => art.content.clone(),
    };

    let trimmed_id = art.id.trim();
    let has_server_id = !trimmed_id.is_empty() && !trimmed_id.starts_with("draft-");
    let old_draft_id = if trimmed_id.starts_with("draft-") {
        Some(trimmed_id.to_string())
    } else {
        None
    };

    let now_iso = js_now_iso();
    let mut pub_art = art.clone();
    if !has_server_id {
        pub_art.id = String::new();
    }
    pub_art.article_type = Some("published".to_string());
    pub_art.status = "publish".to_string();
    pub_art.content = final_content;
    pub_art.publish_date = Some(now_iso.clone());
    pub_art.publish_date_camel = Some(now_iso.clone());
    if pub_art.created_at.is_none() {
        pub_art.created_at = Some(now_iso);
    }
    if let Some(img) = art.effective_image_url().map(|s| s.to_string()) {
        set_article_image(&mut pub_art, &img);
        if let Some(ref mut seo) = pub_art.seo {
            if !img.is_empty() {
                seo.image_url = img;
            }
        }
    }
    pub_art.categories = art.categories.clone().filter(|c| !c.is_empty());
    pub_art.target_sites = None;
    if pub_art.publisher_name.is_none() {
        pub_art.publisher_name = publisher_name;
    }

    let auto_share = art.auto_share.unwrap_or(true);

    spawn_local(async move {
        let Ok(save_val) = serde_json::to_value(&pub_art) else {
            with_state_mut(|s| s.is_posting = false);
            crate::editorial::wasm::render::refresh_right_panel();
            return;
        };

        match api_json("POST", "/api/v1/admin/editorial/articles", Some(&tok), Some(&save_val)).await {
            Ok((ok, _, data)) if ok && data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                if let Some(server_id) = data
                    .get("id")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    pub_art.id = server_id.to_string();
                }

                if let Some(rt) = data.get("readTimeMin").and_then(|v| v.as_i64()) {
                    pub_art.read_time_min = Some(rt as i32);
                }

                if let Some(server_slug) = data
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    pub_art.slug = Some(server_slug.to_string());
                } else if pub_art
                    .slug
                    .as_deref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
                {
                    let gen = slugify_turkish(&pub_art.title);
                    if !gen.is_empty() {
                        pub_art.slug = Some(gen);
                    }
                }

                if let Some(ref did) = old_draft_id {
                    if !did.starts_with("draft-") && did != &pub_art.id {
                        let del_url = format!("/api/v1/admin/editorial/articles/{}", encode_param(did));
                        let _ = api_json("DELETE", &del_url, Some(&tok), None).await;
                    }
                }

                let published_id = pub_art.id.clone();
                if !art.id.trim().is_empty() {
                    release_article_lock(&art.id);
                }
                if published_id != art.id && !published_id.trim().is_empty() {
                    release_article_lock(&published_id);
                }
                clear_editing_draft();
                let is_desktop = with_state_mut(|s| {
                    s.is_posting = false;
                    s.editing_conflict_lock = None;
                    s.cover_watermarked = false;
                    if let Some(ref did) = old_draft_id {
                        let prev_len = s.draft_articles.len();
                        s.draft_articles.retain(|d| &d.id != did);
                        if s.draft_articles.len() < prev_len {
                            s.draft_total = s.draft_total.saturating_sub(1);
                        }
                    }

                    if let Some(existing_slot) =
                        s.published_articles.iter_mut().find(|a| a.id == published_id)
                    {
                        *existing_slot = pub_art.clone();
                    } else {
                        s.published_articles.insert(0, pub_art.clone());
                        s.published_total += 1;
                    }

                    s.selected_article = Some(pub_art);
                    s.preview_lang = "tr".to_string();
                    s.middle_preview_draft = false;
                    s.show_preview_modal = false;
                    s.left_tab = LeftTab::Published;
                    s.editing_article = empty_editing_article();
                    s.reset_editor_lang_buffers();
                    s.editing_baseline_sig.clear();
                    s.editor_step = 1;
                    s.editor_image_error = false;
                    s.crop_loaded_src.clear();
                    s.crop_image_el = None;
                    s.mobile_view = MobileView::Middle;
                    s.is_desktop
                });

                crate::editorial::wasm::render::sync_preview_modal_state();

                if is_desktop {
                    crate::editorial::wasm::render::refresh_left_panel();
                    crate::editorial::wasm::render::refresh_middle_panel();
                    crate::editorial::wasm::render::refresh_right_panel();
                } else {
                    crate::editorial::wasm::render::render_full();
                }
                show_success("Haber başarıyla yayınlandı!");

                if auto_share {
                    let platforms = configured_social_platforms(&cfg);
                    if !platforms.is_empty() {
                        let share_body = json!({
                            "articleId": published_id,
                            "platforms": platforms,
                        });
                        let _ =
                            api_json("POST", "/api/v1/admin/editorial/social/share", Some(&tok), Some(&share_body))
                                .await;
                    }
                }
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_posting = false);
                crate::editorial::wasm::render::refresh_right_panel();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Haber yayınlanamadı.");
                show_error(err);
            }
            Err(e) => {
                with_state_mut(|s| s.is_posting = false);
                crate::editorial::wasm::render::refresh_right_panel();
                show_error(e);
            }
        }
    });
}

pub fn save_draft_to_system() {
    save_draft_internal(false);
}

pub fn save_plain_draft_to_system() {
    save_draft_internal(true);
}

fn save_draft_internal(force_plain_draft: bool) {
    let (art, tok_opt, busy) =
        with_state(|s| (s.canonical_editing_article(), s.token.clone(), s.is_posting));
    if busy {
        return;
    }
    if art.title.trim().is_empty() {
        show_error("Kaydetmek için lütfen önce Türkçe başlık girin.");
        return;
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| {
        s.is_posting = true;
        s.error = None;
        s.success = None;
    });
    crate::editorial::wasm::render::refresh_right_panel();
    crate::editorial::wasm::render::refresh_toasts();

    let final_content = match art.blocks.as_ref().filter(|b| !b.is_empty()) {
        Some(blocks) => blocks_to_wp_html(blocks),
        None => art.content.clone(),
    };

    let target_status = if force_plain_draft {
        "draft".to_string()
    } else {
        match art.status.as_str() {
            "pending_review" | "future" | "archived" => art.status.clone(),
            _ => "draft".to_string(),
        }
    };

    let mut draft = art.clone();
    let transitioning_published_in_place =
        (art.article_type.as_deref() == Some("published") || art.status == "publish")
            && matches!(target_status.as_str(), "draft" | "archived" | "pending_review");
    if draft.id.trim().starts_with("draft-") {
        draft.id = String::new();
    }
    draft.content = final_content;
    draft.article_type = Some("draft".to_string());
    draft.status = target_status.clone();
    if draft.created_at.is_none() {
        draft.created_at = Some(js_now_iso());
    }
    if let Some(img) = art.effective_image_url().map(|s| s.to_string()) {
        set_article_image(&mut draft, &img);
    }
    if let Some(sched) = art.effective_scheduled_at().map(|s| s.to_string()) {
        draft.scheduled_at = Some(sched.clone());
        draft.scheduled_at_camel = Some(sched);
    }
    draft.categories = art.categories.clone().filter(|c| !c.is_empty());
    draft.target_sites = None;

    spawn_local(async move {
        let Ok(body) = serde_json::to_value(&draft) else {
            with_state_mut(|s| s.is_posting = false);
            return;
        };
        match api_json("POST", "/api/v1/admin/editorial/articles", Some(&tok), Some(&body)).await {
            Ok((ok, _, data)) if ok && data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                if let Some(server_id) = data.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                    draft.id = server_id.to_string();
                }
                if let Some(server_slug) = data.get("slug").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                    draft.slug = Some(server_slug.to_string());
                }
                if let Some(rt) = data.get("readTimeMin").and_then(|v| v.as_i64()) {
                    draft.read_time_min = Some(rt as i32);
                }
                clear_editing_draft();
                let saved_id = draft.id.clone();
                if !art.id.trim().is_empty() {
                    release_article_lock(&art.id);
                }
                if saved_id != art.id && !saved_id.trim().is_empty() {
                    release_article_lock(&saved_id);
                }
                with_state_mut(|s| {
                    s.is_posting = false;
                    s.editing_conflict_lock = None;
                    s.cover_watermarked = false;
                    if transitioning_published_in_place {
                        let prev_pub = s.published_articles.len();
                        s.published_articles.retain(|p| p.id != saved_id);
                        if s.published_articles.len() < prev_pub {
                            s.published_total = s.published_total.saturating_sub(1);
                        }
                        s.left_tab = LeftTab::Drafts;
                    }
                    let existed = s.draft_articles.iter().any(|d| d.id == saved_id);
                    s.draft_articles.retain(|d| d.id != saved_id);
                    s.draft_articles.insert(0, draft.clone());
                    if !existed {
                        s.draft_total += 1;
                    }
                    if s.draft_status_filter != "all" && s.draft_status_filter != target_status {
                        s.draft_status_filter = "all".to_string();
                    }
                    s.selected_article = Some(draft);
                    s.preview_lang = "tr".to_string();
                    s.middle_preview_draft = false;
                    s.show_preview_modal = false;
                    s.editing_article = empty_editing_article();
                    s.reset_editor_lang_buffers();
                    s.editing_baseline_sig.clear();
                    s.editor_step = 1;
                    s.editor_image_error = false;
                    s.crop_loaded_src.clear();
                    s.crop_image_el = None;
                });
                crate::editorial::wasm::render::sync_preview_modal_state();
                crate::editorial::wasm::render::refresh_left_panel();
                crate::editorial::wasm::render::refresh_middle_panel();
                crate::editorial::wasm::render::refresh_right_panel();
                let msg = match target_status.as_str() {
                    "pending_review" => "İçerik başeditör onayına gönderildi!",
                    "future" => "Yayın ileri tarihe zamanlandı!",
                    "archived" => "İçerik arşive kaldırıldı!",
                    _ => "Taslak sisteme kaydedildi!",
                };
                show_success(msg);
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_posting = false);
                crate::editorial::wasm::render::refresh_right_panel();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Taslak kaydedilemedi.");
                show_error(err);
            }
            Err(e) => {
                with_state_mut(|s| s.is_posting = false);
                crate::editorial::wasm::render::refresh_right_panel();
                show_error(e);
            }
        }
    });
}

/// Saves the current editor work as a Draft first and then executes the pending reset/move action.
pub fn save_draft_and_execute_pending_confirm(action: PendingEditorConfirm) {
    let (mut art, tok_opt, busy) =
        with_state(|s| (s.canonical_editing_article(), s.token.clone(), s.is_posting));
    if busy {
        return;
    }
    if art.title.trim().is_empty() {
        art.title = "Başlıksız Taslak".to_string();
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| {
        s.pending_editor_confirm = None;
        s.is_posting = true;
    });
    crate::editorial::wasm::render::sync_editor_confirm_modal_state();
    crate::editorial::wasm::render::refresh_right_panel();

    let final_content = match art.blocks.as_ref().filter(|b| !b.is_empty()) {
        Some(blocks) => blocks_to_wp_html(blocks),
        None => art.content.clone(),
    };

    let mut draft = art.clone();
    if draft.id.trim().is_empty() || draft.id.starts_with("pub-") {
        draft.id = format!("draft-{}", js_now_ms());
    }
    draft.content = final_content;
    draft.article_type = Some("draft".to_string());
    draft.status = match art.status.as_str() {
        "pending_review" | "future" | "archived" => art.status.clone(),
        _ => "draft".to_string(),
    };
    if draft.created_at.is_none() {
        draft.created_at = Some(js_now_iso());
    }
    if let Some(img) = art.effective_image_url().map(|s| s.to_string()) {
        set_article_image(&mut draft, &img);
    }
    if let Some(sched) = art.effective_scheduled_at().map(|s| s.to_string()) {
        draft.scheduled_at = Some(sched.clone());
        draft.scheduled_at_camel = Some(sched);
    }
    draft.categories = art.categories.clone().filter(|c| !c.is_empty());
    draft.target_sites = None;

    spawn_local(async move {
        let Ok(body) = serde_json::to_value(&draft) else {
            with_state_mut(|s| s.is_posting = false);
            return;
        };
        match api_json("POST", "/api/v1/admin/editorial/articles", Some(&tok), Some(&body)).await {
            Ok((ok, _, data)) if ok && data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                if let Some(server_id) = data.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                    draft.id = server_id.to_string();
                }
                if let Some(server_slug) = data.get("slug").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                    draft.slug = Some(server_slug.to_string());
                }
                let saved_id = draft.id.clone();
                if !art.id.trim().is_empty() {
                    release_article_lock(&art.id);
                }
                if saved_id != art.id && !saved_id.trim().is_empty() {
                    release_article_lock(&saved_id);
                }
                with_state_mut(|s| {
                    s.is_posting = false;
                    let existed = s.draft_articles.iter().any(|d| d.id == saved_id);
                    s.draft_articles.retain(|d| d.id != saved_id);
                    s.draft_articles.insert(0, draft);
                    if !existed {
                        s.draft_total += 1;
                    }
                });
                match action {
                    PendingEditorConfirm::MoveSelectedToEditor => {
                        crate::editorial::wasm::render::refresh_left_panel();
                        move_selected_to_editor_force();
                    }
                    PendingEditorConfirm::NewArticle => {
                        crate::editorial::wasm::reset_editor_now(true);
                        show_success("Önceki içerik taslaklara kaydedildi, yeni haber açıldı.");
                    }
                    PendingEditorConfirm::ClearEditor => {
                        crate::editorial::wasm::reset_editor_now(false);
                        show_success("İçerik taslaklara kaydedildi ve editör temizlendi.");
                    }
                }
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_posting = false);
                crate::editorial::wasm::render::refresh_right_panel();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Taslak kaydedilemedi.");
                show_error(err);
            }
            Err(e) => {
                with_state_mut(|s| s.is_posting = false);
                crate::editorial::wasm::render::refresh_right_panel();
                show_error(e);
            }
        }
    });
}

pub fn refresh_article_stats(article_id: String) {
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };
    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/articles/{}/refresh-stats", encode_param(&article_id));
        if let Ok((true, _, data)) = api_json("POST", &url, Some(&tok), None).await {
            if data.get("success").and_then(|v| v.as_bool()) == Some(true) {
                let views = data.get("views").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let comments = data.get("comments").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                with_state_mut(|s| {
                    for a in &mut s.published_articles {
                        if a.id == article_id {
                            a.views = views;
                            a.comments = comments;
                        }
                    }
                    if let Some(sel) = &mut s.selected_article {
                        if sel.id == article_id {
                            sel.views = views;
                            sel.comments = comments;
                        }
                    }
                });
                crate::editorial::wasm::render::refresh_left_body();
                show_success("İstatistikler güncellendi.");
            }
        }
    });
}

pub fn manual_social_share(article_id: String) {
    let (tok_opt, cfg) = with_state(|s| (s.token.clone(), s.config.clone()));
    let Some(tok) = tok_opt else { return };
    let mut platforms = configured_social_platforms(&cfg);
    if platforms.is_empty() {
        platforms.push("telegram");
    }

    spawn_local(async move {
        let body = json!({
            "articleId": article_id,
            "platforms": platforms
        });
        match api_json("POST", "/api/v1/admin/editorial/social/share", Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                show_success("Haber sosyal medyada paylaşıldı!");
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Paylaşım başarısız.");
                show_error(err);
            }
            Err(_) => show_error("Bağlantı hatası."),
        }
    });
}
// ============================================================================
// Social Media Settings, Team Management & Connection Tests
// ============================================================================

pub fn save_all_settings() {
    let (tok_opt, can_save_settings, cfg) = with_state(|s| {
        (
            s.token.clone(),
            s.user
                .as_ref()
                .map(|u| {
                    u.capabilities.can_publish
                        || u.permissions.iter().any(|p| p == "*" || p == "settings:write")
                })
                .unwrap_or(false),
            s.config.clone(),
        )
    });
    let Some(tok) = tok_opt else { return };

    spawn_local(async move {
        if can_save_settings {
            if let Ok(cfg_str) = serde_json::to_string(&cfg) {
                let body = json!({ "wp_config": cfg_str });
                match api_json("POST", "/api/v1/admin/editorial/settings", Some(&tok), Some(&body)).await {
                    Ok((true, _, _)) => {}
                    Ok((_, _, data)) => {
                        let err = data
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Ayarlar kaydedilemedi.");
                        show_error(err);
                        return;
                    }
                    Err(e) => {
                        show_error(e);
                        return;
                    }
                }
            }
        }

        crate::editorial::wasm::render::refresh_settings_popover();
        show_success("Ayarlar başarıyla kaydedildi.");
    });
}

pub fn test_twitter_connection() {
    let (tw, tok_opt, busy) =
        with_state(|s| (s.config.twitter.clone(), s.token.clone(), s.is_testing));
    if busy {
        return;
    }
    if tw.api_key.trim().is_empty()
        || tw.api_secret.trim().is_empty()
        || tw.access_token.trim().is_empty()
        || tw.access_secret.trim().is_empty()
    {
        show_error("Lütfen tüm Twitter (X) API bilgilerini doldurun.");
        return;
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| s.is_testing = true);
    crate::editorial::wasm::render::refresh_settings_popover();

    spawn_local(async move {
        let body = json!({
            "twitterApiKey": tw.api_key,
            "twitterApiSecret": tw.api_secret,
            "twitterAccessToken": tw.access_token,
            "twitterAccessTokenSecret": tw.access_secret
        });
        match api_json("POST", "/api/v1/admin/editorial/social/test-twitter", Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                with_state_mut(|s| s.is_testing = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_success("X (Twitter) bağlantısı başarılı! Test tweeti atıldı.");
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_testing = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Bağlantı hatası.");
                show_error(err);
            }
            Err(e) => {
                with_state_mut(|s| s.is_testing = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error(format!("Bağlantı başarısız: {e}"));
            }
        }
    });
}

pub fn test_telegram_connection() {
    let (tg, tok_opt, busy) =
        with_state(|s| (s.config.telegram.clone(), s.token.clone(), s.is_testing));
    if busy {
        return;
    }
    if tg.bot_token.trim().is_empty() || tg.chat_id.trim().is_empty() {
        show_error("Lütfen Telegram bot token ve chat ID bilgilerini doldurun.");
        return;
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| s.is_testing = true);
    crate::editorial::wasm::render::refresh_settings_popover();

    spawn_local(async move {
        let Ok(body) = serde_json::to_value(&tg) else {
            with_state_mut(|s| s.is_testing = false);
            return;
        };
        match api_json("POST", "/api/v1/admin/editorial/social/test-telegram", Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                with_state_mut(|s| s.is_testing = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_success("Telegram bağlantısı başarılı! Test mesajı gönderildi.");
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_testing = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Bağlantı hatası.");
                show_error(err);
            }
            Err(e) => {
                with_state_mut(|s| s.is_testing = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error(format!("Bağlantı başarısız: {e}"));
            }
        }
    });
}

pub fn add_team_user() {
    show_error("Kullanıcı ve rol yönetimi MSI Komuta Merkezi üzerinden yapılır.");
}
pub fn delete_team_user(_id: i32) {
    show_error("Kullanıcı ve rol yönetimi MSI Komuta Merkezi üzerinden yapılır.");
}
// ============================================================================
// Category Management CRUD (`/api/categories`)
// ============================================================================

pub fn create_category() {
    let (name, slug, color, parent_id, tok_opt, busy) = with_state(|s| {
        (
            s.new_cat_name.trim().to_string(),
            s.new_cat_slug.trim().to_string(),
            s.new_cat_color.clone(),
            s.new_cat_parent_id,
            s.token.clone(),
            s.is_saving_category,
        )
    });
    if busy {
        return;
    }
    if name.is_empty() {
        show_error("Lütfen kategori adını girin.");
        return;
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| s.is_saving_category = true);
    crate::editorial::wasm::render::refresh_settings_popover();

    spawn_local(async move {
        let body = json!({
            "name": name,
            "slug": if slug.is_empty() { Value::Null } else { Value::String(slug) },
            "color": color,
            "parentId": parent_id,
        });
        match api_json("POST", "/api/v1/admin/editorial/categories", Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                if let Some(cat) = data
                    .get("category")
                    .and_then(|v| serde_json::from_value::<Category>(v.clone()).ok())
                {
                    with_state_mut(|s| {
                        s.is_saving_category = false;
                        s.categories.push(cat);
                        s.categories
                            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                        s.new_cat_name.clear();
                        s.new_cat_slug.clear();
                    });
                    crate::editorial::wasm::render::refresh_settings_popover();
                    if with_state(|s| s.editor_step == 4) {
                        crate::editorial::wasm::render::refresh_right_panel();
                    }
                    show_success("Kategori oluşturuldu.");
                    return;
                }
                with_state_mut(|s| s.is_saving_category = false);
                crate::editorial::wasm::render::refresh_settings_popover();
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_saving_category = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Kategori eklenemedi.");
                show_error(err);
            }
            Err(_) => {
                with_state_mut(|s| s.is_saving_category = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error("Bağlantı hatası.");
            }
        }
    });
}

pub fn update_category() {
    let (id_opt, name, slug, color, parent_id, tok_opt, busy) = with_state(|s| {
        (
            s.editing_cat_id,
            s.edit_cat_name.trim().to_string(),
            s.edit_cat_slug.trim().to_string(),
            s.edit_cat_color.clone(),
            s.edit_cat_parent_id,
            s.token.clone(),
            s.is_saving_category,
        )
    });
    if busy {
        return;
    }
    let Some(id) = id_opt else { return };
    if name.is_empty() {
        show_error("Kategori adı boş bırakılamaz.");
        return;
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| s.is_saving_category = true);
    crate::editorial::wasm::render::refresh_settings_popover();

    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/categories/{id}");
        let body = json!({
            "name": name,
            "slug": if slug.is_empty() { Value::Null } else { Value::String(slug) },
            "color": color,
            "parentId": parent_id,
        });
        match api_json("PUT", &url, Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                if let Some(updated) = data
                    .get("category")
                    .and_then(|v| serde_json::from_value::<Category>(v.clone()).ok())
                {
                    with_state_mut(|s| {
                        s.is_saving_category = false;
                        s.editing_cat_id = None;
                        if let Some(slot) = s.categories.iter_mut().find(|c| c.id == id) {
                            *slot = updated;
                        }
                        s.categories
                            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    });
                    crate::editorial::wasm::render::refresh_settings_popover();
                    if with_state(|s| s.editor_step == 4) {
                        crate::editorial::wasm::render::refresh_right_panel();
                    }
                    show_success("Kategori güncellendi.");
                    return;
                }
                with_state_mut(|s| s.is_saving_category = false);
                crate::editorial::wasm::render::refresh_settings_popover();
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_saving_category = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Kategori güncellenemedi.");
                show_error(err);
            }
            Err(_) => {
                with_state_mut(|s| s.is_saving_category = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error("Bağlantı hatası.");
            }
        }
    });
}

pub fn delete_category(id: i32) {
    if !confirm_dialog("Bu kategoriyi silmek istediğinize emin misiniz?") {
        return;
    }
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };

    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/categories/{id}");
        match api_json("DELETE", &url, Some(&tok), None).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                let server_cats = data
                    .get("categories")
                    .and_then(|v| serde_json::from_value::<Vec<Category>>(v.clone()).ok());

                with_state_mut(|s| {
                    if let Some(fresh_cats) = server_cats {
                        s.categories = fresh_cats;
                    } else {
                        let parent_of_deleted = s
                            .categories
                            .iter()
                            .find(|c| c.id == id)
                            .and_then(|c| c.parent_id);
                        s.categories.retain(|c| c.id != id);
                        for c in &mut s.categories {
                            if c.parent_id == Some(id) {
                                c.parent_id = parent_of_deleted;
                            }
                        }
                    }
                    if s.editing_cat_id == Some(id) {
                        s.editing_cat_id = None;
                    }
                    if s.new_cat_parent_id == Some(id) {
                        s.new_cat_parent_id = None;
                    }
                    if let Some(cats) = &mut s.editing_article.categories {
                        cats.retain(|cid| *cid != id);
                        save_editing_draft(&s.editing_article);
                    }
                });
                crate::editorial::wasm::render::refresh_settings_popover();
                if with_state(|s| s.editor_step == 4) {
                    crate::editorial::wasm::render::refresh_right_panel();
                }
                show_success("Kategori silindi.");
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Kategori silinemedi.");
                show_error(err);
            }
            Err(_) => show_error("Bağlantı hatası."),
        }
    });
}

// ============================================================================
// Research Sources Management CRUD (`/api/research/sources`)
// ============================================================================

pub fn fetch_research_sources() {
    let Some(tok) = with_state(|s| s.token.clone()) else { return };
    spawn_local(async move {
        if let Ok((true, _, data)) = api_json("GET", "/api/v1/admin/editorial/research/sources", Some(&tok), None).await {
            if let Some(srcs_val) = data.get("sources") {
                if let Ok(srcs) = serde_json::from_value::<Vec<crate::editorial::common::ResearchSource>>(srcs_val.clone()) {
                    with_state_mut(|s| s.research_sources = srcs);
                    crate::editorial::wasm::render::refresh_settings_popover();
                    crate::editorial::wasm::render::refresh_research_slots();
                }
            }
        }
    });
}

pub fn create_research_source() {
    let (url, name, cat, scope, tok_opt, busy) = with_state(|s| {
        (
            s.new_source_url.trim().to_string(),
            s.new_source_name.trim().to_string(),
            if s.new_source_category.trim().is_empty() {
                "Gündem".to_string()
            } else {
                s.new_source_category.trim().to_string()
            },
            if s.new_source_scope.trim().eq_ignore_ascii_case("global") {
                "global".to_string()
            } else {
                "tr".to_string()
            },
            s.token.clone(),
            s.is_saving_source,
        )
    });
    if busy {
        return;
    }
    if url.is_empty() {
        show_error("Lütfen bir RSS akışı veya haber sitesi adresi girin.");
        return;
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| s.is_saving_source = true);
    crate::editorial::wasm::render::refresh_settings_popover();

    spawn_local(async move {
        let body = json!({
            "url": url,
            "name": name,
            "category": cat,
            "scope": scope,
            "enabled": true,
        });
        match api_json("POST", "/api/v1/admin/editorial/research/sources", Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                if let Some(src) = data
                    .get("source")
                    .and_then(|v| serde_json::from_value::<crate::editorial::common::ResearchSource>(v.clone()).ok())
                {
                    with_state_mut(|s| {
                        s.is_saving_source = false;
                        s.new_source_url.clear();
                        s.new_source_name.clear();
                        s.research_sources.retain(|existing| existing.id != src.id);
                        s.research_sources.insert(0, src);
                    });
                    crate::editorial::wasm::render::refresh_settings_popover();
                    crate::editorial::wasm::render::refresh_research_slots();
                    show_success("Haber kaynağı başarıyla eklendi.");
                    return;
                }
                with_state_mut(|s| s.is_saving_source = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error("Kaynak oluşturulamadı.");
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_saving_source = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Kaynak eklenirken bir hata oluştu.");
                show_error(err);
            }
            Err(_) => {
                with_state_mut(|s| s.is_saving_source = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error("Bağlantı hatası.");
            }
        }
    });
}

pub fn update_research_source() {
    let (id_opt, name, url, cat, scope, enabled, tok_opt, busy) = with_state(|s| {
        (
            s.editing_source_id,
            s.edit_source_name.trim().to_string(),
            s.edit_source_url.trim().to_string(),
            if s.edit_source_category.trim().is_empty() {
                "Gündem".to_string()
            } else {
                s.edit_source_category.trim().to_string()
            },
            if s.edit_source_scope.trim().eq_ignore_ascii_case("global") {
                "global".to_string()
            } else {
                "tr".to_string()
            },
            s.edit_source_enabled,
            s.token.clone(),
            s.is_saving_source,
        )
    });
    if busy {
        return;
    }
    let Some(id) = id_opt else { return };
    if url.is_empty() {
        show_error("Kaynak adresi boş bırakılamaz.");
        return;
    }
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| s.is_saving_source = true);
    crate::editorial::wasm::render::refresh_settings_popover();

    spawn_local(async move {
        let req_url = format!("/api/v1/admin/editorial/research/sources/{id}");
        let body = json!({
            "name": name,
            "url": url,
            "category": cat,
            "scope": scope,
            "enabled": enabled,
        });
        match api_json("PUT", &req_url, Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                if let Some(src) = data
                    .get("source")
                    .and_then(|v| serde_json::from_value::<crate::editorial::common::ResearchSource>(v.clone()).ok())
                {
                    with_state_mut(|s| {
                        s.is_saving_source = false;
                        s.editing_source_id = None;
                        if let Some(pos) = s.research_sources.iter().position(|x| x.id == id) {
                            s.research_sources[pos] = src;
                        }
                    });
                    crate::editorial::wasm::render::refresh_settings_popover();
                    crate::editorial::wasm::render::refresh_research_slots();
                    show_success("Kaynak güncellendi.");
                    return;
                }
                with_state_mut(|s| s.is_saving_source = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error("Kaynak güncellenemedi.");
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_saving_source = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Kaynak güncellenemedi.");
                show_error(err);
            }
            Err(_) => {
                with_state_mut(|s| s.is_saving_source = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error("Bağlantı hatası.");
            }
        }
    });
}

pub fn toggle_research_source(id: i32) {
    let (cur_opt, tok_opt) = with_state(|s| {
        (
            s.research_sources.iter().find(|x| x.id == id).cloned(),
            s.token.clone(),
        )
    });
    let Some(cur) = cur_opt else { return };
    let Some(tok) = tok_opt else { return };

    let new_enabled = !cur.enabled;
    with_state_mut(|s| {
        if let Some(src) = s.research_sources.iter_mut().find(|x| x.id == id) {
            src.enabled = new_enabled;
        }
    });
    crate::editorial::wasm::render::refresh_settings_popover();
    crate::editorial::wasm::render::refresh_research_slots();

    spawn_local(async move {
        let req_url = format!("/api/v1/admin/editorial/research/sources/{id}");
        let body = json!({ "enabled": new_enabled });
        let _ = api_json("PUT", &req_url, Some(&tok), Some(&body)).await;
    });
}

pub fn delete_research_source(id: i32) {
    if !confirm_dialog("Bu haber kaynağını silmek istediğinize emin misiniz?") {
        return;
    }
    let Some(tok) = with_state(|s| s.token.clone()) else { return };

    spawn_local(async move {
        let req_url = format!("/api/v1/admin/editorial/research/sources/{id}");
        match api_json("DELETE", &req_url, Some(&tok), None).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                with_state_mut(|s| {
                    s.research_sources.retain(|x| x.id != id);
                    if s.editing_source_id == Some(id) {
                        s.editing_source_id = None;
                    }
                    if s.selected_research_source_id == Some(id) {
                        s.selected_research_source_id = None;
                    }
                });
                crate::editorial::wasm::render::refresh_settings_popover();
                crate::editorial::wasm::render::refresh_research_slots();
                show_success("Kaynak silindi.");
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Kaynak silinemedi.");
                show_error(err);
            }
            Err(_) => show_error("Bağlantı hatası."),
        }
    });
}

// ============================================================================
// Live Editing Conflict Lock (`/api/locks` & `/api/articles/:id/lock`)
// ============================================================================

fn other_locks_signature(s: &crate::editorial::wasm::state::AppState) -> String {
    let mut parts: Vec<String> = s
        .active_locks
        .iter()
        .filter(|l| s.is_other_user_lock(l))
        .map(|l| format!("{}:{}:{}", l.article_id, l.user_id, l.username))
        .collect();
    parts.sort();
    parts.join("|")
}

fn editor_conflict_signature(s: &crate::editorial::wasm::state::AppState) -> String {
    s.effective_editing_conflict()
        .map(|l| format!("{}:{}:{}", l.article_id, l.user_id, l.username))
        .unwrap_or_default()
}

pub fn start_lock_heartbeat_timer() {
    let seq = with_state_mut(|s| {
        s.lock_timer_seq += 1;
        s.lock_timer_seq
    });
    poll_locks_and_heartbeat();
    schedule_next_lock_tick(seq);
}

fn schedule_next_lock_tick(seq: u64) {
    set_timeout_ms(12_000, move || {
        let active = with_state(|s| s.lock_timer_seq == seq && s.token.is_some());
        if active {
            poll_locks_and_heartbeat();
            schedule_next_lock_tick(seq);
        }
    });
}

pub fn poll_locks_and_heartbeat() {
    let (tok_opt, editing_id) = with_state(|s| {
        (
            s.token.clone(),
            s.editing_article.id.trim().to_string(),
        )
    });
    let Some(tok) = tok_opt else { return };

    spawn_local(async move {
        if let Ok((true, _, data)) = api_json("GET", "/api/v1/admin/editorial/locks", Some(&tok), None).await {
            if let Some(locks_val) = data.get("locks") {
                if let Ok(locks) = serde_json::from_value::<Vec<ArticleLock>>(locks_val.clone()) {
                    let (left_diff, right_diff) = with_state_mut(|s| {
                        let prev_other = other_locks_signature(s);
                        let prev_conflict = editor_conflict_signature(s);
                        s.active_locks = locks;
                        if let Some(ref cur_c) = s.editing_conflict_lock {
                            let still_locked = s.active_locks.iter().any(|l| {
                                l.article_id == cur_c.article_id && s.is_other_user_lock(l)
                            });
                            if !still_locked {
                                s.editing_conflict_lock = None;
                            }
                        }
                        let next_other = other_locks_signature(s);
                        let next_conflict = editor_conflict_signature(s);
                        (prev_other != next_other, prev_conflict != next_conflict)
                    });
                    if left_diff {
                        crate::editorial::wasm::render::refresh_left_body();
                    }
                    if right_diff {
                        crate::editorial::wasm::render::refresh_editor_lock_banner();
                    }
                }
            }
        }

        if !editing_id.is_empty() && with_state(|s| s.token.is_some() && s.editing_article.id == editing_id) {
            let lock_url = format!("/api/v1/admin/editorial/articles/{}/lock", encode_param(&editing_id));
            if let Ok((true, _, data)) = api_json("POST", &lock_url, Some(&tok), None).await {
                let conflict = data
                    .get("conflict")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let locked_by = data
                    .get("lockedBy")
                    .and_then(|v| serde_json::from_value::<ArticleLock>(v.clone()).ok());
                let changed = with_state_mut(|s| {
                    if s.editing_article.id != editing_id {
                        return false;
                    }
                    let prev_sig = editor_conflict_signature(s);
                    if conflict {
                        s.editing_conflict_lock = locked_by;
                    } else {
                        s.editing_conflict_lock = None;
                    }
                    prev_sig != editor_conflict_signature(s)
                });
                if changed {
                    crate::editorial::wasm::render::refresh_editor_lock_banner();
                }
            }
        }
    });
}

pub fn acquire_article_lock(article_id: &str) {
    let clean_id = article_id.trim().to_string();
    if clean_id.is_empty() {
        let changed = with_state_mut(|s| s.editing_conflict_lock.take().is_some());
        if changed {
            crate::editorial::wasm::render::refresh_editor_lock_banner();
        }
        return;
    }
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };

    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/articles/{}/lock", encode_param(&clean_id));
        if let Ok((true, _, data)) = api_json("POST", &url, Some(&tok), None).await {
            let conflict = data
                .get("conflict")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let locked_by = data
                .get("lockedBy")
                .and_then(|v| serde_json::from_value::<ArticleLock>(v.clone()).ok());
            let changed = with_state_mut(|s| {
                if s.editing_article.id != clean_id {
                    return false;
                }
                let prev = editor_conflict_signature(s);
                if conflict {
                    s.editing_conflict_lock = locked_by;
                } else {
                    s.editing_conflict_lock = None;
                }
                prev != editor_conflict_signature(s)
            });
            if changed {
                crate::editorial::wasm::render::refresh_editor_lock_banner();
            }
        }
    });
}

pub fn release_article_lock(article_id: &str) {
    let clean_id = article_id.trim().to_string();
    if clean_id.is_empty() {
        return;
    }
    let tok_opt = with_state_mut(|s| {
        if s.editing_conflict_lock
            .as_ref()
            .map(|l| l.article_id == clean_id)
            .unwrap_or(false)
        {
            s.editing_conflict_lock = None;
        }
        let my_uid = s.user.as_ref().map(|u| u.id).unwrap_or(-1);
        s.active_locks
            .retain(|l| !(l.article_id == clean_id && l.user_id == my_uid));
        s.token.clone()
    });
    crate::editorial::wasm::render::refresh_editor_lock_banner();

    let Some(tok) = tok_opt else { return };
    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/articles/{}/lock", encode_param(&clean_id));
        let _ = api_json("DELETE", &url, Some(&tok), None).await;
    });
}

// ============================================================================
// Media Library Archive (`/api/media`)
// ============================================================================

pub fn open_media_library(target: String) {
    with_state_mut(|s| {
        s.media_picker_target = target;
        s.show_media_modal = true;
        s.is_loading_media = true;
    });
    crate::editorial::wasm::render::sync_media_modal_state(true);
    fetch_media_assets();
}

pub fn trigger_debounced_media_search() {
    let seq = with_state_mut(|s| {
        s.media_search_seq += 1;
        s.media_search_seq
    });
    set_timeout_ms(300, move || {
        let active = with_state(|s| s.media_search_seq == seq && s.show_media_modal);
        if active {
            fetch_media_assets();
        }
    });
}

pub fn fetch_media_assets() {
    let (tok_opt, q) = with_state(|s| (s.token.clone(), s.media_search_query.trim().to_string()));
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| s.is_loading_media = true);

    spawn_local(async move {
        let url = if q.is_empty() {
            "/api/v1/admin/editorial/media?offset=0&limit=40".to_string()
        } else {
            format!("/api/v1/admin/editorial/media?q={}&offset=0&limit=40", encode_param(&q))
        };
        if let Ok((true, _, data)) = api_json("GET", &url, Some(&tok), None).await {
            let assets = data
                .get("assets")
                .and_then(|v| serde_json::from_value::<Vec<MediaAsset>>(v.clone()).ok())
                .unwrap_or_default();
            let total = data
                .get("totalCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(assets.len() as u64) as usize;
            with_state_mut(|s| {
                s.media_assets = assets;
                s.media_total = total;
                s.is_loading_media = false;
            });
        } else {
            with_state_mut(|s| s.is_loading_media = false);
        }
        crate::editorial::wasm::render::sync_media_modal_state(true);
    });
}

pub fn delete_media_asset(asset_id: i32) {
    if !confirm_dialog("Bu görseli medya arşivinden kalıcı olarak silmek istediğinize emin misiniz?") {
        return;
    }
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };

    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/media/{asset_id}");
        match api_json("DELETE", &url, Some(&tok), None).await {
            Ok((true, _, _)) => {
                with_state_mut(|s| {
                    let prev = s.media_assets.len();
                    s.media_assets.retain(|a| a.id != asset_id);
                    if s.media_assets.len() < prev {
                        s.media_total = s.media_total.saturating_sub(1);
                    }
                });
                crate::editorial::wasm::render::sync_media_modal_state(true);
                show_success("Görsel arşivden silindi.");
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Görsel silinemedi.");
                show_error(err);
            }
            Err(_) => show_error("Bağlantı hatası."),
        }
    });
}

// ============================================================================
// Step 3 `#Tag` Autocomplete & `301` URL Redirects (`/api/tags`, `/api/redirects`)
// ============================================================================

pub fn trigger_debounced_tag_search() {
    let (seq, q) = with_state_mut(|s| {
        s.tag_debounce_seq += 1;
        (s.tag_debounce_seq, s.tag_input_query.trim().to_string())
    });
    set_timeout_ms(220, move || {
        let should_run = with_state(|s| s.tag_debounce_seq == seq && s.token.is_some());
        if should_run {
            fetch_tag_suggestions(&q);
        }
    });
}

pub fn fetch_tag_suggestions(query: &str) {
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };
    let q_clean = query.trim().trim_start_matches('#').to_string();

    spawn_local(async move {
        let url = if q_clean.is_empty() {
            "/api/v1/admin/editorial/tags".to_string()
        } else {
            format!("/api/v1/admin/editorial/tags?q={}", encode_param(&q_clean))
        };
        if let Ok((true, _, data)) = api_json("GET", &url, Some(&tok), None).await {
            if let Some(tags_val) = data.get("tags") {
                if let Ok(list) = serde_json::from_value::<Vec<TagSuggestion>>(tags_val.clone()) {
                    with_state_mut(|s| s.tag_suggestions = list);
                    if with_state(|s| s.editor_step == 3) {
                        crate::editorial::wasm::render::refresh_step3_tags_ui();
                    }
                }
            }
        }
    });
}

pub fn open_redirects_modal() {
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };
    with_state_mut(|s| {
        s.show_redirects_modal = true;
        s.is_loading_redirects = true;
    });
    crate::editorial::wasm::render::sync_redirects_modal_state();

    spawn_local(async move {
        if let Ok((true, _, data)) = api_json("GET", "/api/v1/admin/editorial/redirects", Some(&tok), None).await {
            let list = data
                .get("redirects")
                .and_then(|v| serde_json::from_value::<Vec<UrlRedirect>>(v.clone()).ok())
                .unwrap_or_default();
            with_state_mut(|s| {
                s.redirects = list;
                s.is_loading_redirects = false;
            });
        } else {
            with_state_mut(|s| s.is_loading_redirects = false);
        }
        crate::editorial::wasm::render::sync_redirects_modal_state();
    });
}

pub fn delete_redirect_rule(id: i32) {
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };
    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/redirects/{id}");
        match api_json("DELETE", &url, Some(&tok), None).await {
            Ok((true, _, data)) => {
                let server_list = data
                    .get("redirects")
                    .and_then(|v| serde_json::from_value::<Vec<UrlRedirect>>(v.clone()).ok());
                with_state_mut(|s| {
                    if let Some(fresh) = server_list {
                        s.redirects = fresh;
                    } else {
                        s.redirects.retain(|r| r.id != id);
                    }
                });
                crate::editorial::wasm::render::sync_redirects_modal_state();
                show_success("301 yönlendirme kuralı silindi.");
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Kural silinemedi.");
                show_error(err);
            }
            Err(_) => show_error("Bağlantı hatası."),
        }
    });
}

// ============================================================================
// Article Revision History (`/api/articles/:id/revisions`)
// ============================================================================

pub fn open_revisions_modal(article_id: String, article_title: String) {
    let aid = article_id.trim().to_string();
    if aid.is_empty() {
        return;
    }
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };

    with_state_mut(|s| {
        s.show_revisions_modal = true;
        s.revisions_article_id = aid.clone();
        s.revisions_article_title = article_title;
        s.revisions.clear();
        s.is_loading_revisions = true;
    });
    crate::editorial::wasm::render::sync_revisions_modal_state();

    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/articles/{}/revisions", encode_param(&aid));
        if let Ok((true, _, data)) = api_json("GET", &url, Some(&tok), None).await {
            let list = data
                .get("revisions")
                .and_then(|v| serde_json::from_value::<Vec<ArticleRevision>>(v.clone()).ok())
                .unwrap_or_default();
            with_state_mut(|s| {
                s.revisions = list;
                s.is_loading_revisions = false;
            });
        } else {
            with_state_mut(|s| s.is_loading_revisions = false);
        }
        crate::editorial::wasm::render::sync_revisions_modal_state();
    });
}

pub fn restore_article_revision(article_id: String, rev_id: i64) {
    let aid = article_id.trim().to_string();
    if aid.is_empty() {
        return;
    }
    if !confirm_dialog("Bu revizyon sürümüne geri dönmek istediğinize emin misiniz?") {
        return;
    }
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };

    with_state_mut(|s| s.is_restoring_revision = true);
    crate::editorial::wasm::render::sync_revisions_modal_state();

    spawn_local(async move {
        let url = format!(
            "/api/v1/admin/editorial/articles/{}/revisions/{rev_id}/restore",
            encode_param(&aid)
        );
        match api_json("POST", &url, Some(&tok), None).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                let maybe_art = data
                    .get("article")
                    .and_then(|v| serde_json::from_value::<Article>(v.clone()).ok());
                let mut updated_editor = false;
                with_state_mut(|s| {
                    s.is_restoring_revision = false;
                    s.show_revisions_modal = false;
                    if let Some(restored) = maybe_art {
                        for a in &mut s.published_articles {
                            if a.id == restored.id {
                                *a = restored.clone();
                            }
                        }
                        for d in &mut s.draft_articles {
                            if d.id == restored.id {
                                *d = restored.clone();
                            }
                        }
                        if s.selected_article
                            .as_ref()
                            .map(|sel| sel.id == restored.id)
                            .unwrap_or(false)
                        {
                            s.selected_article = Some(restored.clone());
                        }
                        if s.editing_article.id == restored.id {
                            let norm = normalize_draft_for_editor(restored);
                            save_editing_draft(&norm);
                            s.editing_article = norm;
                            s.reset_editor_lang_buffers();
                            let base_sig = editor_article_signature(&s.canonical_editing_article());
                            save_editing_baseline_sig(&base_sig);
                            s.editing_baseline_sig = base_sig;
                            s.crop_loaded_src.clear();
                            s.crop_image_el = None;
                            s.editor_image_error = false;
                            updated_editor = true;
                        }
                    }
                });
                crate::editorial::wasm::render::sync_revisions_modal_state();
                crate::editorial::wasm::render::refresh_left_panel();
                crate::editorial::wasm::render::refresh_middle_panel();
                if updated_editor {
                    crate::editorial::wasm::render::refresh_right_panel();
                }
                show_success("Seçilen sürüme başarıyla geri dönüldü!");
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_restoring_revision = false);
                crate::editorial::wasm::render::sync_revisions_modal_state();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Sürüm geri yüklenemedi.");
                show_error(err);
            }
            Err(_) => {
                with_state_mut(|s| s.is_restoring_revision = false);
                crate::editorial::wasm::render::sync_revisions_modal_state();
                show_error("Bağlantı hatası.");
            }
        }
    });
}

// ============================================================================
// Smart Clipboard Image Upload & Pasted Article Image Localization
// ============================================================================

fn update_image_block_url_in_state_and_dom(block_id: &str, new_url: &str) {
    let mid_prev = with_state_mut(|s| {
        if let Some(blocks) = &mut s.editing_article.blocks {
            for b in blocks.iter_mut() {
                if b.id == block_id && b.block_type == "image" {
                    b.content = new_url.to_string();
                }
            }
        }
        save_editing_draft(&s.editing_article);
        s.is_desktop && s.middle_preview_draft
    });

    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(inp) = doc
            .get_element_by_id(&format!("blk-in-{block_id}"))
            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        {
            inp.set_value(new_url);
        }
    }
    crate::editorial::wasm::render::refresh_single_block_image_preview(block_id, new_url);
    if mid_prev {
        crate::editorial::wasm::render::refresh_middle_panel();
    }
}

fn data_image_uri_to_blob(data_uri: &str) -> Option<(Blob, &'static str)> {
    let (header, b64) = data_uri.split_once(";base64,")?;
    let mime = header.strip_prefix("data:")?.trim();
    let ext_name = match mime {
        "image/png" => "pasted-image.png",
        "image/webp" => "pasted-image.webp",
        "image/gif" => "pasted-image.gif",
        _ => "pasted-image.jpg",
    };
    let bytes = decode_base64(b64)?;
    let u8_arr = js_sys::Uint8Array::from(bytes.as_slice());
    let parts = js_sys::Array::new();
    parts.push(&u8_arr.buffer());
    let bag = BlobPropertyBag::new();
    bag.set_type(mime);
    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &bag).ok()?;
    Some((blob, ext_name))
}

/// Uploads image files pasted directly from the system clipboard (`Ctrl+V` screenshot / copied image).
pub fn upload_pasted_clipboard_images(blocks_and_files: Vec<(String, File)>) {
    if blocks_and_files.is_empty() {
        return;
    }
    let wm = with_state(|s| s.upload_with_watermark);
    spawn_local(async move {
        let mut ok_cnt = 0usize;
        for (bid, file) in blocks_and_files {
            let fname = {
                let n = file.name();
                if n.trim().is_empty() {
                    "clipboard-image.png".to_string()
                } else {
                    n
                }
            };
            let blob: &Blob = file.as_ref();
            if let Some(info) = upload_media_blob(blob, &fname, wm).await {
                update_image_block_url_in_state_and_dom(&bid, &info.url);
                ok_cnt += 1;
            }
        }
        if ok_cnt > 0 {
            show_success(format!("{ok_cnt} görsel panodan yüklenip bloğa eklendi."));
        }
    });
}

/// Converts base64 `data:image/*` blocks and mirrors external `http(s)://` pasted article images
/// into optimized local `.webp` assets on our Rust server (non-blocking, silent fallback on external CORS/403).
pub fn localize_pasted_article_images(items: Vec<(String, String)>) {
    if items.is_empty() {
        return;
    }
    let (tok_opt, wm) = with_state(|s| (s.token.clone(), s.upload_with_watermark));
    let Some(tok) = tok_opt else { return };

    spawn_local(async move {
        for (bid, src) in items.into_iter().take(8) {
            if src.starts_with("data:image/") {
                if let Some((blob, fname)) = data_image_uri_to_blob(&src) {
                    if let Some(info) = upload_media_blob(&blob, fname, wm).await {
                        update_image_block_url_in_state_and_dom(&bid, &info.url);
                    }
                }
            } else if src.starts_with("https://") || src.starts_with("http://") {
                let body = json!({ "url": src, "watermark": wm });
                if let Ok((true, _, data)) =
                    api_json("POST", "/api/v1/admin/editorial/fetch-remote-image", Some(&tok), Some(&body)).await
                {
                    if data.get("success").and_then(|v| v.as_bool()) == Some(true) {
                        if let Some(info) = parse_uploaded_media_info(&data) {
                            if let Some(ref asset) = info.asset {
                                upsert_media_asset_in_state(asset.clone());
                            }
                            update_image_block_url_in_state_and_dom(&bid, &info.url);
                        }
                    }
                }
            }
        }
    });
}

// ============================================================================
// Research Desk & Clean Multi-Source Reader API (`/api/research/search`, `/api/research/read`)
// ============================================================================

/// Handles submitting `#hf-research-input`:
/// - Direct `http://` or `https://` URL -> immediately opens Clean Reader in Middle Column!
/// - Topic / keyword (or empty for live breaking headlines) -> searches `/api/research/search`.
pub fn submit_research_query() {
    let raw = with_state(|s| s.research_query.trim().to_string());
    let low = raw.to_ascii_lowercase();
    if low.starts_with("https://") || low.starts_with("http://") {
        open_research_url(
            raw,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
    } else {
        fetch_research_results();
    }
}

pub fn fetch_research_results() {
    let (tok_opt, q, scope, time_f, source_id_opt, category, need_sources) = with_state(|s| {
        (
            s.token.clone(),
            s.research_query.trim().to_string(),
            if s.research_scope == "global" {
                "global".to_string()
            } else {
                "tr".to_string()
            },
            match s.research_time.as_str() {
                "1h" | "24h" | "7d" => s.research_time.clone(),
                _ => "all".to_string(),
            },
            s.selected_research_source_id,
            s.research_category.trim().to_string(),
            s.research_sources.is_empty(),
        )
    });
    let Some(tok) = tok_opt else { return };

    if need_sources {
        fetch_research_sources();
    }

    with_state_mut(|s| {
        s.is_searching_research = true;
        s.research_loaded_once = true;
    });
    crate::editorial::wasm::render::refresh_research_slots();

    spawn_local(async move {
        let mut url = format!(
            "/api/v1/admin/editorial/research/search?q={}&scope={}&time={}",
            encode_param(&q),
            encode_param(&scope),
            encode_param(&time_f)
        );
        if let Some(sid) = source_id_opt {
            url.push_str(&format!("&sourceId={sid}"));
        }
        if !category.is_empty() && !category.eq_ignore_ascii_case("all") {
            url.push_str(&format!("&cat={}", encode_param(&category)));
        }
        match api_json("GET", &url, Some(&tok), None).await {
            Ok((ok, _, data)) if ok && data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                let items = data
                    .get("items")
                    .and_then(|v| {
                        serde_json::from_value::<Vec<crate::editorial::common::ResearchSearchItem>>(v.clone())
                            .ok()
                    })
                    .unwrap_or_default();
                with_state_mut(|s| {
                    s.research_results = items;
                    s.is_searching_research = false;
                });
                crate::editorial::wasm::render::refresh_research_slots();
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_searching_research = false);
                crate::editorial::wasm::render::refresh_research_slots();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Canlı araştırma sonuçları alınamadı.");
                show_error(err);
            }
            Err(e) => {
                with_state_mut(|s| s.is_searching_research = false);
                crate::editorial::wasm::render::refresh_research_slots();
                show_error(e);
            }
        }
    });
}

pub fn open_research_url(
    raw_url: String,
    fallback_title: String,
    fallback_source: String,
    fallback_snippet: String,
    fallback_image: String,
) {
    let Some(clean_url) = crate::editorial::wasm::render_research::safe_http_url(&raw_url) else {
        show_error("Lütfen geçerli bir https:// haber bağlantısı girin.");
        return;
    };
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };

    let (should_fetch, is_desktop) = with_state_mut(|s| {
        s.middle_showing_research = true;
        s.middle_preview_draft = false;
        s.active_research_url = Some(clean_url.clone());
        s.mobile_view = MobileView::Middle;

        if let Some(existing) = s.research_tabs.iter_mut().find(|t| t.url == clean_url) {
            if existing.extracted.is_some() || existing.is_loading {
                return (false, s.is_desktop);
            }
            existing.is_loading = true;
            existing.error = None;
            if !fallback_title.trim().is_empty() {
                existing.title_hint = fallback_title.trim().to_string();
            }
            if !fallback_source.trim().is_empty() {
                existing.source_hint = fallback_source.trim().to_string();
            }
            return (true, s.is_desktop);
        }

        // Enforce up to 6 open source tabs for multi-source fact-checking
        if s.research_tabs.len() >= 6 {
            let remove_idx = s
                .research_tabs
                .iter()
                .position(|t| t.url != clean_url)
                .unwrap_or(0);
            s.research_tabs.remove(remove_idx);
        }

        let source_hint = if fallback_source.trim().is_empty() {
            crate::editorial::wasm::state::get_site_name(&clean_url)
        } else {
            fallback_source.trim().to_string()
        };
        let title_hint = if fallback_title.trim().is_empty() {
            clean_url.clone()
        } else {
            fallback_title.trim().to_string()
        };

        s.research_tabs.push(crate::editorial::wasm::state::ResearchReaderTab {
            url: clean_url.clone(),
            title_hint,
            source_hint,
            is_loading: true,
            error: None,
            extracted: None,
        });
        (true, s.is_desktop)
    });

    if is_desktop {
        crate::editorial::wasm::render::refresh_research_slots();
        crate::editorial::wasm::render::refresh_middle_panel();
    } else {
        crate::editorial::wasm::render::render_full();
    }

    if !should_fetch {
        return;
    }

    spawn_local(async move {
        let body = json!({
            "url": clean_url,
            "fallbackTitle": fallback_title,
            "fallbackSnippet": fallback_snippet,
            "fallbackSource": fallback_source,
            "fallbackImage": fallback_image,
        });
        match api_json("POST", "/api/v1/admin/editorial/research/read", Some(&tok), Some(&body)).await {
            Ok((ok, _, data)) if ok && data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                let maybe_art = data.get("article").and_then(|v| {
                    serde_json::from_value::<crate::editorial::common::ExtractedReaderArticle>(v.clone()).ok()
                });
                with_state_mut(|s| {
                    if let Some(tab) = s.research_tabs.iter_mut().find(|t| t.url == clean_url) {
                        tab.is_loading = false;
                        if let Some(art) = maybe_art {
                            if !art.title.trim().is_empty() {
                                tab.title_hint = art.title.trim().to_string();
                            }
                            if !art.site_name.trim().is_empty() {
                                tab.source_hint = art.site_name.trim().to_string();
                            }
                            tab.error = None;
                            tab.extracted = Some(art);
                        } else {
                            tab.error = Some("Haber metni çözümlenemedi.".to_string());
                        }
                    }
                });
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Kaynak haber sayfası okunamadı.")
                    .to_string();
                with_state_mut(|s| {
                    if let Some(tab) = s.research_tabs.iter_mut().find(|t| t.url == clean_url) {
                        tab.is_loading = false;
                        tab.error = Some(err);
                    }
                });
            }
            Err(e) => {
                with_state_mut(|s| {
                    if let Some(tab) = s.research_tabs.iter_mut().find(|t| t.url == clean_url) {
                        tab.is_loading = false;
                        tab.error = Some(e);
                    }
                });
            }
        }
        if with_state(|s| s.middle_showing_research && !s.middle_preview_draft) {
            crate::editorial::wasm::render::refresh_middle_panel();
        }
    });
}

// ============================================================================
// MSI Dergisi / MSI TDR Editorial Metadata, Issues & Glossary API
// ============================================================================

pub fn fetch_editorial_meta() {
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };
    spawn_local(async move {
        if let Ok((true, _, data)) = api_json("GET", "/api/v1/admin/editorial/meta", Some(&tok), None).await
        {
            let ssb_tags = data
                .get("ssbTags")
                .and_then(|v| serde_json::from_value::<Vec<SsbTag>>(v.clone()).ok());
            let issues = data
                .get("issues")
                .and_then(|v| serde_json::from_value::<Vec<MagazineIssueItem>>(v.clone()).ok());
            let glossary = data
                .get("glossary")
                .and_then(|v| serde_json::from_value::<Vec<GlossaryRule>>(v.clone()).ok());

            let (step, settings_open, has_sel) = with_state_mut(|s| {
                if let Some(tags) = ssb_tags {
                    s.ssb_tags_catalog = tags;
                }
                if let Some(iss) = issues {
                    s.magazine_issues = iss;
                }
                if let Some(gl) = glossary {
                    s.glossary_rules = gl;
                }
                (
                    s.editor_step,
                    s.show_settings_popover,
                    s.selected_article.is_some() || s.middle_preview_draft,
                )
            });

            if settings_open {
                crate::editorial::wasm::render::refresh_settings_popover();
            }
            if step == 1 {
                crate::editorial::wasm::render::refresh_step1_glossary_slots();
            } else if step == 4 {
                crate::editorial::wasm::render::refresh_right_panel();
            }
            if has_sel {
                crate::editorial::wasm::render::refresh_middle_panel();
            }
        }
    });
}

pub fn create_magazine_issue() {
    let (
        num_str,
        lang,
        title_tr,
        title_en,
        month,
        year_str,
        is_special,
        event_name,
        tok_opt,
        busy,
    ) = with_state(|s| {
        (
            s.new_issue_number.trim().to_string(),
            if s.new_issue_lang.trim().eq_ignore_ascii_case("en") {
                "en".to_string()
            } else {
                "tr".to_string()
            },
            s.new_issue_title_tr.trim().to_string(),
            s.new_issue_title_en.trim().to_string(),
            s.new_issue_month.trim().to_string(),
            s.new_issue_year.trim().to_string(),
            s.new_issue_special,
            s.new_issue_event.trim().to_string(),
            s.token.clone(),
            s.is_saving_issue,
        )
    });
    if busy {
        return;
    }
    let Ok(issue_number) = num_str.parse::<i32>() else {
        show_error("Lütfen geçerli bir sayı numarası girin (Örn: 287).");
        return;
    };
    if issue_number <= 0 {
        show_error("Dergi sayı numarası 0'dan büyük olmalıdır.");
        return;
    }
    let year = year_str.parse::<i32>().unwrap_or(2026);
    let final_title_tr = if title_tr.is_empty() {
        if lang == "en" {
            format!("MSI TDR Sayı {issue_number}")
        } else {
            format!("MSI Dergisi Sayı {issue_number}")
        }
    } else {
        title_tr
    };
    let final_title_en = if title_en.is_empty() {
        format!("MSI TDR Issue {issue_number}")
    } else {
        title_en
    };
    let final_month = if month.is_empty() {
        "Kasım".to_string()
    } else {
        month
    };
    let Some(tok) = tok_opt else { return };

    with_state_mut(|s| s.is_saving_issue = true);
    crate::editorial::wasm::render::refresh_settings_popover();

    spawn_local(async move {
        let body = json!({
            "issueNumber": issue_number,
            "editionLang": lang,
            "titleTr": final_title_tr,
            "titleEn": final_title_en,
            "publishMonth": final_month,
            "year": year,
            "isSpecialIssue": is_special,
            "eventName": if event_name.is_empty() { Value::Null } else { Value::String(event_name) },
        });
        match api_json("POST", "/api/v1/admin/editorial/issues", Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                let list = data
                    .get("issues")
                    .and_then(|v| serde_json::from_value::<Vec<MagazineIssueItem>>(v.clone()).ok());
                let step = with_state_mut(|s| {
                    s.is_saving_issue = false;
                    if let Some(iss) = list {
                        s.magazine_issues = iss;
                    }
                    s.new_issue_number.clear();
                    s.new_issue_title_tr.clear();
                    s.new_issue_title_en.clear();
                    s.new_issue_month.clear();
                    s.new_issue_special = false;
                    s.new_issue_event.clear();
                    s.editor_step
                });
                crate::editorial::wasm::render::refresh_settings_popover();
                if step == 4 {
                    crate::editorial::wasm::render::refresh_right_panel();
                }
                show_success("Dergi sayısı kaydedildi.");
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_saving_issue = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Dergi sayısı kaydedilemedi.");
                show_error(err);
            }
            Err(_) => {
                with_state_mut(|s| s.is_saving_issue = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error("Bağlantı hatası.");
            }
        }
    });
}

pub fn delete_magazine_issue(id: i32) {
    if !confirm_dialog("Bu dergi sayısını listeden silmek istediğinize emin misiniz?") {
        return;
    }
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };
    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/issues/{id}");
        match api_json("DELETE", &url, Some(&tok), None).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                let list = data
                    .get("issues")
                    .and_then(|v| serde_json::from_value::<Vec<MagazineIssueItem>>(v.clone()).ok());
                let step = with_state_mut(|s| {
                    if let Some(iss) = list {
                        s.magazine_issues = iss;
                    } else {
                        s.magazine_issues.retain(|x| x.id != id);
                    }
                    s.editor_step
                });
                crate::editorial::wasm::render::refresh_settings_popover();
                if step == 4 {
                    crate::editorial::wasm::render::refresh_right_panel();
                }
                show_success("Dergi sayısı silindi.");
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Dergi sayısı silinemedi.");
                show_error(err);
            }
            Err(_) => show_error("Bağlantı hatası."),
        }
    });
}

pub fn create_glossary_rule() {
    let (
        term_tr,
        abbr_tr,
        preferred_en,
        abbr_en,
        forbidden_raw,
        category,
        rule_type,
        case_sensitive,
        note,
        tok_opt,
        busy,
    ) = with_state(|s| {
        (
            s.new_glossary_term_tr.trim().to_string(),
            s.new_glossary_abbr_tr.trim().to_string(),
            s.new_glossary_preferred_en.trim().to_string(),
            s.new_glossary_abbr_en.trim().to_string(),
            s.new_glossary_forbidden.clone(),
            s.new_glossary_category.trim().to_string(),
            s.new_glossary_rule_type.trim().to_string(),
            s.new_glossary_case_sensitive,
            s.new_glossary_note.trim().to_string(),
            s.token.clone(),
            s.is_saving_glossary,
        )
    });
    if busy {
        return;
    }
    if term_tr.is_empty() && preferred_en.is_empty() {
        show_error("Lütfen Türkçe terim ve onaylı İngilizce karşılığını girin.");
        return;
    }
    if preferred_en.is_empty() {
        show_error("Lütfen onaylı İngilizce (MSI TDR / NATO) karşılığı girin.");
        return;
    }
    let Some(tok) = tok_opt else { return };

    let forbidden_variants: Vec<String> = forbidden_raw
        .split([',', ';', '\n'])
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();

    with_state_mut(|s| s.is_saving_glossary = true);
    crate::editorial::wasm::render::refresh_settings_popover();

    spawn_local(async move {
        let rule = GlossaryRule {
            id: 0,
            term_tr: if term_tr.is_empty() {
                preferred_en.clone()
            } else {
                term_tr
            },
            term_en: preferred_en.clone(),
            abbr_tr: (!abbr_tr.is_empty()).then_some(abbr_tr),
            abbr_en: (!abbr_en.is_empty()).then_some(abbr_en),
            category: if category.is_empty() {
                "institutional".to_string()
            } else {
                category
            },
            rule_type: if rule_type.is_empty() {
                "fixed".to_string()
            } else {
                rule_type
            },
            preferred_en,
            forbidden_variants,
            case_sensitive,
            note: (!note.is_empty()).then_some(note),
        };
        let Ok(body) = serde_json::to_value(&rule) else {
            with_state_mut(|s| s.is_saving_glossary = false);
            return;
        };
        match api_json("POST", "/api/v1/admin/editorial/glossary", Some(&tok), Some(&body)).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                let list = data
                    .get("glossary")
                    .and_then(|v| serde_json::from_value::<Vec<GlossaryRule>>(v.clone()).ok());
                let step = with_state_mut(|s| {
                    s.is_saving_glossary = false;
                    if let Some(gl) = list {
                        s.glossary_rules = gl;
                    }
                    s.new_glossary_term_tr.clear();
                    s.new_glossary_abbr_tr.clear();
                    s.new_glossary_preferred_en.clear();
                    s.new_glossary_abbr_en.clear();
                    s.new_glossary_forbidden.clear();
                    s.new_glossary_note.clear();
                    s.new_glossary_case_sensitive = false;
                    s.editor_step
                });
                crate::editorial::wasm::render::refresh_settings_popover();
                if step == 1 {
                    crate::editorial::wasm::render::refresh_step1_glossary_slots();
                }
                show_success("Terminoloji kuralı eklendi.");
            }
            Ok((_, _, data)) => {
                with_state_mut(|s| s.is_saving_glossary = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Terminoloji kuralı eklenemedi.");
                show_error(err);
            }
            Err(_) => {
                with_state_mut(|s| s.is_saving_glossary = false);
                crate::editorial::wasm::render::refresh_settings_popover();
                show_error("Bağlantı hatası.");
            }
        }
    });
}

pub fn delete_glossary_rule(id: i32) {
    if !confirm_dialog("Bu terminoloji kuralını silmek istediğinize emin misiniz?") {
        return;
    }
    let Some(tok) = with_state(|s| s.token.clone()) else {
        return;
    };
    spawn_local(async move {
        let url = format!("/api/v1/admin/editorial/glossary/{id}");
        match api_json("DELETE", &url, Some(&tok), None).await {
            Ok((true, _, data)) if data.get("success").and_then(|v| v.as_bool()) == Some(true) => {
                let list = data
                    .get("glossary")
                    .and_then(|v| serde_json::from_value::<Vec<GlossaryRule>>(v.clone()).ok());
                let step = with_state_mut(|s| {
                    if let Some(gl) = list {
                        s.glossary_rules = gl;
                    } else {
                        s.glossary_rules.retain(|x| x.id != id);
                    }
                    s.editor_step
                });
                crate::editorial::wasm::render::refresh_settings_popover();
                if step == 1 {
                    crate::editorial::wasm::render::refresh_step1_glossary_slots();
                }
                show_success("Terminoloji kuralı silindi.");
            }
            Ok((_, _, data)) => {
                let err = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Kural silinemedi.");
                show_error(err);
            }
            Err(_) => show_error("Bağlantı hatası."),
        }
    });
}
