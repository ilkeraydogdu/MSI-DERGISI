use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ─── System Stats Models ────────────────────────────────────────────────

/// Full 8-field `/proc/stat` CPU jiffies snapshot for exact per-field delta computation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrevCpuCounters {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl PrevCpuCounters {
    pub fn total(&self) -> u64 {
        self.user
            .saturating_add(self.nice)
            .saturating_add(self.system)
            .saturating_add(self.idle)
            .saturating_add(self.iowait)
            .saturating_add(self.irq)
            .saturating_add(self.softirq)
            .saturating_add(self.steal)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuStats {
    pub user_pct: f64,
    pub system_pct: f64,
    pub idle_pct: f64,
    pub iowait_pct: f64,
    pub steal_pct: f64,
    pub load_avg_1m: f64,
    pub load_avg_5m: f64,
    pub load_avg_15m: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_kb: u64,
    pub available_kb: u64,
    pub free_kb: u64,
    pub buffers_kb: u64,
    pub cached_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
    pub used_pct: f64,
    pub swap_used_pct: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiskStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub used_pct: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkInterfaceStats {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub cpu: CpuStats,
    pub memory: MemoryStats,
    pub disk: DiskStats,
    pub network: Vec<NetworkInterfaceStats>,
    pub net_rx_per_sec: u64,
    pub net_tx_per_sec: u64,
    pub timestamp: String,
}

// ─── Domain & Service Models ────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VhostInfo {
    pub domain: String,
    pub vhost_path: String,
    pub access_log_path: String,
    pub error_log_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainHealthSnapshot {
    pub domain: String,
    pub http_status: Option<u16>,
    pub response_ms: Option<u64>,
    pub ssl_expiry: Option<String>,
    pub ssl_days_left: Option<i64>,
    pub dns_resolves: bool,
    #[serde(default = "default_true_bool")]
    pub tls_valid: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_error: Option<String>,
    pub last_check: String,
}

fn default_true_bool() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub port: u16,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

// ─── Traffic Models ─────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NginxAccessEntry {
    pub ip: String,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub bytes: u64,
    pub user_agent: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainTraffic {
    pub domain: String,
    pub requests_per_min: u64,
    pub status_codes: HashMap<u16, u64>,
    pub top_ips: Vec<(String, u64)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrafficSummary {
    pub per_domain: Vec<DomainTraffic>,
    pub total_rpm: u64,
}

// ─── Attack Models ──────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fail2banEvent {
    pub jail: String,
    pub ip: String,
    pub action: String, // Ban, Found, Unban
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttackEvent {
    pub id: Option<i64>,
    pub source_ip: String,
    pub attack_type: String,
    pub severity: String,
    pub target_service: String,
    pub details: String,
    pub detected_at: String,
}

// ─── SEO Models ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeoCheckResult {
    pub domain: String,
    pub robots_txt: bool,
    pub sitemap_xml: bool,
    pub meta_description: bool,
    pub meta_title: bool,
    pub canonical: bool,
    pub issues: Vec<String>,
    pub checked_at: String,
}

// ─── Full Bundle ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonitoringBundle {
    pub system: Option<SystemSnapshot>,
    pub domains: Vec<DomainHealthSnapshot>,
    pub traffic: TrafficSummary,
    pub services: Vec<ServiceStatus>,
    pub attacks: Vec<AttackEvent>,
    pub seo: Vec<SeoCheckResult>,
    pub updated_at: String,
}

// ─── /proc Parsers ──────────────────────────────────────────────────────

