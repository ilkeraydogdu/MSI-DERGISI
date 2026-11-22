//! Smart Article Clipboard Parser (`text/html` + `text/plain`) for the Step 1 Block Editor.
//!
//! Converts multi-paragraph pasted articles from news websites, Google Docs, Microsoft Word,
//! Markdown, or plain text into discrete `ContentBlock` items:
//! - Every paragraph / line break (`<p>`, `<div>`, `<br>`, `\n`) becomes its own separate block.
//! - Headings (`<h1>`..`<h6>`, Markdown `#`/`##`/`###`, Word `MsoHeading*`, standalone bold subheadings,
//!   and unmarked short editorial subheadings) become `heading1` / `heading` (`H2`) / `heading3` (`H3`).
//! - Images (`<img>`, `<figure>`, lazy `data-src`/`srcset`, `data:image/*`, Markdown `![alt](url)`,
//!   and standalone image URLs) become `image` blocks with captions preserved in `meta`.
//! - Lists (`<ul>`, `<ol>`, `- `, `1. `), quotes (`<blockquote>`, `> `), embeds (`<iframe>`, YouTube/X/IG URLs),
//!   code (`<pre>`, fenced ``` ``` ```), and dividers (`<hr>`, `---`) map to their dedicated block types.

use crate::editorial::common::{sanitize_inline_rich_html, strip_all_html_tags};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPasteBlock {
    pub block_type: String,
    pub content: String,
    pub meta: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteParseOutcome {
    Empty,
    InlineFragment { html: String, plain: String },
    Blocks(Vec<ParsedPasteBlock>),
}

/// Entry point: parses clipboard `text/html` and `text/plain` into either a single inline fragment
/// (for pasting a few words inside an existing sentence) or a full sequence of structured blocks.
pub fn parse_clipboard_payload(html: &str, plain: &str) -> PasteParseOutcome {
    let norm_plain = plain.replace("\r\n", "\n").replace('\r', "\n");
    let has_html = !html.trim().is_empty() && html.contains('<') && html.contains('>');

    if has_html {
        let html_blocks = parse_html_to_blocks(html);
        if !html_blocks.is_empty() {
            // Guard against apps (e.g. code editors / chat apps) that wrap multi-line plain text
            // inside a single `<span>` or `<div>` without `<br>` or `pre-wrap`.
            if html_blocks.len() == 1 && html_blocks[0].block_type == "paragraph" && !norm_plain.trim().is_empty() {
                let plain_blocks = parse_plain_text_to_blocks(&norm_plain);
                if plain_blocks.len() >= 2 {
                    return PasteParseOutcome::Blocks(plain_blocks);
                }
            }
            if is_single_inline_html_paste(&html_blocks, html, &norm_plain) {
                let inline_html = html_blocks[0].content.clone();
                let inline_plain = strip_all_html_tags(&inline_html);
                return PasteParseOutcome::InlineFragment {
                    html: inline_html,
                    plain: inline_plain,
                };
            }
            return PasteParseOutcome::Blocks(html_blocks);
        }
    }

    if norm_plain.trim().is_empty() {
        return PasteParseOutcome::Empty;
    }

    let plain_blocks = parse_plain_text_to_blocks(&norm_plain);
    if plain_blocks.is_empty() {
        return PasteParseOutcome::Empty;
    }

    let trimmed_plain = norm_plain.trim_matches('\n');
    if plain_blocks.len() == 1
        && plain_blocks[0].block_type == "paragraph"
        && !trimmed_plain.contains('\n')
    {
        let inline_html = plain_blocks[0].content.clone();
        let inline_plain = strip_all_html_tags(&inline_html);
        return PasteParseOutcome::InlineFragment {
            html: inline_html,
            plain: inline_plain,
        };
    }

    PasteParseOutcome::Blocks(plain_blocks)
}

fn is_single_inline_html_paste(
    blocks: &[ParsedPasteBlock],
    raw_html: &str,
    norm_plain: &str,
) -> bool {
    if blocks.len() != 1 || blocks[0].block_type != "paragraph" {
        return false;
    }
    if norm_plain.trim_matches('\n').contains('\n') {
        return false;
    }
    let lower = raw_html.to_ascii_lowercase();
    let has_structural_tag = lower.contains("<h1")
        || lower.contains("<h2")
        || lower.contains("<h3")
        || lower.contains("<h4")
        || lower.contains("<img")
        || lower.contains("<figure")
        || lower.contains("<ul")
        || lower.contains("<ol")
        || lower.contains("<blockquote")
        || lower.contains("<pre")
        || lower.contains("<hr")
        || lower.contains("<iframe");
    !has_structural_tag
}

// ============================================================================
// HTML Entity Decoding & UTF-8 Safe Tokenizer
// ============================================================================

pub fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        if ch == '&' {
            let rest = &input[idx + 1..];
            if let Some(semi_rel) = rest.find(';') {
                if semi_rel <= 14 {
                    let entity = &rest[..semi_rel];
                    if let Some(decoded) = resolve_html_entity(entity) {
                        out.push_str(&decoded);
                        let skip_until = idx + 1 + semi_rel + 1;
                        while let Some(&(ni, _)) = chars.peek() {
                            if ni < skip_until {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        continue;
                    }
                }
            }
        }
        out.push(ch);
    }
    out
}

fn resolve_html_entity(ent: &str) -> Option<String> {
    if let Some(hex) = ent.strip_prefix("#x").or_else(|| ent.strip_prefix("#X")) {
        let code = u32::from_str_radix(hex, 16).ok()?;
        let ch = char::from_u32(code)?;
        return Some(if ch == '\u{00A0}' {
            " ".to_string()
        } else {
            ch.to_string()
        });
    }
    if let Some(dec) = ent.strip_prefix('#') {
        let code = dec.parse::<u32>().ok()?;
        let ch = char::from_u32(code)?;
        return Some(if ch == '\u{00A0}' {
            " ".to_string()
        } else {
            ch.to_string()
        });
    }
    let mapped = match ent {
        "nbsp" | "NonBreakingSpace" => " ",
        "amp" | "AMP" => "&",
        "lt" | "LT" => "<",
        "gt" | "GT" => ">",
        "quot" | "QUOT" => "\"",
        "apos" | "#39" => "'",
        "ndash" => "–",
        "mdash" => "—",
        "hellip" => "…",
        "lsquo" | "rsquo" => "’",
        "ldquo" | "rdquo" => "”",
        "laquo" => "«",
        "raquo" => "»",
        "bull" | "middot" => "•",
        "copy" => "©",
        "reg" => "®",
        "trade" => "™",
        "euro" => "€",
        "pound" => "£",
        "yen" => "¥",
        _ => return None,
    };
    Some(mapped.to_string())
}

fn escape_text_for_inline_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

#[derive(Debug)]
enum HtmlToken {
    StartTag {
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Text(String),
}

fn tokenize_html(html: &str) -> Vec<HtmlToken> {
    let mut tokens = Vec::new();
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;

    while i < len {
        if bytes[i] == b'<' {
            let rest = &html[i..];
            // Comment `<!-- ... -->`
            if rest.starts_with("<!--") {
                if let Some(end_c) = rest[4..].find("-->") {
                    i += 4 + end_c + 3;
                    continue;
                } else {
                    break;
                }
            }
            // Doctype / XML declaration `<!...>` or `<?...>`
            if rest.starts_with("<!") || rest.starts_with("<?") {
                if let Some(gt) = rest.find('>') {
                    i += gt + 1;
                    continue;
                } else {
                    break;
                }
            }
            if let Some((tok, consumed)) = parse_tag_at(rest) {
                tokens.push(tok);
                i += consumed;
                continue;
            }
        }

        // Collect text until next '<'
        let next_lt = html[i..].find('<').map(|off| i + off).unwrap_or(len);
        if next_lt > i {
            let raw_txt = &html[i..next_lt];
            tokens.push(HtmlToken::Text(decode_html_entities(raw_txt)));
            i = next_lt;
        } else {
            // Unmatched '<' literal
            let ch = html[i..].chars().next().unwrap_or('<');
            tokens.push(HtmlToken::Text(ch.to_string()));
            i += ch.len_utf8();
        }
    }

    tokens
}

fn parse_tag_at(s: &str) -> Option<(HtmlToken, usize)> {
    if !s.starts_with('<') {
        return None;
    }
    // Find closing '>' respecting quotes
    let mut in_quote: Option<char> = None;
    let mut gt_byte_idx: Option<usize> = None;
    for (idx, ch) in s.char_indices().skip(1) {
        if let Some(q) = in_quote {
            if ch == q {
                in_quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
        } else if ch == '>' {
            gt_byte_idx = Some(idx);
            break;
        }
    }
    let gt_idx = gt_byte_idx?;
    let inside = s[1..gt_idx].trim();
    if inside.is_empty() {
        return None;
    }

    // End tag `</tag>`
    if let Some(after_slash) = inside.strip_prefix('/') {
        let tname: String = after_slash
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == ':' || *c == '-')
            .collect();
        if tname.is_empty() {
            return None;
        }
        return Some((
            HtmlToken::EndTag {
                name: tname.to_ascii_lowercase(),
            },
            gt_idx + 1,
        ));
    }

    // Start tag `<tag attr="val" ...>`
    let self_closing = inside.ends_with('/');
    let clean_inside = if self_closing {
        inside[..inside.len() - 1].trim_end()
    } else {
        inside
    };

    let mut name_end = 0usize;
    for (idx, ch) in clean_inside.char_indices() {
        if ch.is_ascii_alphanumeric() || ch == ':' || ch == '-' || ch == '_' {
            name_end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if name_end == 0 {
        return None;
    }

    let name = clean_inside[..name_end].to_ascii_lowercase();
    let attr_str = clean_inside[name_end..].trim();
    let attrs = parse_tag_attributes(attr_str);

    Some((
        HtmlToken::StartTag {
            name,
            attrs,
            self_closing,
        },
        gt_idx + 1,
    ))
}

fn parse_tag_attributes(attr_str: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let mut rest = attr_str.trim();

    while !rest.is_empty() {
        // Attribute name
        let mut k_end = 0usize;
        for (idx, ch) in rest.char_indices() {
            if ch.is_whitespace() || ch == '=' {
                break;
            }
            k_end = idx + ch.len_utf8();
        }
        if k_end == 0 {
            let step = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            rest = rest[step..].trim_start();
            continue;
        }
        let key = rest[..k_end].to_ascii_lowercase();
        rest = rest[k_end..].trim_start();

        if let Some(after_eq) = rest.strip_prefix('=') {
            rest = after_eq.trim_start();
            if let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') {
                let after_q = &rest[q.len_utf8()..];
                if let Some(end_q) = after_q.find(q) {
                    let val = decode_html_entities(&after_q[..end_q]);
                    attrs.push((key, val));
                    rest = after_q[end_q + q.len_utf8()..].trim_start();
                } else {
                    let val = decode_html_entities(after_q);
                    attrs.push((key, val));
                    break;
                }
            } else {
                let end_v = rest
                    .find(char::is_whitespace)
                    .unwrap_or(rest.len());
                let val = decode_html_entities(&rest[..end_v]);
                attrs.push((key, val));
                rest = rest[end_v..].trim_start();
            }
        } else {
            attrs.push((key, String::new()));
        }
    }
    attrs
}

fn get_attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

// ============================================================================
// Rich HTML -> Structured Editor Blocks Engine
// ============================================================================

#[derive(Debug, Clone)]
enum InlineMark {
    Strong,
    Em,
    Underline,
    Strike,
    Mark,
    Code,
    Link(String),
    LargeHeading(u8), // 1 or 2 from inline font-size style
    Noop,             // e.g. Google Docs `<b style="font-weight:normal">` or plain `<span>`
}

impl InlineMark {
    fn open_html(&self) -> String {
        match self {
            Self::Strong => "<strong>".to_string(),
            Self::Em => "<em>".to_string(),
            Self::Underline => "<u>".to_string(),
            Self::Strike => "<s>".to_string(),
            Self::Mark => "<mark>".to_string(),
            Self::Code => "<code>".to_string(),
            Self::Link(href) => format!(
                "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
                escape_text_for_inline_html(href)
            ),
            Self::LargeHeading(_) | Self::Noop => String::new(),
        }
    }

    fn close_html(&self) -> &'static str {
        match self {
            Self::Strong => "</strong>",
            Self::Em => "</em>",
            Self::Underline => "</u>",
            Self::Strike => "</s>",
            Self::Mark => "</mark>",
            Self::Code => "</code>",
            Self::Link(_) => "</a>",
            Self::LargeHeading(_) | Self::Noop => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Numbered,
}

pub fn parse_html_to_blocks(html: &str) -> Vec<ParsedPasteBlock> {
    let tokens = tokenize_html(html);
    let mut blocks: Vec<ParsedPasteBlock> = Vec::new();

    let mut skip_depth: usize = 0;
    let mut in_pre = false;
    let mut pre_buf = String::new();

    let mut in_blockquote = false;
    let mut in_cite = false;
    let mut quote_cite_buf = String::new();
    let mut last_quote_block_idx: Option<usize> = None;
    let mut blockquote_embed_override: Option<String> = None;

    let mut in_figure = false;
    let mut in_figcaption = false;
    let mut figcaption_buf = String::new();
    let mut last_figure_block_idx: Option<usize> = None;

    let mut active_heading: Option<&'static str> = None; // "heading1" | "heading" | "heading3"
    let mut word_list_para = false;

    let mut list_stack: Vec<(ListKind, Vec<String>)> = Vec::new();
    let mut in_li = false;
    let mut pre_wrap_tags: Vec<String> = Vec::new();

    let mut inline_stack: Vec<(String, Vec<InlineMark>)> = Vec::new();
    let mut buf = String::new();
    let mut buf_saw_large_h1 = false;
    let mut buf_saw_large_h2 = false;

    let close_all_open_marks = |target: &mut String, stack: &[(String, Vec<InlineMark>)]| {
        for (_, marks) in stack.iter().rev() {
            for m in marks.iter().rev() {
                target.push_str(m.close_html());
            }
        }
    };

    let reopen_all_marks = |target: &mut String, stack: &[(String, Vec<InlineMark>)]| {
        for (_, marks) in stack.iter() {
            for m in marks.iter() {
                target.push_str(&m.open_html());
            }
        }
    };

    let flush_buf = |buf: &mut String,
                     blocks: &mut Vec<ParsedPasteBlock>,
                     inline_stack: &[(String, Vec<InlineMark>)],
                     active_heading: Option<&'static str>,
                     in_blockquote: bool,
                     quote_cite_buf: &mut String,
                     last_quote_idx: &mut Option<usize>,
                     list_stack: &mut Vec<(ListKind, Vec<String>)>,
                     in_li: bool,
                     word_list_para: &mut bool,
                     saw_h1: &mut bool,
                     saw_h2: &mut bool| {
        if buf.trim().is_empty() {
            buf.clear();
            *saw_h1 = false;
            *saw_h2 = false;
            return;
        }
        close_all_open_marks(buf, inline_stack);
        let raw_content = std::mem::take(buf);
        let plain = collapse_inline_spaces(&strip_all_html_tags(&raw_content));
        let h1_hint = *saw_h1;
        let h2_hint = *saw_h2;
        *saw_h1 = false;
        *saw_h2 = false;

        if plain.is_empty() {
            return;
        }

        if in_li {
            if let Some((_, items)) = list_stack.last_mut() {
                let clean_item = plain
                    .trim_start_matches(['•', '-', '*'])
                    .trim()
                    .to_string();
                if !clean_item.is_empty() {
                    items.push(clean_item);
                }
            }
            return;
        }

        if *word_list_para {
            *word_list_para = false;
            let clean_item = plain
                .trim_start_matches(['•', '-', '*', '·', 'o'])
                .trim()
                .to_string();
            if !clean_item.is_empty() {
                if let Some(last) = blocks.last_mut() {
                    if last.block_type == "bullet_list" {
                        last.content.push('\n');
                        last.content.push_str(&clean_item);
                        return;
                    }
                }
                blocks.push(ParsedPasteBlock {
                    block_type: "bullet_list".to_string(),
                    content: clean_item,
                    meta: None,
                });
            }
            return;
        }

        if let Some(htype) = active_heading {
            blocks.push(ParsedPasteBlock {
                block_type: htype.to_string(),
                content: plain,
                meta: None,
            });
            return;
        }

        if in_blockquote {
            let sanitized = sanitize_inline_rich_html(raw_content.trim());
            let cite_opt = {
                let c = collapse_inline_spaces(quote_cite_buf);
                quote_cite_buf.clear();
                (!c.is_empty()).then_some(c)
            };
            if let Some(qidx) = *last_quote_idx {
                if let Some(existing_q) = blocks.get_mut(qidx) {
                    if !existing_q.content.is_empty() {
                        existing_q.content.push_str("<br/>");
                    }
                    existing_q.content.push_str(&sanitized);
                    if cite_opt.is_some() {
                        existing_q.meta = cite_opt;
                    }
                    return;
                }
            }
            blocks.push(ParsedPasteBlock {
                block_type: "quote".to_string(),
                content: sanitized,
                meta: cite_opt,
            });
            *last_quote_idx = Some(blocks.len() - 1);
            return;
        }

        // Standalone URL checks (image URL or social/video embed URL inside a paragraph)
        if let Some(img_url) = detect_standalone_image_url(&plain) {
            blocks.push(ParsedPasteBlock {
                block_type: "image".to_string(),
                content: img_url,
                meta: None,
            });
            return;
        }
        if let Some(embed_url) = detect_standalone_embed_url(&plain) {
            blocks.push(ParsedPasteBlock {
                block_type: "embed".to_string(),
                content: embed_url,
                meta: None,
            });
            return;
        }

        // Markdown heading prefix inside HTML paragraph (`<p>## Alt Başlık</p>`)
        if let Some((md_htype, md_txt)) = detect_explicit_markdown_heading(&plain) {
            blocks.push(ParsedPasteBlock {
                block_type: md_htype.to_string(),
                content: md_txt,
                meta: None,
            });
            return;
        }

        let sanitized = sanitize_inline_rich_html(raw_content.trim());

        // Promote inline font-size heading or whole-paragraph bold subheading
        if h1_hint && plain.chars().count() <= 140 && !ends_with_prose_period(&plain) {
            blocks.push(ParsedPasteBlock {
                block_type: "heading1".to_string(),
                content: plain,
                meta: None,
            });
            return;
        }
        if (h2_hint || is_entirely_bold_subheading(&sanitized, &plain))
            && plain.chars().count() <= 115
            && !ends_with_prose_period(&plain)
        {
            blocks.push(ParsedPasteBlock {
                block_type: "heading".to_string(),
                content: plain,
                meta: None,
            });
            return;
        }

        blocks.push(ParsedPasteBlock {
            block_type: "paragraph".to_string(),
            content: sanitized,
            meta: None,
        });
    };

    for tok in tokens {
        match tok {
            HtmlToken::StartTag {
                name,
                attrs,
                self_closing,
            } => {
                if matches!(
                    name.as_str(),
                    "script"
                        | "style"
                        | "noscript"
                        | "svg"
                        | "head"
                        | "title"
                        | "button"
                        | "select"
                        | "form"
                        | "nav"
                        | "template"
                ) {
                    if !self_closing {
                        skip_depth += 1;
                    }
                    continue;
                }
                if skip_depth > 0 {
                    continue;
                }

                if in_pre && name != "br" {
                    continue;
                }

                if !self_closing {
                    if let Some(st) = get_attr(&attrs, "style") {
                        let sl = st.to_ascii_lowercase();
                        if sl.contains("white-space:pre")
                            || sl.contains("white-space: pre")
                        {
                            pre_wrap_tags.push(name.clone());
                        }
                    }
                }

                match name.as_str() {
                    "meta" | "link" | "source" | "wbr" | "col" | "colgroup" => {}

                    "br" => {
                        if in_pre {
                            pre_buf.push('\n');
                        } else if in_figcaption {
                            figcaption_buf.push(' ');
                        } else if in_cite {
                            quote_cite_buf.push(' ');
                        } else if in_blockquote {
                            buf.push_str("<br/>");
                        } else {
                            // Split every <br>-separated line into its own editor block!
                            flush_buf(
                                &mut buf,
                                &mut blocks,
                                &inline_stack,
                                active_heading,
                                in_blockquote,
                                &mut quote_cite_buf,
                                &mut last_quote_block_idx,
                                &mut list_stack,
                                in_li,
                                &mut word_list_para,
                                &mut buf_saw_large_h1,
                                &mut buf_saw_large_h2,
                            );
                            if !inline_stack.is_empty() {
                                reopen_all_marks(&mut buf, &inline_stack);
                            }
                        }
                    }

                    "hr" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        blocks.push(ParsedPasteBlock {
                            block_type: "divider".to_string(),
                            content: String::new(),
                            meta: None,
                        });
                    }

                    "img" => {
                        if let Some((img_url, alt_opt)) = extract_best_img_from_attrs(&attrs) {
                            let reopen_needed = !buf.trim().is_empty() && !inline_stack.is_empty();
                            flush_buf(
                                &mut buf,
                                &mut blocks,
                                &inline_stack,
                                active_heading,
                                in_blockquote,
                                &mut quote_cite_buf,
                                &mut last_quote_block_idx,
                                &mut list_stack,
                                in_li,
                                &mut word_list_para,
                                &mut buf_saw_large_h1,
                                &mut buf_saw_large_h2,
                            );
                            blocks.push(ParsedPasteBlock {
                                block_type: "image".to_string(),
                                content: img_url,
                                meta: alt_opt,
                            });
                            if in_figure {
                                last_figure_block_idx = Some(blocks.len() - 1);
                            }
                            if reopen_needed {
                                reopen_all_marks(&mut buf, &inline_stack);
                            }
                        }
                    }

                    "iframe" => {
                        if let Some(embed_url) = extract_iframe_embed_url(&attrs) {
                            flush_buf(
                                &mut buf,
                                &mut blocks,
                                &inline_stack,
                                active_heading,
                                in_blockquote,
                                &mut quote_cite_buf,
                                &mut last_quote_block_idx,
                                &mut list_stack,
                                in_li,
                                &mut word_list_para,
                                &mut buf_saw_large_h1,
                                &mut buf_saw_large_h2,
                            );
                            blocks.push(ParsedPasteBlock {
                                block_type: "embed".to_string(),
                                content: embed_url,
                                meta: None,
                            });
                            if in_figure {
                                last_figure_block_idx = Some(blocks.len() - 1);
                            }
                        }
                        if !self_closing {
                            skip_depth += 1;
                        }
                    }

                    "figure" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        in_figure = true;
                        figcaption_buf.clear();
                        last_figure_block_idx = None;
                    }

                    "figcaption" => {
                        in_figcaption = true;
                        figcaption_buf.clear();
                    }

                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        active_heading = Some(match name.as_str() {
                            "h1" => "heading1",
                            "h2" => "heading",
                            _ => "heading3",
                        });
                    }

                    "ul" | "ol" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        let kind = if name == "ol" {
                            ListKind::Numbered
                        } else {
                            ListKind::Bullet
                        };
                        list_stack.push((kind, Vec::new()));
                    }

                    "li" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        in_li = true;
                    }

                    "blockquote" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        blockquote_embed_override = get_attr(&attrs, "data-instgrm-permalink")
                            .filter(|u| crate::editorial::wasm::render_right::extract_instagram_embed_url(u).is_some())
                            .or_else(|| {
                                get_attr(&attrs, "cite").filter(|u| {
                                    u.to_ascii_lowercase().contains("tiktok.com/")
                                        || crate::editorial::wasm::render_right::extract_tweet_id(u).is_some()
                                })
                            })
                            .map(|u| u.trim().to_string());
                        in_blockquote = true;
                        quote_cite_buf.clear();
                        last_quote_block_idx = None;
                    }

                    "cite" if in_blockquote => {
                        in_cite = true;
                    }

                    "pre" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        in_pre = true;
                        pre_buf.clear();
                    }

                    "p" | "div" | "section" | "article" | "header" | "main" | "tr" | "dd"
                    | "dt" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        if let Some(cls) = get_attr(&attrs, "class") {
                            let cl = cls.to_ascii_lowercase();
                            if cl.contains("msoheading1") || cl.contains("msotitle") {
                                active_heading = Some("heading1");
                            } else if cl.contains("msoheading2") || cl.contains("msosubtitle") {
                                active_heading = Some("heading");
                            } else if cl.contains("msoheading3") || cl.contains("msoheading4") {
                                active_heading = Some("heading3");
                            } else if cl.contains("msolistparagraph") {
                                word_list_para = true;
                            }
                        }
                        if !inline_stack.is_empty() {
                            reopen_all_marks(&mut buf, &inline_stack);
                        }
                    }

                    "td" | "th" => {
                        if !buf.trim().is_empty() && !buf.ends_with(' ') {
                            buf.push_str(" — ");
                        }
                    }

                    // Inline formatting elements (`b`, `strong`, `i`, `em`, `u`, `s`, `mark`, `code`, `a`, `span`, `font`)
                    _ => {
                        let marks = classify_inline_marks(&name, &attrs);
                        if !marks.is_empty() {
                            for m in &marks {
                                match m {
                                    InlineMark::LargeHeading(1) => buf_saw_large_h1 = true,
                                    InlineMark::LargeHeading(2) => buf_saw_large_h2 = true,
                                    _ => buf.push_str(&m.open_html()),
                                }
                            }
                            if !self_closing {
                                inline_stack.push((name, marks));
                            }
                        }
                    }
                }
            }

            HtmlToken::EndTag { name } => {
                if matches!(
                    name.as_str(),
                    "script"
                        | "style"
                        | "noscript"
                        | "svg"
                        | "head"
                        | "title"
                        | "button"
                        | "select"
                        | "form"
                        | "nav"
                        | "template"
                        | "iframe"
                ) {
                    skip_depth = skip_depth.saturating_sub(1);
                    continue;
                }
                if skip_depth > 0 {
                    continue;
                }

                if name == "pre" && in_pre {
                    in_pre = false;
                    let code_clean = pre_buf.trim_matches('\n').to_string();
                    pre_buf.clear();
                    if !code_clean.trim().is_empty() {
                        blocks.push(ParsedPasteBlock {
                            block_type: "code".to_string(),
                            content: code_clean,
                            meta: None,
                        });
                    }
                    continue;
                }
                if in_pre {
                    continue;
                }

                if let Some(pw_pos) = pre_wrap_tags.iter().rposition(|t| *t == name) {
                    pre_wrap_tags.remove(pw_pos);
                }

                match name.as_str() {
                    "figcaption" => {
                        in_figcaption = false;
                        let cap = collapse_inline_spaces(&figcaption_buf);
                        if !cap.is_empty() {
                            if let Some(idx) = last_figure_block_idx {
                                if let Some(b) = blocks.get_mut(idx) {
                                    b.meta = Some(cap);
                                }
                            }
                        }
                    }

                    "figure" => {
                        let cap = collapse_inline_spaces(&figcaption_buf);
                        if !cap.is_empty() {
                            if let Some(idx) = last_figure_block_idx {
                                if let Some(b) = blocks.get_mut(idx) {
                                    b.meta = Some(cap);
                                }
                            }
                        }
                        in_figure = false;
                        in_figcaption = false;
                        figcaption_buf.clear();
                        last_figure_block_idx = None;
                    }

                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        active_heading = None;
                    }

                    "li" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        in_li = false;
                    }

                    "ul" | "ol" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        in_li = false;
                        if let Some((kind, items)) = list_stack.pop() {
                            if !items.is_empty() {
                                let btype = match kind {
                                    ListKind::Bullet => "bullet_list",
                                    ListKind::Numbered => "numbered_list",
                                };
                                blocks.push(ParsedPasteBlock {
                                    block_type: btype.to_string(),
                                    content: items.join("\n"),
                                    meta: None,
                                });
                            }
                        }
                    }

                    "cite" if in_cite => {
                        in_cite = false;
                        let c = collapse_inline_spaces(&quote_cite_buf);
                        if !c.is_empty() {
                            if let Some(qidx) = last_quote_block_idx {
                                if let Some(b) = blocks.get_mut(qidx) {
                                    b.meta = Some(c);
                                    quote_cite_buf.clear();
                                }
                            }
                        }
                    }

                    "blockquote" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        let c = collapse_inline_spaces(&quote_cite_buf);
                        if !c.is_empty() {
                            if let Some(qidx) = last_quote_block_idx {
                                if let Some(b) = blocks.get_mut(qidx) {
                                    b.meta = Some(c);
                                }
                            }
                        }
                        // Upgrade official social embed blockquotes (`twitter-tweet`, `instagram-media`, `tiktok-embed`) into `embed` blocks!
                        if let Some(override_url) = blockquote_embed_override.take() {
                            if let Some(qidx) = last_quote_block_idx {
                                if let Some(b) = blocks.get_mut(qidx) {
                                    b.block_type = "embed".to_string();
                                    b.content = override_url;
                                    b.meta = None;
                                }
                            } else {
                                blocks.push(ParsedPasteBlock {
                                    block_type: "embed".to_string(),
                                    content: override_url,
                                    meta: None,
                                });
                            }
                        } else if let Some(qidx) = last_quote_block_idx {
                            if let Some(b) = blocks.get_mut(qidx) {
                                let extracted =
                                    crate::editorial::wasm::render_right::normalize_raw_embed_input(&b.content);
                                if crate::editorial::wasm::render_right::extract_tweet_id(&extracted).is_some() {
                                    b.block_type = "embed".to_string();
                                    b.content = extracted;
                                    b.meta = None;
                                }
                            }
                        }
                        in_blockquote = false;
                        in_cite = false;
                        quote_cite_buf.clear();
                        last_quote_block_idx = None;
                    }

                    "p" | "div" | "section" | "article" | "header" | "main" | "tr" | "dd"
                    | "dt" => {
                        flush_buf(
                            &mut buf,
                            &mut blocks,
                            &inline_stack,
                            active_heading,
                            in_blockquote,
                            &mut quote_cite_buf,
                            &mut last_quote_block_idx,
                            &mut list_stack,
                            in_li,
                            &mut word_list_para,
                            &mut buf_saw_large_h1,
                            &mut buf_saw_large_h2,
                        );
                        active_heading = None;
                        word_list_para = false;
                    }

                    _ => {
                        if let Some(pos) = inline_stack.iter().rposition(|(tn, _)| *tn == name) {
                            let (_, popped_marks) = inline_stack.remove(pos);
                            if !buf.trim().is_empty() {
                                for m in popped_marks.iter().rev() {
                                    buf.push_str(m.close_html());
                                }
                            }
                        }
                    }
                }
            }

            HtmlToken::Text(txt) => {
                if skip_depth > 0 {
                    continue;
                }
                if in_pre {
                    pre_buf.push_str(&txt);
                    continue;
                }
                if in_figcaption {
                    figcaption_buf.push_str(&txt);
                    continue;
                }
                if in_cite {
                    quote_cite_buf.push_str(&txt);
                    continue;
                }

                if !pre_wrap_tags.is_empty() && (txt.contains('\n') || txt.contains('\r')) {
                    let norm_lines = txt.replace("\r\n", "\n").replace('\r', "\n");
                    let parts: Vec<&str> = norm_lines.split('\n').collect();
                    for (idx, part) in parts.iter().enumerate() {
                        if !part.trim().is_empty() {
                            if buf.is_empty() && !inline_stack.is_empty() {
                                reopen_all_marks(&mut buf, &inline_stack);
                            }
                            buf.push_str(&escape_text_for_inline_html(&part.replace('\t', " ")));
                        }
                        if idx + 1 < parts.len() {
                            flush_buf(
                                &mut buf,
                                &mut blocks,
                                &inline_stack,
                                active_heading,
                                in_blockquote,
                                &mut quote_cite_buf,
                                &mut last_quote_block_idx,
                                &mut list_stack,
                                in_li,
                                &mut word_list_para,
                                &mut buf_saw_large_h1,
                                &mut buf_saw_large_h2,
                            );
                        }
                    }
                    continue;
                }

                // Normalize raw HTML source newlines into spaces unless meaningful
                let normalized = txt.replace("\r\n", " ").replace(['\r', '\n', '\t'], " ");
                if normalized.trim().is_empty() {
                    if !buf.is_empty() && !buf.ends_with(' ') && !buf.ends_with('>') {
                        buf.push(' ');
                    }
                } else {
                    if buf.is_empty() && !inline_stack.is_empty() {
                        reopen_all_marks(&mut buf, &inline_stack);
                    }
                    buf.push_str(&escape_text_for_inline_html(&normalized));
                }
            }
        }
    }

    flush_buf(
        &mut buf,
        &mut blocks,
        &inline_stack,
        active_heading,
        in_blockquote,
        &mut quote_cite_buf,
        &mut last_quote_block_idx,
        &mut list_stack,
        in_li,
        &mut word_list_para,
        &mut buf_saw_large_h1,
        &mut buf_saw_large_h2,
    );

    promote_unmarked_editorial_subheadings(&mut blocks);
    blocks
}

fn classify_inline_marks(tag: &str, attrs: &[(String, String)]) -> Vec<InlineMark> {
    let style_lower = get_attr(attrs, "style")
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    // Google Docs wraps the entire document in `<b style="font-weight:normal;">`!
    let explicitly_normal_weight = style_lower.contains("font-weight:normal")
        || style_lower.contains("font-weight: normal")
        || style_lower.contains("font-weight:400")
        || style_lower.contains("font-weight: 400");

    let mut marks = Vec::new();

    match tag {
        "b" | "strong" => {
            if explicitly_normal_weight {
                marks.push(InlineMark::Noop);
            } else {
                marks.push(InlineMark::Strong);
            }
        }
        "i" | "em" => marks.push(InlineMark::Em),
        "u" | "ins" => marks.push(InlineMark::Underline),
        "s" | "strike" | "del" => marks.push(InlineMark::Strike),
        "mark" => marks.push(InlineMark::Mark),
        "code" => marks.push(InlineMark::Code),
        "a" => {
            if let Some(raw_href) = get_attr(attrs, "href") {
                if let Some(safe_href) = sanitize_paste_href(raw_href) {
                    marks.push(InlineMark::Link(safe_href));
                }
            }
        }
        _ => {}
    }

    if !style_lower.is_empty() {
        let style_bold = !explicitly_normal_weight
            && (style_lower.contains("font-weight:bold")
                || style_lower.contains("font-weight: bold")
                || style_lower.contains("font-weight:700")
                || style_lower.contains("font-weight: 700")
                || style_lower.contains("font-weight:800")
                || style_lower.contains("font-weight: 800")
                || style_lower.contains("font-weight:900")
                || style_lower.contains("font-weight: 900"));
        if style_bold && !marks.iter().any(|m| matches!(m, InlineMark::Strong)) {
            marks.push(InlineMark::Strong);
        }

        if style_lower.contains("font-style:italic") || style_lower.contains("font-style: italic") {
            if !marks.iter().any(|m| matches!(m, InlineMark::Em)) {
                marks.push(InlineMark::Em);
            }
        }

        if let Some(h_lvl) = detect_large_heading_font_size(&style_lower) {
            marks.push(InlineMark::LargeHeading(h_lvl));
        }
    }

    marks
}

fn detect_large_heading_font_size(style_lower: &str) -> Option<u8> {
    let pos = style_lower.find("font-size:")?;
    let after = style_lower[pos + "font-size:".len()..].trim_start();
    let val_str = after.split([';', '"', '\'']).next()?.trim();
    if let Some(px_str) = val_str.strip_suffix("px") {
        let px = px_str.trim().parse::<f32>().ok()?;
        if px >= 26.0 {
            return Some(1);
        } else if px >= 19.5 {
            return Some(2);
        }
    } else if let Some(pt_str) = val_str.strip_suffix("pt") {
        let pt = pt_str.trim().parse::<f32>().ok()?;
        if pt >= 20.0 {
            return Some(1);
        } else if pt >= 14.5 {
            return Some(2);
        }
    }
    None
}

fn sanitize_paste_href(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("mailto:")
        || (trimmed.starts_with('/') && !trimmed.starts_with("//"))
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn extract_best_img_from_attrs(attrs: &[(String, String)]) -> Option<(String, Option<String>)> {
    // Reject 1x1 tracking pixels
    for dim_key in ["width", "height"] {
        if let Some(v) = get_attr(attrs, dim_key) {
            let clean_d = v.trim().trim_end_matches("px").trim();
            if clean_d == "1" || clean_d == "0" {
                return None;
            }
        }
    }

    let lazy_keys = [
        "data-src",
        "data-original",
        "data-lazy-src",
        "data-full-url",
        "data-actualsrc",
        "data-hi-res-src",
        "data-url",
    ];
    let mut chosen: Option<String> = None;
    for k in lazy_keys {
        if let Some(v) = get_attr(attrs, k) {
            if let Some(norm) = normalize_paste_img_src(v) {
                chosen = Some(norm);
                break;
            }
        }
    }

    if chosen.is_none() {
        if let Some(src) = get_attr(attrs, "src") {
            chosen = normalize_paste_img_src(src);
        }
    }

    if chosen.is_none() {
        for sk in ["data-srcset", "srcset"] {
            if let Some(srcset) = get_attr(attrs, sk) {
                if let Some(best) = pick_best_from_srcset(srcset) {
                    chosen = Some(best);
                    break;
                }
            }
        }
    }

    let url = chosen?;
    let lower_u = url.to_ascii_lowercase();
    if lower_u.contains("spacer.gif") || lower_u.contains("pixel.gif") || lower_u.contains("1x1.") {
        return None;
    }

    let alt_opt = get_attr(attrs, "alt")
        .map(collapse_inline_spaces)
        .filter(|a| {
            let al = a.to_ascii_lowercase();
            !a.is_empty()
                && a.chars().count() >= 3
                && !al.ends_with(".jpg")
                && !al.ends_with(".jpeg")
                && !al.ends_with(".png")
                && !al.ends_with(".webp")
                && !al.starts_with("img_")
                && !al.starts_with("dsc_")
                && al != "image"
                && al != "photo"
                && al != "görsel"
        });

    Some((url, alt_opt))
}

fn normalize_paste_img_src(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("data:image/svg")
        || lower.starts_with("data:image/gif;base64,r0lgodlh")
        || lower.starts_with("javascript:")
        || lower.starts_with("about:")
    {
        return None;
    }
    if lower.starts_with("data:image/png;base64,")
        || lower.starts_with("data:image/jpeg;base64,")
        || lower.starts_with("data:image/jpg;base64,")
        || lower.starts_with("data:image/webp;base64,")
        || lower.starts_with("data:image/gif;base64,")
    {
        return Some(trimmed.to_string());
    }
    if let Some(proto_rel) = trimmed.strip_prefix("//") {
        return Some(format!("https://{proto_rel}"));
    }
    if lower.starts_with("https://") || lower.starts_with("http://") || trimmed.starts_with('/') {
        return Some(trimmed.to_string());
    }
    None
}

fn pick_best_from_srcset(srcset: &str) -> Option<String> {
    let mut best_url: Option<String> = None;
    let mut best_score: u32 = 0;

    for part in srcset.split(',') {
        let mut iter = part.split_whitespace();
        let Some(raw_url) = iter.next() else { continue };
        let Some(norm_url) = normalize_paste_img_src(raw_url) else {
            continue;
        };
        let descriptor = iter.next().unwrap_or("");
        let score = if let Some(w_str) = descriptor.strip_suffix('w') {
            w_str.parse::<u32>().unwrap_or(100)
        } else if let Some(x_str) = descriptor.strip_suffix('x') {
            (x_str.parse::<f32>().unwrap_or(1.0) * 1000.0) as u32
        } else {
            100
        };
        if score >= best_score {
            best_score = score;
            best_url = Some(norm_url);
        }
    }
    best_url
}

fn extract_iframe_embed_url(attrs: &[(String, String)]) -> Option<String> {
    let raw = get_attr(attrs, "src").or_else(|| get_attr(attrs, "data-src"))?;
    let trimmed = raw.trim();
    let full = if let Some(pr) = trimmed.strip_prefix("//") {
        format!("https://{pr}")
    } else {
        trimmed.to_string()
    };
    let lower = full.to_ascii_lowercase();
    if !lower.starts_with("https://") && !lower.starts_with("http://") {
        return None;
    }
    if let Some(pos) = lower.find("/embed/") {
        if lower.contains("youtube.com") || lower.contains("youtube-nocookie.com") {
            let after = &full[pos + "/embed/".len()..];
            if let Some(vid) = after.split(['?', '&', '/', '#']).next() {
                if !vid.is_empty() {
                    return Some(format!("https://www.youtube.com/watch?v={vid}"));
                }
            }
        }
    }
    if lower.contains("youtube.com")
        || lower.contains("youtu.be")
        || lower.contains("twitter.com")
        || lower.contains("x.com")
        || lower.contains("instagram.com")
        || lower.contains("vimeo.com")
        || lower.contains("dailymotion.com")
    {
        return Some(full);
    }
    None
}

// ============================================================================
// Plain-Text / Markdown Multi-Line Article Parser
// ============================================================================

pub fn parse_plain_text_to_blocks(text: &str) -> Vec<ParsedPasteBlock> {
    let mut blocks: Vec<ParsedPasteBlock> = Vec::new();
    let mut in_code_fence = false;
    let mut code_lines: Vec<String> = Vec::new();

    for raw_line in text.lines() {
        let trimmed = raw_line.trim();

        if trimmed.starts_with("```") {
            if in_code_fence {
                in_code_fence = false;
                let code_body = code_lines.join("\n");
                code_lines.clear();
                if !code_body.trim().is_empty() {
                    blocks.push(ParsedPasteBlock {
                        block_type: "code".to_string(),
                        content: code_body,
                        meta: None,
                    });
                }
            } else {
                in_code_fence = true;
                code_lines.clear();
            }
            continue;
        }

        if in_code_fence {
            code_lines.push(raw_line.to_string());
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        // Divider (`---`, `***`, `___`)
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            blocks.push(ParsedPasteBlock {
                block_type: "divider".to_string(),
                content: String::new(),
                meta: None,
            });
            continue;
        }

        // Raw `<img ...>` or `<iframe ...>` pasted as plain text
        if (trimmed.starts_with("<img") || trimmed.starts_with("<iframe") || trimmed.starts_with("<blockquote"))
            && trimmed.contains('>')
        {
            let parsed_snip = parse_html_to_blocks(trimmed);
            if !parsed_snip.is_empty() {
                blocks.extend(parsed_snip);
                continue;
            }
        }

        // Markdown image `![alt](https://...)`
        if let Some((alt, url)) = parse_markdown_image_line(trimmed) {
            blocks.push(ParsedPasteBlock {
                block_type: "image".to_string(),
                content: url,
                meta: alt,
            });
            continue;
        }

        // Standalone image URL (`https://.../photo.jpg` or `/uploads/...`)
        if let Some(img_url) = detect_standalone_image_url(trimmed) {
            blocks.push(ParsedPasteBlock {
                block_type: "image".to_string(),
                content: img_url,
                meta: None,
            });
            continue;
        }

        // Standalone YouTube / X (Twitter) / Instagram embed URL
        if let Some(embed_url) = detect_standalone_embed_url(trimmed) {
            blocks.push(ParsedPasteBlock {
                block_type: "embed".to_string(),
                content: embed_url,
                meta: None,
            });
            continue;
        }

        // Explicit Markdown headings (`# `, `## `, `### `)
        if let Some((htype, htxt)) = detect_explicit_markdown_heading(trimmed) {
            blocks.push(ParsedPasteBlock {
                block_type: htype.to_string(),
                content: htxt,
                meta: None,
            });
            continue;
        }

        // Whole-line Markdown bold subheading (`**Alt Başlık**`)
        if let Some(bold_h) = detect_markdown_bold_heading_line(trimmed) {
            blocks.push(ParsedPasteBlock {
                block_type: "heading".to_string(),
                content: bold_h,
                meta: None,
            });
            continue;
        }

        // Quote (`> ...`)
        if let Some(q) = trimmed.strip_prefix("> ").or_else(|| trimmed.strip_prefix("» ")) {
            let clean_q = sanitize_inline_rich_html(q.trim());
            if !strip_all_html_tags(&clean_q).trim().is_empty() {
                blocks.push(ParsedPasteBlock {
                    block_type: "quote".to_string(),
                    content: clean_q,
                    meta: None,
                });
                continue;
            }
        }

        // Bullet list item (`- `, `* `, `• `) — group consecutive bullet items
        if let Some(b_item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("• "))
        {
            let clean_item = strip_all_html_tags(&sanitize_inline_rich_html(b_item.trim()))
                .trim()
                .to_string();
            if !clean_item.is_empty() {
                if let Some(last) = blocks.last_mut() {
                    if last.block_type == "bullet_list" {
                        last.content.push('\n');
                        last.content.push_str(&clean_item);
                        continue;
                    }
                }
                blocks.push(ParsedPasteBlock {
                    block_type: "bullet_list".to_string(),
                    content: clean_item,
                    meta: None,
                });
                continue;
            }
        }

        // Numbered list item (`1. `, `2) `) — group consecutive numbered items
        if let Some(n_item) = strip_numbered_list_prefix(trimmed) {
            let clean_item = strip_all_html_tags(&sanitize_inline_rich_html(n_item.trim()))
                .trim()
                .to_string();
            if !clean_item.is_empty() {
                if let Some(last) = blocks.last_mut() {
                    if last.block_type == "numbered_list" {
                        last.content.push('\n');
                        last.content.push_str(&clean_item);
                        continue;
                    }
                }
                blocks.push(ParsedPasteBlock {
                    block_type: "numbered_list".to_string(),
                    content: clean_item,
                    meta: None,
                });
                continue;
            }
        }

        // Regular line -> separate `paragraph` block!
        let clean_para = sanitize_inline_rich_html(trimmed);
        if !strip_all_html_tags(&clean_para).trim().is_empty() {
            blocks.push(ParsedPasteBlock {
                block_type: "paragraph".to_string(),
                content: clean_para,
                meta: None,
            });
        }
    }

    if in_code_fence && !code_lines.is_empty() {
        blocks.push(ParsedPasteBlock {
            block_type: "code".to_string(),
            content: code_lines.join("\n"),
            meta: None,
        });
    }

    promote_unmarked_editorial_subheadings(&mut blocks);
    blocks
}

// ============================================================================
// Shared Heading, Image & Embed Heuristics
// ============================================================================

fn collapse_inline_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn ends_with_prose_period(s: &str) -> bool {
    let t = s.trim();
    t.ends_with('.') || t.ends_with(',') || t.ends_with(';') || t.ends_with('…')
}

fn is_entirely_bold_subheading(sanitized_html: &str, plain: &str) -> bool {
    let t = sanitized_html.trim();
    if !(t.starts_with("<strong>") && t.ends_with("</strong>")) {
        return false;
    }
    let inner = &t["<strong>".len()..t.len() - "</strong>".len()];
    // If `</strong>` appears in the middle, then only part of the paragraph was bold!
    if inner.contains("</strong>") {
        return false;
    }
    let char_cnt = plain.chars().count();
    (3..=115).contains(&char_cnt) && !ends_with_prose_period(plain)
}

fn detect_explicit_markdown_heading(line: &str) -> Option<(&'static str, String)> {
    let trimmed = line.trim();
    if let Some(h3) = trimmed
        .strip_prefix("#### ")
        .or_else(|| trimmed.strip_prefix("### "))
    {
        let c = h3.trim().trim_matches('*').trim().to_string();
        if !c.is_empty() {
            return Some(("heading3", c));
        }
    }
    if let Some(h2) = trimmed.strip_prefix("## ") {
        let c = h2.trim().trim_matches('*').trim().to_string();
        if !c.is_empty() {
            return Some(("heading", c));
        }
    }
    if let Some(h1) = trimmed.strip_prefix("# ") {
        let c = h1.trim().trim_matches('*').trim().to_string();
        if !c.is_empty() {
            return Some(("heading1", c));
        }
    }
    None
}

fn detect_markdown_bold_heading_line(line: &str) -> Option<String> {
    let t = line.trim();
    let inner = t.strip_prefix("**")?.strip_suffix("**")?.trim();
    if inner.is_empty() || inner.contains("**") || ends_with_prose_period(inner) {
        return None;
    }
    let len = inner.chars().count();
    (3..=115).contains(&len).then(|| inner.to_string())
}

fn strip_numbered_list_prefix(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    let mut d_end = 0usize;
    while d_end < bytes.len() && bytes[d_end].is_ascii_digit() {
        d_end += 1;
    }
    if d_end == 0 || d_end > 3 || d_end >= bytes.len() {
        return None;
    }
    let sep = bytes[d_end];
    if (sep == b'.' || sep == b')') && line[d_end + 1..].starts_with(' ') {
        Some(line[d_end + 2..].trim_start())
    } else {
        None
    }
}

fn parse_markdown_image_line(line: &str) -> Option<(Option<String>, String)> {
    let t = line.trim();
    if !t.starts_with("![") || !t.ends_with(')') {
        return None;
    }
    let sep = t.find("](")?;
    let alt = t[2..sep].trim();
    let url_part = t[sep + 2..t.len() - 1].trim();
    let url_only = url_part.split_whitespace().next()?.trim();
    let norm = normalize_paste_img_src(url_only)?;
    let alt_opt = (!alt.is_empty()).then(|| alt.to_string());
    Some((alt_opt, norm))
}

fn detect_standalone_image_url(line: &str) -> Option<String> {
    let t = line.trim();
    if t.contains(char::is_whitespace) {
        return None;
    }
    let norm = normalize_paste_img_src(t)?;
    if norm.starts_with("data:image/")
        || norm.starts_with("/uploads/editorial/")
        || norm.starts_with("/uploads/")
        || norm.starts_with("/images/")
    {
        return Some(norm);
    }
    let path_only = norm
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if path_only.ends_with(".jpg")
        || path_only.ends_with(".jpeg")
        || path_only.ends_with(".png")
        || path_only.ends_with(".webp")
        || path_only.ends_with(".gif")
        || path_only.ends_with(".avif")
    {
        Some(norm)
    } else {
        None
    }
}

fn detect_standalone_embed_url(line: &str) -> Option<String> {
    crate::editorial::wasm::render_right::detect_standalone_media_embed_url(line)
}

/// Promotes unmarked standalone editorial subheadings in multi-paragraph pasted articles:
/// e.g. `"Merkez Bankası Ne Yapacak?"` or `"EKONOMİDE YENİ DÖNEM"` or `"Başvuru Şartları Neler"`
/// when immediately followed by a real prose paragraph or list.
fn promote_unmarked_editorial_subheadings(blocks: &mut [ParsedPasteBlock]) {
    if blocks.len() < 2 {
        return;
    }

    for i in 0..blocks.len() {
        if blocks[i].block_type != "paragraph" {
            continue;
        }
        if blocks[i].content.contains("<a ") {
            continue;
        }
        let plain = collapse_inline_spaces(&strip_all_html_tags(&blocks[i].content));
        let char_cnt = plain.chars().count();
        if !(4..=85).contains(&char_cnt) {
            continue;
        }
        if plain.starts_with("http://") || plain.starts_with("https://") {
            continue;
        }
        if ends_with_prose_period(&plain) || plain.ends_with('!') {
            continue;
        }

        let words: Vec<&str> = plain.split_whitespace().collect();
        if words.is_empty() || words.len() > 11 {
            continue;
        }

        let Some(first_ch) = plain.chars().next() else {
            continue;
        };
        if !first_ch.is_uppercase() {
            continue;
        }

        // Case A: ALL CAPS heading line (`>= 2` words, >= 5 alphabetic chars)
        let alpha_chars: Vec<char> = plain.chars().filter(|c| c.is_alphabetic()).collect();
        let is_all_caps = words.len() >= 2
            && alpha_chars.len() >= 5
            && alpha_chars.iter().all(|c| c.is_uppercase());

        // Case B: Followed immediately by a real prose paragraph (ending with `.` or >= 65 chars) or list
        let followed_by_body = blocks.get(i + 1).map_or(false, |next| {
            if matches!(next.block_type.as_str(), "bullet_list" | "numbered_list") {
                return true;
            }
            if next.block_type == "paragraph" {
                let next_plain = strip_all_html_tags(&next.content);
                let nt = next_plain.trim();
                return nt.chars().count() >= 65 || nt.ends_with('.') || nt.ends_with('!');
            }
            false
        });

        if is_all_caps || followed_by_body {
            blocks[i].block_type = "heading".to_string();
            blocks[i].content = plain;
            blocks[i].meta = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_spaced_and_multiline_plain_text_into_separate_blocks_with_headings_and_images() {
        let article = "\
Enflasyon Rakamları Açıklandı

TÜİK kasım ayı verilerini bugün saat 10.00'da kamuoyuyla paylaştı.
Aylık artış piyasa beklentilerinin altında gerçekleşti.

Merkez Bankası Ne Yapacak?

Ekonomistler önümüzdeki toplantıda faiz oranlarının sabit tutulmasını bekliyor.
https://cdn.example.com/uploads/grafik-kasim.webp
";

        let PasteParseOutcome::Blocks(blocks) = parse_clipboard_payload("", article) else {
            panic!("expected multi-block outcome");
        };

        assert_eq!(blocks.len(), 6);
        assert_eq!(blocks[0].block_type, "heading");
        assert_eq!(blocks[0].content, "Enflasyon Rakamları Açıklandı");

        assert_eq!(blocks[1].block_type, "paragraph");
        assert!(blocks[1].content.contains("TÜİK kasım ayı verilerini"));

        assert_eq!(blocks[2].block_type, "paragraph");
        assert!(blocks[2].content.contains("Aylık artış piyasa"));

        assert_eq!(blocks[3].block_type, "heading");
        assert_eq!(blocks[3].content, "Merkez Bankası Ne Yapacak?");

        assert_eq!(blocks[4].block_type, "paragraph");
        assert!(blocks[4].content.contains("Ekonomistler önümüzdeki"));

        assert_eq!(blocks[5].block_type, "image");
        assert_eq!(
            blocks[5].content,
            "https://cdn.example.com/uploads/grafik-kasim.webp"
        );
    }

    #[test]
    fn parses_rich_web_article_html_headings_br_lines_lazy_images_lists_quotes_and_strips_xss() {
        let html = r#"
        <article>
            <script>alert('xss')</script>
            <h1>Türkiye&#8217;nin Dijital Dönüşüm Hamlesi</h1>
            <p>Sanayi ve Teknoloji Bakanlığı <strong>yeni destek paketini</strong> duyurdu.<br>Başvurular <em>pazartesi günü</em> başlıyor.</p>
            <figure>
                <img src="data:image/svg+xml;base64,PHN2Zz4=" data-src="//img.haber.com/manset-1200.jpg" alt="Bakanlık binası" />
                <figcaption>AA / Ankara</figcaption>
            </figure>
            <h2>Başvuru Şartları Neler?</h2>
            <ul>
                <li>KOBİ statüsünde olmak</li>
                <li>En az 2 yıllık faaliyet belgesi</li>
            </ul>
            <blockquote><p>Yerli yazılım ihracatını ikiye katlayacağız.</p><cite>Bakanlık Açıklaması</cite></blockquote>
        </article>"#;

        let PasteParseOutcome::Blocks(blocks) = parse_clipboard_payload(html, "") else {
            panic!("expected blocks");
        };

        assert_eq!(blocks.len(), 7);
        assert_eq!(blocks[0].block_type, "heading1");
        assert_eq!(blocks[0].content, "Türkiye’nin Dijital Dönüşüm Hamlesi");

        assert_eq!(blocks[1].block_type, "paragraph");
        assert_eq!(
            blocks[1].content,
            "Sanayi ve Teknoloji Bakanlığı <strong>yeni destek paketini</strong> duyurdu."
        );

        assert_eq!(blocks[2].block_type, "paragraph");
        assert_eq!(
            blocks[2].content,
            "Başvurular <em>pazartesi günü</em> başlıyor."
        );

        assert_eq!(blocks[3].block_type, "image");
        assert_eq!(blocks[3].content, "https://img.haber.com/manset-1200.jpg");
        assert_eq!(blocks[3].meta.as_deref(), Some("AA / Ankara"));

        assert_eq!(blocks[4].block_type, "heading");
        assert_eq!(blocks[4].content, "Başvuru Şartları Neler?");

        assert_eq!(blocks[5].block_type, "bullet_list");
        assert_eq!(
            blocks[5].content,
            "KOBİ statüsünde olmak\nEn az 2 yıllık faaliyet belgesi"
        );

        assert_eq!(blocks[6].block_type, "quote");
        assert_eq!(
            blocks[6].content,
            "Yerli yazılım ihracatını ikiye katlayacağız."
        );
        assert_eq!(blocks[6].meta.as_deref(), Some("Bakanlık Açıklaması"));
    }

    #[test]
    fn handles_google_docs_normal_weight_b_wrapper_and_word_bold_subheadings() {
        let gdocs_html = r#"
        <meta charset="utf-8"><b style="font-weight:normal;" id="docs-internal-guid-999">
            <p dir="ltr"><span style="font-weight:700">İkinci Yarıda Gelen Goller</span></p>
            <p dir="ltr"><span style="font-weight:400">Normal metin ve </span><span style="font-weight:700">kalın kelime</span><span style="font-weight:400"> devamı.</span></p>
            <p dir="ltr"><img src="https://lh7-rt.googleusercontent.com/docsz/img123" width="600" height="400"></p>
        </b>"#;

        let PasteParseOutcome::Blocks(blocks) = parse_clipboard_payload(gdocs_html, "") else {
            panic!("expected blocks");
        };

        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].block_type, "heading");
        assert_eq!(blocks[0].content, "İkinci Yarıda Gelen Goller");
        assert_eq!(blocks[1].block_type, "paragraph");
        assert_eq!(
            blocks[1].content,
            "Normal metin ve <strong>kalın kelime</strong> devamı."
        );
        assert_eq!(blocks[2].block_type, "image");
        assert_eq!(
            blocks[2].content,
            "https://lh7-rt.googleusercontent.com/docsz/img123"
        );
    }

    #[test]
    fn keeps_single_short_phrase_as_inline_fragment() {
        let out = parse_clipboard_payload("<span>merkez bankası</span>", "merkez bankası");
        assert_eq!(
            out,
            PasteParseOutcome::InlineFragment {
                html: "merkez bankası".to_string(),
                plain: "merkez bankası".to_string(),
            }
        );
    }

    #[test]
    fn splits_pre_wrap_html_from_vscode_whatsapp_or_notes_into_separate_blocks() {
        let pre_wrap_html = r#"<div style="white-space: pre-wrap;">Yeni Vergi Düzenlemesi Meclis'te

Teklif bugün komisyonda görüşülmeye başlandı.
Maddelerin hafta sonuna kadar yasalaşması bekleniyor.</div>"#;
        let plain = "Yeni Vergi Düzenlemesi Meclis'te\n\nTeklif bugün komisyonda görüşülmeye başlandı.\nMaddelerin hafta sonuna kadar yasalaşması bekleniyor.";

        let PasteParseOutcome::Blocks(blocks) = parse_clipboard_payload(pre_wrap_html, plain)
        else {
            panic!("expected blocks");
        };
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].block_type, "heading");
        assert_eq!(blocks[0].content, "Yeni Vergi Düzenlemesi Meclis'te");
        assert_eq!(blocks[1].block_type, "paragraph");
        assert_eq!(blocks[2].block_type, "paragraph");
    }
}
