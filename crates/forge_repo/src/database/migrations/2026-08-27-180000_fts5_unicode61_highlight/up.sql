-- Add porter unicode61 tokenizer with remove_diacritics=1 to conversations_fts.
--
-- The existing FTS5 setup (migration 2026-06-26-000400) uses just `porter`
-- which is English-only and does NOT fold diacritics. This migration upgrades
-- the tokenizer to `porter unicode61 "remove_diacritics 1"`, which:
--   - Stems English text (porter) AND tokenizes non-English text per Unicode
--     rules (unicode61)
--   - Folds diacritics so cafe, cafe-accent, and cafe-diacritic all match
--
-- Matches the target design in docs/requirements.md:52-58 (M3 spec).
-- PRESERVES the contentful-mode pattern from migration 2026-06-26-000400 so
-- compressed rows (is_compressed=1, context=NULL, context_zstd BLOB) continue
-- to be indexed by application-side refresh_fts_index after zstd decompression.
-- Does NOT reintroduce triggers (migration 2026-06-26-000000 dropped them to
-- fix WAL-lock contention; refresh_fts_index at startup remains the population
-- path).
--
-- PREREQUISITES:
--   - SQLite >= 3.44 (the bundled libsqlite3-sys in this crate is 3.51.x)
--   - Application must call refresh_fts_index after this migration completes,
--     either:
--     (a) explicitly on daemon startup (recommended; minimal first-search cost)
--     (b) implicitly via the first :search or /fts-optimize invocation
--
-- INDEX COST:
--   FTS5 has no ALTER TABLE for the tokenizer, so we drop + recreate the
--   virtual table. The existing index shadow content is lost; rebuild via
--   refresh_fts_index in Rust land.
--
-- ROW FORMAT PRESERVED:
--   conversations_fts(title, content, cwd) - same 3 indexed columns as before.
--   Column order matters: search_conversations JOINs on rowid (same),
--   snippet(col_idx, ...) uses content at column index 1 (same).

-- Drop the old porter-only FTS5 table.
DROP TABLE IF EXISTS conversations_fts;

-- Recreate with porter + unicode61 + remove_diacritics=1.
-- porter is the English stemmer; unicode61 handles non-English Unicode text;
-- remove_diacritics=1 folds accents so users searching "cafe" find "cafe-accent".
CREATE VIRTUAL TABLE conversations_fts USING fts5(
    title,
    content,
    cwd,
    tokenize = 'porter unicode61 "remove_diacritics 1"'
);

-- Table is created EMPTY. Application-side refresh_fts_index will populate it
-- with decompressed context from both compressed and uncompressed rows
-- (mirrors the population pattern set by migration 2026-06-26-000400).
--
-- refresh_fts_index lives in crates/forge_repo/src/conversation/conversation_repo.rs
-- and must be invoked once after this migration runs. Recommended call sites:
--   1. DatabasePool::build_pool after run_pending_migrations (crates/forge_repo/src/database/pool.rs:395)
--   2. forge_dbd server startup after open_writer_connection (crates/forge_dbd/src/server.rs:514)
