-- tower-sessions PostgresStore backing schema.
--
-- Replaces the in-memory MemoryStore with a Postgres-backed store so
-- sessions survive container restarts. Schema and table names match
-- tower-sessions-sqlx-store@0.15.0 defaults (see PostgresStore::new),
-- so no with_schema_name/with_table_name override is needed at
-- construction time.
--
-- Schema choice: dedicated `tower_sessions` schema isolates the framework
-- table from application tables and keeps the public schema clean. The
-- crate's own migrate() helper uses the same convention.
--
-- No RLS on this table: SessionStore::load runs before any auth context
-- exists (the cookie's session id is the bootstrap that resolves the
-- user), so RLS-gating the session lookup is chicken-and-egg. Access
-- control is at the role-grant boundary below — reverie_app gets DML;
-- reverie_readonly gets SELECT only.

CREATE SCHEMA IF NOT EXISTS tower_sessions;

CREATE TABLE IF NOT EXISTS tower_sessions.session (
    id text PRIMARY KEY NOT NULL,
    data bytea NOT NULL,
    expiry_date timestamptz NOT NULL
);

-- Supports the ExpiredDeletion sweep
-- (`DELETE … WHERE expiry_date < now()`).  The library doesn't ship an
-- index of its own; without it, the sweep is a sequential scan.
CREATE INDEX IF NOT EXISTS session_expiry_date_idx
    ON tower_sessions.session (expiry_date);

-- Runtime role grants. The schema owner (reverie) runs migrations and
-- keeps full ownership; reverie_app gets the DML it needs and
-- reverie_readonly can observe session counts for diagnostics.
GRANT USAGE ON SCHEMA tower_sessions TO reverie_app, reverie_readonly;
GRANT SELECT, INSERT, UPDATE, DELETE ON tower_sessions.session TO reverie_app;
GRANT SELECT ON tower_sessions.session TO reverie_readonly;
