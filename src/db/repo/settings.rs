#[allow(unused_imports)]
use super::*;

impl Database {
    // =========================================================================
    pub fn get_site_settings(&self) -> Vec<SiteSetting> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.site_settings.clone();
            }
        }
        self.load_site_settings_db()
    }

    pub(super) fn load_site_settings_db(&self) -> Vec<SiteSetting> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT key, value_tr, value_en, setting_group, updated_at
             FROM site_settings ORDER BY setting_group ASC, key ASC",
            [],
            |r| Ok(SiteSetting {
                key: r.get(0)?,
                value_tr: r.get(1)?,
                value_en: r.get(2)?,
                setting_group: r.get(3)?,
                updated_at: r.get(4)?,
            }),
        )
    }

    pub fn get_setting_map(&self, lang: &str) -> HashMap<String, String> {
        let is_en = lang.eq_ignore_ascii_case("en");
        let mut map = HashMap::new();
        for s in self.get_site_settings() {
            let val = if is_en && !s.value_en.is_empty() {
                s.value_en
            } else {
                s.value_tr
            };
            map.insert(s.key, val);
        }
        map
    }

    pub fn upsert_site_setting(&self, setting: &SiteSetting) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now_str = if setting.updated_at.is_empty() {
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            setting.updated_at.clone()
        };
        conn.execute(
            "INSERT OR REPLACE INTO site_settings (key, value_tr, value_en, setting_group, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                setting.key.trim(),
                setting.value_tr,
                if setting.value_en.is_empty() {
                    &setting.value_tr
                } else {
                    &setting.value_en
                },
                if setting.setting_group.is_empty() {
                    "general"
                } else {
                    &setting.setting_group
                },
                now_str
            ],
        )?;
        drop(conn);
        self.refresh_l1_sections(&["site_settings"]);
        Ok(())
    }

    // =========================================================================
    // CENTRAL NOTIFICATION ENGINE PERSISTENCE
    // =========================================================================
    pub fn get_system_notifications(&self, target_email: Option<&str>) -> Vec<SystemNotification> {
        let all = if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                snap.notifications.clone()
            } else {
                self.load_notifications_db()
            }
        } else {
            self.load_notifications_db()
        };

        if let Some(email) = target_email.filter(|e| !e.trim().is_empty()) {
            all.into_iter()
                .filter(|n| {
                    n.target_audience == "all"
                        || n.target_audience == "subscribers"
                        || n.target_email
                            .as_deref()
                            .map(|te| te.eq_ignore_ascii_case(email))
                            .unwrap_or(false)
                })
                .collect()
        } else {
            all
        }
    }

    pub(super) fn load_notifications_db(&self) -> Vec<SystemNotification> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT id, target_audience, target_email, notification_type, severity,
                    title_tr, title_en, message_tr, message_en, action_url,
                    is_read, is_active_banner, created_at
             FROM system_notifications ORDER BY id DESC LIMIT 100",
            [],
            |r| Ok(SystemNotification {
                id: r.get(0)?,
                target_audience: r.get(1)?,
                target_email: r.get(2)?,
                notification_type: r.get(3)?,
                severity: r.get(4)?,
                title_tr: r.get(5)?,
                title_en: r.get(6)?,
                message_tr: r.get(7)?,
                message_en: r.get(8)?,
                action_url: r.get(9)?,
                is_read: r.get::<_, i32>(10)? != 0,
                is_active_banner: r.get::<_, i32>(11)? != 0,
                created_at: r.get(12)?,
            }),
        )
    }

    pub fn insert_notification(&self, notif: &SystemNotification) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now_str = if notif.created_at.is_empty() {
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            notif.created_at.clone()
        };
        conn.execute(
            "INSERT INTO system_notifications (
                target_audience, target_email, notification_type, severity,
                title_tr, title_en, message_tr, message_en, action_url,
                is_read, is_active_banner, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                notif.target_audience,
                notif.target_email,
                notif.notification_type,
                notif.severity,
                notif.title_tr,
                if notif.title_en.is_empty() {
                    &notif.title_tr
                } else {
                    &notif.title_en
                },
                notif.message_tr,
                if notif.message_en.is_empty() {
                    &notif.message_tr
                } else {
                    &notif.message_en
                },
                notif.action_url,
                if notif.is_read { 1 } else { 0 },
                if notif.is_active_banner { 1 } else { 0 },
                now_str
            ],
        )?;
        let rowid = conn.last_insert_rowid();
        drop(conn);
        self.refresh_l1_sections(&["notifications"]);
        Ok(rowid)
    }

    pub fn get_notification_by_id(&self, id: i64) -> Option<SystemNotification> {
        crate::db::query_helpers::query_one(
            self,
            "SELECT id, target_audience, target_email, notification_type, severity,
                    title_tr, title_en, message_tr, message_en, action_url,
                    is_read, is_active_banner, created_at
             FROM system_notifications WHERE id = ?1",
            params![id],
            |r| Ok(SystemNotification {
                id: r.get(0)?,
                target_audience: r.get(1)?,
                target_email: r.get(2)?,
                notification_type: r.get(3)?,
                severity: r.get(4)?,
                title_tr: r.get(5)?,
                title_en: r.get(6)?,
                message_tr: r.get(7)?,
                message_en: r.get(8)?,
                action_url: r.get(9)?,
                is_read: r.get::<_, i32>(10)? != 0,
                is_active_banner: r.get::<_, i32>(11)? != 0,
                created_at: r.get(12)?,
            }),
        )
    }

    pub fn mark_notification_read(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "UPDATE system_notifications SET is_read = 1 WHERE id = ?1",
            params![id],
        )?;
        drop(conn);
        self.refresh_l1_sections(&["notifications"]);
        Ok(())
    }

    // =========================================================================
    // CENTRAL WAF & SECURITY AUDIT LOGS
}
