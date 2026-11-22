-- Migration 004: Analytics Daily Aggregation Table

CREATE TABLE IF NOT EXISTS analytics_daily_agg (
    date TEXT NOT NULL,
    content_slug TEXT NOT NULL DEFAULT '',
    ad_id BIGINT NOT NULL DEFAULT 0,
    organization_category TEXT NOT NULL DEFAULT '',
    event_type TEXT NOT NULL,
    event_count BIGINT NOT NULL DEFAULT 0,
    unique_sessions BIGINT NOT NULL DEFAULT 0,
    avg_duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    avg_scroll_pct DOUBLE PRECISION NOT NULL DEFAULT 0,
    true_read_count BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (date, event_type, content_slug, ad_id, organization_category)
);

CREATE INDEX IF NOT EXISTS idx_daily_agg_date ON analytics_daily_agg(date);
CREATE INDEX IF NOT EXISTS idx_daily_agg_content ON analytics_daily_agg(content_slug, date);
CREATE INDEX IF NOT EXISTS idx_daily_agg_ad ON analytics_daily_agg(ad_id, date);
CREATE INDEX IF NOT EXISTS idx_daily_agg_org ON analytics_daily_agg(organization_category, date);
