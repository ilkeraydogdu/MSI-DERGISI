use base64::Engine as _;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use std::fs;
use super::logs::LEGACY_GMT_DATE_RE;

// ─── Pure-Rust X.509 PEM / ASN.1 DER Certificate Parser ─────────────────

/// Reads a single ASN.1 DER `(tag, value_slice, remaining_slice)` TLV element safely.
fn read_asn1_tlv(data: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let tag = *data.first()?;
    let len_byte = *data.get(1)?;
    let (len, header_len) = if len_byte & 0x80 == 0 {
        (len_byte as usize, 2usize)
    } else {
        let num_octets = (len_byte & 0x7F) as usize;
        if num_octets == 0 || num_octets > 4 || data.len() < 2 + num_octets {
            return None;
        }
        let mut l = 0usize;
        for &b in &data[2..2 + num_octets] {
            l = (l << 8) | (b as usize);
        }
        (l, 2 + num_octets)
    };
    let end = header_len.checked_add(len)?;
    if data.len() < end {
        return None;
    }
    Some((tag, &data[header_len..end], &data[end..]))
}

/// Parses an ASN.1 `UTCTime` (`0x17`) or `GeneralizedTime` (`0x18`) into `DateTime<Utc>`.
fn parse_asn1_time(tag: u8, bytes: &[u8]) -> Option<DateTime<Utc>> {
    let raw = std::str::from_utf8(bytes).ok()?.trim();
    match tag {
        // UTCTime: YYMMDDHHMMSSZ or YYMMDDHHMMZ (RFC 5280 §4.1.2.5.1: YY >= 50 => 19YY, else 20YY)
        0x17 => {
            let core = raw.strip_suffix('Z').unwrap_or(raw);
            if core.len() < 10 {
                return None;
            }
            let yy: i32 = core[0..2].parse().ok()?;
            let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
            let month: u32 = core[2..4].parse().ok()?;
            let day: u32 = core[4..6].parse().ok()?;
            let hour: u32 = core[6..8].parse().ok()?;
            let min: u32 = core[8..10].parse().ok()?;
            let sec: u32 = if core.len() >= 12 {
                core[10..12].parse().ok()?
            } else {
                0
            };
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_opt(hour, min, sec)
                .map(|ndt| ndt.and_utc())
        }
        // GeneralizedTime: YYYYMMDDHHMMSS[.fff]Z (RFC 5280 §4.1.2.5.2)
        0x18 => {
            let core = raw.strip_suffix('Z').unwrap_or(raw);
            let core = core.split('.').next().unwrap_or(core);
            if core.len() < 12 {
                return None;
            }
            let year: i32 = core[0..4].parse().ok()?;
            let month: u32 = core[4..6].parse().ok()?;
            let day: u32 = core[6..8].parse().ok()?;
            let hour: u32 = core[8..10].parse().ok()?;
            let min: u32 = core[10..12].parse().ok()?;
            let sec: u32 = if core.len() >= 14 {
                core[12..14].parse().ok()?
            } else {
                0
            };
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_opt(hour, min, sec)
                .map(|ndt| ndt.and_utc())
        }
        _ => None,
    }
}

/// Extracts DNS names (Subject CN `2.5.4.3` + SubjectAltName `dNSName` `[2]` `2.5.29.17`)
/// and `(notBefore, notAfter)` validity timestamps from a DER-encoded X.509 certificate.
pub fn parse_x509_der_validity_and_names(
    der: &[u8],
) -> Option<(DateTime<Utc>, DateTime<Utc>, Vec<String>)> {
    let (cert_tag, cert_body, _) = read_asn1_tlv(der)?;
    if cert_tag != 0x30 {
        return None;
    }
    let (tbs_tag, tbs_body, _) = read_asn1_tlv(cert_body)?;
    if tbs_tag != 0x30 {
        return None;
    }

    // Walk TBSCertificate fields: [0] version OPTIONAL, serialNumber, signature, issuer, validity, subject, spki, ..., [3] extensions
    let (first_tag, _, after_first) = read_asn1_tlv(tbs_body)?;
    let cursor = if first_tag == 0xA0 {
        let (_, _, after_serial) = read_asn1_tlv(after_first)?;
        after_serial
    } else {
        // v1 certificate without explicit [0] version; first_tag was serialNumber
        after_first
    };

    // signature AlgorithmIdentifier
    let (sig_tag, _, cursor) = read_asn1_tlv(cursor)?;
    if sig_tag != 0x30 {
        return None;
    }
    // issuer Name
    let (issuer_tag, _, cursor) = read_asn1_tlv(cursor)?;
    if issuer_tag != 0x30 {
        return None;
    }
    // validity SEQUENCE { notBefore, notAfter }
    let (val_tag, val_body, cursor) = read_asn1_tlv(cursor)?;
    if val_tag != 0x30 {
        return None;
    }
    let (nb_tag, nb_bytes, val_rem) = read_asn1_tlv(val_body)?;
    let (na_tag, na_bytes, _) = read_asn1_tlv(val_rem)?;
    let not_before = parse_asn1_time(nb_tag, nb_bytes)?;
    let not_after = parse_asn1_time(na_tag, na_bytes)?;

    // subject Name
    let (subj_tag, subj_body, mut tail) = read_asn1_tlv(cursor)?;
    let mut names = Vec::new();
    if subj_tag == 0x30 {
        extract_cn_from_name(subj_body, &mut names);
    }

    // Scan remaining TBSCertificate elements for [3] EXPLICIT Extensions (0xA3)
    while !tail.is_empty() {
        let Some((tag, body, next_tail)) = read_asn1_tlv(tail) else {
            break;
        };
        if tag == 0xA3 {
            extract_san_dns_names(body, &mut names);
        }
        tail = next_tail;
    }

    Some((not_before, not_after, names))
}

/// Extracts CommonName (`OID 2.5.4.3` = `[0x55, 0x04, 0x03]`) strings from an X.509 `Name` SEQUENCE.
fn extract_cn_from_name(mut name_seq: &[u8], out: &mut Vec<String>) {
    const OID_CN: &[u8] = &[0x55, 0x04, 0x03];
    while let Some((rdn_tag, mut rdn_set, rest)) = read_asn1_tlv(name_seq) {
        name_seq = rest;
        if rdn_tag != 0x31 {
            continue;
        }
        while let Some((atv_tag, atv_seq, rdn_rest)) = read_asn1_tlv(rdn_set) {
            rdn_set = rdn_rest;
            if atv_tag != 0x30 {
                continue;
            }
            if let Some((0x06, oid_bytes, val_part)) = read_asn1_tlv(atv_seq) {
                if oid_bytes == OID_CN {
                    if let Some((_, val_bytes, _)) = read_asn1_tlv(val_part) {
                        if let Ok(s) = std::str::from_utf8(val_bytes) {
                            let clean = s.trim().to_ascii_lowercase();
                            if !clean.is_empty() && !out.contains(&clean) {
                                out.push(clean);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extracts `dNSName` (`[2] IA5String`, tag `0x82`) entries from SubjectAltName (`OID 2.5.29.17`).
fn extract_san_dns_names(ext_wrapper: &[u8], out: &mut Vec<String>) {
    const OID_SAN: &[u8] = &[0x55, 0x1D, 0x11];
    let Some((0x30, mut ext_seq, _)) = read_asn1_tlv(ext_wrapper) else {
        return;
    };
    while let Some((0x30, single_ext, rest)) = read_asn1_tlv(ext_seq) {
        ext_seq = rest;
        let Some((0x06, oid_bytes, mut ext_rem)) = read_asn1_tlv(single_ext) else {
            continue;
        };
        if oid_bytes != OID_SAN {
            continue;
        }
        // Optional critical BOOLEAN (0x01) before extnValue OCTET STRING (0x04)
        while let Some((t, v, r)) = read_asn1_tlv(ext_rem) {
            ext_rem = r;
            if t == 0x04 {
                if let Some((0x30, mut gen_names, _)) = read_asn1_tlv(v) {
                    while let Some((gn_tag, gn_val, gn_rest)) = read_asn1_tlv(gen_names) {
                        gen_names = gn_rest;
                        // [2] dNSName context-specific primitive tag = 0x82
                        if gn_tag == 0x82 {
                            if let Ok(s) = std::str::from_utf8(gn_val) {
                                let clean = s.trim().to_ascii_lowercase();
                                if !clean.is_empty() && !out.contains(&clean) {
                                    out.push(clean);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// RFC 6125 hostname matching (exact case-insensitive match or single-label wildcard `*.example.com`).
pub(super) fn cert_pattern_matches_domain(pattern: &str, domain: &str) -> bool {
    let pat = pattern.trim().trim_end_matches('.').to_ascii_lowercase();
    let dom = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if pat == dom {
        return true;
    }
    if let Some(suffix) = pat.strip_prefix("*.") {
        // Single-label wildcard: `sub.example.com` matches `*.example.com`, but `a.b.example.com` and `example.com` do not
        if let Some(prefix) = dom.strip_suffix(suffix) {
            if let Some(sub_label) = prefix.strip_suffix('.') {
                return !sub_label.is_empty() && !sub_label.contains('.');
            }
        }
    }
    false
}

/// Extracts and Base64-decodes the leaf (first) `-----BEGIN CERTIFICATE-----` block from PEM text.
fn decode_first_pem_certificate(pem: &str) -> Option<Vec<u8>> {
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    let start_idx = pem.find(BEGIN)? + BEGIN.len();
    let rest = &pem[start_idx..];
    let end_idx = rest.find(END)?;
    let b64_clean: String = rest[..end_idx]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if b64_clean.is_empty() {
        return None;
    }
    base64::engine::general_purpose::STANDARD
        .decode(b64_clean.as_bytes())
        .ok()
}

/// Checks SSL certificate expiry for `domain` by decoding Plesk PEM/DER certificates in `cert_base_path`
/// (`/usr/local/psa/var/certificates`, supporting both `.pem`/`.crt` files and Plesk's `scf*`/`cert*` files).
/// Returns `Some((rfc3339_expiry_date, days_remaining))` for the newest matching certificate.
pub fn check_ssl_cert_expiry(cert_base_path: &str, domain: &str) -> Option<(String, i64)> {
    let cert_dir = fs::read_dir(cert_base_path).ok()?;
    let mut best_expiry: Option<DateTime<Utc>> = None;

    for entry in cert_dir.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let is_cert_candidate = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "pem" | "crt" | "cer"))
            .unwrap_or(false)
            || name_str.starts_with("scf")
            || name_str.starts_with("cert");

        if !is_cert_candidate {
            continue;
        }

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };

        // 1. Primary Pure-Rust X.509 PEM -> DER -> ASN.1 TBSCertificate parser
        if let Some(der) = decode_first_pem_certificate(&content) {
            if let Some((_not_before, not_after, cert_names)) = parse_x509_der_validity_and_names(&der)
            {
                let matches_domain = cert_names
                    .iter()
                    .any(|pat| cert_pattern_matches_domain(pat, domain));
                if matches_domain
                    && best_expiry.map_or(true, |existing| not_after > existing)
                {
                    best_expiry = Some(not_after);
                }
                continue;
            }
        }

        // 2. Fallback for plain-text certificate dumps containing readable GMT timestamps
        if content.to_ascii_lowercase().contains(&domain.to_ascii_lowercase()) {
            if let Some(caps) = LEGACY_GMT_DATE_RE.captures(&content) {
                if let Ok(naive) = NaiveDateTime::parse_from_str(&caps[1], "%b %e %H:%M:%S %Y")
                    .or_else(|_| NaiveDateTime::parse_from_str(&caps[1], "%b %d %H:%M:%S %Y"))
                {
                    let exp = naive.and_utc();
                    if best_expiry.map_or(true, |existing| exp > existing) {
                        best_expiry = Some(exp);
                    }
                }
            }
        }
    }

    best_expiry.map(|expiry| {
        let days_left = (expiry - Utc::now()).num_days();
        (expiry.to_rfc3339(), days_left)
    })
}
