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

/// Generates a 100% standards-compliant, multi-page PDF 1.4 binary document in pure Rust
/// with exact `xref` byte offsets and unremovable dynamic DRM watermarks on every page.
///
/// Embedded watermark format on every page (header, diagonal, and footer):
/// `MSB/SSB/KURUMSAL LISANS: {user_name} ({org_name}) - {license_label} - TARIH: {timestamp} - IP: {client_ip} - DRM KODU: {hash}`
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
        "KAMU / ABONE E-DERGI LISANSI"
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
        "MSB/SSB/KURUMSAL LISANS: {} ({}) - {} - TARIH: {} - IP: {} - DRM KODU: {}",
        clean_user, clean_org, clean_license, timestamp, clean_ip, drm_hash
    );

    let diagonal_watermark_primary = format!(
        "MSB/SSB/KURUMSAL LISANS: {} ({}) - {}",
        clean_user, clean_org, clean_license
    );
    let diagonal_watermark_secondary = format!(
        "TARIH: {} - IP: {} - DRM KODU: {}",
        timestamp, clean_ip, drm_hash
    );

    let filtered_spreads = filter_spreads_by_selection(&issue.spreads, selected_pages);
    let selection_desc = selected_pages
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"))
        .map(|s| format!("Secili Sayfalar ({})", sanitize_pdf_text(s)))
        .unwrap_or_else(|| format!("Tam Sayi Arsiv Paketi ({} Sayfa)", issue.page_count));

    // Build page content streams:
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

    // Parse numbers or ranges like "1-16, 24, 32-40"
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

    // 1. Background & Diagonal Watermark Layer
    append_page_chrome_and_watermarks(
        &mut s,
        issue.issue_number,
        full_watermark_line,
        diag_primary,
        diag_secondary,
        page_idx,
        total_pages,
    );

    // 2. Issue Title & Cover Dossier Header
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

    // 3. Executive Summary wrapped lines
    s.push_str("BT\n0.12 0.16 0.24 rg\n/F1 10 Tf\n42 662 Td\n(KAPAK DOSYASI VE SAYI OZETI:) Tj\nET\n");
    let summary_lines = wrap_text(&sanitize_pdf_text(&issue.summary_tr), 78);
    let mut y_cursor = 644;
    for line in summary_lines.iter().take(6) {
        s.push_str(&format!(
            "BT\n0.18 0.22 0.30 rg\n/F2 9.5 Tf\n42 {} Td\n({}) Tj\nET\n",
            y_cursor,
            escape_pdf_string(line)
        ));
        y_cursor -= 15;
    }

    // 4. Table of Contents Spreads Overview Box
    y_cursor -= 12;
    s.push_str(&format!(
        "BT\n0.08 0.14 0.26 rg\n/F1 10.5 Tf\n42 {} Td\n(BU SAYIDA YER ALAN DOSYALAR ({}):) Tj\nET\n",
        y_cursor,
        escape_pdf_string(selection_desc)
    ));
    y_cursor -= 18;

    for spread in issue.spreads.iter().take(8) {
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

    // 5. Mandatory Cryptographic DRM Certificate Box
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

    // Spread Metadata Badge
    let badge_line = format!(
        "SAYFA: {}  |  KATEGORI: {}  |  SAYI #{}",
        sanitize_pdf_text(&spread.page_no),
        sanitize_pdf_text(&spread.category_badge).to_uppercase(),
        issue.issue_number
    );
    s.push_str("BT\n0.68 0.50 0.10 rg\n/F1 10 Tf\n42 730 Td\n");
    s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&badge_line)));

    // Turkish Title (Wrapped)
    let title_lines = wrap_text(&sanitize_pdf_text(&spread.title_tr), 62);
    let mut y = 706;
    for tl in title_lines.iter().take(3) {
        s.push_str(&format!(
            "BT\n0.06 0.10 0.18 rg\n/F1 14 Tf\n42 {} Td\n({}) Tj\nET\n",
            y,
            escape_pdf_string(tl)
        ));
        y -= 19;
    }

    // English Title
    y -= 4;
    let en_title = format!("EN: {}", sanitize_pdf_text(&spread.title_en));
    for tl in wrap_text(&en_title, 74).iter().take(2) {
        s.push_str(&format!(
            "BT\n0.35 0.42 0.52 rg\n/F2 10 Tf\n42 {} Td\n({}) Tj\nET\n",
            y,
            escape_pdf_string(tl)
        ));
        y -= 15;
    }

    // Divider line
    y -= 8;
    s.push_str(&format!(
        "0.82 0.85 0.90 RG\n0.8 w\n42 {} m 553 {} l S\n",
        y, y
    ));
    y -= 22;

    // Turkish Excerpt / Article Body
    s.push_str(&format!(
        "BT\n0.08 0.14 0.26 rg\n/F1 10.5 Tf\n42 {} Td\n(DOSYA ICERIGI VE TEKNIK ANALIZ OZETI (TR):) Tj\nET\n",
        y
    ));
    y -= 18;

    for line in wrap_text(&sanitize_pdf_text(&spread.excerpt_tr), 78)
        .iter()
        .take(12)
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
        .take(10)
    {
        s.push_str(&format!(
            "BT\n0.24 0.29 0.38 rg\n/F2 9.5 Tf\n42 {} Td\n({}) Tj\nET\n",
            y,
            escape_pdf_string(line)
        ));
        y -= 15;
    }

    // Mid-page embedded watermark stamp box so cropping top/bottom cannot remove DRM attribution
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
    // 1. Diagonal Watermark Vectors across the page body (35-degree rotation matrix)
    s.push_str("q\n0.87 0.89 0.93 rg\nBT\n/F1 13 Tf\n0.819 0.573 -0.573 0.819 58 265 Tm\n");
    s.push_str(&format!("({}) Tj\nET\nQ\n", escape_pdf_string(diag_primary)));

    s.push_str("q\n0.88 0.90 0.94 rg\nBT\n/F3 10.5 Tf\n0.819 0.573 -0.573 0.819 78 232 Tm\n");
    s.push_str(&format!(
        "({}) Tj\nET\nQ\n",
        escape_pdf_string(diag_secondary)
    ));

    s.push_str("q\n0.89 0.91 0.94 rg\nBT\n/F1 12 Tf\n0.819 0.573 -0.573 0.819 64 470 Tm\n");
    s.push_str(&format!("({}) Tj\nET\nQ\n", escape_pdf_string(diag_primary)));

    // 2. Top Tactical Header Bar
    s.push_str("0.06 0.09 0.16 rg\n0 778 595 64 re f\n");
    s.push_str("0.83 0.68 0.21 rg\n0 776 595 2.5 re f\n");

    let header_title = format!(
        "MSI DERGISI & MSI TDR | SAVUNMA SANAYII VE HAVACILIK PLATFORMU - SAYI #{}  (SAYFA {}/{})",
        issue_number, page_idx, total_pages
    );
    s.push_str("BT\n1 1 1 rg\n/F1 9.5 Tf\n38 818 Td\n");
    s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&header_title)));

    // Header embedded watermark line
    s.push_str("BT\n0.92 0.80 0.42 rg\n/F3 6.6 Tf\n38 792 Td\n");
    s.push_str(&format!(
        "({}) Tj\nET\n",
        escape_pdf_string(&truncate_chars(full_watermark_line, 112))
    ));

    // 3. Bottom Tactical Security Footer Bar
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
) -> Vec<u8> {
    // Object ID layout:
    // 1: Catalog
    // 2: Pages
    // 3: Font F1 (Helvetica-Bold)
    // 4: Font F2 (Helvetica)
    // 5: Font F3 (Courier-Bold)
    // For page i in 0..N:
    //   page_obj_id = 6 + i * 2
    //   content_obj_id = 7 + i * 2
    // info_obj_id = 6 + N * 2
    let page_count = page_streams.len();
    let info_obj_id = 6 + page_count * 2;
    let total_objects = info_obj_id; // 1..=info_obj_id

    let mut buf: Vec<u8> = Vec::with_capacity(16384);
    let mut offsets: Vec<usize> = vec![0; total_objects + 1];

    // PDF 1.4 Header + binary high-bit comment
    buf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    // Object 1: Catalog
    offsets[1] = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Object 2: Pages
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

    // Object 3: Helvetica-Bold
    offsets[3] = buf.len();
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    // Object 4: Helvetica
    offsets[4] = buf.len();
    buf.extend_from_slice(
        b"4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    // Object 5: Courier-Bold
    offsets[5] = buf.len();
    buf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier-Bold /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    // Page & Stream objects
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

    // Document Info Dictionary (also embeds watermark metadata)
    offsets[info_obj_id] = buf.len();
    let info_obj = format!(
        "{} 0 obj\n<< /Title (MSI Dergisi Sayi #{} - {}) /Author (UMSA Produksiyon - MSI Dergisi) /Subject ({}) /Creator (MSI Dergisi & MSI TDR Rust DRM Engine v2.6) /Producer (Pure Rust PDF 1.4 Generator) >>\nendobj\n",
        info_obj_id,
        issue_number,
        escape_pdf_string(issue_title_ascii),
        escape_pdf_string(full_watermark_line)
    );
    buf.extend_from_slice(info_obj.as_bytes());

    // Cross-Reference Table (xref) with exact byte offsets
    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n", total_objects + 1).as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    // Trailer
    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R /Info {} 0 R >>\nstartxref\n{}\n%%EOF\n",
        total_objects + 1,
        info_obj_id,
        xref_offset
    );
    buf.extend_from_slice(trailer.as_bytes());

    buf
}

/// Transliterates Turkish and special UTF-8 characters into clean PDF-safe ASCII so Type1 fonts
/// in Adobe Acrobat, Chrome PDF Viewer, and Edge render 100% crisply without encoding corruption.
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
