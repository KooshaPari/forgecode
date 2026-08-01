-- Rollback hidden flag migration (best-effort, forward-only policy)
-- SQLite rollback by DROP COLUMN is limited in older versions.
-- To fully revert, recreate table from backup.

-- ALTER TABLE conversations DROP COLUMN hidden;
