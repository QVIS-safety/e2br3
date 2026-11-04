CREATE OR REPLACE FUNCTION authz_lock_organization_policy_revision(
	target_organization_id uuid
) RETURNS bigint
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
DECLARE
	request_user_id uuid;
	request_organization_id uuid;
	locked_revision bigint;
BEGIN
	request_user_id :=
		NULLIF(current_setting('app.current_user_id', true), '')::uuid;
	request_organization_id :=
		NULLIF(current_setting('app.current_organization_id', true), '')::uuid;
	IF request_user_id IS NULL
	   OR request_organization_id IS DISTINCT FROM target_organization_id THEN
		RAISE EXCEPTION 'organization revision lock is not bound to this request'
			USING ERRCODE = '42501';
	END IF;

	SELECT revision
	  INTO locked_revision
	  FROM organization_policy_state
	 WHERE organization_id = target_organization_id
	 FOR UPDATE;
	IF NOT FOUND THEN
		RAISE EXCEPTION 'organization authorization revision is unavailable'
			USING ERRCODE = 'P0002';
	END IF;

	RETURN locked_revision;
END;
$$;

REVOKE ALL ON FUNCTION authz_lock_organization_policy_revision(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION authz_lock_organization_policy_revision(uuid)
	TO e2br3_app_role;
