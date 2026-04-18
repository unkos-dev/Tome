-- Step 7 follow-up: grant reverie_ingestion SELECT on field_locks.
--
-- The enrichment orchestrator (`services::enrichment::orchestrator::run_once`)
-- calls `field_lock::is_locked_tx` on every incoming observation. That path
-- is exercised by the Phase D integration tests against the background
-- pipeline role (reverie_ingestion), and by any future code that runs the
-- orchestrator from the ingestion pool. Without this grant, the call fails
-- with `permission denied for table field_locks`.
--
-- Write access to field_locks remains restricted to reverie_app (the web
-- application owns lock/unlock, not the pipeline).

GRANT SELECT ON field_locks TO reverie_ingestion;
