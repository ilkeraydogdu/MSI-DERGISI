use crate::models::{AdCampaign, AdConflictWarning, ContentItem, UserAccount};

/// Returns a UTF-8 char-boundary-safe prefix of at most `max_chars` characters.
pub fn char_prefix(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

fn normalize_date_prefix(raw: Option<&str>) -> Option<&str> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| char_prefix(s, 10))
}

/// Parses an optional date/timestamp string's trimmed 10-character `YYYY-MM-DD` prefix.
/// Returns `None` for empty/`None` (unbounded), `Some(Ok(NaiveDate))` for valid `YYYY-MM-DD`,
/// and `Some(Err(()))` for malformed non-empty strings.
pub fn parse_date_prefix(raw: Option<&str>) -> Option<Result<chrono::NaiveDate, ()>> {
    let prefix = normalize_date_prefix(raw)?;
    if prefix.len() != 10 || !prefix.is_ascii() {
        return Some(Err(()));
    }
    let b = prefix.as_bytes();
    if !(b[0..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit))
    {
        return Some(Err(()));
    }
    Some(chrono::NaiveDate::parse_from_str(prefix, "%Y-%m-%d").map_err(|_| ()))
}

/// Returns `true` when `ad` is active for `today_yyyy_mm_dd` based on optional `start_date`/`end_date`.
/// Malformed non-empty `start_date` or `end_date` values fail closed (`false`).
pub fn is_ad_within_schedule(ad: &AdCampaign, today_yyyy_mm_dd: &str) -> bool {
    let today = match parse_date_prefix(Some(today_yyyy_mm_dd)) {
        Some(Ok(d)) => d,
        _ => return false,
    };
    if let Some(start_res) = parse_date_prefix(ad.start_date.as_deref()) {
        match start_res {
            Ok(start) => {
                if today < start {
                    return false;
                }
            }
            Err(()) => return false,
        }
    }
    if let Some(end_res) = parse_date_prefix(ad.end_date.as_deref()) {
        match end_res {
            Ok(end) => {
                if today > end {
                    return false;
                }
            }
            Err(()) => return false,
        }
    }
    true
}

/// Returns `true` when two optional `[start, end]` schedule windows overlap (open bounds = unbounded).
/// Returns `false` if any non-empty bound is malformed.
pub fn schedules_overlap(
    a_start: Option<&str>,
    a_end: Option<&str>,
    b_start: Option<&str>,
    b_end: Option<&str>,
) -> bool {
    let parse_bound = |raw: Option<&str>| -> Result<Option<chrono::NaiveDate>, ()> {
        match parse_date_prefix(raw) {
            None => Ok(None),
            Some(Ok(d)) => Ok(Some(d)),
            Some(Err(())) => Err(()),
        }
    };
    let (Ok(as_norm), Ok(ae_norm), Ok(bs_norm), Ok(be_norm)) = (
        parse_bound(a_start),
        parse_bound(a_end),
        parse_bound(b_start),
        parse_bound(b_end),
    ) else {
        return false;
    };

    if let (Some(ae), Some(bs)) = (ae_norm, bs_norm) {
        if ae < bs {
            return false;
        }
    }
    if let (Some(be), Some(as_)) = (be_norm, as_norm) {
        if be < as_ {
            return false;
        }
    }
    true
}

/// Selects and ranks active `AdCampaign` items from the database (`all_ads`) for the current page context.
///
/// **STRICT COMPETITOR PROTECTION ON SPONSORED CONTENT**:
/// When `content` is a sponsored article (`is_sponsored == true`), ALL competing `defense_oem`
/// ads are filtered out (unless they belong to the exact same `sponsor_brand`), leaving ONLY
/// `auxiliary_service` and `in_house` / `is_default_fallback` ads from the database.
///
/// Zero hardcoded mock fallback records: if contextual filtering yields no matches, selects real
/// `is_default_fallback` rows directly from `all_ads`.
pub fn select_ads_for_context(
    all_ads: &[AdCampaign],
    page_category: Option<&str>,
    content: Option<&ContentItem>,
    user: Option<&UserAccount>,
) -> Vec<AdCampaign> {
    let is_sponsored_page = content
        .map(crate::services::content_service::is_sponsored_or_advertorial)
        .unwrap_or(false);

    let sponsor_brand_norm = content
        .and_then(|c| c.sponsor_brand.as_ref())
        .map(|b| b.trim().to_lowercase())
        .unwrap_or_default();

    let user_institution = user
        .map(|u| u.organization_category.trim())
        .unwrap_or("");

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let mut scored_ads: Vec<(i64, AdCampaign)> = all_ads
        .iter()
        .filter(|ad| ad.is_active && is_ad_within_schedule(ad, &today))
        .filter(|ad| {
            if !is_sponsored_page {
                return true;
            }
            // Strict competitor shield on sponsored content pages
            if ad.sponsor_type.eq_ignore_ascii_case("defense_oem") {
                return !sponsor_brand_norm.is_empty()
                    && ad.sponsor_name.trim().to_lowercase() == sponsor_brand_norm;
            }
            ad.sponsor_type.eq_ignore_ascii_case("auxiliary_service")
                || ad.sponsor_type.eq_ignore_ascii_case("in_house")
                || ad.is_default_fallback
        })
        .map(|ad| {
            let mut score: i64 = ad.budget_score as i64;
            let mut matched_dimension = false;

            // 1. Institution Targeting (MSB, SSB, K.K.K.lığı, Dz.K.K.lığı, Hv.K.K.lığı, etc.)
            if !user_institution.is_empty() {
                if ad
                    .target_institutions
                    .iter()
                    .any(|inst| inst.eq_ignore_ascii_case(user_institution))
                {
                    score += 160;
                    matched_dimension = true;
                } else if !ad.target_institutions.is_empty()
                    && !ad
                        .target_institutions
                        .iter()
                        .any(|i| i == "*" || i.eq_ignore_ascii_case("all"))
                {
                    score -= 35;
                }
            }

            // 2. SSB Catalog Tag Targeting (Article SSB tags + User SSB interests)
            if let Some(c) = content {
                for tag in &c.ssb_tags {
                    if ad
                        .target_ssb_tags
                        .iter()
                        .any(|t| t.eq_ignore_ascii_case(tag))
                    {
                        score += 130;
                        matched_dimension = true;
                    }
                }
            }
            if let Some(u) = user {
                for interest in &u.ssb_interests {
                    if ad
                        .target_ssb_tags
                        .iter()
                        .any(|t| t.eq_ignore_ascii_case(interest))
                    {
                        score += 95;
                        matched_dimension = true;
                    }
                }
            }
            if let Some(cat) = page_category {
                if ad
                    .target_ssb_tags
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(cat))
                {
                    score += 110;
                    matched_dimension = true;
                }
            }

            // 3. Category & Placement Targeting
            if let Some(cat) = page_category {
                if ad
                    .target_categories
                    .iter()
                    .any(|tc| tc.eq_ignore_ascii_case(cat))
                {
                    score += 110;
                    matched_dimension = true;
                }
            }
            if let Some(c) = content {
                for placement in &c.placements {
                    if ad
                        .target_categories
                        .iter()
                        .any(|tc| tc.eq_ignore_ascii_case(placement))
                    {
                        score += 85;
                        matched_dimension = true;
                    }
                }
            }

            // 4. Default In-House / Fallback boost when no specific targeting matched
            if !matched_dimension && ad.is_default_fallback {
                score += 60;
            }

            // On sponsored pages, prioritize same-sponsor or auxiliary/in-house fallback cleanly
            if is_sponsored_page && ad.sponsor_type.eq_ignore_ascii_case("in_house") {
                score += 40;
            }

            (score, ad.clone())
        })
        .collect();

    scored_ads.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.budget_score.cmp(&a.1.budget_score))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });

    let mut result: Vec<AdCampaign> = scored_ads.into_iter().map(|(_, ad)| ad).collect();

    // Real DB fallback selection (zero hardcoded fake campaigns/metrics)
    if result.is_empty() {
        let mut db_fallbacks: Vec<AdCampaign> = all_ads
            .iter()
            .filter(|ad| ad.is_active && ad.is_default_fallback && is_ad_within_schedule(ad, &today))
            .cloned()
            .collect();
        db_fallbacks.sort_by(|a, b| {
            b.budget_score
                .cmp(&a.budget_score)
                .then_with(|| a.id.cmp(&b.id))
        });
        result = db_fallbacks;
    }

    result
}

/// Checks whether adding or activating `new_ad` causes a keyword/category/institution quota collision
/// against `existing_ads` (based on `max_quota_per_keyword`).
pub fn check_ad_conflict(
    new_ad: &AdCampaign,
    existing_ads: &[AdCampaign],
    max_quota_per_keyword: usize,
) -> AdConflictWarning {
    check_ad_conflict_with_window(
        new_ad,
        existing_ads,
        max_quota_per_keyword,
        None,
        None,
        None,
    )
}

pub fn check_ad_conflict_with_window(
    new_ad: &AdCampaign,
    existing_ads: &[AdCampaign],
    max_quota_per_keyword: usize,
    start_date: Option<&str>,
    end_date: Option<&str>,
    rotation_mode: Option<&str>,
) -> AdConflictWarning {
    let eff_start = normalize_date_prefix(start_date.or(new_ad.start_date.as_deref()));
    let eff_end = normalize_date_prefix(end_date.or(new_ad.end_date.as_deref()));
    let eff_rotation = rotation_mode
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let rm = new_ad.rotation_mode.trim();
            if rm.is_empty() { None } else { Some(rm) }
        });
    let new_is_exclusive = eff_rotation
        .map(|m| m.eq_ignore_ascii_case("exclusive"))
        .unwrap_or(false);
    let base_quota = max_quota_per_keyword.max(1);
    let mut hit_exclusive = new_is_exclusive;
    let mut conflicting_campaigns: Vec<String> = Vec::new();
    let mut max_observed_count: usize = 0;
    let mut conflict_keywords: Vec<String> = Vec::new();

    let active_others: Vec<&AdCampaign> = existing_ads
        .iter()
        .filter(|ad| {
            ad.is_active
                && ad.id != new_ad.id
                && !ad.is_default_fallback
                && schedules_overlap(
                    eff_start,
                    eff_end,
                    ad.start_date.as_deref(),
                    ad.end_date.as_deref(),
                )
        })
        .collect();

    // 1. Check SSB tag quota collisions
    for ssb_tag in &new_ad.target_ssb_tags {
        let matching: Vec<&&AdCampaign> = active_others
            .iter()
            .filter(|ad| {
                ad.target_ssb_tags
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(ssb_tag))
            })
            .collect();

        let count_with_new = matching.len() + 1;
        if count_with_new > max_observed_count {
            max_observed_count = count_with_new;
        }

        let tag_has_exclusive = new_is_exclusive
            || matching
                .iter()
                .any(|m| m.rotation_mode.trim().eq_ignore_ascii_case("exclusive"));
        let tag_quota = if tag_has_exclusive { 1 } else { base_quota };
        if tag_has_exclusive && !matching.is_empty() {
            hit_exclusive = true;
        }

        if matching.len() >= tag_quota {
            conflict_keywords.push(format!("SSB:{}", ssb_tag));
            for m in matching {
                let label = format!("{} ({} - #{})", m.sponsor_name, m.slot_code, m.id);
                if !conflicting_campaigns.contains(&label) {
                    conflicting_campaigns.push(label);
                }
            }
        }
    }

    // 2. Check Same Slot + Target Category / Institution collisions
    for other in &active_others {
        if other.slot_code.eq_ignore_ascii_case(&new_ad.slot_code) {
            let shared_inst = new_ad.target_institutions.iter().any(|inst| {
                other
                    .target_institutions
                    .iter()
                    .any(|oi| oi.eq_ignore_ascii_case(inst))
            });
            let shared_cat = new_ad.target_categories.iter().any(|cat| {
                other
                    .target_categories
                    .iter()
                    .any(|oc| oc.eq_ignore_ascii_case(cat))
            });

            if shared_inst || shared_cat {
                let label = format!(
                    "{} ({} - #{})",
                    other.sponsor_name, other.slot_code, other.id
                );
                if !conflicting_campaigns.contains(&label) {
                    conflicting_campaigns.push(label);
                }
            }
        }
    }

    let same_slot: Vec<&&AdCampaign> = active_others
        .iter()
        .filter(|ad| ad.slot_code.eq_ignore_ascii_case(&new_ad.slot_code))
        .collect();
    let same_slot_count = same_slot.len();
    let slot_has_exclusive = new_is_exclusive
        || same_slot
            .iter()
            .any(|m| m.rotation_mode.trim().eq_ignore_ascii_case("exclusive"));
    let slot_quota = if slot_has_exclusive { 1 } else { base_quota };
    if slot_has_exclusive && same_slot_count > 0 {
        hit_exclusive = true;
    }

    if same_slot_count >= slot_quota {
        if same_slot_count + 1 > max_observed_count {
            max_observed_count = same_slot_count + 1;
        }
        conflict_keywords.push(format!("SLOT:{}", new_ad.slot_code));
        for m in same_slot {
            let label = format!("{} ({} - #{})", m.sponsor_name, m.slot_code, m.id);
            if !conflicting_campaigns.contains(&label) {
                conflicting_campaigns.push(label);
            }
        }
    }

    let effective_quota = if hit_exclusive { 1 } else { base_quota };
    let has_conflict = !conflicting_campaigns.is_empty()
        && (max_observed_count > effective_quota || !conflict_keywords.is_empty());

    let window_note = match (start_date, end_date) {
        (Some(s), Some(e)) if !s.trim().is_empty() && !e.trim().is_empty() => {
            format!(
                " [Tarih Aralığı: {} → {}, Mod: {}]",
                s.trim(),
                e.trim(),
                rotation_mode.unwrap_or("equal_rotation")
            )
        }
        _ => String::new(),
    };

    let message_tr = if has_conflict {
        format!(
            "UYARI: '{}' slotunda veya [{}] hedef etiketlerinde maksimum kampanya kotası ({}/{}) aşıldı{}. Çakışan aktif kampanyalar: {}. Yine de mevcut kampanyaların yanına eklemek istiyor musunuz?",
            new_ad.slot_code,
            conflict_keywords.join(", "),
            max_observed_count,
            effective_quota,
            window_note,
            conflicting_campaigns.join(", ")
        )
    } else if !conflicting_campaigns.is_empty() {
        format!(
            "BİLGİ: '{}' alanında benzer hedeflemeye sahip {} kampanya bulunuyor ({}){}, ancak kota sınırı ({}) aşılmadı.",
            new_ad.slot_code,
            conflicting_campaigns.len(),
            conflicting_campaigns.join(", "),
            window_note,
            effective_quota
        )
    } else {
        format!(
            "Çakışma tespit edilmedi{}. Reklam alanı ve SSB anahtar kelime kotası uygun.",
            window_note
        )
    };

    AdConflictWarning {
        has_conflict,
        conflicting_campaigns,
        message_tr,
        quota_limit: effective_quota,
        current_count: max_observed_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn make_ad(
        id: i64,
        slot_code: &str,
        sponsor_name: &str,
        sponsor_type: &str,
        rotation_mode: &str,
        tags: &[&str],
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> AdCampaign {
        AdCampaign {
            id,
            slot_code: slot_code.to_string(),
            badge_label: "SPONSOR".to_string(),
            dimensions: "728x90".to_string(),
            sponsor_name: sponsor_name.to_string(),
            sponsor_type: sponsor_type.to_string(),
            pill_style: "solid".to_string(),
            title_tr: sponsor_name.to_string(),
            title_en: sponsor_name.to_string(),
            desc_tr: "Desc".to_string(),
            desc_en: "Desc".to_string(),
            target_url: "https://example.com".to_string(),
            image_url: None,
            budget_score: 50,
            is_default_fallback: false,
            is_floating_popup: false,
            is_newsletter_ad: false,
            target_categories: vec!["haberler".to_string()],
            target_ssb_tags: tags.iter().map(|t| t.to_string()).collect(),
            target_institutions: vec!["MSB".to_string()],
            impressions_count: 0,
            clicks_count: 0,
            is_active: true,
            start_date: start_date.map(ToString::to_string),
            end_date: end_date.map(ToString::to_string),
            rotation_mode: rotation_mode.to_string(),
        }
    }

    #[test]
    fn empty_db_ads_returns_empty_vec_without_hardcoded_mock() {
        let selected = select_ads_for_context(&[], None, None, None);
        assert!(selected.is_empty());
    }

    #[test]
    fn char_prefix_and_normalize_date_prefix_handle_multibyte_utf8_safely() {
        assert_eq!(char_prefix("2026-01-1ğxyz", 10), "2026-01-1ğ");
        assert_eq!(char_prefix("ğüşöçiğüşöç", 10), "ğüşöçiğüşö");
        assert_eq!(char_prefix("ğüşö", 10), "ğüşö");
        assert_eq!(
            normalize_date_prefix(Some("  2026-01-0ğ999  ")),
            Some("2026-01-0ğ")
        );
    }

    #[test]
    fn advertorials_plural_with_is_sponsored_false_triggers_competitor_protection() {
        let mut content = ContentItem {
            id: 42,
            slug: "sponsorlu-dosya-test".to_string(),
            content_type: "AdVeRtOrIaLs".to_string(),
            is_sponsored: false,
            sponsor_brand: Some("Aselsan".to_string()),
            ..ContentItem::default()
        };

        let same_sponsor = make_ad(1, "ad-1", "ASELSAN", "defense_oem", "weighted", &[], None, None);
        let competitor = make_ad(2, "ad-2", "Roketsan", "defense_oem", "weighted", &[], None, None);
        let auxiliary = make_ad(
            3,
            "ad-3",
            "Otel Partner",
            "auxiliary_service",
            "weighted",
            &[],
            None,
            None,
        );
        let all_ads = vec![same_sponsor, competitor, auxiliary];

        let selected = select_ads_for_context(&all_ads, Some("haberler"), Some(&content), None);
        let names: Vec<&str> = selected.iter().map(|a| a.sponsor_name.as_str()).collect();
        assert!(names.contains(&"ASELSAN"));
        assert!(names.contains(&"Otel Partner"));
        assert!(
            !names.contains(&"Roketsan"),
            "Competing defense_oem ad must be shielded on advertorials page even when is_sponsored=false"
        );

        // Also test without sponsor_brand
        content.sponsor_brand = None;
        let selected_no_brand = select_ads_for_context(&all_ads, Some("haberler"), Some(&content), None);
        assert!(selected_no_brand
            .iter()
            .all(|a| !a.sponsor_type.eq_ignore_ascii_case("defense_oem")));
    }

    #[test]
    fn check_ad_conflict_is_symmetric_for_weighted_over_exclusive_and_disjoint_safe() {
        let existing_exclusive = make_ad(
            10,
            "ad-mag-top",
            "ExclusiveOEM",
            "defense_oem",
            "exclusive",
            &["hava"],
            Some("2026-01-01"),
            Some("2026-12-31"),
        );

        // 1. Weighted over overlapping exclusive on same slot -> conflict, quota_limit = 1
        let weighted_same_slot = make_ad(
            0,
            "ad-mag-top",
            "WeightedSameSlot",
            "defense_oem",
            "weighted",
            &["deniz"],
            Some("2026-06-01"),
            Some("2026-07-01"),
        );
        let w_slot = check_ad_conflict_with_window(
            &weighted_same_slot,
            std::slice::from_ref(&existing_exclusive),
            3,
            weighted_same_slot.start_date.as_deref(),
            weighted_same_slot.end_date.as_deref(),
            Some("weighted"),
        );
        assert!(w_slot.has_conflict);
        assert_eq!(w_slot.quota_limit, 1);
        assert!(w_slot.current_count >= 2);

        // 2. Weighted over overlapping exclusive on different slot + shared SSB tag -> conflict, quota_limit = 1
        let weighted_shared_tag = make_ad(
            0,
            "ad-news-top",
            "WeightedSharedTag",
            "defense_oem",
            "weighted",
            &["hava"],
            Some("2026-06-01"),
            Some("2026-07-01"),
        );
        let w_tag = check_ad_conflict_with_window(
            &weighted_shared_tag,
            std::slice::from_ref(&existing_exclusive),
            3,
            weighted_shared_tag.start_date.as_deref(),
            weighted_shared_tag.end_date.as_deref(),
            Some("weighted"),
        );
        assert!(w_tag.has_conflict);
        assert_eq!(w_tag.quota_limit, 1);
        assert!(w_tag.current_count >= 2);

        // 3. Disjoint exclusive ad must NOT shrink quota for unrelated weighted ads
        let other_weighted = make_ad(
            11,
            "ad-news-top",
            "OtherWeighted",
            "defense_oem",
            "weighted",
            &["deniz"],
            Some("2026-01-01"),
            Some("2026-12-31"),
        );
        let new_disjoint_weighted = make_ad(
            0,
            "ad-news-top",
            "NewDisjointWeighted",
            "defense_oem",
            "weighted",
            &["deniz"],
            Some("2026-06-01"),
            Some("2026-07-01"),
        );
        let w_disjoint = check_ad_conflict_with_window(
            &new_disjoint_weighted,
            &[existing_exclusive.clone(), other_weighted],
            3,
            new_disjoint_weighted.start_date.as_deref(),
            new_disjoint_weighted.end_date.as_deref(),
            Some("weighted"),
        );
        assert!(!w_disjoint.has_conflict);
        assert_eq!(w_disjoint.quota_limit, 3);

        // 4. Non-overlapping weighted over exclusive -> no conflict
        let weighted_non_overlap = make_ad(
            0,
            "ad-mag-top",
            "WeightedFuture",
            "defense_oem",
            "weighted",
            &["hava"],
            Some("2027-01-01"),
            Some("2027-12-31"),
        );
        let w_non_overlap = check_ad_conflict_with_window(
            &weighted_non_overlap,
            &[existing_exclusive],
            3,
            weighted_non_overlap.start_date.as_deref(),
            weighted_non_overlap.end_date.as_deref(),
            Some("weighted"),
        );
        assert!(!w_non_overlap.has_conflict);
    }

    #[test]
    fn parse_date_prefix_and_is_ad_within_schedule_fail_closed_on_malformed_dates() {
        assert_eq!(parse_date_prefix(None), None);
        assert_eq!(parse_date_prefix(Some("   ")), None);
        assert_eq!(
            parse_date_prefix(Some("2026-11-21T23:59:59Z")),
            Some(Ok(chrono::NaiveDate::from_ymd_opt(2026, 11, 21).unwrap()))
        );
        assert_eq!(parse_date_prefix(Some("invalid-date")), Some(Err(())));
        assert_eq!(parse_date_prefix(Some("2026-01-0ğ999")), Some(Err(())));
        assert_eq!(parse_date_prefix(Some("2026-02-30")), Some(Err(())));
        assert_eq!(parse_date_prefix(Some("+026-01-01")), Some(Err(())));
        assert_eq!(parse_date_prefix(Some("2026- 1-01")), Some(Err(())));

        let malformed_end = make_ad(
            20,
            "ad-home-top",
            "MalformedEndOEM",
            "defense_oem",
            "weighted",
            &[],
            Some("2020-01-01"),
            Some("invalid-date"),
        );
        assert!(
            !is_ad_within_schedule(&malformed_end, "2026-11-21"),
            "Malformed end_date must fail closed"
        );

        let malformed_start = make_ad(
            21,
            "ad-home-top",
            "MalformedStartOEM",
            "defense_oem",
            "weighted",
            &[],
            Some("invalid-date"),
            Some("2035-12-31"),
        );
        assert!(
            !is_ad_within_schedule(&malformed_start, "2026-11-21"),
            "Malformed start_date must fail closed"
        );

        let valid_iso = make_ad(
            22,
            "ad-home-top",
            "ValidIsoOEM",
            "defense_oem",
            "weighted",
            &[],
            Some("2026-11-21T00:00:00Z"),
            Some("2026-11-21T23:59:59Z"),
        );
        assert!(is_ad_within_schedule(&valid_iso, "2026-11-21"));
        assert!(!is_ad_within_schedule(&valid_iso, "2026-11-22"));

        let unbounded = make_ad(
            23,
            "ad-home-top",
            "UnboundedOEM",
            "defense_oem",
            "weighted",
            &[],
            None,
            None,
        );
        assert!(is_ad_within_schedule(&unbounded, "2026-11-21"));
    }
}
