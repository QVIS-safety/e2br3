-- Derived validation summaries only; case and terminology data are unchanged.
DO $$
BEGIN
    -- Trusted schema migration: update derived caches across all organizations.
    -- Local context and schema/data changes share this atomic transaction.
    PERFORM set_config('app.platform_isolation_bypass', 'true', true);
    ALTER TABLE case_validation_summaries
        ADD COLUMN IF NOT EXISTS issue_count INTEGER NOT NULL DEFAULT 0;
    IF EXISTS (
        SELECT 1 FROM pg_attribute
        WHERE attrelid = 'case_validation_summaries'::regclass
          AND attname = 'blocking_count' AND NOT attisdropped
    ) THEN
        UPDATE case_validation_summaries
        SET issue_count = blocking_count + non_blocking_count;
    END IF;
    ALTER TABLE case_validation_summaries
        DROP CONSTRAINT IF EXISTS case_validation_summary_counts_non_negative,
        DROP COLUMN IF EXISTS blocking_count,
        DROP COLUMN IF EXISTS non_blocking_count;
    ALTER TABLE case_validation_summaries
        ADD CONSTRAINT case_validation_summary_counts_non_negative
        CHECK (issue_count >= 0 AND required_count >= 0);

    -- Rules have changed (including removal of duplicate FDA issues).
    UPDATE case_validation_summaries SET stale = true;
END $$;
