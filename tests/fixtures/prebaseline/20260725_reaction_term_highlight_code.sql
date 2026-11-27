-- Historical test fixture only; never loaded by database initialization.
DO $$
DECLARE
    column_type TEXT;
BEGIN
    SELECT data_type
      INTO column_type
      FROM information_schema.columns
     WHERE table_schema = current_schema()
       AND table_name = 'reactions'
       AND column_name = 'term_highlighted';

    IF column_type = 'boolean' THEN
        ALTER TABLE reactions
            ALTER COLUMN term_highlighted TYPE VARCHAR(1)
            USING CASE
                WHEN term_highlighted IS NULL THEN NULL
                WHEN term_highlighted THEN '1'
                ELSE '2'
            END;
    ELSIF column_type <> 'character varying' THEN
        RAISE EXCEPTION
            'Unsupported reactions.term_highlighted type: %',
            COALESCE(column_type, '<missing>');
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conrelid = 'reactions'::regclass
           AND conname = 'reactions_term_highlighted_code'
    ) THEN
        ALTER TABLE reactions
            ADD CONSTRAINT reactions_term_highlighted_code
            CHECK (term_highlighted IS NULL OR term_highlighted IN ('1', '2', '3', '4'));
    END IF;
END
$$;
