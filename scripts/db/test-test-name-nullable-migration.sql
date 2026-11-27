\set ON_ERROR_STOP on
BEGIN;
-- Isolated schema fixtures, not application data or a production result.
SET LOCAL search_path = pg_temp, pg_catalog;
CREATE TEMP TABLE test_results (id INTEGER PRIMARY KEY, test_name TEXT NOT NULL);
INSERT INTO test_results VALUES (1, ''), (2, '  '), (3, 'ALT');
\ir ../../tests/fixtures/prebaseline/20261121_z_test_name_nullable.sql
INSERT INTO test_results VALUES (4, NULL);
DO $$
BEGIN
    IF (SELECT array_agg(test_name ORDER BY id) FROM test_results)
       IS DISTINCT FROM ARRAY['', '  ', 'ALT', NULL]::text[] THEN
        RAISE EXCEPTION 'Migration changed or invented test names';
    END IF;
END
$$;
ROLLBACK;
\echo Test name migration assertions passed; temporary fixtures rolled back.
