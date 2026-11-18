//! Monitoring Engine (`src/engine/monitoring_engine/`)
//!
//! Pure functions for reading `/proc`, Nginx & Fail2ban logs, TCP ports, Plesk vhosts,
//! and X.509 PEM/DER SSL certificates without spawning external binaries.

pub mod health;
pub mod logs;
pub mod seo;
pub mod system;
pub mod types;
pub mod vhosts;
pub mod x509;

pub use health::{check_domain_health, check_domain_health_verified, check_tcp_port};
pub use logs::{parse_fail2ban_log_tail, parse_nginx_access_log_tail, parse_nginx_timestamp};
pub use seo::check_seo;
pub use system::{
    compute_cpu_deltas, read_cpu_stats, read_disk_stats, read_load_avg, read_memory_stats,
    read_network_stats,
};
pub use types::*;
pub use vhosts::discover_vhosts;
pub use x509::{check_ssl_cert_expiry, parse_x509_der_validity_and_names};

#[cfg(test)]
mod tests;
