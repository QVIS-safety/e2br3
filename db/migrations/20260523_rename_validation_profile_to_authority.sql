DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'case_validation_reports'
          AND column_name = 'profile'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'case_validation_reports'
          AND column_name = 'authority'
    ) THEN
        ALTER TABLE case_validation_reports RENAME COLUMN profile TO authority;
    END IF;
END $$;

ALTER TABLE case_validation_reports
    DROP CONSTRAINT IF EXISTS case_validation_reports_profile_valid;
ALTER TABLE case_validation_reports
    DROP CONSTRAINT IF EXISTS case_validation_reports_authority_valid;
ALTER TABLE case_validation_reports
    ADD CONSTRAINT case_validation_reports_authority_valid
    CHECK (authority IN ('ich', 'fda', 'mfds'));

DROP INDEX IF EXISTS idx_case_validation_reports_case_fresh;
CREATE INDEX IF NOT EXISTS idx_case_validation_reports_case_fresh
    ON case_validation_reports (case_id, authority)
    WHERE stale = false;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'xml_import_history'
          AND column_name = 'validation_profile'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'xml_import_history'
          AND column_name = 'validation_authority'
    ) THEN
        ALTER TABLE xml_import_history RENAME COLUMN validation_profile TO validation_authority;
    END IF;
END $$;

ALTER TABLE xml_import_history
    DROP CONSTRAINT IF EXISTS xml_import_history_profile_valid;
ALTER TABLE xml_import_history
    DROP CONSTRAINT IF EXISTS xml_import_history_authority_valid;
ALTER TABLE xml_import_history
    ADD CONSTRAINT xml_import_history_authority_valid
    CHECK (validation_authority IS NULL OR validation_authority IN ('ich', 'fda', 'mfds'));

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'xml_export_history'
          AND column_name = 'validation_profile'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'xml_export_history'
          AND column_name = 'validation_authority'
    ) THEN
        ALTER TABLE xml_export_history RENAME COLUMN validation_profile TO validation_authority;
    END IF;
END $$;

ALTER TABLE xml_export_history
    DROP CONSTRAINT IF EXISTS xml_export_history_profile_valid;
ALTER TABLE xml_export_history
    DROP CONSTRAINT IF EXISTS xml_export_history_authority_valid;
ALTER TABLE xml_export_history
    ADD CONSTRAINT xml_export_history_authority_valid
    CHECK (validation_authority IS NULL OR validation_authority IN ('ich', 'fda', 'mfds'));
