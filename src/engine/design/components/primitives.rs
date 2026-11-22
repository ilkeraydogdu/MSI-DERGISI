use super::super::config::safe_url;
use crate::engine::security_engine::escape_html as esc;

pub fn asset(path: &str) -> String {
    if path.starts_with("https://") {
        esc(path)
    } else {
        format!("/{}", esc(path.trim_start_matches('/')))
    }
}
pub fn link(url: &str) -> String {
    if safe_url(url) {
        esc(url)
    } else {
        "#".into()
    }
}
pub fn icon(name: &str) -> String {
    let path = match name {
        "search" => "<circle cx='10.5' cy='10.5' r='6.5'/><path d='m16 16 5 5'/>",
        "bell" => "<path d='M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4'/>",
        "moon" => "<path d='M21 13a9 9 0 0 1-10-10 9 9 0 1 0 10 10Z'/>",
        "menu" => "<path d='M3 6h18M3 12h18M3 18h18'/>",
        "user" => "<circle cx='12' cy='8' r='4'/><path d='M4 21v-2a8 8 0 0 1 16 0v2'/>",
        "arrow" => "<path d='M4 12h16m-6-6 6 6-6 6'/>",
        "close" => "<path d='m6 6 12 12M6 18 18 6'/>",
        "cart" => "<circle cx='9' cy='20' r='1.5'/><circle cx='17' cy='20' r='1.5'/><path d='M3 4h2l2.4 11.2a1 1 0 0 0 1 .8h9.8a1 1 0 0 0 1-.8L21 8H7'/>",
        _ => "<path d='M3 3h7l2 2 2-2h7v17h-7l-2 2-2-2H3Zm9 2v17'/>",
    };
    format!("<svg class=\"ui-icon\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.6\" aria-hidden=\"true\">{path}</svg>")
}

