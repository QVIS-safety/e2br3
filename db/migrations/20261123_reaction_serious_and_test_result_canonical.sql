SELECT set_current_user_context('00000000-0000-0000-0000-000000000001');

UPDATE reactions
SET criteria_death_null_flavor = CASE WHEN criteria_death IS FALSE THEN 'NI' ELSE criteria_death_null_flavor END,
    criteria_death = CASE WHEN criteria_death IS FALSE THEN NULL ELSE criteria_death END,
    criteria_life_threatening_null_flavor = CASE WHEN criteria_life_threatening IS FALSE THEN 'NI' ELSE criteria_life_threatening_null_flavor END,
    criteria_life_threatening = CASE WHEN criteria_life_threatening IS FALSE THEN NULL ELSE criteria_life_threatening END,
    criteria_hospitalization_null_flavor = CASE WHEN criteria_hospitalization IS FALSE THEN 'NI' ELSE criteria_hospitalization_null_flavor END,
    criteria_hospitalization = CASE WHEN criteria_hospitalization IS FALSE THEN NULL ELSE criteria_hospitalization END,
    criteria_disabling_null_flavor = CASE WHEN criteria_disabling IS FALSE THEN 'NI' ELSE criteria_disabling_null_flavor END,
    criteria_disabling = CASE WHEN criteria_disabling IS FALSE THEN NULL ELSE criteria_disabling END,
    criteria_congenital_anomaly_null_flavor = CASE WHEN criteria_congenital_anomaly IS FALSE THEN 'NI' ELSE criteria_congenital_anomaly_null_flavor END,
    criteria_congenital_anomaly = CASE WHEN criteria_congenital_anomaly IS FALSE THEN NULL ELSE criteria_congenital_anomaly END,
    criteria_other_medically_important_null_flavor = CASE WHEN criteria_other_medically_important IS FALSE THEN 'NI' ELSE criteria_other_medically_important_null_flavor END,
    criteria_other_medically_important = CASE WHEN criteria_other_medically_important IS FALSE THEN NULL ELSE criteria_other_medically_important END
WHERE criteria_death IS FALSE
   OR criteria_life_threatening IS FALSE
   OR criteria_hospitalization IS FALSE
   OR criteria_disabling IS FALSE
   OR criteria_congenital_anomaly IS FALSE
   OR criteria_other_medically_important IS FALSE;

ALTER TABLE reactions
    DROP CONSTRAINT IF EXISTS ck_reaction_criteria_death_true,
    ADD CONSTRAINT ck_reaction_criteria_death_true CHECK (criteria_death IS DISTINCT FROM FALSE),
    DROP CONSTRAINT IF EXISTS ck_reaction_criteria_life_true,
    ADD CONSTRAINT ck_reaction_criteria_life_true CHECK (criteria_life_threatening IS DISTINCT FROM FALSE),
    DROP CONSTRAINT IF EXISTS ck_reaction_criteria_hospital_true,
    ADD CONSTRAINT ck_reaction_criteria_hospital_true CHECK (criteria_hospitalization IS DISTINCT FROM FALSE),
    DROP CONSTRAINT IF EXISTS ck_reaction_criteria_disabling_true,
    ADD CONSTRAINT ck_reaction_criteria_disabling_true CHECK (criteria_disabling IS DISTINCT FROM FALSE),
    DROP CONSTRAINT IF EXISTS ck_reaction_criteria_congenital_true,
    ADD CONSTRAINT ck_reaction_criteria_congenital_true CHECK (criteria_congenital_anomaly IS DISTINCT FROM FALSE),
    DROP CONSTRAINT IF EXISTS ck_reaction_criteria_other_true,
    ADD CONSTRAINT ck_reaction_criteria_other_true CHECK (criteria_other_medically_important IS DISTINCT FROM FALSE);

CREATE OR REPLACE FUNCTION set_reaction_serious_derived()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.criteria_death IS TRUE
       OR NEW.criteria_life_threatening IS TRUE
       OR NEW.criteria_hospitalization IS TRUE
       OR NEW.criteria_disabling IS TRUE
       OR NEW.criteria_congenital_anomaly IS TRUE
       OR NEW.criteria_other_medically_important IS TRUE
       OR btrim(NEW.term_highlighted) IN ('3', '4') THEN
        NEW.serious := TRUE;
    ELSIF btrim(NEW.term_highlighted) IN ('1', '2') THEN
        NEW.serious := FALSE;
    ELSE
        NEW.serious := NULL;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_reaction_serious_derived ON reactions;
CREATE TRIGGER trg_reaction_serious_derived
    BEFORE INSERT OR UPDATE OF serious, term_highlighted, criteria_death,
        criteria_life_threatening, criteria_hospitalization, criteria_disabling,
        criteria_congenital_anomaly, criteria_other_medically_important
    ON reactions
    FOR EACH ROW EXECUTE FUNCTION set_reaction_serious_derived();

UPDATE reactions
SET serious = CASE
    WHEN criteria_death IS TRUE
      OR criteria_life_threatening IS TRUE
      OR criteria_hospitalization IS TRUE
      OR criteria_disabling IS TRUE
      OR criteria_congenital_anomaly IS TRUE
      OR criteria_other_medically_important IS TRUE
      OR btrim(term_highlighted) IN ('3', '4') THEN TRUE
    WHEN btrim(term_highlighted) IN ('1', '2') THEN FALSE
    ELSE NULL
END;

UPDATE test_results
SET test_result_qualifier = NULL
WHERE test_result_qualifier = 'EQ';

ALTER TABLE test_results
    DROP CONSTRAINT IF EXISTS ck_test_result_qualifier,
    ADD CONSTRAINT ck_test_result_qualifier
        CHECK (test_result_qualifier IS NULL OR test_result_qualifier IN ('LT', 'LE', 'GT', 'GE'));
