BEGIN;

DO $$
BEGIN
    IF current_database() !~ '^e2br3_ui_bombadil_[a-z0-9_]+$' THEN
        RAISE EXCEPTION 'Bombadil terminology fixture requires an e2br3_ui_bombadil_* database';
    END IF;
END
$$;

SELECT set_current_user_context('00000000-0000-0000-0000-000000000001');
SELECT set_config('app.current_user_role', 'system_admin', true);
SELECT set_authorization_isolation_context('00000000-0000-0000-0000-000000000000', true);

UPDATE terminology_releases
SET status = 'retired', updated_at = NOW()
WHERE dictionary = 'meddra'
  AND language = 'en'
  AND status = 'active'
  AND version <> '28.1';

INSERT INTO terminology_releases (
    dictionary, version, language, status, loaded_rows, source_path, note
)
VALUES (
    'meddra', '28.1', 'en', 'active', 1,
    'test-fixture:bombadil-native-case-lifecycle',
    'Deterministic Bombadil test term; not licensed MedDRA data'
)
ON CONFLICT (dictionary, version, language) DO UPDATE
SET status = 'active',
    loaded_rows = EXCLUDED.loaded_rows,
    source_path = EXCLUDED.source_path,
    note = EXCLUDED.note,
    updated_at = NOW();

INSERT INTO meddra_terms (code, term, level, version, language, active)
VALUES ('99999999', 'Bombadil test reaction', 'LLT', '28.1', 'en', true)
ON CONFLICT (code, version, language) DO UPDATE
SET term = EXCLUDED.term,
    level = EXCLUDED.level,
    active = true;

COMMIT;
