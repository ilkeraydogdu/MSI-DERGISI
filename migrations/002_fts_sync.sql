-- Migration 002: Keep contents_fts in sync with contents via triggers, then backfill.
-- The FTS5 virtual table itself is created by schema::init_schema(); this migration
-- makes it authoritative for full-text search so repo.search_contents can use MATCH.

CREATE TRIGGER IF NOT EXISTS trg_contents_fts_ai AFTER INSERT ON contents BEGIN
    INSERT INTO contents_fts (title_tr, title_en, summary_tr, summary_en, body_tr, body_en, content_id)
    VALUES (NEW.title_tr, NEW.title_en, NEW.summary_tr, NEW.summary_en, NEW.body_tr, NEW.body_en, NEW.id);
END;

CREATE TRIGGER IF NOT EXISTS trg_contents_fts_ad AFTER DELETE ON contents BEGIN
    DELETE FROM contents_fts WHERE content_id = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_contents_fts_au AFTER UPDATE OF title_tr, title_en, summary_tr, summary_en, body_tr, body_en ON contents BEGIN
    DELETE FROM contents_fts WHERE content_id = OLD.id;
    INSERT INTO contents_fts (title_tr, title_en, summary_tr, summary_en, body_tr, body_en, content_id)
    VALUES (NEW.title_tr, NEW.title_en, NEW.summary_tr, NEW.summary_en, NEW.body_tr, NEW.body_en, NEW.id);
END;

-- One-time backfill for rows that existed before the triggers were installed.
INSERT INTO contents_fts (title_tr, title_en, summary_tr, summary_en, body_tr, body_en, content_id)
SELECT c.title_tr, c.title_en, c.summary_tr, c.summary_en, c.body_tr, c.body_en, c.id
FROM contents c
WHERE NOT EXISTS (SELECT 1 FROM contents_fts f WHERE f.content_id = c.id);