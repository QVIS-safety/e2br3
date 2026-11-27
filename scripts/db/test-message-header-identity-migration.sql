\set ON_ERROR_STOP on
BEGIN;
-- Isolated schema fixtures, not application data or a production result.
SET LOCAL search_path = pg_temp, pg_catalog;
CREATE TEMP TABLE message_headers (
    id INTEGER PRIMARY KEY,
    case_id INTEGER NOT NULL,
    message_number VARCHAR(100) UNIQUE NOT NULL,
    message_date VARCHAR(14) NOT NULL,
    CONSTRAINT unique_message_per_case UNIQUE (case_id)
);
INSERT INTO message_headers VALUES (1, 1, 'EXTERNAL-ID', '20240102030405');
\ir ../../tests/fixtures/prebaseline/20261121_xml_message_header_identity.sql
INSERT INTO message_headers VALUES (2, 2, 'EXTERNAL-ID', '20240229123059.1234+0900');
DO $$
BEGIN
    IF (SELECT message_date FROM message_headers WHERE id = 1) IS DISTINCT FROM '20240102030405'
       OR (SELECT message_date FROM message_headers WHERE id = 2) IS DISTINCT FROM '20240229123059.1234+0900' THEN
        RAISE EXCEPTION 'Message timestamp changed';
    END IF;
    BEGIN
        INSERT INTO message_headers VALUES (3, 2, 'OTHER-ID', '2024');
        RAISE EXCEPTION 'Multiple headers for one case accepted';
    EXCEPTION WHEN unique_violation THEN NULL;
    END;
END
$$;
ROLLBACK;
\echo Message header migration assertions passed; temporary fixtures rolled back.
