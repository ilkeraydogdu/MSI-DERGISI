//! Transactional & Editorial Email Dispatch Engine (`src/services/email_service.rs`)
//!
//! Dispatches KVKK-consented newsletters (with RFC 8058 One-Click `List-Unsubscribe` headers),
//! 5-stage subscription renewal reminders, email verification links, and password reset links
//! using local Postfix/MTA (`sendmail -t -i`) with RFC 2047 UTF-8 headers and Base64 MIME bodies.

use base64::Engine;
use serde::Serialize;
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{info, warn};

use crate::config::AppConfig;
use crate::engine::security_engine::{escape_html, sanitize_rich_html};
use crate::models::{AdCampaign, NewsletterCampaign, RenewalReminderLog, UserAccount};
use crate::services::content_service::user_has_subscriber_access;

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

fn sanitize_header_value(val: &str) -> String {
    val.chars()
        .filter(|c| !matches!(c, '\r' | '\n' | '\0'))
        .collect()
}

/// Builds a standard RFC 2047 + Base64 HTML MIME message.
pub fn build_mime_email(from: &str, to: &str, subject: &str, html_body: &str) -> String {
    build_mime_email_with_unsubscribe(from, to, subject, html_body, None)
}

/// Builds an RFC 2047 + Base64 HTML MIME message with optional RFC 8058 One-Click
/// `List-Unsubscribe` and `List-Unsubscribe-Post` headers for marketing/newsletter compliance.
pub fn build_mime_email_with_unsubscribe(
    from: &str,
    to: &str,
    subject: &str,
    html_body: &str,
    unsub_url: Option<&str>,
) -> String {
    let encoded_subject = encode_rfc2047_utf8(subject);
    let body_b64 = base64::engine::general_purpose::STANDARD
        .encode(html_body.as_bytes())
        .as_bytes()
        .chunks(76)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\r\n");

    let unsub_headers = match unsub_url.map(sanitize_header_value) {
        Some(clean_url) if !clean_url.trim().is_empty() => format!(
            "List-Unsubscribe: <{}>\r\nList-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n",
            clean_url.trim()
        ),
        _ => String::new(),
    };

    format!(
        "From: {}\r\nTo: {}\r\nSubject: {}\r\nMIME-Version: 1.0\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n{}X-Mailer: MSI-Dergisi-Rust-MTA/2.6\r\n\r\n{}\r\n",
        sanitize_header_value(from),
        sanitize_header_value(to),
        encoded_subject,
        unsub_headers,
        body_b64
    )
}

pub async fn send_raw_mime(
    config: &AppConfig,
    to: &str,
    subject: &str,
    html_body: &str,
) -> Result<bool, String> {
    send_raw_mime_with_unsubscribe(config, to, subject, html_body, None).await
}

pub async fn send_raw_mime_with_unsubscribe(
    config: &AppConfig,
    to: &str,
    subject: &str,
    html_body: &str,
    unsub_url: Option<&str>,
) -> Result<bool, String> {
    let raw_msg =
        build_mime_email_with_unsubscribe(&config.smtp_from, to, subject, html_body, unsub_url);
    dispatch_mime_via_sendmail(config, to, &raw_msg).await
}

fn chunk_base64(input: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD
        .encode(input)
        .as_bytes()
        .chunks(76)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\r\n")
}

/// Builds an RFC 2046 `multipart/alternative` MIME message containing both
/// Base64-encoded `text/plain; charset=UTF-8` and `text/html; charset=UTF-8` parts.
pub fn build_multipart_mime_email(
    from: &str,
    to: &str,
    subject: &str,
    text_body: &str,
    html_body: &str,
) -> String {
    let encoded_subject = encode_rfc2047_utf8(subject);
    let boundary = format!("----=_MSI_ALT_{}", uuid::Uuid::new_v4().simple());
    let text_b64 = chunk_base64(text_body.as_bytes());
    let html_b64 = chunk_base64(html_body.as_bytes());

    format!(
        "From: {from}\r\nTo: {to}\r\nSubject: {subj}\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"{boundary}\"\r\nX-Mailer: MSI-Dergisi-Rust-MTA/2.6\r\n\r\n--{boundary}\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{text_b64}\r\n\r\n--{boundary}\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{html_b64}\r\n\r\n--{boundary}--\r\n",
        from = sanitize_header_value(from),
        to = sanitize_header_value(to),
        subj = encoded_subject,
        boundary = boundary,
        text_b64 = text_b64,
        html_b64 = html_b64,
    )
}

/// Sends a `multipart/alternative` (plain text + HTML) MIME email via the configured MTA,
/// respecting `config.mail_delivery_enabled == false`.
pub async fn send_multipart_mime(
    config: &AppConfig,
    to: &str,
    subject: &str,
    text_body: &str,
    html_body: &str,
) -> Result<bool, String> {
    let raw_msg = build_multipart_mime_email(&config.smtp_from, to, subject, text_body, html_body);
    dispatch_mime_via_sendmail(config, to, &raw_msg).await
}

fn extract_envelope_sender(smtp_from: &str) -> Option<String> {
    let candidate = if let (Some(lt), Some(gt)) = (smtp_from.rfind('<'), smtp_from.rfind('>')) {
        if lt < gt {
            smtp_from[lt + 1..gt].trim()
        } else {
            smtp_from.trim()
        }
    } else {
        smtp_from.trim()
    };
    if candidate.is_empty()
        || !candidate.contains('@')
        || candidate.starts_with('-')
        || candidate.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return None;
    }
    Some(candidate.to_string())
}

async fn dispatch_mime_via_sendmail(
    config: &AppConfig,
    to: &str,
    raw_msg: &str,
) -> Result<bool, String> {
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

    let mut args = vec!["-t".to_string(), "-i".to_string()];
    if let Some(envelope_from) = extract_envelope_sender(&config.smtp_from) {
        args.push("-f".to_string());
        args.push(envelope_from);
    }

    let mut child = Command::new(&config.sendmail_bin)
        .args(&args)
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

/// Filters KVKK-consented & marketing-opted-in users (`u.email_consent && u.marketing_emails_opted_in`)
/// matching `target_group_filter` (`"gov_verified"`, `"active_subscribers"`, `"all"`/`""`),
/// `target_institution`, and `target_ssb_tag`.
pub fn filter_newsletter_recipients<'a>(
    users: &'a [UserAccount],
    target_group_filter: &str,
    target_institution: &str,
    target_ssb_tag: &str,
) -> Vec<&'a UserAccount> {
    let group = target_group_filter.trim();
    let inst = target_institution.trim();
    let tag = target_ssb_tag.trim();

    let inst_all = inst.is_empty() || inst == "*" || inst.eq_ignore_ascii_case("all");
    let tag_all = tag.is_empty() || tag == "*" || tag.eq_ignore_ascii_case("all");

    let inst_tokens: Vec<&str> = inst
        .split(['&', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    users
        .iter()
        .filter(|u| u.email_consent && u.marketing_emails_opted_in && u.email.contains('@'))
        .filter(|u| {
            if group.is_empty() || group == "*" || group.eq_ignore_ascii_case("all") {
                true
            } else if group.eq_ignore_ascii_case("gov_verified") {
                u.is_gov_verified
            } else if group.eq_ignore_ascii_case("active_subscribers")
                || group.eq_ignore_ascii_case("subscribers")
            {
                user_has_subscriber_access(u)
            } else if group.eq_ignore_ascii_case("email_verified") {
                u.email_verified
            } else {
                true
            }
        })
        .filter(|u| {
            inst_all
                || inst_tokens
                    .iter()
                    .any(|tok| u.organization_category.eq_ignore_ascii_case(tok))
        })
        .filter(|u| {
            tag_all
                || u.ssb_interests
                    .iter()
                    .any(|interest| interest.eq_ignore_ascii_case(tag))
        })
        .collect()
}

/// Dispatches a newsletter campaign with optional custom editorial `body_html` and
/// RFC 8058 One-Click `List-Unsubscribe` headers per recipient.
pub async fn dispatch_newsletter_campaign(
    config: &AppConfig,
    db: &crate::db::Database,
    campaign: &NewsletterCampaign,
    recipients: &[&UserAccount],
    sponsor_ad: Option<&AdCampaign>,
    body_html: Option<&str>,
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

    let editorial_body_html = match body_html.map(str::trim) {
        Some(custom) if !custom.is_empty() => {
            format!(
                r#"<div style="font-size:14px;line-height:1.65;color:#242b35;">{}</div>"#,
                sanitize_rich_html(custom)
            )
        }
        _ => r#"<p style="font-size:14px;line-height:1.65;">İlgi alanlarınıza ve kurumsal profilinize özel derlenen güncel savunma, havacılık ve uzay teknolojileri bültenimiz yayındadır.</p>"#
            .to_string(),
    };

    let base_url = config.public_url.trim_end_matches('/');
    let mut delivered = 0usize;
    let mut failed = 0usize;

    for user in recipients {
        let raw_unsub_url = if user.id > 0 {
            let unsub_token = uuid::Uuid::new_v4().to_string();
            let _ = db.create_unsubscribe_token(user.id, &unsub_token);
            format!("{}/abonelik-iptal?token={}", base_url, unsub_token)
        } else {
            format!("{}/abonelik-iptal", base_url)
        };
        let escaped_unsub_url = escape_html(&raw_unsub_url);

        let html = format!(
            r#"<!doctype html><html lang="tr"><body style="font-family:Arial,sans-serif;background:#f5f4f1;padding:24px;color:#242b35;">
            <div style="max-width:620px;margin:0 auto;background:#ffffff;border:1px solid #dcdedc;border-top:4px solid #b51230;padding:28px;">
                <div style="font-size:11px;letter-spacing:0.14em;color:#b51230;font-weight:700;">MSI DERGİSİ · KURUMSAL SAVUNMA BÜLTENİ</div>
                <h1 style="font-size:22px;line-height:1.3;margin:12px 0;">{}</h1>
                <p style="font-size:14px;color:#5b626c;">Sayın <strong>{}</strong> ({}),</p>
                {}
                {}
                <p style="margin:24px 0;"><a href="{}/e-dergi" style="display:inline-block;background:#b51230;color:#ffffff;padding:12px 20px;text-decoration:none;font-weight:700;font-size:13px;">Güncel Sayıyı ve Analizleri Oku →</a></p>
                <hr style="border:0;border-top:1px solid #dcdedc;margin:24px 0;">
                <p style="font-size:11px;color:#5b626c;">Bu ileti 6698 sayılı KVKK kapsamındaki açık e-posta izninize istinaden {} adresine gönderilmiştir. Tercihlerinizi <a href="{}/hesabim" style="color:#b51230;">Okuyucu Merkezi</a> üzerinden güncelleyebilirsiniz.</p>
                <p style="font-size:11px;color:#5b626c;margin-top:8px;"><a href="{}" style="color:#b51230;">E-posta aboneliğinden çık</a></p>
            </div></body></html>"#,
            escape_html(&campaign.subject_tr),
            escape_html(&user.full_name),
            escape_html(&user.organization_name),
            editorial_body_html,
            sponsor_html,
            escape_html(base_url),
            escape_html(&user.email),
            escape_html(base_url),
            escaped_unsub_url,
        );

        match send_raw_mime_with_unsubscribe(
            config,
            &user.email,
            &campaign.subject_tr,
            &html,
            Some(&raw_unsub_url),
        )
        .await
        {
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
    let base_url = config.public_url.trim_end_matches('/');
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
        escape_html(base_url),
    );

    send_raw_mime(config, &reminder.user_email, &subject, &html).await
}

/// Sends an email verification email with a secure token link (`/hesap-dogrula?token={token}`).
pub async fn send_verification_email(
    config: &AppConfig,
    to: &str,
    token: &str,
    site_url: &str,
) -> Result<bool, String> {
    let base_url = if site_url.trim().is_empty() {
        config.public_url.trim_end_matches('/')
    } else {
        site_url.trim().trim_end_matches('/')
    };
    let verify_link = format!("{}/hesap-dogrula?token={}", base_url, token);
    let subject = "E-posta Doğrulama - MSI Dergisi";
    let html = format!(
        r#"<!doctype html><html lang="tr"><body style="font-family:Arial,sans-serif;background:#f5f4f1;padding:24px;color:#242b35;">
        <div style="max-width:600px;margin:0 auto;background:#ffffff;border:1px solid #dcdedc;border-top:4px solid #b51230;padding:28px;">
            <div style="font-size:11px;letter-spacing:0.14em;color:#b51230;font-weight:700;">MSI DERGİSİ · E-POSTA DOĞRULAMA</div>
            <h1 style="font-size:20px;margin:12px 0;">E-posta Adresinizi Doğrulayın</h1>
            <p style="font-size:14px;line-height:1.65;">MSI Dergisi hesabınızı tamamlamak için lütfen aşağıdaki bağlantıya tıklayarak e-posta adresinizi doğrulayın.</p>
            <p style="margin:24px 0;"><a href="{}" style="display:inline-block;background:#b51230;color:#ffffff;padding:12px 20px;text-decoration:none;font-weight:700;font-size:13px;">E-postamı Doğrula →</a></p>
            <p style="font-size:13px;color:#5b626c;">Bu link <strong>24 saat</strong> geçerlidir. Eğer bu talebi siz yapmadıysanız, bu e-postayı görmezden gelebilirsiniz.</p>
            <hr style="border:0;border-top:1px solid #dcdedc;margin:24px 0;">
            <p style="font-size:11px;color:#5b626c;">Güvenliğiniz için bu bağlantıyı kimseyle paylaşmayın. Sorularınız için <a href="{}/kurumsal/iletisim" style="color:#b51230;">iletişim sayfamızı</a> ziyaret edin.</p>
        </div></body></html>"#,
        escape_html(&verify_link),
        escape_html(base_url),
    );

    send_raw_mime(config, to, subject, &html).await
}

/// Sends a password reset email linking to `{public_url}/sifre-sifirla?token={token}`.
pub async fn send_password_reset_email(
    config: &AppConfig,
    to: &str,
    token: &str,
    site_url: &str,
) -> Result<bool, String> {
    let base_url = if site_url.trim().is_empty() {
        config.public_url.trim_end_matches('/')
    } else {
        site_url.trim().trim_end_matches('/')
    };
    let reset_link = format!("{}/sifre-sifirla?token={}", base_url, token);
    let subject = "Şifre Sıfırlama - MSI Dergisi";
    let html = format!(
        r#"<!doctype html><html lang="tr"><body style="font-family:Arial,sans-serif;background:#f5f4f1;padding:24px;color:#242b35;">
        <div style="max-width:600px;margin:0 auto;background:#ffffff;border:1px solid #dcdedc;border-top:4px solid #b51230;padding:28px;">
            <div style="font-size:11px;letter-spacing:0.14em;color:#b51230;font-weight:700;">MSI DERGİSİ · ŞİFRE SIFIRLAMA</div>
            <h1 style="font-size:20px;margin:12px 0;">Şifre Sıfırlama Talebi</h1>
            <p style="font-size:14px;line-height:1.65;">Hesabınız için bir şifre sıfırlama talebi aldık. Aşağıdaki bağlantıya tıklayarak yeni şifrenizi belirleyebilirsiniz.</p>
            <p style="margin:24px 0;"><a href="{}" style="display:inline-block;background:#b51230;color:#ffffff;padding:12px 20px;text-decoration:none;font-weight:700;font-size:13px;">Şifremi Sıfırla →</a></p>
            <p style="font-size:13px;color:#5b626c;">Bu link <strong>1 saat</strong> geçerlidir. Eğer bu talebi siz yapmadıysanız, bu e-postayı görmezden gelebilirsiniz.</p>
            <hr style="border:0;border-top:1px solid #dcdedc;margin:24px 0;">
            <p style="font-size:11px;color:#5b626c;">Güvenliğiniz için bu bağlantıyı kimseyle paylaşmayın. Sorularınız için <a href="{}/kurumsal/iletisim" style="color:#b51230;">iletişim sayfamızı</a> ziyaret edin.</p>
        </div></body></html>"#,
        escape_html(&reset_link),
        escape_html(base_url),
    );

    send_raw_mime(config, to, subject, &html).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_rfc2047_and_rfc8058_unsubscribe_mime_message() {
        let raw = build_mime_email_with_unsubscribe(
            "[REDACTED_SESSION_FALLBACK]",
            "okur@msb.gov.tr",
            "Özel Dosya: Çelik Kubbe & KAAN",
            "<p>Merhaba</p>",
            Some("https://msidergisi.com/abonelik-iptal?token=abc-123"),
        );
        assert!(raw.contains("From: [REDACTED_SESSION_FALLBACK]\r\n"));
        assert!(raw.contains("To: okur@msb.gov.tr\r\n"));
        assert!(raw.contains("Subject: =?UTF-8?B?"));
        assert!(raw.contains("Content-Transfer-Encoding: base64\r\n"));
        assert!(raw.contains("List-Unsubscribe: <https://msidergisi.com/abonelik-iptal?token=abc-123>\r\n"));
        assert!(raw.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n"));
    }

    #[test]
    fn filter_newsletter_recipients_enforces_kvkk_opt_in_and_target_groups() {
        let make = |id: i64, gov: bool, tier: &str, opt_in: bool| UserAccount {
            id,
            email: format!("u{}@test.com", id),
            full_name: format!("U{}", id),
            role: if tier == "free" { "registered" } else { "subscriber_digital" }.to_string(),
            organization_category: "SSB".to_string(),
            organization_name: "SSB".to_string(),
            job_title: "Uzman".to_string(),
            is_gov_verified: gov,
            subscription_tier: tier.to_string(),
            subscription_expires_at: Some("2030-12-31".to_string()),
            reminders_halted: false,
            kvkk_consent_at: "2026-01-01".to_string(),
            kvkk_consent_ip: "127.0.0.1".to_string(),
            kvkk_consent_hash: "KVKK".to_string(),
            email_consent: true,
            ssb_interests: vec!["hava-araclari".to_string()],
            created_at: "2026-01-01".to_string(),
            email_verified: true,
            marketing_emails_opted_in: opt_in,
            totp_enabled: false,
            tos_accepted_at: "2026-01-01T00:00:00Z".to_string(),
            tos_version: "2026-01".to_string(),
        };

        let users = vec![
            make(1, true, "Kamu E-Dergi Abonesi", true),
            make(2, false, "Dijital E-Dergi Abonesi", true),
            make(3, false, "free", true),
            make(4, true, "Kamu E-Dergi Abonesi", false), // opted out!
        ];

        let all_opted = filter_newsletter_recipients(&users, "all", "*", "*");
        assert_eq!(all_opted.len(), 3);

        let gov_only = filter_newsletter_recipients(&users, "gov_verified", "*", "*");
        assert_eq!(gov_only.len(), 1);
        assert_eq!(gov_only[0].id, 1);

        let active_subs = filter_newsletter_recipients(&users, "active_subscribers", "*", "*");
        assert_eq!(active_subs.len(), 2);
    }

    #[test]
    fn builds_valid_multipart_alternative_mime_message() {
        let text = "MSI B2B Rapor\n- Test İçerik: %60.0 MSB, %40.0 SSB";
        let html = "<h1>MSI B2B Rapor</h1><p>Test İçerik: %60.0 MSB, %40.0 SSB</p>";
        let raw = build_multipart_mime_email(
            "[REDACTED_SESSION_FALLBACK]",
            "admin@msidergisi.com",
            "MSI Dergisi Aylık B2B Raporu — 2026-10",
            text,
            html,
        );
        assert!(raw.contains("Content-Type: multipart/alternative; boundary=\"----=_MSI_ALT_"));
        assert!(raw.contains("Content-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n"));
        assert!(raw.contains("Content-Type: text/html; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n"));

        let b64_text = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
        let b64_html = base64::engine::general_purpose::STANDARD.encode(html.as_bytes());
        let stripped = raw.replace("\r\n", "");
        assert!(stripped.contains(&b64_text));
        assert!(stripped.contains(&b64_html));
    }

    #[test]
    fn extracts_safe_envelope_sender_from_smtp_from() {
        assert_eq!(
            extract_envelope_sender("MSI Dergisi <mail@ilkeraydogdu.com.tr>"),
            Some("mail@ilkeraydogdu.com.tr".to_string())
        );
        assert_eq!(
            extract_envelope_sender("noreply@msidergisi.com"),
            Some("noreply@msidergisi.com".to_string())
        );
        assert_eq!(extract_envelope_sender("-bad@example.com"), None);
        assert_eq!(extract_envelope_sender("no-at-sign"), None);
    }
}
