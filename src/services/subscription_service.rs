use chrono::{NaiveDate, Utc};
use sha2::{Digest, Sha256};

use crate::models::{OrderInvoice, RenewalReminderLog, UserAccount};

/// The 5 mandatory subscription renewal reminder stages before `subscription_expires_at`:
/// 2 months (60 days), 1 month (30 days), 2 weeks (14 days), 1 week (7 days), and 1 day (1 day).
pub const REMINDER_STAGES: &[(&str, i32)] = &[
    ("2_months", 60),
    ("1_month", 30),
    ("2_weeks", 14),
    ("1_week", 7),
    ("1_day", 1),
];

/// Evaluates the 5-stage subscription renewal reminder schedule (`2_months`, `1_month`, `2_weeks`, `1_week`, `1_day`)
/// across `users` and reconciles with `existing_logs`.
///
/// **Lifecycle Rules**:
/// - Already dispatched reminders (`status == "sent"`) always remain `"sent"` so no stage is ever emailed twice.
/// - Newly due reminders are created with `status = "scheduled"` so the background worker dispatches the email
///   and subsequently marks them `"sent"` via `db.mark_renewal_reminder_sent(...)`.
/// - If `user.reminders_halted` is `true` (subscriber renewed early), any unsent/due reminder is marked `halted_by_renewal`.
pub fn evaluate_5_stage_reminders(
    users: &[UserAccount],
    existing_logs: &[RenewalReminderLog],
) -> Vec<RenewalReminderLog> {
    let today = Utc::now().date_naive();
    let now_str = Utc::now().format("%Y-%m-%d %H:%M").to_string();
    let mut results: Vec<RenewalReminderLog> = Vec::new();
    let mut next_synthetic_id: i64 = existing_logs.iter().map(|l| l.id).max().unwrap_or(100) + 1;

    // 1. Reconcile existing logs with user's `reminders_halted` state (never overwrite "sent")
    for log in existing_logs {
        let mut updated = log.clone();
        if let Some(user) = users.iter().find(|u| u.id == log.user_id) {
            if user.reminders_halted && updated.status != "sent" {
                updated.status = "halted_by_renewal".to_string();
            }
        }
        results.push(updated);
    }

    // 2. Evaluate all 5 stages for users with an expiration date
    for user in users {
        let exp_str = match &user.subscription_expires_at {
            Some(s) if !s.trim().is_empty() => s.trim(),
            _ => continue,
        };

        let Some(exp_date) = parse_flexible_date(exp_str) else {
            continue;
        };

        let days_remaining = (exp_date - today).num_days() as i32;

        for &(stage_label, threshold_days) in REMINDER_STAGES {
            if let Some(existing) = results
                .iter_mut()
                .find(|r| r.user_id == user.id && r.stage_label == stage_label)
            {
                if existing.status != "sent" {
                    existing.days_remaining = days_remaining.max(0);
                    if user.reminders_halted {
                        existing.status = "halted_by_renewal".to_string();
                    }
                }
                continue;
            }

            let within_stage_window = match stage_label {
                "2_months" => (31..=60).contains(&days_remaining),
                "1_month" => (15..=30).contains(&days_remaining),
                "2_weeks" => (8..=14).contains(&days_remaining),
                "1_week" => (2..=7).contains(&days_remaining),
                "1_day" => (0..=1).contains(&days_remaining),
                _ => false,
            };

            if within_stage_window || (user.reminders_halted && (0..=threshold_days).contains(&days_remaining)) {
                let status = if user.reminders_halted {
                    "halted_by_renewal".to_string()
                } else {
                    // Always schedule due reminders so the worker sends the email and then marks "sent"
                    "scheduled".to_string()
                };

                results.push(RenewalReminderLog {
                    id: next_synthetic_id,
                    user_id: user.id,
                    user_email: user.email.clone(),
                    full_name: user.full_name.clone(),
                    organization_name: user.organization_name.clone(),
                    stage_label: stage_label.to_string(),
                    days_remaining: days_remaining.max(0),
                    status,
                    sent_at: now_str.clone(),
                });
                next_synthetic_id += 1;
            }
        }
    }

    results.sort_by(|a, b| a.days_remaining.cmp(&b.days_remaining).then(a.id.cmp(&b.id)));
    results
}

/// Creates a verified `OrderInvoice` record with an official invoice number (`MSI-YYYY-XXXXXX`)
/// and a cryptographic DRM watermark verification code (`pdf_watermark_code`).
#[allow(clippy::too_many_arguments)]
pub fn create_invoice_record(
    user_email: &str,
    full_name: &str,
    organization_name: &str,
    item_type: &str,
    item_code: &str,
    item_title: &str,
    amount_try: f64,
    secret: &str,
) -> OrderInvoice {
    let now = Utc::now();
    let created_at = now.format("%Y-%m-%d %H:%M").to_string();
    let year_str = now.format("%Y").to_string();

    let mut hasher = Sha256::new();
    hasher.update(b"MSI-INVOICE-2026:");
    hasher.update(user_email.trim().as_bytes());
    hasher.update(b"|");
    hasher.update(item_code.trim().as_bytes());
    hasher.update(b"|");
    hasher.update(now.timestamp_nanos_opt().unwrap_or(0).to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(secret.as_bytes());
    let digest = hex::encode(hasher.finalize()).to_uppercase();

    let numeric_suffix: u32 =
        u32::from_str_radix(&digest[..6], 16).unwrap_or(104829) % 900_000 + 100_000;
    let invoice_no = format!("MSI-{}-{:06}", year_str, numeric_suffix);
    let pdf_watermark_code = format!("DRM-MSI-{}-{}", year_str, &digest[6..20]);

    OrderInvoice {
        id: 0,
        invoice_no,
        user_email: user_email.trim().to_string(),
        full_name: full_name.trim().to_string(),
        organization_name: if organization_name.trim().is_empty() {
            "Bireysel / Kurumsal Okur".to_string()
        } else {
            organization_name.trim().to_string()
        },
        item_type: item_type.trim().to_string(),
        item_code: item_code.trim().to_string(),
        item_title: item_title.trim().to_string(),
        amount_try,
        payment_status: "pending_payment".to_string(),
        created_at,
        pdf_watermark_code,
    }
}

/// Activates or extends a user's subscription after successful payment confirmation.
/// - Updates `subscription_tier` based on the purchased item code prefix
/// - Sets/extends `subscription_expires_at` (adds 365 days from today or current expiry, whichever is later)
/// - Sets `reminders_halted = false` to re-enable renewal reminders
/// - Refreshes L1 cache for users section
pub fn activate_subscription_after_payment(
    db: &crate::db::Database,
    user_email: &str,
    item_code: &str,
) -> Result<(), String> {
    let conn = db
        .open_conn()
        .map_err(|e| format!("DB bağlantı hatası: {}", e))?;

    // Determine tier from item_code prefix
    let new_tier = if item_code.starts_with("SUB-CORP") {
        "Kurumsal Tam Yetki"
    } else if item_code.starts_with("SUB-PRO") {
        "Profesyonel Abone"
    } else if item_code.starts_with("SUB-STD") {
        "Standart Abone"
    } else if item_code.starts_with("SUB-PUB") {
        "Kamu E-Dergi Abonesi (Ücretsiz)"
    } else {
        "Standart Abone"
    };

    // Calculate new expiry: max(today + 365 days, current_expiry + 365 days)
    let today = Utc::now().date_naive();
    let extension = chrono::Duration::days(365);

    let current_expiry: Option<String> = conn
        .query_row(
            "SELECT subscription_expires_at FROM users WHERE email = ?1",
            rusqlite::params![user_email.trim()],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    let new_expiry = if let Some(exp_str) = current_expiry {
        if let Some(exp_date) = parse_flexible_date(&exp_str) {
            let extended = exp_date + extension;
            if extended > today + extension {
                extended
            } else {
                today + extension
            }
        } else {
            today + extension
        }
    } else {
        today + extension
    };

    let new_expiry_str = new_expiry.format("%Y-%m-%d").to_string();

    conn.execute(
        "UPDATE users SET subscription_tier = ?1, subscription_expires_at = ?2, reminders_halted = 0 WHERE email = ?3",
        rusqlite::params![new_tier, &new_expiry_str, user_email.trim()],
    )
    .map_err(|e| format!("Abonelik güncelleme hatası: {}", e))?;

    drop(conn);

    // Refresh L1 cache so subsequent requests see updated subscription
    db.refresh_l1_sections(&["users"]);

    tracing::info!(
        "Subscription activated: user={} tier={} expires={}",
        user_email, new_tier, new_expiry_str
    );

    Ok(())
}

fn parse_flexible_date(s: &str) -> Option<NaiveDate> {
    let date_part = s.split(' ').next()?.split('T').next()?;
    NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn user_expiring_in(days: i64, halted: bool) -> UserAccount {
        let exp = (Utc::now().date_naive() + Duration::days(days))
            .format("%Y-%m-%d")
            .to_string();
        UserAccount {
            id: 77,
            email: "abone@aselsan.com.tr".to_string(),
            full_name: "Abone Test".to_string(),
            role: "subscriber_corporate".to_string(),
            organization_category: "Sektör Kuruluşları".to_string(),
            organization_name: "ASELSAN".to_string(),
            job_title: "Uzman".to_string(),
            is_gov_verified: false,
            subscription_tier: "Kurumsal Abone".to_string(),
            subscription_expires_at: Some(exp),
            reminders_halted: halted,
            kvkk_consent_at: "2026-01-01".to_string(),
            kvkk_consent_ip: "127.0.0.1".to_string(),
            kvkk_consent_hash: "KVKK".to_string(),
            email_consent: true,
            ssb_interests: vec![],
            created_at: "2026-01-01".to_string(),
            email_verified: true,
            marketing_emails_opted_in: true,
            totp_enabled: false,
        }
    }

    #[test]
    fn due_reminder_is_created_as_scheduled_not_sent() {
        let user = user_expiring_in(25, false);
        let evaluated = evaluate_5_stage_reminders(&[user], &[]);
        assert_eq!(evaluated.len(), 1);
        assert_eq!(evaluated[0].stage_label, "1_month");
        assert_eq!(evaluated[0].status, "scheduled");
    }

    #[test]
    fn already_sent_reminder_remains_sent_even_after_renewal_halt() {
        let user = user_expiring_in(25, true);
        let sent_log = RenewalReminderLog {
            id: 10,
            user_id: 77,
            user_email: user.email.clone(),
            full_name: user.full_name.clone(),
            organization_name: user.organization_name.clone(),
            stage_label: "2_months".to_string(),
            days_remaining: 55,
            status: "sent".to_string(),
            sent_at: "2026-10-01 09:00".to_string(),
        };
        let evaluated = evaluate_5_stage_reminders(&[user], &[sent_log]);
        let two_m = evaluated.iter().find(|r| r.stage_label == "2_months").unwrap();
        let one_m = evaluated.iter().find(|r| r.stage_label == "1_month").unwrap();
        assert_eq!(two_m.status, "sent");
        assert_eq!(one_m.status, "halted_by_renewal");
    }
}
