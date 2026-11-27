-- Historical test fixture only; never loaded by database initialization.
-- Missing optional values are SQL NULL, never an empty-string sentinel.
ALTER TABLE other_case_identifiers ALTER COLUMN source_of_identifier DROP NOT NULL;
ALTER TABLE other_case_identifiers ALTER COLUMN case_identifier DROP NOT NULL;
ALTER TABLE linked_report_numbers ALTER COLUMN linked_report_number DROP NOT NULL;
ALTER TABLE mfds_product_substances
    ALTER COLUMN material_sequence DROP NOT NULL,
    ALTER COLUMN material_sequence DROP DEFAULT,
    ALTER COLUMN total_amount_sequence DROP NOT NULL,
    ALTER COLUMN total_amount_sequence DROP DEFAULT,
    DROP CONSTRAINT mfds_product_substances_unique,
    ADD CONSTRAINT mfds_product_substances_unique UNIQUE NULLS NOT DISTINCT
        (item_seq, substance_code, material_sequence, total_amount_sequence, version);

CREATE OR REPLACE FUNCTION model_text_array_has_no_blank(values_to_check text[])
RETURNS boolean LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT NOT EXISTS (
        SELECT 1 FROM unnest(values_to_check) AS item
        WHERE item ~ '^[[:space:]]*$'
    );
$$;

DO $$
DECLARE
    field record;
    constraint_name text;
    predicate text;
    has_blank boolean;
BEGIN
    PERFORM set_config('app.current_user_id', '00000000-0000-0000-0000-000000000001', true);
    PERFORM set_config('app.current_organization_id', '00000000-0000-0000-0000-000000000001', true);
    PERFORM set_config('app.current_user_role', 'system_admin', true);

    FOR field IN
        SELECT c.oid AS table_oid, c.relname AS table_name, a.attname AS column_name,
               a.attnotnull AS required, t.typcategory = 'A' AS is_array
        FROM pg_attribute a
        JOIN pg_class c ON c.oid = a.attrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_type t ON t.oid = a.atttypid
        LEFT JOIN pg_type element ON element.oid = t.typelem
        WHERE n.nspname = 'public' AND c.relkind IN ('r', 'p')
          AND a.attnum > 0 AND NOT a.attisdropped
          AND (t.typcategory = 'S' OR (t.typcategory = 'A' AND element.typcategory = 'S'))
        ORDER BY c.relname, a.attnum
    LOOP
        constraint_name := 'model_nonblank_' || md5(field.column_name);
        IF field.is_array THEN
            predicate := format('model_text_array_has_no_blank(%I::text[])', field.column_name);
            -- Array entries have identities/positions; do not silently discard or replace them.
        ELSE
            EXECUTE format('SELECT EXISTS (SELECT 1 FROM public.%I WHERE %I ~ %L)',
                field.table_name, field.column_name, '^[[:space:]]*$') INTO has_blank;
            IF has_blank AND field.required THEN
                RAISE EXCEPTION 'Required field %.% contains blank values; repair the source data before migration',
                    field.table_name, field.column_name;
            END IF;
            IF has_blank THEN
                EXECUTE format('UPDATE public.%I SET %I = NULL WHERE %I ~ %L',
                    field.table_name, field.column_name, field.column_name, '^[[:space:]]*$');
            END IF;
            predicate := format('%I !~ %L', field.column_name, '^[[:space:]]*$');
        END IF;
        IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = field.table_oid AND conname = constraint_name) THEN
            EXECUTE format('ALTER TABLE public.%I ADD CONSTRAINT %I CHECK (%s)',
                field.table_name, constraint_name, predicate);
        END IF;
    END LOOP;
END;
$$;
