CREATE TABLE IF NOT EXISTS authorization_catalog_state (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    schema_version integer NOT NULL CHECK (schema_version > 0),
    catalog_hash text NOT NULL CHECK (catalog_hash ~ '^[0-9a-f]{64}$'),
    reconciled_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS authorization_grant_catalog (
    grant_id text PRIMARY KEY CHECK (grant_id ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$'),
    pdf_order smallint NOT NULL UNIQUE CHECK (pdf_order > 0),
    pdf_menu text NOT NULL,
    pdf_type text NOT NULL,
    pdf_privilege text NOT NULL,
    availability text NOT NULL CHECK (availability IN ('implemented', 'reserved'))
);

CREATE TABLE IF NOT EXISTS authorization_grant_role_classes (
    grant_id text NOT NULL REFERENCES authorization_grant_catalog(grant_id) ON DELETE CASCADE,
    role_class text NOT NULL CHECK (role_class IN (
        'platform_built_in', 'sponsor_cro_built_in',
        'sponsor_company_built_in', 'operational_built_in',
        'service_built_in', 'custom'
    )),
    PRIMARY KEY (grant_id, role_class)
);

CREATE TABLE IF NOT EXISTS authorization_roles (
    id uuid PRIMARY KEY,
    organization_id uuid REFERENCES organizations(id) ON DELETE RESTRICT,
    stable_key text,
    identity_kind text CHECK (identity_kind IS NULL OR identity_kind IN (
        'platform_administrator', 'sponsor_cro_administrator',
        'sponsor_company_administrator', 'operational_user',
        'internal_service_principal'
    )),
    role_class text NOT NULL CHECK (role_class IN (
        'platform_built_in', 'sponsor_cro_built_in',
        'sponsor_company_built_in', 'operational_built_in',
        'service_built_in', 'custom'
    )),
    name text NOT NULL,
    built_in boolean NOT NULL,
    active boolean NOT NULL DEFAULT true,
    deleted_at timestamptz,
    row_version bigint NOT NULL DEFAULT 1 CHECK (row_version > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (
        (built_in AND organization_id IS NULL AND stable_key IS NOT NULL AND identity_kind IS NOT NULL)
        OR
        (NOT built_in AND organization_id IS NOT NULL AND stable_key IS NULL AND identity_kind IS NULL AND role_class = 'custom')
    )
);

ALTER TABLE authorization_roles
    ADD COLUMN IF NOT EXISTS deleted_at timestamptz,
    ADD COLUMN IF NOT EXISTS row_version bigint NOT NULL DEFAULT 1
        CHECK (row_version > 0);

CREATE UNIQUE INDEX IF NOT EXISTS authorization_roles_builtin_stable_key
    ON authorization_roles(stable_key) WHERE built_in;
CREATE UNIQUE INDEX IF NOT EXISTS authorization_roles_builtin_identity_kind
    ON authorization_roles(identity_kind) WHERE built_in;
CREATE UNIQUE INDEX IF NOT EXISTS authorization_roles_custom_org_name
    ON authorization_roles(organization_id, lower(btrim(name))) WHERE NOT built_in AND active;

CREATE OR REPLACE FUNCTION protect_builtin_authorization_role()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.built_in AND current_user = 'e2br3_app_role' THEN
        RAISE EXCEPTION 'built-in authorization roles are registry-owned'
            USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'UPDATE' AND current_user = 'e2br3_app_role' AND (
        NEW.organization_id IS DISTINCT FROM OLD.organization_id
        OR NEW.stable_key IS DISTINCT FROM OLD.stable_key
        OR NEW.identity_kind IS DISTINCT FROM OLD.identity_kind
        OR NEW.role_class IS DISTINCT FROM OLD.role_class
        OR NEW.built_in IS DISTINCT FROM OLD.built_in
    ) THEN
        RAISE EXCEPTION 'authorization role ownership fields are immutable'
            USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'UPDATE' AND current_user = 'e2br3_app_role'
       AND OLD.active AND NOT NEW.active
       AND EXISTS (SELECT 1 FROM user_role_assignments WHERE role_id = OLD.id) THEN
        RAISE EXCEPTION 'an assigned authorization role cannot be deactivated'
            USING ERRCODE = '23514';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

DROP TRIGGER IF EXISTS authorization_roles_builtin_guard ON authorization_roles;
CREATE TRIGGER authorization_roles_builtin_guard
BEFORE UPDATE OR DELETE ON authorization_roles
FOR EACH ROW EXECUTE FUNCTION protect_builtin_authorization_role();

CREATE TABLE IF NOT EXISTS role_grants (
    role_id uuid NOT NULL REFERENCES authorization_roles(id) ON DELETE CASCADE,
    grant_id text NOT NULL REFERENCES authorization_grant_catalog(grant_id) ON DELETE RESTRICT,
    PRIMARY KEY (role_id, grant_id)
);

CREATE OR REPLACE FUNCTION protect_builtin_role_grant()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_role_id uuid;
BEGIN
    target_role_id := CASE WHEN TG_OP = 'DELETE' THEN OLD.role_id ELSE NEW.role_id END;
    IF current_user = 'e2br3_app_role' AND EXISTS (
        SELECT 1 FROM authorization_roles WHERE id = target_role_id AND built_in
    ) THEN
        RAISE EXCEPTION 'built-in role grants are registry-owned'
            USING ERRCODE = '42501';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

DROP TRIGGER IF EXISTS role_grants_builtin_guard ON role_grants;
CREATE TRIGGER role_grants_builtin_guard
BEFORE INSERT OR UPDATE OR DELETE ON role_grants
FOR EACH ROW EXECUTE FUNCTION protect_builtin_role_grant();

CREATE OR REPLACE FUNCTION enforce_role_grant_assignment()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM authorization_roles r
        JOIN authorization_grant_catalog g ON g.grant_id = NEW.grant_id
        JOIN authorization_grant_role_classes c
          ON c.grant_id = NEW.grant_id AND c.role_class = r.role_class
        WHERE r.id = NEW.role_id
          AND r.active
          AND g.availability = 'implemented'
    ) THEN
        RAISE EXCEPTION 'grant % is not assignable to role %', NEW.grant_id, NEW.role_id
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS role_grants_assignment_guard ON role_grants;
CREATE TRIGGER role_grants_assignment_guard
BEFORE INSERT OR UPDATE ON role_grants
FOR EACH ROW EXECUTE FUNCTION enforce_role_grant_assignment();

CREATE TABLE IF NOT EXISTS user_role_assignments (
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    role_id uuid NOT NULL REFERENCES authorization_roles(id) ON DELETE RESTRICT,
    active boolean NOT NULL DEFAULT true,
    row_version bigint NOT NULL DEFAULT 1 CHECK (row_version > 0),
    assigned_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, organization_id),
    FOREIGN KEY (user_id, organization_id)
        REFERENCES user_organization_memberships(user_id, organization_id)
        ON DELETE CASCADE
);

ALTER TABLE user_role_assignments
    ADD COLUMN IF NOT EXISTS active boolean NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS row_version bigint NOT NULL DEFAULT 1
        CHECK (row_version > 0);

CREATE OR REPLACE FUNCTION enforce_user_role_assignment_scope()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_organization_type text;
    target_role_organization_id uuid;
    target_role_identity_kind text;
    target_role_built_in boolean;
    target_role_active boolean;
    target_role_deleted_at timestamptz;
BEGIN
    SELECT lower(org_type)
    INTO target_organization_type
    FROM organizations
    WHERE id = NEW.organization_id
    FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'organization % does not exist', NEW.organization_id
            USING ERRCODE = '23503';
    END IF;
    SELECT organization_id, identity_kind, built_in, active, deleted_at
    INTO target_role_organization_id, target_role_identity_kind,
        target_role_built_in, target_role_active, target_role_deleted_at
    FROM authorization_roles
    WHERE id = NEW.role_id
    FOR SHARE;
    IF NOT FOUND
       OR NOT target_role_active
       OR target_role_deleted_at IS NOT NULL
       OR NOT (target_role_built_in OR target_role_organization_id = NEW.organization_id)
       OR target_role_identity_kind = 'internal_service_principal'
       OR NOT (
           target_role_identity_kind IS NULL
           OR target_role_identity_kind IN ('platform_administrator', 'operational_user')
           OR (
               target_role_identity_kind = 'sponsor_cro_administrator'
               AND target_organization_type = 'cro'
           )
           OR (
               target_role_identity_kind = 'sponsor_company_administrator'
               AND target_organization_type = 'pharmaceutical_company'
           )
       ) THEN
        RAISE EXCEPTION 'role % cannot be assigned in organization %', NEW.role_id, NEW.organization_id
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS user_role_assignments_scope_guard ON user_role_assignments;
CREATE TRIGGER user_role_assignments_scope_guard
BEFORE INSERT OR UPDATE ON user_role_assignments
FOR EACH ROW EXECUTE FUNCTION enforce_user_role_assignment_scope();

CREATE TABLE IF NOT EXISTS organization_policy_state (
    organization_id uuid PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS principal_authorization_state (
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, organization_id),
    FOREIGN KEY (user_id, organization_id)
        REFERENCES user_organization_memberships(user_id, organization_id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS authorization_migration_rejections (
    id bigserial PRIMARY KEY,
    observed_at timestamptz NOT NULL DEFAULT now(),
    user_id uuid,
    organization_id uuid,
    legacy_role text,
    reason text NOT NULL,
    resolved boolean NOT NULL DEFAULT false
);

CREATE UNIQUE INDEX IF NOT EXISTS authorization_migration_rejections_unresolved
    ON authorization_migration_rejections (
        user_id, organization_id, legacy_role, reason
    ) NULLS NOT DISTINCT
    WHERE NOT resolved;

CREATE TABLE IF NOT EXISTS authorization_migration_reconciliations (
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    legacy_role text NOT NULL,
    normalized_role_id uuid NOT NULL,
    legacy_effective_access jsonb NOT NULL,
    normalized_effective_access jsonb NOT NULL,
    evidence_hash text NOT NULL CHECK (evidence_hash ~ '^[0-9a-f]{64}$'),
    proof_hash text CHECK (proof_hash IS NULL OR proof_hash ~ '^[0-9a-f]{64}$'),
    equivalent boolean,
    comparison_status text NOT NULL CHECK (comparison_status IN (
        'pending_action_binding', 'proven_equivalent', 'proven_different'
    )),
    reconciled_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, organization_id),
    CHECK (
        (comparison_status = 'pending_action_binding' AND equivalent IS NULL AND proof_hash IS NULL)
        OR (comparison_status = 'proven_equivalent' AND equivalent IS TRUE AND proof_hash IS NOT NULL)
        OR (comparison_status = 'proven_different' AND equivalent IS FALSE AND proof_hash IS NOT NULL)
    )
);

GRANT SELECT ON authorization_catalog_state, authorization_grant_catalog,
    authorization_grant_role_classes, authorization_roles, role_grants,
    user_role_assignments, organization_policy_state,
    principal_authorization_state TO e2br3_app_role;
REVOKE INSERT, UPDATE, DELETE ON authorization_catalog_state,
    authorization_grant_catalog, authorization_grant_role_classes,
    authorization_roles, role_grants, user_role_assignments,
    organization_policy_state, principal_authorization_state,
    authorization_migration_rejections,
    authorization_migration_reconciliations FROM e2br3_app_role;
REVOKE USAGE, SELECT ON SEQUENCE authorization_migration_rejections_id_seq
    FROM e2br3_app_role;
-- Runtime request connections are read-only over normalized authorization
-- storage during the kernel cutover. Narrow typed mutation entry points are
-- introduced with role administration; registry reconciliation uses the
-- separate migration credential and never SET ROLE e2br3_app_role.
