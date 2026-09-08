-- Run with psql -v ON_ERROR_STOP=1 -f scripts/tests/unify-case-validation-counts.sql
-- Temporary table shadows the real cache; all test changes are rolled back.
BEGIN;
CREATE TEMP TABLE case_validation_summaries (
    case_id UUID PRIMARY KEY,
    blocking_count INTEGER NOT NULL DEFAULT 0,
    non_blocking_count INTEGER NOT NULL DEFAULT 0,
    required_count INTEGER NOT NULL DEFAULT 0,
    stale BOOLEAN NOT NULL DEFAULT FALSE,
    CONSTRAINT case_validation_summary_counts_non_negative
        CHECK (blocking_count >= 0 AND non_blocking_count >= 0 AND required_count >= 0)
);
INSERT INTO case_validation_summaries VALUES
    ('00000000-0000-0000-0000-000000000001', 2, 3, 4, FALSE);
ALTER TABLE case_validation_summaries OWNER TO e2br3_app_role;
ALTER TABLE case_validation_summaries ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_validation_summaries FORCE ROW LEVEL SECURITY;
CREATE POLICY migration_cache_test ON case_validation_summaries USING (
    COALESCE(NULLIF(current_setting('app.platform_isolation_bypass', TRUE), ''), 'false')::BOOLEAN
);
SET LOCAL ROLE e2br3_app_role;
DO $$ BEGIN
    ASSERT (SELECT COUNT(*) FROM case_validation_summaries) = 0;
END $$;

\ir ../../db/migrations/20260907_unify_case_validation_counts.sql
-- The second run must also work against the new schema.
\ir ../../db/migrations/20260907_unify_case_validation_counts.sql

DO $$
BEGIN
    ASSERT (SELECT COUNT(*) FROM case_validation_summaries) = 1;
    ASSERT (SELECT issue_count = 5 AND required_count = 4 AND stale
            FROM case_validation_summaries
            WHERE case_id = '00000000-0000-0000-0000-000000000001');
    ASSERT NOT EXISTS (
        SELECT 1 FROM pg_attribute
        WHERE attrelid = 'case_validation_summaries'::regclass
          AND attname IN ('blocking_count', 'non_blocking_count') AND NOT attisdropped
    );
END $$;
ROLLBACK;
