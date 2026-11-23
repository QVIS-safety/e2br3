-- Missing draft/imported F.r.2.1 is not an invented empty string.
-- Existing strings stay unchanged; conditional requirements gate submission.
ALTER TABLE test_results ALTER COLUMN test_name DROP NOT NULL;
