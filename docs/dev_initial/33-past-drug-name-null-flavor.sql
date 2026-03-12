ALTER TABLE past_drug_history
  ADD COLUMN IF NOT EXISTS drug_name_null_flavor VARCHAR(4)
  CHECK (drug_name_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));
