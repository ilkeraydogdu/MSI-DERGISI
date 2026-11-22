-- Migration 017: MSI Dergisi Editorial Studio Schema Extensions
-- Mirrors migrations/017_editorial_studio.sql for PostgreSQL 16.

ALTER TABLE contents ADD COLUMN IF NOT EXISTS category_id INTEGER;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS primary_category TEXT NOT NULL DEFAULT '';
ALTER TABLE contents ADD COLUMN IF NOT EXISTS category_ids_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE contents ADD COLUMN IF NOT EXISTS blocks_tr_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE contents ADD COLUMN IF NOT EXISTS blocks_en_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE contents ADD COLUMN IF NOT EXISTS seo_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE contents ADD COLUMN IF NOT EXISTS tags_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE contents ADD COLUMN IF NOT EXISTS scheduled_at TEXT;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS issue_page_range TEXT;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS publication_channel TEXT NOT NULL DEFAULT 'web';
ALTER TABLE contents ADD COLUMN IF NOT EXISTS cover_width INTEGER;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS cover_height INTEGER;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS original_url TEXT;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS social_shared BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS created_by BIGINT;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS updated_at TEXT;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_contents_category_ids_json') THEN
        ALTER TABLE contents ADD CONSTRAINT chk_contents_category_ids_json CHECK (category_ids_json::jsonb IS NOT NULL);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_contents_blocks_tr_json') THEN
        ALTER TABLE contents ADD CONSTRAINT chk_contents_blocks_tr_json CHECK (blocks_tr_json::jsonb IS NOT NULL);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_contents_blocks_en_json') THEN
        ALTER TABLE contents ADD CONSTRAINT chk_contents_blocks_en_json CHECK (blocks_en_json::jsonb IS NOT NULL);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_contents_seo_json') THEN
        ALTER TABLE contents ADD CONSTRAINT chk_contents_seo_json CHECK (seo_json::jsonb IS NOT NULL);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_contents_tags_json') THEN
        ALTER TABLE contents ADD CONSTRAINT chk_contents_tags_json CHECK (tags_json::jsonb IS NOT NULL);
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_contents_status_pub ON contents(status, published_at DESC);
CREATE INDEX IF NOT EXISTS idx_contents_scheduled ON contents(scheduled_at);
CREATE INDEX IF NOT EXISTS idx_contents_category_id ON contents(category_id);

ALTER TABLE media_library ADD COLUMN IF NOT EXISTS thumb_url TEXT;
ALTER TABLE media_library ADD COLUMN IF NOT EXISTS width INTEGER NOT NULL DEFAULT 0;
ALTER TABLE media_library ADD COLUMN IF NOT EXISTS height INTEGER NOT NULL DEFAULT 0;
ALTER TABLE media_library ADD COLUMN IF NOT EXISTS watermarked BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS editorial_research_sources (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL DEFAULT 'rss',
    category TEXT NOT NULL DEFAULT 'Savunma',
    scope TEXT NOT NULL DEFAULT 'tr',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TEXT NOT NULL
);

INSERT INTO editorial_research_sources (name, url, kind, category, scope, enabled, created_at) VALUES
('SavunmaSanayiST', 'https://www.savunmasanayist.com/feed/', 'rss', 'Savunma', 'tr', TRUE, '2026-11-22T00:00:00Z'),
('Anadolu Ajansı', 'https://www.aa.com.tr/tr/rss/default?cat=guncel', 'rss', 'Ajans', 'tr', TRUE, '2026-11-22T00:00:00Z'),
('TRT Haber Son Dakika', 'https://www.trthaber.com/sondakika_articles.rss', 'rss', 'Gündem', 'tr', TRUE, '2026-11-22T00:00:00Z'),
('Defense News', 'https://www.defensenews.com/arc/outboundfeeds/rss/?outputType=xml', 'rss', 'Savunma', 'global', TRUE, '2026-11-22T00:00:00Z'),
('Naval News', 'https://www.navalnews.com/feed/', 'rss', 'Denizcilik', 'global', TRUE, '2026-11-22T00:00:00Z'),
('Breaking Defense', 'https://breakingdefense.com/feed/', 'rss', 'Savunma', 'global', TRUE, '2026-11-22T00:00:00Z')
ON CONFLICT (url) DO NOTHING;
