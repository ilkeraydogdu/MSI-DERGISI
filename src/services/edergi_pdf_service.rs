use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::models::{MagazineIssue, MagazinePageSpread};

/// Generates a deterministic, tamper-evident DRM watermark hash code for an E-Dergi download.
pub fn generate_drm_watermark_code(
    issue_number: i32,
    user_name: &str,
    org_name: &str,
    client_ip: &str,
    timestamp: &str,
    secret: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"MSI-EDERGI-DRM-V2:");
    hasher.update(issue_number.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(user_name.trim().as_bytes());
    hasher.update(b"|");
    hasher.update(org_name.trim().as_bytes());
    hasher.update(b"|");
    hasher.update(client_ip.trim().as_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.trim().as_bytes());
    hasher.update(b"|");
    hasher.update(secret.as_bytes());
    let hex_digest = hex::encode(hasher.finalize()).to_uppercase();
    format!("MSI-DRM-{}", &hex_digest[..16])
}

/// Parses a `?pages=` query parameter (e.g., `"1-5"`, `"12"`, `" 4 - 2 "`) into an inclusive
/// 1-based page range `(start, end)`. Returns `None` for empty, `"all"`, or non-numeric values.
pub fn parse_page_range(raw: Option<&str>) -> Option<(usize, usize)> {
    let text = raw?.trim();
    if text.is_empty() || text.eq_ignore_ascii_case("all") {
        return None;
    }

    if let Some((start_s, end_s)) = text.split_once('-') {
        let s = start_s.trim().parse::<usize>().ok()?;
        let e = end_s.trim().parse::<usize>().ok()?;
        let lo = s.min(e).max(1);
        let hi = s.max(e).max(1);
        return Some((lo, hi));
    }

    if let Ok(single) = text.parse::<usize>() {
        let page = single.max(1);
        return Some((page, page));
    }

    None
}

/// Extracts the total page count from any PDF byte slice (classic PDF 1.4 `xref` tables as well
/// as PDF 1.5+ `/FlateDecode` `/ObjStm` and `/XRef` streams) using pure Rust without external crates.
pub fn extract_pdf_page_count(pdf_bytes: &[u8]) -> Option<i32> {
    pdf_codec::extract_page_count(pdf_bytes)
}

/// Applies a visible, semi-transparent diagonal watermark and bottom footer security stamp onto
/// every selected `/Type /Page` of an uploaded PDF, optionally slices `/Pages /Kids` + `/Count`
/// to `page_range`, updates `/Info` `/Subject` & `/Keywords`, appends `%MSI-DRM:{watermark_code}`,
/// and recalculates exact `xref` + `startxref` byte offsets.
pub fn apply_visible_watermark_and_slice(
    pdf_bytes: &[u8],
    watermark_text: &str,
    watermark_code: &str,
    page_range: Option<(usize, usize)>,
) -> Vec<u8> {
    pdf_codec::watermark_and_slice(pdf_bytes, watermark_text, watermark_code, page_range)
}

/// Generates a 100% standards-compliant, multi-page PDF 1.4 binary document in pure Rust
/// with exact `xref` byte offsets and dynamic DRM watermarks on every page.
pub fn generate_watermarked_pdf(
    issue: &MagazineIssue,
    user_name: &str,
    org_name: &str,
    license_label: &str,
    client_ip: &str,
    selected_pages: Option<&str>,
    secret: &str,
) -> Vec<u8> {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    let clean_user = sanitize_pdf_text(if user_name.trim().is_empty() {
        "Kayitli Okur"
    } else {
        user_name.trim()
    });
    let clean_org = sanitize_pdf_text(if org_name.trim().is_empty() {
        "Kurumsal Paydas"
    } else {
        org_name.trim()
    });
    let clean_license = sanitize_pdf_text(if license_label.trim().is_empty() {
        "ABONE E-DERGI LISANSI"
    } else {
        license_label.trim()
    });
    let clean_ip = sanitize_pdf_text(if client_ip.trim().is_empty() {
        "127.0.0.1"
    } else {
        client_ip.trim()
    });

    let drm_hash = generate_drm_watermark_code(
        issue.issue_number,
        &clean_user,
        &clean_org,
        &clean_ip,
        &timestamp,
        secret,
    );

    let full_watermark_line = format!(
        "{}: {} ({}) - TARIH: {} - IP: {} - DRM KODU: {}",
        clean_license, clean_user, clean_org, timestamp, clean_ip, drm_hash
    );

    let diagonal_watermark_primary = format!("{}: {} ({})", clean_license, clean_user, clean_org);
    let diagonal_watermark_secondary =
        format!("TARIH: {} - IP: {} - DRM KODU: {}", timestamp, clean_ip, drm_hash);

    let filtered_spreads = filter_spreads_by_selection(&issue.spreads, selected_pages);
    let selection_desc = selected_pages
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"))
        .map(|s| format!("Secili Sayfalar ({})", sanitize_pdf_text(s)))
        .unwrap_or_else(|| format!("Tam Sayi Arsiv Paketi ({} Sayfa)", issue.page_count));

    // Page 1: Cover Dossier + DRM Certificate + Table of Contents
    // Pages 2..=N: Detailed Spread Pages for each selected spread
    let mut page_streams: Vec<String> = Vec::new();
    let total_pages = 1 + filtered_spreads.len().max(1);

    page_streams.push(build_cover_page_stream(
        issue,
        &selection_desc,
        &full_watermark_line,
        &diagonal_watermark_primary,
        &diagonal_watermark_secondary,
        &drm_hash,
        &clean_user,
        &clean_org,
        &clean_license,
        &clean_ip,
        &timestamp,
        1,
        total_pages,
    ));

    if filtered_spreads.is_empty() {
        let fallback_spread = MagazinePageSpread {
            page_no: "01-16".to_string(),
            title_tr: issue.title_tr.clone(),
            title_en: issue.title_en.clone(),
            category_badge: "OZEL DOSYA".to_string(),
            image_url: issue.cover_image.clone(),
            excerpt_tr: issue.summary_tr.clone(),
            excerpt_en: issue.summary_en.clone(),
            linked_content_slug: None,
        };
        page_streams.push(build_spread_page_stream(
            issue,
            &fallback_spread,
            &full_watermark_line,
            &diagonal_watermark_primary,
            &diagonal_watermark_secondary,
            2,
            total_pages,
        ));
    } else {
        for (idx, spread) in filtered_spreads.iter().enumerate() {
            page_streams.push(build_spread_page_stream(
                issue,
                spread,
                &full_watermark_line,
                &diagonal_watermark_primary,
                &diagonal_watermark_secondary,
                idx + 2,
                total_pages,
            ));
        }
    }

    assemble_pdf_binary(
        &page_streams,
        issue.issue_number,
        &sanitize_pdf_text(&issue.title_tr),
        &full_watermark_line,
        &drm_hash,
    )
}

fn filter_spreads_by_selection(
    spreads: &[MagazinePageSpread],
    selected_pages: Option<&str>,
) -> Vec<MagazinePageSpread> {
    let raw = match selected_pages {
        Some(s) if !s.trim().is_empty() && !s.trim().eq_ignore_ascii_case("all") => s.trim(),
        _ => return spreads.to_vec(),
    };

    let mut requested_numbers: Vec<i32> = Vec::new();
    for part in raw.split(',') {
        let token = part.trim();
        if let Some((start_s, end_s)) = token.split_once('-') {
            if let (Ok(s), Ok(e)) = (start_s.trim().parse::<i32>(), end_s.trim().parse::<i32>()) {
                let lo = s.min(e);
                let hi = s.max(e).min(lo + 200);
                for n in lo..=hi {
                    requested_numbers.push(n);
                }
            }
        } else if let Ok(n) = token.parse::<i32>() {
            requested_numbers.push(n);
        }
    }

    if requested_numbers.is_empty() {
        return spreads.to_vec();
    }

    let matched: Vec<MagazinePageSpread> = spreads
        .iter()
        .filter(|sp| {
            let sp_nums: Vec<i32> = sp
                .page_no
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|p| p.parse::<i32>().ok())
                .collect();
            if sp_nums.len() == 2 {
                let (lo, hi) = (sp_nums[0].min(sp_nums[1]), sp_nums[0].max(sp_nums[1]));
                requested_numbers.iter().any(|&r| r >= lo && r <= hi)
            } else if sp_nums.len() == 1 {
                requested_numbers.contains(&sp_nums[0])
            } else {
                true
            }
        })
        .cloned()
        .collect();

    if matched.is_empty() {
        spreads.to_vec()
    } else {
        matched
    }
}

#[allow(clippy::too_many_arguments)]
fn build_cover_page_stream(
    issue: &MagazineIssue,
    selection_desc: &str,
    full_watermark_line: &str,
    diag_primary: &str,
    diag_secondary: &str,
    drm_hash: &str,
    user_name: &str,
    org_name: &str,
    license_label: &str,
    client_ip: &str,
    timestamp: &str,
    page_idx: usize,
    total_pages: usize,
) -> String {
    let mut s = String::with_capacity(4096);

    append_page_chrome_and_watermarks(
        &mut s,
        issue.issue_number,
        full_watermark_line,
        diag_primary,
        diag_secondary,
        page_idx,
        total_pages,
    );

    let issue_header = format!(
        "MSI DERGISI - SAYI #{} ({} {})",
        issue.issue_number,
        sanitize_pdf_text(&issue.publish_month_tr).to_uppercase(),
        issue.year
    );
    s.push_str("BT\n0.08 0.14 0.26 rg\n/F1 16 Tf\n42 732 Td\n");
    s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&issue_header)));

    let title_tr = sanitize_pdf_text(&issue.title_tr);
    s.push_str("BT\n0.05 0.08 0.15 rg\n/F1 13 Tf\n42 708 Td\n");
    s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&title_tr)));

    let subtitle_en = format!("EN: {}", sanitize_pdf_text(&issue.title_en));
    s.push_str("BT\n0.32 0.38 0.48 rg\n/F2 10 Tf\n42 690 Td\n");
    s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&subtitle_en)));

    s.push_str(
        "BT\n0.12 0.16 0.24 rg\n/F1 10 Tf\n42 662 Td\n(KAPAK DOSYASI VE SAYI OZETI:) Tj\nET\n",
    );
    let summary_lines = wrap_text(&sanitize_pdf_text(&issue.summary_tr), 78);
    let mut y_cursor = 644;
    for line in summary_lines
        .iter()
        .take(crate::db::constants::PDF_SUMMARY_LINES_LIMIT)
    {
        s.push_str(&format!(
            "BT\n0.18 0.22 0.30 rg\n/F2 9.5 Tf\n42 {} Td\n({}) Tj\nET\n",
            y_cursor,
            escape_pdf_string(line)
        ));
        y_cursor -= 15;
    }

    y_cursor -= 12;
    s.push_str(&format!(
        "BT\n0.08 0.14 0.26 rg\n/F1 10.5 Tf\n42 {} Td\n(BU SAYIDA YER ALAN DOSYALAR ({}):) Tj\nET\n",
        y_cursor,
        escape_pdf_string(selection_desc)
    ));
    y_cursor -= 18;

    for spread in issue
        .spreads
        .iter()
        .take(crate::db::constants::PDF_SPREADS_LIMIT)
    {
        if y_cursor < 250 {
            break;
        }
        let item_line = format!(
            "Sayfa {} | [{}] {}",
            sanitize_pdf_text(&spread.page_no),
            sanitize_pdf_text(&spread.category_badge),
            sanitize_pdf_text(&spread.title_tr)
        );
        let truncated = truncate_chars(&item_line, 82);
        s.push_str(&format!(
            "BT\n0.15 0.20 0.28 rg\n/F2 9 Tf\n48 {} Td\n({}) Tj\nET\n",
            y_cursor,
            escape_pdf_string(&truncated)
        ));
        y_cursor -= 15;
    }

    s.push_str("0.95 0.96 0.98 rg\n38 82 519 138 re f\n");
    s.push_str("0.78 0.62 0.18 RG\n1.2 w\n38 82 519 138 re S\n");
    s.push_str("BT\n0.07 0.12 0.22 rg\n/F1 10 Tf\n48 202 Td\n(KRIPTOGRAFIK E-DERGI DRM VE KURUMSAL LISANS SERTIFIKASI) Tj\nET\n");

    let cert_lines = [
        format!("Lisans Sahibi Okur : {} ({})", user_name, org_name),
        format!("Erisim Lisans Tipi : {}", license_label),
        format!("Indirme Kapsami    : {}", selection_desc),
        format!("Zaman Damgasi & IP : {} | IP: {}", timestamp, client_ip),
        format!("Kriptografik Imza  : {}", drm_hash),
        "Yasal Uyari        : Bu PDF dosyasi 5846 sayili FSEK kapsaminda dinamik filigran ile korunmaktadir.".to_string(),
    ];

    let mut cert_y = 184;
    for cl in &cert_lines {
        s.push_str(&format!(
            "BT\n0.15 0.20 0.28 rg\n/F3 8.5 Tf\n48 {} Td\n({}) Tj\nET\n",
            cert_y,
            escape_pdf_string(cl)
        ));
        cert_y -= 15;
    }

    s
}

fn build_spread_page_stream(
    issue: &MagazineIssue,
    spread: &MagazinePageSpread,
    full_watermark_line: &str,
    diag_primary: &str,
    diag_secondary: &str,
    page_idx: usize,
    total_pages: usize,
) -> String {
    let mut s = String::with_capacity(4096);

    append_page_chrome_and_watermarks(
        &mut s,
        issue.issue_number,
        full_watermark_line,
        diag_primary,
        diag_secondary,
        page_idx,
        total_pages,
    );

    let badge_line = format!(
        "SAYFA: {}  |  KATEGORI: {}  |  SAYI #{}",
        sanitize_pdf_text(&spread.page_no),
        sanitize_pdf_text(&spread.category_badge).to_uppercase(),
        issue.issue_number
    );
    s.push_str("BT\n0.68 0.50 0.10 rg\n/F1 10 Tf\n42 730 Td\n");
    s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&badge_line)));

    let title_lines = wrap_text(&sanitize_pdf_text(&spread.title_tr), 62);
    let mut y = 706;
    for tl in title_lines
        .iter()
        .take(crate::db::constants::PDF_TITLE_LINES_TR_LIMIT)
    {
        s.push_str(&format!(
            "BT\n0.06 0.10 0.18 rg\n/F1 14 Tf\n42 {} Td\n({}) Tj\nET\n",
            y,
            escape_pdf_string(tl)
        ));
        y -= 19;
    }

    y -= 4;
    let en_title = format!("EN: {}", sanitize_pdf_text(&spread.title_en));
    for tl in wrap_text(&en_title, 74)
        .iter()
        .take(crate::db::constants::PDF_TITLE_LINES_EN_LIMIT)
    {
        s.push_str(&format!(
            "BT\n0.35 0.42 0.52 rg\n/F2 10 Tf\n42 {} Td\n({}) Tj\nET\n",
            y,
            escape_pdf_string(tl)
        ));
        y -= 15;
    }

    y -= 8;
    s.push_str(&format!(
        "0.82 0.85 0.90 RG\n0.8 w\n42 {} m 553 {} l S\n",
        y, y
    ));
    y -= 22;

    s.push_str(&format!(
        "BT\n0.08 0.14 0.26 rg\n/F1 10.5 Tf\n42 {} Td\n(DOSYA ICERIGI VE TEKNIK ANALIZ OZETI (TR):) Tj\nET\n",
        y
    ));
    y -= 18;

    for line in wrap_text(&sanitize_pdf_text(&spread.excerpt_tr), 78)
        .iter()
        .take(crate::db::constants::PDF_BODY_LINES_LIMIT)
    {
        s.push_str(&format!(
            "BT\n0.14 0.18 0.25 rg\n/F2 9.5 Tf\n42 {} Td\n({}) Tj\nET\n",
            y,
            escape_pdf_string(line)
        ));
        y -= 15;
    }

    y -= 14;
    s.push_str(&format!(
        "BT\n0.08 0.14 0.26 rg\n/F1 10.5 Tf\n42 {} Td\n(EXECUTIVE BRIEFING & TECHNICAL ABSTRACT (EN):) Tj\nET\n",
        y
    ));
    y -= 18;

    for line in wrap_text(&sanitize_pdf_text(&spread.excerpt_en), 78)
        .iter()
        .take(crate::db::constants::PDF_TOC_LINES_LIMIT)
    {
        s.push_str(&format!(
            "BT\n0.24 0.29 0.38 rg\n/F2 9.5 Tf\n42 {} Td\n({}) Tj\nET\n",
            y,
            escape_pdf_string(line)
        ));
        y -= 15;
    }

    s.push_str("0.96 0.97 0.99 rg\n42 86 511 42 re f\n");
    s.push_str("0.82 0.85 0.90 RG\n0.7 w\n42 86 511 42 re S\n");
    s.push_str("BT\n0.22 0.27 0.36 rg\n/F3 7.2 Tf\n48 110 Td\n");
    s.push_str(&format!(
        "({}) Tj\nET\n",
        escape_pdf_string(&truncate_chars(full_watermark_line, 98))
    ));
    s.push_str("BT\n0.35 0.40 0.48 rg\n/F2 7.2 Tf\n48 96 Td\n");
    s.push_str(&format!(
        "(DOGRULAMA: Bu sayfa izinsiz cogaltilamaz ve kurumsal ag disinda paylasilamaz. {}) Tj\nET\n",
        escape_pdf_string(diag_secondary)
    ));

    s
}

fn append_page_chrome_and_watermarks(
    s: &mut String,
    issue_number: i32,
    full_watermark_line: &str,
    diag_primary: &str,
    diag_secondary: &str,
    page_idx: usize,
    total_pages: usize,
) {
    s.push_str("q\n0.87 0.89 0.93 rg\nBT\n/F1 13 Tf\n0.819 0.573 -0.573 0.819 58 265 Tm\n");
    s.push_str(&format!("({}) Tj\nET\nQ\n", escape_pdf_string(diag_primary)));

    s.push_str("q\n0.88 0.90 0.94 rg\nBT\n/F3 10.5 Tf\n0.819 0.573 -0.573 0.819 78 232 Tm\n");
    s.push_str(&format!(
        "({}) Tj\nET\nQ\n",
        escape_pdf_string(diag_secondary)
    ));

    s.push_str("q\n0.89 0.91 0.94 rg\nBT\n/F1 12 Tf\n0.819 0.573 -0.573 0.819 64 470 Tm\n");
    s.push_str(&format!("({}) Tj\nET\nQ\n", escape_pdf_string(diag_primary)));

    s.push_str("0.06 0.09 0.16 rg\n0 778 595 64 re f\n");
    s.push_str("0.83 0.68 0.21 rg\n0 776 595 2.5 re f\n");

    let header_title = format!(
        "MSI DERGISI & MSI TDR | SAVUNMA SANAYII VE HAVACILIK PLATFORMU - SAYI #{}  (SAYFA {}/{})",
        issue_number, page_idx, total_pages
    );
    s.push_str("BT\n1 1 1 rg\n/F1 9.5 Tf\n38 818 Td\n");
    s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&header_title)));

    s.push_str("BT\n0.92 0.80 0.42 rg\n/F3 6.6 Tf\n38 792 Td\n");
    s.push_str(&format!(
        "({}) Tj\nET\n",
        escape_pdf_string(&truncate_chars(full_watermark_line, 112))
    ));

    s.push_str("0.06 0.09 0.16 rg\n0 0 595 54 re f\n");
    s.push_str("0.83 0.68 0.21 rg\n0 54 595 1.8 re f\n");

    s.push_str("BT\n0.95 0.82 0.45 rg\n/F3 6.6 Tf\n32 34 Td\n");
    s.push_str(&format!(
        "({}) Tj\nET\n",
        escape_pdf_string(&truncate_chars(full_watermark_line, 114))
    ));

    s.push_str("BT\n0.75 0.80 0.88 rg\n/F2 7 Tf\n32 16 Td\n");
    s.push_str("(UMSA Produksiyon - MSI Dergisi Dijital DRM Guvenlik Katmani | msidergisi.com & savunmahaber.com) Tj\nET\n");
}

fn assemble_pdf_binary(
    page_streams: &[String],
    issue_number: i32,
    issue_title_ascii: &str,
    full_watermark_line: &str,
    drm_hash: &str,
) -> Vec<u8> {
    let page_count = page_streams.len();
    let info_obj_id = 6 + page_count * 2;
    let total_objects = info_obj_id;

    let mut buf: Vec<u8> = Vec::with_capacity(16384);
    let mut offsets: Vec<usize> = vec![0; total_objects + 1];

    buf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    offsets[1] = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets[2] = buf.len();
    let kids_refs: Vec<String> = (0..page_count)
        .map(|i| format!("{} 0 R", 6 + i * 2))
        .collect();
    let pages_obj = format!(
        "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
        kids_refs.join(" "),
        page_count
    );
    buf.extend_from_slice(pages_obj.as_bytes());

    offsets[3] = buf.len();
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    offsets[4] = buf.len();
    buf.extend_from_slice(
        b"4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    offsets[5] = buf.len();
    buf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier-Bold /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    for (i, stream_content) in page_streams.iter().enumerate() {
        let page_obj_id = 6 + i * 2;
        let content_obj_id = 7 + i * 2;

        offsets[page_obj_id] = buf.len();
        let page_obj = format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents {} 0 R >>\nendobj\n",
            page_obj_id, content_obj_id
        );
        buf.extend_from_slice(page_obj.as_bytes());

        offsets[content_obj_id] = buf.len();
        let stream_bytes = stream_content.as_bytes();
        let content_header = format!(
            "{} 0 obj\n<< /Length {} >>\nstream\n",
            content_obj_id,
            stream_bytes.len()
        );
        buf.extend_from_slice(content_header.as_bytes());
        buf.extend_from_slice(stream_bytes);
        if !stream_content.ends_with('\n') {
            buf.push(b'\n');
        }
        buf.extend_from_slice(b"endstream\nendobj\n");
    }

    offsets[info_obj_id] = buf.len();
    let info_obj = format!(
        "{} 0 obj\n<< /Title (MSI Dergisi Sayi #{} - {}) /Author (UMSA Produksiyon - MSI Dergisi) /Subject ({}) /Keywords (MSI Dergisi, E-Dergi, DRM, {}) /Creator (MSI Dergisi & MSI TDR Rust DRM Engine v2.6) /Producer (Pure Rust PDF 1.4 Generator) >>\nendobj\n",
        info_obj_id,
        issue_number,
        escape_pdf_string(issue_title_ascii),
        escape_pdf_string(full_watermark_line),
        escape_pdf_string(drm_hash)
    );
    buf.extend_from_slice(info_obj.as_bytes());

    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n", total_objects + 1).as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R /Info {} 0 R >>\nstartxref\n{}\n%MSI-DRM:{}\n%%EOF\n",
        total_objects + 1,
        info_obj_id,
        xref_offset,
        sanitize_pdf_text(drm_hash)
    );
    buf.extend_from_slice(trailer.as_bytes());

    buf
}

/// Transliterates Turkish and special UTF-8 characters into clean PDF-safe ASCII so Type1 fonts
/// in Adobe Acrobat, Chrome PDF Viewer, and Edge render crisply without encoding corruption.
pub fn sanitize_pdf_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            'ğ' => out.push('g'),
            'Ğ' => out.push('G'),
            'ü' => out.push('u'),
            'Ü' => out.push('U'),
            'ş' => out.push('s'),
            'Ş' => out.push('S'),
            'ı' => out.push('i'),
            'İ' => out.push('I'),
            'ö' => out.push('o'),
            'Ö' => out.push('O'),
            'ç' => out.push('c'),
            'Ç' => out.push('C'),
            'â' | 'à' | 'á' => out.push('a'),
            'Â' | 'À' | 'Á' => out.push('A'),
            'î' | 'ï' => out.push('i'),
            'Î' | 'Ï' => out.push('I'),
            'û' => out.push('u'),
            'Û' => out.push('U'),
            '’' | '‘' | '`' => out.push('\''),
            '“' | '”' | '«' | '»' => out.push('"'),
            '–' | '—' => out.push('-'),
            '•' | '·' => out.push('*'),
            '…' => out.push_str("..."),
            c if c.is_ascii_graphic() || c == ' ' => out.push(c),
            _ => out.push(' '),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn escape_pdf_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\r' | '\n' => out.push(' '),
            c if c.is_ascii() => out.push(c),
            _ => {}
        }
    }
    out
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        text.chars().take(max_chars).collect()
    }
}

// ============================================================================
// Pure-Rust RFC 1951 DEFLATE / zlib Inflate + PNG Predictor
// ============================================================================

mod flate {
    const MAX_INFLATE_BYTES: usize = 64 * 1024 * 1024;
    const MAX_BITS: usize = 15;

    struct BitReader<'a> {
        data: &'a [u8],
        pos: usize,
        buf: u32,
        cnt: u8,
    }

    impl<'a> BitReader<'a> {
        fn new(data: &'a [u8]) -> Self {
            Self {
                data,
                pos: 0,
                buf: 0,
                cnt: 0,
            }
        }

        #[inline]
        fn ensure(&mut self, n: u8) -> Result<(), ()> {
            while self.cnt < n {
                if self.pos >= self.data.len() {
                    return Err(());
                }
                self.buf |= (self.data[self.pos] as u32) << self.cnt;
                self.pos += 1;
                self.cnt += 8;
            }
            Ok(())
        }

        #[inline]
        fn read_bits(&mut self, n: u8) -> Result<u32, ()> {
            if n == 0 {
                return Ok(0);
            }
            self.ensure(n)?;
            let mask = (1u32 << n) - 1;
            let val = self.buf & mask;
            self.buf >>= n;
            self.cnt -= n;
            Ok(val)
        }

        #[inline]
        fn read_bit(&mut self) -> Result<u32, ()> {
            self.read_bits(1)
        }

        fn align_byte(&mut self) {
            let discard = self.cnt % 8;
            if discard > 0 {
                self.buf >>= discard;
                self.cnt -= discard;
            }
        }

        fn read_u16_le(&mut self) -> Result<u16, ()> {
            self.align_byte();
            let lo = self.read_bits(8)?;
            let hi = self.read_bits(8)?;
            Ok((lo | (hi << 8)) as u16)
        }
    }

    struct HuffmanTable {
        counts: [u16; MAX_BITS + 1],
        symbols: Vec<u16>,
    }

    impl HuffmanTable {
        fn build(lengths: &[u8]) -> Result<Self, ()> {
            let mut counts = [0u16; MAX_BITS + 1];
            for &len in lengths {
                if len as usize > MAX_BITS {
                    return Err(());
                }
                counts[len as usize] += 1;
            }

            if counts[0] as usize == lengths.len() {
                return Ok(Self {
                    counts,
                    symbols: vec![],
                });
            }

            // Verify table is not over-subscribed
            let mut left = 1i32;
            for len in 1..=MAX_BITS {
                left <<= 1;
                left -= counts[len] as i32;
                if left < 0 {
                    return Err(());
                }
            }

            let mut offsets = [0u16; MAX_BITS + 1];
            for len in 1..MAX_BITS {
                offsets[len + 1] = offsets[len] + counts[len];
            }

            let mut symbols = vec![0u16; lengths.len()];
            for (sym, &len) in lengths.iter().enumerate() {
                if len != 0 {
                    let idx = offsets[len as usize] as usize;
                    offsets[len as usize] += 1;
                    symbols[idx] = sym as u16;
                }
            }

            Ok(Self { counts, symbols })
        }

        fn decode(&self, reader: &mut BitReader<'_>) -> Result<u16, ()> {
            let mut code: i32 = 0;
            let mut first: i32 = 0;
            let mut index: usize = 0;
            for len in 1..=MAX_BITS {
                code |= reader.read_bit()? as i32;
                let count = self.counts[len] as i32;
                if code - count < first {
                    return Ok(self.symbols[index + (code - first) as usize]);
                }
                index += count as usize;
                first += count;
                first <<= 1;
                code <<= 1;
            }
            Err(())
        }
    }

    pub fn inflate_zlib(data: &[u8]) -> Result<Vec<u8>, ()> {
        if data.len() >= 2 {
            let cmf = data[0];
            let flg = data[1];
            let check = ((cmf as u16) << 8) | (flg as u16);
            if (cmf & 0x0f) == 8 && check % 31 == 0 {
                let mut offset = 2;
                if (flg & 0x20) != 0 {
                    if data.len() < 6 {
                        return Err(());
                    }
                    offset += 4;
                }
                if let Ok(res) = inflate_raw_deflate(&data[offset..]) {
                    return Ok(res);
                }
            }
        }
        inflate_raw_deflate(data)
    }

    fn fixed_tables() -> Result<(HuffmanTable, HuffmanTable), ()> {
        let mut lit_lengths = [0u8; 288];
        for b in &mut lit_lengths[..=143] {
            *b = 8;
        }
        for b in &mut lit_lengths[144..=255] {
            *b = 9;
        }
        for b in &mut lit_lengths[256..=279] {
            *b = 7;
        }
        for b in &mut lit_lengths[280..=287] {
            *b = 8;
        }
        let dist_lengths = [5u8; 32];
        Ok((
            HuffmanTable::build(&lit_lengths)?,
            HuffmanTable::build(&dist_lengths)?,
        ))
    }

    fn dynamic_tables(reader: &mut BitReader<'_>) -> Result<(HuffmanTable, HuffmanTable), ()> {
        const CL_ORDER: [usize; 19] = [
            16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
        ];
        let hlit = reader.read_bits(5)? as usize + 257;
        let hdist = reader.read_bits(5)? as usize + 1;
        let hclen = reader.read_bits(4)? as usize + 4;
        if hlit > 286 || hdist > 30 {
            return Err(());
        }

        let mut cl_lengths = [0u8; 19];
        for i in 0..hclen {
            cl_lengths[CL_ORDER[i]] = reader.read_bits(3)? as u8;
        }
        let cl_table = HuffmanTable::build(&cl_lengths)?;

        let total = hlit + hdist;
        let mut lengths = Vec::with_capacity(total);
        while lengths.len() < total {
            let sym = cl_table.decode(reader)?;
            if sym < 16 {
                lengths.push(sym as u8);
            } else {
                let (repeat, val) = match sym {
                    16 => {
                        let prev = *lengths.last().ok_or(())?;
                        (3 + reader.read_bits(2)? as usize, prev)
                    }
                    17 => (3 + reader.read_bits(3)? as usize, 0u8),
                    18 => (11 + reader.read_bits(7)? as usize, 0u8),
                    _ => return Err(()),
                };
                if lengths.len() + repeat > total {
                    return Err(());
                }
                lengths.resize(lengths.len() + repeat, val);
            }
        }

        Ok((
            HuffmanTable::build(&lengths[..hlit])?,
            HuffmanTable::build(&lengths[hlit..])?,
        ))
    }

    fn inflate_raw_deflate(src: &[u8]) -> Result<Vec<u8>, ()> {
        const LEN_BASE: [u16; 29] = [
            3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99,
            115, 131, 163, 195, 227, 258,
        ];
        const LEN_EXTRA: [u8; 29] = [
            0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
        ];
        const DIST_BASE: [u16; 30] = [
            1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025,
            1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
        ];
        const DIST_EXTRA: [u8; 30] = [
            0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12,
            12, 13, 13,
        ];

        let mut reader = BitReader::new(src);
        let mut out = Vec::with_capacity(src.len().saturating_mul(2).min(1024 * 1024));

        loop {
            let bfinal = reader.read_bit()?;
            let btype = reader.read_bits(2)?;
            match btype {
                0 => {
                    let len = reader.read_u16_le()? as usize;
                    let nlen = reader.read_u16_le()?;
                    if len as u16 != !nlen {
                        return Err(());
                    }
                    if out.len() + len > MAX_INFLATE_BYTES {
                        return Err(());
                    }
                    reader.align_byte();
                    if reader.pos + len > reader.data.len() {
                        return Err(());
                    }
                    out.extend_from_slice(&reader.data[reader.pos..reader.pos + len]);
                    reader.pos += len;
                }
                1 | 2 => {
                    let (lit_tbl, dist_tbl) = if btype == 1 {
                        fixed_tables()?
                    } else {
                        dynamic_tables(&mut reader)?
                    };

                    loop {
                        let sym = lit_tbl.decode(&mut reader)?;
                        if sym < 256 {
                            if out.len() >= MAX_INFLATE_BYTES {
                                return Err(());
                            }
                            out.push(sym as u8);
                        } else if sym == 256 {
                            break;
                        } else {
                            let lidx = (sym as usize).checked_sub(257).ok_or(())?;
                            if lidx >= LEN_BASE.len() {
                                return Err(());
                            }
                            let length =
                                LEN_BASE[lidx] as usize + reader.read_bits(LEN_EXTRA[lidx])? as usize;
                            let dsym = dist_tbl.decode(&mut reader)? as usize;
                            if dsym >= DIST_BASE.len() {
                                return Err(());
                            }
                            let dist = DIST_BASE[dsym] as usize
                                + reader.read_bits(DIST_EXTRA[dsym])? as usize;
                            if dist == 0 || dist > out.len() || out.len() + length > MAX_INFLATE_BYTES {
                                return Err(());
                            }
                            for _ in 0..length {
                                let b = out[out.len() - dist];
                                out.push(b);
                            }
                        }
                    }
                }
                _ => return Err(()),
            }
            if bfinal != 0 {
                break;
            }
        }
        Ok(out)
    }

    pub fn decode_predictor(
        raw: &[u8],
        predictor: i64,
        columns: usize,
        colors: usize,
        bpc: usize,
    ) -> Result<Vec<u8>, ()> {
        if predictor <= 1 {
            return Ok(raw.to_vec());
        }
        if predictor < 10 {
            return Ok(raw.to_vec());
        }
        let cols = columns.max(1);
        let cols_bits = cols
            .checked_mul(colors)
            .and_then(|v| v.checked_mul(bpc))
            .ok_or(())?;
        let row_bytes = (cols_bits + 7) / 8;
        let bpp = ((colors * bpc) + 7) / 8;
        let stride = row_bytes + 1;
        if row_bytes == 0 || raw.len() % stride != 0 {
            // Best-effort decode for truncated trailing bytes
            if raw.len() < stride {
                return Err(());
            }
        }
        let num_rows = raw.len() / stride;
        let mut out = vec![0u8; num_rows * row_bytes];

        for r in 0..num_rows {
            let src_row = &raw[r * stride..r * stride + stride];
            let filter = src_row[0];
            let cur_in = &src_row[1..];
            let out_start = r * row_bytes;

            for i in 0..row_bytes {
                let left = if i >= bpp { out[out_start + i - bpp] } else { 0 };
                let up = if r > 0 { out[out_start - row_bytes + i] } else { 0 };
                let up_left = if r > 0 && i >= bpp {
                    out[out_start - row_bytes + i - bpp]
                } else {
                    0
                };
                let recon = match filter {
                    0 => cur_in[i],
                    1 => cur_in[i].wrapping_add(left),
                    2 => cur_in[i].wrapping_add(up),
                    3 => cur_in[i].wrapping_add((((left as u16) + (up as u16)) / 2) as u8),
                    4 => cur_in[i].wrapping_add(paeth(left, up, up_left)),
                    _ => cur_in[i],
                };
                out[out_start + i] = recon;
            }
        }

        Ok(out)
    }

    #[inline]
    fn paeth(a: u8, b: u8, c: u8) -> u8 {
        let a = a as i32;
        let b = b as i32;
        let c = c as i32;
        let p = a + b - c;
        let pa = (p - a).abs();
        let pb = (p - b).abs();
        let pc = (p - c).abs();
        if pa <= pb && pa <= pc {
            a as u8
        } else if pb <= pc {
            b as u8
        } else {
            c as u8
        }
    }
}

// ============================================================================
// Pure-Rust PDF Parser, Object Table, Page Slicer & Watermark Injector
// ============================================================================

mod pdf_codec {
    use super::flate::{decode_predictor, inflate_zlib};
    use super::{escape_pdf_string, sanitize_pdf_text, wrap_text};
    use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

    #[derive(Clone, Debug)]
    enum PdfVal {
        Null,
        Bool(bool),
        Int(i64),
        Real(String),
        Name(String),
        LitStr(Vec<u8>),
        HexStr(Vec<u8>),
        Array(Vec<PdfVal>),
        Dict(Vec<(String, PdfVal)>),
        Ref(u32, u16),
    }

    #[derive(Clone, Debug)]
    struct PdfObj {
        val: PdfVal,
        stream: Option<Vec<u8>>,
    }

    struct PdfDoc {
        header_version: String,
        objects: BTreeMap<u32, PdfObj>,
        root_ref: Option<u32>,
        info_ref: Option<u32>,
        encrypted: bool,
    }

    #[inline]
    fn is_ws(b: u8) -> bool {
        matches!(b, b'\x00' | b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
    }

    #[inline]
    fn is_delim(b: u8) -> bool {
        matches!(
            b,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
    }

    fn decode_pdf_name(raw: &str) -> String {
        if !raw.contains('#') {
            return raw.to_string();
        }
        let bytes = raw.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'#' && i + 2 < bytes.len() {
                if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push((h << 4) | l);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).to_string()
    }

    #[inline]
    fn hex_val(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    impl PdfVal {
        fn as_dict(&self) -> Option<&Vec<(String, PdfVal)>> {
            match self {
                PdfVal::Dict(d) => Some(d),
                _ => None,
            }
        }

        fn as_dict_mut(&mut self) -> Option<&mut Vec<(String, PdfVal)>> {
            match self {
                PdfVal::Dict(d) => Some(d),
                _ => None,
            }
        }

        fn dict_get(&self, key: &str) -> Option<&PdfVal> {
            self.as_dict().and_then(|entries| {
                entries
                    .iter()
                    .rev()
                    .find(|(k, _)| decode_pdf_name(k) == key)
                    .map(|(_, v)| v)
            })
        }

        fn dict_set(&mut self, key: &str, val: PdfVal) {
            if let Some(entries) = self.as_dict_mut() {
                entries.retain(|(k, _)| decode_pdf_name(k) != key);
                entries.push((key.to_string(), val));
            }
        }

        fn dict_remove(&mut self, key: &str) {
            if let Some(entries) = self.as_dict_mut() {
                entries.retain(|(k, _)| decode_pdf_name(k) != key);
            }
        }

        fn as_int(&self) -> Option<i64> {
            match self {
                PdfVal::Int(n) => Some(*n),
                _ => None,
            }
        }

        fn as_f64(&self) -> Option<f64> {
            match self {
                PdfVal::Int(n) => Some(*n as f64),
                PdfVal::Real(s) => s.parse::<f64>().ok(),
                _ => None,
            }
        }

        fn as_name_decoded(&self) -> Option<String> {
            match self {
                PdfVal::Name(n) => Some(decode_pdf_name(n)),
                _ => None,
            }
        }

        fn as_ref_id(&self) -> Option<u32> {
            match self {
                PdfVal::Ref(id, _) => Some(*id),
                _ => None,
            }
        }

        fn as_array(&self) -> Option<&Vec<PdfVal>> {
            match self {
                PdfVal::Array(a) => Some(a),
                _ => None,
            }
        }

        fn collect_refs(&self, out: &mut Vec<u32>) {
            match self {
                PdfVal::Ref(id, _) => out.push(*id),
                PdfVal::Array(items) => {
                    for item in items {
                        item.collect_refs(out);
                    }
                }
                PdfVal::Dict(entries) => {
                    for (_, v) in entries {
                        v.collect_refs(out);
                    }
                }
                _ => {}
            }
        }

        fn remap_refs(&mut self, mapping: &HashMap<u32, u32>) {
            match self {
                PdfVal::Ref(id, gen) => {
                    if let Some(&new_id) = mapping.get(id) {
                        *id = new_id;
                        *gen = 0;
                    }
                }
                PdfVal::Array(items) => {
                    for item in items {
                        item.remap_refs(mapping);
                    }
                }
                PdfVal::Dict(entries) => {
                    for (_, v) in entries {
                        v.remap_refs(mapping);
                    }
                }
                _ => {}
            }
        }

        fn serialize(&self, out: &mut Vec<u8>) {
            match self {
                PdfVal::Null => out.extend_from_slice(b"null"),
                PdfVal::Bool(true) => out.extend_from_slice(b"true"),
                PdfVal::Bool(false) => out.extend_from_slice(b"false"),
                PdfVal::Int(n) => out.extend_from_slice(n.to_string().as_bytes()),
                PdfVal::Real(s) => out.extend_from_slice(s.as_bytes()),
                PdfVal::Name(n) => {
                    out.push(b'/');
                    out.extend_from_slice(n.as_bytes());
                }
                PdfVal::LitStr(bytes) => {
                    out.push(b'(');
                    for &b in bytes {
                        match b {
                            b'\\' => out.extend_from_slice(b"\\\\"),
                            b'(' => out.extend_from_slice(b"\\("),
                            b')' => out.extend_from_slice(b"\\)"),
                            b'\r' => out.extend_from_slice(b"\\r"),
                            b'\n' => out.extend_from_slice(b"\\n"),
                            _ => out.push(b),
                        }
                    }
                    out.push(b')');
                }
                PdfVal::HexStr(raw_hex) => {
                    out.push(b'<');
                    out.extend_from_slice(raw_hex);
                    out.push(b'>');
                }
                PdfVal::Array(items) => {
                    out.push(b'[');
                    for (i, item) in items.iter().enumerate() {
                        if i > 0 {
                            out.push(b' ');
                        }
                        item.serialize(out);
                    }
                    out.push(b']');
                }
                PdfVal::Dict(entries) => {
                    out.extend_from_slice(b"<<");
                    for (k, v) in entries {
                        out.push(b' ');
                        out.push(b'/');
                        out.extend_from_slice(k.as_bytes());
                        out.push(b' ');
                        v.serialize(out);
                    }
                    out.extend_from_slice(b" >>");
                }
                PdfVal::Ref(id, gen) => {
                    out.extend_from_slice(format!("{} {} R", id, gen).as_bytes());
                }
            }
        }
    }

    struct Cursor<'a> {
        data: &'a [u8],
        pos: usize,
    }

    impl<'a> Cursor<'a> {
        fn new(data: &'a [u8], pos: usize) -> Self {
            Self { data, pos }
        }

        fn skip_ws(&mut self) {
            while self.pos < self.data.len() {
                let b = self.data[self.pos];
                if is_ws(b) {
                    self.pos += 1;
                } else if b == b'%' {
                    while self.pos < self.data.len()
                        && self.data[self.pos] != b'\n'
                        && self.data[self.pos] != b'\r'
                    {
                        self.pos += 1;
                    }
                } else {
                    break;
                }
            }
        }

        fn read_word(&mut self) -> &'a [u8] {
            self.skip_ws();
            let start = self.pos;
            while self.pos < self.data.len()
                && !is_ws(self.data[self.pos])
                && !is_delim(self.data[self.pos])
            {
                self.pos += 1;
            }
            &self.data[start..self.pos]
        }

        fn parse_val(&mut self, depth: usize) -> Option<PdfVal> {
            if depth > 64 {
                return None;
            }
            self.skip_ws();
            if self.pos >= self.data.len() {
                return None;
            }
            let b = self.data[self.pos];
            match b {
                b'<' => {
                    if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'<' {
                        self.pos += 2;
                        let mut entries = Vec::new();
                        loop {
                            self.skip_ws();
                            if self.pos + 1 < self.data.len()
                                && self.data[self.pos] == b'>'
                                && self.data[self.pos + 1] == b'>'
                            {
                                self.pos += 2;
                                return Some(PdfVal::Dict(entries));
                            }
                            if self.pos >= self.data.len() {
                                return Some(PdfVal::Dict(entries));
                            }
                            let key = match self.parse_val(depth + 1)? {
                                PdfVal::Name(n) => n,
                                _ => return None,
                            };
                            let val = self.parse_val(depth + 1)?;
                            entries.push((key, val));
                        }
                    } else {
                        self.pos += 1;
                        let mut raw_hex = Vec::new();
                        while self.pos < self.data.len() {
                            let c = self.data[self.pos];
                            self.pos += 1;
                            if c == b'>' {
                                break;
                            }
                            if !is_ws(c) {
                                raw_hex.push(c);
                            }
                        }
                        Some(PdfVal::HexStr(raw_hex))
                    }
                }
                b'[' => {
                    self.pos += 1;
                    let mut items = Vec::new();
                    loop {
                        self.skip_ws();
                        if self.pos >= self.data.len() {
                            return Some(PdfVal::Array(items));
                        }
                        if self.data[self.pos] == b']' {
                            self.pos += 1;
                            return Some(PdfVal::Array(items));
                        }
                        let val = self.parse_val(depth + 1)?;
                        items.push(val);
                    }
                }
                b'(' => {
                    self.pos += 1;
                    let mut out = Vec::new();
                    let mut paren_depth = 1usize;
                    while self.pos < self.data.len() {
                        let c = self.data[self.pos];
                        self.pos += 1;
                        if c == b'\\' {
                            if self.pos >= self.data.len() {
                                break;
                            }
                            let esc = self.data[self.pos];
                            self.pos += 1;
                            match esc {
                                b'n' => out.push(b'\n'),
                                b'r' => out.push(b'\r'),
                                b't' => out.push(b'\t'),
                                b'b' => out.push(0x08),
                                b'f' => out.push(0x0c),
                                b'(' => out.push(b'('),
                                b')' => out.push(b')'),
                                b'\\' => out.push(b'\\'),
                                b'\r' => {
                                    if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
                                        self.pos += 1;
                                    }
                                }
                                b'\n' => {}
                                b'0'..=b'7' => {
                                    let mut oct = (esc - b'0') as u16;
                                    for _ in 0..2 {
                                        if self.pos < self.data.len()
                                            && matches!(self.data[self.pos], b'0'..=b'7')
                                        {
                                            oct = oct * 8 + (self.data[self.pos] - b'0') as u16;
                                            self.pos += 1;
                                        } else {
                                            break;
                                        }
                                    }
                                    out.push((oct & 0xff) as u8);
                                }
                                other => out.push(other),
                            }
                        } else if c == b'(' {
                            paren_depth += 1;
                            out.push(b'(');
                        } else if c == b')' {
                            paren_depth -= 1;
                            if paren_depth == 0 {
                                break;
                            }
                            out.push(b')');
                        } else if c == b'\r' {
                            out.push(b'\n');
                            if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
                                self.pos += 1;
                            }
                        } else {
                            out.push(c);
                        }
                    }
                    Some(PdfVal::LitStr(out))
                }
                b'/' => {
                    self.pos += 1;
                    let start = self.pos;
                    while self.pos < self.data.len()
                        && !is_ws(self.data[self.pos])
                        && !is_delim(self.data[self.pos])
                    {
                        self.pos += 1;
                    }
                    let name = String::from_utf8_lossy(&self.data[start..self.pos]).to_string();
                    Some(PdfVal::Name(name))
                }
                _ => {
                    let w = self.read_word();
                    if w.is_empty() {
                        return None;
                    }
                    match w {
                        b"null" => Some(PdfVal::Null),
                        b"true" => Some(PdfVal::Bool(true)),
                        b"false" => Some(PdfVal::Bool(false)),
                        _ => {
                            let s = std::str::from_utf8(w).ok()?;
                            if let Ok(n) = s.parse::<i64>() {
                                if n >= 0 {
                                    let saved = self.pos;
                                    let w2 = self.read_word();
                                    let w3 = self.read_word();
                                    if w3 == b"R" {
                                        if let Ok(gen) =
                                            std::str::from_utf8(w2).unwrap_or("").parse::<u16>()
                                        {
                                            if let Ok(obj_id) = u32::try_from(n) {
                                                return Some(PdfVal::Ref(obj_id, gen));
                                            }
                                        }
                                    }
                                    self.pos = saved;
                                }
                                Some(PdfVal::Int(n))
                            } else if s.parse::<f64>().is_ok() {
                                Some(PdfVal::Real(s.to_string()))
                            } else {
                                None
                            }
                        }
                    }
                }
            }
        }
    }

    fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    fn rfind_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).rposition(|w| w == needle)
    }

    fn detect_pdf_version(data: &[u8]) -> Option<String> {
        let head = &data[..data.len().min(1024)];
        let idx = find_subslice(head, b"%PDF-")?;
        let rest = &head[idx + 5..];
        let end = rest
            .iter()
            .position(|&b| b == b'\r' || b == b'\n' || is_ws(b))
            .unwrap_or(rest.len());
        let ver = std::str::from_utf8(&rest[..end.min(8)]).ok()?.trim();
        if ver.is_empty() {
            None
        } else {
            Some(ver.to_string())
        }
    }

    fn parse_indirect_obj_at(
        data: &[u8],
        offset: usize,
        resolved_lengths: Option<&BTreeMap<u32, PdfObj>>,
    ) -> Option<(u32, PdfObj, usize)> {
        let mut cur = Cursor::new(data, offset);
        let id_b = cur.read_word();
        let gen_b = cur.read_word();
        let kw = cur.read_word();
        if kw != b"obj" {
            return None;
        }
        let obj_id = std::str::from_utf8(id_b).ok()?.parse::<u32>().ok()?;
        let _gen = std::str::from_utf8(gen_b).ok()?.parse::<u16>().ok()?;
        let val = cur.parse_val(0)?;

        cur.skip_ws();
        let mut stream = None;
        if cur.data[cur.pos..].starts_with(b"stream") {
            let after_kw = cur.pos + 6;
            if after_kw < data.len() && (data[after_kw] == b'\r' || data[after_kw] == b'\n') {
                let mut stream_start = after_kw;
                if data[stream_start] == b'\r' {
                    stream_start += 1;
                    if stream_start < data.len() && data[stream_start] == b'\n' {
                        stream_start += 1;
                    }
                } else {
                    stream_start += 1;
                }

                let explicit_len = val.dict_get("Length").and_then(|lv| match lv {
                    PdfVal::Int(n) if *n >= 0 => Some(*n as usize),
                    PdfVal::Ref(lid, _) => resolved_lengths
                        .and_then(|m| m.get(lid))
                        .and_then(|o| o.val.as_int())
                        .filter(|&n| n >= 0)
                        .map(|n| n as usize),
                    _ => None,
                });

                if let Some(len) = explicit_len {
                    if stream_start + len <= data.len() {
                        stream = Some(data[stream_start..stream_start + len].to_vec());
                        cur.pos = stream_start + len;
                    }
                }

                if stream.is_none() {
                    if let Some(rel_end) = find_subslice(&data[stream_start..], b"endstream") {
                        let mut end = stream_start + rel_end;
                        if end > stream_start && data[end - 1] == b'\n' {
                            end -= 1;
                            if end > stream_start && data[end - 1] == b'\r' {
                                end -= 1;
                            }
                        } else if end > stream_start && data[end - 1] == b'\r' {
                            end -= 1;
                        }
                        stream = Some(data[stream_start..end].to_vec());
                        cur.pos = stream_start + rel_end + b"endstream".len();
                    }
                }
            }
        }

        // Advance past endobj if nearby
        let scan_lim = (cur.pos + 32).min(data.len());
        if let Some(rel) = find_subslice(&data[cur.pos..scan_lim], b"endobj") {
            cur.pos += rel + 6;
        }
        Some((obj_id, PdfObj { val, stream }, cur.pos))
    }

    fn decompress_stream_payload(obj: &PdfObj) -> Option<Vec<u8>> {
        let raw = obj.stream.as_ref()?;
        let filter = obj.val.dict_get("Filter").and_then(|f| match f {
            PdfVal::Name(_) => f.as_name_decoded(),
            PdfVal::Array(a) if a.len() == 1 => a[0].as_name_decoded(),
            _ => None,
        });

        let decoded = match filter.as_deref() {
            None => raw.clone(),
            Some("FlateDecode") | Some("Fl") => inflate_zlib(raw).ok()?,
            Some(_) => return None,
        };

        let decode_parms = obj.val.dict_get("DecodeParms").and_then(|dp| match dp {
            PdfVal::Dict(_) => Some(dp),
            PdfVal::Array(a) if !a.is_empty() => Some(&a[0]),
            _ => None,
        });

        if let Some(dp) = decode_parms {
            let predictor = dp.dict_get("Predictor").and_then(|v| v.as_int()).unwrap_or(1);
            if predictor > 1 {
                let columns = dp.dict_get("Columns").and_then(|v| v.as_int()).unwrap_or(1) as usize;
                let colors = dp.dict_get("Colors").and_then(|v| v.as_int()).unwrap_or(1) as usize;
                let bpc = dp
                    .dict_get("BitsPerComponent")
                    .and_then(|v| v.as_int())
                    .unwrap_or(8) as usize;
                return decode_predictor(&decoded, predictor, columns, colors, bpc).ok();
            }
        }
        Some(decoded)
    }

    fn read_be_uint(bytes: &[u8]) -> u64 {
        let mut v = 0u64;
        for &b in bytes {
            v = (v << 8) | (b as u64);
        }
        v
    }

    impl PdfDoc {
        fn parse(data: &[u8]) -> Option<Self> {
            let header_version = detect_pdf_version(data)?;
            let mut doc = Self {
                header_version,
                objects: BTreeMap::new(),
                root_ref: None,
                info_ref: None,
                encrypted: false,
            };

            let mut obj_offsets: BTreeMap<u32, usize> = BTreeMap::new();
            let mut compressed_locs: BTreeMap<u32, (u32, usize)> = BTreeMap::new();
            let mut seen_ids: HashSet<u32> = HashSet::new();

            // 1. Follow xref / XRef stream chain from the last `startxref`
            let tail_start = data.len().saturating_sub(4096);
            if let Some(rel_sx) = rfind_subslice(&data[tail_start..], b"startxref") {
                let mut cur = Cursor::new(data, tail_start + rel_sx + 9);
                if let Some(PdfVal::Int(first_xref_pos)) = cur.parse_val(0) {
                    if first_xref_pos >= 0 {
                        let mut next_pos = Some(first_xref_pos as usize);
                        let mut visited_xref = HashSet::new();
                        let mut hops = 0usize;

                        while let Some(pos) = next_pos {
                            if pos >= data.len() || !visited_xref.insert(pos) || hops > 32 {
                                break;
                            }
                            hops += 1;
                            next_pos = None;

                            let mut xcur = Cursor::new(data, pos);
                            xcur.skip_ws();
                            if xcur.pos < data.len() && data[xcur.pos..].starts_with(b"xref") {
                                xcur.pos += 4;
                                loop {
                                    xcur.skip_ws();
                                    if xcur.pos >= data.len()
                                        || data[xcur.pos..].starts_with(b"trailer")
                                    {
                                        break;
                                    }
                                    let start_id = match xcur.parse_val(0) {
                                        Some(PdfVal::Int(n)) if n >= 0 => n as u32,
                                        _ => break,
                                    };
                                    let count = match xcur.parse_val(0) {
                                        Some(PdfVal::Int(n)) if n >= 0 => n as u32,
                                        _ => break,
                                    };
                                    for i in 0..count {
                                        let off_w = xcur.read_word();
                                        let _gen_w = xcur.read_word();
                                        let flag_w = xcur.read_word();
                                        if flag_w.is_empty() {
                                            break;
                                        }
                                        let oid = start_id + i;
                                        if seen_ids.insert(oid) && flag_w == b"n" {
                                            if let Ok(off) = std::str::from_utf8(off_w)
                                                .unwrap_or("")
                                                .parse::<usize>()
                                            {
                                                if off > 0 && off < data.len() {
                                                    obj_offsets.insert(oid, off);
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(rel_tr) = find_subslice(&data[xcur.pos..], b"trailer") {
                                    let mut tcur = Cursor::new(data, xcur.pos + rel_tr + 7);
                                    if let Some(tr_val) = tcur.parse_val(0) {
                                        if doc.root_ref.is_none() {
                                            doc.root_ref =
                                                tr_val.dict_get("Root").and_then(|v| v.as_ref_id());
                                        }
                                        if doc.info_ref.is_none() {
                                            doc.info_ref =
                                                tr_val.dict_get("Info").and_then(|v| v.as_ref_id());
                                        }
                                        if tr_val.dict_get("Encrypt").is_some() {
                                            doc.encrypted = true;
                                        }
                                        if let Some(prev) =
                                            tr_val.dict_get("Prev").and_then(|v| v.as_int())
                                        {
                                            if prev >= 0 {
                                                next_pos = Some(prev as usize);
                                            }
                                        }
                                    }
                                }
                            } else if let Some((_, xref_obj, _)) =
                                parse_indirect_obj_at(data, xcur.pos, None)
                            {
                                let is_xref = xref_obj
                                    .val
                                    .dict_get("Type")
                                    .and_then(|v| v.as_name_decoded())
                                    .as_deref()
                                    == Some("XRef");
                                if is_xref {
                                    if doc.root_ref.is_none() {
                                        doc.root_ref = xref_obj
                                            .val
                                            .dict_get("Root")
                                            .and_then(|v| v.as_ref_id());
                                    }
                                    if doc.info_ref.is_none() {
                                        doc.info_ref = xref_obj
                                            .val
                                            .dict_get("Info")
                                            .and_then(|v| v.as_ref_id());
                                    }
                                    if xref_obj.val.dict_get("Encrypt").is_some() {
                                        doc.encrypted = true;
                                    }
                                    if let Some(prev) =
                                        xref_obj.val.dict_get("Prev").and_then(|v| v.as_int())
                                    {
                                        if prev >= 0 {
                                            next_pos = Some(prev as usize);
                                        }
                                    }

                                    let size = xref_obj
                                        .val
                                        .dict_get("Size")
                                        .and_then(|v| v.as_int())
                                        .unwrap_or(0)
                                        .max(0) as u32;
                                    let widths: Vec<usize> = xref_obj
                                        .val
                                        .dict_get("W")
                                        .and_then(|v| v.as_array())
                                        .map(|arr| {
                                            arr.iter()
                                                .filter_map(|x| x.as_int())
                                                .map(|n| n.max(0) as usize)
                                                .collect()
                                        })
                                        .unwrap_or_default();
                                    let index_pairs: Vec<(u32, u32)> = xref_obj
                                        .val
                                        .dict_get("Index")
                                        .and_then(|v| v.as_array())
                                        .map(|arr| {
                                            arr.chunks_exact(2)
                                                .filter_map(|c| {
                                                    let s = c[0].as_int()?;
                                                    let cnt = c[1].as_int()?;
                                                    Some((s.max(0) as u32, cnt.max(0) as u32))
                                                })
                                                .collect()
                                        })
                                        .unwrap_or_else(|| vec![(0, size)]);

                                    if widths.len() == 3 {
                                        let entry_len = widths[0] + widths[1] + widths[2];
                                        if entry_len > 0 {
                                            if let Some(decoded) =
                                                decompress_stream_payload(&xref_obj)
                                            {
                                                let mut cursor = 0usize;
                                                for (start_id, count) in index_pairs {
                                                    for i in 0..count {
                                                        if cursor + entry_len > decoded.len() {
                                                            break;
                                                        }
                                                        let rec = &decoded
                                                            [cursor..cursor + entry_len];
                                                        cursor += entry_len;
                                                        let f0 = if widths[0] == 0 {
                                                            1
                                                        } else {
                                                            read_be_uint(&rec[..widths[0]])
                                                        };
                                                        let f1 = read_be_uint(
                                                            &rec[widths[0]..widths[0] + widths[1]],
                                                        );
                                                        let f2 = read_be_uint(
                                                            &rec[widths[0] + widths[1]..entry_len],
                                                        );
                                                        let oid = start_id + i;
                                                        if seen_ids.insert(oid) {
                                                            match f0 {
                                                                1 if f1 > 0
                                                                    && (f1 as usize)
                                                                        < data.len() =>
                                                                {
                                                                    obj_offsets
                                                                        .insert(oid, f1 as usize);
                                                                }
                                                                2 => {
                                                                    compressed_locs.insert(
                                                                        oid,
                                                                        (f1 as u32, f2 as usize),
                                                                    );
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Parse top-level objects discovered via xref/XRef (two passes so indirect /Length refs resolve!)
            if !obj_offsets.is_empty() {
                for (&oid, &off) in &obj_offsets {
                    if let Some((parsed_id, obj, _)) = parse_indirect_obj_at(data, off, None) {
                        if parsed_id == oid {
                            let is_xref = obj
                                .val
                                .dict_get("Type")
                                .and_then(|v| v.as_name_decoded())
                                .as_deref()
                                == Some("XRef");
                            if !is_xref {
                                doc.objects.insert(oid, obj);
                            }
                        }
                    }
                }
                // Second pass for any stream objects whose /Length was an indirect reference
                let snap = doc.objects.clone();
                for (&oid, &off) in &obj_offsets {
                    if let Some(existing) = doc.objects.get(&oid) {
                        if matches!(existing.val.dict_get("Length"), Some(PdfVal::Ref(_, _))) {
                            if let Some((parsed_id, obj, _)) =
                                parse_indirect_obj_at(data, off, Some(&snap))
                            {
                                if parsed_id == oid {
                                    doc.objects.insert(oid, obj);
                                }
                            }
                        }
                    }
                }
            }

            // 3. Fallback top-level scan if xref was missing/broken or didn't yield objects
            if doc.objects.is_empty() || doc.root_ref.is_none() {
                let mut pos = 0usize;
                while pos < data.len() {
                    if let Some(rel) = find_subslice(&data[pos..], b" obj") {
                        let kw_pos = pos + rel;
                        // Walk backwards up to 24 bytes to find start of `<id> <gen> obj`
                        let line_start = data[..kw_pos]
                            .iter()
                            .rposition(|&b| b == b'\n' || b == b'\r')
                            .map(|p| p + 1)
                            .unwrap_or(0);
                        let candidate_start = line_start.max(kw_pos.saturating_sub(24));
                        // Advance to first digit on that line
                        let mut digit_start = candidate_start;
                        while digit_start < kw_pos && is_ws(data[digit_start]) {
                            digit_start += 1;
                        }
                        if digit_start < kw_pos && data[digit_start].is_ascii_digit() {
                            if let Some((oid, obj, end_pos)) =
                                parse_indirect_obj_at(data, digit_start, None)
                            {
                                let is_xref = obj
                                    .val
                                    .dict_get("Type")
                                    .and_then(|v| v.as_name_decoded())
                                    .as_deref()
                                    == Some("XRef");
                                if !is_xref {
                                    if doc.root_ref.is_none()
                                        && obj
                                            .val
                                            .dict_get("Type")
                                            .and_then(|v| v.as_name_decoded())
                                            .as_deref()
                                            == Some("Catalog")
                                    {
                                        doc.root_ref = Some(oid);
                                    }
                                    doc.objects.entry(oid).or_insert(obj);
                                }
                                pos = end_pos.max(kw_pos + 4);
                                continue;
                            }
                        }
                        pos = kw_pos + 4;
                    } else {
                        break;
                    }
                }
            }

            // 4. Unpack all `/Type /ObjStm` object streams
            let objstm_ids: Vec<u32> = doc
                .objects
                .iter()
                .filter(|(_, obj)| {
                    obj.val
                        .dict_get("Type")
                        .and_then(|v| v.as_name_decoded())
                        .as_deref()
                        == Some("ObjStm")
                })
                .map(|(&id, _)| id)
                .collect();

            for stm_id in objstm_ids {
                let Some(stm_obj) = doc.objects.remove(&stm_id) else {
                    continue;
                };
                let n = stm_obj
                    .val
                    .dict_get("N")
                    .and_then(|v| v.as_int())
                    .unwrap_or(0)
                    .max(0) as usize;
                let first = stm_obj
                    .val
                    .dict_get("First")
                    .and_then(|v| v.as_int())
                    .unwrap_or(0)
                    .max(0) as usize;
                let Some(decoded) = decompress_stream_payload(&stm_obj) else {
                    continue;
                };
                if first > decoded.len() || n == 0 {
                    continue;
                }
                let mut hcur = Cursor::new(&decoded[..first], 0);
                let mut entries = Vec::with_capacity(n);
                for _ in 0..n {
                    let Some(PdfVal::Int(oid_i)) = hcur.parse_val(0) else {
                        break;
                    };
                    let Some(PdfVal::Int(rel_i)) = hcur.parse_val(0) else {
                        break;
                    };
                    if oid_i >= 0 && rel_i >= 0 {
                        entries.push((oid_i as u32, first + rel_i as usize));
                    }
                }
                for idx in 0..entries.len() {
                    let (oid, start) = entries[idx];
                    if start >= decoded.len() {
                        continue;
                    }
                    let end = if idx + 1 < entries.len() {
                        entries[idx + 1].1.min(decoded.len())
                    } else {
                        decoded.len()
                    };
                    if start >= end {
                        continue;
                    }
                    // Respect compressed_locs if present; otherwise insert
                    let allow = compressed_locs
                        .get(&oid)
                        .map(|&(target_stm, target_idx)| target_stm == stm_id && target_idx == idx)
                        .unwrap_or(true);
                    if allow {
                        let mut vcur = Cursor::new(&decoded[start..end], 0);
                        if let Some(val) = vcur.parse_val(0) {
                            doc.objects.entry(oid).or_insert(PdfObj { val, stream: None });
                        }
                    }
                }
            }

            Some(doc)
        }

        fn resolve_dict<'a>(&'a self, val: &'a PdfVal) -> Option<&'a PdfVal> {
            match val {
                PdfVal::Dict(_) => Some(val),
                PdfVal::Ref(id, _) => self.objects.get(id).map(|o| &o.val).filter(|v| v.as_dict().is_some()),
                _ => None,
            }
        }

        fn collect_pages_recursive(
            &self,
            node_id: u32,
            inherited_mediabox: Option<PdfVal>,
            inherited_cropbox: Option<PdfVal>,
            inherited_resources: Option<PdfVal>,
            visited: &mut HashSet<u32>,
            out: &mut Vec<PageNodeInfo>,
        ) {
            if !visited.insert(node_id) {
                return;
            }
            let Some(obj) = self.objects.get(&node_id) else {
                return;
            };
            let typ = obj
                .val
                .dict_get("Type")
                .and_then(|v| v.as_name_decoded());

            let cur_mb = obj.val.dict_get("MediaBox").cloned().or(inherited_mediabox);
            let cur_cb = obj.val.dict_get("CropBox").cloned().or(inherited_cropbox);
            let cur_res = obj.val.dict_get("Resources").cloned().or(inherited_resources);

            if typ.as_deref() == Some("Pages") || obj.val.dict_get("Kids").is_some() {
                if let Some(kids) = obj.val.dict_get("Kids").and_then(|v| v.as_array()) {
                    for kid in kids {
                        if let Some(kid_id) = kid.as_ref_id() {
                            self.collect_pages_recursive(
                                kid_id,
                                cur_mb.clone(),
                                cur_cb.clone(),
                                cur_res.clone(),
                                visited,
                                out,
                            );
                        }
                    }
                }
            } else if typ.as_deref() == Some("Page") || obj.val.dict_get("Contents").is_some() {
                out.push(PageNodeInfo {
                    obj_id: node_id,
                    media_box: cur_mb,
                    crop_box: cur_cb,
                    resources: cur_res,
                });
            }
        }

        fn count_pages(&self) -> Option<i32> {
            // 1. Authoritative: Catalog -> /Pages -> /Count (or recursive walk)
            if let Some(root_id) = self.root_ref {
                if let Some(root_obj) = self.objects.get(&root_id) {
                    if let Some(pages_id) =
                        root_obj.val.dict_get("Pages").and_then(|v| v.as_ref_id())
                    {
                        if let Some(pages_obj) = self.objects.get(&pages_id) {
                            if let Some(cnt) = pages_obj.val.dict_get("Count").and_then(|v| v.as_int())
                            {
                                if cnt > 0 {
                                    return i32::try_from(cnt).ok();
                                }
                            }
                        }
                        let mut collected = Vec::new();
                        let mut visited = HashSet::new();
                        self.collect_pages_recursive(
                            pages_id,
                            None,
                            None,
                            None,
                            &mut visited,
                            &mut collected,
                        );
                        if !collected.is_empty() {
                            return i32::try_from(collected.len()).ok();
                        }
                    }
                }
            }

            // 2. Fallback: max `/Type /Pages ... /Count` across all parsed objects (including ObjStm)
            let max_pages_count = self
                .objects
                .values()
                .filter(|obj| {
                    obj.val
                        .dict_get("Type")
                        .and_then(|v| v.as_name_decoded())
                        .as_deref()
                        == Some("Pages")
                })
                .filter_map(|obj| obj.val.dict_get("Count").and_then(|v| v.as_int()))
                .filter(|&c| c > 0)
                .max();
            if let Some(cnt) = max_pages_count {
                return i32::try_from(cnt).ok();
            }

            // 3. Fallback: count `/Type /Page` objects excluding `/Type /Pages`
            let leaf_pages = self
                .objects
                .values()
                .filter(|obj| {
                    obj.val
                        .dict_get("Type")
                        .and_then(|v| v.as_name_decoded())
                        .as_deref()
                        == Some("Page")
                })
                .count();
            if leaf_pages > 0 {
                return i32::try_from(leaf_pages).ok();
            }

            None
        }
    }

    struct PageNodeInfo {
        obj_id: u32,
        media_box: Option<PdfVal>,
        crop_box: Option<PdfVal>,
        resources: Option<PdfVal>,
    }

    pub fn extract_page_count(pdf_bytes: &[u8]) -> Option<i32> {
        if let Some(doc) = PdfDoc::parse(pdf_bytes) {
            if let Some(cnt) = doc.count_pages() {
                return Some(cnt);
            }
        }
        // Final minimal heuristic fallback for partial raw test snippets without `%PDF-` / full objects
        extract_page_count_regex_fallback(pdf_bytes)
    }

    fn extract_page_count_regex_fallback(pdf_bytes: &[u8]) -> Option<i32> {
        let head = &pdf_bytes[..pdf_bytes.len().min(1024)];
        if find_subslice(head, b"%PDF-").is_none() {
            return None;
        }
        let text = String::from_utf8_lossy(pdf_bytes);
        let pages_re = regex::Regex::new(r"/Type\s*/Pages\b[^<>]*?/Count\s+(\d+)").ok()?;
        let max_cnt = pages_re
            .captures_iter(&text)
            .filter_map(|c| c.get(1).and_then(|m| m.as_str().parse::<i32>().ok()))
            .filter(|&n| n > 0)
            .max();
        if max_cnt.is_some() {
            return max_cnt;
        }
        let page_re = regex::Regex::new(r"/Type\s*/Page\b").ok()?;
        let cnt = page_re.find_iter(&text).count();
        if cnt > 0 {
            i32::try_from(cnt).ok()
        } else {
            None
        }
    }

    fn parse_box_coords(doc: &PdfDoc, val: Option<&PdfVal>) -> Option<[f64; 4]> {
        let arr = match val? {
            PdfVal::Array(a) => a,
            PdfVal::Ref(id, _) => doc.objects.get(id)?.val.as_array()?,
            _ => return None,
        };
        if arr.len() < 4 {
            return None;
        }
        Some([
            arr[0].as_f64()?,
            arr[1].as_f64()?,
            arr[2].as_f64()?,
            arr[3].as_f64()?,
        ])
    }

    fn build_overlay_stream(
        box_coords: [f64; 4],
        font_key: &str,
        gs_diag_key: &str,
        gs_bar_key: &str,
        watermark_text: &str,
        watermark_code: &str,
    ) -> Vec<u8> {
        let [bx0, by0, bx1, by1] = box_coords;
        let x0 = bx0.min(bx1);
        let y0 = by0.min(by1);
        let x1 = bx0.max(bx1);
        let y1 = by0.max(by1);
        let w = (x1 - x0).max(200.0);
        let h = (y1 - y0).max(200.0);

        let clean_text = sanitize_pdf_text(watermark_text);
        let clean_code = sanitize_pdf_text(watermark_code);
        let combined_line = if clean_text.is_empty() {
            format!("MSI DRM LISANS: {}", clean_code)
        } else if !clean_code.is_empty() && !clean_text.contains(&clean_code) {
            format!("{} | {}", clean_text, clean_code)
        } else {
            clean_text.clone()
        };

        // Diagonal scale tuned so the line stays comfortably inside page width
        let diag_text_len = combined_line.len().max(24) as f64;
        let diag_font_size = ((w * 0.92) / (diag_text_len * 0.55)).clamp(8.0, 14.0);
        let diag_sub_size = (diag_font_size * 0.85).max(7.5);
        let diag_x1 = x0 + w * 0.11;
        let diag_y1 = y0 + h * 0.28;
        let diag_x2 = x0 + w * 0.14;
        let diag_y2 = y0 + h * 0.56;

        // Bottom footer stamp wrapped cleanly across 1-2 lines
        let max_footer_chars = ((w - 36.0) / 3.9).floor().max(64.0) as usize;
        let footer_lines = wrap_text(&combined_line, max_footer_chars);
        let bar_h = if footer_lines.len() > 1 { 30.0 } else { 20.0 };
        let bar_x = x0 + 12.0;
        let bar_y = y0 + 8.0;
        let bar_w = (w - 24.0).max(160.0);

        let mut s = String::with_capacity(1024);
        // Balance any page-level CTM / clipping state from original content streams
        s.push_str("\nQ\nq\n");

        // 1. Semi-transparent diagonal watermarks (2 bands)
        s.push_str(&format!(
            "q\n/{gs_diag} gs\n0.55 0.08 0.14 rg\nBT\n/{font} {fs:.2} Tf\n0.819 0.574 -0.574 0.819 {x:.2} {y:.2} Tm\n({txt}) Tj\nET\nQ\n",
            gs_diag = gs_diag_key,
            font = font_key,
            fs = diag_font_size,
            x = diag_x1,
            y = diag_y1,
            txt = escape_pdf_string(&combined_line),
        ));
        s.push_str(&format!(
            "q\n/{gs_diag} gs\n0.20 0.25 0.36 rg\nBT\n/{font} {fs:.2} Tf\n0.819 0.574 -0.574 0.819 {x:.2} {y:.2} Tm\n({txt}) Tj\nET\nQ\n",
            gs_diag = gs_diag_key,
            font = font_key,
            fs = diag_sub_size,
            x = diag_x2,
            y = diag_y2,
            txt = escape_pdf_string(&combined_line),
        ));

        // 2. Crisp bottom footer stamp bar
        s.push_str(&format!(
            "q\n/{gs_bar} gs\n0.06 0.09 0.16 rg\n{bx:.2} {by:.2} {bw:.2} {bh:.2} re f\n0.78 0.12 0.18 RG\n1.0 w\n{bx:.2} {top:.2} m {bx2:.2} {top:.2} l S\n",
            gs_bar = gs_bar_key,
            bx = bar_x,
            by = bar_y,
            bw = bar_w,
            bh = bar_h,
            top = bar_y + bar_h,
            bx2 = bar_x + bar_w,
        ));
        let mut text_y = bar_y + bar_h - 10.5;
        for line in footer_lines.iter().take(2) {
            s.push_str(&format!(
                "BT\n0.96 0.86 0.52 rg\n/{font} 6.8 Tf\n{tx:.2} {ty:.2} Td\n({txt}) Tj\nET\n",
                font = font_key,
                tx = bar_x + 8.0,
                ty = text_y,
                txt = escape_pdf_string(line),
            ));
            text_y -= 9.5;
        }
        s.push_str("Q\nQ\n");
        s.into_bytes()
    }

    pub fn watermark_and_slice(
        pdf_bytes: &[u8],
        watermark_text: &str,
        watermark_code: &str,
        page_range: Option<(usize, usize)>,
    ) -> Vec<u8> {
        let Some(mut doc) = PdfDoc::parse(pdf_bytes) else {
            return pdf_bytes.to_vec();
        };
        if doc.encrypted {
            return pdf_bytes.to_vec();
        }
        let Some(root_id) = doc.root_ref else {
            return pdf_bytes.to_vec();
        };
        let Some(pages_root_id) = doc
            .objects
            .get(&root_id)
            .and_then(|o| o.val.dict_get("Pages"))
            .and_then(|v| v.as_ref_id())
        else {
            return pdf_bytes.to_vec();
        };

        let mut all_pages = Vec::new();
        let mut visited = HashSet::new();
        doc.collect_pages_recursive(pages_root_id, None, None, None, &mut visited, &mut all_pages);
        if all_pages.is_empty() {
            return pdf_bytes.to_vec();
        }

        let total_pages = all_pages.len();
        let (selected_pages, is_sliced) = match page_range {
            Some((req_start, req_end)) => {
                let lo = req_start.max(1).min(total_pages);
                let hi = req_end.max(lo).min(total_pages);
                (
                    all_pages[(lo - 1)..hi].iter().collect::<Vec<_>>(),
                    lo > 1 || hi < total_pages,
                )
            }
            None => (all_pages.iter().collect::<Vec<_>>(), false),
        };

        // Allocate fresh object IDs safely above existing max ID
        let mut next_id = doc.objects.keys().copied().max().unwrap_or(1) + 1;
        let mut alloc_id = || {
            let id = next_id;
            next_id += 1;
            id
        };

        // Shared Helvetica Type1 font, 2 ExtGStates (diagonal & footer bar), and `q` prefix stream
        let helvetica_id = alloc_id();
        doc.objects.insert(
            helvetica_id,
            PdfObj {
                val: PdfVal::Dict(vec![
                    ("Type".into(), PdfVal::Name("Font".into())),
                    ("Subtype".into(), PdfVal::Name("Type1".into())),
                    ("BaseFont".into(), PdfVal::Name("Helvetica".into())),
                    (
                        "Encoding".into(),
                        PdfVal::Name("WinAnsiEncoding".into()),
                    ),
                ]),
                stream: None,
            },
        );

        let gs_diag_id = alloc_id();
        doc.objects.insert(
            gs_diag_id,
            PdfObj {
                val: PdfVal::Dict(vec![
                    ("Type".into(), PdfVal::Name("ExtGState".into())),
                    ("ca".into(), PdfVal::Real("0.22".into())),
                    ("CA".into(), PdfVal::Real("0.22".into())),
                    ("BM".into(), PdfVal::Name("Normal".into())),
                ]),
                stream: None,
            },
        );

        let gs_bar_id = alloc_id();
        doc.objects.insert(
            gs_bar_id,
            PdfObj {
                val: PdfVal::Dict(vec![
                    ("Type".into(), PdfVal::Name("ExtGState".into())),
                    ("ca".into(), PdfVal::Real("0.88".into())),
                    ("CA".into(), PdfVal::Real("0.88".into())),
                    ("BM".into(), PdfVal::Name("Normal".into())),
                ]),
                stream: None,
            },
        );

        let prefix_q_id = alloc_id();
        doc.objects.insert(
            prefix_q_id,
            PdfObj {
                val: PdfVal::Dict(vec![("Length".into(), PdfVal::Int(2))]),
                stream: Some(b"q\n".to_vec()),
            },
        );

        // Process each selected page: flatten /Parent, materialize inherited MediaBox/Resources,
        // attach Helvetica + ExtGState resources, and append visible overlay stream
        let mut new_kids = Vec::with_capacity(selected_pages.len());
        let mut overlay_cache: HashMap<String, u32> = HashMap::new();

        for page_info in &selected_pages {
            let page_id = page_info.obj_id;
            new_kids.push(PdfVal::Ref(page_id, 0));

            let effective_box = parse_box_coords(&doc, page_info.crop_box.as_ref())
                .or_else(|| parse_box_coords(&doc, page_info.media_box.as_ref()))
                .unwrap_or([0.0, 0.0, 595.0, 842.0]);

            // Resolve page's existing Resources dictionary (may be inline or indirect, or inherited)
            let mut res_dict = page_info
                .resources
                .as_ref()
                .and_then(|r| doc.resolve_dict(r).cloned())
                .unwrap_or_else(|| PdfVal::Dict(vec![]));

            // Resolve /Font sub-dict inside /Resources
            let mut font_dict = res_dict
                .dict_get("Font")
                .and_then(|f| doc.resolve_dict(f).cloned())
                .unwrap_or_else(|| PdfVal::Dict(vec![]));

            let existing_font_keys: HashSet<String> = font_dict
                .as_dict()
                .map(|d| d.iter().map(|(k, _)| decode_pdf_name(k)).collect())
                .unwrap_or_default();

            let font_key = if !existing_font_keys.contains("F1") {
                "F1".to_string()
            } else {
                let mut idx = 1usize;
                loop {
                    let cand = format!("FMSI{}", idx);
                    if !existing_font_keys.contains(&cand) {
                        break cand;
                    }
                    idx += 1;
                }
            };
            font_dict.dict_set(&font_key, PdfVal::Ref(helvetica_id, 0));
            res_dict.dict_set("Font", font_dict);

            // Resolve /ExtGState sub-dict inside /Resources
            let mut gs_dict = res_dict
                .dict_get("ExtGState")
                .and_then(|g| doc.resolve_dict(g).cloned())
                .unwrap_or_else(|| PdfVal::Dict(vec![]));

            let existing_gs_keys: HashSet<String> = gs_dict
                .as_dict()
                .map(|d| d.iter().map(|(k, _)| decode_pdf_name(k)).collect())
                .unwrap_or_default();

            let gs_diag_key = if !existing_gs_keys.contains("GSWM1") {
                "GSWM1".to_string()
            } else {
                let mut idx = 1usize;
                loop {
                    let cand = format!("GSWM_D{}", idx);
                    if !existing_gs_keys.contains(&cand) {
                        break cand;
                    }
                    idx += 1;
                }
            };
            let gs_bar_key = if !existing_gs_keys.contains("GSWM2") {
                "GSWM2".to_string()
            } else {
                let mut idx = 1usize;
                loop {
                    let cand = format!("GSWM_B{}", idx);
                    if !existing_gs_keys.contains(&cand) {
                        break cand;
                    }
                    idx += 1;
                }
            };
            gs_dict.dict_set(&gs_diag_key, PdfVal::Ref(gs_diag_id, 0));
            gs_dict.dict_set(&gs_bar_key, PdfVal::Ref(gs_bar_id, 0));
            res_dict.dict_set("ExtGState", gs_dict);

            // Reuse identical overlay stream across same-sized pages with identical resource names
            let cache_key = format!(
                "{:.2},{:.2},{:.2},{:.2}|{}|{}|{}",
                effective_box[0],
                effective_box[1],
                effective_box[2],
                effective_box[3],
                font_key,
                gs_diag_key,
                gs_bar_key
            );
            let overlay_id = if let Some(&cached) = overlay_cache.get(&cache_key) {
                cached
            } else {
                let overlay_bytes = build_overlay_stream(
                    effective_box,
                    &font_key,
                    &gs_diag_key,
                    &gs_bar_key,
                    watermark_text,
                    watermark_code,
                );
                let oid = alloc_id();
                doc.objects.insert(
                    oid,
                    PdfObj {
                        val: PdfVal::Dict(vec![("Length".into(), PdfVal::Int(overlay_bytes.len() as i64))]),
                        stream: Some(overlay_bytes),
                    },
                );
                overlay_cache.insert(cache_key, oid);
                oid
            };

            // Resolve original `/Contents` into a list of stream refs
            let orig_content_refs: Vec<PdfVal> = match doc
                .objects
                .get(&page_id)
                .and_then(|o| o.val.dict_get("Contents"))
            {
                Some(PdfVal::Ref(cid, cgen)) => {
                    if let Some(arr) = doc.objects.get(cid).and_then(|o| o.val.as_array()) {
                        arr.clone()
                    } else {
                        vec![PdfVal::Ref(*cid, *cgen)]
                    }
                }
                Some(PdfVal::Array(arr)) => arr.clone(),
                _ => vec![],
            };

            let mut new_contents = Vec::with_capacity(orig_content_refs.len() + 2);
            new_contents.push(PdfVal::Ref(prefix_q_id, 0));
            new_contents.extend(orig_content_refs);
            new_contents.push(PdfVal::Ref(overlay_id, 0));

            if let Some(page_obj) = doc.objects.get_mut(&page_id) {
                page_obj
                    .val
                    .dict_set("Parent", PdfVal::Ref(pages_root_id, 0));
                if page_obj.val.dict_get("MediaBox").is_none() {
                    if let Some(ref mb) = page_info.media_box {
                        page_obj.val.dict_set("MediaBox", mb.clone());
                    }
                }
                if page_obj.val.dict_get("CropBox").is_none() {
                    if let Some(ref cb) = page_info.crop_box {
                        page_obj.val.dict_set("CropBox", cb.clone());
                    }
                }
                page_obj.val.dict_set("Resources", res_dict);
                page_obj
                    .val
                    .dict_set("Contents", PdfVal::Array(new_contents));
            }
        }

        // Update root /Pages node to point directly to selected pages and exact count
        if let Some(pages_obj) = doc.objects.get_mut(&pages_root_id) {
            pages_obj
                .val
                .dict_set("Type", PdfVal::Name("Pages".into()));
            pages_obj
                .val
                .dict_set("Kids", PdfVal::Array(new_kids));
            pages_obj
                .val
                .dict_set("Count", PdfVal::Int(selected_pages.len() as i64));
            // Drop stale inherited boxes on intermediate root if pushed down, or keep harmlessly
        }

        // Prune catalog structure refs that could dangle when sliced
        if is_sliced {
            if let Some(root_obj) = doc.objects.get_mut(&root_id) {
                root_obj.val.dict_remove("StructTreeRoot");
                root_obj.val.dict_remove("Outlines");
                root_obj.val.dict_remove("Dests");
                root_obj.val.dict_remove("Threads");
                root_obj.val.dict_remove("OpenAction");
                root_obj.val.dict_remove("AA");
            }
        }

        // Update or create `/Info` dictionary with `/Subject` and `/Keywords`
        let clean_wm_ascii = sanitize_pdf_text(watermark_text);
        let clean_code_ascii = sanitize_pdf_text(watermark_code);
        let info_id = match doc.info_ref.filter(|id| doc.objects.contains_key(id)) {
            Some(id) => id,
            None => {
                let id = alloc_id();
                doc.objects.insert(
                    id,
                    PdfObj {
                        val: PdfVal::Dict(vec![(
                            "Producer".into(),
                            PdfVal::LitStr(b"MSI Dergisi Rust PDF DRM Engine".to_vec()),
                        )]),
                        stream: None,
                    },
                );
                doc.info_ref = Some(id);
                id
            }
        };

        if let Some(info_obj) = doc.objects.get_mut(&info_id) {
            let subject_text = format!("{} | DRM: {}", clean_wm_ascii, clean_code_ascii);
            let existing_kw = info_obj
                .val
                .dict_get("Keywords")
                .and_then(|v| match v {
                    PdfVal::LitStr(b) => Some(sanitize_pdf_text(&String::from_utf8_lossy(b))),
                    _ => None,
                })
                .unwrap_or_default();
            let keywords_text = if existing_kw.is_empty() {
                format!("MSI-DRM, {}, {}", clean_code_ascii, clean_wm_ascii)
            } else {
                format!("{}, MSI-DRM, {}", existing_kw, clean_code_ascii)
            };
            info_obj
                .val
                .dict_set("Subject", PdfVal::LitStr(subject_text.into_bytes()));
            info_obj
                .val
                .dict_set("Keywords", PdfVal::LitStr(keywords_text.into_bytes()));
        }

        // Collect reachable objects from Catalog & Info, and renumber cleanly 1..=M
        let mut reachable_order = Vec::new();
        let mut visited_ids = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(root_id);
        queue.push_back(info_id);

        while let Some(oid) = queue.pop_front() {
            if !visited_ids.insert(oid) {
                continue;
            }
            if let Some(obj) = doc.objects.get(&oid) {
                reachable_order.push(oid);
                let mut refs = Vec::new();
                obj.val.collect_refs(&mut refs);
                for r in refs {
                    if !visited_ids.contains(&r) {
                        queue.push_back(r);
                    }
                }
            }
        }

        let mut id_map: HashMap<u32, u32> = HashMap::with_capacity(reachable_order.len());
        for (idx, &old_id) in reachable_order.iter().enumerate() {
            id_map.insert(old_id, (idx + 1) as u32);
        }

        let new_root = *id_map.get(&root_id).unwrap_or(&1);
        let new_info = *id_map.get(&info_id).unwrap_or(&2);
        let total_objs = reachable_order.len();

        let version = match doc.header_version.as_str() {
            "1.0" | "1.1" | "1.2" | "1.3" => "1.4",
            other => other,
        };

        let mut out: Vec<u8> = Vec::with_capacity(pdf_bytes.len() + 4096);
        out.extend_from_slice(format!("%PDF-{}\n", version).as_bytes());
        out.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");

        let mut offsets = vec![0usize; total_objs + 1];
        for &old_id in &reachable_order {
            let new_id = id_map[&old_id];
            let mut obj = doc.objects.remove(&old_id).unwrap();
            obj.val.remap_refs(&id_map);
            if let Some(ref st) = obj.stream {
                obj.val.dict_set("Length", PdfVal::Int(st.len() as i64));
            }
            offsets[new_id as usize] = out.len();
            out.extend_from_slice(format!("{} 0 obj\n", new_id).as_bytes());
            obj.val.serialize(&mut out);
            out.push(b'\n');
            if let Some(st) = obj.stream {
                out.extend_from_slice(b"stream\n");
                out.extend_from_slice(&st);
                if !st.ends_with(b"\n") {
                    out.push(b'\n');
                }
                out.extend_from_slice(b"endstream\n");
            }
            out.extend_from_slice(b"endobj\n");
        }

        let xref_offset = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", total_objs + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for off in offsets.iter().skip(1) {
            out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
        }

        let trailer = format!(
            "trailer\n<< /Size {} /Root {} 0 R /Info {} 0 R >>\nstartxref\n{}\n%MSI-DRM:{}\n%%EOF\n",
            total_objs + 1,
            new_root,
            new_info,
            xref_offset,
            clean_code_ascii
        );
        out.extend_from_slice(trailer.as_bytes());

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_multipage_pdf(page_count: usize, compress_pages_objstm: bool) -> Vec<u8> {
        // Minimal standard multi-page PDF 1.4/1.5 fixture for deterministic unit tests
        let mut buf: Vec<u8> = Vec::new();
        let mut offsets: Vec<(u32, usize)> = Vec::new();

        buf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

        // 1: Catalog
        offsets.push((1, buf.len()));
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // 3: Font
        offsets.push((3, buf.len()));
        buf.extend_from_slice(
            b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n",
        );

        let mut kids = Vec::new();
        for i in 0..page_count {
            let page_id = 4 + (i * 2) as u32;
            let content_id = page_id + 1;
            kids.push(format!("{} 0 R", page_id));

            offsets.push((page_id, buf.len()));
            buf.extend_from_slice(
                format!(
                    "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>\nendobj\n",
                    page_id, content_id
                )
                .as_bytes(),
            );

            let stream_str = format!("BT /F1 12 Tf 72 720 Td (Original Page {} Body) Tj ET\n", i + 1);
            offsets.push((content_id, buf.len()));
            buf.extend_from_slice(
                format!(
                    "{} 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
                    content_id,
                    stream_str.len(),
                    stream_str
                )
                .as_bytes(),
            );
        }

        if compress_pages_objstm {
            // Put object 2 (`/Type /Pages`) inside an uncompressed `/Type /ObjStm` (valid PDF 1.5 without deflate encoder)
            let pages_body = format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>",
                kids.join(" "),
                page_count
            );
            let header = b"2 0 ";
            let mut stm = Vec::new();
            stm.extend_from_slice(header);
            stm.extend_from_slice(pages_body.as_bytes());
            let objstm_id = 4 + (page_count * 2) as u32;
            offsets.push((objstm_id, buf.len()));
            buf.extend_from_slice(
                format!(
                    "{} 0 obj\n<< /Type /ObjStm /N 1 /First {} /Length {} >>\nstream\n",
                    objstm_id,
                    header.len(),
                    stm.len()
                )
                .as_bytes(),
            );
            buf.extend_from_slice(&stm);
            buf.extend_from_slice(b"\nendstream\nendobj\n");
        } else {
            offsets.push((2, buf.len()));
            buf.extend_from_slice(
                format!(
                    "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
                    kids.join(" "),
                    page_count
                )
                .as_bytes(),
            );
        }

        offsets.sort_by_key(|(id, _)| *id);
        let max_id = offsets.last().map(|(id, _)| *id).unwrap_or(1);
        let mut table = vec![0usize; (max_id + 1) as usize];
        for (id, off) in offsets {
            table[id as usize] = off;
        }

        let xref_off = buf.len();
        buf.extend_from_slice(format!("xref\n0 {}\n", max_id + 1).as_bytes());
        buf.extend_from_slice(b"0000000000 65535 f \n");
        for &off in table.iter().skip(1) {
            if off == 0 {
                buf.extend_from_slice(b"0000000000 00000 f \n");
            } else {
                buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
            }
        }
        buf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                max_id + 1,
                xref_off
            )
            .as_bytes(),
        );
        buf
    }

    #[test]
    fn extract_pdf_page_count_reads_128_page_catalog_and_nested_trees() {
        let catalog_128 = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 128 >>\nendobj\n%%EOF";
        assert_eq!(extract_pdf_page_count(catalog_128), Some(128));

        // Nested /Pages tree where root /Pages has /Count 128 and child /Pages has /Count 64
        let nested = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 128 >>\nendobj\n3 0 obj\n<< /Type /Pages /Parent 2 0 R /Kids [] /Count 64 >>\nendobj\n4 0 obj\n<< /Type /Pages /Parent 2 0 R /Kids [] /Count 64 >>\nendobj\n%%EOF";
        assert_eq!(extract_pdf_page_count(nested), Some(128));

        // Page counting fallback excluding /Pages when /Count is absent
        let no_count = b"%PDF-1.4\n1 0 obj\n<< /Type /Pages /Kids [2 0 R 3 0 R] >>\nendobj\n2 0 obj\n<< /Type /Page /Parent 1 0 R >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 1 0 R >>\nendobj\n%%EOF";
        assert_eq!(extract_pdf_page_count(no_count), Some(2));

        // Invalid bytes return None
        assert_eq!(extract_pdf_page_count(b"not a pdf"), None);
    }

    #[test]
    fn extract_pdf_page_count_reads_objstm_and_real_pdfs_if_present() {
        let objstm_pdf = build_test_multipage_pdf(5, true);
        assert_eq!(extract_pdf_page_count(&objstm_pdf), Some(5));

        // Check real storage/edergi PDFs if present locally or in main workspace
        for candidate_dir in [
            "storage/edergi",
            "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/storage/edergi",
        ] {
            if let Ok(entries) = std::fs::read_dir(candidate_dir) {
                let mut checked = 0;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
                        let bytes = std::fs::read(&path).expect("read edergi pdf");
                        let count = extract_pdf_page_count(&bytes);
                        assert!(
                            count.is_some() && count.unwrap() >= 1,
                            "Expected valid page count for {:?}, got {:?}",
                            path,
                            count
                        );
                        checked += 1;
                    }
                }
                assert!(checked > 0);
                break;
            }
        }

        // Also check real Acrobat PDF 1.6 with FlateDecode + XRef + Predictor 12 + ObjStm
        let spec_pdf = "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/docs/specifications/altyapi-gereksinimleri-v6.pdf";
        if let Ok(bytes) = std::fs::read(spec_pdf) {
            assert_eq!(extract_pdf_page_count(&bytes), Some(6));
        }
    }

    #[test]
    fn parse_page_range_handles_ranges_single_pages_and_invalid_values() {
        assert_eq!(parse_page_range(None), None);
        assert_eq!(parse_page_range(Some("")), None);
        assert_eq!(parse_page_range(Some("all")), None);
        assert_eq!(parse_page_range(Some("ALL")), None);
        assert_eq!(parse_page_range(Some("1-5")), Some((1, 5)));
        assert_eq!(parse_page_range(Some(" 5 - 2 ")), Some((2, 5)));
        assert_eq!(parse_page_range(Some("0-3")), Some((1, 3)));
        assert_eq!(parse_page_range(Some("7")), Some((7, 7)));
        assert_eq!(parse_page_range(Some("abc")), None);
    }

    #[test]
    fn apply_visible_watermark_and_slice_embeds_overlay_updates_info_and_slices_pages() {
        let src_pdf = build_test_multipage_pdf(5, false);
        assert_eq!(extract_pdf_page_count(&src_pdf), Some(5));

        let wm_code = "MSI-DRM-1234567890ABCDEF";
        let wm_text = "Ahmet Yilmaz | MSB | MSB Kurumsal Kamu Lisansi | 2026-11-18 18:00 UTC | MSI-DRM-1234567890ABCDEF";

        // 1. Full document watermark
        let full_wm = apply_visible_watermark_and_slice(&src_pdf, wm_text, wm_code, None);
        assert!(full_wm.starts_with(b"%PDF-"));
        assert!(full_wm.windows(b"%%EOF".len()).rposition(|w| w == b"%%EOF").is_some());
        assert_eq!(extract_pdf_page_count(&full_wm), Some(5));

        let full_lossy = String::from_utf8_lossy(&full_wm);
        assert!(full_lossy.contains("%MSI-DRM:MSI-DRM-1234567890ABCDEF"));
        assert!(full_lossy.contains("/BaseFont /Helvetica"));
        assert!(full_lossy.contains("/ExtGState"));
        assert!(full_lossy.contains("Ahmet Yilmaz"));
        assert!(full_lossy.contains("2026-11-18 18:00 UTC"));
        assert!(full_lossy.contains("/Subject"));
        assert!(full_lossy.contains("/Keywords"));
        assert!(full_lossy.contains("startxref"));

        // Verify startxref actually points to "xref"
        let sx_pos = full_lossy.rfind("startxref\n").unwrap() + "startxref\n".len();
        let sx_end = full_lossy[sx_pos..].find('\n').unwrap() + sx_pos;
        let xref_offset: usize = full_lossy[sx_pos..sx_end].trim().parse().unwrap();
        assert!(full_wm[xref_offset..].starts_with(b"xref\n"));

        // 2. Partial page range slicing (pages 2-4 => 3 pages)
        let sliced = apply_visible_watermark_and_slice(&src_pdf, wm_text, wm_code, Some((2, 4)));
        assert_eq!(extract_pdf_page_count(&sliced), Some(3));
        let sliced_lossy = String::from_utf8_lossy(&sliced);
        assert!(sliced_lossy.contains("Original Page 2 Body"));
        assert!(sliced_lossy.contains("Original Page 3 Body"));
        assert!(sliced_lossy.contains("Original Page 4 Body"));
        assert!(!sliced_lossy.contains("Original Page 1 Body"));
        assert!(!sliced_lossy.contains("Original Page 5 Body"));
        assert!(sliced_lossy.contains("%MSI-DRM:MSI-DRM-1234567890ABCDEF"));

        // 3. Test real storage/edergi PDF if present
        for candidate in [
            "storage/edergi/msi-dergisi-sayi-263.pdf",
            "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/storage/edergi/msi-dergisi-sayi-263.pdf",
        ] {
            if let Ok(real_pdf) = std::fs::read(candidate) {
                let real_wm = apply_visible_watermark_and_slice(&real_pdf, wm_text, wm_code, Some((1, 1)));
                assert!(real_wm.starts_with(b"%PDF-"));
                assert_eq!(extract_pdf_page_count(&real_wm), Some(1));
                let real_str = String::from_utf8_lossy(&real_wm);
                assert!(real_str.contains("Ahmet Yilmaz"));
                assert!(real_str.contains(wm_code));
                assert!(real_str.contains("%MSI-DRM:MSI-DRM-1234567890ABCDEF"));
                assert_standard_pdf_tools_clean(&real_wm, 1, "Ahmet Yilmaz", wm_code);
                break;
            }
        }

        // 4. Test real multi-page Acrobat PDF 1.6 (XRef + ObjStm + FlateDecode + Predictor 12)
        // and real ReportLab PDF 1.4 with page slicing & standard PDF reader validation
        for (path, expected_total, slice_lo, slice_hi) in [
            (
                "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/docs/specifications/altyapi-gereksinimleri-v6.pdf",
                6i32,
                2usize,
                4usize,
            ),
            (
                "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/docs/specifications/proje-plani-ve-teslim-takvimi.pdf",
                4i32,
                2usize,
                3usize,
            ),
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                assert_eq!(extract_pdf_page_count(&bytes), Some(expected_total));
                let sliced_wm = apply_visible_watermark_and_slice(
                    &bytes,
                    wm_text,
                    wm_code,
                    Some((slice_lo, slice_hi)),
                );
                let expected_slice = (slice_hi - slice_lo + 1) as i32;
                assert_eq!(extract_pdf_page_count(&sliced_wm), Some(expected_slice));
                assert_standard_pdf_tools_clean(
                    &sliced_wm,
                    expected_slice,
                    "Ahmet Yilmaz",
                    wm_code,
                );
            }
        }
    }

    fn assert_standard_pdf_tools_clean(
        pdf_bytes: &[u8],
        expected_pages: i32,
        expected_text: &str,
        expected_code: &str,
    ) {
        let tmp_path = format!(
            "/tmp/msi_pdf_verify_{}_{}.pdf",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::fs::write(&tmp_path, pdf_bytes).expect("write temp pdf");

        if let Ok(out) = std::process::Command::new("pdfinfo").arg(&tmp_path).output() {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                assert!(
                    stderr.trim().is_empty(),
                    "pdfinfo emitted stderr warnings: {}",
                    stderr
                );
                assert!(
                    stdout.contains(&format!("Pages:          {}", expected_pages))
                        || stdout.contains(&format!("Pages:\t{}", expected_pages))
                        || stdout
                            .lines()
                            .any(|l| l.starts_with("Pages:") && l.contains(&expected_pages.to_string())),
                    "Unexpected pdfinfo Pages output: {}",
                    stdout
                );
            }
        }

        if let Ok(out) = std::process::Command::new("pdftotext")
            .args([&tmp_path, "-"])
            .output()
        {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                assert!(
                    stderr.trim().is_empty(),
                    "pdftotext emitted stderr warnings: {}",
                    stderr
                );
                assert!(
                    stdout.contains(expected_text),
                    "pdftotext missing watermark text '{}': {}",
                    expected_text,
                    stdout
                );
                assert!(
                    stdout.contains(expected_code),
                    "pdftotext missing watermark code '{}': {}",
                    expected_code,
                    stdout
                );
            }
        }

        if let Ok(out) = std::process::Command::new("gs")
            .args(["-dBATCH", "-dNOPAUSE", "-q", "-sDEVICE=nullpage", &tmp_path])
            .output()
        {
            assert!(
                out.status.success(),
                "gs failed on watermarked PDF: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                stderr.trim().is_empty(),
                "gs emitted stderr warnings: {}",
                stderr
            );
        }

        let _ = std::fs::remove_file(&tmp_path);
    }
}
