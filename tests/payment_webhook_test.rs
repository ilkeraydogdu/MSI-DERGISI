//! Integration tests for Payment/Webhook handling: PayTR hash generation, callback verification.

#[test]
fn payment_paytr_hash_changes_with_merchant_salt() {
    use msi_dergisi_platform::services::payment_service::{generate_paytr_hash, PayTrConfig};

    let cfg_a = PayTrConfig {
        merchant_id: "100200".into(),
        merchant_key: "key_secret_123".into(),
        merchant_salt: "salt_alpha".into(),
        test_mode: true,
        api_url: "https://www.paytr.com/odeme/api/get-token".into(),
    };
    let mut cfg_b = cfg_a.clone();
    cfg_b.merchant_salt = "salt_beta".into();

    let h1 = generate_paytr_hash(
        &cfg_a, "127.0.0.1", "MSIMSI2026000001", "user@test.com",
        120000, "dGVzdA==", 0, 0, "TL", 1,
    );
    let h2 = generate_paytr_hash(
        &cfg_b, "127.0.0.1", "MSIMSI2026000001", "user@test.com",
        120000, "dGVzdA==", 0, 0, "TL", 1,
    );
    assert_ne!(h1, h2, "Changing merchant_salt must produce different HMAC tokens");
}

#[test]
fn payment_merchant_oid_generation_is_alphanumeric() {
    use msi_dergisi_platform::services::payment_service::generate_merchant_oid;

    let oid = generate_merchant_oid("MSI-2026-481920");
    assert!(oid.chars().all(|c| c.is_ascii_alphanumeric()));
    assert!(!oid.is_empty());
}

#[test]
fn payment_callback_verification_rejects_tampered_amount() {
    use msi_dergisi_platform::services::payment_service::{verify_callback_hash, PayTrConfig};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let cfg = PayTrConfig {
        merchant_id: "100200".into(),
        merchant_key: "key_secret_123".into(),
        merchant_salt: "salt_alpha".into(),
        test_mode: true,
        api_url: "https://www.paytr.com/odeme/api/get-token".into(),
    };
    let oid = msi_dergisi_platform::services::payment_service::generate_merchant_oid("MSI-2026-481920");

    let mut mac = Hmac::<Sha256>::new_from_slice(cfg.merchant_key.as_bytes()).unwrap();
    mac.update(format!("{}{}success120000", oid, cfg.merchant_salt).as_bytes());
    let valid_sig = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        mac.finalize().into_bytes(),
    );

    assert!(verify_callback_hash(&cfg, &oid, "success", "120000", &valid_sig));
    assert!(!verify_callback_hash(&cfg, &oid, "success", "999999", &valid_sig));
    assert!(!verify_callback_hash(&cfg, &oid, "failed", "120000", &valid_sig));
}

#[test]
fn payment_matches_merchant_oid_strips_non_alphanumeric() {
    use msi_dergisi_platform::services::payment_service::matches_merchant_oid;

    assert!(matches_merchant_oid("MSI-2026-481920", "MSIMSI2026481920"));
    assert!(matches_merchant_oid("MSI-2026-481920", "MSI-2026-481920"));
    assert!(!matches_merchant_oid("MSI-2026-481920", "WRONG_OID"));
}