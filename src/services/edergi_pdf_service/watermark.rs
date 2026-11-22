use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::models::{ContentItem, MagazineIssue, MagazinePageSpread};

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

/// Parses a `?pages=` query parameter supporting `"all"`, single pages (`"8"`),
/// ranges (`"1-5"`), and comma-separated combinations (`"1-5, 8, 10-12"`).
/// Returns `Ok(None)` for empty or `"all"`, `Ok(Some(sorted_unique_1based_pages))`,
/// or `Err(...)` for malformed tokens.
pub fn parse_page_selection(raw: Option<&str>) -> Result<Option<Vec<usize>>, String> {
    let Some(raw_str) = raw else {
        return Ok(None);
    };
    let text = raw_str.trim();
    if text.is_empty() || text.eq_ignore_ascii_case("all") {
        return Ok(None);
    }

    let mut pages = std::collections::BTreeSet::new();
    for part in text.split(',') {
        let token = part.trim();
        if token.is_empty() {
            return Err("Sayfa seçimi boş öğe içeremez.".to_string());
        }
        if let Some((start_s, end_s)) = token.split_once('-') {
            if end_s.contains('-') {
                return Err(format!("Geçersiz sayfa aralığı: {}", token));
            }
            let s = start_s
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Geçersiz başlangıç sayfası: {}", token))?;
            let e = end_s
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Geçersiz bitiş sayfası: {}", token))?;
            if s == 0 || e == 0 {
                return Err("Sayfa numaraları 1 veya daha büyük olmalıdır.".to_string());
            }
            let lo = s.min(e);
            let hi = s.max(e);
            if hi - lo > 1000 {
                return Err("Sayfa aralığı çok geniş.".to_string());
            }
            for p in lo..=hi {
                pages.insert(p);
                if pages.len() > 2000 {
                    return Err("Seçilen toplam sayfa sayısı sınırı aşıldı.".to_string());
                }
            }
        } else {
            let p = token
                .parse::<usize>()
                .map_err(|_| format!("Geçersiz sayfa numarası: {}", token))?;
            if p == 0 {
                return Err("Sayfa numaraları 1 veya daha büyük olmalıdır.".to_string());
            }
            pages.insert(p);
            if pages.len() > 2000 {
                return Err("Seçilen toplam sayfa sayısı sınırı aşıldı.".to_string());
            }
        }
    }

    if pages.is_empty() {
        Ok(None)
    } else {
        Ok(Some(pages.into_iter().collect()))
    }
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
    generate_watermarked_pdf_checked(
        issue,
        user_name,
        org_name,
        license_label,
        client_ip,
        selected_pages,
        secret,
    )
    .unwrap_or_else(|_| {
        generate_watermarked_pdf_checked(
            issue,
            user_name,
            org_name,
            license_label,
            client_ip,
            None,
            secret,
        )
        .unwrap_or_default()
    })
}

/// Validates page selection bounds for synthetic PDFs and generates a multi-page PDF 1.4 binary document.
pub fn generate_watermarked_pdf_checked(
    issue: &MagazineIssue,
    user_name: &str,
    org_name: &str,
    license_label: &str,
    client_ip: &str,
    selected_pages: Option<&str>,
    secret: &str,
) -> Result<Vec<u8>, String> {
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

    let filtered_spreads =
        filter_spreads_by_selection(&issue.spreads, issue.page_count, selected_pages)?;
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

    Ok(assemble_pdf_binary(
        &page_streams,
        issue.issue_number,
        &sanitize_pdf_text(&issue.title_tr),
        &full_watermark_line,
        &drm_hash,
    ))
}

fn filter_spreads_by_selection(
    spreads: &[MagazinePageSpread],
    issue_page_count: i32,
    selected_pages: Option<&str>,
) -> Result<Vec<MagazinePageSpread>, String> {
    let requested_numbers: Vec<i32> = match parse_page_selection(selected_pages)? {
        Some(nums) => nums
            .into_iter()
            .filter_map(|n| i32::try_from(n).ok())
            .collect(),
        None => return Ok(spreads.to_vec()),
    };

    if requested_numbers.is_empty() {
        return Ok(spreads.to_vec());
    }

    let max_spread_page = spreads
        .iter()
        .flat_map(|sp| {
            sp.page_no
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|p| p.parse::<i32>().ok())
        })
        .max()
        .unwrap_or(0);
    let effective_total = issue_page_count
        .max(max_spread_page)
        .max(1 + spreads.len() as i32)
        .max(1);

    if requested_numbers.iter().any(|&p| p < 1 || p > effective_total) {
        return Err(format!(
            "Seçilen sayfalar dergi sayfa sayısını ({}) aşıyor.",
            effective_total
        ));
    }

    if spreads.is_empty() {
        return Ok(Vec::new());
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
                false
            }
        })
        .cloned()
        .collect();

    if matched.is_empty() {
        Err(format!(
            "Seçilen sayfa aralığı ({}) için eşleşen içerik bulunamadı.",
            selected_pages.unwrap_or("")
        ))
    } else {
        Ok(matched)
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

pub(super) fn escape_pdf_string(input: &str) -> String {
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

pub(super) fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
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

fn strip_html_for_pdf(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

/// Generates a genuine multi-page (4-6 pages) PDF 1.4 starter archive digest in pure Rust
/// from a real `magazine_issues` row, its `spreads`, and real published DB `contents`.
/// Page 1 explicitly notes that it is the digital archive starter digest pending customer
/// print PDF upload via `/podmin` Tab 18.
pub fn generate_starter_archive_pdf(
    issue: &MagazineIssue,
    linked_articles: &[ContentItem],
) -> Vec<u8> {
    let is_tdr = issue.publication_type.eq_ignore_ascii_case("msi_tdr");
    let pub_label = if is_tdr {
        "MSI TURKISH DEFENCE REVIEW"
    } else {
        "MSI DERGISI"
    };
    let archive_tag = if is_tdr {
        format!("MSI-TDR-ARCHIVE-{}", issue.issue_number)
    } else {
        format!("MSI-ARCHIVE-{}", issue.issue_number)
    };
    let footer_line = format!(
        "{} | Sayi/Issue #{} ({} {}) | Basili Edisyon: {} Sayfa | Dijital Arsiv Ozeti",
        pub_label,
        issue.issue_number,
        sanitize_pdf_text(&issue.publish_month_tr),
        issue.year,
        issue.page_count
    );

    let spreads: Vec<&MagazinePageSpread> = issue.spreads.iter().take(4).collect();
    let mut extra_pages: Vec<String> = Vec::new();

    // Build spread pages (1 page per spread, max 4)
    let mut planned_total = 1 + spreads.len();
    // Prepare article digest pages from real DB contents so total is 4..=6
    let mut article_chunks: Vec<Vec<&ContentItem>> = Vec::new();
    for chunk in linked_articles.chunks(2) {
        if planned_total + article_chunks.len() >= 6 {
            break;
        }
        article_chunks.push(chunk.iter().collect());
    }

    // Ensure at least 4 pages using real issue summary & dossier breakdown if needed
    let mut summary_sections: Vec<(String, String)> = Vec::new();
    let target_pages = (1 + spreads.len() + article_chunks.len()).clamp(4, 6);
    if 1 + spreads.len() + article_chunks.len() < 4 {
        let missing = 4 - (1 + spreads.len() + article_chunks.len());
        let raw_summary = if is_tdr && !issue.summary_en.trim().is_empty() {
            issue.summary_en.as_str()
        } else if !issue.summary_tr.trim().is_empty() {
            issue.summary_tr.as_str()
        } else {
            issue.summary_en.as_str()
        };
        let normalized = sanitize_pdf_text(raw_summary);
        let parts: Vec<&str> = normalized
            .split('*')
            .map(str::trim)
            .filter(|p| p.len() > 12)
            .collect();
        for idx in 0..missing {
            let heading = format!(
                "{} #{} - ARSIV DOSSIER & EDITION BRIEFING ({})",
                pub_label,
                issue.issue_number,
                idx + 1
            );
            let body = if !parts.is_empty() {
                let slice_start = (idx * 2) % parts.len();
                let segment = parts
                    .iter()
                    .cycle()
                    .skip(slice_start)
                    .take(2.min(parts.len()))
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" | ");
                format!(
                    "{} | Arsiv Kaydi: {} {} ({} sayfa baski kapsami, {} dosya).",
                    segment,
                    sanitize_pdf_text(&issue.publish_month_en),
                    issue.year,
                    issue.page_count,
                    issue.dossier_count
                )
            } else {
                format!(
                    "{} ({} {}) | {} | {}",
                    sanitize_pdf_text(&issue.title_tr),
                    sanitize_pdf_text(&issue.publish_month_en),
                    issue.year,
                    sanitize_pdf_text(&issue.summary_tr),
                    sanitize_pdf_text(&issue.summary_en)
                )
            };
            summary_sections.push((heading, body));
        }
    }

    planned_total = target_pages;

    let mut page_streams = Vec::with_capacity(planned_total);

    // Page 1: Cover, Starter Digest Notice & Table of Contents
    {
        let mut s = String::with_capacity(4096);
        append_starter_chrome(&mut s, pub_label, issue.issue_number, &footer_line, 1, planned_total);

        let header = format!(
            "{} - SAYI #{} ({} {})",
            pub_label,
            issue.issue_number,
            sanitize_pdf_text(&issue.publish_month_tr).to_uppercase(),
            issue.year
        );
        s.push_str("BT\n0.08 0.14 0.26 rg\n/F1 15 Tf\n42 736 Td\n");
        s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&header)));

        s.push_str("BT\n0.05 0.08 0.15 rg\n/F1 12 Tf\n42 712 Td\n");
        s.push_str(&format!(
            "({}) Tj\nET\n",
            escape_pdf_string(&truncate_chars(&sanitize_pdf_text(&issue.title_tr), 76))
        ));

        s.push_str("BT\n0.30 0.36 0.46 rg\n/F2 9.5 Tf\n42 694 Td\n");
        s.push_str(&format!(
            "({}) Tj\nET\n",
            escape_pdf_string(&truncate_chars(
                &format!("EN: {}", sanitize_pdf_text(&issue.title_en)),
                84
            ))
        ));

        // Starter digest notice box (explicitly citing /podmin Tab 18)
        s.push_str("0.99 0.96 0.89 rg\n42 608 511 72 re f\n");
        s.push_str("0.80 0.58 0.14 RG\n1.0 w\n42 608 511 72 re S\n");
        s.push_str("BT\n0.55 0.36 0.04 rg\n/F1 9 Tf\n50 664 Td\n(DIJITAL ARSIV BASLANGIC OZETI - DIGITAL ARCHIVE STARTER DIGEST) Tj\nET\n");
        let scope_line = format!(
            "Basili Edisyon Kapsami: {} Sayfa | {} Dosya | Yayin Turu: {}",
            issue.page_count, issue.dossier_count, issue.publication_type
        );
        let notice_lines = [
            "Bu PDF, veritabanindaki gercek sayi meta verileri ve yayinlanmis iceriklerden uretilen baslangic ozetidir.",
            "Tam baski PDF dosyasi /podmin -> Tab 18 (E-Dergi Yonetimi) uzerinden yuklendiginde otomatik degistirilir.",
            scope_line.as_str(),
        ];
        let mut ny = 649;
        for nl in &notice_lines {
            s.push_str(&format!(
                "BT\n0.22 0.24 0.30 rg\n/F2 8 Tf\n50 {} Td\n({}) Tj\nET\n",
                ny,
                escape_pdf_string(nl)
            ));
            ny -= 14;
        }

        // Summary TR / EN
        s.push_str("BT\n0.08 0.14 0.26 rg\n/F1 10 Tf\n42 586 Td\n(SAYI OZETI / ISSUE SUMMARY:) Tj\nET\n");
        let summary_src = if !issue.summary_tr.trim().is_empty() {
            format!(
                "{} | {}",
                sanitize_pdf_text(&issue.summary_tr),
                sanitize_pdf_text(&issue.summary_en)
            )
        } else {
            sanitize_pdf_text(&issue.summary_en)
        };
        let mut y = 568;
        for line in wrap_text(&summary_src, 84).iter().take(6) {
            s.push_str(&format!(
                "BT\n0.16 0.20 0.28 rg\n/F2 9 Tf\n42 {} Td\n({}) Tj\nET\n",
                y,
                escape_pdf_string(line)
            ));
            y -= 14;
        }

        y -= 12;
        s.push_str(&format!(
            "BT\n0.08 0.14 0.26 rg\n/F1 10 Tf\n42 {} Td\n(ICINDEKILER / TABLE OF CONTENTS ({} SAYFA OZET):) Tj\nET\n",
            y, planned_total
        ));
        y -= 18;

        for sp in &issue.spreads {
            if y < 120 {
                break;
            }
            let toc_line = format!(
                "[{}] {} - {}",
                sanitize_pdf_text(&sp.page_no),
                sanitize_pdf_text(&sp.category_badge),
                sanitize_pdf_text(if is_tdr { &sp.title_en } else { &sp.title_tr })
            );
            s.push_str(&format!(
                "BT\n0.14 0.18 0.26 rg\n/F2 8.8 Tf\n48 {} Td\n({}) Tj\nET\n",
                y,
                escape_pdf_string(&truncate_chars(&toc_line, 86))
            ));
            y -= 15;
        }

        for art in linked_articles.iter().take(8) {
            if y < 95 {
                break;
            }
            let art_title = if is_tdr && !art.title_en.trim().is_empty() {
                &art.title_en
            } else {
                &art.title_tr
            };
            let line = format!(
                "* [{}] {}",
                sanitize_pdf_text(&art.content_type).to_uppercase(),
                sanitize_pdf_text(art_title)
            );
            s.push_str(&format!(
                "BT\n0.20 0.26 0.36 rg\n/F2 8.5 Tf\n48 {} Td\n({}) Tj\nET\n",
                y,
                escape_pdf_string(&truncate_chars(&line, 86))
            ));
            y -= 14;
        }

        page_streams.push(s);
    }

    // Spread detail pages
    for sp in &spreads {
        let page_idx = page_streams.len() + 1;
        let mut s = String::with_capacity(4096);
        append_starter_chrome(
            &mut s,
            pub_label,
            issue.issue_number,
            &footer_line,
            page_idx,
            planned_total,
        );
        let badge = format!(
            "SAYFA: {} | KATEGORI: {} | {} #{}",
            sanitize_pdf_text(&sp.page_no),
            sanitize_pdf_text(&sp.category_badge).to_uppercase(),
            pub_label,
            issue.issue_number
        );
        s.push_str("BT\n0.68 0.50 0.10 rg\n/F1 10 Tf\n42 732 Td\n");
        s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&badge)));

        let mut y = 706;
        for tl in wrap_text(&sanitize_pdf_text(&sp.title_tr), 64).iter().take(3) {
            s.push_str(&format!(
                "BT\n0.06 0.10 0.18 rg\n/F1 13.5 Tf\n42 {} Td\n({}) Tj\nET\n",
                y,
                escape_pdf_string(tl)
            ));
            y -= 18;
        }
        y -= 4;
        for tl in wrap_text(&format!("EN: {}", sanitize_pdf_text(&sp.title_en)), 76)
            .iter()
            .take(2)
        {
            s.push_str(&format!(
                "BT\n0.32 0.38 0.48 rg\n/F2 10 Tf\n42 {} Td\n({}) Tj\nET\n",
                y,
                escape_pdf_string(tl)
            ));
            y -= 15;
        }
        y -= 8;
        s.push_str(&format!("0.82 0.85 0.90 RG\n0.8 w\n42 {} m 553 {} l S\n", y, y));
        y -= 22;

        s.push_str(&format!(
            "BT\n0.08 0.14 0.26 rg\n/F1 10 Tf\n42 {} Td\n(DOSYA OZETI / FEATURE SUMMARY (TR):) Tj\nET\n",
            y
        ));
        y -= 18;
        for line in wrap_text(&sanitize_pdf_text(&sp.excerpt_tr), 82).iter().take(10) {
            s.push_str(&format!(
                "BT\n0.14 0.18 0.25 rg\n/F2 9.5 Tf\n42 {} Td\n({}) Tj\nET\n",
                y,
                escape_pdf_string(line)
            ));
            y -= 15;
        }
        y -= 12;
        s.push_str(&format!(
            "BT\n0.08 0.14 0.26 rg\n/F1 10 Tf\n42 {} Td\n(EXECUTIVE BRIEFING (EN):) Tj\nET\n",
            y
        ));
        y -= 18;
        for line in wrap_text(&sanitize_pdf_text(&sp.excerpt_en), 82).iter().take(10) {
            s.push_str(&format!(
                "BT\n0.24 0.29 0.38 rg\n/F2 9.5 Tf\n42 {} Td\n({}) Tj\nET\n",
                y,
                escape_pdf_string(line)
            ));
            y -= 15;
        }
        if let Some(ref slug) = sp.linked_content_slug {
            if !slug.trim().is_empty() && y > 102 {
                s.push_str(&format!(
                    "BT\n0.30 0.36 0.46 rg\n/F3 8 Tf\n42 92 Td\n(BAGLANTILI ICERIK / LINKED SLUG: {}) Tj\nET\n",
                    escape_pdf_string(&sanitize_pdf_text(slug))
                ));
            }
        }
        page_streams.push(s);
    }

    // Linked DB article digest pages
    for chunk in article_chunks {
        let page_idx = page_streams.len() + 1;
        let mut s = String::with_capacity(4096);
        append_starter_chrome(
            &mut s,
            pub_label,
            issue.issue_number,
            &footer_line,
            page_idx,
            planned_total,
        );
        s.push_str(&format!(
            "BT\n0.68 0.50 0.10 rg\n/F1 10 Tf\n42 734 Td\n({} #{} - YAYINLANMIS ICERIK OZETLERI / ARTICLE DIGESTS) Tj\nET\n",
            escape_pdf_string(pub_label),
            issue.issue_number
        ));

        let mut y = 704;
        for art in chunk {
            if y < 170 {
                break;
            }
            let title = if is_tdr && !art.title_en.trim().is_empty() {
                &art.title_en
            } else {
                &art.title_tr
            };
            let summary = if is_tdr && !art.summary_en.trim().is_empty() {
                &art.summary_en
            } else {
                &art.summary_tr
            };
            let body = if is_tdr && !art.body_en.trim().is_empty() {
                &art.body_en
            } else {
                &art.body_tr
            };

            for tl in wrap_text(&sanitize_pdf_text(title), 68).iter().take(2) {
                s.push_str(&format!(
                    "BT\n0.06 0.10 0.18 rg\n/F1 12 Tf\n42 {} Td\n({}) Tj\nET\n",
                    y,
                    escape_pdf_string(tl)
                ));
                y -= 16;
            }
            let meta_line = format!(
                "Yazar/Author: {} | Tarih: {} | Tip: {} | Slug: {}",
                sanitize_pdf_text(&art.author_name),
                sanitize_pdf_text(&art.published_at),
                sanitize_pdf_text(&art.content_type),
                sanitize_pdf_text(&art.slug)
            );
            s.push_str(&format!(
                "BT\n0.35 0.42 0.52 rg\n/F3 7.8 Tf\n42 {} Td\n({}) Tj\nET\n",
                y,
                escape_pdf_string(&truncate_chars(&meta_line, 92))
            ));
            y -= 16;

            let combined = format!(
                "{} {}",
                sanitize_pdf_text(&strip_html_for_pdf(summary)),
                sanitize_pdf_text(&strip_html_for_pdf(body))
            );
            for bl in wrap_text(&combined, 84).iter().take(8) {
                if y < 95 {
                    break;
                }
                s.push_str(&format!(
                    "BT\n0.16 0.20 0.28 rg\n/F2 9 Tf\n42 {} Td\n({}) Tj\nET\n",
                    y,
                    escape_pdf_string(bl)
                ));
                y -= 14;
            }
            y -= 10;
            if y > 130 {
                s.push_str(&format!("0.86 0.88 0.92 RG\n0.6 w\n42 {} m 553 {} l S\n", y, y));
                y -= 18;
            }
        }
        page_streams.push(s);
    }

    // Issue summary/editorial briefing pages if needed to reach 4 pages
    for (heading, body) in summary_sections {
        let page_idx = page_streams.len() + 1;
        let mut s = String::with_capacity(2048);
        append_starter_chrome(
            &mut s,
            pub_label,
            issue.issue_number,
            &footer_line,
            page_idx,
            planned_total,
        );
        s.push_str(&format!(
            "BT\n0.68 0.50 0.10 rg\n/F1 10.5 Tf\n42 732 Td\n({}) Tj\nET\n",
            escape_pdf_string(&truncate_chars(&heading, 82))
        ));
        let mut y = 702;
        for line in wrap_text(&body, 82).iter().take(24) {
            if y < 100 {
                break;
            }
            s.push_str(&format!(
                "BT\n0.14 0.18 0.26 rg\n/F2 9.5 Tf\n42 {} Td\n({}) Tj\nET\n",
                y,
                escape_pdf_string(line)
            ));
            y -= 16;
        }
        page_streams.push(s);
    }

    let _ = &mut extra_pages;
    assemble_pdf_binary(
        &page_streams,
        issue.issue_number,
        &sanitize_pdf_text(&issue.title_tr),
        &footer_line,
        &archive_tag,
    )
}

fn append_starter_chrome(
    s: &mut String,
    pub_label: &str,
    issue_number: i32,
    footer_line: &str,
    page_idx: usize,
    total_pages: usize,
) {
    s.push_str("0.06 0.09 0.16 rg\n0 778 595 64 re f\n");
    s.push_str("0.83 0.68 0.21 rg\n0 776 595 2.5 re f\n");
    let top_title = format!(
        "{} | SAVUNMA SANAYII VE HAVACILIK ARSIVI - SAYI #{} (SAYFA {}/{})",
        pub_label, issue_number, page_idx, total_pages
    );
    s.push_str("BT\n1 1 1 rg\n/F1 9.5 Tf\n38 814 Td\n");
    s.push_str(&format!("({}) Tj\nET\n", escape_pdf_string(&top_title)));
    s.push_str("BT\n0.92 0.80 0.42 rg\n/F3 7.2 Tf\n38 792 Td\n");
    s.push_str("(msidergisi.com & savunmahaber.com | UMSA Produksiyon Dijital Arsiv Baslangic Ozeti) Tj\nET\n");

    s.push_str("0.06 0.09 0.16 rg\n0 42 595 36 re f\n");
    s.push_str("0.83 0.68 0.21 rg\n0 78 595 1.8 re f\n");
    s.push_str("BT\n0.95 0.82 0.45 rg\n/F3 7.0 Tf\n32 62 Td\n");
    s.push_str(&format!(
        "({}) Tj\nET\n",
        escape_pdf_string(&truncate_chars(footer_line, 110))
    ));
    s.push_str("BT\n0.75 0.80 0.88 rg\n/F2 7 Tf\n32 49 Td\n");
    s.push_str("(Tam baski PDF yuklemesi icin /podmin -> Tab 18 (E-Dergi Yonetimi) kullaniniz.) Tj\nET\n");
}

