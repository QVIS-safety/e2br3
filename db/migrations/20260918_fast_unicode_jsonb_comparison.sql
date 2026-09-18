-- Speed long Unicode row comparisons without changing JSONB equality semantics.
-- A hash mismatch proves inequality; hash matches retain the native comparison.


CREATE OR REPLACE FUNCTION audit_jsonb_is_distinct(
    p_old JSONB,
    p_new JSONB
)
RETURNS BOOLEAN
LANGUAGE SQL
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT CASE
        WHEN pg_catalog.jsonb_hash_extended(p_old, 0)
             IS DISTINCT FROM pg_catalog.jsonb_hash_extended(p_new, 0)
            THEN TRUE
        ELSE p_old IS DISTINCT FROM p_new
    END;
$$;

REVOKE ALL ON FUNCTION audit_jsonb_is_distinct(JSONB, JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION audit_jsonb_is_distinct(JSONB, JSONB) TO e2br3_app_role;

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
$$;

CREATE OR REPLACE FUNCTION audit_trigger_function()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
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

CREATE OR REPLACE FUNCTION audit_trigger_function_with_audit_id()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
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

CREATE OR REPLACE FUNCTION audit_trigger_function_with_submission_id()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
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

CREATE OR REPLACE FUNCTION has_business_change_on_update(
  p_old anyelement,
  p_new anyelement
) RETURNS BOOLEAN AS $$
DECLARE
  v_old_business JSONB;
  v_new_business JSONB;
BEGIN
  v_old_business := to_jsonb(p_old) - 'updated_at' - 'updated_by';
  v_new_business := to_jsonb(p_new) - 'updated_at' - 'updated_by';
  RETURN audit_jsonb_is_distinct(v_old_business, v_new_business);
END;
$$ LANGUAGE plpgsql;
