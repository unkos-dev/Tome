-- Database role provisioning for Reverie.
-- This script runs once when the PostgreSQL container is first created
-- (empty pgdata volume). It creates the application roles that sqlx
-- migrations will grant privileges to.
--
-- Role architecture:
--   reverie           — schema owner (created by POSTGRES_USER). Runs migrations.
--                    Bypasses RLS. Never used by the application at runtime.
--   reverie_app       — web application service account. RLS enforced (user-scoped).
--   reverie_ingestion — background pipeline service account. Has own permissive
--                    RLS policy on manifestations. Scoped to pipeline tables.
--   reverie_readonly  — debugging and reporting. SELECT only. RLS enforced.

-- Web application service account
CREATE ROLE reverie_app WITH LOGIN PASSWORD 'reverie_app';
GRANT CONNECT ON DATABASE reverie_dev TO reverie_app;

-- Background ingestion pipeline service account
CREATE ROLE reverie_ingestion WITH LOGIN PASSWORD 'reverie_ingestion';
GRANT CONNECT ON DATABASE reverie_dev TO reverie_ingestion;

-- Read-only account for debugging and reporting
CREATE ROLE reverie_readonly WITH LOGIN PASSWORD 'reverie_readonly';
GRANT CONNECT ON DATABASE reverie_dev TO reverie_readonly;
