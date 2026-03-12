ALTER TABLE patient_death_information
    ADD COLUMN IF NOT EXISTS date_of_death_null_flavor VARCHAR(4)
        CHECK (date_of_death_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));

ALTER TABLE parent_information
    ADD COLUMN IF NOT EXISTS parent_birth_date_null_flavor VARCHAR(4)
        CHECK (parent_birth_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    ADD COLUMN IF NOT EXISTS parent_age_null_flavor VARCHAR(4)
        CHECK (parent_age_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    ADD COLUMN IF NOT EXISTS last_menstrual_period_date_null_flavor VARCHAR(4)
        CHECK (last_menstrual_period_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));
