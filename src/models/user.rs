use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserAccount {
    pub id: i64,
    pub email: String,
    pub full_name: String,
    pub role: String,
    /// Ready list from customer spec:
    /// `MSB`, `SSB`, `K.K.K.lığı`, `Dz.K.K.lığı`, `Hv.K.K.lığı`, `J.G.K.lığı`, `EGM`, `S.G.K.lığı`, `MİT`, `Diğer Kamu Kurumları`, `Sektör Kuruluşları`, `Üniversite / Öğrenci`, `Yurt Dışı Paydaş`
    pub organization_category: String,
    pub organization_name: String,
    pub job_title: String,
    pub is_gov_verified: bool,
    pub subscription_tier: String,
    pub subscription_expires_at: Option<String>,
    pub reminders_halted: bool,
    pub kvkk_consent_at: String,
    pub kvkk_consent_ip: String,
    pub kvkk_consent_hash: String,
    pub email_consent: bool,
    pub ssb_interests: Vec<String>,
    pub created_at: String,
    #[serde(default)]
    pub email_verified: bool,
    #[serde(default = "default_true")]
    pub marketing_emails_opted_in: bool,
    #[serde(default)]
    pub totp_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorporateVerificationRequest {
    pub id: i64,
    pub user_id: i64,
    pub email: String,
    pub full_name: String,
    pub organization_category: String,
    pub organization_name: String,
    pub request_message: String,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub created_at: String,
}