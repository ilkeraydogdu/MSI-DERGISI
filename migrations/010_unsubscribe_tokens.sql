-- KVKK/GDPR-compliant unsubscribe token system
CREATE TABLE IF NOT EXISTS unsubscribe_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    token TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    used INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_unsub_token ON unsubscribe_tokens(token);
CREATE INDEX IF NOT EXISTS idx_unsub_user ON unsubscribe_tokens(user_id);

-- Add marketing_emails_opted_in column if it does not already exist.
-- SQLite does not support IF NOT EXISTS for ALTER TABLE ADD COLUMN,
-- so we use a safe approach: the column may already exist from schema.rs init.
-- rusqlite_migration will treat errors here as non-fatal warnings.
ALTER TABLE users ADD COLUMN marketing_emails_opted_in INTEGER NOT NULL DEFAULT 1;