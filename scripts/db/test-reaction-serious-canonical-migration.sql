BEGIN;
CREATE SCHEMA reaction_canonical_migration_test;
SET LOCAL search_path = reaction_canonical_migration_test, pg_catalog;

CREATE FUNCTION set_current_user_context(UUID) RETURNS VOID
LANGUAGE SQL AS $$ SELECT $$;

CREATE TABLE reactions (
    id INTEGER PRIMARY KEY,
    term_highlighted TEXT,
    serious BOOLEAN,
    criteria_death BOOLEAN,
    criteria_death_null_flavor TEXT,
    criteria_life_threatening BOOLEAN,
    criteria_life_threatening_null_flavor TEXT,
    criteria_hospitalization BOOLEAN,
    criteria_hospitalization_null_flavor TEXT,
    criteria_disabling BOOLEAN,
    criteria_disabling_null_flavor TEXT,
    criteria_congenital_anomaly BOOLEAN,
    criteria_congenital_anomaly_null_flavor TEXT,
    criteria_other_medically_important BOOLEAN,
    criteria_other_medically_important_null_flavor TEXT
);
CREATE TABLE test_results (
    id INTEGER PRIMARY KEY,
    test_result_value TEXT,
    test_result_qualifier VARCHAR(2),
    CONSTRAINT ck_test_result_qualifier
        CHECK (test_result_qualifier IS NULL OR test_result_qualifier IN ('EQ', 'LT', 'LE', 'GT', 'GE'))
);

INSERT INTO reactions (id, term_highlighted, serious, criteria_hospitalization)
VALUES (1, '1', TRUE, FALSE), (2, '1', FALSE, TRUE), (3, '4', FALSE, NULL);
INSERT INTO test_results (id, test_result_value, test_result_qualifier)
VALUES (1, '10', 'EQ');

\ir ../../tests/fixtures/prebaseline/20261123_reaction_serious_and_test_result_canonical.sql

DO $$
DECLARE
    rejected BOOLEAN := FALSE;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM reactions
        WHERE id = 1
          AND serious IS FALSE
          AND criteria_hospitalization IS NULL
          AND criteria_hospitalization_null_flavor = 'NI'
    ) THEN
        RAISE EXCEPTION 'false marker was not normalized to NI';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM reactions WHERE id IN (2, 3) AND serious IS TRUE GROUP BY serious HAVING count(*) = 2) THEN
        RAISE EXCEPTION 'seriousness was not derived from criteria and E.i.3.1';
    END IF;
    IF EXISTS (SELECT 1 FROM test_results WHERE test_result_qualifier IS NOT NULL) THEN
        RAISE EXCEPTION 'EQ qualifier was not normalized';
    END IF;

    INSERT INTO reactions (id, serious) VALUES (4, TRUE);
    IF (SELECT serious FROM reactions WHERE id = 4) IS NOT NULL THEN
        RAISE EXCEPTION 'independent serious value was stored';
    END IF;
    UPDATE reactions SET term_highlighted = '2', serious = TRUE WHERE id = 4;
    IF (SELECT serious FROM reactions WHERE id = 4) IS DISTINCT FROM FALSE THEN
        RAISE EXCEPTION 'non-serious E.i.3.1 code was not derived';
    END IF;

    BEGIN
        INSERT INTO reactions (id, criteria_death) VALUES (5, FALSE);
    EXCEPTION WHEN check_violation THEN
        rejected := TRUE;
    END;
    IF NOT rejected THEN
        RAISE EXCEPTION 'false marker was accepted';
    END IF;

    rejected := FALSE;
    BEGIN
        INSERT INTO test_results (id, test_result_value, test_result_qualifier)
        VALUES (2, '10', 'EQ');
    EXCEPTION WHEN check_violation THEN
        rejected := TRUE;
    END;
    IF NOT rejected THEN
        RAISE EXCEPTION 'EQ qualifier was accepted';
    END IF;
END;
$$;

ROLLBACK;
