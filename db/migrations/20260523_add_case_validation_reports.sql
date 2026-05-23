CREATE TABLE IF NOT EXISTS case_validation_reports (
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    profile TEXT NOT NULL,
    report JSONB NOT NULL,
    stale BOOLEAN NOT NULL DEFAULT false,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (case_id, profile),
    CONSTRAINT case_validation_reports_profile_valid CHECK (profile IN ('ich', 'fda', 'mfds'))
);

CREATE INDEX IF NOT EXISTS idx_case_validation_reports_case_fresh
    ON case_validation_reports (case_id, profile)
    WHERE stale = false;

GRANT SELECT, INSERT, UPDATE, DELETE ON case_validation_reports TO e2br3_app_role;

ALTER TABLE case_validation_reports ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_validation_reports FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS case_validation_reports_via_case ON case_validation_reports;
CREATE POLICY case_validation_reports_via_case ON case_validation_reports
    FOR ALL
    TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = case_validation_reports.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = case_validation_reports.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );
