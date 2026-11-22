use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::controllers::AppState;
use crate::models::UserAccount;
use crate::services::email_service;

/// Claims and dispatches due scheduled newsletter campaigns (`B7`).
/// Returns the number of campaigns processed.
pub async fn run_newsletter_scheduler_tick(state: &AppState) -> usize {
    let now_utc = chrono::Utc::now();
    let now_cmp = now_utc.format("%Y-%m-%d %H:%M:%S").to_string();
    let today_str = now_utc.format("%Y-%m-%d").to_string();
    let due = state.db.claim_due_scheduled_newsletters(&now_cmp);
    if due.is_empty() {
        return 0;
    }

    let all_users = state.db.get_all_users();
    let all_ads = state.db.get_all_ads();
    let mut processed = 0usize;

    for mut campaign in due {
        let group_owned: Vec<UserAccount>;
        let recipient_refs: Vec<&UserAccount> = if let Some(ref code) = campaign
            .email_group_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            group_owned = state.db.resolve_email_group_recipients(
                code,
                &campaign.target_institution,
                &campaign.target_ssb_tag,
            );
            group_owned.iter().collect()
        } else {
            email_service::filter_newsletter_recipients(
                &all_users,
                &campaign.target_group_filter,
                &campaign.target_institution,
                &campaign.target_ssb_tag,
            )
        };

        let sponsor_ad = crate::controllers::api_controller::admin_ops::resolve_newsletter_sponsor_ad(
            &all_ads,
            campaign.sponsor_ad_id,
            &campaign.target_institution,
            &campaign.target_ssb_tag,
            &today_str,
        );
        if campaign.sponsor_ad_id.filter(|&id| id > 0).is_none() {
            if let Some(sp) = sponsor_ad {
                campaign.sponsor_ad_id = Some(sp.id);
                if campaign.sponsor_name.as_deref().map(str::trim).unwrap_or("").is_empty() {
                    campaign.sponsor_name = Some(sp.sponsor_name.clone());
                }
            }
        }
        let body_opt = (!campaign.body_html.trim().is_empty()).then_some(campaign.body_html.as_str());

        let dispatch = email_service::dispatch_newsletter_campaign(
            &state.config,
            &state.db,
            &campaign,
            &recipient_refs,
            sponsor_ad,
            body_opt,
        )
        .await;

        let final_status = if !state.config.mail_delivery_enabled {
            "queued"
        } else if dispatch.failed_count == 0 {
            "sent"
        } else if dispatch.delivered_count > 0 {
            "sent_partial"
        } else {
            "failed"
        };

        let sent_at = chrono::Utc::now().to_rfc3339();
        let _ = state.db.finalize_newsletter_dispatch(
            campaign.id,
            final_status,
            dispatch.eligible_recipients as i64,
            &sent_at,
        );
        processed += 1;
    }

    processed
}

pub fn spawn_newsletter_scheduler_worker(state: AppState, cancel: CancellationToken) {
    tokio::spawn(async move {
        let interval_secs = state.config.newsletter_poll_interval_secs.max(10);
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("Newsletter scheduler worker shutting down gracefully");
                    return;
                }
                _ = ticker.tick() => {
                    let count = run_newsletter_scheduler_tick(&state).await;
                    if count > 0 {
                        info!("Newsletter scheduler dispatched {} campaign(s)", count);
                    }
                }
            }
        }
    });
}
