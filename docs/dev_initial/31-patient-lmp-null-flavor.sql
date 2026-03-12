ALTER TABLE patient_information
    ADD COLUMN IF NOT EXISTS last_menstrual_period_date_null_flavor VARCHAR(4)
        CHECK (last_menstrual_period_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));
