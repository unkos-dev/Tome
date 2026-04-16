-- Remove the unused 'invalid' validation_status enum value.
--
-- Quarantined EPUBs never reach the DB (ProcessResult::Failed writes no row),
-- so 'invalid' is unreachable in application code. PostgreSQL cannot DROP an
-- enum value directly; we rebuild the type instead.
--
-- Safe to run with no data, or with data provided no rows hold 'invalid'
-- (which is guaranteed by the application never writing it).

ALTER TYPE validation_status RENAME TO validation_status_old;

CREATE TYPE validation_status AS ENUM ('pending', 'valid', 'repaired', 'degraded');

ALTER TABLE manifestations
    ALTER COLUMN validation_status TYPE validation_status
    USING validation_status::text::validation_status;

DROP TYPE validation_status_old;
