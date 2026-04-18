REVOKE ALL ON webhook_deliveries FROM reverie_readonly;
REVOKE ALL ON webhooks FROM reverie_readonly;
REVOKE ALL ON ingestion_jobs FROM reverie_readonly;
REVOKE ALL ON api_cache FROM reverie_readonly;

REVOKE ALL ON ingestion_jobs FROM reverie_ingestion;
REVOKE ALL ON api_cache FROM reverie_ingestion;

REVOKE ALL ON webhook_deliveries FROM reverie_app;
REVOKE ALL ON webhooks FROM reverie_app;
REVOKE ALL ON ingestion_jobs FROM reverie_app;
REVOKE ALL ON api_cache FROM reverie_app;

DROP TABLE IF EXISTS webhook_deliveries;
DROP TABLE IF EXISTS webhooks;
DROP TABLE IF EXISTS ingestion_jobs;
DROP TABLE IF EXISTS api_cache;

DROP TYPE IF EXISTS job_status;
