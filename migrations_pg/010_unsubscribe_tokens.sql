-- Migration 010: KVKK/GDPR Unsubscribe Tokens

CREATE TABLE IF NOT EXISTS unsubscribe_tokens (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"'),
    used BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_unsub_token ON unsubscribe_tokens(token);
CREATE INDEX IF NOT EXISTS idx_unsub_user ON unsubscribe_tokens(user_id);

ALTER TABLE users ADD COLUMN IF NOT EXISTS marketing_emails_opted_in BOOLEAN NOT NULL DEFAULT TRUE;
