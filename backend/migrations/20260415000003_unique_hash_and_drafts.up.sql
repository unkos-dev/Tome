-- Enforce file_hash uniqueness on manifestations. Also serves as the index
-- for the duplicate check in the ingestion orchestrator.
ALTER TABLE manifestations ADD CONSTRAINT manifestations_file_hash_unique UNIQUE (file_hash);

-- One draft per source per field per manifestation. Prevents duplicate rows
-- from re-ingestion or retry. Step 7 enrichment creates separate rows with
-- different source values (opf, openlibrary, googlebooks), which is permitted.
ALTER TABLE metadata_versions
    ADD CONSTRAINT metadata_versions_manifestation_source_field_unique
    UNIQUE (manifestation_id, source, field_name);
