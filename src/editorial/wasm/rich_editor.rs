//! Notion/Medium-grade Floating Text Selection Bubble (`#hf-selection-bubble`),
//! live `contenteditable` WYSIWYG rich-block synchronization, smart Range split/convert,
//! inline `<mark>` highlight & URL link popover, and clean plain-text paste handling.

use std::cell::RefCell;

use crate::editorial::common::{sanitize_inline_rich_html, strip_all_html_tags, ContentBlock};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
use web_sys::{ClipboardEvent, Element, KeyboardEvent, MouseEvent};
#[cfg(target_arch = "wasm32")]
use web_sys::{Document, HtmlDocument, HtmlElement, HtmlInputElement, Node, Range};

use crate::editorial::wasm::icons;
#[cfg(target_arch = "wasm32")]
use crate::editorial::wasm::render::refresh_blocks_list_only;
#[cfg(target_arch = "wasm32")]
use crate::editorial::wasm::state::{save_editing_draft, with_state, with_state_mut};

#[cfg(target_arch = "wasm32")]
fn range_clone_contents(range: &Range) -> Result<Node, JsValue> {
    let f = js_sys::Reflect::get(range.as_ref(), &JsValue::from_str("cloneContents"))?
        .dyn_into::<js_sys::Function>()?;
    f.call0(range.as_ref())?.dyn_into::<Node>()
}

#[cfg(target_arch = "wasm32")]
fn range_extract_contents(range: &Range) -> Result<Node, JsValue> {
    let f = js_sys::Reflect::get(range.as_ref(), &JsValue::from_str("extractContents"))?
        .dyn_into::<js_sys::Function>()?;
    f.call0(range.as_ref())?.dyn_into::<Node>()
}

#[cfg(target_arch = "wasm32")]
fn doc_create_text_node(document: &Document, text: &str) -> Option<Node> {
    let f = js_sys::Reflect::get(document.as_ref(), &JsValue::from_str("createTextNode"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    f.call1(document.as_ref(), &JsValue::from_str(text))
        .ok()?
        .dyn_into::<Node>()
        .ok()
}

#[derive(Default)]
struct BubbleRuntimeState {
    active_block_id: Option<String>,
    #[cfg(target_arch = "wasm32")]
    saved_range: Option<Range>,
    link_mode_open: bool,
}

thread_local! {
    static BUBBLE_STATE: RefCell<BubbleRuntimeState> = RefCell::new(BubbleRuntimeState::default());
}

/// Renders the persistent floating selection toolbar shell mounted once inside `#root`.
pub fn render_selection_bubble_shell() -> String {
    format!(
        r#"<div id="hf-selection-bubble" role="toolbar" aria-label="Metin Biçimlendirme Araçları" class="hf-selection-bubble fixed z-[250] bg-zinc-900/95 backdrop-blur-md text-white shadow-2xl rounded-2xl border border-zinc-700/80 px-2 py-1.5 flex items-center gap-0.5 opacity-0 pointer-events-none transition-all duration-150">
            <div id="hf-bubble-tools" class="flex items-center gap-0.5">
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="block-paragraph" class="hf-bubble-btn" title="Metin Paragrafı">P</button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="block-heading1" class="hf-bubble-btn" title="H1 Ana Başlık">H1</button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="block-heading" class="hf-bubble-btn" title="H2 Alt Başlık">H2</button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="block-heading3" class="hf-bubble-btn" title="H3 Ara Başlık">H3</button>
                <span class="hf-bubble-sep"></span>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="bold" class="hf-bubble-btn font-black" title="Kalın (Ctrl+B)">B</button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="italic" class="hf-bubble-btn italic font-serif font-bold" title="İtalik (Ctrl+I)">I</button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="underline" class="hf-bubble-btn underline underline-offset-2 font-bold" title="Altı Çizili (Ctrl+U)">U</button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="strikeThrough" class="hf-bubble-btn line-through font-bold" title="Üstü Çizili">S</button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="highlight" class="hf-bubble-btn hf-bubble-mark-btn" title="Fosforlu Sarı Vurgu">
                    {mark_ico}
                    <span class="hf-bubble-mark-dot"></span>
                </button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="open-link" class="hf-bubble-btn" title="Bağlantı Ekle / Düzenle (Ctrl+K)">
                    {link_ico}
                </button>
                <span class="hf-bubble-sep"></span>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="block-quote" class="hf-bubble-btn" title="Alıntı Bloğuna Dönüştür">
                    {quote_ico}
                </button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="block-bullet-list" class="hf-bubble-btn" title="Madde Listesine Dönüştür">
                    {list_ico}
                </button>
            </div>
            <div id="hf-bubble-link-bar" class="hidden items-center gap-1.5 px-1">
                {link_bar_ico}
                <input id="hf-bubble-link-input" type="url" placeholder="https://..." class="hf-bubble-url-input" />
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="apply-link" class="hf-bubble-action-btn hf-bubble-action-primary">Ekle</button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="remove-link" class="hf-bubble-action-btn hf-bubble-action-danger" title="Bağlantıyı Kaldır">Kaldır</button>
                <button type="button" data-action="bubble-cmd" data-bubble-cmd="close-link" class="hf-bubble-close-btn" title="Kapat">{close_ico}</button>
            </div>
            <div id="hf-bubble-arrow" class="hf-bubble-arrow"></div>
        </div>"#,
        mark_ico = icons::highlighter("w-3.5 h-3.5"),
        link_ico = icons::link2("w-3.5 h-3.5"),
        quote_ico = icons::quote("w-3.5 h-3.5"),
        list_ico = icons::list("w-3.5 h-3.5"),
        link_bar_ico = icons::link2("w-3.5 h-3.5 text-brand-500 shrink-0"),
        close_ico = icons::x_mark("w-3.5 h-3.5"),
    )
}

/// Normalizes user-entered link URLs safely (auto-prefixing `https://` for bare domains, rejecting `javascript:`).
pub fn normalize_editor_link_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
    {
        return None;
    }
    if lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("mailto:")
        || trimmed.starts_with('/')
    {
        Some(trimmed.to_string())
    } else {
        Some(format!("https://{trimmed}"))
    }
}

/// Formats rich HTML content appropriately for a target block type.
pub fn format_content_for_block_type(raw_html: &str, target_type: &str) -> String {
    match target_type {
        "heading1" | "heading" | "heading3" => strip_all_html_tags(raw_html)
            .replace('\n', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        "bullet_list" | "numbered_list" | "summary" => raw_html
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("<br>", "\n")
            .lines()
            .map(|l| sanitize_inline_rich_html(l.trim().trim_start_matches(['•', '-', '*']).trim()))
            .filter(|l| !strip_all_html_tags(l).trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => sanitize_inline_rich_html(raw_html.trim()),
    }
}

/// Pure smart-split / block-conversion engine:
/// - Converts the entire block if >= 70% of its plain text is selected, if converting to `paragraph`,
///   or if the current block is already a heading.
/// - Otherwise splits `[before, converted_selection, after]` cleanly without losing surrounding text!
pub fn build_smart_converted_blocks(
    orig_type: &str,
    orig_full_html: &str,
    orig_meta: Option<&str>,
    before_html: &str,
    selected_html: &str,
    after_html: &str,
    target_type: &str,
) -> Vec<(String, String, Option<String>)> {
    let total_chars = strip_all_html_tags(orig_full_html).trim().chars().count();
    let sel_chars = strip_all_html_tags(selected_html).trim().chars().count();
    let orig_is_heading = matches!(orig_type, "heading1" | "heading" | "heading3");

    let convert_whole = target_type == "paragraph"
        || orig_is_heading
        || total_chars == 0
        || (sel_chars as f64) >= (total_chars as f64) * 0.70;

    if convert_whole {
        let preserved_meta = if target_type == orig_type
            || (matches!(target_type, "quote" | "callout") && orig_type == target_type)
        {
            orig_meta.map(|m| m.to_string())
        } else {
            None
        };
        return vec![(
            target_type.to_string(),
            format_content_for_block_type(orig_full_html, target_type),
            preserved_meta,
        )];
    }

    let mut out = Vec::with_capacity(3);
    let before_clean = sanitize_inline_rich_html(before_html.trim());
    if !strip_all_html_tags(&before_clean).trim().is_empty() {
        out.push((
            orig_type.to_string(),
            before_clean,
            orig_meta.map(|m| m.to_string()),
        ));
    }

    let mid_clean = format_content_for_block_type(selected_html, target_type);
    if !strip_all_html_tags(&mid_clean).trim().is_empty() {
        out.push((target_type.to_string(), mid_clean, None));
    }

    let after_clean = sanitize_inline_rich_html(after_html.trim());
    if !strip_all_html_tags(&after_clean).trim().is_empty() {
        out.push((orig_type.to_string(), after_clean, None));
    }

    if out.is_empty() {
        out.push((
            target_type.to_string(),
            format_content_for_block_type(orig_full_html, target_type),
            None,
        ));
    }
    out
}

/// Converts stored `ContentBlock.content` into safe `contenteditable` inner HTML (empty when blank so `:empty::before` shows).
pub fn rich_block_editable_inner_html(content: &str) -> String {
    let sanitized = sanitize_inline_rich_html(content.trim());
    if strip_all_html_tags(&sanitized).trim().is_empty() {
        String::new()
    } else {
        sanitized.replace('\n', "<br/>")
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn hide_selection_bubble() {
    let _ = ContentBlock::default();
    BUBBLE_STATE.with(|st| {
        let mut b = st.borrow_mut();
        b.active_block_id = None;
        b.link_mode_open = false;
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn update_selection_bubble_from_dom() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_bubble_mousedown(_el: &Element, _ev: &MouseEvent) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_bubble_command(_cmd: &str) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn sync_rich_block_from_element(_rich_el: &Element) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_rich_paste(_el: &Element, _ev: &ClipboardEvent) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_rich_keydown(_el: &Element, _ev: &KeyboardEvent) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn focus_block_by_id(_block_id: &str) {}

// ============================================================================
// Browser WASM DOM Selection, Range & ContentEditable Implementation
// ============================================================================

#[cfg(target_arch = "wasm32")]
fn doc() -> Option<Document> {
    web_sys::window()?.document()
}

#[cfg(target_arch = "wasm32")]
fn node_closest_element(node: &Node, selector: &str) -> Option<Element> {
    if let Ok(el) = node.clone().dyn_into::<Element>() {
        el.closest(selector).ok().flatten()
    } else {
        node.parent_element()?.closest(selector).ok().flatten()
    }
}

#[cfg(target_arch = "wasm32")]
fn set_link_bar_visibility(show_link: bool) {
    let Some(document) = doc() else { return };
    if let Some(tools) = document.get_element_by_id("hf-bubble-tools") {
        if show_link {
            let _ = tools.class_list().add_1("hidden");
        } else {
            let _ = tools.class_list().remove_1("hidden");
        }
    }
    if let Some(bar) = document.get_element_by_id("hf-bubble-link-bar") {
        if show_link {
            let _ = bar.class_list().remove_1("hidden");
            let _ = bar.class_list().add_1("flex");
        } else {
            let _ = bar.class_list().add_1("hidden");
            let _ = bar.class_list().remove_1("flex");
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn hide_selection_bubble() {
    BUBBLE_STATE.with(|st| {
        let mut b = st.borrow_mut();
        b.link_mode_open = false;
        b.active_block_id = None;
        b.saved_range = None;
    });
    set_link_bar_visibility(false);
    let Some(document) = doc() else { return };
    if let Some(bubble) = document.get_element_by_id("hf-selection-bubble") {
        let _ = bubble.class_list().remove_1("hf-bubble-visible");
        let _ = bubble.class_list().remove_1("hf-bubble-below");
        let _ = bubble.class_list().add_2("opacity-0", "pointer-events-none");
    }
}

#[cfg(target_arch = "wasm32")]
fn update_bubble_active_buttons(rich_el: &Element, anchor_node: &Node) {
    let Some(document) = doc() else { return };
    let Some(bubble) = document.get_element_by_id("hf-selection-bubble") else {
        return;
    };
    let bkind = rich_el.get_attribute("data-block-kind").unwrap_or_default();
    let in_bold = node_closest_element(anchor_node, "strong, b")
        .filter(|e| rich_el.contains(Some(e)))
        .is_some();
    let in_italic = node_closest_element(anchor_node, "em, i")
        .filter(|e| rich_el.contains(Some(e)))
        .is_some();
    let in_underline = node_closest_element(anchor_node, "u")
        .filter(|e| rich_el.contains(Some(e)))
        .is_some();
    let in_strike = node_closest_element(anchor_node, "s, strike, del")
        .filter(|e| rich_el.contains(Some(e)))
        .is_some();
    let in_mark = node_closest_element(anchor_node, "mark")
        .filter(|e| rich_el.contains(Some(e)))
        .is_some();
    let in_link = node_closest_element(anchor_node, "a")
        .filter(|e| rich_el.contains(Some(e)))
        .is_some();

    if let Ok(btns) = bubble.query_selector_all("[data-bubble-cmd]") {
        for idx in 0..btns.length() {
            if let Some(Ok(btn)) = btns.item(idx).map(|n| n.dyn_into::<Element>()) {
                let cmd = btn.get_attribute("data-bubble-cmd").unwrap_or_default();
                let is_active = match cmd.as_str() {
                    "block-paragraph" => bkind == "paragraph",
                    "block-heading1" => bkind == "heading1",
                    "block-heading" => bkind == "heading",
                    "block-heading3" => bkind == "heading3",
                    "block-quote" => bkind == "quote",
                    "bold" => in_bold,
                    "italic" => in_italic,
                    "underline" => in_underline,
                    "strikeThrough" => in_strike,
                    "highlight" => in_mark,
                    "open-link" => in_link,
                    _ => false,
                };
                if is_active {
                    let _ = btn.class_list().add_1("hf-bubble-btn-active");
                } else {
                    let _ = btn.class_list().remove_1("hf-bubble-btn-active");
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn position_bubble_over_rect(rect: &web_sys::DomRect) {
    let Some(win) = web_sys::window() else { return };
    let Some(document) = win.document() else {
        return;
    };
    let Some(bubble) = document
        .get_element_by_id("hf-selection-bubble")
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };

    let measured_w = bubble.get_bounding_client_rect().width();
    let bubble_w = if measured_w > 40.0 { measured_w } else { 420.0 };
    let win_w = win
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1280.0);

    let center_x = rect.left() + rect.width() / 2.0;
    let max_left = (win_w - bubble_w - 12.0).max(12.0);
    let left = (center_x - bubble_w / 2.0).clamp(12.0, max_left);

    let place_below = rect.top() < 64.0;
    let top = if place_below {
        rect.bottom() + 10.0
    } else {
        rect.top() - 48.0
    };

    let _ = bubble.style().set_property("left", &format!("{left:.1}px"));
    let _ = bubble.style().set_property("top", &format!("{top:.1}px"));

    if place_below {
        let _ = bubble.class_list().add_1("hf-bubble-below");
    } else {
        let _ = bubble.class_list().remove_1("hf-bubble-below");
    }

    if let Some(arrow) = document
        .get_element_by_id("hf-bubble-arrow")
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let arrow_x = (center_x - left).clamp(18.0, (bubble_w - 18.0).max(18.0));
        let _ = arrow
            .style()
            .set_property("left", &format!("{arrow_x:.1}px"));
    }

    let _ = bubble
        .class_list()
        .remove_2("opacity-0", "pointer-events-none");
    let _ = bubble.class_list().add_1("hf-bubble-visible");
}

#[cfg(target_arch = "wasm32")]
pub fn update_selection_bubble_from_dom() {
    let step_ok = with_state(|s| s.editor_step == 1 && s.token.is_some());
    if !step_ok {
        hide_selection_bubble();
        return;
    }

    let Some(win) = web_sys::window() else { return };
    let Some(document) = win.document() else {
        return;
    };

    let link_open = BUBBLE_STATE.with(|st| st.borrow().link_mode_open);
    if link_open {
        if let Some(active) = document.active_element() {
            if active.closest("#hf-selection-bubble").ok().flatten().is_some() {
                return;
            }
        }
    }

    let Ok(Some(sel)) = win.get_selection() else {
        hide_selection_bubble();
        return;
    };
    if sel.is_collapsed() || sel.range_count() == 0 {
        if !link_open {
            hide_selection_bubble();
        }
        return;
    }

    let selected_text = sel.to_string().as_string().unwrap_or_default();
    if selected_text.trim().is_empty() {
        hide_selection_bubble();
        return;
    }

    let Some(anchor) = sel.anchor_node() else {
        hide_selection_bubble();
        return;
    };
    let Some(focus) = sel.focus_node() else {
        hide_selection_bubble();
        return;
    };
    if !document.contains(Some(&anchor)) || !document.contains(Some(&focus)) {
        hide_selection_bubble();
        return;
    }

    let Some(anchor_blk) = node_closest_element(&anchor, "[data-rich-block='true']") else {
        hide_selection_bubble();
        return;
    };
    let Some(focus_blk) = node_closest_element(&focus, "[data-rich-block='true']") else {
        hide_selection_bubble();
        return;
    };
    if anchor_blk.closest("#right-panel").ok().flatten().is_none() {
        hide_selection_bubble();
        return;
    }

    let block_id = anchor_blk.get_attribute("data-block-id").unwrap_or_default();
    let focus_id = focus_blk.get_attribute("data-block-id").unwrap_or_default();
    if block_id.is_empty() || block_id != focus_id {
        hide_selection_bubble();
        return;
    }

    let Ok(range) = sel.get_range_at(0) else {
        hide_selection_bubble();
        return;
    };
    let rect = range.get_bounding_client_rect();
    if rect.width() <= 0.0 && rect.height() <= 0.0 {
        hide_selection_bubble();
        return;
    }

    BUBBLE_STATE.with(|st| {
        let mut b = st.borrow_mut();
        b.active_block_id = Some(block_id);
        b.saved_range = Some(range.clone_range());
    });

    if !link_open {
        set_link_bar_visibility(false);
    }
    update_bubble_active_buttons(&anchor_blk, &anchor);
    position_bubble_over_rect(&rect);
}

#[cfg(target_arch = "wasm32")]
pub fn handle_bubble_mousedown(el: &Element, ev: &MouseEvent) -> bool {
    if el.closest("#hf-selection-bubble").ok().flatten().is_some() {
        // Allow clicking/typing inside the URL input without blocking focus!
        if !el.tag_name().eq_ignore_ascii_case("INPUT") {
            ev.prevent_default();
        }
        return true;
    }

    let was_link_open = BUBBLE_STATE.with(|st| st.borrow().link_mode_open);
    if was_link_open {
        hide_selection_bubble();
    }
    false
}

#[cfg(target_arch = "wasm32")]
pub fn sync_rich_block_from_element(rich_el: &Element) {
    let bid = rich_el.get_attribute("data-block-id").unwrap_or_default();
    if bid.is_empty() {
        return;
    }

    let raw_inner = rich_el.inner_html();
    let plain_txt = rich_el.text_content().unwrap_or_default();
    if plain_txt.is_empty() && raw_inner.trim().eq_ignore_ascii_case("<br>") {
        rich_el.set_inner_html("");
    }

    // Ensure all inline links inside contenteditable carry safe target/rel
    if let Ok(links) = rich_el.query_selector_all("a[href]") {
        for idx in 0..links.length() {
            if let Some(Ok(a_el)) = links.item(idx).map(|n| n.dyn_into::<Element>()) {
                let _ = a_el.set_attribute("target", "_blank");
                let _ = a_el.set_attribute("rel", "noopener noreferrer");
            }
        }
    }

    let bkind = rich_el.get_attribute("data-block-kind").unwrap_or_default();
    let clean = format_content_for_block_type(&rich_el.inner_html(), &bkind);

    with_state_mut(|s| {
        if let Some(blocks) = &mut s.editing_article.blocks {
            for b in blocks.iter_mut() {
                if b.id == bid {
                    b.content = clean.clone();
                }
            }
        }
        save_editing_draft(&s.editing_article);
    });
}

#[cfg(target_arch = "wasm32")]
fn find_rich_block_element(block_id: &str) -> Option<Element> {
    let document = doc()?;
    let sel = format!("[data-rich-block='true'][data-block-id='{block_id}']");
    document.query_selector(&sel).ok().flatten()
}

#[cfg(target_arch = "wasm32")]
fn restore_saved_selection() -> Option<(Element, Range)> {
    let document = doc()?;
    let (bid, range) = BUBBLE_STATE.with(|st| {
        let b = st.borrow();
        (b.active_block_id.clone(), b.saved_range.clone())
    });
    let bid = bid?;
    let range = range?;
    let rich_el = find_rich_block_element(&bid)?;

    if !document.contains(Some(rich_el.as_ref())) {
        hide_selection_bubble();
        return None;
    }
    let Ok(sc) = range.start_container() else {
        hide_selection_bubble();
        return None;
    };
    let Ok(ec) = range.end_container() else {
        hide_selection_bubble();
        return None;
    };
    if !rich_el.contains(Some(&sc)) || !rich_el.contains(Some(&ec)) {
        hide_selection_bubble();
        return None;
    }

    if let Ok(he) = rich_el.clone().dyn_into::<HtmlElement>() {
        let _ = he.focus();
    }
    if let Some(Ok(Some(sel))) = web_sys::window().map(|w| w.get_selection()) {
        let _ = sel.remove_all_ranges();
        let _ = sel.add_range(&range);
    }
    Some((rich_el, range))
}

#[cfg(target_arch = "wasm32")]
fn unwrap_element_preserving_children(el: &Element) -> Option<(Node, Node)> {
    let parent = el.parent_node()?;
    let mut first_moved: Option<Node> = None;
    let mut last_moved: Option<Node> = None;
    while let Some(child) = el.first_child() {
        let _ = parent.insert_before(&child, Some(el.as_ref()));
        if first_moved.is_none() {
            first_moved = Some(child.clone());
        }
        last_moved = Some(child);
    }
    el.remove();
    Some((first_moved?, last_moved?))
}

#[cfg(target_arch = "wasm32")]
fn toggle_mark_highlight() {
    let Some(document) = doc() else { return };
    let Some((rich_el, _)) = restore_saved_selection() else {
        return;
    };
    let Some(Ok(Some(sel))) = web_sys::window().map(|w| w.get_selection()) else {
        return;
    };
    if sel.range_count() == 0 {
        return;
    }
    let Ok(range) = sel.get_range_at(0) else {
        return;
    };

    let existing_mark = range
        .common_ancestor_container()
        .ok()
        .and_then(|n| node_closest_element(&n, "mark"))
        .or_else(|| {
            sel.anchor_node()
                .and_then(|n| node_closest_element(&n, "mark"))
        })
        .filter(|m| rich_el.contains(Some(m)));

    if let Some(mark_el) = existing_mark {
        if let Some((first_n, last_n)) = unwrap_element_preserving_children(&mark_el) {
            if let Ok(new_r) = document.create_range() {
                let _ = new_r.set_start_before(&first_n);
                let _ = new_r.set_end_after(&last_n);
                let _ = sel.remove_all_ranges();
                let _ = sel.add_range(&new_r);
            }
        }
    } else if let Ok(frag) = range_extract_contents(&range) {
        if let Ok(mark_el) = document.create_element("mark") {
            let _ = mark_el.append_child(&frag);
            let _ = range.insert_node(mark_el.as_ref());
            if let Ok(new_r) = document.create_range() {
                let _ = new_r.select_node_contents(mark_el.as_ref());
                let _ = sel.remove_all_ranges();
                let _ = sel.add_range(&new_r);
            }
        }
    }

    sync_rich_block_from_element(&rich_el);
    update_selection_bubble_from_dom();
}

#[cfg(target_arch = "wasm32")]
fn open_bubble_link_bar() {
    let Some(document) = doc() else { return };
    let Some((rich_el, range)) = restore_saved_selection() else {
        return;
    };

    let existing_href = range
        .common_ancestor_container()
        .ok()
        .and_then(|n| node_closest_element(&n, "a"))
        .filter(|a| rich_el.contains(Some(a)))
        .and_then(|a| a.get_attribute("href"))
        .unwrap_or_default();

    let rect = range.get_bounding_client_rect();
    BUBBLE_STATE.with(|st| {
        st.borrow_mut().link_mode_open = true;
    });
    set_link_bar_visibility(true);
    position_bubble_over_rect(&rect);

    if let Some(inp) = document
        .get_element_by_id("hf-bubble-link-input")
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
    {
        inp.set_value(&existing_href);
        let _ = inp.focus();
        inp.select();
    }
}

#[cfg(target_arch = "wasm32")]
fn apply_bubble_link() {
    let Some(document) = doc() else { return };
    let raw_url = document
        .get_element_by_id("hf-bubble-link-input")
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value())
        .unwrap_or_default();

    let Some(safe_url) = normalize_editor_link_url(&raw_url) else {
        close_bubble_link_bar();
        return;
    };

    BUBBLE_STATE.with(|st| {
        st.borrow_mut().link_mode_open = false;
    });
    set_link_bar_visibility(false);

    let Some((rich_el, _)) = restore_saved_selection() else {
        hide_selection_bubble();
        return;
    };

    if let Ok(html_doc) = document.clone().dyn_into::<HtmlDocument>() {
        let _ = html_doc.exec_command_with_show_ui_and_value("createLink", false, &safe_url);
    }

    sync_rich_block_from_element(&rich_el);
    update_selection_bubble_from_dom();
}

#[cfg(target_arch = "wasm32")]
fn remove_bubble_link() {
    let Some(document) = doc() else { return };
    BUBBLE_STATE.with(|st| {
        st.borrow_mut().link_mode_open = false;
    });
    set_link_bar_visibility(false);

    let Some((rich_el, range)) = restore_saved_selection() else {
        hide_selection_bubble();
        return;
    };

    let existing_a = range
        .common_ancestor_container()
        .ok()
        .and_then(|n| node_closest_element(&n, "a"))
        .filter(|a| rich_el.contains(Some(a)));

    if let Some(a_el) = existing_a {
        if let Some((first_n, last_n)) = unwrap_element_preserving_children(&a_el) {
            if let Some(Ok(Some(sel))) = web_sys::window().map(|w| w.get_selection()) {
                if let Ok(new_r) = document.create_range() {
                    let _ = new_r.set_start_before(&first_n);
                    let _ = new_r.set_end_after(&last_n);
                    let _ = sel.remove_all_ranges();
                    let _ = sel.add_range(&new_r);
                }
            }
        }
    } else if let Ok(html_doc) = document.clone().dyn_into::<HtmlDocument>() {
        let _ = html_doc.exec_command("unlink");
    }

    sync_rich_block_from_element(&rich_el);
    update_selection_bubble_from_dom();
}

#[cfg(target_arch = "wasm32")]
fn close_bubble_link_bar() {
    BUBBLE_STATE.with(|st| {
        st.borrow_mut().link_mode_open = false;
    });
    set_link_bar_visibility(false);
    let _ = restore_saved_selection();
    update_selection_bubble_from_dom();
}

#[cfg(target_arch = "wasm32")]
fn range_to_html_fragment(document: &Document, range: &Range) -> String {
    let Ok(frag) = range_clone_contents(range) else {
        return String::new();
    };
    let Ok(temp_div) = document.create_element("div") else {
        return String::new();
    };
    let _ = temp_div.append_child(&frag);
    temp_div.inner_html()
}

#[cfg(target_arch = "wasm32")]
fn convert_selection_to_block_type(target_type: &str) {
    let Some(document) = doc() else { return };
    let Some((rich_el, range)) = restore_saved_selection() else {
        return;
    };
    let bid = rich_el.get_attribute("data-block-id").unwrap_or_default();
    if bid.is_empty() {
        return;
    }

    // Sync latest typed HTML first
    sync_rich_block_from_element(&rich_el);

    let before_html = (|| -> Option<String> {
        let r = document.create_range().ok()?;
        r.select_node_contents(rich_el.as_ref()).ok()?;
        let sc = range.start_container().ok()?;
        let so = range.start_offset().ok()?;
        r.set_end(&sc, so).ok()?;
        Some(range_to_html_fragment(&document, &r))
    })()
    .unwrap_or_default();

    let selected_html = range_to_html_fragment(&document, &range);

    let after_html = (|| -> Option<String> {
        let r = document.create_range().ok()?;
        r.select_node_contents(rich_el.as_ref()).ok()?;
        let ec = range.end_container().ok()?;
        let eo = range.end_offset().ok()?;
        r.set_start(&ec, eo).ok()?;
        Some(range_to_html_fragment(&document, &r))
    })()
    .unwrap_or_default();

    let focus_target_id = with_state_mut(|s| {
        let Some(blocks_ref) = s.editing_article.blocks.as_ref() else {
            return None;
        };
        let pos = blocks_ref.iter().position(|b| b.id == bid)?;
        let orig = blocks_ref[pos].clone();

        let tuples = build_smart_converted_blocks(
            &orig.block_type,
            &orig.content,
            orig.meta.as_deref(),
            &before_html,
            &selected_html,
            &after_html,
            target_type,
        );

        let mut new_blocks = Vec::with_capacity(tuples.len());
        let mut target_focus_id: Option<String> = None;

        if tuples.len() == 1 {
            let (bt, cnt, mt) = tuples.into_iter().next()?;
            target_focus_id = Some(orig.id.clone());
            new_blocks.push(ContentBlock {
                id: orig.id,
                block_type: bt,
                content: cnt,
                meta: mt,
            });
        } else {
            for (bt, cnt, mt) in tuples {
                let nid = s.next_block_id();
                if bt == target_type && target_focus_id.is_none() {
                    target_focus_id = Some(nid.clone());
                }
                new_blocks.push(ContentBlock {
                    id: nid,
                    block_type: bt,
                    content: cnt,
                    meta: mt,
                });
            }
        }

        if let Some(blocks_mut) = &mut s.editing_article.blocks {
            blocks_mut.splice(pos..=pos, new_blocks);
        }
        save_editing_draft(&s.editing_article);
        target_focus_id
    });

    hide_selection_bubble();
    refresh_blocks_list_only();
    if let Some(fid) = focus_target_id {
        focus_block_by_id(&fid);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn handle_bubble_command(cmd: &str) {
    match cmd {
        "bold" | "italic" | "underline" | "strikeThrough" => {
            let Some(document) = doc() else { return };
            let Some((rich_el, _)) = restore_saved_selection() else {
                return;
            };
            if let Ok(html_doc) = document.dyn_into::<HtmlDocument>() {
                let _ = html_doc.exec_command(cmd);
            }
            sync_rich_block_from_element(&rich_el);
            update_selection_bubble_from_dom();
        }
        "highlight" => toggle_mark_highlight(),
        "open-link" => open_bubble_link_bar(),
        "apply-link" => apply_bubble_link(),
        "remove-link" => remove_bubble_link(),
        "close-link" => close_bubble_link_bar(),
        "block-paragraph" => convert_selection_to_block_type("paragraph"),
        "block-heading1" => convert_selection_to_block_type("heading1"),
        "block-heading" => convert_selection_to_block_type("heading"),
        "block-heading3" => convert_selection_to_block_type("heading3"),
        "block-quote" => convert_selection_to_block_type("quote"),
        "block-bullet-list" => convert_selection_to_block_type("bullet_list"),
        _ => {}
    }
}

#[cfg(target_arch = "wasm32")]
fn insert_plain_text_at_selection(plain: &str) {
    let mut inserted = false;
    if let Some(Ok(html_doc)) = doc().map(|d| d.dyn_into::<HtmlDocument>()) {
        inserted = html_doc
            .exec_command_with_show_ui_and_value("insertText", false, plain)
            .unwrap_or(false);
    }

    if !inserted {
        if let (Some(document), Some(Ok(Some(sel)))) =
            (doc(), web_sys::window().map(|w| w.get_selection()))
        {
            if sel.range_count() > 0 {
                if let Ok(range) = sel.get_range_at(0) {
                    let _ = range.delete_contents();
                    if let Some(text_node) = doc_create_text_node(&document, plain) {
                        let _ = range.insert_node(&text_node);
                        range.collapse_with_to_start(false);
                        let _ = sel.remove_all_ranges();
                        let _ = sel.add_range(&range);
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn extract_cursor_split_around_rich_el(rich_el: &Element) -> (String, String) {
    let Some(document) = doc() else {
        return (String::new(), String::new());
    };
    let Some(Ok(Some(sel))) = web_sys::window().map(|w| w.get_selection()) else {
        return (rich_el.inner_html(), String::new());
    };
    if sel.range_count() == 0 {
        return (rich_el.inner_html(), String::new());
    }
    let Ok(range) = sel.get_range_at(0) else {
        return (rich_el.inner_html(), String::new());
    };
    let Ok(sc) = range.start_container() else {
        return (rich_el.inner_html(), String::new());
    };
    let Ok(ec) = range.end_container() else {
        return (rich_el.inner_html(), String::new());
    };
    if !rich_el.contains(Some(&sc)) || !rich_el.contains(Some(&ec)) {
        return (rich_el.inner_html(), String::new());
    }

    let before_html = (|| -> Option<String> {
        let r = document.create_range().ok()?;
        r.select_node_contents(rich_el.as_ref()).ok()?;
        let so = range.start_offset().ok()?;
        r.set_end(&sc, so).ok()?;
        Some(range_to_html_fragment(&document, &r))
    })()
    .unwrap_or_default();

    let after_html = (|| -> Option<String> {
        let r = document.create_range().ok()?;
        r.select_node_contents(rich_el.as_ref()).ok()?;
        let eo = range.end_offset().ok()?;
        r.set_start(&ec, eo).ok()?;
        Some(range_to_html_fragment(&document, &r))
    })()
    .unwrap_or_default();

    (before_html, after_html)
}

#[cfg(target_arch = "wasm32")]
fn splice_parsed_paste_blocks(
    rich_el_opt: Option<&Element>,
    parsed_blocks: Vec<crate::editorial::wasm::paste_parser::ParsedPasteBlock>,
) {
    if parsed_blocks.is_empty() {
        return;
    }

    let pasted_total = parsed_blocks.len();
    let has_headings = parsed_blocks
        .iter()
        .any(|b| matches!(b.block_type.as_str(), "heading1" | "heading" | "heading3"));
    let has_images = parsed_blocks.iter().any(|b| b.block_type == "image");

    let split_ctx = rich_el_opt.map(|re| {
        sync_rich_block_from_element(re);
        let bid = re.get_attribute("data-block-id").unwrap_or_default();
        let (before_html, after_html) = extract_cursor_split_around_rich_el(re);
        (bid, before_html, after_html)
    });

    let (last_focus_id, images_to_localize, mid_prev) = with_state_mut(|s| {
        let mut localize_queue: Vec<(String, String)> = Vec::new();
        let mut last_id: Option<String> = None;

        let mut make_content_blocks = |s_inner: &mut crate::editorial::wasm::state::AppState| -> Vec<ContentBlock> {
            let mut out = Vec::with_capacity(parsed_blocks.len());
            for pb in &parsed_blocks {
                let nid = s_inner.next_block_id();
                if pb.block_type == "image"
                    && (pb.content.starts_with("data:image/")
                        || pb.content.starts_with("https://")
                        || pb.content.starts_with("http://"))
                {
                    localize_queue.push((nid.clone(), pb.content.clone()));
                }
                last_id = Some(nid.clone());
                out.push(ContentBlock {
                    id: nid,
                    block_type: pb.block_type.clone(),
                    content: pb.content.clone(),
                    meta: pb.meta.clone(),
                });
            }
            out
        };

        if let Some((ref target_bid, ref before_html, ref after_html)) = split_ctx {
            let orig_opt = s
                .editing_article
                .blocks
                .as_ref()
                .and_then(|blks| blks.iter().position(|b| b.id == *target_bid).map(|p| (p, blks[p].clone())));

            if let Some((pos, orig)) = orig_opt {
                let mut replacement: Vec<ContentBlock> = Vec::with_capacity(parsed_blocks.len() + 2);
                let before_clean = format_content_for_block_type(before_html, &orig.block_type);
                if !strip_all_html_tags(&before_clean).trim().is_empty() {
                    let bid_before = s.next_block_id();
                    replacement.push(ContentBlock {
                        id: bid_before,
                        block_type: orig.block_type.clone(),
                        content: before_clean,
                        meta: orig.meta.clone(),
                    });
                }

                replacement.extend(make_content_blocks(s));

                let after_clean = format_content_for_block_type(after_html, &orig.block_type);
                if !strip_all_html_tags(&after_clean).trim().is_empty() {
                    let bid_after = s.next_block_id();
                    replacement.push(ContentBlock {
                        id: bid_after,
                        block_type: orig.block_type,
                        content: after_clean,
                        meta: None,
                    });
                }

                if let Some(blocks_mut) = &mut s.editing_article.blocks {
                    blocks_mut.splice(pos..=pos, replacement);
                }
            } else {
                let generated = make_content_blocks(s);
                let blocks_mut = s.editing_article.blocks.get_or_insert_with(Vec::new);
                blocks_mut.extend(generated);
            }
        } else {
            let generated = make_content_blocks(s);
            let blocks_mut = s.editing_article.blocks.get_or_insert_with(Vec::new);
            let only_single_empty = blocks_mut.len() == 1
                && blocks_mut[0].block_type == "paragraph"
                && strip_all_html_tags(&blocks_mut[0].content).trim().is_empty();
            if only_single_empty {
                *blocks_mut = generated;
            } else {
                blocks_mut.extend(generated);
            }
        }

        save_editing_draft(&s.editing_article);
        (last_id, localize_queue, s.is_desktop && s.middle_preview_draft)
    });

    hide_selection_bubble();
    refresh_blocks_list_only();
    if let Some(fid) = last_focus_id {
        focus_block_by_id(&fid);
    }
    if mid_prev {
        crate::editorial::wasm::render::refresh_middle_panel();
    }
    if !images_to_localize.is_empty() {
        crate::editorial::wasm::api::localize_pasted_article_images(images_to_localize);
    }
    if pasted_total >= 2 {
        let mut extras = Vec::new();
        if has_headings {
            extras.push("başlık");
        }
        if has_images {
            extras.push("görsel");
        }
        extras.push("metin");
        crate::editorial::wasm::api::show_success(format!(
            "Yapıştırılan içerik {pasted_total} ayrı bloğa ({}) ayrıştırıldı.",
            extras.join(", ")
        ));
    }
}

#[cfg(target_arch = "wasm32")]
pub fn handle_rich_paste(el: &Element, ev: &ClipboardEvent) -> bool {
    let step_ok = with_state(|s| s.editor_step == 1 && s.token.is_some());
    if !step_ok {
        return false;
    }

    let rich_el_opt = el.closest("[data-rich-block='true']").ok().flatten();
    if rich_el_opt.is_none() {
        // Allow native paste in normal <input> / <textarea> fields (Title, Spot, URLs, Captions, Search)
        if el
            .closest("input, textarea, select, #hf-selection-bubble")
            .ok()
            .flatten()
            .is_some()
        {
            return false;
        }
        // Only intercept panel-level paste if user is inside `#right-panel`
        if el.closest("#right-panel").ok().flatten().is_none() {
            return false;
        }
    }

    let Some(cd) = ev.clipboard_data() else {
        return false;
    };

    let html = cd.get_data("text/html").unwrap_or_default();
    let plain = cd.get_data("text/plain").unwrap_or_default();

    // 1. Direct clipboard image file paste (`Ctrl+V` screenshot or "Copy Image")
    if plain.trim().is_empty() {
        if let Some(flist) = cd.files() {
            let mut img_files: Vec<web_sys::File> = Vec::new();
            for i in 0..flist.length() {
                if let Some(f) = flist.get(i) {
                    if f.type_().starts_with("image/") {
                        img_files.push(f);
                    }
                }
            }
            if !img_files.is_empty() {
                ev.prevent_default();
                let target_bid = rich_el_opt
                    .as_ref()
                    .and_then(|re| re.get_attribute("data-block-id"));
                let (pairs, mid_prev) = with_state_mut(|s| {
                    let mut created_pairs: Vec<(String, web_sys::File)> =
                        Vec::with_capacity(img_files.len());
                    let mut new_img_blocks = Vec::with_capacity(img_files.len());
                    for f in img_files {
                        let nid = s.next_block_id();
                        created_pairs.push((nid.clone(), f));
                        new_img_blocks.push(ContentBlock {
                            id: nid,
                            block_type: "image".to_string(),
                            content: String::new(),
                            meta: None,
                        });
                    }

                    let blocks_mut = s.editing_article.blocks.get_or_insert_with(Vec::new);
                    let pos_opt = target_bid
                        .as_deref()
                        .and_then(|id| blocks_mut.iter().position(|b| b.id == id));

                    let replace_empty = pos_opt.map_or(false, |p| {
                        blocks_mut[p].block_type == "paragraph"
                            && strip_all_html_tags(&blocks_mut[p].content).trim().is_empty()
                    });

                    if let Some(p) = pos_opt {
                        if replace_empty {
                            blocks_mut.splice(p..=p, new_img_blocks);
                        } else {
                            blocks_mut.splice((p + 1)..(p + 1), new_img_blocks);
                        }
                    } else {
                        blocks_mut.extend(new_img_blocks);
                    }
                    save_editing_draft(&s.editing_article);
                    (created_pairs, s.is_desktop && s.middle_preview_draft)
                });
                hide_selection_bubble();
                refresh_blocks_list_only();
                if mid_prev {
                    crate::editorial::wasm::render::refresh_middle_panel();
                }
                crate::editorial::wasm::api::upload_pasted_clipboard_images(pairs);
                return true;
            }
        }
    }

    // 2. Parse HTML + Plain Text clipboard payload into structured blocks or inline fragment
    match crate::editorial::wasm::paste_parser::parse_clipboard_payload(&html, &plain) {
        crate::editorial::wasm::paste_parser::PasteParseOutcome::Empty => {
            if rich_el_opt.is_some() {
                ev.prevent_default();
                true
            } else {
                false
            }
        }
        crate::editorial::wasm::paste_parser::PasteParseOutcome::InlineFragment {
            html: inline_html,
            plain: inline_plain,
        } => {
            if let Some(rich_el) = rich_el_opt {
                ev.prevent_default();
                let bkind = rich_el.get_attribute("data-block-kind").unwrap_or_default();
                let is_heading = matches!(bkind.as_str(), "heading1" | "heading" | "heading3");
                if !is_heading && inline_html.contains('<') {
                    let mut inserted_rich = false;
                    if let Some(Ok(html_doc)) = doc().map(|d| d.dyn_into::<HtmlDocument>()) {
                        inserted_rich = html_doc
                            .exec_command_with_show_ui_and_value("insertHTML", false, &inline_html)
                            .unwrap_or(false);
                    }
                    if !inserted_rich {
                        insert_plain_text_at_selection(&inline_plain);
                    }
                } else {
                    insert_plain_text_at_selection(&inline_plain);
                }
                sync_rich_block_from_element(&rich_el);
                if with_state(|s| s.is_desktop && s.middle_preview_draft) {
                    crate::editorial::wasm::render::refresh_middle_panel();
                }
                true
            } else {
                ev.prevent_default();
                splice_parsed_paste_blocks(
                    None,
                    vec![crate::editorial::wasm::paste_parser::ParsedPasteBlock {
                        block_type: "paragraph".to_string(),
                        content: inline_html,
                        meta: None,
                    }],
                );
                true
            }
        }
        crate::editorial::wasm::paste_parser::PasteParseOutcome::Blocks(parsed_blocks) => {
            ev.prevent_default();
            splice_parsed_paste_blocks(rich_el_opt.as_ref(), parsed_blocks);
            true
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn handle_rich_keydown(el: &Element, ev: &KeyboardEvent) -> bool {
    // 1. Inside Bubble Link Input: Enter applies link, Escape closes link bar
    if el.id() == "hf-bubble-link-input" {
        if ev.key() == "Enter" {
            ev.prevent_default();
            apply_bubble_link();
            return true;
        }
        if ev.key() == "Escape" {
            ev.prevent_default();
            close_bubble_link_bar();
            return true;
        }
        return false;
    }

    // 2. Escape anywhere hides open selection bubble
    if ev.key() == "Escape" {
        hide_selection_bubble();
        return false;
    }

    let Ok(Some(rich_el)) = el.closest("[data-rich-block='true']") else {
        return false;
    };

    // 3. Ctrl+K / Cmd+K opens Link Bar when text is selected
    if (ev.ctrl_key() || ev.meta_key()) && ev.key().eq_ignore_ascii_case("k") {
        update_selection_bubble_from_dom();
        let has_sel = BUBBLE_STATE.with(|st| st.borrow().saved_range.is_some());
        if has_sel {
            ev.prevent_default();
            open_bubble_link_bar();
            return true;
        }
    }

    // 4. Pressing Enter (without Shift) inside Paragraph or H1/H2/H3 splits/creates a clean Paragraph block right below
    let bkind = rich_el.get_attribute("data-block-kind").unwrap_or_default();
    if ev.key() == "Enter"
        && !ev.shift_key()
        && matches!(
            bkind.as_str(),
            "paragraph" | "heading1" | "heading" | "heading3"
        )
    {
        ev.prevent_default();
        sync_rich_block_from_element(&rich_el);
        let cur_id = rich_el.get_attribute("data-block-id").unwrap_or_default();
        let (before_html, after_html) = if bkind == "paragraph" {
            extract_cursor_split_around_rich_el(&rich_el)
        } else {
            (rich_el.inner_html(), String::new())
        };
        let (new_id, mid_prev) = with_state_mut(|s| {
            let nid = s.next_block_id();
            let blocks = s.editing_article.blocks.get_or_insert_with(Vec::new);
            let idx = blocks
                .iter()
                .position(|b| b.id == cur_id)
                .unwrap_or(blocks.len());
            if bkind == "paragraph" && idx < blocks.len() {
                blocks[idx].content = format_content_for_block_type(&before_html, "paragraph");
            }
            let next_content = if bkind == "paragraph" {
                format_content_for_block_type(&after_html, "paragraph")
            } else {
                String::new()
            };
            let insert_pos = (idx + 1).min(blocks.len());
            blocks.insert(
                insert_pos,
                ContentBlock {
                    id: nid.clone(),
                    block_type: "paragraph".to_string(),
                    content: next_content,
                    meta: None,
                },
            );
            save_editing_draft(&s.editing_article);
            (nid, s.is_desktop && s.middle_preview_draft)
        });
        hide_selection_bubble();
        refresh_blocks_list_only();
        focus_block_by_id(&new_id);
        if mid_prev {
            crate::editorial::wasm::render::refresh_middle_panel();
        }
        return true;
    }

    false
}

#[cfg(target_arch = "wasm32")]
pub fn focus_block_by_id(block_id: &str) {
    let Some(document) = doc() else { return };
    let selector = format!(
        "[data-rich-block='true'][data-block-id='{block_id}'], textarea[data-block-id='{block_id}'], input[data-input='block-content'][data-block-id='{block_id}']"
    );
    if let Some(Ok(he)) = document
        .query_selector(&selector)
        .ok()
        .flatten()
        .map(|e| e.dyn_into::<HtmlElement>())
    {
        let _ = he.focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_link_urls_and_blocks_javascript_xss() {
        assert_eq!(
            normalize_editor_link_url("ntv.com.tr/ekonomi"),
            Some("https://ntv.com.tr/ekonomi".to_string())
        );
        assert_eq!(
            normalize_editor_link_url("https://haber.example.com/a"),
            Some("https://haber.example.com/a".to_string())
        );
        assert_eq!(normalize_editor_link_url("javascript:alert(1)"), None);
        assert_eq!(normalize_editor_link_url("data:text/html,evil"), None);
        assert_eq!(normalize_editor_link_url("   "), None);
    }

    #[test]
    fn smart_splits_subphrase_or_converts_whole_block_at_70_percent() {
        // 1. Sub-phrase (< 70%) splits cleanly into [paragraph, heading1, paragraph]
        let full = "Türkiye ekonomisinde yeni dönem başladı. Merkez Bankası faiz kararını açıkladı ve piyasalar hareketlendi.";
        let before = "Türkiye ekonomisinde yeni dönem başladı. ";
        let selected = "Merkez Bankası faiz kararını açıkladı";
        let after = " ve piyasalar hareketlendi.";
        let split = build_smart_converted_blocks(
            "paragraph",
            full,
            None,
            before,
            selected,
            after,
            "heading1",
        );
        assert_eq!(split.len(), 3);
        assert_eq!(split[0].0, "paragraph");
        assert_eq!(split[1].0, "heading1");
        assert_eq!(split[1].1, "Merkez Bankası faiz kararını açıkladı");
        assert_eq!(split[2].0, "paragraph");

        // 2. >= 70% selection converts the whole block instead of leaving tiny fragments
        let almost_all = "Merkez Bankası faiz kararını açıkladı ve piyasalar hızla tepki verdi";
        let whole = build_smart_converted_blocks(
            "paragraph",
            almost_all,
            None,
            "",
            "Merkez Bankası faiz kararını açıkladı ve piyasalar hızla",
            " tepki verdi",
            "heading",
        );
        assert_eq!(whole.len(), 1);
        assert_eq!(whole[0].0, "heading");
    }
}
