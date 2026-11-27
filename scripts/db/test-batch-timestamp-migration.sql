\set ON_ERROR_STOP on
BEGIN;
-- Isolated migration logic fixtures; no application tables are read or written.
SET LOCAL search_path = pg_temp, pg_catalog;
CREATE TEMP TABLE cases (id INTEGER PRIMARY KEY, raw_xml BYTEA);
CREATE TEMP TABLE message_headers (
    id INTEGER PRIMARY KEY, case_id INTEGER,
    batch_transmission_date TIMESTAMPTZ
);
INSERT INTO cases VALUES
    (1, convert_to('<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><creationTime value="20240229123059.1234+0900"/></MCCI_IN200100UV01>', 'UTF8')),
    (2, convert_to('<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><creationTime value="2024"/></MCCI_IN200100UV01>', 'UTF8')),
    (3, convert_to('<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"/>', 'UTF8')),
    (4, NULL);
INSERT INTO message_headers VALUES
    (1, 1, NULL), (2, 2, '2025-01-01 00:00:00+00'), (3, 3, NULL), (4, 4, NULL);
\ir ../../tests/fixtures/prebaseline/20261121_message_header_batch_date_raw.sql
DO $$
BEGIN
    IF (SELECT batch_transmission_date_raw FROM message_headers WHERE id = 1)
        IS DISTINCT FROM '20240229123059.1234+0900' THEN
        RAISE EXCEPTION 'Original precision/offset was not recovered';
    END IF;
    IF (SELECT batch_transmission_date FROM message_headers WHERE id = 2)
        IS DISTINCT FROM '2025-01-01 00:00:00+00'::TIMESTAMPTZ
        OR EXISTS (SELECT FROM message_headers WHERE id IN (2,3,4) AND batch_transmission_date_raw IS NOT NULL) THEN
        RAISE EXCEPTION 'Existing timestamp overwritten or absent value invented';
    END IF;
    BEGIN
        UPDATE message_headers SET batch_transmission_date = '2025-01-01 00:00:00+00' WHERE id = 1;
        RAISE EXCEPTION 'Conflicting timestamp representations accepted';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
ROLLBACK;
\echo Batch timestamp migration assertions passed; temporary fixtures rolled back.
