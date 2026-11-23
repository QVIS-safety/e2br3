CREATE OR REPLACE FUNCTION app_settings_default_value_for_org(p_org_type TEXT)
RETURNS JSONB
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    sponsor_admin_role TEXT;
BEGIN
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

WITH corrected AS (
    SELECT
        settings.organization_id,
        jsonb_set(
            settings.value,
            '{workflow,statuses}',
            jsonb_agg(
                CASE
                    WHEN jsonb_typeof(status.value->'allowed_roles') = 'array' THEN
                        jsonb_set(
                            status.value,
                            '{allowed_roles}',
                            COALESCE((
                                SELECT jsonb_agg(
                                    CASE
                                        WHEN role.value = to_jsonb(
                                            CASE lower(trim(organization.org_type))
                                                WHEN 'cro' THEN 'sponsor_admin_company'
                                                WHEN 'pharmaceutical_company' THEN 'sponsor_admin_cro'
                                            END
                                        ) THEN to_jsonb(
                                            CASE lower(trim(organization.org_type))
                                                WHEN 'cro' THEN 'sponsor_admin_cro'
                                                WHEN 'pharmaceutical_company' THEN 'sponsor_admin_company'
                                            END
                                        )
                                        ELSE role.value
                                    END
                                    ORDER BY role.ordinality
                                )
                                FROM jsonb_array_elements(status.value->'allowed_roles')
                                    WITH ORDINALITY AS role(value, ordinality)
                            ), '[]'::jsonb),
                            false
                        )
                    ELSE status.value
                END
                ORDER BY status.ordinality
            ),
            false
        ) AS value
    FROM app_settings AS settings
    JOIN organizations AS organization
      ON organization.id = settings.organization_id
    CROSS JOIN LATERAL jsonb_array_elements(
        settings.value#>'{workflow,statuses}'
    ) WITH ORDINALITY AS status(value, ordinality)
    WHERE settings.key = 'system'
      AND lower(trim(organization.org_type)) IN ('cro', 'pharmaceutical_company')
      AND jsonb_typeof(settings.value#>'{workflow,statuses}') = 'array'
    GROUP BY settings.organization_id, settings.value
)
UPDATE app_settings AS settings
SET value = corrected.value,
    updated_at = now()
FROM corrected
WHERE settings.organization_id = corrected.organization_id
  AND settings.key = 'system'
  AND settings.value IS DISTINCT FROM corrected.value;
