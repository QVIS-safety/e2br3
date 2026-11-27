CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS organization_as2_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    authority VARCHAR(16) NOT NULL CHECK (authority IN ('fda', 'mfds')),
    endpoint_url TEXT NOT NULL,
    as2_to VARCHAR(255) NOT NULL,
    expected_ack_as2_from VARCHAR(255) NOT NULL,
    extra_headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    signing_pkcs12_ciphertext BYTEA NOT NULL,
    signing_password_ciphertext BYTEA NOT NULL,
    signing_key_alias VARCHAR(255),
    partner_certificate_pem TEXT NOT NULL,
    async_mdn_url TEXT,
    active BOOLEAN NOT NULL DEFAULT true,
    version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,
    UNIQUE (organization_id, authority)
);

CREATE UNIQUE INDEX IF NOT EXISTS organization_as2_profiles_one_active
    ON organization_as2_profiles(organization_id, authority)
    WHERE active;

GRANT SELECT, INSERT, UPDATE, DELETE ON organization_as2_profiles TO e2br3_app_role;

ALTER TABLE organization_as2_profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_as2_profiles FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS organization_as2_profiles_org_isolation ON organization_as2_profiles;
CREATE POLICY organization_as2_profiles_org_isolation ON organization_as2_profiles
    FOR ALL TO e2br3_app_role
    USING (organization_id = current_organization_id() OR is_current_user_admin())
    WITH CHECK (organization_id = current_organization_id() OR is_current_user_admin());

ALTER TABLE case_submissions
    ADD COLUMN IF NOT EXISTS submitting_organization_id UUID
        REFERENCES organizations(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS as2_profile_id UUID
        REFERENCES organization_as2_profiles(id) ON DELETE RESTRICT;

DROP TRIGGER IF EXISTS update_organization_as2_profiles_updated_at ON organization_as2_profiles;
CREATE TRIGGER update_organization_as2_profiles_updated_at
    BEFORE UPDATE ON organization_as2_profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS audit_organization_as2_profiles ON organization_as2_profiles;
CREATE TRIGGER audit_organization_as2_profiles
    AFTER INSERT OR UPDATE OR DELETE ON organization_as2_profiles
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();
