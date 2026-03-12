ALTER TABLE reactions
ADD COLUMN IF NOT EXISTS criteria_death_null_flavor VARCHAR(4)
    CHECK (criteria_death_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS criteria_life_threatening_null_flavor VARCHAR(4)
    CHECK (criteria_life_threatening_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS criteria_hospitalization_null_flavor VARCHAR(4)
    CHECK (criteria_hospitalization_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS criteria_disabling_null_flavor VARCHAR(4)
    CHECK (criteria_disabling_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS criteria_congenital_anomaly_null_flavor VARCHAR(4)
    CHECK (criteria_congenital_anomaly_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS criteria_other_medically_important_null_flavor VARCHAR(4)
    CHECK (criteria_other_medically_important_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));

UPDATE reactions
SET
    criteria_death_null_flavor = CASE WHEN criteria_death THEN NULL ELSE 'NI' END,
    criteria_life_threatening_null_flavor = CASE WHEN criteria_life_threatening THEN NULL ELSE 'NI' END,
    criteria_hospitalization_null_flavor = CASE WHEN criteria_hospitalization THEN NULL ELSE 'NI' END,
    criteria_disabling_null_flavor = CASE WHEN criteria_disabling THEN NULL ELSE 'NI' END,
    criteria_congenital_anomaly_null_flavor = CASE WHEN criteria_congenital_anomaly THEN NULL ELSE 'NI' END,
    criteria_other_medically_important_null_flavor = CASE WHEN criteria_other_medically_important THEN NULL ELSE 'NI' END
WHERE
    criteria_death_null_flavor IS NULL
    OR criteria_life_threatening_null_flavor IS NULL
    OR criteria_hospitalization_null_flavor IS NULL
    OR criteria_disabling_null_flavor IS NULL
    OR criteria_congenital_anomaly_null_flavor IS NULL
    OR criteria_other_medically_important_null_flavor IS NULL;
