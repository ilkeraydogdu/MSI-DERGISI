-- Migration 002: PostgreSQL Full-Text Search (tsvector + GIN + unaccent trigger)
-- Replaces SQLite FTS5 contents_fts virtual table and 3 sync triggers.

CREATE EXTENSION IF NOT EXISTS unaccent;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

ALTER TABLE contents ADD COLUMN IF NOT EXISTS search_vector TSVECTOR;

CREATE INDEX IF NOT EXISTS idx_contents_search ON contents USING GIN (search_vector);

CREATE OR REPLACE FUNCTION fn_contents_search_vector() RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('pg_catalog.turkish', unaccent(COALESCE(NEW.title_tr, ''))), 'A') ||
        setweight(to_tsvector('pg_catalog.english', unaccent(COALESCE(NEW.title_en, ''))), 'A') ||
        setweight(to_tsvector('pg_catalog.simple',  unaccent(COALESCE(NEW.title_tr, '') || ' ' || COALESCE(NEW.title_en, ''))), 'A') ||
        setweight(to_tsvector('pg_catalog.turkish', unaccent(COALESCE(NEW.summary_tr, ''))), 'B') ||
        setweight(to_tsvector('pg_catalog.english', unaccent(COALESCE(NEW.summary_en, ''))), 'B') ||
        setweight(to_tsvector('pg_catalog.turkish', unaccent(COALESCE(NEW.body_tr, ''))), 'C') ||
        setweight(to_tsvector('pg_catalog.english', unaccent(COALESCE(NEW.body_en, ''))), 'C');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_contents_fts_sync ON contents;
CREATE TRIGGER trg_contents_fts_sync
    BEFORE INSERT OR UPDATE OF title_tr, title_en, summary_tr, summary_en, body_tr, body_en
    ON contents
    FOR EACH ROW
    EXECUTE FUNCTION fn_contents_search_vector();

-- Backfill any pre-existing rows if present
UPDATE contents
SET search_vector =
    setweight(to_tsvector('pg_catalog.turkish', unaccent(COALESCE(title_tr, ''))), 'A') ||
    setweight(to_tsvector('pg_catalog.english', unaccent(COALESCE(title_en, ''))), 'A') ||
    setweight(to_tsvector('pg_catalog.simple',  unaccent(COALESCE(title_tr, '') || ' ' || COALESCE(title_en, ''))), 'A') ||
    setweight(to_tsvector('pg_catalog.turkish', unaccent(COALESCE(summary_tr, ''))), 'B') ||
    setweight(to_tsvector('pg_catalog.english', unaccent(COALESCE(summary_en, ''))), 'B') ||
    setweight(to_tsvector('pg_catalog.turkish', unaccent(COALESCE(body_tr, ''))), 'C') ||
    setweight(to_tsvector('pg_catalog.english', unaccent(COALESCE(body_en, ''))), 'C')
WHERE search_vector IS NULL;
