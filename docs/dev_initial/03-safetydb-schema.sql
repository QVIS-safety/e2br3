CREATE TABLE IF NOT EXISTS organizations (
      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
      name VARCHAR(500) NOT NULL,
      org_type VARCHAR(100),
      address TEXT,
      city VARCHAR(200),
      state VARCHAR(100),
      postcode VARCHAR(50),
      country_code VARCHAR(2),  -- ISO 3166-1 alpha-2
      contact_email VARCHAR(255),
      contact_phone VARCHAR(50),
      active BOOLEAN DEFAULT true,

      -- Audit fields (standardized UUID-based)
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      created_by UUID NOT NULL,
      updated_by UUID
  );

  -- ============================================================================
  -- 2. Users (E2B Version with Roles)
  -- ============================================================================
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL,  -- FK added after organizations table is created

    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(128) UNIQUE NOT NULL,

    -- Auth (reuse your existing pattern)
    pwd VARCHAR(256),
    pwd_salt UUID NOT NULL DEFAULT gen_random_uuid(),
    token_salt UUID NOT NULL DEFAULT gen_random_uuid(),

    role VARCHAR(50) NOT NULL DEFAULT 'user',
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    active BOOLEAN DEFAULT true,
    must_change_password BOOLEAN NOT NULL DEFAULT false,
    last_login_at TIMESTAMP WITH TIME ZONE,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,  -- Nullable for initial system user
    updated_by UUID,

    CONSTRAINT email_format CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$'),
    CONSTRAINT user_role_valid CHECK (role IN ('admin', 'manager', 'user', 'viewer'))
);

CREATE INDEX idx_users_organization ON users(organization_id);
CREATE INDEX idx_users_email ON users(email);

-- Backward-compatible guard for already-created dev DBs.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT false;

    -- ============================================================================
    -- 3. Safety Cases
    -- ============================================================================
CREATE TABLE if NOT EXISTS cases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,

    -- Case identification
    safety_report_id VARCHAR(100) NOT NULL,  -- C.1.1
    version INTEGER NOT NULL DEFAULT 1,      -- C.1.1.r.1
    dg_prd_key TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    validation_profile VARCHAR(10),

    -- Workflow tracking
    created_by UUID NOT NULL REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    submitted_by UUID REFERENCES users(id),
    submitted_at TIMESTAMPTZ,

    -- Raw imported XML (for round-trip fidelity)
    raw_xml BYTEA,

    -- Dirty flags for XML merge (sections C-H)
    dirty_c BOOLEAN NOT NULL DEFAULT FALSE,
    dirty_d BOOLEAN NOT NULL DEFAULT FALSE,
    dirty_e BOOLEAN NOT NULL DEFAULT FALSE,
    dirty_f BOOLEAN NOT NULL DEFAULT FALSE,
    dirty_g BOOLEAN NOT NULL DEFAULT FALSE,
    dirty_h BOOLEAN NOT NULL DEFAULT FALSE,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Unique constraint: one active version per safety_report_id
    CONSTRAINT unique_safety_report_version UNIQUE (safety_report_id, version),
    CONSTRAINT case_status_valid CHECK (status IN ('draft', 'checked', 'validated', 'submitted', 'archived', 'nullified')),
    CONSTRAINT case_validation_profile_valid CHECK (validation_profile IS NULL OR validation_profile IN ('ich', 'fda', 'mfds'))
);

CREATE INDEX idx_cases_organization ON cases(organization_id);
CREATE INDEX idx_cases_safety_report_id ON cases(safety_report_id);
CREATE INDEX idx_cases_status ON cases(status);
CREATE INDEX idx_cases_created_by ON cases(created_by);

    -- ============================================================================
    -- 4. Case Versions (for history tracking)
    -- ============================================================================
CREATE TABLE if NOT EXISTS case_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    snapshot JSONB NOT NULL,  -- Full case data snapshot
    changed_by UUID NOT NULL REFERENCES users(id),
    change_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_case_version UNIQUE (case_id, version)
);

CREATE INDEX idx_case_versions_case ON case_versions(case_id);

    -- ============================================================================
    -- 4.1 Case Submissions (durable submission lifecycle)
    -- ============================================================================
CREATE TABLE if NOT EXISTS case_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    gateway VARCHAR(100) NOT NULL,
    remote_submission_id VARCHAR(200) NOT NULL,
    status VARCHAR(50) NOT NULL,
    xml_bytes INTEGER NOT NULL,
    submitted_by UUID NOT NULL REFERENCES users(id),
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT case_submission_status_valid CHECK (
        status IN ('ack1_received', 'ack2_received', 'ack3_received', 'ack4_received', 'rejected')
    )
);

CREATE INDEX idx_case_submissions_case ON case_submissions(case_id, submitted_at DESC);
CREATE INDEX idx_case_submissions_status ON case_submissions(status, updated_at DESC);

    -- ============================================================================
    -- 4.2 Submission ACKs (durable ACK history)
    -- ============================================================================
CREATE TABLE if NOT EXISTS submission_acks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id UUID NOT NULL REFERENCES case_submissions(id) ON DELETE CASCADE,
    ack_level SMALLINT NOT NULL CHECK (ack_level BETWEEN 1 AND 4),
    success BOOLEAN NOT NULL,
    ack_code VARCHAR(120),
    ack_message TEXT,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    raw_payload JSONB
);

CREATE INDEX idx_submission_acks_submission ON submission_acks(submission_id, ack_level, received_at DESC);
CREATE UNIQUE INDEX idx_submission_acks_unique_event
    ON submission_acks(submission_id, ack_level, success, COALESCE(ack_code, ''), received_at);

    -- ============================================================================
    -- 5. Audit Logs
    -- ============================================================================
CREATE TABLE if NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    table_name VARCHAR(100) NOT NULL,
    record_id UUID NOT NULL,
    action VARCHAR(50) NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    old_values JSONB,
    new_values JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT audit_action_valid CHECK (action IN ('CREATE', 'UPDATE', 'DELETE', 'SUBMIT', 'NULLIFY'))
);

CREATE INDEX idx_audit_logs_table_record ON audit_logs(table_name, record_id);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);

-- ============================================================================
-- 6. System User and Foreign Key Constraints
-- ============================================================================

-- Create system user for migrations and automated processes
-- This user is created BEFORE adding foreign keys so it can be referenced
INSERT INTO users (
    id,
    organization_id,
    email,
    username,
    role,
    first_name,
    last_name,
    active,
    created_at,
    updated_at
) VALUES (
    '00000000-0000-0000-0000-000000000001'::UUID,
    '00000000-0000-0000-0000-000000000000'::UUID,  -- Temporary, will be updated
    'system@e2br3.local',
    'system',
    'admin',
    'System',
    'User',
    true,
    NOW(),
    NOW()
) ON CONFLICT (id) DO NOTHING;

-- Create system organization
INSERT INTO organizations (
    id,
    name,
    org_type,
    country_code,
    active,
    created_by,
    created_at,
    updated_at
) VALUES (
    '00000000-0000-0000-0000-000000000000'::UUID,
    'System',
    'Internal',
    'XX',
    true,
    '00000000-0000-0000-0000-000000000001'::UUID,
    NOW(),
    NOW()
) ON CONFLICT (id) DO NOTHING;

-- Update system user to reference system organization
UPDATE users
SET organization_id = '00000000-0000-0000-0000-000000000000'::UUID
WHERE id = '00000000-0000-0000-0000-000000000001'::UUID;

-- Now add foreign key constraints
ALTER TABLE users
    ADD CONSTRAINT fk_users_organization
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE RESTRICT;

ALTER TABLE users
    ADD CONSTRAINT fk_users_created_by
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE RESTRICT;

ALTER TABLE users
    ADD CONSTRAINT fk_users_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id) ON DELETE RESTRICT;

ALTER TABLE organizations
    ADD CONSTRAINT fk_organizations_created_by
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE RESTRICT;

ALTER TABLE organizations
    ADD CONSTRAINT fk_organizations_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id) ON DELETE RESTRICT;

-- ============================================================================
-- 7. User Context Helper Functions
-- ============================================================================

-- Function to set current user context for transaction
-- This enables audit triggers to capture user_id
CREATE OR REPLACE FUNCTION set_current_user_context(p_user_id UUID)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
    -- Set session variable (transaction-scoped when third parameter is true)
    PERFORM set_config('app.current_user_id', p_user_id::text, true);
END;
$$;

-- Function to get current user context
CREATE OR REPLACE FUNCTION get_current_user_context()
RETURNS UUID
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    v_user_id TEXT;
BEGIN
    v_user_id := current_setting('app.current_user_id', true);

    IF v_user_id IS NULL OR v_user_id = '' THEN
        RAISE EXCEPTION 'No user context set. Call set_current_user_context() first.';
    END IF;

    RETURN v_user_id::UUID;
EXCEPTION
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Invalid user context: %', SQLERRM;
END;
$$;

-- Function to validate user context is set
CREATE OR REPLACE FUNCTION validate_user_context()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    -- Ensure user context is set before any DML operation
    PERFORM get_current_user_context();
    RETURN NEW;
EXCEPTION
    WHEN OTHERS THEN
        RAISE EXCEPTION 'User context validation failed: %. Ensure set_current_user_context() is called.', SQLERRM;
END;
$$;

-- Resolve display name for audit logs.
-- Returns username/email when visible by current role+RLS context, otherwise
-- falls back to the UUID text so audit listing never fails.
CREATE OR REPLACE FUNCTION audit_user_display(p_user_id UUID)
RETURNS TEXT
LANGUAGE plpgsql
STABLE
SECURITY INVOKER
AS $$
DECLARE
    v_display TEXT;
BEGIN
    SELECT COALESCE(NULLIF(u.username, ''), NULLIF(u.email, ''), p_user_id::TEXT)
    INTO v_display
    FROM users u
    WHERE u.id = p_user_id;

    RETURN COALESCE(v_display, p_user_id::TEXT);
EXCEPTION
    WHEN insufficient_privilege THEN
        RETURN p_user_id::TEXT;
END;
$$;

-- ============================================================================
-- 8. Row-Level Security for Audit Logs (Tamper-Proof)
-- ============================================================================

-- Enable Row-Level Security on audit_logs
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_logs FORCE ROW LEVEL SECURITY;
-- Create application role (used by API connections)
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'e2br3_app_role') THEN
        CREATE ROLE e2br3_app_role;
    END IF;
END $$;

GRANT e2br3_app_role TO app_user;

-- Create auditor role (read-only access to audit logs)
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'e2br3_auditor_role') THEN
        CREATE ROLE e2br3_auditor_role;
    END IF;
END $$;

GRANT e2br3_auditor_role TO app_user;

-- Policy 1: Allow INSERT only for application role (append-only)
CREATE POLICY audit_logs_append_only ON audit_logs
    FOR INSERT
    TO e2br3_app_role
    WITH CHECK (true);

-- Policy 2: Deny UPDATE and DELETE for application role
CREATE POLICY audit_logs_no_modify ON audit_logs
    FOR ALL
    TO e2br3_app_role
    USING (false);

-- Policy 3: Allow SELECT for auditor role
CREATE POLICY audit_logs_read_for_auditors ON audit_logs
    FOR SELECT
    TO e2br3_auditor_role
    USING (true);

-- Policy 4: Allow SELECT for app role only when current user is admin/manager
-- App connections run with SET ROLE e2br3_app_role and carry logical role in
-- app.current_user_role via set_org_context().
CREATE POLICY audit_logs_read_for_admin_manager ON audit_logs
    FOR SELECT
    TO e2br3_app_role
    USING (
        COALESCE(current_setting('app.current_user_role', true), '') IN ('admin', 'manager')
    );

-- Grant necessary permissions
GRANT INSERT ON audit_logs TO e2br3_app_role;
GRANT SELECT ON audit_logs TO e2br3_auditor_role;
GRANT USAGE ON SEQUENCE audit_logs_id_seq TO e2br3_app_role;

-- Grant execute permissions for helper functions
GRANT EXECUTE ON FUNCTION set_current_user_context(UUID) TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION get_current_user_context() TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION validate_user_context() TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION audit_user_display(UUID) TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION audit_user_display(UUID) TO e2br3_auditor_role;

-- ============================================================================
-- 9. Row-Level Security for Organization Isolation (Multi-Tenancy)
-- ============================================================================

-- Function to get current organization from session
CREATE OR REPLACE FUNCTION current_organization_id() RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_organization_id', true), '')::UUID;
EXCEPTION
    WHEN OTHERS THEN
        RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to check if current user is admin
CREATE OR REPLACE FUNCTION is_current_user_admin() RETURNS BOOLEAN AS $$
BEGIN
    RETURN COALESCE(current_setting('app.current_user_role', true), '') = 'admin';
EXCEPTION
    WHEN OTHERS THEN
        RETURN false;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to set the organization and role context for the current session
CREATE OR REPLACE FUNCTION set_org_context(org_id UUID, user_role VARCHAR) RETURNS VOID AS $$
BEGIN
    PERFORM set_config('app.current_organization_id', org_id::TEXT, true);
    PERFORM set_config('app.current_user_role', user_role, true);
END;
$$ LANGUAGE plpgsql;

-- Grant permissions for context functions
GRANT EXECUTE ON FUNCTION current_organization_id() TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION is_current_user_admin() TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION set_org_context(UUID, VARCHAR) TO e2br3_app_role;

-- Grant table access for application role (RLS will still enforce isolation)
GRANT USAGE ON SCHEMA public TO e2br3_app_role;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO e2br3_app_role;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO e2br3_app_role;

-- ============================================================================
-- 9.1 Cases Table RLS
-- ============================================================================
ALTER TABLE cases ENABLE ROW LEVEL SECURITY;
ALTER TABLE cases FORCE ROW LEVEL SECURITY;
CREATE POLICY cases_org_isolation ON cases
    FOR ALL
    TO e2br3_app_role
    USING (
        organization_id = current_organization_id()
        OR is_current_user_admin()
    )
    WITH CHECK (
        organization_id = current_organization_id()
        OR is_current_user_admin()
    );

-- ============================================================================
-- 9.2 Case Versions Table RLS
-- ============================================================================
ALTER TABLE case_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_versions FORCE ROW LEVEL SECURITY;
CREATE POLICY case_versions_via_case ON case_versions
    FOR ALL
    TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = case_versions.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = case_versions.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- ============================================================================
-- 9.3 Case Submissions Table RLS
-- ============================================================================
ALTER TABLE case_submissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_submissions FORCE ROW LEVEL SECURITY;
CREATE POLICY case_submissions_via_case ON case_submissions
    FOR ALL
    TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = case_submissions.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = case_submissions.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- ============================================================================
-- 9.4 Submission ACKs Table RLS
-- ============================================================================
ALTER TABLE submission_acks ENABLE ROW LEVEL SECURITY;
ALTER TABLE submission_acks FORCE ROW LEVEL SECURITY;
CREATE POLICY submission_acks_via_submission ON submission_acks
    FOR ALL
    TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1
            FROM case_submissions cs
            JOIN cases c ON c.id = cs.case_id
            WHERE cs.id = submission_acks.submission_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM case_submissions cs
            JOIN cases c ON c.id = cs.case_id
            WHERE cs.id = submission_acks.submission_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- ============================================================================
-- 9.5 Users Table RLS
-- ============================================================================
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE users FORCE ROW LEVEL SECURITY;
-- Users can see users in their organization (or admins see all)
CREATE POLICY users_org_isolation_select ON users
    FOR SELECT
    TO e2br3_app_role
    USING (
        organization_id = current_organization_id()
        OR is_current_user_admin()
        OR email = current_setting('app.auth_email', true)
    );

-- Only admins can create/update/delete users
CREATE POLICY users_org_isolation_modify ON users
    FOR ALL
    TO e2br3_app_role
    USING (is_current_user_admin())
    WITH CHECK (is_current_user_admin());

-- ============================================================================
-- 9.4 Organizations Table RLS
-- ============================================================================
ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organizations FORCE ROW LEVEL SECURITY;
-- Users can see their own organization (or admins see all)
CREATE POLICY orgs_select ON organizations
    FOR SELECT
    TO e2br3_app_role
    USING (
        id = current_organization_id()
        OR is_current_user_admin()
    );

-- Only admins can modify organizations
CREATE POLICY orgs_modify ON organizations
    FOR ALL
    TO e2br3_app_role
    USING (is_current_user_admin())
    WITH CHECK (is_current_user_admin());
