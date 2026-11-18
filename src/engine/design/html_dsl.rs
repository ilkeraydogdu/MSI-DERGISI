//! Compact, composable, auto-escaping HTML5 SSR builder.
//!
//! Uses `security_engine::escape_html` for all text/attribute writes and
//! `config::json_for_script` for inline JSON `<script>` payloads.

use super::config::json_for_script;
use crate::engine::security_engine::escape_html;
use serde::Serialize;
use std::fmt;

/// Buffered HTML5 string builder with explicit escaping primitives.
///
/// Note: `fmt::Write::write_str` appends raw HTML (matching standard Rust
/// buffer semantics). Use `.text()` for untrusted text and `.attr()` for
/// untrusted attribute values.
#[derive(Clone, Debug, Default)]
pub struct HtmlBuf(pub String);

impl HtmlBuf {
    pub fn new() -> Self {
        Self(String::new())
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self(String::with_capacity(cap))
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Appends HTML-escaped text content.
    pub fn text(&mut self, s: &str) -> &mut Self {
        self.0.push_str(&escape_html(s));
        self
    }

    /// Appends raw trusted HTML markup verbatim.
    pub fn raw(&mut self, html: &str) -> &mut Self {
        self.0.push_str(html);
        self
    }

    /// Appends an HTML-escaped attribute ` name="value"`.
    pub fn attr(&mut self, name: &str, value: &str) -> &mut Self {
        let safe_name = sanitize_attr_name(name);
        if !safe_name.is_empty() {
            self.0.push(' ');
            self.0.push_str(&safe_name);
            self.0.push_str("=\"");
            self.0.push_str(&escape_html(value));
            self.0.push('"');
        }
        self
    }

    /// Conditionally appends a boolean HTML5 attribute (` name`).
    pub fn bool_attr(&mut self, name: &str, enabled: bool) -> &mut Self {
        if enabled {
            let safe_name = sanitize_attr_name(name);
            if !safe_name.is_empty() {
                self.0.push(' ');
                self.0.push_str(&safe_name);
            }
        }
        self
    }

    /// Emits a paired HTML element `<tag ...>...</tag>` with static key/value attributes.
    pub fn el<F>(&mut self, tag: &str, attrs: &[(&str, &str)], f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        self.el_with(
            tag,
            |buf| {
                for (k, v) in attrs {
                    buf.attr(k, v);
                }
            },
            f,
        )
    }

    /// Emits a paired HTML element `<tag ...>...</tag>` with custom attribute builder closure.
    pub fn el_with<A, F>(&mut self, tag: &str, attrs_fn: A, f: F) -> &mut Self
    where
        A: FnOnce(&mut Self),
        F: FnOnce(&mut Self),
    {
        let safe_tag = sanitize_tag_name(tag);
        if safe_tag.is_empty() {
            return self;
        }
        self.0.push('<');
        self.0.push_str(&safe_tag);
        attrs_fn(self);
        self.0.push('>');
        f(self);
        self.0.push_str("</");
        self.0.push_str(&safe_tag);
        self.0.push('>');
        self
    }

    /// Emits a void HTML5 element `<tag ...>` (e.g., `meta`, `link`, `input`, `img`).
    pub fn void_el(&mut self, tag: &str, attrs: &[(&str, &str)]) -> &mut Self {
        self.void_with(tag, |buf| {
            for (k, v) in attrs {
                buf.attr(k, v);
            }
        })
    }

    /// Emits a void HTML5 element with custom attribute builder closure.
    pub fn void_with<A>(&mut self, tag: &str, attrs_fn: A) -> &mut Self
    where
        A: FnOnce(&mut Self),
    {
        let safe_tag = sanitize_tag_name(tag);
        if safe_tag.is_empty() {
            return self;
        }
        self.0.push('<');
        self.0.push_str(&safe_tag);
        attrs_fn(self);
        self.0.push('>');
        self
    }

    /// Emits a `<script id="..." type="application/json"[ nonce="..."]>...</script>` tag
    /// using `config::json_for_script` plus line-separator escaping for complete script safety.
    pub fn json_script<T: Serialize>(
        &mut self,
        id: &str,
        value: &T,
        nonce: Option<&str>,
    ) -> &mut Self {
        let safe_json = json_for_script(value)
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029");
        self.el_with(
            "script",
            |b| {
                b.attr("id", id).attr("type", "application/json");
                if let Some(n) = nonce.filter(|s| !s.is_empty()) {
                    b.attr("nonce", n);
                }
            },
            |b| {
                b.raw(&safe_json);
            },
        )
    }

    /// Emits `<link rel="stylesheet" href="...">`.
    pub fn link_css(&mut self, href: &str) -> &mut Self {
        self.void_el("link", &[("rel", "stylesheet"), ("href", href)])
    }

    /// Emits `<script src="..."[ nonce="..."][ defer]></script>`.
    pub fn script_src(&mut self, src: &str, nonce: Option<&str>, defer: bool) -> &mut Self {
        self.el_with(
            "script",
            |b| {
                b.attr("src", src);
                if let Some(n) = nonce.filter(|s| !s.is_empty()) {
                    b.attr("nonce", n);
                }
                b.bool_attr("defer", defer);
            },
            |_| {},
        )
    }

    /// Emits `<script[ type="..."][ nonce="..."]>...</script>` while neutralizing closing `</script` sequences.
    pub fn script_inline(
        &mut self,
        js: &str,
        nonce: Option<&str>,
        script_type: Option<&str>,
    ) -> &mut Self {
        let neutralized = neutralize_closing_tag(js, "script");
        self.el_with(
            "script",
            |b| {
                if let Some(ty) = script_type.filter(|s| !s.is_empty()) {
                    b.attr("type", ty);
                }
                if let Some(n) = nonce.filter(|s| !s.is_empty()) {
                    b.attr("nonce", n);
                }
            },
            |b| {
                b.raw(&neutralized);
            },
        )
    }

    /// Emits `<style[ id="..."]>...</style>` while neutralizing closing `</style` sequences.
    pub fn style_tag(&mut self, id: Option<&str>, css: &str) -> &mut Self {
        let neutralized = neutralize_closing_tag(css, "style");
        self.el_with(
            "style",
            |b| {
                if let Some(style_id) = id.filter(|s| !s.is_empty()) {
                    b.attr("id", style_id);
                }
            },
            |b| {
                b.raw(&neutralized);
            },
        )
    }
}

impl fmt::Write for HtmlBuf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(s);
        Ok(())
    }
}

fn sanitize_tag_name(tag: &str) -> String {
    tag.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

fn sanitize_attr_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
        .collect()
}

/// Escapes ASCII case-insensitive `</tag` occurrences to `<\/tag` inside raw `<script>`/`<style>`.
fn neutralize_closing_tag(body: &str, tag: &str) -> String {
    let needle = format!("</{}", tag.to_ascii_lowercase());
    let lower = body.to_ascii_lowercase();
    let mut out = String::with_capacity(body.len());
    let mut cursor = 0;
    while let Some(rel_pos) = lower[cursor..].find(&needle) {
        let abs_pos = cursor + rel_pos;
        out.push_str(&body[cursor..abs_pos]);
        out.push_str("<\\/");
        cursor = abs_pos + 2;
    }
    out.push_str(&body[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fmt::Write;

    #[test]
    fn escapes_xss_in_text_and_attributes() {
        let mut buf = HtmlBuf::new();
        buf.el(
            "a",
            &[
                ("href", "/search?q=\" onclick=\"alert(1)"),
                ("title", "<script>alert('xss')</script> & more"),
            ],
            |b| {
                b.text("<img src=x onerror=alert(1)> & 'quote'");
            },
        );
        let html = buf.into_string();
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img "));
        assert!(!html.contains("\" onclick=\""));
        assert!(html.contains("&quot; onclick=&quot;alert(1)"));
        assert!(html.contains("&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt; &amp; more"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt; &amp; &#x27;quote&#x27;"));
    }

    #[test]
    fn json_script_escapes_script_breakouts_and_sets_nonce() {
        let mut buf = HtmlBuf::new();
        let payload = json!({
            "xss": "</script><script>alert(1)</script>",
            "upper": "</SCRIPT>",
            "amp": "A & B",
            "sep": "line\u{2028}break\u{2029}here"
        });
        buf.json_script("msi-i18n", &payload, Some("nonce-abc\"123"));
        let html = buf.into_string();
        assert!(html.starts_with("<script id=\"msi-i18n\" type=\"application/json\" nonce=\"nonce-abc&quot;123\">"));
        assert_eq!(html.matches("</script>").count(), 1);
        assert!(!html.to_ascii_lowercase().contains("</script><script>"));
        assert!(html.contains("\\u003c/script\\u003e"));
        assert!(html.contains("\\u003c/SCRIPT\\u003e"));
        assert!(html.contains("A \\u0026 B"));
        assert!(html.contains("\\u2028"));
        assert!(html.contains("\\u2029"));
    }

    #[test]
    fn script_inline_and_style_tag_neutralize_closing_tags() {
        let mut buf = HtmlBuf::new();
        buf.script_inline("const s = '</ScRiPt><script>alert(1)</script>';", Some("n1"), None);
        buf.style_tag(Some("tokens"), "body::after { content: '</STYLE><script>'; }");
        let html = buf.into_string();
        assert_eq!(html.matches("</script>").count(), 1);
        assert_eq!(html.matches("</style>").count(), 1);
        assert!(html.contains("<\\/ScRiPt>"));
        assert!(html.contains("<\\/STYLE>"));
    }

    #[test]
    fn builds_void_links_and_write_impl() {
        let mut buf = HtmlBuf::new();
        buf.link_css("/css/msi-design-system.css?v=14")
            .script_src("/js/msi-ui.js?v=12", Some("n2"), true)
            .void_with("input", |b| {
                b.attr("type", "checkbox")
                    .attr("name", "analytics")
                    .bool_attr("checked", true)
                    .bool_attr("disabled", false);
            });
        write!(buf, "<span>ok</span>").unwrap();
        let html = buf.into_string();
        assert!(html.contains("<link rel=\"stylesheet\" href=\"/css/msi-design-system.css?v=14\">"));
        assert!(html.contains("<script src=\"/js/msi-ui.js?v=12\" nonce=\"n2\" defer></script>"));
        assert!(html.contains("<input type=\"checkbox\" name=\"analytics\" checked>"));
        assert!(!html.contains("disabled"));
        assert!(html.ends_with("<span>ok</span>"));
    }
}
