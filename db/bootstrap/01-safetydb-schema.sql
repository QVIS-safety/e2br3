-- Frozen PostgreSQL 16 schema: actual server snapshot plus all changes through 2026-11-27.
--
-- PostgreSQL database dump
--



SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;

--
-- Name: public; Type: SCHEMA; Schema: -; Owner: -
--



--
-- Name: SCHEMA public; Type: COMMENT; Schema: -; Owner: -
--

COMMENT ON SCHEMA public IS 'standard public schema';


--
-- Name: app_settings_default_value(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.app_settings_default_value() RETURNS jsonb
    LANGUAGE sql STABLE
    AS $_$
    WITH default_meddra AS (
        SELECT
            CASE language
                WHEN 'en' THEN 'English'
                WHEN 'ko' THEN 'Korean'
                ELSE initcap(language)
            END AS language_label,
            version
        FROM terminology_releases
        WHERE dictionary = 'meddra'
          AND status = 'active'
        ORDER BY
            (language = 'en' AND version = '28.1') DESC,
            (language = 'en') DESC,
            CASE
                WHEN version ~ '^[0-9]+(\.[0-9]+)*$'
                    THEN string_to_array(version, '.')::int[]
                ELSE ARRAY[]::int[]
            END DESC,
            activated_at DESC NULLS LAST,
            updated_at DESC
        LIMIT 1
    )
    SELECT jsonb_build_object(
        'timezone', 'Asia/Seoul',
        'meddra_language', COALESCE(
            (SELECT language_label FROM default_meddra),
            'English'
        ),
        'meddra_version', COALESCE(
            (SELECT version FROM default_meddra),
            '28.1'
        ),
        'idf_version', '3.0',
        'orientation', 'Landscape',
        'data_ordering', 'Primary data will appear first',
        'upload_excel_template_without_element_label', false,
        'notation', false,
        'apply_comments_on_exported_xml', false,
        'apply_sender_info_to_imported_cases', false,
        'import_date_update', jsonb_build_object(
            'date_of_creation', false,
            'most_recent_info_date', false,
            'report_first_received_date', false
        ),
        'appendices', jsonb_build_array('ICH'),
        'case_number_setting', 'AE Row No.',
        'case_number_identifier', 'ICSR',
        'case_number_padding', 6,
        'case_number_sequence_condition', 'Per sender',
        'case_number_format_fields', jsonb_build_array('AE Row No.'),
        'workflow_enabled', false,
        'workflow', jsonb_build_object(
            'statuses', jsonb_build_array(jsonb_build_object(
                'name', 'Saved',
                'editable', true,
                'description', 'Default state',
                'due_days', 0,
                'allowed_roles', jsonb_build_array('sponsor_admin_cro')
            ))
        ),
        'idle_session_minutes', 60,
        'session_warning_minutes', 5
    );
$_$;


--
-- Name: app_settings_default_value_for_org(text); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.app_settings_default_value_for_org(p_org_type text) RETURNS jsonb
    LANGUAGE plpgsql STABLE
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


--
-- Name: append_audit_log(text, uuid, uuid, text, jsonb, jsonb, jsonb); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.append_audit_log(p_table_name text, p_record_id uuid, p_organization_id uuid, p_action text, p_old_values jsonb, p_new_values jsonb, p_changed_fields jsonb) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public'
    AS $$
DECLARE
    v_user_id UUID;
BEGIN
    v_user_id := get_current_user_context();

    INSERT INTO audit_logs (
        table_name,
        record_id,
        organization_id,
        action,
        user_id,
        reason_for_change,
        change_category,
        e_signature_id,
        old_values,
        new_values,
        changed_fields
    )
    VALUES (
        p_table_name,
        p_record_id,
        p_organization_id,
        p_action,
        v_user_id,
        get_current_change_reason(),
        get_current_change_category(),
        get_current_esignature_id(),
        p_old_values,
        p_new_values,
        p_changed_fields
    );
EXCEPTION
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Audit trail logging failed for table %.%: %. User context may not be set.',
            p_table_name, p_record_id, SQLERRM;
END;
$$;


--
-- Name: audit_jsonb_is_distinct(jsonb, jsonb); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.audit_jsonb_is_distinct(p_old jsonb, p_new jsonb) RETURNS boolean
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
    SELECT CASE
        WHEN pg_catalog.jsonb_hash_extended(p_old, 0)
             IS DISTINCT FROM pg_catalog.jsonb_hash_extended(p_new, 0)
            THEN TRUE
        ELSE p_old IS DISTINCT FROM p_new
    END;
$$;


--
-- Name: audit_log_organization_id(text, uuid, jsonb, jsonb); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.audit_log_organization_id(p_table_name text, p_record_id uuid, p_old_values jsonb, p_new_values jsonb) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public'
    AS $$
DECLARE
    v_values JSONB;
    v_org_id UUID;
    v_case_id UUID;
    v_submission_id UUID;
    v_drug_id UUID;
BEGIN
    IF p_table_name = 'organizations' THEN
        RETURN p_record_id;
    END IF;

    v_values := COALESCE(p_new_values, p_old_values, '{}'::JSONB);
    v_org_id := NULLIF(v_values->>'organization_id', '')::UUID;
    IF v_org_id IS NOT NULL THEN
        RETURN v_org_id;
    END IF;

    v_case_id := NULLIF(v_values->>'case_id', '')::UUID;
    IF v_case_id IS NOT NULL THEN
        SELECT c.organization_id INTO v_org_id
        FROM cases c
        WHERE c.id = v_case_id;

        IF v_org_id IS NOT NULL THEN
            RETURN v_org_id;
        END IF;
    END IF;

    v_submission_id := NULLIF(v_values->>'submission_id', '')::UUID;
    IF v_submission_id IS NOT NULL THEN
        SELECT c.organization_id INTO v_org_id
        FROM case_submissions cs
        JOIN cases c ON c.id = cs.case_id
        WHERE cs.id = v_submission_id;

        IF v_org_id IS NOT NULL THEN
            RETURN v_org_id;
        END IF;
    END IF;

    IF p_table_name = 'drug_device_characteristics' THEN
        v_drug_id := NULLIF(v_values->>'drug_id', '')::UUID;
        SELECT c.organization_id INTO v_org_id
        FROM drug_information d
        JOIN cases c ON c.id = d.case_id
        WHERE d.id = v_drug_id;

        IF v_org_id IS NOT NULL THEN
            RETURN v_org_id;
        END IF;

        -- A cascading DELETE can remove the parent before this AFTER trigger
        -- runs. Resolve only from immutable parent audits written while the
        -- live parent chain still established the tenant.
        IF p_new_values IS NULL AND p_old_values IS NOT NULL THEN
            SELECT a.organization_id INTO v_org_id
            FROM audit_logs a
            WHERE a.table_name = 'drug_information'
              AND a.record_id = v_drug_id
              AND a.action IN ('CREATE', 'UPDATE')
            ORDER BY a.id DESC
            LIMIT 1;

            IF v_org_id IS NOT NULL THEN
                RETURN v_org_id;
            END IF;
        END IF;

        RAISE EXCEPTION USING
            ERRCODE = '23503',
            MESSAGE = 'drug device characteristic audit parent is missing',
            DETAIL = format(
                'drug_device_characteristics:%s drug_id:%s',
                p_record_id,
                COALESCE(v_values->>'drug_id', '<missing>')
            );
    END IF;

    IF p_table_name IN (
        'sender_presave_gateways',
        'sender_presave_responsible_persons'
    ) THEN
        SELECT p.organization_id INTO v_org_id
        FROM sender_presaves p
        WHERE p.id = NULLIF(v_values->>'sender_presave_id', '')::UUID;

        IF v_org_id IS NOT NULL THEN
            RETURN v_org_id;
        END IF;
    END IF;

    IF p_table_name IN (
        'receiver_presave_consignees',
        'receiver_presave_routes'
    ) THEN
        SELECT p.organization_id INTO v_org_id
        FROM receiver_presaves p
        WHERE p.id = NULLIF(v_values->>'receiver_presave_id', '')::UUID;

        IF v_org_id IS NOT NULL THEN
            RETURN v_org_id;
        END IF;
    END IF;

	IF p_table_name = 'product_presave_active_substances' THEN
        SELECT p.organization_id INTO v_org_id
        FROM product_presaves p
        WHERE p.id = NULLIF(v_values->>'product_presave_id', '')::UUID;

        IF v_org_id IS NOT NULL THEN
            RETURN v_org_id;
        END IF;
    END IF;

    IF p_table_name IN (
        'study_presave_registration_numbers',
        'study_presave_fda_cross_reported_ind_numbers',
        'study_presave_products',
        'study_presave_reporters'
    ) THEN
        SELECT p.organization_id INTO v_org_id
        FROM study_presaves p
        WHERE p.id = NULLIF(v_values->>'study_presave_id', '')::UUID;

        IF v_org_id IS NOT NULL THEN
            RETURN v_org_id;
        END IF;
    END IF;

    RETURN COALESCE(
        current_organization_id(),
        '00000000-0000-0000-0000-000000000000'::UUID
    );
END;
$$;


--
-- Name: audit_logs_hash_chain_before_insert(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.audit_logs_hash_chain_before_insert() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $_$
DECLARE
    v_prev_hash TEXT;
    v_payload TEXT;
BEGIN
    IF NEW.created_at IS NULL THEN
        RAISE EXCEPTION 'audit created_at must not be null'
            USING ERRCODE = '23502';
    END IF;

    PERFORM pg_advisory_xact_lock(
        hashtextextended('e2br3.audit_logs.hash_chain', 0)
    );

    SELECT entry_hash INTO v_prev_hash
      FROM public.audit_logs
     ORDER BY id DESC
     LIMIT 1;

    IF NOT FOUND THEN
        v_prev_hash := repeat('0', 64);
    ELSIF v_prev_hash IS NULL OR v_prev_hash !~ '^[0-9A-Fa-f]{64}$' THEN
        RAISE EXCEPTION 'audit chain head hash is invalid'
            USING ERRCODE = '55000';
    END IF;
    NEW.prev_hash := v_prev_hash;
    NEW.hash_version := 2;
    v_payload := jsonb_build_array(
        NEW.hash_version,
        NEW.id,
        NEW.prev_hash,
        NEW.organization_id,
        NEW.table_name,
        NEW.record_id,
        NEW.action,
        NEW.user_id,
        NEW.reason_for_change,
        NEW.change_category,
        NEW.e_signature_id,
        jsonb_build_array(NEW.old_values IS NULL, NEW.old_values),
        jsonb_build_array(NEW.new_values IS NULL, NEW.new_values),
        jsonb_build_array(NEW.changed_fields IS NULL, NEW.changed_fields),
        NEW.ip_address::TEXT,
        NEW.user_agent,
        to_char(NEW.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
    )::TEXT;
    NEW.entry_hash := encode(public.digest(v_payload, 'sha256'), 'hex');
    RETURN NEW;
END;
$_$;


--
-- Name: audit_parent_record_ids(jsonb, jsonb); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.audit_parent_record_ids(p_old_values jsonb, p_new_values jsonb) RETURNS uuid[]
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    SET search_path TO 'pg_catalog'
    AS $_$
    SELECT ARRAY(
        SELECT DISTINCT field.value::UUID
        FROM pg_catalog.jsonb_each_text(
            COALESCE(p_new_values, '{}'::JSONB)
            || COALESCE(p_old_values, '{}'::JSONB)
        ) AS field
        WHERE field.key = ANY(ARRAY[
            'case_id', 'death_info_id', 'device_id', 'drug_id',
            'drug_reaction_assessment_id', 'e_signature_id',
            'narrative_id', 'parent_id', 'patient_id', 'reaction_id',
            'study_information_id', 'submission_id'
        ])
          AND field.value ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
    );
$_$;


--
-- Name: audit_trigger_function(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.audit_trigger_function() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public'
    AS $$
DECLARE
    v_old_business JSONB;
    v_new_business JSONB;
    v_changed_fields JSONB;
BEGIN
    -- Get user from context (will fail if not set, ensuring user attribution)
    PERFORM get_current_user_context();

    IF TG_OP = 'INSERT' THEN
        v_changed_fields := compute_audit_changed_fields(NULL, to_jsonb(NEW));
        PERFORM append_audit_log(
            TG_TABLE_NAME,
            NEW.id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.id, NULL, to_jsonb(NEW)),
            'CREATE',
            NULL,
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'UPDATE' THEN
        -- Ignore metadata-only updates (updated_at/updated_by) to avoid noisy
        -- audit trails from no-op saves.
        v_old_business := to_jsonb(OLD) - 'updated_at' - 'updated_by';
        v_new_business := to_jsonb(NEW) - 'updated_at' - 'updated_by';
        IF NOT audit_jsonb_is_distinct(v_old_business, v_new_business) THEN
            RETURN NEW;
        END IF;

        v_changed_fields := compute_audit_changed_fields(v_old_business, v_new_business);
        PERFORM append_audit_log(
            TG_TABLE_NAME,
            NEW.id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.id, to_jsonb(OLD), to_jsonb(NEW)),
            'UPDATE',
            to_jsonb(OLD),
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'DELETE' THEN
        v_changed_fields := compute_audit_changed_fields(to_jsonb(OLD), NULL);
        PERFORM append_audit_log(
            TG_TABLE_NAME,
            OLD.id,
            audit_log_organization_id(TG_TABLE_NAME, OLD.id, to_jsonb(OLD), NULL),
            'DELETE',
            to_jsonb(OLD),
            NULL,
            v_changed_fields
        );
        RETURN OLD;
    END IF;

EXCEPTION
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Audit trail logging failed for table %.%: %. User context may not be set.',
            TG_TABLE_SCHEMA, TG_TABLE_NAME, SQLERRM;
END;
$$;


--
-- Name: audit_trigger_function_with_audit_id(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.audit_trigger_function_with_audit_id() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public'
    AS $$
DECLARE
    v_old_business JSONB;
    v_new_business JSONB;
    v_changed_fields JSONB;
BEGIN
    PERFORM get_current_user_context();

    IF TG_OP = 'INSERT' THEN
        v_changed_fields := compute_audit_changed_fields(NULL, to_jsonb(NEW));
        PERFORM append_audit_log(
            TG_TABLE_NAME,
            NEW.audit_id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.audit_id, NULL, to_jsonb(NEW)),
            'CREATE',
            NULL,
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'UPDATE' THEN
        -- Ignore metadata-only updates (updated_at/updated_by) to avoid noisy
        -- audit trails from no-op saves.
        v_old_business := to_jsonb(OLD) - 'updated_at' - 'updated_by';
        v_new_business := to_jsonb(NEW) - 'updated_at' - 'updated_by';
        IF NOT audit_jsonb_is_distinct(v_old_business, v_new_business) THEN
            RETURN NEW;
        END IF;

        v_changed_fields := compute_audit_changed_fields(v_old_business, v_new_business);
        PERFORM append_audit_log(
            TG_TABLE_NAME,
            NEW.audit_id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.audit_id, to_jsonb(OLD), to_jsonb(NEW)),
            'UPDATE',
            to_jsonb(OLD),
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'DELETE' THEN
        v_changed_fields := compute_audit_changed_fields(to_jsonb(OLD), NULL);
        PERFORM append_audit_log(
            TG_TABLE_NAME,
            OLD.audit_id,
            audit_log_organization_id(TG_TABLE_NAME, OLD.audit_id, to_jsonb(OLD), NULL),
            'DELETE',
            to_jsonb(OLD),
            NULL,
            v_changed_fields
        );
        RETURN OLD;
    END IF;

EXCEPTION
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Audit trail logging failed for table %.%: %. User context may not be set.',
            TG_TABLE_SCHEMA, TG_TABLE_NAME, SQLERRM;
END;
$$;


--
-- Name: audit_trigger_function_with_submission_id(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.audit_trigger_function_with_submission_id() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public'
    AS $$
DECLARE
    v_old_business JSONB;
    v_new_business JSONB;
    v_changed_fields JSONB;
BEGIN
    PERFORM get_current_user_context();

    IF TG_OP = 'INSERT' THEN
        v_changed_fields := compute_audit_changed_fields(NULL, to_jsonb(NEW));
        PERFORM append_audit_log(
            TG_TABLE_NAME,
            NEW.submission_id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.submission_id, NULL, to_jsonb(NEW)),
            'CREATE',
            NULL,
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'UPDATE' THEN
        v_old_business := to_jsonb(OLD) - 'updated_at' - 'updated_by';
        v_new_business := to_jsonb(NEW) - 'updated_at' - 'updated_by';
        IF NOT audit_jsonb_is_distinct(v_old_business, v_new_business) THEN
            RETURN NEW;
        END IF;

        v_changed_fields := compute_audit_changed_fields(v_old_business, v_new_business);
        PERFORM append_audit_log(
            TG_TABLE_NAME,
            NEW.submission_id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.submission_id, to_jsonb(OLD), to_jsonb(NEW)),
            'UPDATE',
            to_jsonb(OLD),
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'DELETE' THEN
        v_changed_fields := compute_audit_changed_fields(to_jsonb(OLD), NULL);
        PERFORM append_audit_log(
            TG_TABLE_NAME,
            OLD.submission_id,
            audit_log_organization_id(TG_TABLE_NAME, OLD.submission_id, to_jsonb(OLD), NULL),
            'DELETE',
            to_jsonb(OLD),
            NULL,
            v_changed_fields
        );
        RETURN OLD;
    END IF;

EXCEPTION
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Audit trail logging failed for table %.%: %. User context may not be set.',
            TG_TABLE_SCHEMA, TG_TABLE_NAME, SQLERRM;
END;
$$;


--
-- Name: audit_user_display(uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.audit_user_display(p_user_id uuid) RETURNS text
    LANGUAGE plpgsql STABLE
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


--
-- Name: authz_assert_role_administrator(uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_assert_role_administrator(target_organization_id uuid) RETURNS text
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_catalog'
    AS $$
DECLARE
    actor_user_id uuid;
    actor_organization_id uuid;
    actor_identity_kind text;
BEGIN
    actor_user_id := NULLIF(
        current_setting('app.current_user_id', true), ''
    )::uuid;
    actor_organization_id := NULLIF(
        current_setting('app.current_organization_id', true), ''
    )::uuid;
    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION 'authenticated role administrator is required'
            USING ERRCODE = '42501';
    END IF;

    SELECT COALESCE(role.identity_kind, 'custom')
    INTO actor_identity_kind
    FROM user_role_assignments assignment
    JOIN authorization_roles role ON role.id = assignment.role_id
    WHERE assignment.user_id = actor_user_id
      AND assignment.active
      AND role.active
      AND role.deleted_at IS NULL
      AND (
          role.identity_kind = 'platform_administrator'
          OR assignment.organization_id = actor_organization_id
      )
    ORDER BY (role.identity_kind = 'platform_administrator') DESC
    LIMIT 1;

    IF actor_identity_kind = 'platform_administrator' THEN
        RETURN actor_identity_kind;
    END IF;
    IF actor_identity_kind IS NULL
       OR actor_organization_id IS DISTINCT FROM target_organization_id
       OR actor_identity_kind NOT IN (
           'sponsor_cro_administrator',
           'sponsor_company_administrator'
       ) THEN
        RAISE EXCEPTION 'role administration is not allowed for organization %',
            target_organization_id USING ERRCODE = '42501';
    END IF;
    RETURN actor_identity_kind;
END;
$$;


--
-- Name: authz_assign_baseline_user_role(uuid, uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_assign_baseline_user_role(target_user_id uuid, target_organization_id uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_catalog'
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


--
-- Name: authz_assign_user_role(uuid, uuid, uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_assign_user_role(target_user_id uuid, target_organization_id uuid, target_role_id uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_catalog'
    AS $$
DECLARE
    actor_identity_kind text;
    target_identity_kind text;
    target_built_in boolean;
BEGIN
    actor_identity_kind := authz_assert_role_administrator(
        target_organization_id
    );
    SELECT identity_kind, built_in
    INTO target_identity_kind, target_built_in
    FROM authorization_roles
    WHERE id = target_role_id
      AND active
      AND deleted_at IS NULL;
    IF actor_identity_kind <> 'platform_administrator'
       AND target_built_in
       AND target_identity_kind IS DISTINCT FROM 'operational_user' THEN
        RAISE EXCEPTION 'sponsor administrators cannot assign privileged built-in roles'
            USING ERRCODE = '42501';
    END IF;

    INSERT INTO user_role_assignments (
        user_id, organization_id, role_id, active, assigned_at
    )
    SELECT target_user_id, target_organization_id, role.id, true, now()
    FROM authorization_roles role
    WHERE role.id = target_role_id
      AND role.active
      AND role.deleted_at IS NULL
      AND (role.built_in OR role.organization_id = target_organization_id)
    ON CONFLICT (user_id, organization_id) DO UPDATE SET
        role_id = EXCLUDED.role_id,
        active = true,
        assigned_at = CASE
            WHEN user_role_assignments.role_id IS DISTINCT FROM EXCLUDED.role_id
                THEN now()
            ELSE user_role_assignments.assigned_at
        END;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'role % cannot be assigned in organization %',
            target_role_id, target_organization_id USING ERRCODE = '23514';
    END IF;
END;
$$;


--
-- Name: authz_guard_organization_role_compatibility(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_guard_organization_role_compatibility() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
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
$_$;


--
-- Name: authz_increment_assignment_row_version(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_increment_assignment_row_version() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF ROW(OLD.user_id, OLD.organization_id, OLD.role_id, OLD.active)
       IS DISTINCT FROM ROW(NEW.user_id, NEW.organization_id, NEW.role_id, NEW.active) THEN
        NEW.row_version := OLD.row_version + 1;
    END IF;
    RETURN NEW;
END;
$$;


--
-- Name: authz_increment_role_row_version(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_increment_role_row_version() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF ROW(OLD.organization_id, OLD.identity_kind, OLD.active, OLD.built_in, OLD.deleted_at, OLD.role_class)
       IS DISTINCT FROM ROW(NEW.organization_id, NEW.identity_kind, NEW.active, NEW.built_in, NEW.deleted_at, NEW.role_class) THEN
        NEW.row_version := OLD.row_version + 1;
    END IF;
    RETURN NEW;
END;
$$;


--
-- Name: authz_initialize_membership_revision(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_initialize_membership_revision() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
BEGIN
    EXECUTE format(
        'INSERT INTO %I.principal_authorization_state (user_id, organization_id) VALUES ($1, $2) ON CONFLICT (user_id, organization_id) DO NOTHING',
        TG_TABLE_SCHEMA
    ) USING NEW.user_id, NEW.organization_id;
    RETURN NEW;
END;
$_$;


--
-- Name: authz_initialize_organization_revision(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_initialize_organization_revision() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
BEGIN
    EXECUTE format(
		'INSERT INTO %I.organization_policy_state (organization_id, organization_type) VALUES ($1, lower($2)) ON CONFLICT (organization_id) DO UPDATE SET organization_type = EXCLUDED.organization_type',
        TG_TABLE_SCHEMA
    ) USING NEW.id, NEW.org_type;
    RETURN NEW;
END;
$_$;


--
-- Name: authz_lock_organization_policy_revision(uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_lock_organization_policy_revision(target_organization_id uuid) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_catalog'
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


--
-- Name: authz_lock_policy_revisions(uuid, uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_lock_policy_revisions(target_user_id uuid, target_organization_id uuid) RETURNS TABLE(organization_revision bigint, principal_revision bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_catalog'
    AS $$
DECLARE
	request_user_id uuid;
	request_organization_id uuid;
BEGIN
	request_user_id :=
		NULLIF(current_setting('app.current_user_id', true), '')::uuid;
	request_organization_id :=
		NULLIF(current_setting('app.current_organization_id', true), '')::uuid;
	IF request_user_id IS DISTINCT FROM target_user_id
	   OR request_organization_id IS DISTINCT FROM target_organization_id THEN
		RAISE EXCEPTION 'authorization revision lock is not bound to this request'
			USING ERRCODE = '42501';
	END IF;

	RETURN QUERY
	SELECT organization_state.revision, principal_state.revision
	  FROM organization_policy_state organization_state
	  JOIN principal_authorization_state principal_state
	    ON principal_state.organization_id =
	       organization_state.organization_id
	 WHERE organization_state.organization_id = target_organization_id
	   AND principal_state.user_id = target_user_id
	 FOR UPDATE OF organization_state, principal_state;
END;
$$;


--
-- Name: authz_revision_organization_authorization_roles(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_revision_organization_authorization_roles() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
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
$_$;


--
-- Name: authz_revision_organization_organizations(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_revision_organization_organizations() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
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
$_$;


--
-- Name: authz_revision_organization_role_grants(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_revision_organization_role_grants() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
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
$_$;


--
-- Name: authz_revision_organization_scope_definition(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_revision_organization_scope_definition() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
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
$_$;


--
-- Name: authz_revision_principal_user_organization_memberships(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_revision_principal_user_organization_memberships() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
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
$_$;


--
-- Name: authz_revision_principal_user_role_assignments(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_revision_principal_user_role_assignments() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
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
$_$;


--
-- Name: authz_revision_principal_users(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_revision_principal_users() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
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
$_$;


--
-- Name: authz_touch_organization_revision(text, uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_touch_organization_revision(target_schema text, target_organization_id uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
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
$_$;


--
-- Name: authz_touch_principal_revision(text, uuid, uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_touch_principal_revision(target_schema text, target_user_id uuid, target_organization_id uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
BEGIN
    EXECUTE format(
        'INSERT INTO %I.principal_authorization_state (user_id, organization_id, revision, updated_at) VALUES ($1, $2, 1, now()) ON CONFLICT (user_id, organization_id) DO UPDATE SET revision = %I.principal_authorization_state.revision + 1, updated_at = now()',
        target_schema,
        target_schema
    ) USING target_user_id, target_organization_id;
END;
$_$;


--
-- Name: authz_upsert_custom_role(uuid, uuid, text, boolean, text[]); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.authz_upsert_custom_role(target_role_id uuid, target_organization_id uuid, target_name text, target_active boolean, target_grant_ids text[]) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_catalog'
    AS $$
DECLARE
    existing_organization_id uuid;
    existing_built_in boolean;
BEGIN
	PERFORM authz_assert_role_administrator(target_organization_id);
    SELECT organization_id, built_in
    INTO existing_organization_id, existing_built_in
    FROM authorization_roles
    WHERE id = target_role_id
    FOR UPDATE;
    IF FOUND AND (
        existing_built_in
        OR existing_organization_id IS DISTINCT FROM target_organization_id
    ) THEN
        RAISE EXCEPTION 'role % is not a custom role owned by organization %',
            target_role_id, target_organization_id USING ERRCODE = '23514';
    END IF;
    IF btrim(target_name) = '' THEN
        RAISE EXCEPTION 'custom role name is required' USING ERRCODE = '23514';
    END IF;

    INSERT INTO authorization_roles (
        id, organization_id, stable_key, identity_kind, role_class,
        name, built_in, active
    ) VALUES (
        target_role_id, target_organization_id, NULL, NULL, 'custom',
        target_name, false, target_active
    )
    ON CONFLICT (id) DO UPDATE SET
        name = EXCLUDED.name,
        active = EXCLUDED.active,
        updated_at = now();

    DELETE FROM role_grants WHERE role_id = target_role_id;
    IF target_active THEN
        INSERT INTO role_grants (role_id, grant_id)
        SELECT target_role_id, grant_id
        FROM unnest(target_grant_ids) AS grant_id;
    END IF;
END;
$$;


--
-- Name: case_scope_identifiers(uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.case_scope_identifiers(target_case_id uuid) RETURNS TABLE(scope_kind text, identifier text)
    LANGUAGE sql STABLE
    AS $$
    SELECT 'sender', source_sender_presave_id::text FROM sender_information
     WHERE case_id = target_case_id AND source_sender_presave_id IS NOT NULL
    UNION SELECT 'product', source_product_presave_id::text FROM drug_information
     WHERE case_id = target_case_id AND source_product_presave_id IS NOT NULL
    UNION SELECT 'study', source_study_presave_id::text FROM study_information
     WHERE case_id = target_case_id AND source_study_presave_id IS NOT NULL
    UNION SELECT 'sender', product.sender_presave_id::text FROM drug_information drug
      JOIN product_presaves product ON product.id = drug.source_product_presave_id
     WHERE drug.case_id = target_case_id AND product.sender_presave_id IS NOT NULL
    UNION SELECT 'product', study.product_presave_id::text FROM study_information case_study
      JOIN study_presaves study ON study.id = case_study.source_study_presave_id
     WHERE case_study.case_id = target_case_id AND study.product_presave_id IS NOT NULL
    UNION SELECT 'sender', product.sender_presave_id::text FROM study_information case_study
      JOIN study_presaves study ON study.id = case_study.source_study_presave_id
      JOIN product_presaves product ON product.id = study.product_presave_id
     WHERE case_study.case_id = target_case_id AND product.sender_presave_id IS NOT NULL
    UNION SELECT 'product', product.id::text FROM cases case_row
      JOIN product_presaves product ON product.organization_id = case_row.organization_id
       AND product.product_id = case_row.dg_prd_key AND product.deleted = false
     WHERE case_row.id = target_case_id
    UNION SELECT 'sender', product.sender_presave_id::text FROM cases case_row
      JOIN product_presaves product ON product.organization_id = case_row.organization_id
       AND product.product_id = case_row.dg_prd_key AND product.deleted = false
     WHERE case_row.id = target_case_id AND product.sender_presave_id IS NOT NULL;
$$;


--
-- Name: compute_audit_changed_fields(jsonb, jsonb, text); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.compute_audit_changed_fields(p_old jsonb, p_new jsonb, p_prefix text DEFAULT ''::text) RETURNS jsonb
    LANGUAGE plpgsql STABLE
    AS $_$
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
            ELSIF audit_jsonb_is_distinct(v_old_value, v_new_value) THEN
                v_result := v_result || jsonb_build_object(
                    v_path,
                    jsonb_build_object('old', v_old_value, 'new', v_new_value)
                );
            END IF;
        END LOOP;
        RETURN v_result;
    END IF;

    IF audit_jsonb_is_distinct(p_old, p_new) THEN
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
$_$;


--
-- Name: current_organization_id(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.current_organization_id() RETURNS uuid
    LANGUAGE plpgsql STABLE
    AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_organization_id', true), '')::UUID;
EXCEPTION
    WHEN OTHERS THEN
        RETURN NULL;
END;
$$;


--
-- Name: enforce_role_grant_assignment(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.enforce_role_grant_assignment() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
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


--
-- Name: enforce_user_role_assignment_scope(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.enforce_user_role_assignment_scope() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
DECLARE
    target_organization_type text;
    target_role_organization_id uuid;
    target_role_identity_kind text;
    target_role_built_in boolean;
    target_role_active boolean;
    target_role_deleted_at timestamptz;
BEGIN
    EXECUTE format(
        'SELECT organization_type FROM %I.organization_policy_state WHERE organization_id = $1 FOR SHARE',
        TG_TABLE_SCHEMA
    ) INTO target_organization_type USING NEW.organization_id;
	IF target_organization_type IS NULL THEN
        RAISE EXCEPTION 'organization % does not exist', NEW.organization_id
            USING ERRCODE = '23503';
    END IF;
    EXECUTE format(
        'SELECT organization_id, identity_kind, built_in, active, deleted_at FROM %I.authorization_roles WHERE id = $1 FOR SHARE',
        TG_TABLE_SCHEMA
    ) INTO target_role_organization_id, target_role_identity_kind,
        target_role_built_in, target_role_active, target_role_deleted_at
      USING NEW.role_id;
	IF target_role_active IS NULL
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
$_$;


--
-- Name: enforce_value_null_flavor_exclusivity(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.enforce_value_null_flavor_exclusivity() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    row_data jsonb := to_jsonb(NEW);
    old_data jsonb := CASE WHEN TG_OP = 'UPDATE' THEN to_jsonb(OLD) ELSE '{}'::jsonb END;
    pair_index integer;
    value_column text;
    null_flavor_column text;
    value_present boolean;
    null_flavor_present boolean;
    value_changed boolean;
    null_flavor_changed boolean;
BEGIN
    IF TG_NARGS = 0 OR TG_NARGS % 2 <> 0 THEN
        RAISE EXCEPTION 'NullFlavor trigger on % has invalid pair arguments', TG_TABLE_NAME;
    END IF;

    FOR pair_index IN 0..(TG_NARGS / 2 - 1)
    LOOP
        value_column := TG_ARGV[pair_index * 2];
        null_flavor_column := TG_ARGV[pair_index * 2 + 1];
        value_present := row_data -> value_column IS DISTINCT FROM 'null'::jsonb;
        null_flavor_present := row_data -> null_flavor_column IS DISTINCT FROM 'null'::jsonb;

        IF value_present AND null_flavor_present THEN
            value_changed := TG_OP = 'INSERT'
                OR row_data -> value_column IS DISTINCT FROM old_data -> value_column;
            null_flavor_changed := TG_OP = 'INSERT'
                OR row_data -> null_flavor_column IS DISTINCT FROM old_data -> null_flavor_column;

            IF TG_OP = 'UPDATE' AND value_changed AND NOT null_flavor_changed THEN
                row_data := jsonb_set(
                    row_data,
                    ARRAY[null_flavor_column],
                    'null'::jsonb,
                    true
                );
            ELSIF TG_OP = 'UPDATE' AND null_flavor_changed AND NOT value_changed THEN
                row_data := jsonb_set(
                    row_data,
                    ARRAY[value_column],
                    'null'::jsonb,
                    true
                );
            ELSE
                RAISE EXCEPTION USING
                    ERRCODE = '23514',
                    MESSAGE = format(
                        '%s.%s and %s cannot both be non-null',
                        TG_TABLE_NAME,
                        value_column,
                        null_flavor_column
                    );
            END IF;
        END IF;
    END LOOP;

    NEW := jsonb_populate_record(NEW, row_data);
    RETURN NEW;
END
$$;


--
-- Name: get_current_change_category(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.get_current_change_category() RETURNS text
    LANGUAGE plpgsql STABLE
    AS $$
DECLARE
    v_category TEXT;
BEGIN
    v_category := current_setting('app.change_category', true);
    IF v_category IS NULL OR btrim(v_category) = '' THEN
        RETURN NULL;
    END IF;
    RETURN v_category;
END;
$$;


--
-- Name: get_current_change_reason(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.get_current_change_reason() RETURNS text
    LANGUAGE plpgsql STABLE
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


--
-- Name: get_current_esignature_id(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.get_current_esignature_id() RETURNS uuid
    LANGUAGE plpgsql STABLE
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


--
-- Name: get_current_user_context(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.get_current_user_context() RETURNS uuid
    LANGUAGE plpgsql STABLE
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


--
-- Name: has_business_change_on_update(anyelement, anyelement); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.has_business_change_on_update(p_old anyelement, p_new anyelement) RETURNS boolean
    LANGUAGE plpgsql
    AS $$
DECLARE
  v_old_business JSONB;
  v_new_business JSONB;
BEGIN
  v_old_business := to_jsonb(p_old) - 'updated_at' - 'updated_by';
  v_new_business := to_jsonb(p_new) - 'updated_at' - 'updated_by';
  RETURN audit_jsonb_is_distinct(v_old_business, v_new_business);
END;
$$;


--
-- Name: is_current_user_admin(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.is_current_user_admin() RETURNS boolean
    LANGUAGE plpgsql STABLE
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


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: audit_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs (
    id bigint NOT NULL,
    organization_id uuid NOT NULL,
    table_name character varying(100) NOT NULL,
    record_id uuid NOT NULL,
    action character varying(50) NOT NULL,
    user_id uuid NOT NULL,
    reason_for_change text,
    change_category text,
    e_signature_id uuid,
    old_values jsonb,
    new_values jsonb,
    changed_fields jsonb,
    ip_address inet,
    user_agent text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    prev_hash character(64) NOT NULL,
    entry_hash character(64) NOT NULL,
    parent_record_ids uuid[] GENERATED ALWAYS AS (public.audit_parent_record_ids(old_values, new_values)) STORED,
    hash_version smallint DEFAULT 2 NOT NULL,
    CONSTRAINT audit_action_valid CHECK (((action)::text = ANY (ARRAY[('CREATE'::character varying)::text, ('UPDATE'::character varying)::text, ('DELETE'::character varying)::text, ('SUBMIT'::character varying)::text, ('NULLIFY'::character varying)::text]))),
    CONSTRAINT model_nonblank_2e2ea9d2f30bf9607a0137aaf1875ad4 CHECK ((entry_hash !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3c39b686d934519507aa3c20a29673cf CHECK ((prev_hash !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_418c5509e2171d55b0aee5c2ea4442b5 CHECK (((action)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4b27b6e578e03e53206b2e4688d7a3e5 CHECK (((table_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_822f75383bb6992e7fb794f998b5bb7b CHECK ((change_category !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_83647c700b7d38852412f4f946f00c88 CHECK ((user_agent !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c3f90262d8b7d58d5f7e3c978f0b9675 CHECK ((reason_for_change !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.audit_logs FORCE ROW LEVEL SECURITY;


--
-- Name: list_case_audit_logs(uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.list_case_audit_logs(p_case_id uuid) RETURNS SETOF public.audit_logs
    LANGUAGE plpgsql STABLE
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
DECLARE
    v_organization_id UUID;
    v_seen UUID[];
    v_frontier UUID[];
    v_next_frontier UUID[];
    v_frontier_chunk UUID[];
    v_children UUID[];
    v_offset INTEGER;
BEGIN
    FOR v_organization_id IN
        SELECT DISTINCT logs.organization_id
          FROM public.audit_logs AS logs
         WHERE logs.table_name = 'cases'
           AND logs.record_id = p_case_id
    LOOP
        v_seen := ARRAY[p_case_id];
        v_frontier := ARRAY[p_case_id];

        WHILE pg_catalog.cardinality(v_frontier) > 0 LOOP
            v_next_frontier := ARRAY[]::UUID[];
            v_offset := 1;

            WHILE v_offset <= pg_catalog.cardinality(v_frontier) LOOP
                v_frontier_chunk := v_frontier[v_offset:v_offset + 999];

                SELECT pg_catalog.array_agg(DISTINCT logs.record_id)
                  INTO v_children
                  FROM public.audit_logs AS logs
                 WHERE logs.organization_id = v_organization_id
                   AND logs.parent_record_ids && v_frontier_chunk
                   AND NOT (logs.record_id = ANY(v_seen))
                   AND NOT (logs.record_id = ANY(v_next_frontier));

                IF v_children IS NULL THEN
                    v_offset := v_offset + 1000;
                    CONTINUE;
                END IF;

                v_next_frontier := v_next_frontier || v_children;
                v_offset := v_offset + 1000;
            END LOOP;

            IF pg_catalog.cardinality(v_next_frontier) = 0 THEN
                EXIT;
            END IF;

            v_seen := v_seen || v_next_frontier;
            v_frontier := v_next_frontier;
        END LOOP;

        RETURN QUERY
        SELECT logs.*
          FROM public.audit_logs AS logs
         WHERE logs.organization_id = v_organization_id
           AND logs.record_id = ANY(v_seen);
    END LOOP;
END;
$$;


--
-- Name: lock_audit_log_append(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.lock_audit_log_append() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('e2br3.audit_logs.hash_chain', 0));
    RETURN NULL;
END;
$$;


--
-- Name: mark_case_dirty_c(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_c() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_c = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_c_from_study_info(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_c_from_study_info() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases
    SET dirty_c = TRUE
    WHERE id = (
      SELECT case_id FROM study_information
      WHERE id = COALESCE(NEW.study_information_id, OLD.study_information_id)
    );
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_d(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_d() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_d = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_d_from_death_info(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_d_from_death_info() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases
    SET dirty_d = TRUE
    WHERE id = (
      SELECT p.case_id
      FROM patient_death_information di
      JOIN patient_information p ON p.id = di.patient_id
      WHERE di.id = COALESCE(NEW.death_info_id, OLD.death_info_id)
    );
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_d_from_parent(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_d_from_parent() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases
    SET dirty_d = TRUE
    WHERE id = (
      SELECT p.case_id
      FROM parent_information pi
      JOIN patient_information p ON p.id = pi.patient_id
      WHERE pi.id = COALESCE(NEW.parent_id, OLD.parent_id)
    );
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_d_from_patient(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_d_from_patient() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases
    SET dirty_d = TRUE
    WHERE id = (
      SELECT case_id FROM patient_information
      WHERE id = COALESCE(NEW.patient_id, OLD.patient_id)
    );
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_e(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_e() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_e = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_f(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_f() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_f = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_g(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_g() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_g = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_g_from_drug(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_g_from_drug() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
  v_drug_id UUID;
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;

  IF TG_TABLE_NAME = 'relatedness_assessments' THEN
    SELECT dra.drug_id INTO v_drug_id
    FROM drug_reaction_assessments dra
    WHERE dra.id = COALESCE(NEW.drug_reaction_assessment_id, OLD.drug_reaction_assessment_id);
  ELSE
    v_drug_id := COALESCE(NEW.drug_id, OLD.drug_id);
  END IF;

  IF v_drug_id IS NOT NULL THEN
    UPDATE cases
      SET dirty_g = TRUE
      WHERE id = (
        SELECT case_id FROM drug_information
        WHERE id = v_drug_id
      );
  END IF;
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_h(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_h() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_h = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: mark_case_dirty_h_from_narrative(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.mark_case_dirty_h_from_narrative() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases
    SET dirty_h = TRUE
    WHERE id = (
      SELECT case_id FROM narrative_information
      WHERE id = COALESCE(NEW.narrative_id, OLD.narrative_id)
    );
  RETURN COALESCE(NEW, OLD);
END;
$$;


--
-- Name: model_text_array_has_no_blank(text[]); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.model_text_array_has_no_blank(values_to_check text[]) RETURNS boolean
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $_$
    SELECT NOT EXISTS (
        SELECT 1 FROM unnest(values_to_check) AS item
        WHERE item ~ '^[[:space:]]*$'
    );
$_$;


--
-- Name: prevent_audit_log_mutation(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.prevent_audit_log_mutation() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    RAISE EXCEPTION 'audit logs are append-only' USING ERRCODE = '55000';
END;
$$;


--
-- Name: prevent_authorization_audit_mutation(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.prevent_authorization_audit_mutation() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    RAISE EXCEPTION 'authorization audit events are append-only'
        USING ERRCODE = '55000';
END;
$$;


--
-- Name: protect_builtin_authorization_role(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.protect_builtin_authorization_role() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
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


--
-- Name: protect_builtin_role_grant(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.protect_builtin_role_grant() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
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


--
-- Name: protect_submission_artifact(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.protect_submission_artifact() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF (NEW.authority, NEW.submitted_xml, NEW.case_id, NEW.gateway, NEW.xml_bytes)
        IS DISTINCT FROM
       (OLD.authority, OLD.submitted_xml, OLD.case_id, OLD.gateway, OLD.xml_bytes) THEN
        RAISE EXCEPTION 'Submission identity and original XML are immutable';
    END IF;
    RETURN NEW;
END;
$$;


--
-- Name: require_active_presave_reference(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.require_active_presave_reference() RETURNS trigger
    LANGUAGE plpgsql
    AS $_$
DECLARE
    referenced_id UUID;
    owning_organization_id UUID;
    active_id UUID;
BEGIN
    referenced_id := NULLIF(to_jsonb(NEW) ->> TG_ARGV[1], '')::UUID;
    IF referenced_id IS NULL THEN
        RETURN NEW;
    END IF;

    CASE TG_ARGV[2]
        WHEN 'organization' THEN
            owning_organization_id :=
                NULLIF(to_jsonb(NEW) ->> 'organization_id', '')::UUID;
        WHEN 'case' THEN
            SELECT organization_id
              INTO owning_organization_id
              FROM cases
             WHERE id = NULLIF(to_jsonb(NEW) ->> 'case_id', '')::UUID;
        WHEN 'study' THEN
            SELECT organization_id
              INTO owning_organization_id
              FROM study_presaves
             WHERE id = NULLIF(to_jsonb(NEW) ->> 'study_presave_id', '')::UUID;
        ELSE
            RAISE EXCEPTION 'unsupported presave owner source: %', TG_ARGV[2];
    END CASE;

    EXECUTE format(
        'SELECT id FROM %I WHERE id = $1 AND organization_id = $2 AND deleted = false FOR KEY SHARE',
        TG_ARGV[0]
    )
    INTO active_id
    USING referenced_id, owning_organization_id;

    IF active_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P2001',
            MESSAGE = 'inactive presave reference',
            DETAIL = format('%s:%s', TG_ARGV[0], referenced_id);
    END IF;

    RETURN NEW;
END;
$_$;


--
-- Name: search_active_whodrug_products(text, bigint); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.search_active_whodrug_products(p_query text, p_limit bigint) RETURNS TABLE(id bigint, code character varying, drug_name text, atc_code character varying, version character varying, language character varying, active boolean, created_at timestamp with time zone)
    LANGUAGE sql STABLE STRICT SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'pg_temp'
    AS $$
    SELECT
        products.id,
        products.code,
        products.drug_name,
        products.atc_code,
        products.version,
        products.language,
        TRUE,
        products.created_at
    FROM public.whodrug_products AS products
    JOIN public.terminology_releases AS releases
      ON releases.dictionary = 'whodrug'
     AND releases.version = products.version
     AND releases.language = products.language
    WHERE releases.status = 'active'
      AND products.drug_name ILIKE ('%' || p_query || '%')
    ORDER BY products.drug_name, products.code
    LIMIT LEAST(GREATEST(p_limit, 1), 100);
$$;


--
-- Name: select_current_user_organization(uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.select_current_user_organization(target_organization_id uuid) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_catalog'
    AS $$
DECLARE
	actor_user_id uuid;
BEGIN
	actor_user_id := NULLIF(
		current_setting('app.current_user_id', true), ''
	)::uuid;
	IF actor_user_id IS NULL OR target_organization_id IS NULL THEN
		RETURN false;
	END IF;

	IF NOT EXISTS (
		SELECT 1
		FROM user_organization_memberships membership
		JOIN organizations organization
		  ON organization.id = membership.organization_id
		WHERE membership.user_id = actor_user_id
		  AND membership.organization_id = target_organization_id
		  AND membership.active
		  AND organization.active
	) THEN
		RETURN false;
	END IF;

	UPDATE users
	SET organization_id = target_organization_id,
		updated_by = actor_user_id,
		updated_at = NOW()
	WHERE id = actor_user_id;

	RETURN FOUND;
END;
$$;


--
-- Name: set_authorization_isolation_context(uuid, boolean); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.set_authorization_isolation_context(org_id uuid, requested_platform_bypass boolean) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_catalog'
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


--
-- Name: set_compliance_context(text, text, text); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.set_compliance_context(p_change_reason text, p_change_category text, p_e_signature_id text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    AS $$
BEGIN
    PERFORM set_config(
        'app.change_reason',
        COALESCE(p_change_reason, ''),
        true
    );
    PERFORM set_config(
        'app.change_category',
        COALESCE(p_change_category, ''),
        true
    );
    PERFORM set_config(
        'app.e_signature_id',
        COALESCE(p_e_signature_id, ''),
        true
    );
END;
$$;


--
-- Name: set_current_user_context(uuid); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.set_current_user_context(p_user_id uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    AS $$
BEGIN
    -- Set session variable (transaction-scoped when third parameter is true)
    PERFORM set_config('app.current_user_id', p_user_id::text, true);
END;
$$;


--
-- Name: set_org_context(uuid, character varying); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.set_org_context(org_id uuid, user_role character varying) RETURNS void
    LANGUAGE plpgsql
    AS $$
BEGIN
	PERFORM set_authorization_isolation_context(
		org_id,
		lower(btrim(user_role)) = 'system_admin'
	);
END;
$$;


--
-- Name: set_reaction_serious_derived(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.set_reaction_serious_derived() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF NEW.criteria_death IS TRUE
       OR NEW.criteria_life_threatening IS TRUE
       OR NEW.criteria_hospitalization IS TRUE
       OR NEW.criteria_disabling IS TRUE
       OR NEW.criteria_congenital_anomaly IS TRUE
       OR NEW.criteria_other_medically_important IS TRUE
       OR btrim(NEW.term_highlighted) IN ('3', '4') THEN
        NEW.serious := TRUE;
    ELSIF btrim(NEW.term_highlighted) IN ('1', '2') THEN
        NEW.serious := FALSE;
    ELSE
        NEW.serious := NULL;
    END IF;
    RETURN NEW;
END;
$$;


--
-- Name: update_updated_at_column(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_updated_at_column() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


--
-- Name: validate_scope_assignment(uuid, text[], text[], text[]); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.validate_scope_assignment(target_organization_id uuid, sender_ids text[], product_ids text[], study_ids text[]) RETURNS text
    LANGUAGE plpgsql STABLE
    AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM unnest(sender_ids) requested(id)
         WHERE NOT EXISTS (
             SELECT 1 FROM sender_presaves sender
              WHERE sender.organization_id = target_organization_id
                AND sender.deleted = false
                AND sender.id::text = requested.id
         )
    ) THEN RETURN 'sender_not_found'; END IF;

    IF EXISTS (
        SELECT 1 FROM unnest(product_ids) requested(id)
         WHERE NOT EXISTS (
             SELECT 1 FROM product_presaves product
              WHERE product.organization_id = target_organization_id
                AND product.deleted = false
                AND product.id::text = requested.id
                AND (
                    cardinality(sender_ids) = 0
                    OR product.sender_presave_id::text = ANY(sender_ids)
                )
         )
    ) THEN RETURN 'product_sender_mismatch'; END IF;

    IF EXISTS (
        SELECT 1 FROM unnest(study_ids) requested(id)
         WHERE NOT EXISTS (
             SELECT 1 FROM study_presaves study
              WHERE study.organization_id = target_organization_id
                AND study.deleted = false
                AND study.id::text = requested.id
                AND (
                    cardinality(product_ids) = 0
                    OR study.product_presave_id::text = ANY(product_ids)
                )
         )
    ) THEN RETURN 'study_product_mismatch'; END IF;

    IF cardinality(sender_ids) > 0 AND EXISTS (
        SELECT 1 FROM unnest(study_ids) requested(id)
         WHERE NOT EXISTS (
             SELECT 1
               FROM study_presaves study
               JOIN product_presaves product
                 ON product.id = study.product_presave_id
                AND product.organization_id = target_organization_id
                AND product.deleted = false
              WHERE study.organization_id = target_organization_id
                AND study.deleted = false
                AND study.id::text = requested.id
                AND product.sender_presave_id::text = ANY(sender_ids)
         )
    ) THEN RETURN 'study_sender_mismatch'; END IF;

    RETURN NULL;
END;
$$;


--
-- Name: validate_user_context(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.validate_user_context() RETURNS trigger
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


--
-- Name: verify_audit_log_hash_chain(bigint); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.verify_audit_log_hash_chain(p_since_id bigint DEFAULT NULL::bigint) RETURNS TABLE(total_rows bigint, verified_ok_rows bigint, broken_rows bigint, first_broken_id bigint, first_broken_reason text, checked_at timestamp with time zone)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $$
DECLARE
    v_head_id BIGINT;
    v_next BIGINT := p_since_id;
    v_page RECORD;
BEGIN
    total_rows := 0;
    verified_ok_rows := 0;
    broken_rows := 0;
    first_broken_id := NULL;
    first_broken_reason := NULL;
    checked_at := clock_timestamp();

    SELECT max(a.id) INTO v_head_id FROM public.audit_logs a;
    IF v_head_id IS NULL THEN
        RETURN NEXT;
        RETURN;
    END IF;
    IF p_since_id IS NOT NULL AND p_since_id > v_head_id THEN
        RETURN NEXT;
        RETURN;
    END IF;
    IF p_since_id IS NOT NULL AND p_since_id < 0 THEN
        v_next := NULL;
    END IF;

    LOOP
        SELECT * INTO v_page
          FROM public.verify_audit_log_hash_chain_range(v_next, v_head_id, 10000);
        total_rows := total_rows + v_page.total_rows;
        verified_ok_rows := verified_ok_rows + v_page.verified_ok_rows;
        broken_rows := broken_rows + v_page.broken_rows;
        IF first_broken_id IS NULL AND v_page.first_broken_id IS NOT NULL THEN
            first_broken_id := v_page.first_broken_id;
            first_broken_reason := v_page.first_broken_reason;
        END IF;
        EXIT WHEN NOT v_page.has_more;
        v_next := v_page.next_since_id;
    END LOOP;
    RETURN NEXT;
END;
$$;


--
-- Name: verify_audit_log_hash_chain_range(bigint, bigint, integer); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.verify_audit_log_hash_chain_range(p_since_id bigint DEFAULT NULL::bigint, p_until_id bigint DEFAULT NULL::bigint, p_limit integer DEFAULT 1000) RETURNS TABLE(total_rows bigint, verified_ok_rows bigint, broken_rows bigint, first_broken_id bigint, first_broken_reason text, checked_at timestamp with time zone, checked_from_id bigint, checked_through_id bigint, snapshot_head_id bigint, snapshot_head_hash text, next_since_id bigint, has_more boolean, legacy_rows bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    AS $_$
DECLARE
    v_snapshot_head_id BIGINT;
    v_snapshot_head_hash TEXT;
BEGIN
    IF p_limit IS NULL OR p_limit < 1 OR p_limit > 10000 THEN
        RAISE EXCEPTION 'audit verification limit must be between 1 and 10000'
            USING ERRCODE = '22023';
    END IF;
    IF p_since_id IS NOT NULL AND p_since_id < 0 THEN
        RAISE EXCEPTION 'audit verification since id must be non-negative'
            USING ERRCODE = '22023';
    END IF;
    IF p_until_id IS NOT NULL AND p_until_id < 0 THEN
        RAISE EXCEPTION 'audit verification until id must be non-negative'
            USING ERRCODE = '22023';
    END IF;
    IF p_since_id IS NOT NULL AND p_until_id IS NOT NULL
       AND p_since_id > p_until_id THEN
        RAISE EXCEPTION 'audit verification since id must not exceed until id'
            USING ERRCODE = '22023';
    END IF;
    IF p_until_id IS NULL THEN
        SELECT a.id, a.entry_hash::TEXT
          INTO v_snapshot_head_id, v_snapshot_head_hash
          FROM public.audit_logs a
         ORDER BY a.id DESC LIMIT 1;
    ELSE
        SELECT a.id, a.entry_hash::TEXT
          INTO v_snapshot_head_id, v_snapshot_head_hash
          FROM public.audit_logs a
         WHERE a.id = p_until_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'audit verification until id does not exist'
                USING ERRCODE = '22023';
        END IF;
    END IF;

    RETURN QUERY
    WITH snapshot AS MATERIALIZED (
        SELECT v_snapshot_head_id AS id, v_snapshot_head_hash AS entry_hash
         WHERE v_snapshot_head_id IS NOT NULL
    ),
    -- Scalar bounds stay PK index conditions; cross joins can become late join
    -- filters and scan millions of audit rows before applying the page boundary.
    page AS MATERIALIZED (
        SELECT a.*
          FROM public.audit_logs a
         WHERE a.id >= coalesce(p_since_id, '-9223372036854775808'::BIGINT)
           AND a.id <= (SELECT id FROM snapshot)
         ORDER BY a.id
         LIMIT p_limit + 1
    ),
    bounded AS MATERIALIZED (
        SELECT * FROM page ORDER BY id LIMIT p_limit
    ),
    first_row AS (
        SELECT min(id) AS id FROM bounded
    ),
    predecessor AS (
        SELECT a.id, a.entry_hash::TEXT
          FROM public.audit_logs a
         WHERE a.id < (SELECT id FROM first_row)
         ORDER BY a.id DESC LIMIT 1
    ),
    checked AS (
        SELECT b.id,
               b.hash_version,
               CASE
                   WHEN b.hash_version IS NULL OR b.hash_version NOT IN (1, 2)
                       THEN 'unknown hash version'
                   WHEN b.prev_hash IS NULL OR b.prev_hash !~ '^[0-9A-Fa-f]{64}$'
                       THEN 'prev_hash is not a 64-char hex value'
                   WHEN b.entry_hash IS NULL OR b.entry_hash !~ '^[0-9A-Fa-f]{64}$'
                       THEN 'entry_hash is not a 64-char hex value'
                   WHEN (SELECT id FROM predecessor) IS NOT NULL
                        AND b.id = (SELECT id FROM first_row)
                        AND ((SELECT entry_hash FROM predecessor) IS NULL
                             OR (SELECT entry_hash FROM predecessor) !~ '^[0-9A-Fa-f]{64}$')
                       THEN 'previous entry_hash is not a 64-char hex value'
                   WHEN b.prev_hash::TEXT <> coalesce(
                       lag(b.entry_hash::TEXT) OVER (ORDER BY b.id),
                       (SELECT entry_hash FROM predecessor),
                       repeat('0', 64)
                   ) THEN 'prev_hash does not match previous entry_hash'
                   WHEN b.entry_hash::TEXT <> CASE b.hash_version
                       WHEN 1 THEN encode(public.digest(concat_ws('|',
                           coalesce(b.id::TEXT, ''), coalesce(b.prev_hash::TEXT, ''),
                           b.table_name, b.record_id::TEXT, b.action, b.user_id::TEXT,
                           coalesce(b.reason_for_change, ''), coalesce(b.change_category, ''),
                           coalesce(b.e_signature_id::TEXT, ''), coalesce(b.old_values::TEXT, 'null'),
                           coalesce(b.new_values::TEXT, 'null'), coalesce(b.changed_fields::TEXT, 'null'),
                           coalesce(b.ip_address::TEXT, ''), coalesce(b.user_agent, ''),
                           to_char(b.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                       ), 'sha256'), 'hex')
                       WHEN 2 THEN encode(public.digest(jsonb_build_array(
                           b.hash_version, b.id, b.prev_hash::TEXT, b.organization_id,
                           b.table_name, b.record_id, b.action, b.user_id,
                           b.reason_for_change, b.change_category, b.e_signature_id,
                           jsonb_build_array(b.old_values IS NULL, b.old_values),
                           jsonb_build_array(b.new_values IS NULL, b.new_values),
                           jsonb_build_array(b.changed_fields IS NULL, b.changed_fields),
                           b.ip_address::TEXT, b.user_agent,
                           to_char(b.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                       )::TEXT, 'sha256'), 'hex')
                   END THEN 'entry_hash does not match recomputed payload hash'
               END AS reason
          FROM bounded b
    ),
    summary AS (
        SELECT count(*)::BIGINT AS total_rows,
               count(*) FILTER (WHERE reason IS NULL)::BIGINT AS ok_rows,
               count(*) FILTER (WHERE reason IS NOT NULL)::BIGINT AS broken_rows,
               min(id) FILTER (WHERE reason IS NOT NULL) AS first_broken_id,
               (array_agg(reason ORDER BY id) FILTER (WHERE reason IS NOT NULL))[1] AS first_reason,
               min(id) AS checked_from_id,
               max(id) AS checked_through_id,
               count(*) FILTER (WHERE hash_version = 1)::BIGINT AS legacy_rows
          FROM checked
    )
    SELECT x.total_rows, x.ok_rows, x.broken_rows, x.first_broken_id,
           x.first_reason, clock_timestamp(), x.checked_from_id,
           x.checked_through_id, s.id, s.entry_hash,
           (SELECT id FROM page ORDER BY id OFFSET p_limit LIMIT 1),
           EXISTS (SELECT 1 FROM page OFFSET p_limit), x.legacy_rows
      FROM summary x
      LEFT JOIN snapshot s ON TRUE;
END;
$_$;


--
-- Name: app_settings; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.app_settings (
    organization_id uuid NOT NULL,
    key text NOT NULL,
    value jsonb DEFAULT '{}'::jsonb NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_3c6e0b8a9c15224a8228b9a98ca1531d CHECK ((key !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.app_settings FORCE ROW LEVEL SECURITY;


--
-- Name: audit_logs_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.audit_logs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: audit_logs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.audit_logs_id_seq OWNED BY public.audit_logs.id;


--
-- Name: authorization_audit_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.authorization_audit_events (
    id bigint NOT NULL,
    principal_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    role_id uuid NOT NULL,
    action_id text NOT NULL,
    decision character varying(16) NOT NULL,
    denial_reason text,
    catalog_hash text NOT NULL,
    organization_revision bigint NOT NULL,
    principal_revision bigint NOT NULL,
    target_identifier text,
    request_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT authorization_audit_decision_valid CHECK (((decision)::text = ANY (ARRAY[('allowed'::character varying)::text, ('denied'::character varying)::text]))),
    CONSTRAINT authorization_audit_denial_reason_valid CHECK (((((decision)::text = 'allowed'::text) AND (denial_reason IS NULL)) OR (((decision)::text = 'denied'::text) AND (denial_reason IS NOT NULL)))),
    CONSTRAINT model_nonblank_8f3e452711fa5df7003be2bbfb84ac29 CHECK ((target_identifier !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_909acdf22d65ef36237726a1c4de1b70 CHECK ((action_id !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_94263ba415b8c6ba92fa885da51c9d10 CHECK ((denial_reason !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9a0e81ab12f654126de37c4555332ba5 CHECK ((catalog_hash !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b9b9e36c1a5d1be22158df398f2e29b8 CHECK (((decision)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.authorization_audit_events FORCE ROW LEVEL SECURITY;


--
-- Name: authorization_audit_events_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.authorization_audit_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: authorization_audit_events_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.authorization_audit_events_id_seq OWNED BY public.authorization_audit_events.id;


--
-- Name: authorization_catalog_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.authorization_catalog_state (
    singleton boolean DEFAULT true NOT NULL,
    schema_version integer NOT NULL,
    catalog_hash text NOT NULL,
    reconciled_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT authorization_catalog_state_catalog_hash_check CHECK ((catalog_hash ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT authorization_catalog_state_schema_version_check CHECK ((schema_version > 0)),
    CONSTRAINT authorization_catalog_state_singleton_check CHECK (singleton),
    CONSTRAINT model_nonblank_9a0e81ab12f654126de37c4555332ba5 CHECK ((catalog_hash !~ '^[[:space:]]*$'::text))
);


--
-- Name: authorization_grant_catalog; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.authorization_grant_catalog (
    grant_id text NOT NULL,
    pdf_order smallint NOT NULL,
    pdf_menu text NOT NULL,
    pdf_type text NOT NULL,
    pdf_privilege text NOT NULL,
    availability text NOT NULL,
    CONSTRAINT authorization_grant_catalog_availability_check CHECK ((availability = ANY (ARRAY['implemented'::text, 'reserved'::text]))),
    CONSTRAINT authorization_grant_catalog_grant_id_check CHECK ((grant_id ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$'::text)),
    CONSTRAINT authorization_grant_catalog_pdf_order_check CHECK ((pdf_order > 0)),
    CONSTRAINT model_nonblank_7293916e9dc18d6ffefa1c98673d5ce7 CHECK ((pdf_privilege !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_794ec44125d76220719744d339f865dd CHECK ((pdf_type !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c18d18e7ad8dd7872f54832f71bfe4d5 CHECK ((pdf_menu !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c2418e0763955a29412e22b412c66854 CHECK ((grant_id !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f24431ce9f1b8885678b1ed611c4c214 CHECK ((availability !~ '^[[:space:]]*$'::text))
);


--
-- Name: authorization_grant_role_classes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.authorization_grant_role_classes (
    grant_id text NOT NULL,
    role_class text NOT NULL,
    CONSTRAINT authorization_grant_role_classes_role_class_check CHECK ((role_class = ANY (ARRAY['platform_built_in'::text, 'sponsor_cro_built_in'::text, 'sponsor_company_built_in'::text, 'operational_built_in'::text, 'service_built_in'::text, 'custom'::text]))),
    CONSTRAINT model_nonblank_c2418e0763955a29412e22b412c66854 CHECK ((grant_id !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d16049c1f6f401f4d53745a2b5f0e6eb CHECK ((role_class !~ '^[[:space:]]*$'::text))
);


--
-- Name: authorization_migration_reconciliations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.authorization_migration_reconciliations (
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    legacy_role text NOT NULL,
    normalized_role_id uuid NOT NULL,
    legacy_effective_access jsonb NOT NULL,
    normalized_effective_access jsonb NOT NULL,
    evidence_hash text NOT NULL,
    proof_hash text,
    equivalent boolean,
    comparison_status text NOT NULL,
    reconciled_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT authorization_migration_reconciliations_check CHECK ((((comparison_status = 'pending_action_binding'::text) AND (equivalent IS NULL) AND (proof_hash IS NULL)) OR ((comparison_status = 'proven_equivalent'::text) AND (equivalent IS TRUE) AND (proof_hash IS NOT NULL)) OR ((comparison_status = 'proven_different'::text) AND (equivalent IS FALSE) AND (proof_hash IS NOT NULL)))),
    CONSTRAINT authorization_migration_reconciliations_comparison_status_check CHECK ((comparison_status = ANY (ARRAY['pending_action_binding'::text, 'proven_equivalent'::text, 'proven_different'::text]))),
    CONSTRAINT authorization_migration_reconciliations_evidence_hash_check CHECK ((evidence_hash ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT authorization_migration_reconciliations_proof_hash_check CHECK (((proof_hash IS NULL) OR (proof_hash ~ '^[0-9a-f]{64}$'::text))),
    CONSTRAINT model_nonblank_32c0b73ac5f3f2203c2594e09dc78212 CHECK ((comparison_status !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_50481e375b667dd98dbbba7bece94781 CHECK ((evidence_hash !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_762774e97cd806869205b44e2b77e0d1 CHECK ((proof_hash !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c25430506d2e8361891eb1c17f3072c2 CHECK ((legacy_role !~ '^[[:space:]]*$'::text))
);


--
-- Name: authorization_migration_rejections; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.authorization_migration_rejections (
    id bigint NOT NULL,
    observed_at timestamp with time zone DEFAULT now() NOT NULL,
    user_id uuid,
    organization_id uuid,
    legacy_role text,
    reason text NOT NULL,
    resolved boolean DEFAULT false NOT NULL,
    CONSTRAINT model_nonblank_40bea8d637cdf2c1b07fcf0630482b73 CHECK ((reason !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c25430506d2e8361891eb1c17f3072c2 CHECK ((legacy_role !~ '^[[:space:]]*$'::text))
);


--
-- Name: authorization_migration_rejections_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.authorization_migration_rejections_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: authorization_migration_rejections_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.authorization_migration_rejections_id_seq OWNED BY public.authorization_migration_rejections.id;


--
-- Name: authorization_roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.authorization_roles (
    id uuid NOT NULL,
    organization_id uuid,
    stable_key text,
    identity_kind text,
    role_class text NOT NULL,
    name text NOT NULL,
    built_in boolean NOT NULL,
    active boolean DEFAULT true NOT NULL,
    deleted_at timestamp with time zone,
    row_version bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT authorization_roles_check CHECK (((built_in AND (organization_id IS NULL) AND (stable_key IS NOT NULL) AND (identity_kind IS NOT NULL)) OR ((NOT built_in) AND (organization_id IS NOT NULL) AND (stable_key IS NULL) AND (identity_kind IS NULL) AND (role_class = 'custom'::text)))),
    CONSTRAINT authorization_roles_identity_kind_check CHECK (((identity_kind IS NULL) OR (identity_kind = ANY (ARRAY['platform_administrator'::text, 'sponsor_cro_administrator'::text, 'sponsor_company_administrator'::text, 'operational_user'::text, 'internal_service_principal'::text])))),
    CONSTRAINT authorization_roles_role_class_check CHECK ((role_class = ANY (ARRAY['platform_built_in'::text, 'sponsor_cro_built_in'::text, 'sponsor_company_built_in'::text, 'operational_built_in'::text, 'service_built_in'::text, 'custom'::text]))),
    CONSTRAINT authorization_roles_row_version_check CHECK ((row_version > 0)),
    CONSTRAINT model_nonblank_83ca1d2659aee0ee6d91bfa386980772 CHECK ((stable_key !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b068931cc450442b63f5b3d276ea4297 CHECK ((name !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d16049c1f6f401f4d53745a2b5f0e6eb CHECK ((role_class !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f7e8af8df69a85b96d99743c65e65d21 CHECK ((identity_kind !~ '^[[:space:]]*$'::text))
);


--
-- Name: autopsy_causes_of_death; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.autopsy_causes_of_death (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    death_info_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    meddra_version character varying(10),
    meddra_code character varying(20),
    comments text,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_63c55f93a56ff18a74048153bb2aff09 CHECK (((meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_90883ccd002fbe4c7a849fa7acd9fe43 CHECK (((meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a5d491060952aa8ad5fdee071be752de CHECK ((comments !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.autopsy_causes_of_death FORCE ROW LEVEL SECURITY;


--
-- Name: case_e2b_field_notations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.case_e2b_field_notations (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    record_id uuid,
    e2b_code character varying(64) NOT NULL,
    notation text NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_71f975fda2c66ecb66993537e109b9fb CHECK ((notation !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_96a01c93a4dfa54541768ab0ae78255f CHECK (((e2b_code)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.case_e2b_field_notations FORCE ROW LEVEL SECURITY;


--
-- Name: case_field_notations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.case_field_notations (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    record_id uuid,
    field_path character varying(255) NOT NULL,
    notation text NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_71f975fda2c66ecb66993537e109b9fb CHECK ((notation !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ae193c64c149719633fb18cfb2339a6a CHECK (((field_path)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.case_field_notations FORCE ROW LEVEL SECURITY;


--
-- Name: case_submissions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.case_submissions (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    gateway character varying(100) NOT NULL,
    remote_submission_id character varying(200),
    status character varying(50) NOT NULL,
    xml_bytes integer NOT NULL,
    submitted_by uuid NOT NULL,
    submitted_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    authority text,
    submitted_xml bytea,
    submitting_organization_id uuid,
    as2_profile_id uuid,
    CONSTRAINT case_submission_status_valid CHECK (((status)::text = ANY (ARRAY[('dispatch_failed'::character varying)::text, ('dispatch_unknown'::character varying)::text, ('submitted_ack1_pending'::character varying)::text, ('ack1_received'::character varying)::text, ('ack2_received'::character varying)::text, ('ack3_received'::character varying)::text, ('ack4_received'::character varying)::text, ('rejected'::character varying)::text]))),
    CONSTRAINT case_submissions_authority_check CHECK ((authority = ANY (ARRAY['fda'::text, 'mfds'::text]))),
    CONSTRAINT model_nonblank_3e21ab62fb17400301d9f0156b6c3031 CHECK (((gateway)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_873e9c0b50183b613336eea1020f4369 CHECK ((authority !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9429d1ef887fdddd997bd4e647be9474 CHECK (((remote_submission_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9acb44549b41563697bb490144ec6258 CHECK (((status)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT submission_artifact_complete CHECK ((((authority IS NULL) AND (submitted_xml IS NULL)) OR ((authority IS NOT NULL) AND (submitted_xml IS NOT NULL) AND (octet_length(submitted_xml) > 0) AND (octet_length(submitted_xml) = xml_bytes)))),
    CONSTRAINT submission_authority_transport CHECK (((authority IS NULL) OR ((authority = 'fda'::text) AND ((gateway)::text = ANY (ARRAY[('as2-submitter-http-fda'::character varying)::text, ('fda-esg-nextgen-api'::character varying)::text]))) OR ((authority = 'mfds'::text) AND ((gateway)::text = 'as2-submitter-http-mfds'::text))))
);

ALTER TABLE ONLY public.case_submissions FORCE ROW LEVEL SECURITY;


--
-- Name: case_summary_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.case_summary_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    narrative_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    language_code character varying(3),
    summary_text text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_60716c2f59efe423380e2faeb03472d5 CHECK (((language_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9a4f0a20e38397a783d3bb15342d5b0d CHECK ((summary_text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.case_summary_information FORCE ROW LEVEL SECURITY;


--
-- Name: case_validation_summaries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.case_validation_summaries (
    case_id uuid NOT NULL,
    appendix character varying(16) NOT NULL,
    page_id character varying(16) NOT NULL,
    issue_count integer DEFAULT 0 NOT NULL,
    required_count integer DEFAULT 0 NOT NULL,
    validation_revision text,
    stale boolean DEFAULT false NOT NULL,
    generated_at timestamp with time zone DEFAULT now() NOT NULL,
    issue_keys jsonb DEFAULT '[]'::jsonb NOT NULL,
    CONSTRAINT case_validation_summary_appendix_valid CHECK (((appendix)::text = ANY (ARRAY[('ich'::character varying)::text, ('fda'::character varying)::text, ('mfds'::character varying)::text]))),
    CONSTRAINT case_validation_summary_counts_non_negative CHECK (((issue_count >= 0) AND (required_count >= 0))),
    CONSTRAINT model_nonblank_1a63c8004d716c8b91f5b7af780555b9 CHECK (((page_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_210be2a9590c53e6d97976092fc73992 CHECK ((validation_revision !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c0cb76c26c07e1a5452ae622fa038286 CHECK (((appendix)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.case_validation_summaries FORCE ROW LEVEL SECURITY;


--
-- Name: case_versions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.case_versions (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    version integer NOT NULL,
    snapshot jsonb NOT NULL,
    changed_by uuid NOT NULL,
    change_reason text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_5e4e7e118c324d1369635ff77001a648 CHECK ((change_reason !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.case_versions FORCE ROW LEVEL SECURITY;


--
-- Name: case_workflow_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.case_workflow_events (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    from_status text NOT NULL,
    from_role text,
    from_user_id uuid,
    to_status text NOT NULL,
    target_role text,
    target_user_id uuid,
    comment text,
    date_of_most_recent text,
    due_at timestamp with time zone,
    acted_by uuid NOT NULL,
    actor_role_id text NOT NULL,
    used_admin_override boolean DEFAULT false NOT NULL,
    override_reason text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_06d4cd63bde972fc66a0aed41d2f5c51 CHECK ((comment !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_40b6f56936fc018123ccb07c6913ca06 CHECK ((target_role !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4129e6636cf70cd0e11dcd9413d70df0 CHECK ((date_of_most_recent !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_5d4432a2eaca821c87cdd6ea726921f9 CHECK ((from_status !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9724032c607fa64ed834bdd4cabee498 CHECK ((actor_role_id !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_983d26860073983d25b827ab7917a01b CHECK ((override_reason !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ab9dabaab05ad4b7046c277a9461ed5e CHECK ((to_status !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e3f5a2c642f2bc8d302695e3a43d601a CHECK ((from_role !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.case_workflow_events FORCE ROW LEVEL SECURITY;


--
-- Name: cases; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.cases (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    dg_prd_key text,
    status character varying(50) DEFAULT 'draft'::character varying NOT NULL,
    status_before_lock character varying(50),
    review_receivers_json text,
    workflow_routes_json text,
    workflow_status text DEFAULT 'Saved'::text NOT NULL,
    workflow_assigned_role text,
    workflow_assigned_user_id uuid,
    workflow_due_at timestamp with time zone,
    workflow_description text,
    workflow_updated_at timestamp with time zone DEFAULT now() NOT NULL,
    fda_report_type character varying(20),
    mfds_report_type text,
    report_year character varying(10),
    created_by uuid NOT NULL,
    updated_by uuid,
    submitted_by uuid,
    submitted_at timestamp with time zone,
    raw_xml bytea,
    dirty_c boolean DEFAULT false NOT NULL,
    dirty_d boolean DEFAULT false NOT NULL,
    dirty_e boolean DEFAULT false NOT NULL,
    dirty_f boolean DEFAULT false NOT NULL,
    dirty_g boolean DEFAULT false NOT NULL,
    dirty_h boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT case_status_before_lock_valid CHECK (((status_before_lock IS NULL) OR ((status_before_lock)::text = ANY (ARRAY[('draft'::character varying)::text, ('reviewed'::character varying)::text, ('validated'::character varying)::text])))),
    CONSTRAINT case_status_valid CHECK (((status)::text = ANY (ARRAY[('draft'::character varying)::text, ('reviewed'::character varying)::text, ('validated'::character varying)::text, ('locked'::character varying)::text, ('submitted'::character varying)::text, ('deleted'::character varying)::text, ('archived'::character varying)::text, ('nullified'::character varying)::text]))),
    CONSTRAINT model_nonblank_1261eec4eb09b0c451301dea71570373 CHECK ((workflow_status !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_38aba5072bfda4fafac1a2cb713a3a17 CHECK ((review_receivers_json !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_5ac26b89f13e42add0f8dce0bb549c25 CHECK ((dg_prd_key !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_5ebe41b4af5616b3112b9858be372c18 CHECK ((workflow_assigned_role !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_78270590cdf00b63c199ba03cf5f1763 CHECK (((report_year)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9acb44549b41563697bb490144ec6258 CHECK (((status)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b1752d354c56bc064d5a897270d2dca8 CHECK ((mfds_report_type !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c73f495a8889e070036484cb0a4c1279 CHECK (((status_before_lock)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c9a0cb4d71b0c7023801fde8ec38b277 CHECK (((fda_report_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ec1b8f30d41cc6edef42142c4ae8f7fd CHECK ((workflow_description !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f7493194bfbdb5e56b481b9090392f59 CHECK ((workflow_routes_json !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.cases FORCE ROW LEVEL SECURITY;


--
-- Name: controlled_terminology_terms; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.controlled_terminology_terms (
    id bigint NOT NULL,
    audit_id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    dictionary character varying(40) NOT NULL,
    version character varying(40) NOT NULL,
    language character varying(10) DEFAULT 'en'::character varying NOT NULL,
    scope character varying(40) NOT NULL,
    code character varying(100) NOT NULL,
    display_name text,
    active boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_2af72f100c356273d46284f6fd1dfc08 CHECK (((version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_31a1fd140be4bef2d11e121ec9a18a58 CHECK (((scope)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8512ae7d57b1396273f76fe6ed341a23 CHECK (((language)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_abc20d7bde1df257f890e152af2e3470 CHECK (((dictionary)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c13367945d5d4c91047b3b50234aa7ab CHECK (((code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c2657f133e8615f7dba3ddfe13125bd4 CHECK ((display_name !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.controlled_terminology_terms FORCE ROW LEVEL SECURITY;


--
-- Name: controlled_terminology_terms_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.controlled_terminology_terms_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: controlled_terminology_terms_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.controlled_terminology_terms_id_seq OWNED BY public.controlled_terminology_terms.id;


--
-- Name: dashboard_notices; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.dashboard_notices (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    notice_key text NOT NULL,
    title text NOT NULL,
    body text,
    effective_date text,
    expire_date text,
    writer text,
    sort_order integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_355fe3b444e6b2c6baec5022a74a4ff1 CHECK ((notice_key !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3ecc1577f0a8d4e556529792633767e7 CHECK ((effective_date !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_841a2d689ad86bd1611447453c22c6fc CHECK ((body !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a82feee3cc1af8bcabda979e8775ef0f CHECK ((writer !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d5d3db1765287eef77d7927cc956f50a CHECK ((title !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_de54fa62e2fa51952dbb90a542605b98 CHECK ((expire_date !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.dashboard_notices FORCE ROW LEVEL SECURITY;


--
-- Name: documents_held_by_sender; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.documents_held_by_sender (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    title text,
    document_base64 text,
    file_name text,
    media_type character varying(100),
    representation character varying(10),
    compression character varying(10),
    sequence_number integer NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_07f5d2d550d5961d1dd0984fe666a2f6 CHECK (((compression)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3090ad88ff7c9ca191579b709069fe64 CHECK ((file_name !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_335601aab95ba3f4e0380777ca3ce8f5 CHECK ((document_base64 !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_80a701d4b70ebbe74c0af8763367fd76 CHECK (((media_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d5d3db1765287eef77d7927cc956f50a CHECK ((title !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ee16755af8c89d0c26540daf7a0fe1b3 CHECK (((representation)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.documents_held_by_sender FORCE ROW LEVEL SECURITY;


--
-- Name: dosage_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.dosage_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    drug_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    dose_value numeric(15,5),
    dose_unit character varying(50),
    number_of_units numeric(15,5),
    frequency_unit character varying(50),
    first_administration_date date,
    first_administration_date_raw text,
    last_administration_date date,
    last_administration_date_raw text,
    duration_value numeric(10,2),
    duration_unit character varying(50),
    continuing boolean,
    batch_lot_number character varying(200),
    dosage_text text,
    dose_form character varying(200),
    dose_form_null_flavor character varying(4),
    dose_form_termid character varying(100),
    dose_form_termid_version character varying(10),
    route_of_administration character varying(200),
    route_of_administration_null_flavor character varying(4),
    route_termid character varying(100),
    route_termid_version character varying(10),
    route_termid_code_system text,
    parent_route character varying(60),
    parent_route_null_flavor character varying(4),
    parent_route_termid character varying(100),
    parent_route_termid_version character varying(10),
    parent_route_termid_code_system text,
    first_administration_date_null_flavor character varying(4),
    last_administration_date_null_flavor character varying(4),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    batch_lot_number_null_flavor character varying(4),
    CONSTRAINT ck_nf_dosage_first_date CHECK (((first_administration_date_null_flavor IS NULL) OR ((first_administration_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_dosage_form CHECK (((dose_form_null_flavor IS NULL) OR ((dose_form_null_flavor)::text = ANY (ARRAY[('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_dosage_last_date CHECK (((last_administration_date_null_flavor IS NULL) OR ((last_administration_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_dosage_parent_route CHECK (((parent_route_null_flavor IS NULL) OR ((parent_route_null_flavor)::text = ANY (ARRAY[('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_dosage_route CHECK (((route_of_administration_null_flavor IS NULL) OR ((route_of_administration_null_flavor)::text = ANY (ARRAY[('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nfv_dosage_information_12f028792e1f CHECK (((route_of_administration IS NULL) OR (route_of_administration_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_dosage_information_34847d6dda3d CHECK (((last_administration_date IS NULL) OR (last_administration_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_dosage_information_batch_lot CHECK (((batch_lot_number IS NULL) OR (batch_lot_number_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_dosage_information_c06973001186 CHECK (((first_administration_date IS NULL) OR (first_administration_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_dosage_information_f27137b32806 CHECK (((parent_route IS NULL) OR (parent_route_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_dosage_information_f3e628bb02f1 CHECK (((dose_form IS NULL) OR (dose_form_null_flavor IS NULL))),
    CONSTRAINT dosage_information_batch_lot_number_null_flavor_check CHECK (((batch_lot_number_null_flavor IS NULL) OR ((batch_lot_number_null_flavor)::text = 'UNK'::text))),
    CONSTRAINT model_nonblank_071dfafcd20d687a1eeb70d9cac29121 CHECK ((parent_route_termid_code_system !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_1e48d36b6b08ccb85543e5958c2b0e87 CHECK (((route_of_administration_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_281126993f8ba6eaf4f988dfbe28909c CHECK (((dose_form_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_30f1083674f606a8f3ac4b7bc0d90ed3 CHECK (((dose_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3325d24e108e518128a9a2c1917e6b02 CHECK (((dose_form)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_48e853b059d06f3c73526649dd6e9864 CHECK (((frequency_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4feaef9d8555bcbcfb2862535ab11b24 CHECK (((duration_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_5c119ce433e6b53b28ff1d02a33d6489 CHECK ((dosage_text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_602eb368d49743cf0f4ec7b30cbb4b32 CHECK ((last_administration_date_raw !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6220a6eb7ba9861db5d78e988c44e6e1 CHECK (((dose_form_termid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_62ea1dae393a463e57f5f7bdeb91002e CHECK ((route_termid_code_system !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6a725b8fb673fae13fff0ae38d6f2d56 CHECK (((route_termid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_956cd7ad8f7a940770c8c27a1194ceb0 CHECK (((dose_form_termid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9dc8fedbcb42083dab6803aa08560d0a CHECK (((parent_route)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a46d5413f7b7d31a9fa7c8828e0625b2 CHECK ((first_administration_date_raw !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a756541e0d866ccad9259435171d8f2f CHECK (((batch_lot_number_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ac374b1ee8e496a9aed6c6191a0cbab3 CHECK (((batch_lot_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c07eb731c09bdac33f1716e52a768507 CHECK (((parent_route_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cbc74eb5c9186135d123c6636021b403 CHECK (((route_termid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d4061007bf70b2e3e721763b1df9f790 CHECK (((last_administration_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d5e1d2fae6f1d35725800edafec1a85e CHECK (((parent_route_termid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_de839746debb9aed4fa1e57d347f25f6 CHECK (((route_of_administration)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e42a4d78ebd13c077c596c71ff8db130 CHECK (((parent_route_termid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_fcc73f3d7d3b97f73f6ef0acbceae0b6 CHECK (((first_administration_date_null_flavor)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.dosage_information FORCE ROW LEVEL SECURITY;


--
-- Name: drug_active_substances; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.drug_active_substances (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    drug_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    substance_name character varying(500),
    substance_termid character varying(100),
    substance_termid_version character varying(10),
    substance_termid_code_system text,
    mfds_version character varying(20),
    mfds_id character varying(10),
    strength_value numeric(15,5),
    strength_unit character varying(50),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_59e1a8d4ecf174e6cdf89e800bbf214c CHECK ((substance_termid_code_system !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_66f276dea9b178ab450783de5eb07be2 CHECK (((mfds_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6bb7a56ead4713814decba6c54ed02d4 CHECK (((substance_termid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_73147fa057f5570f178754da4a81de61 CHECK (((mfds_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a0f608f6a176e4ee49444443dc04625e CHECK (((strength_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a8c6311c3ff888b9512c3116936f362d CHECK (((substance_termid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f9262803afc22a311e3f0ab40fe8751e CHECK (((substance_name)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.drug_active_substances FORCE ROW LEVEL SECURITY;


--
-- Name: drug_device_characteristics; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.drug_device_characteristics (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    drug_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    code character varying(50),
    code_system character varying(200),
    code_display_name character varying(200),
    value_type character varying(10),
    value_value character varying(200),
    value_code character varying(50),
    value_code_system character varying(200),
    value_display_name character varying(200),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_2dcaebc0f474145ce78f87e0fa40b995 CHECK (((value_display_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8c34f5cb965484158cff2fbcff927665 CHECK (((code_system)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a01025342dadd5fe1b2b8827cc783cb4 CHECK (((value_code_system)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a54da84141a25ebfb9dda2a29e2760b5 CHECK (((value_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b0e2e066e6cb9595bf4124c14fcca423 CHECK (((code_display_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c13367945d5d4c91047b3b50234aa7ab CHECK (((code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c89c3e1fa965c20ae511e0cb713ba0fd CHECK (((value_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f891bb5dddc77b90459618efdb45f533 CHECK (((value_value)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.drug_device_characteristics FORCE ROW LEVEL SECURITY;


--
-- Name: drug_indications; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.drug_indications (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    drug_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    indication_text character varying(500),
    indication_text_null_flavor character varying(4),
    indication_meddra_version character varying(10),
    indication_meddra_code character varying(20),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_drug_indication CHECK (((indication_text_null_flavor IS NULL) OR ((indication_text_null_flavor)::text = ANY (ARRAY[('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nfv_drug_indications_ebf58bfb5ae6 CHECK (((indication_text IS NULL) OR (indication_text_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_001fa348b794dd8c38146d6453fe4aac CHECK (((indication_text_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_60074d60499f2e98bab4b99966635a70 CHECK (((indication_meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6be6c721f92968c14846c3b7d59d1b9f CHECK (((indication_meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7aa18a7947923cf7594f014481bb57f6 CHECK (((indication_text)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.drug_indications FORCE ROW LEVEL SECURITY;


--
-- Name: drug_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.drug_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    source_product_presave_id uuid,
    source_product_id character varying(255),
    sequence_number integer NOT NULL,
    drug_characterization character varying(1),
    medicinal_product character varying(500),
    mpid character varying(1000),
    mpid_version character varying(20),
    mpid_source_code_system text,
    mpid_source_code_system_version text,
    mfds_mpid_version character varying(20),
    mfds_mpid character varying(10),
    phpid character varying(250),
    phpid_version character varying(10),
    investigational_product_blinded boolean,
    obtain_drug_country character varying(2),
    drug_authorization_number character varying(100),
    manufacturer_name character varying(100),
    manufacturer_country character varying(2),
    batch_lot_number character varying(200),
    cumulative_dose_first_reaction_value numeric(15,5),
    cumulative_dose_first_reaction_unit character varying(50),
    gestation_period_exposure_value numeric(10,2),
    gestation_period_exposure_unit character varying(50),
    action_taken character varying(1),
    fda_additional_info_coded character varying(10),
    fda_additional_info_coded_null_flavor character varying(2),
    drug_additional_info_codes_json jsonb,
    drug_additional_information text,
    fda_specialized_product_category character varying(60),
    fda_other_characterization character varying(10),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    source_xml_id text,
    CONSTRAINT ck_nf_fda_drug_additional_info CHECK (((fda_additional_info_coded_null_flavor IS NULL) OR ((fda_additional_info_coded_null_flavor)::text = 'NA'::text))),
    CONSTRAINT ck_nfv_drug_information_a8697c43de68 CHECK (((fda_additional_info_coded IS NULL) OR (fda_additional_info_coded_null_flavor IS NULL))),
    CONSTRAINT ck_pair_fda_drug_additional_info CHECK ((NOT ((fda_additional_info_coded IS NOT NULL) AND (fda_additional_info_coded_null_flavor IS NOT NULL)))),
    CONSTRAINT drug_information_action_taken_check CHECK (((action_taken)::text = ANY (ARRAY[('0'::character varying)::text, ('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text, ('5'::character varying)::text, ('6'::character varying)::text, ('9'::character varying)::text]))),
    CONSTRAINT drug_information_drug_characterization_check CHECK (((drug_characterization)::text = ANY (ARRAY[(''::character varying)::text, ('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text]))),
    CONSTRAINT drug_information_source_xml_id_check CHECK (((source_xml_id IS NULL) OR (length(btrim(source_xml_id)) > 0))),
    CONSTRAINT model_nonblank_079d0f2a4db10c52c4a73c57cfc32d30 CHECK (((obtain_drug_country)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_0a4286d64e27f56c6bb081cbb58f75dc CHECK (((drug_characterization)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_0aa35796a63223585e4f1e06e0147c5e CHECK (((medicinal_product)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_1ad1367c58755bf1ff227a275f0ea48b CHECK (((mpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_1d1b1a69f2b7550785f7037058a769a7 CHECK (((cumulative_dose_first_reaction_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_397b147870d8a427d123d52514b71d3b CHECK (((fda_specialized_product_category)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3d772e5a055aabdaa06d6ddf24709d67 CHECK ((source_xml_id !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3ff04447d4747f787abb90f204e7b5d0 CHECK (((fda_additional_info_coded_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_467d182e7d276e9508d94777a8e10cbb CHECK (((fda_additional_info_coded)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_54b0b85764f69689bf273885fadd2c00 CHECK (((manufacturer_country)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6527116e9c05497982ed1d512c09c228 CHECK ((mpid_source_code_system_version !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6fc8ae160ef4070316d6fe00d9be258b CHECK (((gestation_period_exposure_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_74a70fc68a9cb130f2fd3a7319768e6d CHECK (((drug_authorization_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7549a20c3aad58018f24f4daf154a0ee CHECK (((source_product_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_861b64238aab302eeea41444fbfee41f CHECK (((manufacturer_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9dd8bd2fa63eff9fe09b50cffa85a401 CHECK (((mfds_mpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ac374b1ee8e496a9aed6c6191a0cbab3 CHECK (((batch_lot_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b787662b9e836a37aaf7452b6ec744d8 CHECK (((phpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cc5f0b3b46164046d4cc74eaab4815b8 CHECK (((phpid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d07133ff686a3a236e33f53256715517 CHECK (((mfds_mpid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d5c37aafab0baebd537a381dc2814cdd CHECK ((mpid_source_code_system !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e03096a310c3101b8a39623aa9c2adaf CHECK (((mpid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e536a9f036f46561e67844f001236792 CHECK ((drug_additional_information !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e6047c968bb39dc3f81653737157694e CHECK (((fda_other_characterization)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_fb6e95556664f51a248331b52db0837c CHECK (((action_taken)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.drug_information FORCE ROW LEVEL SECURITY;


--
-- Name: drug_reaction_assessments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.drug_reaction_assessments (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    drug_id uuid NOT NULL,
    reaction_id uuid NOT NULL,
    administration_start_interval_value numeric(10,2),
    administration_start_interval_unit character varying(50),
    last_dose_interval_value numeric(10,2),
    last_dose_interval_unit character varying(50),
    recurrence_action character varying(1),
    reaction_recurred character varying(1),
    dechallenge_result character varying(1),
    expectedness character varying(1),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT drug_reaction_assessments_dechallenge_result_check CHECK (((dechallenge_result)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text]))),
    CONSTRAINT drug_reaction_assessments_expectedness_check CHECK (((expectedness IS NULL) OR ((expectedness)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text])))),
    CONSTRAINT drug_reaction_assessments_reaction_recurred_check CHECK (((reaction_recurred)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text]))),
    CONSTRAINT drug_reaction_assessments_recurrence_action_check CHECK (((recurrence_action)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text]))),
    CONSTRAINT model_nonblank_40c41c6b8731c940d31abae24c8771dc CHECK (((reaction_recurred)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_55b28d988a028c25d889c296fb105cf7 CHECK (((dechallenge_result)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7ee02b5dffa928d45b84f59cf1c39fe3 CHECK (((administration_start_interval_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a4cd954873762f51a4ec6d0aab53498d CHECK (((recurrence_action)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_bc996cfb018dc9373fabee45cdad7611 CHECK (((expectedness)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c79ff2f93bc759f30942454721d0a7df CHECK (((last_dose_interval_unit)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.drug_reaction_assessments FORCE ROW LEVEL SECURITY;


--
-- Name: e2b_code_lists; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.e2b_code_lists (
    id integer NOT NULL,
    audit_id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    list_name character varying(100) NOT NULL,
    code character varying(10) NOT NULL,
    display_name character varying(200) NOT NULL,
    description text,
    sort_order integer,
    active boolean DEFAULT true,
    CONSTRAINT model_nonblank_67daf92c833c41c95db874e18fcb2786 CHECK ((description !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c13367945d5d4c91047b3b50234aa7ab CHECK (((code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c2657f133e8615f7dba3ddfe13125bd4 CHECK (((display_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_efb2685bf8eb3ff15cae16362113f567 CHECK (((list_name)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.e2b_code_lists FORCE ROW LEVEL SECURITY;


--
-- Name: e2b_code_lists_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.e2b_code_lists_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: e2b_code_lists_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.e2b_code_lists_id_seq OWNED BY public.e2b_code_lists.id;


--
-- Name: e_signatures; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.e_signatures (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid,
    signer_user_id uuid NOT NULL,
    signer_username character varying(128) NOT NULL,
    action character varying(50) NOT NULL,
    meaning text NOT NULL,
    reason text NOT NULL,
    signature_method character varying(50) DEFAULT 'password_reentry'::character varying NOT NULL,
    signed_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_40bea8d637cdf2c1b07fcf0630482b73 CHECK ((reason !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_418c5509e2171d55b0aee5c2ea4442b5 CHECK (((action)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_dc81e93e07e117290ade32f4a201fdae CHECK (((signature_method)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_eca6efb21f6c59397bd6445a810a3501 CHECK (((signer_username)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f2f990f68492c267bd285d13e3e829b6 CHECK ((meaning !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.e_signatures FORCE ROW LEVEL SECURITY;


--
-- Name: fda_device_codes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fda_device_codes (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    device_id uuid NOT NULL,
    element character varying(20) NOT NULL,
    sequence_number integer NOT NULL,
    value_code character varying(7) NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT fda_device_codes_check CHECK (((((element)::text = 'follow_up_type'::text) AND ((value_code)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text]))) OR (((element)::text = 'device_problem'::text) AND ((char_length((value_code)::text) >= 1) AND (char_length((value_code)::text) <= 7))) OR (((element)::text = 'remedial_action'::text) AND ((value_code)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text, ('5'::character varying)::text, ('6'::character varying)::text, ('7'::character varying)::text, ('8'::character varying)::text, ('9'::character varying)::text]))))),
    CONSTRAINT fda_device_codes_element_check CHECK (((element)::text = ANY (ARRAY[('follow_up_type'::character varying)::text, ('device_problem'::character varying)::text, ('remedial_action'::character varying)::text]))),
    CONSTRAINT model_nonblank_8e2dcfd7e7e24b1ca76c1193f645902b CHECK (((element)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a54da84141a25ebfb9dda2a29e2760b5 CHECK (((value_code)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.fda_device_codes FORCE ROW LEVEL SECURITY;


--
-- Name: fda_device_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fda_device_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    drug_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    malfunction boolean,
    device_brand_name character varying(80),
    device_brand_name_null_flavor character varying(2),
    common_device_name character varying(80),
    common_device_name_null_flavor character varying(2),
    device_product_code character varying(10),
    manufacturer_name character varying(100),
    manufacturer_address character varying(100),
    manufacturer_city character varying(35),
    manufacturer_state character varying(40),
    manufacturer_country character varying(2),
    device_usage character varying(1),
    device_lot_number character varying(100),
    operator_of_device character varying(1),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_fda_device_informati_4d9b8e2e5daa CHECK (((common_device_name IS NULL) OR (common_device_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_fda_device_informati_5eeec24d1a9c CHECK (((device_brand_name IS NULL) OR (device_brand_name_null_flavor IS NULL))),
    CONSTRAINT fda_device_information_common_device_name_null_flavor_check CHECK (((common_device_name_null_flavor)::text = 'NI'::text)),
    CONSTRAINT fda_device_information_device_brand_name_null_flavor_check CHECK (((device_brand_name_null_flavor)::text = 'NI'::text)),
    CONSTRAINT fda_device_information_device_usage_check CHECK (((device_usage)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text]))),
    CONSTRAINT fda_device_information_operator_of_device_check CHECK (((operator_of_device)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text]))),
    CONSTRAINT model_nonblank_05011493b082c6deb67901b449e2e96e CHECK (((device_brand_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_2564a9c83416beb93788fb411f87a61c CHECK (((manufacturer_address)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_52cbb6dcfcf6f5e60655e126cbec5f49 CHECK (((common_device_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_54b0b85764f69689bf273885fadd2c00 CHECK (((manufacturer_country)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_65c765f49e45aaff2ad7f8939fe87b28 CHECK (((device_usage)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6e9d19b52a8e21bc72f5bda03025e269 CHECK (((manufacturer_state)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7cffdc15f958d7e343975b35d285e2d9 CHECK (((common_device_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_861b64238aab302eeea41444fbfee41f CHECK (((manufacturer_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8827c657189ccab34bcf407fae2a6cbf CHECK (((device_lot_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_93af83c57f377e57fee11a8a13b594ed CHECK (((manufacturer_city)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e1967d619318607d2b7d2fd0055b4b42 CHECK (((device_brand_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e589a943fc21e213b653f2da436f9311 CHECK (((operator_of_device)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ffda574c351497a85d8a4a87f8028560 CHECK (((device_product_code)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.fda_device_information FORCE ROW LEVEL SECURITY;


--
-- Name: fda_hierarchical_code_lists; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fda_hierarchical_code_lists (
    id bigint NOT NULL,
    audit_id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    list_name character varying(50) NOT NULL,
    ncit_concept_code character varying(20) NOT NULL,
    level1_term character varying(200) NOT NULL,
    level2_term character varying(200),
    level3_term character varying(200),
    fda_code character varying(20) NOT NULL,
    imdrf_code character varying(20) NOT NULL,
    CONSTRAINT model_nonblank_1172e6eb65ecc4324f5d14edd15e9ecb CHECK (((ncit_concept_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_211a2e2e17c5674902f2327963271cdc CHECK (((level2_term)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_243a3e12a4931fea580d17a1a18658fd CHECK (((fda_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_35033f41443ea6722e6cd8cd9bab04d1 CHECK (((imdrf_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_99e8058392e026cbf876c0506c3513cf CHECK (((level3_term)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a2c5186f68a6e408a3db5743284b5cbf CHECK (((level1_term)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_efb2685bf8eb3ff15cae16362113f567 CHECK (((list_name)::text !~ '^[[:space:]]*$'::text))
);


--
-- Name: fda_hierarchical_code_lists_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.fda_hierarchical_code_lists_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: fda_hierarchical_code_lists_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.fda_hierarchical_code_lists_id_seq OWNED BY public.fda_hierarchical_code_lists.id;


--
-- Name: iso_countries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.iso_countries (
    code character varying(2) NOT NULL,
    audit_id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    name character varying(200) NOT NULL,
    active boolean DEFAULT true,
    CONSTRAINT model_nonblank_b068931cc450442b63f5b3d276ea4297 CHECK (((name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c13367945d5d4c91047b3b50234aa7ab CHECK (((code)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.iso_countries FORCE ROW LEVEL SECURITY;


--
-- Name: linked_report_numbers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.linked_report_numbers (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    linked_report_number character varying(100),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_051ed6af213b0581ce5926bf140cf1a8 CHECK (((linked_report_number)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.linked_report_numbers FORCE ROW LEVEL SECURITY;


--
-- Name: literature_references; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.literature_references (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    reference_text text,
    reference_text_null_flavor character varying(4),
    sequence_number integer NOT NULL,
    document_base64 text,
    file_name text,
    media_type character varying(100),
    representation character varying(10),
    compression character varying(10),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_literature_reference_4f69f1cf487a CHECK (((reference_text IS NULL) OR (reference_text_null_flavor IS NULL))),
    CONSTRAINT literature_references_reference_text_null_flavor_check CHECK (((reference_text_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT model_nonblank_068b5685f1afe2ad6070538a40a932a0 CHECK (((reference_text_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_07f5d2d550d5961d1dd0984fe666a2f6 CHECK (((compression)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3090ad88ff7c9ca191579b709069fe64 CHECK ((file_name !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_335601aab95ba3f4e0380777ca3ce8f5 CHECK ((document_base64 !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_80a701d4b70ebbe74c0af8763367fd76 CHECK (((media_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_eba973109e866d102e79ac927b4b4512 CHECK ((reference_text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ee16755af8c89d0c26540daf7a0fe1b3 CHECK (((representation)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.literature_references FORCE ROW LEVEL SECURITY;


--
-- Name: meddra_terms; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.meddra_terms (
    id bigint NOT NULL,
    audit_id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    code character varying(20) NOT NULL,
    term character varying(500) NOT NULL,
    level character varying(10) NOT NULL,
    version character varying(10) NOT NULL,
    language character varying(2) DEFAULT 'en'::character varying,
    active boolean DEFAULT true,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_2af72f100c356273d46284f6fd1dfc08 CHECK (((version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8512ae7d57b1396273f76fe6ed341a23 CHECK (((language)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b4dad0fe5fbef2c0e24d9db1cc69e5a2 CHECK (((term)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c13367945d5d4c91047b3b50234aa7ab CHECK (((code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c9e9a848920877e76685b2e4e76de38d CHECK (((level)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.meddra_terms FORCE ROW LEVEL SECURITY;


--
-- Name: meddra_terms_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.meddra_terms_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: meddra_terms_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.meddra_terms_id_seq OWNED BY public.meddra_terms.id;


--
-- Name: medical_history_episodes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.medical_history_episodes (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    patient_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    meddra_version character varying(10),
    meddra_code character varying(20),
    start_date text,
    start_date_null_flavor character varying(4),
    continuing boolean,
    continuing_null_flavor character varying(4),
    end_date text,
    end_date_null_flavor character varying(4),
    comments text,
    family_history boolean,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_med_history_end CHECK (((end_date_null_flavor IS NULL) OR ((end_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_med_history_start CHECK (((start_date_null_flavor IS NULL) OR ((start_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nfv_medical_history_epis_29e0f3a3c311 CHECK (((continuing IS NULL) OR (continuing_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_medical_history_epis_a90db801a6d0 CHECK (((end_date IS NULL) OR (end_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_medical_history_epis_b192f8a6a0e9 CHECK (((start_date IS NULL) OR (start_date_null_flavor IS NULL))),
    CONSTRAINT medical_history_episodes_continuing_null_flavor_check CHECK (((continuing_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT model_nonblank_07f18aec49991b58247249ea7a6865cd CHECK (((continuing_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_63c55f93a56ff18a74048153bb2aff09 CHECK (((meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_90883ccd002fbe4c7a849fa7acd9fe43 CHECK (((meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_90e8da89bb462061b421c1cb7191de14 CHECK ((end_date !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_951724fcd1707c477ce2b789d96c3a0d CHECK (((end_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a06f51a9a027cc8e6cec7f04dde57f23 CHECK (((start_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a5d491060952aa8ad5fdee071be752de CHECK ((comments !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_eadbb9116952e42cd59aab00b0e4e11a CHECK ((start_date !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.medical_history_episodes FORCE ROW LEVEL SECURITY;


--
-- Name: mfds_product_substances; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.mfds_product_substances (
    id bigint NOT NULL,
    audit_id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    item_seq character varying(10) NOT NULL,
    substance_code character varying(40) NOT NULL,
    substance_name_kr text NOT NULL,
    substance_name_en text,
    quantity text,
    unit text,
    component_content text,
    material_sequence text,
    total_amount_sequence text,
    version character varying(40) NOT NULL,
    active boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_221d2a4bfdae13dbd5aeff3b02adb8c1 CHECK ((quantity !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_2419145c08d8a1cb81ee8cba6501c3fb CHECK (((substance_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_2af72f100c356273d46284f6fd1dfc08 CHECK (((version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3a63e4a7f5ba871b0fdda607e3d8d643 CHECK ((component_content !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3af30fc058f5288aefda71f90ae96ff2 CHECK ((substance_name_en !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3e34bdebd9bd5edda27e8728904a2552 CHECK ((unit !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_440f550d199c1be8812d6132bae49275 CHECK (((item_seq)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_714ad1f6e07b7b23010a8ac5a3eef743 CHECK ((material_sequence !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_74cde074206974a07b06040fd2c64706 CHECK ((total_amount_sequence !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a069fd779b508609a97fd4bd326f9328 CHECK ((substance_name_kr !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.mfds_product_substances FORCE ROW LEVEL SECURITY;


--
-- Name: mfds_product_substances_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.mfds_product_substances_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: mfds_product_substances_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.mfds_product_substances_id_seq OWNED BY public.mfds_product_substances.id;


--
-- Name: mfds_products; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.mfds_products (
    id bigint NOT NULL,
    audit_id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    item_seq character varying(10) NOT NULL,
    product_name_kr text NOT NULL,
    product_name_en text,
    manufacturer_name_kr text,
    manufacturer_name_en text,
    permit_date date,
    cancellation_date date,
    cancellation_status text,
    version character varying(40) NOT NULL,
    active boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_2126e9abf522c027c4429b3e7948fd6d CHECK ((product_name_en !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_2af72f100c356273d46284f6fd1dfc08 CHECK (((version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_440f550d199c1be8812d6132bae49275 CHECK (((item_seq)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_45a8d95a25eeb34387f92fbeca1e384b CHECK ((manufacturer_name_en !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_57b8feb337a510a51c2f9c751367d305 CHECK ((manufacturer_name_kr !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b9cb40bb33ae349ac6228f78c1f6f047 CHECK ((cancellation_status !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ba5e646d70272e1809ad15fe1c8c45c1 CHECK ((product_name_kr !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.mfds_products FORCE ROW LEVEL SECURITY;


--
-- Name: mfds_products_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.mfds_products_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: mfds_products_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.mfds_products_id_seq OWNED BY public.mfds_products.id;


--
-- Name: narrative_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.narrative_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    source_narrative_presave_id uuid,
    case_narrative text,
    reporter_comments text,
    sender_comments text,
    additional_information text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_a35f564798944667e91c53b3a3da359d CHECK ((case_narrative !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_bb2bca0dbea06d35351de4b0c82d2ebb CHECK ((additional_information !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e7dee51c2476c304be6d2e3d8114c546 CHECK ((sender_comments !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_fd19dacf3ceecd9960bfe00e05557bf8 CHECK ((reporter_comments !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.narrative_information FORCE ROW LEVEL SECURITY;


--
-- Name: narrative_presaves; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.narrative_presaves (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    template_title character varying(255),
    case_narrative text,
    case_narrative_notation text,
    additional_information text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_8487c3cb135c0728e2c258727e131629 CHECK ((case_narrative_notation !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a35f564798944667e91c53b3a3da359d CHECK ((case_narrative !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_bb2bca0dbea06d35351de4b0c82d2ebb CHECK ((additional_information !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e0207853be5ec4eb99ed65b51d8d6966 CHECK (((template_title)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.narrative_presaves FORCE ROW LEVEL SECURITY;


--
-- Name: organization_as2_profiles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.organization_as2_profiles (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    authority character varying(16) NOT NULL,
    endpoint_url text NOT NULL,
    as2_to character varying(255) NOT NULL,
    expected_ack_as2_from character varying(255) NOT NULL,
    extra_headers jsonb DEFAULT '{}'::jsonb NOT NULL,
    signing_pkcs12_ciphertext bytea NOT NULL,
    signing_password_ciphertext bytea NOT NULL,
    signing_key_alias character varying(255),
    partner_certificate_pem text NOT NULL,
    async_mdn_url text,
    active boolean DEFAULT true NOT NULL,
    version bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT organization_as2_profiles_authority_check CHECK (((authority)::text = ANY ((ARRAY['fda'::character varying, 'mfds'::character varying])::text[]))),
    CONSTRAINT organization_as2_profiles_version_check CHECK ((version > 0))
);

ALTER TABLE ONLY public.organization_as2_profiles FORCE ROW LEVEL SECURITY;


--
-- Name: organization_policy_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.organization_policy_state (
    organization_id uuid NOT NULL,
    organization_type text,
    revision bigint DEFAULT 1 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_852fb0cd92d4c971b2360c3d41409ce6 CHECK ((organization_type !~ '^[[:space:]]*$'::text)),
    CONSTRAINT organization_policy_state_revision_check CHECK ((revision > 0))
);


--
-- Name: organizations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.organizations (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    name character varying(500) NOT NULL,
    org_type character varying(100),
    address text,
    city character varying(200),
    state character varying(100),
    postcode character varying(50),
    country_code character varying(2),
    contact_email character varying(255),
    contact_phone character varying(50),
    active boolean DEFAULT true,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_4ed5d2eaed1a1fadcc41ad1d58ed603e CHECK (((city)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_55eceb8dde5fdfa0e4bfa50de80513e2 CHECK (((country_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_688f95ebf56cd8171bc98ec1a1634621 CHECK (((contact_phone)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7101816d966577c77a30f9dd1b141077 CHECK (((org_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_884d9804999fc47a3c2694e49ad2536a CHECK ((address !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9ed39e2ea931586b6a985a6942ef573e CHECK (((state)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b068931cc450442b63f5b3d276ea4297 CHECK (((name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b9973d8c69683b273205eb67914beada CHECK (((contact_email)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e90ebd9556fa4031171f043013794b61 CHECK (((postcode)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.organizations FORCE ROW LEVEL SECURITY;


--
-- Name: other_case_identifiers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.other_case_identifiers (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    source_of_identifier character varying(100),
    case_identifier character varying(100),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_a0cfd4c65a42529171f3b1992ace2dc6 CHECK (((source_of_identifier)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f46c787f7036a8dd9855c8d6125ec6b6 CHECK (((case_identifier)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.other_case_identifiers FORCE ROW LEVEL SECURITY;


--
-- Name: parent_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.parent_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    patient_id uuid NOT NULL,
    parent_identification character varying(60),
    parent_identification_null_flavor character varying(4),
    parent_birth_date date,
    parent_birth_date_null_flavor character varying(4),
    parent_age numeric(5,2),
    parent_age_unit character varying(50),
    last_menstrual_period_date date,
    last_menstrual_period_date_null_flavor character varying(4),
    weight_kg numeric(6,2),
    height_cm numeric(6,2),
    sex character varying(1),
    sex_null_flavor character varying(4),
    medical_history_text text,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_parent_birth CHECK (((parent_birth_date_null_flavor IS NULL) OR ((parent_birth_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_parent_lmp CHECK (((last_menstrual_period_date_null_flavor IS NULL) OR ((last_menstrual_period_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nfv_parent_information_2946dc9cea7e CHECK (((last_menstrual_period_date IS NULL) OR (last_menstrual_period_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_parent_information_39b886e49e13 CHECK (((sex IS NULL) OR (sex_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_parent_information_a3636f849a37 CHECK (((parent_identification IS NULL) OR (parent_identification_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_parent_information_cfb40a2a2d39 CHECK (((parent_birth_date IS NULL) OR (parent_birth_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_parent_information_identification CHECK (((parent_identification IS NULL) OR (parent_identification_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_parent_information_sex CHECK (((sex IS NULL) OR (sex_null_flavor IS NULL))),
    CONSTRAINT ck_parent_identification_null_flavor_allowed CHECK (((parent_identification_null_flavor IS NULL) OR ((parent_identification_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_parent_sex_null_flavor_allowed CHECK (((sex_null_flavor IS NULL) OR ((sex_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT model_nonblank_2996d7432049193dde6b016da03f90cb CHECK (((last_menstrual_period_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3c3662bcb661d6de679c636744c66b62 CHECK (((sex)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3ca99f7d8a7947ca39cdaac539e44da7 CHECK (((parent_age_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_47f0c9d8c699a39d932049ff27add602 CHECK ((medical_history_text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4979831ee9a3adc10ca5cd4f6c3c889f CHECK (((parent_identification_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_574e26388cb46fdd5df54d6d14dca1a7 CHECK (((parent_identification)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_77d852154f5f122848db1ffa204a5e8e CHECK (((parent_birth_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b24db916eb6c06a3a89f7cbee6d23d90 CHECK (((sex_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT parent_information_parent_identification_null_flavor_check CHECK (((parent_identification_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT parent_information_sex_check CHECK (((sex)::text = ANY (ARRAY[('0'::character varying)::text, ('1'::character varying)::text, ('2'::character varying)::text]))),
    CONSTRAINT parent_information_sex_null_flavor_check CHECK (((sex_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))
);

ALTER TABLE ONLY public.parent_information FORCE ROW LEVEL SECURITY;


--
-- Name: parent_medical_history; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.parent_medical_history (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    parent_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    meddra_version character varying(10),
    meddra_code character varying(20),
    start_date date,
    start_date_null_flavor character varying(4),
    continuing boolean,
    continuing_null_flavor character varying(4),
    end_date date,
    end_date_null_flavor character varying(4),
    comments text,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_parent_history_end CHECK (((end_date_null_flavor IS NULL) OR ((end_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_parent_history_start CHECK (((start_date_null_flavor IS NULL) OR ((start_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nfv_parent_medical_histo_47d96d4223ba CHECK (((continuing IS NULL) OR (continuing_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_parent_medical_histo_5bb18877ce87 CHECK (((start_date IS NULL) OR (start_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_parent_medical_histo_8a79c08f831f CHECK (((end_date IS NULL) OR (end_date_null_flavor IS NULL))),
    CONSTRAINT ck_parent_medical_history_continuing_null_flavor_allowed CHECK (((continuing_null_flavor IS NULL) OR ((continuing_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_parent_medical_history_continuing_null_flavor_pair CHECK (((continuing IS NULL) OR (continuing_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_07f18aec49991b58247249ea7a6865cd CHECK (((continuing_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_63c55f93a56ff18a74048153bb2aff09 CHECK (((meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_90883ccd002fbe4c7a849fa7acd9fe43 CHECK (((meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_951724fcd1707c477ce2b789d96c3a0d CHECK (((end_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a06f51a9a027cc8e6cec7f04dde57f23 CHECK (((start_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a5d491060952aa8ad5fdee071be752de CHECK ((comments !~ '^[[:space:]]*$'::text)),
    CONSTRAINT parent_medical_history_check CHECK (((continuing IS NULL) OR (continuing_null_flavor IS NULL))),
    CONSTRAINT parent_medical_history_continuing_null_flavor_check CHECK (((continuing_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))
);

ALTER TABLE ONLY public.parent_medical_history FORCE ROW LEVEL SECURITY;


--
-- Name: parent_past_drug_history; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.parent_past_drug_history (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    parent_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    drug_name character varying(500),
    mpid character varying(1000),
    mpid_version character varying(10),
    mfds_medicinal_product_version character varying(20),
    mfds_medicinal_product_id character varying(10),
    phpid character varying(250),
    phpid_version character varying(10),
    start_date date,
    start_date_null_flavor character varying(4),
    end_date date,
    end_date_null_flavor character varying(4),
    indication_meddra_version character varying(10),
    indication_meddra_code character varying(20),
    reaction_meddra_version character varying(10),
    reaction_meddra_code character varying(20),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_parent_past_drug_his_1012ad4b84e4 CHECK (((start_date IS NULL) OR (start_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_parent_past_drug_his_3088219e097d CHECK (((end_date IS NULL) OR (end_date_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_1ad1367c58755bf1ff227a275f0ea48b CHECK (((mpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_2116857e8c67c904c01ced46778e55e2 CHECK (((mfds_medicinal_product_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3343e301e76811eae5cc6186f16b5f14 CHECK (((mfds_medicinal_product_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3c92ae49fa8a2ba9656998c9ca122625 CHECK (((reaction_meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_60074d60499f2e98bab4b99966635a70 CHECK (((indication_meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6be6c721f92968c14846c3b7d59d1b9f CHECK (((indication_meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_951724fcd1707c477ce2b789d96c3a0d CHECK (((end_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9db9d5b59453cf9650b807b2708d1c82 CHECK (((reaction_meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a06f51a9a027cc8e6cec7f04dde57f23 CHECK (((start_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b787662b9e836a37aaf7452b6ec744d8 CHECK (((phpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cc5f0b3b46164046d4cc74eaab4815b8 CHECK (((phpid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d8c1dd46d45c5c380ea8f39d6010506e CHECK (((drug_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e03096a310c3101b8a39623aa9c2adaf CHECK (((mpid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT parent_past_drug_history_end_date_null_flavor_check CHECK (((end_date_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text, ('MSK'::character varying)::text]))),
    CONSTRAINT parent_past_drug_history_start_date_null_flavor_check CHECK (((start_date_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text, ('MSK'::character varying)::text])))
);

ALTER TABLE ONLY public.parent_past_drug_history FORCE ROW LEVEL SECURITY;


--
-- Name: past_drug_history; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.past_drug_history (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    patient_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    drug_name character varying(500),
    drug_name_null_flavor character varying(4),
    mfds_medicinal_product_version character varying(20),
    mfds_medicinal_product_id character varying(10),
    mpid character varying(1000),
    mpid_version character varying(20),
    mpid_source_code_system text,
    mpid_source_code_system_version text,
    phpid character varying(250),
    phpid_version character varying(10),
    start_date date,
    start_date_null_flavor character varying(4),
    end_date date,
    end_date_null_flavor character varying(4),
    indication_meddra_version character varying(10),
    indication_meddra_code character varying(20),
    reaction_meddra_version character varying(10),
    reaction_meddra_code character varying(20),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_past_drug_end CHECK (((end_date_null_flavor IS NULL) OR ((end_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_past_drug_name CHECK (((drug_name_null_flavor IS NULL) OR ((drug_name_null_flavor)::text = ANY (ARRAY[('UNK'::character varying)::text, ('NA'::character varying)::text])))),
    CONSTRAINT ck_nf_past_drug_start CHECK (((start_date_null_flavor IS NULL) OR ((start_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nfv_past_drug_history_2dddb65ef73a CHECK (((start_date IS NULL) OR (start_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_past_drug_history_3b4c9ed8dfb4 CHECK (((drug_name IS NULL) OR (drug_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_past_drug_history_554b5fb02f15 CHECK (((end_date IS NULL) OR (end_date_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_1ad1367c58755bf1ff227a275f0ea48b CHECK (((mpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_2116857e8c67c904c01ced46778e55e2 CHECK (((mfds_medicinal_product_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3343e301e76811eae5cc6186f16b5f14 CHECK (((mfds_medicinal_product_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3c92ae49fa8a2ba9656998c9ca122625 CHECK (((reaction_meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_60074d60499f2e98bab4b99966635a70 CHECK (((indication_meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6527116e9c05497982ed1d512c09c228 CHECK ((mpid_source_code_system_version !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6be6c721f92968c14846c3b7d59d1b9f CHECK (((indication_meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_951724fcd1707c477ce2b789d96c3a0d CHECK (((end_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9db9d5b59453cf9650b807b2708d1c82 CHECK (((reaction_meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a06f51a9a027cc8e6cec7f04dde57f23 CHECK (((start_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b787662b9e836a37aaf7452b6ec744d8 CHECK (((phpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cc5f0b3b46164046d4cc74eaab4815b8 CHECK (((phpid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d5c37aafab0baebd537a381dc2814cdd CHECK ((mpid_source_code_system !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d8c1dd46d45c5c380ea8f39d6010506e CHECK (((drug_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_dcf9e272d6cad7e5e074a4718b320c05 CHECK (((drug_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e03096a310c3101b8a39623aa9c2adaf CHECK (((mpid_version)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.past_drug_history FORCE ROW LEVEL SECURITY;


--
-- Name: patient_death_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.patient_death_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    patient_id uuid NOT NULL,
    date_of_death date,
    date_of_death_null_flavor character varying(4),
    autopsy_performed boolean,
    autopsy_performed_null_flavor character varying(4),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_patient_autopsy CHECK (((autopsy_performed_null_flavor IS NULL) OR ((autopsy_performed_null_flavor)::text = ANY (ARRAY[('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_patient_death_date CHECK (((date_of_death_null_flavor IS NULL) OR ((date_of_death_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nfv_patient_death_inform_8c1cfe838bcc CHECK (((autopsy_performed IS NULL) OR (autopsy_performed_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_patient_death_inform_97601d42a8d3 CHECK (((date_of_death IS NULL) OR (date_of_death_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_b1d1efa95b441d49c42986eb7f601544 CHECK (((date_of_death_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f14dbaf9867cfe55776cecccb814bf21 CHECK (((autopsy_performed_null_flavor)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.patient_death_information FORCE ROW LEVEL SECURITY;


--
-- Name: patient_identifiers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.patient_identifiers (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    patient_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    identifier_type_code character varying(1) NOT NULL,
    identifier_value character varying(100),
    identifier_value_null_flavor character varying(4),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_patient_identifiers_05487b0bcf88 CHECK (((identifier_value IS NULL) OR (identifier_value_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_36ffaed85237e55c31b2d0bf9616d405 CHECK (((identifier_value)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_61e53aff9c53825fb137ca77b72f8c79 CHECK (((identifier_type_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ff3b32049070d215c80a20932a554424 CHECK (((identifier_value_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT patient_identifiers_identifier_value_null_flavor_check CHECK (((identifier_value_null_flavor)::text = 'MSK'::text))
);

ALTER TABLE ONLY public.patient_identifiers FORCE ROW LEVEL SECURITY;


--
-- Name: patient_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.patient_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    patient_initials character varying(60),
    birth_date date,
    age_at_time_of_onset numeric(5,2),
    age_unit character varying(50),
    gestation_period numeric(5,2),
    gestation_period_unit character varying(50),
    age_group character varying(1),
    weight_kg numeric(6,2),
    height_cm numeric(6,2),
    sex character varying(1),
    race_codes character varying(10)[] DEFAULT '{}'::character varying[] NOT NULL,
    race_code_null_flavor character varying(4),
    ethnicity_code character varying(10),
    ethnicity_code_null_flavor character varying(4),
    last_menstrual_period_date date,
    last_menstrual_period_date_null_flavor character varying(4),
    medical_history_text text,
    medical_history_text_null_flavor character varying(4),
    concomitant_therapy boolean,
    patient_initials_null_flavor character varying(4),
    birth_date_null_flavor character varying(4),
    sex_null_flavor character varying(4),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_patient_birth CHECK (((birth_date_null_flavor IS NULL) OR ((birth_date_null_flavor)::text = 'MSK'::text))),
    CONSTRAINT ck_nf_patient_ethnicity CHECK (((ethnicity_code_null_flavor IS NULL) OR ((ethnicity_code_null_flavor)::text = ANY (ARRAY[('NI'::character varying)::text, ('MSK'::character varying)::text, ('UNK'::character varying)::text, ('NA'::character varying)::text])))),
    CONSTRAINT ck_nf_patient_initials CHECK (((patient_initials_null_flavor IS NULL) OR ((patient_initials_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text, ('NA'::character varying)::text])))),
    CONSTRAINT ck_nf_patient_lmp CHECK (((last_menstrual_period_date_null_flavor IS NULL) OR ((last_menstrual_period_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_patient_race CHECK (((race_code_null_flavor IS NULL) OR ((race_code_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('NA'::character varying)::text, ('OTH'::character varying)::text])))),
    CONSTRAINT ck_nf_patient_sex CHECK (((sex_null_flavor IS NULL) OR ((sex_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nfv_patient_information_1b3f2ebfc025 CHECK (((last_menstrual_period_date IS NULL) OR (last_menstrual_period_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_patient_information_235c417accb9 CHECK (((patient_initials IS NULL) OR (patient_initials_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_patient_information_5cb08d64c78d CHECK (((birth_date IS NULL) OR (birth_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_patient_information_87e52f14e13c CHECK (((ethnicity_code IS NULL) OR (ethnicity_code_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_patient_information_d141c6a9f69c CHECK (((medical_history_text IS NULL) OR (medical_history_text_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_patient_information_f18d11fe4b2f CHECK (((sex IS NULL) OR (sex_null_flavor IS NULL))),
    CONSTRAINT ck_patient_race_codes CHECK ((race_codes <@ ARRAY['C16352'::character varying(10), 'C41259'::character varying(10), 'C41260'::character varying(10), 'C41219'::character varying(10), 'C41261'::character varying(10)])),
    CONSTRAINT ck_patient_race_null_flavor_pair CHECK (((cardinality(race_codes) = 0) OR (race_code_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_0a40feea9ddaa115be19d859787e9682 CHECK (((birth_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_265ced44a55652e2e9f19cc261a4efb8 CHECK (((medical_history_text_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_2996d7432049193dde6b016da03f90cb CHECK (((last_menstrual_period_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3c3662bcb661d6de679c636744c66b62 CHECK (((sex)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_47f0c9d8c699a39d932049ff27add602 CHECK ((medical_history_text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6144afee780a5019c124f9832d6598d5 CHECK (((patient_initials)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6841350aa732d4be3e47c1edcaeb54f6 CHECK (((ethnicity_code_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7f830a4795ffa9ee50ea7baf61be25c6 CHECK (((age_group)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a3a251e147d399a289ee715d884b94ee CHECK (public.model_text_array_has_no_blank((race_codes)::text[])),
    CONSTRAINT model_nonblank_ac0b88c2799cf7fcba7b1b10ffe8b204 CHECK (((ethnicity_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b24db916eb6c06a3a89f7cbee6d23d90 CHECK (((sex_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b729e4d12c5f862c01273cb7d2e41040 CHECK (((race_code_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ba69760e9180b4d7c98ac93cbd7b6be9 CHECK (((patient_initials_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_bb9bc833869978b3f0d1486fe4559ffe CHECK (((age_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_da0cdff5a22a3fcc5ceabeac5f951595 CHECK (((gestation_period_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT patient_information_age_group_check CHECK (((age_group)::text = ANY (ARRAY[('0'::character varying)::text, ('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text, ('5'::character varying)::text, ('6'::character varying)::text]))),
    CONSTRAINT patient_information_medical_history_text_null_flavor_check CHECK (((medical_history_text_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT patient_information_sex_check CHECK (((sex)::text = ANY (ARRAY[('0'::character varying)::text, ('1'::character varying)::text, ('2'::character varying)::text])))
);

ALTER TABLE ONLY public.patient_information FORCE ROW LEVEL SECURITY;


--
-- Name: permission_profiles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.permission_profiles (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    name character varying(128) NOT NULL,
    description character varying(512),
    can_view boolean DEFAULT true NOT NULL,
    can_review boolean DEFAULT false NOT NULL,
    can_lock boolean DEFAULT false NOT NULL,
    can_admin boolean DEFAULT false NOT NULL,
    privileges_json jsonb DEFAULT '[]'::jsonb NOT NULL,
    built_in boolean DEFAULT false NOT NULL,
    editable boolean DEFAULT true NOT NULL,
    active boolean DEFAULT true NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_67daf92c833c41c95db874e18fcb2786 CHECK (((description)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b068931cc450442b63f5b3d276ea4297 CHECK (((name)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.permission_profiles FORCE ROW LEVEL SECURITY;


--
-- Name: primary_sources; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.primary_sources (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    source_reporter_presave_id uuid,
    sequence_number integer NOT NULL,
    reporter_title character varying(50),
    reporter_title_null_flavor character varying(4),
    reporter_given_name character varying(60),
    reporter_given_name_null_flavor character varying(4),
    reporter_middle_name character varying(60),
    reporter_middle_name_null_flavor character varying(4),
    reporter_family_name character varying(60),
    reporter_family_name_null_flavor character varying(4),
    organization character varying(60),
    organization_null_flavor character varying(4),
    department character varying(60),
    department_null_flavor character varying(4),
    street character varying(100),
    street_null_flavor character varying(4),
    city character varying(35),
    city_null_flavor character varying(4),
    state character varying(40),
    state_null_flavor character varying(4),
    postcode character varying(15),
    postcode_null_flavor character varying(4),
    telephone character varying(33),
    telephone_null_flavor character varying(4),
    country_code character varying(2),
    email character varying(100),
    email_null_flavor character varying(4),
    qualification character varying(1),
    qualification_null_flavor character varying(4),
    qualification_kr1 character varying(1),
    primary_source_regulatory character varying(1),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_primary_email CHECK (((email_null_flavor IS NULL) OR ((email_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_primary_qualification CHECK (((qualification_null_flavor IS NULL) OR ((qualification_null_flavor)::text = 'UNK'::text))),
    CONSTRAINT ck_nfv_primary_sources_0389c3022801 CHECK (((state IS NULL) OR (state_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_0505285bcca5 CHECK (((reporter_middle_name IS NULL) OR (reporter_middle_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_2258fbdbeeea CHECK (((telephone IS NULL) OR (telephone_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_3575d7734cea CHECK (((reporter_family_name IS NULL) OR (reporter_family_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_38c998aae18e CHECK (((reporter_title IS NULL) OR (reporter_title_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_4a9b6e449113 CHECK (((department IS NULL) OR (department_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_527278bf099a CHECK (((city IS NULL) OR (city_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_6377ac36377c CHECK (((postcode IS NULL) OR (postcode_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_786835a6251e CHECK (((email IS NULL) OR (email_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_8ed2afe445f7 CHECK (((reporter_given_name IS NULL) OR (reporter_given_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_f411630ac18f CHECK (((street IS NULL) OR (street_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_f97381223c85 CHECK (((qualification IS NULL) OR (qualification_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_primary_sources_fcebca9c940a CHECK (((organization IS NULL) OR (organization_null_flavor IS NULL))),
    CONSTRAINT ck_pair_primary_qualification CHECK ((NOT ((qualification IS NOT NULL) AND (qualification_null_flavor IS NOT NULL)))),
    CONSTRAINT model_nonblank_0af5269bbdeba6dc6eaa3068706dcd8e CHECK (((reporter_given_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_0c83f57c786a0b4a39efab23731c7ebc CHECK (((email)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_0e6a7e64db5e1a442bbb5e79ae81bc8c CHECK (((department_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_165cecd3a4bbb5ea094a8fffefcd76bb CHECK (((reporter_given_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_1f4a9685e37e4f940d07a9f6b43dc83c CHECK (((street)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3e92ae4858cfd4c52479c1b71cfdc196 CHECK (((postcode_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_459d9fca17e3a950deae755d13578292 CHECK (((department)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4ed5d2eaed1a1fadcc41ad1d58ed603e CHECK (((city)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_55eceb8dde5fdfa0e4bfa50de80513e2 CHECK (((country_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_5b90677dbcf90d6dfa1901f30536ae90 CHECK (((reporter_title_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_70a9a8f0c42ff96f14050d7db649c9d5 CHECK (((street_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_72e2eeeaf44323dd42b7121353dd0d73 CHECK (((qualification_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_745afbb0fb9b423be76f7287e5df44b8 CHECK (((email_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_85cf433bccd9960b447cee48f4b65429 CHECK (((state_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8b27d9ca661471106e24afe9e7b0cad1 CHECK (((qualification_kr1)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8ec2d8c778502907beaa134f8afbd239 CHECK (((city_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_914fb22f9e2005f408959c53e45ad20e CHECK (((reporter_title)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_934bc20046509870e266117294898038 CHECK (((qualification)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9ed39e2ea931586b6a985a6942ef573e CHECK (((state)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b4c1948c087fafc89a88450fcbb64c77 CHECK (((organization)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b734ba89bd8db0bbacfc4f685093578a CHECK (((organization_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b851b871af414a81f60c5804f559e43f CHECK (((telephone_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b9bb7e7b00a4ba1e0d15fa8b2485d8c4 CHECK (((telephone)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ba597e85fac2e5456c91262cedba01fc CHECK (((reporter_middle_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_bc11f60614574277a5706496f5c402d0 CHECK (((reporter_family_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cfc7f4e630835101cdefa94311535cf3 CHECK (((reporter_family_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d0361b39351dee98219254e7d3a05aab CHECK (((reporter_middle_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e90ebd9556fa4031171f043013794b61 CHECK (((postcode)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ef67ed101cd37565a1f93ffbdef533b1 CHECK (((primary_source_regulatory)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT primary_sources_city_null_flavor_check CHECK (((city_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_department_null_flavor_check CHECK (((department_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_organization_null_flavor_check CHECK (((organization_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_postcode_null_flavor_check CHECK (((postcode_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_primary_source_regulatory_check CHECK (((primary_source_regulatory)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text]))),
    CONSTRAINT primary_sources_qualification_check CHECK (((qualification)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text, ('5'::character varying)::text]))),
    CONSTRAINT primary_sources_qualification_kr1_check CHECK (((qualification_kr1)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text]))),
    CONSTRAINT primary_sources_reporter_family_name_null_flavor_check CHECK (((reporter_family_name_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_reporter_given_name_null_flavor_check CHECK (((reporter_given_name_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_reporter_middle_name_null_flavor_check CHECK (((reporter_middle_name_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_reporter_title_null_flavor_check CHECK (((reporter_title_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_state_null_flavor_check CHECK (((state_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_street_null_flavor_check CHECK (((street_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT primary_sources_telephone_null_flavor_check CHECK (((telephone_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))
);

ALTER TABLE ONLY public.primary_sources FORCE ROW LEVEL SECURITY;


--
-- Name: principal_authorization_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.principal_authorization_state (
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT principal_authorization_state_revision_check CHECK ((revision > 0))
);


--
-- Name: product_presave_active_substances; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.product_presave_active_substances (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    product_presave_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    substance_name character varying(2000),
    substance_termid_version character varying(50),
    substance_termid character varying(100),
    mfds_version character varying(50),
    mfds_id character varying(100),
    strength_value numeric(15,5),
    strength_unit character varying(50),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_66f276dea9b178ab450783de5eb07be2 CHECK (((mfds_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6bb7a56ead4713814decba6c54ed02d4 CHECK (((substance_termid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_73147fa057f5570f178754da4a81de61 CHECK (((mfds_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a0f608f6a176e4ee49444443dc04625e CHECK (((strength_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a8c6311c3ff888b9512c3116936f362d CHECK (((substance_termid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f9262803afc22a311e3f0ab40fe8751e CHECK (((substance_name)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.product_presave_active_substances FORCE ROW LEVEL SECURITY;


--
-- Name: product_presaves; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.product_presaves (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    sender_presave_id uuid,
    receiver_presave_id uuid,
    product_id character varying(255),
    medicinal_product character varying(2000),
    medicinal_product_notation character varying(50),
    preapproval_ip_name character varying(2000),
    brand_name character varying(2000),
    original_manufacturer character varying(500),
    product_description text,
    mpid character varying(1000),
    mpid_version character varying(50),
    mfds_mpid character varying(255),
    mfds_mpid_version character varying(50),
    phpid character varying(255),
    phpid_version character varying(50),
    investigational_product_blinded boolean,
    obtain_drug_country character varying(2),
    drug_authorization_number character varying(100),
    drug_authorization_country character varying(2),
    drug_authorization_holder character varying(500),
    holder_applicant_name_notation character varying(50),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_079d0f2a4db10c52c4a73c57cfc32d30 CHECK (((obtain_drug_country)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_096d98101547641dfb15320f002909dc CHECK (((preapproval_ip_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_0aa35796a63223585e4f1e06e0147c5e CHECK (((medicinal_product)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_1ad1367c58755bf1ff227a275f0ea48b CHECK (((mpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3552cb1a1d013865695ad318a71668c0 CHECK (((holder_applicant_name_notation)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4c1ae50f2bab9b2b4b2e4c66c4a4c7a4 CHECK (((drug_authorization_country)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_58ef5def6e15ac95d63aed6db7bf308f CHECK (((original_manufacturer)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_600066574047e72c835a1b85b26f27c0 CHECK (((medicinal_product_notation)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_74a70fc68a9cb130f2fd3a7319768e6d CHECK (((drug_authorization_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7e13797687b62ff1aa1d07d53c26318e CHECK (((drug_authorization_holder)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9bea82def865d4d0e499eb252805f127 CHECK (((product_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9dd8bd2fa63eff9fe09b50cffa85a401 CHECK (((mfds_mpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b787662b9e836a37aaf7452b6ec744d8 CHECK (((phpid)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cc5f0b3b46164046d4cc74eaab4815b8 CHECK (((phpid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d07133ff686a3a236e33f53256715517 CHECK (((mfds_mpid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d5f525d1bbab1b1328cedd3ec25cc99d CHECK (((brand_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e03096a310c3101b8a39623aa9c2adaf CHECK (((mpid_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f466356d36a557f280b1927a0f31e810 CHECK ((product_description !~ '^[[:space:]]*$'::text)),
    CONSTRAINT product_presaves_investigational_product_blinded_input_contract CHECK (((investigational_product_blinded IS NULL) OR (investigational_product_blinded = true)))
);

ALTER TABLE ONLY public.product_presaves FORCE ROW LEVEL SECURITY;


--
-- Name: rbac_policy_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.rbac_policy_state (
    singleton boolean DEFAULT true NOT NULL,
    version bigint DEFAULT 1 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT rbac_policy_state_singleton_check CHECK (singleton),
    CONSTRAINT rbac_policy_state_version_check CHECK ((version > 0))
);


--
-- Name: reactions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.reactions (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    primary_source_reaction character varying(250),
    primary_source_reaction_translation character varying(250),
    reaction_language character varying(3),
    reaction_meddra_version character varying(10),
    reaction_meddra_code character varying(20),
    term_highlighted character varying(1),
    serious boolean,
    criteria_death boolean,
    criteria_death_null_flavor character varying(4),
    criteria_life_threatening boolean,
    criteria_life_threatening_null_flavor character varying(4),
    criteria_hospitalization boolean,
    criteria_hospitalization_null_flavor character varying(4),
    criteria_disabling boolean,
    criteria_disabling_null_flavor character varying(4),
    criteria_congenital_anomaly boolean,
    criteria_congenital_anomaly_null_flavor character varying(4),
    criteria_other_medically_important boolean,
    criteria_other_medically_important_null_flavor character varying(4),
    required_intervention boolean,
    required_intervention_null_flavor character varying(4),
    mfds_device_ae_classification character varying(1),
    mfds_device_ae_outcome character varying(2),
    mfds_device_cause_medical_device boolean,
    mfds_device_cause_procedure_issue boolean,
    mfds_device_cause_patient_condition boolean,
    mfds_device_cause_unable_to_assess boolean,
    mfds_device_cause_other character varying(20000),
    mfds_device_action_reason character varying(20000),
    mfds_device_action_recall boolean,
    mfds_device_action_repair boolean,
    mfds_device_action_inspection boolean,
    mfds_device_action_replacement boolean,
    mfds_device_action_improvement boolean,
    mfds_device_action_monitoring boolean,
    mfds_device_action_notification boolean,
    mfds_device_action_label_change boolean,
    mfds_device_action_other character varying(20000),
    start_date text,
    end_date text,
    duration_value text,
    duration_unit character varying(50),
    outcome character varying(1),
    medical_confirmation boolean,
    country_code character varying(2),
    start_date_null_flavor character varying(4),
    end_date_null_flavor character varying(4),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    source_xml_id text,
    CONSTRAINT ck_nf_reaction_congenital CHECK (((criteria_congenital_anomaly_null_flavor IS NULL) OR ((criteria_congenital_anomaly_null_flavor)::text = 'NI'::text))),
    CONSTRAINT ck_nf_reaction_death CHECK (((criteria_death_null_flavor IS NULL) OR ((criteria_death_null_flavor)::text = 'NI'::text))),
    CONSTRAINT ck_nf_reaction_disabling CHECK (((criteria_disabling_null_flavor IS NULL) OR ((criteria_disabling_null_flavor)::text = 'NI'::text))),
    CONSTRAINT ck_nf_reaction_end CHECK (((end_date_null_flavor IS NULL) OR ((end_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nf_reaction_hospital CHECK (((criteria_hospitalization_null_flavor IS NULL) OR ((criteria_hospitalization_null_flavor)::text = 'NI'::text))),
    CONSTRAINT ck_nf_reaction_intervention CHECK (((required_intervention_null_flavor IS NULL) OR ((required_intervention_null_flavor)::text = 'NI'::text))),
    CONSTRAINT ck_nf_reaction_life CHECK (((criteria_life_threatening_null_flavor IS NULL) OR ((criteria_life_threatening_null_flavor)::text = 'NI'::text))),
    CONSTRAINT ck_nf_reaction_other CHECK (((criteria_other_medically_important_null_flavor IS NULL) OR ((criteria_other_medically_important_null_flavor)::text = 'NI'::text))),
    CONSTRAINT ck_nf_reaction_start CHECK (((start_date_null_flavor IS NULL) OR ((start_date_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))),
    CONSTRAINT ck_nfv_reactions_20813a7a60e7 CHECK (((start_date IS NULL) OR (start_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reactions_651d605077b0 CHECK (((criteria_congenital_anomaly IS NULL) OR (criteria_congenital_anomaly_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reactions_660ccab3b2d2 CHECK (((end_date IS NULL) OR (end_date_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reactions_81a8c41938b1 CHECK (((criteria_hospitalization IS NULL) OR (criteria_hospitalization_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reactions_8dcbb0ff8aba CHECK (((criteria_other_medically_important IS NULL) OR (criteria_other_medically_important_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reactions_9d67d6c3f73c CHECK (((criteria_life_threatening IS NULL) OR (criteria_life_threatening_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reactions_c34ac60a4979 CHECK (((criteria_death IS NULL) OR (criteria_death_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reactions_d0450ead119f CHECK (((required_intervention IS NULL) OR (required_intervention_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reactions_e9dd2d573e7f CHECK (((criteria_disabling IS NULL) OR (criteria_disabling_null_flavor IS NULL))),
    CONSTRAINT ck_reaction_criteria_congenital_true CHECK ((criteria_congenital_anomaly IS DISTINCT FROM false)),
    CONSTRAINT ck_reaction_criteria_death_true CHECK ((criteria_death IS DISTINCT FROM false)),
    CONSTRAINT ck_reaction_criteria_disabling_true CHECK ((criteria_disabling IS DISTINCT FROM false)),
    CONSTRAINT ck_reaction_criteria_hospital_true CHECK ((criteria_hospitalization IS DISTINCT FROM false)),
    CONSTRAINT ck_reaction_criteria_life_true CHECK ((criteria_life_threatening IS DISTINCT FROM false)),
    CONSTRAINT ck_reaction_criteria_other_true CHECK ((criteria_other_medically_important IS DISTINCT FROM false)),
    CONSTRAINT model_nonblank_1ee28316387a9a7e6fda3ab0030def49 CHECK (((mfds_device_ae_outcome)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3032ad6aed6c5c3cda992d241f4d28bf CHECK (((outcome)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3c92ae49fa8a2ba9656998c9ca122625 CHECK (((reaction_meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3d772e5a055aabdaa06d6ddf24709d67 CHECK ((source_xml_id !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_40351ebe8309f175d032c6f0467b64a8 CHECK (((term_highlighted)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_40e2cc52f1c69d0ba128882ec9ff599b CHECK (((criteria_hospitalization_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_45b895caaf56df45652863ac57a42017 CHECK (((mfds_device_cause_other)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_462a96a9bd30fe4eb05ba2cf6df4dc20 CHECK ((duration_value !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4feaef9d8555bcbcfb2862535ab11b24 CHECK (((duration_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_55eceb8dde5fdfa0e4bfa50de80513e2 CHECK (((country_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_65df06a6d62e91d3d5fc40219e5609dd CHECK (((criteria_other_medically_important_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7ff341d0286ef6218a61fc99b194cd23 CHECK (((criteria_congenital_anomaly_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9023032257aec964674f09bb8fe28c7c CHECK (((criteria_death_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_90e8da89bb462061b421c1cb7191de14 CHECK ((end_date !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9504d76a692b8983f1d9a41961e41bde CHECK (((primary_source_reaction_translation)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_951724fcd1707c477ce2b789d96c3a0d CHECK (((end_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9db9d5b59453cf9650b807b2708d1c82 CHECK (((reaction_meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9fef926347d4113762e4250a1e7d68dd CHECK (((reaction_language)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a06f51a9a027cc8e6cec7f04dde57f23 CHECK (((start_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a2ce855cf4d29084d739502bcbae353d CHECK (((criteria_disabling_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b0ce727f94b42ec3c97cf6fc24cd5372 CHECK (((mfds_device_action_other)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c8300bb3d8ef86ce3d04d009a90c66d2 CHECK (((primary_source_reaction)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e2bcaf6b180a29e720bf1b3b3d65d239 CHECK (((required_intervention_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e5f84c7672d8374148be72093919af10 CHECK (((mfds_device_ae_classification)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_eadbb9116952e42cd59aab00b0e4e11a CHECK ((start_date !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f63fe995ab34519068dd7e1dadc8dbe4 CHECK (((mfds_device_action_reason)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_fb70f92ae4f32cf47faffbee394c1a41 CHECK (((criteria_life_threatening_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT reactions_mfds_device_ae_classification_check CHECK (((mfds_device_ae_classification IS NULL) OR ((mfds_device_ae_classification)::text = ANY (ARRAY[('0'::character varying)::text, ('1'::character varying)::text])))),
    CONSTRAINT reactions_mfds_device_ae_outcome_check CHECK (((mfds_device_ae_outcome IS NULL) OR ((mfds_device_ae_outcome)::text = ANY (ARRAY[('3'::character varying)::text, ('4'::character varying)::text, ('5'::character varying)::text, ('8'::character varying)::text, ('9'::character varying)::text, ('10'::character varying)::text, ('11'::character varying)::text, ('12'::character varying)::text])))),
    CONSTRAINT reactions_outcome_check CHECK (((outcome)::text = ANY (ARRAY[('0'::character varying)::text, ('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text, ('5'::character varying)::text]))),
    CONSTRAINT reactions_source_xml_id_check CHECK (((source_xml_id IS NULL) OR (length(btrim(source_xml_id)) > 0))),
    CONSTRAINT reactions_term_highlighted_code CHECK (((term_highlighted)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text])))
);

ALTER TABLE ONLY public.reactions FORCE ROW LEVEL SECURITY;


--
-- Name: receiver_presave_consignees; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.receiver_presave_consignees (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    receiver_presave_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    name character varying(500),
    phone character varying(50),
    email character varying(255),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_0c83f57c786a0b4a39efab23731c7ebc CHECK (((email)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b068931cc450442b63f5b3d276ea4297 CHECK (((name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f7a42fe7211f98ac7a60a285ac3a9e87 CHECK (((phone)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.receiver_presave_consignees FORCE ROW LEVEL SECURITY;


--
-- Name: receiver_presave_routes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.receiver_presave_routes (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    receiver_presave_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    authority character varying(16) NOT NULL,
    receiver_label character varying(255) NOT NULL,
    batch_receiver_identifier character varying(60),
    message_receiver_identifier character varying(60) NOT NULL,
    condition_page character varying(32) NOT NULL,
    condition_field_code character varying(64) NOT NULL,
    condition_operator character varying(32) NOT NULL,
    condition_value_code character varying(32) NOT NULL,
    condition_value_label character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_0558a732b8224706a1eba35b563e086d CHECK (((condition_operator)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_389240d921ee4da60f2cc19de57f80be CHECK (((batch_receiver_identifier)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_873e9c0b50183b613336eea1020f4369 CHECK (((authority)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_887a797b8f34d59ecac692fb48bd78ef CHECK (((condition_field_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_99f9a95b4f019dce1286167dc8821518 CHECK (((condition_value_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b2a3b9a0142597cab0f10f1dbe338482 CHECK (((receiver_label)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e55873f8e657dbbdf1f7f5eb56a49a19 CHECK (((condition_page)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f2d588b1d2424417784415843dfedf1c CHECK (((condition_value_label)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ff2206a533d787f14115efe3ddc73e85 CHECK (((message_receiver_identifier)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT receiver_presave_routes_authority_check CHECK (((authority)::text = ANY ((ARRAY['ich'::character varying, 'fda'::character varying, 'mfds'::character varying])::text[]))),
    CONSTRAINT receiver_presave_routes_condition_operator_check CHECK (((condition_operator)::text = 'Equal'::text))
);

ALTER TABLE ONLY public.receiver_presave_routes FORCE ROW LEVEL SECURITY;


--
-- Name: receiver_presaves; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.receiver_presaves (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    receiver_type character varying(50),
    organization_name character varying(500),
    day_count_rule character varying(100),
    nsae_solicited_day_count integer,
    nsae_solicited_not_applicable boolean,
    nsae_non_solicited_day_count integer,
    nsae_non_solicited_not_applicable boolean,
    sae_solicited_day_count integer,
    sae_solicited_not_applicable boolean,
    sae_non_solicited_day_count integer,
    sae_non_solicited_not_applicable boolean,
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_526f1e6db51d0d0d7ecd2daefb47d79a CHECK (((day_count_rule)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_67daf92c833c41c95db874e18fcb2786 CHECK ((description !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7725ae9cfe1604d5030f290fb61f83e2 CHECK (((receiver_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d71a8daf161a3b31f7eb6a043afcba3d CHECK (((organization_name)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.receiver_presaves FORCE ROW LEVEL SECURITY;


--
-- Name: relatedness_assessments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.relatedness_assessments (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    drug_reaction_assessment_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    source_of_assessment character varying(100),
    method_of_assessment character varying(100),
    method_of_assessment_kr1 character varying(10),
    result_of_assessment character varying(60),
    result_of_assessment_kr1 character varying(10),
    result_of_assessment_kr1_null_flavor character varying(10),
    result_of_assessment_kr2 character varying(2000),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_relatedness_kr1 CHECK (((result_of_assessment_kr1_null_flavor IS NULL) OR ((result_of_assessment_kr1_null_flavor)::text = 'NA'::text))),
    CONSTRAINT ck_nfv_relatedness_assessme_79e41fd7184e CHECK (((result_of_assessment_kr1 IS NULL) OR (result_of_assessment_kr1_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_638d7d644a04067cd9c9c7a1d3f538aa CHECK (((method_of_assessment)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7a2709164fc92d7375438ce067677c4c CHECK (((source_of_assessment)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c9d9e5d917de8335452618dca0d303f2 CHECK (((result_of_assessment)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ceb89fe3f3f129d7c5a83bcb8c7d6151 CHECK (((method_of_assessment_kr1)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e3cc4cd7506149a71583e78920e981c9 CHECK (((result_of_assessment_kr1_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ec3b52eda65aac6753bb84f9643e538e CHECK (((result_of_assessment_kr2)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f9cd9f0cdb9d8ab8725c0859cac5d6d1 CHECK (((result_of_assessment_kr1)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT relatedness_assessments_result_of_assessment_kr1_null_fla_check CHECK (((result_of_assessment_kr1_null_flavor)::text = 'NA'::text))
);

ALTER TABLE ONLY public.relatedness_assessments FORCE ROW LEVEL SECURITY;


--
-- Name: reported_causes_of_death; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.reported_causes_of_death (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    death_info_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    meddra_version character varying(10),
    meddra_code character varying(20),
    comments text,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_63c55f93a56ff18a74048153bb2aff09 CHECK (((meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_90883ccd002fbe4c7a849fa7acd9fe43 CHECK (((meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a5d491060952aa8ad5fdee071be752de CHECK ((comments !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.reported_causes_of_death FORCE ROW LEVEL SECURITY;


--
-- Name: reporter_presaves; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.reporter_presaves (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    reporter_title character varying(100),
    reporter_title_null_flavor character varying(4),
    reporter_given_name character varying(200),
    reporter_given_name_null_flavor character varying(4),
    reporter_middle_name character varying(200),
    reporter_middle_name_null_flavor character varying(4),
    reporter_family_name character varying(200),
    reporter_family_name_null_flavor character varying(4),
    organization character varying(500),
    organization_null_flavor character varying(4),
    department character varying(500),
    department_null_flavor character varying(4),
    street text,
    street_null_flavor character varying(4),
    city character varying(200),
    city_null_flavor character varying(4),
    state character varying(100),
    state_null_flavor character varying(4),
    postcode character varying(50),
    postcode_null_flavor character varying(4),
    telephone character varying(50),
    telephone_null_flavor character varying(4),
    reporter_email character varying(100),
    reporter_email_null_flavor character varying(4),
    country_code character varying(2),
    qualification character varying(50),
    qualification_kr1 character varying(50),
    primary_source_regulatory character varying(50),
    qualification_null_flavor character varying(4),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_presave_qualification CHECK (((qualification_null_flavor IS NULL) OR ((qualification_null_flavor)::text = 'UNK'::text))),
    CONSTRAINT ck_nfv_reporter_presaves_02161fa0cb84 CHECK (((reporter_title IS NULL) OR (reporter_title_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_05f23ba7477c CHECK (((city IS NULL) OR (city_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_155196ab5259 CHECK (((street IS NULL) OR (street_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_158c69d5d6d8 CHECK (((reporter_given_name IS NULL) OR (reporter_given_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_3b504b77df06 CHECK (((state IS NULL) OR (state_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_4a768b8275bf CHECK (((telephone IS NULL) OR (telephone_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_5700e404661d CHECK (((reporter_middle_name IS NULL) OR (reporter_middle_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_64d2093e70be CHECK (((reporter_family_name IS NULL) OR (reporter_family_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_6da21f4a9973 CHECK (((organization IS NULL) OR (organization_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_710be2ebc74a CHECK (((qualification IS NULL) OR (qualification_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_c73651747c03 CHECK (((department IS NULL) OR (department_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_ce95a252c88c CHECK (((postcode IS NULL) OR (postcode_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_reporter_presaves_ee98453c1250 CHECK (((reporter_email IS NULL) OR (reporter_email_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_08b38884731aa214d50dfa032fe453f2 CHECK (((reporter_email_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_0af5269bbdeba6dc6eaa3068706dcd8e CHECK (((reporter_given_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_0e6a7e64db5e1a442bbb5e79ae81bc8c CHECK (((department_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_165cecd3a4bbb5ea094a8fffefcd76bb CHECK (((reporter_given_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_1f4a9685e37e4f940d07a9f6b43dc83c CHECK ((street !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_3e92ae4858cfd4c52479c1b71cfdc196 CHECK (((postcode_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_459d9fca17e3a950deae755d13578292 CHECK (((department)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4ed5d2eaed1a1fadcc41ad1d58ed603e CHECK (((city)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_55eceb8dde5fdfa0e4bfa50de80513e2 CHECK (((country_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_5b90677dbcf90d6dfa1901f30536ae90 CHECK (((reporter_title_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_70a9a8f0c42ff96f14050d7db649c9d5 CHECK (((street_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_72e2eeeaf44323dd42b7121353dd0d73 CHECK (((qualification_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_85cf433bccd9960b447cee48f4b65429 CHECK (((state_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8b27d9ca661471106e24afe9e7b0cad1 CHECK (((qualification_kr1)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8ec2d8c778502907beaa134f8afbd239 CHECK (((city_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_914fb22f9e2005f408959c53e45ad20e CHECK (((reporter_title)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_934bc20046509870e266117294898038 CHECK (((qualification)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9ed39e2ea931586b6a985a6942ef573e CHECK (((state)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b4c1948c087fafc89a88450fcbb64c77 CHECK (((organization)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b734ba89bd8db0bbacfc4f685093578a CHECK (((organization_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b851b871af414a81f60c5804f559e43f CHECK (((telephone_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b9bb7e7b00a4ba1e0d15fa8b2485d8c4 CHECK (((telephone)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ba597e85fac2e5456c91262cedba01fc CHECK (((reporter_middle_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_bc11f60614574277a5706496f5c402d0 CHECK (((reporter_family_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cfc7f4e630835101cdefa94311535cf3 CHECK (((reporter_family_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d0361b39351dee98219254e7d3a05aab CHECK (((reporter_middle_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e90ebd9556fa4031171f043013794b61 CHECK (((postcode)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ef67ed101cd37565a1f93ffbdef533b1 CHECK (((primary_source_regulatory)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_fa0180c7373934005740b99bdcabc07a CHECK (((reporter_email)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT reporter_presaves_city_null_flavor_check CHECK (((city_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_department_null_flavor_check CHECK (((department_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_organization_null_flavor_check CHECK (((organization_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_postcode_null_flavor_check CHECK (((postcode_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_primary_source_regulatory_input_contract CHECK (((primary_source_regulatory IS NULL) OR ((primary_source_regulatory)::text = '1'::text))),
    CONSTRAINT reporter_presaves_reporter_email_null_flavor_check CHECK (((reporter_email_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_reporter_family_name_null_flavor_check CHECK (((reporter_family_name_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_reporter_given_name_null_flavor_check CHECK (((reporter_given_name_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_reporter_middle_name_null_flavor_check CHECK (((reporter_middle_name_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_reporter_title_null_flavor_check CHECK (((reporter_title_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('UNK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_state_null_flavor_check CHECK (((state_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_street_null_flavor_check CHECK (((street_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT reporter_presaves_telephone_null_flavor_check CHECK (((telephone_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text])))
);

ALTER TABLE ONLY public.reporter_presaves FORCE ROW LEVEL SECURITY;


--
-- Name: role_grants; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.role_grants (
    role_id uuid NOT NULL,
    grant_id text NOT NULL,
    CONSTRAINT model_nonblank_c2418e0763955a29412e22b412c66854 CHECK ((grant_id !~ '^[[:space:]]*$'::text))
);


--
-- Name: safety_report_identification; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.safety_report_identification (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    safety_report_id character varying(100),
    version integer DEFAULT 1 NOT NULL,
    transmission_date text,
    report_type character varying(1),
    date_first_received_from_source text,
    date_of_most_recent_information text,
    fulfil_expedited_criteria boolean,
    fulfil_expedited_criteria_null_flavor character varying(4),
    local_criteria_report_type character varying(10),
    combination_product_report_indicator character varying(10),
    combination_product_report_indicator_null_flavor character varying(4),
    worldwide_unique_id character varying(100),
    first_sender_type character varying(1),
    additional_documents_available boolean,
    other_case_identifiers_exist boolean,
    other_case_identifiers_exist_null_flavor character varying(4),
    nullification_code character varying(10),
    nullification_reason text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_sri_expedited CHECK (((fulfil_expedited_criteria_null_flavor IS NULL) OR ((fulfil_expedited_criteria_null_flavor)::text = 'NI'::text))),
    CONSTRAINT ck_nf_sri_other_ids CHECK (((other_case_identifiers_exist_null_flavor IS NULL) OR ((other_case_identifiers_exist_null_flavor)::text = 'NI'::text))),
    CONSTRAINT ck_nfv_safety_report_identi_3738caff642e CHECK (((other_case_identifiers_exist IS NULL) OR (other_case_identifiers_exist_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_safety_report_identi_d88ab00e17b6 CHECK (((combination_product_report_indicator IS NULL) OR (combination_product_report_indicator_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_safety_report_identi_fe24b9d8d83e CHECK (((fulfil_expedited_criteria IS NULL) OR (fulfil_expedited_criteria_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_1472a0826ae471dc5745317d92391ccb CHECK (((nullification_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_1b3244c17f51ab32b751246277aef66c CHECK (((combination_product_report_indicator_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_554314bf947d4d2ba0df18d46bc8a1c1 CHECK (((report_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6ac9b927ef1658ea91052810fea0d8d5 CHECK (((combination_product_report_indicator)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_70af50f8961d495e3b7a7985cf77f2d5 CHECK (((first_sender_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_83c8c435b1129c59c20d049cda4931ae CHECK ((transmission_date !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_943e5f7598dccffe5b44f2e2238806ef CHECK ((nullification_reason !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9bc0708700017c924bcf302602b360e6 CHECK (((other_case_identifiers_exist_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9ed0cb33ef2c9faa7085d1f3ec4b8f52 CHECK ((date_first_received_from_source !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a321fa2f441930d0ab3824cdc93749dd CHECK (((fulfil_expedited_criteria_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_aa303ac3e22a2ffca4ab39f3762911f8 CHECK ((date_of_most_recent_information !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_aca3edb9fa1e326bb052b06e96724ae0 CHECK (((worldwide_unique_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b7070a59007c058ca23281f9b7544be2 CHECK (((safety_report_id)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f78d333d1d282103443a5fae973a226a CHECK (((local_criteria_report_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT safety_report_identification_combination_product_report_i_check CHECK (((combination_product_report_indicator_null_flavor)::text = 'NI'::text)),
    CONSTRAINT safety_report_identification_first_sender_type_check CHECK (((first_sender_type)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text]))),
    CONSTRAINT safety_report_identification_fulfil_expedited_criteria_nu_check CHECK (((fulfil_expedited_criteria_null_flavor)::text = 'NI'::text)),
    CONSTRAINT safety_report_identification_other_case_identifiers_exist_check CHECK (((other_case_identifiers_exist_null_flavor)::text = 'NI'::text)),
    CONSTRAINT safety_report_identification_report_type_check CHECK (((report_type IS NULL) OR ((report_type)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text]))))
);

ALTER TABLE ONLY public.safety_report_identification FORCE ROW LEVEL SECURITY;


--
-- Name: sender_diagnoses; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sender_diagnoses (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    narrative_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    diagnosis_meddra_version character varying(10),
    diagnosis_meddra_code character varying(20),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_729e9aafc5ec5b44ae665b86ddd7a247 CHECK (((diagnosis_meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_893ddaa7201e72d67d493419afc1704d CHECK (((diagnosis_meddra_version)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.sender_diagnoses FORCE ROW LEVEL SECURITY;


--
-- Name: sender_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sender_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    source_sender_presave_id uuid,
    sender_type character varying(1),
    health_professional_type_kr1 character varying(20),
    organization_name character varying(200),
    department character varying(100),
    street_address character varying(100),
    city character varying(50),
    state character varying(40),
    postcode character varying(15),
    country_code character varying(2),
    person_title character varying(50),
    person_title_null_flavor character varying(4),
    person_given_name character varying(60),
    person_given_name_null_flavor character varying(4),
    person_middle_name character varying(60),
    person_middle_name_null_flavor character varying(4),
    person_family_name character varying(60),
    person_family_name_null_flavor character varying(4),
    telephone character varying(33),
    fax character varying(33),
    email character varying(100),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_sender_information_06f2ea6c0cb2 CHECK (((person_middle_name IS NULL) OR (person_middle_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_sender_information_1b7df42332a3 CHECK (((person_family_name IS NULL) OR (person_family_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_sender_information_8f56d21db35e CHECK (((person_title IS NULL) OR (person_title_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_sender_information_a5fba252c3e8 CHECK (((person_given_name IS NULL) OR (person_given_name_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_0c83f57c786a0b4a39efab23731c7ebc CHECK (((email)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_1cc849fa6c5df93438f616a390f40497 CHECK (((person_middle_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_236c3b7f761221f195b428aca2f06c4b CHECK (((fax)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_287d00bbcfcd428cfe5757a726bf43d9 CHECK (((person_family_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_459d9fca17e3a950deae755d13578292 CHECK (((department)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4ed5d2eaed1a1fadcc41ad1d58ed603e CHECK (((city)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_55eceb8dde5fdfa0e4bfa50de80513e2 CHECK (((country_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_620bb21d1a300eb1b3d55a39e6885591 CHECK (((person_title_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8b07564a42b5fda223f52ffae44b1c1d CHECK (((street_address)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8dd3b76db7785ede468709db003966b5 CHECK (((person_title)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9ed39e2ea931586b6a985a6942ef573e CHECK (((state)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b1f0e235cdd914c1c3ea1cbb0973c4ec CHECK (((sender_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b9bb7e7b00a4ba1e0d15fa8b2485d8c4 CHECK (((telephone)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d5a2060f35f16751f4c0070d3f1ef4ad CHECK (((health_professional_type_kr1)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d71a8daf161a3b31f7eb6a043afcba3d CHECK (((organization_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e4ece79fa7c6b1caea559575f095b90e CHECK (((person_given_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e90ebd9556fa4031171f043013794b61 CHECK (((postcode)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f94c6e1030605fb8c71500ffa3492a62 CHECK (((person_given_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_fb189d5d762e6ffd1cfc517cd412dbf0 CHECK (((person_middle_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_fea0a032fcbc5e723514292f4dd7a8ab CHECK (((person_family_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT sender_information_person_family_name_null_flavor_check CHECK (((person_family_name_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT sender_information_person_given_name_null_flavor_check CHECK (((person_given_name_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT sender_information_person_middle_name_null_flavor_check CHECK (((person_middle_name_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT sender_information_person_title_null_flavor_check CHECK (((person_title_null_flavor)::text = ANY (ARRAY[('MSK'::character varying)::text, ('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT sender_information_sender_type_check CHECK (((sender_type)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text, ('5'::character varying)::text, ('6'::character varying)::text, ('7'::character varying)::text])))
);

ALTER TABLE ONLY public.sender_information FORCE ROW LEVEL SECURITY;


--
-- Name: sender_presave_gateways; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sender_presave_gateways (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    sender_presave_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    gateway_authority character varying(16) NOT NULL,
    sender_identifier character varying(255),
    routing_identifier character varying(255),
    cde_sender_identifier character varying(255),
    cdr_sender_identifier character varying(255),
    is_default_for_authority boolean DEFAULT false NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_014ead2aaedaf12125f037ab8cea1eec CHECK (((gateway_authority)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_73826fa0fc7c9a56bbf06682b373d3fb CHECK (((sender_identifier)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9ca5888b95cfb0b1eb640572f53d1898 CHECK (((cdr_sender_identifier)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_bd078db5cbde18010391a087a9dff256 CHECK (((routing_identifier)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c03c1b9eb58a649ade7c8a44e6cb813e CHECK (((cde_sender_identifier)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT sender_presave_gateways_authority_valid CHECK (((gateway_authority)::text = ANY (ARRAY[('ich'::character varying)::text, ('fda'::character varying)::text, ('mfds'::character varying)::text])))
);

ALTER TABLE ONLY public.sender_presave_gateways FORCE ROW LEVEL SECURITY;


--
-- Name: sender_presave_responsible_persons; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sender_presave_responsible_persons (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    sender_presave_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    department character varying(500),
    person_title character varying(100),
    person_given_name character varying(200),
    person_middle_name character varying(200),
    person_family_name character varying(200),
    is_default boolean DEFAULT false NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_1cc849fa6c5df93438f616a390f40497 CHECK (((person_middle_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_459d9fca17e3a950deae755d13578292 CHECK (((department)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8dd3b76db7785ede468709db003966b5 CHECK (((person_title)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e4ece79fa7c6b1caea559575f095b90e CHECK (((person_given_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_fea0a032fcbc5e723514292f4dd7a8ab CHECK (((person_family_name)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.sender_presave_responsible_persons FORCE ROW LEVEL SECURITY;


--
-- Name: sender_presaves; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sender_presaves (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    is_default boolean DEFAULT false NOT NULL,
    sender_type character varying(50),
    organization_name character varying(500),
    organization_name_notation character varying(50),
    street_address text,
    city character varying(200),
    state character varying(100),
    postcode character varying(50),
    country_code character varying(2),
    telephone character varying(50),
    fax character varying(50),
    email character varying(255),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_0c83f57c786a0b4a39efab23731c7ebc CHECK (((email)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_236c3b7f761221f195b428aca2f06c4b CHECK (((fax)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4ed5d2eaed1a1fadcc41ad1d58ed603e CHECK (((city)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_55eceb8dde5fdfa0e4bfa50de80513e2 CHECK (((country_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8b07564a42b5fda223f52ffae44b1c1d CHECK ((street_address !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9ed39e2ea931586b6a985a6942ef573e CHECK (((state)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b1f0e235cdd914c1c3ea1cbb0973c4ec CHECK (((sender_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b9bb7e7b00a4ba1e0d15fa8b2485d8c4 CHECK (((telephone)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d40317138f094958eb3ceafdafeac060 CHECK (((organization_name_notation)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d71a8daf161a3b31f7eb6a043afcba3d CHECK (((organization_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e90ebd9556fa4031171f043013794b61 CHECK (((postcode)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.sender_presaves FORCE ROW LEVEL SECURITY;


--
-- Name: source_documents; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.source_documents (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    source_document_name text,
    source_document_base64 text,
    source_document_media_type text,
    sequence_number integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_2390ec72e21a99be509658f16273d6fb CHECK ((source_document_name !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a4f97a2ed802afbe9d5d399a6bdc08a1 CHECK ((source_document_base64 !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cf6665f6e86c78fb766564dbd1b7f4a1 CHECK ((source_document_media_type !~ '^[[:space:]]*$'::text))
);


--
-- Name: study_fda_cross_reported_inds; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.study_fda_cross_reported_inds (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    study_information_id uuid NOT NULL,
    ind_number character varying(10),
    ind_number_null_flavor character varying(4),
    sequence_number integer NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_study_fda_ind CHECK (((ind_number_null_flavor IS NULL) OR ((ind_number_null_flavor)::text = 'NA'::text))),
    CONSTRAINT ck_nfv_study_fda_cross_repo_917d91b15f8d CHECK (((ind_number IS NULL) OR (ind_number_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_166e925a0264b2df5a935424f2ef6ef6 CHECK (((ind_number_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ca6437f98042d558d4caa8e9dcdb63da CHECK (((ind_number)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.study_fda_cross_reported_inds FORCE ROW LEVEL SECURITY;


--
-- Name: study_information; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.study_information (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    source_study_presave_id uuid,
    study_name character varying(2000),
    study_name_null_flavor character varying(4),
    sponsor_study_number character varying(50),
    sponsor_study_number_null_flavor character varying(4),
    study_type_reaction character varying(2),
    study_type_reaction_kr1 character varying(1),
    fda_ind_number_occurred character varying(10),
    fda_pre_anda_number_occurred character varying(10),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_study_information_5dbf5f592e06 CHECK (((sponsor_study_number IS NULL) OR (sponsor_study_number_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_study_information_5e89ff8bbf9e CHECK (((study_name IS NULL) OR (study_name_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_0f814e90ebfdbb92f536a2688103fe98 CHECK (((sponsor_study_number_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_2cc87531cb90dcd6d8bdd9f60b85dcc3 CHECK (((study_type_reaction)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_49b02ef9cb95ccc6209fe213ba21d050 CHECK (((study_type_reaction_kr1)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b4088077cb3e2245cf8d93d29da5d2a5 CHECK (((study_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c6495860d9a25b9186a2b0e88d915cc7 CHECK (((sponsor_study_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cdf66d1885c242a9832d664b6874cbbe CHECK (((study_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f6baa0a62db233674f801c5d866d51ca CHECK (((fda_ind_number_occurred)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f8fdadb95f22a0970367dd3e689e299e CHECK (((fda_pre_anda_number_occurred)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT study_information_sponsor_study_number_null_flavor_check CHECK (((sponsor_study_number_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT study_information_study_name_null_flavor_check CHECK (((study_name_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT study_information_study_type_reaction_kr1_check CHECK (((study_type_reaction_kr1)::text = ANY (ARRAY[('1'::character varying)::text, ('2'::character varying)::text, ('3'::character varying)::text, ('4'::character varying)::text])))
);

ALTER TABLE ONLY public.study_information FORCE ROW LEVEL SECURITY;


--
-- Name: study_presave_fda_cross_reported_ind_numbers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.study_presave_fda_cross_reported_ind_numbers (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    study_presave_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    ind_number character varying(10),
    ind_number_null_flavor character varying(4),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_study_presave_fda_cr_657dfdf64b60 CHECK (((ind_number IS NULL) OR (ind_number_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_166e925a0264b2df5a935424f2ef6ef6 CHECK (((ind_number_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ca6437f98042d558d4caa8e9dcdb63da CHECK (((ind_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT study_presave_fda_cross_reported_i_ind_number_null_flavor_check CHECK (((ind_number_null_flavor)::text = 'NA'::text))
);

ALTER TABLE ONLY public.study_presave_fda_cross_reported_ind_numbers FORCE ROW LEVEL SECURITY;


--
-- Name: study_presave_products; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.study_presave_products (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    study_presave_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    product_presave_id uuid,
    product_name character varying(255),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_f5625f45195c610c4ab60fdc118a2cdb CHECK (((product_name)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.study_presave_products FORCE ROW LEVEL SECURITY;


--
-- Name: study_presave_registration_numbers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.study_presave_registration_numbers (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    study_presave_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    registration_number character varying(50),
    registration_number_null_flavor character varying(4),
    country_code character varying(2),
    country_code_null_flavor character varying(4),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_study_presave_regist_57b59fc1a2ed CHECK (((country_code IS NULL) OR (country_code_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_study_presave_regist_7122cffffe8a CHECK (((registration_number IS NULL) OR (registration_number_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_07aa4e6714b37e6dee06da2bf16b35ac CHECK (((registration_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_55eceb8dde5fdfa0e4bfa50de80513e2 CHECK (((country_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b081c226d8816c89320ec0c071b4e7e2 CHECK (((country_code_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d37211f172a399ee958fa6761ffa4728 CHECK (((registration_number_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT study_presave_registration_n_registration_number_null_fla_check CHECK (((registration_number_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT study_presave_registration_numbe_country_code_null_flavor_check CHECK (((country_code_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text])))
);

ALTER TABLE ONLY public.study_presave_registration_numbers FORCE ROW LEVEL SECURITY;


--
-- Name: study_presave_reporters; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.study_presave_reporters (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    study_presave_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    reporter_presave_id uuid,
    reporter_organization character varying(255),
    reporter_given_name character varying(100),
    reporter_qualification character varying(50),
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT model_nonblank_0af5269bbdeba6dc6eaa3068706dcd8e CHECK (((reporter_given_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b91ea0d434bf08899a16723b5b6459cd CHECK (((reporter_organization)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_fe413f623a06d7df7cbb124904e512a8 CHECK (((reporter_qualification)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.study_presave_reporters FORCE ROW LEVEL SECURITY;


--
-- Name: study_presaves; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.study_presaves (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    product_presave_id uuid,
    study_name character varying(2000),
    study_name_null_flavor character varying(4),
    study_name_notation text,
    sponsor_study_number character varying(50),
    sponsor_study_number_null_flavor character varying(4),
    sponsor_study_number_kind character varying(50),
    study_type_reaction character varying(50),
    study_type_reaction_kr1 character varying(50),
    fda_ind_number_occurred character varying(10),
    fda_pre_anda_number_occurred character varying(10),
    edc_sync boolean,
    exclude_case_key_from_sync boolean,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_study_presaves_5d1c1cd27743 CHECK (((study_name IS NULL) OR (study_name_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_study_presaves_c02aa44b2c38 CHECK (((sponsor_study_number IS NULL) OR (sponsor_study_number_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_0f814e90ebfdbb92f536a2688103fe98 CHECK (((sponsor_study_number_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_2cc87531cb90dcd6d8bdd9f60b85dcc3 CHECK (((study_type_reaction)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_49b02ef9cb95ccc6209fe213ba21d050 CHECK (((study_type_reaction_kr1)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9138b9bbf0bd4192f6536f947322e926 CHECK ((study_name_notation !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_98d8f3e69f0cdf8d5cda9f9adee38e62 CHECK (((sponsor_study_number_kind)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b4088077cb3e2245cf8d93d29da5d2a5 CHECK (((study_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c6495860d9a25b9186a2b0e88d915cc7 CHECK (((sponsor_study_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_cdf66d1885c242a9832d664b6874cbbe CHECK (((study_name_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f6baa0a62db233674f801c5d866d51ca CHECK (((fda_ind_number_occurred)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f8fdadb95f22a0970367dd3e689e299e CHECK (((fda_pre_anda_number_occurred)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT study_presaves_sponsor_study_number_kind_valid CHECK (((sponsor_study_number_kind IS NULL) OR ((sponsor_study_number_kind)::text = ANY (ARRAY[('STUDY_NO'::character varying)::text, ('PROTOCOL_NO'::character varying)::text])))),
    CONSTRAINT study_presaves_sponsor_study_number_null_flavor_check CHECK (((sponsor_study_number_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT study_presaves_study_name_null_flavor_check CHECK (((study_name_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text])))
);

ALTER TABLE ONLY public.study_presaves FORCE ROW LEVEL SECURITY;


--
-- Name: study_registration_numbers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.study_registration_numbers (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    study_information_id uuid NOT NULL,
    registration_number character varying(50),
    registration_number_null_flavor character varying(4),
    country_code character varying(2),
    country_code_null_flavor character varying(4),
    sequence_number integer NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nfv_study_registration_n_57aec989d7ca CHECK (((country_code IS NULL) OR (country_code_null_flavor IS NULL))),
    CONSTRAINT ck_nfv_study_registration_n_86b44fc11f3b CHECK (((registration_number IS NULL) OR (registration_number_null_flavor IS NULL))),
    CONSTRAINT model_nonblank_07aa4e6714b37e6dee06da2bf16b35ac CHECK (((registration_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_55eceb8dde5fdfa0e4bfa50de80513e2 CHECK (((country_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_b081c226d8816c89320ec0c071b4e7e2 CHECK (((country_code_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d37211f172a399ee958fa6761ffa4728 CHECK (((registration_number_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT study_registration_numbers_country_code_null_flavor_check CHECK (((country_code_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text]))),
    CONSTRAINT study_registration_numbers_registration_number_null_flavo_check CHECK (((registration_number_null_flavor)::text = ANY (ARRAY[('ASKU'::character varying)::text, ('NASK'::character varying)::text])))
);

ALTER TABLE ONLY public.study_registration_numbers FORCE ROW LEVEL SECURITY;


--
-- Name: submission_acks; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.submission_acks (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    submission_id uuid NOT NULL,
    ack_level smallint NOT NULL,
    success boolean NOT NULL,
    ack_code character varying(120),
    ack_message text,
    received_at timestamp with time zone DEFAULT now() NOT NULL,
    raw_payload jsonb,
    CONSTRAINT model_nonblank_063c7136e5838778de99b3550fa90926 CHECK ((ack_message !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_f472320527e7974066b9edc418eb6a69 CHECK (((ack_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT submission_acks_ack_level_check CHECK (((ack_level >= 1) AND (ack_level <= 4)))
);

ALTER TABLE ONLY public.submission_acks FORCE ROW LEVEL SECURITY;


--
-- Name: submission_dispatch_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.submission_dispatch_state (
    submission_id uuid NOT NULL,
    attempt_count integer DEFAULT 0 NOT NULL,
    last_attempt_at timestamp with time zone,
    last_error text,
    next_retry_at timestamp with time zone,
    terminal_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_30086cacebd0f691a09262daa4032b24 CHECK ((last_error !~ '^[[:space:]]*$'::text)),
    CONSTRAINT submission_dispatch_state_attempt_count_check CHECK ((attempt_count >= 0))
);

ALTER TABLE ONLY public.submission_dispatch_state FORCE ROW LEVEL SECURITY;


--
-- Name: submission_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.submission_events (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    submission_id uuid NOT NULL,
    event_type character varying(80) NOT NULL,
    event_data jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_1cd0361405ee31ff6a59ae778f9744fe CHECK (((event_type)::text !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.submission_events FORCE ROW LEVEL SECURITY;


--
-- Name: submission_idempotency; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.submission_idempotency (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    authority character varying(16) NOT NULL,
    idempotency_key character varying(128) NOT NULL,
    submission_id uuid NOT NULL,
    created_by uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_4824938f143bb3ade0310ef49ccc23a1 CHECK (((idempotency_key)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_873e9c0b50183b613336eea1020f4369 CHECK (((authority)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT submission_idempotency_authority_valid CHECK (((authority)::text = ANY (ARRAY[('fda'::character varying)::text, ('mfds'::character varying)::text])))
);

ALTER TABLE ONLY public.submission_idempotency FORCE ROW LEVEL SECURITY;


--
-- Name: terminology_releases; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.terminology_releases (
    id bigint NOT NULL,
    dictionary character varying(20) NOT NULL,
    version character varying(40) NOT NULL,
    language character varying(10) DEFAULT 'en'::character varying NOT NULL,
    status character varying(20) DEFAULT 'loading'::character varying NOT NULL,
    source_path text,
    source_checksum character varying(128),
    loaded_rows bigint DEFAULT 0 NOT NULL,
    approved_by uuid,
    approved_at timestamp with time zone,
    activated_by uuid,
    activated_at timestamp with time zone,
    rollback_from_version character varying(40),
    note text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    audit_id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    CONSTRAINT model_nonblank_2af72f100c356273d46284f6fd1dfc08 CHECK (((version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_681a94b19c7c830740cc4f1bc7505d3b CHECK ((source_path !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_6fd0ceb5fc1ca5d6b3f492c86250e4cb CHECK (((rollback_from_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8512ae7d57b1396273f76fe6ed341a23 CHECK (((language)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9acb44549b41563697bb490144ec6258 CHECK (((status)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_aad653ca3ee669635f2938b73098b6d7 CHECK ((note !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_abc20d7bde1df257f890e152af2e3470 CHECK (((dictionary)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d4b68f1a18d1d6c15cc07172da9f16ea CHECK (((source_checksum)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT terminology_release_dictionary_chk CHECK (((dictionary)::text = ANY (ARRAY[('meddra'::character varying)::text, ('whodrug'::character varying)::text, ('iso3166'::character varying)::text, ('ich_constrained_ucum'::character varying)::text, ('edqm'::character varying)::text, ('mfds_product'::character varying)::text]))),
    CONSTRAINT terminology_release_status_chk CHECK (((status)::text = ANY (ARRAY[('loading'::character varying)::text, ('validated'::character varying)::text, ('approved'::character varying)::text, ('active'::character varying)::text, ('failed'::character varying)::text, ('retired'::character varying)::text])))
);

ALTER TABLE ONLY public.terminology_releases FORCE ROW LEVEL SECURITY;


--
-- Name: terminology_releases_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.terminology_releases_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: terminology_releases_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.terminology_releases_id_seq OWNED BY public.terminology_releases.id;


--
-- Name: test_results; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.test_results (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    sequence_number integer NOT NULL,
    test_date text,
    test_date_null_flavor character varying(10),
    test_name character varying(250),
    test_meddra_version character varying(10),
    test_meddra_code character varying(20),
    test_result_code character varying(2),
    test_result_value character varying(20000),
    test_result_qualifier character varying(2),
    test_result_unit character varying(50),
    result_unstructured text,
    normal_low_value character varying(50),
    normal_high_value character varying(50),
    comments text,
    more_info_available boolean,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid NOT NULL,
    updated_by uuid,
    CONSTRAINT ck_nf_test_date CHECK (((test_date_null_flavor IS NULL) OR ((test_date_null_flavor)::text = ANY (ARRAY[('UNK'::character varying)::text, ('MSK'::character varying)::text])))),
    CONSTRAINT ck_nfv_test_results_6c4ecee7289c CHECK (((test_date IS NULL) OR (test_date_null_flavor IS NULL))),
    CONSTRAINT ck_test_result_qualifier CHECK (((test_result_qualifier IS NULL) OR ((test_result_qualifier)::text = ANY (ARRAY[('LT'::character varying)::text, ('LE'::character varying)::text, ('GT'::character varying)::text, ('GE'::character varying)::text])))),
    CONSTRAINT ck_test_result_qualifier_value CHECK (((test_result_qualifier IS NULL) OR (test_result_value IS NOT NULL))),
    CONSTRAINT model_nonblank_2315ae30728231885480402f03a1e735 CHECK (((normal_low_value)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_493624d608834caf0a9343f36735235e CHECK (((test_result_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_4fce0bb20fdf6f5d56f900d7782a5d90 CHECK (((test_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_5c7f6e8d223a109412066ce756c40234 CHECK (((test_meddra_version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7287694f88cacfe540d4a911603b28d1 CHECK (((test_result_unit)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7b2407e3f28226537a3feb08e40665d1 CHECK (((test_result_qualifier)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7d95a76e270307055b33f4d6293b3668 CHECK (((test_result_value)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9e96b6fc987d3f8c74d9370202128dd5 CHECK (((test_date_null_flavor)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9fc0636fa8e51e899f825f3c34506000 CHECK (((normal_high_value)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a5d491060952aa8ad5fdee071be752de CHECK ((comments !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d53008a6ebc5024bf695ce793f7d8d5d CHECK ((test_date !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_e7556b2461fd195a1348d3abdf2b629f CHECK ((result_unstructured !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ebaeaecd6fe8af7245d2f7b9c408f91d CHECK (((test_meddra_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT test_results_test_result_qualifier_check CHECK (((test_result_qualifier)::text = ANY (ARRAY[('EQ'::character varying)::text, ('LT'::character varying)::text, ('LE'::character varying)::text, ('GT'::character varying)::text, ('GE'::character varying)::text])))
);

ALTER TABLE ONLY public.test_results FORCE ROW LEVEL SECURITY;


--
-- Name: ucum_units; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ucum_units (
    id integer NOT NULL,
    code character varying(20) NOT NULL,
    display_name character varying(100) NOT NULL,
    description text,
    unit_type character varying(50),
    active boolean DEFAULT true,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_37561c4b1970e3ee0a52d9ba874aeba0 CHECK (((unit_type)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_67daf92c833c41c95db874e18fcb2786 CHECK ((description !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c13367945d5d4c91047b3b50234aa7ab CHECK (((code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c2657f133e8615f7dba3ddfe13125bd4 CHECK (((display_name)::text !~ '^[[:space:]]*$'::text))
);


--
-- Name: ucum_units_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.ucum_units_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: ucum_units_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.ucum_units_id_seq OWNED BY public.ucum_units.id;


--
-- Name: user_organization_memberships; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_organization_memberships (
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid,
    updated_by uuid
);

ALTER TABLE ONLY public.user_organization_memberships FORCE ROW LEVEL SECURITY;


--
-- Name: user_role_assignments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_role_assignments (
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    role_id uuid NOT NULL,
    active boolean DEFAULT true NOT NULL,
    row_version bigint DEFAULT 1 NOT NULL,
    assigned_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT user_role_assignments_row_version_check CHECK ((row_version > 0))
);


--
-- Name: users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.users (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    email character varying(255) NOT NULL,
    username character varying(128) NOT NULL,
    pwd character varying(256),
    pwd_salt uuid DEFAULT public.gen_random_uuid() NOT NULL,
    token_salt uuid DEFAULT public.gen_random_uuid() NOT NULL,
    role character varying(50) NOT NULL,
    comments text,
    other_information text,
    access_start_at timestamp with time zone,
    access_end_at timestamp with time zone,
    access_sender_ids text,
    access_product_ids text,
    access_study_ids text,
    access_blind_allowed boolean DEFAULT false NOT NULL,
    active_sender_identifier text,
    active boolean DEFAULT true,
    must_change_password boolean DEFAULT false NOT NULL,
    last_login_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by uuid,
    updated_by uuid,
    CONSTRAINT email_format CHECK (((email)::text ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$'::text)),
    CONSTRAINT model_nonblank_0c83f57c786a0b4a39efab23731c7ebc CHECK (((email)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_14c4b06b824ec593239362517f538b29 CHECK (((username)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_151e1c5749ebd3d4f183ada8f26ef7aa CHECK ((other_information !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_29a7e96467b69a9f5a93332e29e9b0de CHECK (((role)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_47dc9eae45a990dcf18d00c7b741c0a3 CHECK ((access_product_ids !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_86ed7997641e812cde96e71352eedda1 CHECK ((active_sender_identifier !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9003d1df22eb4d3820015070385194c8 CHECK (((pwd)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_91284838812a3d21e7e57f3e0b538420 CHECK ((access_sender_ids !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a5d491060952aa8ad5fdee071be752de CHECK ((comments !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_ed054376a4842d8e1e35811b09cfcc15 CHECK ((access_study_ids !~ '^[[:space:]]*$'::text)),
    CONSTRAINT user_role_valid CHECK ((((role)::text = ANY (ARRAY[('system_admin'::character varying)::text, ('sponsor_admin_cro'::character varying)::text, ('sponsor_admin_company'::character varying)::text, ('user'::character varying)::text])) OR ((role)::text ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'::text)))
);

ALTER TABLE ONLY public.users FORCE ROW LEVEL SECURITY;


--
-- Name: COLUMN users.access_blind_allowed; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.users.access_blind_allowed IS 'Legacy name: TRUE hides the account from default user listings; unrelated to E2B G.k.2.5.';


--
-- Name: whodrug_products; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.whodrug_products (
    id bigint NOT NULL,
    audit_id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    code character varying(20) NOT NULL,
    drug_name text NOT NULL,
    atc_code character varying(20),
    version character varying(20) NOT NULL,
    language character varying(2) DEFAULT 'en'::character varying,
    active boolean DEFAULT true,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_2af72f100c356273d46284f6fd1dfc08 CHECK (((version)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8512ae7d57b1396273f76fe6ed341a23 CHECK (((language)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_a2b69451b2e0e5c02fa7395cd45ee9ef CHECK (((atc_code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_c13367945d5d4c91047b3b50234aa7ab CHECK (((code)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_d8c1dd46d45c5c380ea8f39d6010506e CHECK ((drug_name !~ '^[[:space:]]*$'::text))
);

ALTER TABLE ONLY public.whodrug_products FORCE ROW LEVEL SECURITY;


--
-- Name: whodrug_products_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.whodrug_products_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: whodrug_products_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.whodrug_products_id_seq OWNED BY public.whodrug_products.id;


--
-- Name: xml_export_history; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.xml_export_history (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    case_id uuid NOT NULL,
    case_number character varying(100),
    file_name character varying(255) NOT NULL,
    status character varying(20) NOT NULL,
    error_message text,
    validation_authority character varying(16),
    exported_by uuid NOT NULL,
    exported_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT model_nonblank_3090ad88ff7c9ca191579b709069fe64 CHECK (((file_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_7d9bd41d97436e5f583d9f36dd779367 CHECK (((validation_authority)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_90f75671c488110a08ef5a32614dc4b4 CHECK (((case_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9acb44549b41563697bb490144ec6258 CHECK (((status)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_bd5239d728d46691ef17539173044de7 CHECK ((error_message !~ '^[[:space:]]*$'::text)),
    CONSTRAINT xml_export_history_authority_valid CHECK (((validation_authority IS NULL) OR ((validation_authority)::text = ANY (ARRAY[('ich'::character varying)::text, ('fda'::character varying)::text, ('mfds'::character varying)::text])))),
    CONSTRAINT xml_export_history_status_valid CHECK (((status)::text = ANY (ARRAY[('success'::character varying)::text, ('error'::character varying)::text])))
);

ALTER TABLE ONLY public.xml_export_history FORCE ROW LEVEL SECURITY;


--
-- Name: xml_import_history; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.xml_import_history (
    id uuid DEFAULT public.gen_random_uuid() NOT NULL,
    uploaded_file_name character varying(255) NOT NULL,
    source_file_name character varying(255) NOT NULL,
    case_id uuid,
    case_number character varying(100),
    status character varying(20) NOT NULL,
    error_message text,
    uploaded_by uuid NOT NULL,
    uploaded_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    public_error jsonb,
    CONSTRAINT model_nonblank_7c6722479b1559262d405fab9120d8f3 CHECK (((source_file_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_8f263d003198a0055613c91e1cc7ce1a CHECK (((uploaded_file_name)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_90f75671c488110a08ef5a32614dc4b4 CHECK (((case_number)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_9acb44549b41563697bb490144ec6258 CHECK (((status)::text !~ '^[[:space:]]*$'::text)),
    CONSTRAINT model_nonblank_bd5239d728d46691ef17539173044de7 CHECK ((error_message !~ '^[[:space:]]*$'::text)),
    CONSTRAINT xml_import_history_public_error_object CHECK (((public_error IS NULL) OR (((status)::text = 'error'::text) AND (jsonb_typeof(public_error) = 'object'::text)))),
    CONSTRAINT xml_import_history_status_valid CHECK (((status)::text = ANY (ARRAY[('success'::character varying)::text, ('warning'::character varying)::text, ('skipped'::character varying)::text, ('error'::character varying)::text])))
);

ALTER TABLE ONLY public.xml_import_history FORCE ROW LEVEL SECURITY;


--
-- Name: audit_logs id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ALTER COLUMN id SET DEFAULT nextval('public.audit_logs_id_seq'::regclass);


--
-- Name: authorization_audit_events id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_audit_events ALTER COLUMN id SET DEFAULT nextval('public.authorization_audit_events_id_seq'::regclass);


--
-- Name: authorization_migration_rejections id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_migration_rejections ALTER COLUMN id SET DEFAULT nextval('public.authorization_migration_rejections_id_seq'::regclass);


--
-- Name: controlled_terminology_terms id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.controlled_terminology_terms ALTER COLUMN id SET DEFAULT nextval('public.controlled_terminology_terms_id_seq'::regclass);


--
-- Name: e2b_code_lists id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.e2b_code_lists ALTER COLUMN id SET DEFAULT nextval('public.e2b_code_lists_id_seq'::regclass);


--
-- Name: fda_hierarchical_code_lists id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_hierarchical_code_lists ALTER COLUMN id SET DEFAULT nextval('public.fda_hierarchical_code_lists_id_seq'::regclass);


--
-- Name: meddra_terms id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.meddra_terms ALTER COLUMN id SET DEFAULT nextval('public.meddra_terms_id_seq'::regclass);


--
-- Name: mfds_product_substances id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mfds_product_substances ALTER COLUMN id SET DEFAULT nextval('public.mfds_product_substances_id_seq'::regclass);


--
-- Name: mfds_products id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mfds_products ALTER COLUMN id SET DEFAULT nextval('public.mfds_products_id_seq'::regclass);


--
-- Name: terminology_releases id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.terminology_releases ALTER COLUMN id SET DEFAULT nextval('public.terminology_releases_id_seq'::regclass);


--
-- Name: ucum_units id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ucum_units ALTER COLUMN id SET DEFAULT nextval('public.ucum_units_id_seq'::regclass);


--
-- Name: whodrug_products id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.whodrug_products ALTER COLUMN id SET DEFAULT nextval('public.whodrug_products_id_seq'::regclass);


--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: -
--

GRANT USAGE ON SCHEMA public TO e2br3_auditor_role;
GRANT USAGE ON SCHEMA public TO e2br3_app_role;
GRANT USAGE ON SCHEMA public TO e2br3_whodrug_search_owner;


--
-- Name: FUNCTION app_settings_default_value(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION app_settings_default_value_for_org(p_org_type text); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION append_audit_log(p_table_name text, p_record_id uuid, p_organization_id uuid, p_action text, p_old_values jsonb, p_new_values jsonb, p_changed_fields jsonb); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.append_audit_log(p_table_name text, p_record_id uuid, p_organization_id uuid, p_action text, p_old_values jsonb, p_new_values jsonb, p_changed_fields jsonb) FROM PUBLIC;
GRANT ALL ON FUNCTION public.append_audit_log(p_table_name text, p_record_id uuid, p_organization_id uuid, p_action text, p_old_values jsonb, p_new_values jsonb, p_changed_fields jsonb) TO e2br3_app_role;


--
-- Name: FUNCTION audit_jsonb_is_distinct(p_old jsonb, p_new jsonb); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.audit_jsonb_is_distinct(p_old jsonb, p_new jsonb) FROM PUBLIC;
GRANT ALL ON FUNCTION public.audit_jsonb_is_distinct(p_old jsonb, p_new jsonb) TO e2br3_app_role;


--
-- Name: FUNCTION audit_log_organization_id(p_table_name text, p_record_id uuid, p_old_values jsonb, p_new_values jsonb); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION audit_logs_hash_chain_before_insert(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION audit_parent_record_ids(p_old_values jsonb, p_new_values jsonb); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION audit_trigger_function(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION audit_trigger_function_with_audit_id(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION audit_trigger_function_with_submission_id(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION audit_user_display(p_user_id uuid); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.audit_user_display(p_user_id uuid) TO e2br3_app_role;
GRANT ALL ON FUNCTION public.audit_user_display(p_user_id uuid) TO e2br3_auditor_role;


--
-- Name: FUNCTION authz_assert_role_administrator(target_organization_id uuid); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_assert_role_administrator(target_organization_id uuid) FROM PUBLIC;


--
-- Name: FUNCTION authz_assign_baseline_user_role(target_user_id uuid, target_organization_id uuid); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_assign_baseline_user_role(target_user_id uuid, target_organization_id uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.authz_assign_baseline_user_role(target_user_id uuid, target_organization_id uuid) TO e2br3_app_role;


--
-- Name: FUNCTION authz_assign_user_role(target_user_id uuid, target_organization_id uuid, target_role_id uuid); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_assign_user_role(target_user_id uuid, target_organization_id uuid, target_role_id uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.authz_assign_user_role(target_user_id uuid, target_organization_id uuid, target_role_id uuid) TO e2br3_app_role;


--
-- Name: FUNCTION authz_guard_organization_role_compatibility(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_guard_organization_role_compatibility() FROM PUBLIC;


--
-- Name: FUNCTION authz_increment_assignment_row_version(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_increment_assignment_row_version() FROM PUBLIC;


--
-- Name: FUNCTION authz_increment_role_row_version(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_increment_role_row_version() FROM PUBLIC;


--
-- Name: FUNCTION authz_initialize_membership_revision(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_initialize_membership_revision() FROM PUBLIC;


--
-- Name: FUNCTION authz_initialize_organization_revision(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_initialize_organization_revision() FROM PUBLIC;


--
-- Name: FUNCTION authz_lock_organization_policy_revision(target_organization_id uuid); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_lock_organization_policy_revision(target_organization_id uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.authz_lock_organization_policy_revision(target_organization_id uuid) TO e2br3_app_role;


--
-- Name: FUNCTION authz_lock_policy_revisions(target_user_id uuid, target_organization_id uuid); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_lock_policy_revisions(target_user_id uuid, target_organization_id uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.authz_lock_policy_revisions(target_user_id uuid, target_organization_id uuid) TO e2br3_app_role;


--
-- Name: FUNCTION authz_revision_organization_authorization_roles(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_revision_organization_authorization_roles() FROM PUBLIC;


--
-- Name: FUNCTION authz_revision_organization_organizations(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_revision_organization_organizations() FROM PUBLIC;


--
-- Name: FUNCTION authz_revision_organization_role_grants(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_revision_organization_role_grants() FROM PUBLIC;


--
-- Name: FUNCTION authz_revision_organization_scope_definition(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_revision_organization_scope_definition() FROM PUBLIC;


--
-- Name: FUNCTION authz_revision_principal_user_organization_memberships(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_revision_principal_user_organization_memberships() FROM PUBLIC;


--
-- Name: FUNCTION authz_revision_principal_user_role_assignments(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_revision_principal_user_role_assignments() FROM PUBLIC;


--
-- Name: FUNCTION authz_revision_principal_users(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_revision_principal_users() FROM PUBLIC;


--
-- Name: FUNCTION authz_touch_organization_revision(target_schema text, target_organization_id uuid); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_touch_organization_revision(target_schema text, target_organization_id uuid) FROM PUBLIC;


--
-- Name: FUNCTION authz_touch_principal_revision(target_schema text, target_user_id uuid, target_organization_id uuid); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_touch_principal_revision(target_schema text, target_user_id uuid, target_organization_id uuid) FROM PUBLIC;


--
-- Name: FUNCTION authz_upsert_custom_role(target_role_id uuid, target_organization_id uuid, target_name text, target_active boolean, target_grant_ids text[]); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.authz_upsert_custom_role(target_role_id uuid, target_organization_id uuid, target_name text, target_active boolean, target_grant_ids text[]) FROM PUBLIC;
GRANT ALL ON FUNCTION public.authz_upsert_custom_role(target_role_id uuid, target_organization_id uuid, target_name text, target_active boolean, target_grant_ids text[]) TO e2br3_app_role;


--
-- Name: FUNCTION case_scope_identifiers(target_case_id uuid); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION compute_audit_changed_fields(p_old jsonb, p_new jsonb, p_prefix text); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.compute_audit_changed_fields(p_old jsonb, p_new jsonb, p_prefix text) TO e2br3_app_role;


--
-- Name: FUNCTION current_organization_id(); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.current_organization_id() TO e2br3_app_role;


--
-- Name: FUNCTION enforce_role_grant_assignment(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION enforce_user_role_assignment_scope(); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.enforce_user_role_assignment_scope() FROM PUBLIC;


--
-- Name: FUNCTION enforce_value_null_flavor_exclusivity(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION get_current_change_category(); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.get_current_change_category() TO e2br3_app_role;


--
-- Name: FUNCTION get_current_change_reason(); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.get_current_change_reason() TO e2br3_app_role;


--
-- Name: FUNCTION get_current_esignature_id(); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.get_current_esignature_id() TO e2br3_app_role;


--
-- Name: FUNCTION get_current_user_context(); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.get_current_user_context() TO e2br3_app_role;


--
-- Name: FUNCTION has_business_change_on_update(p_old anyelement, p_new anyelement); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION is_current_user_admin(); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.is_current_user_admin() TO e2br3_app_role;


--
-- Name: TABLE audit_logs; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT ON TABLE public.audit_logs TO e2br3_app_role;
GRANT SELECT ON TABLE public.audit_logs TO e2br3_auditor_role;


--
-- Name: FUNCTION list_case_audit_logs(p_case_id uuid); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.list_case_audit_logs(p_case_id uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.list_case_audit_logs(p_case_id uuid) TO e2br3_app_role;


--
-- Name: FUNCTION lock_audit_log_append(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_c(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_c_from_study_info(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_d(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_d_from_death_info(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_d_from_parent(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_d_from_patient(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_e(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_f(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_g(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_g_from_drug(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_h(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION mark_case_dirty_h_from_narrative(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION model_text_array_has_no_blank(values_to_check text[]); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION prevent_audit_log_mutation(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION prevent_authorization_audit_mutation(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION protect_builtin_authorization_role(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION protect_builtin_role_grant(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION protect_submission_artifact(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION require_active_presave_reference(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION search_active_whodrug_products(p_query text, p_limit bigint); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.search_active_whodrug_products(p_query text, p_limit bigint) FROM PUBLIC;
GRANT ALL ON FUNCTION public.search_active_whodrug_products(p_query text, p_limit bigint) TO e2br3_app_role;


--
-- Name: FUNCTION select_current_user_organization(target_organization_id uuid); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.select_current_user_organization(target_organization_id uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.select_current_user_organization(target_organization_id uuid) TO e2br3_app_role;


--
-- Name: FUNCTION set_authorization_isolation_context(org_id uuid, requested_platform_bypass boolean); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.set_authorization_isolation_context(org_id uuid, requested_platform_bypass boolean) FROM PUBLIC;
GRANT ALL ON FUNCTION public.set_authorization_isolation_context(org_id uuid, requested_platform_bypass boolean) TO e2br3_app_role;


--
-- Name: FUNCTION set_compliance_context(p_change_reason text, p_change_category text, p_e_signature_id text); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.set_compliance_context(p_change_reason text, p_change_category text, p_e_signature_id text) TO e2br3_app_role;


--
-- Name: FUNCTION set_current_user_context(p_user_id uuid); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.set_current_user_context(p_user_id uuid) TO e2br3_app_role;


--
-- Name: FUNCTION set_org_context(org_id uuid, user_role character varying); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.set_org_context(org_id uuid, user_role character varying) TO e2br3_app_role;


--
-- Name: FUNCTION update_updated_at_column(); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION validate_scope_assignment(target_organization_id uuid, sender_ids text[], product_ids text[], study_ids text[]); Type: ACL; Schema: public; Owner: -
--



--
-- Name: FUNCTION validate_user_context(); Type: ACL; Schema: public; Owner: -
--

GRANT ALL ON FUNCTION public.validate_user_context() TO e2br3_app_role;


--
-- Name: FUNCTION verify_audit_log_hash_chain(p_since_id bigint); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.verify_audit_log_hash_chain(p_since_id bigint) FROM PUBLIC;
GRANT ALL ON FUNCTION public.verify_audit_log_hash_chain(p_since_id bigint) TO e2br3_app_role;


--
-- Name: FUNCTION verify_audit_log_hash_chain_range(p_since_id bigint, p_until_id bigint, p_limit integer); Type: ACL; Schema: public; Owner: -
--

REVOKE ALL ON FUNCTION public.verify_audit_log_hash_chain_range(p_since_id bigint, p_until_id bigint, p_limit integer) FROM PUBLIC;
GRANT ALL ON FUNCTION public.verify_audit_log_hash_chain_range(p_since_id bigint, p_until_id bigint, p_limit integer) TO e2br3_app_role;


--
-- Name: TABLE app_settings; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.app_settings TO e2br3_app_role;


--
-- Name: SEQUENCE audit_logs_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.audit_logs_id_seq TO e2br3_app_role;


--
-- Name: TABLE authorization_audit_events; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT ON TABLE public.authorization_audit_events TO e2br3_app_role;


--
-- Name: SEQUENCE authorization_audit_events_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.authorization_audit_events_id_seq TO e2br3_app_role;


--
-- Name: TABLE authorization_catalog_state; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT ON TABLE public.authorization_catalog_state TO e2br3_app_role;


--
-- Name: TABLE authorization_grant_catalog; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT ON TABLE public.authorization_grant_catalog TO e2br3_app_role;


--
-- Name: TABLE authorization_grant_role_classes; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT ON TABLE public.authorization_grant_role_classes TO e2br3_app_role;


--
-- Name: TABLE authorization_migration_reconciliations; Type: ACL; Schema: public; Owner: -
--



--
-- Name: TABLE authorization_migration_rejections; Type: ACL; Schema: public; Owner: -
--



--
-- Name: SEQUENCE authorization_migration_rejections_id_seq; Type: ACL; Schema: public; Owner: -
--



--
-- Name: TABLE authorization_roles; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT ON TABLE public.authorization_roles TO e2br3_app_role;


--
-- Name: TABLE autopsy_causes_of_death; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.autopsy_causes_of_death TO e2br3_app_role;


--
-- Name: TABLE case_e2b_field_notations; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.case_e2b_field_notations TO e2br3_app_role;


--
-- Name: TABLE case_field_notations; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.case_field_notations TO e2br3_app_role;


--
-- Name: TABLE case_submissions; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.case_submissions TO e2br3_app_role;


--
-- Name: TABLE case_summary_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.case_summary_information TO e2br3_app_role;


--
-- Name: TABLE case_validation_summaries; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.case_validation_summaries TO e2br3_app_role;


--
-- Name: TABLE case_versions; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.case_versions TO e2br3_app_role;


--
-- Name: TABLE case_workflow_events; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.case_workflow_events TO e2br3_app_role;


--
-- Name: TABLE cases; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.cases TO e2br3_app_role;


--
-- Name: TABLE controlled_terminology_terms; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.controlled_terminology_terms TO e2br3_app_role;


--
-- Name: SEQUENCE controlled_terminology_terms_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.controlled_terminology_terms_id_seq TO e2br3_app_role;


--
-- Name: TABLE dashboard_notices; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.dashboard_notices TO e2br3_app_role;


--
-- Name: TABLE documents_held_by_sender; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.documents_held_by_sender TO e2br3_app_role;


--
-- Name: TABLE dosage_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.dosage_information TO e2br3_app_role;


--
-- Name: TABLE drug_active_substances; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.drug_active_substances TO e2br3_app_role;


--
-- Name: TABLE drug_device_characteristics; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.drug_device_characteristics TO e2br3_app_role;


--
-- Name: TABLE drug_indications; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.drug_indications TO e2br3_app_role;


--
-- Name: TABLE drug_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.drug_information TO e2br3_app_role;


--
-- Name: TABLE drug_reaction_assessments; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.drug_reaction_assessments TO e2br3_app_role;


--
-- Name: TABLE e2b_code_lists; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.e2b_code_lists TO e2br3_app_role;


--
-- Name: SEQUENCE e2b_code_lists_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.e2b_code_lists_id_seq TO e2br3_app_role;


--
-- Name: TABLE e_signatures; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.e_signatures TO e2br3_app_role;


--
-- Name: TABLE fda_device_codes; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.fda_device_codes TO e2br3_app_role;


--
-- Name: TABLE fda_device_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.fda_device_information TO e2br3_app_role;


--
-- Name: TABLE fda_hierarchical_code_lists; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.fda_hierarchical_code_lists TO e2br3_app_role;


--
-- Name: SEQUENCE fda_hierarchical_code_lists_id_seq; Type: ACL; Schema: public; Owner: -
--



--
-- Name: TABLE iso_countries; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.iso_countries TO e2br3_app_role;


--
-- Name: TABLE linked_report_numbers; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.linked_report_numbers TO e2br3_app_role;


--
-- Name: TABLE literature_references; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.literature_references TO e2br3_app_role;


--
-- Name: TABLE meddra_terms; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.meddra_terms TO e2br3_app_role;


--
-- Name: SEQUENCE meddra_terms_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.meddra_terms_id_seq TO e2br3_app_role;


--
-- Name: TABLE medical_history_episodes; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medical_history_episodes TO e2br3_app_role;


--
-- Name: TABLE mfds_product_substances; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.mfds_product_substances TO e2br3_app_role;


--
-- Name: SEQUENCE mfds_product_substances_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.mfds_product_substances_id_seq TO e2br3_app_role;


--
-- Name: TABLE mfds_products; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.mfds_products TO e2br3_app_role;


--
-- Name: SEQUENCE mfds_products_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.mfds_products_id_seq TO e2br3_app_role;


--
-- Name: TABLE narrative_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.narrative_information TO e2br3_app_role;


--
-- Name: TABLE narrative_presaves; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.narrative_presaves TO e2br3_app_role;


--
-- Name: TABLE organization_as2_profiles; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.organization_as2_profiles TO e2br3_app_role;


--
-- Name: TABLE organization_policy_state; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT ON TABLE public.organization_policy_state TO e2br3_app_role;


--
-- Name: TABLE organizations; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.organizations TO e2br3_app_role;


--
-- Name: TABLE other_case_identifiers; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.other_case_identifiers TO e2br3_app_role;


--
-- Name: TABLE parent_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.parent_information TO e2br3_app_role;


--
-- Name: TABLE parent_medical_history; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.parent_medical_history TO e2br3_app_role;


--
-- Name: TABLE parent_past_drug_history; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.parent_past_drug_history TO e2br3_app_role;


--
-- Name: TABLE past_drug_history; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.past_drug_history TO e2br3_app_role;


--
-- Name: TABLE patient_death_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.patient_death_information TO e2br3_app_role;


--
-- Name: TABLE patient_identifiers; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.patient_identifiers TO e2br3_app_role;


--
-- Name: TABLE patient_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.patient_information TO e2br3_app_role;


--
-- Name: TABLE permission_profiles; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.permission_profiles TO e2br3_app_role;


--
-- Name: TABLE primary_sources; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.primary_sources TO e2br3_app_role;


--
-- Name: TABLE principal_authorization_state; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT ON TABLE public.principal_authorization_state TO e2br3_app_role;


--
-- Name: TABLE product_presave_active_substances; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.product_presave_active_substances TO e2br3_app_role;


--
-- Name: TABLE product_presaves; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.product_presaves TO e2br3_app_role;


--
-- Name: TABLE rbac_policy_state; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.rbac_policy_state TO e2br3_app_role;


--
-- Name: TABLE reactions; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.reactions TO e2br3_app_role;


--
-- Name: TABLE receiver_presave_consignees; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.receiver_presave_consignees TO e2br3_app_role;


--
-- Name: TABLE receiver_presave_routes; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.receiver_presave_routes TO e2br3_app_role;


--
-- Name: TABLE receiver_presaves; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.receiver_presaves TO e2br3_app_role;


--
-- Name: TABLE relatedness_assessments; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.relatedness_assessments TO e2br3_app_role;


--
-- Name: TABLE reported_causes_of_death; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.reported_causes_of_death TO e2br3_app_role;


--
-- Name: TABLE reporter_presaves; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.reporter_presaves TO e2br3_app_role;


--
-- Name: TABLE role_grants; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT ON TABLE public.role_grants TO e2br3_app_role;


--
-- Name: TABLE safety_report_identification; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.safety_report_identification TO e2br3_app_role;


--
-- Name: TABLE sender_diagnoses; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.sender_diagnoses TO e2br3_app_role;


--
-- Name: TABLE sender_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.sender_information TO e2br3_app_role;


--
-- Name: TABLE sender_presave_gateways; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.sender_presave_gateways TO e2br3_app_role;


--
-- Name: TABLE sender_presave_responsible_persons; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.sender_presave_responsible_persons TO e2br3_app_role;


--
-- Name: TABLE sender_presaves; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.sender_presaves TO e2br3_app_role;


--
-- Name: TABLE source_documents; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.source_documents TO e2br3_app_role;


--
-- Name: TABLE study_fda_cross_reported_inds; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.study_fda_cross_reported_inds TO e2br3_app_role;


--
-- Name: TABLE study_information; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.study_information TO e2br3_app_role;


--
-- Name: TABLE study_presave_fda_cross_reported_ind_numbers; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.study_presave_fda_cross_reported_ind_numbers TO e2br3_app_role;


--
-- Name: TABLE study_presave_products; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.study_presave_products TO e2br3_app_role;


--
-- Name: TABLE study_presave_registration_numbers; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.study_presave_registration_numbers TO e2br3_app_role;


--
-- Name: TABLE study_presave_reporters; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.study_presave_reporters TO e2br3_app_role;


--
-- Name: TABLE study_presaves; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.study_presaves TO e2br3_app_role;


--
-- Name: TABLE study_registration_numbers; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.study_registration_numbers TO e2br3_app_role;


--
-- Name: TABLE submission_acks; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.submission_acks TO e2br3_app_role;


--
-- Name: TABLE submission_dispatch_state; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.submission_dispatch_state TO e2br3_app_role;


--
-- Name: TABLE submission_events; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.submission_events TO e2br3_app_role;


--
-- Name: TABLE submission_idempotency; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.submission_idempotency TO e2br3_app_role;


--
-- Name: TABLE terminology_releases; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.terminology_releases TO e2br3_app_role;
GRANT SELECT ON TABLE public.terminology_releases TO e2br3_whodrug_search_owner;


--
-- Name: SEQUENCE terminology_releases_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.terminology_releases_id_seq TO e2br3_app_role;


--
-- Name: TABLE test_results; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.test_results TO e2br3_app_role;


--
-- Name: TABLE ucum_units; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.ucum_units TO e2br3_app_role;


--
-- Name: SEQUENCE ucum_units_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.ucum_units_id_seq TO e2br3_app_role;


--
-- Name: TABLE user_organization_memberships; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.user_organization_memberships TO e2br3_app_role;


--
-- Name: TABLE user_role_assignments; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT ON TABLE public.user_role_assignments TO e2br3_app_role;


--
-- Name: TABLE users; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.users TO e2br3_app_role;


--
-- Name: TABLE whodrug_products; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.whodrug_products TO e2br3_app_role;
GRANT SELECT ON TABLE public.whodrug_products TO e2br3_whodrug_search_owner;


--
-- Name: SEQUENCE whodrug_products_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,USAGE ON SEQUENCE public.whodrug_products_id_seq TO e2br3_app_role;


--
-- Name: TABLE xml_export_history; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.xml_export_history TO e2br3_app_role;


--
-- Name: TABLE xml_import_history; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.xml_import_history TO e2br3_app_role;


--
-- PostgreSQL database dump complete
--
