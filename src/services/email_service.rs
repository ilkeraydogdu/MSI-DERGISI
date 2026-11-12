//! Transactional & Editorial Email Dispatch Engine (`src/services/email_service.rs`)
//!
//! Dispatches KVKK-consented newsletters and 5-stage subscription renewal reminders
//! using local Postfix/MTA (`sendmail -t -i`) with RFC 2047 UTF-8 headers and Base64 MIME bodies.

use base64::Engine;
use serde::Serialize;
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{info, warn};

use crate::config::AppConfig;
use crate::engine::security_engine::escape_html;
use crate::models::{AdCampaign, NewsletterCampaign, RenewalReminderLog, UserAccount};

#[derive(Debug, Clone, Serialize)]
pub struct EmailDispatchSummary {
    pub eligible_recipients: usize,
    pub delivered_count: usize,
    pub failed_count: usize,
    pub delivery_mode: String,
}

fn encode_rfc2047_utf8(text: &str) -> String {
    let clean = text.replace(['\r', '\n'], " ");
    let b64 = base64::engine::general_purpose::STANDARD.encode(clean.as_bytes());
    format!("=?UTF-8?B?{}?=", b64)
}

fn sanitize_header_addr(addr: &str) -> String {
    addr.chars()
        .filter(|c| !matches!(c, '\r' | '\n' | '\0'))
        .collect()
}

pub fn build_mime_email(from: &str, to: &str, subject: &str, html_body: &str) -> String {
    let encoded_subject = encode_rfc2047_utf8(subject);
    let body_b64 = base64::engine::general_purpose::STANDARD
        .encode(html_body.as_bytes())
        .as_bytes()
        .chunks(76)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\r\n");

    format!(
        "From: {}\r\nTo: {}\r\nSubject: {}\r\nMIME-Version: 1.0\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\nX-Mailer: MSI-Dergisi-Rust-MTA/2.6\r\n\r\n{}\r\n",
        sanitize_header_addr(from),
        sanitize_header_addr(to),
        encoded_subject,
        body_b64
    )
}

pub async fn send_raw_mime(config: &AppConfig, to: &str, subject: &str, html_body: &str) -> Result<bool, String> {
    let raw_msg = build_mime_email(&config.smtp_from, to, subject, html_body);

    if !config.mail_delivery_enabled {
        info!(
            "MTA delivery disabled (MAIL_DELIVERY_ENABLED=false); validated MIME message ({} bytes) for {}",
            raw_msg.len(),
            to
        );
        return Ok(false);
    }

    if !Path::new(&config.sendmail_bin).exists() {
        return Err(format!("MTA binary not found at {}", config.sendmail_bin));
    }

    let mut child = Command::new(&config.sendmail_bin)
        .args(["-t", "-i"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn sendmail: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(raw_msg.as_bytes())
            .await
            .map_err(|e| format!("Failed to write MIME payload to sendmail stdin: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed waiting for sendmail: {}", e))?;

    if output.status.success() {
        info!("Dispatched email via MTA to {}", to);
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        warn!("MTA returned non-zero status for {}: {}", to, stderr);
        Err(format!("sendmail exited with {}: {}", output.status, stderr))
    }
}

/// Filters KVKK-consented users matching the newsletter's target institution & SSB interest criteria.
pub fn filter_newsletter_recipients<'a>(
    users: &'a [UserAccount],
    target_institution: &str,
    target_ssb_tag: &str,
) -> Vec<&'a UserAccount> {
    let inst = target_institution.trim();
    let tag = target_ssb_tag.trim();
    let inst_all = inst.is_empty() || inst == "*" || inst.eq_ignore_ascii_case("all");
    let tag_all = tag.is_empty() || tag == "*" || tag.eq_ignore_ascii_case("all");

    users
        .iter()
        .filter(|u| u.email_consent && u.email.contains('@'))
        .filter(|u| inst_all || u.organization_category.eq_ignore_ascii_case(inst))
        .filter(|u| {
            tag_all
                || u.ssb_interests
                    .iter()
                    .any(|interest| interest.eq_ignore_ascii_case(tag))
        })
        .collect()
}

pub async fn dispatch_newsletter_campaign(
    config: &AppConfig,
    campaign: &NewsletterCampaign,
    recipients: &[&UserAccount],
    sponsor_ad: Option<&AdCampaign>,
) -> EmailDispatchSummary {
    let sponsor_html = sponsor_ad
        .map(|ad| {
            format!(
                r#"<div style="margin:20px 0;padding:16px;border:1px solid #dcdedc;background:#f8f9fa;">
                    <div style="font-size:10px;letter-spacing:0.12em;color:#5b626c;text-transform:uppercase;">SPONSORLU · {}</div>
                    <h3 style="margin:6px 0;font-size:16px;color:#242b35;">{}</h3>
                    <p style="margin:0 0 10px;font-size:13px;color:#5b626c;">{}</p>
                    <a href="{}" style="color:#b51230;font-weight:700;font-size:13px;text-decoration:none;">Detayları İncele →</a>
                </div>"#,
                escape_html(&ad.sponsor_name),
                escape_html(&ad.title_tr),
                escape_html(&ad.desc_tr),
                escape_html(&ad.target_url)
            )
        })
        .unwrap_or_default();

    let mut delivered = 0usize;
    let mut failed = 0usize;

    for user in recipients {
        let html = format!(
            r#"<!doctype html><html lang="tr"><body style="font-family:Arial,sans-serif;background:#f5f4f1;padding:24px;color:#242b35;">
            <div style="max-width:620px;margin:0 auto;background:#ffffff;border:1px solid #dcdedc;border-top:4px solid #b51230;padding:28px;">
                <div style="font-size:11px;letter-spacing:0.14em;color:#b51230;font-weight:700;">MSI DERGİSİ · KURUMSAL SAVUNMA BÜLTENİ</div>
                <h1 style="font-size:22px;line-height:1.3;margin:12px 0;">{}</h1>
                <p style="font-size:14px;color:#5b626c;">Sayın <strong>{}</strong> ({}),</p>
                <p style="font-size:14px;line-height:1.65;">İlgi alanlarınıza ve kurumsal profilinize özel derlenen güncel savunma, havacılık ve uzay teknolojileri bültenimiz yayındadır.</p>
                {}
                <p style="margin:24px 0;"><a href="{}/e-dergi" style="display:inline-block;background:#b51230;color:#ffffff;padding:12px 20px;text-decoration:none;font-weight:700;font-size:13px;">Güncel Sayıyı ve Analizleri Oku →</a></p>
                <hr style="border:0;border-top:1px solid #dcdedc;margin:24px 0;">
                <p style="font-size:11px;color:#5b626c;">Bu ileti 6698 sayılı KVKK kapsamındaki açık e-posta izninize istinaden {} adresine gönderilmiştir. Tercihlerinizi <a href="{}/hesabim" style="color:#b51230;">Okuyucu Merkezi</a> üzerinden güncelleyebilirsiniz.</p>
            </div></body></html>"#,
            escape_html(&campaign.subject_tr),
            escape_html(&user.full_name),
            escape_html(&user.organization_name),
            sponsor_html,
            escape_html(&config.public_url),
            escape_html(&user.email),
            escape_html(&config.public_url),
        );

        match send_raw_mime(config, &user.email, &campaign.subject_tr, &html).await {
            Ok(true) => delivered += 1,
            Ok(false) => {}
            Err(_) => failed += 1,
        }
    }

    EmailDispatchSummary {
        eligible_recipients: recipients.len(),
        delivered_count: delivered,
        failed_count: failed,
        delivery_mode: if config.mail_delivery_enabled {
            "postfix_sendmail_live".to_string()
        } else {
            "validated_queue_ready".to_string()
        },
    }
}

pub async fn dispatch_renewal_reminder_email(
    config: &AppConfig,
    reminder: &RenewalReminderLog,
) -> Result<bool, String> {
    let stage_human = match reminder.stage_label.as_str() {
        "2_months" => "Son 2 Ay (60 Gün)",
        "1_month" => "Son 1 Ay (30 Gün)",
        "2_weeks" => "Son 2 Hafta (14 Gün)",
        "1_week" => "Son 1 Hafta (7 Gün)",
        "1_day" => "Son 24 Saat (1 Gün)",
        other => other,
    };
    let subject = format!(
        "MSI Dergisi Abonelik Yenileme Bilgilendirmesi — {}",
        stage_human
    );
    let html = format!(
        r#"<!doctype html><html lang="tr"><body style="font-family:Arial,sans-serif;background:#f5f4f1;padding:24px;color:#242b35;">
        <div style="max-width:600px;margin:0 auto;background:#ffffff;border:1px solid #dcdedc;border-top:4px solid #b51230;padding:28px;">
            <div style="font-size:11px;letter-spacing:0.14em;color:#b51230;font-weight:700;">ABONELİK YENİLEME HATIRLATMASI ({})</div>
            <h1 style="font-size:20px;margin:12px 0;">Kesintisiz Savunma İstihbaratı Erişiminizi Koruyun</h1>
            <p style="font-size:14px;">Sayın <strong>{}</strong> ({}),</p>
            <p style="font-size:14px;line-height:1.65;">MSI Dergisi aboneliğinizin sona ermesine <strong>{} gün</strong> kalmıştır. Dijital arşiv, özel analiz dosyaları ve e-dergi erişiminizin kesintiye uğramaması için aboneliğinizi yenileyebilirsiniz. Yenileme tamamlandığında kalan otomatik hatırlatmalar anında durdurulur.</p>
            <p style="margin:24px 0;"><a href="{}/abonelik-ve-satis" style="display:inline-block;background:#b51230;color:#ffffff;padding:12px 20px;text-decoration:none;font-weight:700;font-size:13px;">Aboneliğimi Yenile →</a></p>
        </div></body></html>"#,
        escape_html(stage_human),
        escape_html(&reminder.full_name),
        escape_html(&reminder.organization_name),
        reminder.days_remaining,
        escape_html(&config.public_url),
    );

    send_raw_mime(config, &reminder.user_email, &subject, &html).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_rfc2047_base64_mime_message() {
        let raw = build_mime_email(
            "MSI Dergisi <noreply@msidergisi.com>",
            "okur@msb.gov.tr",
            "Özel Dosya: Çelik Kubbe & KAAN",
            "<p>Merhaba</p>",
        );
        assert!(raw.contains("From: MSI Dergisi <noreply@msidergisi.com>\r\n"));
        assert!(raw.contains("To: okur@msb.gov.tr\r\n"));
        assert!(raw.contains("Subject: =?UTF-8?B?"));
        assert!(raw.contains("Content-Transfer-Encoding: base64\r\n"));
    }
}
