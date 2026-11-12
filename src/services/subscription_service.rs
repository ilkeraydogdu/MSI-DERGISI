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
/// **Renewal Halt Rule**:
/// If `user.reminders_halted` is `true` (because the subscriber renewed their subscription early),
/// any pending or existing reminder for that user is immediately marked as `halted_by_renewal`
/// and no further reminder emails are dispatched.
pub fn evaluate_5_stage_reminders(
    users: &[UserAccount],
    existing_logs: &[RenewalReminderLog],
) -> Vec<RenewalReminderLog> {
    let today = Utc::now().date_naive();
    let now_str = Utc::now().format("%Y-%m-%d %H:%M").to_string();
    let mut results: Vec<RenewalReminderLog> = Vec::new();
    let mut next_synthetic_id: i64 = existing_logs.iter().map(|l| l.id).max().unwrap_or(100) + 1;

    // 1. Reconcile existing logs with user's `reminders_halted` state
    for log in existing_logs {
        let mut updated = log.clone();
        if let Some(user) = users.iter().find(|u| u.id == log.user_id) {
            if user.reminders_halted && updated.status != "sent" {
                updated.status = "halted_by_renewal".to_string();
            } else if user.reminders_halted && updated.status == "scheduled" {
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

        let exp_date = match parse_flexible_date(exp_str) {
            Some(d) => d,
            None => continue,
        };

        let days_remaining = (exp_date - today).num_days() as i32;

        // Determine the closest active stage for this user's remaining days
        for &(stage_label, threshold_days) in REMINDER_STAGES {
            // Check if this (user_id, stage_label) already exists in results
            if let Some(existing) = results
                .iter_mut()
                .find(|r| r.user_id == user.id && r.stage_label == stage_label)
            {
                if user.reminders_halted {
                    existing.status = "halted_by_renewal".to_string();
                }
                continue;
            }

            // Only generate stage entries that match the current window or are halted
            let within_stage_window = match stage_label {
                "2_months" => (31..=65).contains(&days_remaining),
                "1_month" => (15..=30).contains(&days_remaining),
                "2_weeks" => (8..=14).contains(&days_remaining),
                "1_week" => (2..=7).contains(&days_remaining),
                "1_day" => (0..=1).contains(&days_remaining),
                _ => false,
            };

            if within_stage_window || (user.reminders_halted && days_remaining <= threshold_days) {
                let status = if user.reminders_halted {
                    "halted_by_renewal".to_string()
                } else if days_remaining <= threshold_days {
                    "sent".to_string()
                } else {
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

/// Creates a verified `OrderInvoice` record with an official invoice number (`MSI-2026-XXXXXX`)
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

    // Generate deterministic 6-digit numerical suffix from digest bytes
    let numeric_suffix: u32 = u32::from_str_radix(&digest[..6], 16).unwrap_or(104829) % 900_000 + 100_000;
    let invoice_no = format!("MSI-2026-{:06}", numeric_suffix);
    let pdf_watermark_code = format!("DRM-MSI-2026-{}", &digest[6..20]);

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

fn parse_flexible_date(s: &str) -> Option<NaiveDate> {
    let date_part = s.split(' ').next()?.split('T').next()?;
    NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
}
