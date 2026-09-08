-- Derived cache only: preserve rule/location identities for cross-appendix totals.
DO $$
BEGIN
    PERFORM set_config('app.platform_isolation_bypass', 'true', true);
    ALTER TABLE case_validation_summaries
        ADD COLUMN IF NOT EXISTS issue_keys JSONB NOT NULL DEFAULT '[]'::jsonb;
    UPDATE case_validation_summaries SET stale = true;
END $$;
