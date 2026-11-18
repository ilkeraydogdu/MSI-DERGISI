-- Phase 4: Approval Workflow System
-- Adds status column to contents and approval_requests table for content review workflow

-- Add status column to contents table (draft, pending_review, approved, published, rejected, archived)
ALTER TABLE contents ADD COLUMN status TEXT NOT NULL DEFAULT 'published';

-- Approval requests table tracks submit/approve/reject lifecycle
CREATE TABLE IF NOT EXISTS approval_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content_id INTEGER NOT NULL,
    requested_by INTEGER NOT NULL,
    requested_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    reviewed_by INTEGER,
    reviewed_at TEXT,
    review_note TEXT,
    FOREIGN KEY (content_id) REFERENCES contents(id),
    FOREIGN KEY (requested_by) REFERENCES users(id),
    FOREIGN KEY (reviewed_by) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_approval_status ON approval_requests(status);
CREATE INDEX IF NOT EXISTS idx_approval_content ON approval_requests(content_id);
CREATE INDEX IF NOT EXISTS idx_approval_requested_by ON approval_requests(requested_by);