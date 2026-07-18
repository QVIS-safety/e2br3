ALTER TABLE cases
    ADD COLUMN IF NOT EXISTS status_before_lock VARCHAR(50);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conname = 'case_status_before_lock_valid'
           AND conrelid = 'cases'::regclass
    ) THEN
        ALTER TABLE cases
            ADD CONSTRAINT case_status_before_lock_valid
            CHECK (
                status_before_lock IS NULL
                OR status_before_lock IN ('draft', 'reviewed', 'validated')
            );
    END IF;
END
$$;
