//! PayTR Payment Gateway Integration for MSI Dergisi Platform
//!
//! Implements the official PayTR iFrame API v1 checkout & callback flow:
//! 1. Generate HMAC-SHA256 token over `merchant_id + user_ip + merchant_oid + email + payment_amount + user_basket + no_installment + max_installment + currency + test_mode + merchant_salt`
//! 2. POST to PayTR API to obtain `iframe_token`
//! 3. Return `iframe_token` to frontend for embedded 3D Secure checkout
//! 4. Verify callback HMAC-SHA256 hash (`merchant_oid + merchant_salt + status + total_amount`) on payment completion

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::{error, info};

use crate::config::AppConfig;

type HmacSha256 = Hmac<Sha256>;

/// Configuration for PayTR gateway (loaded from centralized `AppConfig`)
#[derive(Clone, Debug)]
pub struct PayTrConfig {
    pub merchant_id: String,
    pub merchant_key: String,
    pub merchant_salt: String,
    /// When true, use sandbox/test mode (`test_mode = 1`)
    pub test_mode: bool,
    /// PayTR API endpoint URL (centralized in AppConfig)
    pub api_url: String,
}

impl PayTrConfig {
    /// Build `PayTrConfig` directly from centralized `AppConfig`.
    pub fn from_app_config(cfg: &AppConfig) -> Self {
        Self {
            merchant_id: cfg.paytr_merchant_id.trim().to_string(),
            merchant_key: cfg.paytr_merchant_key.trim().to_string(),
            merchant_salt: cfg.paytr_merchant_salt.trim().to_string(),
            test_mode: cfg.paytr_test_mode,
            api_url: cfg.paytr_api_url.clone(),
        }
    }

    /// Check if PayTR credentials are configured
    pub fn is_configured(&self) -> bool {
        !self.merchant_id.is_empty()
            && !self.merchant_key.is_empty()
            && !self.merchant_salt.is_empty()
    }
}

/// Request payload sent to PayTR get-token API
#[derive(Debug, Serialize)]
struct PayTrTokenRequest {
    merchant_id: String,
    user_ip: String,
    merchant_oid: String,
    email: String,
    payment_amount: i64, // Amount in kuruş (cents)
    paytr_token: String,
    user_basket: String, // Base64-encoded JSON array of [name, price_str, quantity]
    debug_on: u8,
    no_installment: u8,
    max_installment: u8,
    user_name: String,
    user_address: String,
    user_phone: String,
    merchant_ok_url: String,
    merchant_fail_url: String,
    timeout_limit: u32,
    currency: String,
    test_mode: u8,
    lang: String,
}

/// Response from PayTR get-token API
#[derive(Debug, Deserialize)]
struct PayTrTokenResponse {
    status: String,
    token: Option<String>,
    reason: Option<String>,
}

/// Server-validated checkout request data
#[derive(Debug, Clone)]
pub struct PayTrCheckoutRequest {
    pub user_email: String,
    pub full_name: String,
    pub user_phone: String,
    pub user_address: String,
    pub user_ip: String,
    pub merchant_oid: String,
    pub amount_try: f64,
    pub item_title: String,
    pub item_code: String,
    pub currency: String,
    pub lang: String,
}

/// Result of a successful token generation
#[derive(Debug, Clone, Serialize)]
pub struct PayTrCheckoutSession {
    pub iframe_token: String,
    pub merchant_oid: String,
    pub amount_kurus: i64,
}

/// Generate official PayTR iFrame HMAC-SHA256 `paytr_token`:
/// `base64(HMAC-SHA256(merchant_key, merchant_id + user_ip + merchant_oid + email + payment_amount + user_basket + no_installment + max_installment + currency + test_mode + merchant_salt))`
pub fn generate_paytr_hash(
    config: &PayTrConfig,
    user_ip: &str,
    merchant_oid: &str,
    email: &str,
    payment_amount: i64,
    user_basket: &str,
    no_installment: u8,
    max_installment: u8,
    currency: &str,
    test_mode: u8,
) -> String {
    let hash_str = format!(
        "{}{}{}{}{}{}{}{}{}{}",
        config.merchant_id,
        user_ip,
        merchant_oid,
        email,
        payment_amount,
        user_basket,
        no_installment,
        max_installment,
        currency,
        test_mode
    );
    let token_payload = format!("{}{}", hash_str, config.merchant_salt);

    let mut mac =
        HmacSha256::new_from_slice(config.merchant_key.as_bytes()).expect("HMAC key error");
    mac.update(token_payload.as_bytes());
    let result = mac.finalize();
    base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        result.into_bytes(),
    )
}

/// Create a PayTR checkout session by obtaining an iframe token
pub async fn create_checkout_session(
    config: &PayTrConfig,
    req: &PayTrCheckoutRequest,
    ok_url: &str,
    fail_url: &str,
) -> Result<PayTrCheckoutSession, String> {
    if !config.is_configured() {
        return Err("PayTR sanal POS yapılandırılmamış.".to_string());
    }

    let amount_kurus = (req.amount_try * 100.0).round() as i64;
    let test_mode: u8 = if config.test_mode { 1 } else { 0 };
    let no_installment: u8 = 0;
    let max_installment: u8 = 0;

    // PayTR expects unit price formatted as decimal TRY string inside user_basket: [["Item Name", "1200.00", 1]]
    let price_str = format!("{:.2}", req.amount_try);
    let basket_json = serde_json::json!([[req.item_title, price_str, 1]]);
    let user_basket = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        basket_json.to_string().as_bytes(),
    );

    let paytr_token = generate_paytr_hash(
        config,
        &req.user_ip,
        &req.merchant_oid,
        &req.user_email,
        amount_kurus,
        &user_basket,
        no_installment,
        max_installment,
        &req.currency,
        test_mode,
    );

    let api_request = PayTrTokenRequest {
        merchant_id: config.merchant_id.clone(),
        user_ip: req.user_ip.clone(),
        merchant_oid: req.merchant_oid.clone(),
        email: req.user_email.clone(),
        payment_amount: amount_kurus,
        paytr_token,
        user_basket,
        debug_on: if config.test_mode { 1 } else { 0 },
        no_installment,
        max_installment,
        user_name: req.full_name.clone(),
        user_address: req.user_address.clone(),
        user_phone: req.user_phone.clone(),
        merchant_ok_url: ok_url.to_string(),
        merchant_fail_url: fail_url.to_string(),
        timeout_limit: 30,
        currency: req.currency.clone(),
        test_mode,
        lang: req.lang.clone(),
    };

    info!(
        "PayTR: Requesting iframe token for merchant_oid={} item_code={}",
        req.merchant_oid, req.item_code
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&config.api_url)
        .form(&api_request)
        .send()
        .await
        .map_err(|e| {
            error!("PayTR API request failed: {}", e);
            format!("Ödeme geçidi bağlantı hatası: {}", e)
        })?;

    let token_response: PayTrTokenResponse = response.json().await.map_err(|e| {
        error!("PayTR API response parse error: {}", e);
        format!("Ödeme geçidi yanıt hatası: {}", e)
    })?;

    if token_response.status == "success" {
        if let Some(token) = token_response.token {
            info!(
                "PayTR: Token obtained successfully for merchant_oid={}",
                req.merchant_oid
            );
            Ok(PayTrCheckoutSession {
                iframe_token: token,
                merchant_oid: req.merchant_oid.clone(),
                amount_kurus,
            })
        } else {
            Err("PayTR başarılı yanıt döndü ancak iframe token üretmedi.".to_string())
        }
    } else {
        let reason = token_response
            .reason
            .unwrap_or_else(|| "Bilinmeyen hata".to_string());
        error!("PayTR: Token request failed: {}", reason);
        Err(format!("PayTR hatası: {}", reason))
    }
}

/// Verify official PayTR callback hash to confirm payment authenticity:
/// `base64(HMAC-SHA256(merchant_key, merchant_oid + merchant_salt + status + total_amount))`
pub fn verify_callback_hash(
    config: &PayTrConfig,
    merchant_oid: &str,
    status: &str,
    total_amount: &str,
    received_hash: &str,
) -> bool {
    let hash_string = format!(
        "{}{}{}{}",
        merchant_oid, config.merchant_salt, status, total_amount
    );

    let mut mac =
        HmacSha256::new_from_slice(config.merchant_key.as_bytes()).expect("HMAC key error");
    mac.update(hash_string.as_bytes());
    let result = mac.finalize();
    let expected_hash = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        result.into_bytes(),
    );

    constant_time_eq(expected_hash.as_bytes(), received_hash.as_bytes())
}

// SECURITY FIX (H12): Constant-time comparison without length side channel.
// Always compares max(len_a, len_b) bytes to avoid leaking hash length via
// early return on mismatched lengths.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max_len = std::cmp::max(a.len(), b.len());
    let mut diff: u8 = if a.len() != b.len() { 1 } else { 0 };
    for i in 0..max_len {
        let va = if i < a.len() { a[i] } else { 0 };
        let vb = if i < b.len() { b[i] } else { 0 };
        diff |= va ^ vb;
    }
    diff == 0
}

/// Generate an alphanumeric PayTR `merchant_oid` (PayTR forbids `-` or special chars in `merchant_oid`!)
pub fn generate_merchant_oid(invoice_no: &str) -> String {
    let clean: String = invoice_no
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    format!("MSI{}", clean)
}

/// Extract canonical `MSI-YYYY-NNNNNN` invoice lookup key or match stripped alphanumeric `merchant_oid`
pub fn matches_merchant_oid(invoice_no: &str, merchant_oid: &str) -> bool {
    generate_merchant_oid(invoice_no).eq_ignore_ascii_case(merchant_oid.trim())
        || invoice_no.eq_ignore_ascii_case(merchant_oid.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paytr_hash_includes_merchant_salt_and_verifies_callback() {
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
            &cfg_a,
            "194.110.169.165",
            "MSIMSI2026123456",
            "okur@aselsan.com.tr",
            120000,
            "W1siRGlqaXRhbCIsIjEyMDAuMDAiLDFdXQ==",
            0,
            0,
            "TL",
            1,
        );
        let h2 = generate_paytr_hash(
            &cfg_b,
            "194.110.169.165",
            "MSIMSI2026123456",
            "okur@aselsan.com.tr",
            120000,
            "W1siRGlqaXRhbCIsIjEyMDAuMDAiLDFdXQ==",
            0,
            0,
            "TL",
            1,
        );
        assert_ne!(h1, h2, "Changing merchant_salt must change PayTR HMAC token");

        // Verify callback HMAC
        let oid = generate_merchant_oid("MSI-2026-481920");
        assert!(oid.chars().all(|c| c.is_ascii_alphanumeric()));

        let mut mac = HmacSha256::new_from_slice(cfg_a.merchant_key.as_bytes()).unwrap();
        mac.update(format!("{}{}success120000", oid, cfg_a.merchant_salt).as_bytes());
        let valid_sig = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            mac.finalize().into_bytes(),
        );
        assert!(verify_callback_hash(&cfg_a, &oid, "success", "120000", &valid_sig));
        assert!(!verify_callback_hash(&cfg_a, &oid, "success", "999999", &valid_sig));
    }
}
