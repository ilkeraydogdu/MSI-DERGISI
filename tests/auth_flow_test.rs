//! Integration tests for Auth flow: password hashing, session validation, access control.

mod common;

use common::*;

#[test]
fn auth_password_hashing_round_trip() {
    let secret = "integration-test-secret";
    let hash = msi_dergisi_platform::services::auth_service::hash_password("SecurePass2026!", secret);
    assert!(msi_dergisi_platform::services::auth_service::is_argon2_hash(&hash));
    assert!(msi_dergisi_platform::services::auth_service::verify_password(
        "SecurePass2026!",
        secret,
        &hash
    ));
    assert!(!msi_dergisi_platform::services::auth_service::verify_password(
        "WrongPassword!",
        secret,
        &hash
    ));
}

#[test]
fn auth_legacy_sha256_migration_path() {
    let secret = "integration-test-secret";
    let legacy_hash =
        msi_dergisi_platform::services::auth_service::legacy_sha256_password_hash("LegacyPass1!", secret);
    assert!(msi_dergisi_platform::services::auth_service::verify_password(
        "LegacyPass1!",
        secret,
        &legacy_hash
    ));
    assert!(!msi_dergisi_platform::services::auth_service::verify_password(
        "OtherPass!",
        secret,
        &legacy_hash
    ));
}

#[test]
fn auth_public_sector_email_detection() {
    assert!(msi_dergisi_platform::services::auth_service::is_public_sector_email(
        "pilot@hvkk.tsk.tr"
    ));
    assert!(msi_dergisi_platform::services::auth_service::is_public_sector_email(
        "memur@ssb.gov.tr"
    ));
    assert!(!msi_dergisi_platform::services::auth_service::is_public_sector_email(
        "user@gmail.com"
    ));
}

#[test]
fn auth_corporate_verification_logic() {
    // Personal webmail claiming public/defense institution requires operator verification
    assert!(msi_dergisi_platform::services::auth_service::requires_corporate_verification(
        "muhendis@gmail.com",
        "SSB"
    ));
    // Verified corporate domain or official .gov.tr email does not require manual webmail verification
    assert!(!msi_dergisi_platform::services::auth_service::requires_corporate_verification(
        "muhendis@aselsan.com.tr",
        "Sektör Kuruluşları"
    ));
    assert!(!msi_dergisi_platform::services::auth_service::requires_corporate_verification(
        "uzman@ssb.gov.tr",
        "SSB"
    ));
}

#[test]
fn auth_subscriber_access_levels() {
    use msi_dergisi_platform::services::content_service::user_has_subscriber_access;

    let admin = fixture_admin_user(1);
    assert!(user_has_subscriber_access(&admin));

    let gov = fixture_gov_user(2);
    assert!(user_has_subscriber_access(&gov));

    let subscriber = fixture_subscriber_user(3);
    assert!(user_has_subscriber_access(&subscriber));

    let free = fixture_free_user(4);
    assert!(!user_has_subscriber_access(&free));
}

#[test]
fn auth_paywall_visibility_enforcement() {
    use msi_dergisi_platform::services::content_service::apply_paywall_view;

    let open_article = fixture_content(0); // visibility_level 0
    let (body, locked, _) = apply_paywall_view(&open_article, None, &[], "tr");
    assert!(!locked, "Level 0 article should be open to all");
    assert!(body.contains("Test makalesi"));

    let gated_article = fixture_content(2); // visibility_level 2
    let (_, locked, reason) = apply_paywall_view(&gated_article, None, &[], "tr");
    assert!(locked, "Level 2 article should be locked for anonymous users");
    assert!(reason.contains("Abonelere Özel") || reason.contains("Subscriber"));

    // Subscriber can access level 2
    let sub = fixture_subscriber_user(10);
    let (_, locked, _) = apply_paywall_view(&gated_article, Some(&sub), &[], "tr");
    assert!(!locked, "Subscriber should have access to level 2 content");
}