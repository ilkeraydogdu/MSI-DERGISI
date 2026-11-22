use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::controllers::AppState;

pub fn spawn_monitoring_worker(state: AppState, cancel: CancellationToken) {
    let Some(cache) = state.monitoring_cache.clone() else {
        return;
    };
    tokio::spawn(async move {
        use crate::services::monitoring_service as ms;
        let cfg = state.config.clone();
        let fast = Duration::from_secs(cfg.monitoring_fast_interval_secs.max(5));
        let medium = Duration::from_secs(cfg.monitoring_medium_interval_secs.max(15));
        let slow = Duration::from_secs(cfg.monitoring_slow_interval_secs.max(300));

        // Initial warm-up so the admin dashboard has data immediately.
        ms::tick_system_stats(&cache).await;
        ms::tick_domain_and_traffic(
            cfg.port,
            &cfg.monitoring_node_ports,
            &cfg.monitoring_fail2ban_log_path,
            &state.db,
            &cache,
        )
        .await;
        {
            let slow_cfg = cfg.clone();
            let slow_db = state.db.clone();
            let slow_cache = cache.clone();
            tokio::spawn(async move {
                ms::tick_slow_checks(
                    &slow_cfg.monitoring_vhosts_path,
                    &slow_cfg.monitoring_nginx_log_path,
                    &slow_cfg.monitoring_cert_path,
                    slow_cfg.monitoring_retention_days,
                    &slow_db,
                    &slow_cache,
                )
                .await;
            });
        }

        let mut fast_t = tokio::time::interval(fast);
        let mut medium_t = tokio::time::interval(medium);
        let mut slow_t = tokio::time::interval(slow);
        // Consume immediate first ticks already handled by startup warm-up above.
        fast_t.tick().await;
        medium_t.tick().await;
        slow_t.tick().await;
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("Monitoring worker shutting down gracefully");
                    return;
                }
                _ = fast_t.tick() => {
                    ms::tick_system_stats(&cache).await;
                }
                _ = medium_t.tick() => {
                    ms::tick_domain_and_traffic(
                        cfg.port,
                        &cfg.monitoring_node_ports,
                        &cfg.monitoring_fail2ban_log_path,
                        &state.db,
                        &cache,
                    ).await;
                    let _ = crate::services::security_anomaly_service::evaluate_and_emit_anomalies(&state.db);
                }
                _ = slow_t.tick() => {
                    ms::tick_slow_checks(
                        &cfg.monitoring_vhosts_path,
                        &cfg.monitoring_nginx_log_path,
                        &cfg.monitoring_cert_path,
                        cfg.monitoring_retention_days,
                        &state.db,
                        &cache,
                    ).await;
                }
            }
        }
    });
}