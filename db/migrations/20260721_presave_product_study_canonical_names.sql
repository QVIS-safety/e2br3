DO $$
BEGIN
    IF to_regclass('public.product_presave_substances') IS NOT NULL
       AND to_regclass('public.product_presave_active_substances') IS NULL THEN
        ALTER TABLE product_presave_substances
            RENAME TO product_presave_active_substances;
        ALTER TABLE product_presave_active_substances
            RENAME CONSTRAINT product_presave_substances_sequence_unique
            TO product_presave_active_substances_sequence_unique;
        ALTER INDEX idx_product_presave_substances_parent
            RENAME TO idx_product_presave_active_substances_parent;
        ALTER POLICY product_presave_substances_via_parent
            ON product_presave_active_substances
            RENAME TO product_presave_active_substances_via_parent;
        ALTER TRIGGER audit_product_presave_substances
            ON product_presave_active_substances
            RENAME TO audit_product_presave_active_substances;
        ALTER TRIGGER update_product_presave_substances_updated_at
            ON product_presave_active_substances
            RENAME TO update_product_presave_active_substances_updated_at;
    END IF;
END;
$$;

DO $$
BEGIN
    IF to_regclass('public.study_presave_fda_cross_reported_inds') IS NOT NULL
       AND to_regclass('public.study_presave_fda_cross_reported_ind_numbers') IS NULL THEN
        ALTER TABLE study_presave_fda_cross_reported_inds
            RENAME TO study_presave_fda_cross_reported_ind_numbers;
        ALTER TABLE study_presave_fda_cross_reported_ind_numbers
            RENAME CONSTRAINT study_presave_fda_cross_reported_inds_sequence_unique
            TO study_presave_fda_cross_reported_ind_numbers_sequence_unique;
        ALTER INDEX idx_study_presave_fda_cross_reported_inds_parent
            RENAME TO idx_study_presave_fda_cross_reported_ind_numbers_parent;
        ALTER POLICY study_presave_fda_cross_reported_inds_via_parent
            ON study_presave_fda_cross_reported_ind_numbers
            RENAME TO study_presave_fda_cross_reported_ind_numbers_via_parent;
        ALTER TRIGGER audit_study_presave_fda_cross_reported_inds
            ON study_presave_fda_cross_reported_ind_numbers
            RENAME TO audit_study_presave_fda_cross_reported_ind_numbers;
        ALTER TRIGGER update_study_presave_fda_cross_reported_inds_updated_at
            ON study_presave_fda_cross_reported_ind_numbers
            RENAME TO update_study_presave_fda_cross_reported_ind_numbers_updated_at;
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION audit_log_organization_id(
    p_table_name TEXT,
    p_record_id UUID,
    p_old_values JSONB,
    p_new_values JSONB
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_values JSONB;
    v_org_id UUID;
    v_case_id UUID;
    v_submission_id UUID;
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
        SELECT c.organization_id INTO v_org_id FROM cases c WHERE c.id = v_case_id;
        IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
    END IF;

    v_submission_id := NULLIF(v_values->>'submission_id', '')::UUID;
    IF v_submission_id IS NOT NULL THEN
        SELECT c.organization_id INTO v_org_id
        FROM case_submissions cs
        JOIN cases c ON c.id = cs.case_id
        WHERE cs.id = v_submission_id;
        IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
    END IF;

    IF p_table_name IN ('sender_presave_gateways', 'sender_presave_responsible_persons') THEN
        SELECT p.organization_id INTO v_org_id
        FROM sender_presaves p
        WHERE p.id = NULLIF(v_values->>'sender_presave_id', '')::UUID;
        IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
    END IF;

    IF p_table_name IN ('receiver_presave_consignees', 'receiver_presave_routes') THEN
        SELECT p.organization_id INTO v_org_id
        FROM receiver_presaves p
        WHERE p.id = NULLIF(v_values->>'receiver_presave_id', '')::UUID;
        IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
    END IF;

    IF p_table_name = 'product_presave_active_substances' THEN
        SELECT p.organization_id INTO v_org_id
        FROM product_presaves p
        WHERE p.id = NULLIF(v_values->>'product_presave_id', '')::UUID;
        IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
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
        IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
    END IF;

    RETURN COALESCE(
        current_organization_id(),
        '00000000-0000-0000-0000-000000000000'::UUID
    );
END;
$$;
