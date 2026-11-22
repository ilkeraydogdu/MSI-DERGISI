-- Migration 017: MSI Dergisi Editorial Studio Schema Extensions
-- Adds structured block editor, SEO/Discover metadata, taxonomy/scheduling/attribution
-- columns onto `contents` and `media_library`, plus `editorial_research_sources`.

ALTER TABLE contents ADD COLUMN category_id INTEGER;
ALTER TABLE contents ADD COLUMN primary_category TEXT NOT NULL DEFAULT '';
ALTER TABLE contents ADD COLUMN category_ids_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE contents ADD COLUMN blocks_tr_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE contents ADD COLUMN blocks_en_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE contents ADD COLUMN seo_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE contents ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE contents ADD COLUMN scheduled_at TEXT;
ALTER TABLE contents ADD COLUMN issue_page_range TEXT;
ALTER TABLE contents ADD COLUMN publication_channel TEXT NOT NULL DEFAULT 'web';
ALTER TABLE contents ADD COLUMN cover_width INTEGER;
ALTER TABLE contents ADD COLUMN cover_height INTEGER;
ALTER TABLE contents ADD COLUMN original_url TEXT;
ALTER TABLE contents ADD COLUMN social_shared INTEGER NOT NULL DEFAULT 0;
ALTER TABLE contents ADD COLUMN created_by INTEGER;
ALTER TABLE contents ADD COLUMN updated_at TEXT;

CREATE INDEX IF NOT EXISTS idx_contents_status_pub ON contents(status, published_at DESC);
CREATE INDEX IF NOT EXISTS idx_contents_scheduled ON contents(scheduled_at);
CREATE INDEX IF NOT EXISTS idx_contents_category_id ON contents(category_id);

ALTER TABLE media_library ADD COLUMN thumb_url TEXT;
ALTER TABLE media_library ADD COLUMN width INTEGER NOT NULL DEFAULT 0;
ALTER TABLE media_library ADD COLUMN height INTEGER NOT NULL DEFAULT 0;
ALTER TABLE media_library ADD COLUMN watermarked INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS editorial_research_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    url TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL DEFAULT 'rss',
    category TEXT NOT NULL DEFAULT 'Savunma',
    scope TEXT NOT NULL DEFAULT 'tr',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

INSERT OR IGNORE INTO editorial_research_sources (name, url, kind, category, scope, enabled, created_at) VALUES
('SavunmaSanayiST', 'https://www.savunmasanayist.com/feed/', 'rss', 'Savunma', 'tr', 1, '2026-11-22T00:00:00Z'),
('Anadolu Ajansı', 'https://www.aa.com.tr/tr/rss/default?cat=guncel', 'rss', 'Ajans', 'tr', 1, '2026-11-22T00:00:00Z'),
('TRT Haber Son Dakika', 'https://www.trthaber.com/sondakika_articles.rss', 'rss', 'Gündem', 'tr', 1, '2026-11-22T00:00:00Z'),
('Defense News', 'https://www.defensenews.com/arc/outboundfeeds/rss/?outputType=xml', 'rss', 'Savunma', 'global', 1, '2026-11-22T00:00:00Z'),
('Naval News', 'https://www.navalnews.com/feed/', 'rss', 'Denizcilik', 'global', 1, '2026-11-22T00:00:00Z'),
('Breaking Defense', 'https://breakingdefense.com/feed/', 'rss', 'Savunma', 'global', 1, '2026-11-22T00:00:00Z');
