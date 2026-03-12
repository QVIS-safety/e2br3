ALTER TABLE test_results
  ADD COLUMN IF NOT EXISTS test_date_null_flavor VARCHAR(10);
