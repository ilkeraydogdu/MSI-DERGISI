-- Migration 005: TOTP Two-Factor Authentication Support

ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_secret TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_backup_codes_json TEXT NOT NULL DEFAULT '[]';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_users_totp_backup_codes_json'
    ) THEN
        ALTER TABLE users ADD CONSTRAINT chk_users_totp_backup_codes_json CHECK (totp_backup_codes_json::jsonb IS NOT NULL);
    END IF;
END $$;
