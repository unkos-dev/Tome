-- Tighten metadata_versions.confidence_score to NOT NULL.
--
-- All application paths (ingestion::insert_draft, enrichment::upsert_journal_row,
-- enrichment::orchestrator) bind a non-null f32. The compile-time macro migration
-- in routes/metadata.rs (UNK-167 PR3) overrides the column as `f32` via
-- `confidence_score AS "confidence_score!"`, which assumes this constraint.
-- Mirrors the new_value NOT NULL pattern from PR #159.
--
-- Defensive backfill: the column was originally created nullable, so legacy
-- rows on self-hosted deployments may carry NULLs even though current writers
-- never produce them. Coerce to 0.0 (lowest confidence) before the ALTER so
-- the SET NOT NULL cannot abort at apply time on existing data.
UPDATE metadata_versions
   SET confidence_score = 0.0
 WHERE confidence_score IS NULL;

ALTER TABLE metadata_versions
    ALTER COLUMN confidence_score SET NOT NULL;
