use std::time::Duration;
use chrono::Datelike;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::controllers::AppState;
use crate::engine::security_engine::escape_html;
use crate::models::{B2bMonthlyReport, InstitutionBreakdownItem};
use crate::services::{analytics_service, email_service};

#[derive(Clone, Debug)]
pub struct ConsolidatedB2bReport {
    pub report_month: String,
    pub subject: String,
    pub summary_json: Value,
    pub html_body: String,
    pub text_body: String,
}

fn previous_month_yyyy_mm() -> String {
    let today = chrono::Utc::now().date_naive();
    let (year, month) = if today.month() == 1 {
        (today.year() - 1, 12)
    } else {
        (today.year(), today.month() - 1)
    };
    format!("{:04}-{:02}", year, month)
}

fn format_institution_breakdown(items: &[InstitutionBreakdownItem]) -> String {
    if items.is_empty() {
        return "Veri yok".to_string();
    }
    items
        .iter()
        .map(|i| format!("%{:.1} {} ({})", i.percentage, i.organization_category, i.count))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Builds a consolidated monthly B2B report combining subscriber/revenue metrics with
/// per-content and per-ad institutional distribution breakdowns (`%35.0 MSB`, `%25.0 SSB`, etc.).
pub fn build_consolidated_b2b_report(state: &AppState, report_month: &str) -> ConsolidatedB2bReport {
    let mut summary = state.db.compute_subscriber_report(Some(report_month));
    let month_start = format!("{}-01", report_month);
    let month_end = format!("{}-31", report_month);
    let month_events = state.db.get_analytics_events_paginated(
        10_000,
        0,
        Some(&month_start),
        Some(&month_end),
    );

    let all_contents = state.db.get_all_contents();
    let mut content_entries: Vec<(i64, Value)> = all_contents
        .iter()
        .filter_map(|c| {
            let matching_month: Vec<_> = month_events
                .iter()
                .filter(|e| {
                    e.content_slug
                        .as_deref()
                        .map(|s| s.eq_ignore_ascii_case(&c.slug))
                        .unwrap_or(false)
                        && !e.event_type.starts_with("ad_")
                })
                .collect();
            if matching_month.is_empty() {
                return None;
            }
            let month_views = matching_month.len() as i64;
            let month_true_reads = matching_month
                .iter()
                .filter(|e| {
                    (e.duration_sec >= crate::db::constants::TRUE_READ_MIN_DURATION_SEC
                        && e.scroll_depth_pct >= crate::db::constants::TRUE_READ_MIN_SCROLL_PCT)
                        || e.is_true_read
                })
                .count() as i64;
            let report = analytics_service::compute_content_report_with_options(c, &month_events, false);
            Some((
                month_views,
                json!({
                    "content_slug": report.content_slug,
                    "title_tr": report.title_tr,
                    "month_views": month_views,
                    "month_true_reads": month_true_reads,
                    "total_views": report.total_views,
                    "true_reads": report.true_reads,
                    "avg_duration_sec": report.avg_duration_sec,
                    "avg_scroll_pct": report.avg_scroll_pct,
                    "institution_breakdown": report.institution_breakdown,
                    "institution_summary": format_institution_breakdown(&report.institution_breakdown),
                }),
            ))
        })
        .collect();
    content_entries.sort_by_key(|a| std::cmp::Reverse(a.0));
    let content_reports: Vec<Value> = content_entries.into_iter().take(50).map(|(_, v)| v).collect();

    let all_ads = state.db.get_all_ads();
    let mut ad_entries: Vec<(i64, Value)> = all_ads
        .iter()
        .filter_map(|a| {
            let matching_month: Vec<_> = month_events
                .iter()
                .filter(|e| e.ad_id == Some(a.id))
                .collect();
            if matching_month.is_empty() && !a.is_active {
                return None;
            }
            let month_impressions = matching_month
                .iter()
                .filter(|e| {
                    matches!(
                        e.event_type.as_str(),
                        "ad_impression" | "impression" | "ad_view"
                    )
                })
                .count() as i64;
            let month_clicks = matching_month
                .iter()
                .filter(|e| matches!(e.event_type.as_str(), "ad_click" | "click"))
                .count() as i64;
            let report = analytics_service::compute_ad_report_with_options(a, &month_events, false);
            Some((
                month_impressions + month_clicks,
                json!({
                    "ad_id": report.ad_id,
                    "sponsor_name": report.sponsor_name,
                    "slot_code": report.slot_code,
                    "month_impressions": month_impressions,
                    "month_clicks": month_clicks,
                    "impressions": report.impressions,
                    "clicks": report.clicks,
                    "ctr_pct": report.ctr_pct,
                    "impression_institutions": report.impression_institutions,
                    "click_institutions": report.click_institutions,
                    "impression_summary": format_institution_breakdown(&report.impression_institutions),
                    "click_summary": format_institution_breakdown(&report.click_institutions),
                }),
            ))
        })
        .collect();
    ad_entries.sort_by_key(|a| std::cmp::Reverse(a.0));
    let ad_reports: Vec<Value> = ad_entries.into_iter().take(50).map(|(_, v)| v).collect();

    if let Some(obj) = summary.as_object_mut() {
        obj.insert("content_reports".to_string(), json!(content_reports));
        obj.insert("ad_reports".to_string(), json!(ad_reports));
    }

    let subject = format!(
        "MSI Dergisi Aylık B2B Abone, İçerik ve Reklam Raporu — {}",
        report_month
    );

    let total_users = summary["total_users"].as_i64().unwrap_or(0);
    let active_subs = summary["active_subscribers"].as_i64().unwrap_or(0);
    let gov_verified = summary["gov_verified_users"].as_i64().unwrap_or(0);
    let revenue = summary["monthly_revenue_try"].as_f64().unwrap_or(0.0);

    let mut text_lines = vec![
        subject.clone(),
        "=".repeat(subject.len()),
        format!("Toplam Kullanıcı: {}", total_users),
        format!("Aktif Abone: {}", active_subs),
        format!("Doğrulanmış Kamu Okuru: {}", gov_verified),
        format!("Aylık Fatura Geliri: ₺{:.2}", revenue),
        String::new(),
        "İÇERİK BAZLI KURUMSAL DAĞILIM:".to_string(),
    ];

    let mut content_rows_html = String::new();
    if content_reports.is_empty() {
        text_lines.push("  - Bu ay için içerik okuma etkinliği kaydı yok.".to_string());
        content_rows_html.push_str("<tr><td colspan=\"4\" style=\"padding:8px;border:1px solid #dcdedc;color:#5b626c;\">Bu ay için içerik okuma etkinliği kaydı yok.</td></tr>");
    } else {
        for cr in &content_reports {
            let title = cr["title_tr"].as_str().unwrap_or("-");
            let slug = cr["content_slug"].as_str().unwrap_or("-");
            let m_views = cr["month_views"].as_i64().unwrap_or(0);
            let m_true = cr["month_true_reads"].as_i64().unwrap_or(0);
            let inst_sum = cr["institution_summary"].as_str().unwrap_or("Veri yok");
            text_lines.push(format!(
                "  * {} ({}) — {} görüntüleme, {} gerçek okuma | Kurumlar: {}",
                title, slug, m_views, m_true, inst_sum
            ));
            content_rows_html.push_str(&format!(
                "<tr><td style=\"padding:8px;border:1px solid #dcdedc;\"><strong>{}</strong><br><span style=\"font-size:11px;color:#5b626c;\">{}</span></td><td style=\"padding:8px;border:1px solid #dcdedc;\">{}</td><td style=\"padding:8px;border:1px solid #dcdedc;\">{}</td><td style=\"padding:8px;border:1px solid #dcdedc;\">{}</td></tr>",
                escape_html(title),
                escape_html(slug),
                m_views,
                m_true,
                escape_html(inst_sum),
            ));
        }
    }

    text_lines.push(String::new());
    text_lines.push("REKLAM / SPONSOR BAZLI KURUMSAL DAĞILIM:".to_string());

    let mut ad_rows_html = String::new();
    if ad_reports.is_empty() {
        text_lines.push("  - Bu ay için reklam kampanyası etkinliği kaydı yok.".to_string());
        ad_rows_html.push_str("<tr><td colspan=\"5\" style=\"padding:8px;border:1px solid #dcdedc;color:#5b626c;\">Bu ay için reklam kampanyası etkinliği kaydı yok.</td></tr>");
    } else {
        for ar in &ad_reports {
            let sponsor = ar["sponsor_name"].as_str().unwrap_or("-");
            let slot = ar["slot_code"].as_str().unwrap_or("-");
            let m_imp = ar["month_impressions"].as_i64().unwrap_or(0);
            let m_clk = ar["month_clicks"].as_i64().unwrap_or(0);
            let imp_sum = ar["impression_summary"].as_str().unwrap_or("Veri yok");
            let clk_sum = ar["click_summary"].as_str().unwrap_or("Veri yok");
            text_lines.push(format!(
                "  * {} ({}) — {} gösterim, {} tıklama | Görenler: {} | Tıklayanlar: {}",
                sponsor, slot, m_imp, m_clk, imp_sum, clk_sum
            ));
            ad_rows_html.push_str(&format!(
                "<tr><td style=\"padding:8px;border:1px solid #dcdedc;\"><strong>{}</strong><br><span style=\"font-size:11px;color:#5b626c;\">{}</span></td><td style=\"padding:8px;border:1px solid #dcdedc;\">{}</td><td style=\"padding:8px;border:1px solid #dcdedc;\">{}</td><td style=\"padding:8px;border:1px solid #dcdedc;\">{}</td><td style=\"padding:8px;border:1px solid #dcdedc;\">{}</td></tr>",
                escape_html(sponsor),
                escape_html(slot),
                m_imp,
                m_clk,
                escape_html(imp_sum),
                escape_html(clk_sum),
            ));
        }
    }

    let html_body = format!(
        r#"<!doctype html><html lang="tr"><body style="font-family:Arial,sans-serif;background:#f5f4f1;padding:24px;color:#242b35;">
        <div style="max-width:780px;margin:0 auto;background:#ffffff;border:1px solid #dcdedc;border-top:4px solid #b51230;padding:24px;">
            <div style="font-size:11px;letter-spacing:0.12em;color:#b51230;font-weight:700;">MSI DERGİSİ · AYLIK B2B KONSOLİDE RAPOR</div>
            <h2 style="margin:8px 0 16px;">{}</h2>
            <p>Toplam Kullanıcı: <strong>{}</strong> · Aktif Abone: <strong>{}</strong> · Doğrulanmış Kamu Okuru: <strong>{}</strong> · Aylık Fatura Geliri: <strong>₺{:.2}</strong></p>
            <h3 style="margin:20px 0 8px;">İçerik Bazlı Kurumsal Dağılım</h3>
            <table style="width:100%;border-collapse:collapse;font-size:13px;">
                <thead><tr style="background:#f8f9fa;"><th style="padding:8px;border:1px solid #dcdedc;text-align:left;">İçerik</th><th style="padding:8px;border:1px solid #dcdedc;text-align:left;">Aylık Görüntüleme</th><th style="padding:8px;border:1px solid #dcdedc;text-align:left;">Gerçek Okuma</th><th style="padding:8px;border:1px solid #dcdedc;text-align:left;">Kurumsal Dağılım</th></tr></thead>
                <tbody>{}</tbody>
            </table>
            <h3 style="margin:20px 0 8px;">Reklam / Sponsor Bazlı Kurumsal Dağılım</h3>
            <table style="width:100%;border-collapse:collapse;font-size:13px;">
                <thead><tr style="background:#f8f9fa;"><th style="padding:8px;border:1px solid #dcdedc;text-align:left;">Sponsor / Slot</th><th style="padding:8px;border:1px solid #dcdedc;text-align:left;">Aylık Gösterim</th><th style="padding:8px;border:1px solid #dcdedc;text-align:left;">Aylık Tıklama</th><th style="padding:8px;border:1px solid #dcdedc;text-align:left;">Görenlerin Kurum Dağılımı</th><th style="padding:8px;border:1px solid #dcdedc;text-align:left;">Tıklayanların Kurum Dağılımı</th></tr></thead>
                <tbody>{}</tbody>
            </table>
        </div></body></html>"#,
        escape_html(&subject),
        total_users,
        active_subs,
        gov_verified,
        revenue,
        content_rows_html,
        ad_rows_html,
    );

    ConsolidatedB2bReport {
        report_month: report_month.to_string(),
        subject,
        summary_json: summary,
        html_body,
        text_body: text_lines.join("\n"),
    }
}

fn normalize_report_month(target_month: Option<&str>) -> String {
    target_month
        .map(str::trim)
        .filter(|s| s.chars().count() >= 7)
        .map(|s| s.chars().take(7).collect::<String>())
        .unwrap_or_else(previous_month_yyyy_mm)
}

/// Generates and idempotently records monthly B2B subscriber + per-content + per-ad reports (`B8` / `REQ-026`).
/// Returns number of newly inserted/dispatched reports.
pub async fn run_b2b_report_tick(
    state: &AppState,
    target_month: Option<&str>,
) -> Result<usize, String> {
    let report_month = normalize_report_month(target_month);

    let report_type = "monthly_subscriber_summary";
    let recipient = state.config.admin_default_user.trim().to_lowercase();
    if recipient.is_empty() {
        return Ok(0);
    }

    let report = build_consolidated_b2b_report(state, &report_month);
    let summary_json =
        serde_json::to_string(&report.summary_json).unwrap_or_else(|_| "{}".to_string());
    let now_rfc = chrono::Utc::now().to_rfc3339();

    let rec = B2bMonthlyReport {
        id: 0,
        report_month: report_month.clone(),
        report_type: report_type.to_string(),
        recipient_email: recipient.clone(),
        summary_json,
        status: "pending".to_string(),
        sent_at: String::new(),
    };

    let inserted = state.db.insert_b2b_monthly_report_if_new(&rec)?;
    if !inserted {
        return Ok(0);
    }

    let final_status = match email_service::send_multipart_mime(
        &state.config,
        &recipient,
        &report.subject,
        &report.text_body,
        &report.html_body,
    )
    .await
    {
        Ok(true) => "sent",
        Ok(false) => "generated",
        Err(e) => {
            warn!("B2B monthly report email failed for {}: {}", recipient, e);
            "generated"
        }
    };

    state.db.update_b2b_monthly_report_status(
        &report_month,
        report_type,
        &recipient,
        final_status,
        &now_rfc,
    )?;

    Ok(1)
}

pub fn spawn_b2b_report_worker(state: AppState, cancel: CancellationToken) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(6 * 3600));
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("B2B monthly report worker shutting down gracefully");
                    return;
                }
                _ = ticker.tick() => {
                    if let Err(e) = run_b2b_report_tick(&state, None).await {
                        warn!("B2B report worker tick error: {}", e);
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_report_month_is_char_safe_on_multibyte_utf8() {
        assert_eq!(normalize_report_month(Some(" 2026-11-01 ")), "2026-11");
        assert_eq!(normalize_report_month(Some("ğüşöçiğüşö")), "ğüşöçiğ");
        assert_eq!(normalize_report_month(Some("2026-01-ö")), "2026-01");
        assert_eq!(
            normalize_report_month(Some("ğüşö")),
            previous_month_yyyy_mm()
        );
        assert_eq!(normalize_report_month(None), previous_month_yyyy_mm());
    }
}
