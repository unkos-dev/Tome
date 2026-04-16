-- sqlx:disable-transaction
-- ALTER TYPE ... ADD VALUE cannot run inside a PostgreSQL transaction.

-- Restore 'invalid' to validation_status enum.
ALTER TYPE validation_status ADD VALUE IF NOT EXISTS 'invalid';
