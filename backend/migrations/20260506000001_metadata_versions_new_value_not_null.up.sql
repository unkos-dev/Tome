-- All `metadata_versions` writers (services::metadata::draft::insert_draft,
-- services::enrichment::orchestrator::upsert_journal_row, models::work
-- stub_with_user_override fixture, routes::metadata test fixtures) bind
-- `new_value` from a non-Option `serde_json::Value`. Tighten the schema so
-- compile-time sqlx queries no longer need `AS "new_value!"` overrides on
-- nullable JSONB to express the existing application invariant.
ALTER TABLE metadata_versions
    ALTER COLUMN new_value SET NOT NULL;
