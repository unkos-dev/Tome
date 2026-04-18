REVOKE ALL ON shelf_items FROM reverie_readonly;
REVOKE ALL ON shelves FROM reverie_readonly;

REVOKE ALL ON device_tokens FROM reverie_app;
REVOKE ALL ON shelf_items FROM reverie_app;
REVOKE ALL ON shelves FROM reverie_app;

DROP TABLE IF EXISTS device_tokens;
DROP TABLE IF EXISTS shelf_items;
DROP TABLE IF EXISTS shelves;
