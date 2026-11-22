-- Migration 007: Monitoring System Schema

CREATE TABLE IF NOT EXISTS monitoring_snapshots (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    snapshot_type TEXT NOT NULL,
    payload_json TEXT NOT NULL CHECK (payload_json::jsonb IS NOT NULL),
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_monitoring_snapshots_type_created
    ON monitoring_snapshots(snapshot_type, created_at DESC);

CREATE TABLE IF NOT EXISTS monitored_domains (
    domain TEXT PRIMARY KEY,
    vhost_path TEXT NOT NULL,
    access_log_path TEXT NOT NULL DEFAULT '',
    error_log_path TEXT NOT NULL DEFAULT '',
    ssl_cert_path TEXT NOT NULL DEFAULT '',
    last_health_check TEXT,
    last_http_status INTEGER,
    last_response_ms INTEGER,
    ssl_expiry_date TEXT,
    dns_resolves BOOLEAN NOT NULL DEFAULT TRUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS monitoring_attack_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    source_ip TEXT NOT NULL,
    attack_type TEXT NOT NULL,
    target_service TEXT NOT NULL DEFAULT '',
    details TEXT NOT NULL DEFAULT '{}' CHECK (details::jsonb IS NOT NULL),
    severity TEXT NOT NULL DEFAULT 'medium',
    detected_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_monitoring_attacks_detected
    ON monitoring_attack_events(detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_monitoring_attacks_ip
    ON monitoring_attack_events(source_ip);
