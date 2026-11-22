-- Migration 006: Approval Workflow System

ALTER TABLE contents ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'published';

CREATE TABLE IF NOT EXISTS approval_requests (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    content_id BIGINT NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    requested_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    requested_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    reviewed_by BIGINT REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at TEXT,
    review_note TEXT
);

CREATE INDEX IF NOT EXISTS idx_approval_status ON approval_requests(status);
CREATE INDEX IF NOT EXISTS idx_approval_content ON approval_requests(content_id);
CREATE INDEX IF NOT EXISTS idx_approval_requested_by ON approval_requests(requested_by);
