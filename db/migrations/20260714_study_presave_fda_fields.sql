ALTER TABLE study_presaves
    ADD COLUMN IF NOT EXISTS fda_ind_number_occurred VARCHAR(10),
    ADD COLUMN IF NOT EXISTS fda_pre_anda_number_occurred VARCHAR(10);

CREATE TABLE IF NOT EXISTS study_presave_fda_cross_reported_ind_numbers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    study_presave_id UUID NOT NULL REFERENCES study_presaves(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,
    ind_number VARCHAR(10) NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,
    CONSTRAINT study_presave_fda_cross_reported_ind_numbers_sequence_unique UNIQUE (study_presave_id, sequence_number)
);

CREATE INDEX IF NOT EXISTS idx_study_presave_fda_cross_reported_ind_numbers_parent
    ON study_presave_fda_cross_reported_ind_numbers(study_presave_id);

GRANT SELECT, INSERT, UPDATE, DELETE
    ON study_presave_fda_cross_reported_ind_numbers TO e2br3_app_role;

ALTER TABLE study_presave_fda_cross_reported_ind_numbers ENABLE ROW LEVEL SECURITY;
ALTER TABLE study_presave_fda_cross_reported_ind_numbers FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS study_presave_fda_cross_reported_ind_numbers_via_parent ON study_presave_fda_cross_reported_ind_numbers;
CREATE POLICY study_presave_fda_cross_reported_ind_numbers_via_parent ON study_presave_fda_cross_reported_ind_numbers
    FOR ALL TO e2br3_app_role
    USING (EXISTS (
        SELECT 1 FROM study_presaves p
        WHERE p.id = study_presave_fda_cross_reported_ind_numbers.study_presave_id
          AND (p.organization_id = current_organization_id() OR is_current_user_admin())
    ))
    WITH CHECK (EXISTS (
        SELECT 1 FROM study_presaves p
        WHERE p.id = study_presave_fda_cross_reported_ind_numbers.study_presave_id
          AND (p.organization_id = current_organization_id() OR is_current_user_admin())
    ));

DROP TRIGGER IF EXISTS audit_study_presave_fda_cross_reported_ind_numbers ON study_presave_fda_cross_reported_ind_numbers;
CREATE TRIGGER audit_study_presave_fda_cross_reported_ind_numbers
    AFTER INSERT OR UPDATE OR DELETE ON study_presave_fda_cross_reported_ind_numbers
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

DROP TRIGGER IF EXISTS update_study_presave_fda_cross_reported_ind_numbers_updated_at ON study_presave_fda_cross_reported_ind_numbers;
CREATE TRIGGER update_study_presave_fda_cross_reported_ind_numbers_updated_at
    BEFORE UPDATE ON study_presave_fda_cross_reported_ind_numbers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
