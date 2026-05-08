-- Roll back tower-sessions PostgresStore backing schema.
--
-- Drops the table, index, and schema in reverse order. Sessions live
-- on this storage layer only — rolling back the migration logs every
-- user out, which is acceptable since the rollback path implies a
-- redeploy back to MemoryStore (which had the same eviction property).

DROP INDEX IF EXISTS tower_sessions.session_expiry_date_idx;
DROP TABLE IF EXISTS tower_sessions.session;
DROP SCHEMA IF EXISTS tower_sessions;
