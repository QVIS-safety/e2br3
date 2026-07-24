-- Operational PDF grants authorize their registered actions without requiring
-- a built-in administrator identity. Role-assignment actions require both the
-- operational Admin Edit grant and a built-in administrator identity.
UPDATE authorization_catalog_state
SET schema_version = 2,
    catalog_hash = '6d3135091fbb99216747a104abb5d90e460d732b5e73c5845a621f075d602504',
    reconciled_at = now()
WHERE singleton
  AND catalog_hash = '0f0ee103d4ebf9f448c16d68cf5a7e11cfa8c08b4f723845dfc7db44764c66eb';

-- Creating a user without an explicit role is not privileged role
-- administration. This narrowly provisions the built-in operational role for
-- a membership that has no existing assignment; it cannot change an existing
-- user's role.
CREATE OR REPLACE FUNCTION authz_assign_baseline_user_role(
    target_user_id uuid,
    target_organization_id uuid
) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
BEGIN
    INSERT INTO user_role_assignments (
        user_id, organization_id, role_id, active, assigned_at
    )
    SELECT
        target_user_id,
        target_organization_id,
        role.id,
        true,
        now()
    FROM authorization_roles role
    WHERE role.identity_kind = 'operational_user'
      AND role.built_in
      AND role.active
      AND role.deleted_at IS NULL
      AND NOT EXISTS (
          SELECT 1
          FROM user_role_assignments existing
          WHERE existing.user_id = target_user_id
            AND existing.organization_id = target_organization_id
      );
    IF NOT FOUND THEN
        RAISE EXCEPTION 'baseline role cannot be assigned to user % in organization %',
            target_user_id, target_organization_id USING ERRCODE = '23514';
    END IF;
END;
$$;

REVOKE ALL ON FUNCTION authz_assign_baseline_user_role(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION authz_assign_baseline_user_role(uuid, uuid)
    TO e2br3_app_role;
