-- Let the dedicated audit role resolve public.audit_logs and its helper functions.
-- Table-level SELECT alone is insufficient without schema USAGE.
GRANT USAGE ON SCHEMA public TO e2br3_auditor_role;

-- E.i.1.1b uses ISO 639-2 three-letter language codes (for example, "eng").
ALTER TABLE reactions
    ALTER COLUMN reaction_language TYPE VARCHAR(3);
