-- Replace caller-supplied role-label trust with a transaction-local isolation
-- decision validated against the normalized, registry-owned role assignment.

-- The internal system principal participates in the same normalized one-role
-- assignment model as interactive users. Older databases lacked its system
-- organization membership, which prevented reconciliation from assigning the
-- fixed platform role.
INSERT INTO user_organization_memberships (
    user_id, organization_id, active
)
SELECT
    '00000000-0000-0000-0000-000000000001'::uuid,
    '00000000-0000-0000-0000-000000000000'::uuid,
    true
WHERE EXISTS (
    SELECT 1 FROM users
    WHERE id = '00000000-0000-0000-0000-000000000001'::uuid
)
AND EXISTS (
    SELECT 1 FROM organizations
    WHERE id = '00000000-0000-0000-0000-000000000000'::uuid
)
ON CONFLICT (user_id, organization_id) DO UPDATE SET
    active = true;

INSERT INTO user_role_assignments (
    user_id, organization_id, role_id, active
)
SELECT
    '00000000-0000-0000-0000-000000000001'::uuid,
    '00000000-0000-0000-0000-000000000000'::uuid,
    '00000000-0000-0000-0000-000000000101'::uuid,
    true
WHERE EXISTS (
    SELECT 1 FROM authorization_roles
    WHERE id = '00000000-0000-0000-0000-000000000101'::uuid
      AND identity_kind = 'platform_administrator'
      AND active
      AND deleted_at IS NULL
)
AND EXISTS (
    SELECT 1 FROM user_organization_memberships
    WHERE user_id = '00000000-0000-0000-0000-000000000001'::uuid
      AND organization_id = '00000000-0000-0000-0000-000000000000'::uuid
      AND active
)
ON CONFLICT (user_id, organization_id) DO UPDATE SET
    role_id = EXCLUDED.role_id,
    active = true;

CREATE TABLE IF NOT EXISTS authorization_audit_events (
    id bigserial PRIMARY KEY,
    principal_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    role_id uuid NOT NULL,
    action_id text NOT NULL,
    decision varchar(16) NOT NULL,
    denial_reason text,
    catalog_hash text NOT NULL,
    organization_revision bigint NOT NULL,
    principal_revision bigint NOT NULL,
    target_identifier text,
    request_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT authorization_audit_decision_valid
        CHECK (decision IN ('allowed', 'denied')),
    CONSTRAINT authorization_audit_denial_reason_valid
        CHECK (
            (decision = 'allowed' AND denial_reason IS NULL)
            OR (decision = 'denied' AND denial_reason IS NOT NULL)
        )
);

CREATE INDEX IF NOT EXISTS idx_authorization_audit_org_created
    ON authorization_audit_events (organization_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_authorization_audit_request
    ON authorization_audit_events (request_id);

CREATE OR REPLACE FUNCTION prevent_authorization_audit_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'authorization audit events are append-only'
        USING ERRCODE = '55000';
END;
$$;

DROP TRIGGER IF EXISTS authorization_audit_append_only
    ON authorization_audit_events;
CREATE TRIGGER authorization_audit_append_only
    BEFORE UPDATE OR DELETE ON authorization_audit_events
    FOR EACH ROW EXECUTE FUNCTION prevent_authorization_audit_mutation();

ALTER TABLE authorization_audit_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE authorization_audit_events FORCE ROW LEVEL SECURITY;

REVOKE ALL ON authorization_audit_events FROM PUBLIC;
REVOKE UPDATE, DELETE ON authorization_audit_events FROM e2br3_app_role;
GRANT INSERT ON authorization_audit_events TO e2br3_app_role;
GRANT USAGE, SELECT ON SEQUENCE authorization_audit_events_id_seq
    TO e2br3_app_role;

CREATE OR REPLACE FUNCTION is_current_user_admin() RETURNS boolean
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    RETURN COALESCE(
        NULLIF(current_setting('app.platform_isolation_bypass', true), ''),
        'false'
    )::boolean;
EXCEPTION
    WHEN OTHERS THEN
        RETURN false;
END;
$$;

DROP POLICY IF EXISTS authorization_audit_insert
    ON authorization_audit_events;
CREATE POLICY authorization_audit_insert ON authorization_audit_events
    FOR INSERT TO e2br3_app_role
    WITH CHECK (
        organization_id = current_organization_id()
        OR is_current_user_admin()
    );

CREATE OR REPLACE FUNCTION set_authorization_isolation_context(
	org_id uuid,
	requested_platform_bypass boolean
) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
DECLARE
    actor_user_id uuid;
    normalized_platform_administrator boolean;
    authorization_not_reconciled boolean;
BEGIN
    actor_user_id := NULLIF(
        current_setting('app.current_user_id', true), ''
    )::uuid;
    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION 'user context must be set before isolation context'
            USING ERRCODE = '42501';
    END IF;

    SELECT EXISTS (
        SELECT 1
		FROM user_role_assignments assignment
		JOIN authorization_roles role ON role.id = assignment.role_id
		WHERE assignment.user_id = actor_user_id
		  AND assignment.active
		  AND role.active
          AND role.deleted_at IS NULL
          AND role.id = '00000000-0000-0000-0000-000000000101'::uuid
          AND role.identity_kind = 'platform_administrator'
    ) INTO normalized_platform_administrator;

    -- Clean bootstrap creates the fixed system user before Rust performs the
    -- first catalog reconciliation. This exception exists only while the
    -- catalog state is empty and therefore cannot survive readiness.
    authorization_not_reconciled := NOT EXISTS (
        SELECT 1 FROM authorization_catalog_state WHERE singleton
    );
    IF requested_platform_bypass
       AND NOT normalized_platform_administrator
       AND NOT (
           authorization_not_reconciled
           AND actor_user_id = '00000000-0000-0000-0000-000000000001'::uuid
       ) THEN
        RAISE EXCEPTION 'platform isolation bypass requires the fixed platform assignment'
            USING ERRCODE = '42501';
    END IF;

    PERFORM set_config('app.current_organization_id', org_id::text, true);
    PERFORM set_config(
        'app.platform_isolation_bypass',
        CASE WHEN requested_platform_bypass THEN 'true' ELSE 'false' END,
        true
    );
    -- Clear the legacy label so no policy can accidentally keep trusting it.
    PERFORM set_config('app.current_user_role', '', true);
END;
$$;

CREATE OR REPLACE FUNCTION set_org_context(
	org_id uuid,
	user_role varchar
) RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
	PERFORM set_authorization_isolation_context(
		org_id,
		lower(btrim(user_role)) = 'system_admin'
	);
END;
$$;

GRANT EXECUTE ON FUNCTION is_current_user_admin() TO e2br3_app_role;
REVOKE ALL ON FUNCTION set_authorization_isolation_context(uuid, boolean)
	FROM PUBLIC;
GRANT EXECUTE ON FUNCTION set_authorization_isolation_context(uuid, boolean)
	TO e2br3_app_role;
GRANT EXECUTE ON FUNCTION set_org_context(uuid, varchar) TO e2br3_app_role;

-- Authorization belongs to the action/permit kernel. RLS owns tenant
-- isolation only and must not independently reinterpret role labels or the
-- legacy privilege JSON.
DROP POLICY IF EXISTS audit_logs_read_for_admin_manager ON audit_logs;
CREATE POLICY audit_logs_read_for_admin_manager ON audit_logs
    FOR SELECT
    TO e2br3_app_role
    USING (
        organization_id = current_organization_id()
        OR is_current_user_admin()
    );
