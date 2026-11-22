-- Migration 015: Option B Public, Compliance, B2B & Scheduling Features
-- Mirrors migrations/015_option_b_features.sql and PROJECT.md Interface Contracts #3

-- B2: Timestamped Terms of Service (ToS) acceptance tracking on users
ALTER TABLE users ADD COLUMN IF NOT EXISTS tos_accepted_at TEXT NOT NULL DEFAULT '';
ALTER TABLE users ADD COLUMN IF NOT EXISTS tos_version TEXT NOT NULL DEFAULT '2026-01';

-- B2: Immutable KVKK / ToS / ETK / Cookie consent audit log
CREATE TABLE IF NOT EXISTS kvkk_consent_logs (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    email TEXT NOT NULL DEFAULT '',
    consent_type TEXT NOT NULL,
    consent_version TEXT NOT NULL DEFAULT '2026-01',
    accepted BOOLEAN NOT NULL DEFAULT TRUE,
    ip_address TEXT NOT NULL DEFAULT '',
    consent_hash TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
);
CREATE INDEX IF NOT EXISTS idx_kvkk_consent_logs_user
    ON kvkk_consent_logs(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_kvkk_consent_logs_email
    ON kvkk_consent_logs(LOWER(email), created_at DESC);

-- B1: Granular video watch-percentage telemetry (0-100)
ALTER TABLE analytics_events ADD COLUMN IF NOT EXISTS video_watch_pct INTEGER NOT NULL DEFAULT 0;

-- B13: Institution/company-exclusive B2B content gating
ALTER TABLE contents ADD COLUMN IF NOT EXISTS target_institutions_json TEXT NOT NULL DEFAULT '[]';
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_contents_target_institutions_json'
    ) THEN
        ALTER TABLE contents ADD CONSTRAINT chk_contents_target_institutions_json CHECK (target_institutions_json::jsonb IS NOT NULL);
    END IF;
END $$;

-- B7: Scheduled newsletter campaigns, email_group_code targeting & custom HTML body
ALTER TABLE newsletter_campaigns ADD COLUMN IF NOT EXISTS scheduled_at TEXT;
ALTER TABLE newsletter_campaigns ADD COLUMN IF NOT EXISTS email_group_code TEXT;
ALTER TABLE newsletter_campaigns ADD COLUMN IF NOT EXISTS body_html TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_newsletter_campaigns_sched
    ON newsletter_campaigns(status, scheduled_at);

-- B8: Automated monthly B2B & subscriber report deduplication log
CREATE TABLE IF NOT EXISTS b2b_monthly_reports (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    report_month TEXT NOT NULL,
    report_type TEXT NOT NULL,
    recipient_email TEXT NOT NULL,
    summary_json TEXT NOT NULL DEFAULT '{}' CHECK (summary_json::jsonb IS NOT NULL),
    status TEXT NOT NULL DEFAULT 'pending',
    sent_at TEXT NOT NULL DEFAULT '',
    UNIQUE(report_month, report_type, recipient_email)
);
CREATE INDEX IF NOT EXISTS idx_b2b_monthly_reports_month
    ON b2b_monthly_reports(report_month, status);
