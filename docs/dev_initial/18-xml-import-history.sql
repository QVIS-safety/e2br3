CREATE TABLE IF NOT EXISTS xml_import_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    uploaded_file_name VARCHAR(255) NOT NULL,
    source_file_name VARCHAR(255) NOT NULL,
    case_id UUID REFERENCES cases(id) ON DELETE SET NULL,
    case_number VARCHAR(100),
    status VARCHAR(20) NOT NULL,
    error_message TEXT,
    validation_profile VARCHAR(16),
    uploaded_by UUID NOT NULL REFERENCES users(id),
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT xml_import_history_status_valid CHECK (
        status IN ('success', 'warning', 'error')
    ),
    CONSTRAINT xml_import_history_profile_valid CHECK (
        validation_profile IS NULL OR validation_profile IN ('ich', 'fda', 'mfds')
    )
);

CREATE INDEX IF NOT EXISTS idx_xml_import_history_uploaded_at
    ON xml_import_history(uploaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_xml_import_history_case
    ON xml_import_history(case_id, uploaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_xml_import_history_user
    ON xml_import_history(uploaded_by, uploaded_at DESC);

ALTER TABLE xml_import_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE xml_import_history FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS xml_import_history_org_isolation ON xml_import_history;
CREATE POLICY xml_import_history_org_isolation ON xml_import_history
    FOR ALL
    TO e2br3_app_role
    USING (
        uploaded_by = get_current_user_context()
        OR EXISTS (
            SELECT 1 FROM users u
            WHERE u.id = xml_import_history.uploaded_by
            AND (
                u.organization_id = current_organization_id()
                OR is_current_user_admin()
            )
        )
    )
    WITH CHECK (
        uploaded_by = get_current_user_context()
        OR is_current_user_admin()
    );
