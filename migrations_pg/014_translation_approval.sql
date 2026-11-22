-- Migration 014: Translation Approval Workflow & Auto-Translate Support

ALTER TABLE translations ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'approved';
ALTER TABLE translations ADD COLUMN IF NOT EXISTS approved_by BIGINT REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE translations ADD COLUMN IF NOT EXISTS approved_at TEXT;
ALTER TABLE translations ADD COLUMN IF NOT EXISTS auto_translated_at TEXT;
ALTER TABLE translations ADD COLUMN IF NOT EXISTS source_lang TEXT DEFAULT 'tr';

CREATE INDEX IF NOT EXISTS idx_translations_status ON translations(status);

UPDATE translations SET status = 'approved' WHERE status IS NULL OR status = '';
