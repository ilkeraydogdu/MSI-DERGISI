use std::collections::HashMap;
use sha2::{Digest, Sha256};

use crate::models::{
    AdAnalyticsReport, AdCampaign, AnalyticsEvent, ContentAnalyticsReport, ContentItem,
    InstitutionBreakdownItem, ReaderAnalyticsReport, UserAccount,
};

/// Computes a comprehensive B2B defense analytics report for a single `ContentItem`,
/// including total views, true reads (`duration_sec >= 25 && scroll_depth_pct >= 70`),
/// average reading duration, average scroll depth %, and institution breakdown (`MSB`, `SSB`, `K.K.K.lığı`, etc.).
pub fn compute_content_report(
    content: &ContentItem,
    events: &[AnalyticsEvent],
) -> ContentAnalyticsReport {
    compute_content_report_with_options(content, events, false)
}

pub fn compute_content_report_with_options(
    content: &ContentItem,
    events: &[AnalyticsEvent],
    anonymize: bool,
) -> ContentAnalyticsReport {
    let matching_events: Vec<&AnalyticsEvent> = events
        .iter()
        .filter(|e| {
            e.content_slug
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case(&content.slug))
                .unwrap_or(false)
                && !e.event_type.starts_with("ad_")
        })
        .collect();

    let event_count = matching_events.len() as i64;
    let computed_true_reads = matching_events
        .iter()
        .filter(|e| (e.duration_sec >= crate::db::constants::TRUE_READ_MIN_DURATION_SEC && e.scroll_depth_pct >= crate::db::constants::TRUE_READ_MIN_SCROLL_PCT) || e.is_true_read)
        .count() as i64;

    // Only use real analytics events — never fall back to seed/harvested counters
    // which represent traffic from the original site, not this deployment.
    let total_views = event_count;
    let true_reads = computed_true_reads;

    let (avg_duration_sec, avg_scroll_pct) = if !matching_events.is_empty() {
        let sum_dur: i64 = matching_events.iter().map(|e| e.duration_sec as i64).sum();
        let sum_scroll: i64 = matching_events
            .iter()
            .map(|e| e.scroll_depth_pct as i64)
            .sum();
        let n = matching_events.len() as f64;
        (
            round_one_decimal(sum_dur as f64 / n),
            round_one_decimal(sum_scroll as f64 / n),
        )
    } else {
        (0.0, 0.0)
    };

    let min_k = if anonymize { 3 } else { 1 };
    let institution_breakdown = build_institution_breakdown_k(&matching_events, min_k);

    ContentAnalyticsReport {
        content_slug: content.slug.clone(),
        title_tr: content.title_tr.clone(),
        total_views,
        true_reads,
        avg_duration_sec,
        avg_scroll_pct,
        institution_breakdown,
    }
}

/// Computes a sponsor-ready advertising performance report for an `AdCampaign`,
/// calculating impressions, clicks, CTR %, and separate institution breakdowns for
/// both impressions ("Görenlerin kurum dağılımı") and clicks ("Tıklayanların kurum dağılımı").
pub fn compute_ad_report(ad: &AdCampaign, events: &[AnalyticsEvent]) -> AdAnalyticsReport {
    compute_ad_report_with_options(ad, events, false)
}

pub fn compute_ad_report_with_options(
    ad: &AdCampaign,
    events: &[AnalyticsEvent],
    anonymize: bool,
) -> AdAnalyticsReport {
    let ad_events: Vec<&AnalyticsEvent> =
        events.iter().filter(|e| e.ad_id == Some(ad.id)).collect();

    let impression_events: Vec<&AnalyticsEvent> = ad_events
        .iter()
        .copied()
        .filter(|e| {
            matches!(
                e.event_type.as_str(),
                "ad_impression" | "impression" | "ad_view"
            )
        })
        .collect();

    let click_events: Vec<&AnalyticsEvent> = ad_events
        .iter()
        .copied()
        .filter(|e| matches!(e.event_type.as_str(), "ad_click" | "click"))
        .collect();

    // Only use real analytics events — never fall back to seed/harvested counters
    // which represent traffic from the original site, not this deployment.
    let impressions = impression_events.len() as i64;
    let clicks = click_events.len() as i64;

    let ctr_pct = if impressions > 0 {
        ((clicks as f64 / impressions as f64) * crate::db::constants::CTR_MULTIPLIER).round() / 100.0
    } else {
        0.0
    };

    let min_k = if anonymize { 3 } else { 1 };
    let impression_institutions = build_institution_breakdown_k(&impression_events, min_k);
    let click_institutions = build_institution_breakdown_k(&click_events, min_k);

    AdAnalyticsReport {
        ad_id: ad.id,
        sponsor_name: ad.sponsor_name.clone(),
        slot_code: ad.slot_code.clone(),
        impressions,
        clicks,
        ctr_pct,
        impression_institutions,
        click_institutions,
    }
}

/// Generates reader-level engagement and ad interaction reports for all registered `UserAccount`s.
pub fn compute_reader_reports(
    users: &[UserAccount],
    events: &[AnalyticsEvent],
    ads: &[AdCampaign],
) -> Vec<ReaderAnalyticsReport> {
    let ad_lookup: HashMap<i64, String> = ads
        .iter()
        .map(|a| (a.id, format!("{} ({})", a.sponsor_name, a.slot_code)))
        .collect();

    users
        .iter()
        .map(|user| {
            let user_events: Vec<&AnalyticsEvent> = events
                .iter()
                .filter(|e| e.user_id == Some(user.id))
                .collect();

            let mut articles_read: Vec<String> = Vec::new();
            let mut ssb_keywords_engaged: Vec<String> = user.ssb_interests.clone();
            let mut ad_impressions_count: i64 = 0;
            let mut ad_clicks_count: i64 = 0;
            let mut clicked_ads: Vec<String> = Vec::new();

            for ev in user_events {
                if let Some(slug) = &ev.content_slug {
                    if !slug.trim().is_empty() && !articles_read.contains(slug) {
                        articles_read.push(slug.clone());
                    }
                }

                if let Some(tag) = &ev.ssb_tag {
                    if !tag.trim().is_empty() && !ssb_keywords_engaged.contains(tag) {
                        ssb_keywords_engaged.push(tag.clone());
                    }
                }

                if let Some(ad_id) = ev.ad_id {
                    match ev.event_type.as_str() {
                        "ad_impression" | "impression" | "ad_view" => {
                            ad_impressions_count += 1;
                        }
                        "ad_click" | "click" => {
                            ad_clicks_count += 1;
                            let ad_label = ad_lookup
                                .get(&ad_id)
                                .cloned()
                                .unwrap_or_else(|| format!("Kampanya #{}", ad_id));
                            if !clicked_ads.contains(&ad_label) {
                                clicked_ads.push(ad_label);
                            }
                        }
                        _ => {}
                    }
                }
            }

            ReaderAnalyticsReport {
                user_id: user.id,
                full_name: user.full_name.clone(),
                email: user.email.clone(),
                organization_category: user.organization_category.clone(),
                organization_name: user.organization_name.clone(),
                subscription_tier: user.subscription_tier.clone(),
                articles_read,
                ssb_keywords_engaged,
                ad_impressions_count,
                ad_clicks_count,
                clicked_ads,
            }
        })
        .collect()
}

fn anon_hash8(salt: &str, user_id: i64, email: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("msi-anon-b3:{}:{}:{}", salt, user_id, email.trim().to_lowercase()).as_bytes());
    let digest = hex::encode(hasher.finalize());
    digest[..8].to_string()
}

pub fn compute_reader_reports_anonymized(
    users: &[UserAccount],
    events: &[AnalyticsEvent],
    ads: &[AdCampaign],
    salt: &str,
) -> Vec<ReaderAnalyticsReport> {
    let mut org_name_counts: HashMap<String, usize> = HashMap::new();
    let mut org_cat_counts: HashMap<String, usize> = HashMap::new();
    for u in users {
        if u.role == "deleted" {
            continue;
        }
        let on = u.organization_name.trim().to_string();
        if !on.is_empty() {
            *org_name_counts.entry(on).or_insert(0) += 1;
        }
        let oc = u.organization_category.trim().to_string();
        if !oc.is_empty() {
            *org_cat_counts.entry(oc).or_insert(0) += 1;
        }
    }

    let raw = compute_reader_reports(users, events, ads);
    raw.into_iter()
        .map(|mut r| {
            let h8 = anon_hash8(salt, r.user_id, &r.email);
            let org_n = r.organization_name.trim().to_string();
            let org_c = r.organization_category.trim().to_string();
            r.user_id = 0;
            r.full_name = format!("Okur #{}", h8);
            // Must not contain '@' or raw email components
            r.email = format!("anon_{}", h8);
            r.organization_name = if org_n.is_empty() || org_name_counts.get(&org_n).copied().unwrap_or(0) < 3 {
                "Gizli Kurum (k<3)".to_string()
            } else {
                org_n
            };
            r.organization_category = if org_c.is_empty() || org_cat_counts.get(&org_c).copied().unwrap_or(0) < 3 {
                "Diğer Kurumlar (k<3)".to_string()
            } else {
                org_c
            };
            r
        })
        .collect()
}

pub fn apply_k_anonymity_to_institution_counts(
    counts: Vec<(String, i64)>,
    min_k: i64,
) -> Vec<InstitutionBreakdownItem> {
    if counts.is_empty() {
        return Vec::new();
    }
    let total_sum: i64 = counts.iter().map(|(_, c)| *c).sum();
    if total_sum <= 0 {
        return Vec::new();
    }
    let total = total_sum as f64;
    let mut retained: Vec<(String, i64)> = Vec::new();
    let mut suppressed_sum: i64 = 0;
    for (label, count) in counts {
        if count >= min_k {
            retained.push((label, count));
        } else {
            suppressed_sum += count;
        }
    }
    if min_k > 1 && suppressed_sum >= min_k {
        retained.push(("Diğer Kurumlar (k<3)".to_string(), suppressed_sum));
    }
    let mut breakdown: Vec<InstitutionBreakdownItem> = retained
        .into_iter()
        .map(|(organization_category, count)| InstitutionBreakdownItem {
            organization_category,
            count,
            percentage: round_one_decimal((count as f64 / total) * 100.0),
        })
        .collect();
    breakdown.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.organization_category.cmp(&b.organization_category))
    });
    breakdown
}

pub fn anonymize_open_data_users(users: &[UserAccount], salt: &str) -> Vec<serde_json::Value> {
    let mut org_name_counts: HashMap<String, usize> = HashMap::new();
    let mut org_cat_counts: HashMap<String, usize> = HashMap::new();
    for u in users {
        if u.role == "deleted" {
            continue;
        }
        *org_name_counts.entry(u.organization_name.trim().to_string()).or_insert(0) += 1;
        *org_cat_counts.entry(u.organization_category.trim().to_string()).or_insert(0) += 1;
    }
    users
        .iter()
        .map(|u| {
            let h8 = anon_hash8(salt, u.id, &u.email);
            let org_name = if org_name_counts.get(u.organization_name.trim()).copied().unwrap_or(0) < 3 {
                "Gizli Kurum (k<3)"
            } else {
                u.organization_name.trim()
            };
            let org_cat = if org_cat_counts.get(u.organization_category.trim()).copied().unwrap_or(0) < 3 {
                "Diğer Kurumlar (k<3)"
            } else {
                u.organization_category.trim()
            };
            serde_json::json!({
                "user_id": 0,
                "anon_id": format!("anon_{}", h8),
                "full_name": format!("Okur #{}", h8),
                "email": format!("anon_{}", h8),
                "role": u.role,
                "organization_category": org_cat,
                "organization_name": org_name,
                "is_gov_verified": u.is_gov_verified,
                "subscription_tier": u.subscription_tier,
                "ssb_interests": u.ssb_interests,
                "created_at": u.created_at.get(..7).unwrap_or(&u.created_at)
            })
        })
        .collect()
}

fn build_institution_breakdown_k(
    events: &[&AnalyticsEvent],
    min_k: i64,
) -> Vec<InstitutionBreakdownItem> {
    if events.is_empty() {
        return Vec::new();
    }

    let mut counts: HashMap<String, i64> = HashMap::new();
    for ev in events {
        let cat = if ev.organization_category.trim().is_empty() {
            "Diğer Kamu Kurumları".to_string()
        } else {
            ev.organization_category.trim().to_string()
        };
        *counts.entry(cat).or_insert(0) += 1;
    }

    apply_k_anonymity_to_institution_counts(counts.into_iter().collect(), min_k)
}

#[allow(dead_code)]
fn build_institution_breakdown(events: &[&AnalyticsEvent]) -> Vec<InstitutionBreakdownItem> {
    build_institution_breakdown_k(events, 1)
}

fn round_one_decimal(val: f64) -> f64 {
    (val * 10.0).round() / 10.0
}

/// Normalizes ISO `'T'` and SQLite space `' '` timestamps to `YYYY-MM-DD HH:MM:SS`
/// so lexicographical cutoff comparisons never fail when timestamps use `' '` instead of `'T'`.
fn normalize_timestamp_for_cmp(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let replaced = trimmed.replace('T', " ").trim_end_matches('Z').to_string();
    if replaced.len() == 10 {
        format!("{} 00:00:00", replaced)
    } else if replaced.len() >= 19 {
        replaced[..19].to_string()
    } else {
        replaced
    }
}

// =========================================================================
// Content Performance Analytics (Author, Category & Trending)
// =========================================================================

/// Computes author-based performance metrics by grouping content analytics by `author_name`.
/// Returns authors sorted by `total_views` descending.
pub fn compute_author_performance(
    contents: &[ContentItem],
    events: &[AnalyticsEvent],
) -> Vec<serde_json::Value> {
    let mut author_map: HashMap<String, serde_json::Map<String, serde_json::Value>> =
        HashMap::new();

    for content in contents {
        let author = if content.author_name.trim().is_empty() {
            "unknown".to_string()
        } else {
            content.author_name.trim().to_string()
        };
        let report = compute_content_report(content, events);

        let entry = author_map.entry(author.clone()).or_insert_with(|| {
            let mut m = serde_json::Map::new();
            m.insert("author".to_string(), serde_json::json!(author));
            m.insert("content_count".to_string(), serde_json::json!(0i64));
            m.insert("total_views".to_string(), serde_json::json!(0i64));
            m.insert("total_true_reads".to_string(), serde_json::json!(0i64));
            m.insert("sum_duration".to_string(), serde_json::json!(0.0f64));
            m.insert("sum_scroll".to_string(), serde_json::json!(0.0f64));
            m
        });

        entry["content_count"] =
            serde_json::json!(entry["content_count"].as_i64().unwrap_or(0) + 1);
        entry["total_views"] =
            serde_json::json!(entry["total_views"].as_i64().unwrap_or(0) + report.total_views);
        entry["total_true_reads"] =
            serde_json::json!(entry["total_true_reads"].as_i64().unwrap_or(0) + report.true_reads);
        entry["sum_duration"] = serde_json::json!(
            entry["sum_duration"].as_f64().unwrap_or(0.0) + report.avg_duration_sec
        );
        entry["sum_scroll"] =
            serde_json::json!(entry["sum_scroll"].as_f64().unwrap_or(0.0) + report.avg_scroll_pct);
    }

    let mut results: Vec<serde_json::Value> = author_map
        .into_values()
        .map(|mut m| {
            let count = m["content_count"].as_i64().unwrap_or(1).max(1) as f64;
            m.insert(
                "avg_duration_sec".to_string(),
                serde_json::json!(round_one_decimal(
                    m["sum_duration"].as_f64().unwrap_or(0.0) / count
                )),
            );
            m.insert(
                "avg_scroll_pct".to_string(),
                serde_json::json!(round_one_decimal(
                    m["sum_scroll"].as_f64().unwrap_or(0.0) / count
                )),
            );
            let views = m["total_views"].as_i64().unwrap_or(0).max(1) as f64;
            let true_reads = m["total_true_reads"].as_i64().unwrap_or(0) as f64;
            let engagement = round_one_decimal((true_reads / views) * 100.0);
            m.insert("engagement_pct".to_string(), serde_json::json!(engagement));
            m.remove("sum_duration");
            m.remove("sum_scroll");
            serde_json::Value::Object(m)
        })
        .collect();

    results.sort_by(|a, b| {
        b["total_views"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&a["total_views"].as_i64().unwrap_or(0))
    });

    results
}

/// Computes category-based performance trends by grouping content analytics by primary `content_type`.
/// Returns categories sorted by `total_views` descending.
pub fn compute_category_performance(
    contents: &[ContentItem],
    events: &[AnalyticsEvent],
) -> Vec<serde_json::Value> {
    let mut cat_map: HashMap<String, serde_json::Map<String, serde_json::Value>> = HashMap::new();

    for content in contents {
        let category = if content.content_type.trim().is_empty() {
            "uncategorized".to_string()
        } else {
            content.content_type.trim().to_string()
        };
        let report = compute_content_report(content, events);

        let entry = cat_map.entry(category.clone()).or_insert_with(|| {
            let mut m = serde_json::Map::new();
            m.insert("category".to_string(), serde_json::json!(category));
            m.insert("content_count".to_string(), serde_json::json!(0i64));
            m.insert("total_views".to_string(), serde_json::json!(0i64));
            m.insert("total_true_reads".to_string(), serde_json::json!(0i64));
            m.insert("sum_duration".to_string(), serde_json::json!(0.0f64));
            m
        });

        entry["content_count"] =
            serde_json::json!(entry["content_count"].as_i64().unwrap_or(0) + 1);
        entry["total_views"] =
            serde_json::json!(entry["total_views"].as_i64().unwrap_or(0) + report.total_views);
        entry["total_true_reads"] =
            serde_json::json!(entry["total_true_reads"].as_i64().unwrap_or(0) + report.true_reads);
        entry["sum_duration"] = serde_json::json!(
            entry["sum_duration"].as_f64().unwrap_or(0.0) + report.avg_duration_sec
        );
    }

    let mut results: Vec<serde_json::Value> = cat_map
        .into_values()
        .map(|mut m| {
            let count = m["content_count"].as_i64().unwrap_or(1).max(1) as f64;
            m.insert(
                "avg_duration_sec".to_string(),
                serde_json::json!(round_one_decimal(
                    m["sum_duration"].as_f64().unwrap_or(0.0) / count
                )),
            );
            let views = m["total_views"].as_i64().unwrap_or(0).max(1) as f64;
            let true_reads = m["total_true_reads"].as_i64().unwrap_or(0) as f64;
            m.insert(
                "engagement_pct".to_string(),
                serde_json::json!(round_one_decimal((true_reads / views) * 100.0)),
            );
            m.remove("sum_duration");
            serde_json::Value::Object(m)
        })
        .collect();

    results.sort_by(|a, b| {
        b["total_views"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&a["total_views"].as_i64().unwrap_or(0))
    });

    results
}

/// Returns trending content: top `limit` articles by recent event activity over the last 7 days
/// using space-safe timestamp normalization (`YYYY-MM-DD HH:MM:SS`).
pub fn compute_trending_content(
    contents: &[ContentItem],
    events: &[AnalyticsEvent],
    limit: usize,
) -> Vec<serde_json::Value> {
    let seven_days_ago = chrono::Utc::now() - chrono::Duration::days(7);
    let cutoff = seven_days_ago.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut slug_counts: HashMap<String, i64> = HashMap::new();
    for ev in events {
        if ev.event_type.starts_with("ad_") {
            continue;
        }
        let ev_ts = normalize_timestamp_for_cmp(&ev.created_at);
        if ev_ts >= cutoff {
            if let Some(ref slug) = ev.content_slug {
                let norm_slug = slug.trim().to_lowercase();
                if !norm_slug.is_empty() {
                    *slug_counts.entry(norm_slug).or_insert(0) += 1;
                }
            }
        }
    }

    let mut trending: Vec<serde_json::Value> = contents
        .iter()
        .filter(|c| crate::services::content_service::is_publicly_visible(c))
        .filter_map(|c| {
            let recent_views = slug_counts
                .get(&c.slug.trim().to_lowercase())
                .copied()
                .unwrap_or(0);
            if recent_views > 0 {
                Some(serde_json::json!({
                    "slug": c.slug,
                    "title_tr": c.title_tr,
                    "author_name": c.author_name,
                    "content_type": c.content_type,
                    "recent_views_7d": recent_views,
                    "total_views": c.view_count,
                    "true_reads": c.true_read_count,
                }))
            } else {
                None
            }
        })
        .collect();

    trending.sort_by(|a, b| {
        b["recent_views_7d"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&a["recent_views_7d"].as_i64().unwrap_or(0))
            .then_with(|| {
                b["total_views"]
                    .as_i64()
                    .unwrap_or(0)
                    .cmp(&a["total_views"].as_i64().unwrap_or(0))
            })
    });

    trending.truncate(limit);
    trending
}
