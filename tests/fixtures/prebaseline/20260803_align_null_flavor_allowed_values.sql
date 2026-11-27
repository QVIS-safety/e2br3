-- Historical test fixture only; never loaded by database initialization.
UPDATE safety_report_identification SET
    fulfil_expedited_criteria_null_flavor = CASE WHEN fulfil_expedited_criteria_null_flavor IN ('NI') THEN fulfil_expedited_criteria_null_flavor END,
    other_case_identifiers_exist_null_flavor = CASE WHEN other_case_identifiers_exist_null_flavor IN ('NI') THEN other_case_identifiers_exist_null_flavor END;
UPDATE study_fda_cross_reported_inds SET ind_number_null_flavor = NULL WHERE ind_number_null_flavor IS NOT NULL AND ind_number_null_flavor NOT IN ('NA');
UPDATE primary_sources SET
    email_null_flavor = CASE WHEN email_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN email_null_flavor END,
    qualification_null_flavor = CASE WHEN qualification_null_flavor IN ('UNK') THEN qualification_null_flavor END;
UPDATE primary_sources SET qualification = NULL WHERE qualification_null_flavor IS NOT NULL;
UPDATE reporter_presaves SET
    qualification_null_flavor = CASE WHEN qualification_null_flavor IN ('UNK') THEN qualification_null_flavor END;
UPDATE patient_information SET
    race_code_null_flavor = CASE WHEN race_code_null_flavor IN ('MSK', 'UNK', 'NA', 'OTH') THEN race_code_null_flavor END,
    ethnicity_code_null_flavor = CASE WHEN ethnicity_code_null_flavor IN ('NI', 'MSK', 'UNK', 'NA') THEN ethnicity_code_null_flavor END,
    last_menstrual_period_date_null_flavor = CASE WHEN last_menstrual_period_date_null_flavor IN ('MSK') THEN last_menstrual_period_date_null_flavor END,
    patient_initials_null_flavor = CASE WHEN patient_initials_null_flavor IN ('MSK', 'UNK', 'ASKU', 'NASK') THEN patient_initials_null_flavor END,
    birth_date_null_flavor = CASE WHEN birth_date_null_flavor IN ('MSK') THEN birth_date_null_flavor END,
    sex_null_flavor = CASE WHEN sex_null_flavor IN ('MSK', 'UNK', 'ASKU', 'NASK') THEN sex_null_flavor END;
UPDATE medical_history_episodes SET
    start_date_null_flavor = CASE WHEN start_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN start_date_null_flavor END,
    end_date_null_flavor = CASE WHEN end_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN end_date_null_flavor END;
UPDATE past_drug_history SET
    drug_name_null_flavor = CASE WHEN drug_name_null_flavor IN ('UNK', 'NA') THEN drug_name_null_flavor END,
    start_date_null_flavor = CASE WHEN start_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN start_date_null_flavor END,
    end_date_null_flavor = CASE WHEN end_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN end_date_null_flavor END;
UPDATE patient_death_information SET date_of_death_null_flavor = NULL WHERE date_of_death_null_flavor IS NOT NULL AND date_of_death_null_flavor NOT IN ('MSK', 'ASKU', 'NASK');
UPDATE patient_death_information SET autopsy_performed_null_flavor = NULL WHERE autopsy_performed_null_flavor IS NOT NULL AND autopsy_performed_null_flavor NOT IN ('UNK', 'ASKU', 'NASK');
UPDATE parent_information SET
    parent_birth_date_null_flavor = CASE WHEN parent_birth_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN parent_birth_date_null_flavor END,
    last_menstrual_period_date_null_flavor = CASE WHEN last_menstrual_period_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN last_menstrual_period_date_null_flavor END;
UPDATE parent_medical_history SET
    start_date_null_flavor = CASE WHEN start_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN start_date_null_flavor END,
    end_date_null_flavor = CASE WHEN end_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN end_date_null_flavor END;
UPDATE reactions SET
    criteria_death_null_flavor = CASE WHEN criteria_death_null_flavor IN ('NI') THEN criteria_death_null_flavor END,
    criteria_life_threatening_null_flavor = CASE WHEN criteria_life_threatening_null_flavor IN ('NI') THEN criteria_life_threatening_null_flavor END,
    criteria_hospitalization_null_flavor = CASE WHEN criteria_hospitalization_null_flavor IN ('NI') THEN criteria_hospitalization_null_flavor END,
    criteria_disabling_null_flavor = CASE WHEN criteria_disabling_null_flavor IN ('NI') THEN criteria_disabling_null_flavor END,
    criteria_congenital_anomaly_null_flavor = CASE WHEN criteria_congenital_anomaly_null_flavor IN ('NI') THEN criteria_congenital_anomaly_null_flavor END,
    criteria_other_medically_important_null_flavor = CASE WHEN criteria_other_medically_important_null_flavor IN ('NI') THEN criteria_other_medically_important_null_flavor END,
    required_intervention_null_flavor = CASE WHEN required_intervention_null_flavor IN ('NI') THEN required_intervention_null_flavor END,
    start_date_null_flavor = CASE WHEN start_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN start_date_null_flavor END,
    end_date_null_flavor = CASE WHEN end_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN end_date_null_flavor END;
UPDATE test_results SET
    test_date_null_flavor = CASE WHEN test_date_null_flavor IN ('UNK') THEN test_date_null_flavor END;
UPDATE dosage_information SET
    dose_form_null_flavor = CASE WHEN dose_form_null_flavor IN ('UNK', 'ASKU', 'NASK') THEN dose_form_null_flavor END,
    route_of_administration_null_flavor = CASE WHEN route_of_administration_null_flavor IN ('UNK', 'ASKU', 'NASK') THEN route_of_administration_null_flavor END,
    parent_route_null_flavor = CASE WHEN parent_route_null_flavor IN ('UNK', 'ASKU', 'NASK') THEN parent_route_null_flavor END,
    first_administration_date_null_flavor = CASE WHEN first_administration_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN first_administration_date_null_flavor END,
    last_administration_date_null_flavor = CASE WHEN last_administration_date_null_flavor IN ('MSK', 'ASKU', 'NASK') THEN last_administration_date_null_flavor END;
UPDATE drug_indications SET indication_text_null_flavor = NULL WHERE indication_text_null_flavor IS NOT NULL AND indication_text_null_flavor NOT IN ('UNK', 'ASKU', 'NASK');
UPDATE relatedness_assessments SET result_of_assessment_kr1_null_flavor = NULL WHERE result_of_assessment_kr1_null_flavor IS NOT NULL AND result_of_assessment_kr1_null_flavor NOT IN ('NA');

ALTER TABLE safety_report_identification
    DROP CONSTRAINT IF EXISTS safety_report_identification_fulfil_expedited_criteria_null_flavor_check,
    DROP CONSTRAINT IF EXISTS safety_report_identification_other_case_identifiers_exist_null_flavor_check,
    ADD CONSTRAINT ck_nf_sri_expedited CHECK (fulfil_expedited_criteria_null_flavor IS NULL OR fulfil_expedited_criteria_null_flavor IN ('NI')),
    ADD CONSTRAINT ck_nf_sri_other_ids CHECK (other_case_identifiers_exist_null_flavor IS NULL OR other_case_identifiers_exist_null_flavor IN ('NI'));

ALTER TABLE study_fda_cross_reported_inds
    DROP CONSTRAINT IF EXISTS study_fda_cross_reported_inds_ind_number_null_flavor_check,
    ADD CONSTRAINT ck_nf_study_fda_ind CHECK (ind_number_null_flavor IS NULL OR ind_number_null_flavor IN ('NA'));

ALTER TABLE primary_sources
    DROP CONSTRAINT IF EXISTS primary_sources_email_null_flavor_check,
    DROP CONSTRAINT IF EXISTS primary_sources_qualification_null_flavor_check,
    DROP CONSTRAINT IF EXISTS ck_pair_primary_qualification,
    ADD CONSTRAINT ck_nf_primary_email CHECK (email_null_flavor IS NULL OR email_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_primary_qualification CHECK (qualification_null_flavor IS NULL OR qualification_null_flavor IN ('UNK')),
    ADD CONSTRAINT ck_pair_primary_qualification CHECK (NOT (qualification IS NOT NULL AND qualification_null_flavor IS NOT NULL));

ALTER TABLE reporter_presaves
    DROP CONSTRAINT IF EXISTS reporter_presaves_qualification_null_flavor_check,
    ADD CONSTRAINT ck_nf_presave_qualification CHECK (qualification_null_flavor IS NULL OR qualification_null_flavor IN ('UNK'));

ALTER TABLE patient_information
    DROP CONSTRAINT IF EXISTS patient_information_race_code_null_flavor_check,
    DROP CONSTRAINT IF EXISTS patient_information_ethnicity_code_null_flavor_check,
    DROP CONSTRAINT IF EXISTS patient_information_last_menstrual_period_date_null_flavor_check,
    DROP CONSTRAINT IF EXISTS patient_information_patient_initials_null_flavor_check,
    DROP CONSTRAINT IF EXISTS patient_information_birth_date_null_flavor_check,
    DROP CONSTRAINT IF EXISTS patient_information_sex_null_flavor_check,
    ADD CONSTRAINT ck_nf_patient_race CHECK (race_code_null_flavor IS NULL OR race_code_null_flavor IN ('MSK', 'UNK', 'NA', 'OTH')),
    ADD CONSTRAINT ck_nf_patient_ethnicity CHECK (ethnicity_code_null_flavor IS NULL OR ethnicity_code_null_flavor IN ('NI', 'MSK', 'UNK', 'NA')),
    ADD CONSTRAINT ck_nf_patient_lmp CHECK (last_menstrual_period_date_null_flavor IS NULL OR last_menstrual_period_date_null_flavor IN ('MSK')),
    ADD CONSTRAINT ck_nf_patient_initials CHECK (patient_initials_null_flavor IS NULL OR patient_initials_null_flavor IN ('MSK', 'UNK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_patient_birth CHECK (birth_date_null_flavor IS NULL OR birth_date_null_flavor IN ('MSK')),
    ADD CONSTRAINT ck_nf_patient_sex CHECK (sex_null_flavor IS NULL OR sex_null_flavor IN ('MSK', 'UNK', 'ASKU', 'NASK'));

ALTER TABLE medical_history_episodes
    DROP CONSTRAINT IF EXISTS medical_history_episodes_start_date_null_flavor_check,
    DROP CONSTRAINT IF EXISTS medical_history_episodes_end_date_null_flavor_check,
    ADD CONSTRAINT ck_nf_med_history_start CHECK (start_date_null_flavor IS NULL OR start_date_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_med_history_end CHECK (end_date_null_flavor IS NULL OR end_date_null_flavor IN ('MSK', 'ASKU', 'NASK'));

ALTER TABLE past_drug_history
    DROP CONSTRAINT IF EXISTS past_drug_history_drug_name_null_flavor_check,
    DROP CONSTRAINT IF EXISTS past_drug_history_start_date_null_flavor_check,
    DROP CONSTRAINT IF EXISTS past_drug_history_end_date_null_flavor_check,
    ADD CONSTRAINT ck_nf_past_drug_name CHECK (drug_name_null_flavor IS NULL OR drug_name_null_flavor IN ('UNK', 'NA')),
    ADD CONSTRAINT ck_nf_past_drug_start CHECK (start_date_null_flavor IS NULL OR start_date_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_past_drug_end CHECK (end_date_null_flavor IS NULL OR end_date_null_flavor IN ('MSK', 'ASKU', 'NASK'));

ALTER TABLE patient_death_information
    DROP CONSTRAINT IF EXISTS patient_death_information_date_of_death_null_flavor_check,
    DROP CONSTRAINT IF EXISTS patient_death_information_autopsy_performed_null_flavor_check,
    DROP CONSTRAINT IF EXISTS ck_nf_patient_autopsy,
    ADD CONSTRAINT ck_nf_patient_death_date CHECK (date_of_death_null_flavor IS NULL OR date_of_death_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_patient_autopsy CHECK (autopsy_performed_null_flavor IS NULL OR autopsy_performed_null_flavor IN ('UNK', 'ASKU', 'NASK'));

ALTER TABLE parent_information
    DROP CONSTRAINT IF EXISTS parent_information_parent_birth_date_null_flavor_check,
    DROP CONSTRAINT IF EXISTS parent_information_last_menstrual_period_date_null_flavor_check,
    ADD CONSTRAINT ck_nf_parent_birth CHECK (parent_birth_date_null_flavor IS NULL OR parent_birth_date_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_parent_lmp CHECK (last_menstrual_period_date_null_flavor IS NULL OR last_menstrual_period_date_null_flavor IN ('MSK', 'ASKU', 'NASK'));

ALTER TABLE parent_medical_history
    DROP CONSTRAINT IF EXISTS parent_medical_history_start_date_null_flavor_check,
    DROP CONSTRAINT IF EXISTS parent_medical_history_end_date_null_flavor_check,
    ADD CONSTRAINT ck_nf_parent_history_start CHECK (start_date_null_flavor IS NULL OR start_date_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_parent_history_end CHECK (end_date_null_flavor IS NULL OR end_date_null_flavor IN ('MSK', 'ASKU', 'NASK'));

ALTER TABLE reactions
    DROP CONSTRAINT IF EXISTS reactions_criteria_death_null_flavor_check,
    DROP CONSTRAINT IF EXISTS reactions_criteria_life_threatening_null_flavor_check,
    DROP CONSTRAINT IF EXISTS reactions_criteria_hospitalization_null_flavor_check,
    DROP CONSTRAINT IF EXISTS reactions_criteria_disabling_null_flavor_check,
    DROP CONSTRAINT IF EXISTS reactions_criteria_congenital_anomaly_null_flavor_check,
    DROP CONSTRAINT IF EXISTS reactions_criteria_other_medically_important_null_flavor_check,
    DROP CONSTRAINT IF EXISTS reactions_required_intervention_null_flavor_check,
    DROP CONSTRAINT IF EXISTS reactions_start_date_null_flavor_check,
    DROP CONSTRAINT IF EXISTS reactions_end_date_null_flavor_check,
    ADD CONSTRAINT ck_nf_reaction_death CHECK (criteria_death_null_flavor IS NULL OR criteria_death_null_flavor IN ('NI')),
    ADD CONSTRAINT ck_nf_reaction_life CHECK (criteria_life_threatening_null_flavor IS NULL OR criteria_life_threatening_null_flavor IN ('NI')),
    ADD CONSTRAINT ck_nf_reaction_hospital CHECK (criteria_hospitalization_null_flavor IS NULL OR criteria_hospitalization_null_flavor IN ('NI')),
    ADD CONSTRAINT ck_nf_reaction_disabling CHECK (criteria_disabling_null_flavor IS NULL OR criteria_disabling_null_flavor IN ('NI')),
    ADD CONSTRAINT ck_nf_reaction_congenital CHECK (criteria_congenital_anomaly_null_flavor IS NULL OR criteria_congenital_anomaly_null_flavor IN ('NI')),
    ADD CONSTRAINT ck_nf_reaction_other CHECK (criteria_other_medically_important_null_flavor IS NULL OR criteria_other_medically_important_null_flavor IN ('NI')),
    ADD CONSTRAINT ck_nf_reaction_intervention CHECK (required_intervention_null_flavor IS NULL OR required_intervention_null_flavor IN ('NI')),
    ADD CONSTRAINT ck_nf_reaction_start CHECK (start_date_null_flavor IS NULL OR start_date_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_reaction_end CHECK (end_date_null_flavor IS NULL OR end_date_null_flavor IN ('MSK', 'ASKU', 'NASK'));

ALTER TABLE test_results
    DROP CONSTRAINT IF EXISTS test_results_test_date_null_flavor_check,
    ADD CONSTRAINT ck_nf_test_date CHECK (test_date_null_flavor IS NULL OR test_date_null_flavor IN ('UNK'));

ALTER TABLE dosage_information
    DROP CONSTRAINT IF EXISTS dosage_information_dose_form_null_flavor_check,
    DROP CONSTRAINT IF EXISTS dosage_information_route_of_administration_null_flavor_check,
    DROP CONSTRAINT IF EXISTS dosage_information_parent_route_null_flavor_check,
    DROP CONSTRAINT IF EXISTS dosage_information_first_administration_date_null_flavor_check,
    DROP CONSTRAINT IF EXISTS dosage_information_last_administration_date_null_flavor_check,
    ADD CONSTRAINT ck_nf_dosage_form CHECK (dose_form_null_flavor IS NULL OR dose_form_null_flavor IN ('UNK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_dosage_route CHECK (route_of_administration_null_flavor IS NULL OR route_of_administration_null_flavor IN ('UNK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_dosage_parent_route CHECK (parent_route_null_flavor IS NULL OR parent_route_null_flavor IN ('UNK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_dosage_first_date CHECK (first_administration_date_null_flavor IS NULL OR first_administration_date_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD CONSTRAINT ck_nf_dosage_last_date CHECK (last_administration_date_null_flavor IS NULL OR last_administration_date_null_flavor IN ('MSK', 'ASKU', 'NASK'));

ALTER TABLE drug_indications
    DROP CONSTRAINT IF EXISTS drug_indications_indication_text_null_flavor_check,
    ADD CONSTRAINT ck_nf_drug_indication CHECK (indication_text_null_flavor IS NULL OR indication_text_null_flavor IN ('UNK', 'ASKU', 'NASK'));

ALTER TABLE relatedness_assessments
    DROP CONSTRAINT IF EXISTS relatedness_assessments_result_of_assessment_kr1_null_flavor_check,
    ADD CONSTRAINT ck_nf_relatedness_kr1 CHECK (result_of_assessment_kr1_null_flavor IS NULL OR result_of_assessment_kr1_null_flavor IN ('NA'));
