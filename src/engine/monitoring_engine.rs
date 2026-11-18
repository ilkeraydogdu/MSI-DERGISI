//! Monitoring Engine (`src/engine/monitoring_engine.rs`)
//!
//! Pure functions for reading `/proc`, Nginx & Fail2ban logs, TCP ports, Plesk vhosts,
//! and X.509 PEM/DER SSL certificates without spawning external binaries.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use base64::Engine as _;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

// ─── Compiled Static Regexes (zero per-call recompilation) ──────────────

static NGINX_ACCESS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^(\S+)\s+\S+\s+\S+\s+\[([^\]]+)\]\s+"(\S+)\s+(\S+)\s+[^"]*"\s+(\d{3})\s+(\d+)\s+"[^"]*"\s+"([^"]*)""#,
    )
    .expect("valid static nginx access log regex")
});

static FAIL2BAN_BAN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}).*?\[([^\]]+)\]\s+(Ban|Found|Unban)\s+(\S+)"#,
    )
    .expect("valid static fail2ban regex")
});

static LEGACY_GMT_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}\s+\d{4})\s+GMT")
        .expect("valid static GMT date regex")
});

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

fn parse_proc_stat_line(line: &str) -> PrevCpuCounters {
    let parts: Vec<&str> = line.split_whitespace().collect();
    PrevCpuCounters {
        user: parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0),
        nice: parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0),
        system: parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
        idle: parts.get(4).and_then(|v| v.parse().ok()).unwrap_or(0),
        iowait: parts.get(5).and_then(|v| v.parse().ok()).unwrap_or(0),
        irq: parts.get(6).and_then(|v| v.parse().ok()).unwrap_or(0),
        softirq: parts.get(7).and_then(|v| v.parse().ok()).unwrap_or(0),
        steal: parts.get(8).and_then(|v| v.parse().ok()).unwrap_or(0),
    }
}

pub fn compute_cpu_deltas(
    current: PrevCpuCounters,
    prev: Option<PrevCpuCounters>,
    load_avg: [f64; 3],
) -> CpuStats {
    let (user_pct, system_pct, idle_pct, iowait_pct, steal_pct) = if let Some(p) = prev {
        let total_delta = current.total().saturating_sub(p.total());
        if total_delta == 0 {
            (0.0, 0.0, 100.0, 0.0, 0.0)
        } else {
            let td = total_delta as f64;
            let user_delta = current
                .user
                .saturating_add(current.nice)
                .saturating_sub(p.user.saturating_add(p.nice));
            let sys_delta = current
                .system
                .saturating_add(current.irq)
                .saturating_add(current.softirq)
                .saturating_sub(p.system.saturating_add(p.irq).saturating_add(p.softirq));
            let idle_delta = current.idle.saturating_sub(p.idle);
            let iowait_delta = current.iowait.saturating_sub(p.iowait);
            let steal_delta = current.steal.saturating_sub(p.steal);

            (
                (user_delta as f64 / td) * 100.0,
                (sys_delta as f64 / td) * 100.0,
                (idle_delta as f64 / td) * 100.0,
                (iowait_delta as f64 / td) * 100.0,
                (steal_delta as f64 / td) * 100.0,
            )
        }
    } else {
        (0.0, 0.0, 100.0, 0.0, 0.0)
    };

    let round2 = |v: f64| (v * 100.0).round() / 100.0;

    CpuStats {
        user_pct: round2(user_pct),
        system_pct: round2(system_pct),
        idle_pct: round2(idle_pct),
        iowait_pct: round2(iowait_pct),
        steal_pct: round2(steal_pct),
        load_avg_1m: load_avg[0],
        load_avg_5m: load_avg[1],
        load_avg_15m: load_avg[2],
    }
}

/// Reads real per-field CPU counters from `/proc/stat` and computes exact deltas against `prev`.
pub fn read_cpu_stats(prev: Option<PrevCpuCounters>) -> (CpuStats, PrevCpuCounters) {
    let content = fs::read_to_string("/proc/stat").unwrap_or_default();
    let line = content
        .lines()
        .find(|l| l.starts_with("cpu "))
        .unwrap_or("");
    let current = parse_proc_stat_line(line);
    let load_avg = read_load_avg();
    let stats = compute_cpu_deltas(current, prev, load_avg);
    (stats, current)
}

/// Read load average from `/proc/loadavg`.
pub fn read_load_avg() -> [f64; 3] {
    let content = fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let parts: Vec<&str> = content.split_whitespace().collect();
    [
        parts.first().and_then(|v| v.parse().ok()).unwrap_or(0.0),
        parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0),
        parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0),
    ]
}

/// Read memory stats from `/proc/meminfo`.
pub fn read_memory_stats() -> MemoryStats {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut map: HashMap<&str, u64> = HashMap::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(val) = parts[1].parse::<u64>() {
                map.insert(parts[0].trim_end_matches(':'), val);
            }
        }
    }

    let total = map.get("MemTotal").copied().unwrap_or(0);
    let available = map.get("MemAvailable").copied().unwrap_or(0);
    let free = map.get("MemFree").copied().unwrap_or(0);
    let buffers = map.get("Buffers").copied().unwrap_or(0);
    let cached = map.get("Cached").copied().unwrap_or(0);
    let swap_total = map.get("SwapTotal").copied().unwrap_or(0);
    let swap_free = map.get("SwapFree").copied().unwrap_or(0);

    let used = total.saturating_sub(available);
    let used_pct = if total > 0 {
        (used as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let swap_used_pct = if swap_total > 0 {
        (swap_total.saturating_sub(swap_free) as f64 / swap_total as f64) * 100.0
    } else {
        0.0
    };

    MemoryStats {
        total_kb: total,
        available_kb: available,
        free_kb: free,
        buffers_kb: buffers,
        cached_kb: cached,
        swap_total_kb: swap_total,
        swap_free_kb: swap_free,
        used_pct: (used_pct * 100.0).round() / 100.0,
        swap_used_pct: (swap_used_pct * 100.0).round() / 100.0,
    }
}

/// Read disk stats using `libc::statvfs` for the root filesystem.
pub fn read_disk_stats() -> DiskStats {
    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;
        let mut buf = MaybeUninit::<libc::statvfs>::uninit();
        let path = b"/\0";
        let result =
            unsafe { libc::statvfs(path.as_ptr() as *const libc::c_char, buf.as_mut_ptr()) };
        if result == 0 {
            let s = unsafe { buf.assume_init() };
            let block_size = s.f_frsize as u64;
            let total = s.f_blocks as u64 * block_size;
            let free = s.f_bavail as u64 * block_size;
            let used = total.saturating_sub(free);
            let used_pct = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            return DiskStats {
                total_bytes: total,
                used_bytes: used,
                free_bytes: free,
                used_pct: (used_pct * 100.0).round() / 100.0,
            };
        }
    }
    DiskStats {
        total_bytes: 0,
        used_bytes: 0,
        free_bytes: 0,
        used_pct: 0.0,
    }
}

/// Read network interface stats from `/proc/net/dev`.
pub fn read_network_stats() -> Vec<NetworkInterfaceStats> {
    let content = fs::read_to_string("/proc/net/dev").unwrap_or_default();
    let mut result = Vec::new();

    for line in content.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 12 {
            continue;
        }
        let name = parts[0].trim_end_matches(':').to_string();
        if name == "lo" {
            continue;
        }
        result.push(NetworkInterfaceStats {
            name,
            rx_bytes: parts[1].parse().unwrap_or(0),
            rx_packets: parts[2].parse().unwrap_or(0),
            rx_errors: parts[3].parse().unwrap_or(0),
            tx_bytes: parts[9].parse().unwrap_or(0),
            tx_packets: parts[10].parse().unwrap_or(0),
            tx_errors: parts[11].parse().unwrap_or(0),
        });
    }
    result
}

// ─── Log Parsers ────────────────────────────────────────────────────────

/// Parses an Nginx combined log timestamp (e.g. `17/Nov/2026:19:30:14 +0300`) into `DateTime<Utc>`.
pub fn parse_nginx_timestamp(ts: &str) -> Option<DateTime<Utc>> {
    let trimmed = ts.trim();
    if let Ok(dt) = DateTime::parse_from_str(trimmed, "%d/%b/%Y:%H:%M:%S %z") {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }
    None
}

/// Parse last `max_lines` lines of an Nginx/Plesk access log file.
pub fn parse_nginx_access_log_tail(path: &str, max_lines: usize) -> Vec<NginxAccessEntry> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let metadata = match file.metadata() {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    let file_size = metadata.len();
    if file_size == 0 {
        return Vec::new();
    }

    let read_size = std::cmp::min(file_size, (max_lines as u64) * 512);
    let mut file = file;
    let _ = file.seek(SeekFrom::End(-(read_size as i64)));

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let start = lines.len().saturating_sub(max_lines);
    let mut entries = Vec::with_capacity(lines.len() - start);

    for line in &lines[start..] {
        if let Some(caps) = NGINX_ACCESS_RE.captures(line) {
            entries.push(NginxAccessEntry {
                ip: caps[1].to_string(),
                timestamp: caps[2].to_string(),
                method: caps[3].to_string(),
                path: caps[4].to_string(),
                status: caps[5].parse().unwrap_or(0),
                bytes: caps[6].parse().unwrap_or(0),
                user_agent: caps[7].to_string(),
            });
        }
    }
    entries
}

/// Parse fail2ban log tail for `Ban` / `Found` / `Unban` events.
pub fn parse_fail2ban_log_tail(path: &str, max_lines: usize) -> Vec<Fail2banEvent> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let metadata = match file.metadata() {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    let file_size = metadata.len();
    if file_size == 0 {
        return Vec::new();
    }

    let read_size = std::cmp::min(file_size, (max_lines as u64) * 300);
    let mut file = file;
    let _ = file.seek(SeekFrom::End(-(read_size as i64)));

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let start = lines.len().saturating_sub(max_lines);
    let mut events = Vec::new();

    for line in &lines[start..] {
        if let Some(caps) = FAIL2BAN_BAN_RE.captures(line) {
            events.push(Fail2banEvent {
                jail: caps[2].to_string(),
                ip: caps[4].to_string(),
                action: caps[3].to_string(),
                timestamp: caps[1].to_string(),
            });
        }
    }
    events
}

// ─── Vhost Discovery ────────────────────────────────────────────────────

fn pick_best_log_path(candidates: &[String]) -> String {
    // Prefer an existing non-empty log file first (e.g. Plesk's active proxy_access_ssl_log)
    for c in candidates {
        if let Ok(meta) = fs::metadata(c) {
            if meta.is_file() && meta.len() > 0 {
                return c.clone();
            }
        }
    }
    for c in candidates {
        if Path::new(c).is_file() {
            return c.clone();
        }
    }
    String::new()
}

/// Discover all active domain vhosts under `/var/www/vhosts/` (including `.plesk.page` hosts).
/// Only skips Plesk system directories: `system`, `chroot`, `default`, `fs`, `fs-passwd`, `.skel`, and dotfiles.
pub fn discover_vhosts(base_path: &str, nginx_log_path: &str) -> Vec<VhostInfo> {
    let mut vhosts = Vec::new();
    let dir = match fs::read_dir(base_path) {
        Ok(d) => d,
        Err(_) => return vhosts,
    };

    let nginx_dir = {
        let p = Path::new(nginx_log_path);
        if p.is_dir() {
            nginx_log_path.trim_end_matches('/').to_string()
        } else {
            p.parent()
                .and_then(|parent| parent.to_str())
                .unwrap_or("/var/log/nginx")
                .trim_end_matches('/')
                .to_string()
        }
    };

    for entry in dir.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Skip Plesk internal/system directories ONLY (never skip .plesk.page!)
        if name.starts_with('.')
            || matches!(
                name.as_str(),
                "system" | "chroot" | "default" | "fs" | "fs-passwd" | ".skel"
            )
        {
            continue;
        }

        if !name.contains('.') {
            continue;
        }

        let domain = name.clone();
        let vhost_path = path.to_string_lossy().to_string();

        let access_candidates = vec![
            format!("{}/logs/proxy_access_ssl_log", vhost_path),
            format!("{}/logs/access_ssl_log", vhost_path),
            format!("{}/logs/proxy_access_log", vhost_path),
            format!("{}/logs/access_log", vhost_path),
            format!("{}/{}.access.log", nginx_dir, domain),
            format!("/var/www/vhosts/system/{}/logs/proxy_access_ssl_log", domain),
        ];

        let error_candidates = vec![
            format!("{}/logs/proxy_error_log", vhost_path),
            format!("{}/logs/error_log", vhost_path),
            format!("{}/{}.error.log", nginx_dir, domain),
            format!("/var/www/vhosts/system/{}/logs/proxy_error_log", domain),
        ];

        vhosts.push(VhostInfo {
            domain,
            vhost_path,
            access_log_path: pick_best_log_path(&access_candidates),
            error_log_path: pick_best_log_path(&error_candidates),
        });
    }

    vhosts.sort_by(|a, b| a.domain.cmp(&b.domain));
    vhosts
}

// ─── Async Port & Domain Health Checkers ────────────────────────────────

/// Check if a TCP port is reachable within `timeout_ms`.
pub async fn check_tcp_port(host: &str, port: u16, timeout_ms: u64) -> ServiceStatus {
    let addr_str = format!("{}:{}", host, port);
    let start = std::time::Instant::now();

    match tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        tokio::net::TcpStream::connect(&addr_str),
    )
    .await
    {
        Ok(Ok(_stream)) => ServiceStatus {
            name: format!("{}:{}", host, port),
            port,
            reachable: true,
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Ok(Err(e)) => ServiceStatus {
            name: format!("{}:{}", host, port),
            port,
            reachable: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
        Err(_) => ServiceStatus {
            name: format!("{}:{}", host, port),
            port,
            reachable: false,
            latency_ms: None,
            error: Some("Connection timed out".to_string()),
        },
    }
}

/// Checks domain health using a strict TLS-verifying `strict_client` first so expired,
/// self-signed, or hostname-mismatched certificates are accurately flagged (`tls_valid = false`).
/// Falls back to `diagnostic_client` ONLY to record HTTP status/latency when TLS fails.
pub async fn check_domain_health_verified(
    domain: &str,
    strict_client: &reqwest::Client,
    diagnostic_client: &reqwest::Client,
) -> DomainHealthSnapshot {
    let https_url = format!("https://{}", domain);
    let start = std::time::Instant::now();
    let now = Utc::now().to_rfc3339();

    match tokio::time::timeout(Duration::from_secs(5), strict_client.get(&https_url).send()).await {
        Ok(Ok(resp)) => DomainHealthSnapshot {
            domain: domain.to_string(),
            http_status: Some(resp.status().as_u16()),
            response_ms: Some(start.elapsed().as_millis() as u64),
            ssl_expiry: None,
            ssl_days_left: None,
            dns_resolves: true,
            tls_valid: true,
            tls_error: None,
            last_check: now,
        },
        Ok(Err(strict_err)) => {
            // Secondary diagnostic probe: distinguishes TLS certificate failure from DNS/TCP outage
            let diag_start = std::time::Instant::now();
            if let Ok(Ok(resp)) = tokio::time::timeout(
                Duration::from_secs(4),
                diagnostic_client.get(&https_url).send(),
            )
            .await
            {
                return DomainHealthSnapshot {
                    domain: domain.to_string(),
                    http_status: Some(resp.status().as_u16()),
                    response_ms: Some(diag_start.elapsed().as_millis() as u64),
                    ssl_expiry: None,
                    ssl_days_left: None,
                    dns_resolves: true,
                    tls_valid: false,
                    tls_error: Some(strict_err.to_string()),
                    last_check: now,
                };
            }

            // Plain HTTP fallback
            let http_url = format!("http://{}", domain);
            let http_start = std::time::Instant::now();
            if let Ok(Ok(resp)) =
                tokio::time::timeout(Duration::from_secs(3), strict_client.get(&http_url).send())
                    .await
            {
                DomainHealthSnapshot {
                    domain: domain.to_string(),
                    http_status: Some(resp.status().as_u16()),
                    response_ms: Some(http_start.elapsed().as_millis() as u64),
                    ssl_expiry: None,
                    ssl_days_left: None,
                    dns_resolves: true,
                    tls_valid: false,
                    tls_error: Some(format!("HTTPS/TLS başarısız: {}", strict_err)),
                    last_check: now,
                }
            } else {
                DomainHealthSnapshot {
                    domain: domain.to_string(),
                    http_status: None,
                    response_ms: None,
                    ssl_expiry: None,
                    ssl_days_left: None,
                    dns_resolves: false,
                    tls_valid: false,
                    tls_error: Some(strict_err.to_string()),
                    last_check: now,
                }
            }
        }
        Err(_) => DomainHealthSnapshot {
            domain: domain.to_string(),
            http_status: None,
            response_ms: None,
            ssl_expiry: None,
            ssl_days_left: None,
            dns_resolves: false,
            tls_valid: false,
            tls_error: Some("HTTPS bağlantı zaman aşımı".to_string()),
            last_check: now,
        },
    }
}

/// Single-client wrapper retained for compatibility; uses `client` for primary probe.
pub async fn check_domain_health(domain: &str, client: &reqwest::Client) -> DomainHealthSnapshot {
    check_domain_health_verified(domain, client, client).await
}

// ─── Pure-Rust X.509 PEM / ASN.1 DER Certificate Parser ─────────────────

/// Reads a single ASN.1 DER `(tag, value_slice, remaining_slice)` TLV element safely.
fn read_asn1_tlv(data: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let tag = *data.first()?;
    let len_byte = *data.get(1)?;
    let (len, header_len) = if len_byte & 0x80 == 0 {
        (len_byte as usize, 2usize)
    } else {
        let num_octets = (len_byte & 0x7F) as usize;
        if num_octets == 0 || num_octets > 4 || data.len() < 2 + num_octets {
            return None;
        }
        let mut l = 0usize;
        for &b in &data[2..2 + num_octets] {
            l = (l << 8) | (b as usize);
        }
        (l, 2 + num_octets)
    };
    let end = header_len.checked_add(len)?;
    if data.len() < end {
        return None;
    }
    Some((tag, &data[header_len..end], &data[end..]))
}

/// Parses an ASN.1 `UTCTime` (`0x17`) or `GeneralizedTime` (`0x18`) into `DateTime<Utc>`.
fn parse_asn1_time(tag: u8, bytes: &[u8]) -> Option<DateTime<Utc>> {
    let raw = std::str::from_utf8(bytes).ok()?.trim();
    match tag {
        // UTCTime: YYMMDDHHMMSSZ or YYMMDDHHMMZ (RFC 5280 §4.1.2.5.1: YY >= 50 => 19YY, else 20YY)
        0x17 => {
            let core = raw.strip_suffix('Z').unwrap_or(raw);
            if core.len() < 10 {
                return None;
            }
            let yy: i32 = core[0..2].parse().ok()?;
            let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
            let month: u32 = core[2..4].parse().ok()?;
            let day: u32 = core[4..6].parse().ok()?;
            let hour: u32 = core[6..8].parse().ok()?;
            let min: u32 = core[8..10].parse().ok()?;
            let sec: u32 = if core.len() >= 12 {
                core[10..12].parse().ok()?
            } else {
                0
            };
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_opt(hour, min, sec)
                .map(|ndt| ndt.and_utc())
        }
        // GeneralizedTime: YYYYMMDDHHMMSS[.fff]Z (RFC 5280 §4.1.2.5.2)
        0x18 => {
            let core = raw.strip_suffix('Z').unwrap_or(raw);
            let core = core.split('.').next().unwrap_or(core);
            if core.len() < 12 {
                return None;
            }
            let year: i32 = core[0..4].parse().ok()?;
            let month: u32 = core[4..6].parse().ok()?;
            let day: u32 = core[6..8].parse().ok()?;
            let hour: u32 = core[8..10].parse().ok()?;
            let min: u32 = core[10..12].parse().ok()?;
            let sec: u32 = if core.len() >= 14 {
                core[12..14].parse().ok()?
            } else {
                0
            };
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_opt(hour, min, sec)
                .map(|ndt| ndt.and_utc())
        }
        _ => None,
    }
}

/// Extracts DNS names (Subject CN `2.5.4.3` + SubjectAltName `dNSName` `[2]` `2.5.29.17`)
/// and `(notBefore, notAfter)` validity timestamps from a DER-encoded X.509 certificate.
pub fn parse_x509_der_validity_and_names(
    der: &[u8],
) -> Option<(DateTime<Utc>, DateTime<Utc>, Vec<String>)> {
    let (cert_tag, cert_body, _) = read_asn1_tlv(der)?;
    if cert_tag != 0x30 {
        return None;
    }
    let (tbs_tag, tbs_body, _) = read_asn1_tlv(cert_body)?;
    if tbs_tag != 0x30 {
        return None;
    }

    // Walk TBSCertificate fields: [0] version OPTIONAL, serialNumber, signature, issuer, validity, subject, spki, ..., [3] extensions
    let (first_tag, _, after_first) = read_asn1_tlv(tbs_body)?;
    let cursor = if first_tag == 0xA0 {
        let (_, _, after_serial) = read_asn1_tlv(after_first)?;
        after_serial
    } else {
        // v1 certificate without explicit [0] version; first_tag was serialNumber
        after_first
    };

    // signature AlgorithmIdentifier
    let (sig_tag, _, cursor) = read_asn1_tlv(cursor)?;
    if sig_tag != 0x30 {
        return None;
    }
    // issuer Name
    let (issuer_tag, _, cursor) = read_asn1_tlv(cursor)?;
    if issuer_tag != 0x30 {
        return None;
    }
    // validity SEQUENCE { notBefore, notAfter }
    let (val_tag, val_body, cursor) = read_asn1_tlv(cursor)?;
    if val_tag != 0x30 {
        return None;
    }
    let (nb_tag, nb_bytes, val_rem) = read_asn1_tlv(val_body)?;
    let (na_tag, na_bytes, _) = read_asn1_tlv(val_rem)?;
    let not_before = parse_asn1_time(nb_tag, nb_bytes)?;
    let not_after = parse_asn1_time(na_tag, na_bytes)?;

    // subject Name
    let (subj_tag, subj_body, mut tail) = read_asn1_tlv(cursor)?;
    let mut names = Vec::new();
    if subj_tag == 0x30 {
        extract_cn_from_name(subj_body, &mut names);
    }

    // Scan remaining TBSCertificate elements for [3] EXPLICIT Extensions (0xA3)
    while !tail.is_empty() {
        let Some((tag, body, next_tail)) = read_asn1_tlv(tail) else {
            break;
        };
        if tag == 0xA3 {
            extract_san_dns_names(body, &mut names);
        }
        tail = next_tail;
    }

    Some((not_before, not_after, names))
}

/// Extracts CommonName (`OID 2.5.4.3` = `[0x55, 0x04, 0x03]`) strings from an X.509 `Name` SEQUENCE.
fn extract_cn_from_name(mut name_seq: &[u8], out: &mut Vec<String>) {
    const OID_CN: &[u8] = &[0x55, 0x04, 0x03];
    while let Some((rdn_tag, mut rdn_set, rest)) = read_asn1_tlv(name_seq) {
        name_seq = rest;
        if rdn_tag != 0x31 {
            continue;
        }
        while let Some((atv_tag, atv_seq, rdn_rest)) = read_asn1_tlv(rdn_set) {
            rdn_set = rdn_rest;
            if atv_tag != 0x30 {
                continue;
            }
            if let Some((0x06, oid_bytes, val_part)) = read_asn1_tlv(atv_seq) {
                if oid_bytes == OID_CN {
                    if let Some((_, val_bytes, _)) = read_asn1_tlv(val_part) {
                        if let Ok(s) = std::str::from_utf8(val_bytes) {
                            let clean = s.trim().to_ascii_lowercase();
                            if !clean.is_empty() && !out.contains(&clean) {
                                out.push(clean);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extracts `dNSName` (`[2] IA5String`, tag `0x82`) entries from SubjectAltName (`OID 2.5.29.17`).
fn extract_san_dns_names(ext_wrapper: &[u8], out: &mut Vec<String>) {
    const OID_SAN: &[u8] = &[0x55, 0x1D, 0x11];
    let Some((0x30, mut ext_seq, _)) = read_asn1_tlv(ext_wrapper) else {
        return;
    };
    while let Some((0x30, single_ext, rest)) = read_asn1_tlv(ext_seq) {
        ext_seq = rest;
        let Some((0x06, oid_bytes, mut ext_rem)) = read_asn1_tlv(single_ext) else {
            continue;
        };
        if oid_bytes != OID_SAN {
            continue;
        }
        // Optional critical BOOLEAN (0x01) before extnValue OCTET STRING (0x04)
        while let Some((t, v, r)) = read_asn1_tlv(ext_rem) {
            ext_rem = r;
            if t == 0x04 {
                if let Some((0x30, mut gen_names, _)) = read_asn1_tlv(v) {
                    while let Some((gn_tag, gn_val, gn_rest)) = read_asn1_tlv(gen_names) {
                        gen_names = gn_rest;
                        // [2] dNSName context-specific primitive tag = 0x82
                        if gn_tag == 0x82 {
                            if let Ok(s) = std::str::from_utf8(gn_val) {
                                let clean = s.trim().to_ascii_lowercase();
                                if !clean.is_empty() && !out.contains(&clean) {
                                    out.push(clean);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// RFC 6125 hostname matching (exact case-insensitive match or single-label wildcard `*.example.com`).
fn cert_pattern_matches_domain(pattern: &str, domain: &str) -> bool {
    let pat = pattern.trim().trim_end_matches('.').to_ascii_lowercase();
    let dom = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if pat == dom {
        return true;
    }
    if let Some(suffix) = pat.strip_prefix("*.") {
        // Single-label wildcard: `sub.example.com` matches `*.example.com`, but `a.b.example.com` and `example.com` do not
        if let Some(prefix) = dom.strip_suffix(suffix) {
            if let Some(sub_label) = prefix.strip_suffix('.') {
                return !sub_label.is_empty() && !sub_label.contains('.');
            }
        }
    }
    false
}

/// Extracts and Base64-decodes the leaf (first) `-----BEGIN CERTIFICATE-----` block from PEM text.
fn decode_first_pem_certificate(pem: &str) -> Option<Vec<u8>> {
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    let start_idx = pem.find(BEGIN)? + BEGIN.len();
    let rest = &pem[start_idx..];
    let end_idx = rest.find(END)?;
    let b64_clean: String = rest[..end_idx]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if b64_clean.is_empty() {
        return None;
    }
    base64::engine::general_purpose::STANDARD
        .decode(b64_clean.as_bytes())
        .ok()
}

/// Checks SSL certificate expiry for `domain` by decoding Plesk PEM/DER certificates in `cert_base_path`
/// (`/usr/local/psa/var/certificates`, supporting both `.pem`/`.crt` files and Plesk's `scf*`/`cert*` files).
/// Returns `Some((rfc3339_expiry_date, days_remaining))` for the newest matching certificate.
pub fn check_ssl_cert_expiry(cert_base_path: &str, domain: &str) -> Option<(String, i64)> {
    let cert_dir = fs::read_dir(cert_base_path).ok()?;
    let mut best_expiry: Option<DateTime<Utc>> = None;

    for entry in cert_dir.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let is_cert_candidate = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "pem" | "crt" | "cer"))
            .unwrap_or(false)
            || name_str.starts_with("scf")
            || name_str.starts_with("cert");

        if !is_cert_candidate {
            continue;
        }

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };

        // 1. Primary Pure-Rust X.509 PEM -> DER -> ASN.1 TBSCertificate parser
        if let Some(der) = decode_first_pem_certificate(&content) {
            if let Some((_not_before, not_after, cert_names)) = parse_x509_der_validity_and_names(&der)
            {
                let matches_domain = cert_names
                    .iter()
                    .any(|pat| cert_pattern_matches_domain(pat, domain));
                if matches_domain
                    && best_expiry.map_or(true, |existing| not_after > existing)
                {
                    best_expiry = Some(not_after);
                }
                continue;
            }
        }

        // 2. Fallback for plain-text certificate dumps containing readable GMT timestamps
        if content.to_ascii_lowercase().contains(&domain.to_ascii_lowercase()) {
            if let Some(caps) = LEGACY_GMT_DATE_RE.captures(&content) {
                if let Ok(naive) = NaiveDateTime::parse_from_str(&caps[1], "%b %e %H:%M:%S %Y")
                    .or_else(|_| NaiveDateTime::parse_from_str(&caps[1], "%b %d %H:%M:%S %Y"))
                {
                    let exp = naive.and_utc();
                    if best_expiry.map_or(true, |existing| exp > existing) {
                        best_expiry = Some(exp);
                    }
                }
            }
        }
    }

    best_expiry.map(|expiry| {
        let days_left = (expiry - Utc::now()).num_days();
        (expiry.to_rfc3339(), days_left)
    })
}

/// Perform SEO checks for a domain.
pub async fn check_seo(domain: &str, client: &reqwest::Client) -> SeoCheckResult {
    let now = Utc::now().to_rfc3339();
    let mut issues = Vec::new();

    let robots_ok = match client
        .get(format!("https://{}/robots.txt", domain))
        .send()
        .await
    {
        Ok(r) => r.status().is_success(),
        Err(_) => {
            issues.push("robots.txt erişilemiyor".to_string());
            false
        }
    };

    let sitemap_ok = match client
        .get(format!("https://{}/sitemap.xml", domain))
        .send()
        .await
    {
        Ok(r) => r.status().is_success(),
        Err(_) => {
            issues.push("sitemap.xml erişilemiyor".to_string());
            false
        }
    };

    let (meta_title, meta_desc, canonical) =
        match client.get(format!("https://{}", domain)).send().await {
            Ok(resp) => {
                if let Ok(body) = resp.text().await {
                    let lower = body.to_ascii_lowercase();
                    let has_title =
                        lower.contains("<title>") && !lower.contains("<title></title>");
                    let has_desc = lower.contains("name=\"description\"")
                        || lower.contains("name='description'");
                    let has_canonical =
                        lower.contains("rel=\"canonical\"") || lower.contains("rel='canonical'");
                    if !has_title {
                        issues.push("Eksik title etiketi".to_string());
                    }
                    if !has_desc {
                        issues.push("Eksik meta description".to_string());
                    }
                    if !has_canonical {
                        issues.push("Eksik canonical URL".to_string());
                    }
                    (has_title, has_desc, has_canonical)
                } else {
                    (false, false, false)
                }
            }
            Err(_) => {
                issues.push("Ana sayfa erişilemiyor".to_string());
                (false, false, false)
            }
        };

    SeoCheckResult {
        domain: domain.to_string(),
        robots_txt: robots_ok,
        sitemap_xml: sitemap_ok,
        meta_description: meta_desc,
        meta_title,
        canonical,
        issues,
        checked_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_deltas_use_exact_jiffie_fields_without_fake_multipliers() {
        let prev = PrevCpuCounters {
            user: 1000,
            nice: 0,
            system: 500,
            idle: 8000,
            iowait: 300,
            irq: 0,
            softirq: 0,
            steal: 200,
        };
        let curr = PrevCpuCounters {
            user: 1100, // +100 (10%)
            nice: 0,
            system: 750, // +250 (25%)
            idle: 8500,  // +500 (50%)
            iowait: 400, // +100 (10%)
            irq: 0,
            softirq: 0,
            steal: 250, // +50  (5%)
        };
        let stats = compute_cpu_deltas(curr, Some(prev), [0.5, 0.4, 0.3]);
        assert_eq!(stats.user_pct, 10.0);
        assert_eq!(stats.system_pct, 25.0);
        assert_eq!(stats.idle_pct, 50.0);
        assert_eq!(stats.iowait_pct, 10.0);
        assert_eq!(stats.steal_pct, 5.0);
    }

    #[test]
    fn parses_nginx_combined_timestamp_accurately() {
        let dt = parse_nginx_timestamp("17/Nov/2026:19:30:14 +0300").expect("valid timestamp");
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2026-11-17 16:30:14");
    }

    #[test]
    fn wildcard_domain_matching_follows_rfc6125() {
        assert!(cert_pattern_matches_domain("*.msidergisi.com", "portal.msidergisi.com"));
        assert!(!cert_pattern_matches_domain("*.msidergisi.com", "msidergisi.com"));
        assert!(!cert_pattern_matches_domain("*.msidergisi.com", "a.b.msidergisi.com"));
        assert!(cert_pattern_matches_domain(
            "trusting-brahmagupta.194-110-169-165.plesk.page",
            "trusting-brahmagupta.194-110-169-165.plesk.page"
        ));
    }

    #[test]
    fn parses_minimal_der_x509_certificate_validity_and_san() {
        fn tlv(tag: u8, body: &[u8]) -> Vec<u8> {
            let mut out = vec![tag];
            if body.len() < 128 {
                out.push(body.len() as u8);
            } else {
                out.push(0x81);
                out.push(body.len() as u8);
            }
            out.extend_from_slice(body);
            out
        }

        // Build synthetic TBSCertificate DER with UTCTime validity & SAN dNSName
        let version = tlv(0xA0, &tlv(0x02, &[0x02]));
        let serial = tlv(0x02, &[0x01]);
        let alg = tlv(0x30, &tlv(0x06, &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D]));
        let cn_atv = tlv(
            0x30,
            &[tlv(0x06, &[0x55, 0x04, 0x03]), tlv(0x0C, b"msidergisi.com")].concat(),
        );
        let name = tlv(0x30, &tlv(0x31, &cn_atv));
        let validity = tlv(
            0x30,
            &[
                tlv(0x17, b"260101000000Z"),
                tlv(0x17, b"271231235959Z"),
            ]
            .concat(),
        );
        let spki = tlv(0x30, &[]);
        let san_gn = tlv(0x30, &tlv(0x82, b"*.plesk.page"));
        let san_ext = tlv(
            0x30,
            &[tlv(0x06, &[0x55, 0x1D, 0x11]), tlv(0x04, &san_gn)].concat(),
        );
        let exts = tlv(0xA3, &tlv(0x30, &san_ext));

        let tbs = tlv(
            0x30,
            &[version, serial, alg.clone(), name.clone(), validity, name, spki, exts].concat(),
        );
        let cert = tlv(0x30, &[tbs, alg, tlv(0x03, &[0x00])].concat());

        let (nb, na, names) = parse_x509_der_validity_and_names(&cert).expect("parsed X.509 DER");
        assert_eq!(nb.format("%Y-%m-%d").to_string(), "2026-01-01");
        assert_eq!(na.format("%Y-%m-%d").to_string(), "2027-12-31");
        assert!(names.contains(&"msidergisi.com".to_string()));
        assert!(names.contains(&"*.plesk.page".to_string()));
    }
}
