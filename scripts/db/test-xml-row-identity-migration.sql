\set ON_ERROR_STOP on
BEGIN;
SET LOCAL search_path = pg_temp, pg_catalog;
CREATE TEMP TABLE reactions (id INTEGER PRIMARY KEY, case_id INTEGER NOT NULL);
CREATE TEMP TABLE drug_information (id INTEGER PRIMARY KEY, case_id INTEGER NOT NULL);
INSERT INTO reactions VALUES (1, 1), (2, 2), (3, 1);
INSERT INTO drug_information VALUES (1, 1), (2, 2), (3, 1);
\ir ../../db/migrations/20261121_zzz_xml_row_identity.sql
DO $$
DECLARE table_name TEXT;
BEGIN
    IF EXISTS (SELECT FROM reactions WHERE source_xml_id IS NOT NULL)
       OR EXISTS (SELECT FROM drug_information WHERE source_xml_id IS NOT NULL) THEN
        RAISE EXCEPTION 'Historical XML identities were fabricated';
    END IF;
    FOREACH table_name IN ARRAY ARRAY['reactions', 'drug_information'] LOOP
        -- Same external identity is valid in separate cases/organizations.
        EXECUTE format('UPDATE %I SET source_xml_id = %L WHERE id IN (1,2)', table_name, 'external-id');
        BEGIN
            EXECUTE format('UPDATE %I SET source_xml_id = %L WHERE id = 3', table_name, 'external-id');
            RAISE EXCEPTION 'Duplicate identity accepted within a case';
        EXCEPTION WHEN unique_violation THEN NULL;
        END;
        BEGIN
            EXECUTE format('UPDATE %I SET source_xml_id = %L WHERE id = 3', table_name, ' ');
            RAISE EXCEPTION 'Empty external identity accepted';
        EXCEPTION WHEN check_violation THEN NULL;
        END;
    END LOOP;
END
$$;
ROLLBACK;
\echo XML row identity migration assertions passed; isolated temporary fixtures rolled back.
