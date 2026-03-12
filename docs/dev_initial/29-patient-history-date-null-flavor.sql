ALTER TABLE medical_history_episodes
ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)
    CHECK (start_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)
    CHECK (end_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));

ALTER TABLE past_drug_history
ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)
    CHECK (start_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)
    CHECK (end_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));

ALTER TABLE parent_medical_history
ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)
    CHECK (start_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)
    CHECK (end_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));

ALTER TABLE parent_past_drug_history
ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)
    CHECK (start_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)
    CHECK (end_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));
