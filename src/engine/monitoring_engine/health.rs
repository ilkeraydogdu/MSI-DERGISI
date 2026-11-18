use chrono::Utc;
use std::time::Duration;
use super::types::*;

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

