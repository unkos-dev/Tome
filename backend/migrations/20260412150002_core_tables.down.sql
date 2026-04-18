REVOKE ALL ON manifestations FROM reverie_readonly;
REVOKE ALL ON work_authors FROM reverie_readonly;
REVOKE ALL ON authors FROM reverie_readonly;
REVOKE ALL ON works FROM reverie_readonly;
REVOKE ALL ON users FROM reverie_readonly;

REVOKE ALL ON manifestations FROM reverie_ingestion;
REVOKE ALL ON work_authors FROM reverie_ingestion;
REVOKE ALL ON authors FROM reverie_ingestion;
REVOKE ALL ON works FROM reverie_ingestion;

REVOKE ALL ON manifestations FROM reverie_app;
REVOKE ALL ON work_authors FROM reverie_app;
REVOKE ALL ON authors FROM reverie_app;
REVOKE ALL ON works FROM reverie_app;
REVOKE ALL ON users FROM reverie_app;

DROP TABLE IF EXISTS manifestations;
DROP TABLE IF EXISTS work_authors;
DROP TABLE IF EXISTS authors;
DROP TABLE IF EXISTS works;
DROP TABLE IF EXISTS users;
