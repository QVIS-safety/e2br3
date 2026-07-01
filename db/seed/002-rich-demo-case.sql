-- Rich dev/demo case with most subresources populated.
-- Safe to re-run: all inserts are idempotent on fixed UUIDs.
DO $$
DECLARE
    v_org_id UUID := '00000000-0000-0000-0000-000000000001';
    v_user_id UUID := '11111111-1111-1111-1111-111111111111';
    v_case_id UUID := 'd2000000-0000-0000-0000-000000000001';
    v_case_version_id UUID := 'd2000000-0000-0000-0000-000000000002';
    v_message_header_id UUID := 'd2000000-0000-0000-0000-000000000003';
    v_receiver_info_id UUID := 'd2000000-0000-0000-0000-000000000004';
    v_safety_ident_id UUID := 'd2000000-0000-0000-0000-000000000005';
    v_sender_info_id UUID := 'd2000000-0000-0000-0000-000000000006';
    v_study_info_id UUID := 'd2000000-0000-0000-0000-000000000007';
    v_study_reg_id UUID := 'd2000000-0000-0000-0000-000000000008';
    v_primary_source_id UUID := 'd2000000-0000-0000-0000-000000000009';
    v_patient_id UUID := 'd2000000-0000-0000-0000-00000000000a';
    v_patient_identifier_id UUID := 'd2000000-0000-0000-0000-00000000000b';
    v_med_history_id UUID := 'd2000000-0000-0000-0000-00000000000c';
    v_past_drug_id UUID := 'd2000000-0000-0000-0000-00000000000d';
    v_death_info_id UUID := 'd2000000-0000-0000-0000-00000000000e';
    v_reported_death_id UUID := 'd2000000-0000-0000-0000-00000000000f';
    v_autopsy_death_id UUID := 'd2000000-0000-0000-0000-000000000010';
    v_parent_info_id UUID := 'd2000000-0000-0000-0000-000000000011';
    v_parent_med_history_id UUID := 'd2000000-0000-0000-0000-000000000012';
    v_parent_past_drug_id UUID := 'd2000000-0000-0000-0000-000000000013';
    v_reaction_id UUID := 'd2000000-0000-0000-0000-000000000014';
    v_test_result_id UUID := 'd2000000-0000-0000-0000-000000000015';
    v_drug_info_id UUID := 'd2000000-0000-0000-0000-000000000016';
    v_drug_substance_id UUID := 'd2000000-0000-0000-0000-000000000017';
    v_dosage_info_id UUID := 'd2000000-0000-0000-0000-000000000018';
    v_drug_indication_id UUID := 'd2000000-0000-0000-0000-000000000019';
    v_drug_assessment_id UUID := 'd2000000-0000-0000-0000-00000000001a';
    v_relatedness_id UUID := 'd2000000-0000-0000-0000-00000000001b';
    v_recurrence_id UUID := 'd2000000-0000-0000-0000-00000000001c';
    v_device_characteristic_id UUID := 'd2000000-0000-0000-0000-00000000001d';
    v_narrative_id UUID := 'd2000000-0000-0000-0000-00000000001e';
    v_sender_diag_id UUID := 'd2000000-0000-0000-0000-00000000001f';
    v_case_summary_id UUID := 'd2000000-0000-0000-0000-000000000020';
    v_literature_ref_id UUID := 'd2000000-0000-0000-0000-000000000021';
    v_other_case_identifier_id UUID := 'd2000000-0000-0000-0000-000000000022';
    v_linked_report_id UUID := 'd2000000-0000-0000-0000-000000000023';
    v_source_document_id UUID := 'd2000000-0000-0000-0000-000000000024';
BEGIN
    PERFORM set_config('app.current_user_id', v_user_id::text, true);
    PERFORM set_config('app.current_user_role', 'system_admin', true);

    INSERT INTO cases (
        id, organization_id, dg_prd_key, status,
        review_receivers_json, workflow_routes_json,
        report_year,
        created_by, updated_by, submitted_by, submitted_at,
        dirty_c, dirty_d, dirty_e, dirty_f, dirty_g, dirty_h, created_at, updated_at
    )
    VALUES (
        v_case_id, v_org_id, 'PRD-DEMO-ALPHA', 'draft',
        '["qa.lead@example.com","pv.manager@example.com"]',
        '[{"step":"draft","assignee":"demo.cro.admin@example.com"},{"step":"review","assignee":"qa.lead@example.com"}]',
        '2026',
        v_user_id, v_user_id, NULL, NULL,
        false, false, false, false, false, false, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO source_documents (
        id, case_id, source_document_name, source_document_media_type,
        sequence_number, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_source_document_id, v_case_id, 'rich-demo-source.pdf', 'application/pdf',
        1, v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO case_versions (id, case_id, version, snapshot, changed_by, change_reason, created_at)
    VALUES (
        v_case_version_id,
        v_case_id,
        1,
        jsonb_build_object(
            'safety_report_id', 'DEMO-RICH-2026-0001',
            'note', 'Rich demo case bootstrap snapshot'
        ),
        v_user_id,
        'Rich demo case seed',
        NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO message_headers (
        id, case_id, batch_number, batch_sender_identifier, batch_receiver_identifier,
        batch_transmission_date, message_type, message_format_version,
        message_format_release, message_number, message_sender_identifier,
        message_receiver_identifier, message_date_format, message_date,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_message_header_id, v_case_id, 'BATCH-20260410-01', 'QVIS-KR', 'CDER',
        TIMESTAMP '2026-04-10 10:30:00', 'ichicsr', '2.1', '2.0', 'MSG-DEMO-RICH-2026-0001',
        'QVISPV', 'CDER', '204', '20260410103000',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO receiver_information (
        id, case_id, receiver_type, organization_name, department, street_address,
        city, state_province, postcode, country_code, telephone, fax, email,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_receiver_info_id, v_case_id, '2', 'U.S. FDA', 'CDER Pharmacovigilance',
        '10903 New Hampshire Ave', 'Silver Spring', 'MD', '20993', 'US',
        '+1-301-555-0100', '+1-301-555-0101', 'cder-safety@example.gov',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO safety_report_identification (
        id, case_id, safety_report_id, version, transmission_date, report_type, date_first_received_from_source,
        date_of_most_recent_information, fulfil_expedited_criteria,
        local_criteria_report_type, combination_product_report_indicator,
        worldwide_unique_id, first_sender_type, additional_documents_available,
        receiver_organization, other_case_identifiers_exist,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_safety_ident_id, v_case_id, 'DEMO-RICH-2026-0001', 1, DATE '2026-04-10', '1', DATE '2026-04-07',
        DATE '2026-04-09', true, '1', true,
        'KR-QVIS-2026-0001', '1', true,
        'Food and Drug Administration', true,
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO sender_information (
        id, case_id, sender_type, organization_name, department, street_address, city, state,
        postcode, country_code, person_title, person_given_name, person_middle_name,
        person_family_name, telephone, fax, email, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_sender_info_id, v_case_id, '1', 'QVIS Safety Korea', 'Drug Safety Operations',
        '15 Teheran-ro', 'Seoul', 'Seoul', '06130', 'KR', 'Dr.', 'Minji', 'H.',
        'Lee', '+82-2-555-1000', '+82-2-555-1001', 'minji.lee@qvis.example.com',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO other_case_identifiers (
        id, case_id, sequence_number, source_of_identifier, case_identifier,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_other_case_identifier_id, v_case_id, 1, 'Legacy Safety DB', 'LEGACY-2026-4451',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO linked_report_numbers (
        id, case_id, sequence_number, linked_report_number,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_linked_report_id, v_case_id, 1, 'LINK-ICSR-2026-0099',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO literature_references (
        id, case_id, reference_text, sequence_number, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_literature_ref_id, v_case_id,
        'Lee M, Park J. Case report of severe headache associated with Demozumab. J Pharmacovigilance. 2026;14(2):100-104.',
        1, v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO study_information (
        id, case_id, study_name, sponsor_study_number, study_type_reaction, study_type_reaction_kr1,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_study_info_id, v_case_id, 'DEMO-ALPHA Extension Study', 'QVIS-DA-2026-01', '3', '2',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO study_registration_numbers (
        id, study_information_id, registration_number, country_code, sequence_number,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_study_reg_id, v_study_info_id, 'NCT20260001', 'US', 1,
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO primary_sources (
        id, case_id, sequence_number, reporter_title, reporter_given_name, reporter_middle_name,
        reporter_family_name, organization, department, street, city, state, postcode,
        telephone, country_code, email, qualification, qualification_kr1,
        primary_source_regulatory, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_primary_source_id, v_case_id, 1, 'Dr.', 'Ariana', 'S.', 'Kim',
        'Seoul General Hospital', 'Neurology', '101 Medical Ave', 'Seoul', 'Seoul', '04524',
        '+82-2-777-2020', 'KR', 'ariana.kim@examplehospital.kr', '1', '1',
        '1', v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO patient_information (
        id, case_id, patient_initials, patient_given_name, patient_family_name, birth_date,
        age_at_time_of_onset, age_unit, gestation_period, gestation_period_unit, age_group,
        weight_kg, height_cm, sex, race_code, ethnicity_code, last_menstrual_period_date,
        medical_history_text, concomitant_therapy, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_patient_id, v_case_id, 'HJ', 'Hana', 'Jung', DATE '1988-05-14',
        37, '801', 0, '804', '5',
        61.5, 168.0, '2', 'C41260', 'C41222', DATE '2026-03-20',
        'History of migraine, seasonal allergic rhinitis, and intermittent hypertension.',
        true, v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO patient_identifiers (
        id, patient_id, sequence_number, identifier_type_code, identifier_value,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_patient_identifier_id, v_patient_id, 1, 'M', 'MRN-2026-445188',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO medical_history_episodes (
        id, patient_id, sequence_number, meddra_version, meddra_code, start_date, continuing,
        end_date, comments, family_history, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_med_history_id, v_patient_id, 1, '27.1', '10027310', DATE '2018-01-01', true,
        NULL, 'Migraine controlled with intermittent therapy.', false,
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO past_drug_history (
        id, patient_id, sequence_number, drug_name, mpid, mpid_version, phpid, phpid_version,
        start_date, end_date, indication_meddra_version, indication_meddra_code,
        reaction_meddra_version, reaction_meddra_code, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_past_drug_id, v_patient_id, 1, 'Sumatriptan', 'MPID-7788', '2026.1', 'PHPID-7788', '2026.1',
        DATE '2023-01-01', DATE '2025-12-30', '27.1', '10027310',
        '27.1', '10020772', v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO patient_death_information (
        id, patient_id, date_of_death, autopsy_performed,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_death_info_id, v_patient_id, DATE '2026-04-09', false,
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO reported_causes_of_death (
        id, death_info_id, sequence_number, meddra_version, meddra_code, comments,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_reported_death_id, v_death_info_id, 1, '27.1', '10011906', 'Initial reported cause from hospital record.',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO autopsy_causes_of_death (
        id, death_info_id, sequence_number, meddra_version, meddra_code, comments,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_autopsy_death_id, v_death_info_id, 1, '27.1', '10011906', 'No autopsy performed; placeholder demo row.',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO parent_information (
        id, patient_id, parent_identification, parent_birth_date, parent_age, parent_age_unit,
        last_menstrual_period_date, weight_kg, height_cm, sex, medical_history_text,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_parent_info_id, v_patient_id, 'MOTHER-001', DATE '1963-02-11', 63, '801',
        DATE '1988-04-21', 58.0, 162.0, '2', 'History of hypertension and hypothyroidism.',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO parent_medical_history (
        id, parent_id, sequence_number, meddra_version, meddra_code, start_date, continuing,
        comments, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_parent_med_history_id, v_parent_info_id, 1, '27.1', '10020772', DATE '2010-01-01', true,
        'Parent hypertension controlled with medication.',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO parent_past_drug_history (
        id, parent_id, sequence_number, drug_name, start_date, end_date,
        indication_meddra_version, indication_meddra_code, reaction_meddra_version,
        reaction_meddra_code, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_parent_past_drug_id, v_parent_info_id, 1, 'Levothyroxine', DATE '2015-01-01', DATE '2026-04-01',
        '27.1', '10021004', '27.1', '10000081',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO reactions (
        id, case_id, sequence_number, primary_source_reaction, primary_source_reaction_translation,
        reaction_language, reaction_meddra_version, reaction_meddra_code, term_highlighted,
        serious, criteria_death, criteria_life_threatening, criteria_hospitalization,
        criteria_disabling, criteria_congenital_anomaly, criteria_other_medically_important,
        required_intervention, start_date, end_date, duration_value, duration_unit, outcome,
        medical_confirmation, country_code, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_reaction_id, v_case_id, 1, 'Severe headache', 'Severe headache', 'en',
        '27.1', '10019211', true,
        true, true, false, true,
        false, false, true,
        true, DATE '2026-04-08', DATE '2026-04-09', 2, '804', '5',
        true, 'KR', v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO test_results (
        id, case_id, sequence_number, test_date, test_name, test_meddra_version, test_meddra_code,
        test_result_code, test_result_value, test_result_unit, result_unstructured,
        normal_low_value, normal_high_value, comments, more_info_available,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_test_result_id, v_case_id, 1, DATE '2026-04-08', 'Alanine aminotransferase',
        '27.1', '10001927', 'H', '86', 'U/L',
        'ALT elevated above reference interval.', '7', '56',
        'Repeat liver function test recommended.', true,
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO drug_information (
        id, case_id, sequence_number, drug_characterization, medicinal_product, mpid,
        mpid_version, phpid, phpid_version, investigational_product_blinded,
        obtain_drug_country, brand_name, drug_generic_name, drug_authorization_number,
        manufacturer_name, manufacturer_country, batch_lot_number,
        cumulative_dose_first_reaction_value, cumulative_dose_first_reaction_unit,
        gestation_period_exposure_value, gestation_period_exposure_unit, dosage_text,
        action_taken, rechallenge, parent_dosage_text, fda_additional_info_coded, drug_additional_info_codes_json,
        fda_specialized_product_category, fda_device_info_json, drug_additional_information,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_drug_info_id, v_case_id, 1, '1', 'DEMOZUMAB 50 mg tablet', 'MPID-DEMO-50', '2026.1',
        'PHPID-DEMO-50', '2026.1', false,
        'KR', 'Demozumab', 'demozumab', 'KR-DA-7781',
        'QVIS Biopharma', 'KR', 'LOT-2026-A1',
        100.0, 'mg', 0, '804', '50 mg orally once daily for 2 days',
        '2', '1', 'Parent route not applicable; oral tablet.',
        '1', '["3","7"]'::jsonb,
        'combination_product',
        '{"deviceModel":"DV-1000","deviceBrand":"QVIS Smart Pen","operatorType":"patient"}'::jsonb,
        'Suspect combination product used with connected dosing accessory.',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO drug_active_substances (
        id, drug_id, sequence_number, substance_name, substance_termid, substance_termid_version,
        strength_value, strength_unit, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_drug_substance_id, v_drug_info_id, 1, 'demozumab', 'SUB-DEMO-50', '2026.1',
        50.0, 'mg', v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO dosage_information (
        id, drug_id, sequence_number, dose_value, dose_unit, number_of_units,
        frequency_value, frequency_unit, first_administration_date, first_administration_time,
        last_administration_date, last_administration_time, duration_value, duration_unit,
        batch_lot_number, dosage_text, dose_form, dose_form_termid, dose_form_termid_version,
        route_of_administration, route_termid, route_termid_version, parent_route, parent_route_termid,
        parent_route_termid_version, continuing, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_dosage_info_id, v_drug_info_id, 1, 50.0, 'mg', 1,
        1.0, 'day', DATE '2026-04-07', TIME '08:00:00',
        DATE '2026-04-08', TIME '08:00:00', 2, 'day',
        'LOT-2026-A1', '50 mg tablet once daily', 'Tablet', 'TAB', '27.1',
        '001', '20000092', '27.1', 'Oral', '20000092',
        '27.1', false, v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO drug_indications (
        id, drug_id, sequence_number, indication_text, indication_meddra_version, indication_meddra_code,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_drug_indication_id, v_drug_info_id, 1, 'Migraine prophylaxis', '27.1', '10027310',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO drug_reaction_assessments (
        id, drug_id, reaction_id, administration_start_interval_value, administration_start_interval_unit,
        last_dose_interval_value, last_dose_interval_unit, recurrence_action,
        recurrence_meddra_version, recurrence_meddra_code, reaction_recurred,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_drug_assessment_id, v_drug_info_id, v_reaction_id, 1, '801',
        6, '804', '1', '27.1', '10019211', '1',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO relatedness_assessments (
        id, drug_reaction_assessment_id, sequence_number, source_of_assessment,
        method_of_assessment, result_of_assessment, result_of_assessment_kr2,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_relatedness_id, v_drug_assessment_id, 1, '1', '1', '2', '2',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO drug_recurrence_information (
        id, drug_id, sequence_number, rechallenge_action, reaction_meddra_version,
        reaction_meddra_code, reaction_recurred, created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_recurrence_id, v_drug_info_id, 1, '1', '27.1', '10019211', '1',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO drug_device_characteristics (
        id, drug_id, sequence_number, code, code_system, code_display_name,
        value_type, value_value, value_code, value_code_system, value_display_name,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_device_characteristic_id, v_drug_info_id, 1, 'serialNumber', 'FDA-device', 'Serial Number',
        'text', 'SN-DEMO-2026-001', NULL, NULL, NULL,
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO narrative_information (
        id, case_id, case_narrative, reporter_comments, sender_comments,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_narrative_id, v_case_id,
        'A 37-year-old female experienced severe headache and elevated ALT one day after starting Demozumab for migraine prophylaxis. The suspect product was stopped, symptomatic therapy was given, and the event improved after discontinuation.',
        'Temporal association appears plausible. Recommend follow-up on liver enzymes.',
        'Case prepared for internal training and UI walkthrough purposes.',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO sender_diagnoses (
        id, narrative_id, sequence_number, diagnosis_meddra_version, diagnosis_meddra_code,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_sender_diag_id, v_narrative_id, 1, '27.1', '10027310',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO case_summary_information (
        id, narrative_id, sequence_number, summary_type, language_code, summary_text,
        created_by, updated_by, created_at, updated_at
    )
    VALUES (
        v_case_summary_id, v_narrative_id, 1, '01', 'en',
        'Serious case with death and hospitalization criteria populated for demo coverage; values are synthetic and for non-production testing only.',
        v_user_id, v_user_id, NOW(), NOW()
    )
    ON CONFLICT (id) DO NOTHING;
END;
$$;
