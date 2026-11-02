-- Audit future drug device characteristic writes through the standard audit
-- function. Historical writes are not recreated because their actor and reason
-- cannot be recovered reliably.
CREATE OR REPLACE FUNCTION audit_log_organization_id(
    p_table_name TEXT,
    p_record_id UUID,
    p_old_values JSONB,
    p_new_values JSONB
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
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

DROP TRIGGER IF EXISTS update_drug_device_characteristics_updated_at
    ON drug_device_characteristics;
CREATE TRIGGER update_drug_device_characteristics_updated_at
    BEFORE UPDATE ON drug_device_characteristics
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS audit_drug_device_characteristics
    ON drug_device_characteristics;
CREATE TRIGGER audit_drug_device_characteristics
    AFTER INSERT OR UPDATE OR DELETE ON drug_device_characteristics
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();
