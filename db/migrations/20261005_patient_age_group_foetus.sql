BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'patient_information'::regclass
          AND conname = 'patient_information_age_group_check'
    ) THEN
        RAISE EXCEPTION 'required constraint patient_information_age_group_check is missing';
    END IF;
END
$$;

ALTER TABLE patient_information
    DROP CONSTRAINT patient_information_age_group_check,
    ADD CONSTRAINT patient_information_age_group_check
        CHECK (age_group IN ('0', '1', '2', '3', '4', '5', '6'));

SELECT set_current_user_context('00000000-0000-0000-0000-000000000001');

INSERT INTO e2b_code_lists (list_name, code, display_name, sort_order)
VALUES ('age_group', '0', 'Foetus', 0)
ON CONFLICT (list_name, code) DO UPDATE
SET display_name = EXCLUDED.display_name,
    sort_order = EXCLUDED.sort_order,
    active = true;

COMMIT;
