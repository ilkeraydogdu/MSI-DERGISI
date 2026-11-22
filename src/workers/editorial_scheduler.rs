//! Background worker that promotes scheduled Editorial Studio articles whose
//! `scheduled_at` timestamp has passed and dispatches IndexNow + configured social shares.

use crate::app_state::AppState;
use crate::services::editorial_social_service::{
    has_configured_channels, share_on_all, ShareArticlePayload,
};
use crate::services::indexnow_service::spawn_instant_seo_push;
use serde_json::Value;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// Runs one scheduler tick, returning the number of scheduled articles promoted to `published`.
pub async fn run_editorial_scheduler_tick(state: &AppState) -> usize {
    let due = match state.db.editorial_publish_due_scheduled() {
        Ok(items) => items,
        Err(err) => {
            error!("Editorial scheduler query failed: {}", err);
            return 0;
        }
    };

    if due.is_empty() {
        return 0;
    }

    let count = due.len();
    let base_url = state.config.public_url.trim_end_matches('/');
    let paths: Vec<String> = due
        .iter()
        .map(|item| format!("/icerik/{}", item.slug))
        .collect();
    spawn_instant_seo_push((*state.config).clone(), paths);

    let settings_map = state.db.get_setting_map("tr");
    let social_cfg: Option<Value> = settings_map
        .get("editorial_social_config")
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .filter(has_configured_channels);

    if let Some(cfg_json) = social_cfg {
        let http = reqwest::Client::new();
        for item in &due {
            if !item.social_shared {
                continue;
            }
            let full_url = format!("{base_url}/icerik/{}", item.slug);
            let _ = share_on_all(
                &http,
                &cfg_json,
                ShareArticlePayload {
                    title: &item.title,
                    url: &full_url,
                    description: &item.spot,
                },
            )
            .await;
        }
    }

    count
}

pub fn spawn_editorial_scheduler_worker(state: AppState, cancel: CancellationToken) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(30));
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("Editorial scheduler worker shutting down gracefully");
                    return;
                }
                _ = ticker.tick() => {
                    let count = run_editorial_scheduler_tick(&state).await;
                    if count > 0 {
                        info!("Editorial scheduler published {} scheduled article(s)", count);
                    }
                }
            }
        }
    });
}
