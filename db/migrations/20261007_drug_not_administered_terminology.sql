BEGIN;

SELECT set_current_user_context('00000000-0000-0000-0000-000000000001');

INSERT INTO e2b_code_lists (list_name, code, display_name, sort_order)
VALUES ('drug_characterization', '4', 'Drug Not Administered', 4)
ON CONFLICT (list_name, code) DO UPDATE
SET display_name = EXCLUDED.display_name,
    sort_order = EXCLUDED.sort_order,
    active = true;

COMMIT;
