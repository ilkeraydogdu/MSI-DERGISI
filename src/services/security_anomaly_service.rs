//! Security Anomaly Detection Service (`src/services/security_anomaly_service.rs`)
//!
//! Evaluates real `security_audit_logs` (WAF + auth failure/lockout events) and
//! `monitoring_attack_events` (fail2ban) over a rolling time window and emits
//! deduplicated `target_audience = "admin"` alerts into `system_notifications`.

use std::collections::HashMap;

use chrono::{Duration, Utc};

use crate::db::Database;
use crate::models::SystemNotification;

#[derive(Clone, Debug)]
pub struct AnomalyThresholds {
    pub window_minutes: i64,
    pub cooldown_minutes: i64,
    pub waf_spike_threshold: i64,
    pub critical_waf_threshold: i64,
    pub auth_spike_threshold: i64,
    pub auth_ip_threshold: i64,
    pub attack_spike_threshold: i64,
}

impl Default for AnomalyThresholds {
    fn default() -> Self {
        Self {
            window_minutes: 15,
            cooldown_minutes: 30,
            waf_spike_threshold: 5,
            critical_waf_threshold: 1,
            auth_spike_threshold: 5,
            auth_ip_threshold: 3,
            attack_spike_threshold: 5,
        }
    }
}

pub fn normalize_ts_cmp(raw: &str) -> String {
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

fn has_recent_notification(
    existing: &[SystemNotification],
    notif_type: &str,
    cooldown_cutoff: &str,
) -> bool {
    existing.iter().any(|n| {
        n.notification_type == notif_type && normalize_ts_cmp(&n.created_at).as_str() >= cooldown_cutoff
    })
}

fn top_ip_summary(counts: &HashMap<String, i64>) -> String {
    counts
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(ip, c)| format!("{} ({})", ip, c))
        .unwrap_or_else(|| "bilinmeyen".to_string())
}

/// Evaluates security logs and emits deduplicated admin alerts. Returns number of emitted alerts.
pub fn evaluate_and_emit_anomalies(db: &Database) -> usize {
    evaluate_and_emit_anomalies_with_thresholds(db, &AnomalyThresholds::default())
}

pub fn evaluate_and_emit_anomalies_with_thresholds(
    db: &Database,
    thresholds: &AnomalyThresholds,
) -> usize {
    let now = Utc::now();
    let window_cutoff = (now - Duration::minutes(thresholds.window_minutes))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let cooldown_cutoff = (now - Duration::minutes(thresholds.cooldown_minutes))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let now_rfc3339 = now.to_rfc3339();

    let audit_logs = db.get_security_audit_logs();
    let attack_events = db.get_recent_attack_events(200);
    let existing_notifs = db.get_system_notifications(None);

    let mut waf_count: i64 = 0;
    let mut critical_waf_count: i64 = 0;
    let mut waf_ips: HashMap<String, i64> = HashMap::new();

    let mut auth_count: i64 = 0;
    let mut auth_lockout_count: i64 = 0;
    let mut auth_ips: HashMap<String, i64> = HashMap::new();

    for log in &audit_logs {
        if normalize_ts_cmp(&log.created_at) < window_cutoff {
            continue;
        }
        let ip = if log.client_ip.trim().is_empty() {
            "unknown".to_string()
        } else {
            log.client_ip.trim().to_string()
        };
        match log.event_type.as_str() {
            "SQL_INJECTION_ATTEMPT"
            | "XSS_VECTOR_DETECTED"
            | "PATH_TRAVERSAL_ATTEMPT"
            | "RATE_LIMIT_EXCEEDED" => {
                waf_count += 1;
                if log.threat_level.eq_ignore_ascii_case("CRITICAL")
                    || matches!(
                        log.event_type.as_str(),
                        "SQL_INJECTION_ATTEMPT" | "PATH_TRAVERSAL_ATTEMPT"
                    )
                {
                    critical_waf_count += 1;
                }
                *waf_ips.entry(ip).or_insert(0) += 1;
            }
            "AUTH_LOGIN_FAILED" | "AUTH_TOTP_FAILED" | "AUTH_RATE_LIMITED" => {
                auth_count += 1;
                *auth_ips.entry(ip).or_insert(0) += 1;
            }
            "AUTH_ACCOUNT_LOCKED" => {
                auth_count += 1;
                auth_lockout_count += 1;
                *auth_ips.entry(ip).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    let mut f2b_count: i64 = 0;
    let mut f2b_ips: HashMap<String, i64> = HashMap::new();
    for ev in &attack_events {
        if normalize_ts_cmp(&ev.detected_at) < window_cutoff {
            continue;
        }
        f2b_count += 1;
        let ip = if ev.source_ip.trim().is_empty() {
            "unknown".to_string()
        } else {
            ev.source_ip.trim().to_string()
        };
        *f2b_ips.entry(ip).or_insert(0) += 1;
    }

    let mut emitted = 0usize;
    let admin_seg = std::env::var("ADMIN_PATH_SECRET")
        .map(|v| crate::config::normalize_admin_path_secret(&v))
        .unwrap_or_else(|_| "podmin".to_string());
    let admin_security_url = format!("/{}#tab-8", admin_seg);

    // 1. WAF Anomaly
    if (waf_count >= thresholds.waf_spike_threshold
        || critical_waf_count >= thresholds.critical_waf_threshold)
        && !has_recent_notification(&existing_notifs, "security_anomaly_waf", &cooldown_cutoff)
    {
        let top_ip = top_ip_summary(&waf_ips);
        let notif = SystemNotification {
            id: 0,
            target_audience: "admin".to_string(),
            target_email: None,
            notification_type: "security_anomaly_waf".to_string(),
            severity: "critical".to_string(),
            title_tr: format!("Kritik WAF Güvenlik Anomalisi ({} olay)", waf_count),
            title_en: format!("Critical WAF Security Anomaly ({} events)", waf_count),
            message_tr: format!(
                "Son {} dakikada {} WAF engellemesi ({} kritik) tespit edildi. En yoğun kaynak IP: {}.",
                thresholds.window_minutes, waf_count, critical_waf_count, top_ip
            ),
            message_en: format!(
                "{} WAF blocks ({} critical) detected in the last {} minutes. Top source IP: {}.",
                waf_count, critical_waf_count, thresholds.window_minutes, top_ip
            ),
            action_url: Some(admin_security_url.clone()),
            is_read: false,
            is_active_banner: false,
            created_at: now_rfc3339.clone(),
        };
        if db.insert_notification(&notif).is_ok() {
            emitted += 1;
        }
    }

    // 2. Auth Brute-Force / Lockout Anomaly
    let max_auth_ip = auth_ips.values().copied().max().unwrap_or(0);
    if (auth_count >= thresholds.auth_spike_threshold
        || max_auth_ip >= thresholds.auth_ip_threshold
        || auth_lockout_count >= 1)
        && !has_recent_notification(&existing_notifs, "security_anomaly_auth", &cooldown_cutoff)
    {
        let top_ip = top_ip_summary(&auth_ips);
        let notif = SystemNotification {
            id: 0,
            target_audience: "admin".to_string(),
            target_email: None,
            notification_type: "security_anomaly_auth".to_string(),
            severity: "high".to_string(),
            title_tr: format!(
                "Kimlik Doğrulama Anomalisi ({} başarısız giriş / {} kilit)",
                auth_count, auth_lockout_count
            ),
            title_en: format!(
                "Authentication Anomaly ({} failed logins / {} lockouts)",
                auth_count, auth_lockout_count
            ),
            message_tr: format!(
                "Son {} dakikada {} hatalı kimlik doğrulama ve {} hesap kilitleme olayı gözlendi. En yoğun IP: {}.",
                thresholds.window_minutes, auth_count, auth_lockout_count, top_ip
            ),
            message_en: format!(
                "{} failed authentication attempts and {} account lockouts in the last {} minutes. Top IP: {}.",
                auth_count, auth_lockout_count, thresholds.window_minutes, top_ip
            ),
            action_url: Some(admin_security_url.clone()),
            is_read: false,
            is_active_banner: false,
            created_at: now_rfc3339.clone(),
        };
        if db.insert_notification(&notif).is_ok() {
            emitted += 1;
        }
    }

    // 3. Fail2Ban / Infrastructure Attack Spike
    if f2b_count >= thresholds.attack_spike_threshold
        && !has_recent_notification(&existing_notifs, "security_anomaly_f2b", &cooldown_cutoff)
    {
        let top_ip = top_ip_summary(&f2b_ips);
        let notif = SystemNotification {
            id: 0,
            target_audience: "admin".to_string(),
            target_email: None,
            notification_type: "security_anomaly_f2b".to_string(),
            severity: "high".to_string(),
            title_tr: format!("Ağ Saldırı Artışı ({} Fail2Ban olayı)", f2b_count),
            title_en: format!("Network Attack Spike ({} Fail2Ban events)", f2b_count),
            message_tr: format!(
                "Son {} dakikada {} Fail2Ban/ağ saldırı olayı kaydedildi. En yoğun kaynak IP: {}.",
                thresholds.window_minutes, f2b_count, top_ip
            ),
            message_en: format!(
                "{} Fail2Ban/network attack events recorded in the last {} minutes. Top source IP: {}.",
                f2b_count, thresholds.window_minutes, top_ip
            ),
            action_url: Some(admin_security_url),
            is_read: false,
            is_active_banner: false,
            created_at: now_rfc3339,
        };
        if db.insert_notification(&notif).is_ok() {
            emitted += 1;
        }
    }

    emitted
}
