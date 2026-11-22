-- Migration 014: Translation Approval Workflow & Auto-Translate Support
-- Adds approval lifecycle columns to the translations table so UI string
-- edits can go through a review step before going live, mirroring the
-- existing content EN approval workflow (en_status).

-- Status values: 'approved' (default, live), 'draft' (pending review),
-- 'auto_translated' (machine-generated, awaiting human approval)

ALTER TABLE translations ADD COLUMN status TEXT NOT NULL DEFAULT 'approved';
ALTER TABLE translations ADD COLUMN approved_by INTEGER REFERENCES users(id);
ALTER TABLE translations ADD COLUMN approved_at TEXT;
ALTER TABLE translations ADD COLUMN auto_translated_at TEXT;
ALTER TABLE translations ADD COLUMN source_lang TEXT DEFAULT 'tr';

-- Index for filtering by status (admin translation review queue)
CREATE INDEX IF NOT EXISTS idx_translations_status ON translations(status);

-- Seed default status for all existing translations as 'approved'
-- (they were already live before this migration)
UPDATE translations SET status = 'approved' WHERE status IS NULL OR status = '';