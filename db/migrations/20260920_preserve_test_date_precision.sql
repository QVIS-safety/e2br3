-- F.r.1 is an HL7 TS value. Preserve its supplied precision and time/offset.
DO $$
DECLARE
    column_type TEXT;
BEGIN
    SELECT data_type
      INTO column_type
      FROM information_schema.columns
     WHERE table_schema = current_schema()
       AND table_name = 'test_results'
       AND column_name = 'test_date';

    IF column_type = 'date' THEN
        EXECUTE 'ALTER TABLE test_results ALTER COLUMN test_date TYPE TEXT USING to_char(test_date, ''YYYYMMDD'')';
    ELSIF column_type IS DISTINCT FROM 'text' THEN
        RAISE EXCEPTION 'test_results.test_date must be date or text, found %', column_type;
    END IF;
END
$$;
