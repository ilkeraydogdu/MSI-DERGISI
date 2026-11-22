//! Centralized zero-FOUC theme pre-hydration scripts for all platform surfaces.
//!
//! Resolution strategy:
//! 1. OS preference via `window.matchMedia("(prefers-color-scheme: dark|light)")`.
//! 2. Autonomous local-clock fallback (`19:00..06:59` -> `dark`, `07:00..18:59` -> `light`)
//!    when OS color-scheme detection is unavailable or reports no preference.
//! 3. On surfaces with an explicit manual theme toggle (`UNIFIED_THEME_PREHYDRATE_JS`),
//!    a valid stored manual choice (`msi_admin_theme` / `msi_theme`) overrides auto mode.

/// Autonomous-only pre-hydration for surfaces without a manual theme toggle (e.g., `/podmin` login).
/// Always follows OS dark/light preference, falling back dynamically to local clock hours.
pub const AUTO_ONLY_THEME_PREHYDRATE_JS: &str = r#"(function(){try{var d=window.matchMedia&&window.matchMedia("(prefers-color-scheme: dark)"),l=window.matchMedia&&window.matchMedia("(prefers-color-scheme: light)"),h=(new Date).getHours(),t=d&&d.matches?"dark":l&&l.matches?"light":(h>=19||h<7?"dark":"light");document.documentElement.setAttribute("data-theme",t);}catch(_){}})();"#;

/// Unified site-wide pre-hydration for public pages, account portal, command center,
/// design studio, and editorial studio. Honors explicit user toggle if stored; otherwise
/// runs autonomously from OS preference -> dynamic local clock fallback.
pub const UNIFIED_THEME_PREHYDRATE_JS: &str = r#"(function(){try{var m=document.cookie.match(/(?:^|;\s*)msi_admin_theme=([^;]+)/),c=m?decodeURIComponent(m[1]):"",s=localStorage.getItem("msi_admin_theme")||c||localStorage.getItem("msi_theme"),t=s==="dark"||s==="light"?s:"";if(!t){var d=window.matchMedia&&window.matchMedia("(prefers-color-scheme: dark)"),l=window.matchMedia&&window.matchMedia("(prefers-color-scheme: light)"),h=(new Date).getHours();t=d&&d.matches?"dark":l&&l.matches?"light":(h>=19||h<7?"dark":"light");}document.documentElement.setAttribute("data-theme",t);}catch(_){}})();"#;
