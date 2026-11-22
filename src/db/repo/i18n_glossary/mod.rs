#[allow(unused_imports)]
use super::*;

pub use crate::models::category::{
    EmailGroup, GovEmailDomain, InstitutionCategory, LocaleConfig, TranslationGlossaryTerm,
};


mod glossary;
mod institutions;
mod locales;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::api_response::msg_keys;
    use crate::engine::translation_engine::TranslationCache;
    use std::path::Path;

    fn temp_db_path(label: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("/tmp/msi_db_i18n_test_{}_{}_{}.db", label, std::process::id(), nanos)
    }

    fn cleanup_db(path: &str) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn migration_012_populates_i18n_glossary_b2b_and_translations_idempotently() {
        let path = temp_db_path("fresh_012");
        let db = Database::new(&path).expect("Database::new should succeed on fresh db");

        let locales = db.list_locales();
        assert_eq!(locales.len(), 6);
        assert_eq!(locales.iter().filter(|l| l.is_default).count(), 1);
        assert!(locales.iter().any(|l| l.code == "tr" && l.is_default && l.is_enabled));
        assert!(locales.iter().any(|l| l.code == "en" && !l.is_default && l.is_enabled));
        assert_eq!(db.list_enabled_locales().len(), 2);

        let glossary = db.list_glossary_terms(None, true);
        assert!(
            glossary.len() >= 200,
            "Expected >= 200 glossary terms, got {}",
            glossary.len()
        );
        let violations =
            db.check_glossary_violations("The Ministry of Defense in Turkey visited SSM.");
        assert!(violations.iter().any(|v| v["forbidden"] == "Ministry of Defense"));
        assert!(violations.iter().any(|v| v["forbidden"] == "Turkey"));
        assert!(violations.iter().any(|v| v["forbidden"] == "SSM"));

        let insts = db.list_institution_categories(true);
        assert_eq!(insts.len(), 14);
        assert!(db.resolve_institution_category("savunma_medya").is_some());
        assert!(db.resolve_institution_category("Sektör Kuruluşları").is_some());

        let pub_dom = db
            .classify_email_domain("subay@msb.gov.tr")
            .expect("msb.gov.tr should match");
        assert_eq!(pub_dom.domain_kind, "public_sector");
        assert_eq!(pub_dom.grants_role, "subscriber_public");
        assert_eq!(pub_dom.institution_code.as_deref(), Some("msb"));

        let edu_dom = db
            .classify_email_domain("ogrenci@metu.edu.tr")
            .expect("edu.tr should match");
        assert_eq!(edu_dom.domain_kind, "academic");

        let pers_dom = db
            .classify_email_domain("okur@gmail.com")
            .expect("gmail.com should match");
        assert_eq!(pers_dom.domain_kind, "personal");

        let egroups = db.list_email_groups(true);
        assert_eq!(egroups.len(), 8);
        assert!(egroups.iter().any(|g| g.code == "general"));
        assert!(egroups.iter().any(|g| g.code == "edergi-releases"));
        assert!(egroups.iter().any(|g| g.code == "exhibition-alerts"));

        let cache = TranslationCache::new();
        cache.load(db.load_all_translations());
        assert_ne!(
            cache.t("tr", msg_keys::TOO_MANY_REQUESTS),
            msg_keys::TOO_MANY_REQUESTS
        );
        let bundle = cache.bundle_json("en");
        assert_eq!(bundle["lang"], "en");
        let strings = bundle["strings"].as_object().unwrap();
        assert!(strings.contains_key("api.error.too_many_requests"));
        assert!(strings.contains_key("nav.edergi"));
        assert!(strings.contains_key("admin.glossary_title"));
        assert!(strings["seo.default_description"]
            .as_str()
            .unwrap()
            .contains("Defence"));
        assert!(strings["plugin.description_human"]
            .as_str()
            .unwrap()
            .contains("Türkiye's defence"));

        // Verify seed_default_translations does NOT skip when rows already exist
        db.delete_translation("nav.edergi").unwrap();
        db.seed_default_translations().unwrap();
        let reloaded = db.load_all_translations();
        assert!(reloaded.iter().any(|e| e.key == "nav.edergi"));

        cleanup_db(&path);
    }

    #[test]
    fn editorial_repo_fixes_n6_v1_t4_c1_c3_and_composite_issues() {
        let path = temp_db_path("editorial_fixes");
        let db = Database::new(&path).expect("Database::new should succeed");

        // N6: New article with colliding slug must auto-dedupe instead of overwriting
        let mut first = ContentItem {
            id: 0,
            slug: "ornek-savunma-haberi".into(),
            title_tr: "İlk Orijinal Başlık".into(),
            title_en: "First Original Title".into(),
            summary_tr: "İlk özet".into(),
            summary_en: "First summary".into(),
            body_tr: "<p>İlk gövde metni.</p>".into(),
            body_en: "<p>First body text.</p>".into(),
            content_type: "haberler".into(),
            source_group: "gunluk-haber".into(),
            published_at: "2026-11-18 10:00".into(),
            en_status: "approved".into(),
            ..Default::default()
        };
        let id1 = db.upsert_content(&first).unwrap();
        assert!(id1 > 0);

        let second = ContentItem {
            id: 0,
            slug: "ornek-savunma-haberi".into(),
            title_tr: "Çakışan İkinci Başlık".into(),
            body_tr: "<p>İkinci haber gövdesi.</p>".into(),
            content_type: "haberler".into(),
            source_group: "gunluk-haber".into(),
            published_at: "2026-11-18 11:00".into(),
            ..Default::default()
        };
        let id2 = db.upsert_content(&second).unwrap();
        assert_ne!(id1, id2);
        let kept = db.get_content_by_slug("ornek-savunma-haberi").unwrap();
        assert_eq!(kept.title_tr, "İlk Orijinal Başlık");
        let deduped = db.get_content_by_slug("ornek-savunma-haberi-2").unwrap();
        assert_eq!(deduped.title_tr, "Çakışan İkinci Başlık");

        // V1 + EN drift + T4: Updating id1 creates auto-snapshot v1 and downgrades en_status when TR changes
        first.id = id1;
        first.title_tr = "Güncellenen Türkçe Başlık".into();
        db.upsert_content(&first).unwrap();

        let versions = db.get_content_versions(id1);
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0]["title_tr"], "İlk Orijinal Başlık");
        assert_eq!(versions[0]["title_en"], "First Original Title");
        assert_eq!(versions[0]["en_status"], "approved");

        let edited = db.get_content_by_slug("ornek-savunma-haberi").unwrap();
        assert_eq!(edited.en_status, "auto_draft");

        // Now add a v2 snapshot with changed EN and roll back to v1 -> EN must symmetrically roll back too
        db.update_content_en_translation(
            id1,
            "Newer EN Title",
            "Newer EN summary",
            "<p>Newer EN body</p>",
            "approved",
            "editor@msidergisi.com",
        )
        .unwrap();
        db.restore_content_version(id1, 1, "chief@msidergisi.com")
            .unwrap();
        let restored = db.get_content_by_slug("ornek-savunma-haberi").unwrap();
        assert_eq!(restored.title_tr, "İlk Orijinal Başlık");
        assert_eq!(restored.title_en, "First Original Title");
        assert_eq!(restored.en_status, "approved");

        // C1-C3: Strict duplicate category slug & partial update preservation
        let cat_res = db.insert_category(&CategoryNode {
            slug: "haberler".into(),
            name_tr: "Çakışma".into(),
            ..Default::default()
        });
        assert!(cat_res.is_err(), "Duplicate category slug must return Err");

        let child_cat = db
            .get_category_by_slug("kunye")
            .expect("kunye child category should exist");
        assert_eq!(child_cat.parent_slug.as_deref(), Some("hakkimizda"));
        db.update_category(
            child_cat.id,
            &CategoryNode {
                slug: "kunye".into(),
                parent_slug: None,
                name_tr: "Künye Güncel".into(),
                name_en: String::new(),
                group_type: String::new(),
                description_tr: child_cat.description_tr.clone(),
                description_en: String::new(),
                icon_class: String::new(),
                sort_order: child_cat.sort_order,
                is_dynamic: true,
                ..Default::default()
            },
        )
        .unwrap();
        let updated_cat = db.get_category_by_slug("kunye").unwrap();
        assert_eq!(updated_cat.parent_slug.as_deref(), Some("hakkimizda"));
        assert_eq!(updated_cat.group_type, "kurumsal");
        assert!(!updated_cat.description_en.is_empty());

        // Composite magazine_issues PK: same issue_number for msi and msi_tdr coexist
        db.upsert_issue(&MagazineIssue {
            issue_number: 150,
            slug: "msi-dergisi-sayi-150-test".into(),
            title_tr: "MSI Dergisi 150".into(),
            title_en: "MSI Magazine 150".into(),
            publish_month_tr: "Ocak".into(),
            publish_month_en: "January".into(),
            year: 2018,
            publication_type: "msi".into(),
            ..Default::default()
        })
        .unwrap();
        db.upsert_issue(&MagazineIssue {
            issue_number: 150,
            slug: "msi-tdr-issue-150-test".into(),
            title_tr: "MSI TDR 150".into(),
            title_en: "MSI TDR 150".into(),
            publish_month_tr: "January".into(),
            publish_month_en: "January".into(),
            year: 2026,
            publication_type: "msi_tdr".into(),
            ..Default::default()
        })
        .unwrap();
        assert!(db.get_issue("msi", 150).is_some());
        assert!(db.get_issue("msi_tdr", 150).is_some());

        cleanup_db(&path);
    }

    #[test]
    fn migration_012_on_live_db_temp_copy_preserves_data_and_rbac() {
        let live_path = "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/storage/db/msi_platform.db";
        if !Path::new(live_path).exists() {
            return;
        }
        let tmp_path = temp_db_path("live_mig012");
        std::fs::copy(live_path, &tmp_path).unwrap();
        let wal = format!("{}-wal", live_path);
        let shm = format!("{}-shm", live_path);
        if Path::new(&wal).exists() {
            let _ = std::fs::copy(&wal, format!("{}-wal", tmp_path));
        }
        if Path::new(&shm).exists() {
            let _ = std::fs::copy(&shm, format!("{}-shm", tmp_path));
        }

        let db = Database::new(&tmp_path).expect("Migration 012 on temp copy of live DB");
        let issues = db.get_all_issues();
        assert_eq!(issues.len(), 22);
        assert!(issues.iter().all(|i| i.uploaded_pdf_file.is_some()));
        assert_eq!(
            issues
                .iter()
                .filter(|i| i.publication_type == "msi_tdr" && i.en_status == "approved")
                .count(),
            7
        );

        let contents = db.get_all_contents();
        assert_eq!(contents.len(), 1010);
        assert_eq!(
            contents.iter().filter(|c| c.en_status == "approved").count(),
            151
        );

        let super_perms = db.get_role_permissions("super_admin").unwrap();
        assert_eq!(super_perms, vec!["*".to_string()]);
        let admin_perms = db.get_role_permissions("admin").unwrap();
        assert!(!admin_perms.iter().any(|p| p == "*"));
        assert!(admin_perms.iter().any(|p| p == "roles:manage"));
        let op_perms = db.get_role_permissions("site_operator").unwrap();
        assert!(!op_perms.iter().any(|p| p == "*"));
        assert!(!op_perms.iter().any(|p| p == "roles:manage"));
        let support_perms = db.get_role_permissions("support_agent").unwrap();
        assert!(support_perms.iter().any(|p| p == "users:write"));
        assert!(support_perms.iter().any(|p| p == "invoices:read"));

        cleanup_db(&tmp_path);
    }
}
