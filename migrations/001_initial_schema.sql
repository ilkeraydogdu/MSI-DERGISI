-- Migration 001: Initial schema (extracted from monolithic schema.rs)
-- This migration is idempotent via CREATE TABLE IF NOT EXISTS / CREATE INDEX IF NOT EXISTS.
-- Future migrations should use ALTER TABLE or new CREATE statements without IF NOT EXISTS guards.

-- Tables, indexes, FTS5 virtual table, and functional indexes are already created by schema::init_schema().
-- This migration serves as the baseline version marker for rusqlite_migration.
-- No DDL changes needed here; existing schema.rs handles initial creation.

SELECT 1;