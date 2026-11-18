-- Phase 5.1: TOTP Two-Factor Authentication Support
-- Adds nullable totp_secret and backup recovery codes to users table for optional 2FA enrollment

ALTER TABLE users ADD COLUMN totp_secret TEXT;
ALTER TABLE users ADD COLUMN totp_backup_codes_json TEXT NOT NULL DEFAULT '[]';
