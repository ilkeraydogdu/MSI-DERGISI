//! E-Dergi PDF DRM Watermarking & Pure-Rust PDF Codec Service.

mod codec;
pub mod converter;
mod watermark;

#[cfg(test)]
mod tests;

pub use codec::{
    apply_visible_watermark_and_select_pages, apply_visible_watermark_and_slice,
    extract_pdf_page_count,
};
pub use converter::{
    clear_issue_pages_cache, convert_issue_pdf, count_cached_pages, detect_pdf_page_count,
    ensure_single_page_jpeg, issue_pages_cache_dir, render_fallback_page_svg,
    render_watermarked_webp, resolve_cached_page_path, safe_pdf_filename, ConversionResult,
};
pub use watermark::{
    generate_drm_watermark_code, generate_starter_archive_pdf, generate_watermarked_pdf,
    generate_watermarked_pdf_checked, parse_page_range, parse_page_selection, sanitize_pdf_text,
};
