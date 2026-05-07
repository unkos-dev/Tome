-- Database role provisioning for Reverie.
-- Runs once when the PostgreSQL container is first created (uninitialized
-- data directory, no `PG_VERSION` file). The postgres entrypoint skips
-- /docker-entrypoint-initdb.d/* on subsequent restarts.
--
-- The script is DB-name-agnostic and password-env-driven so the same
-- script works for dev (POSTGRES_DB=reverie_dev, default trivial passwords)
-- and staging (POSTGRES_DB=reverie, deploy-supplied passwords). The
-- postgres entrypoint connects the init psql session to POSTGRES_DB and
-- exposes container env vars to the psql shell, so:
--   * `current_database()` resolves to whichever DB POSTGRES_DB names.
--   * `\set` with backticks reads runtime env via the shell, falling back
--     to the role name as the dev default.
--
-- Role architecture:
--   reverie           — schema owner (created by POSTGRES_USER). Runs migrations.
--                    Bypasses RLS. Never used by the application at runtime.
--   reverie_app       — web application service account. RLS enforced (user-scoped).
--   reverie_ingestion — background pipeline service account. Has own permissive
--                    RLS policy on manifestations. Scoped to pipeline tables.
--   reverie_readonly  — debugging and reporting. SELECT only. RLS enforced.

\set app_password         `echo "${REVERIE_APP_PASSWORD:-reverie_app}"`
\set ingestion_password   `echo "${REVERIE_INGESTION_PASSWORD:-reverie_ingestion}"`
\set readonly_password    `echo "${REVERIE_READONLY_PASSWORD:-reverie_readonly}"`

-- Web application service account
CREATE ROLE reverie_app WITH LOGIN PASSWORD :'app_password';

-- Background ingestion pipeline service account
CREATE ROLE reverie_ingestion WITH LOGIN PASSWORD :'ingestion_password';

-- Read-only account for debugging and reporting
CREATE ROLE reverie_readonly WITH LOGIN PASSWORD :'readonly_password';

-- CONNECT grants are kept explicit so they remain load-bearing if a
-- future migration ever issues `REVOKE CONNECT ON DATABASE … FROM PUBLIC`
-- (a common hardening step that would otherwise lock the runtime roles
-- out). `current_database()` adapts to whichever DB POSTGRES_DB names.
DO $$
DECLARE
  db text := current_database();
BEGIN
  EXECUTE format('GRANT CONNECT ON DATABASE %I TO reverie_app', db);
  EXECUTE format('GRANT CONNECT ON DATABASE %I TO reverie_ingestion', db);
  EXECUTE format('GRANT CONNECT ON DATABASE %I TO reverie_readonly', db);
END $$;
