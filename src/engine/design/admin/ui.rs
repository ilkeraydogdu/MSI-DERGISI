//! Reusable `HtmlBuf` UI primitives for the modular Admin Command Center.

use crate::engine::design::html_dsl::HtmlBuf;

pub fn join_cls(base: &str, extra: &str) -> String {
    if extra.is_empty() {
        base.to_string()
    } else {
        format!("{} {}", base, extra)
    }
}

fn svg_inner(name: &str) -> &'static str {
    let key = name.trim().strip_prefix("fa-").unwrap_or(name.trim());
    match key {
        "newspaper-o" | "newspaper" => r#"<path d="M4 22h16a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2H8a2 2 0 0 0-2 2v16a2 2 0 0 1-2 2Zm0 0a2 2 0 0 1-2-2v-9c0-1.1.9-2 2-2h2"/><path d="M18 14h-8"/><path d="M15 18h-5"/><path d="M10 6h8v4h-8V6Z"/>"#,
        "book" => r#"<path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20"/>"#,
        "picture-o" | "image" => r#"<rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/>"#,
        "sitemap" | "code-fork" => r#"<circle cx="12" cy="5" r="2"/><circle cx="5" cy="19" r="2"/><circle cx="19" cy="19" r="2"/><path d="M12 7v4"/><path d="M5 17v-3a3 3 0 0 1 3-3h8a3 3 0 0 1 3 3v3"/>"#,
        "bullhorn" => r#"<path d="m3 11 18-5v12L3 14v-3z"/><path d="M11.6 16.8a3 3 0 1 1-5.8-1.6"/>"#,
        "try" | "revenue" => r#"<path d="M9 3v18"/><path d="M6 8l8-3"/><path d="M6 12l8-3"/><path d="M9 21a7 7 0 0 0 7-7"/>"#,
        "building-o" | "building" => r#"<rect width="16" height="20" x="4" y="2" rx="2" ry="2"/><path d="M9 22v-4h6v4"/><path d="M8 6h.01"/><path d="M16 6h.01"/><path d="M12 6h.01"/><path d="M12 10h.01"/><path d="M12 14h.01"/><path d="M16 10h.01"/><path d="M16 14h.01"/><path d="M8 10h.01"/><path d="M8 14h.01"/>"#,
        "pie-chart" => r#"<path d="M21.21 15.89A10 10 0 1 1 8 2.83"/><path d="M22 12A10 10 0 0 0 12 2v10z"/>"#,
        "line-chart" | "area-chart" => r#"<path d="M3 3v18h18"/><path d="m19 9-5 5-4-4-3 3"/>"#,
        "bar-chart" => r#"<path d="M3 3v18h18"/><path d="M18 17V9"/><path d="M13 17V5"/><path d="M8 17v-3"/>"#,
        "users" => r#"<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>"#,
        "user-plus" => r#"<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><line x1="19" x2="19" y1="8" y2="14"/><line x1="22" x2="16" y1="11" y2="11"/>"#,
        "check-square-o" => r#"<path d="m9 11 3 3L22 4"/><path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"/>"#,
        "sliders" => r#"<line x1="4" x2="4" y1="21" y2="14"/><line x1="4" x2="4" y1="10" y2="3"/><line x1="12" x2="12" y1="21" y2="12"/><line x1="12" x2="12" y1="8" y2="3"/><line x1="20" x2="20" y1="21" y2="16"/><line x1="20" x2="20" y1="12" y2="3"/><line x1="2" x2="6" y1="14" y2="14"/><line x1="10" x2="14" y1="8" y2="8"/><line x1="18" x2="22" y1="16" y2="16"/>"#,
        "envelope-o" | "envelope" => r#"<rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/>"#,
        "cog" | "cogs" => r#"<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>"#,
        "shield" => r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/>"#,
        "server" => r#"<rect width="20" height="8" x="2" y="2" rx="2" ry="2"/><rect width="20" height="8" x="2" y="14" rx="2" ry="2"/><line x1="6" x2="6.01" y1="6" y2="6"/><line x1="6" x2="6.01" y1="18" y2="18"/>"#,
        "exchange" | "random" => r#"<path d="m16 3 4 4-4 4"/><path d="M20 7H4"/><path d="m8 21-4-4 4-4"/><path d="M4 17h16"/>"#,
        "puzzle-piece" | "cube" | "cubes" => r#"<path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>"#,
        "paint-brush" => r#"<path d="m14.622 17.897-10.68-2.913"/><path d="M18.376 2.622a1 1 0 1 1 3.002 3.002L17.36 9.643a.5.5 0 0 0 0 .707l.944.944a2.41 2.41 0 0 1 0 3.408l-.944.944a.5.5 0 0 1-.707 0L8.354 7.348a.5.5 0 0 1 0-.707l.944-.944a2.41 2.41 0 0 1 3.408 0l.944.944a.5.5 0 0 0 .707 0z"/><path d="M9 8c-1.804 2.71-3.97 3.46-6.583 3.948a.507.507 0 0 0-.302.819l7.32 8.883a1 1 0 0 0 1.185.204C12.735 20.405 16 16.792 16 15"/>"#,
        "sign-out" => r#"<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" x2="9" y1="12" y2="12"/>"#,
        "sign-in" => r#"<path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/><polyline points="10 17 15 12 10 7"/><line x1="15" x2="3" y1="12" y2="12"/>"#,
        "bars" => r#"<line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="18" y2="18"/>"#,
        "times" => r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#,
        "search" | "search-plus" => r#"<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>"#,
        "refresh" => r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>"#,
        "adjust" => r#"<circle cx="12" cy="12" r="9"/><path d="M12 3v18"/><path d="M12 8l5-3"/><path d="M12 13l6-3.5"/><path d="M12 18l4-2.5"/>"#,
        "key" => r#"<circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/>"#,
        "plus" => r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#,
        "heartbeat" => r#"<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>"#,
        "eye" => r#"<path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/>"#,
        "qrcode" => r#"<rect width="5" height="5" x="3" y="3" rx="1"/><rect width="5" height="5" x="16" y="3" rx="1"/><rect width="5" height="5" x="3" y="16" rx="1"/><path d="M21 16h-3a2 2 0 0 0-2 2v3"/><path d="M21 21v.01"/><path d="M12 7v3a2 2 0 0 1-2 2H7"/><path d="M3 12h.01"/><path d="M12 3h.01"/><path d="M12 16v.01"/><path d="M16 12h1"/><path d="M21 12v.01"/><path d="M12 21v-1"/>"#,
        "check" => r#"<path d="M20 6 9 17l-5-5"/>"#,
        "check-circle" => r#"<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>"#,
        "exclamation-triangle" => r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/>"#,
        "arrow-left" => r#"<path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>"#,
        "arrow-right" => r#"<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>"#,
        "lock" => r#"<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>"#,
        "unlock-alt" => r#"<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/>"#,
        "eraser" => r#"<path d="m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.6-9.6c1-1 2.5-1 3.4 0l5.6 5.6c1 1 1 2.5 0 3.4L13 21"/><path d="M22 21H7"/><path d="m5 11 9 9"/>"#,
        "upload" => r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/>"#,
        "download" => r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>"#,
        "save" | "floppy-o" => r#"<path d="M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7"/><path d="M7 3v4a1 1 0 0 0 1 1h7"/>"#,
        "file-pdf-o" | "file-text-o" | "file-excel-o" => r#"<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>"#,
        "list" | "tasks" => r#"<line x1="8" x2="21" y1="6" y2="6"/><line x1="8" x2="21" y1="12" y2="12"/><line x1="8" x2="21" y1="18" y2="18"/><line x1="3" x2="3.01" y1="6" y2="6"/><line x1="3" x2="3.01" y1="12" y2="12"/><line x1="3" x2="3.01" y1="18" y2="18"/>"#,
        "star" => r#"<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>"#,
        "clock-o" | "history" => r#"<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>"#,
        "diamond" => r#"<path d="M6 3h12l4 6-10 13L2 9Z"/><path d="M11 3 8 9l4 13 4-13-3-6"/><path d="M2 9h20"/>"#,
        "credit-card" => r#"<rect width="20" height="14" x="2" y="5" rx="2"/><line x1="2" x2="22" y1="10" y2="10"/>"#,
        "filter" => r#"<polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/>"#,
        "th-large" => r#"<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/>"#,
        "play" => r#"<polygon points="6 3 20 12 6 21 6 3"/>"#,
        "paper-plane" => r#"<path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/>"#,
        "bell" | "bell-o" => r#"<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>"#,
        "magic" => r#"<path d="m21.64 3.64-1.28-1.28a1.21 1.21 0 0 0-1.72 0L2.36 18.64a1.21 1.21 0 0 0 0 1.72l1.28 1.28a1.2 1.2 0 0 0 1.72 0L21.64 5.36a1.2 1.2 0 0 0 0-1.72"/><path d="m14 7 3 3"/><path d="M5 6v4"/><path d="M19 14v4"/><path d="M10 2v2"/><path d="M7 8H3"/><path d="M21 16h-4"/><path d="M11 3H9"/>"#,
        "language" => r#"<path d="m5 8 6 6"/><path d="m4 14 6-6 2-3"/><path d="M2 5h12"/><path d="M7 2h1"/><path d="m22 22-5-10-5 10"/><path d="M14 18h6"/>"#,
        "globe" => r#"<circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/>"#,
        "tachometer" => r#"<path d="m12 14 4-4"/><path d="M3.34 19a10 10 0 1 1 17.32 0"/>"#,
        "hdd-o" => r#"<line x1="22" x2="2" y1="12" y2="12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><line x1="6" x2="6.01" y1="16" y2="16"/><line x1="10" x2="10.01" y1="16" y2="16"/>"#,
        "ban" => r#"<circle cx="12" cy="12" r="10"/><path d="m4.9 4.9 14.2 14.2"/>"#,
        "database" => r#"<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14a9 3 0 0 0 18 0V5"/><path d="M3 12a9 3 0 0 0 18 0"/>"#,
        "tags" => r#"<path d="m15 5 6.3 6.3a2.4 2.4 0 0 1 0 3.4L17 19"/><path d="M9.586 5.586A2 2 0 0 0 8.172 5H3a1 1 0 0 0-1 1v5.172a2 2 0 0 0 .586 1.414L8.29 18.29a2.426 2.426 0 0 0 3.42 0l3.58-3.58a2.426 2.426 0 0 0 0-3.42z"/><circle cx="6.5" cy="9.5" r=".5" fill="currentColor"/>"#,
        "bolt" => r#"<path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z"/>"#,
        "pencil-square-o" | "edit" => r#"<path d="M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.375 2.625a2.121 2.121 0 1 1 3 3L12 15l-4 1 1-4Z"/>"#,
        _ => r#"<circle cx="12" cy="12" r="9"/><path d="M12 8v4"/><path d="M12 16h.01"/>"#,
    }
}

pub fn icon(buf: &mut HtmlBuf, name: &str) {
    icon_with_attrs(buf, name, "", None);
}

pub fn icon_with_attrs(buf: &mut HtmlBuf, name: &str, extra_cls: &str, id: Option<&str>) {
    let cls = join_cls("adm-icon", extra_cls);
    buf.el_with(
        "svg",
        |a| {
            a.attr("class", &cls);
            if let Some(svg_id) = id {
                a.attr("id", svg_id);
            }
            a.attr("width", "16")
                .attr("height", "16")
                .attr("viewBox", "0 0 24 24")
                .attr("fill", "none")
                .attr("stroke", "currentColor")
                .attr("stroke-width", "1.85")
                .attr("stroke-linecap", "round")
                .attr("stroke-linejoin", "round")
                .attr("aria-hidden", "true");
        },
        |s| {
            s.raw(svg_inner(name));
        },
    );
}

pub fn action_btn(
    buf: &mut HtmlBuf,
    cls: &str,
    action: &str,
    perm: Option<&str>,
    icon_name: Option<&str>,
    label: &str,
) {
    action_btn_extra(buf, cls, action, perm, icon_name, label, &[]);
}

pub fn action_btn_extra(
    buf: &mut HtmlBuf,
    cls: &str,
    action: &str,
    perm: Option<&str>,
    icon_name: Option<&str>,
    label: &str,
    extra: &[(&str, &str)],
) {
    buf.el_with(
        "button",
        |a| {
            a.attr("type", "button")
                .attr("class", cls)
                .attr("data-admin-action", action);
            if let Some(p) = perm {
                a.attr("data-required-perm", p);
            }
            for (k, v) in extra {
                a.attr(k, v);
            }
        },
        |b| {
            if let Some(ic) = icon_name {
                icon(b, ic);
                if !label.is_empty() {
                    b.raw(" ");
                }
            }
            if !label.is_empty() {
                b.text(label);
            }
        },
    );
}

pub fn tab_pane<F: FnOnce(&mut HtmlBuf)>(
    buf: &mut HtmlBuf,
    id: &str,
    btn_id: &str,
    active: bool,
    body: F,
) {
    let cls = if active { "tab-pane active" } else { "tab-pane" };
    buf.el(
        "section",
        &[("id", id), ("class", cls), ("role", "tabpanel"), ("aria-labelledby", btn_id)],
        body,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn admin_card<H: FnOnce(&mut HtmlBuf), B: FnOnce(&mut HtmlBuf)>(
    buf: &mut HtmlBuf,
    extra_cls: &str,
    perm: Option<&str>,
    title_icon: &str,
    title: &str,
    subtitle: Option<&str>,
    header_right: H,
    body: B,
) {
    let cls = join_cls("admin-card", extra_cls);
    buf.el_with(
        "div",
        |a| {
            a.attr("class", &cls).attr("data-glow-card", "");
            if let Some(p) = perm {
                a.attr("data-required-perm", p);
            }
        },
        |card| {
            if !title.is_empty() {
                card.el("div", &[("class", "panel-header")], |hdr| {
                    hdr.el("div", &[("class", "panel-heading-group")], |left| {
                        left.el("div", &[("class", "panel-title")], |t| {
                            if !title_icon.is_empty() {
                                icon(t, title_icon);
                                t.raw(" ");
                            }
                            t.text(title);
                        });
                        if let Some(sub) = subtitle {
                            left.el("div", &[("class", "panel-subtitle")], |s| {
                                s.text(sub);
                            });
                        }
                    });
                    header_right(hdr);
                });
            }
            body(card);
        },
    );
}

pub fn admin_drawer<A: FnOnce(&mut HtmlBuf), B: FnOnce(&mut HtmlBuf), F: FnOnce(&mut HtmlBuf)>(
    buf: &mut HtmlBuf,
    id: &str,
    heading_id: &str,
    heading: &str,
    close_label: &str,
    header_actions: A,
    body: B,
    footer: F,
) {
    buf.el(
        "div",
        &[
            ("id", id),
            ("class", "admin-drawer-backdrop hidden"),
            ("role", "dialog"),
            ("aria-modal", "true"),
            ("aria-hidden", "true"),
            ("aria-labelledby", heading_id),
        ],
        |bd| {
            bd.el("div", &[("class", "admin-drawer")], |dr| {
                dr.el("div", &[("class", "drawer-header")], |hdr| {
                    hdr.el("h2", &[("class", "drawer-title")], |h| {
                        h.el("span", &[("id", heading_id)], |s| {
                            s.text(heading);
                        });
                    });
                    hdr.el("div", &[("class", "drawer-header-actions")], |act| {
                        header_actions(act);
                        act.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("class", "btn-icon"),
                                ("data-admin-action", "close-drawer"),
                                ("data-drawer-target", id),
                                ("aria-label", close_label),
                            ],
                            |b| icon(b, "fa-times"),
                        );
                    });
                });
                dr.el("div", &[("class", "drawer-body")], body);
                dr.el("div", &[("class", "drawer-footer")], footer);
            });
        },
    );
}

pub fn subtab_bar(buf: &mut HtmlBuf, group: &str, tabs: &[(&str, &str, Option<&str>)]) {
    buf.el(
        "div",
        &[("class", "subtab-bar"), ("data-subtab-group", group), ("role", "tablist")],
        |bar| {
            for (idx, (sub_id, label, perm)) in tabs.iter().enumerate() {
                let active = idx == 0;
                bar.el_with(
                    "button",
                    |a| {
                        a.attr("type", "button")
                            .attr("class", if active { "subtab-btn active" } else { "subtab-btn" })
                            .attr("data-subtab", sub_id)
                            .attr("data-subtab-target", sub_id)
                            .attr("aria-controls", sub_id)
                            .attr("role", "tab")
                            .attr("aria-selected", if active { "true" } else { "false" });
                        if let Some(p) = perm {
                            a.attr("data-required-perm", p);
                        }
                    },
                    |b| {
                        b.text(label);
                    },
                );
            }
        },
    );
}

pub fn subtab_pane<F: FnOnce(&mut HtmlBuf)>(
    buf: &mut HtmlBuf,
    id: &str,
    active: bool,
    perm: Option<&str>,
    body: F,
) {
    let cls = if active { "subtab-pane active" } else { "subtab-pane hidden" };
    buf.el_with(
        "div",
        |a| {
            a.attr("id", id).attr("class", cls);
            if let Some(p) = perm {
                a.attr("data-required-perm", p);
            }
        },
        body,
    );
}

pub fn lang_tabs<Tr: FnOnce(&mut HtmlBuf), En: FnOnce(&mut HtmlBuf)>(
    buf: &mut HtmlBuf,
    tr_label: &str,
    en_label: &str,
    tr_pane: Tr,
    en_pane: En,
) {
    buf.el("div", &[("class", "lang-tabs"), ("role", "tablist")], |bar| {
        bar.el(
            "button",
            &[("type", "button"), ("class", "lang-tab active"), ("data-lang-tab", "tr"), ("aria-selected", "true")],
            |b| { b.text(tr_label); },
        );
        bar.el(
            "button",
            &[("type", "button"), ("class", "lang-tab"), ("data-lang-tab", "en"), ("aria-selected", "false")],
            |b| { b.text(en_label); },
        );
    });
    buf.el("div", &[("class", "lang-pane active"), ("data-lang-pane", "tr")], tr_pane);
    buf.el("div", &[("class", "lang-pane hidden"), ("data-lang-pane", "en")], en_pane);
}

pub fn details_box<F: FnOnce(&mut HtmlBuf)>(
    buf: &mut HtmlBuf,
    extra_cls: &str,
    open: bool,
    perm: Option<&str>,
    summary: &str,
    body: F,
) {
    let cls = join_cls("form-details", extra_cls);
    buf.el_with(
        "details",
        |a| {
            a.attr("class", &cls).bool_attr("open", open);
            if let Some(p) = perm {
                a.attr("data-required-perm", p);
            }
        },
        |d| {
            d.el("summary", &[], |s| {
                s.text(summary);
            });
            body(d);
        },
    );
}

pub fn form_group<F: FnOnce(&mut HtmlBuf)>(
    buf: &mut HtmlBuf,
    for_id: Option<&str>,
    label: &str,
    ctrl: F,
) {
    buf.el("div", &[("class", "form-group")], |g| {
        if !label.is_empty() {
            g.el_with(
                "label",
                |a| {
                    a.attr("class", "form-label");
                    if let Some(fid) = for_id {
                        a.attr("for", fid);
                    }
                },
                |l| {
                    l.text(label);
                },
            );
        }
        ctrl(g);
    });
}

pub fn input_ctrl(
    buf: &mut HtmlBuf,
    kind: &str,
    id: &str,
    extra_cls: &str,
    value: &str,
    placeholder: &str,
    extra: &[(&str, &str)],
) {
    let cls = join_cls("form-control", extra_cls);
    buf.void_with("input", |a| {
        a.attr("type", kind).attr("id", id).attr("class", &cls);
        if !value.is_empty() {
            a.attr("value", value);
        }
        if !placeholder.is_empty() {
            a.attr("placeholder", placeholder);
        }
        for (k, v) in extra {
            a.attr(k, v);
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub fn upload_field(
    buf: &mut HtmlBuf,
    text_id: &str,
    val: &str,
    ph: &str,
    file_id: &str,
    accept: &str,
    action: &str,
    fa: &str,
    btn_label: &str,
) {
    buf.el("div", &[("class", "flex gap-2 items-center")], |row| {
        input_ctrl(row, "text", text_id, "", val, ph, &[]);
        row.void_el("input", &[("type", "file"), ("id", file_id), ("accept", accept), ("class", "hidden")]);
        action_btn(row, "btn-cmd btn-cmd-sm", action, None, Some(fa), btn_label);
    });
}

pub fn select_ctrl(
    buf: &mut HtmlBuf,
    id: &str,
    extra_cls: &str,
    options: &[(&str, &str)],
    extra: &[(&str, &str)],
) {
    let cls = join_cls("form-select", extra_cls);
    buf.el_with(
        "select",
        |a| {
            a.attr("id", id).attr("class", &cls);
            for (k, v) in extra {
                a.attr(k, v);
            }
        },
        |sel| {
            for (val, lbl) in options {
                sel.el("option", &[("value", val)], |o| {
                    o.text(lbl);
                });
            }
        },
    );
}

pub fn textarea_ctrl(
    buf: &mut HtmlBuf,
    id: &str,
    extra_cls: &str,
    placeholder: &str,
    extra: &[(&str, &str)],
) {
    let cls = join_cls("form-textarea", extra_cls);
    buf.el_with(
        "textarea",
        |a| {
            a.attr("id", id).attr("class", &cls);
            if !placeholder.is_empty() {
                a.attr("placeholder", placeholder);
            }
            for (k, v) in extra {
                a.attr(k, v);
            }
        },
        |_| {},
    );
}

pub fn check_pill(
    buf: &mut HtmlBuf,
    id: Option<&str>,
    name: Option<&str>,
    value: Option<&str>,
    checked: bool,
    extra_cls: &str,
    label: &str,
) {
    let cls = join_cls("check-pill", extra_cls);
    buf.el("label", &[("class", &cls)], |lbl| {
        lbl.void_with("input", |a| {
            a.attr("type", "checkbox");
            if let Some(cid) = id {
                a.attr("id", cid);
            }
            if let Some(cname) = name {
                a.attr("name", cname);
            }
            if let Some(cval) = value {
                a.attr("value", cval);
            }
            a.bool_attr("checked", checked);
        });
        lbl.el("span", &[], |s| {
            s.text(label);
        });
    });
}

pub fn check_matrix(
    buf: &mut HtmlBuf,
    id: &str,
    name: &str,
    items: &[(&str, &str)],
    default_checked: &[&str],
) {
    check_matrix_with_attrs(buf, id, name, items, default_checked, &[]);
}

pub fn check_matrix_with_attrs(
    buf: &mut HtmlBuf,
    id: &str,
    name: &str,
    items: &[(&str, &str)],
    default_checked: &[&str],
    extra: &[(&str, &str)],
) {
    buf.el_with(
        "div",
        |a| {
            a.attr("class", "checkbox-matrix").attr("id", id);
            for (k, v) in extra {
                a.attr(k, v);
            }
        },
        |m| {
            for (val, lbl) in items {
                check_pill(m, None, Some(name), Some(val), default_checked.contains(val), "", lbl);
            }
        },
    );
}

#[allow(clippy::too_many_arguments)]
pub fn kpi_card(
    buf: &mut HtmlBuf,
    perm: Option<&str>,
    label: &str,
    val_id: &str,
    default_val: &str,
    val_cls_extra: &str,
    sub_id: Option<&str>,
    sub_text: Option<&str>,
) {
    kpi_card_toned(buf, "", None, perm, label, val_id, default_val, val_cls_extra, sub_id, sub_text);
}

#[allow(clippy::too_many_arguments)]
pub fn kpi_card_toned(
    buf: &mut HtmlBuf,
    tone_cls: &str,
    fa_icon: Option<&str>,
    perm: Option<&str>,
    label: &str,
    val_id: &str,
    default_val: &str,
    val_cls_extra: &str,
    sub_id: Option<&str>,
    sub_text: Option<&str>,
) {
    let card_cls = join_cls("kpi-card", tone_cls);
    let val_cls = join_cls("kpi-value tabular-nums", val_cls_extra);
    buf.el_with(
        "div",
        |a| {
            a.attr("class", &card_cls).attr("data-glow-card", "");
            if let Some(p) = perm {
                a.attr("data-required-perm", p);
            }
        },
        |c| {
            c.el("div", &[("class", "kpi-label")], |l| {
                l.el("span", &[], |s| {
                    s.text(label);
                });
                if let Some(ic) = fa_icon {
                    icon(l, ic);
                }
            });
            c.el("div", &[("class", &val_cls), ("id", val_id)], |v| {
                v.text(default_val);
            });
            if sub_id.is_some() || sub_text.is_some() {
                c.el_with(
                    "div",
                    |a| {
                        a.attr("class", "kpi-sub");
                        if let Some(sid) = sub_id {
                            a.attr("id", sid);
                        }
                    },
                    |s| {
                        if let Some(txt) = sub_text {
                            s.text(txt);
                        }
                    },
                );
            }
        },
    );
}

pub fn ops_table(
    buf: &mut HtmlBuf,
    wrap_cls: &str,
    table_id: Option<&str>,
    cols: &[(&str, &str)],
    tbody_id: &str,
    loading_msg: Option<&str>,
    pagination_id: Option<&str>,
) {
    let wcls = join_cls("table-responsive", wrap_cls);
    buf.el("div", &[("class", &wcls)], |wrap| {
        wrap.el_with(
            "table",
            |a| {
                a.attr("class", "ops-table");
                if let Some(tid) = table_id {
                    a.attr("id", tid);
                }
            },
            |tbl| {
                tbl.el("thead", &[], |th_w| {
                    th_w.el("tr", &[], |tr| {
                        for (col_lbl, col_cls) in cols {
                            tr.el_with(
                                "th",
                                |a| {
                                    if !col_cls.is_empty() {
                                        a.attr("class", col_cls);
                                    }
                                },
                                |th| {
                                    th.text(col_lbl);
                                },
                            );
                        }
                    });
                });
                tbl.el("tbody", &[("id", tbody_id)], |tb| {
                    if let Some(msg) = loading_msg {
                        tb.el("tr", &[], |tr| {
                            tr.el(
                                "td",
                                &[("colspan", &cols.len().to_string()), ("class", "empty-state text-muted")],
                                |td| {
                                    td.text(msg);
                                },
                            );
                        });
                    }
                });
            },
        );
    });
    if let Some(pid) = pagination_id {
        buf.el("div", &[("id", pid)], |_| {});
    }
}
