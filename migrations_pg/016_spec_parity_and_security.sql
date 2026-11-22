-- Migration 016: Spec Parity & Security Hardening (REQ-008, REQ-022, REQ-033)
-- Mirrors migrations/016_spec_parity_and_security.sql for PostgreSQL 16.
-- Note: ad_campaigns.start_date, end_date, rotation_mode were already added in 012.

CREATE TABLE IF NOT EXISTS revoked_tokens (
    token_hash TEXT PRIMARY KEY,
    expires_at TEXT NOT NULL,
    revoked_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_revoked_tokens_expires
    ON revoked_tokens(expires_at);

CREATE TABLE IF NOT EXISTS custom_field_values (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    field_id BIGINT NOT NULL REFERENCES custom_fields(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    value_text TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL,
    UNIQUE(field_id, entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_custom_field_values_entity
    ON custom_field_values(entity_type, entity_id);

CREATE INDEX IF NOT EXISTS idx_ad_campaigns_schedule
    ON ad_campaigns(is_active, start_date, end_date);
