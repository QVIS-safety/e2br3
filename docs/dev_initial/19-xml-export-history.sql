CREATE TABLE IF NOT EXISTS xml_export_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    case_number VARCHAR(100),
    file_name VARCHAR(255) NOT NULL,
    status VARCHAR(20) NOT NULL,
    error_message TEXT,
    validation_profile VARCHAR(16),
    exported_by UUID NOT NULL REFERENCES users(id),
    exported_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT xml_export_history_status_valid CHECK (
        status IN ('success', 'error')
    ),
    CONSTRAINT xml_export_history_profile_valid CHECK (
        validation_profile IS NULL OR validation_profile IN ('ich', 'fda', 'mfds')
    )
);

CREATE INDEX IF NOT EXISTS idx_xml_export_history_exported_at
    ON xml_export_history(exported_at DESC);
CREATE INDEX IF NOT EXISTS idx_xml_export_history_case
    ON xml_export_history(case_id, exported_at DESC);
CREATE INDEX IF NOT EXISTS idx_xml_export_history_user
    ON xml_export_history(exported_by, exported_at DESC);

ALTER TABLE xml_export_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE xml_export_history FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS xml_export_history_via_case ON xml_export_history;
CREATE POLICY xml_export_history_via_case ON xml_export_history
    FOR ALL
    TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = xml_export_history.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = xml_export_history.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );
