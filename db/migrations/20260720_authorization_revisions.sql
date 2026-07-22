CREATE OR REPLACE FUNCTION authz_touch_organization_revision(
    target_schema text,
    target_organization_id uuid
) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    IF target_organization_id IS NULL THEN
        EXECUTE format(
            'UPDATE %I.organization_policy_state SET revision = revision + 1, updated_at = now()',
            target_schema
        );
    ELSE
        EXECUTE format(
            'INSERT INTO %I.organization_policy_state (organization_id, revision, updated_at) VALUES ($1, 1, now()) ON CONFLICT (organization_id) DO UPDATE SET revision = %I.organization_policy_state.revision + 1, updated_at = now()',
            target_schema,
            target_schema
        ) USING target_organization_id;
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION authz_touch_principal_revision(
    target_schema text,
    target_user_id uuid,
    target_organization_id uuid
) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    EXECUTE format(
        'INSERT INTO %I.principal_authorization_state (user_id, organization_id, revision, updated_at) VALUES ($1, $2, 1, now()) ON CONFLICT (user_id, organization_id) DO UPDATE SET revision = %I.principal_authorization_state.revision + 1, updated_at = now()',
        target_schema,
        target_schema
    ) USING target_user_id, target_organization_id;
END;
$$;

CREATE OR REPLACE FUNCTION authz_initialize_organization_revision()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    EXECUTE format(
		'INSERT INTO %I.organization_policy_state (organization_id, organization_type) VALUES ($1, lower($2)) ON CONFLICT (organization_id) DO UPDATE SET organization_type = EXCLUDED.organization_type',
        TG_TABLE_SCHEMA
    ) USING NEW.id, NEW.org_type;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS authz_initialize_organization_revision ON organizations;
CREATE TRIGGER authz_initialize_organization_revision
AFTER INSERT ON organizations
FOR EACH ROW EXECUTE FUNCTION authz_initialize_organization_revision();

CREATE OR REPLACE FUNCTION authz_revision_organization_organizations()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
BEGIN
    IF ROW(OLD.active, OLD.org_type) IS DISTINCT FROM ROW(NEW.active, NEW.org_type) THEN
		EXECUTE format(
			'UPDATE %I.organization_policy_state SET organization_type = lower($2), updated_at = now() WHERE organization_id = $1',
			TG_TABLE_SCHEMA
		) USING NEW.id, NEW.org_type;
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, NEW.id;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS authz_revision_organization_organizations ON organizations;
CREATE TRIGGER authz_revision_organization_organizations
AFTER UPDATE OF active, org_type ON organizations
FOR EACH ROW EXECUTE FUNCTION authz_revision_organization_organizations();

CREATE OR REPLACE FUNCTION authz_guard_organization_role_compatibility()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    incompatible_assignment_exists boolean;
	locked_organization_id uuid;
BEGIN
    IF OLD.org_type IS NOT DISTINCT FROM NEW.org_type THEN
        RETURN NEW;
    END IF;
	EXECUTE format(
		'SELECT organization_id FROM %I.organization_policy_state WHERE organization_id = $1 FOR UPDATE',
		TG_TABLE_SCHEMA
	) INTO locked_organization_id USING NEW.id;
	IF locked_organization_id IS NULL THEN
		RAISE EXCEPTION 'authorization state for organization % does not exist', NEW.id
			USING ERRCODE = '23503';
	END IF;
    EXECUTE format(
        'SELECT EXISTS (SELECT 1 FROM %I.user_role_assignments a JOIN %I.authorization_roles r ON r.id = a.role_id WHERE a.organization_id = $1 AND a.active AND ((r.identity_kind = ''sponsor_cro_administrator'' AND lower(coalesce($2, '''')) <> ''cro'') OR (r.identity_kind = ''sponsor_company_administrator'' AND lower(coalesce($2, '''')) <> ''pharmaceutical_company'')))',
        TG_TABLE_SCHEMA,
        TG_TABLE_SCHEMA
    ) INTO incompatible_assignment_exists USING NEW.id, NEW.org_type;
    IF incompatible_assignment_exists THEN
        RAISE EXCEPTION 'organization type is incompatible with an active sponsor administrator assignment'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS authz_guard_organization_role_compatibility ON organizations;
CREATE TRIGGER authz_guard_organization_role_compatibility
BEFORE UPDATE OF org_type ON organizations
FOR EACH ROW EXECUTE FUNCTION authz_guard_organization_role_compatibility();

CREATE OR REPLACE FUNCTION authz_initialize_membership_revision()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    EXECUTE format(
        'INSERT INTO %I.principal_authorization_state (user_id, organization_id) VALUES ($1, $2) ON CONFLICT (user_id, organization_id) DO NOTHING',
        TG_TABLE_SCHEMA
    ) USING NEW.user_id, NEW.organization_id;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS authz_initialize_membership_revision ON user_organization_memberships;
CREATE TRIGGER authz_initialize_membership_revision
AFTER INSERT ON user_organization_memberships
FOR EACH ROW EXECUTE FUNCTION authz_initialize_membership_revision();

CREATE OR REPLACE FUNCTION authz_increment_role_row_version()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF ROW(OLD.organization_id, OLD.identity_kind, OLD.active, OLD.built_in, OLD.deleted_at, OLD.role_class)
       IS DISTINCT FROM ROW(NEW.organization_id, NEW.identity_kind, NEW.active, NEW.built_in, NEW.deleted_at, NEW.role_class) THEN
        NEW.row_version := OLD.row_version + 1;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS authz_increment_role_row_version ON authorization_roles;
CREATE TRIGGER authz_increment_role_row_version
BEFORE UPDATE OF organization_id, identity_kind, active, built_in, deleted_at, role_class ON authorization_roles
FOR EACH ROW EXECUTE FUNCTION authz_increment_role_row_version();

CREATE OR REPLACE FUNCTION authz_revision_organization_authorization_roles()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
DECLARE
    old_organization_id uuid;
    new_organization_id uuid;
BEGIN
    IF TG_OP = 'UPDATE' AND ROW(OLD.organization_id, OLD.identity_kind, OLD.active, OLD.built_in, OLD.deleted_at, OLD.role_class, OLD.row_version)
        IS NOT DISTINCT FROM ROW(NEW.organization_id, NEW.identity_kind, NEW.active, NEW.built_in, NEW.deleted_at, NEW.role_class, NEW.row_version) THEN
        RETURN NEW;
    END IF;
    old_organization_id := CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE OLD.organization_id END;
    new_organization_id := CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.organization_id END;
    IF TG_OP <> 'INSERT' THEN
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, old_organization_id;
    END IF;
    IF TG_OP = 'INSERT' THEN
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, new_organization_id;
    ELSIF TG_OP = 'UPDATE' AND new_organization_id IS DISTINCT FROM old_organization_id THEN
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, new_organization_id;
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

DROP TRIGGER IF EXISTS authz_revision_organization_authorization_roles ON authorization_roles;
CREATE TRIGGER authz_revision_organization_authorization_roles
AFTER INSERT OR DELETE OR UPDATE OF organization_id, identity_kind, active, built_in, deleted_at, role_class, row_version
ON authorization_roles
FOR EACH ROW EXECUTE FUNCTION authz_revision_organization_authorization_roles();

CREATE OR REPLACE FUNCTION authz_revision_organization_role_grants()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
DECLARE
    old_organization_id uuid;
    new_organization_id uuid;
BEGIN
    IF TG_OP = 'UPDATE' AND ROW(OLD.role_id, OLD.grant_id)
        IS NOT DISTINCT FROM ROW(NEW.role_id, NEW.grant_id) THEN
        RETURN NEW;
    END IF;
    IF TG_OP <> 'INSERT' THEN
        EXECUTE format('SELECT organization_id FROM %I.authorization_roles WHERE id = $1', TG_TABLE_SCHEMA)
            INTO old_organization_id USING OLD.role_id;
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, old_organization_id;
    END IF;
    IF TG_OP = 'INSERT' THEN
        EXECUTE format('SELECT organization_id FROM %I.authorization_roles WHERE id = $1', TG_TABLE_SCHEMA)
            INTO new_organization_id USING NEW.role_id;
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, new_organization_id;
    ELSIF TG_OP = 'UPDATE' AND NEW.role_id IS DISTINCT FROM OLD.role_id THEN
        EXECUTE format('SELECT organization_id FROM %I.authorization_roles WHERE id = $1', TG_TABLE_SCHEMA)
            INTO new_organization_id USING NEW.role_id;
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, new_organization_id;
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

DROP TRIGGER IF EXISTS authz_revision_organization_role_grants ON role_grants;
CREATE TRIGGER authz_revision_organization_role_grants
BEFORE INSERT OR DELETE OR UPDATE OF role_id, grant_id ON role_grants
FOR EACH ROW EXECUTE FUNCTION authz_revision_organization_role_grants();

CREATE OR REPLACE FUNCTION authz_revision_organization_scope_definition()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
DECLARE
    old_organization_id uuid;
    new_organization_id uuid;
BEGIN
    IF TG_OP = 'UPDATE' AND ROW(OLD.organization_id, OLD.deleted)
        IS NOT DISTINCT FROM ROW(NEW.organization_id, NEW.deleted) THEN
        RETURN NEW;
    END IF;
    old_organization_id := CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE OLD.organization_id END;
    new_organization_id := CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.organization_id END;
    IF TG_OP <> 'INSERT' THEN
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, old_organization_id;
    END IF;
    IF TG_OP = 'INSERT' THEN
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, new_organization_id;
    ELSIF TG_OP = 'UPDATE' AND new_organization_id IS DISTINCT FROM old_organization_id THEN
        EXECUTE format('SELECT %I.authz_touch_organization_revision($1, $2)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, new_organization_id;
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

DROP TRIGGER IF EXISTS authz_revision_organization_sender_presaves ON sender_presaves;
CREATE TRIGGER authz_revision_organization_sender_presaves
AFTER INSERT OR DELETE OR UPDATE OF deleted, organization_id ON sender_presaves
FOR EACH ROW EXECUTE FUNCTION authz_revision_organization_scope_definition();

DROP TRIGGER IF EXISTS authz_revision_organization_product_presaves ON product_presaves;
CREATE TRIGGER authz_revision_organization_product_presaves
AFTER INSERT OR DELETE OR UPDATE OF deleted, organization_id ON product_presaves
FOR EACH ROW EXECUTE FUNCTION authz_revision_organization_scope_definition();

DROP TRIGGER IF EXISTS authz_revision_organization_study_presaves ON study_presaves;
CREATE TRIGGER authz_revision_organization_study_presaves
AFTER INSERT OR DELETE OR UPDATE OF deleted, organization_id ON study_presaves
FOR EACH ROW EXECUTE FUNCTION authz_revision_organization_scope_definition();

CREATE OR REPLACE FUNCTION authz_revision_principal_users()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
BEGIN
    IF ROW(OLD.active, OLD.access_start_at, OLD.access_end_at, OLD.access_sender_ids,
        OLD.access_product_ids, OLD.access_study_ids, OLD.access_blind_allowed,
        OLD.active_sender_identifier)
       IS NOT DISTINCT FROM
       ROW(NEW.active, NEW.access_start_at, NEW.access_end_at, NEW.access_sender_ids,
        NEW.access_product_ids, NEW.access_study_ids, NEW.access_blind_allowed,
        NEW.active_sender_identifier) THEN
        RETURN NEW;
    END IF;
    EXECUTE format(
        'INSERT INTO %I.principal_authorization_state (user_id, organization_id, revision, updated_at) SELECT m.user_id, m.organization_id, 1, now() FROM %I.user_organization_memberships m WHERE m.user_id = $1 ON CONFLICT (user_id, organization_id) DO UPDATE SET revision = %I.principal_authorization_state.revision + 1, updated_at = now()',
        TG_TABLE_SCHEMA,
        TG_TABLE_SCHEMA,
        TG_TABLE_SCHEMA
    ) USING NEW.id;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS authz_revision_principal_users ON users;
CREATE TRIGGER authz_revision_principal_users
AFTER UPDATE OF active, access_start_at, access_end_at, access_sender_ids,
    access_product_ids, access_study_ids, access_blind_allowed, active_sender_identifier
ON users
FOR EACH ROW EXECUTE FUNCTION authz_revision_principal_users();

CREATE OR REPLACE FUNCTION authz_revision_principal_user_organization_memberships()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND ROW(OLD.user_id, OLD.organization_id, OLD.active)
        IS NOT DISTINCT FROM ROW(NEW.user_id, NEW.organization_id, NEW.active) THEN
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' THEN
		EXECUTE format('SELECT %I.authz_touch_principal_revision($1, $2, $3)', TG_TABLE_SCHEMA)
			USING TG_TABLE_SCHEMA, OLD.user_id, OLD.organization_id;
		IF ROW(OLD.user_id, OLD.organization_id) IS DISTINCT FROM ROW(NEW.user_id, NEW.organization_id) THEN
        EXECUTE format('SELECT %I.authz_touch_principal_revision($1, $2, $3)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, NEW.user_id, NEW.organization_id;
		END IF;
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

DROP TRIGGER IF EXISTS authz_revision_principal_user_organization_memberships ON user_organization_memberships;
CREATE TRIGGER authz_revision_principal_user_organization_memberships
AFTER UPDATE OF user_id, active, organization_id ON user_organization_memberships
FOR EACH ROW EXECUTE FUNCTION authz_revision_principal_user_organization_memberships();

CREATE OR REPLACE FUNCTION authz_increment_assignment_row_version()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF ROW(OLD.user_id, OLD.organization_id, OLD.role_id, OLD.active)
       IS DISTINCT FROM ROW(NEW.user_id, NEW.organization_id, NEW.role_id, NEW.active) THEN
        NEW.row_version := OLD.row_version + 1;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS authz_increment_assignment_row_version ON user_role_assignments;
CREATE TRIGGER authz_increment_assignment_row_version
BEFORE UPDATE OF user_id, organization_id, role_id, active ON user_role_assignments
FOR EACH ROW EXECUTE FUNCTION authz_increment_assignment_row_version();

CREATE OR REPLACE FUNCTION authz_revision_principal_user_role_assignments()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
DECLARE
    target_user_id uuid;
    target_organization_id uuid;
BEGIN
    IF TG_OP = 'UPDATE' AND ROW(OLD.role_id, OLD.active, OLD.row_version)
        IS NOT DISTINCT FROM ROW(NEW.role_id, NEW.active, NEW.row_version) THEN
        RETURN NEW;
    END IF;
    IF TG_OP <> 'INSERT' THEN
        EXECUTE format('SELECT %I.authz_touch_principal_revision($1, $2, $3)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, OLD.user_id, OLD.organization_id;
    END IF;
    IF TG_OP = 'INSERT' THEN
        target_user_id := NEW.user_id;
        target_organization_id := NEW.organization_id;
        EXECUTE format('SELECT %I.authz_touch_principal_revision($1, $2, $3)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, target_user_id, target_organization_id;
    ELSIF TG_OP = 'UPDATE' AND ROW(NEW.user_id, NEW.organization_id)
        IS DISTINCT FROM ROW(OLD.user_id, OLD.organization_id) THEN
        target_user_id := NEW.user_id;
        target_organization_id := NEW.organization_id;
        EXECUTE format('SELECT %I.authz_touch_principal_revision($1, $2, $3)', TG_TABLE_SCHEMA)
            USING TG_TABLE_SCHEMA, target_user_id, target_organization_id;
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

DROP TRIGGER IF EXISTS authz_revision_principal_user_role_assignments ON user_role_assignments;
CREATE TRIGGER authz_revision_principal_user_role_assignments
AFTER INSERT OR DELETE OR UPDATE OF user_id, organization_id, role_id, active, row_version ON user_role_assignments
FOR EACH ROW EXECUTE FUNCTION authz_revision_principal_user_role_assignments();

REVOKE ALL ON FUNCTION authz_touch_organization_revision(text, uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_touch_principal_revision(text, uuid, uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_initialize_organization_revision() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_revision_organization_organizations() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_guard_organization_role_compatibility() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_initialize_membership_revision() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_increment_role_row_version() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_revision_organization_authorization_roles() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_revision_organization_role_grants() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_revision_organization_scope_definition() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_revision_principal_users() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_revision_principal_user_organization_memberships() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_increment_assignment_row_version() FROM PUBLIC;
REVOKE ALL ON FUNCTION authz_revision_principal_user_role_assignments() FROM PUBLIC;
