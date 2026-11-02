-- Reconcile confirmed runtime omissions without claiming historical execution.
-- Definitions are snapshots of the reviewed migrations named below.

-- From 20261027_whodrug_rls_search.sql
DO $$
DECLARE
    search_owner OID;
    search_owner_login BOOLEAN;
    search_owner_inherit BOOLEAN;
    search_owner_bypass BOOLEAN;
    search_owner_super BOOLEAN;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
        WHERE rolname = current_user AND rolsuper
    ) THEN
        RAISE EXCEPTION
            'WHODrug search migration requires a superuser migration credential'
            USING ERRCODE = '42501';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
        WHERE rolname = 'e2br3_whodrug_search_owner'
    ) THEN
        EXECUTE 'CREATE ROLE e2br3_whodrug_search_owner NOLOGIN NOINHERIT BYPASSRLS';
    END IF;

    SELECT oid, rolcanlogin, rolinherit, rolbypassrls, rolsuper
      INTO search_owner, search_owner_login, search_owner_inherit,
           search_owner_bypass, search_owner_super
      FROM pg_catalog.pg_roles
     WHERE rolname = 'e2br3_whodrug_search_owner';

    IF search_owner_login
       OR search_owner_inherit
       OR NOT (search_owner_bypass OR search_owner_super) THEN
        RAISE EXCEPTION
            'e2br3_whodrug_search_owner must be NOLOGIN, NOINHERIT, and BYPASSRLS or SUPERUSER'
            USING ERRCODE = '42501';
    END IF;

    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members
        WHERE roleid = search_owner OR member = search_owner
    ) THEN
        RAISE EXCEPTION
            'e2br3_whodrug_search_owner must have no role members or memberships'
            USING ERRCODE = '42501';
    END IF;
END;
$$;

REVOKE ALL ON SCHEMA public FROM e2br3_whodrug_search_owner;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM e2br3_whodrug_search_owner;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM e2br3_whodrug_search_owner;
GRANT USAGE ON SCHEMA public TO e2br3_whodrug_search_owner;
GRANT SELECT ON TABLE public.whodrug_products, public.terminology_releases
    TO e2br3_whodrug_search_owner;

CREATE OR REPLACE FUNCTION public.search_active_whodrug_products(
    p_query TEXT,
    p_limit BIGINT
)
RETURNS TABLE (
    id BIGINT,
    code VARCHAR(20),
    drug_name TEXT,
    atc_code VARCHAR(20),
    version VARCHAR(20),
    language VARCHAR(2),
    active BOOLEAN,
    created_at TIMESTAMPTZ
)
LANGUAGE SQL
STABLE
STRICT
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
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

REVOKE ALL ON FUNCTION public.search_active_whodrug_products(TEXT, BIGINT)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.search_active_whodrug_products(TEXT, BIGINT)
    TO e2br3_app_role;
ALTER FUNCTION public.search_active_whodrug_products(TEXT, BIGINT)
    OWNER TO e2br3_whodrug_search_owner;

-- From 20260918_fast_unicode_jsonb_comparison.sql
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

-- From 20260921_fda_device_audit_triggers.sql
DROP TRIGGER IF EXISTS audit_fda_device_information ON fda_device_information;
CREATE TRIGGER audit_fda_device_information
    AFTER INSERT OR UPDATE OR DELETE ON fda_device_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

DROP TRIGGER IF EXISTS audit_fda_device_codes ON fda_device_codes;
CREATE TRIGGER audit_fda_device_codes
    AFTER INSERT OR UPDATE OR DELETE ON fda_device_codes
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

-- From 20261006_source_documents_audit.sql
DROP TRIGGER IF EXISTS audit_source_documents ON source_documents;
CREATE TRIGGER audit_source_documents
    AFTER INSERT OR UPDATE OR DELETE ON source_documents
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

-- Require the known constraint; an unexpected schema must fail explicitly.
ALTER TABLE patient_information
    DROP CONSTRAINT patient_information_age_group_check,
    ADD CONSTRAINT patient_information_age_group_check
        CHECK (age_group IN ('0', '1', '2', '3', '4', '5', '6'));

-- MFDS uses MSK; storage retains the union and request validation selects the authority.
ALTER TABLE test_results
    DROP CONSTRAINT IF EXISTS test_results_test_date_null_flavor_check,
    DROP CONSTRAINT ck_nf_test_date,
    ADD CONSTRAINT ck_nf_test_date CHECK (
        test_date_null_flavor IS NULL OR test_date_null_flavor IN ('UNK', 'MSK')
    );

SELECT set_current_user_context('00000000-0000-0000-0000-000000000001');

INSERT INTO e2b_code_lists (list_name, code, display_name, sort_order, active)
VALUES ('age_group', '0', 'Foetus', 0, true),
       ('drug_characterization', '4', 'Drug Not Administered', 4, true)
ON CONFLICT (list_name, code) DO UPDATE
SET display_name = EXCLUDED.display_name,
    sort_order = EXCLUDED.sort_order,
    active = EXCLUDED.active
WHERE (e2b_code_lists.display_name, e2b_code_lists.sort_order, e2b_code_lists.active)
    IS DISTINCT FROM (EXCLUDED.display_name, EXCLUDED.sort_order, EXCLUDED.active);

-- NULL requires recomputation; do not invent a validation revision for old caches.
ALTER TABLE case_validation_summaries
    ADD COLUMN IF NOT EXISTS validation_revision TEXT;
