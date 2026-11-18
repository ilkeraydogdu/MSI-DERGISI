CREATE TABLE IF NOT EXISTS email_verification_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    token TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_evt_token ON email_verification_tokens(token);
CREATE INDEX IF NOT EXISTS idx_evt_user ON email_verification_tokens(user_id);

ALTER TABLE users ADD COLUMN email_verified INTEGER NOT NULL DEFAULT 0;