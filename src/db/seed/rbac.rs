use rusqlite::params;

/// Seeds the canonical 17 RBAC roles into `user_roles`.
/// Super roles (`super_admin`, `admin`, `site_operator`) remain locked (`is_system = 1`).
/// Staff and subscriber roles are editable in the admin matrix and preserved across restarts.
pub fn seed_user_roles(db: &crate::db::Database) -> Result<(), String> {
    let roles: &[(&str, &str, &str, &str, i32)] = &[
        ("super_admin", "Süper Yönetici", "Super Admin", r#"["*"]"#, 1),
        (
            "admin",
            "Yönetici",
            "Administrator",
            r#"["contents:manage","categories:manage","media:manage","issues:manage","approvals:manage","ads:manage","commerce:manage","subscriptions:manage","invoices:manage","users:read","users:write","users:verify","users:manage","corporate_requests:manage","newsletters:manage","email_groups:manage","notifications:manage","redirects:manage","translations:manage","glossary:manage","locales:manage","analytics:read","revenue:read","audit:read","audit:write","settings:manage","monitoring:read","roles:manage","backup:manage"]"#,
            1,
        ),
        (
            "site_operator",
            "Site Operatörü",
            "Site Operator",
            r#"["contents:manage","categories:manage","media:manage","issues:manage","approvals:manage","ads:manage","commerce:manage","subscriptions:manage","invoices:read","invoices:write","users:read","users:verify","corporate_requests:manage","newsletters:manage","email_groups:manage","notifications:manage","redirects:manage","translations:manage","glossary:manage","analytics:read","revenue:read","audit:read","settings:read","settings:write","monitoring:read"]"#,
            1,
        ),
        (
            "editor_in_chief",
            "Genel Yayın Yönetmeni",
            "Editor-in-Chief",
            r#"["contents:manage","categories:manage","media:manage","issues:manage","approvals:manage","newsletters:write","email_groups:manage","notifications:write","redirects:write","translations:write","glossary:manage","analytics:read","settings:read"]"#,
            0,
        ),
        (
            "managing_editor",
            "Yazı İşleri Müdürü",
            "Managing Editor",
            r#"["contents:manage","categories:write","media:manage","issues:manage","approvals:manage","newsletters:write","email_groups:write","redirects:write","translations:write","glossary:write","settings:read"]"#,
            0,
        ),
        (
            "content_publisher",
            "İçerik Yayınlayıcı",
            "Content Publisher",
            r#"["contents:read","contents:write","contents:publish","contents:delete","categories:write","media:manage","issues:read","approvals:approve","redirects:write"]"#,
            0,
        ),
        (
            "content_editor",
            "İçerik Editörü",
            "Content Editor",
            r#"["contents:read","contents:write","categories:read","media:read","media:upload","issues:read"]"#,
            0,
        ),
        (
            "editor",
            "Editör",
            "Editor",
            r#"["contents:read","contents:write","categories:read","media:read","media:upload","issues:read"]"#,
            0,
        ),
        (
            "business_dev",
            "İş Geliştirme Uzmanı",
            "Business Development",
            r#"["ads:manage","commerce:manage","subscriptions:manage","invoices:write","analytics:read","revenue:read","media:upload"]"#,
            0,
        ),
        (
            "ads_manager",
            "Reklam Yöneticisi",
            "Ads Manager",
            r#"["ads:manage","analytics:read","media:read","media:upload"]"#,
            0,
        ),
        (
            "finance_viewer",
            "Finans Görüntüleyici",
            "Finance Viewer",
            r#"["analytics:read","revenue:read","invoices:read","subscriptions:read","commerce:read"]"#,
            0,
        ),
        (
            "support_agent",
            "Destek Temsilcisi",
            "Support Agent",
            r#"["users:read","users:verify","users:write","corporate_requests:read","corporate_requests:decide","subscriptions:read","invoices:read","notifications:read","audit:read"]"#,
            0,
        ),
        (
            "subscriber_public",
            "Kamu E-Dergi Abonesi",
            "Public Sector Subscriber",
            r#"["edergi:read","edergi:download","articles:premium"]"#,
            0,
        ),
        (
            "subscriber_corporate",
            "Kurumsal Abone",
            "Corporate Subscriber",
            r#"["edergi:read","edergi:download","articles:premium"]"#,
            0,
        ),
        (
            "subscriber_print_digital",
            "Basılı + Dijital Abone",
            "Print & Digital Subscriber",
            r#"["edergi:read","edergi:download","articles:premium"]"#,
            0,
        ),
        (
            "subscriber_digital",
            "Dijital Abone",
            "Digital Subscriber",
            r#"["edergi:read","edergi:download","articles:premium"]"#,
            0,
        ),
        (
            "registered",
            "Kayıtlı Okuyucu",
            "Registered Reader",
            r#"["articles:read"]"#,
            0,
        ),
    ];

    let conn = db
        .open_conn()
        .map_err(|e| format!("RBAC seed bağlantı hatası: {}", e))?;

    for (role_code, name_tr, name_en, permissions_json, is_system) in roles {
        if *is_system == 1 {
            conn.execute(
                "INSERT INTO user_roles (role_code, name_tr, name_en, permissions_json, is_system)
                 VALUES (?1, ?2, ?3, ?4, 1)
                 ON CONFLICT(role_code) DO UPDATE SET
                     name_tr = excluded.name_tr,
                     name_en = excluded.name_en,
                     permissions_json = excluded.permissions_json,
                     is_system = 1",
                params![role_code, name_tr, name_en, permissions_json],
            )
            .map_err(|e| format!("RBAC role '{}' eklenemedi: {}", role_code, e))?;
        } else {
            conn.execute(
                "INSERT OR IGNORE INTO user_roles (role_code, name_tr, name_en, permissions_json, is_system)
                 VALUES (?1, ?2, ?3, ?4, 0)",
                params![role_code, name_tr, name_en, permissions_json],
            )
            .map_err(|e| format!("RBAC role '{}' eklenemedi: {}", role_code, e))?;
        }
    }

    Ok(())
}
