use crate::models::SystemNotification;
use chrono::Utc;

pub struct CentralNotificationEngine;

impl CentralNotificationEngine {
    /// Filters only active top-bar banner notifications.
    pub fn filter_active_banners(notifications: &[SystemNotification]) -> Vec<SystemNotification> {
        notifications
            .iter()
            .filter(|n| n.is_active_banner)
            .cloned()
            .collect()
    }

    /// Filters notifications relevant to the current visitor/subscriber based on email and organization category.
    pub fn filter_user_notifications(
        notifications: &[SystemNotification],
        user_email: Option<&str>,
        org_category: Option<&str>,
    ) -> Vec<SystemNotification> {
        notifications
            .iter()
            .filter(|n| {
                // Direct email match
                if let (Some(target_mail), Some(u_mail)) = (n.target_email.as_deref(), user_email) {
                    if !target_mail.is_empty() && target_mail.eq_ignore_ascii_case(u_mail) {
                        return true;
                    }
                    if !target_mail.is_empty() && !target_mail.eq_ignore_ascii_case(u_mail) {
                        return false;
                    }
                }

                let aud = n.target_audience.trim().to_lowercase();
                if aud.is_empty() || aud == "all" || aud == "public" || aud == "global" {
                    return true;
                }

                if (aud == "authenticated" || aud == "subscribers" || aud == "registered")
                    && user_email.is_some()
                {
                    return true;
                }

                if let Some(org) = org_category {
                    if aud == org.trim().to_lowercase() {
                        return true;
                    }
                    if aud == "gov" || aud == "gov_verified" || aud == "kamu" {
                        let org_upper = org.to_uppercase();
                        if org_upper.contains("MSB")
                            || org_upper.contains("SSB")
                            || org_upper.contains("K.K.LIĞI")
                            || org_upper.contains("EGM")
                            || org_upper.contains("MİT")
                            || org_upper.contains("KAMU")
                        {
                            return true;
                        }
                    }
                }

                false
            })
            .cloned()
            .collect()
    }

    /// Constructs a new structured SystemNotification event ready for DB persistence or immediate dispatch.
    #[allow(clippy::too_many_arguments)]
    pub fn build_notification_event(
        target_audience: &str,
        target_email: Option<&str>,
        ntype: &str,
        severity: &str,
        title_tr: &str,
        title_en: &str,
        msg_tr: &str,
        msg_en: &str,
        action_url: Option<&str>,
        is_banner: bool,
    ) -> SystemNotification {
        SystemNotification {
            id: 0,
            target_audience: target_audience.to_string(),
            target_email: target_email.map(|s| s.to_string()),
            notification_type: ntype.to_string(),
            severity: severity.to_string(),
            title_tr: title_tr.to_string(),
            title_en: title_en.to_string(),
            message_tr: msg_tr.to_string(),
            message_en: msg_en.to_string(),
            action_url: action_url.map(|s| s.to_string()),
            is_read: false,
            is_active_banner: is_banner,
            created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

pub fn filter_active_banners(notifications: &[SystemNotification]) -> Vec<SystemNotification> {
    CentralNotificationEngine::filter_active_banners(notifications)
}

pub fn filter_user_notifications(
    notifications: &[SystemNotification],
    user_email: Option<&str>,
    org_category: Option<&str>,
) -> Vec<SystemNotification> {
    CentralNotificationEngine::filter_user_notifications(notifications, user_email, org_category)
}

#[allow(clippy::too_many_arguments)]
pub fn build_notification_event(
    target_audience: &str,
    target_email: Option<&str>,
    ntype: &str,
    severity: &str,
    title_tr: &str,
    title_en: &str,
    msg_tr: &str,
    msg_en: &str,
    action_url: Option<&str>,
    is_banner: bool,
) -> SystemNotification {
    CentralNotificationEngine::build_notification_event(
        target_audience,
        target_email,
        ntype,
        severity,
        title_tr,
        title_en,
        msg_tr,
        msg_en,
        action_url,
        is_banner,
    )
}
