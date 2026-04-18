REVOKE ALL ON manifestation_tags FROM reverie_readonly;
REVOKE ALL ON tags FROM reverie_readonly;
REVOKE ALL ON metadata_versions FROM reverie_readonly;
REVOKE ALL ON omnibus_contents FROM reverie_readonly;
REVOKE ALL ON series_works FROM reverie_readonly;
REVOKE ALL ON series FROM reverie_readonly;

REVOKE ALL ON manifestation_tags FROM reverie_ingestion;
REVOKE ALL ON tags FROM reverie_ingestion;
REVOKE ALL ON metadata_versions FROM reverie_ingestion;
REVOKE ALL ON omnibus_contents FROM reverie_ingestion;
REVOKE ALL ON series_works FROM reverie_ingestion;
REVOKE ALL ON series FROM reverie_ingestion;

REVOKE ALL ON manifestation_tags FROM reverie_app;
REVOKE ALL ON tags FROM reverie_app;
REVOKE ALL ON metadata_versions FROM reverie_app;
REVOKE ALL ON omnibus_contents FROM reverie_app;
REVOKE ALL ON series_works FROM reverie_app;
REVOKE ALL ON series FROM reverie_app;

DROP TABLE IF EXISTS manifestation_tags;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS metadata_versions;
DROP TABLE IF EXISTS omnibus_contents;
DROP TABLE IF EXISTS series_works;
DROP TABLE IF EXISTS series;
