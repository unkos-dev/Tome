-- Tighten metadata_versions.confidence_score to NOT NULL.
--
-- All application paths (ingestion::insert_draft, enrichment::upsert_journal_row,
-- enrichment::orchestrator) bind a non-null f32. The compile-time macro migration
-- in routes/metadata.rs (UNK-167 PR3) overrides the column as `f32` via
-- `confidence_score AS "confidence_score!"`, which assumes this constraint.
-- Mirrors the new_value NOT NULL pattern from PR #159.

ALTER TABLE metadata_versions
    ALTER COLUMN confidence_score SET NOT NULL;
