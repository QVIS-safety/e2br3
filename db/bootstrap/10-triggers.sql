CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_cases_updated_at BEFORE UPDATE ON cases
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_patient_info_updated_at BEFORE UPDATE ON patient_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_message_headers_updated_at BEFORE UPDATE ON message_headers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_safety_report_identification_updated_at BEFORE UPDATE ON safety_report_identification
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_sender_information_updated_at BEFORE UPDATE ON sender_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_study_information_updated_at BEFORE UPDATE ON study_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_primary_sources_updated_at BEFORE UPDATE ON primary_sources
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_patient_death_info_updated_at BEFORE UPDATE ON patient_death_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_parent_information_updated_at BEFORE UPDATE ON parent_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_reactions_updated_at BEFORE UPDATE ON reactions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_test_results_updated_at BEFORE UPDATE ON test_results
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_drug_info_updated_at BEFORE UPDATE ON drug_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_dosage_information_updated_at BEFORE UPDATE ON dosage_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_narrative_information_updated_at BEFORE UPDATE ON narrative_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_case_submissions_updated_at BEFORE UPDATE ON case_submissions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_submission_dispatch_state_updated_at BEFORE UPDATE ON submission_dispatch_state
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_e_signatures_updated_at BEFORE UPDATE ON e_signatures
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Audit Trail Trigger
-- ============================================================================

-- Build append-only hash chain for audit logs (Part 11 / Annex 11 integrity).
CREATE OR REPLACE FUNCTION audit_logs_hash_chain_before_insert()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_prev_hash TEXT;
    v_payload TEXT;
BEGIN
    IF NEW.created_at IS NULL THEN
        NEW.created_at := NOW();
    END IF;

    -- Serialize inserts so each row references the exact previous row.
    LOCK TABLE audit_logs IN SHARE ROW EXCLUSIVE MODE;

    SELECT entry_hash
      INTO v_prev_hash
      FROM audit_logs
     ORDER BY id DESC
     LIMIT 1;

    NEW.prev_hash := COALESCE(v_prev_hash, repeat('0', 64));

    v_payload := concat_ws(
        '|',
        COALESCE(NEW.id::TEXT, ''),
        NEW.prev_hash,
        NEW.table_name,
        NEW.record_id::TEXT,
        NEW.action,
        NEW.user_id::TEXT,
        COALESCE(NEW.reason_for_change, ''),
        COALESCE(NEW.e_signature_id::TEXT, ''),
        COALESCE(NEW.old_values::TEXT, 'null'),
        COALESCE(NEW.new_values::TEXT, 'null'),
        COALESCE(NEW.changed_fields::TEXT, 'null'),
        COALESCE(NEW.ip_address::TEXT, ''),
        COALESCE(NEW.user_agent, ''),
        to_char(NEW.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
    );

    NEW.entry_hash := encode(digest(v_payload, 'sha256'), 'hex');
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_audit_logs_hash_chain_before_insert ON audit_logs;
CREATE TRIGGER trg_audit_logs_hash_chain_before_insert
    BEFORE INSERT ON audit_logs
    FOR EACH ROW EXECUTE FUNCTION audit_logs_hash_chain_before_insert();

-- Resolve the tenant owner for each audit log row. Tables with their own
-- organization_id use that value; organization rows own their own audit
-- entries; child case/submission rows resolve through the parent case.
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

    RETURN COALESCE(
        current_organization_id(),
        '00000000-0000-0000-0000-000000000000'::UUID
    );
END;
$$;

-- Improved audit trigger function using helper function
CREATE OR REPLACE FUNCTION audit_trigger_function()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_user_id UUID;
    v_old_business JSONB;
    v_new_business JSONB;
    v_changed_fields JSONB;
BEGIN
    -- Get user from context (will fail if not set, ensuring user attribution)
    v_user_id := get_current_user_context();

    IF TG_OP = 'INSERT' THEN
        v_changed_fields := compute_audit_changed_fields(NULL, to_jsonb(NEW));
        INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, changed_fields)
        VALUES (
            TG_TABLE_NAME,
            NEW.id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.id, NULL, to_jsonb(NEW)),
            'CREATE',
            v_user_id,
            get_current_change_reason(),
            get_current_esignature_id(),
            NULL,
            to_jsonb(NEW)
            ,
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'UPDATE' THEN
        -- Ignore metadata-only updates (updated_at/updated_by) to avoid noisy
        -- audit trails from no-op saves.
        v_old_business := to_jsonb(OLD) - 'updated_at' - 'updated_by';
        v_new_business := to_jsonb(NEW) - 'updated_at' - 'updated_by';
        IF v_old_business = v_new_business THEN
            RETURN NEW;
        END IF;

        v_changed_fields := compute_audit_changed_fields(v_old_business, v_new_business);
        INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, changed_fields)
        VALUES (
            TG_TABLE_NAME,
            NEW.id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.id, to_jsonb(OLD), to_jsonb(NEW)),
            'UPDATE',
            v_user_id,
            get_current_change_reason(),
            get_current_esignature_id(),
            to_jsonb(OLD),
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'DELETE' THEN
        v_changed_fields := compute_audit_changed_fields(to_jsonb(OLD), NULL);
        INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, changed_fields)
        VALUES (
            TG_TABLE_NAME,
            OLD.id,
            audit_log_organization_id(TG_TABLE_NAME, OLD.id, to_jsonb(OLD), NULL),
            'DELETE',
            v_user_id,
            get_current_change_reason(),
            get_current_esignature_id(),
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

-- Audit trigger for tables that use audit_id UUID instead of id UUID.
CREATE OR REPLACE FUNCTION audit_trigger_function_with_audit_id()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_user_id UUID;
    v_old_business JSONB;
    v_new_business JSONB;
    v_changed_fields JSONB;
BEGIN
    v_user_id := get_current_user_context();

    IF TG_OP = 'INSERT' THEN
        v_changed_fields := compute_audit_changed_fields(NULL, to_jsonb(NEW));
        INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, changed_fields)
        VALUES (
            TG_TABLE_NAME,
            NEW.audit_id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.audit_id, NULL, to_jsonb(NEW)),
            'CREATE',
            v_user_id,
            get_current_change_reason(),
            get_current_esignature_id(),
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
        IF v_old_business = v_new_business THEN
            RETURN NEW;
        END IF;

        v_changed_fields := compute_audit_changed_fields(v_old_business, v_new_business);
        INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, changed_fields)
        VALUES (
            TG_TABLE_NAME,
            NEW.audit_id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.audit_id, to_jsonb(OLD), to_jsonb(NEW)),
            'UPDATE',
            v_user_id,
            get_current_change_reason(),
            get_current_esignature_id(),
            to_jsonb(OLD),
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'DELETE' THEN
        v_changed_fields := compute_audit_changed_fields(to_jsonb(OLD), NULL);
        INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, changed_fields)
        VALUES (
            TG_TABLE_NAME,
            OLD.audit_id,
            audit_log_organization_id(TG_TABLE_NAME, OLD.audit_id, to_jsonb(OLD), NULL),
            'DELETE',
            v_user_id,
            get_current_change_reason(),
            get_current_esignature_id(),
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

-- Audit trigger for tables that use submission_id UUID as the primary key.
CREATE OR REPLACE FUNCTION audit_trigger_function_with_submission_id()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_user_id UUID;
    v_old_business JSONB;
    v_new_business JSONB;
    v_changed_fields JSONB;
BEGIN
    v_user_id := get_current_user_context();

    IF TG_OP = 'INSERT' THEN
        v_changed_fields := compute_audit_changed_fields(NULL, to_jsonb(NEW));
        INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, changed_fields)
        VALUES (
            TG_TABLE_NAME,
            NEW.submission_id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.submission_id, NULL, to_jsonb(NEW)),
            'CREATE',
            v_user_id,
            get_current_change_reason(),
            get_current_esignature_id(),
            NULL,
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'UPDATE' THEN
        v_old_business := to_jsonb(OLD) - 'updated_at' - 'updated_by';
        v_new_business := to_jsonb(NEW) - 'updated_at' - 'updated_by';
        IF v_old_business = v_new_business THEN
            RETURN NEW;
        END IF;

        v_changed_fields := compute_audit_changed_fields(v_old_business, v_new_business);
        INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, changed_fields)
        VALUES (
            TG_TABLE_NAME,
            NEW.submission_id,
            audit_log_organization_id(TG_TABLE_NAME, NEW.submission_id, to_jsonb(OLD), to_jsonb(NEW)),
            'UPDATE',
            v_user_id,
            get_current_change_reason(),
            get_current_esignature_id(),
            to_jsonb(OLD),
            to_jsonb(NEW),
            v_changed_fields
        );
        RETURN NEW;

    ELSIF TG_OP = 'DELETE' THEN
        v_changed_fields := compute_audit_changed_fields(to_jsonb(OLD), NULL);
        INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, changed_fields)
        VALUES (
            TG_TABLE_NAME,
            OLD.submission_id,
            audit_log_organization_id(TG_TABLE_NAME, OLD.submission_id, to_jsonb(OLD), NULL),
            'DELETE',
            v_user_id,
            get_current_change_reason(),
            get_current_esignature_id(),
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

CREATE OR REPLACE FUNCTION presave_template_audit_trigger_function()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_user_id UUID;
BEGIN
    v_user_id := get_current_user_context();

    IF TG_OP = 'INSERT' THEN
        INSERT INTO presave_template_audits (
            template_id,
            organization_id,
            action,
            changed_by,
            old_values,
            new_values
        ) VALUES (
            NEW.id,
            NEW.organization_id,
            'CREATE',
            v_user_id,
            NULL,
            to_jsonb(NEW)
        );
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        INSERT INTO presave_template_audits (
            template_id,
            organization_id,
            action,
            changed_by,
            old_values,
            new_values
        ) VALUES (
            NEW.id,
            NEW.organization_id,
            'UPDATE',
            v_user_id,
            to_jsonb(OLD),
            to_jsonb(NEW)
        );
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        INSERT INTO presave_template_audits (
            template_id,
            organization_id,
            action,
            changed_by,
            old_values,
            new_values
        ) VALUES (
            OLD.id,
            OLD.organization_id,
            'DELETE',
            v_user_id,
            to_jsonb(OLD),
            NULL
        );
        RETURN OLD;
    END IF;

    RETURN NULL;
END;
$$;

CREATE TRIGGER audit_cases AFTER INSERT OR UPDATE OR DELETE ON cases
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_patient_info AFTER INSERT OR UPDATE OR DELETE ON patient_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_message_headers AFTER INSERT OR UPDATE OR DELETE ON message_headers
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_safety_report_identification AFTER INSERT OR UPDATE OR DELETE ON safety_report_identification
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_sender_information AFTER INSERT OR UPDATE OR DELETE ON sender_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_literature_references AFTER INSERT OR UPDATE OR DELETE ON literature_references
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_study_information AFTER INSERT OR UPDATE OR DELETE ON study_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_study_registration_numbers AFTER INSERT OR UPDATE OR DELETE ON study_registration_numbers
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_primary_sources AFTER INSERT OR UPDATE OR DELETE ON primary_sources
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_documents_held_by_sender AFTER INSERT OR UPDATE OR DELETE ON documents_held_by_sender
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_medical_history_episodes AFTER INSERT OR UPDATE OR DELETE ON medical_history_episodes
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_patient_identifiers AFTER INSERT OR UPDATE OR DELETE ON patient_identifiers
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_past_drug_history AFTER INSERT OR UPDATE OR DELETE ON past_drug_history
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_patient_death_information AFTER INSERT OR UPDATE OR DELETE ON patient_death_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_reported_causes_of_death AFTER INSERT OR UPDATE OR DELETE ON reported_causes_of_death
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_autopsy_causes_of_death AFTER INSERT OR UPDATE OR DELETE ON autopsy_causes_of_death
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_parent_information AFTER INSERT OR UPDATE OR DELETE ON parent_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_reactions AFTER INSERT OR UPDATE OR DELETE ON reactions
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_test_results AFTER INSERT OR UPDATE OR DELETE ON test_results
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_drug_information AFTER INSERT OR UPDATE OR DELETE ON drug_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_drug_active_substances AFTER INSERT OR UPDATE OR DELETE ON drug_active_substances
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_dosage_information AFTER INSERT OR UPDATE OR DELETE ON dosage_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_drug_indications AFTER INSERT OR UPDATE OR DELETE ON drug_indications
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_narrative_information AFTER INSERT OR UPDATE OR DELETE ON narrative_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_presave_templates AFTER INSERT OR UPDATE OR DELETE ON presave_templates
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_presave_templates_dedicated AFTER INSERT OR UPDATE OR DELETE ON presave_templates
    FOR EACH ROW EXECUTE FUNCTION presave_template_audit_trigger_function();

CREATE TRIGGER audit_sender_diagnoses AFTER INSERT OR UPDATE OR DELETE ON sender_diagnoses
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_case_summary_information AFTER INSERT OR UPDATE OR DELETE ON case_summary_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_case_versions AFTER INSERT OR UPDATE OR DELETE ON case_versions
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_case_submissions AFTER INSERT OR UPDATE OR DELETE ON case_submissions
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_submission_events AFTER INSERT OR UPDATE OR DELETE ON submission_events
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_submission_dispatch_state AFTER INSERT OR UPDATE OR DELETE ON submission_dispatch_state
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function_with_submission_id();

CREATE TRIGGER audit_submission_idempotency AFTER INSERT OR UPDATE OR DELETE ON submission_idempotency
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_submission_acks AFTER INSERT OR UPDATE OR DELETE ON submission_acks
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_e_signatures AFTER INSERT OR UPDATE OR DELETE ON e_signatures
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

-- Audit triggers for core tables
CREATE TRIGGER audit_organizations AFTER INSERT OR UPDATE OR DELETE ON organizations
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_users AFTER INSERT OR UPDATE OR DELETE ON users
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

-- Phase 1-3 new tables audit triggers
CREATE TRIGGER audit_receiver_information AFTER INSERT OR UPDATE OR DELETE ON receiver_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_other_case_identifiers AFTER INSERT OR UPDATE OR DELETE ON other_case_identifiers
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_linked_report_numbers AFTER INSERT OR UPDATE OR DELETE ON linked_report_numbers
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_parent_medical_history AFTER INSERT OR UPDATE OR DELETE ON parent_medical_history
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_parent_past_drug_history AFTER INSERT OR UPDATE OR DELETE ON parent_past_drug_history
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_drug_recurrence_information AFTER INSERT OR UPDATE OR DELETE ON drug_recurrence_information
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_drug_reaction_assessments AFTER INSERT OR UPDATE OR DELETE ON drug_reaction_assessments
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER audit_relatedness_assessments AFTER INSERT OR UPDATE OR DELETE ON relatedness_assessments
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

-- Terminology tables audit triggers
CREATE TRIGGER audit_meddra_terms AFTER INSERT OR UPDATE OR DELETE ON meddra_terms
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function_with_audit_id();

CREATE TRIGGER audit_whodrug_products AFTER INSERT OR UPDATE OR DELETE ON whodrug_products
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function_with_audit_id();

CREATE TRIGGER audit_iso_countries AFTER INSERT OR UPDATE OR DELETE ON iso_countries
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function_with_audit_id();

CREATE TRIGGER audit_e2b_code_lists AFTER INSERT OR UPDATE OR DELETE ON e2b_code_lists
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function_with_audit_id();

-- ============================================================================
-- Additional updated_at triggers for new tables
-- ============================================================================

CREATE TRIGGER update_organizations_updated_at BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_literature_references_updated_at BEFORE UPDATE ON literature_references
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_documents_held_by_sender_updated_at BEFORE UPDATE ON documents_held_by_sender
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_study_registration_numbers_updated_at BEFORE UPDATE ON study_registration_numbers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_medical_history_episodes_updated_at BEFORE UPDATE ON medical_history_episodes
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_patient_identifiers_updated_at BEFORE UPDATE ON patient_identifiers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_past_drug_history_updated_at BEFORE UPDATE ON past_drug_history
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_reported_causes_of_death_updated_at BEFORE UPDATE ON reported_causes_of_death
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_autopsy_causes_of_death_updated_at BEFORE UPDATE ON autopsy_causes_of_death
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_drug_active_substances_updated_at BEFORE UPDATE ON drug_active_substances
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_drug_indications_updated_at BEFORE UPDATE ON drug_indications
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_sender_diagnoses_updated_at BEFORE UPDATE ON sender_diagnoses
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_case_summary_information_updated_at BEFORE UPDATE ON case_summary_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Phase 1-3 new tables updated_at triggers
CREATE TRIGGER update_receiver_information_updated_at BEFORE UPDATE ON receiver_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_other_case_identifiers_updated_at BEFORE UPDATE ON other_case_identifiers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_linked_report_numbers_updated_at BEFORE UPDATE ON linked_report_numbers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_parent_medical_history_updated_at BEFORE UPDATE ON parent_medical_history
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_parent_past_drug_history_updated_at BEFORE UPDATE ON parent_past_drug_history
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_drug_recurrence_information_updated_at BEFORE UPDATE ON drug_recurrence_information
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_drug_reaction_assessments_updated_at BEFORE UPDATE ON drug_reaction_assessments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_relatedness_assessments_updated_at BEFORE UPDATE ON relatedness_assessments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_presave_templates_updated_at BEFORE UPDATE ON presave_templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Row-Level Security for Organization Isolation (Case-Related Tables)
-- ============================================================================
-- Note: Core table RLS (cases, users, organizations) is in 03-safetydb-schema.sql
-- This section adds RLS for all case-related nested tables

-- Patient Information
ALTER TABLE patient_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE patient_information FORCE ROW LEVEL SECURITY;
CREATE POLICY patient_info_via_case ON patient_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = patient_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = patient_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Drug Information
ALTER TABLE drug_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE drug_information FORCE ROW LEVEL SECURITY;
CREATE POLICY drug_info_via_case ON drug_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = drug_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = drug_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Reactions
ALTER TABLE reactions ENABLE ROW LEVEL SECURITY;
ALTER TABLE reactions FORCE ROW LEVEL SECURITY;
CREATE POLICY reactions_via_case ON reactions
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = reactions.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = reactions.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Test Results
ALTER TABLE test_results ENABLE ROW LEVEL SECURITY;
ALTER TABLE test_results FORCE ROW LEVEL SECURITY;
CREATE POLICY test_results_via_case ON test_results
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = test_results.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = test_results.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Message Headers
ALTER TABLE message_headers ENABLE ROW LEVEL SECURITY;
ALTER TABLE message_headers FORCE ROW LEVEL SECURITY;
CREATE POLICY message_headers_via_case ON message_headers
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = message_headers.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = message_headers.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Safety Report Identification
ALTER TABLE safety_report_identification ENABLE ROW LEVEL SECURITY;
ALTER TABLE safety_report_identification FORCE ROW LEVEL SECURITY;
CREATE POLICY safety_report_id_via_case ON safety_report_identification
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = safety_report_identification.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = safety_report_identification.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Narrative Information
ALTER TABLE narrative_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE narrative_information FORCE ROW LEVEL SECURITY;
CREATE POLICY narrative_info_via_case ON narrative_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = narrative_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = narrative_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Drug Reaction Assessments (via drug_information)
ALTER TABLE drug_reaction_assessments ENABLE ROW LEVEL SECURITY;
ALTER TABLE drug_reaction_assessments FORCE ROW LEVEL SECURITY;
CREATE POLICY drug_reaction_assessments_via_case ON drug_reaction_assessments
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = drug_reaction_assessments.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = drug_reaction_assessments.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Other Case Identifiers
ALTER TABLE other_case_identifiers ENABLE ROW LEVEL SECURITY;
ALTER TABLE other_case_identifiers FORCE ROW LEVEL SECURITY;
CREATE POLICY other_case_ids_via_case ON other_case_identifiers
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = other_case_identifiers.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = other_case_identifiers.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Linked Report Numbers
ALTER TABLE linked_report_numbers ENABLE ROW LEVEL SECURITY;
ALTER TABLE linked_report_numbers FORCE ROW LEVEL SECURITY;
CREATE POLICY linked_reports_via_case ON linked_report_numbers
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = linked_report_numbers.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = linked_report_numbers.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Documents Held by Sender
ALTER TABLE documents_held_by_sender ENABLE ROW LEVEL SECURITY;
ALTER TABLE documents_held_by_sender FORCE ROW LEVEL SECURITY;
CREATE POLICY documents_held_by_sender_via_case ON documents_held_by_sender
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = documents_held_by_sender.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = documents_held_by_sender.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Primary Sources
ALTER TABLE primary_sources ENABLE ROW LEVEL SECURITY;
ALTER TABLE primary_sources FORCE ROW LEVEL SECURITY;
CREATE POLICY primary_sources_via_case ON primary_sources
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = primary_sources.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = primary_sources.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Literature References
ALTER TABLE literature_references ENABLE ROW LEVEL SECURITY;
ALTER TABLE literature_references FORCE ROW LEVEL SECURITY;
CREATE POLICY literature_refs_via_case ON literature_references
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = literature_references.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = literature_references.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Study Information
ALTER TABLE study_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE study_information FORCE ROW LEVEL SECURITY;
CREATE POLICY study_info_via_case ON study_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = study_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = study_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Sender Information
ALTER TABLE sender_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE sender_information FORCE ROW LEVEL SECURITY;
CREATE POLICY sender_info_via_case ON sender_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = sender_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = sender_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Receiver Information
ALTER TABLE receiver_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE receiver_information FORCE ROW LEVEL SECURITY;
CREATE POLICY receiver_info_via_message ON receiver_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = receiver_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM cases c
            WHERE c.id = receiver_information.case_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Study Registration Numbers (via study_information)
ALTER TABLE study_registration_numbers ENABLE ROW LEVEL SECURITY;
ALTER TABLE study_registration_numbers FORCE ROW LEVEL SECURITY;
CREATE POLICY study_reg_numbers_via_case ON study_registration_numbers
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM study_information si
            JOIN cases c ON c.id = si.case_id
            WHERE si.id = study_registration_numbers.study_information_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM study_information si
            JOIN cases c ON c.id = si.case_id
            WHERE si.id = study_registration_numbers.study_information_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Medical History Episodes (via patient_information)
ALTER TABLE medical_history_episodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE medical_history_episodes FORCE ROW LEVEL SECURITY;
CREATE POLICY medical_history_via_case ON medical_history_episodes
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = medical_history_episodes.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = medical_history_episodes.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Patient Identifiers (via patient_information)
ALTER TABLE patient_identifiers ENABLE ROW LEVEL SECURITY;
ALTER TABLE patient_identifiers FORCE ROW LEVEL SECURITY;
CREATE POLICY patient_identifiers_via_case ON patient_identifiers
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = patient_identifiers.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = patient_identifiers.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Past Drug History (via patient_information)
ALTER TABLE past_drug_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE past_drug_history FORCE ROW LEVEL SECURITY;
CREATE POLICY past_drug_history_via_case ON past_drug_history
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = past_drug_history.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = past_drug_history.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Patient Death Information (via patient_information)
ALTER TABLE patient_death_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE patient_death_information FORCE ROW LEVEL SECURITY;
CREATE POLICY patient_death_info_via_case ON patient_death_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = patient_death_information.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = patient_death_information.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Reported Causes of Death (via patient_death_information -> patient_information)
ALTER TABLE reported_causes_of_death ENABLE ROW LEVEL SECURITY;
ALTER TABLE reported_causes_of_death FORCE ROW LEVEL SECURITY;
CREATE POLICY reported_causes_via_case ON reported_causes_of_death
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1
            FROM patient_death_information pdi
            JOIN patient_information pi ON pi.id = pdi.patient_id
            JOIN cases c ON c.id = pi.case_id
            WHERE pdi.id = reported_causes_of_death.death_info_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM patient_death_information pdi
            JOIN patient_information pi ON pi.id = pdi.patient_id
            JOIN cases c ON c.id = pi.case_id
            WHERE pdi.id = reported_causes_of_death.death_info_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Autopsy Causes of Death (via patient_death_information -> patient_information)
ALTER TABLE autopsy_causes_of_death ENABLE ROW LEVEL SECURITY;
ALTER TABLE autopsy_causes_of_death FORCE ROW LEVEL SECURITY;
CREATE POLICY autopsy_causes_via_case ON autopsy_causes_of_death
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1
            FROM patient_death_information pdi
            JOIN patient_information pi ON pi.id = pdi.patient_id
            JOIN cases c ON c.id = pi.case_id
            WHERE pdi.id = autopsy_causes_of_death.death_info_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM patient_death_information pdi
            JOIN patient_information pi ON pi.id = pdi.patient_id
            JOIN cases c ON c.id = pi.case_id
            WHERE pdi.id = autopsy_causes_of_death.death_info_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Parent Information (via patient_information)
ALTER TABLE parent_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE parent_information FORCE ROW LEVEL SECURITY;
CREATE POLICY parent_information_via_case ON parent_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = parent_information.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM patient_information pi
            JOIN cases c ON c.id = pi.case_id
            WHERE pi.id = parent_information.patient_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Drug Active Substances (via drug_information)
ALTER TABLE drug_active_substances ENABLE ROW LEVEL SECURITY;
ALTER TABLE drug_active_substances FORCE ROW LEVEL SECURITY;
CREATE POLICY drug_active_substances_via_case ON drug_active_substances
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = drug_active_substances.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = drug_active_substances.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Dosage Information (via drug_information)
ALTER TABLE dosage_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE dosage_information FORCE ROW LEVEL SECURITY;
CREATE POLICY dosage_information_via_case ON dosage_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = dosage_information.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = dosage_information.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Drug Indications (via drug_information)
ALTER TABLE drug_indications ENABLE ROW LEVEL SECURITY;
ALTER TABLE drug_indications FORCE ROW LEVEL SECURITY;
CREATE POLICY drug_indications_via_case ON drug_indications
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = drug_indications.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = drug_indications.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Sender Diagnoses (via narrative_information)
ALTER TABLE sender_diagnoses ENABLE ROW LEVEL SECURITY;
ALTER TABLE sender_diagnoses FORCE ROW LEVEL SECURITY;
CREATE POLICY sender_diagnoses_via_case ON sender_diagnoses
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM narrative_information ni
            JOIN cases c ON c.id = ni.case_id
            WHERE ni.id = sender_diagnoses.narrative_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM narrative_information ni
            JOIN cases c ON c.id = ni.case_id
            WHERE ni.id = sender_diagnoses.narrative_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Case Summary Information (via narrative_information)
ALTER TABLE case_summary_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_summary_information FORCE ROW LEVEL SECURITY;
CREATE POLICY case_summary_information_via_case ON case_summary_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM narrative_information ni
            JOIN cases c ON c.id = ni.case_id
            WHERE ni.id = case_summary_information.narrative_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM narrative_information ni
            JOIN cases c ON c.id = ni.case_id
            WHERE ni.id = case_summary_information.narrative_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Parent Medical History (via parent_information -> patient_information)
ALTER TABLE parent_medical_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE parent_medical_history FORCE ROW LEVEL SECURITY;
CREATE POLICY parent_medical_history_via_case ON parent_medical_history
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1
            FROM parent_information pi2
            JOIN patient_information pi ON pi.id = pi2.patient_id
            JOIN cases c ON c.id = pi.case_id
            WHERE pi2.id = parent_medical_history.parent_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM parent_information pi2
            JOIN patient_information pi ON pi.id = pi2.patient_id
            JOIN cases c ON c.id = pi.case_id
            WHERE pi2.id = parent_medical_history.parent_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Parent Past Drug History (via parent_information -> patient_information)
ALTER TABLE parent_past_drug_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE parent_past_drug_history FORCE ROW LEVEL SECURITY;
CREATE POLICY parent_past_drug_history_via_case ON parent_past_drug_history
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1
            FROM parent_information pi2
            JOIN patient_information pi ON pi.id = pi2.patient_id
            JOIN cases c ON c.id = pi.case_id
            WHERE pi2.id = parent_past_drug_history.parent_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM parent_information pi2
            JOIN patient_information pi ON pi.id = pi2.patient_id
            JOIN cases c ON c.id = pi.case_id
            WHERE pi2.id = parent_past_drug_history.parent_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Drug Recurrence Information (via drug_information)
ALTER TABLE drug_recurrence_information ENABLE ROW LEVEL SECURITY;
ALTER TABLE drug_recurrence_information FORCE ROW LEVEL SECURITY;
CREATE POLICY drug_recurrence_via_case ON drug_recurrence_information
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = drug_recurrence_information.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM drug_information di
            JOIN cases c ON c.id = di.case_id
            WHERE di.id = drug_recurrence_information.drug_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Relatedness Assessments (via drug_reaction_assessments -> drug_information)
ALTER TABLE relatedness_assessments ENABLE ROW LEVEL SECURITY;
ALTER TABLE relatedness_assessments FORCE ROW LEVEL SECURITY;
CREATE POLICY relatedness_assessments_via_case ON relatedness_assessments
    FOR ALL TO e2br3_app_role
    USING (
        EXISTS (
            SELECT 1
            FROM drug_reaction_assessments dra
            JOIN drug_information di ON di.id = dra.drug_id
            JOIN cases c ON c.id = di.case_id
            WHERE dra.id = relatedness_assessments.drug_reaction_assessment_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM drug_reaction_assessments dra
            JOIN drug_information di ON di.id = dra.drug_id
            JOIN cases c ON c.id = di.case_id
            WHERE dra.id = relatedness_assessments.drug_reaction_assessment_id
            AND (c.organization_id = current_organization_id() OR is_current_user_admin())
        )
    );

-- Terminology Tables
-- Active terminology is global reference data, not tenant-scoped case data.
-- Reads are available to authenticated app roles with terminology permission;
-- writes remain restricted to the system-admin RLS bypass.
ALTER TABLE meddra_terms ENABLE ROW LEVEL SECURITY;
ALTER TABLE meddra_terms FORCE ROW LEVEL SECURITY;
CREATE POLICY meddra_terms_read ON meddra_terms
    FOR SELECT TO e2br3_app_role
    USING (active = true);
CREATE POLICY meddra_terms_insert ON meddra_terms
    FOR INSERT TO e2br3_app_role
    WITH CHECK (is_current_user_admin());
CREATE POLICY meddra_terms_update ON meddra_terms
    FOR UPDATE TO e2br3_app_role
    USING (is_current_user_admin())
    WITH CHECK (is_current_user_admin());
CREATE POLICY meddra_terms_delete ON meddra_terms
    FOR DELETE TO e2br3_app_role
    USING (is_current_user_admin());

ALTER TABLE whodrug_products ENABLE ROW LEVEL SECURITY;
ALTER TABLE whodrug_products FORCE ROW LEVEL SECURITY;
CREATE POLICY whodrug_products_read ON whodrug_products
    FOR SELECT TO e2br3_app_role
    USING (active = true);
CREATE POLICY whodrug_products_insert ON whodrug_products
    FOR INSERT TO e2br3_app_role
    WITH CHECK (is_current_user_admin());
CREATE POLICY whodrug_products_update ON whodrug_products
    FOR UPDATE TO e2br3_app_role
    USING (is_current_user_admin())
    WITH CHECK (is_current_user_admin());
CREATE POLICY whodrug_products_delete ON whodrug_products
    FOR DELETE TO e2br3_app_role
    USING (is_current_user_admin());

ALTER TABLE iso_countries ENABLE ROW LEVEL SECURITY;
ALTER TABLE iso_countries FORCE ROW LEVEL SECURITY;
CREATE POLICY iso_countries_read ON iso_countries
    FOR SELECT TO e2br3_app_role
    USING (is_current_user_admin());
CREATE POLICY iso_countries_insert ON iso_countries
    FOR INSERT TO e2br3_app_role
    WITH CHECK (is_current_user_admin());
CREATE POLICY iso_countries_update ON iso_countries
    FOR UPDATE TO e2br3_app_role
    USING (is_current_user_admin())
    WITH CHECK (is_current_user_admin());
CREATE POLICY iso_countries_delete ON iso_countries
    FOR DELETE TO e2br3_app_role
    USING (is_current_user_admin());

ALTER TABLE e2b_code_lists ENABLE ROW LEVEL SECURITY;
ALTER TABLE e2b_code_lists FORCE ROW LEVEL SECURITY;
CREATE POLICY e2b_code_lists_read ON e2b_code_lists
    FOR SELECT TO e2br3_app_role
    USING (is_current_user_admin());
CREATE POLICY e2b_code_lists_insert ON e2b_code_lists
    FOR INSERT TO e2br3_app_role
    WITH CHECK (is_current_user_admin());
CREATE POLICY e2b_code_lists_update ON e2b_code_lists
    FOR UPDATE TO e2br3_app_role
    USING (is_current_user_admin())
    WITH CHECK (is_current_user_admin());
CREATE POLICY e2b_code_lists_delete ON e2b_code_lists
    FOR DELETE TO e2br3_app_role
    USING (is_current_user_admin());

-- ============================================================================
-- Section C-H dirty flags (for XML merge)
-- ============================================================================
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
  RETURN v_old_business IS DISTINCT FROM v_new_business;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_c() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_c = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_c_from_study_info() RETURNS trigger AS $$
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
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_d() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_d = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_d_from_patient() RETURNS trigger AS $$
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
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_d_from_parent() RETURNS trigger AS $$
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
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_d_from_death_info() RETURNS trigger AS $$
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
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_e() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_e = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_f() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_f = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_g() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_g = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_g_from_drug() RETURNS trigger AS $$
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
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_h() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND NOT has_business_change_on_update(OLD, NEW) THEN
    RETURN NEW;
  END IF;
  UPDATE cases SET dirty_h = TRUE WHERE id = COALESCE(NEW.case_id, OLD.case_id);
  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION mark_case_dirty_h_from_narrative() RETURNS trigger AS $$
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
$$ LANGUAGE plpgsql;

-- Section C tables
CREATE TRIGGER aa_dirty_c_safety_report_identification
  AFTER INSERT OR UPDATE OR DELETE ON safety_report_identification
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c();
CREATE TRIGGER aa_dirty_c_sender_information
  AFTER INSERT OR UPDATE OR DELETE ON sender_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c();
CREATE TRIGGER aa_dirty_c_literature_references
  AFTER INSERT OR UPDATE OR DELETE ON literature_references
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c();
CREATE TRIGGER aa_dirty_c_documents_held_by_sender
  AFTER INSERT OR UPDATE OR DELETE ON documents_held_by_sender
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c();
CREATE TRIGGER aa_dirty_c_study_information
  AFTER INSERT OR UPDATE OR DELETE ON study_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c();
CREATE TRIGGER aa_dirty_c_study_registration_numbers
  AFTER INSERT OR UPDATE OR DELETE ON study_registration_numbers
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c_from_study_info();
CREATE TRIGGER aa_dirty_c_primary_sources
  AFTER INSERT OR UPDATE OR DELETE ON primary_sources
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c();
CREATE TRIGGER aa_dirty_c_receiver_information
  AFTER INSERT OR UPDATE OR DELETE ON receiver_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c();
CREATE TRIGGER aa_dirty_c_other_case_identifiers
  AFTER INSERT OR UPDATE OR DELETE ON other_case_identifiers
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c();
CREATE TRIGGER aa_dirty_c_linked_report_numbers
  AFTER INSERT OR UPDATE OR DELETE ON linked_report_numbers
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c();

-- Section D tables
CREATE TRIGGER aa_dirty_d_patient_information
  AFTER INSERT OR UPDATE OR DELETE ON patient_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d();
CREATE TRIGGER aa_dirty_d_patient_identifiers
  AFTER INSERT OR UPDATE OR DELETE ON patient_identifiers
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient();
CREATE TRIGGER aa_dirty_d_medical_history_episodes
  AFTER INSERT OR UPDATE OR DELETE ON medical_history_episodes
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient();
CREATE TRIGGER aa_dirty_d_past_drug_history
  AFTER INSERT OR UPDATE OR DELETE ON past_drug_history
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient();
CREATE TRIGGER aa_dirty_d_patient_death_information
  AFTER INSERT OR UPDATE OR DELETE ON patient_death_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient();
CREATE TRIGGER aa_dirty_d_reported_causes_of_death
  AFTER INSERT OR UPDATE OR DELETE ON reported_causes_of_death
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_death_info();
CREATE TRIGGER aa_dirty_d_autopsy_causes_of_death
  AFTER INSERT OR UPDATE OR DELETE ON autopsy_causes_of_death
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_death_info();
CREATE TRIGGER aa_dirty_d_parent_information
  AFTER INSERT OR UPDATE OR DELETE ON parent_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient();
CREATE TRIGGER aa_dirty_d_parent_medical_history
  AFTER INSERT OR UPDATE OR DELETE ON parent_medical_history
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_parent();
CREATE TRIGGER aa_dirty_d_parent_past_drug_history
  AFTER INSERT OR UPDATE OR DELETE ON parent_past_drug_history
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_parent();

-- Section E tables
CREATE TRIGGER aa_dirty_e_reactions
  AFTER INSERT OR UPDATE OR DELETE ON reactions
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_e();

-- Section F tables
CREATE TRIGGER aa_dirty_f_test_results
  AFTER INSERT OR UPDATE OR DELETE ON test_results
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_f();

-- Section G tables
CREATE TRIGGER aa_dirty_g_drug_information
  AFTER INSERT OR UPDATE OR DELETE ON drug_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g();
CREATE TRIGGER aa_dirty_g_drug_active_substances
  AFTER INSERT OR UPDATE OR DELETE ON drug_active_substances
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug();
CREATE TRIGGER aa_dirty_g_dosage_information
  AFTER INSERT OR UPDATE OR DELETE ON dosage_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug();
CREATE TRIGGER aa_dirty_g_drug_indications
  AFTER INSERT OR UPDATE OR DELETE ON drug_indications
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug();
CREATE TRIGGER aa_dirty_g_drug_device_characteristics
  AFTER INSERT OR UPDATE OR DELETE ON drug_device_characteristics
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug();
CREATE TRIGGER aa_dirty_g_drug_recurrence_information
  AFTER INSERT OR UPDATE OR DELETE ON drug_recurrence_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug();
CREATE TRIGGER aa_dirty_g_drug_reaction_assessments
  AFTER INSERT OR UPDATE OR DELETE ON drug_reaction_assessments
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug();
CREATE TRIGGER aa_dirty_g_relatedness_assessments
  AFTER INSERT OR UPDATE OR DELETE ON relatedness_assessments
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug();

-- Section H tables
CREATE TRIGGER aa_dirty_h_narrative_information
  AFTER INSERT OR UPDATE OR DELETE ON narrative_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_h();
CREATE TRIGGER aa_dirty_h_sender_diagnoses
  AFTER INSERT OR UPDATE OR DELETE ON sender_diagnoses
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_h_from_narrative();
CREATE TRIGGER aa_dirty_h_case_summary_information
  AFTER INSERT OR UPDATE OR DELETE ON case_summary_information
  FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_h_from_narrative();

-- Ensure application role has access to all tables created after initial grants.
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO e2br3_app_role;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO e2br3_app_role;
