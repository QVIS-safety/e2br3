-- A missing revision denotes a cache that must be recomputed, never a clean case.
ALTER TABLE case_validation_summaries
    ADD COLUMN IF NOT EXISTS validation_revision TEXT;
