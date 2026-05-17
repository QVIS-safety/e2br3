CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Create database roles before any table grants/policies reference them.
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'e2br3_app_role') THEN
        CREATE ROLE e2br3_app_role;
    END IF;
END $$;

GRANT e2br3_app_role TO app_user;

DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'e2br3_auditor_role') THEN
        CREATE ROLE e2br3_auditor_role;
    END IF;
END $$;

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
    permission_profile_id TEXT,
    comments TEXT,
    other_information TEXT,
    access_start_at TIMESTAMPTZ,
    access_end_at TIMESTAMPTZ,
    access_sender_ids TEXT,
    access_product_ids TEXT,
    access_study_ids TEXT,
    access_blind_allowed BOOLEAN,
    active_sender_identifier TEXT,
    active BOOLEAN DEFAULT true,
    must_change_password BOOLEAN NOT NULL DEFAULT false,
    last_login_at TIMESTAMP WITH TIME ZONE,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,  -- Nullable for initial system user
    updated_by UUID,

    CONSTRAINT email_format CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$'),
    CONSTRAINT user_role_valid CHECK (role IN ('system_admin', 'sponsor_admin_cro', 'sponsor_admin_company', 'user'))
);

CREATE TABLE IF NOT EXISTS app_settings (
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    key text NOT NULL,
    value jsonb NOT NULL DEFAULT '{}'::jsonb,
    updated_at timestamptz NOT NULL DEFAULT now(),
    updated_by uuid NULL REFERENCES users(id) ON DELETE SET NULL,
    PRIMARY KEY (organization_id, key)
);

CREATE TABLE IF NOT EXISTS permission_profiles (
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    profile_id text PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    description VARCHAR(512),
    can_view boolean NOT NULL DEFAULT true,
    can_review boolean NOT NULL DEFAULT false,
    can_lock boolean NOT NULL DEFAULT false,
    can_admin boolean NOT NULL DEFAULT false,
    privileges_json jsonb NOT NULL DEFAULT '[]'::jsonb,
    built_in boolean NOT NULL DEFAULT false,
    editable boolean NOT NULL DEFAULT true,
    sponsor_admin_capable boolean NOT NULL DEFAULT false,
    active boolean NOT NULL DEFAULT true,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS presave_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,
    entity_type VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT presave_templates_entity_type_valid CHECK (
        entity_type IN ('sender', 'receiver', 'product', 'reporter', 'study', 'narrative')
    )
);

CREATE TABLE IF NOT EXISTS presave_template_audits (
    id BIGSERIAL PRIMARY KEY,
    template_id UUID NOT NULL,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    action VARCHAR(50) NOT NULL,
    changed_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    old_values JSONB,
    new_values JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT presave_template_audits_action_valid CHECK (action IN ('CREATE', 'UPDATE', 'DELETE'))
);

CREATE INDEX idx_users_organization ON users(organization_id);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_presave_templates_org ON presave_templates(organization_id);
CREATE INDEX idx_presave_templates_entity_type ON presave_templates(entity_type);
CREATE INDEX idx_presave_templates_created_by ON presave_templates(created_by);
CREATE INDEX idx_presave_templates_created_at ON presave_templates(created_at DESC);
CREATE INDEX idx_presave_template_audits_template_id
    ON presave_template_audits(template_id, created_at DESC);
CREATE INDEX idx_presave_template_audits_org
    ON presave_template_audits(organization_id, created_at DESC);

-- Backward-compatible guard for already-created dev DBs.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS permission_profile_id TEXT;

UPDATE users
SET permission_profile_id = role,
    role = 'user'
WHERE role NOT IN ('system_admin', 'sponsor_admin_cro', 'sponsor_admin_company', 'user')
  AND permission_profile_id IS NULL;

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
    appendices_json TEXT,
    review_receivers_json TEXT,
    workflow_routes_json TEXT,
    workflow_status TEXT NOT NULL DEFAULT 'Saved',
    workflow_assigned_role TEXT,
    workflow_assigned_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    workflow_due_at TIMESTAMPTZ,
    workflow_description TEXT,
    workflow_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    mfds_report_type TEXT,
    report_year VARCHAR(10),
    source_document_name TEXT,
    source_document_base64 TEXT,
    source_document_media_type TEXT,

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
    CONSTRAINT case_status_valid CHECK (status IN ('draft', 'reviewed', 'validated', 'locked', 'submitted', 'deleted', 'archived', 'nullified'))
);

CREATE INDEX idx_cases_organization ON cases(organization_id);
CREATE INDEX idx_cases_safety_report_id ON cases(safety_report_id);
CREATE INDEX idx_cases_status ON cases(status);
CREATE INDEX idx_cases_workflow_status ON cases(workflow_status);
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

CREATE TABLE if NOT EXISTS case_workflow_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    from_status TEXT NOT NULL,
    to_status TEXT NOT NULL,
    target_role TEXT,
    target_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    comment TEXT,
    due_at TIMESTAMPTZ,
    acted_by UUID NOT NULL REFERENCES users(id),
    actor_role_id TEXT NOT NULL,
    used_admin_override BOOLEAN NOT NULL DEFAULT false,
    override_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_case_workflow_events_case ON case_workflow_events(case_id, created_at DESC);
GRANT SELECT, INSERT, UPDATE, DELETE ON case_workflow_events TO e2br3_app_role;

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
    -- 4.2 Submission Events (durable lifecycle history)
    -- ============================================================================
CREATE TABLE if NOT EXISTS submission_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id UUID NOT NULL REFERENCES case_submissions(id) ON DELETE CASCADE,
    event_type VARCHAR(80) NOT NULL,
    event_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_submission_events_submission ON submission_events(submission_id, created_at DESC);
CREATE INDEX idx_submission_events_type ON submission_events(event_type, created_at DESC);

    -- ============================================================================
    -- 4.3 Submission Dispatch State (retry/terminal metadata)
    -- ============================================================================
CREATE TABLE if NOT EXISTS submission_dispatch_state (
    submission_id UUID PRIMARY KEY REFERENCES case_submissions(id) ON DELETE CASCADE,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    last_attempt_at TIMESTAMPTZ,
    last_error TEXT,
    next_retry_at TIMESTAMPTZ,
    terminal_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_submission_dispatch_retry ON submission_dispatch_state(next_retry_at)
    WHERE next_retry_at IS NOT NULL;
CREATE INDEX idx_submission_dispatch_terminal ON submission_dispatch_state(terminal_at);

    -- ============================================================================
    -- 4.4 Submission Idempotency Keys
    -- ============================================================================
CREATE TABLE if NOT EXISTS submission_idempotency (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    authority VARCHAR(16) NOT NULL,
    idempotency_key VARCHAR(128) NOT NULL,
    submission_id UUID NOT NULL REFERENCES case_submissions(id) ON DELETE CASCADE,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT submission_idempotency_authority_valid CHECK (
        authority IN ('fda', 'mfds')
    ),
    CONSTRAINT submission_idempotency_unique UNIQUE (case_id, authority, idempotency_key)
);

CREATE INDEX idx_submission_idempotency_submission ON submission_idempotency(submission_id);

    -- ============================================================================
    -- 4.5 XML Import History
    -- ============================================================================
CREATE TABLE if NOT EXISTS xml_import_history (
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

CREATE INDEX idx_xml_import_history_uploaded_at ON xml_import_history(uploaded_at DESC);
CREATE INDEX idx_xml_import_history_case ON xml_import_history(case_id, uploaded_at DESC);
CREATE INDEX idx_xml_import_history_user ON xml_import_history(uploaded_by, uploaded_at DESC);

    -- ============================================================================
    -- 4.6 XML Export History
    -- ============================================================================
CREATE TABLE if NOT EXISTS xml_export_history (
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

CREATE INDEX idx_xml_export_history_exported_at ON xml_export_history(exported_at DESC);
CREATE INDEX idx_xml_export_history_case ON xml_export_history(case_id, exported_at DESC);
CREATE INDEX idx_xml_export_history_user ON xml_export_history(exported_by, exported_at DESC);

    -- ============================================================================
    -- 4.5 Submission ACKs (durable ACK history)
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
    -- 4.6 Electronic Signatures (Part 11 / Annex 11)
    -- ============================================================================
CREATE TABLE if NOT EXISTS e_signatures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID REFERENCES cases(id) ON DELETE SET NULL,
    signer_user_id UUID NOT NULL REFERENCES users(id),
    signer_username VARCHAR(128) NOT NULL,
    action VARCHAR(50) NOT NULL,
    meaning TEXT NOT NULL,
    reason TEXT NOT NULL,
    signature_method VARCHAR(50) NOT NULL DEFAULT 'password_reentry',
    signed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE INDEX idx_e_signatures_case ON e_signatures(case_id, signed_at DESC);
CREATE INDEX idx_e_signatures_signer ON e_signatures(signer_user_id, signed_at DESC);
CREATE INDEX idx_e_signatures_action ON e_signatures(action, signed_at DESC);

    -- ============================================================================
    -- 5. Audit Logs
    -- ============================================================================
CREATE TABLE if NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    table_name VARCHAR(100) NOT NULL,
    record_id UUID NOT NULL,
    action VARCHAR(50) NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    reason_for_change TEXT,
    e_signature_id UUID REFERENCES e_signatures(id),
    old_values JSONB,
    new_values JSONB,
    changed_fields JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    prev_hash CHAR(64) NOT NULL,
    entry_hash CHAR(64) NOT NULL,

    CONSTRAINT audit_action_valid CHECK (action IN ('CREATE', 'UPDATE', 'DELETE', 'SUBMIT', 'NULLIFY'))
);

CREATE INDEX idx_audit_logs_table_record ON audit_logs(table_name, record_id);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);
CREATE INDEX idx_audit_logs_org_created_at ON audit_logs(organization_id, created_at DESC);
CREATE INDEX idx_audit_logs_org_table_record_created_at ON audit_logs(organization_id, table_name, record_id, created_at DESC);
CREATE INDEX idx_audit_logs_org_user_created_at ON audit_logs(organization_id, user_id, created_at DESC);
CREATE INDEX idx_app_settings_org_key ON app_settings(organization_id, key);
CREATE INDEX idx_permission_profiles_org ON permission_profiles(organization_id);
CREATE UNIQUE INDEX idx_permission_profiles_org_name_unique
    ON permission_profiles(organization_id, lower(btrim(name)));
CREATE INDEX idx_audit_logs_esignature ON audit_logs(e_signature_id);
CREATE INDEX idx_audit_logs_changed_fields ON audit_logs USING GIN (changed_fields);
CREATE INDEX idx_audit_logs_prev_hash ON audit_logs(prev_hash);
CREATE UNIQUE INDEX idx_audit_logs_entry_hash ON audit_logs(entry_hash);

ALTER TABLE audit_logs
    ADD COLUMN IF NOT EXISTS changed_fields JSONB;

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
    active,
    created_at,
    updated_at
) VALUES (
    '00000000-0000-0000-0000-000000000001'::UUID,
    '00000000-0000-0000-0000-000000000000'::UUID,  -- Temporary, will be updated
    'system@e2br3.local',
    'system',
    'system_admin',
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

-- Compliance context setter for audit enrichment.
CREATE OR REPLACE FUNCTION set_compliance_context(
    p_change_reason TEXT,
    p_e_signature_id TEXT
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
    PERFORM set_config(
        'app.change_reason',
        COALESCE(p_change_reason, ''),
        true
    );
    PERFORM set_config(
        'app.e_signature_id',
        COALESCE(p_e_signature_id, ''),
        true
    );
END;
$$;

CREATE OR REPLACE FUNCTION get_current_change_reason()
RETURNS TEXT
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    v_reason TEXT;
BEGIN
    v_reason := current_setting('app.change_reason', true);
    IF v_reason IS NULL OR btrim(v_reason) = '' THEN
        RETURN NULL;
    END IF;
    RETURN v_reason;
END;
$$;

CREATE OR REPLACE FUNCTION get_current_esignature_id()
RETURNS UUID
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    v_sig TEXT;
BEGIN
    v_sig := current_setting('app.e_signature_id', true);
    IF v_sig IS NULL OR btrim(v_sig) = '' THEN
        RETURN NULL;
    END IF;
    RETURN v_sig::UUID;
EXCEPTION
    WHEN OTHERS THEN
        RETURN NULL;
END;
$$;

-- Compute field-level delta as:
-- {"path.to.field": {"old": <jsonb>, "new": <jsonb>}}
CREATE OR REPLACE FUNCTION compute_audit_changed_fields(
    p_old JSONB,
    p_new JSONB,
    p_prefix TEXT DEFAULT ''
)
RETURNS JSONB
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    v_result JSONB := '{}'::JSONB;
    v_nested JSONB;
    v_key TEXT;
    v_old_value JSONB;
    v_new_value JSONB;
    v_path TEXT;
BEGIN
    IF p_old IS NULL THEN
        p_old := '{}'::JSONB;
    END IF;
    IF p_new IS NULL THEN
        p_new := '{}'::JSONB;
    END IF;

    IF jsonb_typeof(p_old) = 'object' AND jsonb_typeof(p_new) = 'object' THEN
        FOR v_key IN
            SELECT key FROM (
                SELECT jsonb_object_keys(p_old) AS key
                UNION
                SELECT jsonb_object_keys(p_new) AS key
            ) keys
        LOOP
            v_old_value := p_old -> v_key;
            v_new_value := p_new -> v_key;
            v_path := CASE
                WHEN p_prefix IS NULL OR p_prefix = '' THEN v_key
                ELSE p_prefix || '.' || v_key
            END;

            IF jsonb_typeof(v_old_value) = 'object' AND jsonb_typeof(v_new_value) = 'object' THEN
                v_nested := compute_audit_changed_fields(v_old_value, v_new_value, v_path);
                IF v_nested <> '{}'::JSONB THEN
                    v_result := v_result || v_nested;
                END IF;
            ELSIF v_old_value IS DISTINCT FROM v_new_value THEN
                v_result := v_result || jsonb_build_object(
                    v_path,
                    jsonb_build_object('old', v_old_value, 'new', v_new_value)
                );
            END IF;
        END LOOP;
        RETURN v_result;
    END IF;

    IF p_old IS DISTINCT FROM p_new THEN
        v_path := CASE
            WHEN p_prefix IS NULL OR p_prefix = '' THEN '$'
            ELSE p_prefix
        END;
        RETURN jsonb_build_object(
            v_path,
            jsonb_build_object('old', p_old, 'new', p_new)
        );
    END IF;

    RETURN '{}'::JSONB;
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
GRANT e2br3_auditor_role TO app_user;

-- Function to get current organization from session. Defined here as well so
-- audit RLS policies can reference it before the general tenant RLS section.
CREATE OR REPLACE FUNCTION current_organization_id() RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_organization_id', true), '')::UUID;
EXCEPTION
    WHEN OTHERS THEN
        RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to check if current user has safety-database admin bypass.
CREATE OR REPLACE FUNCTION is_current_user_admin() RETURNS BOOLEAN AS $$
BEGIN
    RETURN COALESCE(current_setting('app.current_user_role', true), '') = 'system_admin';
EXCEPTION
    WHEN OTHERS THEN
        RETURN false;
END;
$$ LANGUAGE plpgsql STABLE;

-- Policy 1: Allow INSERT only for application role (append-only)
CREATE POLICY audit_logs_append_only ON audit_logs
    FOR INSERT
    TO e2br3_app_role
    WITH CHECK (
        organization_id = current_organization_id()
        OR is_current_user_admin()
    );

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

-- Policy 4: Allow SELECT for app role only when current user has elevated audit access
-- App connections run with SET ROLE e2br3_app_role and carry logical role in
-- app.current_user_role via set_org_context().
CREATE POLICY audit_logs_read_for_admin_manager ON audit_logs
    FOR SELECT
    TO e2br3_app_role
    USING (
        COALESCE(current_setting('app.current_user_role', true), '') IN (
            'system_admin',
            'sponsor_admin_cro',
            'sponsor_admin_company',
            'manager',
            'pvm',
            'head_pv'
        )
        AND (
            organization_id = current_organization_id()
            OR is_current_user_admin()
        )
    );

-- Grant necessary permissions
GRANT INSERT ON audit_logs TO e2br3_app_role;
GRANT SELECT ON audit_logs TO e2br3_auditor_role;
GRANT USAGE ON SEQUENCE audit_logs_id_seq TO e2br3_app_role;

-- Grant execute permissions for helper functions
GRANT EXECUTE ON FUNCTION set_current_user_context(UUID) TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION get_current_user_context() TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION validate_user_context() TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION set_compliance_context(TEXT, TEXT) TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION get_current_change_reason() TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION get_current_esignature_id() TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION compute_audit_changed_fields(JSONB, JSONB, TEXT) TO e2br3_app_role;
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

-- Function to check if current user has safety-database admin bypass.
CREATE OR REPLACE FUNCTION is_current_user_admin() RETURNS BOOLEAN AS $$
BEGIN
    RETURN COALESCE(current_setting('app.current_user_role', true), '') = 'system_admin';
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
-- 9.3 Case Workflow Events Table RLS
-- ============================================================================
ALTER TABLE case_workflow_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_workflow_events FORCE ROW LEVEL SECURITY;
CREATE POLICY case_workflow_events_via_case ON case_workflow_events
    FOR ALL
    TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = case_workflow_events.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = case_workflow_events.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- ============================================================================
-- 9.4 Case Submissions Table RLS
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
-- 9.4 Submission Events Table RLS
-- ============================================================================
ALTER TABLE submission_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE submission_events FORCE ROW LEVEL SECURITY;
CREATE POLICY submission_events_via_submission ON submission_events
    FOR ALL
    TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1
            FROM case_submissions cs
            JOIN cases c ON c.id = cs.case_id
            WHERE cs.id = submission_events.submission_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM case_submissions cs
            JOIN cases c ON c.id = cs.case_id
            WHERE cs.id = submission_events.submission_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- ============================================================================
-- 9.5 Submission Dispatch State Table RLS
-- ============================================================================
ALTER TABLE submission_dispatch_state ENABLE ROW LEVEL SECURITY;
ALTER TABLE submission_dispatch_state FORCE ROW LEVEL SECURITY;
CREATE POLICY submission_dispatch_state_via_submission ON submission_dispatch_state
    FOR ALL
    TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1
            FROM case_submissions cs
            JOIN cases c ON c.id = cs.case_id
            WHERE cs.id = submission_dispatch_state.submission_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM case_submissions cs
            JOIN cases c ON c.id = cs.case_id
            WHERE cs.id = submission_dispatch_state.submission_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- ============================================================================
-- 9.6 Submission Idempotency Table RLS
-- ============================================================================
ALTER TABLE submission_idempotency ENABLE ROW LEVEL SECURITY;
ALTER TABLE submission_idempotency FORCE ROW LEVEL SECURITY;
CREATE POLICY submission_idempotency_via_case ON submission_idempotency
    FOR ALL
    TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = submission_idempotency.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = submission_idempotency.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- ============================================================================
-- 9.7 Submission ACKs Table RLS
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
-- 9.8 XML Import History Table RLS
-- ============================================================================
ALTER TABLE xml_import_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE xml_import_history FORCE ROW LEVEL SECURITY;
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

-- ============================================================================
-- 9.9 XML Export History Table RLS
-- ============================================================================
ALTER TABLE xml_export_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE xml_export_history FORCE ROW LEVEL SECURITY;
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

-- ============================================================================
-- 9.10 Presave Templates Table RLS
-- ============================================================================
ALTER TABLE presave_templates ENABLE ROW LEVEL SECURITY;
ALTER TABLE presave_templates FORCE ROW LEVEL SECURITY;
CREATE POLICY presave_templates_org_isolation ON presave_templates
    FOR ALL
    TO e2br3_app_role
    USING (
        organization_id = current_organization_id() OR is_current_user_admin()
    )
    WITH CHECK (
        organization_id = current_organization_id() OR is_current_user_admin()
    );

ALTER TABLE presave_template_audits ENABLE ROW LEVEL SECURITY;
ALTER TABLE presave_template_audits FORCE ROW LEVEL SECURITY;
CREATE POLICY presave_template_audits_select ON presave_template_audits
    FOR SELECT
    TO e2br3_app_role
    USING (
        organization_id = current_organization_id() OR is_current_user_admin()
    );

CREATE POLICY presave_template_audits_insert ON presave_template_audits
    FOR INSERT
    TO e2br3_app_role
    WITH CHECK (
        organization_id = current_organization_id() OR is_current_user_admin()
    );

-- ============================================================================
-- 9.11 Permission Profiles and App Settings Table RLS
-- ============================================================================
ALTER TABLE permission_profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE permission_profiles FORCE ROW LEVEL SECURITY;
CREATE POLICY permission_profiles_org_isolation ON permission_profiles
    FOR ALL
    TO e2br3_app_role
    USING (
        organization_id = current_organization_id() OR is_current_user_admin()
    )
    WITH CHECK (
        organization_id = current_organization_id() OR is_current_user_admin()
    );

ALTER TABLE app_settings ENABLE ROW LEVEL SECURITY;
ALTER TABLE app_settings FORCE ROW LEVEL SECURITY;
CREATE POLICY app_settings_org_isolation ON app_settings
    FOR ALL
    TO e2br3_app_role
    USING (
        organization_id = current_organization_id() OR is_current_user_admin()
    )
    WITH CHECK (
        organization_id = current_organization_id() OR is_current_user_admin()
    );

-- ============================================================================
-- 9.12 Electronic Signatures Table RLS
-- ============================================================================
ALTER TABLE e_signatures ENABLE ROW LEVEL SECURITY;
ALTER TABLE e_signatures FORCE ROW LEVEL SECURITY;
CREATE POLICY e_signatures_via_case ON e_signatures
    FOR ALL
    TO e2br3_app_role
    USING (
        case_id IS NULL
        OR EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = e_signatures.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        case_id IS NULL
        OR EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = e_signatures.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- ============================================================================
-- 9.13 Users Table RLS
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
    USING (
        organization_id = current_organization_id()
        OR is_current_user_admin()
    )
    WITH CHECK (
        organization_id = current_organization_id()
        OR is_current_user_admin()
    );

-- ============================================================================
-- 9.14 Organizations Table RLS
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
