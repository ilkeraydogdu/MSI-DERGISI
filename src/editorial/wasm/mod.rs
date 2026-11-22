//! MSI Dergisi Editorial Studio Pure Rust WebAssembly Client.

pub mod api;
pub mod cropper;
pub mod icons;
pub mod paste_parser;
pub mod render;
pub mod render_left;
pub mod render_middle;
pub mod render_modals;
pub mod render_research;
pub mod render_right;
pub mod rich_editor;
pub mod state;

use crate::editorial::common::{
    sanitize_inline_rich_html, slugify_turkish, ContentBlock, GalleryItem, RelatedArticleRef,
    SeoData,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    ClipboardEvent, Element, Event, File, HtmlElement, HtmlInputElement, HtmlSelectElement,
    HtmlTextAreaElement, KeyboardEvent, MessageEvent, MouseEvent,
};

use crate::editorial::wasm::cropper::{redraw_cropper_canvas, CROP_HEIGHT, CROP_WIDTH};
use crate::editorial::wasm::render::{
    apply_panel_widths_to_dom, auto_fit_textarea, refresh_attached_source_slot,
    refresh_blocks_list_only, refresh_header_banner, refresh_left_body,
    refresh_middle_panel,
    refresh_modals, refresh_related_search_results, refresh_research_slots, refresh_right_panel,
    refresh_settings_glossary_list_slot, refresh_settings_popover,
    refresh_single_block_embed_preview, refresh_single_block_image_preview,
    refresh_step1_glossary_slots, refresh_step3_live_seo, refresh_step3_tags_ui, refresh_toasts,
    render_full, sync_editor_confirm_modal_state, sync_left_toolbar_state, sync_media_modal_state,
    sync_preview_modal_state, sync_redirects_modal_state, sync_revisions_modal_state,
    sync_settings_popover_state,
};
use crate::editorial::wasm::state::{
    clear_editing_draft, datetime_local_to_iso, empty_editing_article, js_now_hhmm,
    load_editing_baseline_sig, load_editing_draft, load_news_view_mode, load_panel_widths,
    load_stashed_tr_buffer, save_editing_draft, save_news_view_mode, save_panel_widths,
    set_article_image, set_article_source_url, should_confirm_editor_discard, turkish_lowercase,
    with_state, with_state_mut, LeftTab, MobileView, NewsViewMode, PanelWidths,
    PendingEditorConfirm, PreviewDevice, ResizeDrag, ResizeHandleKind, SettingsTab,
    PLACEHOLDER_IMG,
};

fn target_element(ev: &Event) -> Option<Element> {
    let target = ev.target()?;
    if let Some(el) = target.dyn_ref::<Element>() {
        return Some(el.clone());
    }
    target.dyn_ref::<web_sys::Node>()?.parent_element()
}

fn current_viewport_desktop() -> bool {
    web_sys::window()
        .and_then(|w| w.inner_width().ok())
        .and_then(|v| v.as_f64())
        .map(|w| w >= 1024.0)
        .unwrap_or(true)
}

fn set_button_disabled(id: &str, disabled: bool) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(el) = doc.get_element_by_id(id) {
            if disabled {
                let _ = el.set_attribute("disabled", "");
            } else {
                let _ = el.remove_attribute("disabled");
            }
        }
    }
}

pub fn boot_client_runtime() {
    with_state_mut(|s| {
        s.is_desktop = current_viewport_desktop();
        s.panel_widths = load_panel_widths();
        s.news_view_mode = load_news_view_mode();
        s.editing_article = load_editing_draft();
        if let Some((lang, stashed)) = load_stashed_tr_buffer() {
            s.editor_lang = lang;
            s.stashed_tr_title = stashed.title;
            s.stashed_tr_spot = stashed.spot;
            s.stashed_tr_blocks = stashed.blocks;
        }
        if s.editing_article
            .author_name
            .as_deref()
            .map_or(false, |v| !v.trim().is_empty())
            || s.editing_article
                .author_title
                .as_deref()
                .map_or(false, |v| !v.trim().is_empty())
            || s.editing_article
                .video_url
                .as_deref()
                .map_or(false, |v| !v.trim().is_empty())
        {
            s.show_author_meta_bar = true;
        }
        s.editing_baseline_sig = load_editing_baseline_sig();
    });

    register_global_listeners();
    render_full();
    api::init_auth_session();
}

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(target_arch = "wasm32")]
    boot_client_runtime();
}

fn register_global_listeners() {
    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };

    // 1. Delegated Click Handler
    let on_click = Closure::wrap(Box::new(move |ev: MouseEvent| {
        let Some(el) = target_element(ev.as_ref()) else {
            return;
        };
        let Ok(Some(act_el)) = el.closest("[data-action]") else {
            return;
        };
        let action = act_el.get_attribute("data-action").unwrap_or_default();
        handle_action_click(&action, &act_el, &ev);
    }) as Box<dyn FnMut(MouseEvent)>);
    let _ = doc.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());
    on_click.forget();

    // 2. Form Submit (Login & Research Search)
    let on_submit = Closure::wrap(Box::new(move |ev: Event| {
        if let Some(el) = target_element(&ev) {
            if el.id() == "hf-login-form" {
                ev.prevent_default();
                api::submit_login();
            } else if el.id() == "hf-research-form" {
                ev.prevent_default();
                api::submit_research_query();
            }
        }
    }) as Box<dyn FnMut(Event)>);
    let _ = doc.add_event_listener_with_callback("submit", on_submit.as_ref().unchecked_ref());
    on_submit.forget();

    // 3. Focus-Safe Input Handler
    let on_input = Closure::wrap(Box::new(move |ev: Event| {
        let Some(el) = target_element(&ev) else { return };
        if let Some(kind) = el.get_attribute("data-input") {
            handle_input_event(&kind, &el);
        }
    }) as Box<dyn FnMut(Event)>);
    let _ = doc.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref());
    on_input.forget();

    // 4. Select / Checkbox / File Change Handler
    let on_change = Closure::wrap(Box::new(move |ev: Event| {
        let Some(el) = target_element(&ev) else { return };
        if let Some(file_kind) = el.get_attribute("data-file") {
            handle_file_input(&file_kind, &el);
            return;
        }
        if let Some(chg_kind) = el.get_attribute("data-change") {
            handle_change_event(&chg_kind, &el);
        }
    }) as Box<dyn FnMut(Event)>);
    let _ = doc.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
    on_change.forget();

    // 5. Textarea Focus Auto-Fit
    let on_focusin = Closure::wrap(Box::new(move |ev: Event| {
        let Some(el) = target_element(&ev) else { return };
        let inp = el.get_attribute("data-input").unwrap_or_default();
        if inp == "editor-title"
            || inp == "editor-spot"
            || inp == "live-quick-body"
            || inp == "block-content"
        {
            if let Ok(ta) = el.dyn_into::<HtmlTextAreaElement>() {
                auto_fit_textarea(&ta);
            }
        }
    }) as Box<dyn FnMut(Event)>);
    let _ = doc.add_event_listener_with_callback("focusin", on_focusin.as_ref().unchecked_ref());
    on_focusin.forget();

    // 6. Keydown Shortcuts (Rich Editor shortcuts, Escape to close modals, Tag Enter/comma, Step 2 remote URL)
    let on_keydown = Closure::wrap(Box::new(move |ev: KeyboardEvent| {
        if let Some(el) = target_element(ev.as_ref()) {
            if rich_editor::handle_rich_keydown(&el, &ev) {
                return;
            }
        }
        if ev.key() == "Escape" {
            let closed_confirm = with_state_mut(|s| {
                if s.pending_editor_confirm.is_some() {
                    s.pending_editor_confirm = None;
                    true
                } else {
                    false
                }
            });
            if closed_confirm {
                ev.prevent_default();
                sync_editor_confirm_modal_state();
                return;
            }
            let closed_preview = with_state_mut(|s| {
                if s.show_preview_modal {
                    s.show_preview_modal = false;
                    true
                } else {
                    false
                }
            });
            if closed_preview {
                ev.prevent_default();
                sync_preview_modal_state();
                return;
            }
            let (closed_media, closed_redir, closed_revs, closed_settings, closed_pw) =
                with_state_mut(|s| {
                    if s.show_media_modal {
                        s.show_media_modal = false;
                        (true, false, false, false, false)
                    } else if s.show_redirects_modal {
                        s.show_redirects_modal = false;
                        (false, true, false, false, false)
                    } else if s.show_revisions_modal {
                        s.show_revisions_modal = false;
                        (false, false, true, false, false)
                    } else if s.show_settings_popover {
                        s.show_settings_popover = false;
                        s.is_adding_user = false;
                        (false, false, false, true, false)
                    } else if s.show_change_password {
                        s.show_change_password = false;
                        s.change_pw_current.clear();
                        s.change_pw_new.clear();
                        s.change_pw_confirm.clear();
                        (false, false, false, false, true)
                    } else {
                        (false, false, false, false, false)
                    }
                });
            if closed_media {
                ev.prevent_default();
                sync_media_modal_state(false);
                return;
            }
            if closed_redir {
                ev.prevent_default();
                sync_redirects_modal_state();
                return;
            }
            if closed_revs {
                ev.prevent_default();
                sync_revisions_modal_state();
                return;
            }
            if closed_settings {
                ev.prevent_default();
                sync_left_toolbar_state();
                sync_settings_popover_state(false);
                return;
            }
            if closed_pw {
                ev.prevent_default();
                refresh_modals();
                return;
            }
        }

        // Global Tab Navigation Shortcuts (Alt+Left/Right, Alt+1..6, Alt+W, Alt+E, Alt+R, Alt+Y/T/A)
        if ev.alt_key() {
            let key = ev.key();
            if key == "ArrowLeft" {
                ev.prevent_default();
                let target_opt = with_state_mut(|s| s.prev_research_tab());
                if let Some(url) = target_opt {
                    switch_to_research_url(url);
                }
                return;
            }
            if key == "ArrowRight" {
                ev.prevent_default();
                let target_opt = with_state_mut(|s| s.next_research_tab());
                if let Some(url) = target_opt {
                    switch_to_research_url(url);
                }
                return;
            }
            if let Ok(num) = key.parse::<usize>() {
                if (1..=6).contains(&num) {
                    let target_opt = with_state(|s| {
                        s.research_tabs.get(num - 1).map(|t| t.url.clone())
                    });
                    if let Some(url) = target_opt {
                        ev.prevent_default();
                        switch_to_research_url(url);
                        return;
                    }
                }
            }
            if key.eq_ignore_ascii_case("w") {
                let active_url_opt = with_state(|s| s.active_research_url.clone());
                if let Some(url) = active_url_opt {
                    ev.prevent_default();
                    with_state_mut(|s| {
                        s.research_tabs.retain(|t| t.url != url);
                        if s.research_tabs.is_empty() {
                            s.active_research_url = None;
                            s.middle_showing_research = false;
                            s.show_research_tabs_menu = false;
                        } else {
                            s.active_research_url = s.research_tabs.last().map(|t| t.url.clone());
                        }
                    });
                    refresh_research_slots();
                    refresh_middle_panel();
                    return;
                }
            }
            if key.eq_ignore_ascii_case("e") {
                ev.prevent_default();
                with_state_mut(|s| {
                    s.middle_showing_research = false;
                    s.show_research_tabs_menu = false;
                });
                refresh_middle_panel();
                return;
            }
            if key.eq_ignore_ascii_case("r") {
                let has_tabs = with_state(|s| !s.research_tabs.is_empty());
                if has_tabs {
                    ev.prevent_default();
                    with_state_mut(|s| {
                        s.middle_showing_research = true;
                        s.middle_preview_draft = false;
                    });
                    refresh_middle_panel();
                    return;
                }
            }
            if key.eq_ignore_ascii_case("y") {
                ev.prevent_default();
                with_state_mut(|s| s.left_tab = LeftTab::Published);
                sync_left_toolbar_state();
                crate::editorial::wasm::render::refresh_left_panel();
                return;
            }
            if key.eq_ignore_ascii_case("t") {
                ev.prevent_default();
                with_state_mut(|s| s.left_tab = LeftTab::Drafts);
                sync_left_toolbar_state();
                crate::editorial::wasm::render::refresh_left_panel();
                return;
            }
            if key.eq_ignore_ascii_case("a") {
                ev.prevent_default();
                with_state_mut(|s| s.left_tab = LeftTab::Research);
                sync_left_toolbar_state();
                crate::editorial::wasm::render::refresh_left_panel();
                return;
            }
        }
        if let Some(el) = target_element(ev.as_ref()) {
            if el.id() == "hf-tag-input" && (ev.key() == "Enter" || ev.key() == ",") {
                ev.prevent_default();
                commit_current_tag_input();
                return;
            }
            if ev.key() == "Enter" && el.id() == "step2-remote-url-input" {
                ev.prevent_default();
                let u = with_state(|s| s.step2_remote_url.clone());
                api::fetch_remote_image_to_editor(u);
                return;
            }
            if ev.key() == "Enter"
                && (el.id() == "hf-new-cat-name" || el.id() == "hf-new-cat-slug")
            {
                ev.prevent_default();
                api::create_category();
                return;
            }
            if ev.key() == "Enter" && el.id() == "hf-research-input" {
                ev.prevent_default();
                let dom_val = read_input_value(&el);
                with_state_mut(|s| s.research_query = dom_val);
                api::submit_research_query();
                return;
            }
            if ev.key() == "Enter" && el.id() == "hf-edit-cat-name" {
                ev.prevent_default();
                api::update_category();
                return;
            }
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);
    let _ = doc.add_event_listener_with_callback("keydown", on_keydown.as_ref().unchecked_ref());
    on_keydown.forget();

    let on_keyup = Closure::wrap(Box::new(move |_ev: KeyboardEvent| {
        rich_editor::update_selection_bubble_from_dom();
        render_research::update_reader_quote_bubble_from_dom();
    }) as Box<dyn FnMut(KeyboardEvent)>);
    let _ = doc.add_event_listener_with_callback("keyup", on_keyup.as_ref().unchecked_ref());
    on_keyup.forget();

    let on_selectionchange = Closure::wrap(Box::new(move |_ev: Event| {
        rich_editor::update_selection_bubble_from_dom();
        render_research::update_reader_quote_bubble_from_dom();
    }) as Box<dyn FnMut(Event)>);
    let _ = doc.add_event_listener_with_callback(
        "selectionchange",
        on_selectionchange.as_ref().unchecked_ref(),
    );
    on_selectionchange.forget();

    let on_paste = Closure::wrap(Box::new(move |ev: ClipboardEvent| {
        let Some(el) = target_element(ev.as_ref()) else {
            return;
        };
        let handled = rich_editor::handle_rich_paste(&el, &ev);
        if handled && with_state(|s| s.editor_lang == "en") {
            refresh_step1_glossary_slots();
        }
    }) as Box<dyn FnMut(ClipboardEvent)>);
    let _ = doc.add_event_listener_with_callback("paste", on_paste.as_ref().unchecked_ref());
    on_paste.forget();

    // Auto-resize embedded X/Twitter Tweet iframes (`platform.twitter.com/embed/Tweet.html`) via postMessage
    let on_win_msg = Closure::wrap(Box::new(move |ev: MessageEvent| {
        let origin = ev.origin();
        if !origin.contains("twitter.com") && !origin.contains("x.com") {
            return;
        }
        let raw_json = if let Some(s) = ev.data().as_string() {
            s
        } else if let Ok(js_str) = js_sys::JSON::stringify(&ev.data()) {
            js_str.as_string().unwrap_or_default()
        } else {
            return;
        };
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw_json) else {
            return;
        };
        let Some(tw) = parsed.get("twttr.embed") else {
            return;
        };
        if tw.get("method").and_then(|m| m.as_str()) != Some("twttr.private.resize") {
            return;
        }
        let Some(first_param) = tw.get("params").and_then(|p| p.as_array()).and_then(|a| a.first())
        else {
            return;
        };
        let Some(h) = first_param.get("height").and_then(|v| v.as_f64()) else {
            return;
        };
        let safe_h = h.clamp(180.0, 2400.0) + 12.0;
        let tid_opt = first_param
            .get("data")
            .and_then(|d| d.get("tweet_id"))
            .and_then(|t| t.as_str());
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            let selector = match tid_opt {
                Some(tid) if !tid.is_empty() && tid.chars().all(|c| c.is_ascii_digit()) => {
                    format!("iframe[data-tweet-id='{tid}']")
                }
                _ => "iframe[data-tweet-id]".to_string(),
            };
            if let Ok(list) = document.query_selector_all(&selector) {
                for idx in 0..list.length() {
                    if let Some(Ok(ifr)) = list.item(idx).map(|n| n.dyn_into::<HtmlElement>()) {
                        let _ = ifr.style().set_property("height", &format!("{safe_h:.0}px"));
                        if let Some(Ok(wrap)) = ifr.parent_element().map(|p| p.dyn_into::<HtmlElement>()) {
                            let _ = wrap.style().set_property("min-height", &format!("{safe_h:.0}px"));
                        }
                    }
                }
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    let _ = win.add_event_listener_with_callback("message", on_win_msg.as_ref().unchecked_ref());
    on_win_msg.forget();

    let on_scroll_cap = Closure::wrap(Box::new(move |ev: Event| {
        if let Some(el) = target_element(&ev) {
            if el.id() == "right-scroll" {
                rich_editor::update_selection_bubble_from_dom();
            } else if el.id() == "middle-scroll" {
                render_research::update_reader_quote_bubble_from_dom();
            }
        }
    }) as Box<dyn FnMut(Event)>);
    let _ = doc.add_event_listener_with_callback_and_bool(
        "scroll",
        on_scroll_cap.as_ref().unchecked_ref(),
        true,
    );
    on_scroll_cap.forget();

    // 7. Pointer Dragging: Bubble Focus Guard, 3-Column Resize Handles, Block Reorder Grip & Cropper Pan
    let on_mousedown = Closure::wrap(Box::new(move |ev: MouseEvent| {
        if ev.button() != 0 {
            return;
        }
        let Some(el) = target_element(ev.as_ref()) else {
            return;
        };

        // 7.0 Floating Selection & Quote Bridge focus/selection guard
        if render_research::handle_reader_quote_bubble_mousedown(&el, &ev) {
            return;
        }
        if rich_editor::handle_bubble_mousedown(&el, &ev) {
            return;
        }

        // 7a. Column Resize Handle
        if let Ok(Some(handle)) = el.closest("[data-resize-handle]") {
            ev.prevent_default();
            let kind_str = handle.get_attribute("data-resize-handle").unwrap_or_default();
            let kind = if kind_str == "left-middle" {
                ResizeHandleKind::LeftMiddle
            } else {
                ResizeHandleKind::MiddleRight
            };
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(body) = doc.body() {
                    let _ = body.class_list().add_1("hf-resizing-cols");
                }
                if let Some(grp) = doc.get_element_by_id("hf-panel-group") {
                    let gw = grp.get_bounding_client_rect().width().max(320.0);
                    with_state_mut(|s| {
                        s.active_resize = Some(ResizeDrag {
                            kind,
                            start_x: ev.client_x() as f64,
                            group_width: gw,
                            start_left: s.panel_widths.left,
                            start_middle: s.panel_widths.middle,
                            start_right: s.panel_widths.right,
                        });
                    });
                }
            }
            return;
        }

        // 7b. Block Reorder Grip Handle
        if let Ok(Some(grip)) = el.closest("[data-block-grip]") {
            ev.prevent_default();
            if let Some(bid) = grip.get_attribute("data-block-grip") {
                if let Ok(Some(row)) = grip.closest("[data-block-row]") {
                    let _ = row.class_list().add_1("hf-block-dragging");
                }
                with_state_mut(|s| s.dragging_block_id = Some(bid));
            }
            return;
        }

        // 7c. Step 2 Cropper Canvas Pan
        if el.id() == "hf-cropper-canvas" {
            ev.prevent_default();
            with_state_mut(|s| {
                s.crop_drag_origin = Some((
                    ev.client_x() as f64,
                    ev.client_y() as f64,
                    s.crop_pan_x,
                    s.crop_pan_y,
                ));
            });
        }
    }) as Box<dyn FnMut(MouseEvent)>);
    let _ =
        doc.add_event_listener_with_callback("mousedown", on_mousedown.as_ref().unchecked_ref());
    on_mousedown.forget();

    let on_mousemove = Closure::wrap(Box::new(move |ev: MouseEvent| {
        // 1. Panel Resize Drag
        let resize_opt = with_state(|s| s.active_resize);
        if let Some(drag) = resize_opt {
            let dx_pct = ((ev.client_x() as f64 - drag.start_x) / drag.group_width) * 100.0;
            let new_widths = match drag.kind {
                ResizeHandleKind::LeftMiddle => {
                    let max_left = (100.0 - drag.start_right - 30.0).min(55.0).max(15.0);
                    let left = (drag.start_left + dx_pct).clamp(15.0, max_left);
                    let middle = 100.0 - left - drag.start_right;
                    PanelWidths {
                        left,
                        middle,
                        right: drag.start_right,
                    }
                }
                ResizeHandleKind::MiddleRight => {
                    let max_right = (100.0 - drag.start_left - 30.0).min(55.0).max(25.0);
                    let right = (drag.start_right - dx_pct).clamp(25.0, max_right);
                    let middle = 100.0 - drag.start_left - right;
                    PanelWidths {
                        left: drag.start_left,
                        middle,
                        right,
                    }
                }
            };
            with_state_mut(|s| s.panel_widths = new_widths);
            apply_panel_widths_to_dom(new_widths);
            return;
        }

        // 2. Block Drag-Reorder
        let drag_bid = with_state(|s| s.dragging_block_id.clone());
        if let Some(bid) = drag_bid {
            handle_block_drag_move(&bid, ev.client_y() as f64);
            return;
        }

        // 3. Cropper Canvas Pan
        let crop_drag = with_state(|s| s.crop_drag_origin);
        if let Some((sx, sy, spx, spy)) = crop_drag {
            if let Some(canvas) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("hf-cropper-canvas"))
            {
                let rect = canvas.get_bounding_client_rect();
                let rw = rect.width().max(1.0);
                let rh = rect.height().max(1.0);
                let dx_canvas = (ev.client_x() as f64 - sx) * (CROP_WIDTH / rw);
                let dy_canvas = (ev.client_y() as f64 - sy) * (CROP_HEIGHT / rh);
                with_state_mut(|s| {
                    s.crop_pan_x = spx + dx_canvas;
                    s.crop_pan_y = spy + dy_canvas;
                });
                redraw_cropper_canvas();
            }
        }
    }) as Box<dyn FnMut(MouseEvent)>);
    let _ =
        win.add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref());
    on_mousemove.forget();

    let on_mouseup = Closure::wrap(Box::new(move |_ev: MouseEvent| {
        let (was_resizing, was_dragging_block, was_cropping) = with_state_mut(|s| {
            let r = s.active_resize.take().is_some();
            let b = s.dragging_block_id.take().is_some();
            let c = s.crop_drag_origin.take().is_some();
            (r, b, c)
        });

        if was_resizing {
            if let Some(body) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.body())
            {
                let _ = body.class_list().remove_1("hf-resizing-cols");
            }
            let w = with_state(|s| s.panel_widths);
            save_panel_widths(&w);
        }
        if was_dragging_block {
            refresh_blocks_list_only();
        }
        if was_cropping {
            redraw_cropper_canvas();
        }
        rich_editor::update_selection_bubble_from_dom();
        render_research::update_reader_quote_bubble_from_dom();
    }) as Box<dyn FnMut(MouseEvent)>);
    let _ = win.add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref());
    on_mouseup.forget();

    // 8. Capture-Phase Image Error Fallback (`Login.tsx` logo & `NewsImage` `onError`)
    let on_img_err = Closure::wrap(Box::new(move |ev: Event| {
        let Some(el) = target_element(&ev) else { return };
        if el.get_attribute("data-login-logo").as_deref() == Some("1") {
            if let Ok(he) = el.clone().dyn_into::<HtmlElement>() {
                let _ = he.style().set_property("display", "none");
            }
            if let Some(parent) = el.parent_element() {
                if parent
                    .query_selector("[data-login-fallback]")
                    .ok()
                    .flatten()
                    .is_none()
                {
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        if let Ok(span) = doc.create_element("span") {
                            let _ = span.set_attribute("data-login-fallback", "1");
                            span.set_class_name(
                                "text-2xl sm:text-3xl font-black tracking-tighter text-zinc-900",
                            );
                            span.set_text_content(Some("MSI DERGİSİ"));
                            let _ = parent.append_child(&span);
                        }
                    }
                }
            }
            return;
        }

        if el.get_attribute("data-news-image").as_deref() == Some("1") {
            let fallback_cls = el
                .get_attribute("data-fallback-class")
                .unwrap_or_else(|| "w-full h-full object-cover".to_string());
            let alt = el
                .get_attribute("alt")
                .unwrap_or_else(|| "Haber görseli".to_string());
            let replacement = format!(
                r#"<div class="flex items-center justify-center bg-zinc-100 {}"><img src="{}" alt="{}" class="w-2/3 h-auto max-w-[120px] object-contain opacity-80" /></div>"#,
                state::escape_html(&fallback_cls),
                PLACEHOLDER_IMG,
                state::escape_html(&alt),
            );
            el.set_outer_html(&replacement);
        }
    }) as Box<dyn FnMut(Event)>);
    let _ = doc.add_event_listener_with_callback_and_bool(
        "error",
        on_img_err.as_ref().unchecked_ref(),
        true,
    );
    on_img_err.forget();

    // 9. Window Resize Breakpoint Watcher (`>= 1024px`)
    let on_resize = Closure::wrap(Box::new(move |_ev: Event| {
        let now_desktop = current_viewport_desktop();
        let changed = with_state_mut(|s| {
            if s.is_desktop != now_desktop {
                s.is_desktop = now_desktop;
                true
            } else {
                false
            }
        });
        if changed && with_state(|s| s.token.is_some()) {
            render_full();
        }
    }) as Box<dyn FnMut(Event)>);
    let _ = win.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
    on_resize.forget();

    // 10. Tab Visibility Refresh (silent diff-only refresh so DOM nodes are never detached unnecessarily)
    let doc_for_vis = doc.clone();
    let on_vis = Closure::wrap(Box::new(move |_ev: Event| {
        let vis = js_sys::Reflect::get(&doc_for_vis, &JsValue::from_str("visibilityState"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        if vis == "visible" && with_state(|s| s.token.is_some()) {
            api::fetch_articles_silent();
        }
    }) as Box<dyn FnMut(Event)>);
    let _ = doc.add_event_listener_with_callback(
        "visibilitychange",
        on_vis.as_ref().unchecked_ref(),
    );
    on_vis.forget();
}

fn handle_block_drag_move(drag_id: &str, client_y: f64) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(rows) = doc.query_selector_all("[data-block-row]") else {
        return;
    };

    let mut target_idx: Option<usize> = None;
    let mut current_idx: Option<usize> = None;
    let mut positions: Vec<(String, f64)> = Vec::new();

    for i in 0..rows.length() {
        if let Some(node) = rows.item(i) {
            if let Ok(el) = node.dyn_into::<Element>() {
                let id = el.get_attribute("data-block-row").unwrap_or_default();
                let rect = el.get_bounding_client_rect();
                let mid = rect.top() + rect.height() / 2.0;
                if id == drag_id {
                    current_idx = Some(positions.len());
                }
                positions.push((id, mid));
            }
        }
    }

    let Some(cur_i) = current_idx else { return };
    for (i, (_, mid)) in positions.iter().enumerate() {
        if i < cur_i && client_y < *mid {
            target_idx = Some(i);
            break;
        }
        if i > cur_i && client_y > *mid {
            target_idx = Some(i);
        }
    }

    if let Some(dst_i) = target_idx {
        if dst_i != cur_i {
            let moved = with_state_mut(|s| {
                if let Some(blocks) = &mut s.editing_article.blocks {
                    if cur_i < blocks.len() && dst_i < blocks.len() {
                        let item = blocks.remove(cur_i);
                        blocks.insert(dst_i, item);
                        save_editing_draft(&s.editing_article);
                        return true;
                    }
                }
                false
            });
            if moved {
                refresh_blocks_list_only();
            }
        }
    }
}

fn focus_editor_title() {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(ta) = doc
            .get_element_by_id("editor-title-textarea")
            .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
        {
            let _ = ta.focus();
        }
    }
}

fn add_tags_from_str(raw: &str) -> bool {
    let mut added = false;
    with_state_mut(|s| {
        let tags_vec = s.editing_article.tags.get_or_insert_with(Vec::new);
        for part in raw.split(',') {
            let clean = part.trim().trim_start_matches('#').trim();
            if clean.is_empty() {
                continue;
            }
            let low = turkish_lowercase(clean);
            if !tags_vec.iter().any(|t| turkish_lowercase(t.trim()) == low) {
                tags_vec.push(clean.to_string());
                added = true;
            }
        }
        if added {
            save_editing_draft(&s.editing_article);
        }
    });
    added
}

fn commit_current_tag_input() {
    let raw = with_state_mut(|s| {
        let v = s.tag_input_query.clone();
        s.tag_input_query.clear();
        v
    });
    if let Some(inp) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("hf-tag-input"))
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
    {
        let dom_val = inp.value();
        let effective = if raw.trim().is_empty() { dom_val } else { raw };
        let _ = add_tags_from_str(&effective);
        inp.set_value("");
        let _ = inp.focus();
    } else {
        let _ = add_tags_from_str(&raw);
    }
    refresh_step3_tags_ui();
}

pub(crate) fn reset_editor_now(focus_title_and_open_right: bool) {
    let old_id = with_state(|s| s.editing_article.id.clone());
    if !old_id.trim().is_empty() {
        api::release_article_lock(&old_id);
    }
    clear_editing_draft();
    let (desktop, had_mid_prev) = with_state_mut(|s| {
        let had_mp = s.middle_preview_draft;
        s.editing_article = empty_editing_article();
        s.reset_editor_lang_buffers();
        s.show_author_meta_bar = false;
        s.editing_baseline_sig.clear();
        s.editing_conflict_lock = None;
        s.cover_watermarked = false;
        s.editing_related_block_id = None;
        s.middle_preview_draft = false;
        s.show_preview_modal = false;
        s.pending_editor_confirm = None;
        s.editor_step = 1;
        s.editor_image_error = false;
        s.crop_loaded_src.clear();
        s.crop_image_el = None;
        if focus_title_and_open_right {
            s.mobile_view = MobileView::Right;
        }
        (s.is_desktop, had_mp)
    });
    sync_editor_confirm_modal_state();
    sync_preview_modal_state();
    if focus_title_and_open_right && !desktop {
        render_full();
        return;
    }
    refresh_left_panel_or_body();
    if had_mid_prev {
        refresh_middle_panel();
    }
    refresh_right_panel();
    if focus_title_and_open_right {
        focus_editor_title();
    }
}

fn refresh_left_panel_or_body() {
    crate::editorial::wasm::render::refresh_left_panel();
}

fn switch_to_research_url(url: String) {
    let need_fetch = with_state_mut(|s| {
        s.show_research_tabs_menu = false;
        s.active_research_url = Some(url.clone());
        s.middle_showing_research = true;
        s.middle_preview_draft = false;
        s.research_tabs
            .iter()
            .find(|t| t.url == url)
            .filter(|t| t.extracted.is_none() && !t.is_loading && t.error.is_none())
            .map(|t| (t.title_hint.clone(), t.source_hint.clone()))
    });
    if let Some((th, sh)) = need_fetch {
        api::open_research_url(url, th, sh, String::new(), String::new());
    } else {
        refresh_research_slots();
        refresh_middle_panel();
    }
}

fn handle_action_click(action: &str, act_el: &Element, ev: &MouseEvent) {
    match action {
        "toggle-search-filters" => {
            with_state_mut(|s| s.show_search_filters = !s.show_search_filters);
            refresh_header_banner();
        }
        "clear-search-filters" => {
            with_state_mut(|s| {
                s.search_start_date.clear();
                s.search_end_date.clear();
            });
            refresh_header_banner();
        }
        "apply-search-filters" => {
            with_state_mut(|s| s.show_search_filters = false);
            refresh_header_banner();
            api::fetch_articles("all", false);
        }
        "toggle-profile-menu" => {
            with_state_mut(|s| s.is_profile_dropdown_open = !s.is_profile_dropdown_open);
            refresh_header_banner();
        }
        "close-profile-menu" => {
            with_state_mut(|s| s.is_profile_dropdown_open = false);
            refresh_header_banner();
        }
        "open-change-pw" => {
            with_state_mut(|s| {
                s.is_profile_dropdown_open = false;
                s.show_settings_popover = false;
                s.is_adding_user = false;
                s.show_change_password = true;
            });
            refresh_header_banner();
            sync_left_toolbar_state();
            refresh_modals();
        }
        "close-change-pw" => {
            with_state_mut(|s| {
                s.show_change_password = false;
                s.change_pw_current.clear();
                s.change_pw_new.clear();
                s.change_pw_confirm.clear();
            });
            refresh_modals();
        }
        "submit-change-pw" => api::submit_change_password(),
        "logout" => api::perform_logout(),
        "toggle-msi-theme" => {
            let next = crate::editorial::wasm::state::toggle_msi_theme();
            let was_menu_open = with_state_mut(|s| {
                let was = s.is_profile_dropdown_open;
                s.is_profile_dropdown_open = false;
                was
            });
            if was_menu_open {
                refresh_header_banner();
            } else if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(btn) = doc.get_element_by_id("btn-msi-theme-toggle") {
                    btn.set_inner_html(if next == "dark" {
                        "<span>Açık Tema</span>"
                    } else {
                        "<span>Koyu Tema</span>"
                    });
                }
            }
        }

        "set-left-tab" => {
            let tab_str = act_el.get_attribute("data-tab").unwrap_or_default();
            let next_tab = match tab_str.as_str() {
                "drafts" => LeftTab::Drafts,
                "research" => LeftTab::Research,
                _ => LeftTab::Published,
            };
            let (changed, should_auto_fetch) = with_state_mut(|s| {
                let ch = s.left_tab != next_tab;
                s.left_tab = next_tab;
                let auto_fetch =
                    next_tab == LeftTab::Research && !s.research_loaded_once && !s.is_searching_research;
                (ch, auto_fetch)
            });
            sync_left_toolbar_state();
            if changed {
                refresh_left_body();
            }
            if next_tab == LeftTab::Research {
                api::fetch_research_sources();
            }
            if should_auto_fetch {
                api::fetch_research_results();
            }
        }
        "toggle-settings-popover" => {
            let (header_changed, reset_add_user, was_opening) = with_state_mut(|s| {
                s.show_settings_popover = !s.show_settings_popover;
                if s.show_settings_popover {
                    let hc = s.is_profile_dropdown_open || s.show_search_filters;
                    s.is_profile_dropdown_open = false;
                    s.show_search_filters = false;
                    (hc, false, true)
                } else {
                    let had_add = s.is_adding_user;
                    s.is_adding_user = false;
                    (false, had_add, false)
                }
            });
            if header_changed {
                refresh_header_banner();
            }
            sync_left_toolbar_state();
            sync_settings_popover_state(reset_add_user);
            if was_opening {
                api::fetch_research_sources();
            }
        }
        "close-settings-popover" => {
            let reset_add_user = with_state_mut(|s| {
                let had_add = s.is_adding_user;
                s.show_settings_popover = false;
                s.is_adding_user = false;
                had_add
            });
            sync_left_toolbar_state();
            sync_settings_popover_state(reset_add_user);
        }
        "set-view-mode" => {
            let m = act_el.get_attribute("data-mode").unwrap_or_default();
            let mode = NewsViewMode::from_str(&m);
            let changed = with_state_mut(|s| {
                if s.news_view_mode != mode {
                    s.news_view_mode = mode;
                    true
                } else {
                    false
                }
            });
            save_news_view_mode(mode);
            sync_left_toolbar_state();
            if changed {
                refresh_left_body();
            }
        }
        "save-settings" => api::save_all_settings(),
        "refresh-articles" => {
            if with_state(|s| s.left_tab == LeftTab::Research) {
                api::fetch_research_results();
            } else {
                api::fetch_articles("all", false);
            }
        }
        "new-article" => {
            if with_state(should_confirm_editor_discard) {
                with_state_mut(|s| {
                    s.pending_editor_confirm = Some(PendingEditorConfirm::NewArticle);
                });
                sync_editor_confirm_modal_state();
            } else {
                reset_editor_now(true);
            }
        }

        "select-article" => {
            let id = act_el.get_attribute("data-article-id").unwrap_or_default();
            let desktop = with_state_mut(|s| {
                let found = s
                    .published_articles
                    .iter()
                    .find(|a| a.id == id)
                    .or_else(|| s.draft_articles.iter().find(|a| a.id == id))
                    .cloned();
                if let Some(art) = found {
                    s.selected_article = Some(art);
                    s.preview_lang = "tr".to_string();
                    s.middle_showing_research = false;
                    s.middle_preview_draft = false;
                    s.mobile_view = MobileView::Middle;
                }
                s.is_desktop
            });
            if desktop {
                refresh_left_body();
                refresh_middle_panel();
                if with_state(|s| s.editor_step == 4) {
                    refresh_right_panel();
                }
            } else {
                render_full();
            }
        }
        "select-draft" => {
            let id = act_el.get_attribute("data-article-id").unwrap_or_default();
            let desktop = with_state_mut(|s| {
                if let Some(art) = s.draft_articles.iter().find(|d| d.id == id).cloned() {
                    s.selected_article = Some(art);
                    s.preview_lang = "tr".to_string();
                    s.middle_showing_research = false;
                    s.middle_preview_draft = false;
                    s.mobile_view = MobileView::Middle;
                }
                s.is_desktop
            });
            let should_auto_load = with_state(|s| {
                let cur_id = s.editing_article.id.trim();
                let is_clean = !should_confirm_editor_discard(s);
                cur_id.is_empty() || cur_id == id || is_clean
            });
            if should_auto_load {
                api::move_selected_to_editor_force();
            }
            if desktop {
                refresh_left_body();
                refresh_middle_panel();
                refresh_right_panel();
            } else {
                render_full();
            }
        }
        "delete-editing-draft" => {
            let draft_id = with_state(|s| s.editing_article.id.clone());
            if !draft_id.trim().is_empty() {
                api::delete_article(draft_id);
            }
        }
        "delete-article" => {
            ev.stop_propagation();
            if let Some(id) = act_el.get_attribute("data-article-id") {
                api::delete_article(id);
            }
        }
        "load-more" => {
            let k = act_el.get_attribute("data-kind").unwrap_or_default();
            match k.as_str() {
                "draft" => api::fetch_articles("draft", true),
                _ => api::fetch_articles("published", true),
            }
        }
        "manual-share" => {
            ev.stop_propagation();
            if let Some(id) = act_el.get_attribute("data-article-id") {
                api::manual_social_share(id);
            }
        }
        "refresh-stats" => {
            ev.stop_propagation();
            if let Some(id) = act_el.get_attribute("data-article-id") {
                api::refresh_article_stats(id);
            }
        }

        "set-settings-tab" => {
            let st = act_el.get_attribute("data-subtab").unwrap_or_default();
            let next = match st.as_str() {
                "team" => SettingsTab::Team,
                "sources" => SettingsTab::Sources,
                "issues" => SettingsTab::Issues,
                "glossary" => SettingsTab::Glossary,
                _ => SettingsTab::Categories,
            };
            with_state_mut(|s| s.settings_tab = next);
            refresh_settings_popover();
            if next == SettingsTab::Sources {
                api::fetch_research_sources();
            } else if next == SettingsTab::Issues || next == SettingsTab::Glossary {
                api::fetch_editorial_meta();
            }
        }
        "open-settings-issues" => {
            with_state_mut(|s| {
                s.show_settings_popover = true;
                s.settings_tab = SettingsTab::Issues;
                s.is_profile_dropdown_open = false;
                s.show_search_filters = false;
            });
            sync_left_toolbar_state();
            sync_settings_popover_state(true);
            api::fetch_editorial_meta();
        }
        "open-settings-glossary" => {
            with_state_mut(|s| {
                s.show_settings_popover = true;
                s.settings_tab = SettingsTab::Glossary;
                s.is_profile_dropdown_open = false;
                s.show_search_filters = false;
            });
            sync_left_toolbar_state();
            sync_settings_popover_state(true);
            api::fetch_editorial_meta();
        }
        "toggle-new-issue-special" => {
            with_state_mut(|s| s.new_issue_special = !s.new_issue_special);
            refresh_settings_popover();
        }
        "create-magazine-issue" => api::create_magazine_issue(),
        "delete-magazine-issue" => {
            if let Some(id) = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
            {
                api::delete_magazine_issue(id);
            }
        }
        "toggle-new-glossary-case" => {
            with_state_mut(|s| s.new_glossary_case_sensitive = !s.new_glossary_case_sensitive);
            refresh_settings_popover();
        }
        "create-glossary-rule" => api::create_glossary_rule(),
        "delete-glossary-rule" => {
            if let Some(id) = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
            {
                api::delete_glossary_rule(id);
            }
        }
        "pick-new-cat-color" => {
            if let Some(hex) = act_el.get_attribute("data-color") {
                with_state_mut(|s| s.new_cat_color = hex);
                refresh_settings_popover();
            }
        }
        "pick-edit-cat-color" => {
            if let Some(hex) = act_el.get_attribute("data-color") {
                with_state_mut(|s| s.edit_cat_color = hex);
                refresh_settings_popover();
            }
        }
        "create-category" => api::create_category(),
        "quick-add-subcategory" => {
            if let Some(pid) = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
            {
                with_state_mut(|s| {
                    s.new_cat_parent_id = Some(pid);
                    s.editing_cat_id = None;
                    if let Some(pcat) = s.categories.iter().find(|c| c.id == pid) {
                        if let Some(col) = &pcat.color {
                            s.new_cat_color = col.clone();
                        }
                    }
                });
                refresh_settings_popover();
                if let Some(inp) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("hf-new-cat-name"))
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                {
                    let _ = inp.focus();
                }
            }
        }
        "clear-new-cat-parent" => {
            with_state_mut(|s| s.new_cat_parent_id = None);
            refresh_settings_popover();
        }
        "start-edit-category" => {
            if let Some(id) = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
            {
                let found = with_state_mut(|s| {
                    if let Some(cat) = s.categories.iter().find(|c| c.id == id).cloned() {
                        s.editing_cat_id = Some(cat.id);
                        s.edit_cat_name = cat.name;
                        s.edit_cat_slug = cat.slug;
                        s.edit_cat_color =
                            cat.color.unwrap_or_else(|| "#52c2ef".to_string());
                        s.edit_cat_parent_id = cat.parent_id;
                        true
                    } else {
                        false
                    }
                });
                if found {
                    refresh_settings_popover();
                }
            }
        }
        "cancel-edit-category" => {
            with_state_mut(|s| s.editing_cat_id = None);
            refresh_settings_popover();
        }
        "confirm-edit-category" => api::update_category(),
        "delete-category" => {
            if let Some(id) = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
            {
                api::delete_category(id);
            }
        }
        "open-add-user" => {
            with_state_mut(|s| s.is_adding_user = true);
            refresh_settings_popover();
        }
        "cancel-add-user" => {
            with_state_mut(|s| s.is_adding_user = false);
            refresh_settings_popover();
        }
        "confirm-add-user" => api::add_team_user(),
        "delete-user" => {
            if let Some(id) = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
            {
                api::delete_team_user(id);
            }
        }

        // Research Sources Settings Actions
        "create-research-source" => api::create_research_source(),
        "start-edit-source" => {
            if let Some(id) = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
            {
                let found = with_state_mut(|s| {
                    if let Some(src) = s.research_sources.iter().find(|x| x.id == id).cloned() {
                        s.editing_source_id = Some(src.id);
                        s.edit_source_name = src.name;
                        s.edit_source_url = src.url;
                        s.edit_source_category = src.category;
                        s.edit_source_scope = src.scope;
                        s.edit_source_enabled = src.enabled;
                        true
                    } else {
                        false
                    }
                });
                if found {
                    refresh_settings_popover();
                }
            }
        }
        "cancel-edit-source" => {
            with_state_mut(|s| s.editing_source_id = None);
            refresh_settings_popover();
        }
        "confirm-edit-source" => api::update_research_source(),
        "toggle-research-source" => {
            if let Some(id) = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
            {
                api::toggle_research_source(id);
            }
        }
        "delete-research-source" => {
            if let Some(id) = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
            {
                api::delete_research_source(id);
            }
        }

        "move-to-editor" => api::move_selected_to_editor(),
        "mobile-back-left" => {
            with_state_mut(|s| s.mobile_view = MobileView::Left);
            render_full();
        }
        "mobile-back-middle" => {
            with_state_mut(|s| s.mobile_view = MobileView::Middle);
            render_full();
        }

        "toggle-expand-blocks" => {
            with_state_mut(|s| s.all_blocks_expanded = !s.all_blocks_expanded);
            refresh_right_panel();
        }
        "wizard-prev" => {
            with_state_mut(|s| {
                if s.editor_step > 1 {
                    s.editor_step -= 1;
                }
            });
            refresh_right_panel();
        }
        "wizard-next" => {
            let step = with_state(|s| s.editor_step);
            if step == 2 {
                api::handle_crop_and_continue();
            } else if step < 4 {
                let (next_step, is_desk) = with_state_mut(|s| {
                    s.editor_step += 1;
                    if s.editor_step == 4 && s.is_desktop {
                        s.middle_preview_draft = true;
                    }
                    (s.editor_step, s.is_desktop)
                });
                if next_step == 3 && with_state(|s| s.tag_suggestions.is_empty()) {
                    api::fetch_tag_suggestions("");
                }
                if next_step == 4 && is_desk {
                    refresh_middle_panel();
                }
                refresh_right_panel();
            }
        }
        "clear-editor" => {
            let (needs_confirm, was_saved_draft) = with_state(|s| {
                let is_dr = s.editing_article.id.trim().starts_with("draft-")
                    || s.editing_article.article_type.as_deref() == Some("draft");
                (should_confirm_editor_discard(s), is_dr)
            });
            if needs_confirm {
                with_state_mut(|s| {
                    s.pending_editor_confirm = Some(PendingEditorConfirm::ClearEditor);
                });
                sync_editor_confirm_modal_state();
            } else {
                reset_editor_now(false);
                if was_saved_draft {
                    api::show_success("Taslak editörden kapatıldı (Taslaklar listesinde duruyor).");
                }
            }
        }
        "cancel-editor-confirm" => {
            with_state_mut(|s| s.pending_editor_confirm = None);
            sync_editor_confirm_modal_state();
        }
        "confirm-editor-discard" => {
            let pending = with_state_mut(|s| s.pending_editor_confirm.take());
            sync_editor_confirm_modal_state();
            match pending {
                Some(PendingEditorConfirm::NewArticle) => reset_editor_now(true),
                Some(PendingEditorConfirm::ClearEditor) => reset_editor_now(false),
                Some(PendingEditorConfirm::MoveSelectedToEditor) => {
                    api::move_selected_to_editor_force()
                }
                None => {}
            }
        }
        "save-draft-and-confirm-editor" => {
            if let Some(pending) = with_state(|s| s.pending_editor_confirm) {
                api::save_draft_and_execute_pending_confirm(pending);
            }
        }
        "bubble-cmd" => {
            let cmd = act_el.get_attribute("data-bubble-cmd").unwrap_or_default();
            rich_editor::handle_bubble_command(&cmd);
            if with_state(|s| s.editor_lang == "en") {
                refresh_step1_glossary_slots();
            }
        }
        "set-editor-lang" => {
            let lang = act_el
                .get_attribute("data-lang")
                .unwrap_or_else(|| "tr".to_string());
            let (mid_prev, show_prev) = with_state_mut(|s| {
                s.switch_editor_lang(&lang);
                (s.middle_preview_draft, s.show_preview_modal)
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
            if show_prev {
                sync_preview_modal_state();
            }
        }
        "copy-tr-to-en-assisted" => {
            let (mid_prev, show_prev) = with_state_mut(|s| {
                s.copy_tr_to_en_with_glossary_assist();
                (s.middle_preview_draft, s.show_preview_modal)
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
            if show_prev {
                sync_preview_modal_state();
            }
            api::show_success("Türkçe içerik TSK/SSB/NATO sözlük karşılıklarıyla İngilizce editöre aktarıldı.");
        }
        "apply-glossary-fixes" => {
            let (fixed_cnt, mid_prev, show_prev) = with_state_mut(|s| {
                let n = s.apply_active_en_glossary_fixes();
                (n, s.middle_preview_draft, s.show_preview_modal)
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
            if show_prev {
                sync_preview_modal_state();
            }
            if fixed_cnt > 0 {
                api::show_success(&format!(
                    "{fixed_cnt} terminoloji / British English kuralı otomatik düzeltildi."
                ));
            }
        }
        "toggle-glossary-drawer" => {
            with_state_mut(|s| s.show_glossary_drawer = !s.show_glossary_drawer);
            refresh_right_panel();
        }
        "toggle-author-meta-bar" => {
            with_state_mut(|s| s.show_author_meta_bar = !s.show_author_meta_bar);
            refresh_right_panel();
        }
        "toggle-live-blog" => {
            with_state_mut(|s| {
                s.editing_article.is_live_blog = !s.editing_article.is_live_blog;
                if s.editing_article.is_live_blog && s.live_quick_time.trim().is_empty() {
                    s.live_quick_time = js_now_hhmm();
                }
                save_editing_draft(&s.editing_article);
            });
            refresh_right_panel();
        }
        "prepend-live-entry" => {
            let (q_time, q_title, q_body) = with_state(|s| {
                (
                    s.live_quick_time.trim().to_string(),
                    s.live_quick_title.trim().to_string(),
                    s.live_quick_body.trim().to_string(),
                )
            });
            if q_body.is_empty() {
                api::show_error("Lütfen eklenecek canlı gelişme metnini yazın.");
                return;
            }
            let final_time = if q_time.is_empty() {
                js_now_hhmm()
            } else {
                q_time
            };
            let meta_str = if q_title.is_empty() {
                final_time
            } else {
                format!("{final_time}|{q_title}")
            };
            with_state_mut(|s| {
                let bid = s.next_block_id();
                let blocks = s.editing_article.blocks.get_or_insert_with(Vec::new);
                blocks.insert(
                    0,
                    ContentBlock {
                        id: bid,
                        block_type: "live_entry".to_string(),
                        content: sanitize_inline_rich_html(&q_body),
                        meta: Some(meta_str),
                    },
                );
                s.live_quick_title.clear();
                s.live_quick_body.clear();
                s.live_quick_time = js_now_hhmm();
                save_editing_draft(&s.editing_article);
            });
            refresh_right_panel();
            api::show_success("Canlı gelişme en üste eklendi.");
        }
        "add-block" => {
            let btype = act_el
                .get_attribute("data-block-type")
                .unwrap_or_else(|| "paragraph".to_string());
            let default_content = match btype.as_str() {
                "gallery" => "[]".to_string(),
                "related_article" => "{}".to_string(),
                _ => String::new(),
            };
            let default_meta = match btype.as_str() {
                "callout" => Some("info|".to_string()),
                "summary" => Some("Öne Çıkanlar".to_string()),
                "related_article" => Some("BUNU DA OKUYUN".to_string()),
                "live_entry" => Some(format!("{}|", js_now_hhmm())),
                _ => None,
            };
            let new_id = with_state_mut(|s| {
                let bid = s.next_block_id();
                let blocks = s.editing_article.blocks.get_or_insert_with(Vec::new);
                blocks.push(ContentBlock {
                    id: bid.clone(),
                    block_type: btype.clone(),
                    content: default_content,
                    meta: default_meta,
                });
                save_editing_draft(&s.editing_article);
                bid
            });
            rich_editor::hide_selection_bubble();
            refresh_blocks_list_only();
            if btype == "related_article" {
                if let Some(inp) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id(&format!("hf-rel-search-{new_id}")))
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                {
                    let _ = inp.focus();
                }
            } else {
                rich_editor::focus_block_by_id(&new_id);
            }
        }
        "remove-gallery-item" => {
            let bid = act_el.get_attribute("data-block-id").unwrap_or_default();
            if let Some(idx) = act_el
                .get_attribute("data-item-idx")
                .and_then(|s| s.parse::<usize>().ok())
            {
                with_state_mut(|s| {
                    if let Some(blocks) = &mut s.editing_article.blocks {
                        for b in blocks.iter_mut() {
                            if b.id == bid && b.block_type == "gallery" {
                                let mut items: Vec<GalleryItem> =
                                    serde_json::from_str(b.content.trim()).unwrap_or_default();
                                if idx < items.len() {
                                    items.remove(idx);
                                    b.content = serde_json::to_string(&items)
                                        .unwrap_or_else(|_| "[]".to_string());
                                }
                            }
                        }
                    }
                    save_editing_draft(&s.editing_article);
                });
                refresh_blocks_list_only();
            }
        }
        "change-related-article" => {
            let bid = act_el.get_attribute("data-block-id").unwrap_or_default();
            with_state_mut(|s| {
                s.editing_related_block_id = Some(bid.clone());
            });
            refresh_blocks_list_only();
            if let Some(inp) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(&format!("hf-rel-search-{bid}")))
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            {
                let _ = inp.focus();
            }
        }
        "pick-related-article" => {
            let bid = act_el.get_attribute("data-block-id").unwrap_or_default();
            let aid = act_el.get_attribute("data-article-id").unwrap_or_default();
            let updated = with_state_mut(|s| {
                let Some(art) = s.published_articles.iter().find(|a| a.id == aid).cloned() else {
                    return false;
                };
                let slug = art
                    .slug
                    .clone()
                    .filter(|sl| !sl.trim().is_empty())
                    .or_else(|| {
                        let g = slugify_turkish(&art.title);
                        (!g.is_empty()).then_some(g)
                    });
                let img = art
                    .thumb_url
                    .clone()
                    .filter(|u| !u.trim().is_empty())
                    .or_else(|| art.effective_image_url().map(str::to_string));
                let payload = RelatedArticleRef {
                    article_id: art.id,
                    title: art.title,
                    slug,
                    image_url: img,
                };
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == bid && b.block_type == "related_article" {
                            b.content = serde_json::to_string(&payload)
                                .unwrap_or_else(|_| "{}".to_string());
                        }
                    }
                }
                s.editing_related_block_id = None;
                s.related_search_queries.remove(&bid);
                save_editing_draft(&s.editing_article);
                true
            });
            if updated {
                refresh_blocks_list_only();
            }
        }
        "insert-block-below" => {
            let after_id = act_el.get_attribute("data-block-id").unwrap_or_default();
            let new_id = with_state_mut(|s| {
                let bid = s.next_block_id();
                let blocks = s.editing_article.blocks.get_or_insert_with(Vec::new);
                let idx = blocks
                    .iter()
                    .position(|b| b.id == after_id)
                    .map(|p| p + 1)
                    .unwrap_or(blocks.len());
                blocks.insert(
                    idx,
                    ContentBlock {
                        id: bid.clone(),
                        block_type: "paragraph".to_string(),
                        content: String::new(),
                        meta: None,
                    },
                );
                save_editing_draft(&s.editing_article);
                bid
            });
            rich_editor::hide_selection_bubble();
            refresh_blocks_list_only();
            rich_editor::focus_block_by_id(&new_id);
        }
        "duplicate-block" => {
            let src_id = act_el.get_attribute("data-block-id").unwrap_or_default();
            let dup_id = with_state_mut(|s| {
                let nid = s.next_block_id();
                let blocks = s.editing_article.blocks.get_or_insert_with(Vec::new);
                let pos = blocks.iter().position(|b| b.id == src_id)?;
                let mut cloned = blocks[pos].clone();
                cloned.id = nid.clone();
                blocks.insert(pos + 1, cloned);
                save_editing_draft(&s.editing_article);
                Some(nid)
            });
            rich_editor::hide_selection_bubble();
            refresh_blocks_list_only();
            if let Some(fid) = dup_id {
                rich_editor::focus_block_by_id(&fid);
            }
        }
        "set-callout-variant" => {
            let bid = act_el.get_attribute("data-block-id").unwrap_or_default();
            let variant = act_el
                .get_attribute("data-variant")
                .unwrap_or_else(|| "info".to_string());
            with_state_mut(|s| {
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == bid {
                            let existing_title = b
                                .meta
                                .as_deref()
                                .and_then(|m| m.split_once('|').map(|(_, t)| t))
                                .unwrap_or("");
                            b.meta = Some(format!("{variant}|{existing_title}"));
                        }
                    }
                }
                save_editing_draft(&s.editing_article);
            });
            refresh_blocks_list_only();
        }
        "delete-block" => {
            let bid = act_el.get_attribute("data-block-id").unwrap_or_default();
            with_state_mut(|s| {
                if let Some(blocks) = &mut s.editing_article.blocks {
                    blocks.retain(|b| b.id != bid);
                }
                save_editing_draft(&s.editing_article);
            });
            rich_editor::hide_selection_bubble();
            refresh_blocks_list_only();
        }
        "fetch-step2-url" => {
            let u = with_state(|s| s.step2_remote_url.clone());
            api::fetch_remote_image_to_editor(u);
        }
        "fetch-cors-image" => {
            let u = with_state(|s| {
                s.editing_article
                    .effective_image_url()
                    .unwrap_or_default()
                    .to_string()
            });
            api::fetch_remote_image_to_editor(u);
        }
        "clear-step2-image" => {
            with_state_mut(|s| {
                set_article_image(&mut s.editing_article, "");
                s.editing_article.cover_width = None;
                s.editing_article.cover_height = None;
                s.editing_article.thumb_url = None;
                s.cover_watermarked = false;
                s.editor_image_error = false;
                s.crop_loaded_src.clear();
                s.crop_image_el = None;
                save_editing_draft(&s.editing_article);
            });
            refresh_right_panel();
        }
        "open-media-library" => {
            let target = act_el
                .get_attribute("data-media-target")
                .unwrap_or_else(|| "step2".to_string());
            api::open_media_library(target);
        }
        "close-media-library" => {
            with_state_mut(|s| s.show_media_modal = false);
            sync_media_modal_state(false);
        }
        "toggle-upload-watermark" => {
            let (in_modal, step) = with_state_mut(|s| {
                s.upload_with_watermark = !s.upload_with_watermark;
                (s.show_media_modal, s.editor_step)
            });
            if in_modal {
                sync_media_modal_state(false);
            }
            if step == 2 {
                refresh_right_panel();
            }
        }
        "apply-watermark-current" => api::apply_watermark_to_current_cover(),
        "select-media-asset" => {
            let Some(aid) = act_el
                .get_attribute("data-asset-id")
                .and_then(|s| s.parse::<i32>().ok())
            else {
                return;
            };
            let target_outcome = with_state_mut(|s| {
                let asset = s.media_assets.iter().find(|a| a.id == aid)?.clone();
                let target = s.media_picker_target.clone();
                s.show_media_modal = false;
                if let Some(bid) = target.strip_prefix("block:") {
                    if let Some(blocks) = &mut s.editing_article.blocks {
                        for b in blocks.iter_mut() {
                            if b.id == bid {
                                b.content = asset.url.clone();
                            }
                        }
                    }
                    save_editing_draft(&s.editing_article);
                    Some("block")
                } else if let Some(bid) = target.strip_prefix("gallery:") {
                    if let Some(blocks) = &mut s.editing_article.blocks {
                        for b in blocks.iter_mut() {
                            if b.id == bid && b.block_type == "gallery" {
                                let mut items: Vec<GalleryItem> =
                                    serde_json::from_str(b.content.trim()).unwrap_or_default();
                                items.push(GalleryItem {
                                    url: asset.url.clone(),
                                    thumb_url: asset.thumb_url.clone(),
                                    caption: asset.alt_text.clone().unwrap_or_default(),
                                });
                                b.content = serde_json::to_string(&items)
                                    .unwrap_or_else(|_| "[]".to_string());
                            }
                        }
                    }
                    save_editing_draft(&s.editing_article);
                    Some("gallery")
                } else {
                    set_article_image(&mut s.editing_article, &asset.url);
                    s.editing_article.thumb_url =
                        asset.thumb_url.or_else(|| Some(asset.url.clone()));
                    s.editing_article.cover_width = (asset.width > 0).then_some(asset.width);
                    s.editing_article.cover_height = (asset.height > 0).then_some(asset.height);
                    s.cover_watermarked = asset.watermarked;
                    s.editor_image_error = false;
                    s.crop_loaded_src.clear();
                    s.crop_image_el = None;
                    save_editing_draft(&s.editing_article);
                    Some("step2")
                }
            });
            sync_media_modal_state(false);
            match target_outcome {
                Some("block") => refresh_blocks_list_only(),
                Some("gallery") => {
                    refresh_blocks_list_only();
                    api::show_success("Görsel galeriye eklendi.");
                }
                Some("step2") => refresh_right_panel(),
                _ => {}
            }
        }
        "delete-media-asset" => {
            ev.stop_propagation();
            if let Some(id) = act_el
                .get_attribute("data-asset-id")
                .and_then(|s| s.parse::<i32>().ok())
            {
                api::delete_media_asset(id);
            }
        }
        "generate-slug-from-title" => {
            let gen = with_state_mut(|s| {
                let canon_title = s.canonical_editing_article().title;
                let src_title = if canon_title.trim().is_empty() {
                    s.editing_article.title.as_str()
                } else {
                    canon_title.as_str()
                };
                let g = slugify_turkish(src_title);
                s.editing_article.slug = (!g.is_empty()).then(|| g.clone());
                save_editing_draft(&s.editing_article);
                g
            });
            if let Some(inp) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("hf-editor-slug"))
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            {
                inp.set_value(&gen);
            }
            refresh_step3_live_seo();
        }
        "open-redirects-modal" => api::open_redirects_modal(),
        "close-redirects-modal" => {
            with_state_mut(|s| s.show_redirects_modal = false);
            sync_redirects_modal_state();
        }
        "delete-redirect" => {
            if let Some(id) = act_el
                .get_attribute("data-redirect-id")
                .and_then(|s| s.parse::<i32>().ok())
            {
                api::delete_redirect_rule(id);
            }
        }
        "commit-tag-input" => commit_current_tag_input(),
        "pick-tag-suggestion" => {
            if let Some(tag) = act_el.get_attribute("data-tag") {
                if add_tags_from_str(&tag) {
                    with_state_mut(|s| s.tag_input_query.clear());
                    if let Some(inp) = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.get_element_by_id("hf-tag-input"))
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                    {
                        inp.set_value("");
                    }
                    refresh_step3_tags_ui();
                }
            }
        }
        "remove-tag" => {
            if let Some(tag) = act_el.get_attribute("data-tag") {
                let target_low = turkish_lowercase(tag.trim());
                with_state_mut(|s| {
                    if let Some(tags) = &mut s.editing_article.tags {
                        tags.retain(|t| turkish_lowercase(t.trim()) != target_low);
                    }
                    save_editing_draft(&s.editing_article);
                });
                refresh_step3_tags_ui();
            }
        }
        "toggle-comments-locked" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.comments_locked = !s.editing_article.comments_locked;
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "toggle-ads-disabled" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.ads_disabled = !s.editing_article.ads_disabled;
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "open-preview-modal" => {
            with_state_mut(|s| s.show_preview_modal = true);
            sync_preview_modal_state();
        }
        "close-preview-modal" => {
            with_state_mut(|s| s.show_preview_modal = false);
            sync_preview_modal_state();
        }
        "set-preview-device" => {
            let dev_str = act_el.get_attribute("data-device").unwrap_or_default();
            let dev = if dev_str == "mobile" {
                PreviewDevice::Mobile
            } else {
                PreviewDevice::Desktop
            };
            with_state_mut(|s| s.preview_device = dev);
            sync_preview_modal_state();
        }
        "publish-from-preview" => {
            with_state_mut(|s| s.show_preview_modal = false);
            sync_preview_modal_state();
            api::publish_article_now();
        }
        "preview-draft-in-middle" => {
            let is_desk = with_state_mut(|s| {
                s.middle_preview_draft = true;
                s.mobile_view = MobileView::Middle;
                s.is_desktop
            });
            if is_desk {
                refresh_middle_panel();
                refresh_right_panel();
            } else {
                render_full();
            }
        }
        "exit-middle-draft-preview" => {
            with_state_mut(|s| s.middle_preview_draft = false);
            refresh_middle_panel();
            if with_state(|s| s.editor_step == 4) {
                refresh_right_panel();
            }
        }
        "open-revisions-modal" => {
            let aid = act_el.get_attribute("data-article-id").unwrap_or_default();
            let atitle = act_el
                .get_attribute("data-article-title")
                .unwrap_or_default();
            api::open_revisions_modal(aid, atitle);
        }
        "close-revisions-modal" => {
            with_state_mut(|s| s.show_revisions_modal = false);
            sync_revisions_modal_state();
        }
        "restore-revision" => {
            let aid = act_el.get_attribute("data-article-id").unwrap_or_default();
            if let Some(rid) = act_el
                .get_attribute("data-rev-id")
                .and_then(|s| s.parse::<i64>().ok())
            {
                api::restore_article_revision(aid, rid);
            }
        }
        "auto-fill-seo" => api::auto_fill_seo_local(),
        "toggle-auto-share" => {
            with_state_mut(|s| {
                let cur = s.editing_article.auto_share.unwrap_or(true);
                s.editing_article.auto_share = Some(!cur);
                save_editing_draft(&s.editing_article);
            });
            refresh_right_panel();
        }
        "publish-now" => api::publish_article_now(),
        "save-draft" => api::save_draft_to_system(),
        "save-plain-draft" => api::save_plain_draft_to_system(),
        "set-draft-status-filter" => {
            let f = act_el
                .get_attribute("data-filter")
                .unwrap_or_else(|| "all".to_string());
            with_state_mut(|s| s.draft_status_filter = f);
            refresh_left_body();
        }
        "set-preview-lang" => {
            let lang = act_el
                .get_attribute("data-lang")
                .unwrap_or_else(|| "tr".to_string());
            let norm = if lang == "en" { "en" } else { "tr" };
            let show_prev = with_state_mut(|s| {
                s.preview_lang = norm.to_string();
                s.show_preview_modal
            });
            refresh_middle_panel();
            if show_prev {
                sync_preview_modal_state();
            }
        }
        "set-pub-channel" => {
            let ch = act_el
                .get_attribute("data-channel")
                .unwrap_or_else(|| "news".to_string());
            let norm = match ch.as_str() {
                "magazine" | "both" => ch,
                _ => "news".to_string(),
            };
            let mid_prev = with_state_mut(|s| {
                s.editing_article.publication_channel = norm;
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "toggle-is-featured" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.is_featured = !s.editing_article.is_featured;
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "toggle-ssb-tag" => {
            let code = act_el.get_attribute("data-code").unwrap_or_default();
            let clean = code.trim().to_string();
            if !clean.is_empty() {
                let mid_prev = with_state_mut(|s| {
                    let vec = s.editing_article.ssb_tags.get_or_insert_with(Vec::new);
                    if let Some(pos) = vec.iter().position(|x| x.trim() == clean) {
                        vec.remove(pos);
                    } else {
                        vec.push(clean);
                    }
                    save_editing_draft(&s.editing_article);
                    s.middle_preview_draft
                });
                refresh_right_panel();
                if mid_prev {
                    refresh_middle_panel();
                }
            }
        }
        "set-visibility-level" => {
            let lvl = act_el
                .get_attribute("data-level")
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0)
                .clamp(0, 2);
            let mid_prev = with_state_mut(|s| {
                s.editing_article.visibility_level = lvl;
                if lvl == 2 && s.editing_article.single_price_try <= 0.0 {
                    s.editing_article.single_price_try = 45.0;
                }
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "toggle-is-sponsored" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.is_sponsored = !s.editing_article.is_sponsored;
                if s.editing_article.is_sponsored
                    && s.editing_article
                        .sponsor_legal_label
                        .as_deref()
                        .map_or(true, |l| l.trim().is_empty())
                {
                    s.editing_article.sponsor_legal_label =
                        Some("ADVERTORIAL / KURUMSAL İŞ BİRLİĞİ".to_string());
                }
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }

        // Research Desk & Clean Multi-Source Reader View
        "run-research-search" => {
            if let Some(inp) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("hf-research-input"))
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            {
                let dom_q = inp.value();
                with_state_mut(|s| s.research_query = dom_q);
            }
            api::submit_research_query();
        }
        "toggle-research-info" => {
            with_state_mut(|s| s.show_research_info = !s.show_research_info);
            refresh_research_slots();
        }
        "apply-research-example" => {
            if let Some(query) = act_el.get_attribute("data-query") {
                with_state_mut(|s| {
                    s.research_query = query.clone();
                    s.selected_research_source_id = None;
                });
                if let Some(inp) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("hf-research-input"))
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                {
                    inp.set_value(&query);
                }
                refresh_research_slots();
                api::fetch_research_results();
            }
        }
        "clear-research-query" => {
            with_state_mut(|s| {
                s.research_query.clear();
            });
            if let Some(inp) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("hf-research-input"))
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            {
                inp.set_value("");
                let _ = inp.focus();
            }
            if let Some(btn) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("btn-research-clear-query"))
            {
                let _ = btn.class_list().add_1("hidden");
            }
            refresh_research_slots();
            api::fetch_research_results();
        }
        "toggle-research-collapse" => {
            with_state_mut(|s| {
                s.research_panel_collapsed = !s.research_panel_collapsed;
            });
            refresh_left_body();
        }
        "select-research-source" => {
            let sid = act_el
                .get_attribute("data-id")
                .and_then(|x| x.parse::<i32>().ok())
                .filter(|&id| id > 0);
            with_state_mut(|s| s.selected_research_source_id = sid);
            refresh_research_slots();
            api::fetch_research_results();
        }
        "set-research-scope" => {
            let sc = act_el.get_attribute("data-scope").unwrap_or_default();
            let norm = if sc == "global" { "global" } else { "tr" };
            let changed = with_state_mut(|s| {
                if s.research_scope != norm {
                    s.research_scope = norm.to_string();
                    s.selected_research_source_id = None;
                    true
                } else {
                    false
                }
            });
            if changed {
                refresh_research_slots();
                api::fetch_research_results();
            }
        }
        "set-research-time" => {
            let tm = act_el.get_attribute("data-time").unwrap_or_default();
            let norm = match tm.as_str() {
                "1h" | "24h" | "7d" => tm.as_str(),
                _ => "all",
            };
            let changed = with_state_mut(|s| {
                if s.research_time != norm {
                    s.research_time = norm.to_string();
                    true
                } else {
                    false
                }
            });
            if changed {
                api::fetch_research_results();
            }
        }
        "quick-research-topic" => {
            let topic = act_el.get_attribute("data-topic").unwrap_or_default();
            let norm_topic = topic.trim().to_string();
            let is_gundem = turkish_lowercase(&norm_topic) == "gündem";
            with_state_mut(|s| {
                s.research_category = if is_gundem {
                    String::new()
                } else {
                    norm_topic.clone()
                };
                s.research_query.clear();
                s.selected_research_source_id = None;
            });
            if let Some(inp) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("hf-research-input"))
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            {
                inp.set_value("");
            }
            if let Some(btn) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("btn-research-clear-query"))
            {
                let _ = btn.class_list().add_1("hidden");
            }
            refresh_research_slots();
            api::fetch_research_results();
        }
        "open-research-item" => {
            let url = act_el.get_attribute("data-url").unwrap_or_default();
            let title = act_el.get_attribute("data-title").unwrap_or_default();
            let source = act_el.get_attribute("data-source").unwrap_or_default();
            let snippet = act_el.get_attribute("data-snippet").unwrap_or_default();
            let image = act_el.get_attribute("data-image").unwrap_or_default();
            api::open_research_url(url, title, source, snippet, image);
        }
        "switch-research-tab" => {
            let url = act_el.get_attribute("data-url").unwrap_or_default();
            switch_to_research_url(url);
        }
        "prev-research-tab" => {
            let target_opt = with_state_mut(|s| s.prev_research_tab());
            if let Some(url) = target_opt {
                switch_to_research_url(url);
            }
        }
        "next-research-tab" => {
            let target_opt = with_state_mut(|s| s.next_research_tab());
            if let Some(url) = target_opt {
                switch_to_research_url(url);
            }
        }
        "toggle-research-tabs-menu" => {
            with_state_mut(|s| {
                s.show_research_tabs_menu = !s.show_research_tabs_menu;
            });
            refresh_middle_panel();
        }
        "close-all-research-tabs" => {
            with_state_mut(|s| {
                s.research_tabs.clear();
                s.active_research_url = None;
                s.show_research_tabs_menu = false;
                s.middle_showing_research = false;
            });
            refresh_research_slots();
            refresh_middle_panel();
        }
        "set-wizard-step" => {
            if let Ok(st) = act_el.get_attribute("data-step").unwrap_or_default().parse::<u8>() {
                if (1..=4).contains(&st) {
                    with_state_mut(|s| s.editor_step = st);
                    refresh_right_panel();
                }
            }
        }
        "retry-research-tab" => {
            let url = act_el.get_attribute("data-url").unwrap_or_default();
            let (th, sh) = with_state(|s| {
                s.research_tabs
                    .iter()
                    .find(|t| t.url == url)
                    .map(|t| (t.title_hint.clone(), t.source_hint.clone()))
                    .unwrap_or_default()
            });
            api::open_research_url(url, th, sh, String::new(), String::new());
        }
        "close-research-tab" => {
            ev.stop_propagation();
            let url = act_el.get_attribute("data-url").unwrap_or_default();
            with_state_mut(|s| {
                s.research_tabs.retain(|t| t.url != url);
                if s.research_tabs.is_empty() {
                    s.active_research_url = None;
                    s.middle_showing_research = false;
                    s.show_research_tabs_menu = false;
                } else if s.active_research_url.as_deref() == Some(url.as_str()) {
                    s.active_research_url = s.research_tabs.last().map(|t| t.url.clone());
                }
            });
            refresh_research_slots();
            refresh_middle_panel();
        }
        "enter-research-reader" => {
            let is_desk = with_state_mut(|s| {
                if !s.research_tabs.is_empty() {
                    s.middle_showing_research = true;
                    s.middle_preview_draft = false;
                    s.mobile_view = MobileView::Middle;
                    let has_active = s
                        .active_research_url
                        .as_deref()
                        .map(|u| s.research_tabs.iter().any(|t| t.url == u))
                        .unwrap_or(false);
                    if !has_active {
                        s.active_research_url = s.research_tabs.last().map(|t| t.url.clone());
                    }
                }
                s.is_desktop
            });
            if is_desk {
                refresh_research_slots();
                refresh_middle_panel();
                if with_state(|s| s.editor_step == 4) {
                    refresh_right_panel();
                }
            } else {
                render_full();
            }
        }
        "exit-research-reader" => {
            with_state_mut(|s| s.middle_showing_research = false);
            refresh_middle_panel();
        }
        "attach-research-source" => {
            let url = act_el.get_attribute("data-url").unwrap_or_default();
            if let Some(safe_u) = render_research::safe_http_url(&url) {
                with_state_mut(|s| {
                    set_article_source_url(&mut s.editing_article, Some(&safe_u));
                    save_editing_draft(&s.editing_article);
                });
                refresh_attached_source_slot();
                refresh_middle_panel();
                api::show_success("Kaynak referans linki taslağa eklendi");
            }
        }
        "clear-research-source" => {
            let mid_res = with_state_mut(|s| {
                set_article_source_url(&mut s.editing_article, None);
                save_editing_draft(&s.editing_article);
                s.middle_showing_research && !s.middle_preview_draft
            });
            refresh_attached_source_slot();
            if mid_res {
                refresh_middle_panel();
            }
            api::show_success("Kaynak referans linki taslaktan kaldırıldı.");
        }
        "insert-research-facts" => {
            let outcome = with_state_mut(|s| {
                let (facts, site_name) = {
                    let tab = render_research::active_research_tab(s)?;
                    let art = tab.extracted.as_ref()?;
                    if art.key_facts.is_empty() {
                        return None;
                    }
                    let sn = if !art.site_name.trim().is_empty() {
                        art.site_name.trim().to_string()
                    } else if !tab.source_hint.trim().is_empty() {
                        tab.source_hint.trim().to_string()
                    } else {
                        state::get_site_name(&art.url)
                    };
                    (art.key_facts.clone(), sn)
                };
                let lines: Vec<String> = facts
                    .iter()
                    .map(|f| state::escape_html(crate::editorial::common::strip_all_html_tags(f).trim()))
                    .filter(|f| !f.is_empty())
                    .map(|f| format!("• {f}"))
                    .collect();
                if lines.is_empty() {
                    return None;
                }
                let heading = if site_name.is_empty() {
                    "info|Önemli Rakamlar & Veriler".to_string()
                } else {
                    format!("info|Önemli Rakamlar & Veriler ({site_name})")
                };
                s.append_or_replace_initial_block("callout", lines.join("\n"), Some(heading));
                Some(s.editor_step)
            });
            if let Some(step) = outcome {
                if step == 1 {
                    refresh_blocks_list_only();
                } else {
                    refresh_right_panel();
                }
                api::show_success("Önemli rakamlar Bilgi Kutusu olarak sağ editöre eklendi!");
            }
        }
        "insert-research-embed" => {
            let emb_raw = act_el.get_attribute("data-embed-url").unwrap_or_default();
            if let Some(safe_emb) = render_research::safe_http_url(&emb_raw) {
                let step = with_state_mut(|s| {
                    let site_name = render_research::active_research_tab(s)
                        .map(|t| {
                            t.extracted
                                .as_ref()
                                .map(|a| a.site_name.trim().to_string())
                                .filter(|n| !n.is_empty())
                                .unwrap_or_else(|| t.source_hint.trim().to_string())
                        })
                        .unwrap_or_default();
                    let meta = (!site_name.is_empty()).then(|| format!("Kaynak: {site_name}"));
                    s.append_or_replace_initial_block("embed", safe_emb, meta);
                    s.editor_step
                });
                if step == 1 {
                    refresh_blocks_list_only();
                } else {
                    refresh_right_panel();
                }
                api::show_success("Video / Tweet bloğu sağ editöre eklendi!");
            }
        }
        "send-reader-selection-quote" => {
            let Some(sel_text) = render_research::take_reader_selected_text_and_clear() else {
                return;
            };
            let step = with_state_mut(|s| {
                let site_name = render_research::active_research_tab(s)
                    .map(|t| {
                        t.extracted
                            .as_ref()
                            .map(|a| a.site_name.trim().to_string())
                            .filter(|n| !n.is_empty())
                            .unwrap_or_else(|| {
                                if !t.source_hint.trim().is_empty() {
                                    t.source_hint.trim().to_string()
                                } else {
                                    state::get_site_name(&t.url)
                                }
                            })
                    })
                    .unwrap_or_default();
                let safe_body = state::escape_html(sel_text.trim());
                let meta = (!site_name.is_empty()).then_some(site_name);
                s.append_or_replace_initial_block("quote", safe_body, meta);
                s.editor_step
            });
            refresh_blocks_list_only();
            if step != 1 {
                refresh_right_panel();
            }
            api::show_success("Seçili cümle kaynak adıyla Alıntı bloğu olarak eklendi!");
        }
        "send-reader-selection-callout" => {
            let Some(sel_text) = render_research::take_reader_selected_text_and_clear() else {
                return;
            };
            let step = with_state_mut(|s| {
                let site_name = render_research::active_research_tab(s)
                    .map(|t| {
                        t.extracted
                            .as_ref()
                            .map(|a| a.site_name.trim().to_string())
                            .filter(|n| !n.is_empty())
                            .unwrap_or_else(|| {
                                if !t.source_hint.trim().is_empty() {
                                    t.source_hint.trim().to_string()
                                } else {
                                    state::get_site_name(&t.url)
                                }
                            })
                    })
                    .unwrap_or_default();
                let safe_body = state::escape_html(sel_text.trim());
                let meta = Some(format!("info|{site_name}"));
                s.append_or_replace_initial_block("callout", safe_body, meta);
                s.editor_step
            });
            refresh_blocks_list_only();
            if step != 1 {
                refresh_right_panel();
            }
            api::show_success("Seçili cümle Bilgi Notu olarak sağ editöre eklendi!");
        }

        "dismiss-error" => {
            with_state_mut(|s| s.error = None);
            refresh_toasts();
        }
        "dismiss-success" => {
            with_state_mut(|s| s.success = None);
            refresh_toasts();
        }
        _ => {}
    }
}

fn read_input_value(el: &Element) -> String {
    if let Ok(inp) = el.clone().dyn_into::<HtmlInputElement>() {
        inp.value()
    } else if let Ok(ta) = el.clone().dyn_into::<HtmlTextAreaElement>() {
        ta.value()
    } else if let Ok(sel) = el.clone().dyn_into::<HtmlSelectElement>() {
        sel.value()
    } else {
        String::new()
    }
}

fn handle_input_event(kind: &str, el: &Element) {
    let val = read_input_value(el);

    match kind {
        "login-username" => with_state_mut(|s| s.login_username = val),
        "login-password" => with_state_mut(|s| s.login_password = val),

        "search-query" => {
            with_state_mut(|s| s.search_query = val);
            api::trigger_debounced_search();
        }
        "search-start-date" => with_state_mut(|s| s.search_start_date = val),
        "search-end-date" => with_state_mut(|s| s.search_end_date = val),
        "research-query" => {
            let is_empty = val.trim().is_empty();
            with_state_mut(|s| s.research_query = val);
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(btn) = doc.get_element_by_id("btn-research-clear-query") {
                    if is_empty {
                        let _ = btn.class_list().add_1("hidden");
                    } else {
                        let _ = btn.class_list().remove_1("hidden");
                    }
                }
            }
        }

        "editor-title" => {
            if let Ok(ta) = el.clone().dyn_into::<HtmlTextAreaElement>() {
                auto_fit_textarea(&ta);
            }
            let (disable_next, mid_prev, is_en) = with_state_mut(|s| {
                s.editing_article.title = val;
                save_editing_draft(&s.editing_article);
                let canon_empty = s.canonical_editing_article().title.trim().is_empty();
                (
                    s.editor_step == 1 && canon_empty && s.editing_article.title.trim().is_empty(),
                    s.is_desktop && s.middle_preview_draft,
                    s.editor_lang == "en",
                )
            });
            set_button_disabled("btn-wizard-next", disable_next);
            if is_en {
                refresh_step1_glossary_slots();
            }
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "editor-spot" => {
            if let Ok(ta) = el.clone().dyn_into::<HtmlTextAreaElement>() {
                auto_fit_textarea(&ta);
            }
            let (mid_prev, is_en) = with_state_mut(|s| {
                s.editing_article.spot = if val.trim().is_empty() {
                    None
                } else {
                    Some(val)
                };
                save_editing_draft(&s.editing_article);
                (s.is_desktop && s.middle_preview_draft, s.editor_lang == "en")
            });
            if is_en {
                refresh_step1_glossary_slots();
            }
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "glossary-drawer-query" => {
            with_state_mut(|s| s.glossary_drawer_query = val);
            refresh_step1_glossary_slots();
        }
        "article-author-name" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.author_name = if val.trim().is_empty() {
                    None
                } else {
                    Some(val)
                };
                save_editing_draft(&s.editing_article);
                s.is_desktop && s.middle_preview_draft
            });
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "article-author-title" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.author_title = if val.trim().is_empty() {
                    None
                } else {
                    Some(val)
                };
                save_editing_draft(&s.editing_article);
                s.is_desktop && s.middle_preview_draft
            });
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "article-video-url" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.video_url = if val.trim().is_empty() {
                    None
                } else {
                    Some(val)
                };
                save_editing_draft(&s.editing_article);
                s.is_desktop && s.middle_preview_draft
            });
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "live-quick-time" => with_state_mut(|s| s.live_quick_time = val),
        "live-quick-title" => with_state_mut(|s| s.live_quick_title = val),
        "live-quick-body" => {
            if let Ok(ta) = el.clone().dyn_into::<HtmlTextAreaElement>() {
                auto_fit_textarea(&ta);
            }
            with_state_mut(|s| s.live_quick_body = val);
        }
        "live-entry-time" => {
            let bid = el.get_attribute("data-block-id").unwrap_or_default();
            with_state_mut(|s| {
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == bid && b.block_type == "live_entry" {
                            let existing_title = b
                                .meta
                                .as_deref()
                                .and_then(|m| m.split_once('|').map(|(_, t)| t))
                                .unwrap_or("");
                            b.meta = if existing_title.is_empty() {
                                Some(val.clone())
                            } else {
                                Some(format!("{val}|{existing_title}"))
                            };
                        }
                    }
                }
                save_editing_draft(&s.editing_article);
            });
        }
        "live-entry-title" => {
            let bid = el.get_attribute("data-block-id").unwrap_or_default();
            with_state_mut(|s| {
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == bid && b.block_type == "live_entry" {
                            let existing_time = b
                                .meta
                                .as_deref()
                                .map(|m| m.split_once('|').map(|(t, _)| t).unwrap_or(m))
                                .unwrap_or("");
                            b.meta = if val.trim().is_empty() {
                                Some(existing_time.to_string())
                            } else {
                                Some(format!("{existing_time}|{val}"))
                            };
                        }
                    }
                }
                save_editing_draft(&s.editing_article);
            });
        }
        "gallery-caption" => {
            let bid = el.get_attribute("data-block-id").unwrap_or_default();
            if let Some(idx) = el
                .get_attribute("data-item-idx")
                .and_then(|s| s.parse::<usize>().ok())
            {
                with_state_mut(|s| {
                    if let Some(blocks) = &mut s.editing_article.blocks {
                        for b in blocks.iter_mut() {
                            if b.id == bid && b.block_type == "gallery" {
                                let mut items: Vec<GalleryItem> =
                                    serde_json::from_str(b.content.trim()).unwrap_or_default();
                                if let Some(it) = items.get_mut(idx) {
                                    it.caption = val.clone();
                                    b.content = serde_json::to_string(&items)
                                        .unwrap_or_else(|_| "[]".to_string());
                                }
                            }
                        }
                    }
                    save_editing_draft(&s.editing_article);
                });
            }
        }
        "related-search" => {
            let bid = el.get_attribute("data-block-id").unwrap_or_default();
            with_state_mut(|s| {
                s.related_search_queries.insert(bid.clone(), val);
            });
            refresh_related_search_results(&bid);
        }
        "media-search-query" => {
            with_state_mut(|s| s.media_search_query = val);
            api::trigger_debounced_media_search();
        }
        "editor-slug" => {
            let has_trailing_dash = val.ends_with('-') || val.ends_with(' ');
            let mut norm = slugify_turkish(&val);
            if has_trailing_dash && !norm.is_empty() && !norm.ends_with('-') {
                norm.push('-');
            }
            with_state_mut(|s| {
                s.editing_article.slug = if norm.trim_matches('-').is_empty() {
                    None
                } else {
                    Some(norm.trim_matches('-').to_string())
                };
                save_editing_draft(&s.editing_article);
            });
            refresh_step3_live_seo();
        }
        "tag-query" => {
            if val.contains(',') {
                let trailing = val.rsplit_once(',').map(|(_, r)| r).unwrap_or("");
                let prefix = val.rsplit_once(',').map(|(p, _)| p).unwrap_or("");
                let _ = add_tags_from_str(prefix);
                let rem = trailing.trim_start().to_string();
                with_state_mut(|s| s.tag_input_query = rem.clone());
                if let Ok(inp) = el.clone().dyn_into::<HtmlInputElement>() {
                    inp.set_value(&rem);
                }
                refresh_step3_tags_ui();
                api::trigger_debounced_tag_search();
            } else {
                with_state_mut(|s| s.tag_input_query = val);
                api::trigger_debounced_tag_search();
            }
        }

        "rich-block" => {
            rich_editor::sync_rich_block_from_element(el);
            let (mid_prev, is_en) =
                with_state(|s| (s.is_desktop && s.middle_preview_draft, s.editor_lang == "en"));
            if is_en {
                refresh_step1_glossary_slots();
            }
            if mid_prev {
                refresh_middle_panel();
            }
        }

        "block-content" => {
            let bid = el.get_attribute("data-block-id").unwrap_or_default();
            let bkind = el.get_attribute("data-block-kind").unwrap_or_default();
            if let Ok(ta) = el.clone().dyn_into::<HtmlTextAreaElement>() {
                auto_fit_textarea(&ta);
            }
            let (cur_meta, mid_prev, is_en) = with_state_mut(|s| {
                let mut found_meta = None;
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == bid {
                            b.content = val.clone();
                            found_meta = b.meta.clone();
                        }
                    }
                }
                save_editing_draft(&s.editing_article);
                (
                    found_meta,
                    s.is_desktop && s.middle_preview_draft,
                    s.editor_lang == "en",
                )
            });
            if bkind == "image" {
                refresh_single_block_image_preview(&bid, &val);
            } else if bkind == "embed" {
                refresh_single_block_embed_preview(&bid, &val, cur_meta.as_deref());
            }
            if is_en {
                refresh_step1_glossary_slots();
            }
            if mid_prev {
                refresh_middle_panel();
            }
        }

        "block-meta" => {
            let bid = el.get_attribute("data-block-id").unwrap_or_default();
            let bkind = el.get_attribute("data-block-kind").unwrap_or_default();
            let (cur_content, is_en) = with_state_mut(|s| {
                let mut found_content = String::new();
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == bid {
                            found_content = b.content.clone();
                            b.meta = if val.trim().is_empty() {
                                None
                            } else {
                                Some(val.clone())
                            };
                        }
                    }
                }
                save_editing_draft(&s.editing_article);
                (found_content, s.editor_lang == "en")
            });
            if bkind == "embed" {
                let cap_opt = (!val.trim().is_empty()).then_some(val.as_str());
                refresh_single_block_embed_preview(&bid, &cur_content, cap_opt);
            }
            if is_en {
                refresh_step1_glossary_slots();
            }
        }

        "callout-title" => {
            let bid = el.get_attribute("data-block-id").unwrap_or_default();
            let variant = el
                .get_attribute("data-callout-variant")
                .unwrap_or_else(|| "info".to_string());
            let is_en = with_state_mut(|s| {
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == bid {
                            b.meta = Some(format!("{variant}|{val}"));
                        }
                    }
                }
                save_editing_draft(&s.editing_article);
                s.editor_lang == "en"
            });
            if is_en {
                refresh_step1_glossary_slots();
            }
        }

        "step2-remote-url" => with_state_mut(|s| s.step2_remote_url = val),

        "crop-zoom" => {
            if let Ok(z) = val.parse::<f64>() {
                with_state_mut(|s| s.crop_zoom = z.clamp(1.0, 3.0));
                redraw_cropper_canvas();
            }
        }

        "seo-title" => {
            with_state_mut(|s| {
                let seo = s.editing_article.seo.get_or_insert_with(SeoData::default);
                seo.title = val;
                save_editing_draft(&s.editing_article);
            });
            refresh_step3_live_seo();
        }
        "seo-description" => {
            with_state_mut(|s| {
                let seo = s.editing_article.seo.get_or_insert_with(SeoData::default);
                seo.description = val;
                save_editing_draft(&s.editing_article);
            });
            refresh_step3_live_seo();
        }
        "seo-keywords" => {
            with_state_mut(|s| {
                let seo = s.editing_article.seo.get_or_insert_with(SeoData::default);
                seo.keywords = val;
                save_editing_draft(&s.editing_article);
            });
            refresh_step3_live_seo();
        }

        "scheduled-at" => {
            let iso = datetime_local_to_iso(&val);
            let dis = with_state_mut(|s| {
                s.editing_article.scheduled_at = iso.clone();
                s.editing_article.scheduled_at_camel = iso.clone();
                save_editing_draft(&s.editing_article);
                s.is_posting || iso.is_none()
            });
            set_button_disabled("btn-schedule-post", dis);
        }

        // Category & Team Settings fields
        "new-cat-name" => {
            let auto_ph = if val.trim().is_empty() {
                "otomatik-kisa-ad (opsiyonel)".to_string()
            } else {
                crate::editorial::common::slugify_turkish(&val)
            };
            with_state_mut(|s| s.new_cat_name = val);
            if let Some(slug_el) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("hf-new-cat-slug"))
            {
                let _ = slug_el.set_attribute("placeholder", &auto_ph);
            }
        }
        "new-cat-slug" => with_state_mut(|s| s.new_cat_slug = val),
        "edit-cat-name" => with_state_mut(|s| s.edit_cat_name = val),
        "edit-cat-slug" => with_state_mut(|s| s.edit_cat_slug = val),
        "new-user-username" => with_state_mut(|s| s.new_user_username = val),
        "new-user-password" => with_state_mut(|s| s.new_user_password = val),

        // Sources Settings fields
        "new-source-url" => with_state_mut(|s| s.new_source_url = val),
        "new-source-name" => with_state_mut(|s| s.new_source_name = val),
        "edit-source-name" => with_state_mut(|s| s.edit_source_name = val),
        "edit-source-url" => with_state_mut(|s| s.edit_source_url = val),

        // Step 4 MSI Editorial & Sponsorship inputs
        "article-issue-pages" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.issue_page_range = if val.trim().is_empty() {
                    None
                } else {
                    Some(val)
                };
                save_editing_draft(&s.editing_article);
                s.is_desktop && s.middle_preview_draft
            });
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "article-single-price" => {
            let parsed = val
                .trim()
                .replace(',', ".")
                .parse::<f64>()
                .unwrap_or(0.0)
                .max(0.0);
            let mid_prev = with_state_mut(|s| {
                s.editing_article.single_price_try = parsed;
                save_editing_draft(&s.editing_article);
                s.is_desktop && s.middle_preview_draft
            });
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "article-sponsor-brand" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.sponsor_brand = if val.trim().is_empty() {
                    None
                } else {
                    Some(val)
                };
                save_editing_draft(&s.editing_article);
                s.is_desktop && s.middle_preview_draft
            });
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "article-sponsor-label" => {
            let mid_prev = with_state_mut(|s| {
                s.editing_article.sponsor_legal_label = if val.trim().is_empty() {
                    None
                } else {
                    Some(val)
                };
                save_editing_draft(&s.editing_article);
                s.is_desktop && s.middle_preview_draft
            });
            if mid_prev {
                refresh_middle_panel();
            }
        }

        // Settings: Magazine Issues & Glossary inputs
        "new-issue-number" => with_state_mut(|s| s.new_issue_number = val),
        "new-issue-title-tr" => with_state_mut(|s| s.new_issue_title_tr = val),
        "new-issue-title-en" => with_state_mut(|s| s.new_issue_title_en = val),
        "new-issue-month" => with_state_mut(|s| s.new_issue_month = val),
        "new-issue-year" => with_state_mut(|s| s.new_issue_year = val),
        "new-issue-event" => with_state_mut(|s| s.new_issue_event = val),
        "settings-glossary-query" => {
            with_state_mut(|s| s.settings_glossary_query = val);
            refresh_settings_glossary_list_slot();
        }
        "new-glossary-term-tr" => with_state_mut(|s| s.new_glossary_term_tr = val),
        "new-glossary-abbr-tr" => with_state_mut(|s| s.new_glossary_abbr_tr = val),
        "new-glossary-preferred-en" => with_state_mut(|s| s.new_glossary_preferred_en = val),
        "new-glossary-abbr-en" => with_state_mut(|s| s.new_glossary_abbr_en = val),
        "new-glossary-forbidden" => with_state_mut(|s| s.new_glossary_forbidden = val),
        "new-glossary-note" => with_state_mut(|s| s.new_glossary_note = val),

        "pw-current" => {
            let dis = with_state_mut(|s| {
                s.change_pw_current = val;
                s.is_changing_password
                    || s.change_pw_current.is_empty()
                    || s.change_pw_new.is_empty()
                    || s.change_pw_confirm.is_empty()
            });
            set_button_disabled("btn-submit-change-pw", dis);
        }
        "pw-new" => {
            let dis = with_state_mut(|s| {
                s.change_pw_new = val;
                s.is_changing_password
                    || s.change_pw_current.is_empty()
                    || s.change_pw_new.is_empty()
                    || s.change_pw_confirm.is_empty()
            });
            set_button_disabled("btn-submit-change-pw", dis);
        }
        "pw-confirm" => {
            let dis = with_state_mut(|s| {
                s.change_pw_confirm = val;
                s.is_changing_password
                    || s.change_pw_current.is_empty()
                    || s.change_pw_new.is_empty()
                    || s.change_pw_confirm.is_empty()
            });
            set_button_disabled("btn-submit-change-pw", dis);
        }
        _ => {}
    }
}

fn handle_change_event(kind: &str, el: &Element) {
    match kind {
        "new-user-role" => {
            let val = read_input_value(el);
            with_state_mut(|s| s.new_user_role = val);
        }
        "new-source-category" => {
            let val = read_input_value(el);
            with_state_mut(|s| s.new_source_category = val);
        }
        "new-source-scope" => {
            let val = read_input_value(el);
            with_state_mut(|s| s.new_source_scope = val);
        }
        "edit-source-category" => {
            let val = read_input_value(el);
            with_state_mut(|s| s.edit_source_category = val);
        }
        "edit-source-scope" => {
            let val = read_input_value(el);
            with_state_mut(|s| s.edit_source_scope = val);
        }
        "new-cat-parent" => {
            let pid = read_input_value(el)
                .parse::<i32>()
                .ok()
                .filter(|v| *v > 0);
            with_state_mut(|s| s.new_cat_parent_id = pid);
            refresh_settings_popover();
        }
        "edit-cat-parent" => {
            let pid = read_input_value(el)
                .parse::<i32>()
                .ok()
                .filter(|v| *v > 0);
            with_state_mut(|s| s.edit_cat_parent_id = pid);
        }
        "step4-parent-category" => {
            let rid = read_input_value(el)
                .parse::<i32>()
                .ok()
                .filter(|v| *v > 0);
            let mid_prev = with_state_mut(|s| {
                s.editing_article.categories = rid.map(|id| vec![id]);
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "step4-sub-category" => {
            let sid = read_input_value(el)
                .parse::<i32>()
                .ok()
                .filter(|v| *v > 0);
            let mid_prev = with_state_mut(|s| {
                if let Some(leaf_id) = sid {
                    let chain = crate::editorial::common::category_ancestor_chain(&s.categories, leaf_id);
                    s.editing_article.categories = if chain.is_empty() {
                        Some(vec![leaf_id])
                    } else {
                        Some(chain)
                    };
                } else {
                    // Keep only the currently selected root category when sub-category is cleared to "0"
                    let cur_root = s
                        .editing_article
                        .categories
                        .as_deref()
                        .and_then(|ids| ids.first().copied())
                        .and_then(|first_id| {
                            crate::editorial::common::category_ancestor_chain(&s.categories, first_id)
                                .first()
                                .copied()
                        });
                    s.editing_article.categories = cur_root.map(|rid| vec![rid]);
                }
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "change-block-type" => {
            let bid = el.get_attribute("data-block-id").unwrap_or_default();
            let new_type = read_input_value(el);
            with_state_mut(|s| {
                if let Some(blocks) = &mut s.editing_article.blocks {
                    for b in blocks.iter_mut() {
                        if b.id == bid {
                            if new_type == "gallery" && b.block_type != "gallery" {
                                b.content = "[]".to_string();
                                b.meta = None;
                            } else if new_type == "related_article"
                                && b.block_type != "related_article"
                            {
                                b.content = "{}".to_string();
                                b.meta = Some("BUNU DA OKUYUN".to_string());
                            } else {
                                b.content = rich_editor::format_content_for_block_type(
                                    &b.content, &new_type,
                                );
                                if new_type == "callout"
                                    && !b
                                        .meta
                                        .as_deref()
                                        .map(|m| m.contains('|'))
                                        .unwrap_or(false)
                                {
                                    b.meta = Some("info|".to_string());
                                } else if new_type == "live_entry" && b.meta.is_none() {
                                    b.meta = Some(format!("{}|", js_now_hhmm()));
                                } else if new_type == "summary" && b.meta.is_none() {
                                    b.meta = Some("Öne Çıkanlar".to_string());
                                } else if !matches!(
                                    new_type.as_str(),
                                    "image"
                                        | "quote"
                                        | "callout"
                                        | "summary"
                                        | "embed"
                                        | "gallery"
                                        | "related_article"
                                        | "live_entry"
                                ) {
                                    b.meta = None;
                                }
                            }
                            b.block_type = new_type.clone();
                        }
                    }
                }
                save_editing_draft(&s.editing_article);
            });
            rich_editor::hide_selection_bubble();
            refresh_blocks_list_only();
            rich_editor::focus_block_by_id(&bid);
        }
        "article-status" => {
            let val = read_input_value(el);
            with_state_mut(|s| {
                s.editing_article.status = val;
                save_editing_draft(&s.editing_article);
            });
            refresh_right_panel();
        }
        "article-en-status" => {
            let val = read_input_value(el);
            let mid_prev = with_state_mut(|s| {
                s.editing_article.en_status = val;
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "article-content-type" => {
            let val = read_input_value(el);
            let mid_prev = with_state_mut(|s| {
                s.editing_article.content_type = val;
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "article-issue-number" => {
            let inum = read_input_value(el)
                .parse::<i32>()
                .ok()
                .filter(|v| *v > 0);
            let mid_prev = with_state_mut(|s| {
                s.editing_article.issue_number = inum;
                save_editing_draft(&s.editing_article);
                s.middle_preview_draft
            });
            refresh_right_panel();
            if mid_prev {
                refresh_middle_panel();
            }
        }
        "new-issue-lang" => {
            let val = read_input_value(el);
            with_state_mut(|s| s.new_issue_lang = val);
        }
        "new-glossary-category" => {
            let val = read_input_value(el);
            with_state_mut(|s| s.new_glossary_category = val);
        }
        "new-glossary-rule-type" => {
            let val = read_input_value(el);
            with_state_mut(|s| s.new_glossary_rule_type = val);
        }
        _ => {}
    }
}

fn handle_file_input(kind: &str, el: &Element) {
    let Ok(inp) = el.clone().dyn_into::<HtmlInputElement>() else {
        return;
    };
    let Some(files) = inp.files() else { return };
    if files.length() == 0 {
        return;
    }

    match kind {
        "step2-image" => {
            if let Some(file) = files.get(0) {
                inp.set_value("");
                api::upload_file_for_step2(file);
            }
        }
        "block-image" => {
            let bid = inp.get_attribute("data-block-id").unwrap_or_default();
            if let Some(file) = files.get(0) {
                inp.set_value("");
                if !bid.is_empty() {
                    api::upload_file_for_block(bid, file);
                }
            }
        }
        "gallery-images" => {
            let bid = inp.get_attribute("data-block-id").unwrap_or_default();
            let mut list: Vec<File> = Vec::with_capacity(files.length() as usize);
            for i in 0..files.length() {
                if let Some(f) = files.get(i) {
                    list.push(f);
                }
            }
            inp.set_value("");
            if !bid.is_empty() && !list.is_empty() {
                api::upload_files_for_gallery(bid, list);
            }
        }
        "media-library-upload" => {
            let mut list: Vec<File> = Vec::with_capacity(files.length() as usize);
            for i in 0..files.length() {
                if let Some(f) = files.get(i) {
                    list.push(f);
                }
            }
            inp.set_value("");
            if !list.is_empty() {
                api::upload_files_to_media_library(list);
            }
        }
        _ => {}
    }
}
