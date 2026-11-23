-- The platform organization has no tenant-specific sponsor role.
CREATE OR REPLACE FUNCTION app_settings_default_value_for_org(p_org_type TEXT)
RETURNS JSONB
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    sponsor_admin_role TEXT;
BEGIN
    IF lower(trim(p_org_type)) = 'internal' THEN
        RETURN app_settings_default_value();
    END IF;
    sponsor_admin_role := CASE lower(trim(p_org_type))
        WHEN 'cro' THEN 'sponsor_admin_cro'
        WHEN 'pharmaceutical_company' THEN 'sponsor_admin_company'
        ELSE NULL
    END;
    IF sponsor_admin_role IS NULL THEN
        RAISE EXCEPTION 'unsupported organization type for app settings: %', p_org_type
            USING ERRCODE = '22023';
    END IF;
    RETURN jsonb_set(
        app_settings_default_value(),
        '{workflow,statuses,0,allowed_roles}',
        jsonb_build_array(sponsor_admin_role),
        false
    );
END;
$$;
