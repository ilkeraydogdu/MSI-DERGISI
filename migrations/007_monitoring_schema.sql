-- Monitoring System Schema Migration
-- Rolling snapshot storage (only latest N kept via pruning)
CREATE TABLE IF NOT EXISTS monitoring_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_monitoring_snapshots_type_created
    ON monitoring_snapshots(snapshot_type, created_at DESC);

-- Domain registry (discovered from /var/www/vhosts/ at startup)
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
    dns_resolves INTEGER NOT NULL DEFAULT 1,
    is_active INTEGER NOT NULL DEFAULT 1
);

-- Attack event log (append-only, pruned by retention policy)
CREATE TABLE IF NOT EXISTS monitoring_attack_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_ip TEXT NOT NULL,
    attack_type TEXT NOT NULL,
    target_service TEXT NOT NULL DEFAULT '',
    details TEXT NOT NULL DEFAULT '{}',
    severity TEXT NOT NULL DEFAULT 'medium',
    detected_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_monitoring_attacks_detected
    ON monitoring_attack_events(detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_monitoring_attacks_ip
    ON monitoring_attack_events(source_ip);