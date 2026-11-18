use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::controllers::AppState;

pub fn spawn_renewal_reminder_worker(state: AppState, cancel: CancellationToken) {
    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            _ = cancel.cancelled() => return,
        }
        let mut interval = tokio::time::interval(Duration::from_secs(6 * 3600));
        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = cancel.cancelled() => {
                    info!("Renewal reminder worker shutting down gracefully");
                    return;
                }
            }
            let users = state.db.get_all_users();
            let existing = state.db.get_renewal_reminders();
            let evaluated =
                crate::services::subscription_service::evaluate_5_stage_reminders(&users, &existing);
            let _ = state.db.sync_evaluated_reminders(&evaluated);

            if state.config.mail_delivery_enabled {
                let due: Vec<_> = state
                    .db
                    .get_renewal_reminders()
                    .into_iter()
                    .filter(|r| r.status == "scheduled")
                    .collect();
                for reminder in due {
                    if crate::services::email_service::dispatch_renewal_reminder_email(
                        &state.config,
                        &reminder,
                    )
                    .await
                    .is_ok()
                    {
                        let _ = state.db.mark_renewal_reminder_sent(reminder.id);
                    }
                }
            }
        }
    });
}