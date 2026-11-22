use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::{MagazineIssue, MagazinePageSpread};
use crate::services::edergi_pdf_service::codec::extract_pdf_page_count;
use crate::services::edergi_pdf_service::watermark::sanitize_pdf_text;

#[derive(Clone, Debug)]
pub struct ConversionResult {
    pub publication_type: String,
    pub issue_number: i32,
    pub page_count: i32,
    pub cached_pages_count: usize,
    pub spreads: Vec<MagazinePageSpread>,
}

/// Validates that `raw` is a safe `.pdf` filename (no directory traversal).
pub fn safe_pdf_filename(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return None;
    }
    let name = Path::new(trimmed).file_name()?.to_str()?;
    if name != trimmed || !name.to_ascii_lowercase().ends_with(".pdf") {
        return None;
    }
    Some(name.to_string())
}

/// Returns `{edergi_dir}/.pages/{pub_type}_{issue_number}` safely.
pub fn issue_pages_cache_dir(edergi_dir: &str, publication_type: &str, issue_number: i32) -> PathBuf {
    let pub_slug = if publication_type.trim().eq_ignore_ascii_case("msi_tdr") {
        "msi_tdr"
    } else {
        "msi"
    };
    Path::new(edergi_dir)
        .join(".pages")
        .join(format!("{}_{}", pub_slug, issue_number))
}

/// Resolves an existing cached page JPEG for `page_num` (1-based), supporting canonical
/// `page-0001.jpg` as well as poppler's variable zero-padding (`page-1.jpg`, `page-01.jpg`, etc.).
pub fn resolve_cached_page_path(
    edergi_dir: &str,
    publication_type: &str,
    issue_number: i32,
    page_num: usize,
) -> Option<PathBuf> {
    let dir = issue_pages_cache_dir(edergi_dir, publication_type, issue_number);
    if !dir.is_dir() {
        return None;
    }
    let canonical = dir.join(format!("page-{:04}.jpg", page_num));
    if canonical.is_file() {
        return Some(canonical);
    }
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Some(num_str) = stem.strip_prefix("page-") {
                    if num_str.parse::<usize>().ok() == Some(page_num)
                        && path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg")).unwrap_or(false)
                    {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

/// Counts cached JPEG pages inside `{edergi_dir}/.pages/{pub_type}_{issue_number}`.
pub fn count_cached_pages(edergi_dir: &str, publication_type: &str, issue_number: i32) -> usize {
    let dir = issue_pages_cache_dir(edergi_dir, publication_type, issue_number);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| {
            let p = e.path();
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("page-") && n.to_ascii_lowercase().ends_with(".jpg"))
                    .unwrap_or(false)
        })
        .count()
}

/// Removes cached page renders for an issue (e.g. on issue delete or before re-conversion).
pub fn clear_issue_pages_cache(edergi_dir: &str, publication_type: &str, issue_number: i32) {
    let dir = issue_pages_cache_dir(edergi_dir, publication_type, issue_number);
    if dir.exists() {
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Detects the physical page count of an uploaded PDF via `pdfinfo`, falling back to pure-Rust codec.
pub fn detect_pdf_page_count(pdf_path: &Path, pdf_bytes: &[u8]) -> Option<i32> {
    if let Ok(out) = Command::new("pdfinfo").arg(pdf_path).output() {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Some(rest) = line.strip_prefix("Pages:") {
                    if let Ok(cnt) = rest.trim().parse::<i32>() {
                        if cnt > 0 {
                            return Some(cnt);
                        }
                    }
                }
            }
        }
    }
    extract_pdf_page_count(pdf_bytes).filter(|&c| c > 0)
}

/// One-click Print-PDF -> Web E-Dergi conversion pipeline (blocking; call via `spawn_blocking`).
pub fn convert_issue_pdf(
    edergi_dir: &str,
    issue: &MagazineIssue,
    pdf_filename: &str,
) -> Result<ConversionResult, String> {
    let safe_name = safe_pdf_filename(pdf_filename)
        .ok_or_else(|| "Geçersiz PDF dosya adı.".to_string())?;
    let pdf_path = Path::new(edergi_dir).join(&safe_name);
    let pdf_bytes = std::fs::read(&pdf_path)
        .map_err(|e| format!("PDF dosyası okunamadı ({}): {}", safe_name, e))?;
    if !pdf_bytes.starts_with(b"%PDF-") {
        return Err("Dosya geçerli bir PDF belgesi değil.".to_string());
    }

    let pub_type = if issue.publication_type.trim().eq_ignore_ascii_case("msi_tdr") {
        "msi_tdr".to_string()
    } else {
        "msi".to_string()
    };

    let page_count = detect_pdf_page_count(&pdf_path, &pdf_bytes)
        .ok_or_else(|| "PDF sayfa sayısı belirlenemedi.".to_string())?;

    // Extract TOC / spreads via pdftohtml XML -> pdftotext -> synthetic fallback
    let spreads = extract_spreads_from_pdf(&pdf_path, issue, &pub_type, page_count);

    // Pre-cache unwatermarked base JPEG pages via pdftoppm (skip gracefully on synthetic PDFs)
    clear_issue_pages_cache(edergi_dir, &pub_type, issue.issue_number);
    let cache_dir = issue_pages_cache_dir(edergi_dir, &pub_type, issue.issue_number);
    let cached_pages_count = if std::fs::create_dir_all(&cache_dir).is_ok() {
        let prefix = cache_dir.join("page");
        match Command::new("pdftoppm")
            .args([
                "-jpeg",
                "-jpegopt",
                "quality=88",
                "-r",
                "110",
            ])
            .arg(&pdf_path)
            .arg(&prefix)
            .output()
        {
            Ok(out) if out.status.success() => normalize_cached_page_filenames(&cache_dir),
            _ => 0,
        }
    } else {
        0
    };

    Ok(ConversionResult {
        publication_type: pub_type,
        issue_number: issue.issue_number,
        page_count,
        cached_pages_count,
        spreads,
    })
}

/// Renames `page-1.jpg` / `page-01.jpg` / `page-001.jpg` to deterministic `page-0001.jpg`.
fn normalize_cached_page_filenames(cache_dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return 0;
    };
    let mut count = 0usize;
    let mut renames: Vec<(PathBuf, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if let Some(stem) = name.strip_suffix(".jpg").or_else(|| name.strip_suffix(".JPG")) {
            if let Some(num_str) = stem.strip_prefix("page-") {
                if let Ok(num) = num_str.parse::<usize>() {
                    if num >= 1 {
                        let canonical = cache_dir.join(format!("page-{:04}.jpg", num));
                        if path != canonical {
                            renames.push((path, canonical));
                        } else {
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    for (src, dst) in renames {
        if std::fs::rename(&src, &dst).is_ok() {
            count += 1;
        }
    }
    count
}

/// Lazily renders a single page JPEG via `pdftoppm -singlefile` into the issue cache directory.
pub fn ensure_single_page_jpeg(
    edergi_dir: &str,
    publication_type: &str,
    issue_number: i32,
    pdf_filename: &str,
    page_num: usize,
) -> Option<PathBuf> {
    if let Some(existing) = resolve_cached_page_path(edergi_dir, publication_type, issue_number, page_num) {
        return Some(existing);
    }
    let safe_name = safe_pdf_filename(pdf_filename)?;
    let pdf_path = Path::new(edergi_dir).join(safe_name);
    if !pdf_path.is_file() {
        return None;
    }
    let cache_dir = issue_pages_cache_dir(edergi_dir, publication_type, issue_number);
    std::fs::create_dir_all(&cache_dir).ok()?;
    let target_stem = cache_dir.join(format!("page-{:04}", page_num));
    let target_jpg = cache_dir.join(format!("page-{:04}.jpg", page_num));
    let page_s = page_num.to_string();
    let out = Command::new("pdftoppm")
        .args([
            "-jpeg",
            "-jpegopt",
            "quality=88",
            "-r",
            "110",
            "-f",
            &page_s,
            "-l",
            &page_s,
            "-singlefile",
        ])
        .arg(&pdf_path)
        .arg(&target_stem)
        .output()
        .ok()?;
    if out.status.success() && target_jpg.is_file() {
        Some(target_jpg)
    } else {
        None
    }
}

/// Sanitizes arbitrary user/org metadata before passing to ImageMagick `-annotate`.
/// Neutralizes `%` format specifiers, `@` file-inclusion directives, `\`, quotes, and control chars.
pub fn sanitize_imagemagick_text(input: &str) -> String {
    let ascii = sanitize_pdf_text(input);
    let mut out = String::with_capacity(ascii.len());
    for ch in ascii.chars() {
        match ch {
            '%' => out.push_str("pct"),
            '@' => out.push_str("(at)"),
            '\\' | '"' | '\'' | '`' | '$' | ';' => out.push(' '),
            c if c.is_ascii_control() => out.push(' '),
            c => out.push(c),
        }
    }
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_utf8(&collapsed, 140)
}

/// Renders a cached JPEG page into a DRM-watermarked WebP byte buffer using ImageMagick `convert`.
pub fn render_watermarked_webp(
    cached_jpg_path: &Path,
    diag_text: &str,
    footer_text: &str,
) -> Option<Vec<u8>> {
    let safe_diag = sanitize_imagemagick_text(diag_text);
    let safe_footer = sanitize_imagemagick_text(footer_text);
    let diag_arg = if safe_diag.is_empty() {
        "MSI E-DERGI DRM".to_string()
    } else {
        safe_diag
    };
    let footer_arg = if safe_footer.is_empty() {
        "MSI DERGISI DRM LISANSLI KOPYA".to_string()
    } else {
        safe_footer
    };

    let out = Command::new("convert")
        .arg(cached_jpg_path)
        .args([
            "-fill",
            "rgba(180,20,30,0.22)",
            "-pointsize",
            "20",
            "-gravity",
            "Center",
            "-annotate",
            "-30x-30+0+0",
            &diag_arg,
            "-gravity",
            "South",
            "-fill",
            "rgba(15,23,42,0.88)",
            "-draw",
            "rectangle 0,0 10000,34",
            "-fill",
            "#f5dc84",
            "-pointsize",
            "12",
            "-annotate",
            "+0+10",
            &footer_arg,
            "-quality",
            "82",
            "webp:-",
        ])
        .output()
        .ok()?;

    if out.status.success() && out.stdout.len() >= 12 && &out.stdout[..4] == b"RIFF" && &out.stdout[8..12] == b"WEBP" {
        Some(out.stdout)
    } else {
        None
    }
}

/// Escapes XML special characters and strips control characters for SVG output.
pub fn escape_xml_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if c.is_control() && c != ' ' => {}
            c => out.push(c),
        }
    }
    out
}

/// Pure-Rust SVG page renderer fallback when no raster PDF is available (e.g. synthetic test PDFs).
pub fn render_fallback_page_svg(
    issue: &MagazineIssue,
    page_num: usize,
    total_pages: usize,
    diag_text: &str,
    footer_text: &str,
) -> Vec<u8> {
    let pub_label = if issue.publication_type.eq_ignore_ascii_case("msi_tdr") {
        "MSI TDR"
    } else {
        "MSI DERGISI"
    };
    let spread_opt = issue.spreads.iter().find(|sp| {
        sp.page_no
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|p| p.parse::<usize>().ok())
            .any(|n| n == page_num)
    }).or_else(|| {
        if page_num >= 2 {
            issue.spreads.get(page_num - 2)
        } else {
            issue.spreads.first()
        }
    });

    let heading = spread_opt
        .map(|s| {
            if issue.publication_type.eq_ignore_ascii_case("msi_tdr") && !s.title_en.trim().is_empty() {
                s.title_en.trim().to_string()
            } else {
                s.title_tr.trim().to_string()
            }
        })
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| issue.title_tr.clone());

    let badge = spread_opt
        .map(|s| s.category_badge.trim().to_string())
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| format!("SAYFA {}", page_num));

    let excerpt = spread_opt
        .map(|s| {
            if issue.publication_type.eq_ignore_ascii_case("msi_tdr") && !s.excerpt_en.trim().is_empty() {
                s.excerpt_en.trim().to_string()
            } else {
                s.excerpt_tr.trim().to_string()
            }
        })
        .filter(|e| !e.is_empty())
        .unwrap_or_else(|| issue.summary_tr.clone());

    let wrapped_heading = wrap_svg_lines(&heading, 46, 3);
    let wrapped_excerpt = wrap_svg_lines(&excerpt, 64, 8);

    let mut heading_tspans = String::new();
    for (idx, line) in wrapped_heading.iter().enumerate() {
        heading_tspans.push_str(&format!(
            r##"<text x="48" y="{}" font-family="Georgia,serif" font-size="22" font-weight="700" fill="#0f172a">{}</text>"##,
            175 + idx * 30,
            escape_xml_text(line)
        ));
    }

    let excerpt_start_y = 195 + wrapped_heading.len() * 30;
    let mut excerpt_tspans = String::new();
    for (idx, line) in wrapped_excerpt.iter().enumerate() {
        excerpt_tspans.push_str(&format!(
            r##"<text x="48" y="{}" font-family="system-ui,sans-serif" font-size="14" fill="#334155">{}</text>"##,
            excerpt_start_y + idx * 24,
            escape_xml_text(line)
        ));
    }

    let safe_diag = escape_xml_text(&truncate_utf8(diag_text, 72));
    let safe_footer = escape_xml_text(&truncate_utf8(footer_text, 96));

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 595 842" width="595" height="842">
  <rect width="595" height="842" fill="#fcfbf8"/>
  <rect x="0" y="0" width="595" height="68" fill="#0f172a"/>
  <rect x="0" y="68" width="595" height="3" fill="#b91c1c"/>
  <text x="48" y="34" font-family="system-ui,sans-serif" font-size="13" font-weight="700" fill="#f8fafc">{pub_label} · #{issue_no}</text>
  <text x="48" y="53" font-family="system-ui,sans-serif" font-size="11" fill="#cbd5e1">{issue_title}</text>
  <text x="547" y="42" text-anchor="end" font-family="monospace" font-size="13" font-weight="700" fill="#f5dc84">{page_num} / {total_pages}</text>
  <text x="48" y="132" font-family="system-ui,sans-serif" font-size="11" font-weight="700" letter-spacing="1.2" fill="#b91c1c">{badge}</text>
  {heading_tspans}
  <line x1="48" y1="{rule_y}" x2="547" y2="{rule_y}" stroke="#e2e8f0" stroke-width="1"/>
  {excerpt_tspans}
  <g transform="translate(297 440) rotate(-32)" opacity="0.18">
    <text x="0" y="0" text-anchor="middle" font-family="monospace" font-size="16" font-weight="700" fill="#991b1b">{safe_diag}</text>
  </g>
  <rect x="0" y="796" width="595" height="46" fill="#0f172a"/>
  <rect x="0" y="794" width="595" height="2" fill="#d4af37"/>
  <text x="297" y="823" text-anchor="middle" font-family="monospace" font-size="10.5" fill="#f5dc84">{safe_footer}</text>
</svg>"##,
        pub_label = escape_xml_text(pub_label),
        issue_no = issue.issue_number,
        issue_title = escape_xml_text(&truncate_utf8(&issue.title_tr, 60)),
        page_num = page_num,
        total_pages = total_pages,
        badge = escape_xml_text(&badge),
        heading_tspans = heading_tspans,
        rule_y = excerpt_start_y - 18,
        excerpt_tspans = excerpt_tspans,
        safe_diag = safe_diag,
        safe_footer = safe_footer,
    );

    svg.into_bytes()
}

fn wrap_svg_lines(text: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let cur_len = current.chars().count();
        let word_len = word.chars().count();
        if current.is_empty() {
            current.push_str(word);
        } else if cur_len + 1 + word_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            if lines.len() >= max_lines {
                return lines;
            }
            current = word.to_string();
        }
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    lines
}

// ============================================================================
// TOC / Headings / Spread Extraction (pdftohtml XML -> pdftotext -> fallback)
// ============================================================================

struct XmlTextItem {
    font_id: String,
    top: i32,
    is_bold: bool,
    text: String,
}

fn extract_spreads_from_pdf(
    pdf_path: &Path,
    issue: &MagazineIssue,
    pub_type: &str,
    page_count: i32,
) -> Vec<MagazinePageSpread> {
    if let Ok(out) = Command::new("pdftohtml")
        .args(["-xml", "-i", "-stdout"])
        .arg(pdf_path)
        .output()
    {
        if out.status.success() && !out.stdout.is_empty() {
            let xml = String::from_utf8_lossy(&out.stdout);
            let parsed = parse_pdftohtml_xml(&xml, issue, pub_type, page_count);
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }

    if let Ok(out) = Command::new("pdftotext").arg(pdf_path).arg("-").output() {
        if out.status.success() && !out.stdout.is_empty() {
            let raw = String::from_utf8_lossy(&out.stdout);
            let parsed = parse_pdftotext_pages(&raw, issue, pub_type, page_count);
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }

    build_fallback_spreads(issue, pub_type, page_count)
}

fn parse_pdftohtml_xml(
    xml: &str,
    issue: &MagazineIssue,
    pub_type: &str,
    page_count: i32,
) -> Vec<MagazinePageSpread> {
    let font_re = regex::Regex::new(r#"<fontspec\b[^>]*\bid="([^"]+)"[^>]*\bsize="([0-9.]+)"#).ok();
    let page_re = regex::Regex::new(r#"(?s)<page\b[^>]*\bnumber="(\d+)"[^>]*>(.*?)</page>"#).ok();
    let text_re = regex::Regex::new(
        r#"(?s)<text\b[^>]*\btop="(-?\d+)"[^>]*\bfont="([^"]+)"[^>]*>(.*?)</text>"#,
    )
    .ok();
    let outline_re = regex::Regex::new(r#"(?s)<outline\b[^>]*>(.*?)</outline>"#).ok();
    let item_re = regex::Regex::new(r#"(?s)<item\b([^>]*)>(.*?)</item>"#).ok();
    let page_attr_re = regex::Regex::new(r#"\bpage="(\d+)""#).ok();

    let mut font_sizes: HashMap<String, f64> = HashMap::new();
    if let Some(ref re) = font_re {
        for cap in re.captures_iter(xml) {
            if let Ok(sz) = cap[2].parse::<f64>() {
                font_sizes.insert(cap[1].to_string(), sz);
            }
        }
    }

    let mut pages_items: BTreeMap<usize, Vec<XmlTextItem>> = BTreeMap::new();
    let mut line_freq: HashMap<String, usize> = HashMap::new();

    if let (Some(ref p_re), Some(ref t_re)) = (page_re, text_re) {
        for pcap in p_re.captures_iter(xml) {
            let Ok(pnum) = pcap[1].parse::<usize>() else {
                continue;
            };
            if pnum == 0 || pnum > page_count as usize {
                continue;
            }
            let inner = &pcap[2];
            let mut items = Vec::new();
            let mut seen_on_page = HashSet::new();
            for tcap in t_re.captures_iter(inner) {
                let top = tcap[1].parse::<i32>().unwrap_or(0);
                let font_id = tcap[2].to_string();
                let raw_inner = &tcap[3];
                let is_bold = raw_inner.contains("<b>") || raw_inner.contains("<B>");
                let clean = clean_xml_fragment(raw_inner);
                if clean.is_empty() {
                    continue;
                }
                let norm = clean.to_lowercase();
                if seen_on_page.insert(norm.clone()) {
                    *line_freq.entry(norm).or_insert(0) += 1;
                }
                items.push(XmlTextItem {
                    font_id,
                    top,
                    is_bold,
                    text: clean,
                });
            }
            if !items.is_empty() {
                pages_items.insert(pnum, items);
            }
        }
    }

    // 1. Authoritative <outline> bookmarks if present
    let mut outline_entries: Vec<(usize, String)> = Vec::new();
    if let (Some(ref o_re), Some(ref i_re), Some(ref pa_re)) = (outline_re, item_re, page_attr_re) {
        if let Some(ocap) = o_re.captures(xml) {
            let obody = &ocap[1];
            for icap in i_re.captures_iter(obody) {
                let attrs = &icap[1];
                // Strip any nested <item> tags from title capture if nested
                let raw_body = icap[2].split("<item").next().unwrap_or(&icap[2]);
                let title = clean_xml_fragment(raw_body);
                if title.len() < 3 {
                    continue;
                }
                if let Some(pacap) = pa_re.captures(attrs) {
                    if let Ok(pnum) = pacap[1].parse::<usize>() {
                        if pnum >= 1 && pnum <= page_count as usize {
                            outline_entries.push((pnum, title));
                        }
                    }
                }
            }
        }
    }

    if !outline_entries.is_empty() {
        let mut spreads = Vec::new();
        let mut seen_pages = HashSet::new();
        for (pnum, title) in outline_entries.into_iter().take(60) {
            if !seen_pages.insert((pnum, title.clone())) {
                continue;
            }
            let excerpt = pages_items
                .get(&pnum)
                .map(|items| build_page_excerpt(items, &title, &line_freq, page_count as usize))
                .unwrap_or_default();
            spreads.push(make_spread(
                issue.issue_number,
                pub_type,
                pnum,
                &title,
                "DOSYA",
                &excerpt,
            ));
        }
        if !spreads.is_empty() {
            return spreads;
        }
    }

    if pages_items.is_empty() {
        return Vec::new();
    }

    // 2. Compute modal body font size weighted by character length
    let mut size_weights: HashMap<i64, usize> = HashMap::new();
    for items in pages_items.values() {
        for it in items {
            let sz = font_sizes.get(&it.font_id).copied().unwrap_or(11.0);
            let bucket = (sz * 2.0).round() as i64;
            *size_weights.entry(bucket).or_insert(0) += it.text.chars().count();
        }
    }
    let body_size = size_weights
        .into_iter()
        .max_by_key(|(_, w)| *w)
        .map(|(b, _)| (b as f64) / 2.0)
        .unwrap_or(11.0);

    let total_pages = page_count as usize;
    let mut heading_spreads = Vec::new();
    let mut fallback_spreads = Vec::new();

    for (pnum, items) in &pages_items {
        let mut best_heading: Option<(f64, i32, String)> = None;
        for it in items {
            if is_noise_line(&it.text, &line_freq, total_pages) {
                continue;
            }
            let char_len = it.text.chars().count();
            if !(4..=120).contains(&char_len) {
                continue;
            }
            let sz = font_sizes.get(&it.font_id).copied().unwrap_or(body_size);
            let is_large = sz >= body_size + 1.5;
            let is_bold_heading = it.is_bold && sz >= body_size - 0.5 && !it.text.ends_with('.');
            if is_large || is_bold_heading {
                let score = (sz - body_size) * 10.0 + if it.is_bold { 4.0 } else { 0.0 };
                let replace = match &best_heading {
                    None => true,
                    Some((best_score, best_top, _)) => {
                        score > *best_score + 0.5 || ((score - *best_score).abs() <= 0.5 && it.top < *best_top)
                    }
                };
                if replace {
                    best_heading = Some((score, it.top, it.text.clone()));
                }
            }
        }

        if let Some((_, _, heading)) = best_heading {
            let excerpt = build_page_excerpt(items, &heading, &line_freq, total_pages);
            let badge = if *pnum == 1 { "KAPAK" } else { "BÖLÜM" };
            heading_spreads.push(make_spread(
                issue.issue_number,
                pub_type,
                *pnum,
                &heading,
                badge,
                &excerpt,
            ));
        } else if fallback_spreads.len() < 24 {
            let first_line = items
                .iter()
                .find(|it| !is_noise_line(&it.text, &line_freq, total_pages) && it.text.chars().count() >= 4)
                .map(|it| it.text.clone())
                .unwrap_or_else(|| format!("Sayfa {}", pnum));
            let excerpt = build_page_excerpt(items, &first_line, &line_freq, total_pages);
            fallback_spreads.push(make_spread(
                issue.issue_number,
                pub_type,
                *pnum,
                &truncate_utf8(&first_line, 96),
                "SAYFA",
                &excerpt,
            ));
        }
    }

    if !heading_spreads.is_empty() {
        heading_spreads.truncate(60);
        heading_spreads
    } else {
        fallback_spreads
    }
}

fn parse_pdftotext_pages(
    raw: &str,
    issue: &MagazineIssue,
    pub_type: &str,
    page_count: i32,
) -> Vec<MagazinePageSpread> {
    let mut spreads = Vec::new();
    for (idx, chunk) in raw.split('\x0c').enumerate() {
        let pnum = idx + 1;
        if pnum > page_count as usize || spreads.len() >= 40 {
            break;
        }
        let lines: Vec<String> = chunk
            .lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|l| l.chars().count() >= 4)
            .collect();
        if lines.is_empty() {
            continue;
        }
        let title = truncate_utf8(&lines[0], 96);
        let excerpt_raw = lines.iter().skip(1).take(4).cloned().collect::<Vec<_>>().join(" ");
        let excerpt = truncate_utf8(&excerpt_raw, 220);
        spreads.push(make_spread(
            issue.issue_number,
            pub_type,
            pnum,
            &title,
            if pnum == 1 { "KAPAK" } else { "SAYFA" },
            &excerpt,
        ));
    }
    spreads
}

fn build_fallback_spreads(
    issue: &MagazineIssue,
    pub_type: &str,
    page_count: i32,
) -> Vec<MagazinePageSpread> {
    let total = page_count.max(1) as usize;
    let count = total.min(12);
    let mut spreads = Vec::with_capacity(count);
    for i in 0..count {
        let pnum = if count == total {
            i + 1
        } else {
            ((i * (total - 1)) / (count - 1).max(1)) + 1
        };
        let title = if pnum == 1 {
            issue.title_tr.clone()
        } else if pub_type == "msi_tdr" {
            format!("{} — Page {}", issue.title_en, pnum)
        } else {
            format!("{} — Sayfa {}", issue.title_tr, pnum)
        };
        spreads.push(make_spread(
            issue.issue_number,
            pub_type,
            pnum,
            &title,
            if pnum == 1 { "KAPAK" } else { "SAYFA" },
            &truncate_utf8(&issue.summary_tr, 200),
        ));
    }
    spreads
}

fn make_spread(
    issue_number: i32,
    pub_type: &str,
    page_num: usize,
    title: &str,
    badge: &str,
    excerpt: &str,
) -> MagazinePageSpread {
    let clean_title = truncate_utf8(title.trim(), 120);
    let clean_excerpt = if excerpt.trim().eq_ignore_ascii_case(&clean_title) {
        String::new()
    } else {
        truncate_utf8(excerpt.trim(), 240)
    };
    let is_tdr = pub_type.eq_ignore_ascii_case("msi_tdr");
    MagazinePageSpread {
        page_no: format!("S. {}", page_num),
        title_tr: clean_title.clone(),
        title_en: if is_tdr { clean_title } else { String::new() },
        category_badge: badge.to_string(),
        image_url: format!(
            "/api/v1/edergi/{}/pages/{}?publication_type={}",
            issue_number, page_num, pub_type
        ),
        excerpt_tr: clean_excerpt.clone(),
        excerpt_en: if is_tdr { clean_excerpt } else { String::new() },
        linked_content_slug: None,
    }
}

fn build_page_excerpt(
    items: &[XmlTextItem],
    heading: &str,
    line_freq: &HashMap<String, usize>,
    total_pages: usize,
) -> String {
    let heading_lower = heading.trim().to_lowercase();
    let mut parts = Vec::new();
    let mut total_chars = 0usize;
    for it in items {
        if is_noise_line(&it.text, line_freq, total_pages) {
            continue;
        }
        if it.text.trim().to_lowercase() == heading_lower {
            continue;
        }
        if it.text.chars().count() < 4 {
            continue;
        }
        let len = it.text.chars().count();
        parts.push(it.text.as_str());
        total_chars += len + 1;
        if total_chars >= 220 {
            break;
        }
    }
    truncate_utf8(&parts.join(" "), 220)
}

fn is_noise_line(line: &str, line_freq: &HashMap<String, usize>, total_pages: usize) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '/' || c == '-') {
        return true;
    }
    let lower = trimmed.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.contains("issn ") {
        return true;
    }
    if total_pages >= 4 && line_freq.get(&lower).copied().unwrap_or(0) >= 3 && trimmed.chars().count() < 80 {
        return true;
    }
    false
}

fn clean_xml_fragment(raw: &str) -> String {
    let mut no_tags = String::with_capacity(raw.len());
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => no_tags.push(ch),
            _ => {}
        }
    }
    let decoded = decode_xml_entities(&no_tags);
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_xml_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        if let Some(semi) = after.find(';') {
            let entity = &after[..semi];
            if entity.len() <= 10 {
                match entity {
                    "amp" => out.push('&'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "quot" => out.push('"'),
                    "apos" => out.push('\''),
                    "nbsp" => out.push(' '),
                    _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                        if let Ok(code) = u32::from_str_radix(&entity[2..], 16) {
                            if let Some(c) = char::from_u32(code) {
                                out.push(c);
                            }
                        }
                    }
                    _ if entity.starts_with('#') => {
                        if let Ok(code) = entity[1..].parse::<u32>() {
                            if let Some(c) = char::from_u32(code) {
                                out.push(c);
                            }
                        }
                    }
                    _ => {
                        out.push('&');
                        out.push_str(entity);
                        out.push(';');
                    }
                }
                rest = &after[semi + 1..];
                continue;
            }
        }
        out.push('&');
        rest = after;
    }
    out.push_str(rest);
    out
}

fn truncate_utf8(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let collected: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", collected.trim_end())
    } else {
        collected
    }
}
