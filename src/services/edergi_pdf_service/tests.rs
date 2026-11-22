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

#[test]
fn parse_page_selection_and_non_contiguous_slicing_work() {
    assert_eq!(parse_page_selection(None), Ok(None));
    assert_eq!(parse_page_selection(Some("all")), Ok(None));
    assert_eq!(parse_page_selection(Some("1-5, 8")), Ok(Some(vec![1, 2, 3, 4, 5, 8])));
    assert_eq!(parse_page_selection(Some("8, 2-4, 3")), Ok(Some(vec![2, 3, 4, 8])));
    assert!(parse_page_selection(Some("abc")).is_err());
    assert!(parse_page_selection(Some("0")).is_err());
    assert!(parse_page_selection(Some("1-2-3")).is_err());
    assert!(parse_page_selection(Some("1,,2")).is_err());

    let src_pdf = build_test_multipage_pdf(5, false);
    let sliced = apply_visible_watermark_and_select_pages(
        &src_pdf,
        "Test User | ASELSAN",
        "MSI-DRM-TESTCODE1234",
        Some(&[1, 3, 5]),
    );
    assert_eq!(extract_pdf_page_count(&sliced), Some(3));
    let txt = String::from_utf8_lossy(&sliced);
    assert!(txt.contains("Original Page 1 Body"));
    assert!(!txt.contains("Original Page 2 Body"));
    assert!(txt.contains("Original Page 3 Body"));
    assert!(!txt.contains("Original Page 4 Body"));
    assert!(txt.contains("Original Page 5 Body"));

    let spec_pdf = "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/docs/specifications/altyapi-gereksinimleri-v6.pdf";
    if let Ok(bytes) = std::fs::read(spec_pdf) {
        let sel = parse_page_selection(Some("1-2, 5")).unwrap().unwrap();
        let real_sliced = apply_visible_watermark_and_select_pages(
            &bytes,
            "Ahmet Yilmaz | MSB",
            "MSI-DRM-ABCDEF1234567890",
            Some(&sel),
        );
        assert_eq!(extract_pdf_page_count(&real_sliced), Some(3));
        assert_standard_pdf_tools_clean(
            &real_sliced,
            3,
            "Ahmet Yilmaz",
            "MSI-DRM-ABCDEF1234567890",
        );
    }
}

#[test]
fn converter_sanitization_neutralizes_injection_and_traversal() {
    assert_eq!(converter::safe_pdf_filename("issue-286.pdf"), Some("issue-286.pdf".to_string()));
    assert_eq!(converter::safe_pdf_filename("../secret.pdf"), None);
    assert_eq!(converter::safe_pdf_filename("sub/file.pdf"), None);
    assert_eq!(converter::safe_pdf_filename("shell.sh"), None);

    let sanitized = converter::sanitize_imagemagick_text("@/etc/passwd %x %[fx:w] \"q\" \\test");
    assert!(!sanitized.contains('%'));
    assert!(!sanitized.contains('@'));
    assert!(!sanitized.contains('\\'));
    assert!(!sanitized.contains('"'));
    assert!(sanitized.contains("(at)/etc/passwd"));

    let escaped = converter::escape_xml_text("<script>\"a\" & 'b'</script>");
    assert!(!escaped.contains('<'));
    assert!(!escaped.contains('>'));
    assert!(escaped.contains("&lt;script&gt;"));
    assert!(escaped.contains("&amp;"));
}
