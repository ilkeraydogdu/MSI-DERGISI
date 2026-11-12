use std::collections::HashMap;

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
        .filter(|e| (e.duration_sec >= 25 && e.scroll_depth_pct >= 70) || e.is_true_read)
        .count() as i64;

    let total_views = if event_count > 0 {
        event_count.max(content.view_count)
    } else {
        content.view_count
    };

    let true_reads = if event_count > 0 {
        computed_true_reads.max(content.true_read_count)
    } else {
        content.true_read_count
    };

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

    let institution_breakdown = build_institution_breakdown(&matching_events);

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

    let impressions = if !impression_events.is_empty() {
        (impression_events.len() as i64).max(ad.impressions_count)
    } else {
        ad.impressions_count
    };

    let clicks = if !click_events.is_empty() {
        (click_events.len() as i64).max(ad.clicks_count)
    } else {
        ad.clicks_count
    };

    let ctr_pct = if impressions > 0 {
        ((clicks as f64 / impressions as f64) * 10000.0).round() / 100.0
    } else {
        0.0
    };

    let impression_institutions = build_institution_breakdown(&impression_events);
    let click_institutions = build_institution_breakdown(&click_events);

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

fn build_institution_breakdown(events: &[&AnalyticsEvent]) -> Vec<InstitutionBreakdownItem> {
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

    let total = events.len() as f64;
    let mut breakdown: Vec<InstitutionBreakdownItem> = counts
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

fn round_one_decimal(val: f64) -> f64 {
    (val * 10.0).round() / 10.0
}
