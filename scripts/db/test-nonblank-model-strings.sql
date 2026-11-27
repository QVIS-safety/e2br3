\set ON_ERROR_STOP on
BEGIN;
SET LOCAL lock_timeout = '5s';
CREATE TABLE public.nonblank_model_probe (
    optional_value text,
    required_value varchar(20) NOT NULL,
    labels text[]
);
INSERT INTO public.nonblank_model_probe VALUES ('', 'present', ARRAY['present']);
\ir ../../tests/fixtures/prebaseline/20261123_nonblank_model_strings.sql
DO $$
DECLARE
    rejected boolean;
    candidate text;
BEGIN
    IF EXISTS (SELECT 1 FROM nonblank_model_probe WHERE optional_value IS NOT NULL) THEN
        RAISE EXCEPTION 'existing blank was not normalized to NULL';
    END IF;
    FOREACH candidate IN ARRAY ARRAY['', ' ', E'\t\n'] LOOP
        rejected := false;
        BEGIN
            INSERT INTO nonblank_model_probe VALUES (candidate, 'present', NULL);
        EXCEPTION WHEN check_violation THEN rejected := true;
        END;
        IF NOT rejected THEN RAISE EXCEPTION 'optional blank was accepted'; END IF;
        rejected := false;
        BEGIN
            INSERT INTO nonblank_model_probe VALUES (NULL, candidate, NULL);
        EXCEPTION WHEN check_violation THEN rejected := true;
        END;
        IF NOT rejected THEN RAISE EXCEPTION 'required blank was accepted'; END IF;
        rejected := false;
        BEGIN
            UPDATE nonblank_model_probe SET labels = ARRAY['valid', candidate];
        EXCEPTION WHEN check_violation THEN rejected := true;
        END;
        IF NOT rejected THEN RAISE EXCEPTION 'blank array entry was accepted'; END IF;
    END LOOP;
    INSERT INTO nonblank_model_probe VALUES (NULL, 'valid', ARRAY[]::text[]);
    UPDATE nonblank_model_probe SET optional_value = ' retained ';
    UPDATE nonblank_model_probe SET optional_value = NULL;
    BEGIN
        INSERT INTO nonblank_model_probe VALUES (NULL, NULL, NULL);
        RAISE EXCEPTION 'required NULL was accepted';
    EXCEPTION WHEN not_null_violation THEN NULL;
    END;
    IF EXISTS (
        SELECT 1 FROM pg_attribute a
        JOIN pg_class c ON c.oid = a.attrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_type t ON t.oid = a.atttypid
        LEFT JOIN pg_type element ON element.oid = t.typelem
        WHERE n.nspname = 'public' AND c.relkind IN ('r','p')
          AND a.attnum > 0 AND NOT a.attisdropped
          AND (t.typcategory = 'S' OR (t.typcategory = 'A' AND element.typcategory = 'S'))
          AND NOT EXISTS (SELECT 1 FROM pg_constraint x WHERE x.conrelid=c.oid
              AND x.conname='model_nonblank_' || md5(a.attname) AND x.convalidated)
    ) THEN RAISE EXCEPTION 'a string model field lacks a validated nonblank constraint'; END IF;
END;
$$;
SELECT count(*) AS validated_model_nonblank_constraints
FROM pg_constraint WHERE conname LIKE 'model_nonblank_%' AND convalidated;
ROLLBACK;
