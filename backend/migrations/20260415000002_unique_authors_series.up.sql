-- Add unique constraints on authors.name and series.name to prevent duplicates
-- during concurrent ingestion. The find_or_create pattern in work.rs uses
-- INSERT ... ON CONFLICT to safely deduplicate.

ALTER TABLE authors ADD CONSTRAINT authors_name_unique UNIQUE (name);
ALTER TABLE series ADD CONSTRAINT series_name_unique UNIQUE (name);
