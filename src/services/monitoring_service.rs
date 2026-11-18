//! Monitoring Service (`src/services/monitoring_service.rs`)
//!
//! Cache management, background worker ticks, and zero-load server telemetry orchestration.
//! All API handlers read from the in-memory `MonitoringCache`; only background ticks touch `/proc`, logs, or sockets.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::engine::monitoring_engine::*;

// ─── Cache Structure ────────────────────────────────────────────────────

pub struct MonitoringCache {
    pub system: Option<SystemSnapshot>,
    pub domains: HashMap<String, DomainHealthSnapshot>,
    pub traffic: TrafficSummary,
    pub services: HashMap<String, ServiceStatus>,
    pub attacks: VecDeque<AttackEvent>,
    pub seo: HashMap<String, SeoCheckResult>,
    pub prev_cpu_counters: Option<PrevCpuCounters>,
    pub prev_net_bytes: HashMap<String, (u64, u64)>,
    pub last_system_tick: Option<Instant>,
    pub vhosts: Vec<VhostInfo>,
    pub last_updated: Instant,
}

impl MonitoringCache {
    pub fn new() -> Self {
        Self {
            system: None,
            domains: HashMap::new(),
            traffic: TrafficSummary {
                per_domain: Vec::new(),
                total_rpm: 0,
            },
            services: HashMap::new(),
            attacks: VecDeque::with_capacity(crate::db::constants::ATTACK_BUFFER_CAPACITY),
            seo: HashMap::new(),
            prev_cpu_counters: None,
            prev_net_bytes: HashMap::new(),
            last_system_tick: None,
            vhosts: Vec::new(),
            last_updated: Instant::now(),
        }
    }
}

impl Default for MonitoringCache {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedMonitoringCache = Arc<RwLock<MonitoringCache>>;

/// Creates an empty thread-safe `SharedMonitoringCache`.
pub fn new_monitoring_cache() -> SharedMonitoringCache {
    Arc::new(RwLock::new(MonitoringCache::new()))
}

/// Initialize the monitoring cache, prime CPU/network baselines, and discover Plesk vhosts.
pub async fn init_monitoring(vhosts_path: &str, nginx_log_path: &str) -> SharedMonitoringCache {
    let cache = new_monitoring_cache();

    let vhosts = discover_vhosts(vhosts_path, nginx_log_path);
    let (_, initial_cpu) = read_cpu_stats(None);
    let initial_net = read_network_stats();
    info!("Monitoring: discovered {} vhosts", vhosts.len());

    {
        let mut c = cache.write().await;
        c.vhosts = vhosts;
        c.prev_cpu_counters = Some(initial_cpu);
        for iface in &initial_net {
            c.prev_net_bytes
                .insert(iface.name.clone(), (iface.rx_bytes, iface.tx_bytes));
        }
        c.last_system_tick = Some(Instant::now());
    }

    cache
}

/// Get full monitoring bundle from cache (for API).
pub async fn get_full_snapshot(cache: &SharedMonitoringCache) -> MonitoringBundle {
    let c = cache.read().await;
    MonitoringBundle {
        system: c.system.clone(),
        domains: c.domains.values().cloned().collect(),
        traffic: c.traffic.clone(),
        services: c.services.values().cloned().collect(),
        attacks: c.attacks.iter().cloned().collect(),
        seo: c.seo.values().cloned().collect(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Alias for `get_full_snapshot` returning the full `MonitoringBundle` from cache.
pub async fn collect_full_bundle(cache: &SharedMonitoringCache) -> MonitoringBundle {
    get_full_snapshot(cache).await
}

// ─── Fast Tick: System Stats (every 15s) ────────────────────────────────

pub async fn tick_system_stats(cache: &SharedMonitoringCache) {
    let mut c = cache.write().await;

    let (cpu, new_counters) = read_cpu_stats(c.prev_cpu_counters);
    let memory = read_memory_stats();
    let disk = read_disk_stats();
    let network = read_network_stats();

    let now_inst = Instant::now();
    let interval_secs = c
        .last_system_tick
        .map(|prev| now_inst.duration_since(prev).as_secs_f64().max(0.25))
        .unwrap_or(15.0);

    let mut net_rx_per_sec: u64 = 0;
    let mut net_tx_per_sec: u64 = 0;

    for iface in &network {
        if let Some((prev_rx, prev_tx)) = c.prev_net_bytes.get(&iface.name) {
            let rx_delta = iface.rx_bytes.saturating_sub(*prev_rx);
            let tx_delta = iface.tx_bytes.saturating_sub(*prev_tx);
            net_rx_per_sec += (rx_delta as f64 / interval_secs) as u64;
            net_tx_per_sec += (tx_delta as f64 / interval_secs) as u64;
        }
        c.prev_net_bytes
            .insert(iface.name.clone(), (iface.rx_bytes, iface.tx_bytes));
    }

    c.prev_cpu_counters = Some(new_counters);
    c.last_system_tick = Some(now_inst);
    c.system = Some(SystemSnapshot {
        cpu,
        memory,
        disk,
        network,
        net_rx_per_sec,
        net_tx_per_sec,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    c.last_updated = now_inst;
}

// ─── Medium Tick: Domains, Traffic, Services, Attacks (every 60s) ───────

pub async fn tick_domain_and_traffic(
    app_port: u16,
    node_ports: &[u16],
    fail2ban_log_path: &str,
    db: &crate::db::Database,
    cache: &SharedMonitoringCache,
) {
    let (vhosts, services_config) = {
        let c = cache.read().await;
        let redis_port = std::env::var("MONITORING_REDIS_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6379);
        let smtp_port = std::env::var("MONITORING_SMTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(25);
        let mut svc: Vec<(String, &'static str, u16)> = vec![
            ("MariaDB".to_string(), "127.0.0.1", 3306u16),
            ("Redis".to_string(), "127.0.0.1", redis_port),
            ("Nginx HTTP".to_string(), "127.0.0.1", 80),
            ("Nginx HTTPS".to_string(), "127.0.0.1", 443),
            ("Postfix SMTP".to_string(), "127.0.0.1", smtp_port),
            ("Dovecot IMAPS".to_string(), "127.0.0.1", 993),
            ("App Server".to_string(), "127.0.0.1", app_port),
        ];
        for p in node_ports {
            if *p == app_port {
                continue;
            }
            svc.push((format!("Port {}", p), "127.0.0.1", *p));
        }
        (c.vhosts.clone(), svc)
    };

    // Primary client enforces full TLS certificate validation (expiry, chain, hostname)
    let strict_client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("Monitoring: failed to build strict HTTP client: {}", e);
            return;
        }
    };

    // Secondary diagnostic client used ONLY when strict TLS fails, to record HTTP status while flagging TLS invalid
    let diagnostic_client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("Monitoring: failed to build diagnostic HTTP client: {}", e);
            return;
        }
    };

    // Check domain health with semaphore-limited concurrency (max 12, zero `.unwrap()`)
    let semaphore = Arc::new(tokio::sync::Semaphore::new(12));
    let mut domain_handles = Vec::with_capacity(vhosts.len());

    for vhost in &vhosts {
        let sem = semaphore.clone();
        let strict_c = strict_client.clone();
        let diag_c = diagnostic_client.clone();
        let domain = vhost.domain.clone();
        domain_handles.push(tokio::spawn(async move {
            let Ok(_permit) = sem.acquire().await else {
                return None;
            };
            Some(check_domain_health_verified(&domain, &strict_c, &diag_c).await)
        }));
    }

    let mut domain_results = HashMap::new();
    for handle in domain_handles {
        if let Ok(Some(result)) = handle.await {
            domain_results.insert(result.domain.clone(), result);
        }
    }

    // Check service ports (each port has a unique key — e.g. "Port 3000", "Port 3004")
    let mut service_results = HashMap::new();
    for (name, host, port) in &services_config {
        let mut status = check_tcp_port(host, *port, 3000).await;
        status.name = name.clone();
        service_results.insert(name.clone(), status);
    }

    // Parse traffic from nginx access logs — count ONLY requests within the last 60 seconds for RPM!
    let now_utc = chrono::Utc::now();
    let mut per_domain_traffic = Vec::new();
    let mut total_rpm: u64 = 0;

    for vhost in &vhosts {
        if vhost.access_log_path.is_empty() {
            continue;
        }
        let entries = parse_nginx_access_log_tail(&vhost.access_log_path, crate::db::constants::NGINX_LOG_TAIL_LINES);
        if entries.is_empty() {
            continue;
        }

        let mut status_codes: HashMap<u16, u64> = HashMap::new();
        let mut ip_counts: HashMap<String, u64> = HashMap::new();
        let mut rpm: u64 = 0;

        for entry in &entries {
            *status_codes.entry(entry.status).or_insert(0) += 1;
            *ip_counts.entry(entry.ip.clone()).or_insert(0) += 1;

            if let Some(entry_dt) = parse_nginx_timestamp(&entry.timestamp) {
                let age_secs = (now_utc - entry_dt).num_seconds();
                if (-5..=60).contains(&age_secs) {
                    rpm += 1;
                }
            }
        }

        let mut top_ips: Vec<(String, u64)> = ip_counts.into_iter().collect();
        top_ips.sort_by(|a, b| b.1.cmp(&a.1));
        top_ips.truncate(5);

        total_rpm += rpm;

        per_domain_traffic.push(DomainTraffic {
            domain: vhost.domain.clone(),
            requests_per_min: rpm,
            status_codes,
            top_ips,
        });
    }

    // Parse fail2ban for attack events
    let f2b_events = parse_fail2ban_log_tail(fail2ban_log_path, crate::db::constants::F2B_LOG_TAIL_LINES);
    let mut new_attacks: Vec<AttackEvent> = Vec::new();

    for event in &f2b_events {
        if event.action == "Ban" {
            let severity = match event.jail.as_str() {
                "sshd" | "ssh" | "recidive" => "high",
                "nginx-http-auth" | "apache-auth" | "plesk-panel" => "medium",
                _ => "low",
            };
            new_attacks.push(AttackEvent {
                id: None,
                source_ip: event.ip.clone(),
                attack_type: format!("fail2ban_ban:{}", event.jail),
                severity: severity.to_string(),
                target_service: event.jail.clone(),
                details: serde_json::json!({
                    "jail": event.jail,
                    "timestamp": event.timestamp,
                })
                .to_string(),
                detected_at: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    // Update cache while preserving previously resolved SSL expiry metadata from slow tick
    {
        let mut c = cache.write().await;
        for (domain, snapshot) in &mut domain_results {
            if let Some(prev_snap) = c.domains.get(domain) {
                if snapshot.ssl_expiry.is_none() {
                    snapshot.ssl_expiry = prev_snap.ssl_expiry.clone();
                }
                if snapshot.ssl_days_left.is_none() {
                    snapshot.ssl_days_left = prev_snap.ssl_days_left;
                }
            }
        }
        c.domains = domain_results;
        c.services = service_results;
        c.traffic = TrafficSummary {
            per_domain: per_domain_traffic,
            total_rpm,
        };

        for attack in new_attacks {
            let is_dup = c.attacks.iter().any(|existing| {
                existing.source_ip == attack.source_ip
                    && existing.attack_type == attack.attack_type
            });
            if !is_dup {
                let _ = db.insert_attack_event(
                    &attack.source_ip,
                    &attack.attack_type,
                    &attack.target_service,
                    &attack.details,
                    &attack.severity,
                );
                if c.attacks.len() >= crate::db::constants::ATTACK_BUFFER_CAPACITY {
                    c.attacks.pop_front();
                }
                c.attacks.push_back(attack);
            }
        }

        c.last_updated = Instant::now();
    }
}

// ─── Slow Tick: SSL, SEO, DB Persistence (every 3600s) ──────────────────

pub async fn tick_slow_checks(
    vhosts_path: &str,
    nginx_log_path: &str,
    cert_base_path: &str,
    retention_days: u32,
    db: &crate::db::Database,
    cache: &SharedMonitoringCache,
) {
    let fresh_vhosts = discover_vhosts(vhosts_path, nginx_log_path);
    info!(
        "Monitoring: refreshed vhost list, {} domains found",
        fresh_vhosts.len()
    );

    for vhost in &fresh_vhosts {
        let _ = db.upsert_monitored_domain(
            &vhost.domain,
            &vhost.vhost_path,
            &vhost.access_log_path,
            &vhost.error_log_path,
        );
    }

    // Pure-Rust X.509 PEM/DER SSL certificate expiry checks
    let mut ssl_updates: HashMap<String, (String, i64)> = HashMap::new();
    for vhost in &fresh_vhosts {
        if let Some((expiry, days_left)) = check_ssl_cert_expiry(cert_base_path, &vhost.domain) {
            ssl_updates.insert(vhost.domain.clone(), (expiry, days_left));
        }
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!(
                "Monitoring: failed to build HTTP client for slow checks: {}",
                e
            );
            return;
        }
    };

    // SEO checks (batched, max 12 concurrent, zero `.unwrap()`)
    let semaphore = Arc::new(tokio::sync::Semaphore::new(12));
    let mut seo_handles = Vec::with_capacity(fresh_vhosts.len());

    for vhost in &fresh_vhosts {
        let sem = semaphore.clone();
        let client = client.clone();
        let domain = vhost.domain.clone();
        seo_handles.push(tokio::spawn(async move {
            let Ok(_permit) = sem.acquire().await else {
                return None;
            };
            Some(check_seo(&domain, &client).await)
        }));
    }

    let mut seo_results = HashMap::new();
    for handle in seo_handles {
        if let Ok(Some(result)) = handle.await {
            seo_results.insert(result.domain.clone(), result);
        }
    }

    // Persist snapshot to DB (hourly)
    {
        let c = cache.read().await;
        if let Some(ref sys) = c.system {
            let payload = serde_json::to_string(sys).unwrap_or_default();
            let _ = db.insert_monitoring_snapshot("system", &payload);
        }
    }

    // Prune old data
    let _ = db.prune_monitoring_data(retention_days);

    // Update cache with fresh vhosts, SEO results, and SSL data
    {
        let mut c = cache.write().await;
        c.vhosts = fresh_vhosts;
        c.seo = seo_results;

        for (domain, (expiry, days_left)) in ssl_updates {
            if let Some(snapshot) = c.domains.get_mut(&domain) {
                snapshot.ssl_expiry = Some(expiry);
                snapshot.ssl_days_left = Some(days_left);
                if days_left < 0 {
                    snapshot.tls_valid = false;
                }
            }
        }

        c.last_updated = Instant::now();
    }

    info!("Monitoring: slow tick completed — SSL checked, SEO checked, snapshot persisted, old data pruned");
}

/// Force immediate re-collection of all monitoring data.
#[allow(clippy::too_many_arguments)]
pub async fn force_refresh(
    app_port: u16,
    node_ports: &[u16],
    fail2ban_log_path: &str,
    vhosts_path: &str,
    nginx_log_path: &str,
    cert_base_path: &str,
    retention_days: u32,
    db: &crate::db::Database,
    cache: &SharedMonitoringCache,
) {
    tick_system_stats(cache).await;
    tick_domain_and_traffic(app_port, node_ports, fail2ban_log_path, db, cache).await;
    tick_slow_checks(
        vhosts_path,
        nginx_log_path,
        cert_base_path,
        retention_days,
        db,
        cache,
    )
    .await;
}
