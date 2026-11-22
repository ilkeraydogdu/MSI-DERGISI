pub mod ads;
pub mod archive;
pub mod commerce;
pub mod issues;
pub mod rbac;
pub mod settings;
pub mod taxonomy;

pub use rbac::seed_user_roles;
pub use settings::seed_site_settings_and_notifications;

use rusqlite::Connection;

/// Bootstraps an empty database from canonical configuration and the harvested live archive.
/// Never inserts fictional demo users, fake articles, fake invoices, or synthetic analytics.
pub fn seed_initial_data(conn: &Connection) -> rusqlite::Result<()> {
    taxonomy::seed_categories(conn)?;
    taxonomy::seed_ssb_tags(conn)?;
    archive::seed_archive_from_json(conn)?;
    issues::seed_magazine_issues(conn)?;
    ads::seed_ad_campaigns(conn)?;
    commerce::seed_subscription_plans_and_commerce(conn)?;
    settings::seed_site_settings_and_notifications(conn)?;
    taxonomy::seed_operational_defaults(conn)?;
    Ok(())
}
