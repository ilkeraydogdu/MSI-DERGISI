-- Migration 015: Option B Public, Compliance, B2B & Scheduling Features
-- Portable SQLite DDL paired with Option A's migrations_pg/015_option_b_features.sql.
-- Timestamps (created_at, sent_at, tos_accepted_at, scheduled_at) are supplied
-- as UTC RFC3339 strings from Rust (no SQLite-specific datetime('now')).

-- B2: Timestamped Terms of Service (ToS) acceptance tracking on users
ALTER TABLE users ADD COLUMN tos_accepted_at TEXT NOT NULL DEFAULT '';
ALTER TABLE users ADD COLUMN tos_version TEXT NOT NULL DEFAULT '2026-01';

-- B2: Immutable KVKK / ToS / ETK / Cookie consent audit log
CREATE TABLE IF NOT EXISTS kvkk_consent_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    email TEXT NOT NULL DEFAULT '',
    consent_type TEXT NOT NULL,
    consent_version TEXT NOT NULL DEFAULT '2026-01',
    accepted INTEGER NOT NULL DEFAULT 1 CHECK (accepted IN (0, 1)),
    ip_address TEXT NOT NULL DEFAULT '',
    consent_hash TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_kvkk_consent_logs_user
    ON kvkk_consent_logs(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_kvkk_consent_logs_email
    ON kvkk_consent_logs(LOWER(email), created_at DESC);

-- B1: Granular video watch-percentage telemetry (0-100)
ALTER TABLE analytics_events ADD COLUMN video_watch_pct INTEGER NOT NULL DEFAULT 0;

-- B13: Institution/company-exclusive B2B content gating
ALTER TABLE contents ADD COLUMN target_institutions_json TEXT NOT NULL DEFAULT '[]';

-- B7: Scheduled newsletter campaigns, email_group_code targeting & custom HTML body
ALTER TABLE newsletter_campaigns ADD COLUMN scheduled_at TEXT;
ALTER TABLE newsletter_campaigns ADD COLUMN email_group_code TEXT;
ALTER TABLE newsletter_campaigns ADD COLUMN body_html TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_newsletter_campaigns_sched
    ON newsletter_campaigns(status, scheduled_at);

-- B8: Automated monthly B2B & subscriber report deduplication log
CREATE TABLE IF NOT EXISTS b2b_monthly_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_month TEXT NOT NULL,
    report_type TEXT NOT NULL,
    recipient_email TEXT NOT NULL,
    summary_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'pending',
    sent_at TEXT NOT NULL DEFAULT '',
    UNIQUE(report_month, report_type, recipient_email)
);
CREATE INDEX IF NOT EXISTS idx_b2b_monthly_reports_month
    ON b2b_monthly_reports(report_month, status);
