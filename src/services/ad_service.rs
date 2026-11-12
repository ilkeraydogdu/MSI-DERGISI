use crate::models::{AdCampaign, AdConflictWarning, ContentItem, UserAccount};

/// Selects and ranks active `AdCampaign` items for the current page context.
///
/// **STRICT COMPETITOR PROTECTION ON SPONSORED CONTENT**:
/// When `content` is a sponsored article (`is_sponsored == true`), ALL competing `defense_oem`
/// ads are filtered out (unless they belong to the exact same `sponsor_brand`), leaving ONLY
/// `auxiliary_service` (logistics, travel, event services) and `in_house` fallback ads
/// (`MSI Dergisi`, `AMAC Magazine`, `KasabaWorks`).
///
/// Multi-dimensional scoring ranks eligible ads by:
/// 1. Reader's institution (`target_institutions` matching `user.organization_category`)
/// 2. Content & reader SSB interest tags (`target_ssb_tags`)
/// 3. Page & content section categories (`target_categories`)
/// 4. Campaign `budget_score` and `is_default_fallback`
pub fn select_ads_for_context(
    all_ads: &[AdCampaign],
    page_category: Option<&str>,
    content: Option<&ContentItem>,
    user: Option<&UserAccount>,
) -> Vec<AdCampaign> {
    let is_sponsored_page = content
        .map(|c| c.is_sponsored || c.content_type.eq_ignore_ascii_case("advertorial"))
        .unwrap_or(false);

    let sponsor_brand_norm = content
        .and_then(|c| c.sponsor_brand.as_ref())
        .map(|b| b.trim().to_lowercase())
        .unwrap_or_default();

    let user_institution = user
        .map(|u| u.organization_category.trim())
        .unwrap_or("");

    let mut scored_ads: Vec<(i64, AdCampaign)> = all_ads
        .iter()
        .filter(|ad| ad.is_active)
        .filter(|ad| {
            if !is_sponsored_page {
                return true;
            }
            // Strict competitor shield on sponsored content pages
            if ad.sponsor_type.eq_ignore_ascii_case("defense_oem") {
                if !sponsor_brand_norm.is_empty()
                    && ad.sponsor_name.trim().to_lowercase() == sponsor_brand_norm
                {
                    return true;
                }
                return false;
            }
            // Allow auxiliary_service and in_house (MSI Dergisi, AMAC Magazine, KasabaWorks)
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
    if result.is_empty() {
        result.push(AdCampaign {
            id: 9999,
            slot_code: "ad-inhouse-rbf-fallback".to_string(),
            badge_label: "KURUMSAL ÇÖZÜM ORTAĞI".to_string(),
            dimensions: "970 × 90 STANDART VARSAYILAN VİTRİN".to_string(),
            sponsor_name: "REMOVE BEFORE PHOTOS (RBF) & MSI DERGİSİ".to_string(),
            sponsor_type: "in_house".to_string(),
            pill_style: "navy".to_string(),
            title_tr: "Hava-Hava, Kara ve Deniz Platformlarında Telifli Havacılık & Savunma Fotoğrafçılığı".to_string(),
            title_en: "Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms".to_string(),
            desc_tr: "Onur Kurç & Tayfun Yaşar yönetiminde Remove Before Photos yüksek çözünürlüklü telifli savunma görsel arşivi ve MSI Dergisi kurumsal abonelik çözümleri.".to_string(),
            desc_en: "Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.".to_string(),
            target_url: "/abonelik-ve-satis".to_string(),
            image_url: Some("images/msi/msi-dergisi-sayi-286-kapak.webp".to_string()),
            budget_score: 80,
            is_default_fallback: true,
            is_floating_popup: false,
            is_newsletter_ad: true,
            target_categories: vec!["*".to_string()],
            target_ssb_tags: vec!["hava-araclari".to_string()],
            target_institutions: vec!["*".to_string()],
            impressions_count: 19500,
            clicks_count: 840,
            is_active: true,
        });
    }
    result
}

/// Checks whether adding or activating `new_ad` causes a keyword/category/institution quota collision
/// against `existing_ads` (based on `max_quota_per_keyword`).
/// Returns a detailed `AdConflictWarning` so the operator receives an interactive warning without hard-blocking override.
pub fn check_ad_conflict(
    new_ad: &AdCampaign,
    existing_ads: &[AdCampaign],
    max_quota_per_keyword: usize,
) -> AdConflictWarning {
    check_ad_conflict_with_window(new_ad, existing_ads, max_quota_per_keyword, None, None, None)
}

pub fn check_ad_conflict_with_window(
    new_ad: &AdCampaign,
    existing_ads: &[AdCampaign],
    max_quota_per_keyword: usize,
    start_date: Option<&str>,
    end_date: Option<&str>,
    rotation_mode: Option<&str>,
) -> AdConflictWarning {
    let is_exclusive = rotation_mode
        .map(|m| m.eq_ignore_ascii_case("exclusive"))
        .unwrap_or(false);
    let effective_quota = if is_exclusive {
        1
    } else {
        max_quota_per_keyword.max(1)
    };
    let mut conflicting_campaigns: Vec<String> = Vec::new();
    let mut max_observed_count: usize = 0;
    let mut conflict_keywords: Vec<String> = Vec::new();

    let active_others: Vec<&AdCampaign> = existing_ads
        .iter()
        .filter(|ad| ad.is_active && ad.id != new_ad.id && !ad.is_default_fallback)
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

        if matching.len() >= effective_quota {
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
                let label = format!("{} ({} - #{})", other.sponsor_name, other.slot_code, other.id);
                if !conflicting_campaigns.contains(&label) {
                    conflicting_campaigns.push(label);
                }
            }
        }
    }

    let same_slot_count = active_others
        .iter()
        .filter(|ad| ad.slot_code.eq_ignore_ascii_case(&new_ad.slot_code))
        .count();

    if same_slot_count >= effective_quota {
        if same_slot_count + 1 > max_observed_count {
            max_observed_count = same_slot_count + 1;
        }
        conflict_keywords.push(format!("SLOT:{}", new_ad.slot_code));
        for m in active_others
            .iter()
            .filter(|ad| ad.slot_code.eq_ignore_ascii_case(&new_ad.slot_code))
        {
            let label = format!("{} ({} - #{})", m.sponsor_name, m.slot_code, m.id);
            if !conflicting_campaigns.contains(&label) {
                conflicting_campaigns.push(label);
            }
        }
    }

    let has_conflict = !conflicting_campaigns.is_empty()
        && (max_observed_count > effective_quota || !conflict_keywords.is_empty());

    let window_note = match (start_date, end_date) {
        (Some(s), Some(e)) if !s.trim().is_empty() && !e.trim().is_empty() => {
            format!(" [Tarih Aralığı: {} → {}, Mod: {}]", s.trim(), e.trim(), rotation_mode.unwrap_or("equal_rotation"))
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
