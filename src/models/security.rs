use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityAuditLog {
    pub id: i64,
    pub client_ip: String,
    pub request_path: String,
    pub event_type: String,
    pub threat_level: String,
    pub details: String,
    pub blocked: bool,
    pub created_at: String,
}