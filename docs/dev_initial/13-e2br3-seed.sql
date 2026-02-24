-- Seed data for E2B(R3) schema (dev only).
DO $$
DECLARE
    v_org_id UUID := '00000000-0000-0000-0000-000000000001';
    v_user_id UUID := '11111111-1111-1111-1111-111111111111';
    v_case_id UUID := '22222222-2222-2222-2222-222222222222';
    v_case_version_id UUID := '22222222-2222-2222-2222-222222222223';
    v_message_header_id UUID := '33333333-3333-3333-3333-333333333333';
    v_safety_ident_id UUID := '44444444-4444-4444-4444-444444444444';
    v_sender_info_id UUID := '55555555-5555-5555-5555-555555555555';
    v_study_info_id UUID := '66666666-6666-6666-6666-666666666666';
    v_study_reg_id UUID := '66666666-6666-6666-6666-666666666667';
    v_primary_source_id UUID := '77777777-7777-7777-7777-777777777777';
    v_patient_id UUID := '88888888-8888-8888-8888-888888888888';
    v_med_history_id UUID := '88888888-8888-8888-8888-888888888889';
    v_past_drug_id UUID := '88888888-8888-8888-8888-888888888890';
    v_death_info_id UUID := '88888888-8888-8888-8888-888888888891';
    v_reported_death_id UUID := '88888888-8888-8888-8888-888888888892';
    v_autopsy_death_id UUID := '88888888-8888-8888-8888-888888888893';
    v_parent_info_id UUID := '88888888-8888-8888-8888-888888888894';
    v_reaction_id UUID := '99999999-9999-9999-9999-999999999999';
    v_test_result_id UUID := '99999999-9999-9999-9999-999999999998';
    v_drug_info_id UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    v_drug_substance_id UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaab';
    v_dosage_info_id UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaac';
    v_drug_indication_id UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaad';
    v_narrative_id UUID := 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    v_sender_diag_id UUID := 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbc';
    v_case_summary_id UUID := 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbd';
    v_literature_ref_id UUID := 'cccccccc-cccc-cccc-cccc-cccccccccccc';
BEGIN
    -- Use system user for initial inserts (demo user doesn't exist yet)
    PERFORM set_config('app.current_user_id', '00000000-0000-0000-0000-000000000001', true);
    -- Required by RLS orgs_modify policy during bootstrap inserts.
    PERFORM set_config('app.current_user_role', 'admin', true);

    -- Insert demo organization (created by system user)
    INSERT INTO organizations (id, name, org_type, address, city, state, postcode, country_code, contact_email, contact_phone, active, created_by, created_at, updated_at)
    VALUES (v_org_id, 'Demo Organization', 'internal', '123 Demo St', 'Metropolis', 'CA', '12345', 'US', 'demo@example.com', '555-1234', true, '00000000-0000-0000-0000-000000000001'::UUID, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    -- Insert demo user
    INSERT INTO users (id, organization_id, email, username, role, active, must_change_password, created_by, created_at, updated_at)
    VALUES (v_user_id, v_org_id, 'demo.user@example.com', 'demo_user', 'admin', true, false, '00000000-0000-0000-0000-000000000001'::UUID, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    -- Switch context to demo user for remaining demo data
    PERFORM set_config('app.current_user_id', v_user_id::text, true);

    INSERT INTO cases (id, organization_id, safety_report_id, version, status, created_by, updated_by, submitted_by, submitted_at, created_at, updated_at)
    VALUES (v_case_id, v_org_id, 'SR-001', 1, 'draft', v_user_id, v_user_id, v_user_id, NOW(), NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO case_versions (id, case_id, version, snapshot, changed_by, change_reason, created_at)
    VALUES (v_case_version_id, v_case_id, 1, '{}'::JSONB, v_user_id, 'Initial import', NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO message_headers (id, case_id, batch_number, batch_sender_identifier, message_type, message_format_version, message_format_release, message_number, message_sender_identifier, message_receiver_identifier, message_date_format, message_date, created_by, created_at, updated_at)
    VALUES (v_message_header_id, v_case_id, 'B-001', 'SENDER-1', 'ichicsr', '2.1', '2.0', 'MSG-001', 'ORG-SENDER', 'ORG-RECEIVER', '204', '20240101120000', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO safety_report_identification (id, case_id, transmission_date, report_type, date_first_received_from_source, date_of_most_recent_information, fulfil_expedited_criteria, receiver_organization, created_by, created_at, updated_at)
    VALUES (v_safety_ident_id, v_case_id, CURRENT_DATE, '1', CURRENT_DATE, CURRENT_DATE, TRUE, 'Demo Receiver', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO sender_information (id, case_id, sender_type, organization_name, created_by, created_at, updated_at)
    VALUES (v_sender_info_id, v_case_id, '1', 'Demo Sender Org', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO literature_references (id, case_id, reference_text, sequence_number, created_by, created_at, updated_at)
    VALUES (v_literature_ref_id, v_case_id, 'Sample literature reference', 1, v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO study_information (id, case_id, study_name, sponsor_study_number, study_type_reaction, created_by, created_at, updated_at)
    VALUES (v_study_info_id, v_case_id, 'Study A', 'SSN-1', '01', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO study_registration_numbers (id, study_information_id, registration_number, country_code, sequence_number, created_by, created_at, updated_at)
    VALUES (v_study_reg_id, v_study_info_id, 'REG-001', 'US', 1, v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO primary_sources (id, case_id, sequence_number, reporter_given_name, reporter_family_name, country_code, qualification, primary_source_regulatory, created_by, created_at, updated_at)
    VALUES (v_primary_source_id, v_case_id, 1, 'Jane', 'Doe', 'US', '1', '1', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO patient_information (id, case_id, patient_initials, sex, created_by, created_at, updated_at)
    VALUES (v_patient_id, v_case_id, 'JD', '1', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO medical_history_episodes (id, patient_id, sequence_number, meddra_version, meddra_code, start_date, created_by, created_at, updated_at)
    VALUES (v_med_history_id, v_patient_id, 1, '26.0', '12345678', CURRENT_DATE - 365, v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO past_drug_history (
        id,
        patient_id,
        sequence_number,
        drug_name,
        start_date,
        end_date,
        indication_meddra_version,
        indication_meddra_code,
        reaction_meddra_version,
        reaction_meddra_code,
        created_by,
        created_at,
        updated_at
    )
    VALUES (
        v_past_drug_id,
        v_patient_id,
        1,
        'Historical Drug',
        CURRENT_DATE - 400,
        CURRENT_DATE - 350,
        '26.0',
        '11223344',
        '26.0',
        '55667788',
        v_user_id,
        NOW(),
        NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO patient_death_information (id, patient_id, date_of_death, autopsy_performed, created_by, created_at, updated_at)
    VALUES (v_death_info_id, v_patient_id, CURRENT_DATE - 1, FALSE, v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO reported_causes_of_death (id, death_info_id, sequence_number, meddra_version, meddra_code, created_by, created_at, updated_at)
    VALUES (v_reported_death_id, v_death_info_id, 1, '26.0', '87654321', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO autopsy_causes_of_death (id, death_info_id, sequence_number, meddra_version, meddra_code, created_by, created_at, updated_at)
    VALUES (v_autopsy_death_id, v_death_info_id, 1, '26.0', '87654322', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO parent_information (
        id,
        patient_id,
        parent_identification,
        parent_age,
        parent_age_unit,
        last_menstrual_period_date,
        weight_kg,
        height_cm,
        sex,
        medical_history_text,
        created_by,
        created_at,
        updated_at
    )
    VALUES (
        v_parent_info_id,
        v_patient_id,
        'Parent-1',
        30,
        'a',
        CURRENT_DATE - 100,
        55,
        165,
        '2',
        'Parent history text',
        v_user_id,
        NOW(),
        NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO reactions (id, case_id, sequence_number, primary_source_reaction, serious, outcome, created_by, created_at, updated_at)
    VALUES (v_reaction_id, v_case_id, 1, 'Headache', FALSE, '0', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO test_results (id, case_id, sequence_number, test_date, test_name, created_by, created_at, updated_at)
    VALUES (v_test_result_id, v_case_id, 1, CURRENT_DATE, 'Blood Test', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO drug_information (id, case_id, sequence_number, drug_characterization, medicinal_product, action_taken, created_by, created_at, updated_at)
    VALUES (v_drug_info_id, v_case_id, 1, '1', 'Demo Drug', '1', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO drug_active_substances (id, drug_id, sequence_number, substance_name, strength_value, strength_unit, created_by, created_at, updated_at)
    VALUES (v_drug_substance_id, v_drug_info_id, 1, 'Substance A', 10.0, 'mg', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO dosage_information (id, drug_id, sequence_number, dose_value, dose_unit, frequency_value, frequency_unit, created_by, created_at, updated_at)
    VALUES (v_dosage_info_id, v_drug_info_id, 1, 1.0, 'tab', 1.0, 'day', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO drug_indications (id, drug_id, sequence_number, indication_text, indication_meddra_version, indication_meddra_code, created_by, created_at, updated_at)
    VALUES (v_drug_indication_id, v_drug_info_id, 1, 'Indication text', '26.0', '135790', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO narrative_information (id, case_id, case_narrative, reporter_comments, sender_comments, created_by, created_at, updated_at)
    VALUES (v_narrative_id, v_case_id, 'Case narrative text', 'Reporter comment', 'Sender comment', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO sender_diagnoses (id, narrative_id, sequence_number, diagnosis_meddra_version, diagnosis_meddra_code, created_by, created_at, updated_at)
    VALUES (v_sender_diag_id, v_narrative_id, 1, '26.0', '246810', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO case_summary_information (id, narrative_id, sequence_number, summary_type, language_code, summary_text, created_by, created_at, updated_at)
    VALUES (v_case_summary_id, v_narrative_id, 1, '01', 'en', 'Case summary text', v_user_id, NOW(), NOW())
    ON CONFLICT (id) DO NOTHING;

    -- ------------------------------------------------------------------------
    -- DEV-ONLY MedDRA stub set (non-production)
    -- ------------------------------------------------------------------------
    INSERT INTO meddra_terms (code, term, level, version, language, active)
    VALUES
        ('10000001', 'Stub MedDRA Term 0001', 'PT', '26.0', 'en', true),
        ('10000002', 'Stub MedDRA Term 0002', 'PT', '26.0', 'en', true),
        ('10000003', 'Stub MedDRA Term 0003', 'LLT', '26.0', 'en', true),
        ('10000004', 'Stub MedDRA Term 0004', 'PT', '26.0', 'en', true),
        ('10000005', 'Stub MedDRA Term 0005', 'LLT', '26.0', 'en', true),
        ('10000006', 'Stub MedDRA Term 0006', 'HLT', '26.0', 'en', true),
        ('10000007', 'Stub MedDRA Term 0007', 'HLGT', '26.0', 'en', true),
        ('10000008', 'Stub MedDRA Term 0008', 'SOC', '26.0', 'en', true),
        ('10000009', 'Stub MedDRA Term 0009', 'PT', '26.0', 'en', true),
        ('10000010', 'Stub MedDRA Term 0010', 'PT', '26.0', 'en', true),
        ('10000011', 'Stub MedDRA Term 0011', 'LLT', '26.0', 'en', true),
        ('10000012', 'Stub MedDRA Term 0012', 'LLT', '26.0', 'en', true),
        ('10000013', 'Stub MedDRA Term 0013', 'PT', '26.0', 'en', true),
        ('10000014', 'Stub MedDRA Term 0014', 'PT', '26.0', 'en', true),
        ('10000015', 'Stub MedDRA Term 0015', 'HLT', '26.0', 'en', true),
        ('10000016', 'Stub MedDRA Term 0016', 'HLGT', '26.0', 'en', true),
        ('10000017', 'Stub MedDRA Term 0017', 'SOC', '26.0', 'en', true),
        ('10000018', 'Stub MedDRA Term 0018', 'PT', '26.0', 'en', true),
        ('10000019', 'Stub MedDRA Term 0019', 'PT', '26.0', 'en', true),
        ('10000020', 'Stub MedDRA Term 0020', 'LLT', '26.0', 'en', true)
    ON CONFLICT (code, version, language) DO NOTHING;

    -- ------------------------------------------------------------------------
    -- DEV-ONLY WHODrug stub set (non-production)
    -- ------------------------------------------------------------------------
    INSERT INTO whodrug_products (code, drug_name, atc_code, version, language, active)
    VALUES
        ('WTEST1', 'Stub WHODrug Product 1', 'A01AA01', '2024.1', 'en', true),
        ('WTEST2', 'Stub WHODrug Product 2', 'A01AA02', '2024.1', 'en', true),
        ('WTEST3', 'Stub WHODrug Product 3', 'A01AA03', '2024.1', 'en', true)
    ON CONFLICT (code, version, language) DO NOTHING;
END;
$$;
