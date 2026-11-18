use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::controllers::AppState;

pub fn spawn_analytics_aggregation_worker(state: AppState, cancel: CancellationToken) {
    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
            _ = cancel.cancelled() => return,
        }
        let mut interval = tokio::time::interval(Duration::from_secs(6 * 3600));
        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = cancel.cancelled() => {
                    info!("Analytics aggregation worker shutting down gracefully");
                    return;
                }
            }
            let today = chrono::Utc::now().date_naive();
            for day in [today - chrono::Duration::days(1), today] {
                let ds = day.format("%Y-%m-%d").to_string();
                match state.db.aggregate_analytics_for_date(&ds) {
                    Ok(n) => info!("Analytics rollup {} -> {} rows", ds, n),
                    Err(e) => warn!("Analytics rollup {} failed: {}", ds, e),
                }
            }
        }
    });
}