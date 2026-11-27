\set ON_ERROR_STOP on
BEGIN;
SET LOCAL search_path = pg_temp, pg_catalog;
-- Isolated schema fixtures; never application or clinical data.
CREATE TEMP TABLE xml_import_history (id INTEGER PRIMARY KEY, status TEXT NOT NULL, error_message TEXT);
INSERT INTO xml_import_history VALUES (1, 'error', 'PRIVATE_LEGACY_DIAGNOSTIC'), (2, 'success', NULL);
\ir ../../tests/fixtures/prebaseline/20261121_zz_xml_import_public_error.sql
DO $$
BEGIN
    IF (SELECT error_message FROM xml_import_history WHERE id=1) IS DISTINCT FROM 'PRIVATE_LEGACY_DIAGNOSTIC'
       OR EXISTS (SELECT FROM xml_import_history WHERE public_error IS NOT NULL) THEN
        RAISE EXCEPTION 'Migration changed or classified historical data';
    END IF;
    UPDATE xml_import_history SET public_error='{"httpStatus":400,"error":{"message":"INVALID_REQUEST"}}' WHERE id=1;
    BEGIN
        UPDATE xml_import_history SET public_error='"raw diagnostic"' WHERE id=1;
        RAISE EXCEPTION 'Raw-string public error accepted';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    BEGIN
        UPDATE xml_import_history SET public_error='{}' WHERE id=2;
        RAISE EXCEPTION 'Successful history acquired an error';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
ROLLBACK;
\echo Public import error migration assertions passed; temporary fixtures rolled back.
